# 数学与三角函数缓存对照 Excel 16.0。编号 Xmh。
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

$seed = Get-OutPath 'xlsx-math-seed.xlsx'
$book = Get-OutPath 'xlsx-math.xlsx'
foreach ($p in @($seed, $book)) { if (Test-Path $p) { Remove-Item -Force $p } }

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $seed.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xmh-seed' | Out-Null

$cells = @(
    @{ ref = 'A1'; value = 1 }, @{ ref = 'B1'; value = 2 }
    @{ ref = 'A2'; value = 3 }, @{ ref = 'B2'; value = 4 }
    @{ ref = 'E1'; value = 1 }, @{ ref = 'E2'; value = 2 }
    @{ ref = 'E3'; value = 1 }, @{ ref = 'E4'; value = 3 }
    @{ ref = 'F1'; value = 10 }, @{ ref = 'F2'; value = 20 }
    @{ ref = 'F3'; value = 30 }, @{ ref = 'F4'; value = 40 }
    @{ ref = 'G1'; formula = '=SUBTOTAL(9,F1)' }
    @{ ref = 'G2'; value = 5 }
    @{ ref = 'C1'; formula = '=CEILING(2.5,1)' }
    @{ ref = 'C2'; formula = '=CEILING(-2.5,-1)' }
    @{ ref = 'C3'; formula = '=ISO.CEILING(-2.5,-1)' }
    @{ ref = 'C4'; formula = '=FLOOR(-2.5,1)' }
    @{ ref = 'C5'; formula = '=FLOOR(2.5,-1)' }
    @{ ref = 'C6'; formula = '=ROUND(2.15,1)' }
    @{ ref = 'C7'; formula = '=EVEN(-3)' }
    @{ ref = 'C8'; formula = '=MROUND(-10,3)' }
    @{ ref = 'C9'; formula = '=SUMIF(E1:E4,1,F1:F4)' }
    @{ ref = 'C10'; formula = '=SUBTOTAL(109,F1:F4)' }
    @{ ref = 'C11'; formula = '=SUBTOTAL(9,G1:G2)' }
    @{ ref = 'C12'; formula = '=PRODUCT(2,3,4)' }
    @{ ref = 'C13'; formula = '=ROMAN(1999)' }
    @{ ref = 'C14'; formula = '=MMULT(A1:B2,A1:B2)' }
    @{ ref = 'C15'; formula = '=ACOS(0)' }
    @{ ref = 'C16'; formula = '=PI()' }
    @{ ref = 'C17'; formula = '=RAND()' }
    @{ ref = 'C18'; formula = '=ECMA.SIN(0)' }
    @{ ref = 'C19'; formula = '=MOD(1,0)' }
)
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'; format = 'xlsx'
    path = $book.Replace('\', '/')
    workbook = @{ sheets = @(@{
        name = 'Sheet1'
        rows = @(@{ row = 2; hidden = $true })
        cells = $cells
    }) }
} 'xlsx-xmh-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'; format = 'xlsx'
    path = $book.Replace('\', '/')
    scope = @{ sheet = 'Sheet1'; range = 'A1:G19' }
} 'xlsx-xmh-inspect'

$excel = Ensure-MsExcel $seed
$excel.DisplayAlerts = $false
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $book
try { $excel.CalculateFullRebuild() } catch { Write-Log "calc $($_.Exception.Message)" }
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item(1)

function Read-Live($ws, [string]$addr) {
    $range = $ws.Range($addr)
    try { return [string]$range.Text }
    finally { Release-Com $range }
}

function Cache-Of($id) {
    return Find-Cell $tree.sheets[0].cells $id
}

function Same-Cell($id, $addr) {
    $c = Cache-Of $id
    $live = Read-Live $ws $addr
    if ($null -eq $c) { return "$addr missing" }
    if ($c.type -eq 'error' -or $c.type -eq 'text') {
        if ([string]$c.value -eq $live) { return $null }
        return "$addr live=$live cache=$($c.value)"
    }
    $cached = [double]$c.value
    $liveNum = 0.0
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

$round = @('C1','C2','C3','C4','C5','C6','C7','C8')
$agg = @('C9','C10','C11','C12','C13')
$rest = @('C14','C15','C16','C19')
$badRound = @($round | ForEach-Object { Same-Cell "cell:Sheet1!$_" $_ } | Where-Object { $_ })
$badAgg = @($agg | ForEach-Object { Same-Cell "cell:Sheet1!$_" $_ } | Where-Object { $_ })
$badRest = @($rest | ForEach-Object { Same-Cell "cell:Sheet1!$_" $_ } | Where-Object { $_ })
$randLive = 0.0
$randText = Read-Live $ws 'C17'
$randOk = [double]::TryParse($randText, [Globalization.NumberStyles]::Float, [Globalization.CultureInfo]::InvariantCulture, [ref]$randLive)
if (-not $randOk) { $randOk = [double]::TryParse($randText, [ref]$randLive) }
$randCache = [double](Cache-Of 'cell:Sheet1!C17').value
$randOk = $randOk -and ($randLive -ge 0) -and ($randLive -lt 1) -and ($randCache -ge 0) -and ($randCache -lt 1)
$ecma = Cache-Of 'cell:Sheet1!C18'
$ecmaLive = Read-Live $ws 'C18'
$noRepair = $cap -notmatch 'Repair'
Release-Com $ws
Close-AllWorkbooks $excel

Add-Check 'xlsx-Xmh1' ($noRepair -and $badRound.Count -eq 0) ("caption=$cap " + ($badRound -join '; '))
Add-Check 'xlsx-Xmh2' ($noRepair -and $badAgg.Count -eq 0) ($badAgg -join '; ')
$nameOk = ($null -ne $ecma) -and ($null -eq $ecma.value) -and ($ecmaLive -eq '#NAME?')
Add-Check 'xlsx-Xmh3' ($noRepair -and $badRest.Count -eq 0 -and $randOk -and $nameOk) ("rest=$($badRest -join '; ') randLive=$randText ecma=$ecmaLive caption=$cap")

if ($MyInvocation.InvocationName -ne '.') {
    Close-OfficeApps -Quit
}
