# pptx 媒体事后几何：docs/checklist-pptx.md 的 Pmg1–Pmg5。
# 编号 Pmg，不要和 Pm、Pmd、Mt 撞。探测脚本不进本文件。不要 Stop-Process PowerPoint。

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

function Get-PptMediaShape($hostShapes) {
    $found = $null
    try {
        for ($i = 1; $i -le $hostShapes.Count; $i++) {
            $sh = $hostShapes.Item($i)
            if ([int]$sh.Type -eq 16) {
                $found = $sh
                break
            }
            Release-Com $sh
        }
    } finally { Release-Com $hostShapes }
    return $found
}

$mp4 = Ensure-OoxpMp4
$mp4File = $mp4.Replace('\', '/')

$src = Get-OutPath 'pptx-media-geom-src.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'pptx'
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'pptx-media-geom-create-src' | Out-Null

$ppt = Ensure-MsPpt $src
Close-PptPresByPath $ppt $src

$inserted = Get-OutPath 'pptx-media-geom-inserted.pptx'
Close-PptPresByPath $ppt $inserted
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{ op = 'insertVideo'; id = 'slide:1'; file = $mp4File })
} 'pptx-media-geom-insert' | Out-Null
$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'pptx'
    scope = @{ media = $true }
} 'pptx-media-geom-inspect-insert'
$item = @($tree1.slides[0].videos) | Select-Object -First 1
$id = [string]$item.id

