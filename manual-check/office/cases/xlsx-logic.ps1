# 逻辑函数缓存对照 Excel 16.0。编号 Xlg。
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

function Read-LiveText($ws, [string]$addr) {
    $range = $ws.Range($addr)
    try { return [string]$range.Text }
    finally { Release-Com $range }
}

$seed = Get-OutPath 'xlsx-logic-seed.xlsx'
$book = Get-OutPath 'xlsx-logic.xlsx'
foreach ($p in @($seed, $book)) { if (Test-Path $p) { Remove-Item -Force $p } }

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $seed.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xlg-seed' | Out-Null

$cells = @(
    @{ ref = 'A1'; formula = '=IF(TRUE,1,1/0)' }
    @{ ref = 'A2'; formula = '=IF(FALSE,1/0,2)' }
    @{ ref = 'A3'; formula = '=IFERROR(1/0,7)' }
    @{ ref = 'A4'; formula = '=IFERROR(3,1/0)' }
    @{ ref = 'A5'; formula = '=AND(FALSE,1/0)' }
    @{ ref = 'A6'; formula = '=OR(TRUE,1/0)' }
    @{ ref = 'A7'; formula = '=AND(TRUE,"x")' }
    @{ ref = 'A8'; formula = '=AND("x")' }
    @{ ref = 'A9'; formula = '=NOT("a")' }
    @{ ref = 'A10'; formula = '=TRUE()' }
    @{ ref = 'A11'; formula = '=FALSE()' }
    @{ ref = 'A12'; formula = '=IF(FALSE,5)' }
    @{ ref = 'A13'; formula = '=ECMA.IF(TRUE,4,1/0)' }
    @{ ref = 'B1'; value = $true }
    @{ ref = 'B2'; value = 'x' }
    @{ ref = 'B3'; formula = '=AND(B1:B2)' }
    @{ ref = 'B4'; formula = '=IF(B6,1,2)' }
    @{ ref = 'B5'; formula = '=IFERROR(B6,"e")' }
)
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $book.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = $cells }) }
} 'xlsx-xlg-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'; format = 'xlsx'
    path = $book.Replace('\', '/')
    scope = @{ sheet = 'Sheet1'; range = 'A1:B13' }
} 'xlsx-xlg-inspect'

$excel = Ensure-MsExcel $seed
$excel.DisplayAlerts = $false
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $book
try { $excel.CalculateFullRebuild() } catch { Write-Log "calc $($_.Exception.Message)" }
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item(1)

function Cache-Text($id) {
    $c = Find-Cell $tree.sheets[0].cells $id
    if ($null -eq $c -or $null -eq $c.value) { return '' }
    if ($c.type -eq 'boolean') { if ($c.value) { return 'TRUE' } else { return 'FALSE' } }
    return [string]$c.value
}

$expect = @(
    @{ id = 'cell:Sheet1!A1'; addr = 'A1'; text = '1' }
    @{ id = 'cell:Sheet1!A2'; addr = 'A2'; text = '2' }
    @{ id = 'cell:Sheet1!A3'; addr = 'A3'; text = '7' }
    @{ id = 'cell:Sheet1!A4'; addr = 'A4'; text = '3' }
    @{ id = 'cell:Sheet1!A5'; addr = 'A5'; text = '#DIV/0!' }
    @{ id = 'cell:Sheet1!A6'; addr = 'A6'; text = '#DIV/0!' }
    @{ id = 'cell:Sheet1!A7'; addr = 'A7'; text = 'TRUE' }
    @{ id = 'cell:Sheet1!A8'; addr = 'A8'; text = '#VALUE!' }
    @{ id = 'cell:Sheet1!A9'; addr = 'A9'; text = '#VALUE!' }
    @{ id = 'cell:Sheet1!A10'; addr = 'A10'; text = 'TRUE' }
    @{ id = 'cell:Sheet1!A11'; addr = 'A11'; text = 'FALSE' }
    @{ id = 'cell:Sheet1!A12'; addr = 'A12'; text = 'FALSE' }
    @{ id = 'cell:Sheet1!B3'; addr = 'B3'; text = 'TRUE' }
    @{ id = 'cell:Sheet1!B4'; addr = 'B4'; text = '2' }
    @{ id = 'cell:Sheet1!B5'; addr = 'B5'; text = '0' }
)

$bad = @()
foreach ($row in $expect) {
    $live = Read-LiveText $ws $row.addr
    $cached = Cache-Text $row.id
    if ($live -ne $row.text -or $cached -ne $row.text) {
        $bad += "$($row.addr) live=$live cache=$cached expect=$($row.text)"
    }
}
$ecma = Find-Cell $tree.sheets[0].cells 'cell:Sheet1!A13'
$ecmaLive = Read-LiveText $ws 'A13'
$noRepair = $cap -notmatch 'Repair'
Release-Com $ws
Close-AllWorkbooks $excel

$short = @($bad | Where-Object { $_ -match '^(A1|A2|A3|A4) ' })
Add-Check 'xlsx-Xlg1' ($noRepair -and $short.Count -eq 0) ("caption=$cap " + ($short -join '; '))
$logic = @($bad | Where-Object { $_ -notmatch '^(A1|A2|A3|A4) ' })
Add-Check 'xlsx-Xlg2' ($noRepair -and $logic.Count -eq 0) ($logic -join '; ')
$ecmaOk = $noRepair -and ($null -ne $ecma) -and ($null -eq $ecma.value) -and ($ecmaLive -eq '#NAME?')
Add-Check 'xlsx-Xlg3' $ecmaOk "cache=$($ecma.value) excel=$ecmaLive caption=$cap"

if ($MyInvocation.InvocationName -ne '.') {
    Close-OfficeApps -Quit
}
