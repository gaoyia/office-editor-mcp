# docx 嵌入图表：docs/checklist-docx.md 的 Ch1–Ch8。
# 源文件由 Word AddChart2 另存柱形图和组合图。不要用 create 产物当唯一样例。

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

function Get-SeriesNums($chart, [int]$index) {
    $ser = $null
    try { $ser = $chart.SeriesCollection($index) } catch {
        try { $ser = $chart.FullSeriesCollection($index) } catch { return @() }
    }
    try {
        $raw = @($ser.Values)
        $nums = @()
        foreach ($v in $raw) {
            if ($null -eq $v) { continue }
            try { $nums += [double]$v } catch {}
        }
        return $nums
    }
    finally { if ($ser) { Release-Com $ser } }
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

function Get-ZipEntryBytes([string]$docx, [string]$pattern) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $z = [IO.Compression.ZipFile]::OpenRead($docx)
    try {
        $entry = $z.Entries | Where-Object { $_.FullName -replace '\\','/' -like $pattern } | Select-Object -First 1
        if (-not $entry) { return $null }
        $ms = New-Object IO.MemoryStream
        $es = $entry.Open()
        try { $es.CopyTo($ms) } finally { $es.Dispose() }
        return $ms.ToArray()
    }
    finally { $z.Dispose() }
}

# Ch1：无 charts 时只有计数；charts:true 时 id / type / 系列值能对上 COM。
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-charts-inspect-counts'
$noList = -not ($tree0.PSObject.Properties.Name -contains 'charts')
Add-Check 'docx-Ch1a' (($tree0.chartCount -ge 1) -and $noList) "count=$($tree0.chartCount) noList=$noList"

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; charts = $true }
} 'docx-charts-inspect-lists'
$col = Find-ChartByType $tree1 'col'
if (-not $col) { $col = @($tree1.charts) | Where-Object { $_.type -notin @('combo','other','chartex') } | Select-Object -First 1 }
$combo = Find-ChartByType $tree1 'combo'
$idOk = $col -and ([string]$col.id -like 'para:*/chart:*')
$ser0 = if ($col) { @($col.series)[0] } else { $null }
$serVals = if ($ser0) { @($ser0.values) } else { @() }

$docSrc = Open-WordDoc $word $src
$comCharts = Get-WordChartObjects $docSrc
$comMatch = $false
$comDetail = "wordCharts=$($comCharts.Count)"
foreach ($ch in $comCharts) {
    try {
        $cv = Get-SeriesNums $ch 1
        if ($serVals.Count -gt 0 -and (Test-NumsClose $cv $serVals)) {
            $comMatch = $true
            $comDetail = "com=$($cv -join ',') inspect=$($serVals -join ',')"
            break
        }
        if (-not $comMatch) { $comDetail = "com=$($cv -join ',') inspect=$($serVals -join ',')" }
    } catch {
        $comDetail = $_.Exception.Message
    }
}
Close-WordDoc $docSrc
$titleOk = $col -and ($null -ne $col.title)
Add-Check 'docx-Ch1' ($idOk -and ($serVals.Count -ge 1) -and $comMatch -and $titleOk) "id=$($col.id) type=$($col.type) title=$($col.title) $comDetail combo=$($combo.type)"

$para1 = $tree1.paragraphs[0].id
$colId = $col.id
$serId = if ($ser0) { $ser0.id } else { "$colId/series:0" }
$nCat = $serVals.Count
if ($nCat -lt 1) { $nCat = 3 }
$newVals = @()
for ($i = 1; $i -le $nCat; $i++) { $newVals += (11 * $i) }

# Ch2：改系列。
$serOut = Get-OutPath 'docx-charts-series.docx'
Close-WordDocByPath $word $serOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $serOut.Replace('\', '/')
    ops = @(@{ op = 'setChartSeries'; id = $serId; values = $newVals })
} 'docx-charts-set-series' | Out-Null

