# 删掉第一个系列后，给原来第二个系列加趋势线。
# Excel 16.0 打开后趋势线挂在 Q2 上，不报修复。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$hostPath = Get-OutPath 'xlsx-chart-id-pin-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
        })
    }
} 'xlsx-chart-id-pin-create' | Out-Null

$mid = Get-OutPath 'xlsx-chart-id-pin-mid.xlsx'
if (Test-Path $mid) { Remove-Item -Force $mid }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $mid.Replace('\', '/')
    ops = @(
        @{
            op = 'insertChart'; id = 'sheet:Sheet1'; type = 'col'; from = 'E2'; dataFrom = 'A10'
            series = @(
                @{ name = 'Q1'; categories = @('甲', '乙'); values = @(1, 2) },
                @{ name = 'Q2'; categories = @('甲', '乙'); values = @(3, 4) }
            )
        }
    )
} 'xlsx-chart-id-pin-chart' | Out-Null

$out = Get-OutPath 'xlsx-chart-id-pin.xlsx'
if (Test-Path $out) { Remove-Item -Force $out }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $mid.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(
        @{ op = 'deleteChartSeries'; id = 'sheet:Sheet1/chart:1/series:0' },
        @{ op = 'insertTrendline'; id = 'sheet:Sheet1/chart:1/series:1'; type = 'linear' }
    )
} 'xlsx-chart-id-pin-apply' | Out-Null

$excel = Ensure-MsExcel $out
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $out
$cap = [string]$wb.Windows.Item(1).Caption
$s1 = $wb.Worksheets.Item('Sheet1')
$name = ''
$count = -1
$trend = -1
try {
    $chart = $s1.ChartObjects().Item(1).Chart
    $count = [int]$chart.SeriesCollection().Count
    $ser = $chart.SeriesCollection(1)
    $name = [string]$ser.Name
    $trend = [int]$ser.Trendlines().Count
} catch {
    $name = "err:$($_.Exception.Message)"
}
$noRepair = $cap -notmatch 'Repair'
$ok = $noRepair -and ($count -eq 1) -and ($name -eq 'Q2') -and ($trend -eq 1)
Add-Check 'xlsx-chart-id-pin' $ok "repair=$noRepair count=$count name=$name trend=$trend"
Close-AllWorkbooks $excel
