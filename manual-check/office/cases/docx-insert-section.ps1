# docx 插删节：docs/checklist-docx.md 的 Sc1–Sc6。
# 用 ooxp 插分节符、断开页眉、删节，再用 Word 打开核对无修复、无孤儿 header。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$wdPrimary = 1
$wdSectionNewPage = 2

function Get-ZipEntryText($z, [string]$name) {
    $e = $z.Entries | Where-Object { ($_.FullName -replace '\\', '/') -eq $name } | Select-Object -First 1
    if (-not $e) { return $null }
    $sr = New-Object IO.StreamReader($e.Open(), [Text.Encoding]::UTF8)
    try { $sr.ReadToEnd() } finally { $sr.Dispose() }
}

function Get-OrphanHeaderParts([string]$docx) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $z = [IO.Compression.ZipFile]::OpenRead($docx)
    try {
        $inZip = @(
            $z.Entries |
                ForEach-Object { $_.FullName -replace '\\', '/' } |
                Where-Object { $_ -match '^word/header\d+\.xml$' }
        )
        $doc = Get-ZipEntryText $z 'word/document.xml'
        $rels = Get-ZipEntryText $z 'word/_rels/document.xml.rels'
        $rids = @()
        if ($doc) {
            $rids = @(
                [regex]::Matches($doc, '<w:headerReference\b[^>]*/?>') |
                    ForEach-Object { [regex]::Match($_.Value, '\br:id="([^"]+)"').Groups[1].Value } |
                    Where-Object { $_ }
            )
        }
        $used = @()
        foreach ($rid in $rids) {
            if (-not $rels) { continue }
            $esc = [regex]::Escape($rid)
            $m = [regex]::Match($rels, "<Relationship\b[^>]*\bId=`"$esc`"[^>]*/?>")
            if (-not $m.Success) { continue }
            $t = [regex]::Match($m.Value, '\bTarget="([^"]+)"').Groups[1].Value
            if (-not $t) { continue }
            $norm = 'word/' + (($t -replace '\\', '/') -replace '^\./', '')
            $used += $norm
        }
        $orphans = @($inZip | Where-Object { $used -notcontains $_ })
        return @{ InZip = $inZip; Used = $used; Orphans = $orphans }
    }
    finally { $z.Dispose() }
}

function Test-HeaderCovers([object]$headers, [int]$n) {
    foreach ($h in @($headers)) {
        if ([string]$h.type -ne 'default') { continue }
        foreach ($s in @($h.sections)) {
            if ([int]$s -eq $n) { return $h }
        }
    }
    return $null
}

$src = Get-OutPath 'docx-insert-section-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = $S.chapter1 }
            @{ text = $S.hello }
        )
        headers = @(
            @{ type = 'default'; paragraphs = @(@{ text = $S.headerSrc }) }
        )
    }
} 'docx-insert-section-create' | Out-Null

$word = Ensure-MsWord $src

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-insert-section-inspect-src'
$p1 = @($tree0.paragraphs) | Where-Object { [string]$_.text -eq $S.chapter1 } | Select-Object -First 1

$split = Get-OutPath 'docx-insert-section-split.docx'
Close-WordDocByPath $word $split
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $split.Replace('\', '/')
    ops = @(@{ op = 'insertSection'; id = [string]$p1.id; type = 'nextPage' })
} 'docx-insert-section-insert' | Out-Null

$treeSplit = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $split.Replace('\', '/'); format = 'docx'
} 'docx-insert-section-inspect-split'
$secs = @($treeSplit.sections)
$s2 = $secs | Where-Object { $_.id -eq 'section:2' } | Select-Object -First 1
$link2 = $false
if ($s2 -and $s2.headerLink) { $link2 = [bool]$s2.headerLink.default }
Add-Check 'docx-Sc1' (($secs.Count -eq 2) -and $link2) "sections=$($secs.Count) link2=$link2"

$docSplit = Open-WordDoc $word $split
$capSplit = [string]$docSplit.ActiveWindow.Caption
$secCount = [int]$docSplit.Sections.Count
$secStart = [int]$docSplit.Sections.Item(2).PageSetup.SectionStart
Add-Check 'docx-Sc2' (
    ($capSplit -notmatch 'Repair') -and ($secCount -eq 2) -and ($secStart -eq $wdSectionNewPage)
) "caption=$capSplit sections=$secCount start=$secStart"
Close-WordDoc $docSplit

$unlinked = Get-OutPath 'docx-insert-section-unlinked.docx'
Close-WordDocByPath $word $unlinked
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $split.Replace('\', '/')
    output = $unlinked.Replace('\', '/')
    ops = @(@{
        op = 'setHeaderFooterLink'
        id = 'section:2'
        kind = 'header'
        type = 'default'
        linked = $false
    })
} 'docx-insert-section-unlink' | Out-Null

$treeHf = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $unlinked.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true }
} 'docx-insert-section-inspect-hf'
$h1 = Test-HeaderCovers $treeHf.headers 1
$h2 = Test-HeaderCovers $treeHf.headers 2
$h1id = $null
$h2id = $null
if ($h1 -and $h1.paragraphs) { $h1id = [string]@($h1.paragraphs)[0].id }
if ($h2 -and $h2.paragraphs) { $h2id = [string]@($h2.paragraphs)[0].id }

$edited = Get-OutPath 'docx-insert-section-headers.docx'
Close-WordDocByPath $word $edited
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $unlinked.Replace('\', '/')
    output = $edited.Replace('\', '/')
    ops = @(
        @{ op = 'setParaText'; id = $h1id; text = $S.headerNew }
        @{ op = 'setParaText'; id = $h2id; text = $S.headerSec2New }
    )
} 'docx-insert-section-edit-headers' | Out-Null

$docH = Open-WordDoc $word $edited
$capH = [string]$docH.ActiveWindow.Caption
$hdr1 = $docH.Sections.Item(1).Headers.Item($wdPrimary)
$hdr2 = $docH.Sections.Item(2).Headers.Item($wdPrimary)
$linked2 = [bool]$hdr2.LinkToPrevious
$t1 = ([string]$hdr1.Range.Text).TrimEnd([char]13, [char]7, [char]10)
$t2 = ([string]$hdr2.Range.Text).TrimEnd([char]13, [char]7, [char]10)
$sc3ok = ($capH -notmatch 'Repair') -and (-not $linked2) -and
    ($t1 -eq $S.headerNew) -and ($t2 -eq $S.headerSec2New) -and ($h1id -ne $h2id)
Add-Check 'docx-Sc3' $sc3ok "caption=$capH linked=$linked2 s1=$t1 s2=$t2 h1=$h1id h2=$h2id"
Release-Com $hdr2
Release-Com $hdr1
Close-WordDoc $docH

$merged = Get-OutPath 'docx-insert-section-merged.docx'
Close-WordDocByPath $word $merged
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $unlinked.Replace('\', '/')
    output = $merged.Replace('\', '/')
    ops = @(@{ op = 'deleteSection'; id = 'section:1' })
} 'docx-insert-section-delete' | Out-Null

$treeMerged = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $merged.Replace('\', '/'); format = 'docx'
} 'docx-insert-section-inspect-merged'
$mergedSecs = @($treeMerged.sections)
$docM = Open-WordDoc $word $merged
$capM = [string]$docM.ActiveWindow.Caption
$mCount = [int]$docM.Sections.Count
Add-Check 'docx-Sc4' (
    ($mergedSecs.Count -eq 1) -and ($capM -notmatch 'Repair') -and ($mCount -eq 1)
) "tree=$($mergedSecs.Count) caption=$capM com=$mCount"
Close-WordDoc $docM

$orphan = Get-OrphanHeaderParts $merged
Add-Check 'docx-Sc5' (
    ($orphan.Orphans.Count -eq 0) -and ($orphan.Used.Count -ge 1)
) "zip=$($orphan.InZip -join ',') used=$($orphan.Used -join ',') orphans=$($orphan.Orphans -join ',')"

$created = Get-OutPath 'docx-insert-section-create-two.docx'
Close-WordDocByPath $word $created
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $created.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = $S.chapter1; sectionType = 'nextPage' }
            @{ text = $S.hello }
        )
    }
} 'docx-insert-section-create-two' | Out-Null

$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $created.Replace('\', '/'); format = 'docx'
} 'docx-insert-section-inspect-create'
$cSecs = @($treeC.sections)
$docC = Open-WordDoc $word $created
$capC = [string]$docC.ActiveWindow.Caption
$cCount = [int]$docC.Sections.Count
Add-Check 'docx-Sc6' (
    ($cSecs.Count -eq 2) -and ($capC -notmatch 'Repair') -and ($cCount -eq 2)
) "tree=$($cSecs.Count) caption=$capC com=$cCount"
Close-WordDoc $docC
