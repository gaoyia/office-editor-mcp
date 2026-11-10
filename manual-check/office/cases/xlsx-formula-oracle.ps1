# 公式求值对照：库不求值。本机 Excel 算出缓存后，inspect 读到的 value 必须和 Excel 的结果一致。
# 编号 Xfo。单独运行时会关掉自己启动的 Excel。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function Find-Cell($cells, [string]$id) {
    foreach ($c in $cells) {
        if ($c.id -eq $id) { return $c }
    }
    return $null
}

function Read-LiveCell($ws, [string]$addr) {
    $range = $ws.Range($addr)
    try {
        $text = [string]$range.Text
        $value = $null
        $isError = $false
        try { $value = $range.Value2 } catch { $isError = $true }
        if ($text.StartsWith('#')) { $isError = $true }
        return @{ text = $text; value = $value; isError = $isError }
    }
    finally { Release-Com $range }
}

function Same-Number($live, $cached) {
    if ($null -eq $live -or $null -eq $cached) { return $false }
    $a = 0.0
    $b = 0.0
    if (-not [double]::TryParse([string]$live, [ref]$a)) { return $false }
    if (-not [double]::TryParse([string]$cached, [ref]$b)) { return $false }
    return [math]::Abs($a - $b) -lt 0.001
}

$book = Get-OutPath 'xlsx-formula-oracle.xlsx'
if (Test-Path $book) { Remove-Item -Force $book }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $book.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        definedNames = @(@{ name = 'Rate'; formula = '数据!$B$1' })
        sheets = @(
            @{
                name = '数据'
                cells = @(
                    @{ ref = 'A1'; value = 2 }
                    @{ ref = 'A2'; value = 11 }
                    @{ ref = 'B1'; value = 3 }
                    @{ ref = 'C1'; formula = '=A1+B1' }
                    @{ ref = 'C2'; formula = '=来源!A1' }
                    @{ ref = 'C3'; formula = '=Rate*2' }
                    @{ ref = 'D1'; formula = '=A1*2'; sharedRef = 'D1:D2' }
                    @{ ref = 'E1'; formula = '=A1:A2*10'; arrayRef = 'E1:E2' }
                    @{ ref = 'G2'; value = 10 }
                    @{ ref = 'G3'; value = 20 }
                    @{ ref = 'H1'; formula = '=SUM(QtyTable[Qty])' }
                    @{ ref = 'I1'; formula = '=1/0' }
                    @{ ref = 'J1'; formula = '=DATE(2026,1,2)' }
                    @{ ref = 'K1'; formula = '=HYPERLINK("#A1",A1)' }
                )
                tables = @(@{ ref = 'G1:G3'; name = 'QtyTable'; columns = @('Qty') })
            }
            @{
                name = '来源'
                cells = @(@{ ref = 'A1'; value = 7 })
            }
        )
    }
} 'xlsx-xfo-create' | Out-Null

$before = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $book.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = '数据'; range = 'A1:K3' }
} 'xlsx-xfo-before'
$c1before = Find-Cell $before.sheets[0].cells 'cell:数据!C1'
$xfo0 = ($null -eq $c1before.value) -and ($c1before.formula -eq '=A1+B1')
Add-Check 'xlsx-Xfo0' $xfo0 "库未求值 value=$($c1before.value) formula=$($c1before.formula)"

$excel = Ensure-MsExcel $book
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $book
try { $excel.CalculateFullRebuild() } catch { Write-Log "CalculateFullRebuild $($_.Exception.Message)" }
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item('数据')
$live = @{}
foreach ($addr in @('C1', 'C2', 'C3', 'D1', 'D2', 'E1', 'E2', 'H1', 'I1', 'J1', 'K1')) {
    $live[$addr] = Read-LiveCell $ws $addr
    Write-Log ("Excel 数据!{0} text={1} value={2}" -f $addr, $live[$addr].text, $live[$addr].value)
}
$wb.Save()
Release-Com $ws
Close-AllWorkbooks $excel

