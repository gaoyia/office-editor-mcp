# xlsx 两种纯色的图案。金样是 Excel 16.0 的 CopyPicture，整张剪贴板，不裁边。
# 列 A 宽 10，第 1 行高 16 磅，用来把图案相位错开。红区回绘制引擎修，不要加大 MaxDelta。

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

$cells = [System.Collections.Generic.List[object]]::new()
foreach ($col in 'A', 'B', 'C', 'D', 'E') {
    foreach ($row in 1..4) {
        $cells.Add(@{ ref = "$col$row"; value = '' })
    }
}

$src = Get-OutPath 'xlsx-pattern-image-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @($cells) }) }
} 'xlsx-pattern-image-create' | Out-Null

$specs = @(
    @{ ref = 'A1'; pattern = 'gray125'; fg = 'FF000000'; bg = 'FFFFFFFF' }
    @{ ref = 'B1'; pattern = 'gray0625'; fg = 'FF000000'; bg = 'FFFFFFFF' }
    @{ ref = 'C1'; pattern = 'lightHorizontal'; fg = 'FF000000'; bg = 'FFFFFFFF' }
    @{ ref = 'D1'; pattern = 'darkHorizontal'; fg = 'FF000000'; bg = 'FFFFFFFF' }
    @{ ref = 'E1'; pattern = 'mediumGray'; fg = 'FF000000'; bg = 'FFFFFFFF' }
    @{ ref = 'A2'; pattern = 'lightVertical'; fg = 'FF000000'; bg = 'FFFFFFFF' }
    @{ ref = 'B2'; pattern = 'darkVertical'; fg = 'FF000000'; bg = 'FFFFFFFF' }
    @{ ref = 'C2'; pattern = 'lightGrid'; fg = 'FF000000'; bg = 'FFFFFFFF' }
    @{ ref = 'D2'; pattern = 'darkGrid'; fg = 'FF000000'; bg = 'FFFFFFFF' }
    @{ ref = 'E2'; pattern = 'lightGray'; fg = 'FF000000'; bg = 'FFFFFFFF' }
    @{ ref = 'A3'; pattern = 'darkGray'; fg = 'FF000000'; bg = 'FFFFFFFF' }
    @{ ref = 'B3'; pattern = 'darkDown'; fg = 'FF000000'; bg = 'FFFFFFFF' }
    @{ ref = 'C3'; pattern = 'lightDown'; fg = 'FF000000'; bg = 'FFFFFFFF' }
    @{ ref = 'D3'; pattern = 'darkUp'; fg = 'FF000000'; bg = 'FFFFFFFF' }
    @{ ref = 'E3'; pattern = 'lightUp'; fg = 'FF000000'; bg = 'FFFFFFFF' }
    @{ ref = 'A4'; pattern = 'darkTrellis'; fg = 'FF000000'; bg = 'FFFFFFFF' }
    @{ ref = 'B4'; pattern = 'lightTrellis'; fg = 'FF000000'; bg = 'FFFFFFFF' }
    @{ ref = 'C4'; pattern = 'gray125'; fg = 'FFEE4455'; bg = 'FF112233' }
    @{ ref = 'D4'; pattern = 'gray125' }
    @{ ref = 'E4'; pattern = 'gray125'; fg = 'FFEE4455' }
)

$ops = [System.Collections.Generic.List[object]]::new()
$ops.Add(@{ op = 'setColWidth'; id = 'sheet:Sheet1'; col = 'A'; width = 10 })
$ops.Add(@{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 1; height = 16 })
foreach ($spec in $specs) {
    $fill = @{ pattern = $spec.pattern }
    if ($spec.ContainsKey('fg')) { $fill.fg = @{ rgb = $spec.fg } }
    if ($spec.ContainsKey('bg')) { $fill.bg = @{ rgb = $spec.bg } }
    $ops.Add(@{ op = 'setCellStyle'; id = "cell:Sheet1!$($spec.ref)"; fill = $fill })
}

$book = Get-OutPath 'xlsx-pattern-image.xlsx'
if (Test-Path $book) { Remove-Item -Force $book }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $book.Replace('\', '/')
    ops = @($ops)
} 'xlsx-pattern-image-style' | Out-Null

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

$png = Get-OutPath 'xlsx-pattern-image-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSheetImage'
    path = $book.Replace('\', '/'); format = 'xlsx'
    id = 'sheet:Sheet1'
    output = $png.Replace('\', '/')
} 'xlsx-pattern-image-export' | Out-Null

function Save-Clipboard([string]$path) {
    Start-Sleep -Milliseconds 200
    $img = [System.Windows.Forms.Clipboard]::GetImage()
    if ($null -eq $img) { throw '剪贴板没有工作表图' }
    try {
        $img.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    finally { $img.Dispose() }
}

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$gold = Get-OutPath 'xlsx-pattern-image-gold.png'
try {
    $null = $wb.Activate()
    $ws = $wb.Worksheets.Item(1)
    $null = $ws.Activate()
    $excel.ActiveWindow.Zoom = 100
    Write-Log ("gridlines=" + $excel.ActiveWindow.DisplayGridlines + " book=" + $wb.Name)
    $ws.Range('A1:E4').CopyPicture(1, 2)
    Save-Clipboard $gold
    Release-Com $ws
}
finally {
    $wb.Close($false)
    Release-Com $wb
}

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$out = Get-OutPath 'xlsx-pattern-image-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $out -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "xlsx-pattern-image-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'xlsx-pattern-image-px' $ok $text
