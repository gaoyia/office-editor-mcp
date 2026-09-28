# 动画写入第 0 步：Office 16.0 进入淡入另存，看最小 timing 树。不进 run.ps1。

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

function Add-PptFade($slide, $shapeKey, [int]$trigger) {
    $box = Get-PptShape $slide $shapeKey
    $tl = $slide.TimeLine
    $seq = $tl.MainSequence
    try {
        $eff = $seq.AddEffect($box, 10, 0, $trigger)
        Release-Com $eff
        return [int]$seq.Count
    }
    finally {
        Release-Com $seq
        Release-Com $tl
        Release-Com $box
    }
}

$hostPath = Ensure-PptxCreate
$one = Get-OutPath 'pptx-anim-write-one.pptx'
$two = Get-OutPath 'pptx-anim-write-two.pptx'
$xmlOne = Get-OutPath 'pptx-anim-write-one-slide1.xml'
$xmlTwo = Get-OutPath 'pptx-anim-write-two-slide1.xml'
$report = Get-OutPath 'pptx-anim-write-probe.txt'

$tmpDir = Join-Path $env:TEMP 'ooxp-office'
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
$tmp = Join-Path $tmpDir 'pptx-anim-write-probe.pptx'
Copy-Item -Force $hostPath $tmp

$ppt = Ensure-MsPpt $hostPath
Close-PptPresByPath $ppt $tmp
$pres = Open-PptPres $ppt $tmp
$s1 = Get-PptSlide $pres 1
$n1 = Add-PptFade $s1 1 1
Write-Log "一条进入淡入 MainSequence=$n1"
$pres.Save()
Copy-Item -Force $tmp $one
$xml1 = Get-ZipEntryText $one 'ppt/slides/slide1.xml'
[IO.File]::WriteAllText($xmlOne, $xml1, $global:OoxpUtf8)

$n2 = Add-PptFade $s1 2 1
Write-Log "两条进入淡入 MainSequence=$n2"
$pres.Save()
Copy-Item -Force $tmp $two
$xml2 = Get-ZipEntryText $two 'ppt/slides/slide1.xml'
[IO.File]::WriteAllText($xmlTwo, $xml2, $global:OoxpUtf8)

Release-Com $s1
Close-PptPres $pres

$pres1 = Open-PptPres $ppt $one
$cap1 = Get-PptCaption $pres1
Close-PptPres $pres1
$pres2 = Open-PptPres $ppt $two
$cap2 = Get-PptCaption $pres2
Close-PptPres $pres2

$hasMain = $xml1 -match 'nodeType="mainSeq"'
$hasClick = $xml1 -match 'nodeType="clickEffect"'
$hasBld = $xml1 -match '<p:bldLst'
$hasFade = $xml1 -match 'filter="fade"'
$hasTxEl = $xml1 -match '<p:txEl'

$lines = @(
    "one=$one"
    "two=$two"
    "xmlOne=$xmlOne"
    "xmlTwo=$xmlTwo"
    "capOne=$cap1"
    "capTwo=$cap2"
    "mainSeq=$hasMain"
    "clickEffect=$hasClick"
    "fade=$hasFade"
    "bldLst=$hasBld"
    "txEl=$hasTxEl"
    "oneLen=$($xml1.Length) twoLen=$($xml2.Length)"
)
[IO.File]::WriteAllLines($report, $lines, $global:OoxpUtf8)
Write-Log ($lines -join '; ')
