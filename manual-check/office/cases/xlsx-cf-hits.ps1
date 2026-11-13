# xlsx 条件格式命中：docs/checklist-xlsx.md 的 Xn6。
# inspect 回答命中格。Excel 只用来看填充，库不把颜色写进格子。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'xlsx-cf-hits-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = 2 },
                @{ ref = 'A2'; value = 0 }
            )
        })
    }
} 'xlsx-xn6-create' | Out-Null

$out = Get-OutPath 'xlsx-cf-hits.xlsx'
if (Test-Path $out) { Remove-Item -Force $out }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(@{
        op = 'insertConditionalFormat'
        id = 'sheet:Sheet1'
        sqref = 'A1:A3'
        type = 'cellIs'
        operator = 'greaterThan'
        formulas = @('0')
        dxf = @{ fill = @{ pattern = 'solid'; fg = @{ rgb = 'FFC6EFCE' } } }
    })
} 'xlsx-xn6-cf' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $out.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:A3' }
} 'xlsx-xn6-inspect'
$hits = @($tree.sheets[0].conditionalFormats[0].hits)
$hitOk = ($hits.Count -eq 1) -and ([string]$hits[0] -eq 'A1')

$excel = Ensure-MsExcel $out
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $out
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item(1)
$c1 = [int]$ws.Range('A1').DisplayFormat.Interior.Color
$c2 = [int]$ws.Range('A2').DisplayFormat.Interior.Color
$c3 = [int]$ws.Range('C1').DisplayFormat.Interior.Color
Close-AllWorkbooks $excel

$ok = $hitOk -and ($cap -notmatch 'Repair') -and ($c1 -ne $c2) -and ($c1 -ne $c3)
Add-Check 'xlsx-Xn6' $ok "hits=$($hits -join ',') caption=$cap a1=$c1 a2=$c2 c1=$c3"
