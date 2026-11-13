# xlsx 图表三维引用：docs/checklist-xlsx.md 的 Xc8。
# 库把系列公式改成两张表，再改格子。打开前先看缓存，再交给 Excel。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function Set-ChartFormula([string]$path) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [System.IO.Compression.ZipFile]::Open($path, 'Update')
    try {
        $entry = $zip.GetEntry('xl/charts/chart1.xml')
        $reader = New-Object System.IO.StreamReader($entry.Open())
        $text = $reader.ReadToEnd()
        $reader.Dispose()
        $text = $text.Replace('Sheet1!$B$11', 'Sheet1:Sheet2!$B$11')
        $entry.Delete()
        $neu = $zip.CreateEntry('xl/charts/chart1.xml')
        $writer = New-Object System.IO.StreamWriter($neu.Open())
        $writer.Write($text)
        $writer.Dispose()
    }
    finally { $zip.Dispose() }
}

function Read-ChartXml([string]$path) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [System.IO.Compression.ZipFile]::OpenRead($path)
    try {
        $entry = $zip.GetEntry('xl/charts/chart1.xml')
        $reader = New-Object System.IO.StreamReader($entry.Open())
        $text = $reader.ReadToEnd()
        $reader.Dispose()
        return $text
    }
    finally { $zip.Dispose() }
}

$src = Get-OutPath 'xlsx-chart-3d-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xc8-create' | Out-Null

$charted = Get-OutPath 'xlsx-chart-3d-chart.xlsx'
if (Test-Path $charted) { Remove-Item -Force $charted }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $charted.Replace('\', '/')
    ops = @(
        @{ op = 'insertSheet'; id = 'xlsx:workbook'; name = 'Sheet2' },
        @{
            op = 'insertChart'; id = 'sheet:Sheet1'; type = 'col'; from = 'E2'; dataFrom = 'A10'
            series = @(@{ name = 'Q1'; categories = @('甲'); values = @(1) })
        }
    )
} 'xlsx-xc8-chart' | Out-Null

Set-ChartFormula $charted

$out = Get-OutPath 'xlsx-chart-3d.xlsx'
if (Test-Path $out) { Remove-Item -Force $out }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $charted.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(
        @{ op = 'setCell'; id = 'cell:Sheet1!B11'; value = 4 },
        @{ op = 'setCell'; id = 'cell:Sheet2!B11'; value = 7 }
    )
} 'xlsx-xc8-set' | Out-Null

$xml = Read-ChartXml $out
$cacheOk = ($xml -match 'Sheet1:Sheet2!\$B\$11') -and ($xml -match '<c:pt idx="0"><c:v>4</c:v></c:pt>') -and ($xml -match '<c:pt idx="1"><c:v>7</c:v></c:pt>')

$excel = Ensure-MsExcel $out
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $out
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item(1)
$chart = $ws.ChartObjects(1).Chart
$ser = $chart.SeriesCollection(1)
$vals = @($ser.Values)
$v0 = [double]$vals[0]
$v1 = [double]$vals[1]
Release-Com $ser
Release-Com $chart
Close-AllWorkbooks $excel

$ok = $cacheOk -and ($cap -notmatch 'Repair') -and ($v0 -eq 4) -and ($v1 -eq 7)
Add-Check 'xlsx-Xc8' $ok "cache=$cacheOk caption=$cap v0=$v0 v1=$v1"
