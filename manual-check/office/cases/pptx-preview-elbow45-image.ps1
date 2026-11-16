# 直角折线连接符 bentConnector4 / bentConnector5。金样是 PowerPoint 16.0 的 Slide.Export。
# 不重做已差 0 的 bentConnector2 / bentConnector3。红区回绘制引擎修，不要加大 MaxDelta。
# 覆盖多种尺寸、方向、adj、翻转和 1 / 2 像素线宽。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'pptx-preview-elbow45-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'create'
    path          = $src.Replace('\', '/')
    presentation  = @{
        slides = @(
            @{ connectors = @(@{ prst = 'bentConnector4'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'bentConnector4'; x = 914400; y = 914400; cx = 1828800; cy = 762000 }) }
            @{ connectors = @(@{ prst = 'bentConnector4'; x = 914400; y = 914400; cx = 762000; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'bentConnector4'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'bentConnector4'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'bentConnector4'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'bentConnector4'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'bentConnector4'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'bentConnector4'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'bentConnector4'; x = 914400; y = 914400; cx = 771525; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'bentConnector4'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'bentConnector4'; x = 457200; y = 609600; cx = 1828800; cy = 762000 }) }
            @{ connectors = @(@{ prst = 'bentConnector5'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'bentConnector5'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'bentConnector5'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'bentConnector5'; x = 914400; y = 914400; cx = 771525; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'bentConnector5'; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ connectors = @(@{ prst = 'bentConnector5'; x = 457200; y = 609600; cx = 1828800; cy = 762000 }) }
        )
    }
} 'pptx-preview-elbow45-create' | Out-Null

function Set-SlideElbow45([string]$path, [string]$part, [string]$w, [string]$flip, [string]$adj) {
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
            $parts = $adj.Split(',')
            $a1 = $parts[0]
            $a2 = $parts[1]
            $a3 = if ($parts.Count -ge 3) { $parts[2] } else { '' }
            if ($a3 -ne '') {
                $text = $text.Replace(
                    '<a:prstGeom prst="bentConnector5"><a:avLst/>',
                    ('<a:prstGeom prst="bentConnector5"><a:avLst><a:gd name="adj1" fmla="val {0}"/><a:gd name="adj2" fmla="val {1}"/><a:gd name="adj3" fmla="val {2}"/></a:avLst>' -f $a1, $a2, $a3)
                )
            }
            else {
                $text = $text.Replace(
                    '<a:prstGeom prst="bentConnector4"><a:avLst/>',
                    ('<a:prstGeom prst="bentConnector4"><a:avLst><a:gd name="adj1" fmla="val {0}"/><a:gd name="adj2" fmla="val {1}"/></a:avLst>' -f $a1, $a2)
                )
            }
        }
        $entry.Delete()
        $created = $zip.CreateEntry($part)
        $writer = New-Object System.IO.StreamWriter($created.Open(), $utf8)
        try { $writer.Write($text) } finally { $writer.Dispose() }
    }
    finally { $zip.Dispose() }
}

$deck = Get-OutPath 'pptx-preview-elbow45.pptx'
Copy-Item -Force $src $deck
Set-SlideElbow45 $deck 'ppt/slides/slide1.xml'  '9525'  ''   ''
Set-SlideElbow45 $deck 'ppt/slides/slide2.xml'  '9525'  ''   ''
Set-SlideElbow45 $deck 'ppt/slides/slide3.xml'  '9525'  ''   ''
Set-SlideElbow45 $deck 'ppt/slides/slide4.xml'  '9525'  ''   '25000,50000'
Set-SlideElbow45 $deck 'ppt/slides/slide5.xml'  '9525'  ''   '50000,25000'
Set-SlideElbow45 $deck 'ppt/slides/slide6.xml'  '9525'  ''   '75000,75000'
Set-SlideElbow45 $deck 'ppt/slides/slide7.xml'  '9525'  ''   '30000,30000'
Set-SlideElbow45 $deck 'ppt/slides/slide8.xml'  '9525'  'H'  '25000,50000'
Set-SlideElbow45 $deck 'ppt/slides/slide9.xml'  '9525'  'V'  '50000,25000'
Set-SlideElbow45 $deck 'ppt/slides/slide10.xml' '9525'  ''   '30000,40000'
Set-SlideElbow45 $deck 'ppt/slides/slide11.xml' '19050' ''   ''
Set-SlideElbow45 $deck 'ppt/slides/slide12.xml' '19050' 'H'  '25000,75000'
Set-SlideElbow45 $deck 'ppt/slides/slide13.xml' '9525'  ''   ''
Set-SlideElbow45 $deck 'ppt/slides/slide14.xml' '9525'  ''   '25000,50000,75000'
Set-SlideElbow45 $deck 'ppt/slides/slide15.xml' '9525'  'HV' '25000,50000,75000'
Set-SlideElbow45 $deck 'ppt/slides/slide16.xml' '9525'  ''   '30000,30000,70000'
Set-SlideElbow45 $deck 'ppt/slides/slide17.xml' '19050' ''   ''
Set-SlideElbow45 $deck 'ppt/slides/slide18.xml' '19050' 'V'  '75000,25000,40000'

