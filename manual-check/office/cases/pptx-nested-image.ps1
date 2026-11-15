# 幻灯片上的纯色矩形可以放在两层组合里，外层还可以转 90 度。
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

$src = Get-OutPath 'pptx-nested-src.pptx'
$deck = Get-OutPath 'pptx-nested.pptx'
foreach ($p in @($src, $deck)) { if (Test-Path $p) { Remove-Item -Force $p } }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'pptx-nested-create' | Out-Null
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-nested-inspect'
$titleId = [string]$tree.slides[0].shapes[0].id
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $deck.Replace('\', '/')
    ops = @(
        @{ op = 'setPageBackground'; id = 'slide:1'; color = 'FFFFFF' }
        @{ op = 'deleteShape'; id = $titleId }
    )
} 'pptx-nested-blank' | Out-Null

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
    function Add-Nest([double]$y, [bool]$turn) {
        $inner = $shapes.Range([string[]]@(
            (Add-Box (100 * $pt) ($y * $pt) (80 * $pt) (24 * $pt)),
            (Add-Box (200 * $pt) ($y * $pt) (40 * $pt) (24 * $pt))
        )).Group()
        $innerName = [string]$inner.Name
        Release-Com $inner
        $extra = Add-Box (100 * $pt) (($y + 40) * $pt) (60 * $pt) (20 * $pt)
        $outer = $shapes.Range([string[]]@($innerName, $extra)).Group()
        if ($turn) { $outer.Rotation = 90 }
        Release-Com $outer
    }
    Add-Nest 80 $false
    Add-Nest 360 $true
    $pres.Save()
    Release-Com $shapes
    Release-Com $slide
}
finally { Close-PptPres $pres }
Stop-Ppt

$png = Get-OutPath 'pptx-nested-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSlideImage'
    path = $deck.Replace('\', '/'); format = 'pptx'
    id = 'slide:1'
    output = $png.Replace('\', '/')
} 'pptx-nested-export' | Out-Null

$ppt = Ensure-MsPpt $deck
$pres = Open-PptPres $ppt $deck
$gold = Get-OutPath 'pptx-nested-gold.png'
try {
    $slide = Get-PptSlide $pres 1
    if (Test-Path $gold) { Remove-Item -Force $gold }
    $slide.Export($gold, 'PNG')
    Release-Com $slide
}
finally { Close-PptPres $pres }
Stop-Ppt

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$out = Get-OutPath 'pptx-nested-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $out -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "pptx-nested-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'pptx-nested-px' $ok $text
