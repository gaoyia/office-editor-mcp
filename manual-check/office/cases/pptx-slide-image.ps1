# pptx 纯色页底、等像素 PNG 和没有描边的实心矩形。金样是 PowerPoint 16.0 的 Slide.Export。
# 像素差用 pixel-diff 标红。红区要回绘制引擎修，不要加大 MaxDelta。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.Drawing
$png = Get-OutPath 'pptx-slide-image-dot.png'
$bmp = New-Object System.Drawing.Bitmap 2, 2
$color = [System.Drawing.Color]::FromArgb(255, 0x11, 0x22, 0x33)
foreach ($x in 0, 1) {
    foreach ($y in 0, 1) { $bmp.SetPixel($x, $y, $color) }
}
$bmp.Save($png, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()

$src = Get-OutPath 'pptx-slide-image-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'pptx-slide-image-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-slide-image-inspect'
$shapeId = [string]$tree.slides[0].shapes[0].id

$deck = Get-OutPath 'pptx-slide-image.pptx'
if (Test-Path $deck) { Remove-Item -Force $deck }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $deck.Replace('\', '/')
    ops = @(
        @{ op = 'setPageBackground'; id = 'slide:1'; color = 'AABBCC' }
        @{ op = 'deleteShape'; id = $shapeId }
        @{
            op = 'insertPicture'; id = 'slide:1'; file = $png.Replace('\', '/')
            x = 914400; y = 914400; cx = 19050; cy = 19050
        }
        @{
            op = 'insertShape'; id = 'slide:1'; fill = 'EE4455'
            x = 1828800; y = 914400; cx = 914400; cy = 457200
        }
    )
} 'pptx-slide-image-apply' | Out-Null

$lib = Get-OutPath 'pptx-slide-image-lib.png'
if (Test-Path $lib) { Remove-Item -Force $lib }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSlideImage'
    path = $deck.Replace('\', '/'); format = 'pptx'
    id = 'slide:1'
    output = $lib.Replace('\', '/')
} 'pptx-slide-image-export' | Out-Null

$ppt = Ensure-MsPpt $deck
$pres = $null
$slide = $null
$gold = Get-OutPath 'pptx-slide-image-gold.png'
try {
    $pres = Open-PptPres $ppt $deck
    $cap = Get-PptCaption $pres
    $slide = Get-PptSlide $pres 1
    if (Test-Path $gold) { Remove-Item -Force $gold }
    $slide.Export($gold, 'PNG')
    $ok = $cap -notmatch 'Repair'
    Add-Check 'pptx-slide-image-open' $ok "caption=$cap"
}
finally {
    Release-Com $slide
    Close-PptPres $pres
}

$diff = Get-OutPath 'pptx-slide-image-diff.png'
$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
$code = $LASTEXITCODE
$text = ($line | Out-String).Trim()
Write-Log "pptx-slide-image-px exit=$code $text"
# pixel-diff 宽高不同才退出 2。有差异时退出码仍是 0，是否通过只看 differ。
$pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'pptx-slide-image-px' $pxOk "exit=$code $text"
