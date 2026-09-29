# pptx 修改密码与放映保护：docs/checklist-pptx.md 的 Pw1–Pw5。
# 不算放映引擎。不写哈希。带哈希样例只 ZIP 注入，不要 Open-PptPres。

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

function Get-ShowSettings($pres) {
    $sss = $pres.SlideShowSettings
    try {
        return @{
            ShowType = [int]$sss.ShowType
            Loop = [int]$sss.LoopUntilStopped
        }
    } finally { Release-Com $sss }
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

$tmpDir = Join-Path $env:TEMP 'ooxp-office'
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
$src = Join-Path $tmpDir 'pptx-protect-src.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'pptx'
    presentation = @{ slides = @(@{ shapes = @(@{ text = '页1' }) }) }
} 'pptx-pw-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
} 'pptx-pw-inspect-bare'
$sp0 = $tree0.showPr
$mp0 = $tree0.modifyProtection
$pw1ok = ([string]$sp0.type -eq 'present') `
    -and ([bool]$sp0.loop -eq $false) `
    -and ([bool]$sp0.showNarration -eq $true) `
    -and ([bool]$sp0.showAnimation -eq $true) `
    -and ([bool]$sp0.useTimings -eq $true) `
    -and ([bool]$mp0.hasPassword -eq $false)
Add-Check 'pptx-Pw1' $pw1ok "type=$($sp0.type) loop=$($sp0.loop) narr=$($sp0.showNarration) hash=$($mp0.hasPassword)"

$hostPath = Ensure-PptxCreate
$ppt = Ensure-MsPpt $hostPath

$kioskOut = Get-OutPath 'pptx-protect-kiosk.pptx'
Close-PptPresByPath $ppt $kioskOut
if (Test-Path $kioskOut) { Remove-Item -Force $kioskOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $kioskOut.Replace('\', '/')
    ops = @(@{ op = 'setShowPr'; id = 'pptx:presentation'; type = 'kiosk'; loop = $true })
} 'pptx-pw-kiosk' | Out-Null
$presK = Open-PptPres $ppt $kioskOut
$capK = Get-PptCaption $presK
$snapK = Get-ShowSettings $presK
Close-PptPres $presK
$pw2ok = ($capK -notmatch 'Repair') `
    -and ([int]$snapK.ShowType -eq 3) `
    -and ([int]$snapK.Loop -ne 0)
Add-Check 'pptx-Pw2' $pw2ok "cap=$capK ShowType=$($snapK.ShowType) Loop=$($snapK.Loop)"

$browseOut = Get-OutPath 'pptx-protect-browse.pptx'
Close-PptPresByPath $ppt $browseOut
if (Test-Path $browseOut) { Remove-Item -Force $browseOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $kioskOut.Replace('\', '/')
    output = $browseOut.Replace('\', '/')
    ops = @(@{ op = 'setShowPr'; id = 'pptx:presentation'; type = 'browse'; loop = $false })
} 'pptx-pw-browse' | Out-Null
$presB = Open-PptPres $ppt $browseOut
$capB = Get-PptCaption $presB
$snapB = Get-ShowSettings $presB
Close-PptPres $presB
$pw3ok = ($capB -notmatch 'Repair') -and ([int]$snapB.ShowType -eq 2)
Add-Check 'pptx-Pw3' $pw3ok "cap=$capB ShowType=$($snapB.ShowType)"

$presentOut = Get-OutPath 'pptx-protect-present.pptx'
Close-PptPresByPath $ppt $presentOut
if (Test-Path $presentOut) { Remove-Item -Force $presentOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $browseOut.Replace('\', '/')
    output = $presentOut.Replace('\', '/')
    ops = @(@{ op = 'setShowPr'; id = 'pptx:presentation'; type = 'present'; loop = $false })
} 'pptx-pw-present' | Out-Null
$presP = Open-PptPres $ppt $presentOut
$capP = Get-PptCaption $presP
$snapP = Get-ShowSettings $presP
Close-PptPres $presP
$pw4ok = ($capP -notmatch 'Repair') -and ([int]$snapP.ShowType -eq 1)
Add-Check 'pptx-Pw4' $pw4ok "cap=$capP ShowType=$($snapP.ShowType)"

$failPw = Get-OutPath 'pptx-protect-bad-password.pptx'
$failEn = Get-OutPath 'pptx-protect-bad-enforced.pptx'
$failId = Get-OutPath 'pptx-protect-bad-id.pptx'
$failX = Get-OutPath 'pptx-protect-fail-xlsx.xlsx'
$hashPath = Get-OutPath 'pptx-protect-hash.pptx'
$hashOut = Get-OutPath 'pptx-protect-hash-out.pptx'
Copy-Item -Force $src $hashPath
Rewrite-ZipPart $hashPath 'ppt/presentation.xml' {
    param($xml)
    return $xml.Replace('</p:presentation>', '<p:modifyVerifier algorithmName="SHA-512" hashValue="YWJj" saltValue="ZGVm" spinCount="100000"/></p:presentation>')
}
$hashTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hashPath.Replace('\', '/'); format = 'pptx'
} 'pptx-pw-hash-inspect'
$hashFlag = [bool]$hashTree.modifyProtection.hasPassword
$hashFail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hashPath.Replace('\', '/')
    output = $hashOut.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'pptx:presentation'; enforced = $false })
} 'pptx-pw-hash-apply'

$pwFail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failPw.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'pptx:presentation'; enforced = $false; password = 'secret' })
} 'pptx-pw-password'
$enFail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failEn.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'pptx:presentation'; enforced = $true })
} 'pptx-pw-enforced'
$idFail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failId.Replace('\', '/')
    ops = @(@{ op = 'setShowPr'; id = 'slide:1'; loop = $true })
} 'pptx-pw-bad-id'

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $failX.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'pptx-pw-xlsx-create' | Out-Null
$xFail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $failX.Replace('\', '/')
    output = (Get-OutPath 'pptx-protect-fail-xlsx-out.xlsx').Replace('\', '/')
    ops = @(@{ op = 'setShowPr'; id = 'pptx:presentation'; loop = $true })
} 'pptx-pw-xlsx'

$codeHash = Get-PptErrorCode $hashFail
$codePw = Get-PptErrorCode $pwFail
$codeEn = Get-PptErrorCode $enFail
$codeId = Get-PptErrorCode $idFail
$codeX = Get-PptErrorCode $xFail
$pw5ok = $hashFlag `
    -and ($hashFail.ExitCode -ne 0) -and ($codeHash -eq 'NotImplemented') -and (-not (Test-Path $hashOut)) `
    -and ($pwFail.ExitCode -ne 0) -and ($codePw -eq 'NotImplemented') -and (-not (Test-Path $failPw)) `
    -and ($enFail.ExitCode -ne 0) -and ($codeEn -eq 'NotImplemented') -and (-not (Test-Path $failEn)) `
    -and ($idFail.ExitCode -ne 0) -and ($codeId -eq 'InvalidId') -and (-not (Test-Path $failId)) `
    -and ($xFail.ExitCode -ne 0) -and ($codeX -eq 'InvalidField')
Add-Check 'pptx-Pw5' $pw5ok "hash=$hashFlag hashCode=$codeHash pw=$codePw en=$codeEn id=$codeId x=$codeX"
