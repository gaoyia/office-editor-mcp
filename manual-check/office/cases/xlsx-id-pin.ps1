# 先删第一条数据验证、再改原来第三条；先删第一条条件格式、再改原来第二条。
# Excel 16.0 不报修复，改动落在原来那条上。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$hostPath = Get-OutPath 'xlsx-id-pin-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
        })
    }
} 'xlsx-id-pin-create' | Out-Null

$mid = Get-OutPath 'xlsx-id-pin-mid.xlsx'
if (Test-Path $mid) { Remove-Item -Force $mid }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $mid.Replace('\', '/')
    ops = @(
        @{ op = 'insertDataValidation'; id = 'sheet:Sheet1'; sqref = 'A1'; type = 'list'; formula1 = 'one' }
        @{ op = 'insertDataValidation'; id = 'sheet:Sheet1'; sqref = 'B1'; type = 'list'; formula1 = 'two' }
        @{ op = 'insertDataValidation'; id = 'sheet:Sheet1'; sqref = 'C1'; type = 'list'; formula1 = 'three' }
        @{ op = 'insertConditionalFormat'; id = 'sheet:Sheet1'; sqref = 'A2'; type = 'cellIs'; operator = 'greaterThan'; formulas = @('1') }
        @{ op = 'insertConditionalFormat'; id = 'sheet:Sheet1'; sqref = 'B2'; type = 'cellIs'; operator = 'greaterThan'; formulas = @('2') }
    )
} 'xlsx-id-pin-insert' | Out-Null

$out = Get-OutPath 'xlsx-id-pin.xlsx'
if (Test-Path $out) { Remove-Item -Force $out }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $mid.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(
        @{ op = 'deleteDataValidation'; id = 'sheet:Sheet1/dv:1' }
        @{ op = 'setDataValidation'; id = 'sheet:Sheet1/dv:3'; prompt = '第三条' }
        @{ op = 'deleteConditionalFormat'; id = 'sheet:Sheet1/cf:1' }
        @{ op = 'setConditionalFormat'; id = 'sheet:Sheet1/cf:2'; sqref = 'B2:B9' }
    )
} 'xlsx-id-pin-apply' | Out-Null

$excel = Ensure-MsExcel $out
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $out
$cap = [string]$wb.Windows.Item(1).Caption
$s1 = $wb.Worksheets.Item('Sheet1')

function Read-ListFormula($cell) {
    try {
        if (-not $cell.Validation.Type) { return '' }
        return [string]$cell.Validation.Formula1
    } catch {
        return ''
    }
}

$a = Read-ListFormula $s1.Range('A1')
$b = Read-ListFormula $s1.Range('B1')
$c = Read-ListFormula $s1.Range('C1')
$cMsg = ''
try { $cMsg = [string]$s1.Range('C1').Validation.InputMessage } catch { $cMsg = '' }
$fcA = 0
$fcB = 0
$fcB9 = 0
try { $fcA = [int]$s1.Range('A2').FormatConditions.Count } catch { $fcA = -1 }
try { $fcB = [int]$s1.Range('B2').FormatConditions.Count } catch { $fcB = -1 }
try { $fcB9 = [int]$s1.Range('B9').FormatConditions.Count } catch { $fcB9 = -1 }

$noRepair = $cap -notmatch 'Repair'
$dvOk = ($a -eq '') -and ($b -match 'two') -and ($c -match 'three') -and ($cMsg -eq '第三条')
$cfOk = ($fcA -eq 0) -and ($fcB -ge 1) -and ($fcB9 -ge 1)
Add-Check 'xlsx-id-pin' ($noRepair -and $dvOk -and $cfOk) "repair=$noRepair a=$a b=$b c=$c msg=$cMsg fcA=$fcA fcB=$fcB fcB9=$fcB9"
Close-AllWorkbooks $excel
