# 恒等组合可以跨列、跨行。里面是没有描边的纯色矩形。
# 金样是 Excel 16.0 编组后的 CopyPicture。红区回绘制引擎修，不要加大 MaxDelta。

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

$src = Get-OutPath 'xlsx-cross-group-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(
        @{ ref = 'A1'; value = '' }
        @{ ref = 'B1'; value = '' }
        @{ ref = 'A2'; value = '' }
        @{ ref = 'A3'; value = '' }
    ) }) }
} 'xlsx-cross-group-create' | Out-Null

$book = Get-OutPath 'xlsx-cross-group.xlsx'
if (Test-Path $book) { Remove-Item -Force $book }
Copy-Item $src $book -Force

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
try {
    $ws = $wb.Worksheets.Item(1)
    1..3 | ForEach-Object { $ws.Rows.Item($_).RowHeight = 30 }
    $excel.ActiveWindow.DisplayGridlines = $false
    $pt = 0.75
    $rowPt = 30
    function Add-Box([double]$left, [double]$top, [double]$width, [double]$height) {
        $sh = $ws.Shapes.AddShape(1, $left, $top, $width, $height)
        $sh.Fill.ForeColor.RGB = 0x5544EE
        $sh.Line.Visible = 0
        $name = [string]$sh.Name
        Release-Com $sh
        return $name
    }
    $across = @(
        (Add-Box (8 * $pt) (8 * $pt) (16 * $pt) (12 * $pt)),
        (Add-Box ((64 + 8) * $pt) (8 * $pt) (10 * $pt) (12 * $pt))
    )
    $down = @(
        (Add-Box (8 * $pt) ($rowPt + 8 * $pt) (12 * $pt) (10 * $pt)),
        (Add-Box (8 * $pt) (2 * $rowPt + 8 * $pt) (12 * $pt) (10 * $pt))
    )
    $g1 = $ws.Shapes.Range([string[]]$across).Group()
    $g2 = $ws.Shapes.Range([string[]]$down).Group()
    $wb.Save()
    Release-Com $g1
    Release-Com $g2
    Release-Com $ws
}
finally { $wb.Close($true); Release-Com $wb }
Stop-Excel

$png = Get-OutPath 'xlsx-cross-group-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSheetImage'
    path = $book.Replace('\', '/'); format = 'xlsx'
    id = 'sheet:Sheet1'
    output = $png.Replace('\', '/')
} 'xlsx-cross-group-export' | Out-Null

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$gold = Get-OutPath 'xlsx-cross-group-gold.png'
try {
    $null = $wb.Activate()
    $ws = $wb.Worksheets.Item(1)
    $null = $ws.Activate()
    $excel.ActiveWindow.Zoom = 100
    $excel.ActiveWindow.DisplayGridlines = $false
    $ws.Range('A1:B3').CopyPicture(1, 2)
    Save-Clipboard $gold
    Release-Com $ws
}
finally { $wb.Close($false); Release-Com $wb }
Stop-Excel

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$out = Get-OutPath 'xlsx-cross-group-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $out -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "xlsx-cross-group-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'xlsx-cross-group-px' $ok $text
