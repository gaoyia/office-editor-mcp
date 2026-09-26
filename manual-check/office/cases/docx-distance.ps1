# docx 距边界：docs/checklist-docx.md 的 D1–D4。
# 用 Word 亲手另存的源文件，COM 读 PageSetup（单位磅 = twip/20）。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-DocxWordSource
$word = Ensure-MsWord $src

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-dist-inspect'

$sects = @($tree0.sections)
$s1 = $sects | Where-Object { $_.id -eq 'section:1' } | Select-Object -First 1
$d1ok = ($null -ne $s1) -and ($null -ne $s1.headerDistance) -and ($null -ne $s1.footerDistance)
Add-Check 'docx-D1' $d1ok "id=$($s1.id) h=$($s1.headerDistance) f=$($s1.footerDistance)"

$bodyBefore = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-dist-body-before'
$bodyTexts = @($bodyBefore.paragraphs | ForEach-Object { $_.text })

$headerTwip = 1440
$footerTwip = 720
$applied = Get-OutPath 'docx-dist-applied.docx'
Close-WordDocByPath $word $applied
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $applied.Replace('\', '/')
    ops = @(@{
        op = 'setHeaderFooterDistance'
        id = 'section:1'
        headerDistance = $headerTwip
        footerDistance = $footerTwip
    })
} 'docx-dist-apply' | Out-Null

$doc = Open-WordDoc $word $applied
$caption = [string]$doc.ActiveWindow.Caption
Add-Check 'docx-D2' ($caption -notmatch 'Repair') $caption

$ps = $doc.PageSetup
$hdrPt = [double]$ps.HeaderDistance
$ftrPt = [double]$ps.FooterDistance
$hdrOk = [math]::Abs($hdrPt - ($headerTwip / 20.0)) -lt 0.05
$ftrOk = [math]::Abs($ftrPt - ($footerTwip / 20.0)) -lt 0.05
Add-Check 'docx-D3' ($hdrOk -and $ftrOk) "hdrPt=$hdrPt ftrPt=$ftrPt"
Release-Com $ps

$p1 = Get-ParaText $doc.Paragraphs.Item(1)
$bodyAfter = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $applied.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-dist-body-after'
$bodyTexts2 = @($bodyAfter.paragraphs | ForEach-Object { $_.text })
$bodyOk = (@(Compare-Object $bodyTexts $bodyTexts2).Count -eq 0) -and ($p1 -eq $S.chapter1)
Add-Check 'docx-D4' $bodyOk "p1=$p1"
