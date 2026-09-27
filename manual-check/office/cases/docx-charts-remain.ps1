# docx 图表剩余：docs/checklist-docx.md 的 Cr1–Cr6。
# 源文件：Word 另存的扩展图表样例（含股价），再 insertChart 一张双系列柱形。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-DocxWordChartsExtSource
$word = Ensure-MsWord $src

function Find-ChartByType($tree, [string]$type) {
    foreach ($c in @($tree.charts)) {
        if ([string]$c.type -eq $type) { return $c }
    }
    return $null
}

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

function Get-SeriesNums($chart, [int]$index) {
    $ser = $null
    try { $ser = $chart.SeriesCollection($index) } catch {
        try { $ser = $chart.FullSeriesCollection($index) } catch { return @() }
    }
    try {
        return @(Flatten-Nums $ser.Values)
    }
    finally { if ($ser) { Release-Com $ser } }
}

function Get-WordSeries($chart, [int]$index) {
    try { return $chart.SeriesCollection($index) } catch {
        try { return $chart.FullSeriesCollection($index) } catch { return $null }
    }
}

function Get-TrendlineCount($chart, [int]$index) {
    $ser = Get-WordSeries $chart $index
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
    $ser = Get-WordSeries $chart $index
    if (-not $ser) { return 0 }
    try { return [int]$ser.AxisGroup } catch { return 0 }
    finally { Release-Com $ser }
}

function Find-ComChartByVals($doc, $want) {
    foreach ($ch in (Get-WordChartObjects $doc)) {
        $cv = Get-SeriesNums $ch 1
        if (Test-NumsClose $cv $want) { return $ch }
        Release-Com $ch
    }
    return $null
}

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 30; charts = $true }
} 'docx-charts-remain-inspect'
$stock = Find-ChartByType $tree 'stock'
$para1 = $tree.paragraphs[0].id
$oldIds = @(@($tree.charts) | ForEach-Object { [string]$_.id })

# Cr1：股价 type=stock。
Add-Check 'docx-Cr1' (($null -ne $stock) -and ($stock.type -eq 'stock')) "type=$($stock.type) id=$($stock.id)"

# Cr2：股价改数。
$ser0 = if ($stock) { @($stock.series) | Select-Object -First 1 } else { $null }
$serId = if ($ser0) { [string]$ser0.id } else { '' }
$oldVals = if ($ser0) { @(Flatten-Nums $ser0.values) } else { @() }
$nCat = $oldVals.Count
if ($nCat -lt 1) { $nCat = 3 }
$newVals = @()
for ($i = 1; $i -le $nCat; $i++) { $newVals += (10 * $i) }
$serOut = Get-OutPath 'docx-charts-remain-stock.docx'
Close-WordDocByPath $word $serOut
if (Test-Path $serOut) { Remove-Item -Force $serOut }
$cr2Ok = $false
$cr2Detail = 'no-stock'
if ($serId) {
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $src.Replace('\', '/')
        output = $serOut.Replace('\', '/')
        ops = @(@{ op = 'setChartSeries'; id = $serId; values = $newVals })
    } 'docx-charts-remain-stock' | Out-Null
    $treeS = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $serOut.Replace('\', '/'); format = 'docx'
        scope = @{ charts = $true }
    } 'docx-charts-remain-stock-inspect'
    $stockS = @($treeS.charts) | Where-Object { $_.id -eq $stock.id } | Select-Object -First 1
    $got = if ($stockS) { @(Flatten-Nums $stockS.series[0].values) } else { @() }
    $docS = Open-WordDoc $word $serOut
    $capS = [string]$docS.ActiveWindow.Caption
    $comHit = $false
    foreach ($ch in (Get-WordChartObjects $docS)) {
        try {
            $cv = Get-SeriesNums $ch 1
            if (Test-NumsClose $cv $newVals) { $comHit = $true }
        }
        catch {}
        finally { Release-Com $ch }
    }
    Close-WordDoc $docS
    $cr2Ok = ($capS -notmatch 'Repair') -and (Test-NumsClose $got $newVals) -and $comHit
    $cr2Detail = "caption=$capS inspect=$($got -join ',') want=$($newVals -join ',') com=$comHit"
}
Add-Check 'docx-Cr2' $cr2Ok $cr2Detail

# 插一张双系列柱形，后面的趋势线 / 误差线 / 次轴都打在这张上。
$insOut = Get-OutPath 'docx-charts-remain-col.docx'
Close-WordDocByPath $word $insOut
if (Test-Path $insOut) { Remove-Item -Force $insOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'
        id = $para1
        type = 'col'
        title = $S.chartTitle
        series = @(
            @{ name = $S.chartSer1; categories = @($S.saItem1, $S.saItem2, $S.saItem3); values = @(1, 2, 3) }
            @{ name = $S.chartSer2; categories = @($S.saItem1, $S.saItem2, $S.saItem3); values = @(4, 5, 6) }
        )
    })
} 'docx-charts-remain-insert' | Out-Null
$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'docx'
    scope = @{ charts = $true }
} 'docx-charts-remain-insert-inspect'
$col = @($treeI.charts) | Where-Object {
    ($oldIds -notcontains [string]$_.id) -and ([string]$_.type -eq 'col') -and (@($_.flags) -notcontains 'pivot')
} | Select-Object -First 1
$colId = if ($col) { [string]$col.id } else { '' }
$serA = if ($col) { [string]$col.series[0].id } else { '' }
$serB = if ($col) { [string]$col.series[1].id } else { '' }

