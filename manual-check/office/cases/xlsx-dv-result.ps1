# xlsx 数据验证结果：docs/checklist-xlsx.md 的 Xv6。
# inspect 展开列表并回答过不过。不把答案写进文件。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'xlsx-dv-result-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = 1 },
                @{ ref = 'B1'; value = 9 }
            )
        })
    }
} 'xlsx-xv6-create' | Out-Null

$out = Get-OutPath 'xlsx-dv-result.xlsx'
if (Test-Path $out) { Remove-Item -Force $out }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(@{
        op = 'insertDataValidation'
        id = 'sheet:Sheet1'
        sqref = 'A1:B1'
        type = 'list'
        formula1 = '1,2,3'
    })
} 'xlsx-xv6-dv' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $out.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:B1' }
} 'xlsx-xv6-inspect'
$dv = $tree.sheets[0].dataValidations[0]
$items = @($dv.items)
$results = @($dv.results)
$a = @($results | Where-Object { $_.ref -eq 'A1' } | Select-Object -First 1)
$b = @($results | Where-Object { $_.ref -eq 'B1' } | Select-Object -First 1)
$treeOk = ($items -join ',') -eq '1,2,3' -and ($a.ok -eq $true) -and ($b.ok -eq $false)

$excel = Ensure-MsExcel $out
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $out
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item(1)
$va = [bool]$ws.Range('A1').Validation.Value
$vb = [bool]$ws.Range('B1').Validation.Value
Close-AllWorkbooks $excel

$ok = $treeOk -and ($cap -notmatch 'Repair') -and $va -and (-not $vb)
Add-Check 'xlsx-Xv6' $ok "items=$($items -join ',') a=$($a.ok) b=$($b.ok) caption=$cap va=$va vb=$vb"
