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
        definedNames = @(
            @{ name = 'Rate'; formula = '数据!$B$1' }
            @{ name = 'LocalAdd'; formula = '2'; localSheet = '数据' }
        )
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
                    @{ ref = 'J1'; formula = '=DATE(2026,1,2)'; numFmt = 'yyyy-mm-dd' }
                    @{ ref = 'K1'; formula = '=HYPERLINK("#A1",A1)' }
                    @{ ref = 'L1'; formula = '=SUM(A1,B1)' }
                    @{ ref = 'M1'; formula = '=LocalAdd' }
                    @{ ref = 'N1'; formula = '=TRUE' }
                    @{ ref = 'O1'; formula = '=NoSuchName' }
                    @{ ref = 'P1'; formula = '="abc"+1' }
                    @{ ref = 'Q1'; formula = '=NA()' }
                    @{ ref = 'R1'; formula = '=SQRT(-1)' }
                    @{ ref = 'S1'; formula = '=NOW()' }
                    @{ ref = 'T1'; formula = '=1/3'; numFmt = '0.00' }
                    @{ ref = 'U1'; formula = '=B1%' }
                    @{ ref = 'V1'; formula = '=A1>B1' }
                    @{ ref = 'W1'; formula = '=A1&"x"' }
                    @{ ref = 'X1'; formula = '=SUM((A1,B1))' }
                    @{ ref = 'Y1'; formula = '=A1:B2 B1:C1' }
                    @{ ref = 'Z1'; formula = '=A1:A2 B1:B2' }
                    @{ ref = 'AA1'; formula = '=SUM(数据:来源!A1)' }
                    @{ ref = 'AB1'; formula = '=SUM({1,2,3})' }
                    @{ ref = 'AC1'; formula = '=ABS(A1:A2)' }
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
    scope = @{ sheet = '数据'; range = 'A1:AC3' }
} 'xlsx-xfo-before'
$c1before = Find-Cell $before.sheets[0].cells 'cell:数据!C1'
$xfo0 = (Same-Number $c1before.value 5) -and ($c1before.formula -eq '=A1+B1')
Add-Check 'xlsx-Xfo0' $xfo0 "库已写入缓存 value=$($c1before.value) formula=$($c1before.formula)"

$excel = Ensure-MsExcel $book
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $book
try { $excel.CalculateFullRebuild() } catch { Write-Log "CalculateFullRebuild $($_.Exception.Message)" }
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item('数据')
$excel.ReferenceStyle = -4150
$r1c1 = [string]$ws.Range('C1').FormulaR1C1
$excel.ReferenceStyle = 1
Write-Log "C1 FormulaR1C1=$r1c1"
$wb.Save()
$live = @{}
foreach ($addr in @('C1', 'C2', 'C3', 'D1', 'D2', 'E1', 'E2', 'H1', 'I1', 'J1', 'K1', 'L1', 'M1', 'N1', 'O1', 'P1', 'Q1', 'R1', 'S1', 'T1', 'U1', 'V1', 'W1', 'X1', 'Y1', 'Z1', 'AA1', 'AB1', 'AC1')) {
    $live[$addr] = Read-LiveCell $ws $addr
    Write-Log ("Excel 数据!{0} text={1} value={2}" -f $addr, $live[$addr].text, $live[$addr].value)
}
Release-Com $ws
Close-AllWorkbooks $excel

$after = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $book.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = '数据'; range = 'A1:AC3' }
} 'xlsx-xfo-after'

