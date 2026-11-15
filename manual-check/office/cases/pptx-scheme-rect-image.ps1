# 幻灯片上没有描边的矩形，填充是没有颜色变换的 schemeClr。
# 金样是 PowerPoint 16.0 的 Slide.Export。红区回绘制引擎修，不要加大 MaxDelta。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'office-com.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Set-PartText([string]$path, [string]$entryName, [scriptblock]$edit) {
    $zip = [System.IO.Compression.ZipFile]::Open($path, [System.IO.Compression.ZipArchiveMode]::Update)
    try {
        $entry = $zip.GetEntry($entryName)
        if ($null -eq $entry) { throw "没有 $entryName" }
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

$src = Get-OutPath 'pptx-scheme-rect-src.pptx'
$deck = Get-OutPath 'pptx-scheme-rect.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
if (Test-Path $deck) { Remove-Item -Force $deck }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'pptx-scheme-rect-create' | Out-Null
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-scheme-rect-inspect'
$title = [string]$tree.slides[0].shapes[0].id
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $deck.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeColors'; id = 'master:1'; accent1 = 'EE4455' }
        @{ op = 'setPageBackground'; id = 'slide:1'; color = 'AABBCC' }
        @{ op = 'deleteShape'; id = $title }
        @{ op = 'insertShape'; id = 'slide:1'; fill = 'FFFFFF'; x = 1828800; y = 914400; cx = 914400; cy = 457200 }
        @{ op = 'insertShape'; id = 'slide:1'; fill = 'FFFFFF'; x = 3657600; y = 914400; cx = 457200; cy = 457200 }
        @{ op = 'insertShape'; id = 'slide:1'; fill = 'FFFFFF'; x = 4572000; y = 1828800; cx = 457200; cy = 457200 }
    )
} 'pptx-scheme-rect-apply' | Out-Null
Set-PartText $deck 'ppt/slides/slide1.xml' {
    param($text)
    $fill = '<a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill>'
    foreach ($scheme in @('accent1', 'dk1', 'bg1')) {
        $at = $text.IndexOf($fill)
        if ($at -lt 0) { throw "找不到 $scheme 要换的填充" }
        $text = $text.Remove($at, $fill.Length).Insert($at, "<a:solidFill><a:schemeClr val=`"$scheme`"/></a:solidFill>")
    }
    $text
}

$lib = Get-OutPath 'pptx-scheme-rect-lib.png'
if (Test-Path $lib) { Remove-Item -Force $lib }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSlideImage'
    path = $deck.Replace('\', '/'); format = 'pptx'
    id = 'slide:1'
    output = $lib.Replace('\', '/')
} 'pptx-scheme-rect-export' | Out-Null

$ppt = Ensure-MsPpt $deck
$pres = $null
$slide = $null
$gold = Get-OutPath 'pptx-scheme-rect-gold.png'
try {
    $pres = Open-PptPres $ppt $deck
    $cap = Get-PptCaption $pres
    $slide = Get-PptSlide $pres 1
    if (Test-Path $gold) { Remove-Item -Force $gold }
    $slide.Export($gold, 'PNG')
    $ok = $cap -notmatch 'Repair'
    Add-Check 'pptx-scheme-rect-open' $ok "caption=$cap"
}
finally {
    Release-Com $slide
    Close-PptPres $pres
}
Close-OfficeApps -Quit
if ($global:OoxpPptPid) { Stop-Process -Id $global:OoxpPptPid -Force -ErrorAction SilentlyContinue }

$diff = Get-OutPath 'pptx-scheme-rect-diff.png'
$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
$code = $LASTEXITCODE
$text = ($line | Out-String).Trim()
Write-Log "pptx-scheme-rect-px exit=$code $text"
$pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'pptx-scheme-rect-px' $pxOk "exit=$code $text"
