# xlsx sheet crop-then-upscale opaque PNG. Gold = Excel 16.0 CopyPicture, same address.
# Integer srcRect only; upscale both dims (no downscale/mixed). Not the 64x20->63x19 twoCell case.

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

function Save-Png([System.Drawing.Bitmap]$bmp, [string]$path) {
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
}

function Compare-Px([string]$name, [string]$gold, [string]$lib) {
    $diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
    $diff = Get-OutPath ($name + '-diff.png')
    $line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
    $text = ($line | Out-String).Trim()
    Write-Log ($name + '-px ' + $text)
    $ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
    Add-Check ($name + '-px') $ok $text
}

function Export-And-Gold([string]$xlsx, [string]$range, [string]$stem) {
    $png = Get-OutPath ($stem + '-lib.png')
    if (Test-Path $png) { Remove-Item -Force $png }
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'exportSheetImage'
        path = $xlsx.Replace('\', '/'); format = 'xlsx'
        id = 'sheet:Sheet1'
        output = $png.Replace('\', '/')
    } ($stem + '-export') | Out-Null

    $excel = Ensure-MsExcel $xlsx
    $wb = Open-ExcelBook $excel $xlsx
    $gold = Get-OutPath ($stem + '-gold.png')
    try {
        $null = $wb.Activate()
        $ws = $wb.Worksheets.Item(1)
        $null = $ws.Activate()
        $excel.ActiveWindow.Zoom = 100
        $excel.ActiveWindow.DisplayGridlines = $false
        $ws.Range($range).CopyPicture(1, 2)
        Start-Sleep -Milliseconds 300
        $img = [System.Windows.Forms.Clipboard]::GetImage()
        if ($null -eq $img) { throw 'clipboard empty' }
        try { $img.Save($gold, [System.Drawing.Imaging.ImageFormat]::Png) }
        finally { $img.Dispose() }
        Release-Com $ws
    }
    finally {
        if ($wb) { $wb.Close($false); Release-Com $wb }
    }
    Compare-Px $stem $gold $png
}

$px = 9525

# 4x4 checker for center crop 2x2 -> 4x4 upscale; 4x3 top crop -> 8x6
$dots = New-Object System.Drawing.Bitmap 4, 4
$colors = @(
    @(255,0,0),@(0,255,0),@(0,0,255),@(255,255,0),
    @(255,0,255),@(0,255,255),@(17,34,51),@(68,85,102),
    @(119,136,153),@(170,187,204),@(221,238,255),@(1,2,3),
    @(4,5,6),@(7,8,9),@(10,11,12),@(13,14,15)
)
for ($y = 0; $y -lt 4; $y++) {
    for ($x = 0; $x -lt 4; $x++) {
        $c = $colors[$y * 4 + $x]
        $dots.SetPixel($x, $y, [System.Drawing.Color]::FromArgb(255, $c[0], $c[1], $c[2]))
    }
}
$dotsPath = Get-OutPath 'xlsx-sheet-crop-scale-dots.png'
Save-Png $dots $dotsPath
$dots.Dispose()

$tall = New-Object System.Drawing.Bitmap 4, 3
for ($y = 0; $y -lt 3; $y++) {
    for ($x = 0; $x -lt 4; $x++) {
        $v = [byte](40 + $x * 40 + $y * 20)
        $tall.SetPixel($x, $y, [System.Drawing.Color]::FromArgb(255, $v, [byte](80 + $y * 30), [byte](120 + $x * 20)))
    }
}
$tallPath = Get-OutPath 'xlsx-sheet-crop-scale-tall.png'
Save-Png $tall $tallPath
$tall.Dispose()

$src = Get-OutPath 'xlsx-sheet-crop-scale-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(
        @{ ref = 'A1'; value = '' }
        @{ ref = 'A2'; value = '' }
        @{ ref = 'A3'; value = '' }
        @{ ref = 'A4'; value = '' }
        @{ ref = 'A5'; value = '' }
        @{ ref = 'A6'; value = '' }
    ) }) }
} 'xlsx-sheet-crop-scale-create' | Out-Null

$book = Get-OutPath 'xlsx-sheet-crop-scale.xlsx'
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
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 5; height = 15 }
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 6; height = 15 }
        # center 2x2 -> 4x4 at (1,1)
        @{ op = 'insertPicture'; id = 'sheet:Sheet1'; file = $dotsPath.Replace('\', '/'); from = 'A1'; fromOffX = (1 * $px); fromOffY = (1 * $px); cx = (4 * $px); cy = (4 * $px) }
        # left crop 3x4 -> 6x8 at another origin
        @{ op = 'insertPicture'; id = 'sheet:Sheet1'; file = $dotsPath.Replace('\', '/'); from = 'A1'; fromOffX = (20 * $px); fromOffY = (2 * $px); cx = (6 * $px); cy = (8 * $px) }
        # top crop 4x2 from 4x3 -> 8x6 (drop bottom row via b=33333 on h=3)
        @{ op = 'insertPicture'; id = 'sheet:Sheet1'; file = $tallPath.Replace('\', '/'); from = 'A1'; fromOffX = (40 * $px); fromOffY = (1 * $px); cx = (8 * $px); cy = (6 * $px) }
        # integer crop 1:1 still present (regression): center 2x2 -> 2x2
        @{ op = 'insertPicture'; id = 'sheet:Sheet1'; file = $dotsPath.Replace('\', '/'); from = 'A1'; fromOffX = (1 * $px); fromOffY = (40 * $px); cx = (2 * $px); cy = (2 * $px) }
    )
} 'xlsx-sheet-crop-scale-apply' | Out-Null

Set-PartText $book 'xl/worksheets/sheet1.xml' {
    param($text)
    if ($text -notmatch '<dimension [^>]*/>') { throw 'dimension missing' }
    $views = '<sheetViews><sheetView showGridLines="0" workbookViewId="0"/></sheetViews>'
    [regex]::Replace($text, '(<dimension [^>]*/>)', ('$1' + $views), 1)
}
Set-PartText $book 'xl/drawings/drawing1.xml' {
    param($text)
    $rects = @(
        '<a:srcRect l="25000" t="25000" r="25000" b="25000"/>',
        '<a:srcRect l="25000"/>',
        '<a:srcRect b="33333"/>',
        '<a:srcRect l="25000" t="25000" r="25000" b="25000"/>'
    )
    $script:i = 0
    $text = [regex]::Replace($text, '(<a:blip\b[^>]*/>)', {
        param($m)
        $r = $rects[$script:i]
        $script:i++
        ($m.Value + $r)
    })
    if ($script:i -ne 4) { throw ('srcRect inject count ' + $script:i) }
    $text
}

Export-And-Gold $book 'A1:A6' 'xlsx-sheet-crop-scale'

