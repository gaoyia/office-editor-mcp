# pptx 幻灯片公式框事后几何：docs/checklist-pptx.md 的 Pog1–Pog5。
# 编号 Pog，不要和 Pom、Pmg 撞。探测脚本不进本文件。不要 Stop-Process PowerPoint。

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

$src = Get-OutPath 'pptx-math-geom-src.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'pptx'
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'pptx-math-geom-create-src' | Out-Null

$ppt = Ensure-MsPpt $src
Close-PptPresByPath $ppt $src

$inserted = Get-OutPath 'pptx-math-geom-inserted.pptx'
Close-PptPresByPath $ppt $inserted
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{ op = 'insertMath'; id = 'slide:1'; text = 'E=mc^2' })
} 'pptx-math-geom-insert' | Out-Null
$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'pptx'
    scope = @{ maths = $true }
} 'pptx-math-geom-inspect-insert'
$item = @($tree1.slides[0].maths) | Select-Object -First 1
$id = [string]$item.id

$sized = Get-OutPath 'pptx-math-geom-sized.pptx'
Close-PptPresByPath $ppt $sized
if (Test-Path $sized) { Remove-Item $sized -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $sized.Replace('\', '/')
    ops = @(@{ op = 'setMathSize'; id = $id; cx = 1828800; cy = 914400 })
} 'pptx-math-geom-size' | Out-Null
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sized.Replace('\', '/'); format = 'pptx'
    scope = @{ maths = $true }
} 'pptx-math-geom-inspect-size'
$itemS = @($treeS.slides[0].maths) | Select-Object -First 1
$presS = Open-PptPres $ppt $sized
$capS = Get-PptCaption $presS
Add-Check 'pptx-Pog1' (
    ([int]$itemS.cx -eq 1828800) -and ([int]$itemS.cy -eq 914400) -and ($capS -notmatch 'Repair')
) "cx=$($itemS.cx) cy=$($itemS.cy) caption=$capS"
Close-PptPres $presS

$moved = Get-OutPath 'pptx-math-geom-moved.pptx'
Close-PptPresByPath $ppt $moved
if (Test-Path $moved) { Remove-Item $moved -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sized.Replace('\', '/')
    output = $moved.Replace('\', '/')
    ops = @(@{ op = 'setMathPos'; id = $id; x = 914400; y = 457200 })
} 'pptx-math-geom-pos' | Out-Null
$treeM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $moved.Replace('\', '/'); format = 'pptx'
    scope = @{ maths = $true }
} 'pptx-math-geom-inspect-pos'
$itemM = @($treeM.slides[0].maths) | Select-Object -First 1
$presM = Open-PptPres $ppt $moved
$capM = Get-PptCaption $presM
Add-Check 'pptx-Pog2' (
    ([int]$itemM.x -eq 914400) -and ([int]$itemM.y -eq 457200) -and
    ([int]$itemM.cx -eq 1828800) -and ($capM -notmatch 'Repair')
) "x=$($itemM.x) y=$($itemM.y) cx=$($itemM.cx) caption=$capM"
Close-PptPres $presM

$notesOut = Get-OutPath 'pptx-math-geom-notes.pptx'
Close-PptPresByPath $ppt $notesOut
if (Test-Path $notesOut) { Remove-Item $notesOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $notesOut.Replace('\', '/')
    ops = @(@{ op = 'insertMath'; id = 'slide:1/notes'; text = 'E=mc^2' })
} 'pptx-math-geom-notes-insert' | Out-Null
$treeN0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $notesOut.Replace('\', '/'); format = 'pptx'
    scope = @{ maths = $true; notes = $true }
} 'pptx-math-geom-inspect-notes-ins'
$nId = [string](@($treeN0.slides[0].notes.maths) | Select-Object -First 1).id
$notesSz = Get-OutPath 'pptx-math-geom-notes-sz.pptx'
Close-PptPresByPath $ppt $notesSz
if (Test-Path $notesSz) { Remove-Item $notesSz -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $notesOut.Replace('\', '/')
    output = $notesSz.Replace('\', '/')
    ops = @(@{ op = 'setMathSize'; id = $nId; cx = 1828800; cy = 914400 })
} 'pptx-math-geom-notes-size' | Out-Null
$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $notesSz.Replace('\', '/'); format = 'pptx'
    scope = @{ maths = $true; notes = $true }
} 'pptx-math-geom-inspect-notes'
$nItem = @($treeN.slides[0].notes.maths) | Select-Object -First 1
$presN = Open-PptPres $ppt $notesSz
$capN = Get-PptCaption $presN
Add-Check 'pptx-Pog3' (
    ([int]$nItem.cx -eq 1828800) -and ([int]$nItem.cy -eq 914400) -and
    ([int]$treeN.mathCount -eq 0) -and ($capN -notmatch 'Repair')
) "cx=$($nItem.cx) root=$($treeN.mathCount) caption=$capN"
Close-PptPres $presN

$replaced = Get-OutPath 'pptx-math-geom-replaced.pptx'
Close-PptPresByPath $ppt $replaced
if (Test-Path $replaced) { Remove-Item $replaced -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $moved.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{ op = 'setMath'; id = $id; text = 'a^2+b^2=c^2' })
} 'pptx-math-geom-set' | Out-Null
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'pptx'
    scope = @{ maths = $true }
} 'pptx-math-geom-inspect-set'
$itemR = @($treeR.slides[0].maths) | Select-Object -First 1
$presR = Open-PptPres $ppt $replaced
$capR = Get-PptCaption $presR
$failSp = Get-OutPath 'pptx-math-geom-fail-sp.pptx'
$rawSp = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $replaced.Replace('\', '/')
    output = $failSp.Replace('\', '/')
    ops = @(@{ op = 'setShapePos'; id = $id; x = 1000; y = 1000 })
} 'pptx-math-geom-fail-sp'
Add-Check 'pptx-Pog4' (
    ([string]$itemR.text -eq 'a^2+b^2=c^2') -and
    ([int]$itemR.cx -eq 1828800) -and ([int]$itemR.cy -eq 914400) -and
    ([int]$itemR.x -eq 914400) -and ([int]$itemR.y -eq 457200) -and
    ($capR -notmatch 'Repair') -and
    ((Get-FailCode $rawSp) -eq 'InvalidId')
) "text=$($itemR.text) x=$($itemR.x) cx=$($itemR.cx) sp=$(Get-FailCode $rawSp) caption=$capR"
Close-PptPres $presR

$failPos = Get-OutPath 'pptx-math-geom-fail-pos.pptx'
$rawPos = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failPos.Replace('\', '/')
    ops = @(@{ op = 'setMathPos'; id = $id })
} 'pptx-math-geom-fail-pos'
$failAnchor = Get-OutPath 'pptx-math-geom-fail-anchor.pptx'
$rawAnchor = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failAnchor.Replace('\', '/')
    ops = @(@{ op = 'setMathAnchor'; id = $id; from = 'C3' })
} 'pptx-math-geom-fail-anchor'
Add-Check 'pptx-Pog5' (
    ((Get-FailCode $rawPos) -eq 'InvalidField') -and
    ((Get-FailCode $rawAnchor) -eq 'InvalidField') -and
    (-not (Test-Path $failPos)) -and (-not (Test-Path $failAnchor))
) "pos=$(Get-FailCode $rawPos) anchor=$(Get-FailCode $rawAnchor)"
