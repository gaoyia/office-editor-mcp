# pptx 图表稀有类型：docs/checklist-pptx.md 的 Pq1–Pq5。
# 源文件：PowerPoint 另存柱形图。插入股价 / 曲面 / 复合饼，再测改类型与拒绝。

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

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ charts = $true }
} 'pptx-charts-rare-inspect'
$chartex = Find-SlideChartByType $tree0 'chartex'

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
    return @{ Out = $out; Tree = $tree; Chart = $hit; Caption = $cap }
}

# Pq1：插入股价。
$r1 = Insert-And-Open 'stock' $stockSer @{ x = 914400; y = 200000 } $null 'pptx-charts-rare-stock.pptx' 'pptx-charts-rare-stock'
$serN = 0
if ($r1.Chart) { $serN = @($r1.Chart.series).Count }
Add-Check 'pptx-Pq1' (($r1.Caption -notmatch 'Repair') -and ($null -ne $r1.Chart) -and ($serN -eq 3)) "caption=$($r1.Caption) type=$($r1.Chart.type) ser=$serN"

# Pq2：插入曲面。
$r2 = Insert-And-Open 'surface' $surfaceSer @{ x = 914400; y = 2200000 } $null 'pptx-charts-rare-surface.pptx' 'pptx-charts-rare-surface'
$serN2 = 0
if ($r2.Chart) { $serN2 = @($r2.Chart.series).Count }
Add-Check 'pptx-Pq2' (($r2.Caption -notmatch 'Repair') -and ($null -ne $r2.Chart) -and ($serN2 -eq 3)) "caption=$($r2.Caption) type=$($r2.Chart.type) ser=$serN2"

# Pq3：插入复合条。
$r3 = Insert-And-Open 'ofPie' $ofPieSer @{ x = 4700000; y = 200000 } @{ ofPieType = 'bar' } 'pptx-charts-rare-ofpie.pptx' 'pptx-charts-rare-ofpie'
$ofOk = $r3.Chart -and ([string]$r3.Chart.ofPieType -eq 'bar')
Add-Check 'pptx-Pq3' (($r3.Caption -notmatch 'Repair') -and $ofOk) "caption=$($r3.Caption) type=$($r3.Chart.type) ofPieType=$($r3.Chart.ofPieType)"

# Pq4：股价三条转柱形；复合饼转饼。
$bothOut = Get-OutPath 'pptx-charts-rare-both.pptx'
$typeOut = Get-OutPath 'pptx-charts-rare-type.pptx'
Close-PptPresByPath $ppt $bothOut
Close-PptPresByPath $ppt $typeOut
if (Test-Path $bothOut) { Remove-Item -Force $bothOut }
if (Test-Path $typeOut) { Remove-Item -Force $typeOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $bothOut.Replace('\', '/')
    ops = @(
        @{
            op = 'insertChart'; id = 'slide:1'; type = 'stock'
            x = 914400; y = 200000; cx = 3657600; cy = 1828800
            series = $stockSer
        }
        @{
            op = 'insertChart'; id = 'slide:1'; type = 'ofPie'
            x = 4700000; y = 200000; cx = 3657600; cy = 1828800
            series = $ofPieSer
        }
    )
} 'pptx-charts-rare-both' | Out-Null
$treeB = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $bothOut.Replace('\', '/'); format = 'pptx'
    scope = @{ charts = $true }
} 'pptx-charts-rare-both-inspect'
$stB = Find-SlideChartByType $treeB 'stock'
$ofB = Find-SlideChartByType $treeB 'ofPie'
$pq4Ok = $false
$pq4Detail = 'missing'
if ($stB -and $ofB) {
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $bothOut.Replace('\', '/')
        output = $typeOut.Replace('\', '/')
        ops = @(
            @{ op = 'setChartType'; id = $stB.id; type = 'col' }
            @{ op = 'setChartType'; id = $ofB.id; type = 'pie' }
        )
    } 'pptx-charts-rare-type' | Out-Null
    $treeT = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $typeOut.Replace('\', '/'); format = 'pptx'
        scope = @{ charts = $true }
    } 'pptx-charts-rare-type-inspect'
    $stAfter = $null
    $ofAfter = $null
    foreach ($sl in @($treeT.slides)) {
        foreach ($c in @($sl.charts)) {
            if ($c.id -eq $stB.id) { $stAfter = $c }
            if ($c.id -eq $ofB.id) { $ofAfter = $c }
        }
    }
    $presT = Open-PptPres $ppt $typeOut
    $capT = Get-PptCaption $presT
    Close-PptPresByPath $ppt $typeOut
    $pq4Ok = ($capT -notmatch 'Repair') -and ([string]$stAfter.type -eq 'col') -and ([string]$ofAfter.type -eq 'pie')
    $pq4Detail = "caption=$capT stock=$($stAfter.type) ofPie=$($ofAfter.type)"
}
Add-Check 'pptx-Pq4' $pq4Ok $pq4Detail

# Pq5：股价增系列仍拒；chartex 改标题或 insert type=chartex 拒；两条股价 insert 拒。
$addOut = Get-OutPath 'pptx-charts-rare-add.pptx'
$rawAdd = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $r1.Out.Replace('\', '/')
    output = $addOut.Replace('\', '/')
    ops = @(@{ op = 'addChartSeries'; id = $r1.Chart.id; values = @(1, 2, 3) })
} 'pptx-charts-rare-add'
$codeAdd = Get-FailCode $rawAdd
$noAdd = -not (Test-Path $addOut)

$cxOut = Get-OutPath 'pptx-charts-rare-chartex.pptx'
if ($chartex) {
    $rawCx = Invoke-OoxpExpectFail @{
        schemaVersion = 1; action = 'apply'
        path = $src.Replace('\', '/')
        output = $cxOut.Replace('\', '/')
        ops = @(@{ op = 'setChartTitle'; id = $chartex.id; title = '不该写' })
    } 'pptx-charts-rare-chartex-title'
} else {
    $rawCx = Invoke-OoxpExpectFail @{
        schemaVersion = 1; action = 'apply'
        path = $src.Replace('\', '/')
        output = $cxOut.Replace('\', '/')
        ops = @(@{
            op = 'insertChart'; id = 'slide:1'; type = 'chartex'
            series = @(@{ values = @(1, 2) })
        })
    } 'pptx-charts-rare-chartex'
}
$codeCx = Get-FailCode $rawCx
$noCx = -not (Test-Path $cxOut)

$twoOut = Get-OutPath 'pptx-charts-rare-stock2.pptx'
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
} 'pptx-charts-rare-stock2'
$codeTwo = Get-FailCode $rawTwo
$noTwo = -not (Test-Path $twoOut)

$cxWant = if ($chartex) { 'NotImplemented' } else { 'InvalidField' }
$pq5Ok = ($codeAdd -eq 'NotImplemented') -and $noAdd -and ($codeCx -eq $cxWant) -and $noCx -and ($codeTwo -eq 'InvalidField') -and $noTwo
Add-Check 'pptx-Pq5' $pq5Ok "add=$codeAdd/$noAdd chartex=$codeCx/$noCx stock2=$codeTwo/$noTwo"