$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $serOut.Replace('\', '/'); format = 'docx'
    scope = @{ charts = $true }
} 'docx-charts-series-inspect'
$colS = @($treeS.charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$gotVals = if ($colS) { @(@($colS.series)[0].values) } else { @() }
$docS = Open-WordDoc $word $serOut
$capS = [string]$docS.ActiveWindow.Caption
$comNew = $false
foreach ($ch in (Get-WordChartObjects $docS)) {
    try {
        $cv = Get-SeriesNums $ch 1
        if (Test-NumsClose $cv $newVals) { $comNew = $true; break }
    } catch {}
}
Close-WordDoc $docS
Add-Check 'docx-Ch2' (($capS -notmatch 'Repair') -and (Test-NumsClose $gotVals $newVals) -and $comNew) "caption=$capS inspect=$($gotVals -join ',') want=$($newVals -join ',') com=$comNew"

# Ch3：插入饼图。
$insOut = Get-OutPath 'docx-charts-insert.docx'
Close-WordDocByPath $word $insOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'
        id = $para1
        type = 'pie'
        title = $S.chartTitle
        series = @(@{ name = $S.chartSer1; categories = @('A','B'); values = @(1, 2) })
    })
} 'docx-charts-insert' | Out-Null

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'docx'
    scope = @{ charts = $true }
} 'docx-charts-insert-inspect'
$pie = Find-ChartByType $treeI 'pie'
$docI = Open-WordDoc $word $insOut
$capI = [string]$docI.ActiveWindow.Caption
$nI = @(Get-WordChartObjects $docI).Count
$pieTypeOk = $false
foreach ($ch in (Get-WordChartObjects $docI)) {
    try {
        $t = [int]$ch.ChartType
        if (@(5, 69, 68, 70, -4102) -contains $t) { $pieTypeOk = $true }
    } catch {}
}
Close-WordDoc $docI
Add-Check 'docx-Ch3' (($capI -notmatch 'Repair') -and ($null -ne $pie) -and ($treeI.chartCount -gt $tree0.chartCount) -and ($nI -ge 1)) "caption=$capI pie=$($pie.id) count=$($treeI.chartCount) word=$nI pieType=$pieTypeOk"

# Ch4：删掉刚插的饼图。
$delOut = Get-OutPath 'docx-charts-delete.docx'
Close-WordDocByPath $word $delOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteChart'; id = $pie.id })
} 'docx-charts-delete' | Out-Null

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'docx'
    scope = @{ charts = $true }
} 'docx-charts-delete-inspect'
$stillPie = Find-ChartByType $treeD 'pie'
$docD = Open-WordDoc $word $delOut
$capD = [string]$docD.ActiveWindow.Caption
Close-WordDoc $docD
Add-Check 'docx-Ch4' (($capD -notmatch 'Repair') -and ($null -eq $stillPie) -and ($treeD.chartCount -lt $treeI.chartCount)) "caption=$capD count=$($treeD.chartCount) pie=$($stillPie.id)"

# Ch5：柱形改折线。
$lineOut = Get-OutPath 'docx-charts-line.docx'
Close-WordDocByPath $word $lineOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $lineOut.Replace('\', '/')
    ops = @(@{ op = 'setChartType'; id = $colId; type = 'line' })
} 'docx-charts-set-type' | Out-Null

