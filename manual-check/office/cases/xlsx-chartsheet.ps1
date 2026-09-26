# xlsx 图表工作表：docs/checklist-xlsx.md 的 Xs1–Xs4。
# 源文件：Excel 亲手另存的带工作表柱形图工作簿；手测里再用 insertChartSheet 加一张。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-XlsxExcelChartSource
$excel = Ensure-MsExcel $src
Close-AllWorkbooks $excel

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

function Invoke-OoxpExpectFail($obj, [string]$name) {
    $path = Join-Path $global:OoxpReqDir "$name.json"
    Write-JsonFile $path $obj
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $global:OoxpExe
    $psi.Arguments = "--input `"$path`""
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = [System.Text.Encoding]::UTF8
    $psi.StandardErrorEncoding = [System.Text.Encoding]::UTF8
    $proc = [System.Diagnostics.Process]::Start($psi)
    $stdout = $proc.StandardOutput.ReadToEnd().Trim()
    $stderr = $proc.StandardError.ReadToEnd().Trim()
    $proc.WaitForExit()
    $code = $proc.ExitCode
    $proc.Dispose()
    Write-Log "ooxp $name 退出码 $code stderr=$stderr"
    return @{ ExitCode = $code; Stderr = $stderr; Stdout = $stdout }
}

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'xlsx-chartsheet-src-inspect'
$dataSheet = @($tree0.sheets) | Where-Object { $_.kind -eq 'worksheet' } | Select-Object -First 1
$dataName = [string]$dataSheet.name

$inserted = Get-OutPath 'xlsx-chartsheet-inserted.xlsx'
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{
        op = 'insertChartSheet'
        id = 'xlsx:workbook'
        name = 'Chart1'
        dataFrom = 'A10'
        type = 'col'
        title = $S.chartTitle
        series = @(@{
            name = $S.chartSer1
            categories = @($S.saItem1, $S.saItem2, $S.saItem3)
            values = @(1, 2, 3)
        })
    })
} 'xlsx-chartsheet-insert' | Out-Null

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'xlsx-chartsheet-insert-inspect'
$cs = @($treeI.sheets) | Where-Object { $_.name -eq 'Chart1' } | Select-Object -First 1
$one = $null
if ($cs) { $one = @($cs.charts) | Select-Object -First 1 }

$wbI = Open-ExcelBook $excel $inserted
$capI = [string]$wbI.Windows.Item(1).Caption
$excelCs = $null
try { $excelCs = $wbI.Charts.Item('Chart1') } catch {
    try { $excelCs = $wbI.Sheets.Item('Chart1') } catch { $excelCs = $null }
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
Add-Check 'xlsx-Xs1' (($capI -notmatch 'Repair') -and ($null -ne $cs) -and ($cs.kind -eq 'chartsheet') -and ([int]$cs.chartCount -eq 1) -and $excelKindOk) "caption=$capI kind=$($cs.kind) count=$($cs.chartCount) excel=$excelKindOk"

$treeData = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = $dataName; range = 'A10:B13' }
} 'xlsx-chartsheet-insert-data'
$dataSheet2 = @($treeData.sheets) | Where-Object { $_.name -eq $dataName } | Select-Object -First 1
$dataCells = @($dataSheet2.cells)
$hasName = $false
$hasCat = $false
$hasVal = $false
foreach ($c in $dataCells) {
    if ([string]$c.id -eq "cell:${dataName}!B10" -and [string]$c.value -eq $S.chartSer1) { $hasName = $true }
    if ([string]$c.id -eq "cell:${dataName}!A11" -and [string]$c.value -eq $S.saItem1) { $hasCat = $true }
    try { if ([string]$c.id -eq "cell:${dataName}!B11" -and [double]$c.value -eq 1) { $hasVal = $true } } catch {}
}
$inspectVals = @()
if ($one) { $inspectVals = @($one.series[0].values) }
Add-Check 'xlsx-Xs2' ((Test-NumsClose $inspectVals @(1, 2, 3)) -and (Test-NumsClose $comVals @(1, 2, 3)) -and $hasName -and $hasCat -and $hasVal) "inspect=$($inspectVals -join ',') com=$($comVals -join ',') data=$hasName/$hasCat/$hasVal"

$deleted = Get-OutPath 'xlsx-chartsheet-deleted.xlsx'
if (Test-Path $deleted) { Remove-Item $deleted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $deleted.Replace('\', '/')
    ops = @(@{
        op = 'deleteChartSheet'
        id = 'sheet:Chart1'
    })
} 'xlsx-chartsheet-delete' | Out-Null

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $deleted.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true; sheet = $dataName; range = 'A10:B13' }
} 'xlsx-chartsheet-delete-inspect'
$gone = @($treeD.sheets) | Where-Object { $_.name -eq 'Chart1' }
$sheetD = @($treeD.sheets) | Where-Object { $_.name -eq $dataName } | Select-Object -First 1
$stillVal = $false
foreach ($c in @($sheetD.cells)) {
    try { if ([string]$c.id -eq "cell:${dataName}!B11" -and [double]$c.value -eq 1) { $stillVal = $true } } catch {}
}
$wbD = Open-ExcelBook $excel $deleted
$capD = [string]$wbD.Windows.Item(1).Caption
$stillThere = $false
try {
    $sGone = $wbD.Sheets.Item('Chart1')
    $stillThere = $true
    Release-Com $sGone
} catch { $stillThere = $false }
Close-AllWorkbooks $excel
Add-Check 'xlsx-Xs3' (($capD -notmatch 'Repair') -and ($gone.Count -eq 0) -and $stillVal -and (-not $stillThere)) "caption=$capD gone=$($gone.Count) cells=$stillVal excelSheet=$stillThere"

$raw = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = (Get-OutPath 'xlsx-chartsheet-reject.xlsx').Replace('\', '/')
    ops = @(@{
        op = 'deleteChartSheet'
        id = "sheet:$dataName"
    })
} 'xlsx-chartsheet-reject-ws'
$code = ''
try {
    $errObj = $raw.Stderr | ConvertFrom-Json
    $code = [string]$errObj.error.code
}
catch { $code = $raw.Stderr }
Add-Check 'xlsx-Xs4' (($raw.ExitCode -ne 0) -and ($code -eq 'InvalidId')) "code=$code exit=$($raw.ExitCode)"
