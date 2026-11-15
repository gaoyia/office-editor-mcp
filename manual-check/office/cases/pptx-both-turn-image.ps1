# 幻灯片子矩形的 90 度可以和组合自己的 90 度或水平翻转叠在一起。
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

$src = Get-OutPath 'pptx-both-turn-src.pptx'
$deck = Get-OutPath 'pptx-both-turn.pptx'
foreach ($p in @($src, $deck)) { if (Test-Path $p) { Remove-Item -Force $p } }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'pptx-both-turn-create' | Out-Null
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-both-turn-inspect'
$titleId = [string]$tree.slides[0].shapes[0].id
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $deck.Replace('\', '/')
    ops = @(
        @{ op = 'setPageBackground'; id = 'slide:1'; color = 'FFFFFF' }
        @{ op = 'deleteShape'; id = $titleId }
    )
} 'pptx-both-turn-blank' | Out-Null

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
    $turned = $shapes.Range([string[]]@(
        (Add-Box (200 * $pt) (160 * $pt) (160 * $pt) (32 * $pt)),
        (Add-Box (400 * $pt) (160 * $pt) (48 * $pt) (32 * $pt))
    )).Group()
    $items = $turned.GroupItems
    $child = $items.Item(1)
    $child.Rotation = 90
    $turned.Rotation = 90
    $flipped = $shapes.Range([string[]]@(
        (Add-Box (200 * $pt) (480 * $pt) (160 * $pt) (32 * $pt)),
        (Add-Box (400 * $pt) (480 * $pt) (48 * $pt) (32 * $pt))
    )).Group()
    $items2 = $flipped.GroupItems
    $child2 = $items2.Item(1)
    $child2.Rotation = 90
    $flipped.Flip(0)
    $pres.Save()
    Release-Com $child
    Release-Com $items
    Release-Com $child2
    Release-Com $items2
    Release-Com $turned
    Release-Com $flipped
    Release-Com $shapes
    Release-Com $slide
}
finally { Close-PptPres $pres }
Stop-Ppt

$png = Get-OutPath 'pptx-both-turn-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSlideImage'
    path = $deck.Replace('\', '/'); format = 'pptx'
    id = 'slide:1'
    output = $png.Replace('\', '/')
} 'pptx-both-turn-export' | Out-Null

$ppt = Ensure-MsPpt $deck
$pres = Open-PptPres $ppt $deck
$gold = Get-OutPath 'pptx-both-turn-gold.png'
try {
    $slide = Get-PptSlide $pres 1
    if (Test-Path $gold) { Remove-Item -Force $gold }
    $slide.Export($gold, 'PNG')
    Release-Com $slide
}
finally { Close-PptPres $pres }
Stop-Ppt

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$out = Get-OutPath 'pptx-both-turn-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $out -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "pptx-both-turn-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'pptx-both-turn-px' $ok $text
