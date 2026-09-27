# docx 扩展图表：docs/checklist-docx.md 的 Cx1–Cx8。
# 源文件由 Word AddChart2 另存圆环、雷达、气泡、柱线组合、股价，再向独立柱形写入 pivotSource。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Ensure-DocxWordChartsExtSource
$word = Ensure-MsWord $src

function Find-ChartByType($tree, [string]$type) {
    foreach ($c in @($tree.charts)) {
        if ([string]$c.type -eq $type) { return $c }
    }
    return $null
}

function Get-WordChartObjects($doc) {
    $out = New-Object System.Collections.ArrayList
    $inlines = $doc.InlineShapes
    try {
        $n = [int]$inlines.Count
        for ($i = 1; $i -le $n; $i++) {
            $ish = $inlines.Item($i)
            try {
                if ([int]$ish.Type -eq 12) { [void]$out.Add($ish.Chart) }
            }
            finally { Release-Com $ish }
        }
    }
    finally { Release-Com $inlines }
    $shapes = $doc.Shapes
    try {
        $n = [int]$shapes.Count
        for ($i = 1; $i -le $n; $i++) {
            $shp = $shapes.Item($i)
            try {
                $has = $false
                try { $has = [bool]$shp.HasChart } catch {
                    $has = ([int]$shp.Type -eq 3)
                }
                if ($has) { [void]$out.Add($shp.Chart) }
            }
            finally { Release-Com $shp }
        }
    }
    finally { Release-Com $shapes }
    return @($out)
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

function Get-BubbleSizeNums($chart, [int]$index) {
    $ser = $null
    try { $ser = $chart.SeriesCollection($index) } catch {
        try { $ser = $chart.FullSeriesCollection($index) } catch { return @() }
    }
    try {
        $raw = $null
        try { $raw = $ser.BubbleSizes } catch { return @() }
        return @(Flatten-Nums $raw)
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

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 30; charts = $true }
} 'docx-charts-ext-inspect'
$doughnut = Find-ChartByType $tree 'doughnut'
$radar = Find-ChartByType $tree 'radar'
$bubble = Find-ChartByType $tree 'bubble'
$combo = Find-ChartByType $tree 'combo'
$stock = Find-ChartByType $tree 'stock'
if (-not $stock) { $stock = Find-ChartByType $tree 'other' }
$pivot = $null
foreach ($c in @($tree.charts)) {
    $flags = @($c.flags)
    if ($flags -contains 'pivot') { $pivot = $c; break }
}
$para1 = $tree.paragraphs[0].id

$docSrc = Open-WordDoc $word $src
$capSrc = [string]$docSrc.ActiveWindow.Caption
$comCharts = Get-WordChartObjects $docSrc
$bubbleMatch = $false
$bubbleDetail = 'no-bubble'
if ($bubble) {
    $bVals = @(Flatten-Nums $bubble.series[0].values)
    $bSizes = @(Flatten-Nums $bubble.series[0].sizes)
    foreach ($ch in $comCharts) {
        try {
            $cv = Get-SeriesNums $ch 1
            $cs = Get-BubbleSizeNums $ch 1
            if ((Test-NumsClose $cv $bVals) -and ($cs.Count -eq 0 -or (Test-NumsClose $cs $bSizes))) {
                $bubbleMatch = $true
                $bubbleDetail = "comY=$($cv -join ',') comS=$($cs -join ',') sizes=$($bSizes -join ',')"
                break
            }
            $bubbleDetail = "comY=$($cv -join ',') comS=$($cs -join ',') sizes=$($bSizes -join ',')"
        } catch {
            $bubbleDetail = $_.Exception.Message
        }
    }
}
Close-WordDoc $docSrc

$typesOk = ($null -ne $doughnut) -and ($null -ne $radar) -and ($null -ne $bubble) -and ($null -ne $bubble.series[0].sizes)
Add-Check 'docx-Cx1' ($typesOk -and $bubbleMatch -and ($capSrc -notmatch 'Repair')) "dn=$($doughnut.type) rd=$($radar.type) bb=$($bubble.type) $bubbleDetail caption=$capSrc"

# Cx2：改圆环或雷达。
$dnOut = Get-OutPath 'docx-charts-ext-doughnut.docx'
Close-WordDocByPath $word $dnOut
$dnSer = @($doughnut.series)[0]
$nCat = @($dnSer.values).Count
if ($nCat -lt 1) { $nCat = 3 }
$dnNew = @()
for ($i = 1; $i -le $nCat; $i++) { $dnNew += (21 * $i) }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $dnOut.Replace('\', '/')
    ops = @(@{ op = 'setChartSeries'; id = $dnSer.id; values = $dnNew })
} 'docx-charts-ext-doughnut' | Out-Null
$dnTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $dnOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 30; charts = $true }
} 'docx-charts-ext-doughnut-inspect'
$dnAfter = Find-ChartByType $dnTree 'doughnut'
$docDn = Open-WordDoc $word $dnOut
$capDn = [string]$docDn.ActiveWindow.Caption
$dnWord = $false
foreach ($ch in (Get-WordChartObjects $docDn)) {
    if (Test-NumsClose (Get-SeriesNums $ch 1) $dnNew) { $dnWord = $true; break }
}
Close-WordDoc $docDn
Add-Check 'docx-Cx2' (($capDn -notmatch 'Repair') -and $dnWord -and (Test-NumsClose $dnAfter.series[0].values $dnNew)) "caption=$capDn word=$dnWord vals=$($dnNew -join ',')"