$sized = Get-OutPath 'pptx-media-geom-sized.pptx'
Close-PptPresByPath $ppt $sized
if (Test-Path $sized) { Remove-Item $sized -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $sized.Replace('\', '/')
    ops = @(@{ op = 'setPictureSize'; id = $id; cx = 1828800; cy = 914400 })
} 'pptx-media-geom-size' | Out-Null
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sized.Replace('\', '/'); format = 'pptx'
    scope = @{ media = $true }
} 'pptx-media-geom-inspect-size'
$itemS = @($treeS.slides[0].videos) | Select-Object -First 1
$presS = Open-PptPres $ppt $sized
$capS = Get-PptCaption $presS
$s1 = Get-PptSlide $presS 1
$mediaS = Get-PptMediaShape $s1.Shapes
$wPt = $null
$hPt = $null
if ($null -ne $mediaS) {
    $wPt = [math]::Round([double]$mediaS.Width, 2)
    $hPt = [math]::Round([double]$mediaS.Height, 2)
    Release-Com $mediaS
}
Release-Com $s1
Add-Check 'pptx-Pmg1' (
    ([int]$itemS.cx -eq 1828800) -and ([int]$itemS.cy -eq 914400) -and
    ($capS -notmatch 'Repair') -and ($null -ne $wPt) -and ([math]::Abs($wPt - 144) -lt 2) -and
    ($null -ne $hPt) -and ([math]::Abs($hPt - 72) -lt 2)
) "cx=$($itemS.cx) cy=$($itemS.cy) caption=$capS w=$wPt h=$hPt"
Close-PptPres $presS

$moved = Get-OutPath 'pptx-media-geom-moved.pptx'
Close-PptPresByPath $ppt $moved
if (Test-Path $moved) { Remove-Item $moved -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sized.Replace('\', '/')
    output = $moved.Replace('\', '/')
    ops = @(@{ op = 'setPicturePos'; id = $id; x = 914400; y = 457200 })
} 'pptx-media-geom-pos' | Out-Null
$treeM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $moved.Replace('\', '/'); format = 'pptx'
    scope = @{ media = $true }
} 'pptx-media-geom-inspect-pos'
$itemM = @($treeM.slides[0].videos) | Select-Object -First 1
$presM = Open-PptPres $ppt $moved
$capM = Get-PptCaption $presM
Add-Check 'pptx-Pmg2' (
    ([int]$itemM.x -eq 914400) -and ([int]$itemM.y -eq 457200) -and
    ([int]$itemM.cx -eq 1828800) -and ([int]$itemM.cy -eq 914400) -and
    ($capM -notmatch 'Repair')
) "x=$($itemM.x) y=$($itemM.y) cx=$($itemM.cx) caption=$capM"
Close-PptPres $presM

$notesOut = Get-OutPath 'pptx-media-geom-notes.pptx'
Close-PptPresByPath $ppt $notesOut
if (Test-Path $notesOut) { Remove-Item $notesOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $notesOut.Replace('\', '/')
    ops = @(@{ op = 'insertVideo'; id = 'slide:1/notes'; file = $mp4File })
} 'pptx-media-geom-notes-insert' | Out-Null
$treeN0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $notesOut.Replace('\', '/'); format = 'pptx'
    scope = @{ media = $true; notes = $true }
} 'pptx-media-geom-inspect-notes-ins'
$nId = [string](@($treeN0.slides[0].notes.videos) | Select-Object -First 1).id
$notesSz = Get-OutPath 'pptx-media-geom-notes-sz.pptx'
Close-PptPresByPath $ppt $notesSz
if (Test-Path $notesSz) { Remove-Item $notesSz -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $notesOut.Replace('\', '/')
    output = $notesSz.Replace('\', '/')
    ops = @(@{ op = 'setPictureSize'; id = $nId; cx = 1828800; cy = 914400 })
} 'pptx-media-geom-notes-size' | Out-Null
$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $notesSz.Replace('\', '/'); format = 'pptx'
    scope = @{ media = $true; notes = $true }
} 'pptx-media-geom-inspect-notes'
$nItem = @($treeN.slides[0].notes.videos) | Select-Object -First 1
$presN = Open-PptPres $ppt $notesSz
$capN = Get-PptCaption $presN
Add-Check 'pptx-Pmg3' (
    ([int]$nItem.cx -eq 1828800) -and ([int]$nItem.cy -eq 914400) -and
    ([int]$treeN.videoCount -eq 0) -and ($capN -notmatch 'Repair')
) "cx=$($nItem.cx) root=$($treeN.videoCount) caption=$capN"
Close-PptPres $presN

$trimmed = Get-OutPath 'pptx-media-geom-trim.pptx'
Close-PptPresByPath $ppt $trimmed
if (Test-Path $trimmed) { Remove-Item $trimmed -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $moved.Replace('\', '/')
    output = $trimmed.Replace('\', '/')
    ops = @(@{ op = 'setMedia'; id = $id; st = 100; end = 200 })
} 'pptx-media-geom-set-media' | Out-Null
$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $trimmed.Replace('\', '/'); format = 'pptx'
    scope = @{ media = $true }
} 'pptx-media-geom-inspect-trim'
$itemT = @($treeT.slides[0].videos) | Select-Object -First 1
$presT = Open-PptPres $ppt $trimmed
$capT = Get-PptCaption $presT
Add-Check 'pptx-Pmg4' (
    ([int]$itemT.st -eq 100) -and ([int]$itemT.end -eq 200) -and
    ([int]$itemT.cx -eq 1828800) -and ([int]$itemT.cy -eq 914400) -and
    ([int]$itemT.x -eq 914400) -and ([int]$itemT.y -eq 457200) -and
    ($capT -notmatch 'Repair')
) "st=$($itemT.st) x=$($itemT.x) cx=$($itemT.cx) caption=$capT"
Close-PptPres $presT

$failPos = Get-OutPath 'pptx-media-geom-fail-pos.pptx'
$rawPos = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failPos.Replace('\', '/')
    ops = @(@{ op = 'setPicturePos'; id = $id })
} 'pptx-media-geom-fail-pos'
$failPic = Get-OutPath 'pptx-media-geom-fail-pic.pptx'
$rawPic = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failPic.Replace('\', '/')
    ops = @(@{ op = 'setMedia'; id = 'slide:1/pic:3'; st = 1 })
} 'pptx-media-geom-fail-pic'
$deleted = Get-OutPath 'pptx-media-geom-deleted.pptx'
Close-PptPresByPath $ppt $deleted
if (Test-Path $deleted) { Remove-Item $deleted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $deleted.Replace('\', '/')
    ops = @(@{ op = 'deleteMedia'; id = $id })
} 'pptx-media-geom-delete' | Out-Null
$failMiss = Get-OutPath 'pptx-media-geom-fail-miss.pptx'
$rawMiss = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $deleted.Replace('\', '/')
    output = $failMiss.Replace('\', '/')
    ops = @(@{ op = 'setPictureSize'; id = $id; cx = 1000; cy = 1000 })
} 'pptx-media-geom-fail-miss'
Add-Check 'pptx-Pmg5' (
    ((Get-FailCode $rawPos) -eq 'InvalidField') -and
    ((Get-FailCode $rawPic) -eq 'InvalidId') -and
    ((Get-FailCode $rawMiss) -eq 'IdNotFound') -and
    (-not (Test-Path $failPos)) -and (-not (Test-Path $failPic)) -and
    (-not (Test-Path $failMiss))
) "pos=$(Get-FailCode $rawPos) pic=$(Get-FailCode $rawPic) miss=$(Get-FailCode $rawMiss)"
