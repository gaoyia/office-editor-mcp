# xlsx 工作表图表：docs/checklist-xlsx.md 的 Xc1–Xc7。
# 源文件：Excel 亲手另存的工作表柱形图，系列公式指向本表单元格。

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

function Pick-ValueSeries($chart) {
    foreach ($s in @($chart.series)) {
        if ((@(Flatten-Nums $s.values).Count) -ge 2) { return $s }
    }
    return @($chart.series) | Select-Object -First 1
}

function Get-ExcelSheetCharts($ws) {
    $out = New-Object System.Collections.ArrayList
    $shapes = $ws.Shapes
    try {
        $n = [int]$shapes.Count
        for ($i = 1; $i -le $n; $i++) {
            $shp = $shapes.Item($i)
            try {
                $has = $false
                try { $has = [bool]$shp.HasChart } catch {
                    try { $has = ([int]$shp.Type -eq 3) } catch { $has = $false }
                }
                if ($has) { [void]$out.Add($shp.Chart) }
            }
            finally { Release-Com $shp }
        }
    }
    finally { Release-Com $shapes }
    return @($out)
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

function Find-ExcelShapeByName($ws, [string]$name) {
    $shapes = $ws.Shapes
    try {
        $n = [int]$shapes.Count
        for ($i = 1; $i -le $n; $i++) {
            $shp = $shapes.Item($i)
            if ([string]$shp.Name -eq $name) { return $shp }
            Release-Com $shp
        }
    }
    finally { Release-Com $shapes }
    return $null
}

function Get-TreeCell($tree, [string]$id) {
    $sheet = @($tree.sheets) | Where-Object { $_.kind -eq 'worksheet' } | Select-Object -First 1
    foreach ($c in @($sheet.cells)) {
        if ([string]$c.id -eq $id) { return $c.value }
    }
    return $null
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

# Xc1：无 charts 时只有计数；charts:true 时 id / type / 系列值能对上工作表和 COM。
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'xlsx'
} 'xlsx-chart-inspect-list'
$sheet0 = @($tree0.sheets) | Where-Object { $_.kind -eq 'worksheet' } | Select-Object -First 1
$noList = -not ($sheet0.PSObject.Properties.Name -contains 'charts')
Add-Check 'xlsx-Xc1a' (($tree0.chartCount -ge 1) -and ($sheet0.chartCount -ge 1) -and $noList) "wb=$($tree0.chartCount) sheet=$($sheet0.chartCount) noList=$noList"

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'xlsx-chart-inspect-expand'
$sheet = @($tree1.sheets) | Where-Object { $_.name -eq $sheet0.name } | Select-Object -First 1
$charts = @($sheet.charts)
$col = $charts | Where-Object { [string]$_.type -eq 'col' } | Select-Object -First 1
if (-not $col) { $col = $charts | Select-Object -First 1 }
$idOk = $col -and ([string]$col.id -like 'sheet:*/chart:*')
$ser0 = if ($col) { Pick-ValueSeries $col } else { $null }
$serVals = if ($ser0) { @(Flatten-Nums $ser0.values) } else { @() }
$serId = if ($ser0) { [string]$ser0.id } else { '' }

$wbSrc = Open-ExcelBook $excel $src
$sSrc = $wbSrc.Worksheets.Item($sheet.name)
$comCharts = Get-ExcelSheetCharts $sSrc
$comMatch = $false
$comDetail = "excelCharts=$($comCharts.Count)"
foreach ($ch in $comCharts) {
    try {
        $cv = Get-SeriesNums $ch 1
        if ($serVals.Count -gt 0 -and (Test-NumsClose $cv $serVals)) {
            $comMatch = $true
            $comDetail = "com=$($cv -join ',') inspect=$($serVals -join ',')"
            break
        }
        $comDetail = "com=$($cv -join ',') inspect=$($serVals -join ',')"
    }
    catch {
        $comDetail = $_.Exception.Message
    }
}
$srcChartCount = @($comCharts).Count
foreach ($ch in $comCharts) { Release-Com $ch }
Release-Com $sSrc
Close-AllWorkbooks $excel

Add-Check 'xlsx-Xc1' ($idOk -and ($serVals.Count -ge 2) -and $comMatch -and ([string]$col.type -eq 'col')) "id=$($col.id) type=$($col.type) $comDetail valRef=$($ser0.valRef)"

$sheetId = $sheet.id
if (-not $sheetId) { $sheetId = "sheet:$($sheet.name)" }
$colId = if ($col) { [string]$col.id } else { '' }
$nCat = $serVals.Count
if ($nCat -lt 1) { $nCat = 3 }
$newVals = @()
for ($i = 1; $i -le $nCat; $i++) { $newVals += (100 * $i) }

# Xc2：改系列。单元格与图上的值都是新值，Excel 无修复。
$seriesOut = Get-OutPath 'xlsx-chart-series.xlsx'
if (Test-Path $seriesOut) { Remove-Item $seriesOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $seriesOut.Replace('\', '/')
    ops = @(@{ op = 'setChartSeries'; id = $serId; values = $newVals })
} 'xlsx-chart-set-series' | Out-Null

$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $seriesOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'xlsx-chart-series-inspect'
$sheetS = @($treeS.sheets) | Where-Object { $_.name -eq $sheet.name } | Select-Object -First 1
$colS = @($sheetS.charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$gotSer = if ($colS) { Pick-ValueSeries $colS } else { $null }
$gotVals = if ($gotSer) { @(Flatten-Nums $gotSer.values) } else { @() }

$cellsRange = 'A1:B20'
$treeCellsS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $seriesOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = $sheet.name; range = $cellsRange }
} 'xlsx-chart-series-cells'
$cellHit = 0
foreach ($c in @($treeCellsS.sheets[0].cells)) {
    foreach ($nv in $newVals) {
        try {
            if ([double]$c.value -eq [double]$nv) { $cellHit++; break }
        } catch {}
    }
}

$wbS = Open-ExcelBook $excel $seriesOut
$capS = [string]$wbS.Windows.Item(1).Caption
$sS = $wbS.Worksheets.Item($sheet.name)
$comNew = $false
foreach ($ch in (Get-ExcelSheetCharts $sS)) {
    try {
        $cv = Get-SeriesNums $ch 1
        if (Test-NumsClose $cv $newVals) { $comNew = $true }
    }
    catch {}
    finally { Release-Com $ch }
}
Release-Com $sS
Close-AllWorkbooks $excel
Add-Check 'xlsx-Xc2' (($capS -notmatch 'Repair') -and (Test-NumsClose $gotVals $newVals) -and $comNew -and ($cellHit -ge $newVals.Count)) "caption=$capS inspect=$($gotVals -join ',') want=$($newVals -join ',') com=$comNew cells=$cellHit"

# Xc3：往 worksheet 插图，Excel 能看见且无修复；dataFrom 一带有数据表。
$inserted = Get-OutPath 'xlsx-chart-inserted.xlsx'
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $seriesOut.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'
        id = $sheetId
        type = 'col'
        from = 'E12'
        dataFrom = 'A10'
        title = $S.chartTitle
        series = @(@{
            name = $S.chartSer1
            categories = @($S.saItem1, $S.saItem2, $S.saItem3)
            values = @(1, 2, 3)
        })
    })
} 'xlsx-chart-insert' | Out-Null

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'xlsx-chart-insert-inspect'
$sheetI = @($treeI.sheets) | Where-Object { $_.name -eq $sheet.name } | Select-Object -First 1
$oldIds = @($charts | ForEach-Object { $_.id })
$newChart = @($sheetI.charts) | Where-Object { $oldIds -notcontains $_.id } | Select-Object -First 1

