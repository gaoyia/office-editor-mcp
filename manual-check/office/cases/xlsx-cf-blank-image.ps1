# xlsx 空白和非空白的纯色。字色和底色相同，字形不进入对照。
# 金样是 Excel 16.0 的 CopyPicture，整张剪贴板，不裁边。红区回绘制引擎修，不要加大 MaxDelta。

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

function Save-Clipboard([string]$path) {
    Start-Sleep -Milliseconds 200
    $img = [System.Windows.Forms.Clipboard]::GetImage()
    if ($null -eq $img) { throw '剪贴板没有工作表图' }
    try { $img.Save($path, [System.Drawing.Imaging.ImageFormat]::Png) }
    finally { $img.Dispose() }
}

$white = @{ color = @{ rgb = 'FFFFFFFF' } }
$cells = @(
    @{ ref = 'A1'; value = 1 }
    @{ ref = 'A2'; value = '' }
    @{ ref = 'A3'; value = ' ' }
    @{ ref = 'A5'; value = 'a' }
    @{ ref = 'A6'; value = '1' }
    @{ ref = 'B1'; value = 1 }
    @{ ref = 'B2'; value = '' }
    @{ ref = 'B3'; value = ' a ' }
)

$src = Get-OutPath 'xlsx-cf-blank-image-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = $cells }) }
} 'xlsx-cf-blank-create' | Out-Null

$ops = @()
foreach ($row in 1..6) {
    $ops += @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = $row; height = 15 }
}
foreach ($ref in @('A1','A2','A3','A5','A6','B1','B2','B3')) {
    $ops += @{ op = 'setCellStyle'; id = "cell:Sheet1!$ref"; font = $white }
}

$book = Get-OutPath 'xlsx-cf-blank-image.xlsx'
if (Test-Path $book) { Remove-Item -Force $book }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/'); output = $book.Replace('\', '/')
    ops = $ops
} 'xlsx-cf-blank-style' | Out-Null

Set-PartText $book 'xl/styles.xml' {
    param($text)
    if ($text.Contains('<dxfs')) { throw '样式里已经有 dxfs' }
    $dxf = '<dxfs count="1"><dxf><font><color rgb="FFEE4455"/></font><fill><patternFill><bgColor rgb="FFEE4455"/></patternFill></fill></dxf></dxfs>'
    $text.Replace('</styleSheet>', ($dxf + '</styleSheet>'))
}
Set-PartText $book 'xl/worksheets/sheet1.xml' {
    param($text)
    if ($text -notmatch '<dimension [^>]*/>') { throw '找不到 dimension' }
    $views = '<sheetViews><sheetView showGridLines="0" workbookViewId="0"/></sheetViews>'
    $text = [regex]::Replace($text, '(<dimension [^>]*/>)', ('$1' + $views), 1)
    $viewsAt = $text.IndexOf('sheetViews')
    $colsAt = $text.IndexOf('<cols>')
    if ($colsAt -ge 0 -and $viewsAt -gt $colsAt) { throw 'sheetViews 落在 cols 后面，Excel 会要求修复' }
    $rules = '<conditionalFormatting sqref="A1:A6"><cfRule type="containsBlanks" dxfId="0" priority="1"><formula>LEN(TRIM(A1))=0</formula></cfRule></conditionalFormatting><conditionalFormatting sqref="B1:B4"><cfRule type="notContainsBlanks" dxfId="0" priority="2"><formula>LEN(TRIM(B1))&gt;0</formula></cfRule></conditionalFormatting>'
    $text.Replace('</worksheet>', ($rules + '</worksheet>'))
}

$png = Get-OutPath 'xlsx-cf-blank-image-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSheetImage'
    path = $book.Replace('\', '/'); format = 'xlsx'
    id = 'sheet:Sheet1'
    output = $png.Replace('\', '/')
} 'xlsx-cf-blank-export' | Out-Null

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$gold = Get-OutPath 'xlsx-cf-blank-image-gold.png'
try {
    $null = $wb.Activate()
    $ws = $wb.Worksheets.Item(1)
    $null = $ws.Activate()
    $excel.ActiveWindow.Zoom = 100
    Write-Log ("gridlines=" + $excel.ActiveWindow.DisplayGridlines + " book=" + $wb.Name)
    $ws.Range('A1:B6').CopyPicture(1, 2)
    Save-Clipboard $gold
    Release-Com $ws
}
finally {
    $wb.Close($false)
    Release-Com $wb
}

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$out = Get-OutPath 'xlsx-cf-blank-image-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $out -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "xlsx-cf-blank-image-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'xlsx-cf-blank-image-px' $ok $text
