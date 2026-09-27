# docx 节属性：docs/checklist-docx.md 的 Sp1–Sp8。
# Word COM 的 PageSetup 边距与纸张单位是磅，等于 twip ÷ 20。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-DocxWordSource
$word = Ensure-MsWord $src

function Twip-ToPt([int]$twip) { $twip / 20.0 }
function Close-To([double]$a, [double]$b) { [math]::Abs($a - $b) -lt 0.05 }

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-sect-inspect'

$s1 = @($tree0.sections) | Where-Object { $_.id -eq 'section:1' } | Select-Object -First 1
$sp1ok = ($null -ne $s1) -and ($null -ne $s1.top) -and ($null -ne $s1.pageWidth) -and
    ($tree0.PSObject.Properties.Name -contains 'mirrorMargins')
Add-Check 'docx-Sp1' $sp1ok "top=$($s1.top) w=$($s1.pageWidth) gutter=$($s1.gutter) mirror=$($tree0.mirrorMargins)"

$gutterBefore = $s1.gutter

$topTwip = 1440
$bottomTwip = 1440
$leftTwip = 1800
$rightTwip = 1800
$wide = 16838
$high = 11906
$applied = Get-OutPath 'docx-sect-applied.docx'
Close-WordDocByPath $word $applied
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $applied.Replace('\', '/')
    ops = @(
        @{
            op = 'setPageMargin'; id = 'section:1'
            top = $topTwip; bottom = $bottomTwip; left = $leftTwip; right = $rightTwip
        }
        @{
            op = 'setPageSize'; id = 'section:1'
            pageWidth = $wide; pageHeight = $high; orient = 'landscape'
        }
    )
} 'docx-sect-apply' | Out-Null

$doc = Open-WordDoc $word $applied
$caption = [string]$doc.ActiveWindow.Caption
Add-Check 'docx-Sp2' ($caption -notmatch 'Repair') $caption

$ps = $doc.PageSetup
$topOk = Close-To ([double]$ps.TopMargin) (Twip-ToPt $topTwip)
$botOk = Close-To ([double]$ps.BottomMargin) (Twip-ToPt $bottomTwip)
$leftOk = Close-To ([double]$ps.LeftMargin) (Twip-ToPt $leftTwip)
$rightOk = Close-To ([double]$ps.RightMargin) (Twip-ToPt $rightTwip)
$treeMid = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $applied.Replace('\', '/'); format = 'docx'
} 'docx-sect-inspect-mid'
$gutterAfter = $treeMid.sections[0].gutter
$gutterOk = ($null -eq $gutterBefore -and $null -eq $gutterAfter) -or ($gutterBefore -eq $gutterAfter)
Add-Check 'docx-Sp3' ($topOk -and $botOk -and $leftOk -and $rightOk -and $gutterOk) `
    "top=$($ps.TopMargin) bot=$($ps.BottomMargin) left=$($ps.LeftMargin) right=$($ps.RightMargin) gutter=$gutterAfter"

$orient = [int]$ps.Orientation
$wOk = Close-To ([double]$ps.PageWidth) (Twip-ToPt $wide)
$hOk = Close-To ([double]$ps.PageHeight) (Twip-ToPt $high)
Add-Check 'docx-Sp4' (($orient -eq 1) -and $wOk -and $hOk) "orient=$orient w=$($ps.PageWidth) h=$($ps.PageHeight)"
Release-Com $ps

$headerTwip = 1440
$distOut = Get-OutPath 'docx-sect-dist.docx'
Close-WordDocByPath $word $distOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $applied.Replace('\', '/')
    output = $distOut.Replace('\', '/')
    ops = @(@{ op = 'setHeaderFooterDistance'; id = 'section:1'; headerDistance = $headerTwip })
} 'docx-sect-dist' | Out-Null
$doc2 = Open-WordDoc $word $distOut
$ps2 = $doc2.PageSetup
$hdrOk = Close-To ([double]$ps2.HeaderDistance) (Twip-ToPt $headerTwip)
$topStill = Close-To ([double]$ps2.TopMargin) (Twip-ToPt $topTwip)
Add-Check 'docx-Sp5' ($hdrOk -and $topStill) "hdr=$($ps2.HeaderDistance) top=$($ps2.TopMargin)"
Release-Com $ps2

$colsOut = Get-OutPath 'docx-sect-cols.docx'
Close-WordDocByPath $word $colsOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $applied.Replace('\', '/')
    output = $colsOut.Replace('\', '/')
    ops = @(@{ op = 'setColumns'; id = 'section:1'; num = 2; space = 720; equalWidth = $true })
} 'docx-sect-cols' | Out-Null
$doc3 = Open-WordDoc $word $colsOut
$cap3 = [string]$doc3.ActiveWindow.Caption
$ps3 = $doc3.PageSetup
$cols = $ps3.TextColumns
$colCount = [int]$cols.Count
Add-Check 'docx-Sp6' (($cap3 -notmatch 'Repair') -and ($colCount -eq 2)) "caption=$cap3 cols=$colCount"
Release-Com $cols
Release-Com $ps3

$vaOut = Get-OutPath 'docx-sect-valign.docx'
Close-WordDocByPath $word $vaOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $applied.Replace('\', '/')
    output = $vaOut.Replace('\', '/')
    ops = @(
        @{ op = 'setVerticalAlign'; id = 'section:1'; vAlign = 'center' }
        @{ op = 'setLineNumbering'; id = 'section:1'; countBy = 1 }
    )
} 'docx-sect-valign' | Out-Null
$doc4 = Open-WordDoc $word $vaOut
$cap4 = [string]$doc4.ActiveWindow.Caption
$ps4 = $doc4.PageSetup
$va = [int]$ps4.VerticalAlignment
$ln = $ps4.LineNumbering
$lnOn = [bool]$ln.Active
$lnBy = [int]$ln.CountBy
Add-Check 'docx-Sp7' (($cap4 -notmatch 'Repair') -and ($va -eq 1) -and $lnOn -and ($lnBy -eq 1)) `
    "caption=$cap4 va=$va lnOn=$lnOn countBy=$lnBy"
Release-Com $ln
Release-Com $ps4

$mirOut = Get-OutPath 'docx-sect-mirror.docx'
Close-WordDocByPath $word $mirOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $applied.Replace('\', '/')
    output = $mirOut.Replace('\', '/')
    ops = @(@{ op = 'setMirrorMargins'; id = 'docx:document'; mirrorMargins = $true })
} 'docx-sect-mirror' | Out-Null
$doc5 = Open-WordDoc $word $mirOut
$cap5 = [string]$doc5.ActiveWindow.Caption
$ps5 = $doc5.PageSetup
$mir = [bool]$ps5.MirrorMargins
Add-Check 'docx-Sp8' (($cap5 -notmatch 'Repair') -and $mir) "caption=$cap5 mirror=$mir"
Release-Com $ps5
