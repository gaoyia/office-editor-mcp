# 信息函数缓存对照 Excel 16.0。编号 Xinf。
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

function Read-LiveText($ws, [string]$addr) {
    $range = $ws.Range($addr)
    try { return [string]$range.Text }
    finally { Release-Com $range }
}

$seed = Get-OutPath 'xlsx-info-seed.xlsx'
$book = Get-OutPath 'xlsx-info.xlsx'
foreach ($p in @($seed, $book)) { if (Test-Path $p) { Remove-Item -Force $p } }

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $seed.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xinf-seed' | Out-Null

$cells = @(
    @{ ref = 'A2'; value = 0 }
    @{ ref = 'A4'; value = 'hi' }
    @{ ref = 'A5'; value = $true }
    @{ ref = 'A6'; formula = '=1/0' }
    @{ ref = 'A7'; value = 8 }
    @{ ref = 'A8'; value = 2.9 }
    @{ ref = 'B1'; formula = '=ISBLANK(A1:A3)' }
    @{ ref = 'B2'; formula = '=ISBLANK(A2)' }
    @{ ref = 'B6'; formula = '=ISERR(A6)' }
    @{ ref = 'B7'; formula = '=ISNUMBER(A7)' }
    @{ ref = 'B8'; formula = '=ISEVEN(A8)' }
    @{ ref = 'B14'; formula = '=ISODD(A1)' }
    @{ ref = 'B19'; formula = '=ISEVEN(A5)' }
    @{ ref = 'B26'; formula = '=ISREF(A1)' }
    @{ ref = 'B28'; formula = '=ISREF(#REF!)' }
    @{ ref = 'B29'; formula = '=ISREF(Sheet1:Sheet2!A1)' }
    @{ ref = 'B30'; formula = '=ISREF(Sheet2!A1)' }
    @{ ref = 'C2'; formula = '=N(A7:A8)' }
    @{ ref = 'C7'; formula = '=N(A5)' }
    @{ ref = 'C10'; formula = '=N({12.5,13.6})' }
    @{ ref = 'C14'; formula = '=ERROR.TYPE(A6)' }
    @{ ref = 'C15'; formula = '=ERROR.TYPE(1)' }
    @{ ref = 'D1'; formula = '=CELL("address",A4)' }
    @{ ref = 'D3'; formula = '=CELL("col",A4)' }
    @{ ref = 'D4'; formula = '=CELL("row",A8)' }
    @{ ref = 'D5'; formula = '=CELL("contents",A4)' }
    @{ ref = 'D6'; formula = '=CELL("contents",A5)' }
    @{ ref = 'D8'; formula = '=CELL("contents",A1)' }
    @{ ref = 'D9'; formula = '=CELL("type",A1)' }
    @{ ref = 'D10'; formula = '=CELL("type",A4)' }
    @{ ref = 'D11'; formula = '=CELL("type",A7)' }
    @{ ref = 'D15'; formula = '=INFO("system")' }
    @{ ref = 'D16'; formula = '=INFO("recalc")' }
    @{ ref = 'D17'; formula = '=INFO("release")' }
    @{ ref = 'D18'; formula = '=INFO("numfile")' }
    @{ ref = 'D19'; formula = '=INFO("origin")' }
    @{ ref = 'D20'; formula = '=INFO("nope")' }
    @{ ref = 'D22'; formula = '=ECMA.ISBLANK(A1)' }
)
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $book.Replace('\', '/')
    workbook = @{
        sheets = @(
            @{ name = 'Sheet1'; cells = $cells }
            @{ name = 'Sheet2'; cells = @(@{ ref = 'A1'; value = 9 }) }
        )
    }
} 'xlsx-xinf-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'; format = 'xlsx'
    path = $book.Replace('\', '/')
    scope = @{ sheet = 'Sheet1'; range = 'A1:D30' }
} 'xlsx-xinf-inspect'

$excel = Ensure-MsExcel $seed
$excel.DisplayAlerts = $false
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $book
try { $excel.CalculateFullRebuild() } catch { Write-Log "calc $($_.Exception.Message)" }
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item(1)

function Cache-Text($id) {
    $c = Find-Cell $tree.sheets[0].cells $id
    if ($null -eq $c -or $null -eq $c.value) { return '' }
    if ($c.type -eq 'boolean') { if ($c.value) { return 'TRUE' } else { return 'FALSE' } }
    return [string]$c.value
}

