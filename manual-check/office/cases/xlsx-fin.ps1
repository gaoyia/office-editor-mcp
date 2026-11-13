# 财务函数缓存对照 Excel 16.0。编号 Xfn。
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

$seed = Get-OutPath 'xlsx-fin-seed.xlsx'
$book = Get-OutPath 'xlsx-fin.xlsx'
foreach ($p in @($seed, $book)) { if (Test-Path $p) { Remove-Item -Force $p } }

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $seed.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xfn-seed' | Out-Null

$cells = @(
    @{ ref = 'A1'; value = -100 }, @{ ref = 'A2'; value = 60 }, @{ ref = 'A3'; value = 60 }
    @{ ref = 'E1'; formula = '=PV(0.08/12,12*20,500)' }
    @{ ref = 'E2'; formula = '=PMT(0.08/12,12*10,10000)' }
    @{ ref = 'E3'; formula = '=RATE(12*10,-100,10000)' }
    @{ ref = 'E4'; formula = '=IRR(A1:A3)' }
    @{ ref = 'E5'; formula = '=PRICE(DATE(2018,4,1),DATE(2021,1,1),0.08,0.09,100,2,0)' }
    @{ ref = 'E6'; formula = '=SLN(10000,1000,5)' }
    @{ ref = 'E7'; formula = '=DOLLARDE(1.02,16)' }
    @{ ref = 'E8'; formula = '=COUPNUM(DATE(2018,4,1),DATE(2021,1,1),2,0)' }
    @{ ref = 'E9'; formula = '=CUMIPMT(0.08/12,120,10000,13,1,0)' }
    @{ ref = 'E10'; formula = '=SLN(1000,100,0)' }
    @{ ref = 'E11'; formula = '=ECMA.PV(0.08/12,12*20,500)' }
)
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $book.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = $cells }) }
} 'xlsx-xfn-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'; format = 'xlsx'
    path = $book.Replace('\', '/')
    scope = @{ sheet = 'Sheet1'; range = 'A1:E11' }
} 'xlsx-xfn-inspect'

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

$loan = @('E1','E2','E3','E4')
$bond = @('E5','E6','E7','E8')
$badLoan = @($loan | ForEach-Object { Same-Cell $ws "cell:Sheet1!$_" $_ } | Where-Object { $_ })
$badBond = @($bond | ForEach-Object { Same-Cell $ws "cell:Sheet1!$_" $_ } | Where-Object { $_ })
$numErr = Cache-Of 'cell:Sheet1!E9'
$divErr = Cache-Of 'cell:Sheet1!E10'
$ecma = Cache-Of 'cell:Sheet1!E11'
$numLive = Read-Shown $ws 'E9'
$divLive = Read-Shown $ws 'E10'
$ecmaLive = Read-Shown $ws 'E11'
$noRepair = $cap -notmatch 'Repair'
$errOk = ($numErr.type -eq 'error') -and ($numErr.value -eq '#NUM!') -and ($numLive -eq '#NUM!') -and
    ($divErr.type -eq 'error') -and ($divErr.value -eq '#DIV/0!') -and ($divLive -eq '#DIV/0!')
$nameOk = ($null -ne $ecma) -and ($null -eq $ecma.value) -and ($ecmaLive -eq '#NAME?')
Release-Com $ws
Close-AllWorkbooks $excel

Add-Check 'xlsx-Xfn1' ($noRepair -and $badLoan.Count -eq 0) ("caption=$cap " + ($badLoan -join '; '))
Add-Check 'xlsx-Xfn2' ($noRepair -and $badBond.Count -eq 0) ($badBond -join '; ')
Add-Check 'xlsx-Xfn3' ($noRepair -and $errOk -and $nameOk) ("num=$numLive div=$divLive ecma=$ecmaLive caption=$cap")

if ($MyInvocation.InvocationName -ne '.') {
    Close-OfficeApps -Quit
}
