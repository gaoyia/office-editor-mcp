# pptx 切换声音与细节：docs/checklist-pptx.md 的 Td1–Td5。

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
$wav = Ensure-OoxpWav
$wavName = [IO.Path]::GetFileName($wav)
$ppt = Ensure-MsPpt $hostPath

# Td1：切换声音。
$sndOut = Get-OutPath 'pptx-td-sound.pptx'
Close-PptPresByPath $ppt $sndOut
if (Test-Path $sndOut) { Remove-Item -Force $sndOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $sndOut.Replace('\', '/')
    ops = @(@{ op = 'setTransition'; id = 'slide:1'; type = 'fade'; sound = $wav.Replace('\', '/') })
} 'pptx-td-sound' | Out-Null
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sndOut.Replace('\', '/'); format = 'pptx'
} 'pptx-td-sound-inspect'
$xmlS = Get-ZipPartText $sndOut 'ppt/slides/slide1.xml'
$presS = Open-PptPres $ppt $sndOut
$capS = Get-PptCaption $presS
Close-PptPres $presS
$trS = $treeS.slides[0].transition
$td1ok = ($capS -notmatch 'Repair') `
    -and ([string]$trS.type -eq 'fade') `
    -and ([string]$trS.sound -eq $wavName) `
    -and ($xmlS -match '<p:sndAc>') `
    -and ($xmlS -match '<p:stSnd>')
Add-Check 'pptx-Td1' $td1ok "cap=$capS type=$($trS.type) sound=$($trS.sound) expect=$wavName"

# Td2：split orient。
$splitOut = Get-OutPath 'pptx-td-split.pptx'
Close-PptPresByPath $ppt $splitOut
if (Test-Path $splitOut) { Remove-Item -Force $splitOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $splitOut.Replace('\', '/')
    ops = @(@{ op = 'setTransition'; id = 'slide:1'; type = 'split'; orient = 'vert'; dir = 'in' })
} 'pptx-td-split' | Out-Null
$treeP = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $splitOut.Replace('\', '/'); format = 'pptx'
} 'pptx-td-split-inspect'
$presP = Open-PptPres $ppt $splitOut
$capP = Get-PptCaption $presP
Close-PptPres $presP
$trP = $treeP.slides[0].transition
$td2ok = ($capP -notmatch 'Repair') `
    -and ([string]$trP.type -eq 'split') `
    -and ([string]$trP.orient -eq 'vert') `
    -and ([string]$trP.dir -eq 'in')
Add-Check 'pptx-Td2' $td2ok "cap=$capP type=$($trP.type) orient=$($trP.orient) dir=$($trP.dir)"

# Td3：wheel spokes。
$wheelOut = Get-OutPath 'pptx-td-wheel.pptx'
Close-PptPresByPath $ppt $wheelOut
if (Test-Path $wheelOut) { Remove-Item -Force $wheelOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $wheelOut.Replace('\', '/')
    ops = @(@{ op = 'setTransition'; id = 'slide:1'; type = 'wheel'; spokes = 8 })
} 'pptx-td-wheel' | Out-Null
$treeW = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $wheelOut.Replace('\', '/'); format = 'pptx'
} 'pptx-td-wheel-inspect'
$presW = Open-PptPres $ppt $wheelOut
$capW = Get-PptCaption $presW
Close-PptPres $presW
$trW = $treeW.slides[0].transition
$td3ok = ($capW -notmatch 'Repair') `
    -and ([string]$trW.type -eq 'wheel') `
    -and ([int]$trW.spokes -eq 8)
Add-Check 'pptx-Td3' $td3ok "cap=$capW type=$($trW.type) spokes=$($trW.spokes)"

# Td4：母版切换。
$masterOut = Get-OutPath 'pptx-td-master.pptx'
Close-PptPresByPath $ppt $masterOut
if (Test-Path $masterOut) { Remove-Item -Force $masterOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $masterOut.Replace('\', '/')
    ops = @(@{ op = 'setTransition'; id = 'master:1'; type = 'fade' })
} 'pptx-td-master' | Out-Null
$treeM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $masterOut.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-td-master-inspect'
$presM = Open-PptPres $ppt $masterOut
$capM = Get-PptCaption $presM
Close-PptPres $presM
$td4ok = ($capM -notmatch 'Repair') `
    -and ([string]$treeM.masters[0].transition.type -eq 'fade')
Add-Check 'pptx-Td4' $td4ok "cap=$capM type=$($treeM.masters[0].transition.type)"

# Td5：拒错。
$badNm = Get-OutPath 'pptx-td-fail-notesmaster.pptx'
$badOrient = Get-OutPath 'pptx-td-fail-orient.pptx'
$badSpokes = Get-OutPath 'pptx-td-fail-spokes.pptx'
Close-PptPresByPath $ppt $badNm
Close-PptPresByPath $ppt $badOrient
Close-PptPresByPath $ppt $badSpokes
if (Test-Path $badNm) { Remove-Item -Force $badNm }
if (Test-Path $badOrient) { Remove-Item -Force $badOrient }
if (Test-Path $badSpokes) { Remove-Item -Force $badSpokes }
$failNm = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badNm.Replace('\', '/')
    ops = @(@{ op = 'setTransition'; id = 'notesMaster'; type = 'fade' })
} 'pptx-td-fail-nm'
$failOrient = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badOrient.Replace('\', '/')
    ops = @(@{ op = 'setTransition'; id = 'slide:1'; type = 'fade'; orient = 'vert' })
} 'pptx-td-fail-orient'
$failSpokes = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badSpokes.Replace('\', '/')
    ops = @(@{ op = 'setTransition'; id = 'slide:1'; type = 'fade'; spokes = 8 })
} 'pptx-td-fail-spokes'
$td5ok = ((Get-PptErrorCode $failNm) -eq 'InvalidId') `
    -and (-not (Test-Path $badNm)) `
    -and ((Get-PptErrorCode $failOrient) -eq 'InvalidField') `
    -and (-not (Test-Path $badOrient)) `
    -and ((Get-PptErrorCode $failSpokes) -eq 'InvalidField') `
    -and (-not (Test-Path $badSpokes))
Add-Check 'pptx-Td5' $td5ok "nm=$(Get-PptErrorCode $failNm) orient=$(Get-PptErrorCode $failOrient) spokes=$(Get-PptErrorCode $failSpokes)"
