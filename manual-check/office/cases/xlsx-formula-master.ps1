# xlsx 共享公式改主格：docs/checklist-xlsx.md 的 Xsm1–Xsm5。
# 用 create 产物再 apply。不求值。不要占用 Xf。

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

function Find-Cell($cells, [string]$id) {
    foreach ($c in $cells) {
        if ($c.id -eq $id) { return $c }
    }
    return $null
}

$hostPath = Get-OutPath 'xlsx-formula-master-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = 10 }
                @{ ref = 'A2'; value = 11 }
                @{ ref = 'A3'; value = 12 }
            )
        })
    }
} 'xlsx-xsm-create' | Out-Null

$groupPath = Get-OutPath 'xlsx-formula-master-b2.xlsx'
if (Test-Path $groupPath) { Remove-Item -Force $groupPath }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $groupPath.Replace('\', '/')
    ops = @(@{
        op = 'setCell'; id = 'cell:Sheet1!B2'; formula = '=A2*2'; sharedRef = 'B2:B3'
    })
} 'xlsx-xsm-group' | Out-Null

$upPath = Get-OutPath 'xlsx-formula-master-up.xlsx'
if (Test-Path $upPath) { Remove-Item -Force $upPath }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $groupPath.Replace('\', '/')
    output = $upPath.Replace('\', '/')
    ops = @(@{
        op = 'setCell'; id = 'cell:Sheet1!B1'; formula = '=A1*2'; sharedRef = 'B1:B3'
    })
} 'xlsx-xsm-up' | Out-Null

$treeUp = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $upPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:C3' }
} 'xlsx-xsm-inspect-up'
$u1 = Find-Cell $treeUp.sheets[0].cells 'cell:Sheet1!B1'
$u2 = Find-Cell $treeUp.sheets[0].cells 'cell:Sheet1!B2'
$u3 = Find-Cell $treeUp.sheets[0].cells 'cell:Sheet1!B3'
$xsm1ok = ($u1.formula -eq '=A1*2') -and ($u1.shared -eq $true) -and ($u1.sharedRef -eq 'B1:B3') -and ($u2.formula -eq '=A2*2') -and ($u2.shared -eq $true) -and (-not $u2.sharedRef) -and ($u3.formula -eq '=A3*2') -and ($u3.shared -eq $true)
Add-Check 'xlsx-Xsm1' $xsm1ok "b1f=$($u1.formula) ref=$($u1.sharedRef) b2f=$($u2.formula) b2ref=$($u2.sharedRef) b3f=$($u3.formula)"

$excel = Ensure-MsExcel $upPath
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $upPath
$cap = [string]$wb.Windows.Item(1).Caption
$s1 = $wb.Worksheets.Item('Sheet1')
$b1v = [double]$s1.Range('B1').Value2
$b2v = [double]$s1.Range('B2').Value2
$b3v = [double]$s1.Range('B3').Value2
$b1f = [string]$s1.Range('B1').Formula
$b2f = [string]$s1.Range('B2').Formula
$b3f = [string]$s1.Range('B3').Formula
$xsm2ok = ($cap -notmatch 'Repair') -and ([math]::Abs($b1v - 20) -lt 0.001) -and ([math]::Abs($b2v - 22) -lt 0.001) -and ([math]::Abs($b3v - 24) -lt 0.001) -and ($b1f -eq '=A1*2') -and ($b2f -eq '=A2*2') -and ($b3f -eq '=A3*2')
Add-Check 'xlsx-Xsm2' $xsm2ok "caption=$cap b1v=$b1v b2v=$b2v b3v=$b3v b1f=$b1f b2f=$b2f b3f=$b3f"
Release-Com $s1
Close-AllWorkbooks $excel

$fullPath = Get-OutPath 'xlsx-formula-master-full.xlsx'
if (Test-Path $fullPath) { Remove-Item -Force $fullPath }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $fullPath.Replace('\', '/')
    ops = @(@{
        op = 'setCell'; id = 'cell:Sheet1!B1'; formula = '=A1*2'; sharedRef = 'B1:B3'
    })
} 'xlsx-xsm-full' | Out-Null

