# xlsx 公式条件格式的纯色。金样是 Excel 16.0 的 CopyPicture，整张剪贴板，不裁边。
# A2 没有格子，规则仍要铺到这一行。红区回绘制引擎修，不要加大 MaxDelta。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'office-com.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

function Set-PartText([string]$path, [string]$entryName, [scriptblock]$edit) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
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

$src = Get-OutPath 'xlsx-cf-expression-image-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(
        @{ ref = 'A1'; value = '' }
        @{ ref = 'A3'; value = '' }
        @{ ref = 'A4'; value = '' }
    ) }) }
} 'xlsx-cf-expression-create' | Out-Null

$fill = @{ pattern = 'solid'; fg = @{ rgb = 'FFEE4455' } }
$book = Get-OutPath 'xlsx-cf-expression-image.xlsx'
if (Test-Path $book) { Remove-Item -Force $book }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $book.Replace('\', '/')
    ops = @(
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 1; height = 15 }
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 2; height = 15 }
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 3; height = 15 }
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 4; height = 15 }
        @{ op = 'insertConditionalFormat'; id = 'sheet:Sheet1'; sqref = 'A1:A3'; type = 'expression'; formulas = @('TRUE'); dxf = @{ fill = $fill } }
        @{ op = 'insertConditionalFormat'; id = 'sheet:Sheet1'; sqref = 'A4'; type = 'expression'; formulas = @('FALSE'); dxf = @{ fill = $fill } }
    )
} 'xlsx-cf-expression-apply' | Out-Null

Set-PartText $book 'xl/worksheets/sheet1.xml' {
    param($text)
    if ($text -notmatch '<dimension [^>]*/>') { throw '找不到 dimension' }
    $views = '<sheetViews><sheetView showGridLines="0" workbookViewId="0"/></sheetViews>'
    $text = [regex]::Replace($text, '(<dimension [^>]*/>)', ('$1' + $views), 1)
    $text
}

$png = Get-OutPath 'xlsx-cf-expression-image-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSheetImage'
    path = $book.Replace('\', '/'); format = 'xlsx'
    id = 'sheet:Sheet1'
    output = $png.Replace('\', '/')
} 'xlsx-cf-expression-export' | Out-Null

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$gold = Get-OutPath 'xlsx-cf-expression-image-gold.png'
try {
    $null = $wb.Activate()
    $ws = $wb.Worksheets.Item(1)
    $null = $ws.Activate()
    $excel.ActiveWindow.Zoom = 100
    $excel.ActiveWindow.DisplayGridlines = $false
    $ws.Range('A1:A4').CopyPicture(1, 2)
    Start-Sleep -Milliseconds 200
    $img = [System.Windows.Forms.Clipboard]::GetImage()
    if ($null -eq $img) { throw '剪贴板没有工作表图' }
    try { $img.Save($gold, [System.Drawing.Imaging.ImageFormat]::Png) }
    finally { $img.Dispose() }
    Release-Com $ws
}
finally {
    if ($wb) { $wb.Close($false); Release-Com $wb }
}

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$diff = Get-OutPath 'xlsx-cf-expression-image-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $diff -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log ("xlsx-cf-expression-image-px " + $text)
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'xlsx-cf-expression-image-px' $ok $text
