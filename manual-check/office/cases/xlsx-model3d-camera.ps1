# xlsx 工作表 3D 相机与灯光：docs/checklist-xlsx.md 的 X3c1–X3c5。
# 编号 X3c，不要和 X3d、X3g 撞。探测脚本不进本文件。不要 Stop-Process Excel。

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

$src = Get-OutPath 'xlsx-model3d-camera-src.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-model3d-camera-create-src' | Out-Null

$excel = Ensure-MsExcel $src
Close-AllWorkbooks $excel

$inserted = Get-OutPath 'xlsx-model3d-camera-inserted.xlsx'
Close-AllWorkbooks $excel
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{
        op = 'insertModel3d'
        id = 'sheet:Sheet1'
        file = $glbFile
        from = 'B2'
    })
} 'xlsx-model3d-camera-insert' | Out-Null

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'xlsx'
    scope = @{ model3ds = $true }
} 'xlsx-model3d-camera-inspect-insert'
$item = @($tree1.sheets[0].model3ds) | Select-Object -First 1
$id = [string]$item.id
$wb1 = Open-ExcelBook $excel $inserted
$cap1 = [string]$wb1.Windows.Item(1).Caption
Add-Check 'xlsx-X3c1' (
    (Test-DefaultView $item) -and ($cap1 -notmatch 'Repair')
) "z=$($item.camera.pos.z) fov=$($item.camera.fov) light=$($item.lightRig) caption=$cap1"
Close-AllWorkbooks $excel

$patched = Get-OutPath 'xlsx-model3d-camera-patched.xlsx'
Close-AllWorkbooks $excel
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
} 'xlsx-model3d-camera-set' | Out-Null

$treeP = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $patched.Replace('\', '/'); format = 'xlsx'
    scope = @{ model3ds = $true }
} 'xlsx-model3d-camera-inspect-set'
$itemP = @($treeP.sheets[0].model3ds) | Select-Object -First 1
$wbP = Open-ExcelBook $excel $patched
$capP = [string]$wbP.Windows.Item(1).Caption
Add-Check 'xlsx-X3c2' (
    (Test-PatchedView $itemP) -and ($capP -notmatch 'Repair')
) "x=$($itemP.camera.pos.x) fov=$($itemP.camera.fov) rev=$($itemP.lightRig.rev) caption=$capP"
Close-AllWorkbooks $excel

$cleared = Get-OutPath 'xlsx-model3d-camera-cleared.xlsx'
Close-AllWorkbooks $excel
if (Test-Path $cleared) { Remove-Item $cleared -Force }
# ConvertTo-Json 会丢掉 $null，lightRig 必须显式写出 null。
$clearPath = Join-Path $global:OoxpReqDir 'xlsx-model3d-camera-clear.json'
$clearJson = '{"schemaVersion":1,"action":"apply","path":"' + ($patched.Replace('\', '/')) + '","output":"' + ($cleared.Replace('\', '/')) + '","ops":[{"op":"setModel3dCamera","id":"' + $id + '","lightRig":null}]}'
[System.IO.File]::WriteAllText($clearPath, $clearJson, $global:OoxpUtf8)
Invoke-Ooxp $clearPath | Out-Null

$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cleared.Replace('\', '/'); format = 'xlsx'
    scope = @{ model3ds = $true }
} 'xlsx-model3d-camera-inspect-clear'
$itemC = @($treeC.sheets[0].model3ds) | Select-Object -First 1
$wbC = Open-ExcelBook $excel $cleared
$capC = [string]$wbC.Windows.Item(1).Caption
Add-Check 'xlsx-X3c3' (
    ($null -eq $itemC.lightRig) -and ([int]$itemC.camera.fov -eq 1892375) -and ($capC -notmatch 'Repair')
) "light=$($itemC.lightRig) fov=$($itemC.camera.fov) caption=$capC"
Close-AllWorkbooks $excel

$failPic = Get-OutPath 'xlsx-model3d-camera-fail-pic.xlsx'
$rawPic = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failPic.Replace('\', '/')
    ops = @(@{ op = 'setPictureSize'; id = $id; cx = 1000; cy = 1000 })
} 'xlsx-model3d-camera-fail-pic'
Add-Check 'xlsx-X3c4' (
    (Get-FailCode $rawPic) -eq 'InvalidId'
) "pic=$(Get-FailCode $rawPic)"

$failEmpty = Get-OutPath 'xlsx-model3d-camera-fail-empty.xlsx'
$rawEmpty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failEmpty.Replace('\', '/')
    ops = @(@{ op = 'setModel3dCamera'; id = $id })
} 'xlsx-model3d-camera-fail-empty'
$failFov = Get-OutPath 'xlsx-model3d-camera-fail-fov.xlsx'
$rawFov = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failFov.Replace('\', '/')
    ops = @(@{ op = 'setModel3dCamera'; id = $id; camera = @{ fov = -1 } })
} 'xlsx-model3d-camera-fail-fov'
Add-Check 'xlsx-X3c5' (
    ((Get-FailCode $rawEmpty) -eq 'InvalidField') -and
    ((Get-FailCode $rawFov) -eq 'InvalidField')
) "empty=$(Get-FailCode $rawEmpty) fov=$(Get-FailCode $rawFov)"