$ppt = Ensure-MsPpt $deck
$diffScript = [IO.Path]::Combine($PSScriptRoot, '..', '..', 'pixel-diff.ps1')
$cases = @(
    @{ tag = 'b4-sq-1'; id = 'slide:1'; n = 1; title = 'b4 1px square default' }
    @{ tag = 'b4-wide-1'; id = 'slide:2'; n = 2; title = 'b4 1px wide default' }
    @{ tag = 'b4-tall-1'; id = 'slide:3'; n = 3; title = 'b4 1px tall default' }
    @{ tag = 'b4-adj25-50'; id = 'slide:4'; n = 4; title = 'b4 1px adj 25/50' }
    @{ tag = 'b4-adj50-25'; id = 'slide:5'; n = 5; title = 'b4 1px adj 50/25' }
    @{ tag = 'b4-adj75-75'; id = 'slide:6'; n = 6; title = 'b4 1px adj 75/75' }
    @{ tag = 'b4-adj30-30'; id = 'slide:7'; n = 7; title = 'b4 1px subpixel adj' }
    @{ tag = 'b4-flipH'; id = 'slide:8'; n = 8; title = 'b4 1px adj 25/50 flipH' }
    @{ tag = 'b4-flipV'; id = 'slide:9'; n = 9; title = 'b4 1px adj 50/25 flipV' }
    @{ tag = 'b4-odd-1'; id = 'slide:10'; n = 10; title = 'b4 1px odd width subpixel adj' }
    @{ tag = 'b4-sq-2'; id = 'slide:11'; n = 11; title = 'b4 2px square default' }
    @{ tag = 'b4-mix-2'; id = 'slide:12'; n = 12; title = 'b4 2px adj 25/75 flipH other origin' }
    @{ tag = 'b5-sq-1'; id = 'slide:13'; n = 13; title = 'b5 1px square default' }
    @{ tag = 'b5-adj-spread'; id = 'slide:14'; n = 14; title = 'b5 1px adj 25/50/75' }
    @{ tag = 'b5-flipHV'; id = 'slide:15'; n = 15; title = 'b5 1px adj 25/50/75 flipHV' }
    @{ tag = 'b5-odd-1'; id = 'slide:16'; n = 16; title = 'b5 1px odd width subpixel adj' }
    @{ tag = 'b5-sq-2'; id = 'slide:17'; n = 17; title = 'b5 2px square default' }
    @{ tag = 'b5-mix-2'; id = 'slide:18'; n = 18; title = 'b5 2px adj 75/25/40 flipV other origin' }
)
foreach ($case in $cases) {
    $lib = Get-OutPath ("pptx-preview-elbow45-{0}-lib.png" -f $case.tag)
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'exportSlideImage'
        path          = $deck.Replace('\', '/')
        format        = 'pptx'
        id            = $case.id
        output        = $lib.Replace('\', '/')
    } ("pptx-preview-elbow45-{0}-export" -f $case.tag) | Out-Null

    $gold = Get-OutPath ("pptx-preview-elbow45-{0}-gold.png" -f $case.tag)
    $pres = $null
    $slide = $null
    try {
        $pres = Open-PptPres $ppt $deck
        $cap = Get-PptCaption $pres
        $slide = Get-PptSlide $pres $case.n
        if (Test-Path $gold) { Remove-Item -Force $gold }
        $slide.Export($gold, 'PNG')
        $ok = $cap -notmatch 'Repair'
        Add-Check ("pptx-preview-elbow45-{0}-open" -f $case.tag) $ok "caption=$cap $($case.title)"
    }
    finally {
        Release-Com $slide
        Close-PptPres $pres
    }

    $diff = Get-OutPath ("pptx-preview-elbow45-{0}-diff.png" -f $case.tag)
    $line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
    $code = $LASTEXITCODE
    $text = ($line | Out-String).Trim()
    Write-Log "pptx-preview-elbow45-$($case.tag)-px exit=$code $text"
    $pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
    Add-Check ("pptx-preview-elbow45-{0}-px" -f $case.tag) $pxOk "exit=$code $text"
}