$expect = @(
    @{ id = 'cell:Sheet1!B1'; addr = 'B1'; text = 'TRUE' }
    @{ id = 'cell:Sheet1!B2'; addr = 'B2'; text = 'FALSE' }
    @{ id = 'cell:Sheet1!B6'; addr = 'B6'; text = 'TRUE' }
    @{ id = 'cell:Sheet1!B7'; addr = 'B7'; text = 'TRUE' }
    @{ id = 'cell:Sheet1!B8'; addr = 'B8'; text = 'TRUE' }
    @{ id = 'cell:Sheet1!B14'; addr = 'B14'; text = 'FALSE' }
    @{ id = 'cell:Sheet1!B19'; addr = 'B19'; text = '#VALUE!' }
    @{ id = 'cell:Sheet1!B26'; addr = 'B26'; text = 'TRUE' }
    @{ id = 'cell:Sheet1!B28'; addr = 'B28'; text = 'FALSE' }
    @{ id = 'cell:Sheet1!B29'; addr = 'B29'; text = 'FALSE' }
    @{ id = 'cell:Sheet1!B30'; addr = 'B30'; text = 'TRUE' }
    @{ id = 'cell:Sheet1!C2'; addr = 'C2'; text = '8' }
    @{ id = 'cell:Sheet1!C7'; addr = 'C7'; text = '1' }
    @{ id = 'cell:Sheet1!C10'; addr = 'C10'; text = '12.5' }
    @{ id = 'cell:Sheet1!C14'; addr = 'C14'; text = '2' }
    @{ id = 'cell:Sheet1!C15'; addr = 'C15'; text = '#N/A' }
    @{ id = 'cell:Sheet1!D1'; addr = 'D1'; text = '$A$4' }
    @{ id = 'cell:Sheet1!D3'; addr = 'D3'; text = '1' }
    @{ id = 'cell:Sheet1!D4'; addr = 'D4'; text = '8' }
    @{ id = 'cell:Sheet1!D5'; addr = 'D5'; text = 'hi' }
    @{ id = 'cell:Sheet1!D6'; addr = 'D6'; text = 'TRUE' }
    @{ id = 'cell:Sheet1!D8'; addr = 'D8'; text = '0' }
    @{ id = 'cell:Sheet1!D9'; addr = 'D9'; text = 'b' }
    @{ id = 'cell:Sheet1!D10'; addr = 'D10'; text = 'l' }
    @{ id = 'cell:Sheet1!D11'; addr = 'D11'; text = 'v' }
    @{ id = 'cell:Sheet1!D15'; addr = 'D15'; text = 'pcdos' }
    @{ id = 'cell:Sheet1!D17'; addr = 'D17'; text = '16.0' }
    @{ id = 'cell:Sheet1!D18'; addr = 'D18'; text = '2' }
    @{ id = 'cell:Sheet1!D19'; addr = 'D19'; text = '$A:$A$1' }
    @{ id = 'cell:Sheet1!D20'; addr = 'D20'; text = '#VALUE!' }
)

$bad = @()
foreach ($row in $expect) {
    $live = Read-LiveText $ws $row.addr
    $cached = Cache-Text $row.id
    if ($live -ne $row.text -or $cached -ne $row.text) {
        $bad += "$($row.addr) live=$live cache=$cached expect=$($row.text)"
    }
}
$recalcCache = Cache-Text 'cell:Sheet1!D16'
$recalcLive = Read-LiveText $ws 'D16'
$ecma = Find-Cell $tree.sheets[0].cells 'cell:Sheet1!D22'
$ecmaLive = Read-LiveText $ws 'D22'
$noRepair = $cap -notmatch 'Repair'
Release-Com $ws
Close-AllWorkbooks $excel

$isRows = @($bad | Where-Object { $_ -match '^(B|C)' })
$cellRows = @($bad | Where-Object { $_ -match '^D(1|3|4|5|6|8|9|10|11) ' })
$infoRows = @($bad | Where-Object { $_ -match '^D(15|17|18|19|20) ' })
Add-Check 'xlsx-Xinf1' ($noRepair -and $isRows.Count -eq 0) ("caption=$cap " + ($isRows -join '; '))
Add-Check 'xlsx-Xinf2' ($noRepair -and $cellRows.Count -eq 0) ($cellRows -join '; ')
$infoOk = $noRepair -and ($infoRows.Count -eq 0) -and ($recalcCache -eq 'Automatic') -and ($recalcLive -eq '自动')
$infoOk = $infoOk -and ($null -ne $ecma) -and ($null -eq $ecma.value) -and ($ecmaLive -eq '#NAME?')
Add-Check 'xlsx-Xinf3' $infoOk "cacheRecalc=$recalcCache liveRecalc=$recalcLive ecma=$ecmaLive info=$($infoRows -join '; ') caption=$cap"

if ($MyInvocation.InvocationName -ne '.') {
    Close-OfficeApps -Quit
}