$treeData = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = $sheet.name; range = 'A10:B13' }
} 'xlsx-chart-insert-data'
$dataCells = @($treeData.sheets[0].cells)
$hasName = $false
$hasCat = $false
$hasVal = $false
foreach ($c in $dataCells) {
    if ([string]$c.id -eq "cell:$($sheet.name)!B10" -and [string]$c.value -eq $S.chartSer1) { $hasName = $true }
    if ([string]$c.id -eq "cell:$($sheet.name)!A11" -and [string]$c.value -eq $S.saItem1) { $hasCat = $true }
    try { if ([string]$c.id -eq "cell:$($sheet.name)!B11" -and [double]$c.value -eq 1) { $hasVal = $true } } catch {}
}

$wbI = Open-ExcelBook $excel $inserted
$capI = [string]$wbI.Windows.Item(1).Caption
$sI = $wbI.Worksheets.Item($sheet.name)
$chartsI = Get-ExcelSheetCharts $sI
$nI = $chartsI.Count
foreach ($ch in $chartsI) { Release-Com $ch }
Release-Com $sI
Close-AllWorkbooks $excel
Add-Check 'xlsx-Xc3' (($capI -notmatch 'Repair') -and ($null -ne $newChart) -and ($newChart.anchor -eq 'oneCell') -and ($newChart.from -eq 'E12') -and ($nI -gt $srcChartCount) -and $hasName -and $hasCat -and $hasVal) "caption=$capI id=$($newChart.id) from=$($newChart.from) excel=$nI data=$hasName/$hasCat/$hasVal"

