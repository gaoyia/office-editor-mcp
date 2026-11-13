# xlsx 数据验证自定义：docs/checklist-xlsx.md 的 Xv7。
# custom 公式能算出时，inspect 的 ok 与 Excel Validation.Value 一致。不把答案写进文件。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'xlsx-dv-custom-src.xlsx'
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
                @{ ref = 'C1'; value = 1 }, @{ ref = 'C2'; value = -1 }, @{ ref = 'C3'; value = 5 },
                @{ ref = 'F2'; value = 1 },
                @{ ref = 'H1'; value = 1 }, @{ ref = 'K1'; value = 1 },
                @{ ref = 'L1'; value = 1 }, @{ ref = 'M1'; value = 1 }, @{ ref = 'N1'; value = 1 }
            )
        })
    }
} 'xlsx-xv7-create' | Out-Null

$out = Get-OutPath 'xlsx-dv-custom.xlsx'
if (Test-Path $out) { Remove-Item -Force $out }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(
        @{ op = 'insertDataValidation'; id = 'sheet:Sheet1'; sqref = 'A1:A4'; type = 'custom'; formula1 = 'A1>0' },
        @{ op = 'insertDataValidation'; id = 'sheet:Sheet1'; sqref = 'C1:C3'; type = 'custom'; formula1 = 'B1>0' },
        @{ op = 'insertDataValidation'; id = 'sheet:Sheet1'; sqref = 'F1:F2'; type = 'custom'; formula1 = 'F1>0'; allowBlank = $false },
        @{ op = 'insertDataValidation'; id = 'sheet:Sheet1'; sqref = 'G1'; type = 'custom'; formula1 = 'TRUE'; allowBlank = $false },
        @{ op = 'insertDataValidation'; id = 'sheet:Sheet1'; sqref = 'H1'; type = 'custom'; formula1 = '1/0' },
        @{ op = 'insertDataValidation'; id = 'sheet:Sheet1'; sqref = 'K1'; type = 'custom'; formula1 = '"hello"' },
        @{ op = 'insertDataValidation'; id = 'sheet:Sheet1'; sqref = 'L1'; type = 'custom'; formula1 = '1' },
        @{ op = 'insertDataValidation'; id = 'sheet:Sheet1'; sqref = 'M1'; type = 'custom'; formula1 = '0' },
        @{ op = 'insertDataValidation'; id = 'sheet:Sheet1'; sqref = 'N1'; type = 'custom'; formula1 = 'A1>0' }
    )
} 'xlsx-xv7-dv' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $out.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:N1' }
} 'xlsx-xv7-inspect'
$want = @{
    A1 = $true; A2 = $false; A3 = $true; A4 = $true
    C1 = $false; C2 = $true; C3 = $false
    F1 = $false; F2 = $true; G1 = $true; H1 = $false
    K1 = $false; L1 = $true; M1 = $false; N1 = $true
}
$libOk = $true
$libBits = @()
foreach ($rule in @($tree.sheets[0].dataValidations)) {
    foreach ($row in @($rule.results)) {
        if (-not $want.ContainsKey($row.ref)) { continue }
        $pass = [bool]$row.ok
        if ($pass -ne $want[$row.ref]) { $libOk = $false }
        $libBits += "$($row.ref)=$pass"
    }
}

$excel = Ensure-MsExcel $out
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $out
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item(1)
$xlOk = $true
$xlBits = @()
foreach ($addr in @($want.Keys)) {
    $pass = [bool]$ws.Range($addr).Validation.Value
    if ($pass -ne $want[$addr]) { $xlOk = $false }
    $xlBits += "$addr=$pass"
}
Close-AllWorkbooks $excel

$ok = $libOk -and $xlOk -and ($cap -notmatch 'Repair')
Add-Check 'xlsx-Xv7' $ok "caption=$cap lib=$($libBits -join ',') xl=$($xlBits -join ',')"
