# pptx 幻灯片 3D 相机与灯光：docs/checklist-pptx.md 的 P3k1–P3k5。
# 编号 P3k，不要和 P3d、P3g、P3c 撞。探测脚本不进本文件。不要 Stop-Process PowerPoint。

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

function Get-FailCode($raw) {
    $code = $null
    try { $code = ($raw.Stderr | ConvertFrom-Json).error.code } catch { $code = $null }
    return $code
}

function Test-DefaultView($item) {
    return (
        ([int]$item.camera.pos.z -eq 66519324) -and
        ([int]$item.camera.fov -eq 2700000) -and
        ($null -eq $item.rot.ax) -and ($null -eq $item.rot.ay) -and ($null -eq $item.rot.az) -and
        ($null -eq $item.lightRig)
    )
}

function Test-PatchedView($item) {
    return (
        ([int]$item.camera.pos.x -eq 50400000) -and
        ([int]$item.camera.pos.z -eq 66519324) -and
        ([int]$item.camera.fov -eq 1892375) -and
        ([int]$item.rot.ax -eq 1060962) -and
        ([int]$item.rot.ay -eq -532862) -and
        ([int]$item.rot.az -eq -169045) -and
        ([string]$item.lightRig.rig -eq 'freezing') -and
        ([string]$item.lightRig.dir -eq 't') -and
        ([int]$item.lightRig.rev -eq 18000000)
    )
}

$glb = Ensure-OoxpGlb
$glbFile = $glb.Replace('\', '/')

$src = Get-OutPath 'pptx-model3d-camera-src.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'pptx'
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'pptx-model3d-camera-create-src' | Out-Null

$ppt = Ensure-MsPpt $src
Close-PptPresByPath $ppt $src

$inserted = Get-OutPath 'pptx-model3d-camera-inserted.pptx'
Close-PptPresByPath $ppt $inserted
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{ op = 'insertModel3d'; id = 'slide:1'; file = $glbFile })
} 'pptx-model3d-camera-insert' | Out-Null

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'pptx'
    scope = @{ model3ds = $true }
} 'pptx-model3d-camera-inspect-insert'
$item = @($tree1.slides[0].model3ds) | Select-Object -First 1
$id = [string]$item.id
$pres1 = Open-PptPres $ppt $inserted
$cap1 = Get-PptCaption $pres1
Add-Check 'pptx-P3k1' (
    (Test-DefaultView $item) -and ($cap1 -notmatch 'Repair')
) "z=$($item.camera.pos.z) fov=$($item.camera.fov) light=$($item.lightRig) caption=$cap1"
Close-PptPres $pres1

