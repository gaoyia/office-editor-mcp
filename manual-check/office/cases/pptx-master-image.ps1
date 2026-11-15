# 母版和版式上的实心矩形画在本页内容下面。showMasterSp 为假时不画。
# 金样是 PowerPoint 16.0 的 Slide.Export。红区回绘制引擎修，不要加大 MaxDelta。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))

function Set-PartText([string]$path, [string]$entryName, [scriptblock]$edit) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [System.IO.Compression.ZipFile]::Open($path, [System.IO.Compression.ZipArchiveMode]::Update)
    try {
        $entry = $zip.GetEntry($entryName)
        $reader = New-Object System.IO.StreamReader($entry.Open())
        try { $text = $reader.ReadToEnd() } finally { $reader.Dispose() }
        $text = & $edit $text
        $entry.Delete()
        $created = $zip.CreateEntry($entryName)
        $writer = New-Object System.IO.StreamWriter($created.Open())
        try { $writer.Write($text) } finally { $writer.Dispose() }
    }
    finally { $zip.Dispose() }
}

if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'pptx-master-image-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'pptx-master-image-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-master-image-inspect'
$titleId = [string]$tree.slides[0].shapes[0].id

$deck = Get-OutPath 'pptx-master-image.pptx'
if (Test-Path $deck) { Remove-Item -Force $deck }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $deck.Replace('\', '/')
    ops = @(
        @{ op = 'setPageBackground'; id = 'slide:1'; color = 'F0F0F0' }
        @{ op = 'deleteShape'; id = $titleId }
        @{
            op = 'insertShape'; id = 'master:1/layout:1'; fill = 'EE4455'
            x = 914400; y = 914400; cx = 914400; cy = 457200
        }
        @{
            op = 'insertShape'; id = 'master:1'; fill = '112233'
            x = 914400; y = 1828800; cx = 914400; cy = 457200
        }
        @{
            op = 'insertShape'; id = 'slide:1'; fill = '00AA66'
            x = 1524000; y = 914400; cx = 304800; cy = 457200
        }
    )
} 'pptx-master-image-apply' | Out-Null

$hidden = Get-OutPath 'pptx-master-image-hidden.pptx'
Copy-Item -LiteralPath $deck -Destination $hidden -Force
Set-PartText $hidden 'ppt/slides/slide1.xml' { param($text) $text.Replace('<p:sld ', '<p:sld showMasterSp="0" ') }

$layoutHidden = Get-OutPath 'pptx-master-image-layout.pptx'
Copy-Item -LiteralPath $deck -Destination $layoutHidden -Force
Set-PartText $layoutHidden 'ppt/slideLayouts/slideLayout1.xml' {
    param($text) $text.Replace('<p:sldLayout ', '<p:sldLayout showMasterSp="0" ')
}

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

$lib = Export-Lib $deck 'pptx-master-image-lib.png' 'pptx-master-image-export'
$libHidden = Export-Lib $hidden 'pptx-master-image-hidden-lib.png' 'pptx-master-image-hidden-export'
$libLayout = Export-Lib $layoutHidden 'pptx-master-image-layout-lib.png' 'pptx-master-image-layout-export'

$ppt = Ensure-MsPpt $deck
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

Compare-Slide $deck $lib 'pptx-master-image-gold.png' 'pptx-master-image-diff.png' 'pptx-master-image'
Compare-Slide $hidden $libHidden 'pptx-master-image-hidden-gold.png' 'pptx-master-image-hidden-diff.png' 'pptx-master-image-hidden'
Compare-Slide $layoutHidden $libLayout 'pptx-master-image-layout-gold.png' 'pptx-master-image-layout-diff.png' 'pptx-master-image-layout'
