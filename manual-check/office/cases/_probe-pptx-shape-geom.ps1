# 形状几何剩余第 0 步：椭圆、连接线、组合另存。不进 run.ps1。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
Initialize-Suite -Name 'office'

Add-Type -AssemblyName System.IO.Compression.FileSystem

function Get-ZipEntryText([string]$zipPath, [string]$entryName) {
    $zip = [IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        $e = $zip.GetEntry($entryName)
        if ($null -eq $e) { throw "zip 没有 $entryName" }
        $sr = New-Object IO.StreamReader($e.Open(), [Text.Encoding]::UTF8)
        try { return $sr.ReadToEnd() }
        finally { $sr.Dispose() }
    }
    finally { $zip.Dispose() }
}

function Get-SpBlocks([string]$xml) {
    return [regex]::Matches($xml, '(?s)<p:sp\b.*?</p:sp>')
}

function Get-CxnBlocks([string]$xml) {
    return [regex]::Matches($xml, '(?s)<p:cxnSp\b.*?</p:cxnSp>')
}

function Summarize-Sp([string]$sp) {
    $id = [regex]::Match($sp, '<p:cNvPr id="(\d+)" name="([^"]*)"').Groups
    $prst = [regex]::Match($sp, 'prstGeom prst="([^"]+)"').Groups[1].Value
    $txBox = [regex]::Match($sp, 'cNvSpPr([^>]*)>').Groups[1].Value
    $hasTx = $sp -match '<p:txBody'
    $txBoxAttr = if ($txBox -match 'txBox="1"') { '1' } else { '0' }
    $off = [regex]::Match($sp, '<a:off x="([^"]+)" y="([^"]+)"').Groups
    $ext = [regex]::Match($sp, '<a:ext cx="([^"]+)" cy="([^"]+)"').Groups
    return "id=$($id[1].Value) name=$($id[2].Value) prst=$prst txBox=$txBoxAttr txBody=$hasTx off=$($off[1].Value),$($off[2].Value) ext=$($ext[1].Value),$($ext[2].Value)"
}

function Summarize-Cxn([string]$cxn) {
    $id = [regex]::Match($cxn, '<p:cNvPr id="(\d+)" name="([^"]*)"').Groups
    $prst = [regex]::Match($cxn, 'prstGeom prst="([^"]+)"').Groups[1].Value
    $st = [regex]::Match($cxn, '<a:stCxn([^/]*)/>').Value
    $en = [regex]::Match($cxn, '<a:endCxn([^/]*)/>').Value
    $hasTx = $cxn -match '<p:txBody'
    return "id=$($id[1].Value) name=$($id[2].Value) prst=$prst txBody=$hasTx st=$st end=$en"
}

function Summarize-Grp([string]$xml) {
    $grp = [regex]::Match($xml, '(?s)<p:grpSp\b.*</p:grpSp>').Value
    if (-not $grp) { return 'no grpSp' }
    $id = [regex]::Match($grp, '<p:cNvPr id="(\d+)" name="([^"]*)"').Groups
    $off = [regex]::Match($grp, '<a:off x="([^"]+)" y="([^"]+)"').Groups
    $ext = [regex]::Match($grp, '<a:ext cx="([^"]+)" cy="([^"]+)"').Groups
    $chOff = [regex]::Match($grp, '<a:chOff x="([^"]+)" y="([^"]+)"').Groups
    $chExt = [regex]::Match($grp, '<a:chExt cx="([^"]+)" cy="([^"]+)"').Groups
    $childIds = ([regex]::Matches($grp, '<p:cNvPr id="(\d+)"') | ForEach-Object { $_.Groups[1].Value }) -join ','
    $childPrst = ([regex]::Matches($grp, 'prstGeom prst="([^"]+)"') | ForEach-Object { $_.Groups[1].Value }) -join ','
    return "grpId=$($id[1].Value) name=$($id[2].Value) off=$($off[1].Value),$($off[2].Value) ext=$($ext[1].Value),$($ext[2].Value) chOff=$($chOff[1].Value),$($chOff[2].Value) chExt=$($chExt[1].Value),$($chExt[2].Value) childIds=$childIds childPrst=$childPrst"
}

function Save-Slide1($pres, [string]$pptxOut, [string]$xmlOut) {
    $pres.Save()
    Copy-Item -Force $pres.FullName $pptxOut
    $xml = Get-ZipEntryText $pptxOut 'ppt/slides/slide1.xml'
    [IO.File]::WriteAllText($xmlOut, $xml, $global:OoxpUtf8)
    return $xml
}

