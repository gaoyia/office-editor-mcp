# xlsx 重复值的纯色。字色和底色相同，字形不进入对照。
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
    @{ ref = 'A2'; value = 2 }
    @{ ref = 'A3'; value = '1.00' }
    @{ ref = 'A4'; value = '' }
    @{ ref = 'A5'; value = '' }
    @{ ref = 'A6'; value = 'Ab' }
    @{ ref = 'A7'; value = 'ab' }
    @{ ref = 'A8'; value = ' ' }
    @{ ref = 'A9'; value = 0.5 }
    @{ ref = 'A10'; value = '50 %' }
    @{ ref = 'A11'; value = 100 }
    @{ ref = 'A12'; value = '1e2' }
    @{ ref = 'A13'; value = -1 }
    @{ ref = 'A14'; value = '(1)' }
    @{ ref = 'A15'; value = 1000 }
    @{ ref = 'A16'; value = '1,000' }
    @{ ref = 'B1'; value = 1 }
)

$src = Get-OutPath 'xlsx-cf-duplicate-image-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = $cells }) }
} 'xlsx-cf-duplicate-create' | Out-Null

$ops = @()
foreach ($row in 1..16) {
    $ops += @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = $row; height = 15 }
}
foreach ($ref in @('A1','A2','A3','A4','A5','A6','A7','A8','A9','A10','A11','A12','A13','A14','A15','A16')) {
    $ops += @{ op = 'setCellStyle'; id = "cell:Sheet1!$ref"; font = $white }
}
$ops += @{ op = 'setCellStyle'; id = 'cell:Sheet1!B1'; numFmt = '0.00'; font = $white }

$book = Get-OutPath 'xlsx-cf-duplicate-image.xlsx'
if (Test-Path $book) { Remove-Item -Force $book }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/'); output = $book.Replace('\', '/')
    ops = $ops
} 'xlsx-cf-duplicate-style' | Out-Null

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
    $rule = '<conditionalFormatting sqref="A1:B16"><cfRule type="duplicateValues" dxfId="0" priority="1"/></conditionalFormatting>'
    if ($text.Contains('</worksheet>')) {
        $text = $text.Replace('</worksheet>', ($rule + '</worksheet>'))
    } else {
        throw '找不到 worksheet 结尾'
    }
    $text
}

$png = Get-OutPath 'xlsx-cf-duplicate-image-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSheetImage'
    path = $book.Replace('\', '/'); format = 'xlsx'
    id = 'sheet:Sheet1'
    output = $png.Replace('\', '/')
} 'xlsx-cf-duplicate-export' | Out-Null

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$gold = Get-OutPath 'xlsx-cf-duplicate-image-gold.png'
try {
    $null = $wb.Activate()
    $ws = $wb.Worksheets.Item(1)
    $null = $ws.Activate()
    $excel.ActiveWindow.Zoom = 100
    Write-Log ("gridlines=" + $excel.ActiveWindow.DisplayGridlines + " book=" + $wb.Name)
    $ws.Range('A1:B16').CopyPicture(1, 2)
    Save-Clipboard $gold
    Release-Com $ws
}
finally {
    $wb.Close($false)
    Release-Com $wb
}

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$out = Get-OutPath 'xlsx-cf-duplicate-image-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $out -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "xlsx-cf-duplicate-image-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'xlsx-cf-duplicate-image-px' $ok $text
