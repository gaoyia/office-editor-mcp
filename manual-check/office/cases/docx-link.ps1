# docx 链接前节：docs/checklist-docx.md 的 L1–L4。
# 源文件由 Word 亲手插入分节并勾选「链接到前一节」后另存。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-DocxWordLinkSource
$word = Ensure-MsWord $src

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-link-inspect'

$s2 = @($tree0.sections) | Where-Object { $_.id -eq 'section:2' } | Select-Object -First 1
$srcDoc = Open-WordDoc $word $src
$srcSec2 = $srcDoc.Sections.Item(2)
$srcHdr2 = $srcSec2.Headers.Item(1)
$srcLinked = [bool]$srcHdr2.LinkToPrevious
Add-Check 'docx-L1' (($null -ne $s2) -and ($s2.headerLink.default -eq $true) -and $srcLinked) "link=$($s2.headerLink.default) com=$srcLinked"
Release-Com $srcHdr2
Release-Com $srcSec2

$bodyBefore = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-link-body-before'
$bodyTexts = @($bodyBefore.paragraphs | ForEach-Object { $_.text })

$unlinked = Get-OutPath 'docx-link-unlinked.docx'
Close-WordDocByPath $word $unlinked
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $unlinked.Replace('\', '/')
    ops = @(@{
        op = 'setHeaderFooterLink'
        id = 'section:2'
        kind = 'header'
        type = 'default'
        linked = $false
    })
} 'docx-link-unlink' | Out-Null

$doc = Open-WordDoc $word $unlinked
$caption = [string]$doc.ActiveWindow.Caption
$sec2 = $doc.Sections.Item(2)
$hdr2 = $sec2.Headers.Item(1)
$nowLinked = [bool]$hdr2.LinkToPrevious
Add-Check 'docx-L2' (($caption -notmatch 'Repair') -and (-not $nowLinked)) "caption=$caption linked=$nowLinked"
Release-Com $hdr2
Release-Com $sec2

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $unlinked.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true }
} 'docx-link-inspect-unlinked'
$h2 = @($tree1.headers) | Where-Object {
    $_.type -eq 'default' -and (@($_.sections) -contains 2)
} | Select-Object -First 1
$hid = @($h2.paragraphs)[0].id

$applied = Get-OutPath 'docx-link-applied.docx'
Close-WordDocByPath $word $applied
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $unlinked.Replace('\', '/')
    output = $applied.Replace('\', '/')
    ops = @(@{ op = 'setParaText'; id = $hid; text = $S.headerSec2New })
} 'docx-link-edit' | Out-Null

$doc2 = Open-WordDoc $word $applied
$sec1 = $doc2.Sections.Item(1)
$sec2b = $doc2.Sections.Item(2)
$hdr1 = $sec1.Headers.Item(1)
$hdr2b = $sec2b.Headers.Item(1)
$t1 = ([string]$hdr1.Range.Text).TrimEnd([char]13, [char]7, [char]10)
$t2 = ([string]$hdr2b.Range.Text).TrimEnd([char]13, [char]7, [char]10)
Add-Check 'docx-L3' (($t1 -eq $S.headerSrc) -and ($t2 -eq $S.headerSec2New)) "s1=$t1 s2=$t2"
Release-Com $hdr2b
Release-Com $hdr1
Release-Com $sec2b
Release-Com $sec1

$bodyAfter = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $applied.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-link-body-after'
$bodyTexts2 = @($bodyAfter.paragraphs | ForEach-Object { $_.text })
$p1 = Get-ParaText $doc2.Paragraphs.Item(1)
$bodyOk = (@(Compare-Object $bodyTexts $bodyTexts2).Count -eq 0) -and ($p1 -eq $S.chapter1)
Add-Check 'docx-L4' $bodyOk "p1=$p1"
