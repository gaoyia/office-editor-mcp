# 幻灯片组合里的纯色矩形可以绕自己的中心转 90 度。
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

$src = Get-OutPath 'pptx-child-turn-src.pptx'
$deck = Get-OutPath 'pptx-child-turn.pptx'
foreach ($p in @($src, $deck)) { if (Test-Path $p) { Remove-Item -Force $p } }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'pptx-child-turn-create' | Out-Null
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-child-turn-inspect'
$titleId = [string]$tree.slides[0].shapes[0].id
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $deck.Replace('\', '/')
    ops = @(
        @{ op = 'setPageBackground'; id = 'slide:1'; color = 'FFFFFF' }
        @{ op = 'deleteShape'; id = $titleId }
    )
} 'pptx-child-turn-blank' | Out-Null

$ppt = Ensure-MsPpt $deck
$pres = Open-PptPres $ppt $deck
try {
    $slide = Get-PptSlide $pres 1
    $shapes = $slide.Shapes
    $pt = 0.75
    function Add-Box([double]$left, [double]$top, [double]$width, [double]$height, [double]$rot) {
        $sh = $shapes.AddShape(1, $left, $top, $width, $height)
        $sh.Fill.ForeColor.RGB = 0x5544EE
        $sh.Line.Visible = 0
        if ($rot -ne 0) { $sh.Rotation = $rot }
        $name = [string]$sh.Name
        Release-Com $sh
        return $name
    }
    $before = $shapes.Range([string[]]@(
        (Add-Box (80 * $pt) (80 * $pt) (160 * $pt) (32 * $pt) 90),
        (Add-Box (280 * $pt) (80 * $pt) (48 * $pt) (32 * $pt) 0)
    )).Group()
    $a = Add-Box (80 * $pt) (280 * $pt) (160 * $pt) (32 * $pt) 0
    $b = Add-Box (280 * $pt) (280 * $pt) (48 * $pt) (32 * $pt) 0
    $after = $shapes.Range([string[]]@($a, $b)).Group()
    $items = $after.GroupItems
    $child = $items.Item(1)
    $child.Rotation = 90
    $pres.Save()
    Release-Com $child
    Release-Com $items
    Release-Com $before
    Release-Com $after
    Release-Com $shapes
    Release-Com $slide
}
finally { Close-PptPres $pres }
Stop-Ppt

$png = Get-OutPath 'pptx-child-turn-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSlideImage'
    path = $deck.Replace('\', '/'); format = 'pptx'
    id = 'slide:1'
    output = $png.Replace('\', '/')
} 'pptx-child-turn-export' | Out-Null

$ppt = Ensure-MsPpt $deck
$pres = Open-PptPres $ppt $deck
$gold = Get-OutPath 'pptx-child-turn-gold.png'
try {
    $slide = Get-PptSlide $pres 1
    if (Test-Path $gold) { Remove-Item -Force $gold }
    $slide.Export($gold, 'PNG')
    Release-Com $slide
}
finally { Close-PptPres $pres }
Stop-Ppt

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$out = Get-OutPath 'pptx-child-turn-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $out -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "pptx-child-turn-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'pptx-child-turn-px' $ok $text
