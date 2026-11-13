# 工程函数缓存对照 Excel 16.0。编号 Xeg。
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

$seed = Get-OutPath 'xlsx-eng-seed.xlsx'
$book = Get-OutPath 'xlsx-eng.xlsx'
foreach ($p in @($seed, $book)) { if (Test-Path $p) { Remove-Item -Force $p } }

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $seed.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xeg-seed' | Out-Null

$cells = @(
    @{ ref = 'E1'; formula = '=BESSELI(1.5,1)' }
    @{ ref = 'E2'; formula = '=BESSELJ(1.5,1)' }
    @{ ref = 'E3'; formula = '=BESSELK(1.5,1)' }
    @{ ref = 'E4'; formula = '=BESSELY(1.5,1)' }
    @{ ref = 'E5'; formula = '=ERF(1)' }
    @{ ref = 'E6'; formula = '=ERFC(1)' }
    @{ ref = 'E7'; formula = '=CONVERT(1,"lbm","kg")' }
    @{ ref = 'E8'; formula = '=CONVERT(100,"C","F")' }
    @{ ref = 'E9'; formula = '=DELTA(5,5)' }
    @{ ref = 'E10'; formula = '=GESTEP(5)' }
    @{ ref = 'E11'; formula = '=COMPLEX(3,4)' }
    @{ ref = 'E12'; formula = '=COMPLEX(0,1)' }
    @{ ref = 'E13'; formula = '=DEC2BIN(10,8)' }
    @{ ref = 'E14'; formula = '=DEC2BIN(-1,4)' }
    @{ ref = 'E15'; formula = '=IMDIV("1+2i","3+4i")' }
    @{ ref = 'E16'; formula = '=IMSQRT("3+4i")' }
    @{ ref = 'E17'; formula = '=IMCOS("3+4i")' }
    @{ ref = 'E18'; formula = '=DEC2BIN(512)' }
    @{ ref = 'E19'; formula = '=BIN2DEC("2")' }
    @{ ref = 'E20'; formula = '=CONVERT(1,"m","kg")' }
    @{ ref = 'E21'; formula = '=ECMA.DEC2BIN(10)' }
    @{ ref = 'E22'; formula = '=IMSUM("i","j")' }
)
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $book.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = $cells }) }
} 'xlsx-xeg-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'; format = 'xlsx'
    path = $book.Replace('\', '/')
    scope = @{ sheet = 'Sheet1'; range = 'A1:E22' }
} 'xlsx-xeg-inspect'

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

function Same-Number($ws, [string]$id, [string]$addr) {
    $c = Cache-Of $id
    if ($null -eq $c) { return "$addr missing" }
    $cached = [double]$c.value
    $liveNum = Read-Number $ws $addr
    if ($null -ne $liveNum -and [math]::Abs($cached - $liveNum) -lt 0.00001) { return $null }
    return "$addr live=$liveNum cache=$cached"
}

function Same-Text($ws, [string]$id, [string]$addr) {
    $c = Cache-Of $id
    if ($null -eq $c) { return "$addr missing" }
    $live = Read-Shown $ws $addr
    if ($c.type -eq 'text' -and $live -eq [string]$c.value) { return $null }
    return "$addr live=$live cache=$($c.value) type=$($c.type)"
}

$nums = @('E1','E2','E3','E4','E5','E6','E7','E8','E9','E10')
$texts = @('E11','E12','E13','E14','E15','E16','E17')
$badNum = @($nums | ForEach-Object { Same-Number $ws "cell:Sheet1!$_" $_ } | Where-Object { $_ })
$badText = @($texts | ForEach-Object { Same-Text $ws "cell:Sheet1!$_" $_ } | Where-Object { $_ })
$overflow = Cache-Of 'cell:Sheet1!E18'
$badDigit = Cache-Of 'cell:Sheet1!E19'
$unit = Cache-Of 'cell:Sheet1!E20'
$ecma = Cache-Of 'cell:Sheet1!E21'
$suffix = Cache-Of 'cell:Sheet1!E22'
$overflowLive = Read-Shown $ws 'E18'
$digitLive = Read-Shown $ws 'E19'
$unitLive = Read-Shown $ws 'E20'
$ecmaLive = Read-Shown $ws 'E21'
$suffixLive = Read-Shown $ws 'E22'
$noRepair = $cap -notmatch 'Repair'
$errOk = ($overflow.type -eq 'error') -and ($overflow.value -eq '#NUM!') -and ($overflowLive -eq '#NUM!') -and
    ($badDigit.type -eq 'error') -and ($badDigit.value -eq '#NUM!') -and ($digitLive -eq '#NUM!') -and
    ($unit.type -eq 'error') -and ($unit.value -eq '#N/A') -and ($unitLive -eq '#N/A') -and
    ($suffix.type -eq 'error') -and ($suffix.value -eq '#VALUE!') -and ($suffixLive -eq '#VALUE!')
$nameOk = ($null -ne $ecma) -and ($null -eq $ecma.value) -and ($ecmaLive -eq '#NAME?')
Release-Com $ws
Close-AllWorkbooks $excel

Add-Check 'xlsx-Xeg1' ($noRepair -and $badNum.Count -eq 0) ("caption=$cap " + ($badNum -join '; '))
Add-Check 'xlsx-Xeg2' ($noRepair -and $badText.Count -eq 0) ($badText -join '; ')
Add-Check 'xlsx-Xeg3' ($noRepair -and $errOk -and $nameOk) ("overflow=$overflowLive digit=$digitLive unit=$unitLive suffix=$suffixLive ecma=$ecmaLive caption=$cap")

if ($MyInvocation.InvocationName -ne '.') {
    Close-OfficeApps -Quit
}
