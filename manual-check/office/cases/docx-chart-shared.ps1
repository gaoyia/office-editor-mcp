# docx 图表走共享层：柱形图、改系列、趋势线。Word 能看见图，编辑数据能打开 embedding。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function Get-WordCharts($doc) {
    $out = New-Object System.Collections.ArrayList
    $inlines = $doc.InlineShapes
    try {
        $n = [int]$inlines.Count
        for ($i = 1; $i -le $n; $i++) {
            $ish = $inlines.Item($i)
            try {
                if ([int]$ish.Type -eq 12) { [void]$out.Add($ish.Chart) }
            } finally { Release-Com $ish }
        }
    } finally { Release-Com $inlines }
    $shapes = $doc.Shapes
    try {
        $n = [int]$shapes.Count
        for ($i = 1; $i -le $n; $i++) {
            $shp = $shapes.Item($i)
            try {
                $has = $false
                try { $has = [bool]$shp.HasChart } catch { $has = ([int]$shp.Type -eq 3) }
                if ($has) { [void]$out.Add($shp.Chart) }
            } finally { Release-Com $shp }
        }
    } finally { Release-Com $shapes }
    return @($out)
}

function Test-Embed($chart) {
    $wb = $null
    try { $wb = $chart.ChartData.Workbook } catch { $wb = $null }
    if ($null -eq $wb) {
        try {
            $chart.ChartData.Activate()
            $wb = $chart.ChartData.Workbook
        } catch { $wb = $null }
    }
    $ok = $null -ne $wb
    if ($null -ne $wb) {
        try { $wb.Close($false) } catch {}
        Release-Com $wb
    }
    return $ok
}

function Open-Check([string]$path, [string]$id) {
    $doc = Open-WordDoc $word $path
    $cap = [string]$doc.ActiveWindow.Caption
    $charts = @(Get-WordCharts $doc)
    $embed = $false
    foreach ($ch in $charts) {
        if (Test-Embed $ch) { $embed = $true }
        Release-Com $ch
    }
    Close-WordDoc $doc
    $ok = ($cap -notmatch 'Repair') -and ($charts.Count -ge 1) -and $embed
    Add-Check $id $ok "caption=$cap charts=$($charts.Count) embed=$embed"
}

$hostPath = Get-OutPath 'docx-chart-shared-host.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/')
    document = @{ paragraphs = @(@{ text = '图' }) }
} 'docx-chart-shared-create' | Out-Null
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 5 }
} 'docx-chart-shared-host'
$para = [string]$tree0.paragraphs[0].id

$word = Ensure-MsWord $hostPath

$colOut = Get-OutPath 'docx-chart-shared-col.docx'
if (Test-Path $colOut) { Remove-Item -Force $colOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $colOut.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'; id = $para; type = 'col'; title = '柱'
        series = @(@{ name = '甲'; categories = @('A','B','C'); values = @(1, 2, 3) })
    })
} 'docx-chart-shared-col' | Out-Null
Open-Check $colOut 'docx-chart-shared-col'

$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $colOut.Replace('\', '/'); format = 'docx'
    scope = @{ charts = $true }
} 'docx-chart-shared-col-inspect'
$ser = [string]$treeC.charts[0].series[0].id
$chartId = [string]$treeC.charts[0].id

$serOut = Get-OutPath 'docx-chart-shared-series.docx'
if (Test-Path $serOut) { Remove-Item -Force $serOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $colOut.Replace('\', '/')
    output = $serOut.Replace('\', '/')
    ops = @(@{ op = 'setChartSeries'; id = $ser; values = @(4, 5, 6) })
} 'docx-chart-shared-series' | Out-Null
Open-Check $serOut 'docx-chart-shared-series'

$trOut = Get-OutPath 'docx-chart-shared-trend.docx'
if (Test-Path $trOut) { Remove-Item -Force $trOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $serOut.Replace('\', '/')
    output = $trOut.Replace('\', '/')
    ops = @(@{ op = 'insertTrendline'; id = $ser; type = 'linear' })
} 'docx-chart-shared-trend' | Out-Null
Open-Check $trOut 'docx-chart-shared-trend'
