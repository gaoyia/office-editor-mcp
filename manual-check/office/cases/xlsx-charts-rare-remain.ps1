# xlsx 图表稀有剩余：docs/checklist-xlsx.md 的 Xcx1–Xcx5。
# 源文件：Excel 另存工作表柱形图。OHLC / 俯视线框 / 自定义误差线 / chartex funnel。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-XlsxExcelChartSource
$excel = Ensure-MsExcel $src
Close-AllWorkbooks $excel

function Find-SheetChartByType($tree, [string]$type) {
    foreach ($sh in @($tree.sheets)) {
        foreach ($c in @($sh.charts)) {
            if ([string]$c.type -eq $type) { return $c }
        }
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

function Get-FailCode($raw) {
    try { return [string](($raw.Stderr | ConvertFrom-Json).error.code) } catch { return [string]$raw.Stderr }
}

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

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'xlsx-charts-rare-remain-inspect'
$sheet = @($tree0.sheets) | Select-Object -First 1
$sheetId = if ($sheet.id) { [string]$sheet.id } else { "sheet:$($sheet.name)" }
$oldIds = @()
foreach ($sh in @($tree0.sheets)) {
    foreach ($c in @($sh.charts)) { $oldIds += [string]$c.id }
}

$ohlcSer = @(
    @{ name = '开'; categories = @($S.saItem1, $S.saItem2); values = @(20, 25) }
    @{ name = '高'; categories = @($S.saItem1, $S.saItem2); values = @(30, 40) }
    @{ name = '低'; categories = @($S.saItem1, $S.saItem2); values = @(10, 15) }
    @{ name = '收'; categories = @($S.saItem1, $S.saItem2); values = @(22, 28) }
)
$surfaceSer = @(
    @{ name = '甲'; categories = @('A', 'B'); values = @(1, 2) }
    @{ name = '乙'; categories = @('A', 'B'); values = @(3, 3) }
    @{ name = '丙'; categories = @('A', 'B'); values = @(2, 4) }
)
$funnelSer = @(@{ name = '阶段'; categories = @('A', 'B', 'C'); values = @(10, 6, 2) })

function Insert-And-Open([string]$type, $series, [string]$from, [string]$dataFrom, $extra, [string]$outName, [string]$reqName) {
    $out = Get-OutPath $outName
    if (Test-Path $out) { Remove-Item -Force $out }
    $op = @{
        op = 'insertChart'
        id = $sheetId
        type = $type
        from = $from
        dataFrom = $dataFrom
        title = $S.chartTitle
        series = $series
    }
    if ($extra) { foreach ($k in $extra.Keys) { $op[$k] = $extra[$k] } }
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $src.Replace('\', '/')
        output = $out.Replace('\', '/')
        ops = @($op)
    } $reqName | Out-Null
    $tree = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $out.Replace('\', '/'); format = 'xlsx'
        scope = @{ charts = $true }
    } "$reqName-inspect"
    $hit = Find-SheetChartByType $tree $type
    $wb = Open-ExcelBook $excel $out
    $cap = [string]$wb.Windows.Item(1).Caption
    Close-AllWorkbooks $excel
    return @{ Out = $out; Tree = $tree; Chart = $hit; Caption = $cap }
}

# Xcx1：OHLC。
$r1 = Insert-And-Open 'stock' $ohlcSer 'E80' 'AA40' @{ stockKind = 'ohlc' } 'xlsx-charts-rare-remain-ohlc.xlsx' 'xlsx-charts-rare-remain-ohlc'
$kind1 = if ($r1.Chart) { [string]$r1.Chart.stockKind } else { '' }
$serN = 0
if ($r1.Chart) { $serN = @($r1.Chart.series).Count }
Add-Check 'xlsx-Xcx1' (($r1.Caption -notmatch 'Repair') -and ($null -ne $r1.Chart) -and ($kind1 -eq 'ohlc') -and ($serN -eq 4)) "caption=$($r1.Caption) type=$($r1.Chart.type) stockKind=$kind1 ser=$serN"

# Xcx2：俯视线框。
$r2 = Insert-And-Open 'surface' $surfaceSer 'E100' 'AF40' @{ surfaceKind = 'contourWire' } 'xlsx-charts-rare-remain-contour.xlsx' 'xlsx-charts-rare-remain-contour'
$kind2 = if ($r2.Chart) { [string]$r2.Chart.surfaceKind } else { '' }
Add-Check 'xlsx-Xcx2' (($r2.Caption -notmatch 'Repair') -and ($null -ne $r2.Chart) -and ($kind2 -eq 'contourWire')) "caption=$($r2.Caption) type=$($r2.Chart.type) surfaceKind=$kind2"

# Xcx3：自定义误差线。
$colOut = Get-OutPath 'xlsx-charts-rare-remain-col.xlsx'
if (Test-Path $colOut) { Remove-Item -Force $colOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $colOut.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'
        id = $sheetId
        type = 'col'
        from = 'E120'
        dataFrom = 'AK40'
        title = $S.chartTitle
        series = @(
            @{ name = $S.chartSer1; categories = @($S.saItem1, $S.saItem2, $S.saItem3); values = @(1, 2, 3) }
        )
    })
} 'xlsx-charts-rare-remain-col' | Out-Null
$treeCol = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $colOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'xlsx-charts-rare-remain-col-inspect'
$col = $null
foreach ($sh in @($treeCol.sheets)) {
    foreach ($c in @($sh.charts)) {
        if ($oldIds -notcontains [string]$c.id) { $col = $c; break }
    }
}
$serId = if ($col) { [string]$col.series[0].id } else { '' }
$ebOut = Get-OutPath 'xlsx-charts-rare-remain-errbars.xlsx'
if (Test-Path $ebOut) { Remove-Item -Force $ebOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $colOut.Replace('\', '/')
    output = $ebOut.Replace('\', '/')
    ops = @(@{
        op = 'insertErrorBars'
        id = $serId
        valType = 'cust'
        plus = @(0.5, 0.4, 0.3)
        minus = @(0.2, 0.1, 0.1)
    })
} 'xlsx-charts-rare-remain-errbars' | Out-Null
$treeEb = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ebOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'xlsx-charts-rare-remain-errbars-inspect'
$ebChart = $null
foreach ($sh in @($treeEb.sheets)) {
    foreach ($c in @($sh.charts)) {
        if ([string]$c.id -eq [string]$col.id) { $ebChart = $c; break }
    }
}
$bars = $null
if ($ebChart) { $bars = @($ebChart.series[0].errorBars) | Select-Object -First 1 }
$wbEb = Open-ExcelBook $excel $ebOut
$capEb = [string]$wbEb.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$plusOk = $bars -and (Test-NumsClose $bars.plus @(0.5, 0.4, 0.3))
$minusOk = $bars -and (Test-NumsClose $bars.minus @(0.2, 0.1, 0.1))
$vt = if ($bars) { [string]$bars.valType } else { '' }
Add-Check 'xlsx-Xcx3' (($capEb -notmatch 'Repair') -and ($vt -eq 'cust') -and $plusOk -and $minusOk) "caption=$capEb valType=$vt plus=$plusOk minus=$minusOk"

# Xcx4：chartex funnel。
$r4 = Insert-And-Open 'chartex' $funnelSer 'E140' 'AQ40' @{ layoutId = 'funnel' } 'xlsx-charts-rare-remain-funnel.xlsx' 'xlsx-charts-rare-remain-funnel'
$lid = if ($r4.Chart) { [string]$r4.Chart.layoutId } else { '' }
Add-Check 'xlsx-Xcx4' (($r4.Caption -notmatch 'Repair') -and ($null -ne $r4.Chart) -and ([string]$r4.Chart.type -eq 'chartex') -and ($lid -eq 'funnel')) "caption=$($r4.Caption) type=$($r4.Chart.type) layoutId=$lid"

# Xcx5：仍拒绝。
$missOut = Get-OutPath 'xlsx-charts-rare-remain-nolayout.xlsx'
$rawMiss = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $missOut.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'; id = $sheetId; type = 'chartex'
        from = 'E20'; dataFrom = 'AA10'
        series = @(@{ values = @(1, 2) })
    })
} 'xlsx-charts-rare-remain-nolayout'
$codeMiss = Get-FailCode $rawMiss
$noMiss = -not (Test-Path $missOut)

