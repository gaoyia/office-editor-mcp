# docx 3D 相机与灯光：docs/checklist-docx.md 的 D3c1–D3c5。
# 编号 D3c，不要和 D3d、D3g 撞。探测脚本不进本文件。不要 Stop-Process Word。

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

$src = Get-OutPath 'docx-model3d-camera-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{ paragraphs = @(@{ text = 'model3d-camera-host' }) }
} 'docx-model3d-camera-create-host' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; model3ds = $true }
} 'docx-model3d-camera-inspect-src'
$para = [string]$tree0.paragraphs[0].id

$inserted = Get-OutPath 'docx-model3d-camera-inserted.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{ op = 'insertModel3d'; id = $para; file = $glbFile })
} 'docx-model3d-camera-insert' | Out-Null

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; model3ds = $true }
} 'docx-model3d-camera-inspect-insert'
$item = @($tree1.paragraphs[0].model3ds) | Select-Object -First 1
$id = [string]$item.id
$word = Ensure-MsWord $inserted
$doc1 = Open-WordDoc $word $inserted
$cap1 = [string]$doc1.ActiveWindow.Caption
Add-Check 'docx-D3c1' (
    (Test-DefaultView $item) -and ($cap1 -notmatch 'Repair')
) "z=$($item.camera.pos.z) fov=$($item.camera.fov) light=$($item.lightRig) caption=$cap1"
Close-WordDoc $doc1

$patched = Get-OutPath 'docx-model3d-camera-patched.docx'
Close-WordDocByPath $word $patched
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
} 'docx-model3d-camera-set' | Out-Null

$treeP = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $patched.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; model3ds = $true }
} 'docx-model3d-camera-inspect-set'
$itemP = @($treeP.paragraphs[0].model3ds) | Select-Object -First 1
$docP = Open-WordDoc $word $patched
$capP = [string]$docP.ActiveWindow.Caption
Add-Check 'docx-D3c2' (
    (Test-PatchedView $itemP) -and ($capP -notmatch 'Repair')
) "x=$($itemP.camera.pos.x) fov=$($itemP.camera.fov) rev=$($itemP.lightRig.rev) caption=$capP"
Close-WordDoc $docP

$cleared = Get-OutPath 'docx-model3d-camera-cleared.docx'
Close-WordDocByPath $word $cleared
# ConvertTo-Json 会丢掉 $null，lightRig 必须显式写出 null。
$clearPath = Join-Path $global:OoxpReqDir 'docx-model3d-camera-clear.json'
$clearJson = '{"schemaVersion":1,"action":"apply","path":"' + ($patched.Replace('\', '/')) + '","output":"' + ($cleared.Replace('\', '/')) + '","ops":[{"op":"setModel3dCamera","id":"' + $id + '","lightRig":null}]}'
[System.IO.File]::WriteAllText($clearPath, $clearJson, $global:OoxpUtf8)
Invoke-Ooxp $clearPath | Out-Null

$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cleared.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; model3ds = $true }
} 'docx-model3d-camera-inspect-clear'
$itemC = @($treeC.paragraphs[0].model3ds) | Select-Object -First 1
$docC = Open-WordDoc $word $cleared
$capC = [string]$docC.ActiveWindow.Caption
Add-Check 'docx-D3c3' (
    ($null -eq $itemC.lightRig) -and ([int]$itemC.camera.fov -eq 1892375) -and ($capC -notmatch 'Repair')
) "light=$($itemC.lightRig) fov=$($itemC.camera.fov) caption=$capC"
Close-WordDoc $docC

$hfSrc = Get-OutPath 'docx-model3d-camera-hf-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hfSrc.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(@{ text = 'body' })
        headers = @(@{ type = 'default'; paragraphs = @(@{ text = 'hdr' }) })
    }
} 'docx-model3d-camera-hf-create' | Out-Null
$treeH0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfSrc.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true; model3ds = $true }
} 'docx-model3d-camera-inspect-hf-src'
$hPara = [string]$treeH0.headers[0].paragraphs[0].id
$hfIns = Get-OutPath 'docx-model3d-camera-hf-ins.docx'
Close-WordDocByPath $word $hfIns
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hfSrc.Replace('\', '/')
    output = $hfIns.Replace('\', '/')
    ops = @(@{ op = 'insertModel3d'; id = $hPara; file = $glbFile })
} 'docx-model3d-camera-hf-insert' | Out-Null
$treeH1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfIns.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true; model3ds = $true }
} 'docx-model3d-camera-inspect-hf-ins'
$hId = [string](@($treeH1.headers[0].model3ds) | Select-Object -First 1).id
$hfOut = Get-OutPath 'docx-model3d-camera-hf.docx'
Close-WordDocByPath $word $hfOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hfIns.Replace('\', '/')
    output = $hfOut.Replace('\', '/')
    ops = @(@{ op = 'setModel3dCamera'; id = $hId; camera = @{ fov = 1892375 } })
} 'docx-model3d-camera-hf-set' | Out-Null
$treeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfOut.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true; model3ds = $true }
} 'docx-model3d-camera-inspect-hf'
$hItem = @($treeH.headers[0].model3ds) | Select-Object -First 1
$docH = Open-WordDoc $word $hfOut
$capH = [string]$docH.ActiveWindow.Caption
Add-Check 'docx-D3c4' (
    ([int]$hItem.camera.fov -eq 1892375) -and ([int]$treeH.model3dCount -eq 0) -and ($capH -notmatch 'Repair')
) "fov=$($hItem.camera.fov) count=$($treeH.model3dCount) caption=$capH"
Close-WordDoc $docH

$failEmpty = Get-OutPath 'docx-model3d-camera-fail-empty.docx'
$rawEmpty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failEmpty.Replace('\', '/')
    ops = @(@{ op = 'setModel3dCamera'; id = $id })
} 'docx-model3d-camera-fail-empty'
$failFov = Get-OutPath 'docx-model3d-camera-fail-fov.docx'
$rawFov = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failFov.Replace('\', '/')
    ops = @(@{ op = 'setModel3dCamera'; id = $id; camera = @{ fov = -1 } })
} 'docx-model3d-camera-fail-fov'
Add-Check 'docx-D3c5' (
    ((Get-FailCode $rawEmpty) -eq 'InvalidField') -and
    ((Get-FailCode $rawFov) -eq 'InvalidField')
) "empty=$(Get-FailCode $rawEmpty) fov=$(Get-FailCode $rawFov)"