function Want-Number([string]$id, [string]$addr, $expect) {
    $cell = Find-Cell $after.sheets[0].cells $id
    $earlier = Find-Cell $before.sheets[0].cells $id
    $ok = ($cap -notmatch 'Repair') -and (Same-Number $live[$addr].value $expect) -and (Same-Number $cell.value $live[$addr].value) -and (Same-Number $earlier.value $expect)
    Add-Check "xlsx-Xfo-$addr" $ok "excel=$($live[$addr].value) inspect=$($cell.value) library=$($earlier.value) expect=$expect caption=$cap"
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
Want-Number 'cell:数据!L1' 'L1' 5
Want-Number 'cell:数据!M1' 'M1' 2
Want-Number 'cell:数据!U1' 'U1' 0.03
Want-Number 'cell:数据!X1' 'X1' 5
Want-Number 'cell:数据!Y1' 'Y1' 3
Want-Number 'cell:数据!AA1' 'AA1' 9
Want-Number 'cell:数据!AB1' 'AB1' 6

function Want-Flag([string]$id, [string]$addr, [bool]$expect) {
    $cell = Find-Cell $after.sheets[0].cells $id
    $earlier = Find-Cell $before.sheets[0].cells $id
    $excelFlag = [System.Convert]::ToBoolean($live[$addr].value)
    $inspectFlag = [System.Convert]::ToBoolean($cell.value)
    $libraryFlag = [System.Convert]::ToBoolean($earlier.value)
    $ok = ($cap -notmatch 'Repair') -and ($excelFlag -eq $expect) -and ($inspectFlag -eq $excelFlag) -and ($libraryFlag -eq $expect) -and ($cell.type -eq 'boolean')
    Add-Check "xlsx-Xfo-$addr" $ok "excel=$($live[$addr].value) inspect=$($cell.type):$($cell.value) library=$($earlier.value) expect=$expect"
}

Want-Flag 'cell:数据!N1' 'N1' $true
Want-Flag 'cell:数据!V1' 'V1' $false

function Want-Text([string]$id, [string]$addr, [string]$expect) {
    $cell = Find-Cell $after.sheets[0].cells $id
    $earlier = Find-Cell $before.sheets[0].cells $id
    $ok = ($cap -notmatch 'Repair') -and ([string]$live[$addr].value -eq $expect) -and ([string]$cell.value -eq $expect) -and ([string]$earlier.value -eq $expect) -and ($cell.type -eq 'text')
    Add-Check "xlsx-Xfo-$addr" $ok "excel=$($live[$addr].value) inspect=$($cell.type):$($cell.value) library=$($earlier.value) expect=$expect"
}

Want-Text 'cell:数据!W1' 'W1' '2x'

function Want-Error([string]$id, [string]$addr, [string]$token) {
    $cell = Find-Cell $after.sheets[0].cells $id
    $earlier = Find-Cell $before.sheets[0].cells $id
    $ok = ($cap -notmatch 'Repair') -and ($live[$addr].text -eq $token) -and ($cell.type -eq 'error') -and ($cell.value -eq $token) -and ($earlier.type -eq 'error') -and ($earlier.value -eq $token)
    Add-Check "xlsx-Xfo-$addr" $ok "excelText=$($live[$addr].text) inspect=$($cell.type):$($cell.value) library=$($earlier.value) expect=$token"
}

Want-Error 'cell:数据!O1' 'O1' '#NAME?'
Want-Error 'cell:数据!P1' 'P1' '#VALUE!'
Want-Error 'cell:数据!Q1' 'Q1' '#N/A'
Want-Error 'cell:数据!R1' 'R1' '#NUM!'
Want-Error 'cell:数据!Z1' 'Z1' '#NULL!'

$s = Find-Cell $after.sheets[0].cells 'cell:数据!S1'
$sok = ($cap -notmatch 'Repair') -and (Same-Number $s.value $live['S1'].value) -and ($s.type -eq 'number')
Add-Check 'xlsx-Xfo-S1' $sok "excel=$($live['S1'].value) inspect=$($s.value) text=$($live['S1'].text)"

$t = Find-Cell $after.sheets[0].cells 'cell:数据!T1'
$tok = ($cap -notmatch 'Repair') -and (Same-Number $t.value $live['T1'].value) -and ($live['T1'].text -ne [string]$live['T1'].value) -and ($t.type -eq 'number')
Add-Check 'xlsx-Xfo-T1' $tok "excel=$($live['T1'].value) text=$($live['T1'].text) inspect=$($t.value) numFmt=$($t.numFmt)"

Want-Number 'cell:数据!AC1' 'AC1' 2

$c1f = Find-Cell $after.sheets[0].cells 'cell:数据!C1'
$rok = ($cap -notmatch 'Repair') -and ($r1c1 -eq '=RC[-2]+RC[-1]') -and ($c1f.formula -eq '=A1+B1')
Add-Check 'xlsx-Xfo-R1C1' $rok "r1c1=$r1c1 formula=$($c1f.formula)"

$j = Find-Cell $after.sheets[0].cells 'cell:数据!J1'
$jlib = Find-Cell $before.sheets[0].cells 'cell:数据!J1'
$jok = ($cap -notmatch 'Repair') -and (Same-Number $j.value 46024) -and (Same-Number $jlib.value 46024) -and (Same-Number $j.value $live['J1'].value) -and ($live['J1'].text -ne '46024')
Add-Check 'xlsx-Xfo-J1' $jok "excel=$($live['J1'].value) inspect=$($j.value) library=$($jlib.value) text=$($live['J1'].text) numFmt=$($j.numFmt)"

$i = Find-Cell $after.sheets[0].cells 'cell:数据!I1'
$ilib = Find-Cell $before.sheets[0].cells 'cell:数据!I1'
$iok = ($cap -notmatch 'Repair') -and $live['I1'].isError -and ($i.type -eq 'error') -and ($i.value -eq '#DIV/0!') -and ($ilib.value -eq '#DIV/0!')
Add-Check 'xlsx-Xfo-I1' $iok "excelText=$($live['I1'].text) inspect=$($i.type):$($i.value) library=$($ilib.value)"

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

$dateBook = Get-OutPath 'xlsx-formula-oracle-1904.xlsx'
if (Test-Path $dateBook) { Remove-Item -Force $dateBook }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $dateBook.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; formula = '=DATE(2026,1,2)' })
        })
    }
} 'xlsx-xfo-1904-create' | Out-Null
$excel3 = Ensure-MsExcel $dateBook
Close-AllWorkbooks $excel3
$excel3.DisplayAlerts = $false
$wb3 = Open-ExcelBook $excel3 $dateBook
$wb3.Date1904 = $true
try { $excel3.CalculateFullRebuild() } catch { Write-Log "CalculateFullRebuild 1904 $($_.Exception.Message)" }
$cap3 = [string]$wb3.Windows.Item(1).Caption
$ws3 = $wb3.Worksheets.Item('Sheet1')
$wb3.Save()
$dateLive = Read-LiveCell $ws3 'A1'
Write-Log "Excel 1904 A1 text=$($dateLive.text) value=$($dateLive.value)"
Release-Com $ws3
Close-AllWorkbooks $excel3
$dateTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $dateBook.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1' }
} 'xlsx-xfo-1904-inspect'
$dateCell = Find-Cell $dateTree.sheets[0].cells 'cell:Sheet1!A1'
$dateOk = ($cap3 -notmatch 'Repair') -and (Same-Number $dateCell.value $dateLive.value) -and -not (Same-Number $dateLive.value 46024)
Add-Check 'xlsx-Xfo-1904' $dateOk "excel=$($dateLive.value) inspect=$($dateCell.value)"

if ($MyInvocation.InvocationName -ne '.') {
    Close-OfficeApps -Quit
}
