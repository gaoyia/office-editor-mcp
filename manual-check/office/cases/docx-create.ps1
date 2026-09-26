# docx create：docs/checklist-docx.md 的 C1、C3、C4。
# C2 是 WPS 打开。验收只认官方 Office，不再跑。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$path = Ensure-DocxCreate
$word = Ensure-MsWord $path
$doc = Open-WordDoc $word $path

$p1 = $doc.Paragraphs.Item(1)
$p2 = $doc.Paragraphs.Item(2)
$t1 = Get-ParaText $p1
$t2 = Get-ParaText $p2
# 要读这份文档自己的标题栏。$word.ActiveWindow 可能是别的文档的窗口。
$caption = [string]$doc.ActiveWindow.Caption
# create 产物写 settings.xml 的 compatibilityMode=15，Word 应按非兼容模式打开。
$compat = $null
try { $compat = $doc.CompatibilityMode } catch {}

Add-Check 'docx-C1' (($caption -notmatch 'Repair') -and ([int]$compat -eq 15)) "$caption compat=$compat"
Add-Skip  'docx-C2' '验收只认 Microsoft Word，不再跑 WPS'

$styleOk = ($p1.Style.NameLocal -eq $S.heading1) -or ($p1.Style.NameLocal -eq 'Heading 1')
Add-Check 'docx-C3' ($styleOk -and $t1 -eq $S.title) "style=$($p1.Style.NameLocal) text=$t1"
Add-Check 'docx-C4' ($t2 -eq $S.hello) "text=$t2"

Release-Com $p1
Release-Com $p2
Close-WordDoc $doc
