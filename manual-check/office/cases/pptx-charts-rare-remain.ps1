# pptx 图表稀有剩余：docs/checklist-pptx.md 的 Pcx1–Pcx5。
# 源文件：PowerPoint 另存柱形图。OHLC / 俯视线框 / 自定义误差线 / chartex funnel。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-PptxPptChartSource
$ppt = Ensure-MsPpt $src
Close-PptPresByPath $ppt $src

function Find-SlideChartByType($tree, [string]$type) {
    foreach ($sl in @($tree.slides)) {
        foreach ($c in @($sl.charts)) {
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

function Insert-And-Open([string]$type, $series, $xy, $extra, [string]$outName, [string]$reqName) {
    $out = Get-OutPath $outName
    Close-PptPresByPath $ppt $out
    if (Test-Path $out) { Remove-Item -Force $out }
    $op = @{
        op = 'insertChart'
        id = 'slide:1'
        type = $type
        title = $S.chartTitle
        x = $xy.x
        y = $xy.y
        cx = 3657600
        cy = 1828800
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
        path = $out.Replace('\', '/'); format = 'pptx'
        scope = @{ charts = $true }
    } "$reqName-inspect"
    $hit = Find-SlideChartByType $tree $type
    $pres = Open-PptPres $ppt $out
    $cap = Get-PptCaption $pres
    Close-PptPresByPath $ppt $out
    Stop-ChartEmbedProcess
    return @{ Out = $out; Tree = $tree; Chart = $hit; Caption = $cap }
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

# Pcx1：OHLC。
$r1 = Insert-And-Open 'stock' $ohlcSer @{ x = 914400; y = 200000 } @{ stockKind = 'ohlc' } 'pptx-charts-rare-remain-ohlc.pptx' 'pptx-charts-rare-remain-ohlc'
$kind1 = if ($r1.Chart) { [string]$r1.Chart.stockKind } else { '' }
$serN = 0
if ($r1.Chart) { $serN = @($r1.Chart.series).Count }
Add-Check 'pptx-Pcx1' (($r1.Caption -notmatch 'Repair') -and ($null -ne $r1.Chart) -and ($kind1 -eq 'ohlc') -and ($serN -eq 4)) "caption=$($r1.Caption) type=$($r1.Chart.type) stockKind=$kind1 ser=$serN"

# Pcx2：俯视线框。
$r2 = Insert-And-Open 'surface' $surfaceSer @{ x = 914400; y = 2200000 } @{ surfaceKind = 'contourWire' } 'pptx-charts-rare-remain-contour.pptx' 'pptx-charts-rare-remain-contour'
$kind2 = if ($r2.Chart) { [string]$r2.Chart.surfaceKind } else { '' }
Add-Check 'pptx-Pcx2' (($r2.Caption -notmatch 'Repair') -and ($null -ne $r2.Chart) -and ($kind2 -eq 'contourWire')) "caption=$($r2.Caption) type=$($r2.Chart.type) surfaceKind=$kind2"

# Pcx3：自定义误差线。
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ charts = $true }
} 'pptx-charts-rare-remain-src'
$oldIds = @(@($tree0.slides[0].charts) | ForEach-Object { [string]$_.id })
$colOut = Get-OutPath 'pptx-charts-rare-remain-col.pptx'
Close-PptPresByPath $ppt $colOut
if (Test-Path $colOut) { Remove-Item -Force $colOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $colOut.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'
        id = 'slide:1'
        type = 'col'
        title = $S.chartTitle
        series = @(
            @{ name = $S.chartSer1; categories = @($S.saItem1, $S.saItem2, $S.saItem3); values = @(1, 2, 3) }
        )
    })
} 'pptx-charts-rare-remain-col' | Out-Null
$treeCol = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $colOut.Replace('\', '/'); format = 'pptx'
    scope = @{ charts = $true }
} 'pptx-charts-rare-remain-col-inspect'
$col = @($treeCol.slides[0].charts) | Where-Object { $oldIds -notcontains [string]$_.id } | Select-Object -First 1
$serId = if ($col) { [string]$col.series[0].id } else { '' }
$ebOut = Get-OutPath 'pptx-charts-rare-remain-errbars.pptx'
Close-PptPresByPath $ppt $ebOut
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
} 'pptx-charts-rare-remain-errbars' | Out-Null
$treeEb = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ebOut.Replace('\', '/'); format = 'pptx'
    scope = @{ charts = $true }
} 'pptx-charts-rare-remain-errbars-inspect'
$ebChart = @($treeEb.slides[0].charts) | Where-Object { [string]$_.id -eq [string]$col.id } | Select-Object -First 1
$bars = $null
if ($ebChart) { $bars = @($ebChart.series[0].errorBars) | Select-Object -First 1 }
$presEb = Open-PptPres $ppt $ebOut
$capEb = Get-PptCaption $presEb
Close-PptPresByPath $ppt $ebOut
$plusOk = $bars -and (Test-NumsClose $bars.plus @(0.5, 0.4, 0.3))
$minusOk = $bars -and (Test-NumsClose $bars.minus @(0.2, 0.1, 0.1))
$vt = if ($bars) { [string]$bars.valType } else { '' }
Add-Check 'pptx-Pcx3' (($capEb -notmatch 'Repair') -and ($vt -eq 'cust') -and $plusOk -and $minusOk) "caption=$capEb valType=$vt plus=$plusOk minus=$minusOk"

