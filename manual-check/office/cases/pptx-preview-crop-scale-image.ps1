# 不透明整像素裁剪后再放大，对照 PowerPoint 16.0 Slide.Export，要求 differ=0 / maxDelta=0。
# 顶裁 dest_y=65 的不透明插值相位差（官方已切高相位、GDI+ 仍低相位）不在 t93u 透明度放宽范围，不得当作已接受；该页像素检查须照实失败并记录原文，不能靠只锁几何让套件 PASS 掩盖 maxDelta=2。
# 透明裁剪后再放大沿用同一插值后再 over 的机制，只验收几何，颜色残差照实记录，不把 t93u 自动扩成零差。
# 缩小或一边放大一边缩小不在本切片验收范围，不在本用例里冒充成功。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Save-RgbaPng([string]$path, [int]$w, [int]$h, [object[]]$px) {
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    try {
        for ($i = 0; $i -lt $px.Count; $i++) {
            $c = $px[$i]
            $bmp.SetPixel($i % $w, [Math]::Floor($i / $w), [System.Drawing.Color]::FromArgb([int]$c[3], [int]$c[0], [int]$c[1], [int]$c[2]))
        }
        if (Test-Path $path) { Remove-Item -Force $path }
        $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    finally { $bmp.Dispose() }
}

function Get-Hex([System.Drawing.Bitmap]$bmp, [int]$x, [int]$y) {
    $c = $bmp.GetPixel($x, $y)
    return ('{0:X2}{1:X2}{2:X2}' -f $c.R, $c.G, $c.B)
}

function Assert-BoxGeometry([System.Drawing.Bitmap]$libBmp, [System.Drawing.Bitmap]$goldBmp, [int]$x0, [int]$y0, [int]$bw, [int]$bh, [string]$bg) {
    $notes = @()
    $geoOk = $true
    $points = @()
    if ($x0 -gt 0) {
        $points += @{ x = ($x0 - 1); y = $y0 }
        $points += @{ x = ($x0 - 1); y = ($y0 + $bh - 1) }
    }
    if ($y0 -gt 0) {
        $points += @{ x = $x0; y = ($y0 - 1) }
        $points += @{ x = ($x0 + $bw - 1); y = ($y0 - 1) }
    }
    $points += @{ x = ($x0 + $bw); y = $y0 }
    $points += @{ x = ($x0 + $bw); y = ($y0 + $bh - 1) }
    $points += @{ x = $x0; y = ($y0 + $bh) }
    $points += @{ x = ($x0 + $bw - 1); y = ($y0 + $bh) }
    foreach ($pt in $points) {
        $libHex = Get-Hex $libBmp $pt.x $pt.y
        $goldHex = Get-Hex $goldBmp $pt.x $pt.y
        if ($libHex -ne $bg -or $goldHex -ne $bg) {
            $geoOk = $false
            $notes += ('outside({0},{1}) lib={2} gold={3}' -f $pt.x, $pt.y, $libHex, $goldHex)
        }
    }
    return @{ ok = $geoOk; notes = $notes }
}

$png4 = Get-OutPath 'pptx-preview-crop-scale-4x4.png'
$px4 = @()
foreach ($c in @(
    @(0xFF, 0x00, 0x00, 255), @(0x00, 0xFF, 0x00, 255), @(0x00, 0x00, 0xFF, 255), @(0xFF, 0xFF, 0x00, 255),
    @(0xFF, 0x00, 0xFF, 255), @(0x00, 0xFF, 0xFF, 255), @(0x11, 0x22, 0x33, 255), @(0x44, 0x55, 0x66, 255),
    @(0x77, 0x88, 0x99, 255), @(0xAA, 0xBB, 0xCC, 255), @(0xDD, 0xEE, 0xFF, 255), @(0x01, 0x02, 0x03, 255),
    @(0x04, 0x05, 0x06, 255), @(0x07, 0x08, 0x09, 255), @(0x0A, 0x0B, 0x0C, 255), @(0x0D, 0x0E, 0x0F, 255)
)) { $px4 += , $c }
Save-RgbaPng $png4 4 4 $px4

$png8 = Get-OutPath 'pptx-preview-crop-scale-8x8.png'
$px8 = @()
for ($i = 0; $i -lt 64; $i++) {
    $px8 += , @((($i * 41) % 256), (($i * 73 + 20) % 256), (($i * 17 + 90) % 256), 255)
}
Save-RgbaPng $png8 8 8 $px8

