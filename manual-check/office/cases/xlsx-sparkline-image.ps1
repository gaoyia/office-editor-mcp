# xlsx 折线迷你图只读 PNG。金样是 Excel 16.0 对空单元格的 CopyPicture。
# 连续两次复制必须差 0。库和金样的差记在检查里，不靠加大 MaxDelta。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'office-com.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$src = Get-OutPath 'xlsx-sparkline-image-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'B1'; value = 1 },
                @{ ref = 'C1'; value = 4 },
                @{ ref = 'D1'; value = 2 }
            )
        })
    }
} 'xlsx-sparkline-image-create' | Out-Null

$book = Get-OutPath 'xlsx-sparkline-image.xlsx'
if (Test-Path $book) { Remove-Item -Force $book }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $book.Replace('\', '/')
    ops = @(@{
        op = 'insertSparkline'; id = 'sheet:Sheet1'; ref = 'A1'; formula = 'B1:D1'
    })
} 'xlsx-sparkline-image-insert' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $book.Replace('\', '/'); format = 'xlsx'
    scope = @{ sparklines = $true }
} 'xlsx-sparkline-image-inspect'
$id = [string]$tree.sheets[0].sparklines[0].id
Add-Check 'xlsx-sparkline-image-id' ($id -eq 'sheet:Sheet1/spark:1') "id=$id"

$lib = Get-OutPath 'xlsx-sparkline-image-lib.png'
if (Test-Path $lib) { Remove-Item -Force $lib }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportSparklineImage'
    path = $book.Replace('\', '/'); format = 'xlsx'
    id = $id
    output = $lib.Replace('\', '/')
} 'xlsx-sparkline-image-export' | Out-Null

function Save-CellBitmap($img, $path) {
    $w = $img.Width - 1
    $h = $img.Height - 1
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    try {
        $srcRect = New-Object System.Drawing.Rectangle 0, 0, $w, $h
        $g.DrawImage($img, (New-Object System.Drawing.Rectangle 0, 0, $w, $h), $srcRect, [System.Drawing.GraphicsUnit]::Pixel)
    }
    finally { $g.Dispose() }
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

function Copy-Cell($ws, $path) {
    $ws.Range('A1').CopyPicture(1, 2)
    Start-Sleep -Milliseconds 200
    $img = [System.Windows.Forms.Clipboard]::GetImage()
    if ($null -eq $img) { throw '剪贴板没有单元格图' }
    Save-CellBitmap $img $path
    $img.Dispose()
}

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$ws = $wb.Worksheets.Item(1)
$excel.ActiveWindow.DisplayGridlines = $false
$excel.ActiveWindow.Zoom = 100
$gold1 = Get-OutPath 'xlsx-sparkline-image-gold-1.png'
$gold2 = Get-OutPath 'xlsx-sparkline-image-gold-2.png'
try {
    Copy-Cell $ws $gold1
    Copy-Cell $ws $gold2
}
finally {
    $wb.Close($false)
    Release-Com $ws
    Release-Com $wb
}

$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$stableOut = Get-OutPath 'xlsx-sparkline-image-stable-diff.png'
$stable = & $diffScript -Left $gold1 -Right $gold2 -Out $stableOut -MaxDelta 0
$stableText = ($stable | Out-String).Trim()
$stableOk = ($stableText -match 'differ=0\b') -and ($stableText -match 'maxDelta=0\b')
Add-Check 'xlsx-sparkline-image-stable' $stableOk $stableText

$diffOut = Get-OutPath 'xlsx-sparkline-image-diff.png'
$line = & $diffScript -Left $gold1 -Right $lib -Out $diffOut -MaxDelta 0
$text = ($line | Out-String).Trim()
Write-Log "xlsx-sparkline-image-px $text"
$ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'xlsx-sparkline-image-px' $ok $text
