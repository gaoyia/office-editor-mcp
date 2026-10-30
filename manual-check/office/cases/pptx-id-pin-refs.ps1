# 删掉第一页之后，节、Zoom 和自定义放映仍按开始时的页号。
# 见 .plan/2026年-10月-30日-❌-pptx固定页号手测补跑.md

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function New-SectionDeck {
    $path = Get-OutPath 'id-pin-refs-src.pptx'
    if (Test-Path $path) { Remove-Item -Force $path }
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'create'
        path = $path.Replace('\', '/')
        presentation = @{
            slides = @(
                @{ shapes = @(@{ text = 'S1' }) },
                @{ shapes = @(@{ text = 'S2' }) },
                @{ shapes = @(@{ text = 'S3' }) },
                @{ shapes = @(@{ text = 'S4' }) }
            )
            sections = @(
                @{ name = 'A'; start = 1 },
                @{ name = 'B'; start = 4 }
            )
        }
    } 'id-pin-refs-create' | Out-Null
    return $path
}

function Apply-Deck([string]$src, [string]$tag, $ops) {
    $out = Get-OutPath "id-pin-refs-$tag.pptx"
    if (Test-Path $out) { Remove-Item -Force $out }
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $src.Replace('\', '/')
        output = $out.Replace('\', '/')
        ops = $ops
    } "id-pin-refs-$tag" | Out-Null
    return $out
}

function Get-SlideTexts($pres) {
    $slides = $pres.Slides
    try {
        $n = [int]$slides.Count
        $got = @()
        for ($i = 1; $i -le $n; $i++) {
            $slide = $slides.Item($i)
            $shapes = $slide.Shapes
            $text = ''
            try {
                $sc = [int]$shapes.Count
                for ($j = 1; $j -le $sc; $j++) {
                    $sh = $shapes.Item($j)
                    try {
                        $piece = ''
                        try { $piece = [string]$sh.TextFrame.TextRange.Text } catch { $piece = '' }
                        if ($piece.Trim().Length -gt 0) { $text = $piece.Trim() }
                    } finally { Release-Com $sh }
                }
            } finally {
                Release-Com $shapes
                Release-Com $slide
            }
            $got += $text
        }
        return $got
    } finally { Release-Com $slides }
}

function Get-SectionRows($pres) {
    $sp = $pres.SectionProperties
    try {
        $n = [int]$sp.Count
        $rows = @()
        for ($i = 1; $i -le $n; $i++) {
            $rows += ('{0}:{1}+{2}' -f [string]$sp.Name($i), [int]$sp.FirstSlide($i), [int]$sp.SlidesCount($i))
        }
        return $rows
    } finally { Release-Com $sp }
}

function Get-ZoomOnText($pres, [string]$hostText) {
    $slides = $pres.Slides
    try {
        $n = [int]$slides.Count
        for ($i = 1; $i -le $n; $i++) {
            $slide = $slides.Item($i)
            $shapes = $slide.Shapes
            $text = ''
            $hasZoom = $false
            try {
                $sc = [int]$shapes.Count
                for ($j = 1; $j -le $sc; $j++) {
                    $sh = $shapes.Item($j)
                    try {
                        if (([string]$sh.Name) -like 'Zoom*') { $hasZoom = $true }
                        $piece = ''
                        try { $piece = [string]$sh.TextFrame.TextRange.Text } catch { $piece = '' }
                        if ($piece.Trim().Length -gt 0) { $text = $piece.Trim() }
                    } finally { Release-Com $sh }
                }
            } finally { Release-Com $shapes }
            Release-Com $slide
            if ($hasZoom -and ($text -eq $hostText)) { return $true }
        }
        return $false
    } finally { Release-Com $slides }
}

function Get-ShowSlideText($pres) {
    $sss = $pres.SlideShowSettings
    $nss = $sss.NamedSlideShows
    try {
        if ([int]$nss.Count -lt 1) { return '' }
        $item = $nss.Item(1)
        try {
            $ids = @($item.SlideIDs | Where-Object { [int]$_ -gt 0 })
            if ($ids.Count -lt 1) { return '' }
            $sid = [int]$ids[0]
            $slides = $pres.Slides
            try {
                $n = [int]$slides.Count
                for ($i = 1; $i -le $n; $i++) {
                    $slide = $slides.Item($i)
                    try {
                        if ([int]$slide.SlideID -ne $sid) { continue }
                        $shapes = $slide.Shapes
                        try {
                            $sc = [int]$shapes.Count
                            for ($j = 1; $j -le $sc; $j++) {
                                $sh = $shapes.Item($j)
                                try {
                                    $piece = ''
                                    try { $piece = [string]$sh.TextFrame.TextRange.Text } catch { $piece = '' }
                                    if ($piece.Trim().Length -gt 0) { return $piece.Trim() }
                                } finally { Release-Com $sh }
                            }
                        } finally { Release-Com $shapes }
                    } finally { Release-Com $slide }
                }
            } finally { Release-Com $slides }
            return ''
        } finally { Release-Com $item }
    } finally {
        Release-Com $nss
        Release-Com $sss
    }
}

function Open-Checked([string]$path) {
    $ppt = Ensure-MsPpt $path
    Close-PptPresByPath $ppt $path
    $pres = Open-PptPres $ppt $path
    $cap = Get-PptCaption $pres
    return @{ Pres = $pres; Caption = $cap; Ok = ($cap -notmatch 'Repair') }
}

$src = New-SectionDeck

$insertOut = Apply-Deck $src 'insert' @(
    @{ op = 'deleteSlide'; id = 'slide:1' },
    @{ op = 'insertSlide'; id = 'pptx:presentation'; after = 'slide:3' }
)
$opened = Open-Checked $insertOut
$texts = Get-SlideTexts $opened.Pres
$secs = Get-SectionRows $opened.Pres
$insertOk = $opened.Ok -and (($texts -join '|') -eq 'S2|S3||S4') -and (($secs -join ',') -eq 'A:1+3,B:4+1')
Add-Check 'id-pin-insert-section' $insertOk ("cap=$($opened.Caption) texts=" + ($texts -join '|') + ' secs=' + ($secs -join ','))
Close-PptPres $opened.Pres

$setOut = Apply-Deck $src 'set-section' @(
    @{ op = 'deleteSlide'; id = 'slide:1' },
    @{ op = 'setSlideSection'; id = 'slide:3'; section = 'section:2' }
)
$opened = Open-Checked $setOut
$texts = Get-SlideTexts $opened.Pres
$secs = Get-SectionRows $opened.Pres
$setOk = $opened.Ok -and (($texts[0]) -eq 'S2') -and (($secs -join ',') -eq 'A:1+1,B:2+2') -and ($texts -contains 'S3') -and ($texts -contains 'S4')
Add-Check 'id-pin-set-section' $setOk ("cap=$($opened.Caption) texts=" + ($texts -join '|') + ' secs=' + ($secs -join ','))
Close-PptPres $opened.Pres

$startOut = Apply-Deck $src 'new-section' @(
    @{ op = 'deleteSlide'; id = 'slide:1' },
    @{ op = 'insertSlideSection'; id = 'pptx:presentation'; name = 'C'; start = 'slide:3' }
)
$opened = Open-Checked $startOut
$texts = Get-SlideTexts $opened.Pres
$secs = Get-SectionRows $opened.Pres
$startOk = $opened.Ok -and (($texts -join '|') -eq 'S2|S3|S4') -and (($secs -join ',') -eq 'A:1+1,C:2+1,B:3+1')
Add-Check 'id-pin-new-section' $startOk ("cap=$($opened.Caption) texts=" + ($texts -join '|') + ' secs=' + ($secs -join ','))
Close-PptPres $opened.Pres

$zoomOut = Apply-Deck $src 'zoom' @(
    @{ op = 'deleteSlide'; id = 'slide:1' },
    @{ op = 'insertZoom'; id = 'slide:2'; type = 'slide'; target = 'slide:3' }
)
$opened = Open-Checked $zoomOut
$zoomOnS2 = Get-ZoomOnText $opened.Pres 'S2'
$opened.Pres.Save()
Close-PptPres $opened.Pres
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::OpenRead($zoomOut)
try {
    $entry = $zip.GetEntry('ppt/slides/_rels/slide2.xml.rels')
    $stream = $entry.Open()
    $reader = New-Object System.IO.StreamReader($stream)
    $rels = $reader.ReadToEnd()
    $reader.Dispose()
    $stream.Dispose()
} finally { $zip.Dispose() }
$zoomOk = $opened.Ok -and $zoomOnS2 -and ($rels.Contains('Target="slide3.xml"'))
Add-Check 'id-pin-zoom' $zoomOk ("cap=$($opened.Caption) onS2=$zoomOnS2 rel=$($rels.Contains('slide3.xml'))")

$showOut = Apply-Deck $src 'show' @(
    @{ op = 'deleteSlide'; id = 'slide:1' },
    @{ op = 'insertCustomShow'; id = 'pptx:presentation'; name = 'T'; slides = @('slide:3') }
)
$opened = Open-Checked $showOut
$showText = Get-ShowSlideText $opened.Pres
$showOk = $opened.Ok -and ($showText -eq 'S3')
Add-Check 'id-pin-custom-show' $showOk ("cap=$($opened.Caption) slide=$showText")
Close-PptPres $opened.Pres
