# 不透明嵌入 PNG 页底：无旋转、无裁剪、拉伸铺满。金样是 PowerPoint 16.0 的 Slide.Export。
# 红区回绘制引擎修，不要加大 MaxDelta。不把 t93u 透明放大残差接受扩到页底。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.Drawing

function Strip-PngAncillary([string]$path) {
    $src = [IO.File]::ReadAllBytes($path)
    if ($src.Length -lt 8) { throw "png too small: $path" }
    $ms = New-Object IO.MemoryStream
    try {
        $ms.Write($src, 0, 8)
        $i = 8
        while ($i + 8 -le $src.Length) {
            $len = ([int]$src[$i] -shl 24) -bor ([int]$src[$i + 1] -shl 16) -bor ([int]$src[$i + 2] -shl 8) -bor ([int]$src[$i + 3])
            $typ = [Text.Encoding]::ASCII.GetString($src, $i + 4, 4)
            $end = $i + 12 + $len
            if ($end -gt $src.Length) { throw "truncated png chunk $typ in $path" }
            if ($typ -in @('IHDR', 'IDAT', 'PLTE', 'tRNS', 'IEND')) {
                $ms.Write($src, $i, ($end - $i))
            }
            $i = $end
            if ($typ -eq 'IEND') { break }
        }
        [IO.File]::WriteAllBytes($path, $ms.ToArray())
    }
    finally { $ms.Dispose() }
}

function Save-PatternPng([string]$path, [int]$w, [int]$h, [int]$seed) {
    $bmp = New-Object System.Drawing.Bitmap $w, $h, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $rect = New-Object System.Drawing.Rectangle 0, 0, $w, $h
    $data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::WriteOnly, $bmp.PixelFormat)
    try {
        $bytes = New-Object byte[] ($data.Stride * $h)
        for ($y = 0; $y -lt $h; $y++) {
            $row = $y * $data.Stride
            for ($x = 0; $x -lt $w; $x++) {
                $i = $y * $w + $x + $seed
                $o = $row + ($x * 4)
                $bytes[$o] = [byte](($i * 17 + 90) % 256)
                $bytes[($o + 1)] = [byte](($i * 73 + 20) % 256)
                $bytes[($o + 2)] = [byte](($i * 41) % 256)
                $bytes[($o + 3)] = 255
            }
        }
        [Runtime.InteropServices.Marshal]::Copy($bytes, 0, $data.Scan0, $bytes.Length)
    }
    finally {
        $bmp.UnlockBits($data)
    }
    if (Test-Path $path) { Remove-Item -Force $path }
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Strip-PngAncillary $path
}