$patched = Get-OutPath 'pptx-model3d-camera-patched.pptx'
Close-PptPresByPath $ppt $patched
if (Test-Path $patched) { Remove-Item $patched -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $patched.Replace('\', '/')
    ops = @(@{
        op = 'setModel3dCamera'; id = $id
        camera = @{ pos = @{ x = 50400000 }; fov = 1892375 }
        rot = @{ ax = 1060962; ay = -532862; az = -169045 }
        lightRig = @{ rig = 'freezing'; dir = 't'; rev = 18000000 }
    })
} 'pptx-model3d-camera-set' | Out-Null

$treeP = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $patched.Replace('\', '/'); format = 'pptx'
    scope = @{ model3ds = $true }
} 'pptx-model3d-camera-inspect-set'
$itemP = @($treeP.slides[0].model3ds) | Select-Object -First 1
$presP = Open-PptPres $ppt $patched
$capP = Get-PptCaption $presP
Add-Check 'pptx-P3k2' (
    (Test-PatchedView $itemP) -and ($capP -notmatch 'Repair')
) "x=$($itemP.camera.pos.x) fov=$($itemP.camera.fov) rev=$($itemP.lightRig.rev) caption=$capP"
Close-PptPres $presP

$cleared = Get-OutPath 'pptx-model3d-camera-cleared.pptx'
Close-PptPresByPath $ppt $cleared
if (Test-Path $cleared) { Remove-Item $cleared -Force }
# ConvertTo-Json 会丢掉 $null，lightRig 必须显式写出 null。
$clearPath = Join-Path $global:OoxpReqDir 'pptx-model3d-camera-clear.json'
$clearJson = '{"schemaVersion":1,"action":"apply","path":"' + ($patched.Replace('\', '/')) + '","output":"' + ($cleared.Replace('\', '/')) + '","ops":[{"op":"setModel3dCamera","id":"' + $id + '","lightRig":null}]}'
[System.IO.File]::WriteAllText($clearPath, $clearJson, $global:OoxpUtf8)
Invoke-Ooxp $clearPath | Out-Null

$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cleared.Replace('\', '/'); format = 'pptx'
    scope = @{ model3ds = $true }
} 'pptx-model3d-camera-inspect-clear'
$itemC = @($treeC.slides[0].model3ds) | Select-Object -First 1
$presC = Open-PptPres $ppt $cleared
$capC = Get-PptCaption $presC
Add-Check 'pptx-P3k3' (
    ($null -eq $itemC.lightRig) -and ([int]$itemC.camera.fov -eq 1892375) -and ($capC -notmatch 'Repair')
) "light=$($itemC.lightRig) fov=$($itemC.camera.fov) caption=$capC"
Close-PptPres $presC

$notesOut = Get-OutPath 'pptx-model3d-camera-notes.pptx'
Close-PptPresByPath $ppt $notesOut
if (Test-Path $notesOut) { Remove-Item $notesOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $notesOut.Replace('\', '/')
    ops = @(@{ op = 'insertModel3d'; id = 'slide:1/notes'; file = $glbFile })
} 'pptx-model3d-camera-notes-insert' | Out-Null
$treeN0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $notesOut.Replace('\', '/'); format = 'pptx'
    scope = @{ model3ds = $true; notes = $true }
} 'pptx-model3d-camera-inspect-notes-ins'
$nId = [string](@($treeN0.slides[0].notes.model3ds) | Select-Object -First 1).id
$notesCam = Get-OutPath 'pptx-model3d-camera-notes-fov.pptx'
Close-PptPresByPath $ppt $notesCam
if (Test-Path $notesCam) { Remove-Item $notesCam -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $notesOut.Replace('\', '/')
    output = $notesCam.Replace('\', '/')
    ops = @(@{ op = 'setModel3dCamera'; id = $nId; camera = @{ fov = 1892375 } })
} 'pptx-model3d-camera-notes-set' | Out-Null
$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $notesCam.Replace('\', '/'); format = 'pptx'
    scope = @{ model3ds = $true; notes = $true }
} 'pptx-model3d-camera-inspect-notes'
$nItem = @($treeN.slides[0].notes.model3ds) | Select-Object -First 1
$presN = Open-PptPres $ppt $notesCam
$capN = Get-PptCaption $presN
Add-Check 'pptx-P3k4' (
    ([int]$nItem.camera.fov -eq 1892375) -and ([int]$treeN.model3dCount -eq 0) -and ($capN -notmatch 'Repair')
) "fov=$($nItem.camera.fov) count=$($treeN.model3dCount) caption=$capN"
Close-PptPres $presN

$failEmpty = Get-OutPath 'pptx-model3d-camera-fail-empty.pptx'
$rawEmpty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failEmpty.Replace('\', '/')
    ops = @(@{ op = 'setModel3dCamera'; id = $id })
} 'pptx-model3d-camera-fail-empty'
$failFov = Get-OutPath 'pptx-model3d-camera-fail-fov.pptx'
$rawFov = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failFov.Replace('\', '/')
    ops = @(@{ op = 'setModel3dCamera'; id = $id; camera = @{ fov = -1 } })
} 'pptx-model3d-camera-fail-fov'
Add-Check 'pptx-P3k5' (
    ((Get-FailCode $rawEmpty) -eq 'InvalidField') -and
    ((Get-FailCode $rawFov) -eq 'InvalidField')
) "empty=$(Get-FailCode $rawEmpty) fov=$(Get-FailCode $rawFov)"
