# xlsx 从右到左、没有文字。金样是 Excel 16.0 的 CopyPicture，整张剪贴板。
# 红区回绘制引擎修，不要加大 MaxDelta。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'office-com.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

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

$src = Get-OutPath 'xlsx-rtl-image-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = '' },
                @{ ref = 'B1'; value = '' },
                @{ ref = 'A2'; value = '' }
            )
        })
    }
} 'xlsx-rtl-image-create' | Out-Null

$book = Get-OutPath 'xlsx-rtl-image.xlsx'
if (Test-Path $book) { Remove-Item -Force $book }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $book.Replace('\', '/')
    ops = @(
        @{ op = 'setCellStyle'; id = 'cell:Sheet1!A1'; fill = @{ pattern = 'solid'; fg = @{ rgb = 'FFEE4455' } } },
        @{ op = 'setCellStyle'; id = 'cell:Sheet1!B1'; fill = @{ pattern = 'solid'; fg = @{ rgb = 'FF00AA66' } } },
        @{ op = 'setCellStyle'; id = 'cell:Sheet1!A2'; fill = @{ pattern = 'solid'; fg = @{ rgb = 'FF112233' } } }
    )
} 'xlsx-rtl-image-style' | Out-Null

Set-PartText $book 'xl/worksheets/sheet1.xml' {
    param($text)
    $text.Replace('<sheetData>', '<sheetViews><sheetView rightToLeft="1" workbookViewId="0"/></sheetViews><sheetData>')
}

$lib = Get-OutPath 'xlsx-rtl-image-lib.png'
if (Test-Path $lib) { Remove-Item -Force $lib }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSheetImage'
    path = $book.Replace('\', '/'); format = 'xlsx'
    id = 'sheet:Sheet1'
    output = $lib.Replace('\', '/')
} 'xlsx-rtl-image-export' | Out-Null

function Save-Clipboard([string]$path) {
    Start-Sleep -Milliseconds 200
    $img = [System.Windows.Forms.Clipboard]::GetImage()
    if ($null -eq $img) { throw '剪贴板没有工作表图' }
    try { $img.Save($path, [System.Drawing.Imaging.ImageFormat]::Png) }
    finally { $img.Dispose() }
}

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$gold1 = Get-OutPath 'xlsx-rtl-image-gold-1.png'
$gold2 = Get-OutPath 'xlsx-rtl-image-gold-2.png'
try {
    $null = $wb.Activate()
    $ws = $wb.Worksheets.Item(1)
    $null = $ws.Activate()
    $excel.ActiveWindow.Zoom = 100
    Write-Log ("rtl=" + $excel.ActiveWindow.DisplayRightToLeft)
    $ws.Range('A1:B2').CopyPicture(1, 2)
    Save-Clipboard $gold1
    $ws.Range('A1:B2').CopyPicture(1, 2)
    Save-Clipboard $gold2
    Release-Com $ws
}
finally {
    $wb.Close($false)
    Release-Com $wb
}

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))

function Add-PixelCheck([string]$name, [string]$left, [string]$right, [string]$outName) {
    $out = Get-OutPath $outName
    $line = & $diffScript -Left $left -Right $right -Out $out -MaxDelta 0
    $text = ($line | Out-String).Trim()
    Write-Log "$name $text"
    $ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
    Add-Check $name $ok $text
}

Add-PixelCheck 'xlsx-rtl-image-stable' $gold1 $gold2 'xlsx-rtl-image-stable-diff.png'
Add-PixelCheck 'xlsx-rtl-image-px' $gold1 $lib 'xlsx-rtl-image-diff.png'
