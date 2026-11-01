# 版式上的内容占位符和图片占位符。PowerPoint 16.0 用该版式新建一页后能看见单击提示，没有 Repair。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'pptx-ph-more-src.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'pptx'
    presentation = @{ slides = @(@{ shapes = @(@{ text = '页' }) }) }
} 'pptx-ph-more-create' | Out-Null

$out = Get-OutPath 'pptx-ph-more.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(
        @{ op = 'insertPlaceholder'; id = 'master:1/layout:1'; type = 'obj' }
        @{ op = 'insertPlaceholder'; id = 'master:1/layout:1'; type = 'pic' }
    )
} 'pptx-ph-more-insert' | Out-Null

$ppt = Ensure-MsPpt $out
$pres = Open-PptPres $ppt $out
$cap = ''
try { $cap = [string]$pres.Windows.Item(1).Caption } catch { $cap = '' }
$master = $pres.SlideMaster
$layouts = $master.CustomLayouts
$layout = $layouts.Item(1)
$slides = $pres.Slides
$slide = $slides.AddSlide($slides.Count + 1, $layout)
$shapes = $slide.Shapes
$hits = @()
$dump = @()
for ($i = 1; $i -le $shapes.Count; $i++) {
    $s = $shapes.Item($i)
    $text = ''
    $ph = ''
    try { $text = [string]$s.TextFrame.TextRange.Text } catch { $text = '' }
    try { $ph = [string]$s.PlaceholderFormat.Type } catch { $ph = '' }
    $dump += ("{0}:{1}:ph={2}:{3}" -f $s.Name, $s.Type, $ph, ($text -replace '\s+',' '))
    if ($ph -eq '7' -or $ph -eq '18') { $hits += $ph }
    Release-Com $s
}
Release-Com $shapes
Release-Com $slide
Release-Com $slides
Release-Com $layout
Release-Com $layouts
Release-Com $master
Add-Check 'pptx-ph-more' (($cap -notmatch 'Repair') -and ($hits.Count -ge 1)) "caption=$cap hits=$($hits -join '|') dump=$($dump -join ' || ')"
Close-PptPres $pres
