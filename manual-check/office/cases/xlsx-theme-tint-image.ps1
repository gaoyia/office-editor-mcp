# 格子上的主题色 tint 按 Excel 的 240 级 HLS 取亮度。
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

$src = Get-OutPath 'xlsx-theme-tint-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
$cells = @('A1', 'B1', 'C1', 'D1', 'E1') | ForEach-Object { @{ ref = $_; value = '' } }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = $cells }) }
} 'xlsx-theme-tint-create' | Out-Null

$book = Get-OutPath 'xlsx-theme-tint.xlsx'
if (Test-Path $book) { Remove-Item -Force $book }
Copy-Item $src $book -Force

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
try {
    $colors = $wb.Theme.ThemeColorScheme
    # msoThemeAccent1 = 5。RGB 是 BGR，EE4455 写成 5544EE。
    $colors.Colors(5).RGB = 0x5544EE
    $ws = $wb.Worksheets.Item(1)
    $ws.Rows.Item(1).RowHeight = 30
    $excel.ActiveWindow.DisplayGridlines = $false
    $ws.Range('A1').Interior.ThemeColor = 5
    $ws.Range('A1').Interior.TintAndShade = 0
    $ws.Range('B1').Interior.ThemeColor = 5
    $ws.Range('B1').Interior.TintAndShade = 0.4
    $ws.Range('C1').Interior.ThemeColor = 5
    $ws.Range('C1').Interior.TintAndShade = -0.25
    $ws.Range('D1').Interior.ThemeColor = 1
    $ws.Range('D1').Interior.TintAndShade = 0
    $ws.Range('E1').Interior.ThemeColor = 2
    $ws.Range('E1').Interior.TintAndShade = -0.15
    $wb.Save()
    Release-Com $colors
    Release-Com $ws
}
finally { $wb.Close($true); Release-Com $wb }
Stop-Excel

$png = Get-OutPath 'xlsx-theme-tint-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSheetImage'
    path = $book.Replace('\', '/'); format = 'xlsx'
    id = 'sheet:Sheet1'
    output = $png.Replace('\', '/')
} 'xlsx-theme-tint-export' | Out-Null

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$gold = Get-OutPath 'xlsx-theme-tint-gold.png'
try {
    $null = $wb.Activate()
    $ws = $wb.Worksheets.Item(1)
    $null = $ws.Activate()
    $excel.ActiveWindow.Zoom = 100
    $excel.ActiveWindow.DisplayGridlines = $false
    $ws.Range('A1:E1').CopyPicture(1, 2)
    Save-Clipboard $gold
    Release-Com $ws
}
finally { $wb.Close($false); Release-Com $wb }
Stop-Excel

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$out = Get-OutPath 'xlsx-theme-tint-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $out -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "xlsx-theme-tint-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'xlsx-theme-tint-px' $ok $text
