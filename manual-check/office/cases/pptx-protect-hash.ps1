# pptx 修改密码哈希：docs/checklist-pptx.md 的 Pph1–Pph5。
# 用 create 产物再 apply。编号 Pph，不要和 Pw、Pz、Ph 撞。不破解密码。
# 带哈希或空 verifier 不要 Open-PptPres。解开后再开。

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

function Get-PptErrorCode($raw) {
    try { return ($raw.Stderr | ConvertFrom-Json).error.code } catch { return $null }
}

function Rewrite-ZipPart([string]$zipPath, [string]$part, [scriptblock]$edit) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::Open($zipPath, 'Update')
    try {
        $entry = $zip.Entries | Where-Object { ($_.FullName -replace '\\', '/') -eq $part } | Select-Object -First 1
        if (-not $entry) { throw "zip 里没有 $part" }
        $ms = New-Object IO.MemoryStream
        $es = $entry.Open()
        $es.CopyTo($ms)
        $es.Dispose()
        $text = [Text.Encoding]::UTF8.GetString($ms.ToArray())
        $text2 = & $edit $text
        $name = $entry.FullName
        $entry.Delete()
        $newEntry = $zip.CreateEntry($name)
        $bytes = [Text.Encoding]::UTF8.GetBytes($text2)
        $ws = $newEntry.Open()
        $ws.Write($bytes, 0, $bytes.Length)
        $ws.Dispose()
    }
    finally { $zip.Dispose() }
}

function Get-SaltData([string]$pptx) {
    $xml = Get-ZipPartText $pptx 'ppt/presentation.xml'
    if ($xml -match 'saltData="([^"]+)"') { return $Matches[1] }
    return $null
}

$hostPath = Get-OutPath 'pptx-pph-host.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    presentation = @{ slides = @(@{ shapes = @(@{ text = '页1' }) }) }
} 'pptx-pph-create' | Out-Null

