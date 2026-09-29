# xlsx 图表稀有类型：docs/checklist-xlsx.md 的 Xq1–Xq5。
# 源文件：Excel 另存工作表柱形图。插入股价 / 曲面 / 复合饼，再测改类型与拒绝。

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

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'xlsx-charts-rare-inspect'
$sheet = @($tree0.sheets) | Select-Object -First 1
$sheetId = if ($sheet.id) { [string]$sheet.id } else { "sheet:$($sheet.name)" }
$chartex = Find-SheetChartByType $tree0 'chartex'

$stockSer = @(
    @{ name = '高'; categories = @($S.saItem1, $S.saItem2, $S.saItem3); values = @(30, 40, 50) }
    @{ name = '低'; categories = @($S.saItem1, $S.saItem2, $S.saItem3); values = @(10, 15, 20) }
    @{ name = '收'; categories = @($S.saItem1, $S.saItem2, $S.saItem3); values = @(20, 25, 35) }
)
$surfaceSer = @(
    @{ name = '甲'; categories = @('A', 'B'); values = @(1, 2) }
    @{ name = '乙'; categories = @('A', 'B'); values = @(3, 3) }
    @{ name = '丙'; categories = @('A', 'B'); values = @(2, 4) }
)
$ofPieSer = @(@{ name = '份额'; categories = @('A', 'B', 'C'); values = @(1, 2, 3) })

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

# Xq1：插入股价。
$r1 = Insert-And-Open 'stock' $stockSer 'E20' 'AA10' $null 'xlsx-charts-rare-stock.xlsx' 'xlsx-charts-rare-stock'
$serN = 0
if ($r1.Chart) { $serN = @($r1.Chart.series).Count }
Add-Check 'xlsx-Xq1' (($r1.Caption -notmatch 'Repair') -and ($null -ne $r1.Chart) -and ($serN -eq 3)) "caption=$($r1.Caption) type=$($r1.Chart.type) ser=$serN"

# Xq2：插入曲面。
$r2 = Insert-And-Open 'surface' $surfaceSer 'E40' 'AF10' $null 'xlsx-charts-rare-surface.xlsx' 'xlsx-charts-rare-surface'
$serN2 = 0
if ($r2.Chart) { $serN2 = @($r2.Chart.series).Count }
Add-Check 'xlsx-Xq2' (($r2.Caption -notmatch 'Repair') -and ($null -ne $r2.Chart) -and ($serN2 -eq 3)) "caption=$($r2.Caption) type=$($r2.Chart.type) ser=$serN2"

# Xq3：插入复合条。
$r3 = Insert-And-Open 'ofPie' $ofPieSer 'E60' 'AK10' @{ ofPieType = 'bar' } 'xlsx-charts-rare-ofpie.xlsx' 'xlsx-charts-rare-ofpie'
$ofOk = $r3.Chart -and ([string]$r3.Chart.ofPieType -eq 'bar')
Add-Check 'xlsx-Xq3' (($r3.Caption -notmatch 'Repair') -and $ofOk) "caption=$($r3.Caption) type=$($r3.Chart.type) ofPieType=$($r3.Chart.ofPieType)"

