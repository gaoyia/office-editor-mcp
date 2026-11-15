# xlsx 工作表上不缩放的 PNG。金样是 Excel 16.0 的 CopyPicture，整张剪贴板，不裁边。
# 红区回绘制引擎修，不要加大 MaxDelta。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'office-com.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Save-Png([System.Drawing.Bitmap]$bmp, [string]$path) {
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
}

function Set-PartText([string]$path, [string]$entryName, [scriptblock]$edit) {
    $zip = [System.IO.Compression.ZipFile]::Open($path, [System.IO.Compression.ZipArchiveMode]::Update)
    try {
        $entry = $zip.GetEntry($entryName)
        $reader = New-Object System.IO.StreamReader($entry.Open())
        try { $text = $reader.ReadToEnd() } finally { $reader.Dispose() }
        $text = & $edit $text
        $entry.Delete()
        $created = $zip.CreateEntry($entryName)
        $writer = New-Object System.IO.StreamWriter($created.Open())
        try { $writer.Write($text) } finally { $writer.Dispose() }
    }
    finally { $zip.Dispose() }
}

function Save-Clipboard([string]$path) {
    Start-Sleep -Milliseconds 200
    $img = [System.Windows.Forms.Clipboard]::GetImage()
    if ($null -eq $img) { throw '剪贴板没有工作表图' }
    try { $img.Save($path, [System.Drawing.Imaging.ImageFormat]::Png) }
    finally { $img.Dispose() }
}

$checker = New-Object System.Drawing.Bitmap 8, 8
$solid = New-Object System.Drawing.Bitmap 10, 8
for ($y = 0; $y -lt 8; $y++) {
    for ($x = 0; $x -lt 8; $x++) {
        $on = (($x + $y) % 2) -eq 0
        $c = if ($on) { [System.Drawing.Color]::Black } else { [System.Drawing.Color]::White }
        $checker.SetPixel($x, $y, $c)
    }
    for ($x = 0; $x -lt 10; $x++) {
        $solid.SetPixel($x, $y, [System.Drawing.Color]::FromArgb(255, 0xEE, 0x44, 0x55))
    }
}
$checkerPath = Get-OutPath 'xlsx-sheet-picture-checker.png'
$solidPath = Get-OutPath 'xlsx-sheet-picture-solid.png'
Save-Png $checker $checkerPath
Save-Png $solid $solidPath
$checker.Dispose()
$solid.Dispose()

$src = Get-OutPath 'xlsx-sheet-picture-image-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = '' }) }) }
} 'xlsx-sheet-picture-create' | Out-Null

$book = Get-OutPath 'xlsx-sheet-picture-image.xlsx'
if (Test-Path $book) { Remove-Item -Force $book }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/'); output = $book.Replace('\', '/')
    ops = @(
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 1; height = 30 }
        @{ op = 'setCellStyle'; id = 'cell:Sheet1!A1'; font = @{ color = @{ rgb = 'FFFFFFFF' } } }
    )
} 'xlsx-sheet-picture-style' | Out-Null

Set-PartText $book 'xl/worksheets/sheet1.xml' {
    param($text)
    if ($text -notmatch '<dimension [^>]*/>') { throw '找不到 dimension' }
    $views = '<sheetViews><sheetView showGridLines="0" workbookViewId="0"/></sheetViews>'
    $text = [regex]::Replace($text, '(<dimension [^>]*/>)', ('$1' + $views), 1)
    $viewsAt = $text.IndexOf('sheetViews')
    $colsAt = $text.IndexOf('<cols>')
    if ($colsAt -ge 0 -and $viewsAt -gt $colsAt) { throw 'sheetViews 落在 cols 后面，Excel 会要求修复' }
    $text
}

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
try {
    $ws = $wb.Worksheets.Item(1)
    $null = $ws.Activate()
    $excel.ActiveWindow.Zoom = 100
    $null = $ws.Shapes.AddPicture($checkerPath, 0, -1, 2, 2, (8 * 0.75), (8 * 0.75))
    $null = $ws.Shapes.AddPicture($solidPath, 0, -1, 20, 2, (10 * 0.75), (8 * 0.75))
    $wb.Save()
    Release-Com $ws
}
finally {
    $wb.Close($true)
    Release-Com $wb
}

$png = Get-OutPath 'xlsx-sheet-picture-image-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSheetImage'
    path = $book.Replace('\', '/'); format = 'xlsx'
    id = 'sheet:Sheet1'
    output = $png.Replace('\', '/')
} 'xlsx-sheet-picture-export' | Out-Null

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$gold = Get-OutPath 'xlsx-sheet-picture-image-gold.png'
try {
    $null = $wb.Activate()
    $ws = $wb.Worksheets.Item(1)
    $null = $ws.Activate()
    $excel.ActiveWindow.Zoom = 100
    Write-Log ("gridlines=" + $excel.ActiveWindow.DisplayGridlines + " book=" + $wb.Name)
    $ws.Range('A1').CopyPicture(1, 2)
    Save-Clipboard $gold
    Release-Com $ws
}
finally {
    $wb.Close($false)
    Release-Com $wb
}

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$out = Get-OutPath 'xlsx-sheet-picture-image-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $out -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "xlsx-sheet-picture-image-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'xlsx-sheet-picture-image-px' $ok $text
