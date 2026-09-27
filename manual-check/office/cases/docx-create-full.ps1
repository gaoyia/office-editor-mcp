# docx create 补齐：docs/checklist-docx.md 的 Cf1–Cf6。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$wdSectionNewPage = 2
$path = Ensure-DocxCreateFull -Force
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'docx'
} 'docx-create-full-inspect-counts'

$secs = @($tree0.sections)
Add-Check 'docx-Cf1' (
    ($secs.Count -eq 2) -and
    ($tree0.chartCount -eq 1) -and
    ($tree0.footnoteCount -eq 1) -and
    ($tree0.endnoteCount -eq 1) -and
    ($tree0.commentCount -eq 1) -and
    -not ($tree0.PSObject.Properties.Name -contains 'charts')
) "sections=$($secs.Count) chart=$($tree0.chartCount) fn=$($tree0.footnoteCount) en=$($tree0.endnoteCount) cmt=$($tree0.commentCount)"

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; charts = $true; notes = $true; comments = $true }
} 'docx-create-full-inspect'

$pie = @($tree.charts) | Select-Object -First 1
Add-Check 'docx-Cf2' (
    ($tree.chartCount -eq 1) -and ($pie) -and ([string]$pie.type -eq 'pie')
) "count=$($tree.chartCount) type=$($pie.type) id=$($pie.id)"

$fn = @($tree.footnotes) | Select-Object -First 1
$fnText = if ($fn) { [string]@($fn.paragraphs)[0].text } else { '' }
Add-Check 'docx-Cf3' (($tree.footnoteCount -eq 1) -and ($fnText -eq '脚注一')) "count=$($tree.footnoteCount) text=$fnText"

$en = @($tree.endnotes) | Select-Object -First 1
$enText = if ($en) { [string]@($en.paragraphs)[0].text } else { '' }
Add-Check 'docx-Cf4' (($tree.endnoteCount -eq 1) -and ($enText -eq '尾注一')) "count=$($tree.endnoteCount) text=$enText"

$cmt = @($tree.comments) | Select-Object -First 1
$cmtText = if ($cmt) { [string]@($cmt.paragraphs)[0].text } else { '' }
Add-Check 'docx-Cf5' (($tree.commentCount -eq 1) -and ($cmtText -eq '批注一')) "count=$($tree.commentCount) text=$cmtText"

function Get-WordChartObjects($doc) {
    $out = New-Object System.Collections.ArrayList
    $inlines = $doc.InlineShapes
    try {
        $n = [int]$inlines.Count
        for ($i = 1; $i -le $n; $i++) {
            $ish = $inlines.Item($i)
            try {
                if ([int]$ish.Type -eq 12) { [void]$out.Add($ish.Chart) }
            }
            finally { Release-Com $ish }
        }
    }
    finally { Release-Com $inlines }
    $shapes = $doc.Shapes
    try {
        $n = [int]$shapes.Count
        for ($i = 1; $i -le $n; $i++) {
            $shp = $shapes.Item($i)
            try {
                $has = $false
                try { $has = [bool]$shp.HasChart } catch {
                    $has = ([int]$shp.Type -eq 3)
                }
                if ($has) { [void]$out.Add($shp.Chart) }
            }
            finally { Release-Com $shp }
        }
    }
    finally { Release-Com $shapes }
    return @($out)
}

$word = Ensure-MsWord $path
$doc = Open-WordDoc $word $path
$caption = [string]$doc.ActiveWindow.Caption
$secN = [int]$doc.Sections.Count
$secStart = $null
try { $secStart = [int]$doc.Sections.Item(1).PageSetup.SectionStart } catch {}
$fnN = [int]$doc.Footnotes.Count
$enN = [int]$doc.Endnotes.Count
$cmtN = [int]$doc.Comments.Count
$charts = @(Get-WordChartObjects $doc)
$pieTypeOk = $false
foreach ($ch in $charts) {
    try {
        $t = [int]$ch.ChartType
        if (@(5, 69, 68, 70, -4102) -contains $t) { $pieTypeOk = $true }
    } catch {}
}
$fnSaw = $false
for ($i = 1; $i -le $fnN; $i++) {
    $t = ([string]$doc.Footnotes.Item($i).Range.Text).Trim()
    if ($t -match [regex]::Escape('脚注一')) { $fnSaw = $true }
}
$enSaw = $false
for ($i = 1; $i -le $enN; $i++) {
    $t = ([string]$doc.Endnotes.Item($i).Range.Text).Trim()
    if ($t -match [regex]::Escape('尾注一')) { $enSaw = $true }
}
$cmtSaw = $false
for ($i = 1; $i -le $cmtN; $i++) {
    $t = ([string]$doc.Comments.Item($i).Range.Text).Trim()
    if ($t -match [regex]::Escape('批注一')) { $cmtSaw = $true }
}
Add-Check 'docx-Cf6' (
    ($caption -notmatch 'Repair') -and
    ($secN -eq 2) -and
    ($secStart -eq $wdSectionNewPage) -and
    ($charts.Count -ge 1) -and
    $pieTypeOk -and
    ($fnN -ge 1) -and $fnSaw -and
    ($enN -ge 1) -and $enSaw -and
    ($cmtN -ge 1) -and $cmtSaw
) "caption=$caption secs=$secN start=$secStart charts=$($charts.Count) pie=$pieTypeOk fn=$fnN/$fnSaw en=$enN/$enSaw cmt=$cmtN/$cmtSaw"

Close-WordDoc $doc
