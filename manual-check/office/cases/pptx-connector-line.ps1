# pptx 连接线 prst=line。PowerPoint 能打开，线可见，可以选中并改位置。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$hostPath = Ensure-PptxCreate
$ppt = Ensure-MsPpt $hostPath
$out = Get-OutPath 'pptx-connector-line.pptx'
Close-PptPresByPath $ppt $out
if (Test-Path $out) { Remove-Item -Force $out }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(@{
        op = 'insertConnector'; id = 'slide:1'; prst = 'line'
        x = 400000; y = 500000; cx = 1200000; cy = 300000
    })
} 'pptx-line-apply' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $out.Replace('\', '/'); format = 'pptx'
    scope = @{ connectors = $true; slide = 1 }
} 'pptx-line-inspect'
$cxn = @($tree.slides[0].connectors) | Select-Object -First 1
$pres = Open-PptPres $ppt $out
$cap = Get-PptCaption $pres
$slide = Get-PptSlide $pres 1
$line = $null
$shapes = $slide.Shapes
try {
    $n = [int]$shapes.Count
    for ($i = 1; $i -le $n; $i++) {
        $s = $shapes.Item($i)
        $isLine = $false
        try { $isLine = [int]$s.Connector -ne 0 } catch { $isLine = $false }
        if ($isLine) { $line = $s; break }
        Release-Com $s
    }
} finally { Release-Com $shapes }

$moved = $false
if ($null -ne $line) {
    $before = [double]$line.Left
    $line.Left = $before + 12
    $moved = ([math]::Abs([double]$line.Left - ($before + 12)) -lt 0.5)
    Release-Com $line
}
Release-Com $slide
Close-PptPres $pres
$ok = ($null -ne $cxn) `
    -and ([string]$cxn.prst -eq 'line') `
    -and ($cap -notmatch 'Repair') `
    -and $moved
Add-Check 'pptx-connector-line' $ok "prst=$($cxn.prst) moved=$moved cap=$cap"
