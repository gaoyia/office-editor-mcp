# 切换剩余第 0 步：Office 16.0 另存自动进片、wipe 方向、prstTrans。不进 run.ps1。

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

function Set-ZipEntryText([string]$zipPath, [string]$entryName, [string]$text) {
    $zip = [IO.Compression.ZipFile]::Open($zipPath, 'Update')
    try {
        $e = $zip.GetEntry($entryName)
        if ($null -ne $e) { $e.Delete() }
        $new = $zip.CreateEntry($entryName, [IO.Compression.CompressionLevel]::Optimal)
        $bytes = $global:OoxpUtf8.GetBytes($text)
        $s = $new.Open()
        try { $s.Write($bytes, 0, $bytes.Length) }
        finally { $s.Dispose() }
    }
    finally { $zip.Dispose() }
}

function Save-SlideXml($pres, [string]$pptxOut, [string]$xmlOut) {
    $pres.Save()
    Copy-Item -Force $pres.FullName $pptxOut
    $xml = Get-ZipEntryText $pptxOut 'ppt/slides/slide1.xml'
    [IO.File]::WriteAllText($xmlOut, $xml, $global:OoxpUtf8)
    return $xml
}

function Snip-Transition([string]$xml) {
    $m = [regex]::Match($xml, '(?s)(<p:transition\b.*?</p:transition>|<mc:AlternateContent\b.*?</mc:AlternateContent>)')
    if ($m.Success) { return $m.Value }
    return '(no transition)'
}

function Set-PptTransition($slide, [int]$effect, [bool]$onClick, [bool]$onTime, [double]$seconds) {
    $t = $slide.SlideShowTransition
    try {
        $t.EntryEffect = $effect
        $t.Duration = 0.5
        if ($onClick) { $t.AdvanceOnClick = -1 } else { $t.AdvanceOnClick = 0 }
        if ($onTime) { $t.AdvanceOnTime = -1 } else { $t.AdvanceOnTime = 0 }
        if ($onTime) { $t.AdvanceTime = $seconds }
        return [pscustomobject]@{
            EntryEffect = [int]$t.EntryEffect
            AdvanceOnClick = [int]$t.AdvanceOnClick
            AdvanceOnTime = [int]$t.AdvanceOnTime
            AdvanceTime = [double]$t.AdvanceTime
        }
    }
    finally { Release-Com $t }
}

$report = Get-OutPath 'pptx-transition-remain-probe.txt'
$hostPath = Ensure-PptxCreate
$tmpDir = Join-Path $env:TEMP 'ooxp-office'
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
$tmp = Join-Path $tmpDir 'pptx-transition-remain-probe.pptx'
Copy-Item -Force $hostPath $tmp

$ppt = Ensure-MsPpt $hostPath
Close-PptPresByPath $ppt $tmp
$pres = Open-PptPres $ppt $tmp
$s1 = Get-PptSlide $pres 1

$lines = New-Object System.Collections.Generic.List[string]

$auto = Set-PptTransition $s1 3849 $true $true 2
$xmlAuto = Save-SlideXml $pres (Get-OutPath 'pptx-tr-remain-auto.pptx') (Get-OutPath 'pptx-tr-remain-auto-slide1.xml')
$capAuto = Get-PptCaption $pres
$lines.Add("auto COM effect=$($auto.EntryEffect) click=$($auto.AdvanceOnClick) time=$($auto.AdvanceOnTime) seconds=$($auto.AdvanceTime)")
$lines.Add("auto cap=$capAuto")
$lines.Add("auto xml=$(Snip-Transition $xmlAuto)")
$lines.Add("auto advClick=$($xmlAuto -match 'advClick') advTm=$($xmlAuto -match 'advTm')")

$clickOnly = Set-PptTransition $s1 3849 $true $false 0
$xmlClick = Save-SlideXml $pres (Get-OutPath 'pptx-tr-remain-click.pptx') (Get-OutPath 'pptx-tr-remain-click-slide1.xml')
$lines.Add("clickOnly COM click=$($clickOnly.AdvanceOnClick) time=$($clickOnly.AdvanceOnTime)")
$lines.Add("clickOnly xml=$(Snip-Transition $xmlClick)")

$noClick = Set-PptTransition $s1 3849 $false $true 3
$xmlNoClick = Save-SlideXml $pres (Get-OutPath 'pptx-tr-remain-noclick.pptx') (Get-OutPath 'pptx-tr-remain-noclick-slide1.xml')
$lines.Add("noClick COM click=$($noClick.AdvanceOnClick) time=$($noClick.AdvanceOnTime) seconds=$($noClick.AdvanceTime)")
$lines.Add("noClick xml=$(Snip-Transition $xmlNoClick)")

$wipeL = Set-PptTransition $s1 2817 $true $false 0
$xmlWipeL = Save-SlideXml $pres (Get-OutPath 'pptx-tr-remain-wipe-l.pptx') (Get-OutPath 'pptx-tr-remain-wipe-l-slide1.xml')
$lines.Add("wipeL COM=$($wipeL.EntryEffect) xml=$(Snip-Transition $xmlWipeL)")