$setOut = Get-OutPath 'pptx-pph-set.pptx'
if (Test-Path $setOut) { Remove-Item -Force $setOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $setOut.Replace('\', '/')
    ops = @(@{
        op = 'setProtection'
        id = 'pptx:presentation'
        enforced = $true
        password = 'secret'
    })
} 'pptx-pph-set' | Out-Null

$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $setOut.Replace('\', '/'); format = 'pptx'
} 'pptx-pph-inspect-set'
$dump = ($treeS | ConvertTo-Json -Depth 12 -Compress)
$xmlS = Get-ZipPartText $setOut 'ppt/presentation.xml'
$pph1ok = ([bool]$treeS.modifyProtection.hasPassword) `
    -and ($null -eq $treeS.modifyProtection.hashData) `
    -and ($dump -notmatch 'secret') `
    -and ($dump -notmatch 'hashData') `
    -and ($xmlS -match 'cryptAlgorithmSid="14"') `
    -and ($xmlS -match 'hashData="') `
    -and ($xmlS -match 'saltData="')
Add-Check 'pptx-Pph1' $pph1ok "hash=$($treeS.modifyProtection.hasPassword)"

$openOut = Get-OutPath 'pptx-pph-open.pptx'
if (Test-Path $openOut) { Remove-Item -Force $openOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $setOut.Replace('\', '/')
    output = $openOut.Replace('\', '/')
    ops = @(@{
        op = 'setProtection'
        id = 'pptx:presentation'
        enforced = $false
        password = 'secret'
    })
} 'pptx-pph-open' | Out-Null
$treeO = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $openOut.Replace('\', '/'); format = 'pptx'
} 'pptx-pph-inspect-open'
$xmlO = Get-ZipPartText $openOut 'ppt/presentation.xml'
$ppt = Ensure-MsPpt $hostPath
Close-PptPresByPath $ppt $openOut
$presO = Open-PptPres $ppt $openOut
$capO = Get-PptCaption $presO
Close-PptPres $presO
$pph2ok = (-not [bool]$treeO.modifyProtection.hasPassword) `
    -and ($xmlO -notmatch 'modifyVerifier') `
    -and ($capO -notmatch 'Repair')
Add-Check 'pptx-Pph2' $pph2ok "hash=$($treeO.modifyProtection.hasPassword) cap=$capO"

$emptyOut = Get-OutPath 'pptx-pph-empty.pptx'
if (Test-Path $emptyOut) { Remove-Item -Force $emptyOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $emptyOut.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'pptx:presentation'; enforced = $true })
} 'pptx-pph-empty' | Out-Null
$treeE = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $emptyOut.Replace('\', '/'); format = 'pptx'
} 'pptx-pph-inspect-empty'
$xmlE = Get-ZipPartText $emptyOut 'ppt/presentation.xml'
$pph3ok = (-not [bool]$treeE.modifyProtection.hasPassword) `
    -and ($xmlE -match '<p:modifyVerifier/>') `
    -and ($xmlE -notmatch 'hashData=')
Add-Check 'pptx-Pph3' $pph3ok "hash=$($treeE.modifyProtection.hasPassword) xmlHas=$($xmlE -match 'modifyVerifier')"

$salt1 = Get-SaltData $setOut
$againOut = Get-OutPath 'pptx-pph-again.pptx'
if (Test-Path $againOut) { Remove-Item -Force $againOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $setOut.Replace('\', '/')
    output = $againOut.Replace('\', '/')
    ops = @(@{
        op = 'setProtection'
        id = 'pptx:presentation'
        enforced = $true
        password = 'secret'
    })
} 'pptx-pph-again' | Out-Null
$treeA = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $againOut.Replace('\', '/'); format = 'pptx'
} 'pptx-pph-inspect-again'
$salt2 = Get-SaltData $againOut
$pph4ok = ([bool]$treeA.modifyProtection.hasPassword) `
    -and ($null -ne $salt1) `
    -and ($null -ne $salt2) `
    -and ($salt1 -ne $salt2)
Add-Check 'pptx-Pph4' $pph4ok "salt1=$salt1 salt2=$salt2"

$failBlank = Get-OutPath 'pptx-pph-blank.pptx'
$failWrong = Get-OutPath 'pptx-pph-wrong.pptx'
$failNoPw = Get-OutPath 'pptx-pph-nopw.pptx'
$failIso = Get-OutPath 'pptx-pph-iso.pptx'
$failIsoOut = Get-OutPath 'pptx-pph-iso-out.pptx'

$blankFail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failBlank.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'pptx:presentation'; enforced = $true; password = '' })
} 'pptx-pph-blank'
$wrongFail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $setOut.Replace('\', '/')
    output = $failWrong.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'pptx:presentation'; enforced = $false; password = 'wrong' })
} 'pptx-pph-wrong'
$nopwFail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $setOut.Replace('\', '/')
    output = $failNoPw.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'pptx:presentation'; enforced = $false })
} 'pptx-pph-nopw'

Copy-Item -Force $hostPath $failIso
Rewrite-ZipPart $failIso 'ppt/presentation.xml' {
    param($xml)
    return $xml.Replace('</p:presentation>', '<p:modifyVerifier algorithmName="SHA-512" hashValue="YWJj" saltValue="ZGVm" spinCount="100000"/></p:presentation>')
}
$isoFail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $failIso.Replace('\', '/')
    output = $failIsoOut.Replace('\', '/')
    ops = @(@{
        op = 'setProtection'
        id = 'pptx:presentation'
        enforced = $false
        password = 'secret'
    })
} 'pptx-pph-iso'

$codeBlank = Get-PptErrorCode $blankFail
$codeWrong = Get-PptErrorCode $wrongFail
$codeNoPw = Get-PptErrorCode $nopwFail
$codeIso = Get-PptErrorCode $isoFail
$pph5ok = ($blankFail.ExitCode -ne 0) -and ($codeBlank -eq 'InvalidField') -and (-not (Test-Path $failBlank)) `
    -and ($wrongFail.ExitCode -ne 0) -and ($codeWrong -eq 'InvalidField') -and (-not (Test-Path $failWrong)) `
    -and ($nopwFail.ExitCode -ne 0) -and ($codeNoPw -eq 'NotImplemented') -and (-not (Test-Path $failNoPw)) `
    -and ($isoFail.ExitCode -ne 0) -and ($codeIso -eq 'NotImplemented') -and (-not (Test-Path $failIsoOut))
Add-Check 'pptx-Pph5' $pph5ok "blank=$codeBlank wrong=$codeWrong nopw=$codeNoPw iso=$codeIso"
