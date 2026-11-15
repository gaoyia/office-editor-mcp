# 工作表上的纯色矩形可以放在两层组合里，外层还可以转 90 度。
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

$src = Get-OutPath 'xlsx-nested-src.xlsx'
$book = Get-OutPath 'xlsx-nested.xlsx'
foreach ($p in @($src, $book)) { if (Test-Path $p) { Remove-Item -Force $p } }
$cells = @()
foreach ($col in @('A', 'B', 'C')) {
    foreach ($row in 1..8) { $cells += @{ ref = "$col$row"; value = '' } }
}
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = $cells }) }
} 'xlsx-nested-create' | Out-Null
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
    function Add-Nest([double]$y, [bool]$turn) {
        $inner = $ws.Shapes.Range([string[]]@(
            (Add-Box (16 * $pt) ($y * $pt) (48 * $pt) (16 * $pt)),
            (Add-Box (80 * $pt) ($y * $pt) (24 * $pt) (16 * $pt))
        )).Group()
        $innerName = [string]$inner.Name
        Release-Com $inner
        $extra = Add-Box (16 * $pt) (($y + 32) * $pt) (32 * $pt) (16 * $pt)
        $outer = $ws.Shapes.Range([string[]]@($innerName, $extra)).Group()
        if ($turn) { $outer.Rotation = 90 }
        Release-Com $outer
    }
    Add-Nest 16 $false
    Add-Nest 160 $true
    $wb.Save()
    Release-Com $ws
}
finally { $wb.Close($true); Release-Com $wb }
Stop-Excel

$png = Get-OutPath 'xlsx-nested-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSheetImage'
    path = $book.Replace('\', '/'); format = 'xlsx'
    id = 'sheet:Sheet1'
    output = $png.Replace('\', '/')
} 'xlsx-nested-export' | Out-Null

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$gold = Get-OutPath 'xlsx-nested-gold.png'
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
$out = Get-OutPath 'xlsx-nested-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $out -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "xlsx-nested-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'xlsx-nested-px' $ok $text
