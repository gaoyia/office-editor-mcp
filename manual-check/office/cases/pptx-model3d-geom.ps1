# pptx 幻灯片 3D 事后几何：docs/checklist-pptx.md 的 P3g1–P3g6。
# 编号 P3g，不要和 P3d 撞。探测脚本不进本文件。不要 Stop-Process PowerPoint。

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

function Get-PptModel3dShape($hostShapes) {
    $found = $null
    try {
        for ($i = 1; $i -le $hostShapes.Count; $i++) {
            $sh = $hostShapes.Item($i)
            if ([int]$sh.Type -eq 30) {
                $found = $sh
                break
            }
            Release-Com $sh
        }
    } finally { Release-Com $hostShapes }
    return $found
}

$glb = Ensure-OoxpGlb
$glbB = Ensure-OoxpGlbB
$glbFile = $glb.Replace('\', '/')
$glbBFile = $glbB.Replace('\', '/')

$src = Get-OutPath 'pptx-model3d-geom-src.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'pptx'
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'pptx-model3d-geom-create-src' | Out-Null

$ppt = Ensure-MsPpt $src
Close-PptPresByPath $ppt $src

$inserted = Get-OutPath 'pptx-model3d-geom-inserted.pptx'
Close-PptPresByPath $ppt $inserted
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{ op = 'insertModel3d'; id = 'slide:1'; file = $glbFile })
} 'pptx-model3d-geom-insert' | Out-Null

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'pptx'
    scope = @{ model3ds = $true }
} 'pptx-model3d-geom-inspect-insert'
$item = @($tree1.slides[0].model3ds) | Select-Object -First 1
$id = [string]$item.id

