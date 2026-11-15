# 只有分号的数字格式不画字。四段时文字和布尔也不画。
# 金样是 Excel 16.0 的 CopyPicture。红区回绘制引擎修，不要加大 MaxDelta。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'office-com.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

function Save-Clipboard([string]$path) {
    Start-Sleep -Milliseconds 200
    $img = [System.Windows.Forms.Clipboard]::GetImage()
    if ($null -eq $img) { throw '剪贴板没有工作表图' }
    try { $img.Save($path, [System.Drawing.Imaging.ImageFormat]::Png) }
    finally { $img.Dispose() }
}

function Stop-Excel {
    Close-OfficeApps -Quit
    if ($global:OoxpExcelPid) {
        Stop-Process -Id $global:OoxpExcelPid -Force -ErrorAction SilentlyContinue
        $global:OoxpExcelPid = $null
        $global:OoxpExcel = $null
    }
}

$src = Get-OutPath 'xlsx-hidden-format-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'xlsx-hidden-format-create' | Out-Null

$book = Get-OutPath 'xlsx-hidden-format.xlsx'
if (Test-Path $book) { Remove-Item -Force $book }
Copy-Item $src $book -Force

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
try {
    $ws = $wb.Worksheets.Item(1)
    $ws.Rows.Item(1).RowHeight = 30
    $excel.ActiveWindow.DisplayGridlines = $false
    function Set-Num([int]$col, [double]$value, [string]$fmt, [int]$fill) {
        $cell = $ws.Cells.Item(1, $col)
        $cell.Value2 = $value
        $cell.NumberFormat = $fmt
        $cell.Interior.Color = $fill
        Release-Com $cell
    }
    function Set-Formula([int]$col, [string]$formula, [string]$fmt, [int]$fill) {
        $cell = $ws.Cells.Item(1, $col)
        $cell.Formula = $formula
        $cell.NumberFormat = $fmt
        $cell.Interior.Color = $fill
        Release-Com $cell
    }
    Set-Num 1 123 ';;;' 0x5544EE
    Set-Num 2 -5 ';;;' 0x332211
    Set-Num 3 0 ';;;' 0x00AA00
    Set-Formula 4 '="字"' ';;;' 0x0000AA
    Set-Formula 5 '=TRUE()' ';;;' 0xAA00AA
    Set-Num 6 123 ';;' 0xAAAA00
    Set-Num 7 123 ';' 0x888888
    $wb.Save()
    Release-Com $ws
}
finally { $wb.Close($true); Release-Com $wb }
Stop-Excel

$png = Get-OutPath 'xlsx-hidden-format-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSheetImage'
    path = $book.Replace('\', '/'); format = 'xlsx'
    id = 'sheet:Sheet1'
    output = $png.Replace('\', '/')
} 'xlsx-hidden-format-export' | Out-Null

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$gold = Get-OutPath 'xlsx-hidden-format-gold.png'
try {
    $null = $wb.Activate()
    $ws = $wb.Worksheets.Item(1)
    $null = $ws.Activate()
    $excel.ActiveWindow.Zoom = 100
    $excel.ActiveWindow.DisplayGridlines = $false
    $ws.Range('A1:G1').CopyPicture(1, 2)
    Save-Clipboard $gold
    Release-Com $ws
}
finally { $wb.Close($false); Release-Com $wb }
Stop-Excel

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$out = Get-OutPath 'xlsx-hidden-format-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $out -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "xlsx-hidden-format-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'xlsx-hidden-format-px' $ok $text
