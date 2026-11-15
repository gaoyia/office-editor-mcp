# 没有颜色变换的主题色页底。金样是 PowerPoint 16.0 的 Slide.Export。
# 红区回绘制引擎修，不要加大 MaxDelta。

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

function Replace-Part([string]$path, [string]$entryName, [string]$from, [string]$to) {
    $edit = {
        param($text)
        if ($text.IndexOf($from) -lt 0) { throw "在 $entryName 里找不到要换的片段" }
        $text.Replace($from, $to)
    }.GetNewClosure()
    Set-PartText $path $entryName $edit
}

if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'pptx-theme-bg-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'pptx-theme-bg-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-theme-bg-inspect'
$titleId = [string]$tree.slides[0].shapes[0].id

$base = Get-OutPath 'pptx-theme-bg-base.pptx'
if (Test-Path $base) { Remove-Item -Force $base }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $base.Replace('\', '/')
    ops = @(@{ op = 'deleteShape'; id = $titleId })
} 'pptx-theme-bg-blank' | Out-Null

function New-Variant([string]$name) {
    $dest = Get-OutPath "pptx-theme-bg-$name.pptx"
    if (Test-Path $dest) { Remove-Item -Force $dest }
    Copy-Item $base $dest
    return $dest
}

$slide = 'ppt/slides/slide1.xml'
$master = 'ppt/slideMasters/slideMaster1.xml'
$layout = 'ppt/slideLayouts/slideLayout1.xml'
$theme = 'ppt/theme/theme1.xml'
$masterBg = '<p:bgRef idx="1001"><a:schemeClr val="bg1"/></p:bgRef>'
$masterAccent = '<p:bgRef idx="1001"><a:schemeClr val="accent1"/></p:bgRef>'
$follow = '<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>'

function New-Override([string]$bg1) {
    return '<p:clrMapOvr><a:overrideClrMapping bg1="' + $bg1 + '" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/></p:clrMapOvr>'
}

$custom = New-Variant 'custom'
Replace-Part $custom $slide '<p:cSld>' '<p:cSld><p:bg><p:bgPr><a:solidFill><a:schemeClr val="accent1"/></a:solidFill><a:effectLst/></p:bgPr></p:bg>'
Replace-Part $custom $theme '<a:accent1><a:srgbClr val="4F81BD"/></a:accent1>' '<a:accent1><a:srgbClr val="EE4455"/></a:accent1>'

$ref = New-Variant 'ref'
Replace-Part $ref $slide '<p:cSld>' '<p:cSld><p:bg><p:bgRef idx="1001"><a:schemeClr val="accent1"/></p:bgRef></p:bg>'

$refRgb = New-Variant 'ref-rgb'
Replace-Part $refRgb $slide '<p:cSld>' '<p:cSld><p:bg><p:bgRef idx="1001"><a:srgbClr val="112233"/></p:bgRef></p:bg>'

$inherit = New-Variant 'inherit'
Replace-Part $inherit $master $masterBg $masterAccent

$hidden = New-Variant 'hidden'
Replace-Part $hidden $slide '<p:cSld>' '<p:cSld showMasterSp="0">'
Replace-Part $hidden $master $masterBg $masterAccent

$layoutDeck = New-Variant 'layout'
Replace-Part $layoutDeck $layout '<p:cSld name="空白">' '<p:cSld name="空白"><p:bg><p:bgPr><a:solidFill><a:schemeClr val="accent2"/></a:solidFill><a:effectLst/></p:bgPr></p:bg>'

$remap = New-Variant 'remap'
Replace-Part $remap $slide '<p:cSld>' '<p:cSld><p:bg><p:bgPr><a:solidFill><a:schemeClr val="bg1"/></a:solidFill><a:effectLst/></p:bgPr></p:bg>'
Replace-Part $remap $slide $follow (New-Override 'accent2')

$layoutMap = New-Variant 'layout-map'
Replace-Part $layoutMap $layout '<p:cSld name="空白">' '<p:cSld name="空白"><p:bg><p:bgPr><a:solidFill><a:schemeClr val="bg1"/></a:solidFill><a:effectLst/></p:bgPr></p:bg>'
Replace-Part $layoutMap $layout $follow (New-Override 'accent2')

$inheritMap = New-Variant 'inherit-map'
Replace-Part $inheritMap $slide $follow (New-Override 'accent1')

$idx0 = New-Variant 'idx0'
Replace-Part $idx0 $slide '<p:cSld>' '<p:cSld><p:bg><p:bgRef idx="0"><a:schemeClr val="accent1"/></p:bgRef></p:bg>'
Replace-Part $idx0 $master $masterBg $masterAccent

$wins = New-Variant 'wins'
Replace-Part $wins $slide '<p:cSld>' '<p:cSld><p:bg><p:bgPr><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill><a:effectLst/></p:bgPr></p:bg>'
Replace-Part $wins $master $masterBg $masterAccent

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

$ppt = Ensure-MsPpt $custom
$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))

function Compare-Slide([string]$deckPath, [string]$libPng, [string]$goldName, [string]$diffName, [string]$checkId) {
    $pres = $null
    $slideObj = $null
    $gold = Get-OutPath $goldName
    try {
        $pres = Open-PptPres $ppt $deckPath
        $cap = Get-PptCaption $pres
        $slideObj = Get-PptSlide $pres 1
        if (Test-Path $gold) { Remove-Item -Force $gold }
        $slideObj.Export($gold, 'PNG')
        $ok = $cap -notmatch 'Repair'
        Add-Check ($checkId + '-open') $ok "caption=$cap"
    }
    finally {
        Release-Com $slideObj
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

$pairs = @(
    @{ Name = 'custom'; Deck = $custom }
    @{ Name = 'ref'; Deck = $ref }
    @{ Name = 'ref-rgb'; Deck = $refRgb }
    @{ Name = 'inherit'; Deck = $inherit }
    @{ Name = 'hidden'; Deck = $hidden }
    @{ Name = 'layout'; Deck = $layoutDeck }
    @{ Name = 'remap'; Deck = $remap }
    @{ Name = 'layout-map'; Deck = $layoutMap }
    @{ Name = 'inherit-map'; Deck = $inheritMap }
    @{ Name = 'idx0'; Deck = $idx0 }
    @{ Name = 'wins'; Deck = $wins }
)
foreach ($pair in $pairs) {
    $name = $pair.Name
    $deck = $pair.Deck
    $lib = Export-Lib $deck "pptx-theme-bg-$name-lib.png" "pptx-theme-bg-$name-export"
    Compare-Slide $deck $lib "pptx-theme-bg-$name-gold.png" "pptx-theme-bg-$name-diff.png" "pptx-theme-bg-$name"
}
