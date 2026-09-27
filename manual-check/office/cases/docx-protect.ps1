# docx 文档保护：docs/checklist-docx.md 的 Pr1–Pr6。
# Pr1 / Pr4 / Pr5 的样例由 Word 另存。不要用 create 产物当 Pr1 唯一样例。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$ProtectPassword = 'ooxp-secret'

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

function New-ProtectSkeleton([string]$name) {
    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $skel = Join-Path $tmpDir $name
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'create'
        path = $skel.Replace('\', '/'); format = 'docx'
        document = @{
            paragraphs = @(
                @{ style = 'Heading1'; text = $S.chapter1 }
                @{ text = $S.hello }
            )
        }
    } ("docx-protect-skel-" + [IO.Path]::GetFileNameWithoutExtension($name)) | Out-Null
    return $skel
}

function Save-WordCopy($doc, [string]$dest) {
    Write-Log ("word Save() 原地覆盖 TEMP 再复制到 " + $dest)
    $doc.Save()
    $full = [string]$doc.FullName
    Close-WordDoc $doc
    Copy-Item -Force $full $dest
}

function Ensure-DocxWordReadonlySource([switch]$Force) {
    $path = Get-OutPath 'docx-word-protect-readonly.docx'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $skel = New-ProtectSkeleton 'docx-word-protect-ro-skel.docx'
    $word = Ensure-MsWord (Ensure-DocxCreate)
    Close-WordDocByPath $word $skel
    $doc = Open-WordDoc $word $skel
    $doc.Protect(3)
    Save-WordCopy $doc $path
    Write-Log "saved word readonly protect $path"
    return $path
}

function Ensure-DocxWordRecommendedSource([switch]$Force) {
    $path = Get-OutPath 'docx-word-protect-recommended.docx'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $skel = New-ProtectSkeleton 'docx-word-protect-ror-skel.docx'
    $word = Ensure-MsWord (Ensure-DocxCreate)
    Close-WordDocByPath $word $skel
    $doc = Open-WordDoc $word $skel
    $doc.ReadOnlyRecommended = $true
    Save-WordCopy $doc $path
    Write-Log "saved word read-only recommended $path"
    return $path
}

function Ensure-DocxWordPasswordSource([switch]$Force) {
    $path = Get-OutPath 'docx-word-protect-password.docx'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $skel = New-ProtectSkeleton 'docx-word-protect-pw-skel.docx'
    $word = Ensure-MsWord (Ensure-DocxCreate)
    Close-WordDocByPath $word $skel
    $doc = Open-WordDoc $word $skel
    $doc.Protect(3, $false, $ProtectPassword)
    Save-WordCopy $doc $path
    Write-Log "saved word password protect $path"
    return $path
}

$roSrc = Ensure-DocxWordReadonlySource
$rorSrc = Ensure-DocxWordRecommendedSource
$pwSrc = Ensure-DocxWordPasswordSource
$word = Ensure-MsWord (Ensure-DocxCreate)
Close-WordDocByPath $word $roSrc
Close-WordDocByPath $word $rorSrc
Close-WordDocByPath $word $pwSrc

# Pr1：Word 无密码只读保护另存后 inspect。
$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $roSrc.Replace('\', '/'); format = 'docx'
} 'docx-protect-inspect-ro'
$hasProt = $tree1.PSObject.Properties.Name -contains 'protection'
$pr1ok = $hasProt -and ([bool]$tree1.protection.enforced) -and ([string]$tree1.protection.edit -eq 'readOnly') -and (-not [bool]$tree1.protection.hasPassword)
Add-Check 'docx-Pr1' $pr1ok "enforced=$($tree1.protection.enforced) edit=$($tree1.protection.edit) hasPassword=$($tree1.protection.hasPassword)"

# Pr2：关掉后 Word ProtectionType 为 -1，无修复。
$offOut = Get-OutPath 'docx-protect-off.docx'
Close-WordDocByPath $word $offOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $roSrc.Replace('\', '/')
    output = $offOut.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'docx:document'; enforced = $false })
} 'docx-protect-off' | Out-Null
$docOff = Open-WordDoc $word $offOut
$ptypeOff = [int]$docOff.ProtectionType
$capOff = [string]$docOff.ActiveWindow.Caption
Close-WordDoc $docOff
Add-Check 'docx-Pr2' (($ptypeOff -eq -1) -and ($capOff -notmatch 'Repair')) "ProtectionType=$ptypeOff caption=$capOff"

# Pr3：打开窗体保护后 Word ProtectionType 为 2，无修复。
$hostPath = Get-OutPath 'docx-protect-forms-host.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = $S.chapter1 }
            @{ text = $S.hello }
        )
    }
} 'docx-protect-forms-host' | Out-Null
$formsOut = Get-OutPath 'docx-protect-forms.docx'
Close-WordDocByPath $word $formsOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $formsOut.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'docx:document'; enforced = $true; edit = 'forms' })
} 'docx-protect-forms' | Out-Null
$docF = Open-WordDoc $word $formsOut
$ptypeF = [int]$docF.ProtectionType
$capF = [string]$docF.ActiveWindow.Caption
Close-WordDoc $docF
Add-Check 'docx-Pr3' (($ptypeF -eq 2) -and ($capF -notmatch 'Repair')) "ProtectionType=$ptypeF caption=$capF"

# Pr4：Word 只读推荐。
$tree4 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $rorSrc.Replace('\', '/'); format = 'docx'
} 'docx-protect-inspect-ror'
$pr4ok = [bool]$tree4.protection.readOnlyRecommended
Add-Check 'docx-Pr4' $pr4ok "readOnlyRecommended=$($tree4.protection.readOnlyRecommended)"

# Pr5：带密码 inspect 不报明文；改 enforced 得 NotImplemented。
$tree5 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $pwSrc.Replace('\', '/'); format = 'docx'
} 'docx-protect-inspect-pw'
$tree5json = $tree5 | ConvertTo-Json -Compress -Depth 20
$hasPw = [bool]$tree5.protection.hasPassword
$noPlain = $tree5json -notmatch [regex]::Escape($ProtectPassword)
$failPw = Get-OutPath 'docx-protect-pw-off.docx'
if (Test-Path $failPw) { Remove-Item -Force $failPw }
$badPw = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $pwSrc.Replace('\', '/')
    output = $failPw.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'docx:document'; enforced = $false })
} 'docx-protect-pw-off'
$codePw = $null
try { $codePw = ($badPw.Stderr | ConvertFrom-Json).error.code } catch {}
$wrotePw = Test-Path $failPw
Add-Check 'docx-Pr5' ($hasPw -and $noPlain -and ($codePw -eq 'NotImplemented') -and (-not $wrotePw)) "hasPassword=$hasPw noPlain=$noPlain code=$codePw wrote=$wrotePw"

# Pr6：错 id 与空字段。
$failId = Get-OutPath 'docx-protect-bad-id.docx'
$failEmpty = Get-OutPath 'docx-protect-empty.docx'
if (Test-Path $failId) { Remove-Item -Force $failId }
if (Test-Path $failEmpty) { Remove-Item -Force $failEmpty }
$badId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failId.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'para:00000001'; enforced = $true })
} 'docx-protect-bad-id'
$badEmpty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failEmpty.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'docx:document' })
} 'docx-protect-empty'
$codeId = $null
$codeEmpty = $null
try { $codeId = ($badId.Stderr | ConvertFrom-Json).error.code } catch {}
try { $codeEmpty = ($badEmpty.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'docx-Pr6' (($codeId -eq 'InvalidId') -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failId))) "id=$codeId empty=$codeEmpty wrote=$([bool](Test-Path $failId))"
