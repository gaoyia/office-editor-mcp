# xlsx create 页眉页脚：docs/checklist-xlsx.md 的 C14–C16。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$path = Ensure-XlsxCreateHeaderFooter
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'xlsx'
    scope = @{ headerFooter = $true; pictures = $true }
} 'xlsx-create-headerfooter-inspect'

$sheet = @($tree.sheets) | Where-Object { $_.name -eq 'Sheet1' } | Select-Object -First 1
$slots = @($sheet.headerFooters)
$oddH = $slots | Where-Object { $_.kind -eq 'odd' -and $_.side -eq 'header' } | Select-Object -First 1
$textOk = ($sheet.hasHeaderFooter -eq $true) -and ($null -ne $oddH) -and ([string]$oddH.text -match 'C14页眉')
Add-Check 'xlsx-C14' $textOk "has=$($sheet.hasHeaderFooter) text=$($oddH.text)"

$excel = Ensure-MsExcel $path
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $path
$cap = [string]$wb.Windows.Item(1).Caption
$s1 = $wb.Worksheets.Item('Sheet1')
$header = [string]$s1.PageSetup.CenterHeader
$footer = [string]$s1.PageSetup.CenterFooter
Release-Com $s1
Close-AllWorkbooks $excel
Add-Check 'xlsx-C15' (($cap -notmatch 'Repair') -and ($header -match 'C14页眉') -and ($footer -match 'C14页脚')) "caption=$cap header=$header footer=$footer"

$hfPics = @($oddH.pictures)
$pic = $hfPics | Select-Object -First 1
Add-Check 'xlsx-C16' (($cap -notmatch 'Repair') -and ($null -ne $pic) -and ($pic.id -eq 'sheet:Sheet1/hf:odd:header/pic:2')) "caption=$cap pic=$($pic.id) n=$($hfPics.Count)"