$downPath = Get-OutPath 'xlsx-formula-master-down.xlsx'
if (Test-Path $downPath) { Remove-Item -Force $downPath }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $fullPath.Replace('\', '/')
    output = $downPath.Replace('\', '/')
    ops = @(@{
        op = 'setCell'; id = 'cell:Sheet1!B2'; formula = '=A2*2'; sharedRef = 'B2:B3'
    })
} 'xlsx-xsm-down' | Out-Null

$treeDn = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $downPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:C3' }
} 'xlsx-xsm-inspect-down'
$d1 = Find-Cell $treeDn.sheets[0].cells 'cell:Sheet1!B1'
$d2 = Find-Cell $treeDn.sheets[0].cells 'cell:Sheet1!B2'
$d3 = Find-Cell $treeDn.sheets[0].cells 'cell:Sheet1!B3'
$inspectDn = ($d1.formula -eq '=A1*2') -and ($null -eq $d1.shared) -and ($d2.formula -eq '=A2*2') -and ($d2.shared -eq $true) -and ($d2.sharedRef -eq 'B2:B3') -and ($d3.formula -eq '=A3*2') -and ($d3.shared -eq $true)

$excel2 = Ensure-MsExcel $downPath
Close-AllWorkbooks $excel2
$wb2 = Open-ExcelBook $excel2 $downPath
$cap2 = [string]$wb2.Windows.Item(1).Caption
$s1b = $wb2.Worksheets.Item('Sheet1')
$e1v = [double]$s1b.Range('B1').Value2
$e2v = [double]$s1b.Range('B2').Value2
$e3v = [double]$s1b.Range('B3').Value2
$e1f = [string]$s1b.Range('B1').Formula
$e2f = [string]$s1b.Range('B2').Formula
$xsm3ok = $inspectDn -and ($cap2 -notmatch 'Repair') -and ([math]::Abs($e1v - 20) -lt 0.001) -and ([math]::Abs($e2v - 22) -lt 0.001) -and ([math]::Abs($e3v - 24) -lt 0.001) -and ($e1f -eq '=A1*2') -and ($e2f -eq '=A2*2')
Add-Check 'xlsx-Xsm3' $xsm3ok "caption=$cap2 inspect=$inspectDn e1v=$e1v e2v=$e2v e3v=$e3v e1f=$e1f e2f=$e2f"
Release-Com $s1b
Close-AllWorkbooks $excel2

$twoPath = Get-OutPath 'xlsx-formula-master-two.xlsx'
if (Test-Path $twoPath) { Remove-Item -Force $twoPath }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $twoPath.Replace('\', '/')
    ops = @(
        @{ op = 'setCell'; id = 'cell:Sheet1!B1'; formula = '=A1*2'; sharedRef = 'B1:B2' }
        @{ op = 'setCell'; id = 'cell:Sheet1!D1'; formula = '=A1*2'; sharedRef = 'D1:D2' }
    )
} 'xlsx-xsm-two' | Out-Null

$failOut = Get-OutPath 'xlsx-formula-master-overlap.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $twoPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'setCell'; id = 'cell:Sheet1!B1'; formula = '=A1*2'; sharedRef = 'B1:D2'
    })
} 'xlsx-xsm-overlap'
$codeTwo = $null
try { $codeTwo = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$xsm4ok = ($fail.ExitCode -ne 0) -and ($codeTwo -eq 'NotImplemented') -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xsm4' $xsm4ok "exit=$($fail.ExitCode) code=$codeTwo wrote=$(Test-Path $failOut)"

$fail2Out = Get-OutPath 'xlsx-formula-master-needref.xlsx'
$fail2 = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $groupPath.Replace('\', '/')
    output = $fail2Out.Replace('\', '/')
    ops = @(@{
        op = 'setCell'; id = 'cell:Sheet1!B3'; formula = '=A3*2'; shared = $true
    })
} 'xlsx-xsm-needref'
$codeNeed = $null
try { $codeNeed = ($fail2.Stderr | ConvertFrom-Json).error.code } catch {}
$xsm5ok = ($fail2.ExitCode -ne 0) -and ($codeNeed -eq 'InvalidField') -and (-not (Test-Path $fail2Out))
Add-Check 'xlsx-Xsm5' $xsm5ok "exit=$($fail2.ExitCode) code=$codeNeed wrote=$(Test-Path $fail2Out)"
