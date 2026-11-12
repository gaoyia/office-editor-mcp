# 文本函数缓存对照 Excel 16.0。编号 Xtx。
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

$seed = Get-OutPath 'xlsx-text-seed.xlsx'
$book = Get-OutPath 'xlsx-text.xlsx'
foreach ($p in @($seed, $book)) { if (Test-Path $p) { Remove-Item -Force $p } }

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $seed.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xtx-seed' | Out-Null

$cells = @(
    @{ ref = 'A1'; value = 'AbC 中文' }
    @{ ref = 'A2'; value = '  hi  ' }
    @{ ref = 'A3'; value = 1234.5 }
    @{ ref = 'A4'; value = 'ＡＢ１２' }
    @{ ref = 'C1'; formula = '=LEFT(A1,3)' }
    @{ ref = 'C2'; formula = '=RIGHT(A1,2)' }
    @{ ref = 'C3'; formula = '=MID(A1,4,2)' }
    @{ ref = 'C4'; formula = '=LOWER(A1)' }
    @{ ref = 'C7'; formula = '=TRIM(A2)' }
    @{ ref = 'C9'; formula = '=CONCATENATE(A1,"!",1)' }
    @{ ref = 'C11'; formula = '=T(A3)' }
    @{ ref = 'C13'; formula = '=TEXT(A3,"0.00")' }
    @{ ref = 'C15'; formula = '=DOLLAR(A3)' }
    @{ ref = 'C16'; formula = '=DOLLAR(-1234.5)' }
    @{ ref = 'C18'; formula = '=FIXED(A3,1,TRUE)' }
    @{ ref = 'C19'; formula = '=SUBSTITUTE(A1,"中文","文")' }
    @{ ref = 'C22'; formula = '=ASC(A4)' }
    @{ ref = 'C23'; formula = '=BAHTTEXT(1234)' }
    @{ ref = 'C27'; formula = '=PHONETIC(A1)' }
    @{ ref = 'C28'; formula = '=CLEAN("hi"&CHAR(7)&"x")' }
    @{ ref = 'C29'; formula = '=LEN(A1)' }
    @{ ref = 'C32'; formula = '=SEARCH("中?",A1)' }
    @{ ref = 'C34'; formula = '=VALUE("1,234.5")' }
    @{ ref = 'C36'; formula = '=FIND("c",A1)' }
    @{ ref = 'C37'; formula = '=EXACT("Ab","ab")' }
    @{ ref = 'C38'; formula = '=ECMA.LEFT(A1,1)' }
    @{ ref = 'C40'; formula = '=LEFTB(A1,5)' }
    @{ ref = 'C41'; formula = '=RIGHTB(A1,3)' }
    @{ ref = 'C42'; formula = '=MIDB(A1,4,2)' }
    @{ ref = 'C44'; formula = '=REPLACEB(A1,4,2,"Z")' }
    @{ ref = 'C46'; formula = '=CHAR(54992)' }
    @{ ref = 'C47'; formula = '=LENB(A1)' }
    @{ ref = 'C51'; formula = '=CODE("中")' }
)
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $book.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = $cells }) }
} 'xlsx-xtx-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'; format = 'xlsx'
    path = $book.Replace('\', '/')
    scope = @{ sheet = 'Sheet1'; range = 'A1:C51' }
} 'xlsx-xtx-inspect'

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

function Read-Value($ws, [string]$addr) {
    $range = $ws.Range($addr)
    try {
        $shown = [string]$range.Text
        if ($shown.StartsWith('#')) { return $shown }
        $live = $range.Value2
        if ($null -eq $live) { return $shown }
        return [string]$live
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
    if ($c.type -eq 'boolean') {
        $live = (Read-Shown $ws $addr).ToUpperInvariant()
        $want = if ($c.value) { 'TRUE' } else { 'FALSE' }
        if ($live -eq $want) { return $null }
        return "$addr live=$live cache=$want"
    }
    if ($c.type -eq 'text') {
        $live = Read-Value $ws $addr
        if ($live -eq [string]$c.value) { return $null }
        return "$addr live=[$live] cache=[$($c.value)]"
    }
    $cached = [double]$c.value
    $liveNum = 0.0
    $live = Read-Shown $ws $addr
    $parsed = [double]::TryParse(
        $live,
        [Globalization.NumberStyles]::Float,
        [Globalization.CultureInfo]::InvariantCulture,
        [ref]$liveNum
    )
    if (-not $parsed) {
        $parsed = [double]::TryParse($live, [ref]$liveNum)
    }
    if ($parsed -and [math]::Abs($cached - $liveNum) -lt 0.00001) { return $null }
    return "$addr live=$live cache=$cached"
}

$slice = @('C1','C2','C3','C40','C41','C42','C44')
$change = @('C4','C7','C9','C11','C28','C29','C32','C34','C36','C37','C46','C47','C51')
$format = @('C13','C15','C16','C18','C19','C22','C23','C27')
$badSlice = @($slice | ForEach-Object { Same-Cell $ws "cell:Sheet1!$_" $_ } | Where-Object { $_ })
$badChange = @($change | ForEach-Object { Same-Cell $ws "cell:Sheet1!$_" $_ } | Where-Object { $_ })
$badFormat = @($format | ForEach-Object { Same-Cell $ws "cell:Sheet1!$_" $_ } | Where-Object { $_ })
$ecma = Cache-Of 'cell:Sheet1!C38'
$ecmaLive = Read-Shown $ws 'C38'
$noRepair = $cap -notmatch 'Repair'
Release-Com $ws
Close-AllWorkbooks $excel

Add-Check 'xlsx-Xtx1' ($noRepair -and $badSlice.Count -eq 0) ("caption=$cap " + ($badSlice -join '; '))
Add-Check 'xlsx-Xtx2' ($noRepair -and $badChange.Count -eq 0) ($badChange -join '; ')
$nameOk = ($null -ne $ecma) -and ($null -eq $ecma.value) -and ($ecmaLive -eq '#NAME?')
Add-Check 'xlsx-Xtx3' ($noRepair -and $badFormat.Count -eq 0 -and $nameOk) ("format=$($badFormat -join '; ') ecma=$ecmaLive caption=$cap")

if ($MyInvocation.InvocationName -ne '.') {
    Close-OfficeApps -Quit
}
