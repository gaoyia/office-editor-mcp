# xlsx GETPIVOTDATA 其余版式：docs/checklist-xlsx.md 的 Xpt8。
# 库按已写入的格子回答。Excel 16.0 打开后应一致，且没有修复。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'xlsx-pivot-get-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
$cells = @(
    @{ ref = 'A1'; value = '产品' }, @{ ref = 'B1'; value = '地区' }, @{ ref = 'C1'; value = '金额' },
    @{ ref = 'A2'; value = '甲' }, @{ ref = 'B2'; value = '东' }, @{ ref = 'C2'; value = 10 },
    @{ ref = 'A3'; value = '甲' }, @{ ref = 'B3'; value = '西' }, @{ ref = 'C3'; value = 5 },
    @{ ref = 'A4'; value = '乙' }, @{ ref = 'B4'; value = '东' }, @{ ref = 'C4'; value = 7 },
    @{ ref = 'A10'; value = '渠道' }, @{ ref = 'B10'; value = '产品' }, @{ ref = 'C10'; value = '地区' },
    @{ ref = 'D10'; value = '金额' }, @{ ref = 'E10'; value = '数量' },
    @{ ref = 'A11'; value = '线上' }, @{ ref = 'B11'; value = '甲' }, @{ ref = 'C11'; value = '东' }, @{ ref = 'D11'; value = 10 }, @{ ref = 'E11'; value = 1 },
    @{ ref = 'A12'; value = '线上' }, @{ ref = 'B12'; value = '甲' }, @{ ref = 'C12'; value = '西' }, @{ ref = 'D12'; value = 5 }, @{ ref = 'E12'; value = 1 },
    @{ ref = 'A13'; value = '线上' }, @{ ref = 'B13'; value = '乙' }, @{ ref = 'C13'; value = '东' }, @{ ref = 'D13'; value = 7 }, @{ ref = 'E13'; value = 1 },
    @{ ref = 'A14'; value = '线下' }, @{ ref = 'B14'; value = '甲' }, @{ ref = 'C14'; value = '东' }, @{ ref = 'D14'; value = 3 }, @{ ref = 'E14'; value = 2 },
    @{ ref = 'AA5'; formula = '=GETPIVOTDATA("金额",$E$1,"产品","乙","地区","西")' },
    @{ ref = 'AA11'; formula = '=GETPIVOTDATA("金额",$E$16,"产品","乙","地区","西")' },
    @{ ref = 'AA13'; formula = '=GETPIVOTDATA("金额",$E$40)' },
    @{ ref = 'AA14'; formula = '=GETPIVOTDATA("金额",$E$40,"产品","甲")' },
    @{ ref = 'AA19'; formula = '=GETPIVOTDATA("金额",$I$1,"地区","东")' },
    @{ ref = 'AA22'; formula = '=GETPIVOTDATA("金额",$I$1,"渠道","线上")' },
    @{ ref = 'AA32'; formula = '=GETPIVOTDATA("数量",$I$60)' }
)
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = $cells }) }
} 'xlsx-xpt8-create' | Out-Null

$out = Get-OutPath 'xlsx-pivot-get.xlsx'
if (Test-Path $out) { Remove-Item -Force $out }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(
        @{ op = 'insertPivotTable'; id = 'sheet:Sheet1'; source = @{ sheet = 'Sheet1'; ref = 'A1:C4' }; location = 'E1'; rows = @('产品', '地区'); values = @('金额') },
        @{ op = 'insertPivotTable'; id = 'sheet:Sheet1'; source = @{ sheet = 'Sheet1'; ref = 'A1:C4' }; location = 'E16'; rows = @('产品'); cols = @('地区'); values = @('金额') },
        @{ op = 'insertPivotTable'; id = 'sheet:Sheet1'; source = @{ sheet = 'Sheet1'; ref = 'A1:C4' }; location = 'E40'; rows = @('产品', '地区'); values = @(@{ field = '金额'; function = 'average' }) },
        @{ op = 'insertPivotTable'; id = 'sheet:Sheet1'; source = @{ sheet = 'Sheet1'; ref = 'A10:E14' }; location = 'I1'; rows = @('产品'); cols = @('地区', '渠道'); values = @('金额') },
        @{ op = 'insertPivotTable'; id = 'sheet:Sheet1'; source = @{ sheet = 'Sheet1'; ref = 'A10:E14' }; location = 'I60'; rows = @('产品'); cols = @('地区'); values = @('金额', '数量') }
    )
} 'xlsx-xpt8-apply' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $out.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'AA5:AA32' }
} 'xlsx-xpt8-inspect'

function Get-CellValue($id) {
    $hit = @($tree.sheets[0].cells | Where-Object { $_.id -eq $id } | Select-Object -First 1)
    if ($hit.Count -eq 0) { return $null }
    return $hit[0].value
}

$libOk = (
    ((Get-CellValue 'cell:Sheet1!AA5') -eq '#REF!') -and
    ([double](Get-CellValue 'cell:Sheet1!AA11') -eq 0) -and
    ([math]::Abs([double](Get-CellValue 'cell:Sheet1!AA13') - (22 / 3)) -lt 1e-6) -and
    ([double](Get-CellValue 'cell:Sheet1!AA14') -eq 7.5) -and
    ([double](Get-CellValue 'cell:Sheet1!AA19') -eq 20) -and
    ((Get-CellValue 'cell:Sheet1!AA22') -eq '#REF!') -and
    ([double](Get-CellValue 'cell:Sheet1!AA32') -eq 5)
)

$excel = Ensure-MsExcel $out
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $out
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item(1)

function Same-Num($addr, $expect) {
    $got = $ws.Range($addr).Value2
    if ($null -eq $got) { return $false }
    return [math]::Abs([double]$got - [double]$expect) -lt 1e-4
}
function Same-Err($addr) {
    return ([string]$ws.Range($addr).Text) -eq '#REF!'
}

$xlOk = (
    (Same-Err 'AA5') -and (Same-Num 'AA11' 0) -and
    (Same-Num 'AA13' (22 / 3)) -and (Same-Num 'AA14' 7.5) -and
    (Same-Num 'AA19' 20) -and (Same-Err 'AA22') -and (Same-Num 'AA32' 5)
)
Close-AllWorkbooks $excel

$ok = $libOk -and $xlOk -and ($cap -notmatch 'Repair')
Add-Check 'xlsx-Xpt8' $ok "lib=$libOk xl=$xlOk caption=$cap"