# Cr3：insertTrendline。
$tlOut = Get-OutPath 'docx-charts-remain-trendline.docx'
Close-WordDocByPath $word $tlOut
if (Test-Path $tlOut) { Remove-Item -Force $tlOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $tlOut.Replace('\', '/')
    ops = @(@{ op = 'insertTrendline'; id = $serA; type = 'linear' })
} 'docx-charts-remain-tl' | Out-Null
$treeTl = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tlOut.Replace('\', '/'); format = 'docx'
    scope = @{ charts = $true }
} 'docx-charts-remain-tl-inspect'
$colTl = @($treeTl.charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$tls = if ($colTl) { @($colTl.series[0].trendlines) } else { @() }
$flagsTl = if ($colTl) { @($colTl.flags) } else { @() }
$tlId = if ($tls.Count) { [string]$tls[0].id } else { '' }
$docTl = Open-WordDoc $word $tlOut
$capTl = [string]$docTl.ActiveWindow.Caption
$chTl = Find-ComChartByVals $docTl @(1, 2, 3)
$tlCount = 0
if ($chTl) {
    $tlCount = Get-TrendlineCount $chTl 1
    Release-Com $chTl
}
Close-WordDoc $docTl
Add-Check 'docx-Cr3' (($capTl -notmatch 'Repair') -and ($tls.Count -ge 1) -and ($flagsTl -contains 'trendline') -and ($tlCount -ge 1)) "caption=$capTl n=$($tls.Count) flags=$($flagsTl -join ',') com=$tlCount"

# Cr4：改成 exp 再删。
$tlSet = Get-OutPath 'docx-charts-remain-tl-set.docx'
Close-WordDocByPath $word $tlSet
if (Test-Path $tlSet) { Remove-Item -Force $tlSet }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $tlOut.Replace('\', '/')
    output = $tlSet.Replace('\', '/')
    ops = @(@{ op = 'setTrendline'; id = $tlId; type = 'exp' })
} 'docx-charts-remain-tl-set' | Out-Null
$treeSet = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tlSet.Replace('\', '/'); format = 'docx'
    scope = @{ charts = $true }
} 'docx-charts-remain-tl-set-inspect'
$colSet = @($treeSet.charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$tySet = if ($colSet) { [string]$colSet.series[0].trendlines[0].type } else { '' }
$tlId2 = if ($colSet) { [string]$colSet.series[0].trendlines[0].id } else { '' }
$docSet = Open-WordDoc $word $tlSet
$capSet = [string]$docSet.ActiveWindow.Caption
Close-WordDoc $docSet
$tlDel = Get-OutPath 'docx-charts-remain-tl-del.docx'
Close-WordDocByPath $word $tlDel
if (Test-Path $tlDel) { Remove-Item -Force $tlDel }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $tlSet.Replace('\', '/')
    output = $tlDel.Replace('\', '/')
    ops = @(@{ op = 'deleteTrendline'; id = $tlId2 })
} 'docx-charts-remain-tl-del' | Out-Null
$treeDel = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tlDel.Replace('\', '/'); format = 'docx'
    scope = @{ charts = $true }
} 'docx-charts-remain-tl-del-inspect'
$colDel = @($treeDel.charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$gone = $colDel -and -not ($colDel.series[0].PSObject.Properties.Name -contains 'trendlines')
$docDel = Open-WordDoc $word $tlDel
$capDel = [string]$docDel.ActiveWindow.Caption
Close-WordDoc $docDel
Add-Check 'docx-Cr4' (($capSet -notmatch 'Repair') -and ($tySet -eq 'exp') -and ($capDel -notmatch 'Repair') -and $gone) "setCap=$capSet type=$tySet delCap=$capDel gone=$gone"

# Cr5：误差线。
$ebOut = Get-OutPath 'docx-charts-remain-eb.docx'
Close-WordDocByPath $word $ebOut
if (Test-Path $ebOut) { Remove-Item -Force $ebOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $ebOut.Replace('\', '/')
    ops = @(@{ op = 'insertErrorBars'; id = $serA; valType = 'fixedVal'; value = 1.5 })
} 'docx-charts-remain-eb' | Out-Null
$treeEb = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ebOut.Replace('\', '/'); format = 'docx'
    scope = @{ charts = $true }
} 'docx-charts-remain-eb-inspect'
$colEb = @($treeEb.charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$ebs = if ($colEb) { @($colEb.series[0].errorBars) } else { @() }
$ebId = if ($ebs.Count) { [string]$ebs[0].id } else { '' }
$docEb = Open-WordDoc $word $ebOut
$capEb = [string]$docEb.ActiveWindow.Caption
Close-WordDoc $docEb
$ebDel = Get-OutPath 'docx-charts-remain-eb-del.docx'
Close-WordDocByPath $word $ebDel
if (Test-Path $ebDel) { Remove-Item -Force $ebDel }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $ebOut.Replace('\', '/')
    output = $ebDel.Replace('\', '/')
    ops = @(@{ op = 'deleteErrorBars'; id = $ebId })
} 'docx-charts-remain-eb-del' | Out-Null
$treeEbD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ebDel.Replace('\', '/'); format = 'docx'
    scope = @{ charts = $true }
} 'docx-charts-remain-eb-del-inspect'
$colEbD = @($treeEbD.charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$ebGone = $colEbD -and -not ($colEbD.series[0].PSObject.Properties.Name -contains 'errorBars')
$docEbD = Open-WordDoc $word $ebDel
$capEbD = [string]$docEbD.ActiveWindow.Caption
Close-WordDoc $docEbD
Add-Check 'docx-Cr5' (($capEb -notmatch 'Repair') -and ($ebs.Count -ge 1) -and ($ebs[0].valType -eq 'fixedVal') -and $ebGone -and ($capEbD -notmatch 'Repair')) "insCap=$capEb n=$($ebs.Count) delCap=$capEbD gone=$ebGone"

# Cr6：次轴。
$axOut = Get-OutPath 'docx-charts-remain-axis.docx'
Close-WordDocByPath $word $axOut
if (Test-Path $axOut) { Remove-Item -Force $axOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $axOut.Replace('\', '/')
    ops = @(@{ op = 'setChartSeriesAxis'; id = $serB; axis = 'secondary' })
} 'docx-charts-remain-axis' | Out-Null
$treeAx = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $axOut.Replace('\', '/'); format = 'docx'
    scope = @{ charts = $true }
} 'docx-charts-remain-axis-inspect'
$colAx = @($treeAx.charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$axis1 = if ($colAx) { [string]$colAx.series[0].axis } else { '' }
$axis2 = if ($colAx) { [string]$colAx.series[1].axis } else { '' }
$flagsAx = if ($colAx) { @($colAx.flags) } else { @() }
$docAx = Open-WordDoc $word $axOut
$capAx = [string]$docAx.ActiveWindow.Caption
$chAx = Find-ComChartByVals $docAx @(1, 2, 3)
$grp = 0
if ($chAx) {
    $grp = Get-AxisGroup $chAx 2
    Release-Com $chAx
}
Close-WordDoc $docAx
Add-Check 'docx-Cr6' (($capAx -notmatch 'Repair') -and ($axis1 -eq 'primary') -and ($axis2 -eq 'secondary') -and ($flagsAx -contains 'secondaryAxis') -and ($grp -eq 2)) "caption=$capAx a1=$axis1 a2=$axis2 flags=$($flagsAx -join ',') comGroup=$grp"
