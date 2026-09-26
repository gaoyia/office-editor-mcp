# xlsx create 带图表：docs/checklist-xlsx.md 的 C11–C13。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$path = Ensure-XlsxCreateCharts
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true; sheet = 'Sheet1'; range = 'A10:B13' }
} 'xlsx-create-charts-inspect'

$sheet = @($tree.sheets) | Where-Object { $_.name -eq 'Sheet1' } | Select-Object -First 1
$charts = @($sheet.charts)
$c = $charts | Select-Object -First 1
Add-Check 'xlsx-C11' (($null -ne $c) -and ($c.id -eq 'sheet:Sheet1/chart:1') -and ($c.type -eq 'col') -and ($c.from -eq 'E2')) "id=$($c.id) type=$($c.type) from=$($c.from) n=$($charts.Count)"

$ser = @($c.series) | Select-Object -First 1
$vals = @($ser.values)
$cells = @($sheet.cells)
$b11 = $cells | Where-Object { $_.id -eq 'cell:Sheet1!B11' } | Select-Object -First 1
Add-Check 'xlsx-C12' (($vals.Count -eq 3) -and ([int]$vals[0] -eq 1) -and ([int]$vals[1] -eq 2) -and ([int]$vals[2] -eq 3) -and ($null -ne $b11) -and ([int]$b11.value -eq 1)) "values=$($vals -join ',') b11=$($b11.value)"

$excel = Ensure-MsExcel $path
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $path
$cap = [string]$wb.Windows.Item(1).Caption
$s1 = $wb.Worksheets.Item('Sheet1')
$shapes = $s1.Shapes
$n = [int]$shapes.Count
$hasChart = $false
if ($n -ge 1) {
    $shp = $shapes.Item(1)
    try {
        try { $hasChart = [bool]$shp.HasChart } catch { $hasChart = ([int]$shp.Type -eq 3) }
    }
    finally { Release-Com $shp }
}
Release-Com $shapes
Release-Com $s1
Close-AllWorkbooks $excel
Add-Check 'xlsx-C13' (($cap -notmatch 'Repair') -and ($n -ge 1) -and $hasChart) "caption=$cap shapes=$n hasChart=$hasChart"
