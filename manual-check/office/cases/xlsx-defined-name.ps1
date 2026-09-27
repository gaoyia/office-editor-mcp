# xlsx 命名区域：docs/checklist-xlsx.md 的 Xnm1–Xnm5。
# 用 create 产物再 apply，不另做 Excel 另存样例。

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

$hostPath = Get-OutPath 'xlsx-defined-name-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = 1 }
                @{ ref = 'A10'; value = 10 }
                @{ ref = 'A20'; value = 20 }
            )
        })
    }
} 'xlsx-xnm-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xnm-inspect'
$xnm1ok = ($tree0.definedNameCount -eq 0) -and ($null -ne $tree0.definedNames) -and ($tree0.definedNames.Count -eq 0)
Add-Check 'xlsx-Xnm1' $xnm1ok "count=$($tree0.definedNameCount) n=$($tree0.definedNames.Count)"

$namedOut = Get-OutPath 'xlsx-defined-name-create.xlsx'
if (Test-Path $namedOut) { Remove-Item -Force $namedOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $namedOut.Replace('\', '/')
    ops = @(@{
        op = 'insertDefinedName'
        id = 'xlsx:workbook'
        name = 'Sales'
        formula = 'Sheet1!$A$1:$A$10'
    })
} 'xlsx-xnm-insert' | Out-Null

$excel = Ensure-MsExcel $namedOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $namedOut
$cap = [string]$wb.Windows.Item(1).Caption
$nmCount = $null
$nmName = $null
$nmRef = $null
try {
    $names = $wb.Names
    try {
        $nmCount = [int]$names.Count
        if ($nmCount -ge 1) {
            $nm = $null
            try { $nm = $names.Item('Sales') } catch { $nm = $names.Item(1) }
            try {
                $nmName = [string]$nm.Name
                $nmRef = [string]$nm.RefersTo
            } finally { Release-Com $nm }
        }
    } finally { Release-Com $names }
} catch {
    Write-Log "Xnm2 Names 读失败 $($_.Exception.Message)"
    $nmCount = -1
}
$xnm2ok = ($cap -notmatch 'Repair') -and ($nmCount -ge 1) -and ($nmName -eq 'Sales')
Add-Check 'xlsx-Xnm2' $xnm2ok "caption=$cap count=$nmCount name=$nmName refers=$nmRef"
Close-AllWorkbooks $excel

$setOut = Get-OutPath 'xlsx-defined-name-set.xlsx'
if (Test-Path $setOut) { Remove-Item -Force $setOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $namedOut.Replace('\', '/')
    output = $setOut.Replace('\', '/')
    ops = @(@{
        op = 'setDefinedName'
        id = 'name:1'
        formula = 'Sheet1!$A$1:$A$20'
    })
} 'xlsx-xnm-set' | Out-Null

$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $setOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xnm-set-inspect'
Close-AllWorkbooks $excel
$wb2 = Open-ExcelBook $excel $setOut
$cap2 = [string]$wb2.Windows.Item(1).Caption
$ref2 = $null
try {
    $names2 = $wb2.Names
    try {
        $nm2 = $names2.Item('Sales')
        try { $ref2 = [string]$nm2.RefersTo } finally { Release-Com $nm2 }
    } finally { Release-Com $names2 }
} catch {
    Write-Log "Xnm3 Names 读失败 $($_.Exception.Message)"
}
$xnm3ok = ($cap2 -notmatch 'Repair') -and ($treeS.definedNames[0].formula -eq 'Sheet1!$A$1:$A$20') -and ($ref2 -match 'A\$20')
Add-Check 'xlsx-Xnm3' $xnm3ok "caption=$cap2 formula=$($treeS.definedNames[0].formula) refers=$ref2"
Close-AllWorkbooks $excel

$delOut = Get-OutPath 'xlsx-defined-name-delete.xlsx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $setOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteDefinedName'; id = 'name:1' })
} 'xlsx-xnm-delete' | Out-Null

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xnm-del-inspect'
Close-AllWorkbooks $excel
$wb3 = Open-ExcelBook $excel $delOut
$cap3 = [string]$wb3.Windows.Item(1).Caption
$delCount = $null
try {
    $names3 = $wb3.Names
    try { $delCount = [int]$names3.Count } finally { Release-Com $names3 }
} catch {
    Write-Log "Xnm4 Names 读失败 $($_.Exception.Message)"
    $delCount = -1
}
$xnm4ok = ($cap3 -notmatch 'Repair') -and ($treeD.definedNameCount -eq 0) -and ($delCount -eq 0)
Add-Check 'xlsx-Xnm4' $xnm4ok "caption=$cap3 inspect=$($treeD.definedNameCount) comCount=$delCount"
Close-AllWorkbooks $excel

$failOut = Get-OutPath 'xlsx-defined-name-bad.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'insertDefinedName'; id = 'xlsx:workbook' })
} 'xlsx-xnm-missing'
$codeEmpty = $null
try { $codeEmpty = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$xnm5ok = ($fail.ExitCode -ne 0) -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xnm5' $xnm5ok "exit=$($fail.ExitCode) code=$codeEmpty wrote=$(Test-Path $failOut)"
