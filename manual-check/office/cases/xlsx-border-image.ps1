# xlsx 虚线、双线、点划线和实线粗细。金样是 Excel 16.0 的 CopyPicture，整张剪贴板，不裁边。
# 每一行都写 15 磅，列 B 宽 10，用来把虚线相位错开。红区回绘制引擎修，不要加大 MaxDelta。

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

$src = Get-OutPath 'xlsx-border-image-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @($cells) }) }
} 'xlsx-border-image-create' | Out-Null

$red = @{ rgb = 'FFEE4455' }
$blue = @{ rgb = 'FF0000FF' }
$specs = @(
    @{ ref = 'A1'; edge = 'bottom'; style = 'thin'; color = $red }
    @{ ref = 'B1'; edge = 'bottom'; style = 'medium'; color = $blue }
    @{ ref = 'C1'; edge = 'bottom'; style = 'dashed'; color = $red }
    @{ ref = 'D1'; edge = 'bottom'; style = 'dotted'; color = $red }
    @{ ref = 'E1'; edge = 'bottom'; style = 'double'; color = $red }
    @{ ref = 'A2'; edge = 'bottom'; style = 'hair'; color = $red }
    @{ ref = 'B2'; edge = 'bottom'; style = 'mediumDashed'; color = $red }
    @{ ref = 'C2'; edge = 'bottom'; style = 'dashDot'; color = $red }
    @{ ref = 'D2'; edge = 'bottom'; style = 'mediumDashDot'; color = $red }
    @{ ref = 'E2'; edge = 'bottom'; style = 'dashDotDot'; color = $red }
    @{ ref = 'A3'; edge = 'bottom'; style = 'mediumDashDotDot'; color = $red }
    @{ ref = 'B3'; edge = 'bottom'; style = 'slantDashDot'; color = $red }
    @{ ref = 'C3'; edge = 'bottom'; style = 'thick'; color = $blue }
    @{ ref = 'D3'; edge = 'left'; style = 'dashed'; color = $red }
    @{ ref = 'E3'; edge = 'left'; style = 'double'; color = $red }
    @{ ref = 'B4'; edge = 'top'; style = 'medium'; color = $blue }
    @{ ref = 'C4'; edge = 'right'; style = 'medium'; color = $blue }
    @{ ref = 'D4'; edge = 'left'; style = 'slantDashDot'; color = $red }
)

$ops = [System.Collections.Generic.List[object]]::new()
$ops.Add(@{ op = 'setColWidth'; id = 'sheet:Sheet1'; col = 'B'; width = 10 })
foreach ($row in 1..4) {
    $ops.Add(@{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = $row; height = 15 })
}
foreach ($spec in $specs) {
    $ops.Add(@{
        op = 'setCellStyle'
        id = "cell:Sheet1!$($spec.ref)"
        border = @{ $spec.edge = @{ style = $spec.style; color = $spec.color } }
    })
}

$book = Get-OutPath 'xlsx-border-image.xlsx'
if (Test-Path $book) { Remove-Item -Force $book }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $book.Replace('\', '/')
    ops = @($ops)
} 'xlsx-border-image-style' | Out-Null

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

$png = Get-OutPath 'xlsx-border-image-lib.png'
if (Test-Path $png) { Remove-Item -Force $png }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSheetImage'
    path = $book.Replace('\', '/'); format = 'xlsx'
    id = 'sheet:Sheet1'
    output = $png.Replace('\', '/')
} 'xlsx-border-image-export' | Out-Null

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
$gold = Get-OutPath 'xlsx-border-image-gold.png'
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
$out = Get-OutPath 'xlsx-border-image-diff.png'
$line = & $diffScript -Left $gold -Right $png -Out $out -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "xlsx-border-image-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'xlsx-border-image-px' $ok $text
