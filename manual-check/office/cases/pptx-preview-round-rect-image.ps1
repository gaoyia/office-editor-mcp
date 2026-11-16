# 不透明无描边纯色圆角矩形：对照 PowerPoint 16.0 Slide.Export，要求整图像素 differ=0 / maxDelta=0。
# 覆盖多种尺寸、半径（adj）、页底、填色和原点。用于直边实色与拐角覆盖的独立官方验收。
# 不与形状渐变收口混用。t93u 不适用于圆角形状。不得用几何锚点替代全图像素检查。
# 不测描边、文字、半透明、渐变、翻转或旋转。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

$pngNoise = Get-OutPath 'pptx-preview-round-rect-noise.png'
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
    @{ tag = 'card-def'; title = '192x96 adj 16667 at 384,96'; x = 3657600; y = 914400; cx = 1828800; cy = 914400; adj = 16667; fill = '00AA00'; bg = 'AABBCC' }
    @{ tag = 'card-r1k'; title = '192x96 adj 1000 at 384,96'; x = 3657600; y = 914400; cx = 1828800; cy = 914400; adj = 1000; fill = '00AA00'; bg = 'AABBCC' }
    @{ tag = 'card-r5k'; title = '192x96 adj 5000 at 384,96'; x = 3657600; y = 914400; cx = 1828800; cy = 914400; adj = 5000; fill = '00AA00'; bg = 'AABBCC' }
    @{ tag = 'card-r33k'; title = '192x96 adj 33333 at 384,96'; x = 3657600; y = 914400; cx = 1828800; cy = 914400; adj = 33333; fill = '00AA00'; bg = 'AABBCC' }
    @{ tag = 'card-r50k'; title = '192x96 adj 50000 at 384,96'; x = 3657600; y = 914400; cx = 1828800; cy = 914400; adj = 50000; fill = '00AA00'; bg = 'AABBCC' }
    @{ tag = 'card-r0'; title = '192x96 adj 0 sharp control'; x = 3657600; y = 914400; cx = 1828800; cy = 914400; adj = 0; fill = '00AA00'; bg = 'AABBCC' }
    @{ tag = 'sq64-def'; title = '64x64 adj 16667 at 192,96'; x = 1828800; y = 914400; cx = 609600; cy = 609600; adj = 16667; fill = '00AA00'; bg = 'AABBCC' }
    @{ tag = 'wide96-def'; title = '96x48 adj 16667 at 192,96'; x = 1828800; y = 914400; cx = 914400; cy = 457200; adj = 16667; fill = '00AA00'; bg = 'AABBCC' }
    @{ tag = 'tall48-def'; title = '48x96 adj 16667 at 192,96'; x = 1828800; y = 914400; cx = 457200; cy = 914400; adj = 16667; fill = '00AA00'; bg = 'AABBCC' }
    @{ tag = 'sq32-def'; title = '32x32 adj 16667 at 192,96'; x = 1828800; y = 914400; cx = 304800; cy = 304800; adj = 16667; fill = '00AA00'; bg = 'AABBCC' }
    @{ tag = 'wide256-def'; title = '256x128 adj 16667 at 192,96'; x = 1828800; y = 914400; cx = 2438400; cy = 1219200; adj = 16667; fill = '00AA00'; bg = 'AABBCC' }
    @{ tag = 'card-dark'; title = '192x96 adj 16667 on 334455'; x = 3657600; y = 914400; cx = 1828800; cy = 914400; adj = 16667; fill = '00AA00'; bg = '334455' }
    @{ tag = 'card-white'; title = '192x96 adj 16667 on FFFFFF'; x = 3657600; y = 914400; cx = 1828800; cy = 914400; adj = 16667; fill = '00AA00'; bg = 'FFFFFF' }
    @{ tag = 'card-red'; title = '192x96 adj 16667 fill EE4455'; x = 3657600; y = 914400; cx = 1828800; cy = 914400; adj = 16667; fill = 'EE4455'; bg = 'AABBCC' }
    @{ tag = 'card-xy10'; title = '192x96 adj 16667 at 10,200'; x = 95250; y = 1905000; cx = 1828800; cy = 914400; adj = 16667; fill = '00AA00'; bg = 'AABBCC' }
    @{ tag = 'card-odd'; title = '192x96 adj 16667 at 193,97'; x = 1838325; y = 923925; cx = 1828800; cy = 914400; adj = 16667; fill = '00AA00'; bg = 'AABBCC' }
)