$after = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $book.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = '数据'; range = 'A1:K3' }
} 'xlsx-xfo-after'

function Want-Number([string]$id, [string]$addr, $expect) {
    $cell = Find-Cell $after.sheets[0].cells $id
    $ok = ($cap -notmatch 'Repair') -and (Same-Number $live[$addr].value $expect) -and (Same-Number $cell.value $live[$addr].value)
    Add-Check "xlsx-Xfo-$addr" $ok "excel=$($live[$addr].value) inspect=$($cell.value) expect=$expect caption=$cap"
}

Want-Number 'cell:数据!C1' 'C1' 5
Want-Number 'cell:数据!C2' 'C2' 7
Want-Number 'cell:数据!C3' 'C3' 6
Want-Number 'cell:数据!D1' 'D1' 4
Want-Number 'cell:数据!D2' 'D2' 22
Want-Number 'cell:数据!E1' 'E1' 20
Want-Number 'cell:数据!E2' 'E2' 110
Want-Number 'cell:数据!H1' 'H1' 30
Want-Number 'cell:数据!K1' 'K1' 2

$j = Find-Cell $after.sheets[0].cells 'cell:数据!J1'
$jok = ($cap -notmatch 'Repair') -and (Same-Number $j.value $live['J1'].value) -and ($null -ne $live['J1'].value)
Add-Check 'xlsx-Xfo-J1' $jok "excel=$($live['J1'].value) inspect=$($j.value) text=$($live['J1'].text)"

$i = Find-Cell $after.sheets[0].cells 'cell:数据!I1'
$iok = ($cap -notmatch 'Repair') -and $live['I1'].isError -and ($i.type -eq 'error') -and ($i.value -eq '#DIV/0!')
Add-Check 'xlsx-Xfo-I1' $iok "excelText=$($live['I1'].text) inspect=$($i.type):$($i.value)"

$refBook = Get-OutPath 'xlsx-formula-oracle-ref.xlsx'
if (Test-Path $refBook) { Remove-Item -Force $refBook }
$refHost = Get-OutPath 'xlsx-formula-oracle-ref-host.xlsx'
if (Test-Path $refHost) { Remove-Item -Force $refHost }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $refHost.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(
            @{ name = '留'; cells = @(@{ ref = 'A1'; formula = '=删!A1' }) }
            @{ name = '删'; cells = @(@{ ref = 'A1'; value = 9 }) }
        )
    }
} 'xlsx-xfo-ref-host' | Out-Null
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $refHost.Replace('\', '/')
    output = $refBook.Replace('\', '/')
    ops = @(@{ op = 'deleteSheet'; id = 'sheet:删' })
} 'xlsx-xfo-ref-del' | Out-Null

$excel2 = Ensure-MsExcel $refBook
Close-AllWorkbooks $excel2
$wb2 = Open-ExcelBook $excel2 $refBook
try { $excel2.CalculateFullRebuild() } catch { Write-Log "CalculateFullRebuild ref $($_.Exception.Message)" }
$cap2 = [string]$wb2.Windows.Item(1).Caption
$ws2 = $wb2.Worksheets.Item('留')
$refLive = Read-LiveCell $ws2 'A1'
Write-Log "Excel 留!A1 text=$($refLive.text) value=$($refLive.value)"
$wb2.Save()
Release-Com $ws2
Close-AllWorkbooks $excel2

$refTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $refBook.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = '留'; range = 'A1' }
} 'xlsx-xfo-ref-inspect'
$refCell = Find-Cell $refTree.sheets[0].cells 'cell:留!A1'
$refOk = ($cap2 -notmatch 'Repair') -and $refLive.isError -and ($refCell.type -eq 'error') -and ($refCell.value -eq '#REF!')
Add-Check 'xlsx-Xfo-REF' $refOk "excelText=$($refLive.text) formula=$($refCell.formula) inspect=$($refCell.type):$($refCell.value)"

if ($MyInvocation.InvocationName -ne '.') {
    Close-OfficeApps -Quit
}
