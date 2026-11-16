# 弯曲连接符（curvedConnector2 / curvedConnector3）。金样是 PowerPoint 16.0 的 Slide.Export。
# 红区回绘制引擎修，不要加大 MaxDelta。覆盖多种方向、控制点、线宽和位置，避免只对一个样例。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'pptx-preview-curve-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'create'
    path          = $src.Replace('\', '/')
    presentation  = @{
        slides = @(
            @{ connectors = @(@{ prst = 'curvedConnector2'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'curvedConnector2'; x = 914400; y = 914400; cx = 1828800; cy = 762000 }) }
            @{ connectors = @(@{ prst = 'curvedConnector2'; x = 914400; y = 914400; cx = 762000; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'curvedConnector2'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'curvedConnector2'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'curvedConnector2'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'curvedConnector2'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'curvedConnector2'; x = 457200; y = 609600; cx = 1828800; cy = 762000 }) }
            @{ connectors = @(@{ prst = 'curvedConnector3'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'curvedConnector3'; x = 914400; y = 914400; cx = 1828800; cy = 762000 }) }
            @{ connectors = @(@{ prst = 'curvedConnector3'; x = 914400; y = 914400; cx = 762000; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'curvedConnector3'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'curvedConnector3'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'curvedConnector3'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'curvedConnector3'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'curvedConnector3'; x = 914400; y = 914400; cx = 771525; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'curvedConnector3'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'curvedConnector3'; x = 457200; y = 609600; cx = 1828800; cy = 762000 }) }
        )
    }
} 'pptx-preview-curve-create' | Out-Null

function Set-SlideCurve([string]$path, [string]$part, [string]$w, [string]$flip, [string]$adj) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [System.IO.Compression.ZipFile]::Open($path, [System.IO.Compression.ZipArchiveMode]::Update)
    try {
        $entry = $zip.GetEntry($part)
        $utf8 = New-Object System.Text.UTF8Encoding $false
        $reader = New-Object System.IO.StreamReader($entry.Open(), $utf8)
        try { $text = $reader.ReadToEnd() } finally { $reader.Dispose() }
        $text = $text.Replace('w="12700"', ('w="{0}"' -f $w))
        if ($flip -eq 'H') {
            $text = $text.Replace('<a:xfrm><a:off x="914400"', '<a:xfrm flipH="1"><a:off x="914400"')
            $text = $text.Replace('<a:xfrm><a:off x="457200"', '<a:xfrm flipH="1"><a:off x="457200"')
        }
        elseif ($flip -eq 'V') {
            $text = $text.Replace('<a:xfrm><a:off x="914400"', '<a:xfrm flipV="1"><a:off x="914400"')
            $text = $text.Replace('<a:xfrm><a:off x="457200"', '<a:xfrm flipV="1"><a:off x="457200"')
        }
        elseif ($flip -eq 'HV') {
            $text = $text.Replace('<a:xfrm><a:off x="914400"', '<a:xfrm flipH="1" flipV="1"><a:off x="914400"')
            $text = $text.Replace('<a:xfrm><a:off x="457200"', '<a:xfrm flipH="1" flipV="1"><a:off x="457200"')
        }
        if ($adj -ne '') {
            $text = $text.Replace('<a:prstGeom prst="curvedConnector3"><a:avLst/>', ('<a:prstGeom prst="curvedConnector3"><a:avLst><a:gd name="adj1" fmla="val {0}"/></a:avLst>' -f $adj))
        }
        $entry.Delete()
        $created = $zip.CreateEntry($part)
        $writer = New-Object System.IO.StreamWriter($created.Open(), $utf8)
        try { $writer.Write($text) } finally { $writer.Dispose() }
    }
    finally { $zip.Dispose() }
}

$deck = Get-OutPath 'pptx-preview-curve.pptx'
Copy-Item -Force $src $deck
Set-SlideCurve $deck 'ppt/slides/slide1.xml'  '9525'  ''   ''
Set-SlideCurve $deck 'ppt/slides/slide2.xml'  '9525'  ''   ''
Set-SlideCurve $deck 'ppt/slides/slide3.xml'  '9525'  ''   ''
Set-SlideCurve $deck 'ppt/slides/slide4.xml'  '9525'  'H'  ''
Set-SlideCurve $deck 'ppt/slides/slide5.xml'  '9525'  'V'  ''
Set-SlideCurve $deck 'ppt/slides/slide6.xml'  '9525'  'HV' ''
Set-SlideCurve $deck 'ppt/slides/slide7.xml'  '19050' ''   ''
Set-SlideCurve $deck 'ppt/slides/slide8.xml'  '9525'  ''   ''
Set-SlideCurve $deck 'ppt/slides/slide9.xml'  '9525'  ''   ''
Set-SlideCurve $deck 'ppt/slides/slide10.xml' '9525'  ''   ''
Set-SlideCurve $deck 'ppt/slides/slide11.xml' '9525'  ''   ''
Set-SlideCurve $deck 'ppt/slides/slide12.xml' '9525'  ''   '25000'
Set-SlideCurve $deck 'ppt/slides/slide13.xml' '9525'  ''   '75000'
Set-SlideCurve $deck 'ppt/slides/slide14.xml' '9525'  ''   '30000'
Set-SlideCurve $deck 'ppt/slides/slide15.xml' '9525'  'V'  '25000'
Set-SlideCurve $deck 'ppt/slides/slide16.xml' '9525'  ''   '30000'
Set-SlideCurve $deck 'ppt/slides/slide17.xml' '19050' ''   ''
Set-SlideCurve $deck 'ppt/slides/slide18.xml' '19050' 'H'  '25000'

