# 幻灯片上的纯色组合可以绕中心转 90 度的倍数，也可以水平翻转。
# 金样是 PowerPoint 16.0 的 Slide.Export。红区回绘制引擎修，不要加大 MaxDelta。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'office-com.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function Stop-Ppt {
    Close-OfficeApps -Quit
    if ($global:OoxpPptPid) {
        Stop-Process -Id $global:OoxpPptPid -Force -ErrorAction SilentlyContinue
        $global:OoxpPptPid = $null
        $global:OoxpPpt = $null
    }
}

$src = Get-OutPath 'pptx-turn-group-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'pptx-turn-group-create' | Out-Null
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-turn-group-inspect'
$titleId = [string]$tree.slides[0].shapes[0].id
$deck = Get-OutPath 'pptx-turn-group.pptx'
if (Test-Path $deck) { Remove-Item -Force $deck }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $deck.Replace('\', '/')
    ops = @(
        @{ op = 'setPageBackground'; id = 'slide:1'; color = 'FFFFFF' }
        @{ op = 'deleteShape'; id = $titleId }
    )
} 'pptx-turn-group-blank' | Out-Null

$ppt = Ensure-MsPpt $deck
$pres = Open-PptPres $ppt $deck
try {
    $slide = Get-PptSlide $pres 1
    $shapes = $slide.Shapes
    $pt = 0.75
    function Add-Box([double]$left, [double]$top, [double]$width, [double]$height) {
        $sh = $shapes.AddShape(1, $left, $top, $width, $height)
        $sh.Fill.ForeColor.RGB = 0x5544EE
        $sh.Line.Visible = 0
        $name = [string]$sh.Name
        Release-Com $sh
        return $name
    }
    $g90 = $shapes.Range([string[]]@(
        (Add-Box (80 * $pt) (80 * $pt) (160 * $pt) (32 * $pt)),
        (Add-Box (200 * $pt) (48 * $pt) (32 * $pt) (48 * $pt))
    )).Group()
    $g90.Rotation = 90
    $g180 = $shapes.Range([string[]]@(
        (Add-Box (400 * $pt) (80 * $pt) (96 * $pt) (32 * $pt)),
        (Add-Box (448 * $pt) (128 * $pt) (32 * $pt) (48 * $pt))
    )).Group()
    $g180.Rotation = 180
    $gh = $shapes.Range([string[]]@(
        (Add-Box (80 * $pt) (320 * $pt) (96 * $pt) (32 * $pt)),
        (Add-Box (128 * $pt) (368 * $pt) (32 * $pt) (48 * $pt))
    )).Group()
    $gh.Flip(0)
    $pres.Save()
    Release-Com $g90
    Release-Com $g180
    Release-Com $gh
    Release-Com $shapes
    Release-Com $slide
}
finally { Close-PptPres $pres }
Stop-Ppt

$png = Get-OutPath 'pptx-turn-group-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSlideImage'
    path = $deck.Replace('\', '/'); format = 'pptx'
    id = 'slide:1'
    output = $png.Replace('\', '/')
} 'pptx-turn-group-export' | Out-Null

$ppt = Ensure-MsPpt $deck
$pres = Open-PptPres $ppt $deck
$gold = Get-OutPath 'pptx-turn-group-gold.png'
try {
    $slide = Get-PptSlide $pres 1
    if (Test-Path $gold) { Remove-Item -Force $gold }
    $slide.Export($gold, 'PNG')
    Release-Com $slide
}
finally { Close-PptPres $pres }
Stop-Ppt

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$out = Get-OutPath 'pptx-turn-group-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $out -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "pptx-turn-group-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'pptx-turn-group-px' $ok $text
