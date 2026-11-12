# 查找与引用缓存对照 Excel 16.0。编号 Xlk。
# 先用无公式工作簿启动 Excel，再打开公式簿。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function Find-Cell($cells, [string]$id) {
    foreach ($c in $cells) {
        if ($c.id -eq $id) { return $c }
    }
    return $null
}

$seed = Get-OutPath 'xlsx-lookup-seed.xlsx'
$book = Get-OutPath 'xlsx-lookup.xlsx'
foreach ($p in @($seed, $book)) { if (Test-Path $p) { Remove-Item -Force $p } }

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $seed.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xlk-seed' | Out-Null

$cells = @(
    @{ ref = 'A1'; value = '产品' }, @{ ref = 'B1'; value = '金额' }
    @{ ref = 'A2'; value = '甲' }, @{ ref = 'B2'; value = 10 }
    @{ ref = 'A3'; value = '甲' }, @{ ref = 'B3'; value = 5 }
    @{ ref = 'A4'; value = '乙' }, @{ ref = 'B4'; value = 7 }
    @{ ref = 'U1'; value = 1 }, @{ ref = 'V1'; value = 'a' }
    @{ ref = 'U2'; value = 3 }, @{ ref = 'V2'; value = 'b' }
    @{ ref = 'U3'; value = 5 }, @{ ref = 'V3'; value = 'c' }
    @{ ref = 'E1'; formula = '=VLOOKUP("乙",A2:B4,2,FALSE)' }
    @{ ref = 'E2'; formula = '=VLOOKUP(4,U1:V3,2,TRUE)' }
    @{ ref = 'E3'; formula = '=MATCH("乙",A2:A4,0)' }
    @{ ref = 'E4'; formula = '=INDEX(B2:B4,3)' }
    @{ ref = 'E5'; formula = '=CHOOSE(2,"a","b","c")' }
    @{ ref = 'E6'; formula = '=ADDRESS(2,3)' }
    @{ ref = 'E7'; formula = '=SUM(OFFSET(B2,0,0,2,1))' }
    @{ ref = 'E8'; formula = '=INDIRECT("B4")' }
    @{ ref = 'E9'; formula = '=COLUMN()' }
    @{ ref = 'E10'; formula = '=VLOOKUP("丁",A2:B4,2,FALSE)' }
    @{ ref = 'E11'; formula = '=OFFSET(A1,-1,0)' }
    @{ ref = 'E12'; formula = '=GETPIVOTDATA("金额",R1,"产品","甲")' }
    @{ ref = 'E13'; formula = '=GETPIVOTDATA("金额",R1,"产品","丁")' }
    @{ ref = 'E14'; formula = '=RTD("No.Such.Server","", "x")' }
    @{ ref = 'E15'; formula = '=ECMA.VLOOKUP("乙",A2:B4,2,FALSE)' }
    @{ ref = 'J1'; formula = '=TRANSPOSE(A2:B3)'; arrayRef = 'J1:K2' }
)
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $book.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = $cells }) }
} 'xlsx-xlk-create' | Out-Null
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'; format = 'xlsx'
    path = $book.Replace('\', '/')
    ops = @(@{
        op = 'insertPivotTable'; id = 'sheet:Sheet1'
        source = @{ sheet = 'Sheet1'; ref = 'A1:B4' }
        location = 'R1'; rows = @('产品'); values = @('金额')
    })
} 'xlsx-xlk-pivot' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'; format = 'xlsx'
    path = $book.Replace('\', '/')
    scope = @{ sheet = 'Sheet1'; range = 'A1:V20' }
} 'xlsx-xlk-inspect'

$excel = Ensure-MsExcel $seed
$excel.DisplayAlerts = $false
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $book
try { $excel.CalculateFullRebuild() } catch { Write-Log "calc $($_.Exception.Message)" }
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item(1)

function Read-Shown($ws, [string]$addr) {
    $range = $ws.Range($addr)
    try { return [string]$range.Text }
    finally { Release-Com $range }
}

