# docx 保护密码哈希：docs/checklist-docx.md 的 Ph1–Ph5。
# 用 create 产物再 apply。编号 Ph，不要和 Pr、Pb 撞。不破解密码。

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

$S = $global:OoxpStrings
$hostPath = Get-OutPath 'docx-protect-hash-host.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = $S.chapter1 }
            @{ text = $S.hello }
        )
    }
} 'docx-ph-create' | Out-Null

$setOut = Get-OutPath 'docx-protect-hash-set.docx'
if (Test-Path $setOut) { Remove-Item -Force $setOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $setOut.Replace('\', '/')
    ops = @(@{
        op = 'setProtection'
        id = 'docx:document'
        enforced = $true
        edit = 'readOnly'
        password = 'secret'
    })
} 'docx-ph-set' | Out-Null

$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $setOut.Replace('\', '/'); format = 'docx'
} 'docx-ph-inspect-set'
$dump = ($treeS | ConvertTo-Json -Depth 12 -Compress)
$ph1ok = ([bool]$treeS.protection.enforced) `
    -and ([bool]$treeS.protection.hasPassword) `
    -and ([string]$treeS.protection.edit -eq 'readOnly') `
    -and ($null -eq $treeS.protection.hash) `
    -and ($dump -notmatch 'secret') `
    -and ($dump -notmatch 'hashValue')
Add-Check 'docx-Ph1' $ph1ok "enforced=$($treeS.protection.enforced) hash=$($treeS.protection.hasPassword)"

$word = Ensure-MsWord (Ensure-DocxCreate)
Close-WordDocByPath $word $setOut
$doc = Open-WordDoc $word $setOut
$cap = [string]$doc.ActiveWindow.Caption
$ptype = [int]$doc.ProtectionType
$wrongFailed = $false
try {
    $doc.Unprotect('wrong')
    $wrongFailed = $false
} catch {
    $wrongFailed = $true
}
$ok = $false
try {
    $doc.Unprotect('secret')
    $ok = ([int]$doc.ProtectionType -eq -1)
} catch {
    Write-Log ("Unprotect secret FAIL " + $_.Exception.Message)
}
Close-WordDoc $doc
Add-Check 'docx-Ph2' (($ptype -eq 3) -and $wrongFailed -and $ok -and ($cap -notmatch 'Repair')) "type=$ptype wrong=$wrongFailed ok=$ok caption=$cap"

$offOut = Get-OutPath 'docx-protect-hash-off.docx'
if (Test-Path $offOut) { Remove-Item -Force $offOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $setOut.Replace('\', '/')
    output = $offOut.Replace('\', '/')
    ops = @(@{
        op = 'setProtection'
        id = 'docx:document'
        enforced = $false
        password = 'secret'
    })
} 'docx-ph-off' | Out-Null
$treeOff = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $offOut.Replace('\', '/'); format = 'docx'
} 'docx-ph-inspect-off'
$ph3ok = (-not [bool]$treeOff.protection.enforced) -and (-not [bool]$treeOff.protection.hasPassword)
Add-Check 'docx-Ph3' $ph3ok "enforced=$($treeOff.protection.enforced) hash=$($treeOff.protection.hasPassword)"

Close-WordDocByPath $word $offOut
$docOff = Open-WordDoc $word $offOut
$ptypeOff = [int]$docOff.ProtectionType
$capOff = [string]$docOff.ActiveWindow.Caption
Close-WordDoc $docOff
Add-Check 'docx-Ph4' (($ptypeOff -eq -1) -and ($capOff -notmatch 'Repair')) "ProtectionType=$ptypeOff caption=$capOff"

$failBlank = Get-OutPath 'docx-protect-hash-blank.docx'
$failWrong = Get-OutPath 'docx-protect-hash-wrong.docx'
$failNone = Get-OutPath 'docx-protect-hash-none.docx'
if (Test-Path $failBlank) { Remove-Item -Force $failBlank }
if (Test-Path $failWrong) { Remove-Item -Force $failWrong }
if (Test-Path $failNone) { Remove-Item -Force $failNone }
$badBlank = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failBlank.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'docx:document'; enforced = $true; password = '' })
} 'docx-ph-blank'
$badWrong = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $setOut.Replace('\', '/')
    output = $failWrong.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'docx:document'; enforced = $false; password = 'wrong' })
} 'docx-ph-wrong'
$badNone = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $setOut.Replace('\', '/')
    output = $failNone.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'docx:document'; enforced = $false })
} 'docx-ph-none'
$codeBlank = $null
$codeWrong = $null
$codeNone = $null
try { $codeBlank = ($badBlank.Stderr | ConvertFrom-Json).error.code } catch {}
try { $codeWrong = ($badWrong.Stderr | ConvertFrom-Json).error.code } catch {}
try { $codeNone = ($badNone.Stderr | ConvertFrom-Json).error.code } catch {}
$wrote = (Test-Path $failBlank) -or (Test-Path $failWrong) -or (Test-Path $failNone)
Add-Check 'docx-Ph5' (($codeBlank -eq 'InvalidField') -and ($codeWrong -eq 'InvalidField') -and ($codeNone -eq 'NotImplemented') -and (-not $wrote)) "blank=$codeBlank wrong=$codeWrong none=$codeNone wrote=$wrote"