# Xc4：删掉刚插的图，数据单元格还在，无修复。
$deleted = Get-OutPath 'xlsx-chart-deleted.xlsx'
if (Test-Path $deleted) { Remove-Item $deleted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $deleted.Replace('\', '/')
    ops = @(@{ op = 'deleteChart'; id = $newChart.id })
} 'xlsx-chart-delete' | Out-Null

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $deleted.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'xlsx-chart-delete-inspect'
$sheetD = @($treeD.sheets) | Where-Object { $_.name -eq $sheet.name } | Select-Object -First 1
$stillNew = @($sheetD.charts) | Where-Object { $_.id -eq $newChart.id } | Select-Object -First 1
$treeDataD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $deleted.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = $sheet.name; range = 'A10:B13' }
} 'xlsx-chart-delete-data'
$stillVal = $false
foreach ($c in @($treeDataD.sheets[0].cells)) {
    try { if ([string]$c.id -eq "cell:$($sheet.name)!B11" -and [double]$c.value -eq 1) { $stillVal = $true } } catch {}
}
$wbD = Open-ExcelBook $excel $deleted
$capD = [string]$wbD.Windows.Item(1).Caption
$sD = $wbD.Worksheets.Item($sheet.name)
$chartsD = Get-ExcelSheetCharts $sD
$nD = $chartsD.Count
foreach ($ch in $chartsD) { Release-Com $ch }
Release-Com $sD
Close-AllWorkbooks $excel
Add-Check 'xlsx-Xc4' (($capD -notmatch 'Repair') -and ($null -eq $stillNew) -and ($sheetD.chartCount -lt $sheetI.chartCount) -and $stillVal -and ($nD -eq $srcChartCount)) "caption=$capD count=$($sheetD.chartCount) excel=$nD src=$srcChartCount data=$stillVal"

