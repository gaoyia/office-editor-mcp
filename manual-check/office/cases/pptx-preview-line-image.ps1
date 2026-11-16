# 水平或垂直的直线 / 直连接线。金样是 PowerPoint 16.0 的 Slide.Export。
# 红区回绘制引擎修，不要加大 MaxDelta。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'pptx-preview-line-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'create'
    path          = $src.Replace('\', '/')
    presentation  = @{
        slides = @(
            @{ connectors = @(@{ prst = 'straightConnector1'; x = 914400; y = 914400; cx = 1828800; cy = 762000 }) }
            @{ connectors = @(@{ prst = 'line'; x = 914400; y = 914400; cx = 9525; cy = 762000 }) }
            @{ connectors = @(@{ prst = 'line'; x = 914400; y = 914400; cx = 1828800; cy = 762000 }) }
        )
    }
} 'pptx-preview-line-create' | Out-Null

function Set-SlideExt([string]$path, [string]$part, [string]$cx, [string]$cy, [string]$w) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [System.IO.Compression.ZipFile]::Open($path, [System.IO.Compression.ZipArchiveMode]::Update)
    try {
        $entry = $zip.GetEntry($part)
        $utf8 = New-Object System.Text.UTF8Encoding $false
        $reader = New-Object System.IO.StreamReader($entry.Open(), $utf8)
        try { $text = $reader.ReadToEnd() } finally { $reader.Dispose() }
        $text = $text.Replace('cx="1828800"', ('cx="{0}"' -f $cx))
        $text = $text.Replace('cx="9525"', ('cx="{0}"' -f $cx))
        $text = $text.Replace('cy="762000"', ('cy="{0}"' -f $cy))
        $text = $text.Replace('w="12700"', ('w="{0}"' -f $w))
        $entry.Delete()
        $created = $zip.CreateEntry($part)
        $writer = New-Object System.IO.StreamWriter($created.Open(), $utf8)
        try { $writer.Write($text) } finally { $writer.Dispose() }
    }
    finally { $zip.Dispose() }
}

$deck = Get-OutPath 'pptx-preview-line.pptx'
Copy-Item -Force $src $deck
Set-SlideExt $deck 'ppt/slides/slide1.xml' '1828800' '0' '19050'
Set-SlideExt $deck 'ppt/slides/slide2.xml' '0' '1828800' '19050'
Set-SlideExt $deck 'ppt/slides/slide3.xml' '1828800' '0' '9525'

$ppt = Ensure-MsPpt $deck
$diffScript = [IO.Path]::Combine($PSScriptRoot, '..', '..', 'pixel-diff.ps1')
$cases = @(
    @{ tag = 'h2'; id = 'slide:1'; n = 1; title = '2px horizontal' }
    @{ tag = 'v2'; id = 'slide:2'; n = 2; title = '2px vertical' }
    @{ tag = 'h1'; id = 'slide:3'; n = 3; title = '1px horizontal' }
)
foreach ($case in $cases) {
    $lib = Get-OutPath ("pptx-preview-line-{0}-lib.png" -f $case.tag)
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'exportSlideImage'
        path          = $deck.Replace('\', '/')
        format        = 'pptx'
        id            = $case.id
        output        = $lib.Replace('\', '/')
    } ("pptx-preview-line-{0}-export" -f $case.tag) | Out-Null

    $gold = Get-OutPath ("pptx-preview-line-{0}-gold.png" -f $case.tag)
    $pres = $null
    $slide = $null
    try {
        $pres = Open-PptPres $ppt $deck
        $cap = Get-PptCaption $pres
        $slide = Get-PptSlide $pres $case.n
        if (Test-Path $gold) { Remove-Item -Force $gold }
        $slide.Export($gold, 'PNG')
        $ok = $cap -notmatch 'Repair'
        Add-Check ("pptx-preview-line-{0}-open" -f $case.tag) $ok "caption=$cap $($case.title)"
    }
    finally {
        Release-Com $slide
        Close-PptPres $pres
    }

    $diff = Get-OutPath ("pptx-preview-line-{0}-diff.png" -f $case.tag)
    $line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
    $code = $LASTEXITCODE
    $text = ($line | Out-String).Trim()
    Write-Log "pptx-preview-line-$($case.tag)-px exit=$code $text"
    $pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
    Add-Check ("pptx-preview-line-{0}-px" -f $case.tag) $pxOk "exit=$code $text"
}