$treeL = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $lineOut.Replace('\', '/'); format = 'docx'
    scope = @{ charts = $true }
} 'docx-charts-line-inspect'
$lineC = @($treeL.charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$docL = Open-WordDoc $word $lineOut
$capL = [string]$docL.ActiveWindow.Caption
$lineTypeOk = $false
$lineTypes = @(4, 63, 64, 65, 66, 67)
foreach ($ch in (Get-WordChartObjects $docL)) {
    try {
        if ($lineTypes -contains [int]$ch.ChartType) { $lineTypeOk = $true }
    } catch {}
}
Close-WordDoc $docL
Add-Check 'docx-Ch5' (($capL -notmatch 'Repair') -and ($lineC.type -eq 'line') -and $lineTypeOk) "caption=$capL type=$($lineC.type) wordLine=$lineTypeOk"

# Ch6：图例关、数据标签开。
$legOut = Get-OutPath 'docx-charts-legend.docx'
Close-WordDocByPath $word $legOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $legOut.Replace('\', '/')
    ops = @(
        @{ op = 'setChartLegend'; id = $colId; show = $false }
        @{ op = 'setChartDataLabels'; id = $colId; show = $true }
    )
} 'docx-charts-legend' | Out-Null

$treeG = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $legOut.Replace('\', '/'); format = 'docx'
    scope = @{ charts = $true }
} 'docx-charts-legend-inspect'
$colG = @($treeG.charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$docG = Open-WordDoc $word $legOut
$capG = [string]$docG.ActiveWindow.Caption
$legOff = $false
$lblOn = $false
foreach ($ch in (Get-WordChartObjects $docG)) {
    try {
        if (-not [bool]$ch.HasLegend) { $legOff = $true }
    } catch {}
    try {
        $s1 = $null
        try { $s1 = $ch.SeriesCollection(1) } catch {
            try { $s1 = $ch.FullSeriesCollection(1) } catch { $s1 = $null }
        }
        if ($s1) {
            try { if ([bool]$s1.HasDataLabels) { $lblOn = $true } } finally { Release-Com $s1 }
        }
    } catch {}
}
Close-WordDoc $docG
Add-Check 'docx-Ch6' (($capG -notmatch 'Repair') -and ($colG.legend.show -eq $false) -and ($colG.dataLabels.show -eq $true) -and $legOff) "caption=$capG legend=$($colG.legend.show) labels=$($colG.dataLabels.show) wordLegOff=$legOff wordLbl=$lblOn"

# Ch7：组合图改类型得到 NotImplemented，不写坏文件。
$comboOut = Get-OutPath 'docx-charts-combo.docx'
if (Test-Path $comboOut) { Remove-Item -Force $comboOut }
$comboId = if ($combo) { $combo.id } else { $null }
$comboFail = $false
$comboCode = ''
if ($comboId) {
    Close-WordDocByPath $word $comboOut
    $raw = Invoke-OoxpExpectFail @{
        schemaVersion = 1; action = 'apply'
        path = $src.Replace('\', '/')
        output = $comboOut.Replace('\', '/')
        ops = @(@{ op = 'setChartType'; id = $comboId; type = 'line' })
    } 'docx-charts-combo-type'
    try {
        $errObj = $raw.Stderr | ConvertFrom-Json
        $comboCode = [string]$errObj.error.code
    } catch { $comboCode = $raw.Stderr }
    $comboFail = ($raw.ExitCode -ne 0) -and ($comboCode -eq 'NotImplemented')
}
$noWrite = -not (Test-Path $comboOut)
$docC = Open-WordDoc $word $src
$capC = [string]$docC.ActiveWindow.Caption
Close-WordDoc $docC
Add-Check 'docx-Ch7' ($comboFail -and $noWrite -and ($capC -notmatch 'Repair')) "code=$comboCode wrote=$(-not $noWrite) caption=$capC comboId=$comboId"

# Ch8：exportChartData 与 embedding 部件字节一致。
$expOut = Get-OutPath 'docx-charts-export.xlsx'
Close-WordDocByPath $word $expOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportChartData'
    path = $src.Replace('\', '/')
    id = $colId
    output = $expOut.Replace('\', '/')
} 'docx-charts-export' | Out-Null
$embed = Get-ZipEntryBytes $src 'word/embeddings/*'
$exported = if (Test-Path $expOut) { [IO.File]::ReadAllBytes($expOut) } else { @() }
$same = $false
if ($embed -and $exported -and ($embed.Length -eq $exported.Length)) {
    $same = $true
    for ($i = 0; $i -lt $embed.Length; $i++) {
        if ($embed[$i] -ne $exported[$i]) { $same = $false; break }
    }
}
Add-Check 'docx-Ch8' $same "embed=$($embed.Length) export=$($exported.Length)"