$pngA = Get-OutPath 'pptx-preview-crop-scale-alpha.png'
$pxA = @()
for ($i = 0; $i -lt 16; $i++) { $pxA += , @(0x20, 0x20, 0x20, 255) }
$pxA[0] = @(255, 0, 0, 255)
$pxA[1] = @(0, 255, 0, 128)
$pxA[4] = @(0, 0, 255, 0)
$pxA[5] = @(255, 255, 0, 64)
Save-RgbaPng $pngA 4 4 $pxA

$pngNoise = Get-OutPath 'pptx-preview-crop-scale-noise.png'
$noisePx = @()
for ($y = 0; $y -lt 64; $y++) {
    for ($x = 0; $x -lt 64; $x++) {
        $noisePx += , @((($x * 7 + $y * 3) % 256), (($x * 11 + $y * 5) % 256), (($x * 13 + $y * 17) % 256), 255)
    }
}
Save-RgbaPng $pngNoise 64 64 $noisePx

$specs = @(
    @{ tag = 'c4-center-2to4'; title = 'center 2x2 to 4x4'; file = $png4; x = 1828800; y = 914400; cx = 38100; cy = 38100; l = 25000; t = 25000; r = 25000; b = 25000; dw = 4; dh = 4; kind = 'pic'; expect = 'zero' }
    @{ tag = 'c4-left-3x4to6x8'; title = 'left 3x4 to 6x8'; file = $png4; x = 1828800; y = 914400; cx = 57150; cy = 76200; l = 25000; t = 0; r = 0; b = 0; dw = 6; dh = 8; kind = 'pic'; expect = 'zero' }
    @{ tag = 'c4-right-3x4to6x8'; title = 'right 3x4 to 6x8'; file = $png4; x = 1828800; y = 914400; cx = 57150; cy = 76200; l = 0; t = 0; r = 25000; b = 0; dw = 6; dh = 8; kind = 'pic'; expect = 'zero' }
    @{ tag = 'c4-top-4x3to8x6'; title = 'top 4x3 to 8x6 at y=96'; file = $png4; x = 1828800; y = 914400; cx = 76200; cy = 57150; l = 0; t = 25000; r = 0; b = 0; dw = 8; dh = 6; kind = 'pic'; expect = 'zero' }
    @{ tag = 'c4-bottom-4x3to8x6'; title = 'bottom 4x3 to 8x6'; file = $png4; x = 1828800; y = 914400; cx = 76200; cy = 57150; l = 0; t = 0; r = 0; b = 25000; dw = 8; dh = 6; kind = 'pic'; expect = 'zero' }
    @{ tag = 'c4-center-2to5'; title = 'center 2x2 to 5x5'; file = $png4; x = 1828800; y = 914400; cx = 47625; cy = 47625; l = 25000; t = 25000; r = 25000; b = 25000; dw = 5; dh = 5; kind = 'pic'; expect = 'zero' }
    @{ tag = 'c4-center-2to6x4'; title = 'center 2x2 to 6x4'; file = $png4; x = 1828800; y = 914400; cx = 57150; cy = 38100; l = 25000; t = 25000; r = 25000; b = 25000; dw = 6; dh = 4; kind = 'pic'; expect = 'zero' }
    @{ tag = 'c8-left1-7x8to14x16'; title = '8x8 left 7x8 to 14x16'; file = $png8; x = 1828800; y = 914400; cx = 133350; cy = 152400; l = 12500; t = 0; r = 0; b = 0; dw = 14; dh = 16; kind = 'pic'; expect = 'zero' }
    @{ tag = 'c4-top-y0'; title = 'top 4x3 to 8x6 at y=0'; file = $png4; x = 1828800; y = 0; cx = 76200; cy = 57150; l = 0; t = 25000; r = 0; b = 0; dw = 8; dh = 6; kind = 'pic'; expect = 'zero' }
    @{ tag = 'c4-top-y64'; title = 'top 4x3 to 8x6 at y=64'; file = $png4; x = 1828800; y = 609600; cx = 76200; cy = 57150; l = 0; t = 25000; r = 0; b = 0; dw = 8; dh = 6; kind = 'pic'; expect = 'zero' }
    @{ tag = 'c4-top-y66'; title = 'top 4x3 to 8x6 at y=66'; file = $png4; x = 1828800; y = 628650; cx = 76200; cy = 57150; l = 0; t = 25000; r = 0; b = 0; dw = 8; dh = 6; kind = 'pic'; expect = 'zero' }
    @{ tag = 'c4-top-y65'; title = 'top 4x3 to 8x6 at y=65 unaccepted opaque phase'; file = $png4; x = 1828800; y = 619125; cx = 76200; cy = 57150; l = 0; t = 25000; r = 0; b = 0; dw = 8; dh = 6; kind = 'pic'; expect = 'phase' }
    @{ tag = 'c4-center-2to4-shape'; title = 'center 2x2 to 4x4 shape blipFill'; file = $png4; x = 1828800; y = 914400; cx = 38100; cy = 38100; l = 25000; t = 25000; r = 25000; b = 25000; dw = 4; dh = 4; kind = 'shape'; expect = 'zero' }
    @{ tag = 'c4-tl-2to4a'; title = 'transparent top-left 2x2 to 4x4'; file = $pngA; x = 1828800; y = 914400; cx = 38100; cy = 38100; l = 0; t = 0; r = 50000; b = 50000; dw = 4; dh = 4; kind = 'pic'; expect = 'geometry' }
)

