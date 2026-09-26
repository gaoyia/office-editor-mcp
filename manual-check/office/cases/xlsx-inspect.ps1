# xlsx inspect：docs/checklist-xlsx.md 的 I1–I6。
# 只调 ooxp，不开 Excel，所以这一组是秒级的。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$book = (Ensure-XlsxCreate).Replace('\', '/')

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $book; format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:D20' }
} 'xlsx-inspect'

$ids = @($tree.sheets | ForEach-Object { $_.id })
$cells = @($tree.sheets[0].cells)
$refs = @($cells | ForEach-Object { $_.ref })
$cA1 = $cells | Where-Object { $_.ref -eq 'A1' } | Select-Object -First 1
$cA2 = $cells | Where-Object { $_.ref -eq 'A2' } | Select-Object -First 1
$cB1 = $cells | Where-Object { $_.ref -eq 'B1' } | Select-Object -First 1

Add-Check 'xlsx-I1' ($ids -contains 'sheet:Sheet1') ($ids -join ',')
Add-Check 'xlsx-I2' ($refs -notcontains 'E1') ($refs -join ',')
Add-Check 'xlsx-I3' ($cA1.type -eq 'number' -and $cA1.value -eq 1) "$($cA1.type)/$($cA1.value)"
Add-Check 'xlsx-I4' ($cA2.type -eq 'text' -and $cA2.value -eq 'hello') "$($cA2.type)/$($cA2.value)"
Add-Check 'xlsx-I5' ($cB1.formula -eq '=SUM(A1)') $cB1.formula

# 不给 scope 时不许把整本工作簿的格子倒出来。
$tree2 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $book; format = 'xlsx'
} 'xlsx-inspect-sheets'

$hasCells = $false
foreach ($sh in @($tree2.sheets)) {
    if ($sh.PSObject.Properties.Name -contains 'cells') { $hasCells = $true }
}
Add-Check 'xlsx-I6' (-not $hasCells) "hasCells=$hasCells"
