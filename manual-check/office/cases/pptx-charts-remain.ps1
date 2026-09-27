# pptx 图表剩余：docs/checklist-pptx.md 的 Pr1–Pr3。
# 源文件：PowerPoint 亲手另存的柱形图。趋势线与次轴打在新插的双系列柱形上。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-PptxPptChartSource
$ppt = Ensure-MsPpt $src
Close-PptPresByPath $ppt $src

function Flatten-Nums($vals) {
    $out = @()
    foreach ($v in @($vals)) {
        if ($null -eq $v) { continue }
        try { $out += [double]$v } catch {}
    }
    return $out
}

function Test-NumsClose($a, $b) {
    $x = @(Flatten-Nums $a)
    $y = @(Flatten-Nums $b)
    if ($x.Count -ne $y.Count) { return $false }
    for ($i = 0; $i -lt $x.Count; $i++) {
        if ([math]::Abs($x[$i] - $y[$i]) -gt 0.01) { return $false }
    }
    return $true
}

function Get-PptSeries($chart, [int]$index) {
    try { return $chart.SeriesCollection($index) } catch {
        try { return $chart.FullSeriesCollection($index) } catch { return $null }
    }
}

function Get-PptSeriesNums($chart, [int]$index) {
    $ser = Get-PptSeries $chart $index
    if (-not $ser) { return @() }
    try { return @(Flatten-Nums $ser.Values) }
    finally { Release-Com $ser }
}

function Get-TrendlineCount($chart, [int]$index) {
    $ser = Get-PptSeries $chart $index
    if (-not $ser) { return 0 }
    try {
        $tls = $null
        try { $tls = $ser.Trendlines() } catch { try { $tls = $ser.Trendlines } catch { return 0 } }
        try { return [int]$tls.Count } finally { Release-Com $tls }
    }
    catch { return 0 }
    finally { Release-Com $ser }
}

function Get-AxisGroup($chart, [int]$index) {
    $ser = Get-PptSeries $chart $index
    if (-not $ser) { return 0 }
    try { return [int]$ser.AxisGroup } catch { return 0 }
    finally { Release-Com $ser }
}

function Find-PptChartByVals($slide, $want) {
    $shapes = $slide.Shapes
    try {
        for ($i = 1; $i -le $shapes.Count; $i++) {
            $cand = $shapes.Item($i)
            $has = $false
            try { $has = [bool]$cand.HasChart } catch {
                try { $has = ([int]$cand.Type -eq 3) } catch { $has = $false }
            }
            if ($has) {
                $ch = $null
                try { $ch = $cand.Chart } catch {}
                if ($ch) {
                    $cv = Get-PptSeriesNums $ch 1
                    if (Test-NumsClose $cv $want) {
                        Release-Com $ch
                        return $cand
                    }
                    Release-Com $ch
                }
            }
            Release-Com $cand
        }
    }
    finally { Release-Com $shapes }
    return $null
}

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ charts = $true }
} 'pptx-charts-remain-src'
$oldIds = @(@($tree0.slides[0].charts) | ForEach-Object { [string]$_.id })

$inserted = Get-OutPath 'pptx-charts-remain-col.pptx'
Close-PptPresByPath $ppt $inserted
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'
        id = 'slide:1'
        type = 'col'
        title = $S.chartTitle
        series = @(
            @{ name = $S.chartSer1; categories = @($S.saItem1, $S.saItem2, $S.saItem3); values = @(1, 2, 3) }
            @{ name = $S.chartSer2; categories = @($S.saItem1, $S.saItem2, $S.saItem3); values = @(4, 5, 6) }
        )
    })
} 'pptx-charts-remain-insert' | Out-Null

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'pptx'
    scope = @{ charts = $true }
} 'pptx-charts-remain-insert-inspect'
$col = @($treeI.slides[0].charts) | Where-Object { $oldIds -notcontains [string]$_.id } | Select-Object -First 1
$colId = if ($col) { [string]$col.id } else { '' }
$serA = if ($col) { [string]$col.series[0].id } else { '' }
$serB = if ($col) { [string]$col.series[1].id } else { '' }

