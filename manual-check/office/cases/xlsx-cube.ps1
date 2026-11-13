# 没有连接时的多维数据集函数。编号 Xcu。
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

$seed = Get-OutPath 'xlsx-cube-seed.xlsx'
$book = Get-OutPath 'xlsx-cube.xlsx'
foreach ($p in @($seed, $book)) { if (Test-Path $p) { Remove-Item -Force $p } }

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $seed.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xcu-seed' | Out-Null

$cells = @(
    @{ ref = 'A1'; formula = '=CUBEKPIMEMBER("NoCube","Sales","2")' }
    @{ ref = 'A2'; formula = '=CUBEMEMBER("NoCube","[Measures].[Sales]")' }
    @{ ref = 'A3'; formula = '=CUBEMEMBERPROPERTY("NoCube","[Measures].[Sales]","MEMBER_CAPTION")' }
    @{ ref = 'A4'; formula = '=CUBERANKEDMEMBER("NoCube","[Product].[All]",1)' }
    @{ ref = 'A5'; formula = '=CUBESET("NoCube","[Product].[All]")' }
    @{ ref = 'A6'; formula = '=CUBESETCOUNT(A5)' }
    @{ ref = 'A7'; formula = '=CUBEVALUE("NoCube","[Measures].[Sales]")' }
    @{ ref = 'A8'; formula = '=ECMA.CUBEVALUE("NoCube","[Measures].[Sales]")' }
)
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $book.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = $cells }) }
} 'xlsx-xcu-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'; format = 'xlsx'
    path = $book.Replace('\', '/')
    scope = @{ sheet = 'Sheet1'; range = 'A1:A8' }
} 'xlsx-xcu-inspect'

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

function Cache-Of($id) {
    return Find-Cell $tree.sheets[0].cells $id
}

$na = @('A1','A2','A3','A4','A5','A6','A7')
$bad = @()
foreach ($addr in $na) {
    $c = Cache-Of "cell:Sheet1!$addr"
    $live = Read-Shown $ws $addr
    if ($null -eq $c -or $c.type -ne 'error' -or $c.value -ne '#N/A' -or $live -ne '#N/A') {
        $bad += "$addr live=$live cache=$($c.value) type=$($c.type)"
    }
}
$ecma = Cache-Of 'cell:Sheet1!A8'
$ecmaLive = Read-Shown $ws 'A8'
$noRepair = $cap -notmatch 'Repair'
$nameOk = ($null -ne $ecma) -and ($null -eq $ecma.value) -and ($ecmaLive -eq '#NAME?')
Release-Com $ws
Close-AllWorkbooks $excel

Add-Check 'xlsx-Xcu1' ($noRepair -and $bad.Count -eq 0) ("caption=$cap " + ($bad -join '; '))
Add-Check 'xlsx-Xcu2' ($noRepair -and $nameOk) ("ecma=$ecmaLive caption=$cap")

if ($MyInvocation.InvocationName -ne '.') {
    Close-OfficeApps -Quit
}
