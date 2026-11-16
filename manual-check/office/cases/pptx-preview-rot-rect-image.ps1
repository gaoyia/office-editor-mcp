# 不透明无描边纯色矩形的非直角旋转，对照 PowerPoint 16.0 Slide.Export，要求 differ=0 / maxDelta=0。
# 覆盖多个角度、方向、尺寸和原点。Windows 上走公开 GDI+ 平移到中心、RotateTransform、FillRectangle，以及已有的抗锯齿和半像素偏移。
# 不把 t93u 透明度放宽扩到旋转形状。描边、翻转、组合、文字、半透明和其它几何仍失败。直角旋转和翻转仍走原路径。
# 像素检查不得省略；不得用几何锚点过就标通过。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

$pngNoise = Get-OutPath 'pptx-preview-rot-rect-noise.png'
$bmp = New-Object System.Drawing.Bitmap 64, 64
try {
    for ($y = 0; $y -lt 64; $y++) {
        for ($x = 0; $x -lt 64; $x++) {
            $bmp.SetPixel($x, $y, [System.Drawing.Color]::FromArgb((($x * 7 + $y * 3) % 256), (($x * 11 + $y * 5) % 256), (($x * 13 + $y * 17) % 256)))
        }
    }
    if (Test-Path $pngNoise) { Remove-Item -Force $pngNoise }
    $bmp.Save($pngNoise, [System.Drawing.Imaging.ImageFormat]::Png)
}
finally { $bmp.Dispose() }

$specs = @(
    @{ tag = 'sq45-y96'; title = '64x64 at 45 deg'; x = 1828800; y = 914400; cx = 609600; cy = 609600; rot = 2700000; fill = 'EE4455' }
    @{ tag = 'sq30-y96'; title = '64x64 at 30 deg'; x = 1828800; y = 914400; cx = 609600; cy = 609600; rot = 1800000; fill = 'EE4455' }
    @{ tag = 'sq15-y96'; title = '64x64 at 15 deg'; x = 1828800; y = 914400; cx = 609600; cy = 609600; rot = 900000; fill = 'EE4455' }
    @{ tag = 'sq60-y96'; title = '64x64 at 60 deg'; x = 1828800; y = 914400; cx = 609600; cy = 609600; rot = 3600000; fill = 'EE4455' }
    @{ tag = 'sq120-y96'; title = '64x64 at 120 deg'; x = 1828800; y = 914400; cx = 609600; cy = 609600; rot = 7200000; fill = 'EE4455' }
    @{ tag = 'sq315-y96'; title = '64x64 at 315 deg'; x = 1828800; y = 914400; cx = 609600; cy = 609600; rot = 18900000; fill = 'EE4455' }
    @{ tag = 'sq22-y96'; title = '64x64 at 22.5 deg'; x = 1828800; y = 914400; cx = 609600; cy = 609600; rot = 1350000; fill = 'EE4455' }
    @{ tag = 'wide45-y96'; title = '96x48 at 45 deg'; x = 1828800; y = 914400; cx = 914400; cy = 457200; rot = 2700000; fill = 'EE4455' }
    @{ tag = 'tall30-y96'; title = '32x96 at 30 deg'; x = 1828800; y = 914400; cx = 304800; cy = 914400; rot = 1800000; fill = 'EE4455' }
    @{ tag = 'sq45-x400'; title = '64x64 at 45 deg origin 400,200'; x = 3810000; y = 1905000; cx = 609600; cy = 609600; rot = 2700000; fill = 'EE4455' }
    @{ tag = 'wide45-x10'; title = '96x48 at 45 deg origin 10,200'; x = 95250; y = 1905000; cx = 914400; cy = 457200; rot = 2700000; fill = 'EE4455' }
    @{ tag = 'sq45-blue'; title = '64x64 blue at 45 deg'; x = 1828800; y = 914400; cx = 609600; cy = 609600; rot = 2700000; fill = '0000FF' }
    @{ tag = 'sq90-ctrl'; title = '96x48 at 90 deg control'; x = 1828800; y = 914400; cx = 914400; cy = 457200; rot = 5400000; fill = 'EE4455' }
)

$src = Get-OutPath 'pptx-preview-rot-rect-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
$slides = @()
foreach ($s in $specs) {
    $slides += @{ shapes = @(@{ text = ''; prst = 'rect'; fill = $s.fill; x = $s.x; y = $s.y; cx = $s.cx; cy = $s.cy }) }
}
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'create'
    path          = $src.Replace('\', '/')
    presentation  = @{ slides = $slides }
} 'pptx-preview-rot-rect-create' | Out-Null

