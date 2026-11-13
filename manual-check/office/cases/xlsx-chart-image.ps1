# xlsx 簇状柱形图只读 PNG。金样是 Excel 16.0 的 Chart.Export。
# 像素差用 pixel-diff 标红。红区要回绘制引擎修，不要加大 MaxDelta。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function New-ColumnChart($from, $dataFrom, $cx, $cy) {
    return @{
        op = 'insertChart'; id = 'sheet:Sheet1'; type = 'col'; anchor = 'oneCell'
        from = $from; dataFrom = $dataFrom; cx = $cx; cy = $cy
        series = @(
            @{ name = 'S1'; categories = @('A', 'B', 'C', 'D'); values = @(1, 4, 3, 2) },
            @{ name = 'S2'; values = @(2, 1, 4, 3) }
        )
    }
}

$src = Get-OutPath 'xlsx-chart-image-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-chart-image-create' | Out-Null

$book = Get-OutPath 'xlsx-chart-image.xlsx'
if (Test-Path $book) { Remove-Item -Force $book }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $book.Replace('\', '/')
    ops = @(
        (New-ColumnChart 'E2' 'A10' 5486400 3200400),
        (New-ColumnChart 'E20' 'A30' 3657600 2286000)
    )
} 'xlsx-chart-image-insert' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $book.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'xlsx-chart-image-inspect'
$charts = @($tree.sheets[0].charts)
Add-Check 'xlsx-chart-image-count' ($charts.Count -eq 2) "count=$($charts.Count)"

$libs = @()
for ($i = 0; $i -lt $charts.Count; $i++) {
    $id = [string]$charts[$i].id
    $png = Get-OutPath ("xlsx-chart-image-lib-{0}.png" -f ($i + 1))
    if (Test-Path $png) { Remove-Item -Force $png }
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'exportChartImage'
        path = $book.Replace('\', '/'); format = 'xlsx'
        id = $id
        output = $png.Replace('\', '/')
    } ("xlsx-chart-image-export-{0}" -f ($i + 1)) | Out-Null
    $libs += $png
}

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$ws = $wb.Worksheets.Item(1)
$golds = @()
$shapes = $ws.Shapes
try {
    $n = [int]$shapes.Count
    $k = 0
    for ($i = 1; $i -le $n; $i++) {
        $shp = $shapes.Item($i)
        $has = $false
        try { $has = [bool]$shp.HasChart } catch { $has = $false }
        if (-not $has) { Release-Com $shp; continue }
        $k++
        $chart = $shp.Chart
        $plot = $chart.PlotArea
        $leg = $null
        try { $leg = $chart.Legend } catch {}
        $legText = ''
        if ($leg) { $legText = " leg=$($leg.Left),$($leg.Top),$($leg.Width),$($leg.Height)" }
        Write-Log ("shape$k w=$($shp.Width) h=$($shp.Height) plot=$($plot.InsideLeft),$($plot.InsideTop),$($plot.InsideWidth),$($plot.InsideHeight)$legText")
        $gold = Get-OutPath ("xlsx-chart-image-gold-{0}.png" -f $k)
        if (Test-Path $gold) { Remove-Item -Force $gold }
        [void]$chart.Export($gold, 'PNG')
        $golds += $gold
        if ($leg) { Release-Com $leg }
        Release-Com $plot
        Release-Com $chart
        Release-Com $shp
    }
}
finally { Release-Com $shapes }
Release-Com $ws
$wb.Close($false)
Release-Com $wb

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
for ($i = 0; $i -lt $libs.Count; $i++) {
    $name = "xlsx-chart-image-px-{0}" -f ($i + 1)
    if ($i -ge $golds.Count) {
        Add-Check $name $false 'Excel 没有导出对应的图'
        continue
    }
    $out = Get-OutPath ("xlsx-chart-image-diff-{0}.png" -f ($i + 1))
    $line = & $diffScript -Left $golds[$i] -Right $libs[$i] -Out $out -MaxDelta 0
    $code = $LASTEXITCODE
    $text = ($line | Out-String).Trim()
    Write-Log "$name exit=$code $text"
    $ok = ($code -eq 0) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
    Add-Check $name $ok "exit=$code $text"
}
