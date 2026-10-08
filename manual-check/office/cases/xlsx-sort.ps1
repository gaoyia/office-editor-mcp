# xlsx 排序：docs/checklist-xlsx.md 的 Xsr1–Xsr5。
# 用 create 产物再 apply。不重排格子，不算排序引擎。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function Invoke-OoxpExpectFail($obj, [string]$name) {
    $path = Join-Path $global:OoxpReqDir "$name.json"
    Write-JsonFile $path $obj
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $global:OoxpExe
    $psi.Arguments = "--input `"$path`""
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = [System.Text.Encoding]::UTF8
    $psi.StandardErrorEncoding = [System.Text.Encoding]::UTF8
    $proc = [System.Diagnostics.Process]::Start($psi)
    $stdout = $proc.StandardOutput.ReadToEnd().Trim()
    $stderr = $proc.StandardError.ReadToEnd().Trim()
    $proc.WaitForExit()
    $code = $proc.ExitCode
    $proc.Dispose()
    Write-Log "ooxp $name 退出码 $code stderr=$stderr"
    return @{ ExitCode = $code; Stderr = $stderr; Stdout = $stdout }
}

$hostPath = Get-OutPath 'xlsx-sort-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = 3 }
                @{ ref = 'A2'; value = 1 }
                @{ ref = 'A3'; value = 2 }
            )
        })
    }
} 'xlsx-xsr-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xsr-inspect'
$xsr1ok = $null -eq $tree0.sheets[0].sortState
Add-Check 'xlsx-Xsr1' $xsr1ok "sortState=$($tree0.sheets[0].sortState)"

$sortOut = Get-OutPath 'xlsx-sort-set.xlsx'
if (Test-Path $sortOut) { Remove-Item -Force $sortOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $sortOut.Replace('\', '/')
    ops = @(@{
        op = 'setSortState'
        id = 'sheet:Sheet1'
        ref = 'A1:A3'
        columns = @(@{ ref = 'A1:A3'; descending = $true })
    })
} 'xlsx-xsr-set' | Out-Null

$treeSet = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sortOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:A3' }
} 'xlsx-xsr-inspect-set'
$st = $treeSet.sheets[0].sortState
$a1cell = $treeSet.sheets[0].cells | Where-Object { $_.ref -eq 'A1' } | Select-Object -First 1
$a1val = if ($null -ne $a1cell) { [int]$a1cell.value } else { -1 }
$inspectOk = ($null -ne $st) -and ($st.ref -eq 'A1:A3') -and ([bool]$st.columns[0].descending) -and ($a1val -eq 3)

$excel = Ensure-MsExcel $sortOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $sortOut
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item(1)
$comA1 = [int]$ws.Range('A1').Value2
$xsr2ok = ($cap -notmatch 'Repair') -and $inspectOk -and ($comA1 -eq 3)
Add-Check 'xlsx-Xsr2' $xsr2ok "caption=$cap ref=$($st.ref) desc=$($st.columns[0].descending) a1=$a1val comA1=$comA1"

$cnt = 0
$key = ''
$order = 0
try { $cnt = [int]$ws.Sort.SortFields.Count } catch { $cnt = -1 }
if ($cnt -ge 1) {
    try { $key = [string]$ws.Sort.SortFields.Item(1).Key.Address($true, $true) } catch { $key = '' }
    try { $order = [int]$ws.Sort.SortFields.Item(1).Order } catch { $order = 0 }
}
# xlDescending = 2
$xsr3ok = ($cap -notmatch 'Repair') -and ($cnt -ge 1) -and ($order -eq 2) -and ($key -match 'A\$1:\$A\$3')
Add-Check 'xlsx-Xsr3' $xsr3ok "caption=$cap count=$cnt key=$key order=$order"
Close-AllWorkbooks $excel

$clearOut = Get-OutPath 'xlsx-sort-clear.xlsx'
if (Test-Path $clearOut) { Remove-Item -Force $clearOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sortOut.Replace('\', '/')
    output = $clearOut.Replace('\', '/')
    ops = @(@{ op = 'setSortState'; id = 'sheet:Sheet1'; ref = $null })
} 'xlsx-xsr-clear' | Out-Null

$treeClr = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $clearOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xsr-inspect-clear'
Close-AllWorkbooks $excel
$wbC = Open-ExcelBook $excel $clearOut
$capC = [string]$wbC.Windows.Item(1).Caption
$xsr4ok = ($capC -notmatch 'Repair') -and ($null -eq $treeClr.sheets[0].sortState)
Add-Check 'xlsx-Xsr4' $xsr4ok "caption=$capC sortState=$($treeClr.sheets[0].sortState)"
Close-AllWorkbooks $excel

$failOut = Get-OutPath 'xlsx-sort-bad.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'setSortState'; id = 'sheet:Sheet1' })
} 'xlsx-xsr-missing'
$codeEmpty = $null
try { $codeEmpty = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}

$fail2Out = Get-OutPath 'xlsx-sort-oob.xlsx'
$fail2 = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $fail2Out.Replace('\', '/')
    ops = @(@{
        op = 'setSortState'
        id = 'sheet:Sheet1'
        ref = 'A1:A3'
        columns = @(@{ ref = 'B1:B3' })
    })
} 'xlsx-xsr-oob'
$codeOob = $null
try { $codeOob = ($fail2.Stderr | ConvertFrom-Json).error.code } catch {}

$xsr5ok = ($fail.ExitCode -ne 0) -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failOut)) -and ($fail2.ExitCode -ne 0) -and ($codeOob -eq 'InvalidField') -and (-not (Test-Path $fail2Out))
Add-Check 'xlsx-Xsr5' $xsr5ok "exit=$($fail.ExitCode)/$($fail2.ExitCode) code=$codeEmpty/$codeOob wrote=$(Test-Path $failOut)/$(Test-Path $fail2Out)"
