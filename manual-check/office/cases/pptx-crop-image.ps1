# 整像素、不缩放的 srcRect 原样拷贝。半像素百分比整页失败。
# 金样是 PowerPoint 16.0 的 Slide.Export。红区回绘制引擎修，不要加大 MaxDelta。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))

function Add-SrcRect([string]$text, [string]$off, [string]$rect) {
    $needle = '<a:off ' + $off + '/>'
    $at = $text.IndexOf($needle)
    if ($at -lt 0) { throw "找不到 $needle" }
    $blip = $text.LastIndexOf('<a:blip ', $at)
    $end = $text.IndexOf('/>', $blip)
    if ($blip -lt 0 -or $end -lt 0) { throw '找不到 blip' }
    return $text.Substring(0, $end + 2) + $rect + $text.Substring($end + 2)
}

function Write-SlideXml([string]$path, [string]$text) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [System.IO.Compression.ZipFile]::Open($path, [System.IO.Compression.ZipArchiveMode]::Update)
    try {
        $entry = $zip.GetEntry('ppt/slides/slide1.xml')
        $entry.Delete()
        $created = $zip.CreateEntry('ppt/slides/slide1.xml')
        $writer = New-Object System.IO.StreamWriter($created.Open())
        try { $writer.Write($text) } finally { $writer.Dispose() }
    }
    finally { $zip.Dispose() }
}

function Read-SlideXml([string]$path) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [System.IO.Compression.ZipFile]::OpenRead($path)
    try {
        $entry = $zip.GetEntry('ppt/slides/slide1.xml')
        $reader = New-Object System.IO.StreamReader($entry.Open())
        try { return $reader.ReadToEnd() } finally { $reader.Dispose() }
    }
    finally { $zip.Dispose() }
}

if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.Drawing
$png = Get-OutPath 'pptx-crop-image-dots.png'
$bmp = New-Object System.Drawing.Bitmap 4, 4
$colors = @(
    @(0xFF, 0x00, 0x00), @(0x00, 0xFF, 0x00), @(0x00, 0x00, 0xFF), @(0xFF, 0xFF, 0x00),
    @(0xFF, 0x00, 0xFF), @(0x00, 0xFF, 0xFF), @(0x11, 0x22, 0x33), @(0x44, 0x55, 0x66),
    @(0x77, 0x88, 0x99), @(0xAA, 0xBB, 0xCC), @(0xDD, 0xEE, 0xFF), @(0x01, 0x02, 0x03),
    @(0x04, 0x05, 0x06), @(0x07, 0x08, 0x09), @(0x0A, 0x0B, 0x0C), @(0x0D, 0x0E, 0x0F)
)
for ($i = 0; $i -lt 16; $i++) {
    $c = $colors[$i]
    $bmp.SetPixel($i % 4, [Math]::Floor($i / 4), [System.Drawing.Color]::FromArgb(255, $c[0], $c[1], $c[2]))
}
$bmp.Save($png, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()

$png3 = Get-OutPath 'pptx-crop-image-dots3.png'
$bmp3 = New-Object System.Drawing.Bitmap 3, 1
$bmp3.SetPixel(0, 0, [System.Drawing.Color]::FromArgb(255, 0x10, 0x20, 0x30))
$bmp3.SetPixel(1, 0, [System.Drawing.Color]::FromArgb(255, 0x40, 0x50, 0x60))
$bmp3.SetPixel(2, 0, [System.Drawing.Color]::FromArgb(255, 0x70, 0x80, 0x90))
$bmp3.Save($png3, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp3.Dispose()

$src = Get-OutPath 'pptx-crop-image-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'pptx-crop-image-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-crop-image-inspect'
$titleId = [string]$tree.slides[0].shapes[0].id

$deck = Get-OutPath 'pptx-crop-image.pptx'
if (Test-Path $deck) { Remove-Item -Force $deck }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $deck.Replace('\', '/')
    ops = @(
        @{ op = 'setPageBackground'; id = 'slide:1'; color = 'F0F0F0' }
        @{ op = 'deleteShape'; id = $titleId }
        @{
            op = 'insertPicture'; id = 'slide:1'; file = $png.Replace('\', '/')
            x = 914400; y = 914400; cx = 19050; cy = 19050
        }
        @{
            op = 'insertPicture'; id = 'slide:1'; file = $png.Replace('\', '/')
            x = 1828800; y = 914400; cx = 28575; cy = 38100
        }
        @{
            op = 'insertPicture'; id = 'slide:1'; file = $png3.Replace('\', '/')
            x = 2743200; y = 914400; cx = 19050; cy = 9525
        }
    )
} 'pptx-crop-image-apply' | Out-Null

$xml = Read-SlideXml $deck
$xml = Add-SrcRect $xml 'x="914400" y="914400"' '<a:srcRect l="25000" t="25000" r="25000" b="25000"/>'
$xml = Add-SrcRect $xml 'x="1828800" y="914400"' '<a:srcRect l="25000"/>'
$xml = Add-SrcRect $xml 'x="2743200" y="914400"' '<a:srcRect l="33333"/>'
Write-SlideXml $deck $xml

$fractional = Get-OutPath 'pptx-crop-image-frac.pptx'
Copy-Item -LiteralPath $deck -Destination $fractional -Force
$fracXml = Read-SlideXml $fractional
$fracXml = $fracXml.Replace('<a:srcRect l="25000" t="25000" r="25000" b="25000"/>', '<a:srcRect l="12500"/>')
Write-SlideXml $fractional $fracXml

$lib = Get-OutPath 'pptx-crop-image-lib.png'
if (Test-Path $lib) { Remove-Item -Force $lib }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSlideImage'
    path = $deck.Replace('\', '/'); format = 'pptx'
    id = 'slide:1'
    output = $lib.Replace('\', '/')
} 'pptx-crop-image-export' | Out-Null

$rotOk = $false
$rotDetail = ''
try {
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'exportSlideImage'
        path = $fractional.Replace('\', '/'); format = 'pptx'
        id = 'slide:1'
        output = (Get-OutPath 'pptx-crop-image-frac.png').Replace('\', '/')
    } 'pptx-crop-image-frac' | Out-Null
    $rotDetail = '半像素裁剪应当整页失败'
}
catch {
    $rotDetail = $_.Exception.Message
    $rotOk = $rotDetail -match 'NotImplemented'
}
Add-Check 'pptx-crop-image-frac' $rotOk $rotDetail

$ppt = Ensure-MsPpt $deck
$pres = $null
$slide = $null
$gold = Get-OutPath 'pptx-crop-image-gold.png'
try {
    $pres = Open-PptPres $ppt $deck
    $cap = Get-PptCaption $pres
    $slide = Get-PptSlide $pres 1
    if (Test-Path $gold) { Remove-Item -Force $gold }
    $slide.Export($gold, 'PNG')
    $ok = $cap -notmatch 'Repair'
    Add-Check 'pptx-crop-image-open' $ok "caption=$cap"
}
finally {
    Release-Com $slide
    Close-PptPres $pres
}

$diff = Get-OutPath 'pptx-crop-image-diff.png'
$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
$code = $LASTEXITCODE
$text = ($line | Out-String).Trim()
Write-Log "pptx-crop-image-px exit=$code $text"
$pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'pptx-crop-image-px' $pxOk "exit=$code $text"