$ppt = Ensure-MsPpt $deck
$diffScript = [IO.Path]::Combine($PSScriptRoot, '..', '..', 'pixel-diff.ps1')
$cases = @(
    @{ tag = 'c2-sq-1'; id = 'slide:1'; n = 1; title = 'c2 1px square' }
    @{ tag = 'c2-wide-1'; id = 'slide:2'; n = 2; title = 'c2 1px wide' }
    @{ tag = 'c2-tall-1'; id = 'slide:3'; n = 3; title = 'c2 1px tall' }
    @{ tag = 'c2-flipH'; id = 'slide:4'; n = 4; title = 'c2 1px flipH' }
    @{ tag = 'c2-flipV'; id = 'slide:5'; n = 5; title = 'c2 1px flipV' }
    @{ tag = 'c2-flipHV'; id = 'slide:6'; n = 6; title = 'c2 1px flipHV' }
    @{ tag = 'c2-sq-2'; id = 'slide:7'; n = 7; title = 'c2 2px square' }
    @{ tag = 'c2-pos-1'; id = 'slide:8'; n = 8; title = 'c2 1px other origin' }
    @{ tag = 'c3-sq-1'; id = 'slide:9'; n = 9; title = 'c3 1px default mid' }
    @{ tag = 'c3-wide-1'; id = 'slide:10'; n = 10; title = 'c3 1px wide' }
    @{ tag = 'c3-tall-1'; id = 'slide:11'; n = 11; title = 'c3 1px tall' }
    @{ tag = 'c3-adj25-1'; id = 'slide:12'; n = 12; title = 'c3 1px adj 25%' }
    @{ tag = 'c3-adj75-1'; id = 'slide:13'; n = 13; title = 'c3 1px adj 75%' }
    @{ tag = 'c3-adj30-1'; id = 'slide:14'; n = 14; title = 'c3 1px subpixel adj' }
    @{ tag = 'c3-adj25-flipV'; id = 'slide:15'; n = 15; title = 'c3 1px adj 25% flipV' }
    @{ tag = 'c3-odd-1'; id = 'slide:16'; n = 16; title = 'c3 1px odd width subpixel adj' }
    @{ tag = 'c3-sq-2'; id = 'slide:17'; n = 17; title = 'c3 2px default mid' }
    @{ tag = 'c3-mix-2'; id = 'slide:18'; n = 18; title = 'c3 2px adj 25% flipH other origin' }
)
foreach ($case in $cases) {
    $lib = Get-OutPath ("pptx-preview-curve-{0}-lib.png" -f $case.tag)
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'exportSlideImage'
        path          = $deck.Replace('\', '/')
        format        = 'pptx'
        id            = $case.id
        output        = $lib.Replace('\', '/')
    } ("pptx-preview-curve-{0}-export" -f $case.tag) | Out-Null

    $gold = Get-OutPath ("pptx-preview-curve-{0}-gold.png" -f $case.tag)
    $pres = $null
    $slide = $null
    try {
        $pres = Open-PptPres $ppt $deck
        $cap = Get-PptCaption $pres
        $slide = Get-PptSlide $pres $case.n
        if (Test-Path $gold) { Remove-Item -Force $gold }
        $slide.Export($gold, 'PNG')
        $ok = $cap -notmatch 'Repair'
        Add-Check ("pptx-preview-curve-{0}-open" -f $case.tag) $ok "caption=$cap $($case.title)"
    }
    finally {
        Release-Com $slide
        Close-PptPres $pres
    }

    $diff = Get-OutPath ("pptx-preview-curve-{0}-diff.png" -f $case.tag)
    $line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
    $code = $LASTEXITCODE
    $text = ($line | Out-String).Trim()
    Write-Log "pptx-preview-curve-$($case.tag)-px exit=$code $text"
    $pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
    Add-Check ("pptx-preview-curve-{0}-px" -f $case.tag) $pxOk "exit=$code $text"
}
