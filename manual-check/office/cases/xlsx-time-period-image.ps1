# 时间段条件格式按 Excel 存进包里的公式铺纯色。数字格式 ;;; 不画字。
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

$src = Get-OutPath 'xlsx-time-period-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'xlsx-time-period-create' | Out-Null

$book = Get-OutPath 'xlsx-time-period.xlsx'
if (Test-Path $book) { Remove-Item -Force $book }
Copy-Item $src $book -Force

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
try {
    $ws = $wb.Worksheets.Item(1)
    $today = (Get-Date).Date
    $firstThis = Get-Date -Year $today.Year -Month $today.Month -Day 1
    $dow = [int]$today.DayOfWeek
    $sunday = $today.AddDays(-$dow)
    $dates = @(
        $today,
        $today.AddDays(-1),
        $today.AddDays(1),
        $today.AddDays(-6),
        $today.AddDays(-7),
        $today.AddDays(-8),
        $firstThis,
        $firstThis.AddDays(-1),
        $firstThis.AddMonths(1),
        $sunday,
        $sunday.AddDays(1),
        $sunday.AddDays(6),
        $sunday.AddDays(7),
        $sunday.AddDays(-1)
    )
    for ($i = 0; $i -lt $dates.Count; $i++) {
        $serial = ($dates[$i] - [datetime]'1899-12-30').TotalDays
        $ws.Cells.Item($i + 1, 1).Value2 = [double]$serial
    }
    $ws.Range('A1:A14').Copy($ws.Range('B1:J14')) | Out-Null
    $ws.Range('A1:J14').NumberFormat = ';;;'
    $ws.Rows.Item('1:14').RowHeight = 15
    $excel.ActiveWindow.DisplayGridlines = $false
    $missing = [Type]::Missing
    $ops = @(0, 1, 6, 2, 9, 5, 8, 3, 4, 7)
    $colors = @(0x5555EE, 0x55EE55, 0xEE5555, 0x55EEEE, 0xEE55EE, 0xEEEE55, 0x8855EE, 0x55EE88, 0xEE8855, 0x888888)
    for ($c = 0; $c -lt $ops.Count; $c++) {
        $range = $ws.Range($ws.Cells.Item(1, $c + 1), $ws.Cells.Item(14, $c + 1))
        $fc = $range.FormatConditions.Add(11, $missing, $missing, $missing, $missing, $missing, $ops[$c])
        $fc.Interior.Color = $colors[$c]
        Release-Com $fc
        Release-Com $range
    }
    $wb.Save()
    Release-Com $ws
}
finally { $wb.Close($true); Release-Com $wb }
Stop-Excel

$png = Get-OutPath 'xlsx-time-period-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSheetImage'
    path = $book.Replace('\', '/'); format = 'xlsx'
    id = 'sheet:Sheet1'
    output = $png.Replace('\', '/')
} 'xlsx-time-period-export' | Out-Null

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$gold = Get-OutPath 'xlsx-time-period-gold.png'
try {
    $null = $wb.Activate()
    $ws = $wb.Worksheets.Item(1)
    $null = $ws.Activate()
    $excel.ActiveWindow.Zoom = 100
    $excel.ActiveWindow.DisplayGridlines = $false
    $ws.Range('A1:J14').CopyPicture(1, 2)
    Save-Clipboard $gold
    Release-Com $ws
}
finally { $wb.Close($false); Release-Com $wb }
Stop-Excel

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$out = Get-OutPath 'xlsx-time-period-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $out -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "xlsx-time-period-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'xlsx-time-period-px' $ok $text