$hostPath = Ensure-PptxCreate
$tmpDir = Join-Path $env:TEMP 'ooxp-office'
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
$tmp = Join-Path $tmpDir 'pptx-shape-geom-probe.pptx'
Copy-Item -Force $hostPath $tmp

$ppt = Ensure-MsPpt $hostPath
Close-PptPresByPath $ppt $tmp
$pres = Open-PptPres $ppt $tmp
$s1 = Get-PptSlide $pres 1
$shapes = $s1.Shapes
$lines = New-Object System.Collections.Generic.List[string]

$box1 = Get-PptShape $s1 1
$box2 = Get-PptShape $s1 2
$name1 = [string]$box1.Name
$name2 = [string]$box2.Name
$lines.Add("seed boxes name1=$name1 name2=$name2")
Release-Com $box1
Release-Com $box2

# msoShapeOval = 9；msoShapeRightArrow = 33
$oval = $shapes.AddShape(9, 50, 200, 120, 80)
$lines.Add("oval AutoShapeType=$([int]$oval.AutoShapeType) HasTextFrame=$([int]$oval.HasTextFrame) Type=$([int]$oval.Type)")
Release-Com $oval
$arrow = $shapes.AddShape(33, 200, 200, 120, 50)
$lines.Add("arrow AutoShapeType=$([int]$arrow.AutoShapeType) HasTextFrame=$([int]$arrow.HasTextFrame) Type=$([int]$arrow.Type)")
Release-Com $arrow

$xmlOval = Save-Slide1 $pres (Get-OutPath 'pptx-sg-oval.pptx') (Get-OutPath 'pptx-sg-oval-slide1.xml')
foreach ($m in Get-SpBlocks $xmlOval) { $lines.Add('oval-sp ' + (Summarize-Sp $m.Value)) }
$lines.Add("oval cxn=$($xmlOval -match '<p:cxnSp') grp=$($xmlOval -match '<p:grpSp[\s>]')")

# msoConnectorStraight = 1
$cxn = $shapes.AddConnector(1, 50, 320, 200, 380)
$lines.Add("cxn AutoShapeType=$([int]$cxn.AutoShapeType) Connector=$([int]$cxn.Connector) Type=$([int]$cxn.Type)")
Release-Com $cxn
$xmlCxn = Save-Slide1 $pres (Get-OutPath 'pptx-sg-cxn.pptx') (Get-OutPath 'pptx-sg-cxn-slide1.xml')
foreach ($m in Get-CxnBlocks $xmlCxn) { $lines.Add('cxn ' + (Summarize-Cxn $m.Value)) }

# 组合：原来的两个文本框
$arr = New-Object 'System.Object[]' 2
$arr[0] = $name1
$arr[1] = $name2
$range = $shapes.Range($arr)
$grp = $range.Group()
$lines.Add("group Type=$([int]$grp.Type) Name=$($grp.Name) Count=$([int]$grp.GroupItems.Count)")
Release-Com $grp
Release-Com $range
$xmlGrp = Save-Slide1 $pres (Get-OutPath 'pptx-sg-group.pptx') (Get-OutPath 'pptx-sg-group-slide1.xml')
$lines.Add((Summarize-Grp $xmlGrp))
$ids = [regex]::Matches($xmlGrp, '<p:cNvPr id="(\d+)"') | ForEach-Object { $_.Groups[1].Value }
$lines.Add("cNvPr ids=$($ids -join ',')")
foreach ($m in Get-SpBlocks $xmlGrp) { $lines.Add('grp-sp ' + (Summarize-Sp $m.Value)) }
$cap = Get-PptCaption $pres
$lines.Add("cap=$cap")

Release-Com $shapes
Release-Com $s1
Close-PptPres $pres

# 现有 inspect 怎么看这三份。
foreach ($pair in @(
    @('pptx-sg-oval.pptx', 'pptx-sg-oval-inspect'),
    @('pptx-sg-cxn.pptx', 'pptx-sg-cxn-inspect'),
    @('pptx-sg-group.pptx', 'pptx-sg-group-inspect')
)) {
    $p = Get-OutPath $pair[0]
    $tree = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $p.Replace('\', '/'); format = 'pptx'
        scope = @{ slide = 1 }
    } $pair[1]
    $shapeIds = @($tree.slides[0].shapes | ForEach-Object { [string]$_.id })
    $lines.Add("$($pair[0]) inspect shapes=$($shapeIds -join ',') count=$($tree.slides[0].shapes.Count)")
}

$report = Get-OutPath 'pptx-shape-geom-probe.txt'
[IO.File]::WriteAllLines($report, $lines, $global:OoxpUtf8)
Write-Log ($lines -join "`n")
Close-OfficeApps -Quit
