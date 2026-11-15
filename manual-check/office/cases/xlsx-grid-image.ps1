# xlsx 关掉网格后的已用区域。金样是 Excel 16.0 的 CopyPicture，整张剪贴板，不裁边。
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

$src = Get-OutPath 'xlsx-grid-image-src.xlsx'
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
} 'xlsx-grid-image-create' | Out-Null

$on = Get-OutPath 'xlsx-grid-image-on.xlsx'
if (Test-Path $on) { Remove-Item -Force $on }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $on.Replace('\', '/')
    ops = @(@{
        op = 'setCellStyle'
        id = 'cell:Sheet1!A1'
        fill = @{ pattern = 'solid'; fg = @{ rgb = 'FFEE4455' } }
    })
} 'xlsx-grid-image-style' | Out-Null

$off = Get-OutPath 'xlsx-grid-image-off.xlsx'
Copy-Item -LiteralPath $on -Destination $off -Force
Set-PartText $off 'xl/worksheets/sheet1.xml' {
    param($text)
    $text.Replace('<sheetData>', '<sheetViews><sheetView showGridLines="0" workbookViewId="0"/></sheetViews><sheetData>')
}

function Export-Lib([string]$srcPath, [string]$pngName, [string]$reqName) {
    $png = Get-OutPath $pngName
    if (Test-Path $png) { Remove-Item -Force $png }
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'exportSheetImage'
        path = $srcPath.Replace('\', '/'); format = 'xlsx'
        id = 'sheet:Sheet1'
        output = $png.Replace('\', '/')
    } $reqName | Out-Null
    return $png
}

$libOn = Export-Lib $on 'xlsx-grid-image-on-lib.png' 'xlsx-grid-image-export-on'
$libOff = Export-Lib $off 'xlsx-grid-image-off-lib.png' 'xlsx-grid-image-export-off'

function Save-Clipboard([string]$path) {
    Start-Sleep -Milliseconds 200
    $img = [System.Windows.Forms.Clipboard]::GetImage()
    if ($null -eq $img) { throw '剪贴板没有工作表图' }
    try {
        $img.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    finally { $img.Dispose() }
}

function Copy-Used($excel, $wb, [string]$path) {
    $null = $wb.Activate()
    $ws = $wb.Worksheets.Item(1)
    $null = $ws.Activate()
    $excel.ActiveWindow.Zoom = 100
    Write-Log ("gridlines=" + $excel.ActiveWindow.DisplayGridlines + " book=" + $wb.Name)
    $ws.Range('A1:B2').CopyPicture(1, 2)
    Save-Clipboard $path
    Release-Com $ws
}

$excel = Ensure-MsExcel $on
$wbOn = Open-ExcelBook $excel $on
$wbOff = Open-ExcelBook $excel $off
$goldOn = Get-OutPath 'xlsx-grid-image-on-gold.png'
$goldOff1 = Get-OutPath 'xlsx-grid-image-off-gold-1.png'
$goldOff2 = Get-OutPath 'xlsx-grid-image-off-gold-2.png'
try {
    Copy-Used $excel $wbOn $goldOn
    Copy-Used $excel $wbOff $goldOff1
    Copy-Used $excel $wbOff $goldOff2
}
finally {
    $wbOff.Close($false)
    $wbOn.Close($false)
    Release-Com $wbOff
    Release-Com $wbOn
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

Add-PixelCheck 'xlsx-grid-image-off-stable' $goldOff1 $goldOff2 'xlsx-grid-image-off-stable-diff.png'
Add-PixelCheck 'xlsx-grid-image-on-px' $goldOn $libOn 'xlsx-grid-image-on-diff.png'
Add-PixelCheck 'xlsx-grid-image-off-px' $goldOff1 $libOff 'xlsx-grid-image-off-diff.png'
