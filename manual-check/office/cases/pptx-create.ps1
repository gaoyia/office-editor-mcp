# pptx create：docs/checklist-pptx.md 的 C1、C3、C4。
# C2 是 WPS 打开。验收只认官方 Office，不再跑。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$path = Ensure-PptxCreate
$ppt = Ensure-MsPpt $path
$pres = Open-PptPres $ppt $path

$s1 = Get-PptSlide $pres 1
$s2 = Get-PptSlide $pres 2
$box1 = Get-PptShape $s1 '文本框 1'
$box2 = Get-PptShape $s1 '文本框 2'
$box3 = Get-PptShape $s2 '文本框 1'
$t1 = Get-ShapeText $box1
$t2 = Get-ShapeText $box2
$t3 = Get-ShapeText $box3
$caption = Get-PptCaption $pres

Add-Check 'pptx-C1' ($caption -notmatch 'Repair') $caption
Add-Skip  'pptx-C2' '验收只认 Microsoft PowerPoint，不再跑 WPS'
Add-Check 'pptx-C3' (($t1 -eq $S.title) -and ($t2 -eq $S.hello)) "s1=$t1 / $t2"
Add-Check 'pptx-C4' ($t3 -eq $S.page2) "s2=$t3"

Release-Com $box1
Release-Com $box2
Release-Com $box3
Release-Com $s1
Release-Com $s2
Close-PptPres $pres
