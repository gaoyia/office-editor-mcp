# xlsx 隐藏行、隐藏列不占像素。金样是 Excel 16.0 的 CopyPicture，整张剪贴板。
# 红区回绘制引擎修，不要加大 MaxDelta。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'office-com.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

function New-Book([string]$name, $cells, $ops) {
    $src = Get-OutPath "$name-src.xlsx"
    if (Test-Path $src) { Remove-Item -Force $src }
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'create'
        path = $src.Replace('\', '/'); format = 'xlsx'
        workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = $cells }) }
    } "$name-create" | Out-Null
    $out = Get-OutPath "$name.xlsx"
    if (Test-Path $out) { Remove-Item -Force $out }
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $src.Replace('\', '/')
        output = $out.Replace('\', '/')
        ops = $ops
    } "$name-apply" | Out-Null
    return $out
}

function Fill([string]$ref, [string]$rgb) {
    return @{
        op = 'setCellStyle'; id = "cell:Sheet1!$ref"
        fill = @{ pattern = 'solid'; fg = @{ rgb = $rgb } }
    }
}

$rows = @(
    @{ ref = 'A1'; value = '' },
    @{ ref = 'A2'; value = '' },
    @{ ref = 'A3'; value = '' }
)
$cols = @(
    @{ ref = 'A1'; value = '' },
    @{ ref = 'B1'; value = '' },
    @{ ref = 'C1'; value = '' }
)

$midRow = New-Book 'xlsx-hidden-row' $rows @(
    (Fill 'A1' 'FFEE4455'),
    (Fill 'A2' 'FF112233'),
    (Fill 'A3' 'FF00AA66'),
    @{ op = 'setRowHidden'; id = 'sheet:Sheet1'; rows = '2'; hidden = $true }
)
$midCol = New-Book 'xlsx-hidden-col' $cols @(
    (Fill 'A1' 'FFEE4455'),
    (Fill 'B1' 'FF112233'),
    (Fill 'C1' 'FF00AA66'),
    @{ op = 'setColHidden'; id = 'sheet:Sheet1'; col = 'B'; hidden = $true }
)
$onlyRow = New-Book 'xlsx-hidden-only' @(@{ ref = 'A1'; value = '' }) @(
    (Fill 'A1' 'FFEE4455'),
    @{ op = 'setRowHidden'; id = 'sheet:Sheet1'; rows = '1'; hidden = $true }
)
$anchor = New-Book 'xlsx-hidden-anchor' $rows @(
    (Fill 'A1' 'FFEE4455'),
    @{ op = 'mergeRange'; id = 'sheet:Sheet1'; range = 'A1:A3' },
    @{ op = 'setRowHidden'; id = 'sheet:Sheet1'; rows = '1'; hidden = $true }
)

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

$libRow = Export-Lib $midRow 'xlsx-hidden-row-lib.png' 'xlsx-hidden-row-export'
$libCol = Export-Lib $midCol 'xlsx-hidden-col-lib.png' 'xlsx-hidden-col-export'
$libOnly = Export-Lib $onlyRow 'xlsx-hidden-only-lib.png' 'xlsx-hidden-only-export'
$libAnchor = Export-Lib $anchor 'xlsx-hidden-anchor-lib.png' 'xlsx-hidden-anchor-export'

function Save-Clipboard([string]$path) {
    Start-Sleep -Milliseconds 200
    $img = [System.Windows.Forms.Clipboard]::GetImage()
    if ($null -eq $img) { throw '剪贴板没有工作表图' }
    try { $img.Save($path, [System.Drawing.Imaging.ImageFormat]::Png) }
    finally { $img.Dispose() }
}

function Copy-Range($excel, $wb, [string]$range, [string]$path) {
    $null = $wb.Activate()
    $ws = $wb.Worksheets.Item(1)
    $null = $ws.Activate()
    $excel.ActiveWindow.Zoom = 100
    $ws.Range($range).CopyPicture(1, 2)
    Save-Clipboard $path
    Release-Com $ws
}

$excel = Ensure-MsExcel $midRow
$books = @(
    @{ Wb = (Open-ExcelBook $excel $midRow); Range = 'A1:A3'; Gold = 'xlsx-hidden-row-gold.png'; Lib = $libRow; Name = 'xlsx-hidden-row-px' },
    @{ Wb = (Open-ExcelBook $excel $midCol); Range = 'A1:C1'; Gold = 'xlsx-hidden-col-gold.png'; Lib = $libCol; Name = 'xlsx-hidden-col-px' },
    @{ Wb = (Open-ExcelBook $excel $onlyRow); Range = 'A1'; Gold = 'xlsx-hidden-only-gold.png'; Lib = $libOnly; Name = 'xlsx-hidden-only-px' },
    @{ Wb = (Open-ExcelBook $excel $anchor); Range = 'A1:A3'; Gold = 'xlsx-hidden-anchor-gold.png'; Lib = $libAnchor; Name = 'xlsx-hidden-anchor-px' }
)
$goldRow2 = Get-OutPath 'xlsx-hidden-row-gold-2.png'
try {
    Copy-Range $excel $books[0].Wb $books[0].Range (Get-OutPath $books[0].Gold)
    Copy-Range $excel $books[0].Wb $books[0].Range $goldRow2
    for ($i = 1; $i -lt $books.Count; $i++) {
        Copy-Range $excel $books[$i].Wb $books[$i].Range (Get-OutPath $books[$i].Gold)
    }
}
finally {
    foreach ($item in $books) {
        $item.Wb.Close($false)
        Release-Com $item.Wb
    }
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

Add-PixelCheck 'xlsx-hidden-row-stable' (Get-OutPath 'xlsx-hidden-row-gold.png') $goldRow2 'xlsx-hidden-row-stable-diff.png'
foreach ($item in $books) {
    Add-PixelCheck $item.Name (Get-OutPath $item.Gold) $item.Lib ($item.Name + '-diff.png')
}