# Pcx4：chartex funnel。
$r4 = Insert-And-Open 'chartex' $funnelSer @{ x = 4700000; y = 200000 } @{ layoutId = 'funnel' } 'pptx-charts-rare-remain-funnel.pptx' 'pptx-charts-rare-remain-funnel'
$lid = if ($r4.Chart) { [string]$r4.Chart.layoutId } else { '' }
Add-Check 'pptx-Pcx4' (($r4.Caption -notmatch 'Repair') -and ($null -ne $r4.Chart) -and ([string]$r4.Chart.type -eq 'chartex') -and ($lid -eq 'funnel')) "caption=$($r4.Caption) type=$($r4.Chart.type) layoutId=$lid"

# Pcx5：仍拒绝。
$missOut = Get-OutPath 'pptx-charts-rare-remain-nolayout.pptx'
$rawMiss = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $missOut.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'; id = 'slide:1'; type = 'chartex'
        series = @(@{ values = @(1, 2) })
    })
} 'pptx-charts-rare-remain-nolayout'
$codeMiss = Get-FailCode $rawMiss
$noMiss = -not (Test-Path $missOut)

$regOut = Get-OutPath 'pptx-charts-rare-remain-region.pptx'
$rawReg = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $regOut.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'; id = 'slide:1'; type = 'chartex'; layoutId = 'regionMap'
        series = @(@{ values = @(1) })
    })
} 'pptx-charts-rare-remain-region'
$codeReg = Get-FailCode $rawReg
$noReg = -not (Test-Path $regOut)

$twoOut = Get-OutPath 'pptx-charts-rare-remain-stock2.pptx'
$rawTwo = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $twoOut.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'; id = 'slide:1'; type = 'stock'
        series = @(
            @{ values = @(1, 2) }
            @{ values = @(3, 4) }
        )
    })
} 'pptx-charts-rare-remain-stock2'
$codeTwo = Get-FailCode $rawTwo
$noTwo = -not (Test-Path $twoOut)

$pcx5Ok = ($codeMiss -eq 'InvalidField') -and $noMiss -and ($codeReg -eq 'NotImplemented') -and $noReg -and ($codeTwo -eq 'InvalidField') -and $noTwo
Add-Check 'pptx-Pcx5' $pcx5Ok "miss=$codeMiss/$noMiss region=$codeReg/$noReg stock2=$codeTwo/$noTwo"

Stop-ChartEmbedProcess