$sized = Get-OutPath 'pptx-model3d-geom-sized.pptx'
Close-PptPresByPath $ppt $sized
if (Test-Path $sized) { Remove-Item $sized -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $sized.Replace('\', '/')
    ops = @(@{ op = 'setModel3dSize'; id = $id; cx = 3657600; cy = 1828800 })
} 'pptx-model3d-geom-size' | Out-Null

$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sized.Replace('\', '/'); format = 'pptx'
    scope = @{ model3ds = $true }
} 'pptx-model3d-geom-inspect-size'
$itemS = @($treeS.slides[0].model3ds) | Select-Object -First 1
$presS = Open-PptPres $ppt $sized
$capS = Get-PptCaption $presS
$s1 = Get-PptSlide $presS 1
$modelS = Get-PptModel3dShape $s1.Shapes
$wPt = $null
$hPt = $null
if ($null -ne $modelS) {
    $wPt = [math]::Round([double]$modelS.Width, 2)
    $hPt = [math]::Round([double]$modelS.Height, 2)
    Release-Com $modelS
}
Release-Com $s1
Add-Check 'pptx-P3g1' (
    ([int]$itemS.cx -eq 3657600) -and ([int]$itemS.cy -eq 1828800) -and
    ($capS -notmatch 'Repair') -and ($null -ne $wPt) -and ([math]::Abs($wPt - 288) -lt 2) -and
    ($null -ne $hPt) -and ([math]::Abs($hPt - 144) -lt 2)
) "cx=$($itemS.cx) cy=$($itemS.cy) caption=$capS w=$wPt h=$hPt"
Close-PptPres $presS

$moved = Get-OutPath 'pptx-model3d-geom-moved.pptx'
Close-PptPresByPath $ppt $moved
if (Test-Path $moved) { Remove-Item $moved -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sized.Replace('\', '/')
    output = $moved.Replace('\', '/')
    ops = @(@{ op = 'setModel3dPos'; id = $id; x = 914400; y = 457200 })
} 'pptx-model3d-geom-pos' | Out-Null

$treeM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $moved.Replace('\', '/'); format = 'pptx'
    scope = @{ model3ds = $true }
} 'pptx-model3d-geom-inspect-pos'
$itemM = @($treeM.slides[0].model3ds) | Select-Object -First 1
$presM = Open-PptPres $ppt $moved
$capM = Get-PptCaption $presM
Add-Check 'pptx-P3g2' (
    ([int]$itemM.x -eq 914400) -and ([int]$itemM.y -eq 457200) -and ($capM -notmatch 'Repair')
) "x=$($itemM.x) y=$($itemM.y) caption=$capM"
Close-PptPres $presM

$notesOut = Get-OutPath 'pptx-model3d-geom-notes.pptx'
Close-PptPresByPath $ppt $notesOut
if (Test-Path $notesOut) { Remove-Item $notesOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $notesOut.Replace('\', '/')
    ops = @(@{ op = 'insertModel3d'; id = 'slide:1/notes'; file = $glbFile })
} 'pptx-model3d-geom-notes-insert' | Out-Null
$treeN0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $notesOut.Replace('\', '/'); format = 'pptx'
    scope = @{ model3ds = $true; notes = $true }
} 'pptx-model3d-geom-inspect-notes-ins'
$nId = [string](@($treeN0.slides[0].notes.model3ds) | Select-Object -First 1).id
$notesSz = Get-OutPath 'pptx-model3d-geom-notes-sz.pptx'
Close-PptPresByPath $ppt $notesSz
if (Test-Path $notesSz) { Remove-Item $notesSz -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $notesOut.Replace('\', '/')
    output = $notesSz.Replace('\', '/')
    ops = @(@{ op = 'setModel3dSize'; id = $nId; cx = 3657600; cy = 1828800 })
} 'pptx-model3d-geom-notes-size' | Out-Null
$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $notesSz.Replace('\', '/'); format = 'pptx'
    scope = @{ model3ds = $true; notes = $true }
} 'pptx-model3d-geom-inspect-notes'
$nItem = @($treeN.slides[0].notes.model3ds) | Select-Object -First 1
$presN = Open-PptPres $ppt $notesSz
$capN = Get-PptCaption $presN
Add-Check 'pptx-P3g3' (
    ([int]$nItem.cx -eq 3657600) -and ([int]$nItem.cy -eq 1828800) -and
    ([int]$treeN.model3dCount -eq 0) -and ($capN -notmatch 'Repair')
) "cx=$($nItem.cx) root=$($treeN.model3dCount) caption=$capN"
Close-PptPres $presN

$replaced = Get-OutPath 'pptx-model3d-geom-replaced.pptx'
Close-PptPresByPath $ppt $replaced
if (Test-Path $replaced) { Remove-Item $replaced -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sized.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{ op = 'replaceModel3d'; id = $id; file = $glbBFile })
} 'pptx-model3d-geom-replace' | Out-Null
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'pptx'
    scope = @{ model3ds = $true }
} 'pptx-model3d-geom-inspect-replace'
$itemR = @($treeR.slides[0].model3ds) | Select-Object -First 1
$presR = Open-PptPres $ppt $replaced
$capR = Get-PptCaption $presR
Add-Check 'pptx-P3g4' (
    ([int]$itemR.cx -eq 3657600) -and ([int]$itemR.cy -eq 1828800) -and ($capR -notmatch 'Repair')
) "cx=$($itemR.cx) cy=$($itemR.cy) caption=$capR"
Close-PptPres $presR

$failPic = Get-OutPath 'pptx-model3d-geom-fail-pic.pptx'
$rawPic = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failPic.Replace('\', '/')
    ops = @(@{ op = 'setPictureSize'; id = $id; cx = 1000; cy = 1000 })
} 'pptx-model3d-geom-fail-pic'
Add-Check 'pptx-P3g5' (
    (Get-FailCode $rawPic) -eq 'InvalidId'
) "pic=$(Get-FailCode $rawPic)"

$failPos = Get-OutPath 'pptx-model3d-geom-fail-pos.pptx'
$rawPos = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failPos.Replace('\', '/')
    ops = @(@{ op = 'setModel3dPos'; id = $id })
} 'pptx-model3d-geom-fail-pos'
$failWrap = Get-OutPath 'pptx-model3d-geom-fail-wrap.pptx'
$rawWrap = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failWrap.Replace('\', '/')
    ops = @(@{ op = 'setModel3dWrap'; id = $id; kind = 'inline' })
} 'pptx-model3d-geom-fail-wrap'
Add-Check 'pptx-P3g6' (
    ((Get-FailCode $rawPos) -eq 'InvalidField') -and
    ((Get-FailCode $rawWrap) -eq 'InvalidField')
) "pos=$(Get-FailCode $rawPos) wrap=$(Get-FailCode $rawWrap)"
