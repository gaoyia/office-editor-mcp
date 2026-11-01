# docx 参考文献：docs/checklist-docx.md 的 Bi1–Bi6。
# 用 ooxp 插入书目源、引文与书目，再用 Word 更新域核对可见 Ecma / 2006，最后删掉。

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

function Get-BiblioFieldText($doc) {
    $cite = ''
    $bib = ''
    $fields = $doc.Fields
    try {
        foreach ($f in @($fields)) {
            try {
                [void]$f.Update()
                $ty = [int]$f.Type
                $r = $null
                try {
                    $r = $f.Result
                    $t = [string]$r.Text
                    # wdFieldCitation = 96；wdFieldBibliography = 97
                    if ($ty -eq 96) { $cite += $t }
                    elseif ($ty -eq 97) { $bib += $t }
                } finally { Release-Com $r }
            } finally { Release-Com $f }
        }
    } finally { Release-Com $fields }
    return @{ Cite = $cite; Bib = $bib }
}

$src = Get-OutPath 'docx-biblio-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = $S.chapter1 }
            @{ text = 'hello' }
        )
        headers = @(@{ type = 'default'; paragraphs = @(@{ text = $S.headerSrc }) })
    }
} 'docx-biblio-create' | Out-Null

$bare = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-biblio-inspect-bare'
$bareHasSrc = $null -ne $bare.sources
$bareHasCite = $null -ne $bare.citations
$bareHasBib = $null -ne $bare.bibliographies

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; headerFooter = $true }
} 'docx-biblio-inspect-src'
$p2 = [string]$srcTree.paragraphs[1].id
$hfPara = [string]$srcTree.headers[0].paragraphs[0].id

$full = Get-OutPath 'docx-biblio-insert.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $full.Replace('\', '/')
    ops = @(
        @{
            op = 'insertSource'; id = 'docx:document'; tag = 'Book01'; sourceType = 'Book'
            title = 'Office Open XML'; year = '2006'; last = 'Ecma'; first = 'International'
            city = 'Geneva'; publisher = 'Ecma International'
        }
        @{ op = 'insertCitation'; id = $p2; tag = 'Book01' }
        @{ op = 'insertBibliography'; id = 'docx:document' }
    )
} 'docx-biblio-insert' | Out-Null

$treeCap = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $full.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; sources = $true; citations = $true; bibliographies = $true; fields = $true }
} 'docx-biblio-inspect-insert'
$sources = @($treeCap.sources)
$citations = @($treeCap.citations)
$biblios = @($treeCap.bibliographies)
$fields = @($treeCap.fields)
$shit = @($sources | Where-Object { [string]$_.id -eq 'source:1' }) | Select-Object -First 1
$chit = @($citations | Where-Object { [string]$_.id -eq 'cite:1' }) | Select-Object -First 1
$bhit = @($biblios | Where-Object { [string]$_.id -eq 'biblio:1' }) | Select-Object -First 1
$fldC = @($fields | Where-Object { [string]$_.code -eq 'CITATION' }) | Select-Object -First 1
$fldB = @($fields | Where-Object { [string]$_.code -eq 'BIBLIOGRAPHY' }) | Select-Object -First 1
Add-Check 'docx-Bi1' (
    ([int]$bare.sourceCount -eq 0) -and ([int]$bare.citationCount -eq 0) -and ([int]$bare.bibliographyCount -eq 0) -and
    (-not $bareHasSrc) -and (-not $bareHasCite) -and (-not $bareHasBib) -and
    ([int]$treeCap.sourceCount -eq 1) -and ([int]$treeCap.citationCount -eq 1) -and ([int]$treeCap.bibliographyCount -eq 1) -and
    ([int]$treeCap.contentControlCount -eq 0) -and
    ($null -ne $shit) -and ([string]$shit.tag -eq 'Book01') -and
    ($null -ne $chit) -and ($null -ne $bhit) -and
    ($null -ne $fldC) -and ($null -ne $fldB)
) "bareSrc=$($bare.sourceCount) countS=$($treeCap.sourceCount) cite=$($treeCap.citationCount) bib=$($treeCap.bibliographyCount) sdt=$($treeCap.contentControlCount)"

$word = Ensure-MsWord $src
$docC = Open-WordDoc $word $full
$capC = [string]$docC.ActiveWindow.Caption
Add-Check 'docx-Bi2' ($capC -notmatch 'Repair') $capC
Close-WordDoc $docC

$docX = Open-WordDoc $word $full
$capX = [string]$docX.ActiveWindow.Caption
$ft = Get-BiblioFieldText $docX
Add-Check 'docx-Bi3' (
    ($capX -notmatch 'Repair') -and ($ft.Cite -match 'Ecma') -and ($ft.Cite -match '2006') -and ($ft.Bib.Trim().Length -gt 0)
) "caption=$capX cite=$($ft.Cite) bib=$($ft.Bib)"
Close-WordDoc $docX

