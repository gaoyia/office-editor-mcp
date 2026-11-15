# 纯色组合可以绕中心转 90 度的倍数，也可以翻转。
# 金样是 Excel 16.0 编组、旋转、翻转后的 CopyPicture。红区回绘制引擎修，不要加大 MaxDelta。

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

$src = Get-OutPath 'xlsx-turn-group-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
$cells = @()
foreach ($col in @('A', 'B', 'C')) {
    foreach ($row in 1..8) { $cells += @{ ref = "$col$row"; value = '' } }
}
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = $cells }) }
} 'xlsx-turn-group-create' | Out-Null

$book = Get-OutPath 'xlsx-turn-group.xlsx'
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
    $row = 30
    $g90 = $ws.Shapes.Range([string[]]@(
        (Add-Box (8 * $pt) (16 * $pt) (40 * $pt) (8 * $pt)),
        (Add-Box (36 * $pt) (8 * $pt) (8 * $pt) (12 * $pt))
    )).Group()
    $g90.Rotation = 90
    $g270 = $ws.Shapes.Range([string[]]@(
        (Add-Box (8 * $pt) ($row * 2 + 16 * $pt) (40 * $pt) (8 * $pt)),
        (Add-Box (36 * $pt) ($row * 2 + 8 * $pt) (8 * $pt) (12 * $pt))
    )).Group()
    $g270.Rotation = 270
    $gflip = $ws.Shapes.Range([string[]]@(
        (Add-Box (8 * $pt) ($row * 4 + 16 * $pt) (40 * $pt) (8 * $pt)),
        (Add-Box (36 * $pt) ($row * 4 + 8 * $pt) (8 * $pt) (12 * $pt))
    )).Group()
    $gflip.Rotation = 90
    $gflip.Flip(0)
    $wb.Save()
    Release-Com $g90
    Release-Com $g270
    Release-Com $gflip
    Release-Com $ws
}
finally { $wb.Close($true); Release-Com $wb }
Stop-Excel

$png = Get-OutPath 'xlsx-turn-group-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSheetImage'
    path = $book.Replace('\', '/'); format = 'xlsx'
    id = 'sheet:Sheet1'
    output = $png.Replace('\', '/')
} 'xlsx-turn-group-export' | Out-Null

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$gold = Get-OutPath 'xlsx-turn-group-gold.png'
try {
    $null = $wb.Activate()
    $ws = $wb.Worksheets.Item(1)
    $null = $ws.Activate()
    $excel.ActiveWindow.Zoom = 100
    $excel.ActiveWindow.DisplayGridlines = $false
    $ws.Range('A1:C8').CopyPicture(1, 2)
    Save-Clipboard $gold
    Release-Com $ws
}
finally { $wb.Close($false); Release-Com $wb }
Stop-Excel

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$out = Get-OutPath 'xlsx-turn-group-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $out -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "xlsx-turn-group-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'xlsx-turn-group-px' $ok $text
