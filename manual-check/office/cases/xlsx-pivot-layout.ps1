# xlsx 透视其余版式：docs/checklist-xlsx.md 的 Xpt6。
# 库先把合计写进格子。Excel 16.0 打开后应与这些格子一致，且没有修复。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'xlsx-pivot-layout-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = '产品' },
                @{ ref = 'B1'; value = '地区' },
                @{ ref = 'C1'; value = '金额' },
                @{ ref = 'A2'; value = '甲' },
                @{ ref = 'B2'; value = '东' },
                @{ ref = 'C2'; value = 10 },
                @{ ref = 'A3'; value = '甲' },
                @{ ref = 'B3'; value = '西' },
                @{ ref = 'C3'; value = 5 },
                @{ ref = 'A4'; value = '乙' },
                @{ ref = 'B4'; value = '东' },
                @{ ref = 'C4'; value = 7 }
            )
        })
    }
} 'xlsx-xpt6-create' | Out-Null

$out = Get-OutPath 'xlsx-pivot-layout.xlsx'
if (Test-Path $out) { Remove-Item -Force $out }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(
        @{
            op = 'insertPivotTable'; id = 'sheet:Sheet1'
            source = @{ sheet = 'Sheet1'; ref = 'A1:C4' }
            location = 'E1'; rows = @('产品', '地区'); values = @('金额')
        },
        @{
            op = 'insertPivotTable'; id = 'sheet:Sheet1'
            source = @{ sheet = 'Sheet1'; ref = 'A1:C4' }
            location = 'E16'; rows = @('产品'); cols = @('地区'); values = @('金额')
        },
        @{
            op = 'insertPivotTable'; id = 'sheet:Sheet1'
            source = @{ sheet = 'Sheet1'; ref = 'A1:C4' }
            location = 'E32'; rows = @('产品'); filters = @('地区'); values = @('金额')
        },
        @{
            op = 'insertPivotTable'; id = 'sheet:Sheet1'
            source = @{ sheet = 'Sheet1'; ref = 'A1:C4' }
            location = 'E40'; rows = @('产品')
            values = @(@{ field = '金额'; function = 'count' })
        },
        @{
            op = 'insertPivotTable'; id = 'sheet:Sheet1'
            source = @{ sheet = 'Sheet1'; ref = 'A1:C4' }
            location = 'E48'; rows = @('产品')
            values = @(@{ field = '金额'; function = 'average' })
        },
        @{
            op = 'insertPivotTable'; id = 'sheet:Sheet1'
            source = @{ sheet = 'Sheet1'; ref = 'A1:C4' }
            location = 'E56'; rows = @('产品')
            values = @(@{ field = '金额'; showAs = 'percent' })
        }
    )
} 'xlsx-xpt6-apply' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $out.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'E1:H59' }
} 'xlsx-xpt6-inspect'

function Get-CellValue($id) {
    $hit = @($tree.sheets[0].cells | Where-Object { $_.id -eq $id } | Select-Object -First 1)
    if ($hit.Count -eq 0) { return $null }
    return $hit[0].value
}

$libOk = (
    ((Get-CellValue 'cell:Sheet1!E4') -eq '甲 汇总') -and
    ([double](Get-CellValue 'cell:Sheet1!G7') -eq 22) -and
    ((Get-CellValue 'cell:Sheet1!G19') -eq $null) -and
    ((Get-CellValue 'cell:Sheet1!F30') -eq '(全部)') -and
    ([double](Get-CellValue 'cell:Sheet1!F43') -eq 3) -and
    ([math]::Abs([double](Get-CellValue 'cell:Sheet1!F49') - 7.5) -lt 1e-9) -and
    ([math]::Abs([double](Get-CellValue 'cell:Sheet1!F59') - 1) -lt 1e-9)
)

$excel = Ensure-MsExcel $out
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $out
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item(1)

function Same-Text($addr, $expect) {
    $got = $ws.Range($addr).Text
    return ([string]$got) -eq $expect
}
function Same-Num($addr, $expect) {
    $got = $ws.Range($addr).Value2
    if ($null -eq $got) { return $false }
    return [math]::Abs([double]$got - [double]$expect) -lt 1e-6
}
function Is-Blank($addr) {
    $got = $ws.Range($addr).Value2
    return ($null -eq $got) -or ([string]$got -eq '')
}

$xlOk = (
    (Same-Text 'E4' '甲 汇总') -and (Same-Num 'G7' 22) -and (Is-Blank 'E3') -and
    (Same-Text 'F17' '东') -and (Same-Num 'H18' 15) -and (Is-Blank 'G19') -and (Same-Num 'F20' 17) -and
    (Same-Text 'F30' '(全部)') -and (Same-Num 'F35' 22) -and
    (Same-Num 'F41' 2) -and (Same-Num 'F43' 3) -and
    (Same-Num 'F49' 7.5) -and (Same-Num 'F51' (22 / 3)) -and
    (Same-Num 'F57' (15 / 22)) -and (Same-Num 'F59' 1)
)
Close-AllWorkbooks $excel

$ok = $libOk -and $xlOk -and ($cap -notmatch 'Repair')
Add-Check 'xlsx-Xpt6' $ok "lib=$libOk xl=$xlOk caption=$cap"
