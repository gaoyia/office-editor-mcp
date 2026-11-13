# xlsx 图表系列三维与定义名：docs/checklist-xlsx.md 的 Xc10、Xc11。
# setChartSeries 把新值写进三维引用和 OFFSET 定义名落到的格子，公式文本留下，缓存跟着变。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function Set-ChartFormula([string]$path, [string]$from, [string]$to) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [System.IO.Compression.ZipFile]::Open($path, 'Update')
    try {
        $entry = $zip.GetEntry('xl/charts/chart1.xml')
        $reader = New-Object System.IO.StreamReader($entry.Open())
        $text = $reader.ReadToEnd()
        $reader.Dispose()
        $text = $text.Replace($from, $to)
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

function Read-SeriesValues($path) {
    $excel = Ensure-MsExcel $path
    Close-AllWorkbooks $excel
    $wb = Open-ExcelBook $excel $path
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
    return @{ caption = $cap; v0 = $v0; v1 = $v1 }
}

$src = Get-OutPath 'xlsx-chart-series-3d-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xc10-create' | Out-Null

$charted = Get-OutPath 'xlsx-chart-series-3d-chart.xlsx'
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
} 'xlsx-xc10-chart' | Out-Null
Set-ChartFormula $charted 'Sheet1!$B$11' 'Sheet1:Sheet2!$B$11'

$out = Get-OutPath 'xlsx-chart-series-3d.xlsx'
if (Test-Path $out) { Remove-Item -Force $out }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $charted.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(@{
        op = 'setChartSeries'; id = 'sheet:Sheet1/chart:1/series:0'; values = @(8, 9)
    })
} 'xlsx-xc10-set' | Out-Null

$xml = Read-ChartXml $out
$cacheOk = ($xml -match 'Sheet1:Sheet2!\$B\$11') -and ($xml -match '<c:v>8</c:v>') -and ($xml -match '<c:v>9</c:v>')
$seen = Read-SeriesValues $out
$ok = $cacheOk -and ($seen.caption -notmatch 'Repair') -and ($seen.v0 -eq 8) -and ($seen.v1 -eq 9)
Add-Check 'xlsx-Xc10' $ok "cache=$cacheOk caption=$($seen.caption) v0=$($seen.v0) v1=$($seen.v1)"

$src2 = Get-OutPath 'xlsx-chart-series-name-src.xlsx'
if (Test-Path $src2) { Remove-Item -Force $src2 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src2.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xc11-create' | Out-Null

$charted2 = Get-OutPath 'xlsx-chart-series-name-chart.xlsx'
if (Test-Path $charted2) { Remove-Item -Force $charted2 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src2.Replace('\', '/')
    output = $charted2.Replace('\', '/')
    ops = @(
        @{
            op = 'insertDefinedName'; id = 'xlsx:workbook'; name = 'Dyn'
            formula = 'OFFSET(Sheet1!$B$11,0,0,2,1)'
        },
        @{
            op = 'insertChart'; id = 'sheet:Sheet1'; type = 'col'; from = 'E2'; dataFrom = 'A10'
            series = @(@{ name = 'Q1'; categories = @('甲'); values = @(1) })
        }
    )
} 'xlsx-xc11-chart' | Out-Null
Set-ChartFormula $charted2 'Sheet1!$B$11:$B$11' 'Dyn'

$out2 = Get-OutPath 'xlsx-chart-series-name.xlsx'
if (Test-Path $out2) { Remove-Item -Force $out2 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $charted2.Replace('\', '/')
    output = $out2.Replace('\', '/')
    ops = @(@{
        op = 'setChartSeries'; id = 'sheet:Sheet1/chart:1/series:0'; values = @(1, 2)
    })
} 'xlsx-xc11-set' | Out-Null

$xml2 = Read-ChartXml $out2
$cache2 = ($xml2 -match '>Dyn<') -and ($xml2 -match '<c:v>1</c:v>') -and ($xml2 -match '<c:v>2</c:v>')
$seen2 = Read-SeriesValues $out2
$ok2 = $cache2 -and ($seen2.caption -notmatch 'Repair') -and ($seen2.v0 -eq 1) -and ($seen2.v1 -eq 2)
Add-Check 'xlsx-Xc11' $ok2 "cache=$cache2 caption=$($seen2.caption) v0=$($seen2.v0) v1=$($seen2.v1)"