# Xq4：股价三条转柱形；复合饼转饼。
$bothOut = Get-OutPath 'xlsx-charts-rare-both.xlsx'
$typeOut = Get-OutPath 'xlsx-charts-rare-type.xlsx'
if (Test-Path $bothOut) { Remove-Item -Force $bothOut }
if (Test-Path $typeOut) { Remove-Item -Force $typeOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $bothOut.Replace('\', '/')
    ops = @(
        @{ op = 'insertChart'; id = $sheetId; type = 'stock'; from = 'E20'; dataFrom = 'AA10'; series = $stockSer }
        @{ op = 'insertChart'; id = $sheetId; type = 'ofPie'; from = 'E60'; dataFrom = 'AK10'; series = $ofPieSer }
    )
} 'xlsx-charts-rare-both' | Out-Null
$treeB = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $bothOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ charts = $true }
} 'xlsx-charts-rare-both-inspect'
$stB = Find-SheetChartByType $treeB 'stock'
$ofB = Find-SheetChartByType $treeB 'ofPie'
$xq4Ok = $false
$xq4Detail = 'missing'
if ($stB -and $ofB) {
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $bothOut.Replace('\', '/')
        output = $typeOut.Replace('\', '/')
        ops = @(
            @{ op = 'setChartType'; id = $stB.id; type = 'col' }
            @{ op = 'setChartType'; id = $ofB.id; type = 'pie' }
        )
    } 'xlsx-charts-rare-type' | Out-Null
    $treeT = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $typeOut.Replace('\', '/'); format = 'xlsx'
        scope = @{ charts = $true }
    } 'xlsx-charts-rare-type-inspect'
    $stAfter = $null
    $ofAfter = $null
    foreach ($sh in @($treeT.sheets)) {
        foreach ($c in @($sh.charts)) {
            if ($c.id -eq $stB.id) { $stAfter = $c }
            if ($c.id -eq $ofB.id) { $ofAfter = $c }
        }
    }
    $wbT = Open-ExcelBook $excel $typeOut
    $capT = [string]$wbT.Windows.Item(1).Caption
    Close-AllWorkbooks $excel
    $xq4Ok = ($capT -notmatch 'Repair') -and ([string]$stAfter.type -eq 'col') -and ([string]$ofAfter.type -eq 'pie')
    $xq4Detail = "caption=$capT stock=$($stAfter.type) ofPie=$($ofAfter.type)"
}
Add-Check 'xlsx-Xq4' $xq4Ok $xq4Detail

# Xq5：股价增系列仍拒；chartex 改标题或 insert type=chartex 拒；两条股价 insert 拒。
$addOut = Get-OutPath 'xlsx-charts-rare-add.xlsx'
$rawAdd = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $r1.Out.Replace('\', '/')
    output = $addOut.Replace('\', '/')
    ops = @(@{ op = 'addChartSeries'; id = $r1.Chart.id; values = @(1, 2, 3) })
} 'xlsx-charts-rare-add'
$codeAdd = Get-FailCode $rawAdd
$noAdd = -not (Test-Path $addOut)

$cxOut = Get-OutPath 'xlsx-charts-rare-chartex.xlsx'
if ($chartex) {
    $rawCx = Invoke-OoxpExpectFail @{
        schemaVersion = 1; action = 'apply'
        path = $src.Replace('\', '/')
        output = $cxOut.Replace('\', '/')
        ops = @(@{ op = 'setChartTitle'; id = $chartex.id; title = '不该写' })
    } 'xlsx-charts-rare-chartex-title'
} else {
    $rawCx = Invoke-OoxpExpectFail @{
        schemaVersion = 1; action = 'apply'
        path = $src.Replace('\', '/')
        output = $cxOut.Replace('\', '/')
        ops = @(@{
            op = 'insertChart'; id = $sheetId; type = 'chartex'
            from = 'E20'; dataFrom = 'AA10'
            series = @(@{ values = @(1, 2) })
        })
    } 'xlsx-charts-rare-chartex'
}
$codeCx = Get-FailCode $rawCx
$noCx = -not (Test-Path $cxOut)

$twoOut = Get-OutPath 'xlsx-charts-rare-stock2.xlsx'
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
} 'xlsx-charts-rare-stock2'
$codeTwo = Get-FailCode $rawTwo
$noTwo = -not (Test-Path $twoOut)

$cxWant = if ($chartex) { 'NotImplemented' } else { 'InvalidField' }
$xq5Ok = ($codeAdd -eq 'NotImplemented') -and $noAdd -and ($codeCx -eq $cxWant) -and $noCx -and ($codeTwo -eq 'InvalidField') -and $noTwo
Add-Check 'xlsx-Xq5' $xq5Ok "add=$codeAdd/$noAdd chartex=$codeCx/$noCx stock2=$codeTwo/$noTwo"