$regOut = Get-OutPath 'xlsx-charts-rare-remain-region.xlsx'
$rawReg = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $regOut.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'; id = $sheetId; type = 'chartex'; layoutId = 'regionMap'
        from = 'E20'; dataFrom = 'AA10'
        series = @(@{ values = @(1) })
    })
} 'xlsx-charts-rare-remain-region'
$codeReg = Get-FailCode $rawReg
$noReg = -not (Test-Path $regOut)

$twoOut = Get-OutPath 'xlsx-charts-rare-remain-stock2.xlsx'
$rawTwo = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $twoOut.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'; id = $sheetId; type = 'stock'
        from = 'E20'; dataFrom = 'AA10'
        series = @(
            @{ values = @(1, 2) }
            @{ values = @(3, 4) }
        )
    })
} 'xlsx-charts-rare-remain-stock2'
$codeTwo = Get-FailCode $rawTwo
$noTwo = -not (Test-Path $twoOut)

$xcx5Ok = ($codeMiss -eq 'InvalidField') -and $noMiss -and ($codeReg -eq 'NotImplemented') -and $noReg -and ($codeTwo -eq 'InvalidField') -and $noTwo
Add-Check 'xlsx-Xcx5' $xcx5Ok "miss=$codeMiss/$noMiss region=$codeReg/$noReg stock2=$codeTwo/$noTwo"
