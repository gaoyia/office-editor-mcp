# 柱形迷你图的高点、低点、首点和末点。金样是 Excel 16.0 对空单元格的 CopyPicture。
# 红区回绘制引擎修，不要加大 MaxDelta。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'office-com.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

function Save-CellBitmap($img, $path) {
    $w = $img.Width - 1
    $h = $img.Height - 1
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    try {
        $src = New-Object System.Drawing.Rectangle 0, 0, $w, $h
        $g.DrawImage($img, $src, $src, [System.Drawing.GraphicsUnit]::Pixel)
    }
    finally { $g.Dispose() }
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

function Copy-Cell($ws, [string]$addr, [string]$path) {
    $cell = $ws.Range($addr)
    $cell.CopyPicture(1, 2)
    Start-Sleep -Milliseconds 300
    $img = [System.Windows.Forms.Clipboard]::GetImage()
    if ($null -eq $img) { throw "剪贴板没有 $addr" }
    Save-CellBitmap $img $path
    $img.Dispose()
    Release-Com $cell
}

$src = Get-OutPath 'xlsx-sparkline-column-color-src.xlsx'
$book = Get-OutPath 'xlsx-sparkline-column-color.xlsx'
foreach ($p in @($src, $book)) { if (Test-Path $p) { Remove-Item -Force $p } }

$sheet1 = @()
$sheet2 = @()
foreach ($row in 1..4) {
    $vals = switch ($row) {
        2 { @(3, 3, 3) }
        3 { @(2, 4, 1) }
        default { @(1, 4, 2) }
    }
    $sheet1 += @{ ref = "B$row"; value = $vals[0] }
    $sheet1 += @{ ref = "C$row"; value = $vals[1] }
    $sheet1 += @{ ref = "D$row"; value = $vals[2] }
}
$sheet2 += @{ ref = 'B1'; value = 1 }
$sheet2 += @{ ref = 'C1'; value = 4 }
$sheet2 += @{ ref = 'D1'; value = 2 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(
            @{ name = 'Sheet1'; cells = $sheet1 },
            @{ name = 'Sheet2'; cells = $sheet2 }
        )
    }
} 'xlsx-sparkline-column-color-create' | Out-Null

$colors = @{
    high = @{ rgb = 'FFEE4455' }
    low = @{ rgb = 'FF112233' }
    last = @{ rgb = 'FFABCDEF' }
}
$ops = @(
    @{ op = 'insertSparkline'; id = 'sheet:Sheet1'; ref = 'A1'; formula = 'B1:D1' },
    @{
        op = 'setSparkline'; id = 'sheet:Sheet1/spark:1'; type = 'column'
        high = $true; low = $true
        colors = @{ high = $colors.high; low = $colors.low }
    },
    @{ op = 'insertSparkline'; id = 'sheet:Sheet1'; ref = 'A2'; formula = 'B2:D2' },
    @{
        op = 'setSparkline'; id = 'sheet:Sheet1/spark:2'; type = 'column'
        high = $true; low = $true
        colors = @{ high = $colors.high; low = $colors.low }
    },
    @{ op = 'insertSparkline'; id = 'sheet:Sheet1'; ref = 'A3'; formula = 'B3:D3' },
    @{
        op = 'setSparkline'; id = 'sheet:Sheet1/spark:3'; type = 'column'
        low = $true; last = $true
        colors = @{ low = $colors.low; last = $colors.last }
    },
    @{ op = 'insertSparkline'; id = 'sheet:Sheet1'; ref = 'A4'; formula = 'B4:D4' },
    @{ op = 'setSparkline'; id = 'sheet:Sheet1/spark:4'; type = 'column'; markers = $true },
    @{ op = 'insertSparkline'; id = 'sheet:Sheet2'; ref = 'A1'; formula = 'B1:D1' },
    @{
        op = 'setSparkline'; id = 'sheet:Sheet2/spark:1'; type = 'column'
        high = $true; low = $true
        colors = @{ high = $colors.high; low = $colors.low }
    },
    @{ op = 'setColWidth'; id = 'sheet:Sheet2'; col = 'A'; width = 15 }
)
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $book.Replace('\', '/')
    ops = $ops
} 'xlsx-sparkline-column-color-insert' | Out-Null

$checks = @(
    @{ id = 'sheet:Sheet1/spark:1'; sheet = 1; addr = 'A1'; name = 'high-low' },
    @{ id = 'sheet:Sheet1/spark:2'; sheet = 1; addr = 'A2'; name = 'equal' },
    @{ id = 'sheet:Sheet1/spark:3'; sheet = 1; addr = 'A3'; name = 'low-last' },
    @{ id = 'sheet:Sheet1/spark:4'; sheet = 1; addr = 'A4'; name = 'markers' },
    @{ id = 'sheet:Sheet2/spark:1'; sheet = 2; addr = 'A1'; name = 'wide' }
)
$libs = @{}
foreach ($item in $checks) {
    $png = Get-OutPath "xlsx-sparkline-column-color-$($item.name)-lib.png"
    if (Test-Path $png) { Remove-Item -Force $png }
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'exportSparklineImage'
        path = $book.Replace('\', '/'); format = 'xlsx'
        id = $item.id
        output = $png.Replace('\', '/')
    } "xlsx-sparkline-column-color-$($item.name)" | Out-Null
    $libs[$item.name] = $png
}

$excel = Ensure-MsExcel $book
$wb = Open-ExcelBook $excel $book
$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
try {
    $excel.ActiveWindow.DisplayGridlines = $false
    $excel.ActiveWindow.Zoom = 100
    foreach ($item in $checks) {
        $sheets = $wb.Worksheets
        $ws = $sheets.Item([int]$item.sheet)
        Release-Com $sheets
        $ws.Activate() | Out-Null
        $excel.ActiveWindow.DisplayGridlines = $false
        $gold = Get-OutPath "xlsx-sparkline-column-color-$($item.name)-gold.png"
        Copy-Cell $ws $item.addr $gold
        Release-Com $ws
        $out = Get-OutPath "xlsx-sparkline-column-color-$($item.name)-diff.png"
        $line = & $diffScript -Left $gold -Right $libs[$item.name] -Out $out -MaxDelta 0
        $text = ($line | Out-String).Trim()
        Write-Log "xlsx-sparkline-column-color-$($item.name) $text"
        $ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
        Add-Check "xlsx-sparkline-column-color-$($item.name)" $ok $text
    }
}
finally {
    if ($wb) { $wb.Close($false) }
    Release-Com $wb
}