function Save-SolidPng([string]$path, [int]$w, [int]$h, [int]$r, [int]$g, [int]$b) {
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    try {
        $gdi = [System.Drawing.Graphics]::FromImage($bmp)
        try {
            $gdi.Clear([System.Drawing.Color]::FromArgb(255, $r, $g, $b))
        }
        finally { $gdi.Dispose() }
        if (Test-Path $path) { Remove-Item -Force $path }
        $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    finally { $bmp.Dispose() }
    Strip-PngAncillary $path
}

function New-PageBgDeck {
    param(
        [string]$stem,
        [int]$pageW,
        [int]$pageH,
        [object[]]$specs
    )
    $src = Get-OutPath ($stem + '-src.pptx')
    if (Test-Path $src) { Remove-Item -Force $src }
    $slides = @()
    foreach ($s in $specs) {
        $slides += , @{ shapes = @(, @{ text = '标题' }) }
    }
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'create'
        path          = $src.Replace('\', '/')
        presentation  = @{ slides = $slides }
    } ($stem + '-create') | Out-Null

    $ops = @(
        @{
            op = 'setSlideSize'
            id = 'pptx:presentation'
            cx = ($pageW * 9525)
            cy = ($pageH * 9525)
        }
    )
    for ($i = 0; $i -lt $specs.Count; $i++) {
        $s = $specs[$i]
        $n = $i + 1
        $tree = Invoke-OoxpTree @{
            schemaVersion = 1
            action        = 'inspect'
            path          = $src.Replace('\', '/')
            format        = 'pptx'
            scope         = @{ slide = $n }
        } ('{0}-inspect-{1}' -f $stem, $n)
        $titleId = [string]$tree.slides[$i].shapes[0].id
        $ops += , @{ op = 'deleteShape'; id = $titleId }
        if ($s.host -eq 'slide') {
            $ops += , @{
                op    = 'setPageBackground'
                id    = ('slide:{0}' -f $n)
                color = 'FFFFFF'
                fill  = @{ kind = 'picture'; file = $s.file.Replace('\', '/') }
            }
        }
        if ($s.cover) {
            $ops += , @{
                op   = 'insertShape'
                id   = ('slide:{0}' -f $n)
                prst = 'rect'
                text = ''
                x    = (192 * 9525)
                y    = (96 * 9525)
                cx   = (256 * 9525)
                cy   = (128 * 9525)
                fill = '203040'
                line = @{ color = $null }
            }
        }
        if ($s.pic) {
            $ops += , @{
                op   = 'insertPicture'
                id   = ('slide:{0}' -f $n)
                file = $s.pic.Replace('\', '/')
                x    = (192 * 9525)
                y    = (96 * 9525)
                cx   = (64 * 9525)
                cy   = (64 * 9525)
            }
        }
    }
    foreach ($s in $specs) {
        if ($s.host -eq 'layout') {
            $ops += , @{
                op    = 'setPageBackground'
                id    = 'master:1/layout:1'
                color = 'FFFFFF'
                fill  = @{ kind = 'picture'; file = $s.file.Replace('\', '/') }
            }
        }
        if ($s.host -eq 'master') {
            $ops += , @{
                op    = 'setPageBackground'
                id    = 'master:1'
                color = 'FFFFFF'
                fill  = @{ kind = 'picture'; file = $s.file.Replace('\', '/') }
            }
        }
    }
    $deck = Get-OutPath ($stem + '.pptx')
    if (Test-Path $deck) { Remove-Item -Force $deck }
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'apply'
        path          = $src.Replace('\', '/')
        output        = $deck.Replace('\', '/')
        ops           = $ops
    } ($stem + '-apply') | Out-Null
    return $deck
}

function Compare-PageBgSlide([string]$deck, [object]$ppt, [int]$n, [string]$tag, [string]$title) {
    $lib = Get-OutPath ('pptx-preview-page-bg-{0}-lib.png' -f $tag)
    if (Test-Path $lib) { Remove-Item -Force $lib }
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'exportSlideImage'
        path          = $deck.Replace('\', '/')
        format        = 'pptx'
        id            = ('slide:{0}' -f $n)
        output        = $lib.Replace('\', '/')
    } ('pptx-preview-page-bg-{0}-export' -f $tag) | Out-Null

    $gold = Get-OutPath ('pptx-preview-page-bg-{0}-gold.png' -f $tag)
    $pres = $null
    $slide = $null
    try {
        $pres = Open-PptPres $ppt $deck
        $cap = Get-PptCaption $pres
        $slide = Get-PptSlide $pres $n
        if (Test-Path $gold) { Remove-Item -Force $gold }
        $slide.Export($gold, 'PNG')
        $ok = $cap -notmatch 'Repair'
        Add-Check ('pptx-preview-page-bg-{0}-open' -f $tag) $ok ('caption={0} {1}' -f $cap, $title)
    }
    finally {
        Release-Com $slide
        Close-PptPres $pres
    }

    $diff = Get-OutPath ('pptx-preview-page-bg-{0}-diff.png' -f $tag)
    $line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
    $code = $LASTEXITCODE
    $text = ($line | Out-String).Trim()
    Write-Log ('pptx-preview-page-bg-{0}-px exit={1} {2}' -f $tag, $code, $text)
    $pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
    Add-Check ('pptx-preview-page-bg-{0}-px' -f $tag) $pxOk ('exit={0} {1}' -f $code, $text)
}

$pngW1 = Get-OutPath 'pptx-preview-page-bg-1280x720.png'
Save-PatternPng $pngW1 1280 720 1
$pngW2 = Get-OutPath 'pptx-preview-page-bg-640x360.png'
Save-PatternPng $pngW2 640 360 2
$pngW4 = Get-OutPath 'pptx-preview-page-bg-320x180.png'
Save-PatternPng $pngW4 320 180 3
$pngWc = Get-OutPath 'pptx-preview-page-bg-cover-src.png'
Save-PatternPng $pngWc 1280 720 4
$pngWl = Get-OutPath 'pptx-preview-page-bg-layout-src.png'
Save-PatternPng $pngWl 1280 720 5
$pngWp = Get-OutPath 'pptx-preview-page-bg-pic-src.png'
Save-PatternPng $pngWp 1280 720 6
$pngOver = Get-OutPath 'pptx-preview-page-bg-over.png'
Save-SolidPng $pngOver 64 64 0x20 0x30 0x40
$pngC1 = Get-OutPath 'pptx-preview-page-bg-960x720.png'
Save-PatternPng $pngC1 960 720 7
$pngC2 = Get-OutPath 'pptx-preview-page-bg-480x360.png'
Save-PatternPng $pngC2 480 360 8
$pngM = Get-OutPath 'pptx-preview-page-bg-master-src.png'
Save-PatternPng $pngM 1280 720 9

$wideSpecs = @(
    @{ tag = 'w-1to1'; title = '1280x720 1:1 opaque PNG page bg'; host = 'slide'; file = $pngW1 }
    @{ tag = 'w-2x'; title = '640x360 to 1280x720 opaque PNG page bg'; host = 'slide'; file = $pngW2 }
    @{ tag = 'w-4x'; title = '320x180 to 1280x720 opaque PNG page bg'; host = 'slide'; file = $pngW4 }
    @{ tag = 'w-cover'; title = 'rect covers 1280x720 1:1 opaque PNG page bg'; host = 'slide'; file = $pngWc; cover = $true }
    @{ tag = 'w-layout'; title = 'layout inherited 1280x720 1:1 opaque PNG page bg'; host = 'layout'; file = $pngWl }
    @{ tag = 'w-pic'; title = 'picture covers 1280x720 1:1 opaque PNG page bg'; host = 'slide'; file = $pngWp; pic = $pngOver }
)

$customSpecs = @(
    @{ tag = 'c-1to1'; title = '960x720 1:1 opaque PNG page bg'; host = 'slide'; file = $pngC1 }
    @{ tag = 'c-2x'; title = '480x360 to 960x720 opaque PNG page bg'; host = 'slide'; file = $pngC2 }
)

$masterSpecs = @(
    @{ tag = 'm-1to1'; title = 'master inherited 1280x720 1:1 opaque PNG page bg'; host = 'master'; file = $pngM }
)

$wideDeck = New-PageBgDeck 'pptx-preview-page-bg-wide' 1280 720 $wideSpecs
$customDeck = New-PageBgDeck 'pptx-preview-page-bg-custom' 960 720 $customSpecs
$masterDeck = New-PageBgDeck 'pptx-preview-page-bg-master' 1280 720 $masterSpecs

$ppt = Ensure-MsPpt $wideDeck
$diffScript = [IO.Path]::Combine($PSScriptRoot, '..', '..', 'pixel-diff.ps1')

for ($i = 0; $i -lt $wideSpecs.Count; $i++) {
    Compare-PageBgSlide $wideDeck $ppt ($i + 1) $wideSpecs[$i].tag $wideSpecs[$i].title
}
for ($i = 0; $i -lt $customSpecs.Count; $i++) {
    Compare-PageBgSlide $customDeck $ppt ($i + 1) $customSpecs[$i].tag $customSpecs[$i].title
}
for ($i = 0; $i -lt $masterSpecs.Count; $i++) {
    Compare-PageBgSlide $masterDeck $ppt ($i + 1) $masterSpecs[$i].tag $masterSpecs[$i].title
}
