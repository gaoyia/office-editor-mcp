# 统计函数缓存对照 Excel 16.0。编号 Xst。
# 先用无公式工作簿启动 Excel，再打开公式簿。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function Find-Cell($cells, [string]$id) {
    foreach ($c in $cells) {
        if ($c.id -eq $id) { return $c }
    }
    return $null
}

$seed = Get-OutPath 'xlsx-stat-seed.xlsx'
$book = Get-OutPath 'xlsx-stat.xlsx'
foreach ($p in @($seed, $book)) { if (Test-Path $p) { Remove-Item -Force $p } }

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $seed.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xst-seed' | Out-Null

$cells = @(
    @{ ref = 'A1'; value = 1 }, @{ ref = 'A2'; value = 2 }, @{ ref = 'A3'; value = 3 }
    @{ ref = 'A4'; value = 4 }, @{ ref = 'A5'; value = 10 }
    @{ ref = 'B1'; value = 'x' }, @{ ref = 'B2'; value = $true }
    @{ ref = 'C1'; value = 2 }, @{ ref = 'C2'; value = 4 }, @{ ref = 'C3'; value = 6 }
    @{ ref = 'C4'; value = 8 }, @{ ref = 'C5'; value = 20 }
    @{ ref = 'F1'; value = 'cat' }, @{ ref = 'F2'; value = 'car' }, @{ ref = 'F3'; value = 'dog' }
    @{ ref = 'G1'; value = 1 }, @{ ref = 'G2'; value = $true }, @{ ref = 'G3'; value = 'x' }
    @{ ref = 'H1'; value = 1 }, @{ ref = 'H2'; value = 2 }, @{ ref = 'H3'; value = 2 }
    @{ ref = 'E1'; formula = '=COUNT(A1:B2)' }
    @{ ref = 'E2'; formula = '=COUNTA(A1:B3)' }
    @{ ref = 'E3'; formula = '=COUNTBLANK(A1:B3)' }
    @{ ref = 'E4'; formula = '=COUNTIF(F1:F3,"C*")' }
    @{ ref = 'E5'; formula = '=AVERAGE(G1:G3)' }
    @{ ref = 'E6'; formula = '=AVERAGEA(G1:G3)' }
    @{ ref = 'E7'; formula = '=STDEV(A1:A5)' }
    @{ ref = 'E8'; formula = '=PERCENTRANK(A1:A5,6)' }
    @{ ref = 'E9'; formula = '=COVAR(A1:A5,C1:C5)' }
    @{ ref = 'E10'; formula = '=NORMDIST(1,0,1,FALSE)' }
    @{ ref = 'E11'; formula = '=CHIDIST(3,2)' }
    @{ ref = 'E12'; formula = '=MODE(H1:H5)' }
    @{ ref = 'E13'; formula = '=MODE(A1:A5)' }
    @{ ref = 'E14'; formula = '=LARGE(A1:A5,9)' }
    @{ ref = 'E15'; formula = '=ECMA.AVERAGE(A1:A5)' }
)
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $book.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = $cells }) }
} 'xlsx-xst-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'; format = 'xlsx'
    path = $book.Replace('\', '/')
    scope = @{ sheet = 'Sheet1'; range = 'A1:H15' }
} 'xlsx-xst-inspect'

$excel = Ensure-MsExcel $seed
$excel.DisplayAlerts = $false
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $book
try { $excel.CalculateFullRebuild() } catch { Write-Log "calc $($_.Exception.Message)" }
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item(1)

function Read-Shown($ws, [string]$addr) {
    $range = $ws.Range($addr)
    try { return [string]$range.Text }
    finally { Release-Com $range }
}

function Read-Number($ws, [string]$addr) {
    $range = $ws.Range($addr)
    try {
        $shown = [string]$range.Text
        if ($shown.StartsWith('#')) { return $null }
        $live = $range.Value2
        if ($null -eq $live) { return $null }
        return [double]$live
    }
    finally { Release-Com $range }
}

function Cache-Of($id) {
    return Find-Cell $tree.sheets[0].cells $id
}

function Same-Cell($ws, [string]$id, [string]$addr) {
    $c = Cache-Of $id
    if ($null -eq $c) { return "$addr missing" }
    if ($c.type -eq 'error') {
        $live = Read-Shown $ws $addr
        if ($live -eq [string]$c.value) { return $null }
        return "$addr live=$live cache=$($c.value)"
    }
    $cached = [double]$c.value
    $liveNum = Read-Number $ws $addr
    if ($null -ne $liveNum -and [math]::Abs($cached - $liveNum) -lt 0.00001) { return $null }
    return "$addr live=$liveNum cache=$cached"
}

$count = @('E1','E2','E3','E4','E5','E6')
$spread = @('E7','E8','E9','E10','E11','E12')
$badCount = @($count | ForEach-Object { Same-Cell $ws "cell:Sheet1!$_" $_ } | Where-Object { $_ })
$badSpread = @($spread | ForEach-Object { Same-Cell $ws "cell:Sheet1!$_" $_ } | Where-Object { $_ })
$mode = Cache-Of 'cell:Sheet1!E13'
$large = Cache-Of 'cell:Sheet1!E14'
$ecma = Cache-Of 'cell:Sheet1!E15'
$modeLive = Read-Shown $ws 'E13'
$largeLive = Read-Shown $ws 'E14'
$ecmaLive = Read-Shown $ws 'E15'
$noRepair = $cap -notmatch 'Repair'
$errOk = ($mode.type -eq 'error') -and ($mode.value -eq '#N/A') -and ($modeLive -eq '#N/A') -and
    ($large.type -eq 'error') -and ($large.value -eq '#NUM!') -and ($largeLive -eq '#NUM!')
$nameOk = ($null -ne $ecma) -and ($null -eq $ecma.value) -and ($ecmaLive -eq '#NAME?')
Release-Com $ws
Close-AllWorkbooks $excel

Add-Check 'xlsx-Xst1' ($noRepair -and $badCount.Count -eq 0) ("caption=$cap " + ($badCount -join '; '))
Add-Check 'xlsx-Xst2' ($noRepair -and $badSpread.Count -eq 0) ($badSpread -join '; ')
Add-Check 'xlsx-Xst3' ($noRepair -and $errOk -and $nameOk) ("mode=$modeLive large=$largeLive ecma=$ecmaLive caption=$cap")

if ($MyInvocation.InvocationName -ne '.') {
    Close-OfficeApps -Quit
}