$wipeR = Set-PptTransition $s1 2819 $true $false 0
$xmlWipeR = Save-SlideXml $pres (Get-OutPath 'pptx-tr-remain-wipe-r.pptx') (Get-OutPath 'pptx-tr-remain-wipe-r-slide1.xml')
$lines.Add("wipeR COM=$($wipeR.EntryEffect) xml=$(Snip-Transition $xmlWipeR)")

$pushL = Set-PptTransition $s1 3853 $true $false 0
$xmlPushL = Save-SlideXml $pres (Get-OutPath 'pptx-tr-remain-push-l.pptx') (Get-OutPath 'pptx-tr-remain-push-l-slide1.xml')
$lines.Add("pushL COM=$($pushL.EntryEffect) xml=$(Snip-Transition $xmlPushL)")

$coverU = Set-PptTransition $s1 1282 $true $false 0
$xmlCoverU = Save-SlideXml $pres (Get-OutPath 'pptx-tr-remain-cover-u.pptx') (Get-OutPath 'pptx-tr-remain-cover-u-slide1.xml')
$lines.Add("coverU COM=$($coverU.EntryEffect) xml=$(Snip-Transition $xmlCoverU)")

Release-Com $s1
Close-PptPres $pres

# 注入 p15:prstTrans，再让 Office 另存看它怎么改。
$inject = Join-Path $tmpDir 'pptx-tr-remain-inject.pptx'
Copy-Item -Force $hostPath $inject
$srcXml = Get-ZipEntryText $inject 'ppt/slides/slide1.xml'
if ($srcXml -notmatch 'xmlns:p15=') {
    $srcXml = $srcXml -replace '<p:sld ', '<p:sld xmlns:p15="http://schemas.microsoft.com/office/powerpoint/2012/main" xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" '
}
$prstChunk = @'
<mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"><mc:Choice Requires="p15"><p:transition spd="fast" p14:dur="800"><p15:prstTrans prst="fallOver"/></p:transition></mc:Choice><mc:Fallback><p:transition spd="fast" p14:dur="800"><p:fade/></p:transition></mc:Fallback></mc:AlternateContent>
'@
$injXml = $srcXml -replace '</p:sld>', ($prstChunk + '</p:sld>')
Set-ZipEntryText $inject 'ppt/slides/slide1.xml' $injXml
Copy-Item -Force $inject (Get-OutPath 'pptx-tr-remain-inject-before.pptx')
[IO.File]::WriteAllText((Get-OutPath 'pptx-tr-remain-inject-before-slide1.xml'), $injXml, $global:OoxpUtf8)

Close-PptPresByPath $ppt $inject
$pres2 = Open-PptPres $ppt $inject
$capInj = Get-PptCaption $pres2
$xmlInj = Save-SlideXml $pres2 (Get-OutPath 'pptx-tr-remain-inject-saved.pptx') (Get-OutPath 'pptx-tr-remain-inject-saved-slide1.xml')
$sInj = Get-PptSlide $pres2 1
$tInj = $sInj.SlideShowTransition
try {
    $lines.Add("inject cap=$capInj COM effect=$([int]$tInj.EntryEffect)")
}
finally {
    Release-Com $tInj
    Release-Com $sInj
}
$lines.Add("inject saved xml=$(Snip-Transition $xmlInj)")
$lines.Add("inject hasPrst=$($xmlInj -match 'prstTrans') prstFall=$($xmlInj -match 'fallOver')")
Close-PptPres $pres2

# COM 扫一段未写入档的 EntryEffect，看会不会写出 prstTrans。
$scan = Join-Path $tmpDir 'pptx-tr-remain-scan.pptx'
Copy-Item -Force $hostPath $scan
Close-PptPresByPath $ppt $scan
$pres3 = Open-PptPres $ppt $scan
$s3 = Get-PptSlide $pres3 1
$hits = New-Object System.Collections.Generic.List[string]
foreach ($n in 3934..3980) {
    try {
        $info = Set-PptTransition $s3 $n $true $false 0
        $pres3.Save()
        $snap = Get-OutPath ("pptx-tr-remain-scan-{0}.pptx" -f $n)
        Copy-Item -Force $scan $snap
        $xmlN = Get-ZipEntryText $snap 'ppt/slides/slide1.xml'
        if ($xmlN -match 'prstTrans') {
            $hits.Add("$n=>$($info.EntryEffect) $(Snip-Transition $xmlN)")
        }
        else {
            Remove-Item -Force $snap
        }
    }
    catch {
        $hits.Add("$n ERR $($_.Exception.Message)")
    }
}
Release-Com $s3
Close-PptPres $pres3
if ($hits.Count -eq 0) { $lines.Add('scan prstTrans=none 3934-3980') }
else { foreach ($h in $hits) { $lines.Add("scan $h") } }

[IO.File]::WriteAllLines($report, $lines, $global:OoxpUtf8)
Write-Log ($lines -join "`n")

Close-OfficeApps -Quit
