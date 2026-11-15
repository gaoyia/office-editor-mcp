# xlsx 没写行高时的空行，以及 customHeight 钉住的常规行。
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

function New-GapBook([string]$name, [string]$format) {
    $src = Get-OutPath ($name + '-src.xlsx')
    if (Test-Path $src) { Remove-Item -Force $src }
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'create'
        path = $src.Replace('\', '/'); format = 'xlsx'
        workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(
            @{ ref = 'A1'; value = '' }
            @{ ref = 'A3'; value = '' }
        ) }) }
    } ($name + '-create') | Out-Null

    $book = Get-OutPath ($name + '.xlsx')
    if (Test-Path $book) { Remove-Item -Force $book }
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $src.Replace('\', '/')
        output = $book.Replace('\', '/')
        ops = @(
            @{ op = 'setCellStyle'; id = 'cell:Sheet1!A1'; fill = @{ pattern = 'solid'; fg = @{ rgb = 'FFEE4455' } } }
            @{ op = 'setCellStyle'; id = 'cell:Sheet1!A3'; fill = @{ pattern = 'solid'; fg = @{ rgb = 'FF112233' } } }
        )
    } ($name + '-style') | Out-Null

    $insert = '<sheetViews><sheetView showGridLines="0" workbookViewId="0"/></sheetViews>' + $format
    Set-PartText $book 'xl/worksheets/sheet1.xml' {
        param($text)
        if ($text -notmatch '<dimension [^>]*/>') { throw '找不到 dimension' }
        if ($text -match 'defaultRowHeight' -and $insert -notmatch 'defaultRowHeight') {
            throw '建表时不该写 defaultRowHeight'
        }
        $text = [regex]::Replace($text, '(<dimension [^>]*/>)', ('$1' + $insert), 1)
        $viewsAt = $text.IndexOf('sheetViews')
        $colsAt = $text.IndexOf('<cols>')
        if ($colsAt -ge 0 -and $viewsAt -gt $colsAt) { throw 'sheetViews 落在 cols 后面，Excel 会要求修复' }
        $text
    }.GetNewClosure()
    return $book
}

function Export-Sheet([string]$book, [string]$png) {
    if (Test-Path $png) { Remove-Item -Force $png }
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'exportSheetImage'
        path = $book.Replace('\', '/'); format = 'xlsx'
        id = 'sheet:Sheet1'
        output = $png.Replace('\', '/')
    } 'xlsx-row-gap-export' | Out-Null
}

function Save-Clipboard([string]$path) {
    Start-Sleep -Milliseconds 200
    $img = [System.Windows.Forms.Clipboard]::GetImage()
    if ($null -eq $img) { throw '剪贴板没有工作表图' }
    try { $img.Save($path, [System.Drawing.Imaging.ImageFormat]::Png) }
    finally { $img.Dispose() }
}

$gap = New-GapBook 'xlsx-row-gap-image' ''
$lock = New-GapBook 'xlsx-row-lock-image' '<sheetFormatPr defaultRowHeight="12" customHeight="1"/>'
$gapPng = Get-OutPath 'xlsx-row-gap-image-lib.png'
$lockPng = Get-OutPath 'xlsx-row-lock-image-lib.png'
Export-Sheet $gap $gapPng
Export-Sheet $lock $lockPng

$excel = Ensure-MsExcel $gap
$wb = $null
try {
    foreach ($item in @(
        @{ book = $gap; gold = (Get-OutPath 'xlsx-row-gap-image-gold.png'); range = 'A1:A3' }
        @{ book = $lock; gold = (Get-OutPath 'xlsx-row-lock-image-gold.png'); range = 'A1:A3' }
    )) {
        $wb = Open-ExcelBook $excel $item.book
        $null = $wb.Activate()
        $ws = $wb.Worksheets.Item(1)
        $null = $ws.Activate()
        $excel.ActiveWindow.Zoom = 100
        Write-Log ("gridlines=" + $excel.ActiveWindow.DisplayGridlines + " book=" + $wb.Name)
        $ws.Range($item.range).CopyPicture(1, 2)
        Save-Clipboard $item.gold
        Release-Com $ws
        $wb.Close($false)
        Release-Com $wb
        $wb = $null
    }
}
finally {
    if ($wb) {
        $wb.Close($false)
        Release-Com $wb
    }
}

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
foreach ($item in @(
    @{ name = 'xlsx-row-gap-image-px'; gold = (Get-OutPath 'xlsx-row-gap-image-gold.png'); png = $gapPng; diff = (Get-OutPath 'xlsx-row-gap-image-diff.png') }
    @{ name = 'xlsx-row-lock-image-px'; gold = (Get-OutPath 'xlsx-row-lock-image-gold.png'); png = $lockPng; diff = (Get-OutPath 'xlsx-row-lock-image-diff.png') }
)) {
    $line = & $diffScript -Left $item.gold -Right $item.png -Out $item.diff -MaxDelta 0
    $text = ($line | Out-String).Trim()
    Write-Log ($item.name + ' ' + $text)
    $ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
    Add-Check $item.name $ok $text
}
