# ???????? PNG ?? PowerPoint 16.0 Slide.Export?????? differ=0 / maxDelta=0?
# ?? PNG ? t93u??????/alpha ?????????????????????
# ??????????????????????? pixel-diff ???????????? MaxDelta=0?
# ????????????????????????? blipFill??????????????????

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.Drawing

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

$png2 = Get-OutPath 'pptx-preview-scale-2x2.png'
Save-RgbaPng $png2 2 2 @(
    @(255, 0, 0, 255), @(0, 255, 0, 255),
    @(0, 0, 255, 255), @(255, 255, 0, 255)
)
$png32 = Get-OutPath 'pptx-preview-scale-3x2.png'
Save-RgbaPng $png32 3 2 @(
    @(255, 0, 0, 255), @(0, 255, 0, 255), @(0, 0, 255, 255),
    @(255, 255, 0, 255), @(255, 0, 255, 255), @(0, 255, 255, 255)
)
$png3 = Get-OutPath 'pptx-preview-scale-3x3.png'
Save-RgbaPng $png3 3 3 @(
    @(255, 0, 0, 255), @(0, 255, 0, 255), @(0, 0, 255, 255),
    @(255, 255, 0, 255), @(255, 0, 255, 255), @(0, 255, 255, 255),
    @(128, 0, 0, 255), @(0, 128, 0, 255), @(0, 0, 128, 255)
)
$png4 = Get-OutPath 'pptx-preview-scale-4x4.png'
$colors4 = @()
for ($i = 0; $i -lt 16; $i++) {
    $colors4 += , @((($i * 37) % 256), (($i * 73 + 40) % 256), (($i * 19 + 90) % 256), 255)
}
Save-RgbaPng $png4 4 4 $colors4
$png1 = Get-OutPath 'pptx-preview-scale-1x1.png'
Save-RgbaPng $png1 1 1 @(, @(255, 0, 0, 255))
$pngA = Get-OutPath 'pptx-preview-scale-2x2a.png'
Save-RgbaPng $pngA 2 2 @(
    @(255, 0, 0, 255), @(0, 255, 0, 128),
    @(0, 0, 255, 0), @(255, 255, 0, 64)
)
$pngNoise = Get-OutPath 'pptx-preview-scale-noise.png'
$noisePx = @()
for ($y = 0; $y -lt 64; $y++) {
    for ($x = 0; $x -lt 64; $x++) {
        $noisePx += , @((($x * 7 + $y * 3) % 256), (($x * 11 + $y * 5) % 256), (($x * 13 + $y * 17) % 256), 255)
    }
}
Save-RgbaPng $pngNoise 64 64 $noisePx

$specs = @(
    @{ tag = 's2to4'; title = '2x2 to 4x4 picture'; file = $png2; x = 1828800; y = 914400; cx = 38100; cy = 38100; kind = 'pic'; expect = 'zero' }
    @{ tag = 's2to5'; title = '2x2 to 5x5 non-integer'; file = $png2; x = 1828800; y = 914400; cx = 47625; cy = 47625; kind = 'pic'; expect = 'zero' }
    @{ tag = 's2to3'; title = '2x2 to 3x3 non-integer'; file = $png2; x = 1828800; y = 914400; cx = 28575; cy = 28575; kind = 'pic'; expect = 'zero' }
    @{ tag = 's2to6'; title = '2x2 to 6x6'; file = $png2; x = 1828800; y = 914400; cx = 57150; cy = 57150; kind = 'pic'; expect = 'zero' }
    @{ tag = 's2to4x2'; title = '2x2 to 4x2 uneven'; file = $png2; x = 1828800; y = 914400; cx = 38100; cy = 19050; kind = 'pic'; expect = 'zero' }
    @{ tag = 's2to4x6'; title = '2x2 to 4x6 uneven'; file = $png2; x = 1828800; y = 914400; cx = 38100; cy = 57150; kind = 'pic'; expect = 'zero' }
    @{ tag = 's3x2to6x4'; title = '3x2 to 6x4'; file = $png32; x = 1828800; y = 914400; cx = 57150; cy = 38100; kind = 'pic'; expect = 'zero' }
    @{ tag = 's3to6'; title = '3x3 to 6x6'; file = $png3; x = 1828800; y = 914400; cx = 57150; cy = 57150; kind = 'pic'; expect = 'zero' }
    @{ tag = 's3to9'; title = '3x3 to 9x9'; file = $png3; x = 1828800; y = 914400; cx = 85725; cy = 85725; kind = 'pic'; expect = 'zero' }
    @{ tag = 's4to8'; title = '4x4 to 8x8'; file = $png4; x = 1828800; y = 914400; cx = 76200; cy = 76200; kind = 'pic'; expect = 'zero' }
    @{ tag = 's1to4'; title = '1x1 solid to 4x4'; file = $png1; x = 1828800; y = 914400; cx = 38100; cy = 38100; kind = 'pic'; expect = 'zero' }
    @{ tag = 's2to2'; title = '2x2 one-to-one'; file = $png2; x = 1828800; y = 914400; cx = 19050; cy = 19050; kind = 'pic'; expect = 'zero' }
    @{ tag = 's2to4-shape'; title = '2x2 to 4x4 shape blipFill'; file = $png2; x = 1828800; y = 914400; cx = 38100; cy = 38100; kind = 'shape'; expect = 'zero' }
    @{ tag = 's2to4-clip'; title = '2x2 to 4x4 clipped at left'; file = $png2; x = -9525; y = 1828800; cx = 38100; cy = 38100; kind = 'shape'; expect = 'zero' }
    @{ tag = 's2to4a'; title = 'transparent 2x2 to 4x4'; file = $pngA; x = 1828800; y = 914400; cx = 38100; cy = 38100; kind = 'pic'; expect = 'geometry' }
)

