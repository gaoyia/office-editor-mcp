# Excel 另存去掉 chartex 缓存后，inspect 仍能读到单元格。
$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'chartex-cache.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'chartex-cache-create' | Out-Null
$made = Get-OutPath 'chartex-cache-made.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $made.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'; id = 'sheet:Sheet1'; type = 'chartex'; layoutId = 'waterfall'
        from = 'E2'; dataFrom = 'A1'; title = '水量'
        series = @(@{ name = '值'; categories = @('甲','乙'); values = @(10, 4) })
    })
} 'chartex-cache-insert' | Out-Null

$excel = Ensure-MsExcel $made
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $made
$ws = $wb.Worksheets.Item('Sheet1')
$ws.Range('B2').Value2 = 42
$saved = Get-OutPath 'chartex-cache-saved.xlsx'
if (Test-Path $saved) { Remove-Item -Force $saved }
$wb.SaveAs($saved)
$wb.Close($false)
Release-Com $ws
Release-Com $wb
Close-AllWorkbooks $excel

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $saved.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'chartex-cache-inspect'
$chart = $tree.sheets[0].charts[0]
$ser = $chart.series[0]
$vals = @($ser.values)
$hit = ($vals.Count -ge 1) -and ([string]$vals[0] -eq '42')
Add-Check 'xlsx-Cca1' $hit "values=$($vals -join ',') id=$($chart.id)"

$set = Get-OutPath 'chartex-cache-set.xlsx'
if (Test-Path $set) { Remove-Item -Force $set }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $saved.Replace('\', '/')
    output = $set.Replace('\', '/')
    ops = @(@{
        op = 'setChartSeries'; id = "$($chart.id)/series:0"; values = @(7, 8)
    })
} 'chartex-cache-set' | Out-Null
$wb2 = Open-ExcelBook $excel $set
$cap = [string]$wb2.Windows.Item(1).Caption
$ws2 = $wb2.Worksheets.Item('Sheet1')
$b2 = $ws2.Range('B2').Value2
$b3 = $ws2.Range('B3').Value2
Add-Check 'xlsx-Cca2' (($cap -notmatch 'Repair') -and ($b2 -eq 7) -and ($b3 -eq 8)) "caption=$cap b2=$b2 b3=$b3"
$wb2.Close($false)
Release-Com $ws2
Release-Com $wb2
Close-AllWorkbooks $excel

$layouts = @(
    @{ key = 'funnel'; title = '漏斗'; data = 'A1'; from = 'E2'; cell = 'B2'; series = @(@{ name = '值'; categories = @('甲','乙'); values = @(10, 4) }) }
    @{ key = 'treemap'; title = '树状'; data = 'A10'; from = 'E16'; cell = 'B11'; series = @(@{ name = '值'; categories = @('甲','乙'); values = @(10, 4) }) }
    @{ key = 'sunburst'; title = '旭日'; data = 'A20'; from = 'E30'; cell = 'B21'; series = @(@{ name = '值'; categories = @('甲','乙'); values = @(10, 4) }) }
    @{ key = 'clusteredColumn'; title = '直方'; data = 'A30'; from = 'E44'; cell = 'B31'; series = @(@{ name = '值'; categories = @('甲','乙'); values = @(10, 4) }) }
    @{ key = 'pareto'; title = '帕累托'; data = 'A40'; from = 'E58'; cell = 'B41'; series = @(
        @{ name = '值'; categories = @('甲','乙'); values = @(10, 4) }
        @{ name = '线'; categories = @('甲','乙'); values = @(10, 14) }
    ) }
    @{ key = 'boxWhisker'; title = '箱线'; data = 'A50'; from = 'E72'; cell = 'B51'; series = @(
        @{ name = '甲'; categories = @('一','二'); values = @(10, 4) }
        @{ name = '乙'; categories = @('一','二'); values = @(8, 6) }
        @{ name = '丙'; categories = @('一','二'); values = @(3, 9) }
    ) }
)
$sixSrc = Get-OutPath 'chartex-cache-six.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $sixSrc.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'chartex-cache-six-create' | Out-Null
$ops = @()
foreach ($L in $layouts) {
    $ops += @{
        op = 'insertChart'; id = 'sheet:Sheet1'; type = 'chartex'; layoutId = $L.key
        from = $L.from; dataFrom = $L.data; title = $L.title; series = $L.series
    }
}
$sixMade = Get-OutPath 'chartex-cache-six-made.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sixSrc.Replace('\', '/')
    output = $sixMade.Replace('\', '/')
    ops = $ops
} 'chartex-cache-six-insert' | Out-Null
$wb6 = Open-ExcelBook $excel $sixMade
$cap6 = [string]$wb6.Windows.Item(1).Caption
$ws6 = $wb6.Worksheets.Item('Sheet1')
foreach ($L in $layouts) { $ws6.Range($L.cell).Value2 = 42 }
$sixSaved = Get-OutPath 'chartex-cache-six-saved.xlsx'
if (Test-Path $sixSaved) { Remove-Item -Force $sixSaved }
$wb6.SaveAs($sixSaved)
$wb6.Close($false)
Release-Com $ws6
Release-Com $wb6
Close-AllWorkbooks $excel
$tree6 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sixSaved.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'chartex-cache-six-inspect'
$charts6 = @($tree6.sheets[0].charts)
$ids = @{}
$vals = @{}
foreach ($c in $charts6) {
    $ids[[string]$c.layoutId] = [string]$c.id
    $vals[[string]$c.layoutId] = [string]$c.series[0].values[0]
}
$want = @('funnel','treemap','sunburst','clusteredColumn','pareto','boxWhisker')
$missing = @($want | Where-Object { -not $ids.ContainsKey($_) })
$hit42 = ($vals['funnel'] -eq '42') -and ($vals['clusteredColumn'] -eq '42') -and ($vals['pareto'] -eq '42')
Add-Check 'xlsx-Cca3' (($cap6 -notmatch 'Repair') -and ($charts6.Count -eq 6) -and ($missing.Count -eq 0) -and $hit42) "caption=$cap6 count=$($charts6.Count) missing=$($missing -join ',') funnel=$($vals['funnel']) col=$($vals['clusteredColumn']) pareto=$($vals['pareto'])"

$setOps = @()
foreach ($key in @('funnel','clusteredColumn','pareto')) {
    $setOps += @{ op = 'setChartSeries'; id = "$($ids[$key])/series:0"; values = @(7, 8) }
}
$sixSet = Get-OutPath 'chartex-cache-six-set.xlsx'
if (Test-Path $sixSet) { Remove-Item -Force $sixSet }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sixSaved.Replace('\', '/')
    output = $sixSet.Replace('\', '/')
    ops = $setOps
} 'chartex-cache-six-set' | Out-Null
$wb7 = Open-ExcelBook $excel $sixSet
$cap7 = [string]$wb7.Windows.Item(1).Caption
$ws7 = $wb7.Worksheets.Item('Sheet1')
$b2 = $ws7.Range('B2').Value2
$b31 = $ws7.Range('B31').Value2
$b41 = $ws7.Range('B41').Value2
Add-Check 'xlsx-Cca4' (($cap7 -notmatch 'Repair') -and ($b2 -eq 7) -and ($b31 -eq 7) -and ($b41 -eq 7)) "caption=$cap7 b2=$b2 b31=$b31 b41=$b41"
$wb7.Close($false)
Release-Com $ws7
Release-Com $wb7
Close-AllWorkbooks $excel
