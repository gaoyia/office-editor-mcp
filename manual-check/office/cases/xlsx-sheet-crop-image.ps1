# xlsx 工作表上整像素、不缩放的 PNG 裁剪。金样是 Excel 16.0 的 CopyPicture，整张剪贴板，不裁边。
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

function Add-SrcRects([string]$text, [string[]]$rects) {
    $rest = 0
    foreach ($rect in $rects) {
        $at = $text.IndexOf('<a:blip ', $rest)
        if ($at -lt 0) { throw '找不到 blip' }
        $end = $text.IndexOf('/>', $at)
        if ($end -lt 0) { throw 'blip 不是空元素' }
        $text = $text.Substring(0, $end + 2) + $rect + $text.Substring($end + 2)
        $rest = $end + 2 + $rect.Length
    }
    return $text
}

function Save-Clipboard([string]$path) {
    Start-Sleep -Milliseconds 200
    $img = [System.Windows.Forms.Clipboard]::GetImage()
    if ($null -eq $img) { throw '剪贴板没有工作表图' }
    try { $img.Save($path, [System.Drawing.Imaging.ImageFormat]::Png) }
    finally { $img.Dispose() }
}

$dots = New-Object System.Drawing.Bitmap 4, 4
$colors = @(
    @(0xFF, 0x00, 0x00), @(0x00, 0xFF, 0x00), @(0x00, 0x00, 0xFF), @(0xFF, 0xFF, 0x00),
    @(0xFF, 0x00, 0xFF), @(0x00, 0xFF, 0xFF), @(0x11, 0x22, 0x33), @(0x44, 0x55, 0x66),
    @(0x77, 0x88, 0x99), @(0xAA, 0xBB, 0xCC), @(0xDD, 0xEE, 0xFF), @(0x01, 0x02, 0x03),
    @(0x04, 0x05, 0x06), @(0x07, 0x08, 0x09), @(0x0A, 0x0B, 0x0C), @(0x0D, 0x0E, 0x0F)
)
for ($i = 0; $i -lt 16; $i++) {
    $c = $colors[$i]
    $dots.SetPixel($i % 4, [Math]::Floor($i / 4), [System.Drawing.Color]::FromArgb(255, $c[0], $c[1], $c[2]))
}
$strip = New-Object System.Drawing.Bitmap 3, 1
$strip.SetPixel(0, 0, [System.Drawing.Color]::FromArgb(255, 0x10, 0x20, 0x30))
$strip.SetPixel(1, 0, [System.Drawing.Color]::FromArgb(255, 0x40, 0x50, 0x60))
$strip.SetPixel(2, 0, [System.Drawing.Color]::FromArgb(255, 0x70, 0x80, 0x90))
$dotsPath = Get-OutPath 'xlsx-sheet-crop-dots.png'
$stripPath = Get-OutPath 'xlsx-sheet-crop-strip.png'
Save-Png $dots $dotsPath
Save-Png $strip $stripPath
$dots.Dispose()
$strip.Dispose()

$src = Get-OutPath 'xlsx-sheet-crop-image-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = '' }) }) }
} 'xlsx-sheet-crop-create' | Out-Null

$book = Get-OutPath 'xlsx-sheet-crop-image.xlsx'
if (Test-Path $book) { Remove-Item -Force $book }
$px = 9525
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/'); output = $book.Replace('\', '/')
    ops = @(
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 1; height = 30 }
        @{ op = 'setCellStyle'; id = 'cell:Sheet1!A1'; font = @{ color = @{ rgb = 'FFFFFFFF' } } }
        @{ op = 'insertPicture'; id = 'sheet:Sheet1'; file = $dotsPath.Replace('\', '/'); from = 'A1'; fromOffX = $px; fromOffY = $px; cx = 3 * $px; cy = 4 * $px }
        @{ op = 'insertPicture'; id = 'sheet:Sheet1'; file = $dotsPath.Replace('\', '/'); from = 'A1'; fromOffX = 20 * $px; fromOffY = $px; cx = 2 * $px; cy = 2 * $px }
        @{ op = 'insertPicture'; id = 'sheet:Sheet1'; file = $stripPath.Replace('\', '/'); from = 'A1'; fromOffX = 40 * $px; fromOffY = $px; cx = 2 * $px; cy = $px }
    )
} 'xlsx-sheet-crop-style' | Out-Null

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
Set-PartText $book 'xl/drawings/drawing1.xml' {
    param($text)
    Add-SrcRects $text @(
        '<a:srcRect l="25000"/>',
        '<a:srcRect l="25000" t="25000" r="25000" b="25000"/>',
        '<a:srcRect l="33333"/>'
    )
}

$png = Get-OutPath 'xlsx-sheet-crop-image-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSheetImage'
    path = $book.Replace('\', '/'); format = 'xlsx'
    id = 'sheet:Sheet1'
    output = $png.Replace('\', '/')
} 'xlsx-sheet-crop-export' | Out-Null

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$gold = Get-OutPath 'xlsx-sheet-crop-image-gold.png'
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
$out = Get-OutPath 'xlsx-sheet-crop-image-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $out -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "xlsx-sheet-crop-image-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'xlsx-sheet-crop-image-px' $ok $text