$src = Get-OutPath 'pptx-preview-scale-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
$slides = @()
foreach ($spec in $specs) {
    $slides += @{ shapes = @(@{ text = '??' }) }
}
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'create'
    path          = $src.Replace('\', '/')
    presentation  = @{ slides = $slides }
} 'pptx-preview-scale-create' | Out-Null

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
    } ('pptx-preview-scale-inspect-{0}' -f $n)
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

$deck = Get-OutPath 'pptx-preview-scale.pptx'
if (Test-Path $deck) { Remove-Item -Force $deck }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'apply'
    path          = $src.Replace('\', '/')
    output        = $deck.Replace('\', '/')
    ops           = $ops
} 'pptx-preview-scale-apply' | Out-Null

$ppt = Ensure-MsPpt $deck
$diffScript = [IO.Path]::Combine($PSScriptRoot, '..', '..', 'pixel-diff.ps1')

for ($i = 0; $i -lt $specs.Count; $i++) {
    $spec = $specs[$i]
    $n = $i + 1
    $lib = Get-OutPath ('pptx-preview-scale-{0}-lib.png' -f $spec.tag)
    if (Test-Path $lib) { Remove-Item -Force $lib }
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'exportSlideImage'
        path          = $deck.Replace('\', '/')
        format        = 'pptx'
        id            = ('slide:{0}' -f $n)
        output        = $lib.Replace('\', '/')
    } ('pptx-preview-scale-{0}-export' -f $spec.tag) | Out-Null

    $gold = Get-OutPath ('pptx-preview-scale-{0}-gold.png' -f $spec.tag)
    $pres = $null
    $slide = $null
    try {
        $pres = Open-PptPres $ppt $deck
        $cap = Get-PptCaption $pres
        $slide = Get-PptSlide $pres $n
        if (Test-Path $gold) { Remove-Item -Force $gold }
        $slide.Export($gold, 'PNG')
        $ok = $cap -notmatch 'Repair'
        Add-Check ('pptx-preview-scale-{0}-open' -f $spec.tag) $ok ('caption={0} {1}' -f $cap, $spec.title)
    }
    finally {
        Release-Com $slide
        Close-PptPres $pres
    }

    $diff = Get-OutPath ('pptx-preview-scale-{0}-diff.png' -f $spec.tag)
    $line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
    $code = $LASTEXITCODE
    $text = ($line | Out-String).Trim()
    Write-Log ('pptx-preview-scale-{0}-px exit={1} {2}' -f $spec.tag, $code, $text)

    if ($spec.expect -eq 'zero') {
        $pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
        Add-Check ('pptx-preview-scale-{0}-px' -f $spec.tag) $pxOk ('exit={0} {1}' -f $code, $text)
    }
    else {
        # t93u???????????? MaxDelta=0 ?????????????
        Write-Log ('pptx-preview-scale-{0}-residual (t93u, not a pass gate) exit={1} {2}' -f $spec.tag, $code, $text)
        $libBmp = [System.Drawing.Bitmap]::FromFile($lib)
        $goldBmp = [System.Drawing.Bitmap]::FromFile($gold)
        try {
            $bg = 'AABBCC'
            $notes = @()
            $geoOk = $true
            foreach ($pt in @(
                @{ x = 191; y = 96 }, @{ x = 191; y = 99 },
                @{ x = 192; y = 95 }, @{ x = 195; y = 95 },
                @{ x = 196; y = 96 }, @{ x = 196; y = 99 },
                @{ x = 192; y = 100 }, @{ x = 195; y = 100 }
            )) {
                $libHex = Get-Hex $libBmp $pt.x $pt.y
                $goldHex = Get-Hex $goldBmp $pt.x $pt.y
                if ($libHex -ne $bg -or $goldHex -ne $bg) {
                    $geoOk = $false
                    $notes += ('outside({0},{1}) lib={2} gold={3}' -f $pt.x, $pt.y, $libHex, $goldHex)
                }
            }
            $libTl = Get-Hex $libBmp 192 96
            $goldTl = Get-Hex $goldBmp 192 96
            if ($libTl -ne 'FF0000' -or $goldTl -ne 'FF0000') {
                $geoOk = $false
                $notes += ('opaque-corner lib={0} gold={1}' -f $libTl, $goldTl)
            }
            $libBl = Get-Hex $libBmp 192 99
            $goldBl = Get-Hex $goldBmp 192 99
            $libBl2 = Get-Hex $libBmp 193 99
            $goldBl2 = Get-Hex $goldBmp 193 99
            if ($libBl -ne $bg -or $goldBl -ne $bg -or $libBl2 -ne $bg -or $goldBl2 -ne $bg) {
                $geoOk = $false
                $notes += ('transparent-corner lib={0}/{1} gold={2}/{3}' -f $libBl, $libBl2, $goldBl, $goldBl2)
            }
            $msg = if ($notes.Count -gt 0) { ($notes -join '; ') } else { 'geometry anchors ok' }
            Add-Check ('pptx-preview-scale-{0}-geom' -f $spec.tag) $geoOk ('{0}; official {1}' -f $msg, $text)
        }
        finally {
            $libBmp.Dispose()
            $goldBmp.Dispose()
        }
    }
}
