# xlsx 数据验证：docs/checklist-xlsx.md 的 Xv1–Xv5。
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

$hostPath = Get-OutPath 'xlsx-validation-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
        })
    }
} 'xlsx-xv-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:C3' }
} 'xlsx-xv-inspect'
$xv1ok = ($tree0.sheets[0].dataValidationCount -eq 0) -and ($null -ne $tree0.sheets[0].dataValidations) -and ($tree0.sheets[0].dataValidations.Count -eq 0)
Add-Check 'xlsx-Xv1' $xv1ok "count=$($tree0.sheets[0].dataValidationCount) n=$($tree0.sheets[0].dataValidations.Count)"

$listOut = Get-OutPath 'xlsx-validation-list.xlsx'
if (Test-Path $listOut) { Remove-Item -Force $listOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $listOut.Replace('\', '/')
    ops = @(@{
        op = 'insertDataValidation'
        id = 'sheet:Sheet1'
        sqref = 'A1'
        type = 'list'
        formula1 = '1,2,3'
    })
} 'xlsx-xv-list' | Out-Null

$excel = Ensure-MsExcel $listOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $listOut
$cap = [string]$wb.Windows.Item(1).Caption
$s1 = $wb.Worksheets.Item('Sheet1')
$a1 = $s1.Range('A1')
$dvType = $null
$formula1 = $null
$dropdown = $null
try {
    $dvType = [int]$a1.Validation.Type
    $formula1 = [string]$a1.Validation.Formula1
    $dropdown = [bool]$a1.Validation.InCellDropdown
} catch {
    Write-Log "Xv2 Validation 读失败 $($_.Exception.Message)"
}
$xv2ok = ($cap -notmatch 'Repair') -and ($dvType -eq 3) -and ($formula1 -match '1') -and ($dropdown -eq $true)
Add-Check 'xlsx-Xv2' $xv2ok "caption=$cap type=$dvType f1=$formula1 dd=$dropdown"
Release-Com $a1
Release-Com $s1
Close-AllWorkbooks $excel

$multiOut = Get-OutPath 'xlsx-validation-multi.xlsx'
if (Test-Path $multiOut) { Remove-Item -Force $multiOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $listOut.Replace('\', '/')
    output = $multiOut.Replace('\', '/')
    ops = @(@{
        op = 'setDataValidation'
        id = 'sheet:Sheet1/dv:1'
        sqref = @('A1', 'C1')
    })
} 'xlsx-xv-multi' | Out-Null

$treeM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $multiOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:C3' }
} 'xlsx-xv-multi-inspect'
$sq = @($treeM.sheets[0].dataValidations[0].sqref)
$wb2 = Open-ExcelBook $excel $multiOut
$cap2 = [string]$wb2.Windows.Item(1).Caption
$s2 = $wb2.Worksheets.Item('Sheet1')
$c1type = $null
try { $c1type = [int]$s2.Range('C1').Validation.Type } catch { $c1type = 0 }
$xv3ok = ($cap2 -notmatch 'Repair') -and ($sq.Count -eq 2) -and ($sq -contains 'A1') -and ($sq -contains 'C1') -and ($c1type -eq 3)
Add-Check 'xlsx-Xv3' $xv3ok "caption=$cap2 sq=$($sq -join ',') c1type=$c1type"
Release-Com $s2
Close-AllWorkbooks $excel

$delOut = Get-OutPath 'xlsx-validation-delete.xlsx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $multiOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteDataValidation'; id = 'sheet:Sheet1/dv:1' })
} 'xlsx-xv-delete' | Out-Null

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:C3' }
} 'xlsx-xv-delete-inspect'
$wb3 = Open-ExcelBook $excel $delOut
$cap3 = [string]$wb3.Windows.Item(1).Caption
$s3 = $wb3.Worksheets.Item('Sheet1')
$delType = $null
$delF1 = $null
try {
    $delType = [int]$s3.Range('A1').Validation.Type
    $delF1 = [string]$s3.Range('A1').Validation.Formula1
} catch {
    $delType = 0
    $delF1 = ''
}
# 没有校验时 COM 往往不抛错，Type 为 0（xlValidateInputOnly）。列表校验是 3。
$xv4ok = ($cap3 -notmatch 'Repair') -and ($treeD.sheets[0].dataValidationCount -eq 0) -and ($delType -ne 3) -and ($delF1 -notmatch '1,2,3')
Add-Check 'xlsx-Xv4' $xv4ok "caption=$cap3 count=$($treeD.sheets[0].dataValidationCount) type=$delType f1=$delF1"
Release-Com $s3
Close-AllWorkbooks $excel

$failOut = Get-OutPath 'xlsx-validation-bad.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'insertDataValidation'; id = 'sheet:Sheet1' })
} 'xlsx-xv-missing'
$codeEmpty = $null
try { $codeEmpty = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$xv5ok = ($fail.ExitCode -ne 0) -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xv5' $xv5ok "exit=$($fail.ExitCode) code=$codeEmpty wrote=$(Test-Path $failOut)"