# Pr1：趋势线。
$tlOut = Get-OutPath 'pptx-charts-remain-trendline.pptx'
Close-PptPresByPath $ppt $tlOut
if (Test-Path $tlOut) { Remove-Item $tlOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $tlOut.Replace('\', '/')
    ops = @(@{ op = 'insertTrendline'; id = $serA; type = 'linear' })
} 'pptx-charts-remain-tl' | Out-Null
$treeTl = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tlOut.Replace('\', '/'); format = 'pptx'
    scope = @{ charts = $true }
} 'pptx-charts-remain-tl-inspect'
$colTl = @($treeTl.slides[0].charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$tls = if ($colTl) { @($colTl.series[0].trendlines) } else { @() }
$flagsTl = if ($colTl) { @($colTl.flags) } else { @() }
$tlId = if ($tls.Count) { [string]$tls[0].id } else { '' }
$presTl = Open-PptPres $ppt $tlOut
$capTl = Get-PptCaption $presTl
$sTl = Get-PptSlide $presTl 1
$shapeTl = Find-PptChartByVals $sTl @(1, 2, 3)
$tlCount = 0
if ($shapeTl) {
    $chTl = $null
    try { $chTl = $shapeTl.Chart } catch {}
    if ($chTl) {
        $tlCount = Get-TrendlineCount $chTl 1
        Release-Com $chTl
    }
    Release-Com $shapeTl
}
Release-Com $sTl
Close-PptPres $presTl
Add-Check 'pptx-Pr1' (($capTl -notmatch 'Repair') -and ($tls.Count -ge 1) -and ($flagsTl -contains 'trendline') -and ($tlCount -ge 1)) "caption=$capTl n=$($tls.Count) com=$tlCount"

# Pr2：改成 exp 再删。
$tlSet = Get-OutPath 'pptx-charts-remain-tl-set.pptx'
Close-PptPresByPath $ppt $tlSet
if (Test-Path $tlSet) { Remove-Item $tlSet -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $tlOut.Replace('\', '/')
    output = $tlSet.Replace('\', '/')
    ops = @(@{ op = 'setTrendline'; id = $tlId; type = 'exp' })
} 'pptx-charts-remain-tl-set' | Out-Null
$treeSet = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tlSet.Replace('\', '/'); format = 'pptx'
    scope = @{ charts = $true }
} 'pptx-charts-remain-tl-set-inspect'
$colSet = @($treeSet.slides[0].charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$tySet = if ($colSet) { [string]$colSet.series[0].trendlines[0].type } else { '' }
$tlId2 = if ($colSet) { [string]$colSet.series[0].trendlines[0].id } else { '' }
$presSet = Open-PptPres $ppt $tlSet
$capSet = Get-PptCaption $presSet
Close-PptPres $presSet
$tlDel = Get-OutPath 'pptx-charts-remain-tl-del.pptx'
Close-PptPresByPath $ppt $tlDel
if (Test-Path $tlDel) { Remove-Item $tlDel -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $tlSet.Replace('\', '/')
    output = $tlDel.Replace('\', '/')
    ops = @(@{ op = 'deleteTrendline'; id = $tlId2 })
} 'pptx-charts-remain-tl-del' | Out-Null
$treeDel = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tlDel.Replace('\', '/'); format = 'pptx'
    scope = @{ charts = $true }
} 'pptx-charts-remain-tl-del-inspect'
$colDel = @($treeDel.slides[0].charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$gone = $colDel -and -not ($colDel.series[0].PSObject.Properties.Name -contains 'trendlines')
$presDel = Open-PptPres $ppt $tlDel
$capDel = Get-PptCaption $presDel
Close-PptPres $presDel
Add-Check 'pptx-Pr2' (($capSet -notmatch 'Repair') -and ($tySet -eq 'exp') -and ($capDel -notmatch 'Repair') -and $gone) "setCap=$capSet type=$tySet delCap=$capDel gone=$gone"

# Pr3：次轴。
$axOut = Get-OutPath 'pptx-charts-remain-axis.pptx'
Close-PptPresByPath $ppt $axOut
if (Test-Path $axOut) { Remove-Item $axOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $axOut.Replace('\', '/')
    ops = @(@{ op = 'setChartSeriesAxis'; id = $serB; axis = 'secondary' })
} 'pptx-charts-remain-axis' | Out-Null
$treeAx = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $axOut.Replace('\', '/'); format = 'pptx'
    scope = @{ charts = $true }
} 'pptx-charts-remain-axis-inspect'
$colAx = @($treeAx.slides[0].charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$axis2 = if ($colAx) { [string]$colAx.series[1].axis } else { '' }
$flagsAx = if ($colAx) { @($colAx.flags) } else { @() }
$presAx = Open-PptPres $ppt $axOut
$capAx = Get-PptCaption $presAx
$sAx = Get-PptSlide $presAx 1
$shapeAx = Find-PptChartByVals $sAx @(1, 2, 3)
$grp = 0
if ($shapeAx) {
    $chAx = $null
    try { $chAx = $shapeAx.Chart } catch {}
    if ($chAx) {
        $grp = Get-AxisGroup $chAx 2
        Release-Com $chAx
    }
    Release-Com $shapeAx
}
Release-Com $sAx
Close-PptPres $presAx
Add-Check 'pptx-Pr3' (($capAx -notmatch 'Repair') -and ($axis2 -eq 'secondary') -and ($flagsAx -contains 'secondaryAxis') -and ($grp -eq 2)) "caption=$capAx axis=$axis2 comGroup=$grp"
