# docx 图表稀有剩余：docs/checklist-docx.md 的 Ccx1–Ccx5。
# 源文件：Word 另存柱形图。OHLC / 俯视线框 / 自定义误差线 / chartex funnel。
# Word 对 funnel 的 AddChart2 会失败，必须 COM 打开手写 chartex。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-DocxWordChartsSource
$word = Ensure-MsWord $src

function Find-ChartByType($tree, [string]$type) {
    foreach ($c in @($tree.charts)) {
        if ([string]$c.type -eq $type) { return $c }
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

function Stop-ChartEmbedProcess {
    Get-Process EXCEL -ErrorAction SilentlyContinue |
        Where-Object { $_.MainWindowTitle -match '图表|Chart' } |
        ForEach-Object {
            $procId = $_.Id
            Write-Log "结束残留 Excel $procId $($_.MainWindowTitle)"
            Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
        }
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
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; charts = $true }
} 'docx-charts-rare-remain-inspect'
$para1 = $tree0.paragraphs[0].id
$oldIds = @(@($tree0.charts) | ForEach-Object { [string]$_.id })

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

function Insert-And-Open([string]$type, $series, $extra, [string]$outName, [string]$reqName) {
    $out = Get-OutPath $outName
    Close-WordDocByPath $word $out
    if (Test-Path $out) { Remove-Item -Force $out }
    $op = @{
        op = 'insertChart'
        id = $para1
        type = $type
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
        path = $out.Replace('\', '/'); format = 'docx'
        scope = @{ from = 1; maxParas = 20; charts = $true }
    } "$reqName-inspect"
    $hit = Find-ChartByType $tree $type
    $doc = Open-WordDoc $word $out
    $cap = [string]$doc.ActiveWindow.Caption
    Close-WordDoc $doc
    Stop-ChartEmbedProcess
    return @{ Out = $out; Tree = $tree; Chart = $hit; Caption = $cap }
}

# Ccx1：OHLC。
$r1 = Insert-And-Open 'stock' $ohlcSer @{ stockKind = 'ohlc' } 'docx-charts-rare-remain-ohlc.docx' 'docx-charts-rare-remain-ohlc'
$kind1 = if ($r1.Chart) { [string]$r1.Chart.stockKind } else { '' }
$serN = 0
if ($r1.Chart) { $serN = @($r1.Chart.series).Count }
Add-Check 'docx-Ccx1' (($r1.Caption -notmatch 'Repair') -and ($null -ne $r1.Chart) -and ($kind1 -eq 'ohlc') -and ($serN -eq 4)) "caption=$($r1.Caption) type=$($r1.Chart.type) stockKind=$kind1 ser=$serN"

# Ccx2：俯视线框。
$r2 = Insert-And-Open 'surface' $surfaceSer @{ surfaceKind = 'contourWire' } 'docx-charts-rare-remain-contour.docx' 'docx-charts-rare-remain-contour'
$kind2 = if ($r2.Chart) { [string]$r2.Chart.surfaceKind } else { '' }
Add-Check 'docx-Ccx2' (($r2.Caption -notmatch 'Repair') -and ($null -ne $r2.Chart) -and ($kind2 -eq 'contourWire')) "caption=$($r2.Caption) type=$($r2.Chart.type) surfaceKind=$kind2"

# Ccx3：自定义误差线。
$colOut = Get-OutPath 'docx-charts-rare-remain-col.docx'
Close-WordDocByPath $word $colOut
if (Test-Path $colOut) { Remove-Item -Force $colOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $colOut.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'
        id = $para1
        type = 'col'
        title = $S.chartTitle
        series = @(
            @{ name = $S.chartSer1; categories = @($S.saItem1, $S.saItem2, $S.saItem3); values = @(1, 2, 3) }
        )
    })
} 'docx-charts-rare-remain-col' | Out-Null
$treeCol = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $colOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; charts = $true }
} 'docx-charts-rare-remain-col-inspect'
$col = @($treeCol.charts) | Where-Object { $oldIds -notcontains [string]$_.id } | Select-Object -First 1
$serId = if ($col) { [string]$col.series[0].id } else { '' }
$ebOut = Get-OutPath 'docx-charts-rare-remain-errbars.docx'
Close-WordDocByPath $word $ebOut
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
} 'docx-charts-rare-remain-errbars' | Out-Null
$treeEb = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ebOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; charts = $true }
} 'docx-charts-rare-remain-errbars-inspect'
$ebChart = @($treeEb.charts) | Where-Object { [string]$_.id -eq [string]$col.id } | Select-Object -First 1
$bars = $null
if ($ebChart) { $bars = @($ebChart.series[0].errorBars) | Select-Object -First 1 }
$docEb = Open-WordDoc $word $ebOut
$capEb = [string]$docEb.ActiveWindow.Caption
Close-WordDoc $docEb
$plusOk = $bars -and (Test-NumsClose $bars.plus @(0.5, 0.4, 0.3))
$minusOk = $bars -and (Test-NumsClose $bars.minus @(0.2, 0.1, 0.1))
$vt = if ($bars) { [string]$bars.valType } else { '' }
Add-Check 'docx-Ccx3' (($capEb -notmatch 'Repair') -and ($vt -eq 'cust') -and $plusOk -and $minusOk) "caption=$capEb valType=$vt plus=$plusOk minus=$minusOk"

# Ccx4：chartex funnel。必须 COM 打开手写部件，不能用 Word AddChart。
$r4 = Insert-And-Open 'chartex' $funnelSer @{ layoutId = 'funnel' } 'docx-charts-rare-remain-funnel.docx' 'docx-charts-rare-remain-funnel'
$lid = if ($r4.Chart) { [string]$r4.Chart.layoutId } else { '' }
Add-Check 'docx-Ccx4' (($r4.Caption -notmatch 'Repair') -and ($null -ne $r4.Chart) -and ([string]$r4.Chart.type -eq 'chartex') -and ($lid -eq 'funnel')) "caption=$($r4.Caption) type=$($r4.Chart.type) layoutId=$lid"

# Ccx5：仍拒绝。
$missOut = Get-OutPath 'docx-charts-rare-remain-nolayout.docx'
$rawMiss = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $missOut.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'; id = $para1; type = 'chartex'
        series = @(@{ values = @(1, 2) })
    })
} 'docx-charts-rare-remain-nolayout'
$codeMiss = Get-FailCode $rawMiss
$noMiss = -not (Test-Path $missOut)

$regOut = Get-OutPath 'docx-charts-rare-remain-region.docx'
$rawReg = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $regOut.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'; id = $para1; type = 'chartex'; layoutId = 'regionMap'
        series = @(@{ values = @(1) })
    })
} 'docx-charts-rare-remain-region'
$codeReg = Get-FailCode $rawReg
$noReg = -not (Test-Path $regOut)

$twoOut = Get-OutPath 'docx-charts-rare-remain-stock2.docx'
$rawTwo = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $twoOut.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'; id = $para1; type = 'stock'
        series = @(
            @{ values = @(1, 2) }
            @{ values = @(3, 4) }
        )
    })
} 'docx-charts-rare-remain-stock2'
$codeTwo = Get-FailCode $rawTwo
$noTwo = -not (Test-Path $twoOut)

$ccx5Ok = ($codeMiss -eq 'InvalidField') -and $noMiss -and ($codeReg -eq 'NotImplemented') -and $noReg -and ($codeTwo -eq 'InvalidField') -and $noTwo
Add-Check 'docx-Ccx5' $ccx5Ok "miss=$codeMiss/$noMiss region=$codeReg/$noReg stock2=$codeTwo/$noTwo"

Stop-ChartEmbedProcess
