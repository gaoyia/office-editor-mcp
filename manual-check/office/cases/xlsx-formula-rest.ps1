# 公式对照里还能造出来的类：各记一条 Excel 16.0 结果。
# 编号 Xfr。单独运行时会关掉自己启动的 Excel。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function Find-Cell($cells, [string]$id) {
    foreach ($c in $cells) {
        if ($c.id -eq $id) { return $c }
    }
    return $null
}

function Read-LiveCell($ws, [string]$addr) {
    $range = $ws.Range($addr)
    try {
        $text = [string]$range.Text
        $value = $null
        try { $value = $range.Value2 } catch { $value = $text }
        return @{ text = $text; value = $value }
    }
    finally { Release-Com $range }
}

function Same-Number($live, $cached) {
    if ($null -eq $live -or $null -eq $cached) { return $false }
    $a = 0.0
    $b = 0.0
    if (-not [double]::TryParse([string]$live, [ref]$a)) { return $false }
    if (-not [double]::TryParse([string]$cached, [ref]$b)) { return $false }
    return [math]::Abs($a - $b) -lt 0.001
}

function Open-Calc($excel, [string]$path) {
    Close-AllWorkbooks $excel
    $wb = Open-ExcelBook $excel $path
    try { $excel.CalculateFullRebuild() } catch { Write-Log "CalculateFullRebuild $($_.Exception.Message)" }
    return $wb
}

function Part-Text([string]$path, [string]$name) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($path)
    try {
        $entry = $zip.GetEntry($name)
        if (-not $entry) { return $null }
        $reader = New-Object IO.StreamReader($entry.Open())
        try { return $reader.ReadToEnd() } finally { $reader.Dispose() }
    }
    finally { $zip.Dispose() }
}

# 循环引用。迭代关闭。
# 先用一份没有环的工作簿把 Excel 拉起来，再打开样本。
# 启动参数会在关掉提示之前打开文件，循环引用提示框会挡住自动化。
$seed = Get-OutPath 'xlsx-formula-rest-seed.xlsx'
$circ = Get-OutPath 'xlsx-formula-rest-circ.xlsx'
foreach ($p in @($seed, $circ)) { if (Test-Path $p) { Remove-Item -Force $p } }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $seed.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xfr-seed' | Out-Null
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $circ.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{
        name = 'Sheet1'
        cells = @(
            @{ ref = 'A1'; formula = '=B1+1' }
            @{ ref = 'B1'; formula = '=A1+1' }
        )
    }) }
} 'xlsx-xfr-circ' | Out-Null
$excel = Ensure-MsExcel $seed
$excel.Iteration = $false
$excel.DisplayAlerts = $false
$wb = Open-Calc $excel $circ
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item('Sheet1')
$a1 = Read-LiveCell $ws 'A1'
$b1 = Read-LiveCell $ws 'B1'
Write-Log "circ A1=$($a1.text)/$($a1.value) B1=$($b1.text)/$($b1.value) caption=$cap"
$wb.Save()
Release-Com $ws
Close-AllWorkbooks $excel
$circTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $circ.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:B1' }
} 'xlsx-xfr-circ-inspect'
$ia = Find-Cell $circTree.sheets[0].cells 'cell:Sheet1!A1'
$ib = Find-Cell $circTree.sheets[0].cells 'cell:Sheet1!B1'
Write-Log "circ inspect A1=$($ia.type)/$($ia.value) B1=$($ib.type)/$($ib.value)"
Add-Check 'xlsx-Xfr-circ' (($cap -notmatch 'Repair') -and ($a1.text -eq '#VALUE!') -and ($b1.text -eq '#VALUE!') -and ($ia.type -eq 'error') -and ($ia.value -eq '#VALUE!') -and ($ib.type -eq 'error') -and ($ib.value -eq '#VALUE!')) "A1=$($a1.text) B1=$($b1.text) inspect=$($ia.value),$($ib.value) caption=$cap"

