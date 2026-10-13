# pptx 主题剩余：docs/checklist-pptx.md 的 Tfm1–Tfm5。
# 编号 Tfm，不要和 Ts、Tf、Tfl、Fm 撞。不算主题排版引擎。不要 Stop-Process PowerPoint。

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
} 'pptx-tfm-inspect'
$theme0 = $null
$themeKeys = @()
$fontKeys = @()
$fmtKeys = @()
$extra0 = $null
try {
    $theme0 = $tree0.masters[0].theme
    $themeKeys = @($theme0.PSObject.Properties.Name)
    $fontKeys = @($theme0.fonts.PSObject.Properties.Name)
    $fmtKeys = @($theme0.fmt.PSObject.Properties.Name)
    $extra0 = @($theme0.extraSchemes)
} catch {}
$tfm1ok = ($themeKeys.Count -eq 5) `
    -and ($themeKeys[0] -eq 'name') `
    -and ($themeKeys[1] -eq 'colors') `
    -and ($themeKeys[2] -eq 'fonts') `
    -and ($themeKeys[3] -eq 'fmt') `
    -and ($themeKeys[4] -eq 'extraSchemes') `
    -and ($fontKeys -contains 'majorScripts') `
    -and ($fontKeys -contains 'minorScripts') `
    -and ($fmtKeys -contains 'name') `
    -and ($fmtKeys -contains 'fills') `
    -and ($fmtKeys -contains 'lines') `
    -and ($fmtKeys -contains 'effects') `
    -and ($fmtKeys -contains 'bgFills') `
    -and ($extra0.Count -eq 0)
Add-Check 'pptx-Tfm1' $tfm1ok "keys=$($themeKeys -join ',') fonts=$($fontKeys -join ',') extra=$($extra0.Count)"

$fmtOut = Get-OutPath 'pptx-tfm-fmt.pptx'
Close-PptPresByPath $ppt $fmtOut
if (Test-Path $fmtOut) { Remove-Item -Force $fmtOut }
$fmtJsonPath = Join-Path $global:OoxpReqDir 'pptx-tfm-fmt.json'
$fmtJson = '{"schemaVersion":1,"action":"apply","path":"' + $hostPath.Replace('\', '/') + '","output":"' + $fmtOut.Replace('\', '/') + '","ops":[{"op":"setThemeFmt","id":"master:1","line":{"index":0,"w":38100},"fill":{"index":0,"kind":"solid"}}]}'
[System.IO.File]::WriteAllText($fmtJsonPath, $fmtJson, $global:OoxpUtf8)
Invoke-Ooxp $fmtJsonPath | Out-Null
$treeF = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $fmtOut.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-tfm-fmt-inspect'
$lw = $null
$fk = $null
try {
    $lw = [int64]$treeF.masters[0].theme.fmt.lines[0].w
    $fk = [string]$treeF.masters[0].theme.fmt.fills[0].kind
} catch {}
$presF = Open-PptPres $ppt $fmtOut
$capF = Get-PptCaption $presF
Close-PptPres $presF
$tfm2ok = ($lw -eq 38100) -and ($fk -eq 'solid') -and ($capF -notmatch 'Repair')
Add-Check 'pptx-Tfm2' $tfm2ok "w=$lw kind=$fk cap=$capF"

$nameOut = Get-OutPath 'pptx-tfm-names.pptx'
Close-PptPresByPath $ppt $nameOut
if (Test-Path $nameOut) { Remove-Item -Force $nameOut }
$nameJsonPath = Join-Path $global:OoxpReqDir 'pptx-tfm-names.json'
$nameJson = '{"schemaVersion":1,"action":"apply","path":"' + $hostPath.Replace('\', '/') + '","output":"' + $nameOut.Replace('\', '/') + '","ops":[{"op":"setThemeColors","id":"master:1","name":"Custom","themeName":"Custom Theme"},{"op":"setThemeFonts","id":"master:1","name":"CustomFonts","majorScripts":[{"script":"Jpan","typeface":"Yu Mincho"}]}]}'
[System.IO.File]::WriteAllText($nameJsonPath, $nameJson, $global:OoxpUtf8)
Invoke-Ooxp $nameJsonPath | Out-Null
$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $nameOut.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-tfm-names-inspect'
$tn = $null; $cn = $null; $fn = $null; $sc = $null
try {
    $tn = [string]$treeN.masters[0].theme.name
    $cn = [string]$treeN.masters[0].theme.colors.name
    $fn = [string]$treeN.masters[0].theme.fonts.name
    $sc = [string]$treeN.masters[0].theme.fonts.majorScripts[0].script
} catch {}
$presN = Open-PptPres $ppt $nameOut
$capN = Get-PptCaption $presN
Close-PptPres $presN
$tfm3ok = ($tn -eq 'Custom Theme') -and ($cn -eq 'Custom') -and ($fn -eq 'CustomFonts') -and ($sc -eq 'Jpan') -and ($capN -notmatch 'Repair')
Add-Check 'pptx-Tfm3' $tfm3ok "theme=$tn colors=$cn fonts=$fn script=$sc cap=$capN"

$extraOut = Get-OutPath 'pptx-tfm-extra.pptx'
Close-PptPresByPath $ppt $extraOut
if (Test-Path $extraOut) { Remove-Item -Force $extraOut }
$extraJsonPath = Join-Path $global:OoxpReqDir 'pptx-tfm-extra.json'
$extraJson = '{"schemaVersion":1,"action":"apply","path":"' + $hostPath.Replace('\', '/') + '","output":"' + $extraOut.Replace('\', '/') + '","ops":[{"op":"setThemeExtraSchemes","id":"master:1","extraSchemes":[{"name":"Extra","colors":{"accent1":"0F6FC6"}}]}]}'
[System.IO.File]::WriteAllText($extraJsonPath, $extraJson, $global:OoxpUtf8)
Invoke-Ooxp $extraJsonPath | Out-Null
$treeE = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $extraOut.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-tfm-extra-inspect'
$en = $null; $ea = $null
try {
    $en = [string]$treeE.masters[0].theme.extraSchemes[0].name
    $ea = [string]$treeE.masters[0].theme.extraSchemes[0].colors.accent1
} catch {}
$presE = Open-PptPres $ppt $extraOut
$capE = Get-PptCaption $presE
Close-PptPres $presE

$clearOut = Get-OutPath 'pptx-tfm-extra-clear.pptx'
Close-PptPresByPath $ppt $clearOut
if (Test-Path $clearOut) { Remove-Item -Force $clearOut }
$clearJsonPath = Join-Path $global:OoxpReqDir 'pptx-tfm-extra-clear.json'
$clearJson = '{"schemaVersion":1,"action":"apply","path":"' + $extraOut.Replace('\', '/') + '","output":"' + $clearOut.Replace('\', '/') + '","ops":[{"op":"setThemeExtraSchemes","id":"master:1","extraSchemes":null}]}'
[System.IO.File]::WriteAllText($clearJsonPath, $clearJson, $global:OoxpUtf8)
Invoke-Ooxp $clearJsonPath | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $clearOut.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-tfm-extra-clear-inspect'
$cleared = @()
try { $cleared = @($treeC.masters[0].theme.extraSchemes) } catch {}
$presC = Open-PptPres $ppt $clearOut
$capC = Get-PptCaption $presC
Close-PptPres $presC
$tfm4ok = ($en -eq 'Extra') -and ($ea -eq '0F6FC6') -and ($capE -notmatch 'Repair') `
    -and ($cleared.Count -eq 0) -and ($capC -notmatch 'Repair')
