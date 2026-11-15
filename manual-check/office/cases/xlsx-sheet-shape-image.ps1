# xlsx 没有描边的纯色矩形。金样是 Excel 16.0 的 CopyPicture，整张剪贴板，不裁边。
# 红区回绘制引擎修，不要加大 MaxDelta。

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

$src = Get-OutPath 'xlsx-sheet-shape-image-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(
        @{ ref = 'A1'; value = '' }
        @{ ref = 'A2'; value = '' }
        @{ ref = 'A3'; value = '' }
    ) }) }
} 'xlsx-sheet-shape-create' | Out-Null

$px = 9525
$shaped = Get-OutPath 'xlsx-sheet-shape-image.xlsx'
if (Test-Path $shaped) { Remove-Item -Force $shaped }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $shaped.Replace('\', '/')
    ops = @(
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 1; height = 15 }
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 2; height = 15 }
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 3; height = 15 }
        @{ op = 'insertShape'; id = 'sheet:Sheet1'; prst = 'rect'; anchor = 'oneCell'; from = 'A1'; cx = (8 * $px); cy = (8 * $px) }
        @{ op = 'insertShape'; id = 'sheet:Sheet1'; prst = 'rect'; anchor = 'oneCell'; from = 'A2'; fromOffX = (5 * $px); fromOffY = (5 * $px); cx = (20 * $px); cy = (8 * $px) }
        @{ op = 'insertShape'; id = 'sheet:Sheet1'; prst = 'rect'; anchor = 'oneCell'; from = 'A3'; fromOffX = 4763; cx = (16 * $px); cy = (8 * $px) }
    )
} 'xlsx-sheet-shape-apply' | Out-Null

Set-PartText $shaped 'xl/worksheets/sheet1.xml' {
    param($text)
    if ($text -notmatch '<dimension [^>]*/>') { throw '找不到 dimension' }
    $views = '<sheetViews><sheetView showGridLines="0" workbookViewId="0"/></sheetViews>'
    $text = [regex]::Replace($text, '(<dimension [^>]*/>)', ('$1' + $views), 1)
    $text
}
Set-PartText $shaped 'xl/drawings/drawing1.xml' {
    param($text)
    $text = $text.Replace('val="FFFFFF"', 'val="EE4455"')
    $line = '<a:ln w="9525"><a:solidFill><a:srgbClr val="000000"/></a:solidFill></a:ln>'
    if (-not $text.Contains($line)) { throw '找不到矩形描边' }
    $text.Replace($line, '<a:ln><a:noFill/></a:ln>')
}

$png = Get-OutPath 'xlsx-sheet-shape-image-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSheetImage'
    path = $shaped.Replace('\', '/'); format = 'xlsx'
    id = 'sheet:Sheet1'
    output = $png.Replace('\', '/')
} 'xlsx-sheet-shape-export' | Out-Null

$excel = Ensure-MsExcel $shaped
$wb = Open-ExcelBook $excel $shaped
$gold = Get-OutPath 'xlsx-sheet-shape-image-gold.png'
try {
    $null = $wb.Activate()
    $ws = $wb.Worksheets.Item(1)
    $null = $ws.Activate()
    $excel.ActiveWindow.Zoom = 100
    $excel.ActiveWindow.DisplayGridlines = $false
    $ws.Range('A1:A3').CopyPicture(1, 2)
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
$diff = Get-OutPath 'xlsx-sheet-shape-image-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $diff -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log ("xlsx-sheet-shape-image-px " + $text)
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'xlsx-sheet-shape-image-px' $ok $text
