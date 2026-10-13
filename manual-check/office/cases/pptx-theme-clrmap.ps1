# pptx 配色映射与页级覆盖：docs/checklist-pptx.md 的 Tcm1–Tcm5。
# 编号 Tcm，不要和 Ts、Tfm、Pcm、Tc 撞。不算主题排版引擎。不要 Stop-Process PowerPoint。

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

$hostPath = Get-OutPath 'pptx-tcm-src.pptx'
if (Test-Path $hostPath) { Remove-Item -Force $hostPath }
$createReq = Join-Path $global:OoxpReqDir 'pptx-tcm-create.json'
$createJson = '{"schemaVersion":1,"action":"create","path":"' + $hostPath.Replace('\', '/') + '","presentation":{"handout":{"header":"讲义头"},"slides":[{"shapes":[{"text":"标题"}],"notes":"备注"}]}}'
[System.IO.File]::WriteAllText($createReq, $createJson, $global:OoxpUtf8)
Invoke-Ooxp $createReq | Out-Null

$ppt = Ensure-MsPpt $hostPath

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true; notesMaster = $true; handout = $true; notes = $true }
} 'pptx-tcm-inspect'
$masterKeys = @()
$bg1 = $null
$tx1 = $null
$ovr0 = $null
$th0 = $null
try {
    $masterKeys = @($tree0.masters[0].PSObject.Properties.Name)
    $bg1 = $tree0.masters[0].clrMap.bg1
    $tx1 = $tree0.masters[0].clrMap.tx1
    $ovr0 = [string]$tree0.slides[0].clrMapOvr
    $th0 = $tree0.slides[0].themeOverride
} catch {}
$themeIdx = [Array]::IndexOf($masterKeys, 'theme')
$clrIdx = [Array]::IndexOf($masterKeys, 'clrMap')
$tcm1ok = ($clrIdx -eq ($themeIdx + 1)) -and ($bg1 -eq 'lt1') -and ($tx1 -eq 'dk1') `
    -and ($ovr0 -eq 'master') -and ($null -eq $th0)
Add-Check 'pptx-Tcm1' $tcm1ok "keys=$($masterKeys -join ',') bg1=$bg1 tx1=$tx1 ovr=$ovr0 th=$th0"

$mapOut = Get-OutPath 'pptx-tcm-map.pptx'
Close-PptPresByPath $ppt $mapOut
if (Test-Path $mapOut) { Remove-Item -Force $mapOut }
$mapReq = Join-Path $global:OoxpReqDir 'pptx-tcm-map.json'
$mapJson = '{"schemaVersion":1,"action":"apply","path":"' + $hostPath.Replace('\', '/') + '","output":"' + $mapOut.Replace('\', '/') + '","ops":[{"op":"setClrMap","id":"master:1","bg1":"dk1"},{"op":"setClrMap","id":"notesMaster","tx1":"lt1"},{"op":"setClrMap","id":"handout","bg2":"dk2"}]}'
[System.IO.File]::WriteAllText($mapReq, $mapJson, $global:OoxpUtf8)
Invoke-Ooxp $mapReq | Out-Null
$treeM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $mapOut.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true; notesMaster = $true; handout = $true }
} 'pptx-tcm-map-inspect'
$mbg = $null; $ntx = $null; $hbg = $null
try {
    $mbg = $treeM.masters[0].clrMap.bg1
    $ntx = $treeM.notesMaster.clrMap.tx1
    $hbg = $treeM.handout.clrMap.bg2
} catch {}
$presM = Open-PptPres $ppt $mapOut
$capM = Get-PptCaption $presM
Close-PptPres $presM
$tcm2ok = ($mbg -eq 'dk1') -and ($ntx -eq 'lt1') -and ($hbg -eq 'dk2') -and ($capM -notmatch 'Repair')
Add-Check 'pptx-Tcm2' $tcm2ok "master.bg1=$mbg notes.tx1=$ntx handout.bg2=$hbg cap=$capM"

$ovrOut = Get-OutPath 'pptx-tcm-ovr.pptx'
Close-PptPresByPath $ppt $ovrOut
if (Test-Path $ovrOut) { Remove-Item -Force $ovrOut }
$ovrReq = Join-Path $global:OoxpReqDir 'pptx-tcm-ovr.json'
$ovrJson = '{"schemaVersion":1,"action":"apply","path":"' + $hostPath.Replace('\', '/') + '","output":"' + $ovrOut.Replace('\', '/') + '","ops":[{"op":"setClrMapOvr","id":"slide:1","clrMapOvr":{"bg1":"dk1"}},{"op":"setClrMapOvr","id":"master:1/layout:1","clrMapOvr":{"tx1":"lt1"}}]}'
[System.IO.File]::WriteAllText($ovrReq, $ovrJson, $global:OoxpUtf8)
Invoke-Ooxp $ovrReq | Out-Null
$treeO = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ovrOut.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-tcm-ovr-inspect'
$sbg = $null; $ltx = $null
try {
    $sbg = $treeO.slides[0].clrMapOvr.bg1
    $ltx = $treeO.masters[0].layouts[0].clrMapOvr.tx1
} catch {}
$masterBack = Get-OutPath 'pptx-tcm-ovr-master.pptx'
Close-PptPresByPath $ppt $masterBack
if (Test-Path $masterBack) { Remove-Item -Force $masterBack }
$backReq = Join-Path $global:OoxpReqDir 'pptx-tcm-ovr-back.json'
$backJson = '{"schemaVersion":1,"action":"apply","path":"' + $ovrOut.Replace('\', '/') + '","output":"' + $masterBack.Replace('\', '/') + '","ops":[{"op":"setClrMapOvr","id":"slide:1","clrMapOvr":"master"}]}'
[System.IO.File]::WriteAllText($backReq, $backJson, $global:OoxpUtf8)
Invoke-Ooxp $backReq | Out-Null
$treeB = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $masterBack.Replace('\', '/'); format = 'pptx'
} 'pptx-tcm-ovr-back-inspect'
$back = [string]$treeB.slides[0].clrMapOvr
$presO = Open-PptPres $ppt $ovrOut
$capO = Get-PptCaption $presO
Close-PptPres $presO
$tcm3ok = ($sbg -eq 'dk1') -and ($ltx -eq 'lt1') -and ($back -eq 'master') -and ($capO -notmatch 'Repair')
Add-Check 'pptx-Tcm3' $tcm3ok "slide.bg1=$sbg layout.tx1=$ltx back=$back cap=$capO"

$thOut = Get-OutPath 'pptx-tcm-theme.pptx'
Close-PptPresByPath $ppt $thOut
if (Test-Path $thOut) { Remove-Item -Force $thOut }
$thReq = Join-Path $global:OoxpReqDir 'pptx-tcm-theme.json'
$thJson = '{"schemaVersion":1,"action":"apply","path":"' + $hostPath.Replace('\', '/') + '","output":"' + $thOut.Replace('\', '/') + '","ops":[{"op":"setThemeOverride","id":"slide:1","accent1":"FF0000"},{"op":"setThemeOverride","id":"slide:1/notes","accent1":"00FF00"}]}'
[System.IO.File]::WriteAllText($thReq, $thJson, $global:OoxpUtf8)
Invoke-Ooxp $thReq | Out-Null
$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $thOut.Replace('\', '/'); format = 'pptx'
    scope = @{ notes = $true }
} 'pptx-tcm-theme-inspect'
$acc = $null; $nacc = $null
try {
    $acc = $treeT.slides[0].themeOverride.colors.accent1
    $nacc = $treeT.slides[0].notes.themeOverride.colors.accent1
} catch {}
$clearOut = Get-OutPath 'pptx-tcm-theme-clear.pptx'
Close-PptPresByPath $ppt $clearOut
if (Test-Path $clearOut) { Remove-Item -Force $clearOut }
$clearReq = Join-Path $global:OoxpReqDir 'pptx-tcm-theme-clear.json'
$clearJson = '{"schemaVersion":1,"action":"apply","path":"' + $thOut.Replace('\', '/') + '","output":"' + $clearOut.Replace('\', '/') + '","ops":[{"op":"setThemeOverride","id":"slide:1","enabled":false}]}'
[System.IO.File]::WriteAllText($clearReq, $clearJson, $global:OoxpUtf8)
Invoke-Ooxp $clearReq | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $clearOut.Replace('\', '/'); format = 'pptx'
} 'pptx-tcm-theme-clear-inspect'
$cleared = $treeC.slides[0].themeOverride
$presT = Open-PptPres $ppt $thOut
$capT = Get-PptCaption $presT
Close-PptPres $presT
$tcm4ok = ($acc -eq 'FF0000') -and ($nacc -eq '00FF00') -and ($null -eq $cleared) -and ($capT -notmatch 'Repair')
Add-Check 'pptx-Tcm4' $tcm4ok "accent1=$acc notes=$nacc cleared=$cleared cap=$capT"

$failSlideOut = Get-OutPath 'pptx-tcm-fail-slide.pptx'
$failEmptyOut = Get-OutPath 'pptx-tcm-fail-empty.pptx'
$failTokOut = Get-OutPath 'pptx-tcm-fail-token.pptx'
$failOvOut = Get-OutPath 'pptx-tcm-fail-override.pptx'
foreach ($p in @($failSlideOut, $failEmptyOut, $failTokOut, $failOvOut)) {
    if (Test-Path $p) { Remove-Item -Force $p }
}
$failSlide = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failSlideOut.Replace('\', '/')
    ops = @(@{ op = 'setClrMap'; id = 'slide:1'; bg1 = 'dk1' })
} 'pptx-tcm-fail-slide'
$failEmpty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failEmptyOut.Replace('\', '/')
    ops = @(@{ op = 'setClrMap'; id = 'master:1' })
} 'pptx-tcm-fail-empty'
$failTok = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failTokOut.Replace('\', '/')
    ops = @(@{ op = 'setClrMap'; id = 'master:1'; bg1 = 'nope' })
} 'pptx-tcm-fail-token'
$failOv = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOvOut.Replace('\', '/')
    ops = @(@{ op = 'setThemeOverride'; id = 'master:1'; accent1 = 'FF0000' })
} 'pptx-tcm-fail-override'
$tcm5ok = ((Get-PptErrorCode $failSlide) -eq 'InvalidId') `
    -and ((Get-PptErrorCode $failEmpty) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failTok) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failOv) -eq 'InvalidId') `
    -and (-not (Test-Path $failSlideOut)) `
    -and (-not (Test-Path $failEmptyOut)) `
    -and (-not (Test-Path $failTokOut)) `
    -and (-not (Test-Path $failOvOut))
Add-Check 'pptx-Tcm5' $tcm5ok "slide=$(Get-PptErrorCode $failSlide) empty=$(Get-PptErrorCode $failEmpty) tok=$(Get-PptErrorCode $failTok) ov=$(Get-PptErrorCode $failOv)"