Add-Check 'pptx-Tfm4' $tfm4ok "name=$en accent1=$ea extra=$($cleared.Count) capE=$capE capC=$capC"

$failSlideOut = Get-OutPath 'pptx-tfm-fail-slide.pptx'
$failEmptyOut = Get-OutPath 'pptx-tfm-fail-empty.pptx'
$failNotesOut = Get-OutPath 'pptx-tfm-fail-notes.pptx'
$failOvOut = Get-OutPath 'pptx-tfm-fail-override.pptx'
if (Test-Path $failSlideOut) { Remove-Item -Force $failSlideOut }
if (Test-Path $failEmptyOut) { Remove-Item -Force $failEmptyOut }
if (Test-Path $failNotesOut) { Remove-Item -Force $failNotesOut }
if (Test-Path $failOvOut) { Remove-Item -Force $failOvOut }
$failSlide = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failSlideOut.Replace('\', '/')
    ops = @(@{ op = 'setThemeFmt'; id = 'slide:1'; name = 'X' })
} 'pptx-tfm-fail-slide'
$failEmpty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failEmptyOut.Replace('\', '/')
    ops = @(@{ op = 'setThemeFmt'; id = 'master:1' })
} 'pptx-tfm-fail-empty'
$failNotes = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failNotesOut.Replace('\', '/')
    ops = @(@{ op = 'setThemeColors'; id = 'notesMaster'; accent1 = 'FF0000' })
} 'pptx-tfm-fail-notes'
$failOv = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOvOut.Replace('\', '/')
    ops = @(@{ op = 'setThemeOverride'; id = 'master:1'; accent1 = 'FF0000' })
} 'pptx-tfm-fail-override'
$tfm5ok = ((Get-PptErrorCode $failSlide) -eq 'InvalidId') `
    -and ((Get-PptErrorCode $failEmpty) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failNotes) -eq 'InvalidId') `
    -and ((Get-PptErrorCode $failOv) -eq 'InvalidId') `
    -and (-not (Test-Path $failSlideOut)) `
    -and (-not (Test-Path $failEmptyOut)) `
    -and (-not (Test-Path $failNotesOut)) `
    -and (-not (Test-Path $failOvOut))
Add-Check 'pptx-Tfm5' $tfm5ok "slide=$(Get-PptErrorCode $failSlide) empty=$(Get-PptErrorCode $failEmpty) notes=$(Get-PptErrorCode $failNotes) ov=$(Get-PptErrorCode $failOv)"