$ops = @()
for ($i = 0; $i -lt $specs.Count; $i++) {
    $n = $i + 1
    $ops += @{ op = 'setPageBackground'; id = ('slide:{0}' -f $n); color = 'AABBCC' }
    $ops += @{ op = 'insertPicture'; id = ('slide:{0}' -f $n); file = $pngNoise.Replace('\', '/'); x = 0; y = 0; cx = 609600; cy = 609600 }
}
$deck0 = Get-OutPath 'pptx-preview-rot-rect-plain.pptx'
if (Test-Path $deck0) { Remove-Item -Force $deck0 }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'apply'
    path          = $src.Replace('\', '/')
    output        = $deck0.Replace('\', '/')
    ops           = $ops
} 'pptx-preview-rot-rect-apply' | Out-Null

$deck = Get-OutPath 'pptx-preview-rot-rect.pptx'
if (Test-Path $deck) { Remove-Item -Force $deck }
Copy-Item $deck0 $deck
$zip = [IO.Compression.ZipFile]::Open($deck, [IO.Compression.ZipArchiveMode]::Update)
try {
    for ($i = 0; $i -lt $specs.Count; $i++) {
        $s = $specs[$i]
        $name = 'ppt/slides/slide{0}.xml' -f ($i + 1)
        $entry = $zip.GetEntry($name)
        $reader = New-Object IO.StreamReader($entry.Open())
        $text = $reader.ReadToEnd(); $reader.Dispose()
        $needle = ('<a:xfrm><a:off x="{0}" y="{1}"/><a:ext cx="{2}" cy="{3}"/>' -f $s.x, $s.y, $s.cx, $s.cy)
        $repl = ('<a:xfrm rot="{0}"><a:off x="{1}" y="{2}"/><a:ext cx="{3}" cy="{4}"/>' -f $s.rot, $s.x, $s.y, $s.cx, $s.cy)
        if ($text.IndexOf($needle) -lt 0) { throw ("missing box {0}" -f $name) }
        $text2 = $text.Replace($needle, $repl)
        $entry.Delete()
        $created = $zip.CreateEntry($name)
        $writer = New-Object IO.StreamWriter($created.Open()); $writer.Write($text2); $writer.Dispose()
    }
} finally { $zip.Dispose() }

$ppt = Ensure-MsPpt $deck
$diffScript = [IO.Path]::Combine($PSScriptRoot, '..', '..', 'pixel-diff.ps1')
for ($i = 0; $i -lt $specs.Count; $i++) {
    $s = $specs[$i]
    $n = $i + 1
    $lib = Get-OutPath ('pptx-preview-rot-rect-{0}-lib.png' -f $s.tag)
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'exportSlideImage'
        path          = $deck.Replace('\', '/')
        format        = 'pptx'
        id            = ('slide:{0}' -f $n)
        output        = $lib.Replace('\', '/')
    } ('pptx-preview-rot-rect-{0}-export' -f $s.tag) | Out-Null

    $gold = Get-OutPath ('pptx-preview-rot-rect-{0}-gold.png' -f $s.tag)
    $pres = $null
    $slide = $null
    try {
        $pres = Open-PptPres $ppt $deck
        $cap = Get-PptCaption $pres
        $slide = Get-PptSlide $pres $n
        if (Test-Path $gold) { Remove-Item -Force $gold }
        $slide.Export($gold, 'PNG')
        $ok = $cap -notmatch 'Repair'
        Add-Check ('pptx-preview-rot-rect-{0}-open' -f $s.tag) $ok ('caption={0} {1}' -f $cap, $s.title)
    }
    finally {
        Release-Com $slide
        Close-PptPres $pres
    }

    $diff = Get-OutPath ('pptx-preview-rot-rect-{0}-diff.png' -f $s.tag)
    $line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
    $code = $LASTEXITCODE
    $text = ($line | Out-String).Trim()
    Write-Log ('pptx-preview-rot-rect-{0}-px exit={1} {2}' -f $s.tag, $code, $text)
    $pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
    Add-Check ('pptx-preview-rot-rect-{0}-px' -f $s.tag) $pxOk ('exit={0} {1}' -f $code, $text)
}
