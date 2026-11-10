# 新建数据表：Excel 16.0 打开不修复，并算出列输入的结果。
# 编号 Xdt。单独运行时会关掉自己启动的 Excel。

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
        return @{ text = [string]$range.Text; value = $range.Value2 }
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

$book = Get-OutPath 'xlsx-data-table.xlsx'
if (Test-Path $book) { Remove-Item -Force $book }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $book.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'C1'; value = 10 }
                @{ ref = 'B1'; formula = '=C1*2' }
                @{ ref = 'A2'; value = 1 }
                @{ ref = 'A3'; value = 2 }
                @{ ref = 'A4'; value = 3 }
                @{ ref = 'B2'; formula = '=TABLE(,C1)'; dataTableRef = 'B2:B4' }
            )
        })
    }
} 'xlsx-xdt-create' | Out-Null

$excel = Ensure-MsExcel $book
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $book
try { $excel.CalculateFullRebuild() } catch { Write-Log "CalculateFullRebuild $($_.Exception.Message)" }
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item('Sheet1')
$b2 = Read-LiveCell $ws 'B2'
$b3 = Read-LiveCell $ws 'B3'
$b4 = Read-LiveCell $ws 'B4'
Write-Log "Excel B2=$($b2.value) B3=$($b3.value) B4=$($b4.value) caption=$cap"
$wb.Save()
Release-Com $ws
Close-AllWorkbooks $excel

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $book.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:C4' }
} 'xlsx-xdt-inspect'
$cell = Find-Cell $tree.sheets[0].cells 'cell:Sheet1!B2'
$ok = ($cap -notmatch 'Repair') -and ($cell.formula -eq '=TABLE(,C1)') -and ($cell.dataTableRef -eq 'B2:B4') -and (Same-Number $b2.value 2) -and (Same-Number $b3.value 4) -and (Same-Number $b4.value 6) -and (Same-Number $cell.value $b2.value)
Add-Check 'xlsx-Xdt' $ok "excel=$($b2.value),$($b3.value),$($b4.value) inspect=$($cell.value) formula=$($cell.formula) caption=$cap"

if ($MyInvocation.InvocationName -ne '.') {
    Close-OfficeApps -Quit
}
