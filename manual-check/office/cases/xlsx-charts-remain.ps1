# xlsx 图表剩余：docs/checklist-xlsx.md 的 Xr1–Xr4。
# 源文件：Excel 亲手另存的工作表柱形图。趋势线与次轴打在新插的双系列柱形上。

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

function Get-ExcelSeries($chart, [int]$index) {
    try { return $chart.SeriesCollection($index) } catch {
        try { return $chart.FullSeriesCollection($index) } catch { return $null }
    }
}

function Get-SeriesNums($chart, [int]$index) {
    $ser = Get-ExcelSeries $chart $index
    if (-not $ser) { return @() }
    try { return @(Flatten-Nums $ser.Values) }
    finally { Release-Com $ser }
}

function Get-TrendlineCount($chart, [int]$index) {
    $ser = Get-ExcelSeries $chart $index
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
    $ser = Get-ExcelSeries $chart $index
    if (-not $ser) { return 0 }
    try { return [int]$ser.AxisGroup } catch { return 0 }
    finally { Release-Com $ser }
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

function Get-ZipText([string]$zipPath, [string]$part) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        $entry = $zip.Entries | Where-Object { ($_.FullName -replace '\\', '/') -eq $part } | Select-Object -First 1
        if (-not $entry) { return $null }
        $ms = New-Object IO.MemoryStream
        $es = $entry.Open()
        $es.CopyTo($ms)
        $es.Dispose()
        return [Text.Encoding]::UTF8.GetString($ms.ToArray())
    }
    finally { $zip.Dispose() }
}

function Find-ZipPartContaining([string]$zipPath, [string]$prefix, [string]$needle) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        foreach ($e in $zip.Entries) {
            $n = $e.FullName -replace '\\', '/'
            if ($n -like $prefix) {
                $ms = New-Object IO.MemoryStream
                $es = $e.Open()
                $es.CopyTo($ms)
                $es.Dispose()
                $text = [Text.Encoding]::UTF8.GetString($ms.ToArray())
                if ($text.Contains($needle)) { return $n }
            }
        }
    }
    finally { $zip.Dispose() }
    return $null
}

function Rewrite-ZipPart([string]$zipPath, [string]$part, [scriptblock]$edit) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::Open($zipPath, 'Update')
    try {
        $entry = $zip.Entries | Where-Object { ($_.FullName -replace '\\', '/') -eq $part } | Select-Object -First 1
        if (-not $entry) { throw "zip 里没有 $part" }
        $ms = New-Object IO.MemoryStream
        $es = $entry.Open()
        $es.CopyTo($ms)
        $es.Dispose()
        $text = [Text.Encoding]::UTF8.GetString($ms.ToArray())
        $text2 = & $edit $text
        $name = $entry.FullName
        $entry.Delete()
        $newEntry = $zip.CreateEntry($name)
        $bytes = [Text.Encoding]::UTF8.GetBytes($text2)
        $ws = $newEntry.Open()
        $ws.Write($bytes, 0, $bytes.Length)
        $ws.Dispose()
    }
    finally { $zip.Dispose() }
}

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'xlsx-charts-remain-src'
$sheet = @($tree0.sheets) | Where-Object { $_.kind -eq 'worksheet' } | Select-Object -First 1
$sheetName = [string]$sheet.name
$sheetId = if ($sheet.id) { [string]$sheet.id } else { "sheet:$sheetName" }
$oldIds = @(@($sheet.charts) | ForEach-Object { [string]$_.id })

$inserted = Get-OutPath 'xlsx-charts-remain-col.xlsx'
Close-AllWorkbooks $excel
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'
        id = $sheetId
        type = 'col'
        from = 'E12'
        dataFrom = 'A10'
        title = $S.chartTitle
        series = @(
            @{ name = $S.chartSer1; categories = @($S.saItem1, $S.saItem2, $S.saItem3); values = @(1, 2, 3) }
            @{ name = $S.chartSer2; categories = @($S.saItem1, $S.saItem2, $S.saItem3); values = @(4, 5, 6) }
        )
    })
} 'xlsx-charts-remain-insert' | Out-Null

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'xlsx-charts-remain-insert-inspect'
$sheetI = @($treeI.sheets) | Where-Object { $_.name -eq $sheetName } | Select-Object -First 1
$col = @($sheetI.charts) | Where-Object { $oldIds -notcontains [string]$_.id } | Select-Object -First 1
$colId = if ($col) { [string]$col.id } else { '' }
$serA = if ($col) { [string]$col.series[0].id } else { '' }
$serB = if ($col) { [string]$col.series[1].id } else { '' }
$valRef = if ($col) { [string]$col.series[0].valRef } else { '' }

