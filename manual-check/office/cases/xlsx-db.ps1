# 数据库函数缓存对照 Excel 16.0。编号 Xdb。
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

$seed = Get-OutPath 'xlsx-db-seed.xlsx'
$book = Get-OutPath 'xlsx-db.xlsx'
foreach ($p in @($seed, $book)) { if (Test-Path $p) { Remove-Item -Force $p } }

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $seed.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xdb-seed' | Out-Null

$cells = @(
    @{ ref = 'A1'; value = '树' }, @{ ref = 'B1'; value = '高度' }, @{ ref = 'D1'; value = '产量' }
    @{ ref = 'A2'; value = 'Apple' }, @{ ref = 'B2'; value = 18 }, @{ ref = 'D2'; value = 3 }
    @{ ref = 'A3'; value = 'Pear' }, @{ ref = 'B3'; value = 12 }, @{ ref = 'D3'; value = 5 }
    @{ ref = 'A4'; value = 'Apple' }, @{ ref = 'B4'; value = 14 }, @{ ref = 'D4'; value = 2 }
    @{ ref = 'G1'; value = '树' }, @{ ref = 'G2'; value = 'Apple' }
    @{ ref = 'J1'; value = '树' }, @{ ref = 'J2'; value = 'A*' }
    @{ ref = 'K1'; value = '树' }, @{ ref = 'L1'; value = '高度' }
    @{ ref = 'K2'; value = 'Apple' }, @{ ref = 'K3'; value = 'Pear' }
    @{ ref = 'M1'; value = '树' }, @{ ref = 'N1'; value = '高度' }
    @{ ref = 'M2'; value = 'Apple' }, @{ ref = 'N2'; value = '>15' }
    @{ ref = 'O1'; value = '树' }, @{ ref = 'O2'; value = 'None' }
    @{ ref = 'P1'; value = '树' }, @{ ref = 'P2'; value = 'Pear' }
    @{ ref = 'E1'; formula = '=DSUM(A1:D4,"高度",G1:G2)' }
    @{ ref = 'E2'; formula = '=DCOUNT(A1:D4,"高度",G1:G2)' }
    @{ ref = 'E3'; formula = '=DAVERAGE(A1:D4,"高度",G1:G2)' }
    @{ ref = 'E4'; formula = '=DMAX(A1:D4,"高度",G1:G2)' }
    @{ ref = 'E5'; formula = '=DSUM(A1:D4,"高度",M1:N2)' }
    @{ ref = 'E6'; formula = '=DSUM(A1:D4,"高度",K1:L3)' }
    @{ ref = 'E7'; formula = '=DSUM(A1:D4,"高度",J1:J2)' }
    @{ ref = 'E8'; formula = '=DGET(A1:D4,"高度",P1:P2)' }
    @{ ref = 'E9'; formula = '=DGET(A1:D4,"高度",G1:G2)' }
    @{ ref = 'E10'; formula = '=DGET(A1:D4,"高度",O1:O2)' }
    @{ ref = 'E11'; formula = '=DAVERAGE(A1:D4,"高度",O1:O2)' }
    @{ ref = 'E12'; formula = '=ECMA.DSUM(A1:D4,"高度",G1:G2)' }
)
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $book.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = $cells }) }
} 'xlsx-xdb-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'; format = 'xlsx'
    path = $book.Replace('\', '/')
    scope = @{ sheet = 'Sheet1'; range = 'A1:P12' }
} 'xlsx-xdb-inspect'

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

$agg = @('E1','E2','E3','E4')
$filt = @('E5','E6','E7','E8')
$badAgg = @($agg | ForEach-Object { Same-Cell $ws "cell:Sheet1!$_" $_ } | Where-Object { $_ })
$badFilt = @($filt | ForEach-Object { Same-Cell $ws "cell:Sheet1!$_" $_ } | Where-Object { $_ })
$multi = Cache-Of 'cell:Sheet1!E9'
$none = Cache-Of 'cell:Sheet1!E10'
$div = Cache-Of 'cell:Sheet1!E11'
$ecma = Cache-Of 'cell:Sheet1!E12'
$multiLive = Read-Shown $ws 'E9'
$noneLive = Read-Shown $ws 'E10'
$divLive = Read-Shown $ws 'E11'
$ecmaLive = Read-Shown $ws 'E12'
$noRepair = $cap -notmatch 'Repair'
$errOk = ($multi.type -eq 'error') -and ($multi.value -eq '#NUM!') -and ($multiLive -eq '#NUM!') -and
    ($none.type -eq 'error') -and ($none.value -eq '#VALUE!') -and ($noneLive -eq '#VALUE!') -and
    ($div.type -eq 'error') -and ($div.value -eq '#DIV/0!') -and ($divLive -eq '#DIV/0!')
$nameOk = ($null -ne $ecma) -and ($null -eq $ecma.value) -and ($ecmaLive -eq '#NAME?')
Release-Com $ws
Close-AllWorkbooks $excel

Add-Check 'xlsx-Xdb1' ($noRepair -and $badAgg.Count -eq 0) ("caption=$cap " + ($badAgg -join '; '))
Add-Check 'xlsx-Xdb2' ($noRepair -and $badFilt.Count -eq 0) ($badFilt -join '; ')
Add-Check 'xlsx-Xdb3' ($noRepair -and $errOk -and $nameOk) ("multi=$multiLive none=$noneLive div=$divLive ecma=$ecmaLive caption=$cap")

if ($MyInvocation.InvocationName -ne '.') {
    Close-OfficeApps -Quit
}
