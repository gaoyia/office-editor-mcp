# pptx 主题 objectDefaults：docs/checklist-pptx.md 的 Pdf1–Pdf5。
# 编号 Pdf，不要和 Tfm、Ts、Th 撞。不算主题排版引擎。不要 Stop-Process PowerPoint。

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

$hostPath = Ensure-PptxCreate
$ppt = Ensure-MsPpt $hostPath

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-pdf-inspect'
$od0 = $null
try { $od0 = $tree0.masters[0].theme.objectDefaults } catch {}
Add-Check 'pptx-Pdf1' ($null -eq $od0) "objectDefaults=$od0"

$lnOut = Get-OutPath 'pptx-theme-defaults-lnw.pptx'
Close-PptPresByPath $ppt $lnOut
if (Test-Path $lnOut) { Remove-Item -Force $lnOut }
$lnJsonPath = Join-Path $global:OoxpReqDir 'pptx-pdf-lnw.json'
$lnJson = '{"schemaVersion":1,"action":"apply","path":"' + $hostPath.Replace('\', '/') + '","output":"' + $lnOut.Replace('\', '/') + '","ops":[{"op":"setThemeObjectDefaults","id":"master:1","objectDefaults":{"sp":{"lnW":38100}}}]}'
[System.IO.File]::WriteAllText($lnJsonPath, $lnJson, $global:OoxpUtf8)
Invoke-Ooxp $lnJsonPath | Out-Null
$treeL = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $lnOut.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-pdf-lnw-inspect'
Close-PptPresByPath $ppt $lnOut
$pres = Open-PptPres $ppt $lnOut
$capL = Get-PptCaption $pres
Close-PptPres $pres
$lnW = $treeL.masters[0].theme.objectDefaults.sp.lnW
Add-Check 'pptx-Pdf2' (
    ($capL -notmatch 'Repair') -and
    ($lnW -eq 38100)
) "caption=$capL lnW=$lnW"

$stOut = Get-OutPath 'pptx-theme-defaults-style.pptx'
Close-PptPresByPath $ppt $stOut
if (Test-Path $stOut) { Remove-Item -Force $stOut }
$stJsonPath = Join-Path $global:OoxpReqDir 'pptx-pdf-style.json'
$stJson = '{"schemaVersion":1,"action":"apply","path":"' + $lnOut.Replace('\', '/') + '","output":"' + $stOut.Replace('\', '/') + '","ops":[{"op":"setThemeObjectDefaults","id":"master:1","objectDefaults":{"sp":{"style":{"lnRef":1,"fontRef":"minor"}}}}]}'
[System.IO.File]::WriteAllText($stJsonPath, $stJson, $global:OoxpUtf8)
Invoke-Ooxp $stJsonPath | Out-Null
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $stOut.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-pdf-style-inspect'
Close-PptPresByPath $ppt $stOut
$pres = Open-PptPres $ppt $stOut
$capS = Get-PptCaption $pres
Close-PptPres $pres
$lnRef = $treeS.masters[0].theme.objectDefaults.sp.style.lnRef
$fontRef = [string]$treeS.masters[0].theme.objectDefaults.sp.style.fontRef
$lnW2 = $treeS.masters[0].theme.objectDefaults.sp.lnW
Add-Check 'pptx-Pdf3' (
    ($capS -notmatch 'Repair') -and
    ($lnRef -eq 1) -and
    ($fontRef -eq 'minor') -and
    ($lnW2 -eq 38100)
) "caption=$capS lnRef=$lnRef fontRef=$fontRef lnW=$lnW2"

$clOut = Get-OutPath 'pptx-theme-defaults-clear.pptx'
Close-PptPresByPath $ppt $clOut
if (Test-Path $clOut) { Remove-Item -Force $clOut }
$clJsonPath = Join-Path $global:OoxpReqDir 'pptx-pdf-clear.json'
$clJson = '{"schemaVersion":1,"action":"apply","path":"' + $stOut.Replace('\', '/') + '","output":"' + $clOut.Replace('\', '/') + '","ops":[{"op":"setThemeObjectDefaults","id":"master:1","objectDefaults":null}]}'
[System.IO.File]::WriteAllText($clJsonPath, $clJson, $global:OoxpUtf8)
Invoke-Ooxp $clJsonPath | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $clOut.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-pdf-clear-inspect'
Close-PptPresByPath $ppt $clOut
$pres = Open-PptPres $ppt $clOut
$capC = Get-PptCaption $pres
Close-PptPres $pres
$odC = $treeC.masters[0].theme.objectDefaults
Add-Check 'pptx-Pdf4' (
    ($capC -notmatch 'Repair') -and
    ($null -eq $odC)
) "caption=$capC objectDefaults=$odC"

$failId = Get-OutPath 'pptx-theme-defaults-bad-id.pptx'
$badId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failId.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeObjectDefaults'; id = 'slide:1'; objectDefaults = @{ sp = @{ lnW = 1 } } }
    )
} 'pptx-pdf-bad-id'
$failMiss = Get-OutPath 'pptx-theme-defaults-missing.pptx'
$miss = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failMiss.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeObjectDefaults'; id = 'master:1' }
    )
} 'pptx-pdf-missing'
$failFont = Get-OutPath 'pptx-theme-defaults-font.pptx'
$font = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failFont.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeObjectDefaults'; id = 'master:1'; objectDefaults = @{ sp = @{ style = @{ fontRef = 'latin' } } } }
    )
} 'pptx-pdf-font'
Add-Check 'pptx-Pdf5' (
    ((Get-PptErrorCode $badId) -eq 'InvalidId') -and
    ((Get-PptErrorCode $miss) -eq 'InvalidField') -and
    ((Get-PptErrorCode $font) -eq 'InvalidField') -and
    -not (Test-Path $failId) -and
    -not (Test-Path $failMiss) -and
    -not (Test-Path $failFont)
) "id=$(Get-PptErrorCode $badId) miss=$(Get-PptErrorCode $miss) font=$(Get-PptErrorCode $font)"
