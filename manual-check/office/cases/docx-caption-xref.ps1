# docx 题注与交叉引用：docs/checklist-docx.md 的 Cp1–Cp6。
# 用 ooxp 插入 SEQ / REF，再用 Word 更新域核对可见 Figure，最后删掉。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings

function Invoke-OoxpExpectFail($obj, [string]$name) {
    $path = Join-Path $global:OoxpReqDir "$name.json"
    Write-JsonFile $path $obj
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $global:OoxpExe
    $psi.Arguments = "--input `"$path`""
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = [System.Text.Encoding]::UTF8
    $psi.StandardErrorEncoding = [System.Text.Encoding]::UTF8
    $proc = [System.Diagnostics.Process]::Start($psi)
    $stdout = $proc.StandardOutput.ReadToEnd().Trim()
    $stderr = $proc.StandardError.ReadToEnd().Trim()
    $proc.WaitForExit()
    $code = $proc.ExitCode
    $proc.Dispose()
    Write-Log "ooxp $name 退出码 $code stderr=$stderr"
    return @{ ExitCode = $code; Stderr = $stderr; Stdout = $stdout }
}

function Get-FieldResultText($doc) {
    $parts = @()
    $fields = $doc.Fields
    try {
        foreach ($f in @($fields)) {
            try {
                [void]$f.Update()
                $r = $null
                try {
                    $r = $f.Result
                    $parts += [string]$r.Text
                } finally { Release-Com $r }
            } finally { Release-Com $f }
        }
    } finally { Release-Com $fields }
    return ($parts -join ' ')
}

$src = Get-OutPath 'docx-caption-xref-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = $S.chapter1 }
            @{ text = $S.hello }
        )
        headers = @(@{ type = 'default'; paragraphs = @(@{ text = $S.headerSrc }) })
    }
} 'docx-caption-xref-create' | Out-Null

$bare = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-caption-xref-inspect-bare'
$bareHasCap = $null -ne $bare.captions
$bareHasXref = $null -ne $bare.crossRefs

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; headerFooter = $true }
} 'docx-caption-xref-inspect-src'
$p1 = [string]$srcTree.paragraphs[0].id
$hfPara = [string]$srcTree.headers[0].paragraphs[0].id

$cap = Get-OutPath 'docx-caption-xref-insert.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $cap.Replace('\', '/')
    ops = @(@{ op = 'insertCaption'; id = $p1; label = 'Figure'; title = 'diagram' })
} 'docx-caption-xref-insert' | Out-Null

$treeCap = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cap.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; captions = $true; fields = $true }
} 'docx-caption-xref-inspect-insert'
$caps = @($treeCap.captions)
$fields = @($treeCap.fields)
$hit = @($caps | Where-Object { [string]$_.id -eq 'caption:1' }) | Select-Object -First 1
$fld = @($fields | Where-Object { [string]$_.code -eq 'SEQ' }) | Select-Object -First 1
$bm = [string]$hit.bookmark
Add-Check 'docx-Cp1' (
    ([int]$bare.captionCount -eq 0) -and ([int]$bare.crossRefCount -eq 0) -and
    (-not $bareHasCap) -and (-not $bareHasXref) -and
    ([int]$treeCap.captionCount -eq 1) -and
    ($null -ne $hit) -and ([string]$hit.identifier -eq 'Figure') -and
    ($null -ne $fld) -and ([int]$treeCap.bookmarkCount -eq 0) -and
    ($bm -like '_Ref*')
) "bareCap=$($bare.captionCount) bareList=$bareHasCap count=$($treeCap.captionCount) ident=$($hit.identifier) bm=$bm field=$($fld.id) bmCount=$($treeCap.bookmarkCount)"

$word = Ensure-MsWord $src
$docC = Open-WordDoc $word $cap
$capC = [string]$docC.ActiveWindow.Caption
Add-Check 'docx-Cp2' ($capC -notmatch 'Repair') $capC
Close-WordDoc $docC

$xref = Get-OutPath 'docx-caption-xref-xref.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $cap.Replace('\', '/')
    output = $xref.Replace('\', '/')
    ops = @(@{ op = 'insertCrossRef'; id = $p1; bookmark = $bm; kind = 'ref' })
} 'docx-caption-xref-xref' | Out-Null

$treeX = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $xref.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; crossRefs = $true }
} 'docx-caption-xref-inspect-xref'
$xrefs = @($treeX.crossRefs)
$xhit = @($xrefs | Where-Object { [string]$_.id -eq 'xref:1' }) | Select-Object -First 1

$docX = Open-WordDoc $word $xref
$capX = [string]$docX.ActiveWindow.Caption
$xrefText = Get-FieldResultText $docX
Add-Check 'docx-Cp3' (
    ([int]$treeX.crossRefCount -eq 1) -and ([string]$xhit.kind -eq 'ref') -and
    ($capX -notmatch 'Repair') -and ($xrefText -match 'Figure')
) "count=$($treeX.crossRefCount) kind=$($xhit.kind) caption=$capX text=$xrefText"
Close-WordDoc $docX

$goneX = Get-OutPath 'docx-caption-xref-del-xref.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $xref.Replace('\', '/')
    output = $goneX.Replace('\', '/')
    ops = @(@{ op = 'deleteCrossRef'; id = 'xref:1' })
} 'docx-caption-xref-del-xref' | Out-Null

$treeDx = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $goneX.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; captions = $true; crossRefs = $true }
} 'docx-caption-xref-inspect-del-xref'
$dxTexts = @($treeDx.paragraphs | ForEach-Object { [string]$_.text })
$docDx = Open-WordDoc $word $goneX
$capDx = [string]$docDx.ActiveWindow.Caption
Add-Check 'docx-Cp4' (
    ([int]$treeDx.crossRefCount -eq 0) -and
    ([int]$treeDx.captionCount -eq 1) -and
    ($dxTexts -contains $S.chapter1) -and
    ($capDx -notmatch 'Repair')
) "xref=$($treeDx.crossRefCount) cap=$($treeDx.captionCount) texts=$($dxTexts -join '|') caption=$capDx"
Close-WordDoc $docDx

$gone = Get-OutPath 'docx-caption-xref-del-cap.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $goneX.Replace('\', '/')
    output = $gone.Replace('\', '/')
    ops = @(@{ op = 'deleteCaption'; id = 'caption:1' })
} 'docx-caption-xref-del-cap' | Out-Null

$treeDel = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $gone.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; captions = $true }
} 'docx-caption-xref-inspect-delete'
$delTexts = @($treeDel.paragraphs | ForEach-Object { [string]$_.text })
$docD = Open-WordDoc $word $gone
$capD = [string]$docD.ActiveWindow.Caption
Add-Check 'docx-Cp5' (
    ([int]$treeDel.captionCount -eq 0) -and
    ($delTexts -contains $S.chapter1) -and
    ($capD -notmatch 'Repair')
) "count=$($treeDel.captionCount) texts=$($delTexts -join '|') caption=$capD"
Close-WordDoc $docD

$hfOut = Get-OutPath 'docx-caption-xref-hf.docx'
Close-WordDocByPath $word $hfOut
if (Test-Path $hfOut) { Remove-Item -Force $hfOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $hfOut.Replace('\', '/')
    ops = @(@{ op = 'insertCaption'; id = $hfPara; label = 'Figure'; title = 'header' })
} 'docx-caption-xref-fail-hf' | Out-Null
$treeHf = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfOut.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true; captions = $true }
} 'docx-caption-xref-inspect-hf'
$hfId = ''
if ($treeHf.headers -and $treeHf.headers[0].captions) {
    $hfId = [string]$treeHf.headers[0].captions[0].id
}
$docHf = Open-WordDoc $word $hfOut
$capHf = [string]$docHf.ActiveWindow.Caption
Close-WordDoc $docHf

$xlsx = Get-OutPath 'docx-caption-xref-xlsx.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-caption-xref-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-caption-xref-fail-xlsx.xlsx'
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'insertCaption'; id = 'docx:document' })
} 'docx-caption-xref-fail-xlsx'
try { $codeX = ($rawX.Stderr | ConvertFrom-Json).error.code } catch { $codeX = $null }

Add-Check 'docx-Cp6' (
    ($hfId -like 'header:*/caption:*') -and
    ([int]$treeHf.captionCount -eq 0) -and
    ([int]$treeHf.headers[0].captionCount -eq 1) -and
    ($capHf -notmatch 'Repair') -and
    ($rawX.ExitCode -ne 0) -and ($codeX -eq 'InvalidField') -and (-not (Test-Path $failX))
) "hf=$hfId root=$($treeHf.captionCount) header=$($treeHf.headers[0].captionCount) caption=$capHf xlsx=$codeX"