# Xr1：趋势线。
$tlOut = Get-OutPath 'xlsx-charts-remain-trendline.xlsx'
Close-AllWorkbooks $excel
if (Test-Path $tlOut) { Remove-Item $tlOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $tlOut.Replace('\', '/')
    ops = @(@{ op = 'insertTrendline'; id = $serA; type = 'linear' })
} 'xlsx-charts-remain-tl' | Out-Null
$treeTl = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tlOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'xlsx-charts-remain-tl-inspect'
$sheetTl = @($treeTl.sheets) | Where-Object { $_.name -eq $sheetName } | Select-Object -First 1
$colTl = @($sheetTl.charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$tls = if ($colTl) { @($colTl.series[0].trendlines) } else { @() }
$flagsTl = if ($colTl) { @($colTl.flags) } else { @() }
$wbTl = Open-ExcelBook $excel $tlOut
$capTl = [string]$wbTl.Windows.Item(1).Caption
$sTl = $wbTl.Worksheets.Item($sheetName)
$tlCount = 0
foreach ($ch in (Get-ExcelSheetCharts $sTl)) {
    try {
        $cv = Get-SeriesNums $ch 1
        if (Test-NumsClose $cv @(1, 2, 3)) { $tlCount = Get-TrendlineCount $ch 1 }
    }
    catch {}
    finally { Release-Com $ch }
}
Release-Com $sTl
Close-AllWorkbooks $excel
Add-Check 'xlsx-Xr1' (($capTl -notmatch 'Repair') -and ($tls.Count -ge 1) -and ($flagsTl -contains 'trendline') -and ($tlCount -ge 1)) "caption=$capTl n=$($tls.Count) com=$tlCount"

# Xr2：次轴。
$axOut = Get-OutPath 'xlsx-charts-remain-axis.xlsx'
Close-AllWorkbooks $excel
if (Test-Path $axOut) { Remove-Item $axOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $axOut.Replace('\', '/')
    ops = @(@{ op = 'setChartSeriesAxis'; id = $serB; axis = 'secondary' })
} 'xlsx-charts-remain-axis' | Out-Null
$treeAx = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $axOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'xlsx-charts-remain-axis-inspect'
$sheetAx = @($treeAx.sheets) | Where-Object { $_.name -eq $sheetName } | Select-Object -First 1
$colAx = @($sheetAx.charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$axis2 = if ($colAx) { [string]$colAx.series[1].axis } else { '' }
$flagsAx = if ($colAx) { @($colAx.flags) } else { @() }
$wbAx = Open-ExcelBook $excel $axOut
$capAx = [string]$wbAx.Windows.Item(1).Caption
$sAx = $wbAx.Worksheets.Item($sheetName)
$grp = 0
foreach ($ch in (Get-ExcelSheetCharts $sAx)) {
    try {
        $cv = Get-SeriesNums $ch 1
        if (Test-NumsClose $cv @(1, 2, 3)) { $grp = Get-AxisGroup $ch 2 }
    }
    catch {}
    finally { Release-Com $ch }
}
Release-Com $sAx
Close-AllWorkbooks $excel
Add-Check 'xlsx-Xr2' (($capAx -notmatch 'Repair') -and ($axis2 -eq 'secondary') -and ($flagsAx -contains 'secondaryAxis') -and ($grp -eq 2)) "caption=$capAx axis=$axis2 comGroup=$grp"

# Xr3：命名区域。把新图第一条系列的 c:f 改成定义名。
$named = Get-OutPath 'xlsx-charts-remain-named.xlsx'
Close-AllWorkbooks $excel
Copy-Item -Force $inserted $named
$chartPart = Find-ZipPartContaining $named 'xl/charts/chart*.xml' $valRef
if ($chartPart -and $valRef) {
    Rewrite-ZipPart $named $chartPart { param($t) $t.Replace($valRef, 'ChartVals') }
}
Rewrite-ZipPart $named 'xl/workbook.xml' {
    param($t)
    $el = "<definedName name=`"ChartVals`">$valRef</definedName>"
    if ($t -match '</definedNames>') {
        return $t.Replace('</definedNames>', "$el</definedNames>")
    }
    $block = "<definedNames>$el</definedNames>"
    # 必须插在 sheets 之后、calcPr / extLst 之前，否则 Excel 打不开。
    if ($t -match '<calcPr') {
        return $t.Replace('<calcPr', "$block<calcPr")
    }
    if ($t -match '</sheets>') {
        return $t.Replace('</sheets>', "</sheets>$block")
    }
    return $t.Replace('</workbook>', "$block</workbook>")
}
$treeNm = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $named.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'xlsx-charts-remain-named-inspect'
$sheetNm = @($treeNm.sheets) | Where-Object { $_.name -eq $sheetName } | Select-Object -First 1
$colNm = @($sheetNm.charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$nmRef = if ($colNm) { [string]$colNm.series[0].valRef } else { '' }
$nmOut = Get-OutPath 'xlsx-charts-remain-named-out.xlsx'
if (Test-Path $nmOut) { Remove-Item $nmOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $named.Replace('\', '/')
    output = $nmOut.Replace('\', '/')
    ops = @(@{ op = 'setChartSeries'; id = $serA; values = @(90, 80, 70) })
} 'xlsx-charts-remain-named-set' | Out-Null
$treeCells = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $nmOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = $sheetName; range = 'A10:C13' }
} 'xlsx-charts-remain-named-cells'
$hit90 = $false
$hit70 = $false
foreach ($c in @($treeCells.sheets[0].cells)) {
    try { if ([double]$c.value -eq 90) { $hit90 = $true } } catch {}
    try { if ([double]$c.value -eq 70) { $hit70 = $true } } catch {}
}
$xmlAfter = if ($chartPart) { Get-ZipText $nmOut $chartPart } else { '' }
$stillName = $xmlAfter -and $xmlAfter.Contains('ChartVals')
$capNm = 'no-open'
try {
    Close-AllWorkbooks $excel
    Start-Sleep -Seconds 1
    $wbNm = Open-ExcelBook $excel $nmOut
    $capNm = [string]$wbNm.Windows.Item(1).Caption
    Close-AllWorkbooks $excel
} catch {
    $capNm = 'open-fail'
}
Add-Check 'xlsx-Xr3' (($nmRef -eq 'ChartVals') -and $hit90 -and $hit70 -and $stillName -and ($capNm -notmatch 'Repair') -and ($capNm -ne 'open-fail') -and ($capNm -ne 'no-open')) "valRef=$nmRef cells=$hit90/$hit70 keepName=$stillName caption=$capNm"

# Xr4：并集改数；三维仍拒绝。
$union = Get-OutPath 'xlsx-charts-remain-union.xlsx'
Close-AllWorkbooks $excel
Copy-Item -Force $inserted $union
$unionRef = $null
if ($valRef -match '^(.*!)\$?([A-Z]+)\$?(\d+):\$?([A-Z]+)\$?(\d+)$') {
    $prefix = $Matches[1]
    $c1 = $Matches[2]
    $r1 = $Matches[3]
    $c2 = $Matches[4]
    $r2 = $Matches[5]
    $unionRef = "$prefix`$$c1`$$r1,$prefix`$$c2`$$r2"
}
$uPart = Find-ZipPartContaining $union 'xl/charts/chart*.xml' $valRef
if ($uPart -and $valRef -and $unionRef) {
    Rewrite-ZipPart $union $uPart { param($t) $t.Replace($valRef, $unionRef) }
}
$unionOut = Get-OutPath 'xlsx-charts-remain-union-out.xlsx'
if (Test-Path $unionOut) { Remove-Item $unionOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $union.Replace('\', '/')
    output = $unionOut.Replace('\', '/')
    ops = @(@{ op = 'setChartSeries'; id = $serA; values = @(11, 13) })
} 'xlsx-charts-remain-union-set' | Out-Null
$treeU = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $unionOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = $sheetName; range = 'A10:C13' }
} 'xlsx-charts-remain-union-cells'
$hit11 = $false
$hit13 = $false
foreach ($c in @($treeU.sheets[0].cells)) {
    try { if ([double]$c.value -eq 11) { $hit11 = $true } } catch {}
    try { if ([double]$c.value -eq 13) { $hit13 = $true } } catch {}
}
$xmlU = if ($uPart) { Get-ZipText $unionOut $uPart } else { '' }
$keepUnion = $unionRef -and $xmlU.Contains($unionRef)
$capU = 'no-open'
try {
    $wbU = Open-ExcelBook $excel $unionOut
    $capU = [string]$wbU.Windows.Item(1).Caption
    Close-AllWorkbooks $excel
} catch {
    $capU = 'open-fail'
}

$three = Get-OutPath 'xlsx-charts-remain-3d.xlsx'
Copy-Item -Force $inserted $three
$threeRef = $null
if ($valRef -match '^(.*)!(.+)$') {
    $threeRef = ($Matches[1] + ':' + $Matches[1] + '!' + $Matches[2])
}
# Sheet1!$B$11:$B$13 → Sheet1:Sheet1!$B$11:$B$13 仍可能被当三维。规范三维是 Sheet1:Sheet2!A1。
$threeRef = "$sheetName`:Sheet2!`$B`$11"
$tPart = Find-ZipPartContaining $three 'xl/charts/chart*.xml' $valRef
if ($tPart -and $valRef) {
    Rewrite-ZipPart $three $tPart { param($t) $t.Replace($valRef, $threeRef) }
}
$threeOut = Get-OutPath 'xlsx-charts-remain-3d-out.xlsx'
if (Test-Path $threeOut) { Remove-Item $threeOut -Force }
$raw = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $three.Replace('\', '/')
    output = $threeOut.Replace('\', '/')
    ops = @(@{ op = 'setChartSeries'; id = $serA; values = @(1) })
} 'xlsx-charts-remain-3d'
$code = ''
try { $code = [string](($raw.Stderr | ConvertFrom-Json).error.code) } catch { $code = $raw.Stderr }
$noWrite = -not (Test-Path $threeOut)
Add-Check 'xlsx-Xr4' (($capU -notmatch 'Repair') -and ($capU -ne 'open-fail') -and ($capU -ne 'no-open') -and $hit11 -and $hit13 -and $keepUnion -and ($code -eq 'NotImplemented') -and $noWrite) "unionCap=$capU cells=$hit11/$hit13 keep=$keepUnion code=$code wrote=$(-not $noWrite)"
