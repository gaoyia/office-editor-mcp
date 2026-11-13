# xlsx 条件格式公式命中：docs/checklist-xlsx.md 的 Xn7。
# expression 与非常量 cellIs 的 hits。Excel 只用来看填充。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'xlsx-cf-formula-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = 1 }, @{ ref = 'A2'; value = -1 }, @{ ref = 'A3'; value = 5 },
                @{ ref = 'B1'; value = -5 }, @{ ref = 'B2'; value = 100 }, @{ ref = 'B3'; value = 0 },
                @{ ref = 'C1'; value = 1 }, @{ ref = 'C2'; value = -1 }, @{ ref = 'C3'; value = 5 }
            )
        })
    }
} 'xlsx-xn7-create' | Out-Null

$out = Get-OutPath 'xlsx-cf-formula.xlsx'
if (Test-Path $out) { Remove-Item -Force $out }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(
        @{
            op = 'insertConditionalFormat'; id = 'sheet:Sheet1'
            sqref = 'A1:A3'; type = 'expression'; formulas = @('A1>1')
            dxf = @{ fill = @{ pattern = 'solid'; fg = @{ rgb = 'FFC6EFCE' } } }
        },
        @{
            op = 'insertConditionalFormat'; id = 'sheet:Sheet1'
            sqref = 'C1:C3'; type = 'cellIs'; operator = 'greaterThan'; formulas = @('B1')
            dxf = @{ fill = @{ pattern = 'solid'; fg = @{ rgb = 'FFFFC7CE' } } }
        }
    )
} 'xlsx-xn7-cf' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $out.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:C3' }
} 'xlsx-xn7-inspect'
$expr = @($tree.sheets[0].conditionalFormats[0].hits) -join ','
$cellis = @($tree.sheets[0].conditionalFormats[1].hits) -join ','
$libOk = ($expr -eq 'A3') -and ($cellis -eq 'C1,C3')

$excel = Ensure-MsExcel $out
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $out
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item(1)
function Color($addr) { [int]$ws.Range($addr).DisplayFormat.Interior.Color }
$a1 = Color 'A1'; $a3 = Color 'A3'
$c1 = Color 'C1'; $c2 = Color 'C2'; $c3 = Color 'C3'
Close-AllWorkbooks $excel

$xlOk = ($a3 -ne $a1) -and ($c1 -ne $c2) -and ($c3 -ne $c2) -and ($c1 -eq $c3)
$ok = $libOk -and $xlOk -and ($cap -notmatch 'Repair')
Add-Check 'xlsx-Xn7' $ok "expr=$expr cellis=$cellis caption=$cap a1=$a1 a3=$a3 c1=$c1 c2=$c2 c3=$c3"
