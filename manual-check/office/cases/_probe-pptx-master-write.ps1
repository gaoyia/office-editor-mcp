# 母版写入剩余第 0 步：新建版式、插占位符、第二份母版另存。不进 run.ps1。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
Initialize-Suite -Name 'office'

Add-Type -AssemblyName System.IO.Compression.FileSystem

function Get-ZipEntryText([string]$zipPath, [string]$entryName) {
    $zip = [IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        $e = $zip.GetEntry($entryName)
        if ($null -eq $e) { return $null }
        $sr = New-Object IO.StreamReader($e.Open(), [Text.Encoding]::UTF8)
        try { return $sr.ReadToEnd() }
        finally { $sr.Dispose() }
    }
    finally { $zip.Dispose() }
}

function Get-ZipNames([string]$zipPath) {
    $zip = [IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        return @($zip.Entries | ForEach-Object { $_.FullName })
    }
    finally { $zip.Dispose() }
}

function Snip-Ph([string]$xml) {
    $ms = [regex]::Matches($xml, '(?s)<p:sp\b.*?</p:sp>')
    $out = New-Object System.Collections.Generic.List[string]
    foreach ($m in $ms) {
        $sp = $m.Value
        $ph = [regex]::Match($sp, '<p:ph([^>]*)/?>')
        if (-not $ph.Success) { continue }
        $id = [regex]::Match($sp, '<p:cNvPr id="(\d+)" name="([^"]*)"').Groups
        $prst = [regex]::Match($sp, 'prstGeom prst="([^"]+)"').Groups[1].Value
        $out.Add("id=$($id[1].Value) name=$($id[2].Value) ph=$($ph.Groups[1].Value.Trim()) prst=$prst")
    }
    if ($out.Count -eq 0) { return 'no ph' }
    return ($out -join ' | ')
}

function Save-Copy($pres, [string]$pptxOut) {
    $pres.Save()
    Copy-Item -Force $pres.FullName $pptxOut
}

$report = Get-OutPath 'pptx-master-write-probe.txt'
$hostPath = Ensure-PptxCreate
$tmpDir = Join-Path $env:TEMP 'ooxp-office'
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
$tmp = Join-Path $tmpDir 'pptx-master-write-probe.pptx'
Copy-Item -Force $hostPath $tmp

$ppt = Ensure-MsPpt $hostPath
Close-PptPresByPath $ppt $tmp
$pres = Open-PptPres $ppt $tmp
$lines = New-Object System.Collections.Generic.List[string]
$cap0 = Get-PptCaption $pres
$lines.Add("open cap=$cap0")

$master = $pres.SlideMaster
$layouts = $master.CustomLayouts
$n0 = [int]$layouts.Count
$lines.Add("layoutCount=$n0")
for ($i = 1; $i -le $n0; $i++) {
    $cl = $layouts.Item($i)
    $nm = [string]$cl.Name
    $typ = $null
    try { $typ = [int]$cl.Type } catch { $typ = 'na' }
    $lines.Add("layout[$i] name=$nm type=$typ")
    Release-Com $cl
}

$slideXml0 = Get-ZipEntryText $tmp 'ppt/slides/slide1.xml'
$lines.Add("slide1 before len=$($slideXml0.Length)")

# 复制最后一份版式。
$last = $layouts.Item($n0)
$dup = $null
try {
    $dup = $last.Duplicate()
    $lines.Add("duplicate ok name=$([string]$dup.Name) count=$([int]$layouts.Count)")
}
catch {
    $lines.Add("duplicate fail=$($_.Exception.Message)")
}
Release-Com $last

# 往新版式插正文占位符。ppPlaceholderBody = 2
if ($null -ne $dup) {
    $sh = $null
    try {
        $sh = $dup.Shapes
        $ph = $sh.AddPlaceholder(2, 50, 80, 400, 200)
        $lines.Add("addPlaceholder body Type=$([int]$ph.Type) AutoShapeType=$([int]$ph.AutoShapeType) HasTextFrame=$([int]$ph.HasTextFrame)")
        Release-Com $ph
    }
    catch {
        $lines.Add("addPlaceholder fail=$($_.Exception.Message)")
    }
    finally { Release-Com $sh }
}