$src = Get-OutPath 'pptx-preview-crop-scale-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
$slides = @()
foreach ($spec in $specs) {
    $slides += @{ shapes = @(@{ text = '标题' }) }
}
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'create'
    path          = $src.Replace('\', '/')
    presentation  = @{ slides = $slides }
} 'pptx-preview-crop-scale-create' | Out-Null

$ops = @()
for ($i = 0; $i -lt $specs.Count; $i++) {
    $n = $i + 1
    $spec = $specs[$i]
    $tree = Invoke-OoxpTree @{
        schemaVersion = 1
        action        = 'inspect'
        path          = $src.Replace('\', '/')
        format        = 'pptx'
        scope         = @{ slide = $n }
    } ('pptx-preview-crop-scale-inspect-{0}' -f $n)
    $titleId = [string]$tree.slides[$i].shapes[0].id
    $ops += @{ op = 'setPageBackground'; id = ('slide:{0}' -f $n); color = 'AABBCC' }
    $ops += @{ op = 'deleteShape'; id = $titleId }
    $ops += @{
        op   = 'insertPicture'
        id   = ('slide:{0}' -f $n)
        file = $pngNoise.Replace('\', '/')
        x    = 0
        y    = 0
        cx   = 609600
        cy   = 609600
    }
    if ($spec.kind -eq 'shape') {
        $ops += @{
            op   = 'insertShape'
            id   = ('slide:{0}' -f $n)
            fill = @{ type = 'blip'; file = $spec.file.Replace('\', '/') }
            x    = $spec.x
            y    = $spec.y
            cx   = $spec.cx
            cy   = $spec.cy
        }
    }
    else {
        $ops += @{
            op   = 'insertPicture'
            id   = ('slide:{0}' -f $n)
            file = $spec.file.Replace('\', '/')
            x    = $spec.x
            y    = $spec.y
            cx   = $spec.cx
            cy   = $spec.cy
        }
    }
}

$deck0 = Get-OutPath 'pptx-preview-crop-scale-nocrop.pptx'
if (Test-Path $deck0) { Remove-Item -Force $deck0 }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'apply'
    path          = $src.Replace('\', '/')
    output        = $deck0.Replace('\', '/')
    ops           = $ops
} 'pptx-preview-crop-scale-apply' | Out-Null

$deck = Get-OutPath 'pptx-preview-crop-scale.pptx'
if (Test-Path $deck) { Remove-Item -Force $deck }
Copy-Item $deck0 $deck
$zip = [IO.Compression.ZipFile]::Open($deck, [IO.Compression.ZipArchiveMode]::Update)
try {
    for ($i = 0; $i -lt $specs.Count; $i++) {
        $spec = $specs[$i]
        $name = 'ppt/slides/slide{0}.xml' -f ($i + 1)
        $entry = $zip.GetEntry($name)
        $reader = New-Object IO.StreamReader($entry.Open())
        $text = $reader.ReadToEnd()
        $reader.Dispose()
        $rect = '<a:srcRect l="{0}" t="{1}" r="{2}" b="{3}"/>' -f $spec.l, $spec.t, $spec.r, $spec.b
        if ($text -notmatch '<a:blip r:embed="rId3"/>') { throw ("missing rId3 in {0}" -f $name) }
        $text2 = $text.Replace('<a:blip r:embed="rId3"/>', '<a:blip r:embed="rId3"/>' + $rect)
        if ($text2 -eq $text) { throw ("srcRect patch failed {0}" -f $name) }
        $entry.Delete()
        $created = $zip.CreateEntry($name)
        $writer = New-Object IO.StreamWriter($created.Open())
        $writer.Write($text2)
        $writer.Dispose()
    }
}
finally { $zip.Dispose() }

$ppt = Ensure-MsPpt $deck
$diffScript = [IO.Path]::Combine($PSScriptRoot, '..', '..', 'pixel-diff.ps1')

for ($i = 0; $i -lt $specs.Count; $i++) {
    $spec = $specs[$i]
    $n = $i + 1
    $lib = Get-OutPath ('pptx-preview-crop-scale-{0}-lib.png' -f $spec.tag)
    if (Test-Path $lib) { Remove-Item -Force $lib }
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'exportSlideImage'
        path          = $deck.Replace('\', '/')
        format        = 'pptx'
        id            = ('slide:{0}' -f $n)
        output        = $lib.Replace('\', '/')
    } ('pptx-preview-crop-scale-{0}-export' -f $spec.tag) | Out-Null

    $gold = Get-OutPath ('pptx-preview-crop-scale-{0}-gold.png' -f $spec.tag)
    $pres = $null
    $slide = $null
    try {
        $pres = Open-PptPres $ppt $deck
        $cap = Get-PptCaption $pres
        $slide = Get-PptSlide $pres $n
        if (Test-Path $gold) { Remove-Item -Force $gold }
        $slide.Export($gold, 'PNG')
        $ok = $cap -notmatch 'Repair'
        Add-Check ('pptx-preview-crop-scale-{0}-open' -f $spec.tag) $ok ('caption={0} {1}' -f $cap, $spec.title)
    }
    finally {
        Release-Com $slide
        Close-PptPres $pres
    }

    $diff = Get-OutPath ('pptx-preview-crop-scale-{0}-diff.png' -f $spec.tag)
    $line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
    $code = $LASTEXITCODE
    $text = ($line | Out-String).Trim()
    Write-Log ('pptx-preview-crop-scale-{0}-px exit={1} {2}' -f $spec.tag, $code, $text)

    $x0 = [int]($spec.x / 9525)
    $y0 = [int]($spec.y / 9525)

    if ($spec.expect -eq 'zero') {
        $pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
        Add-Check ('pptx-preview-crop-scale-{0}-px' -f $spec.tag) $pxOk ('exit={0} {1}' -f $code, $text)
    }
    else {
        Write-Log ('pptx-preview-crop-scale-{0}-residual exit={1} {2}' -f $spec.tag, $code, $text)
        $libBmp = [System.Drawing.Bitmap]::FromFile($lib)
        $goldBmp = [System.Drawing.Bitmap]::FromFile($gold)
        try {
            $geom = Assert-BoxGeometry $libBmp $goldBmp $x0 $y0 ([int]$spec.dw) ([int]$spec.dh) 'AABBCC'
            $geoOk = [bool]$geom.ok
            $notes = @($geom.notes)
            if ($spec.expect -eq 'geometry') {
                $libTl = Get-Hex $libBmp $x0 $y0
                $goldTl = Get-Hex $goldBmp $x0 $y0
                if ($libTl -ne 'FF0000' -or $goldTl -ne 'FF0000') {
                    $geoOk = $false
                    $notes += ('opaque-corner lib={0} gold={1}' -f $libTl, $goldTl)
                }
            }
            $msg = if ($notes.Count -gt 0) { ($notes -join '; ') } else { 'geometry anchors ok' }
            Add-Check ('pptx-preview-crop-scale-{0}-geom' -f $spec.tag) $geoOk ('{0}; official {1}' -f $msg, $text)
            if ($spec.expect -eq 'phase') {
                $pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
                Add-Check ('pptx-preview-crop-scale-{0}-px' -f $spec.tag) $pxOk ('opaque interpolator phase residual is not in t93u and is not accepted; official {0}' -f $text)
            }
        }
        finally {
            $libBmp.Dispose()
            $goldBmp.Dispose()
        }
    }
}
