# xlsx apply：docs/checklist-xlsx.md 的 A1–A7，以及 O2（未改区域肉眼无差异）。
# 源文件由 Excel 亲手存出，见 lib/fixtures.ps1。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Ensure-XlsxExcelSource
$excel = Ensure-MsExcel $src
# Excel 抓着源文件不放会让 apply 写不出去，先全部关掉。
Close-AllWorkbooks $excel

$applied = Get-OutPath 'xlsx-applied.xlsx'
if (Test-Path $applied) {
    try { Remove-Item $applied -Force }
    catch {
        $applied = Get-OutPath ("xlsx-applied-" + (Get-Date -Format 'HHmmss') + ".xlsx")
        Write-Log "目标被占用，改写到 $applied"
    }
}

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $applied.Replace('\', '/')
    ops = @(
        @{ op = 'setCell'; id = 'cell:Sheet1!A1'; value = 42 }
        @{ op = 'setCell'; id = 'cell:Sheet1!B1'; formula = '=SUM(A1:A10)' }
        @{ op = 'setCell'; id = 'cell:Sheet1!C1'; value = $null }
    )
} 'xlsx-apply' | Out-Null

$wb = Open-ExcelBook $excel $applied
$excel.CalculateFullRebuild()
$s1 = $wb.Worksheets.Item('Sheet1')
$s2 = $wb.Worksheets.Item('Sheet2')
$caption = [string]$wb.Windows.Item(1).Caption

Add-Check 'xlsx-A1' ($caption -notmatch 'Repair') $caption
Add-Check 'xlsx-A2' ($s1.Range('A1').Value2 -eq 42) "A1=$($s1.Range('A1').Value2)"

$bf = [string]$s1.Range('B1').Formula
$bv = $s1.Range('B1').Value2
Add-Check 'xlsx-A3' (($bf -eq '=SUM(A1:A10)') -and ($bv -eq 47)) "B1 $bf value=$bv"

# 清成空值之后，数字格式要还在。
$cEmpty = ($null -eq $s1.Range('C1').Value2) -or ($s1.Range('C1').Value2 -eq '')
$cFmt = [string]$s1.Range('C1').NumberFormat
Add-Check 'xlsx-A4' ($cEmpty -and $cFmt -eq '0.00') "empty=$cEmpty fmt=$cFmt"

$s2ok = ($s2.Range('A1').Value2 -eq 'untouched') -and
        ($s2.Range('B1').Value2 -eq 99) -and
        ($s2.Range('A1').Font.Bold -eq $true)
Add-Check 'xlsx-A5' $s2ok ("A1=$($s2.Range('A1').Value2) B1=$($s2.Range('B1').Value2) bold=$($s2.Range('A1').Font.Bold)")

$e1 = $s1.Range('E1')
$width = $s1.Columns.Item(5).ColumnWidth
$styleOk = ($e1.Value2 -eq 'keep') -and
           ($e1.Font.Bold -eq $true) -and
           ($e1.Font.Color -eq 255) -and
           ([math]::Abs($e1.Interior.Color - 65535) -lt 1) -and
           ([math]::Abs($width - 20) -lt 0.2)
Add-Check 'xlsx-A6' $styleOk ("E1=$($e1.Value2) bold=$($e1.Font.Bold) color=$($e1.Font.Color) fill=$($e1.Interior.Color) width=$width")

# 依赖 A1 的公式要重算，不能留旧缓存。
Add-Check 'xlsx-A7' ($s1.Range('D1').Value2 -eq 84) "D1=$($s1.Range('D1').Value2)"
Add-Check 'xlsx-O2' ($s2ok -and $styleOk) 'Sheet2 与 E1 的样式都还在'

# 工作簿留着不关，方便肉眼再看一遍。
Release-Com $e1
Release-Com $s1
Release-Com $s2
