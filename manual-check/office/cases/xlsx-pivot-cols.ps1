# xlsx 透视多列版式：docs/checklist-xlsx.md 的 Xpt7。
# 库先把合计写进格子。Excel 16.0 打开后应与这些格子一致，且没有修复。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'xlsx-pivot-cols-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = '渠道' },
                @{ ref = 'B1'; value = '产品' },
                @{ ref = 'C1'; value = '地区' },
                @{ ref = 'D1'; value = '金额' },
                @{ ref = 'E1'; value = '数量' },
                @{ ref = 'A2'; value = '线上' }, @{ ref = 'B2'; value = '甲' }, @{ ref = 'C2'; value = '东' }, @{ ref = 'D2'; value = 10 }, @{ ref = 'E2'; value = 1 },
                @{ ref = 'A3'; value = '线上' }, @{ ref = 'B3'; value = '甲' }, @{ ref = 'C3'; value = '西' }, @{ ref = 'D3'; value = 5 }, @{ ref = 'E3'; value = 1 },
                @{ ref = 'A4'; value = '线上' }, @{ ref = 'B4'; value = '乙' }, @{ ref = 'C4'; value = '东' }, @{ ref = 'D4'; value = 7 }, @{ ref = 'E4'; value = 1 },
                @{ ref = 'A5'; value = '线下' }, @{ ref = 'B5'; value = '甲' }, @{ ref = 'C5'; value = '东' }, @{ ref = 'D5'; value = 3 }, @{ ref = 'E5'; value = 2 }
            )
        })
    }
} 'xlsx-xpt7-create' | Out-Null

$out = Get-OutPath 'xlsx-pivot-cols.xlsx'
if (Test-Path $out) { Remove-Item -Force $out }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(
        @{
            op = 'insertPivotTable'; id = 'sheet:Sheet1'
            source = @{ sheet = 'Sheet1'; ref = 'A1:E5' }
            location = 'G1'; rows = @('产品'); cols = @('地区', '渠道'); values = @('金额')
        },
        @{
            op = 'insertPivotTable'; id = 'sheet:Sheet1'
            source = @{ sheet = 'Sheet1'; ref = 'A1:E5' }
            location = 'G30'; rows = @('产品', '地区'); cols = @('渠道'); values = @('金额')
        },
        @{
            op = 'insertPivotTable'; id = 'sheet:Sheet1'
            source = @{ sheet = 'Sheet1'; ref = 'A1:E5' }
            location = 'G60'; rows = @('产品'); cols = @('地区'); values = @('金额', '数量')
        }
    )
} 'xlsx-xpt7-apply' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $out.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'G1:M65' }
} 'xlsx-xpt7-inspect'

function Get-CellValue($id) {
    $hit = @($tree.sheets[0].cells | Where-Object { $_.id -eq $id } | Select-Object -First 1)
    if ($hit.Count -eq 0) { return $null }
    return $hit[0].value
}

$libOk = (
    ((Get-CellValue 'cell:Sheet1!J2') -eq '东 汇总') -and
    ([double](Get-CellValue 'cell:Sheet1!M6') -eq 25) -and
    ((Get-CellValue 'cell:Sheet1!K5') -eq $null) -and
    ((Get-CellValue 'cell:Sheet1!G34') -eq '甲 汇总') -and
    ([double](Get-CellValue 'cell:Sheet1!I34') -eq 15) -and
    ((Get-CellValue 'cell:Sheet1!I60') -eq 'Values') -and
    ((Get-CellValue 'cell:Sheet1!L61') -eq '金额汇总') -and
    ((Get-CellValue 'cell:Sheet1!J64') -eq $null) -and
    ([double](Get-CellValue 'cell:Sheet1!L64') -eq 7)
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
    (Same-Text 'J2' '东 汇总') -and (Same-Num 'M6' 25) -and (Is-Blank 'K5') -and
    (Same-Text 'G34' '甲 汇总') -and (Same-Num 'I34' 15) -and (Same-Num 'K37' 25) -and
    (Same-Text 'I60' 'Values') -and (Same-Text 'L61' '金额汇总') -and
    (Is-Blank 'J64') -and (Same-Num 'L64' 7) -and (Same-Num 'M65' 5)
)
Close-AllWorkbooks $excel

$ok = $libOk -and $xlOk -and ($cap -notmatch 'Repair')
Add-Check 'xlsx-Xpt7' $ok "lib=$libOk xl=$xlOk caption=$cap"
