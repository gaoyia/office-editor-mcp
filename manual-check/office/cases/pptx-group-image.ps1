# 组合里的实心矩形。恒等组合和按 chExt 放大两倍的组合，都跟 PowerPoint 16.0 的 Slide.Export 比像素。
# 组合自身旋转时整页失败。红区回绘制引擎修，不要加大 MaxDelta。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))

function Set-GroupChExt([string]$path) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [System.IO.Compression.ZipFile]::Open($path, [System.IO.Compression.ZipArchiveMode]::Update)
    try {
        $entry = $zip.GetEntry('ppt/slides/slide1.xml')
        $reader = New-Object System.IO.StreamReader($entry.Open())
        try { $text = $reader.ReadToEnd() } finally { $reader.Dispose() }
        $needle = '<a:chExt cx="1828800" cy="457200"/>'
        if (-not $text.Contains($needle)) { throw "找不到要缩放的组合" }
        $text = $text.Replace($needle, '<a:chExt cx="914400" cy="228600"/>')
        $entry.Delete()
        $created = $zip.CreateEntry('ppt/slides/slide1.xml')
        $writer = New-Object System.IO.StreamWriter($created.Open())
        try { $writer.Write($text) } finally { $writer.Dispose() }
    }
    finally { $zip.Dispose() }
}

function Set-GroupRot([string]$path) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [System.IO.Compression.ZipFile]::Open($path, [System.IO.Compression.ZipArchiveMode]::Update)
    try {
        $entry = $zip.GetEntry('ppt/slides/slide1.xml')
        $reader = New-Object System.IO.StreamReader($entry.Open())
        try { $text = $reader.ReadToEnd() } finally { $reader.Dispose() }
        $needle = '<a:xfrm><a:off x="1828800" y="914400"/><a:ext cx="1828800"'
        if (-not $text.Contains($needle)) { throw "找不到要旋转的组合" }
        $text = $text.Replace($needle, '<a:xfrm rot="5400000"><a:off x="1828800" y="914400"/><a:ext cx="1828800"')
        $entry.Delete()
        $created = $zip.CreateEntry('ppt/slides/slide1.xml')
        $writer = New-Object System.IO.StreamWriter($created.Open())
        try { $writer.Write($text) } finally { $writer.Dispose() }
    }
    finally { $zip.Dispose() }
}

if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'pptx-group-image-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'pptx-group-image-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-group-image-inspect'
$titleId = [string]$tree.slides[0].shapes[0].id

$deck = Get-OutPath 'pptx-group-image.pptx'
if (Test-Path $deck) { Remove-Item -Force $deck }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $deck.Replace('\', '/')
    ops = @(
        @{ op = 'setPageBackground'; id = 'slide:1'; color = 'AABBCC' }
        @{ op = 'deleteShape'; id = $titleId }
        @{
            op = 'insertShape'; id = 'slide:1'; fill = 'EE4455'
            x = 1828800; y = 914400; cx = 914400; cy = 457200
        }
        @{
            op = 'insertShape'; id = 'slide:1'; fill = '112233'
            x = 2743200; y = 914400; cx = 914400; cy = 457200
        }
    )
} 'pptx-group-image-apply' | Out-Null

$placed = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $deck.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-group-image-ids'
$ids = @($placed.slides[0].shapes | ForEach-Object { [string]$_.id })
if ($ids.Count -ne 2) { throw "应有两个形状，实际 $($ids.Count)" }

$grouped = Get-OutPath 'pptx-group-image-grouped.pptx'
if (Test-Path $grouped) { Remove-Item -Force $grouped }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $deck.Replace('\', '/')
    output = $grouped.Replace('\', '/')
    ops = @(@{ op = 'groupShapes'; id = 'slide:1'; ids = $ids })
} 'pptx-group-image-group' | Out-Null

$scaled = Get-OutPath 'pptx-group-image-scaled.pptx'
Copy-Item -LiteralPath $grouped -Destination $scaled -Force
Set-GroupChExt $scaled

$rotated = Get-OutPath 'pptx-group-image-rot.pptx'
Copy-Item -LiteralPath $grouped -Destination $rotated -Force
Set-GroupRot $rotated

function Export-Lib([string]$srcPath, [string]$pngName, [string]$reqName) {
    $png = Get-OutPath $pngName
    if (Test-Path $png) { Remove-Item -Force $png }
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'exportSlideImage'
        path = $srcPath.Replace('\', '/'); format = 'pptx'
        id = 'slide:1'
        output = $png.Replace('\', '/')
    } $reqName | Out-Null
    return $png
}

$libSame = Export-Lib $grouped 'pptx-group-image-lib.png' 'pptx-group-image-export'
$libScale = Export-Lib $scaled 'pptx-group-image-scaled-lib.png' 'pptx-group-image-scaled-export'

$rotOk = $false
$rotDetail = ''
try {
    Export-Lib $rotated 'pptx-group-image-rot.png' 'pptx-group-image-rot-export' | Out-Null
    $rotDetail = '旋转的组合应当整页失败'
}
catch {
    $rotDetail = $_.Exception.Message
    $rotOk = $rotDetail -match 'NotImplemented'
}
Add-Check 'pptx-group-image-rot' $rotOk $rotDetail

$ppt = Ensure-MsPpt $grouped
$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))

function Compare-Slide([string]$deckPath, [string]$libPng, [string]$goldName, [string]$diffName, [string]$checkId) {
    $pres = $null
    $slide = $null
    $gold = Get-OutPath $goldName
    try {
        $pres = Open-PptPres $ppt $deckPath
        $cap = Get-PptCaption $pres
        $slide = Get-PptSlide $pres 1
        if (Test-Path $gold) { Remove-Item -Force $gold }
        $slide.Export($gold, 'PNG')
        $ok = $cap -notmatch 'Repair'
        Add-Check ($checkId + '-open') $ok "caption=$cap"
    }
    finally {
        Release-Com $slide
        Close-PptPres $pres
    }
    $diff = Get-OutPath $diffName
    $line = & $diffScript -Left $gold -Right $libPng -Out $diff -MaxDelta 0
    $code = $LASTEXITCODE
    $text = ($line | Out-String).Trim()
    Write-Log "$checkId exit=$code $text"
    $pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
    Add-Check ($checkId + '-px') $pxOk "exit=$code $text"
}

Compare-Slide $grouped $libSame 'pptx-group-image-gold.png' 'pptx-group-image-diff.png' 'pptx-group-image'
Compare-Slide $scaled $libScale 'pptx-group-image-scaled-gold.png' 'pptx-group-image-scaled-diff.png' 'pptx-group-image-scaled'
