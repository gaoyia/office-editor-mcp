# docx 索引：docs/checklist-docx.md 的 Ix1–Ix6。
# 用 ooxp 插入 XE / INDEX，再用 Word 更新域核对可见 fox，最后删掉。

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

function Get-IndexResultText($doc) {
    $parts = @()
    $fields = $doc.Fields
    try {
        foreach ($f in @($fields)) {
            try {
                [void]$f.Update()
                # wdFieldIndex = 8
                if ([int]$f.Type -eq 8) {
                    $r = $null
                    try {
                        $r = $f.Result
                        $parts += [string]$r.Text
                    } finally { Release-Com $r }
                }
            } finally { Release-Com $f }
        }
    } finally { Release-Com $fields }
    $idxs = $doc.Indexes
    try {
        if ([int]$idxs.Count -ge 1) {
            $ix = $idxs.Item(1)
            try {
                $r = $ix.Range
                try { $parts += [string]$r.Text } finally { Release-Com $r }
            } finally { Release-Com $ix }
        }
    } finally { Release-Com $idxs }
    return ($parts -join ' ')
}

$src = Get-OutPath 'docx-index-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = $S.chapter1 }
            @{ text = 'the quick brown fox' }
        )
        headers = @(@{ type = 'default'; paragraphs = @(@{ text = $S.headerSrc }) })
    }
} 'docx-index-create' | Out-Null

$bare = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-index-inspect-bare'
$bareHasIx = $null -ne $bare.indexes
$bareHasXe = $null -ne $bare.indexEntries

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; headerFooter = $true }
} 'docx-index-inspect-src'
$p2 = [string]$srcTree.paragraphs[1].id
$hfPara = [string]$srcTree.headers[0].paragraphs[0].id

$xe = Get-OutPath 'docx-index-entry.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $xe.Replace('\', '/')
    ops = @(@{ op = 'insertIndexEntry'; id = $p2; entry = 'fox' })
} 'docx-index-entry' | Out-Null

$ix = Get-OutPath 'docx-index-insert.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $xe.Replace('\', '/')
    output = $ix.Replace('\', '/')
    ops = @(@{ op = 'insertIndex'; id = 'docx:document' })
} 'docx-index-insert' | Out-Null

$treeCap = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ix.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; indexes = $true; indexEntries = $true; fields = $true }
} 'docx-index-inspect-insert'
$indexes = @($treeCap.indexes)
$entries = @($treeCap.indexEntries)
$fields = @($treeCap.fields)
$ihit = @($indexes | Where-Object { [string]$_.id -eq 'index:1' }) | Select-Object -First 1
$ehit = @($entries | Where-Object { [string]$_.id -eq 'xe:1' }) | Select-Object -First 1
$fldIx = @($fields | Where-Object { [string]$_.code -eq 'INDEX' }) | Select-Object -First 1
$fldXe = @($fields | Where-Object { [string]$_.code -eq 'XE' }) | Select-Object -First 1
Add-Check 'docx-Ix1' (
    ([int]$bare.indexCount -eq 0) -and ([int]$bare.indexEntryCount -eq 0) -and
    (-not $bareHasIx) -and (-not $bareHasXe) -and
    ([int]$treeCap.indexCount -eq 1) -and ([int]$treeCap.indexEntryCount -eq 1) -and
    ($null -ne $ihit) -and ($null -ne $ehit) -and ([string]$ehit.entry -eq 'fox') -and
    ($null -ne $fldIx) -and ($null -ne $fldXe)
) "bareIx=$($bare.indexCount) bareList=$bareHasIx count=$($treeCap.indexCount) xe=$($treeCap.indexEntryCount) entry=$($ehit.entry)"

$word = Ensure-MsWord $src
$docC = Open-WordDoc $word $ix
$capC = [string]$docC.ActiveWindow.Caption
Add-Check 'docx-Ix2' ($capC -notmatch 'Repair') $capC
Close-WordDoc $docC

$docX = Open-WordDoc $word $ix
$capX = [string]$docX.ActiveWindow.Caption
$ixText = Get-IndexResultText $docX
Add-Check 'docx-Ix3' (
    ($capX -notmatch 'Repair') -and ($ixText -match 'fox')
) "caption=$capX text=$ixText"
Close-WordDoc $docX

$goneX = Get-OutPath 'docx-index-del-index.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $ix.Replace('\', '/')
    output = $goneX.Replace('\', '/')
    ops = @(@{ op = 'deleteIndex'; id = 'index:1' })
} 'docx-index-del-index' | Out-Null

$treeDx = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $goneX.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; indexes = $true; indexEntries = $true }
} 'docx-index-inspect-del-index'
$dxTexts = @($treeDx.paragraphs | ForEach-Object { [string]$_.text })
$docDx = Open-WordDoc $word $goneX
$capDx = [string]$docDx.ActiveWindow.Caption
Add-Check 'docx-Ix4' (
    ([int]$treeDx.indexCount -eq 0) -and
    ([int]$treeDx.indexEntryCount -eq 1) -and
    ($dxTexts -contains $S.chapter1) -and
    ($capDx -notmatch 'Repair')
) "ix=$($treeDx.indexCount) xe=$($treeDx.indexEntryCount) texts=$($dxTexts -join '|') caption=$capDx"
Close-WordDoc $docDx

$gone = Get-OutPath 'docx-index-del-entry.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $goneX.Replace('\', '/')
    output = $gone.Replace('\', '/')
    ops = @(@{ op = 'deleteIndexEntry'; id = 'xe:1' })
} 'docx-index-del-entry' | Out-Null

$treeDel = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $gone.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; indexEntries = $true }
} 'docx-index-inspect-delete'
$delTexts = @($treeDel.paragraphs | ForEach-Object { [string]$_.text })
$docD = Open-WordDoc $word $gone
$capD = [string]$docD.ActiveWindow.Caption
Add-Check 'docx-Ix5' (
    ([int]$treeDel.indexEntryCount -eq 0) -and
    ($delTexts -contains $S.chapter1) -and
    ($capD -notmatch 'Repair')
) "count=$($treeDel.indexEntryCount) texts=$($delTexts -join '|') caption=$capD"
Close-WordDoc $docD

$failHf = Get-OutPath 'docx-index-fail-hf.docx'
$rawHf = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failHf.Replace('\', '/')
    ops = @(@{ op = 'insertIndex'; id = $hfPara })
} 'docx-index-fail-hf'
try { $codeHf = ($rawHf.Stderr | ConvertFrom-Json).error.code } catch { $codeHf = $null }

$xlsx = Get-OutPath 'docx-index-xlsx.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-index-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-index-fail-xlsx.xlsx'
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'insertIndex'; id = 'docx:document' })
} 'docx-index-fail-xlsx'
try { $codeX = ($rawX.Stderr | ConvertFrom-Json).error.code } catch { $codeX = $null }

Add-Check 'docx-Ix6' (
    ($rawHf.ExitCode -ne 0) -and ($codeHf -eq 'InvalidId') -and (-not (Test-Path $failHf)) -and
    ($rawX.ExitCode -ne 0) -and ($codeX -eq 'InvalidField') -and (-not (Test-Path $failX))
) "hf=$codeHf wroteHf=$([bool](Test-Path $failHf)) xlsx=$codeX wroteX=$([bool](Test-Path $failX))"
