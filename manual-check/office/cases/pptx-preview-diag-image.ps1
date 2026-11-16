# 角到角的不透明实线斜线。金样是 PowerPoint 16.0 的 Slide.Export。
# 红区回绘制引擎修，不要加大 MaxDelta。覆盖多种斜率、翻转方向和 1/2 像素线宽。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'pptx-preview-diag-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'create'
    path          = $src.Replace('\', '/')
    presentation  = @{
        slides = @(
            @{ connectors = @(@{ prst = 'line'; x = 914400; y = 914400; cx = 1828800; cy = 762000 }) }
            @{ connectors = @(@{ prst = 'line'; x = 914400; y = 914400; cx = 1828800; cy = 762000 }) }
            @{ connectors = @(@{ prst = 'line'; x = 914400; y = 914400; cx = 1828800; cy = 762000 }) }
            @{ connectors = @(@{ prst = 'line'; x = 914400; y = 914400; cx = 1828800; cy = 762000 }) }
            @{ connectors = @(@{ prst = 'line'; x = 914400; y = 914400; cx = 1828800; cy = 762000 }) }
            @{ connectors = @(@{ prst = 'line'; x = 914400; y = 914400; cx = 1828800; cy = 762000 }) }
            @{ connectors = @(@{ prst = 'line'; x = 914400; y = 914400; cx = 1828800; cy = 762000 }) }
            @{ connectors = @(@{ prst = 'line'; x = 914400; y = 914400; cx = 1828800; cy = 762000 }) }
            @{ connectors = @(@{ prst = 'straightConnector1'; x = 914400; y = 914400; cx = 1828800; cy = 762000 }) }
        )
    }
} 'pptx-preview-diag-create' | Out-Null

function Set-SlideDiag([string]$path, [string]$part, [string]$cx, [string]$cy, [string]$w, [string]$flip) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [System.IO.Compression.ZipFile]::Open($path, [System.IO.Compression.ZipArchiveMode]::Update)
    try {
        $entry = $zip.GetEntry($part)
        $utf8 = New-Object System.Text.UTF8Encoding $false
        $reader = New-Object System.IO.StreamReader($entry.Open(), $utf8)
        try { $text = $reader.ReadToEnd() } finally { $reader.Dispose() }
        $text = $text.Replace('cx="1828800"', ('cx="{0}"' -f $cx))
        $text = $text.Replace('cy="762000"', ('cy="{0}"' -f $cy))
        $text = $text.Replace('w="12700"', ('w="{0}"' -f $w))
        $needle = '<a:xfrm><a:off x="914400"'
        if ($flip -eq 'H') { $text = $text.Replace($needle, '<a:xfrm flipH="1"><a:off x="914400"') }
        elseif ($flip -eq 'V') { $text = $text.Replace($needle, '<a:xfrm flipV="1"><a:off x="914400"') }
        $entry.Delete()
        $created = $zip.CreateEntry($part)
        $writer = New-Object System.IO.StreamWriter($created.Open(), $utf8)
        try { $writer.Write($text) } finally { $writer.Dispose() }
    }
    finally { $zip.Dispose() }
}

$deck = Get-OutPath 'pptx-preview-diag.pptx'
Copy-Item -Force $src $deck
Set-SlideDiag $deck 'ppt/slides/slide1.xml' '1828800' '1828800' '9525' ''
Set-SlideDiag $deck 'ppt/slides/slide2.xml' '1828800' '457200'  '9525' ''
Set-SlideDiag $deck 'ppt/slides/slide3.xml' '457200'  '1828800' '9525' ''
Set-SlideDiag $deck 'ppt/slides/slide4.xml' '1828800' '762000'  '9525' ''
Set-SlideDiag $deck 'ppt/slides/slide5.xml' '1828800' '1828800' '9525' 'H'
Set-SlideDiag $deck 'ppt/slides/slide6.xml' '1828800' '1828800' '9525' 'V'
Set-SlideDiag $deck 'ppt/slides/slide7.xml' '1828800' '1828800' '19050' ''
Set-SlideDiag $deck 'ppt/slides/slide8.xml' '1828800' '457200'  '19050' ''
Set-SlideDiag $deck 'ppt/slides/slide9.xml' '1828800' '1828800' '9525' ''

$ppt = Ensure-MsPpt $deck
$diffScript = [IO.Path]::Combine($PSScriptRoot, '..', '..', 'pixel-diff.ps1')
$cases = @(
    @{ tag = 'd45-1'; id = 'slide:1'; n = 1; title = '1px 45deg' }
    @{ tag = 'd14-1'; id = 'slide:2'; n = 2; title = '1px 1:4' }
    @{ tag = 'd41-1'; id = 'slide:3'; n = 3; title = '1px 4:1' }
    @{ tag = 'd512-1'; id = 'slide:4'; n = 4; title = '1px 5:12' }
    @{ tag = 'd45-flipH'; id = 'slide:5'; n = 5; title = '1px 45deg flipH' }
    @{ tag = 'd45-flipV'; id = 'slide:6'; n = 6; title = '1px 45deg flipV' }
    @{ tag = 'd45-2'; id = 'slide:7'; n = 7; title = '2px 45deg' }
    @{ tag = 'd14-2'; id = 'slide:8'; n = 8; title = '2px 1:4' }
    @{ tag = 'd45-cxn'; id = 'slide:9'; n = 9; title = '1px 45deg connector' }
)
foreach ($case in $cases) {
    $lib = Get-OutPath ("pptx-preview-diag-{0}-lib.png" -f $case.tag)
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'exportSlideImage'
        path          = $deck.Replace('\', '/')
        format        = 'pptx'
        id            = $case.id
        output        = $lib.Replace('\', '/')
    } ("pptx-preview-diag-{0}-export" -f $case.tag) | Out-Null

    $gold = Get-OutPath ("pptx-preview-diag-{0}-gold.png" -f $case.tag)
    $pres = $null
    $slide = $null
    try {
        $pres = Open-PptPres $ppt $deck
        $cap = Get-PptCaption $pres
        $slide = Get-PptSlide $pres $case.n
        if (Test-Path $gold) { Remove-Item -Force $gold }
        $slide.Export($gold, 'PNG')
        $ok = $cap -notmatch 'Repair'
        Add-Check ("pptx-preview-diag-{0}-open" -f $case.tag) $ok "caption=$cap $($case.title)"
    }
    finally {
        Release-Com $slide
        Close-PptPres $pres
    }

    $diff = Get-OutPath ("pptx-preview-diag-{0}-diff.png" -f $case.tag)
    $line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
    $code = $LASTEXITCODE
    $text = ($line | Out-String).Trim()
    Write-Log "pptx-preview-diag-$($case.tag)-px exit=$code $text"
    $pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
    Add-Check ("pptx-preview-diag-{0}-px" -f $case.tag) $pxOk "exit=$code $text"
}