function Read-Number($ws, [string]$addr) {
    $range = $ws.Range($addr)
    try {
        $shown = [string]$range.Text
        if ($shown.StartsWith('#')) { return $null }
        $live = $range.Value2
        if ($null -eq $live) { return $null }
        return [double]$live
    }
    finally { Release-Com $range }
}

function Cache-Of($id) {
    return Find-Cell $tree.sheets[0].cells $id
}

function Same-Cell($ws, [string]$id, [string]$addr) {
    $c = Cache-Of $id
    if ($null -eq $c) { return "$addr missing" }
    if ($c.type -eq 'error') {
        $live = Read-Shown $ws $addr
        if ($live -eq [string]$c.value) { return $null }
        return "$addr live=$live cache=$($c.value)"
    }
    if ($c.type -eq 'text') {
        $live = Read-Shown $ws $addr
        if ($live -eq [string]$c.value) { return $null }
        return "$addr live=$live cache=$($c.value)"
    }
    $cached = [double]$c.value
    $liveNum = Read-Number $ws $addr
    if ($null -ne $liveNum -and [math]::Abs($cached - $liveNum) -lt 0.00001) { return $null }
    return "$addr live=$liveNum cache=$cached"
}

$find = @('E1','E2','E3','E4','E5')
$place = @('E6','E7','E8','E9','J1','J2')
$badFind = @($find | ForEach-Object { Same-Cell $ws "cell:Sheet1!$_" $_ } | Where-Object { $_ })
$badPlace = @($place | ForEach-Object { Same-Cell $ws "cell:Sheet1!$_" $_ } | Where-Object { $_ })
$miss = Cache-Of 'cell:Sheet1!E10'
$off = Cache-Of 'cell:Sheet1!E11'
$hit = Cache-Of 'cell:Sheet1!E12'
$gone = Cache-Of 'cell:Sheet1!E13'
$rtd = Cache-Of 'cell:Sheet1!E14'
$ecma = Cache-Of 'cell:Sheet1!E15'
$missLive = Read-Shown $ws 'E10'
$offLive = Read-Shown $ws 'E11'
$hitLive = Read-Number $ws 'E12'
$goneLive = Read-Shown $ws 'E13'
$rtdLive = Read-Shown $ws 'E14'
$ecmaLive = Read-Shown $ws 'E15'
$noRepair = $cap -notmatch 'Repair'
$errOk = ($miss.type -eq 'error') -and ($miss.value -eq '#N/A') -and ($missLive -eq '#N/A') -and
    ($off.type -eq 'error') -and ($off.value -eq '#REF!') -and ($offLive -eq '#REF!') -and
    ($gone.type -eq 'error') -and ($gone.value -eq '#REF!') -and ($goneLive -eq '#REF!') -and
    ($rtd.type -eq 'error') -and ($rtd.value -eq '#N/A') -and ($rtdLive -eq '#N/A') -and
    ($null -ne $hitLive) -and ([math]::Abs([double]$hit.value - 15) -lt 0.00001) -and
    ([math]::Abs($hitLive - 15) -lt 0.00001)
$nameOk = ($null -ne $ecma) -and ($null -eq $ecma.value) -and ($ecmaLive -eq '#NAME?')
Release-Com $ws
Close-AllWorkbooks $excel

Add-Check 'xlsx-Xlk1' ($noRepair -and $badFind.Count -eq 0) ("caption=$cap " + ($badFind -join '; '))
Add-Check 'xlsx-Xlk2' ($noRepair -and $badPlace.Count -eq 0) ($badPlace -join '; ')
Add-Check 'xlsx-Xlk3' ($noRepair -and $errOk -and $nameOk) ("miss=$missLive off=$offLive pivot=$hitLive gone=$goneLive rtd=$rtdLive ecma=$ecmaLive caption=$cap")

if ($MyInvocation.InvocationName -ne '.') {
    Close-OfficeApps -Quit
}
