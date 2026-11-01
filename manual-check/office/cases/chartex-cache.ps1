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
