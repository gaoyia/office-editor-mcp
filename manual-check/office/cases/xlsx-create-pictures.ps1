# xlsx create 带图：docs/checklist-xlsx.md 的 C8–C10。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$path = Ensure-XlsxCreatePictures
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-create-pictures-inspect'

$sheet = @($tree.sheets) | Where-Object { $_.name -eq 'Sheet1' } | Select-Object -First 1
$pics = @($sheet.pictures)
$one = $pics | Where-Object { $_.anchor -eq 'oneCell' } | Select-Object -First 1
$two = $pics | Where-Object { $_.anchor -eq 'twoCell' } | Select-Object -First 1
Add-Check 'xlsx-C8' (($null -ne $one) -and ($one.id -eq 'sheet:Sheet1/pic:1') -and ($one.from -eq 'B2')) "id=$($one.id) from=$($one.from) n=$($pics.Count)"
Add-Check 'xlsx-C9' (($null -ne $two) -and ($two.from -eq 'D2') -and ($two.to -eq 'F8')) "id=$($two.id) from=$($two.from) to=$($two.to)"

$excel = Ensure-MsExcel $path
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $path
$cap = [string]$wb.Windows.Item(1).Caption
$s1 = $wb.Worksheets.Item('Sheet1')
$shapes = $s1.Shapes
$n = [int]$shapes.Count
Release-Com $shapes
Release-Com $s1
Close-AllWorkbooks $excel
Add-Check 'xlsx-C10' (($cap -notmatch 'Repair') -and ($n -ge 2)) "caption=$cap shapes=$n"
