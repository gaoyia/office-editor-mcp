# 组合里的纯色矩形翻转后，外接框不变。和 90 度叠在一起也一样。
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

$src = Get-OutPath 'xlsx-child-flip-src.xlsx'
$book = Get-OutPath 'xlsx-child-flip.xlsx'
foreach ($p in @($src, $book)) { if (Test-Path $p) { Remove-Item -Force $p } }
$cells = @()
foreach ($col in @('A', 'B', 'C')) {
    foreach ($row in 1..6) { $cells += @{ ref = "$col$row"; value = '' } }
}
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = $cells }) }
} 'xlsx-child-flip-create' | Out-Null
Copy-Item $src $book -Force

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
try {
    $ws = $wb.Worksheets.Item(1)
    1..6 | ForEach-Object { $ws.Rows.Item($_).RowHeight = 30 }
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
    $flipped = $ws.Shapes.Range([string[]]@(
        (Add-Box (16 * $pt) (16 * $pt) (48 * $pt) (16 * $pt)),
        (Add-Box (80 * $pt) (16 * $pt) (24 * $pt) (16 * $pt))
    )).Group()
    $items = $flipped.GroupItems
    $child = $items.Item(1)
    $child.Flip(0)
    $side = $items.Item(2)
    $side.Flip(1)
    $turned = $ws.Shapes.Range([string[]]@(
        (Add-Box (16 * $pt) (120 * $pt) (48 * $pt) (16 * $pt)),
        (Add-Box (80 * $pt) (120 * $pt) (24 * $pt) (16 * $pt))
    )).Group()
    $items2 = $turned.GroupItems
    $child2 = $items2.Item(1)
    $child2.Rotation = 90
    $child2.Flip(0)
    $wb.Save()
    Release-Com $child
    Release-Com $side
    Release-Com $items
    Release-Com $child2
    Release-Com $items2
    Release-Com $flipped
    Release-Com $turned
    Release-Com $ws
}
finally { $wb.Close($true); Release-Com $wb }
Stop-Excel

$png = Get-OutPath 'xlsx-child-flip-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSheetImage'
    path = $book.Replace('\', '/'); format = 'xlsx'
    id = 'sheet:Sheet1'
    output = $png.Replace('\', '/')
} 'xlsx-child-flip-export' | Out-Null

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$gold = Get-OutPath 'xlsx-child-flip-gold.png'
try {
    $null = $wb.Activate()
    $ws = $wb.Worksheets.Item(1)
    $null = $ws.Activate()
    $excel.ActiveWindow.Zoom = 100
    $excel.ActiveWindow.DisplayGridlines = $false
    $ws.Range('A1:C6').CopyPicture(1, 2)
    Save-Clipboard $gold
    Release-Com $ws
}
finally { $wb.Close($false); Release-Com $wb }
Stop-Excel

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$out = Get-OutPath 'xlsx-child-flip-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $out -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "xlsx-child-flip-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'xlsx-child-flip-px' $ok $text
