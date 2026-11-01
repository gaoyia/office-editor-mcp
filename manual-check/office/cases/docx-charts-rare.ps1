# docx 图表稀有类型：docs/checklist-docx.md 的 Cq1–Cq5。
# 源文件：Word 另存柱形图。插入股价 / 曲面 / 复合饼，再测改类型与拒绝。

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

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; charts = $true }
} 'docx-charts-rare-inspect'
$para1 = $tree0.paragraphs[0].id
$chartex = Find-ChartByType $tree0 'chartex'

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
    return @{ Out = $out; Tree = $tree; Chart = $hit; Caption = $cap }
}

# Cq1：插入股价。
$r1 = Insert-And-Open 'stock' $stockSer $null 'docx-charts-rare-stock.docx' 'docx-charts-rare-stock'
$serN = 0
if ($r1.Chart) { $serN = @($r1.Chart.series).Count }
Add-Check 'docx-Cq1' (($r1.Caption -notmatch 'Repair') -and ($null -ne $r1.Chart) -and ($serN -eq 3)) "caption=$($r1.Caption) type=$($r1.Chart.type) ser=$serN"

# Cq2：插入曲面。
$r2 = Insert-And-Open 'surface' $surfaceSer $null 'docx-charts-rare-surface.docx' 'docx-charts-rare-surface'
$serN2 = 0
if ($r2.Chart) { $serN2 = @($r2.Chart.series).Count }
Add-Check 'docx-Cq2' (($r2.Caption -notmatch 'Repair') -and ($null -ne $r2.Chart) -and ($serN2 -eq 3)) "caption=$($r2.Caption) type=$($r2.Chart.type) ser=$serN2"

# Cq3：插入复合条。
$r3 = Insert-And-Open 'ofPie' $ofPieSer @{ ofPieType = 'bar' } 'docx-charts-rare-ofpie.docx' 'docx-charts-rare-ofpie'
$ofOk = $r3.Chart -and ([string]$r3.Chart.ofPieType -eq 'bar')
Add-Check 'docx-Cq3' (($r3.Caption -notmatch 'Repair') -and $ofOk) "caption=$($r3.Caption) type=$($r3.Chart.type) ofPieType=$($r3.Chart.ofPieType)"

# Cq4：股价三条转柱形；复合饼转饼。
$typeOut = Get-OutPath 'docx-charts-rare-type.docx'
Close-WordDocByPath $word $typeOut
if (Test-Path $typeOut) { Remove-Item -Force $typeOut }
$bothOut = Get-OutPath 'docx-charts-rare-both.docx'
Close-WordDocByPath $word $bothOut
if (Test-Path $bothOut) { Remove-Item -Force $bothOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $bothOut.Replace('\', '/')
    ops = @(
        @{ op = 'insertChart'; id = $para1; type = 'stock'; series = $stockSer }
        @{ op = 'insertChart'; id = $para1; type = 'ofPie'; series = $ofPieSer }
    )
} 'docx-charts-rare-both' | Out-Null
$treeB = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $bothOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; charts = $true }
} 'docx-charts-rare-both-inspect'
$stB = Find-ChartByType $treeB 'stock'
$ofB = Find-ChartByType $treeB 'ofPie'
$cq4Ok = $false
$cq4Detail = 'missing'
if ($stB -and $ofB) {
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $bothOut.Replace('\', '/')
        output = $typeOut.Replace('\', '/')
        ops = @(
            @{ op = 'setChartType'; id = $stB.id; type = 'col' }
            @{ op = 'setChartType'; id = $ofB.id; type = 'pie' }
        )
    } 'docx-charts-rare-type' | Out-Null
    $treeT = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $typeOut.Replace('\', '/'); format = 'docx'
        scope = @{ from = 1; maxParas = 20; charts = $true }
    } 'docx-charts-rare-type-inspect'
    $stAfter = @($treeT.charts) | Where-Object { $_.id -eq $stB.id } | Select-Object -First 1
    $ofAfter = @($treeT.charts) | Where-Object { $_.id -eq $ofB.id } | Select-Object -First 1
    $docT = Open-WordDoc $word $typeOut
    $capT = [string]$docT.ActiveWindow.Caption
    Close-WordDoc $docT
    $cq4Ok = ($capT -notmatch 'Repair') -and ([string]$stAfter.type -eq 'col') -and ([string]$ofAfter.type -eq 'pie')
    $cq4Detail = "caption=$capT stock=$($stAfter.type) ofPie=$($ofAfter.type)"
}
Add-Check 'docx-Cq4' $cq4Ok $cq4Detail

# Cq5：股价增系列仍拒；chartex 改标题或 insert type=chartex 拒；两条股价 insert 拒。
$addOut = Get-OutPath 'docx-charts-rare-add.docx'
$rawAdd = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $r1.Out.Replace('\', '/')
    output = $addOut.Replace('\', '/')
    ops = @(@{ op = 'addChartSeries'; id = $r1.Chart.id; values = @(1, 2, 3) })
} 'docx-charts-rare-add'
$codeAdd = Get-FailCode $rawAdd
$noAdd = -not (Test-Path $addOut)

$cxOut = Get-OutPath 'docx-charts-rare-chartex.docx'
if ($chartex) {
    $rawCx = Invoke-OoxpExpectFail @{
        schemaVersion = 1; action = 'apply'
        path = $src.Replace('\', '/')
        output = $cxOut.Replace('\', '/')
        ops = @(@{ op = 'setChartType'; id = $chartex.id; type = 'col' })
    } 'docx-charts-rare-chartex-title'
} else {
    $rawCx = Invoke-OoxpExpectFail @{
        schemaVersion = 1; action = 'apply'
        path = $src.Replace('\', '/')
        output = $cxOut.Replace('\', '/')
        ops = @(@{
            op = 'insertChart'; id = $para1; type = 'chartex'
            series = @(@{ values = @(1, 2) })
        })
    } 'docx-charts-rare-chartex'
}
$codeCx = Get-FailCode $rawCx
$noCx = -not (Test-Path $cxOut)

$twoOut = Get-OutPath 'docx-charts-rare-stock2.docx'
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
} 'docx-charts-rare-stock2'
$codeTwo = Get-FailCode $rawTwo
$noTwo = -not (Test-Path $twoOut)

$cxWant = if ($chartex) { 'NotImplemented' } else { 'InvalidField' }
$cq5Ok = ($codeAdd -eq 'NotImplemented') -and $noAdd -and ($codeCx -eq $cxWant) -and $noCx -and ($codeTwo -eq 'InvalidField') -and $noTwo
Add-Check 'docx-Cq5' $cq5Ok "add=$codeAdd/$noAdd chartex=$codeCx/$noCx stock2=$codeTwo/$noTwo"