# 外部工作簿。
$src = Get-OutPath 'xlsx-formula-rest-src.xlsx'
$hostBook = Get-OutPath 'xlsx-formula-rest-ext.xlsx'
foreach ($p in @($src, $hostBook)) { if (Test-Path $p) { Remove-Item -Force $p } }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 9 }) }) }
} 'xlsx-xfr-src' | Out-Null
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostBook.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xfr-host' | Out-Null
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostBook.Replace('\', '/'); format = 'xlsx'
    ops = @(
        @{ op = 'insertExternalLink'; id = 'xlsx:workbook'; path = $src; sheets = @('Sheet1') }
        @{ op = 'setCell'; id = 'cell:Sheet1!B1'; formula = '=[1]Sheet1!A1' }
    )
} 'xlsx-xfr-link' | Out-Null
$wb = $excel.Workbooks.Open($hostBook, 3, $false)
try { $excel.CalculateFullRebuild() } catch { Write-Log "ext calc $($_.Exception.Message)" }
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item('Sheet1')
$ext = Read-LiveCell $ws 'B1'
Write-Log "ext B1=$($ext.text)/$($ext.value) caption=$cap"
$wb.Save()
Release-Com $ws
Close-AllWorkbooks $excel
Add-Check 'xlsx-Xfr-ext' (($cap -notmatch 'Repair') -and (Same-Number $ext.value 9)) "B1=$($ext.value) caption=$cap"

# 图表公式。先建图，再改一个源格。
$chart = Get-OutPath 'xlsx-formula-rest-chart.xlsx'
if (Test-Path $chart) { Remove-Item -Force $chart }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $chart.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{
        name = 'Sheet1'
        charts = @(@{
            type = 'col'; from = 'E2'; dataFrom = 'A1'; title = '数'
            series = @(@{ name = '数'; categories = @('甲', '乙', '丙'); values = @(1, 2, 3) })
        })
    }) }
} 'xlsx-xfr-chart' | Out-Null
$before = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $chart.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:C5'; charts = $true }
} 'xlsx-xfr-chart-before'
$series = $before.sheets[0].charts[0].series[0]
Write-Log "chart valRef=$($series.valRef) values=$($series.values -join ',') stale=$($series.cacheStale)"
$wb = Open-Calc $excel $chart
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item('Sheet1')
$c2 = Read-LiveCell $ws 'B2'
Write-Log "chart live B2=$($c2.text)/$($c2.value) caption=$cap"
$wb.Save()
Release-Com $ws
Close-AllWorkbooks $excel
$after = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $chart.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:C5'; charts = $true }
} 'xlsx-xfr-chart-after'
$series2 = $after.sheets[0].charts[0].series[0]
Write-Log "chart after valRef=$($series2.valRef) values=$($series2.values -join ',') stale=$($series2.cacheStale)"
Add-Check 'xlsx-Xfr-chart' (($cap -notmatch 'Repair') -and ($series2.valRef -eq 'Sheet1!$B$2:$B$4') -and (($series2.values -join ',') -eq '1,2,3') -and (-not $series2.cacheStale)) "valRef=$($series2.valRef) values=$($series2.values -join ',') stale=$($series2.cacheStale) caption=$cap"

# 迷你图、条件格式公式、数据验证公式。
$sheet = Get-OutPath 'xlsx-formula-rest-sheet.xlsx'
if (Test-Path $sheet) { Remove-Item -Force $sheet }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $sheet.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{
        name = 'Sheet1'
        cells = @(
            @{ ref = 'A1'; value = 2 }
            @{ ref = 'A2'; value = 4 }
            @{ ref = 'A3'; value = 6 }
        )
        sparklines = @(@{ ref = 'D1'; formula = 'A1:A3' })
        conditionalFormats = @(@{
            sqref = 'B1'; type = 'expression'; formulas = @('A1>1')
            dxf = @{ font = @{ color = @{ rgb = 'FFFF0000' } } }
        })
        dataValidations = @(@{ sqref = 'C1'; type = 'custom'; formula1 = 'A1>0' })
    }) }
} 'xlsx-xfr-sheet' | Out-Null
$wb = Open-Calc $excel $sheet
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item('Sheet1')
$liveA1 = Read-LiveCell $ws 'A1'
Write-Log "sheet A1=$($liveA1.value) caption=$cap"
$wb.Save()
Release-Com $ws
Close-AllWorkbooks $excel
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sheet.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:D3'; sparklines = $true }
} 'xlsx-xfr-sheet-inspect'
$spark = $tree.sheets[0].sparklines[0].sparklines[0].formula
$cf = $tree.sheets[0].conditionalFormats[0].formulas -join '|'
$dv = $tree.sheets[0].dataValidations[0].formula1
Write-Log "spark=$spark cf=$cf dv=$dv"
Add-Check 'xlsx-Xfr-sheet' (($cap -notmatch 'Repair') -and ($spark -eq 'Sheet1!A1:A3') -and ($cf -eq 'A1>1') -and ($dv -eq 'A1>0') -and (Same-Number $liveA1.value 2)) "spark=$spark cf=$cf dv=$dv caption=$cap"

