# xlsx create 图表工作表：docs/checklist-xlsx.md 的 C22–C24。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

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

$path = Ensure-XlsxCreateChartSheet
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true; sheet = 'Sheet1'; range = 'A10:B13' }
} 'xlsx-create-chartsheet-inspect'

$cs = @($tree.sheets) | Where-Object { $_.name -eq 'Chart1' } | Select-Object -First 1
$one = $null
if ($cs) { $one = @($cs.charts) | Select-Object -First 1 }
Add-Check 'xlsx-C22' (($null -ne $cs) -and ($cs.kind -eq 'chartsheet') -and ([int]$cs.chartCount -eq 1) -and ($null -ne $one) -and ($one.id -eq 'sheet:Chart1/chart:2')) "kind=$($cs.kind) count=$($cs.chartCount) id=$($one.id)"

$sheet = @($tree.sheets) | Where-Object { $_.name -eq 'Sheet1' } | Select-Object -First 1
$cells = @($sheet.cells)
$b11 = $cells | Where-Object { $_.id -eq 'cell:Sheet1!B11' } | Select-Object -First 1
$vals = @()
if ($one) { $vals = @($one.series[0].values) }
Add-Check 'xlsx-C23' ((Test-NumsClose $vals @(1, 2, 3)) -and ($null -ne $b11) -and ([int]$b11.value -eq 1)) "values=$($vals -join ',') b11=$($b11.value)"

$excel = Ensure-MsExcel $path
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $path
$cap = [string]$wb.Windows.Item(1).Caption
$excelCs = $null
try { $excelCs = $wb.Charts.Item('Chart1') } catch {
    try { $excelCs = $wb.Sheets.Item('Chart1') } catch { $excelCs = $null }
}
$excelKindOk = $false
$comVals = @()
if ($excelCs) {
    $excelKindOk = $true
    $ch = $null
    $ownChart = $false
    try {
        $tmp = $excelCs.Chart
        if ($tmp) { $ch = $tmp; $ownChart = $true }
    } catch {}
    if (-not $ch) { $ch = $excelCs }
    $comVals = @(Get-SeriesNums $ch 1)
    if ($ownChart -and $ch) { Release-Com $ch }
    Release-Com $excelCs
}
Close-AllWorkbooks $excel
Add-Check 'xlsx-C24' (($cap -notmatch 'Repair') -and $excelKindOk -and (Test-NumsClose $comVals @(1, 2, 3))) "caption=$cap excel=$excelKindOk com=$($comVals -join ',')"
