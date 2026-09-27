# docx 目录：docs/checklist-docx.md 的 Tc1–Tc5。
# 用 ooxp 插入 TOC / TOF 域，再用 Word 更新域核对可见标题，最后删掉目录块。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings

function Get-TocResultText($doc) {
    $parts = @()
    $fields = $doc.Fields
    try {
        foreach ($f in @($fields)) {
            try {
                if ([int]$f.Type -eq 13) {
                    [void]$f.Update()
                    $r = $null
                    try {
                        $r = $f.Result
                        $parts += [string]$r.Text
                    } finally { Release-Com $r }
                }
            } finally { Release-Com $f }
        }
    } finally { Release-Com $fields }
    $tocs = $null
    try {
        $tocs = $doc.TablesOfContents
        if ($null -ne $tocs -and [int]$tocs.Count -ge 1) {
            $item = $tocs.Item(1)
            try {
                [void]$item.Update()
                $rg = $null
                try {
                    $rg = $item.Range
                    $parts += [string]$rg.Text
                } finally { Release-Com $rg }
            } finally { Release-Com $item }
        }
    } finally { Release-Com $tocs }
    return ($parts -join ' ')
}

$src = Get-OutPath 'docx-toc-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = $S.chapter1 }
            @{ text = $S.hello }
        )
    }
} 'docx-toc-create' | Out-Null

$bare = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-toc-inspect-bare'
$bareHasList = $null -ne $bare.tocs

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-toc-inspect-src'
$p1 = [string]$srcTree.paragraphs[0].id

$toc = Get-OutPath 'docx-toc-insert.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $toc.Replace('\', '/')
    ops = @(@{ op = 'insertToc'; id = $p1; kind = 'toc' })
} 'docx-toc-insert' | Out-Null

$treeToc = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $toc.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; tocs = $true; fields = $true }
} 'docx-toc-inspect-insert'
$tocs = @($treeToc.tocs)
$fields = @($treeToc.fields)
$hit = @($tocs | Where-Object { [string]$_.id -eq 'toc:1' }) | Select-Object -First 1
$fld = @($fields | Where-Object { [string]$_.code -eq 'TOC' }) | Select-Object -First 1
Add-Check 'docx-Tc1' (
    ([int]$bare.tocCount -eq 0) -and (-not $bareHasList) -and
    ([int]$treeToc.tocCount -eq 1) -and
    ($null -ne $hit) -and ([string]$hit.kind -eq 'toc') -and
    ($null -ne $fld)
) "bareCount=$($bare.tocCount) bareList=$bareHasList count=$($treeToc.tocCount) kind=$($hit.kind) field=$($fld.id)"

$word = Ensure-MsWord $src
$docT = Open-WordDoc $word $toc
$capT = [string]$docT.ActiveWindow.Caption
Add-Check 'docx-Tc2' ($capT -notmatch 'Repair') $capT

$tocText = Get-TocResultText $docT
Add-Check 'docx-Tc3' (
    ($tocText -match [regex]::Escape($S.chapter1))
) "tocText=$tocText"
Close-WordDoc $docT

$gone = Get-OutPath 'docx-toc-delete.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $toc.Replace('\', '/')
    output = $gone.Replace('\', '/')
    ops = @(@{ op = 'deleteToc'; id = 'toc:1' })
} 'docx-toc-delete' | Out-Null

$treeDel = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $gone.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; tocs = $true }
} 'docx-toc-inspect-delete'
$delTexts = @($treeDel.paragraphs | ForEach-Object { [string]$_.text })
$docD = Open-WordDoc $word $gone
$capD = [string]$docD.ActiveWindow.Caption
Add-Check 'docx-Tc4' (
    ([int]$treeDel.tocCount -eq 0) -and
    ($delTexts -contains $S.chapter1) -and
    ($capD -notmatch 'Repair')
) "count=$($treeDel.tocCount) texts=$($delTexts -join '|') caption=$capD"
Close-WordDoc $docD

$tof = Get-OutPath 'docx-toc-tof.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $tof.Replace('\', '/')
    ops = @(@{ op = 'insertToc'; id = 'docx:document'; kind = 'tof' })
} 'docx-toc-tof' | Out-Null

$treeTof = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tof.Replace('\', '/'); format = 'docx'
    scope = @{ tocs = $true }
} 'docx-toc-inspect-tof'
$tofHit = @($treeTof.tocs) | Select-Object -First 1
$docF = Open-WordDoc $word $tof
$capF = [string]$docF.ActiveWindow.Caption
Add-Check 'docx-Tc5' (
    ([string]$tofHit.kind -eq 'tof') -and ($capF -notmatch 'Repair')
) "kind=$($tofHit.kind) caption=$capF"
Close-WordDoc $docF