# 透视。
$pivot = Get-OutPath 'xlsx-formula-rest-pivot.xlsx'
if (Test-Path $pivot) { Remove-Item -Force $pivot }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $pivot.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{
        name = 'Sheet1'
        cells = @(
            @{ ref = 'A1'; value = '产品' }
            @{ ref = 'B1'; value = '金额' }
            @{ ref = 'A2'; value = '甲' }
            @{ ref = 'B2'; value = 10 }
            @{ ref = 'A3'; value = '甲' }
            @{ ref = 'B3'; value = 5 }
            @{ ref = 'A4'; value = '乙' }
            @{ ref = 'B4'; value = 7 }
        )
        pivots = @(@{
            source = @{ sheet = 'Sheet1'; ref = 'A1:B4' }
            location = 'E1'; name = 'P'; rows = @('产品'); values = @('金额')
        })
    }) }
} 'xlsx-xfr-pivot' | Out-Null
$wb = Open-Calc $excel $pivot
try { $wb.RefreshAll() } catch { Write-Log "RefreshAll $($_.Exception.Message)" }
try { $excel.CalculateFullRebuild() } catch { Write-Log "pivot calc $($_.Exception.Message)" }
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item('Sheet1')
$lines = @()
foreach ($addr in @('E1', 'F1', 'E2', 'F2', 'E3', 'F3', 'E4', 'F4')) {
    $live = Read-LiveCell $ws $addr
    $lines += "$addr=$($live.text)"
}
Write-Log ("pivot " + ($lines -join ' ') + " caption=$cap")
$wb.Save()
Release-Com $ws
Close-AllWorkbooks $excel
$pivotText = $lines -join ' '
Add-Check 'xlsx-Xfr-pivot' (($cap -notmatch 'Repair') -and ($pivotText -eq 'E1=产品 F1=金额 E2=甲 F2=15 E3=乙 F3=7 E4=总计 F4=22')) "caption=$cap $pivotText"

# 计算链。普通公式交给 Excel 保存。
$chain = Get-OutPath 'xlsx-formula-rest-chain.xlsx'
if (Test-Path $chain) { Remove-Item -Force $chain }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $chain.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{
        name = 'Sheet1'
        cells = @(
            @{ ref = 'A1'; value = 1 }
            @{ ref = 'B1'; formula = '=A1+1' }
            @{ ref = 'C1'; formula = '=B1*2' }
        )
    }) }
} 'xlsx-xfr-chain' | Out-Null
$wb = Open-Calc $excel $chain
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item('Sheet1')
$c1 = Read-LiveCell $ws 'C1'
$wb.Save()
Release-Com $ws
Close-AllWorkbooks $excel
$chainXml = Part-Text $chain 'xl/calcChain.xml'
Write-Log "chain C1=$($c1.value) xml=$chainXml caption=$cap"
Add-Check 'xlsx-Xfr-chain' (($cap -notmatch 'Repair') -and (Same-Number $c1.value 4) -and ($chainXml -match 'r="B1"') -and ($chainXml -match 'r="C1"[^>]*s="1"')) "C1=$($c1.value) hasChain=$([bool]$chainXml) caption=$cap"

if ($MyInvocation.InvocationName -ne '.') {
    Close-OfficeApps -Quit
}
