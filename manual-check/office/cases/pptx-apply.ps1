# pptx apply：docs/checklist-pptx.md 的 A1–A5。
# 源文件由 PowerPoint 亲手存出，第一页第二个文本框加过粗。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-PptxPptSource
$ppt = Ensure-MsPpt $src

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1; maxShapes = 200 }
} 'pptx-source-inspect'
$id2 = $srcTree.slides[0].shapes[1].id
Write-Log ("源文件形状 id：" + (@($srcTree.slides[0].shapes | ForEach-Object { $_.id }) -join ' '))

$applied = Get-OutPath 'pptx-applied.pptx'
Close-PptPresByPath $ppt $applied
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $applied.Replace('\', '/')
    ops = @(@{ op = 'setShapeText'; id = $id2; text = $S.shape2new })
} 'pptx-apply' | Out-Null

$pres = Open-PptPres $ppt $applied
$s1 = Get-PptSlide $pres 1
$s2 = Get-PptSlide $pres 2
$box1 = Get-PptShape $s1 1
$box2 = Get-PptShape $s1 2
$box3 = Get-PptShape $s2 1
$t1 = Get-ShapeText $box1
$t2 = Get-ShapeText $box2
$t3 = Get-ShapeText $box3
$caption = Get-PptCaption $pres

# 别写成 $box2.TextFrame.TextRange.Font.Bold：那样点出去的三个中间对象
# 都是没人松手的 COM 包装，PowerPoint 会以为客户端还连着，收尾时不肯退。
$frame2 = $box2.TextFrame
$range2 = $frame2.TextRange
$font2 = $range2.Font
$bold = $font2.Bold

Add-Check 'pptx-A1' ($caption -notmatch 'Repair') $caption
Add-Check 'pptx-A2' ($t2 -eq $S.shape2new) $t2
Add-Check 'pptx-A3' ($t1 -eq $S.chapter1) $t1
Add-Check 'pptx-A4' (($bold -eq -1) -or ($bold -eq $true)) "bold=$bold"
Add-Check 'pptx-A5' ($t3 -eq $S.page2keep) $t3

Release-Com $font2
Release-Com $range2
Release-Com $frame2
Release-Com $box1
Release-Com $box2
Release-Com $box3
Release-Com $s1
Release-Com $s2
# 与 pptx-create.ps1 对齐：文稿要关、$pres 要松手，否则进程退不掉。
Close-PptPres $pres
