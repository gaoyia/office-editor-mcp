# 计算链。依赖排得全时库自己写出 xl/calcChain.xml。排不全则没有这条链。
# 编号 Xcc。单独运行时会关掉自己启动的 Excel。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function Part-Text([string]$path, [string]$name) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($path)
    try {
        $entry = $zip.GetEntry($name)
        if (-not $entry) { return $null }
        $reader = New-Object IO.StreamReader($entry.Open())
        try { return $reader.ReadToEnd() } finally { $reader.Dispose() }
    }
    finally { $zip.Dispose() }
}

function Read-LiveCell($ws, [string]$addr) {
    $range = $ws.Range($addr)
    try {
        $value = $null
        try { $value = $range.Value2 } catch { $value = $null }
        return $value
    }
    finally { Release-Com $range }
}

$seed = Get-OutPath 'xlsx-calc-chain-seed.xlsx'
$simple = Get-OutPath 'xlsx-calc-chain.xlsx'
$two = Get-OutPath 'xlsx-calc-chain-two.xlsx'
$indirect = Get-OutPath 'xlsx-calc-chain-indirect.xlsx'
foreach ($p in @($seed, $simple, $two, $indirect)) {
    if (Test-Path $p) { Remove-Item -Force $p }
}

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $seed.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xcc-seed' | Out-Null

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $simple.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{
        name = 'Sheet1'
        cells = @(
            @{ ref = 'A1'; value = 1 }
            @{ ref = 'B1'; formula = '=A1+1' }
            @{ ref = 'C1'; formula = '=B1*2' }
        )
    }) }
} 'xlsx-xcc-simple' | Out-Null

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $two.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(
        @{ name = 'Sheet1'; cells = @(
            @{ ref = 'A1'; value = 1 }
            @{ ref = 'B1'; formula = '=A1+1' }
        ) }
        @{ name = 'Sheet2'; cells = @(
            @{ ref = 'A1'; formula = '=Sheet1!B1+1' }
        ) }
    ) }
} 'xlsx-xcc-two' | Out-Null

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $indirect.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{
        name = 'Sheet1'
        cells = @(
            @{ ref = 'A1'; value = 1 }
            @{ ref = 'B1'; formula = '=INDIRECT("A1")' }
        )
    }) }
} 'xlsx-xcc-indirect' | Out-Null

$simpleXml = Part-Text $simple 'xl/calcChain.xml'
$simpleSheet = Part-Text $simple 'xl/worksheets/sheet1.xml'
$twoXml = Part-Text $two 'xl/calcChain.xml'
$indirectXml = Part-Text $indirect 'xl/calcChain.xml'
Add-Check 'xlsx-Xcc-simple-xml' (
    ($simpleXml -match 'r="B1"') -and ($simpleXml -match 'r="C1"[^>]*s="1"') -and
    ($simpleSheet -match '<v>4</v>') -and ($null -eq $indirectXml)
) "simple=$simpleXml indirect=$indirectXml"

$excel = Ensure-MsExcel $seed
$excel.DisplayAlerts = $false
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $simple
try { $excel.CalculateFullRebuild() } catch { Write-Log "calc $($_.Exception.Message)" }
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item('Sheet1')
$b1 = Read-LiveCell $ws 'B1'
$c1 = Read-LiveCell $ws 'C1'
Release-Com $ws
Close-AllWorkbooks $excel
Add-Check 'xlsx-Xcc-simple-open' (($cap -notmatch 'Repair') -and ($b1 -eq 2) -and ($c1 -eq 4)) "B1=$b1 C1=$c1 caption=$cap"

Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $two
try { $excel.CalculateFullRebuild() } catch { Write-Log "calc two $($_.Exception.Message)" }
$cap2 = [string]$wb.Windows.Item(1).Caption
$ws2 = $wb.Worksheets.Item('Sheet2')
$a1 = Read-LiveCell $ws2 'A1'
Release-Com $ws2
Close-AllWorkbooks $excel
Add-Check 'xlsx-Xcc-two' (($cap2 -notmatch 'Repair') -and ($a1 -eq 3) -and ($twoXml -match 'i="2"')) "A1=$a1 caption=$cap2 chain=$twoXml"

Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $indirect
try { $excel.CalculateFullRebuild() } catch { Write-Log "calc indirect $($_.Exception.Message)" }
$cap3 = [string]$wb.Windows.Item(1).Caption
$ws3 = $wb.Worksheets.Item('Sheet1')
$ib = Read-LiveCell $ws3 'B1'
Release-Com $ws3
Close-AllWorkbooks $excel
Add-Check 'xlsx-Xcc-indirect' (($cap3 -notmatch 'Repair') -and ($ib -eq 1)) "B1=$ib caption=$cap3"

if ($MyInvocation.InvocationName -ne '.') {
    Close-OfficeApps -Quit
}
