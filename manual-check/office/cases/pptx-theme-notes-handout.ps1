# pptx 备注与讲义主题：docs/checklist-pptx.md 的 Tnh1–Tnh5。
# 编号 Tnh，不要和 Tfm、Tcm、Ts 撞。不算主题排版引擎。不要 Stop-Process PowerPoint。

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

$hostPath = Get-OutPath 'pptx-tnh-src.pptx'
if (Test-Path $hostPath) { Remove-Item -Force $hostPath }
$createReq = Join-Path $global:OoxpReqDir 'pptx-tnh-create.json'
$createJson = '{"schemaVersion":1,"action":"create","path":"' + $hostPath.Replace('\', '/') + '","presentation":{"handout":{"header":"讲义头"},"slides":[{"shapes":[{"text":"标题"}],"notes":"备注"}]}}'
[System.IO.File]::WriteAllText($createReq, $createJson, $global:OoxpUtf8)
Invoke-Ooxp $createReq | Out-Null

$ppt = Ensure-MsPpt $hostPath

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true; notesMaster = $true; handout = $true }
} 'pptx-tnh-inspect'
$nKeys = @(); $hKeys = @()
$nAcc = $null; $hAcc = $null; $mAcc = $null
try {
    $nKeys = @($tree0.notesMaster.PSObject.Properties.Name)
    $hKeys = @($tree0.handout.PSObject.Properties.Name)
    $nAcc = [string]$tree0.notesMaster.theme.colors.accent1
    $hAcc = [string]$tree0.handout.theme.colors.accent1
    $mAcc = [string]$tree0.masters[0].theme.colors.accent1
} catch {}
$nThemeIdx = [Array]::IndexOf($nKeys, 'theme')
$nClrIdx = [Array]::IndexOf($nKeys, 'clrMap')
$hThemeIdx = [Array]::IndexOf($hKeys, 'theme')
$hClrIdx = [Array]::IndexOf($hKeys, 'clrMap')
$tnh1ok = ($nThemeIdx -eq 1) -and ($nClrIdx -eq 2) -and ($hThemeIdx -eq 1) -and ($hClrIdx -eq 2) `
    -and ($nAcc -eq $mAcc) -and ($hAcc -eq $mAcc) -and ($nAcc -ne '')
Add-Check 'pptx-Tnh1' $tnh1ok "nKeys=$($nKeys -join ',') hKeys=$($hKeys -join ',') n=$nAcc h=$hAcc m=$mAcc"

$colorOut = Get-OutPath 'pptx-tnh-colors.pptx'
Close-PptPresByPath $ppt $colorOut
if (Test-Path $colorOut) { Remove-Item -Force $colorOut }
$colorReq = Join-Path $global:OoxpReqDir 'pptx-tnh-colors.json'
$colorJson = '{"schemaVersion":1,"action":"apply","path":"' + $hostPath.Replace('\', '/') + '","output":"' + $colorOut.Replace('\', '/') + '","ops":[{"op":"setThemeColors","id":"notesMaster","accent1":"FF0000"},{"op":"setThemeColors","id":"handout","accent1":"00FF00"}]}'
[System.IO.File]::WriteAllText($colorReq, $colorJson, $global:OoxpUtf8)
Invoke-Ooxp $colorReq | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $colorOut.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true; notesMaster = $true; handout = $true }
} 'pptx-tnh-colors-inspect'
$n2 = $null; $h2 = $null; $m2 = $null
try {
    $n2 = [string]$treeC.notesMaster.theme.colors.accent1
    $h2 = [string]$treeC.handout.theme.colors.accent1
    $m2 = [string]$treeC.masters[0].theme.colors.accent1
} catch {}
$presC = Open-PptPres $ppt $colorOut
$capC = Get-PptCaption $presC
Close-PptPres $presC
$tnh2ok = ($n2 -eq 'FF0000') -and ($h2 -eq '00FF00') -and ($m2 -eq $mAcc) -and ($capC -notmatch 'Repair')
Add-Check 'pptx-Tnh2' $tnh2ok "n=$n2 h=$h2 m=$m2 cap=$capC"

$fmtOut = Get-OutPath 'pptx-tnh-fmt.pptx'
Close-PptPresByPath $ppt $fmtOut
if (Test-Path $fmtOut) { Remove-Item -Force $fmtOut }
$fmtReq = Join-Path $global:OoxpReqDir 'pptx-tnh-fmt.json'
$fmtJson = '{"schemaVersion":1,"action":"apply","path":"' + $hostPath.Replace('\', '/') + '","output":"' + $fmtOut.Replace('\', '/') + '","ops":[{"op":"setThemeFonts","id":"notesMaster","majorLatin":"Arial"},{"op":"setThemeFmt","id":"notesMaster","line":{"index":0,"w":38100}}]}'
[System.IO.File]::WriteAllText($fmtReq, $fmtJson, $global:OoxpUtf8)
Invoke-Ooxp $fmtReq | Out-Null
$treeF = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $fmtOut.Replace('\', '/'); format = 'pptx'
    scope = @{ notesMaster = $true }
} 'pptx-tnh-fmt-inspect'
$face = $null; $lw = $null
try {
    $face = [string]$treeF.notesMaster.theme.fonts.majorLatin
    $lw = [int]$treeF.notesMaster.theme.fmt.lines[0].w
} catch {}
$presF = Open-PptPres $ppt $fmtOut
$capF = Get-PptCaption $presF
Close-PptPres $presF
$tnh3ok = ($face -eq 'Arial') -and ($lw -eq 38100) -and ($capF -notmatch 'Repair')
Add-Check 'pptx-Tnh3' $tnh3ok "face=$face w=$lw cap=$capF"

$extraOut = Get-OutPath 'pptx-tnh-extra.pptx'
Close-PptPresByPath $ppt $extraOut
if (Test-Path $extraOut) { Remove-Item -Force $extraOut }
$extraReq = Join-Path $global:OoxpReqDir 'pptx-tnh-extra.json'
$extraJson = '{"schemaVersion":1,"action":"apply","path":"' + $hostPath.Replace('\', '/') + '","output":"' + $extraOut.Replace('\', '/') + '","ops":[{"op":"setThemeExtraSchemes","id":"handout","extraSchemes":[{"name":"Extra","colors":{"accent1":"0F6FC6"}}]}]}'
[System.IO.File]::WriteAllText($extraReq, $extraJson, $global:OoxpUtf8)
Invoke-Ooxp $extraReq | Out-Null
$treeE = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $extraOut.Replace('\', '/'); format = 'pptx'
    scope = @{ handout = $true }
} 'pptx-tnh-extra-inspect'
$en = $null; $ea = $null
try {
    $en = [string]$treeE.handout.theme.extraSchemes[0].name
    $ea = [string]$treeE.handout.theme.extraSchemes[0].colors.accent1
} catch {}
$presE = Open-PptPres $ppt $extraOut
$capE = Get-PptCaption $presE
Close-PptPres $presE
$tnh4ok = ($en -eq 'Extra') -and ($ea -eq '0F6FC6') -and ($capE -notmatch 'Repair')
Add-Check 'pptx-Tnh4' $tnh4ok "name=$en accent1=$ea cap=$capE"

$failSlideOut = Get-OutPath 'pptx-tnh-fail-slide.pptx'
$failMissOut = Get-OutPath 'pptx-tnh-fail-missing.pptx'
$failXlsxOut = Get-OutPath 'pptx-tnh-fail.xlsx'
foreach ($p in @($failSlideOut, $failMissOut, $failXlsxOut)) {
    if (Test-Path $p) { Remove-Item -Force $p }
}
$failSlide = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failSlideOut.Replace('\', '/')
    ops = @(@{ op = 'setThemeColors'; id = 'slide:1'; accent1 = 'FF0000' })
} 'pptx-tnh-fail-slide'

$plainPath = Get-OutPath 'pptx-tnh-plain.pptx'
if (Test-Path $plainPath) { Remove-Item -Force $plainPath }
$plainReq = Join-Path $global:OoxpReqDir 'pptx-tnh-plain.json'
$plainJson = '{"schemaVersion":1,"action":"create","path":"' + $plainPath.Replace('\', '/') + '","presentation":{"slides":[{"shapes":[{"text":"标题"}]}]}}'
[System.IO.File]::WriteAllText($plainReq, $plainJson, $global:OoxpUtf8)
Invoke-Ooxp $plainReq | Out-Null
$failMiss = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $plainPath.Replace('\', '/')
    output = $failMissOut.Replace('\', '/')
    ops = @(@{ op = 'setThemeColors'; id = 'notesMaster'; accent1 = 'FF0000' })
} 'pptx-tnh-fail-missing'

$xlsxPath = Get-OutPath 'pptx-tnh.xlsx'
if (Test-Path $xlsxPath) { Remove-Item -Force $xlsxPath }
$xlsxReq = Join-Path $global:OoxpReqDir 'pptx-tnh-xlsx.json'
$xlsxJson = '{"schemaVersion":1,"action":"create","path":"' + $xlsxPath.Replace('\', '/') + '","workbook":{"sheets":[{"name":"Sheet1","cells":[{"ref":"A1","value":1}]}]}}'
[System.IO.File]::WriteAllText($xlsxReq, $xlsxJson, $global:OoxpUtf8)
Invoke-Ooxp $xlsxReq | Out-Null
$failXlsx = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsxPath.Replace('\', '/')
    output = $failXlsxOut.Replace('\', '/')
    ops = @(@{ op = 'setThemeColors'; id = 'notesMaster'; accent1 = 'FF0000' })
} 'pptx-tnh-fail-xlsx'

$tnh5ok = ((Get-PptErrorCode $failSlide) -eq 'InvalidId') `
    -and ((Get-PptErrorCode $failMiss) -eq 'IdNotFound') `
    -and ((Get-PptErrorCode $failXlsx) -eq 'InvalidId') `
    -and (-not (Test-Path $failSlideOut)) `
    -and (-not (Test-Path $failMissOut)) `
    -and (-not (Test-Path $failXlsxOut))
Add-Check 'pptx-Tnh5' $tnh5ok "slide=$(Get-PptErrorCode $failSlide) miss=$(Get-PptErrorCode $failMiss) xlsx=$(Get-PptErrorCode $failXlsx)"