# Xc5：柱形改折线。允许样式重置，无修复。
$lined = Get-OutPath 'xlsx-chart-line.xlsx'
if (Test-Path $lined) { Remove-Item $lined -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $seriesOut.Replace('\', '/')
    output = $lined.Replace('\', '/')
    ops = @(@{ op = 'setChartType'; id = $colId; type = 'line' })
} 'xlsx-chart-set-type' | Out-Null

$treeL = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $lined.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'xlsx-chart-line-inspect'
$sheetL = @($treeL.sheets) | Where-Object { $_.name -eq $sheet.name } | Select-Object -First 1
$lineC = @($sheetL.charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$wbL = Open-ExcelBook $excel $lined
$capL = [string]$wbL.Windows.Item(1).Caption
$sL = $wbL.Worksheets.Item($sheet.name)
$lineTypeOk = $false
$lineTypes = @(4, 63, 64, 65, 66, 67)
foreach ($ch in (Get-ExcelSheetCharts $sL)) {
    try {
        if ($lineTypes -contains [int]$ch.ChartType) { $lineTypeOk = $true }
    }
    catch {}
    finally { Release-Com $ch }
}
Release-Com $sL
Close-AllWorkbooks $excel
Add-Check 'xlsx-Xc5' (($capL -notmatch 'Repair') -and ($lineC.type -eq 'line') -and $lineTypeOk) "caption=$capL type=$($lineC.type) excelLine=$lineTypeOk"

# Xc6：对 insert 出来的 oneCell 改尺寸和锚点。
$moved = Get-OutPath 'xlsx-chart-moved.xlsx'
if (Test-Path $moved) { Remove-Item $moved -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $moved.Replace('\', '/')
    ops = @(
        @{
            op = 'setChartSize'
            id = $newChart.id
            cx = 1828800
            cy = 914400
        }
        @{
            op = 'setChartAnchor'
            id = $newChart.id
            from = 'F20'
        }
    )
} 'xlsx-chart-size-anchor' | Out-Null

$treeM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $moved.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'xlsx-chart-moved-inspect'
$sheetM = @($treeM.sheets) | Where-Object { $_.name -eq $sheet.name } | Select-Object -First 1
$oneM = @($sheetM.charts) | Where-Object { $_.id -eq $newChart.id } | Select-Object -First 1
$wbM = Open-ExcelBook $excel $moved
$capM = [string]$wbM.Windows.Item(1).Caption
$sM = $wbM.Worksheets.Item($sheet.name)
$shapeM = $null
if ($oneM -and $oneM.name) { $shapeM = Find-ExcelShapeByName $sM ([string]$oneM.name) }
$wPt = $null
$hPt = $null
$sizeOk = $true
if ($shapeM) {
    $wPt = [double]$shapeM.Width
    $hPt = [double]$shapeM.Height
    $sizeOk = ([math]::Abs($wPt - 144) -lt 8) -and ([math]::Abs($hPt - 72) -lt 8)
    Release-Com $shapeM
}
Release-Com $sM
Close-AllWorkbooks $excel
$inspectOk = $oneM -and ($oneM.anchor -eq 'oneCell') -and ($oneM.from -eq 'F20') -and ([int64]$oneM.cx -eq 1828800) -and ([int64]$oneM.cy -eq 914400)
Add-Check 'xlsx-Xc6' (($capM -notmatch 'Repair') -and $inspectOk -and $sizeOk) "caption=$capM from=$($oneM.from) cx=$($oneM.cx) w=$wPt h=$hPt"

# Xc7：xlsx 上 exportChartData 得到 InvalidField。不开 Excel。
$expOut = Get-OutPath 'xlsx-chart-export.xlsx'
if (Test-Path $expOut) { Remove-Item $expOut -Force }
$raw = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'exportChartData'
    path = $src.Replace('\', '/')
    id = $colId
    output = $expOut.Replace('\', '/')
} 'xlsx-chart-export'
$code = ''
try {
    $errObj = $raw.Stderr | ConvertFrom-Json
    $code = [string]$errObj.error.code
}
catch { $code = $raw.Stderr }
$noWrite = -not (Test-Path $expOut)
Add-Check 'xlsx-Xc7' (($raw.ExitCode -ne 0) -and ($code -eq 'InvalidField') -and $noWrite) "code=$code exit=$($raw.ExitCode) wrote=$(-not $noWrite)"