$outDup = Get-OutPath 'pptx-mw-layout.pptx'
Save-Copy $pres $outDup
$capD = Get-PptCaption $pres
$lines.Add("after-layout cap=$capD")
$names = Get-ZipNames $outDup
$lines.Add("parts layouts=" + (($names | Where-Object { $_ -like 'ppt/slideLayouts/*' -and $_ -notlike '*_rels*' }) -join ','))
$lines.Add("parts masters=" + (($names | Where-Object { $_ -like 'ppt/slideMasters/*' -and $_ -notlike '*_rels*' }) -join ','))
$lines.Add("parts theme=" + (($names | Where-Object { $_ -like 'ppt/theme/*' }) -join ','))
$masterRels = Get-ZipEntryText $outDup 'ppt/slideMasters/_rels/slideMaster1.xml.rels'
$lines.Add("master1.rels=$masterRels")
$layoutXmls = @($names | Where-Object { $_ -match '^ppt/slideLayouts/slideLayout\d+\.xml$' })
foreach ($p in $layoutXmls) {
    $xml = Get-ZipEntryText $outDup $p
    $type = [regex]::Match($xml, '<p:sldLayout[^>]*type="([^"]+)"').Groups[1].Value
    $name = [regex]::Match($xml, '<p:cSld name="([^"]+)"').Groups[1].Value
    $preserve = [regex]::Match($xml, 'preserve="([^"]+)"').Groups[1].Value
    $clr = $xml -match '<p:clrMapOvr'
    $lines.Add("layout-xml $p type=$type name=$name preserve=$preserve clrMapOvr=$clr ph=$(Snip-Ph $xml)")
}
$slideXml1 = Get-ZipEntryText $outDup 'ppt/slides/slide1.xml'
$lines.Add("slide1 after-layout same=$($slideXml1 -eq $slideXml0) len=$($slideXml1.Length)")
$ct = Get-ZipEntryText $outDup '[Content_Types].xml'
$layoutOverrides = [regex]::Matches($ct, 'PartName="(/ppt/slideLayouts/[^"]+)"').Groups | Where-Object { $_.Name -eq 1 } | ForEach-Object { $_.Value }
$lines.Add("contentTypes layouts=$($layoutOverrides -join ',')")

# 第二份母版。Designs.Add 需要模板路径；试 COM。
$designs = $null
try {
    $designs = $pres.Designs
    $nDes0 = [int]$designs.Count
    $lines.Add("designCount=$nDes0")
    $added = $null
    try {
        $added = $designs.Add()
        $lines.Add("designs.Add ok count=$([int]$designs.Count) name=$([string]$added.Name)")
    }
    catch {
        $lines.Add("designs.Add fail=$($_.Exception.Message)")
        try {
            $added = $designs.Load($hostPath)
            $lines.Add("designs.Load ok count=$([int]$designs.Count) name=$([string]$added.Name)")
        }
        catch {
            $lines.Add("designs.Load fail=$($_.Exception.Message)")
        }
    }
    if ($null -ne $added) { Release-Com $added }
}
catch {
    $lines.Add("designs fail=$($_.Exception.Message)")
}
finally { Release-Com $designs }

$outMaster = Get-OutPath 'pptx-mw-master.pptx'
Save-Copy $pres $outMaster
$capM = Get-PptCaption $pres
$lines.Add("after-master cap=$capM")
$namesM = Get-ZipNames $outMaster
$lines.Add("master parts masters=" + (($namesM | Where-Object { $_ -like 'ppt/slideMasters/*' -and $_ -notlike '*_rels*' }) -join ','))
$lines.Add("master parts layouts=" + (($namesM | Where-Object { $_ -like 'ppt/slideLayouts/*' -and $_ -notlike '*_rels*' }) -join ','))
$lines.Add("master parts theme=" + (($namesM | Where-Object { $_ -like 'ppt/theme/*' }) -join ','))
$presXml = Get-ZipEntryText $outMaster 'ppt/presentation.xml'
$sldMasterIds = [regex]::Matches($presXml, '<p:sldMasterId ').Count
$lines.Add("presentation sldMasterId count=$sldMasterIds")
$presRels = Get-ZipEntryText $outMaster 'ppt/_rels/presentation.xml.rels'
$lines.Add("presentation.rels slideMaster=" + ([regex]::Matches($presRels, 'slideMaster\d+').Count))

# 删刚插的占位符：找新版式上 Type=14（msoPlaceholder）再 Delete。
if ($null -ne $dup) {
    try {
        $sh = $dup.Shapes
        $nSh = [int]$sh.Count
        $lines.Add("dup shapes=$nSh")
        for ($i = $nSh; $i -ge 1; $i--) {
            $s = $sh.Item($i)
            $ty = [int]$s.Type
            $lines.Add("dup shape[$i] Type=$ty Name=$([string]$s.Name)")
            if ($ty -eq 14) {
                $s.Delete()
                $lines.Add("deleted placeholder i=$i")
            }
            else { Release-Com $s }
        }
        Release-Com $sh
    }
    catch {
        $lines.Add("delete placeholder fail=$($_.Exception.Message)")
    }
}

$outDel = Get-OutPath 'pptx-mw-delph.pptx'
Save-Copy $pres $outDel
$capX = Get-PptCaption $pres
$lines.Add("after-delph cap=$capX")
$layoutXmls2 = @(Get-ZipNames $outDel | Where-Object { $_ -match '^ppt/slideLayouts/slideLayout\d+\.xml$' })
foreach ($p in $layoutXmls2) {
    $xml = Get-ZipEntryText $outDel $p
    $lines.Add("after-del $p ph=$(Snip-Ph $xml)")
}

Release-Com $dup
Release-Com $layouts
Release-Com $master
Close-PptPres $pres
[IO.File]::WriteAllLines($report, $lines, $global:OoxpUtf8)
Write-Log "wrote $report"
$lines | ForEach-Object { Write-Host $_ }