$src = Get-OutPath 'pptx-preview-round-rect-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
$slides = @()
foreach ($s in $specs) {
    $slides += @{ shapes = @(@{ text = ''; prst = 'roundRect'; fill = $s.fill; line = @{ color = $null }; x = $s.x; y = $s.y; cx = $s.cx; cy = $s.cy }) }
}
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'create'
    path          = $src.Replace('\', '/')
    presentation  = @{ slides = $slides }
} 'pptx-preview-round-rect-create' | Out-Null

$ops = @()
for ($i = 0; $i -lt $specs.Count; $i++) {
    $n = $i + 1
    $ops += @{ op = 'setPageBackground'; id = ('slide:{0}' -f $n); color = $specs[$i].bg }
    $ops += @{ op = 'insertPicture'; id = ('slide:{0}' -f $n); file = $pngNoise.Replace('\', '/'); x = 0; y = 0; cx = 609600; cy = 609600 }
}
$deck0 = Get-OutPath 'pptx-preview-round-rect-plain.pptx'
if (Test-Path $deck0) { Remove-Item -Force $deck0 }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'apply'
    path          = $src.Replace('\', '/')
    output        = $deck0.Replace('\', '/')
    ops           = $ops
} 'pptx-preview-round-rect-apply' | Out-Null

$deck = Get-OutPath 'pptx-preview-round-rect.pptx'
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
        $needle = '<a:prstGeom prst="roundRect"><a:avLst/></a:prstGeom>'
        $repl = ('<a:prstGeom prst="roundRect"><a:avLst><a:gd name="adj" fmla="val {0}"/></a:avLst></a:prstGeom>' -f $s.adj)
        if ($text.IndexOf($needle) -lt 0) { throw ("missing geom {0}" -f $name) }
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
    $lib = Get-OutPath ('pptx-preview-round-rect-{0}-lib.png' -f $s.tag)
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'exportSlideImage'
        path          = $deck.Replace('\', '/')
        format        = 'pptx'
        id            = ('slide:{0}' -f $n)
        output        = $lib.Replace('\', '/')
    } ('pptx-preview-round-rect-{0}-export' -f $s.tag) | Out-Null

    $gold = Get-OutPath ('pptx-preview-round-rect-{0}-gold.png' -f $s.tag)
    $pres = $null
    $slide = $null
    try {
        $pres = Open-PptPres $ppt $deck
        $cap = Get-PptCaption $pres
        $slide = Get-PptSlide $pres $n
        if (Test-Path $gold) { Remove-Item -Force $gold }
        $slide.Export($gold, 'PNG')
        $ok = $cap -notmatch 'Repair'
        Add-Check ('pptx-preview-round-rect-{0}-open' -f $s.tag) $ok ('caption={0} {1}' -f $cap, $s.title)
    }
    finally {
        Release-Com $slide
        Close-PptPres $pres
    }

    $diff = Get-OutPath ('pptx-preview-round-rect-{0}-diff.png' -f $s.tag)
    $line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
    $code = $LASTEXITCODE
    $text = ($line | Out-String).Trim()
    Write-Log ('pptx-preview-round-rect-{0}-px exit={1} {2}' -f $s.tag, $code, $text)
    $pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
    Add-Check ('pptx-preview-round-rect-{0}-px' -f $s.tag) $pxOk ('exit={0} {1}' -f $code, $text)
}
