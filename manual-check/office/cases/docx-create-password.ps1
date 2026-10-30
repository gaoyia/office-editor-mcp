# create 的限制编辑口令与 apply 同一套哈希。Word 16.0 能用口令解开。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$path = Get-OutPath 'docx-create-password.docx'
if (Test-Path $path) { Remove-Item -Force $path }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action = 'create'
    path = $path.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(@{ text = 'Locked' })
        protection = @{
            enforced = $true
            edit = 'readOnly'
            password = 'secret'
        }
    }
} 'docx-create-password' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1
    action = 'inspect'
    path = $path.Replace('\', '/')
    format = 'docx'
} 'docx-create-password-inspect'

$word = Ensure-MsWord (Ensure-DocxCreate)
Close-WordDocByPath $word $path
$doc = Open-WordDoc $word $path
$caption = [string]$doc.ActiveWindow.Caption
$ptype = [int]$doc.ProtectionType
$wrongFailed = $false
try {
    $doc.Unprotect('wrong')
} catch {
    $wrongFailed = $true
}
$opened = $false
try {
    $doc.Unprotect('secret')
    $opened = ([int]$doc.ProtectionType -eq -1)
} catch {
    Write-Log ("Unprotect secret FAIL " + $_.Exception.Message)
}
Close-WordDoc $doc
$ok = ([bool]$tree.protection.enforced) -and ([bool]$tree.protection.hasPassword) `
    -and ($ptype -eq 3) -and $wrongFailed -and $opened -and ($caption -notmatch 'Repair')
Add-Check 'docx-create-password' $ok "type=$ptype wrong=$wrongFailed opened=$opened caption=$caption"
