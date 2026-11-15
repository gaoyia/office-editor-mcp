# 纯色组合可以按整数比例缩放。金样是 Excel 16.0 编组并缩放后的 CopyPicture。
# 红区回绘制引擎修，不要加大 MaxDelta。

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

$src = Get-OutPath 'xlsx-scale-group-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
$cells = 1..8 | ForEach-Object { @{ ref = "A$_"; value = '' } }
$cells += @{ ref = 'B1'; value = '' }, @{ ref = 'C1'; value = '' }, @{ ref = 'D1'; value = '' }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @($cells) }) }
} 'xlsx-scale-group-create' | Out-Null

$book = Get-OutPath 'xlsx-scale-group.xlsx'
if (Test-Path $book) { Remove-Item -Force $book }
Copy-Item $src $book -Force

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
try {
    $ws = $wb.Worksheets.Item(1)
    1..8 | ForEach-Object { $ws.Rows.Item($_).RowHeight = 30 }
    $excel.ActiveWindow.DisplayGridlines = $false
    $pt = 0.75
    function Add-Box([double]$left, [double]$top, [double]$width, [double]$height) {
        $sh = $ws.Shapes.AddShape(1, $left, $top, $width, $height)
        $sh.Fill.ForeColor.RGB = 0x5544EE
        $sh.Line.Visible = 0
        $name = [string]$sh.Name
        Release-Com $sh
        return $name
    }
    function Group-Scale($names, [double]$sx, [double]$sy) {
        $g = $ws.Shapes.Range([string[]]$names).Group()
        if ($sx -ne 1) { $g.ScaleWidth($sx, $false, 0) }
        if ($sy -ne 1) { $g.ScaleHeight($sy, $false, 0) }
        Release-Com $g
    }
    $row = 30
    Group-Scale @(
        (Add-Box (8 * $pt) (8 * $pt) (16 * $pt) (12 * $pt)),
        (Add-Box (32 * $pt) (8 * $pt) (10 * $pt) (12 * $pt))
    ) 2 1
    Group-Scale @(
        (Add-Box (8 * $pt) ($row + 8 * $pt) (16 * $pt) (10 * $pt)),
        (Add-Box (28 * $pt) ($row + 8 * $pt) (12 * $pt) (10 * $pt))
    ) 1.5 1
    Group-Scale @(
        (Add-Box (8 * $pt) (2 * $row + 8 * $pt) (14 * $pt) (6 * $pt)),
        (Add-Box (8 * $pt) (2 * $row + 16 * $pt) (14 * $pt) (6 * $pt))
    ) 1 2
    $names = @(
        (Add-Box 6.2 ($row * 4 + 5.3) 11.7 7.4),
        (Add-Box 20.8 ($row * 4 + 5.3) 8.1 7.4)
    )
    $odd = $ws.Shapes.Range([string[]]$names).Group()
    $odd.Width = 47.3
    $odd.Height = 22.6
    $wb.Save()
    Release-Com $odd
    Release-Com $ws
}
finally { $wb.Close($true); Release-Com $wb }
Stop-Excel

$png = Get-OutPath 'xlsx-scale-group-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSheetImage'
    path = $book.Replace('\', '/'); format = 'xlsx'
    id = 'sheet:Sheet1'
    output = $png.Replace('\', '/')
} 'xlsx-scale-group-export' | Out-Null

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$gold = Get-OutPath 'xlsx-scale-group-gold.png'
try {
    $null = $wb.Activate()
    $ws = $wb.Worksheets.Item(1)
    $null = $ws.Activate()
    $excel.ActiveWindow.Zoom = 100
    $excel.ActiveWindow.DisplayGridlines = $false
    $ws.Range('A1:D8').CopyPicture(1, 2)
    Save-Clipboard $gold
    Release-Com $ws
}
finally { $wb.Close($false); Release-Com $wb }
Stop-Excel

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$out = Get-OutPath 'xlsx-scale-group-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $out -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "xlsx-scale-group-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'xlsx-scale-group-px' $ok $text
