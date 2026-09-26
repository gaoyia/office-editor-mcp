# xlsx create：docs/checklist-xlsx.md 的 C1、C3–C7。
# C2 是 WPS 打开，归 WPS 那一轮。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$book = Ensure-XlsxCreate
$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$excel.CalculateFullRebuild()
$s1 = $wb.Worksheets.Item('Sheet1')

$a1 = $s1.Range('A1').Value2
$a2 = [string]$s1.Range('A2').Value2
$a3 = [string]$s1.Range('A3').Text
$b1f = [string]$s1.Range('B1').Formula
$b1v = $s1.Range('B1').Value2
# 要读这本工作簿自己的标题栏，不是 Excel 当前的活动窗口。
$caption = [string]$wb.Windows.Item(1).Caption

Add-Check 'xlsx-C1' ($caption -notmatch 'Repair') $caption
Add-Skip  'xlsx-C2' 'WPS 打开，归 WPS 那一轮'
Add-Check 'xlsx-C3' ($s1.Name -eq 'Sheet1') $s1.Name
Add-Check 'xlsx-C4' ($a1 -eq 1) "A1=$a1"
Add-Check 'xlsx-C5' ($a2 -eq 'hello') "A2=$a2"
Add-Check 'xlsx-C6' ($a3 -match 'TRUE') "A3.text=$a3"
Add-Check 'xlsx-C7' (($b1f -eq '=SUM(A1)') -and ($b1v -eq 1)) "B1 formula=$b1f value=$b1v"

$wb.Close($false)
Release-Com $s1
Release-Com $wb
