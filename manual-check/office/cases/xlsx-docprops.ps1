# xlsx 文档属性：docs/checklist-xlsx.md 的 Xd1–Xd5。
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

$hostPath = Get-OutPath 'xlsx-docprops-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) })
        props = @{ company = '测试公司' }
    }
} 'xlsx-xd-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xd-inspect'
$hasProps = $tree0.PSObject.Properties.Name -contains 'props'
$xd1ok = $hasProps -and ([string]$tree0.props.creator -eq 'ooxp') -and ([string]$tree0.props.company -eq '测试公司')
Add-Check 'xlsx-Xd1' $xd1ok "keys=$($tree0.PSObject.Properties.Name -join ',') creator=$($tree0.props.creator) company=$($tree0.props.company)"

$excel = Ensure-MsExcel $hostPath
Close-AllWorkbooks $excel

$titleOut = Get-OutPath 'xlsx-docprops-title.xlsx'
if (Test-Path $titleOut) { Remove-Item -Force $titleOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $titleOut.Replace('\', '/')
    ops = @(@{ op = 'setProps'; id = 'xlsx:workbook'; title = '新标题' })
} 'xlsx-xd-set-title' | Out-Null

$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $titleOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xd-title-inspect'
$comTitle = Get-ShellFileProp $titleOut @('Title', '标题')
$comCompany = Get-ShellFileProp $titleOut @('Company', '公司')
$wbT = Open-ExcelBook $excel $titleOut
$capT = [string]$wbT.Windows.Item(1).Caption
$xd2ok = ($capT -notmatch 'Repair') -and ($comTitle -eq '新标题') -and ($comCompany -eq '测试公司') -and ([string]$treeT.props.title -eq '新标题')
Add-Check 'xlsx-Xd2' $xd2ok "caption=$capT title=$comTitle company=$comCompany"
Close-AllWorkbooks $excel

$customOut = Get-OutPath 'xlsx-docprops-custom.xlsx'
if (Test-Path $customOut) { Remove-Item -Force $customOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $titleOut.Replace('\', '/')
    output = $customOut.Replace('\', '/')
    ops = @(@{ op = 'setCustomProp'; id = 'xlsx:workbook'; name = '部门'; value = '研发' })
} 'xlsx-xd-set-custom' | Out-Null

$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $customOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xd-custom-inspect'
$c0 = $null
if ($treeC.props.custom) { $c0 = @($treeC.props.custom) | Select-Object -First 1 }
$comCustom = Get-CustomPropFromZip $customOut '部门'
$wbC = Open-ExcelBook $excel $customOut
$capC = [string]$wbC.Windows.Item(1).Caption
$xd3ok = ($capC -notmatch 'Repair') -and ($null -ne $c0) -and ([string]$c0.id -eq 'prop:部门') -and ([string]$comCustom -eq '研发')
Add-Check 'xlsx-Xd3' $xd3ok "caption=$capC id=$($c0.id) com=$comCustom"
Close-AllWorkbooks $excel

$delOut = Get-OutPath 'xlsx-docprops-deleted.xlsx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $customOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteCustomProp'; id = 'prop:部门' })
} 'xlsx-xd-del-custom' | Out-Null

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xd-del-inspect'
$left = @($treeD.props.custom).Count
$gone = Get-CustomPropFromZip $delOut '部门'
$wbD = Open-ExcelBook $excel $delOut
$capD = [string]$wbD.Windows.Item(1).Caption
$xd4ok = ($capD -notmatch 'Repair') -and ($left -eq 0) -and ($null -eq $gone)
Add-Check 'xlsx-Xd4' $xd4ok "caption=$capD left=$left gone=$gone"
Close-AllWorkbooks $excel

$failOut = Get-OutPath 'xlsx-docprops-bad.xlsx'
if (Test-Path $failOut) { Remove-Item -Force $failOut }
$raw = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'setProps'; id = 'cell:Sheet1!A1'; title = 'x' })
} 'xlsx-xd-bad-id'
$code = $null
try { $code = ($raw.Stderr | ConvertFrom-Json).error.code } catch {}
$noWrite = -not (Test-Path $failOut)
Add-Check 'xlsx-Xd5' (($raw.ExitCode -ne 0) -and ($code -eq 'InvalidId') -and $noWrite) `
    "code=$code exit=$($raw.ExitCode) wrote=$(-not $noWrite)"