# Cx3：改气泡 values 与 sizes。
$bbOut = Get-OutPath 'docx-charts-ext-bubble.docx'
Close-WordDocByPath $word $bbOut
$bbSer = @($bubble.series)[0]
$nBb = @($bbSer.values).Count
if ($nBb -lt 1) { $nBb = 3 }
$bbVals = @(); $bbSizes = @()
for ($i = 1; $i -le $nBb; $i++) { $bbVals += (15 * $i); $bbSizes += (8 * $i) }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $bbOut.Replace('\', '/')
    ops = @(@{ op = 'setChartSeries'; id = $bbSer.id; values = $bbVals; sizes = $bbSizes })
} 'docx-charts-ext-bubble' | Out-Null
$bbTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $bbOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 30; charts = $true }
} 'docx-charts-ext-bubble-inspect'
$bbAfter = Find-ChartByType $bbTree 'bubble'
$docBb = Open-WordDoc $word $bbOut
$capBb = [string]$docBb.ActiveWindow.Caption
$bbWord = $false
foreach ($ch in (Get-WordChartObjects $docBb)) {
    $cv = Get-SeriesNums $ch 1
    if (Test-NumsClose $cv $bbVals) { $bbWord = $true; break }
}
Close-WordDoc $docBb
Add-Check 'docx-Cx3' (($capBb -notmatch 'Repair') -and $bbWord -and (Test-NumsClose $bbAfter.series[0].values $bbVals) -and (Test-NumsClose $bbAfter.series[0].sizes $bbSizes)) "caption=$capBb word=$bbWord"

# Cx4：插入圆环与雷达。
$insOut = Get-OutPath 'docx-charts-ext-insert.docx'
Close-WordDocByPath $word $insOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(
        @{
            op = 'insertChart'; id = $para1; type = 'doughnut'; title = '插入环'
            series = @(@{ name = '环'; categories = @('A', 'B'); values = @(2, 5) })
        }
        @{
            op = 'insertChart'; id = $para1; type = 'radar'
            series = @(@{ name = '雷'; categories = @('A', 'B', 'C'); values = @(1, 3, 4) })
        }
    )
} 'docx-charts-ext-insert' | Out-Null
$insTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 30; charts = $true }
} 'docx-charts-ext-insert-inspect'
$insDn = @($insTree.charts | Where-Object { $_.type -eq 'doughnut' })
$insRd = @($insTree.charts | Where-Object { $_.type -eq 'radar' })
$docIns = Open-WordDoc $word $insOut
$capIns = [string]$docIns.ActiveWindow.Caption
$wordCharts = @(Get-WordChartObjects $docIns)
Close-WordDoc $docIns
Add-Check 'docx-Cx4' (($capIns -notmatch 'Repair') -and ($insDn.Count -ge 2) -and ($insRd.Count -ge 2) -and ($wordCharts.Count -ge 2)) "caption=$capIns dn=$($insDn.Count) rd=$($insRd.Count) word=$($wordCharts.Count)"

# Cx5：组合图改数。
$comboOut = Get-OutPath 'docx-charts-ext-combo-ser.docx'
Close-WordDocByPath $word $comboOut
$c0 = @($combo.series)[0]
$c1 = @($combo.series)[1]
$n0 = @($c0.values).Count
if ($n0 -lt 1) { $n0 = 3 }
$c0New = @(); $c1New = @()
for ($i = 1; $i -le $n0; $i++) { $c0New += (31 * $i); $c1New += (7 * $i) }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $comboOut.Replace('\', '/')
    ops = @(
        @{ op = 'setChartSeries'; id = $c0.id; values = $c0New }
        @{ op = 'setChartSeries'; id = $c1.id; values = $c1New }
    )
} 'docx-charts-ext-combo-ser' | Out-Null
$comboTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $comboOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 30; charts = $true }
} 'docx-charts-ext-combo-ser-inspect'
$comboAfter = Find-ChartByType $comboTree 'combo'
$docCb = Open-WordDoc $word $comboOut
$capCb = [string]$docCb.ActiveWindow.Caption
$cb0 = Test-NumsClose $comboAfter.series[0].values $c0New
$cb1 = Test-NumsClose $comboAfter.series[1].values $c1New
Close-WordDoc $docCb
Add-Check 'docx-Cx5' (($capCb -notmatch 'Repair') -and ([string]$comboAfter.type -eq 'combo') -and $cb0 -and $cb1) "caption=$capCb type=$($comboAfter.type) s0=$cb0 s1=$cb1"