$goneB = Get-OutPath 'docx-biblio-del-bib.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $full.Replace('\', '/')
    output = $goneB.Replace('\', '/')
    ops = @(@{ op = 'deleteBibliography'; id = 'biblio:1' })
} 'docx-biblio-del-bib' | Out-Null

$treeDb = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $goneB.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; sources = $true; citations = $true; bibliographies = $true }
} 'docx-biblio-inspect-del-bib'
$dbTexts = @($treeDb.paragraphs | ForEach-Object { [string]$_.text })
$docDb = Open-WordDoc $word $goneB
$capDb = [string]$docDb.ActiveWindow.Caption
Add-Check 'docx-Bi4' (
    ([int]$treeDb.bibliographyCount -eq 0) -and
    ([int]$treeDb.sourceCount -eq 1) -and
    ([int]$treeDb.citationCount -eq 1) -and
    ($dbTexts -contains $S.chapter1) -and
    ($capDb -notmatch 'Repair')
) "bib=$($treeDb.bibliographyCount) src=$($treeDb.sourceCount) cite=$($treeDb.citationCount) caption=$capDb"
Close-WordDoc $docDb

$goneC = Get-OutPath 'docx-biblio-del-cite.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $goneB.Replace('\', '/')
    output = $goneC.Replace('\', '/')
    ops = @(@{ op = 'deleteCitation'; id = 'cite:1' })
} 'docx-biblio-del-cite' | Out-Null
$goneS = Get-OutPath 'docx-biblio-del-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $goneC.Replace('\', '/')
    output = $goneS.Replace('\', '/')
    ops = @(@{ op = 'deleteSource'; id = 'source:1' })
} 'docx-biblio-del-src' | Out-Null

$treeDel = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $goneS.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; sources = $true; citations = $true }
} 'docx-biblio-inspect-delete'
$delTexts = @($treeDel.paragraphs | ForEach-Object { [string]$_.text })
$docD = Open-WordDoc $word $goneS
$capD = [string]$docD.ActiveWindow.Caption
Add-Check 'docx-Bi5' (
    ([int]$treeDel.citationCount -eq 0) -and
    ([int]$treeDel.sourceCount -eq 0) -and
    ($delTexts -contains $S.chapter1) -and
    ($capD -notmatch 'Repair')
) "cite=$($treeDel.citationCount) src=$($treeDel.sourceCount) texts=$($delTexts -join '|') caption=$capD"
Close-WordDoc $docD

$hfOut = Get-OutPath 'docx-biblio-hf.docx'
Close-WordDocByPath $word $hfOut
if (Test-Path $hfOut) { Remove-Item -Force $hfOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $full.Replace('\', '/')
    output = $hfOut.Replace('\', '/')
    ops = @(@{ op = 'insertBibliography'; id = $hfPara })
} 'docx-biblio-fail-hf' | Out-Null
$treeHf = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfOut.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true; bibliographies = $true }
} 'docx-biblio-inspect-hf'
$hfId = ''
if ($treeHf.headers -and $treeHf.headers[0].bibliographies) {
    $hfId = [string]$treeHf.headers[0].bibliographies[0].id
}
$docHf = Open-WordDoc $word $hfOut
$capHf = [string]$docHf.ActiveWindow.Caption
$bib = ''
$hdr = $docHf.Sections.Item(1).Headers.Item(1)
$hfFields = $hdr.Range.Fields
if ($hfFields.Count -ge 1) {
    $f = $hfFields.Item(1)
    try { [void]$f.Update() } catch {}
    try { $bib = ([string]$f.Result.Text).Trim() } catch { $bib = '' }
    Release-Com $f
}
Release-Com $hfFields
Release-Com $hdr
Close-WordDoc $docHf

$xlsx = Get-OutPath 'docx-biblio-xlsx.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-biblio-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-biblio-fail-xlsx.xlsx'
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'insertBibliography'; id = 'docx:document' })
} 'docx-biblio-fail-xlsx'
try { $codeX = ($rawX.Stderr | ConvertFrom-Json).error.code } catch { $codeX = $null }

Add-Check 'docx-Bi6' (
    ($hfId -like 'header:*/biblio:1') -and
    ([int]$treeHf.bibliographyCount -eq 1) -and
    ([int]$treeHf.headers[0].bibliographyCount -eq 1) -and
    ($capHf -notmatch 'Repair') -and ($bib -match 'Ecma') -and ($bib -match '2006') -and
    ($rawX.ExitCode -ne 0) -and ($codeX -eq 'InvalidField') -and (-not (Test-Path $failX))
) "hf=$hfId bib=$bib caption=$capHf xlsx=$codeX"
