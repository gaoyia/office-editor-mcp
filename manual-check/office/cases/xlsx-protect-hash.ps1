# xlsx 保护密码哈希：docs/checklist-xlsx.md 的 Xph1–Xph5。
# 用 create 产物再 apply。编号 Xph，不要和 Xpr、Xp、Xh 撞。不破解密码。

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

$hostPath = Get-OutPath 'xlsx-protect-hash-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
        })
    }
} 'xlsx-xph-create' | Out-Null

$sheetOut = Get-OutPath 'xlsx-protect-hash-sheet.xlsx'
if (Test-Path $sheetOut) { Remove-Item -Force $sheetOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $sheetOut.Replace('\', '/')
    ops = @(@{
        op = 'setProtection'
        id = 'sheet:Sheet1'
        enforced = $true
        password = 'secret'
    })
} 'xlsx-xph-sheet' | Out-Null

$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sheetOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xph-inspect-sheet'
$sp = $treeS.sheets[0].sheetProtection
$dump = ($treeS | ConvertTo-Json -Depth 12 -Compress)
$xph1ok = ([bool]$sp.enforced) `
    -and ([bool]$sp.hasPassword) `
    -and ($null -eq $sp.hashValue) `
    -and ($null -eq $sp.password) `
    -and ($dump -notmatch 'secret') `
    -and ($dump -notmatch 'hashValue')
Add-Check 'xlsx-Xph1' $xph1ok "enforced=$($sp.enforced) hash=$($sp.hasPassword)"

$excel = Ensure-MsExcel $sheetOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $sheetOut
$cap = [string]$wb.Windows.Item(1).Caption
$s = $wb.Worksheets.Item(1)
$contents = [bool]$s.ProtectContents
$wrongFailed = $false
try {
    $s.Unprotect('wrong')
    $wrongFailed = $false
} catch {
    $wrongFailed = $true
}
$unprotOk = $false
try {
    $s.Unprotect('secret')
    $unprotOk = -not [bool]$s.ProtectContents
} catch {
    $unprotOk = $false
}
$xph2ok = ($cap -notmatch 'Repair') -and $contents -and $wrongFailed -and $unprotOk
Add-Check 'xlsx-Xph2' $xph2ok "caption=$cap ProtectContents=$contents wrongFailed=$wrongFailed unprot=$unprotOk"
Close-AllWorkbooks $excel

$wbOut = Get-OutPath 'xlsx-protect-hash-workbook.xlsx'
if (Test-Path $wbOut) { Remove-Item -Force $wbOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $wbOut.Replace('\', '/')
    ops = @(@{
        op = 'setProtection'
        id = 'xlsx:workbook'
        lockStructure = $true
        password = 'secret'
    })
} 'xlsx-xph-workbook' | Out-Null

Close-AllWorkbooks $excel
$wbS = Open-ExcelBook $excel $wbOut
$capS = [string]$wbS.Windows.Item(1).Caption
$struct = [bool]$wbS.ProtectStructure
$wbUnprot = $false
try {
    $wbS.Unprotect('secret')
    $wbUnprot = -not [bool]$wbS.ProtectStructure
} catch {
    $wbUnprot = $false
}
$xph3ok = ($capS -notmatch 'Repair') -and $struct -and $wbUnprot
Add-Check 'xlsx-Xph3' $xph3ok "caption=$capS ProtectStructure=$struct unprot=$wbUnprot"
Close-AllWorkbooks $excel

$openOut = Get-OutPath 'xlsx-protect-hash-clear.xlsx'
if (Test-Path $openOut) { Remove-Item -Force $openOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sheetOut.Replace('\', '/')
    output = $openOut.Replace('\', '/')
    ops = @(@{
        op = 'setProtection'
        id = 'sheet:Sheet1'
        enforced = $false
        password = 'secret'
    })
} 'xlsx-xph-clear' | Out-Null

$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $openOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xph-inspect-clear'
$spC = $treeC.sheets[0].sheetProtection
Close-AllWorkbooks $excel
$wbC = Open-ExcelBook $excel $openOut
$capC = [string]$wbC.Windows.Item(1).Caption
$contentsC = [bool]$wbC.Worksheets.Item(1).ProtectContents
$xph4ok = (-not [bool]$spC.hasPassword) -and (-not [bool]$spC.enforced) `
    -and ($capC -notmatch 'Repair') -and (-not $contentsC)
Add-Check 'xlsx-Xph4' $xph4ok "hasPassword=$($spC.hasPassword) caption=$capC ProtectContents=$contentsC"
Close-AllWorkbooks $excel

$blankOut = Get-OutPath 'xlsx-protect-hash-blank.xlsx'
$blank = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $blankOut.Replace('\', '/')
    ops = @(@{
        op = 'setProtection'
        id = 'sheet:Sheet1'
        enforced = $true
        password = ''
    })
} 'xlsx-xph-blank'
$codeBlank = $null
try { $codeBlank = ($blank.Stderr | ConvertFrom-Json).error.code } catch {}

$wrongOut = Get-OutPath 'xlsx-protect-hash-wrong.xlsx'
$wrong = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $sheetOut.Replace('\', '/')
    output = $wrongOut.Replace('\', '/')
    ops = @(@{
        op = 'setProtection'
        id = 'sheet:Sheet1'
        enforced = $false
        password = 'wrong'
    })
} 'xlsx-xph-wrong'
$codeWrong = $null
try { $codeWrong = ($wrong.Stderr | ConvertFrom-Json).error.code } catch {}

$noPwOut = Get-OutPath 'xlsx-protect-hash-nopw.xlsx'
$noPw = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $sheetOut.Replace('\', '/')
    output = $noPwOut.Replace('\', '/')
    ops = @(@{
        op = 'setProtection'
        id = 'sheet:Sheet1'
        enforced = $false
    })
} 'xlsx-xph-nopw'
$codeNoPw = $null
try { $codeNoPw = ($noPw.Stderr | ConvertFrom-Json).error.code } catch {}

$xph5ok = ($blank.ExitCode -ne 0) -and ($codeBlank -eq 'InvalidField') -and (-not (Test-Path $blankOut)) `
    -and ($wrong.ExitCode -ne 0) -and ($codeWrong -eq 'InvalidField') -and (-not (Test-Path $wrongOut)) `
    -and ($noPw.ExitCode -ne 0) -and ($codeNoPw -eq 'NotImplemented') -and (-not (Test-Path $noPwOut))
Add-Check 'xlsx-Xph5' $xph5ok "blank=$codeBlank wrong=$codeWrong nopw=$codeNoPw"