# Cx6：组合图改成柱形。
$colOut = Get-OutPath 'docx-charts-ext-combo-col.docx'
Close-WordDocByPath $word $colOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $colOut.Replace('\', '/')
    ops = @(@{ op = 'setChartType'; id = $combo.id; type = 'col' })
} 'docx-charts-ext-combo-col' | Out-Null
$colTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $colOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 30; charts = $true }
} 'docx-charts-ext-combo-col-inspect'
$colAfter = Find-ChartByType $colTree 'col'
$stillCombo = Find-ChartByType $colTree 'combo'
$docCol = Open-WordDoc $word $colOut
$capCol = [string]$docCol.ActiveWindow.Caption
Close-WordDoc $docCol
Add-Check 'docx-Cx6' (($capCol -notmatch 'Repair') -and ($null -ne $colAfter) -and ($null -eq $stillCombo)) "caption=$capCol col=$($colAfter.type) comboLeft=$($null -ne $stillCombo)"

# Cx7：透视标题可改，改系列报 NotImplemented。
$pvTitle = Get-OutPath 'docx-charts-ext-pivot-title.docx'
$pvSer = Get-OutPath 'docx-charts-ext-pivot-series.docx'
$pvOk = $false
$pvDetail = 'no-pivot'
if ($pivot) {
    Close-WordDocByPath $word $pvTitle
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $src.Replace('\', '/')
        output = $pvTitle.Replace('\', '/')
        ops = @(@{ op = 'setChartTitle'; id = $pivot.id; title = '透视新标题' })
    } 'docx-charts-ext-pivot-title' | Out-Null
    $pvTree = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $pvTitle.Replace('\', '/'); format = 'docx'
        scope = @{ from = 1; maxParas = 30; charts = $true }
    } 'docx-charts-ext-pivot-title-inspect'
    $pvAfter = $null
    foreach ($c in @($pvTree.charts)) {
        if ((@($c.flags) -contains 'pivot')) { $pvAfter = $c; break }
    }
    $docPv = Open-WordDoc $word $pvTitle
    $capPv = [string]$docPv.ActiveWindow.Caption
    Close-WordDoc $docPv
    if (Test-Path $pvSer) { Remove-Item -Force $pvSer }
    $raw = Invoke-OoxpExpectFail @{
        schemaVersion = 1; action = 'apply'
        path = $src.Replace('\', '/')
        output = $pvSer.Replace('\', '/')
        ops = @(@{ op = 'setChartSeries'; id = $pivot.series[0].id; values = @(1, 2, 3) })
    } 'docx-charts-ext-pivot-series'
    $code = ''
    try { $code = [string](($raw.Stderr | ConvertFrom-Json).error.code) } catch { $code = $raw.Stderr }
    $noWrite = -not (Test-Path $pvSer)
    $pvOk = ($pvAfter.title -eq '透视新标题') -and ($code -eq 'NotImplemented') -and $noWrite -and ($capPv -notmatch 'Repair')
    $pvDetail = "title=$($pvAfter.title) code=$code wrote=$(-not $noWrite) caption=$capPv"
}
Add-Check 'docx-Cx7' $pvOk $pvDetail

# Cx8：股价 type=stock，改类型仍拒绝。
$otOut = Get-OutPath 'docx-charts-ext-other.docx'
$otOk = $false
$otDetail = 'no-stock'
if ($stock) {
    if (Test-Path $otOut) { Remove-Item -Force $otOut }
    $raw = Invoke-OoxpExpectFail @{
        schemaVersion = 1; action = 'apply'
        path = $src.Replace('\', '/')
        output = $otOut.Replace('\', '/')
        ops = @(@{ op = 'setChartType'; id = $stock.id; type = 'col' })
    } 'docx-charts-ext-other'
    $code = ''
    try { $code = [string](($raw.Stderr | ConvertFrom-Json).error.code) } catch { $code = $raw.Stderr }
    $noWrite = -not (Test-Path $otOut)
    $docOt = Open-WordDoc $word $src
    $capOt = [string]$docOt.ActiveWindow.Caption
    Close-WordDoc $docOt
    $otOk = ($stock.type -eq 'stock') -and ($code -eq 'NotImplemented') -and $noWrite -and ($capOt -notmatch 'Repair')
    $otDetail = "type=$($stock.type) code=$code wrote=$(-not $noWrite) caption=$capOt"
}
Add-Check 'docx-Cx8' $otOk $otDetail
