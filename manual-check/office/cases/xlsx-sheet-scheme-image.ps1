# xlsx 没有描边的矩形，填充是没有颜色变换的 schemeClr。
# 金样是 Excel 16.0 改过 accent1 并另存后的 CopyPicture。红区回绘制引擎修，不要加大 MaxDelta。

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
        if ($null -eq $entry) { throw "没有 $entryName" }
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

function Stop-Excel {
    Close-OfficeApps -Quit
    if ($global:OoxpExcelPid) {
        Stop-Process -Id $global:OoxpExcelPid -Force -ErrorAction SilentlyContinue
        $global:OoxpExcelPid = $null
        $global:OoxpExcel = $null
    }
}

$src = Get-OutPath 'xlsx-sheet-scheme-image-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = '' }) }) }
} 'xlsx-sheet-scheme-create' | Out-Null

$book = Get-OutPath 'xlsx-sheet-scheme-image.xlsx'
if (Test-Path $book) { Remove-Item -Force $book }
$px = 9525
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/'); output = $book.Replace('\', '/')
    ops = @(
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 1; height = 30 }
        @{ op = 'setCellStyle'; id = 'cell:Sheet1!A1'; font = @{ color = @{ rgb = 'FFFFFFFF' } } }
        @{ op = 'insertShape'; id = 'sheet:Sheet1'; prst = 'rect'; anchor = 'oneCell'; from = 'A1'; fromOffX = $px; fromOffY = $px; cx = (8 * $px); cy = (8 * $px) }
        @{ op = 'insertShape'; id = 'sheet:Sheet1'; prst = 'rect'; anchor = 'oneCell'; from = 'A1'; fromOffX = (20 * $px); fromOffY = $px; cx = (4 * $px); cy = (4 * $px) }
    )
} 'xlsx-sheet-scheme-apply' | Out-Null

Set-PartText $book 'xl/worksheets/sheet1.xml' {
    param($text)
    if ($text -notmatch '<dimension [^>]*/>') { throw '找不到 dimension' }
    $views = '<sheetViews><sheetView showGridLines="0" workbookViewId="0"/></sheetViews>'
    $text = [regex]::Replace($text, '(<dimension [^>]*/>)', ('$1' + $views), 1)
    $text
}
Set-PartText $book 'xl/drawings/drawing1.xml' {
    param($text)
    $line = '<a:ln w="9525"><a:solidFill><a:srgbClr val="000000"/></a:solidFill></a:ln>'
    if (-not $text.Contains($line)) { throw '找不到描边' }
    $text = $text.Replace($line, '<a:ln><a:noFill/></a:ln>')
    $fill = '<a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill>'
    $at = $text.IndexOf($fill)
    if ($at -lt 0) { throw '找不到第一块填充' }
    $text = $text.Remove($at, $fill.Length).Insert($at, '<a:solidFill><a:schemeClr val="accent1"/></a:solidFill>')
    $at = $text.IndexOf($fill)
    if ($at -lt 0) { throw '找不到第二块填充' }
    $text.Remove($at, $fill.Length).Insert($at, '<a:solidFill><a:schemeClr val="dk1"/></a:solidFill>')
}

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
try { $wb.Save() }
finally { $wb.Close($true); Release-Com $wb }
Stop-Excel

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
try {
    $colors = $wb.Theme.ThemeColorScheme
    # msoThemeAccent1 = 5。RGB 是 BGR，EE4455 写成 5544EE。
    $colors.Colors(5).RGB = 0x5544EE
    $wb.Save()
    Release-Com $colors
}
finally { $wb.Close($true); Release-Com $wb }
Stop-Excel

$png = Get-OutPath 'xlsx-sheet-scheme-image-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSheetImage'
    path = $book.Replace('\', '/'); format = 'xlsx'
    id = 'sheet:Sheet1'
    output = $png.Replace('\', '/')
} 'xlsx-sheet-scheme-export' | Out-Null

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$gold = Get-OutPath 'xlsx-sheet-scheme-image-gold.png'
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
finally { $wb.Close($false); Release-Com $wb }

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$out = Get-OutPath 'xlsx-sheet-scheme-image-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $out -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "xlsx-sheet-scheme-image-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'xlsx-sheet-scheme-image-px' $ok $text
