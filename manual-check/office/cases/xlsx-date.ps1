# 日期与时间函数缓存对照 Excel 16.0。编号 Xdt。
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

$seed = Get-OutPath 'xlsx-date-seed.xlsx'
$book = Get-OutPath 'xlsx-date.xlsx'
foreach ($p in @($seed, $book)) { if (Test-Path $p) { Remove-Item -Force $p } }

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $seed.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xdt-seed' | Out-Null

$cells = @(
    @{ ref = 'A1'; formula = '=DATE(2024,2,29)' }
    @{ ref = 'A2'; formula = '=DATE(1900,2,28)' }
    @{ ref = 'A3'; formula = '=DATE(1900,3,1)' }
    @{ ref = 'A4'; formula = '=DATE(1900,2,29)' }
    @{ ref = 'A6'; formula = '=DATE(2024,3,4)' }
    @{ ref = 'C1'; formula = '=YEAR(A1)' }
    @{ ref = 'C2'; formula = '=MONTH(A1)' }
    @{ ref = 'C3'; formula = '=DAY(A4)' }
    @{ ref = 'C4'; formula = '=HOUR(1.5)' }
    @{ ref = 'C7'; formula = '=DATEVALUE("2024/2/29")' }
    @{ ref = 'C9'; formula = '=TIME(13,45,6)' }
    @{ ref = 'C11'; formula = '=EDATE(DATE(2024,1,31),1)' }
    @{ ref = 'C12'; formula = '=EOMONTH(A1,1)' }
    @{ ref = 'C15'; formula = '=WEEKDAY(A1)' }
    @{ ref = 'C20'; formula = '=WEEKNUM(DATE(2023,1,1),21)' }
    @{ ref = 'C21'; formula = '=WORKDAY(DATE(2024,3,1),1)' }
    @{ ref = 'C23'; formula = '=WORKDAY(DATE(2024,3,1),1,A6)' }
    @{ ref = 'C25'; formula = '=NETWORKDAYS(DATE(2024,3,4),DATE(2024,3,1))' }
    @{ ref = 'C32'; formula = '=DATEDIF(DATE(2024,1,31),DATE(2024,3,1),"MD")' }
    @{ ref = 'C35'; formula = '=DAYS360(A1,DATE(2025,3,1))' }
    @{ ref = 'C36'; formula = '=DAYS360(DATE(2024,1,31),DATE(2024,2,29),TRUE)' }
    @{ ref = 'C37'; formula = '=YEARFRAC(A1,DATE(2025,3,1))' }
    @{ ref = 'C41'; formula = '=YEARFRAC(DATE(2024,2,29),DATE(2025,3,1),1)' }
    @{ ref = 'C48'; formula = '=TODAY()' }
    @{ ref = 'C50'; formula = '=ECMA.YEAR(A1)' }
)
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $book.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = $cells }) }
} 'xlsx-xdt-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'; format = 'xlsx'
    path = $book.Replace('\', '/')
    scope = @{ sheet = 'Sheet1'; range = 'A1:C50' }
} 'xlsx-xdt-inspect'

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

$leap = @('A1','A2','A3','A4','C1','C3')
$shift = @('C4','C7','C9','C11','C12','C15','C20','C21','C23','C25','C32')
$span = @('C35','C36','C37','C41')
$badLeap = @($leap | ForEach-Object { Same-Cell $ws "cell:Sheet1!$_" $_ } | Where-Object { $_ })
$badShift = @($shift | ForEach-Object { Same-Cell $ws "cell:Sheet1!$_" $_ } | Where-Object { $_ })
$badSpan = @($span | ForEach-Object { Same-Cell $ws "cell:Sheet1!$_" $_ } | Where-Object { $_ })
$today = Cache-Of 'cell:Sheet1!C48'
$todayLive = Read-Number $ws 'C48'
$todayOk = ($null -ne $today.value) -and ($null -ne $todayLive) -and ([math]::Abs([double]$today.value - $todayLive) -lt 0.00001)
$ecma = Cache-Of 'cell:Sheet1!C50'
$ecmaLive = Read-Shown $ws 'C50'
$noRepair = $cap -notmatch 'Repair'
Release-Com $ws
Close-AllWorkbooks $excel

Add-Check 'xlsx-Xdt1' ($noRepair -and $badLeap.Count -eq 0) ("caption=$cap " + ($badLeap -join '; '))
Add-Check 'xlsx-Xdt2' ($noRepair -and $badShift.Count -eq 0) ($badShift -join '; ')
$nameOk = ($null -ne $ecma) -and ($null -eq $ecma.value) -and ($ecmaLive -eq '#NAME?')
Add-Check 'xlsx-Xdt3' ($noRepair -and $badSpan.Count -eq 0 -and $todayOk -and $nameOk) ("span=$($badSpan -join '; ') today=$todayLive ecma=$ecmaLive caption=$cap")

if ($MyInvocation.InvocationName -ne '.') {
    Close-OfficeApps -Quit
}
