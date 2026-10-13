# pptx Zoom 画布几何：docs/checklist-pptx.md 的 Pzg1–Pzg5。
# 编号 Pzg，不要和 Pz、Pzr 撞。探测脚本不进本文件。不要 Stop-Process PowerPoint。

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

function Get-PptZoomShape($hostShapes) {
    $found = $null
    try {
        for ($i = 1; $i -le $hostShapes.Count; $i++) {
            $sh = $hostShapes.Item($i)
            $isZoom = $false
            try { $isZoom = ([int]$sh.Type -eq -2) -and ([string]$sh.Name -like 'Zoom*') } catch { $isZoom = $false }
            if ($isZoom) {
                $found = $sh
                break
            }
            Release-Com $sh
        }
    } finally { Release-Com $hostShapes }
    return $found
}

$src = Get-OutPath 'pptx-zoom-geom-src.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(
            @{ shapes = @(@{ text = '页1' }) }
            @{ shapes = @(@{ text = '页2' }) }
            @{ shapes = @(@{ text = '页3' }) }
        )
    }
} 'pptx-zoom-geom-create-src' | Out-Null

$ppt = Ensure-MsPpt $src
Close-PptPresByPath $ppt $src

$inserted = Get-OutPath 'pptx-zoom-geom-inserted.pptx'
Close-PptPresByPath $ppt $inserted
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{ op = 'insertZoom'; id = 'slide:1'; type = 'slide'; target = 'slide:2' })
} 'pptx-zoom-geom-insert' | Out-Null
$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'pptx'
    scope = @{ zooms = $true }
} 'pptx-zoom-geom-inspect-insert'
$item = @($tree1.slides[0].zooms) | Select-Object -First 1
$id = [string]$item.id

$sized = Get-OutPath 'pptx-zoom-geom-sized.pptx'
Close-PptPresByPath $ppt $sized
if (Test-Path $sized) { Remove-Item $sized -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $sized.Replace('\', '/')
    ops = @(@{ op = 'setZoom'; id = $id; cx = 1828800; cy = 914400 })
} 'pptx-zoom-geom-size' | Out-Null
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sized.Replace('\', '/'); format = 'pptx'
    scope = @{ zooms = $true }
} 'pptx-zoom-geom-inspect-size'
$itemS = @($treeS.slides[0].zooms) | Select-Object -First 1
$presS = Open-PptPres $ppt $sized
$capS = Get-PptCaption $presS
$s1 = Get-PptSlide $presS 1
$zoomS = Get-PptZoomShape $s1.Shapes
$wPt = $null
$hPt = $null
if ($null -ne $zoomS) {
    $wPt = [math]::Round([double]$zoomS.Width, 2)
    $hPt = [math]::Round([double]$zoomS.Height, 2)
    Release-Com $zoomS
}
Release-Com $s1
Add-Check 'pptx-Pzg1' (
    ([int]$itemS.cx -eq 1828800) -and ([int]$itemS.cy -eq 914400) -and
    ($capS -notmatch 'Repair') -and ($null -ne $wPt) -and ([math]::Abs($wPt - 144) -lt 2) -and
    ($null -ne $hPt) -and ([math]::Abs($hPt - 72) -lt 2)
) "cx=$($itemS.cx) cy=$($itemS.cy) caption=$capS w=$wPt h=$hPt"
Close-PptPres $presS

$moved = Get-OutPath 'pptx-zoom-geom-moved.pptx'
Close-PptPresByPath $ppt $moved
if (Test-Path $moved) { Remove-Item $moved -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sized.Replace('\', '/')
    output = $moved.Replace('\', '/')
    ops = @(@{ op = 'setZoom'; id = $id; x = 914400; y = 457200 })
} 'pptx-zoom-geom-pos' | Out-Null
$treeM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $moved.Replace('\', '/'); format = 'pptx'
    scope = @{ zooms = $true }
} 'pptx-zoom-geom-inspect-pos'
$itemM = @($treeM.slides[0].zooms) | Select-Object -First 1
$presM = Open-PptPres $ppt $moved
$capM = Get-PptCaption $presM
Add-Check 'pptx-Pzg2' (
    ([int]$itemM.x -eq 914400) -and ([int]$itemM.y -eq 457200) -and
    ([int]$itemM.cx -eq 1828800) -and ([int]$itemM.cy -eq 914400) -and
    ($capM -notmatch 'Repair')
) "x=$($itemM.x) y=$($itemM.y) cx=$($itemM.cx) caption=$capM"
Close-PptPres $presM

$notesOut = Get-OutPath 'pptx-zoom-geom-notes.pptx'
Close-PptPresByPath $ppt $notesOut
if (Test-Path $notesOut) { Remove-Item $notesOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $notesOut.Replace('\', '/')
    ops = @(@{ op = 'insertZoom'; id = 'slide:1/notes'; type = 'slide'; target = 'slide:2' })
} 'pptx-zoom-geom-notes-insert' | Out-Null
$treeN0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $notesOut.Replace('\', '/'); format = 'pptx'
    scope = @{ zooms = $true; notes = $true }
} 'pptx-zoom-geom-inspect-notes-ins'
$nId = [string](@($treeN0.slides[0].notes.zooms) | Select-Object -First 1).id
$notesSz = Get-OutPath 'pptx-zoom-geom-notes-sz.pptx'
Close-PptPresByPath $ppt $notesSz
if (Test-Path $notesSz) { Remove-Item $notesSz -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $notesOut.Replace('\', '/')
    output = $notesSz.Replace('\', '/')
    ops = @(@{ op = 'setZoom'; id = $nId; cx = 1828800; cy = 914400 })
} 'pptx-zoom-geom-notes-size' | Out-Null
$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $notesSz.Replace('\', '/'); format = 'pptx'
    scope = @{ zooms = $true; notes = $true }
} 'pptx-zoom-geom-inspect-notes'
$nItem = @($treeN.slides[0].notes.zooms) | Select-Object -First 1
$presN = Open-PptPres $ppt $notesSz
$capN = Get-PptCaption $presN
Add-Check 'pptx-Pzg3' (
    ([int]$nItem.cx -eq 1828800) -and ([int]$nItem.cy -eq 914400) -and
    ([int]$treeN.zoomCount -eq 0) -and ($capN -notmatch 'Repair')
) "cx=$($nItem.cx) root=$($treeN.zoomCount) caption=$capN"
Close-PptPres $presN

$retarget = Get-OutPath 'pptx-zoom-geom-target.pptx'
Close-PptPresByPath $ppt $retarget
if (Test-Path $retarget) { Remove-Item $retarget -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $moved.Replace('\', '/')
    output = $retarget.Replace('\', '/')
    ops = @(@{ op = 'setZoom'; id = $id; target = 'slide:3' })
} 'pptx-zoom-geom-target' | Out-Null
$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $retarget.Replace('\', '/'); format = 'pptx'
    scope = @{ zooms = $true }
} 'pptx-zoom-geom-inspect-target'
$itemT = @($treeT.slides[0].zooms) | Select-Object -First 1
$presT = Open-PptPres $ppt $retarget
$capT = Get-PptCaption $presT
Add-Check 'pptx-Pzg4' (
    ([string]$itemT.target -eq 'slide:3') -and
    ([int]$itemT.cx -eq 1828800) -and ([int]$itemT.cy -eq 914400) -and
    ([int]$itemT.x -eq 914400) -and ([int]$itemT.y -eq 457200) -and
    ($capT -notmatch 'Repair')
) "target=$($itemT.target) x=$($itemT.x) cx=$($itemT.cx) caption=$capT"
Close-PptPres $presT

$failEmpty = Get-OutPath 'pptx-zoom-geom-fail-empty.pptx'
$rawEmpty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failEmpty.Replace('\', '/')
    ops = @(@{ op = 'setZoom'; id = $id })
} 'pptx-zoom-geom-fail-empty'
$failPair = Get-OutPath 'pptx-zoom-geom-fail-pair.pptx'
$rawPair = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failPair.Replace('\', '/')
    ops = @(@{ op = 'setZoom'; id = $id; cx = 1828800 })
} 'pptx-zoom-geom-fail-pair'
Add-Check 'pptx-Pzg5' (
    ((Get-FailCode $rawEmpty) -eq 'InvalidField') -and
    ((Get-FailCode $rawPair) -eq 'InvalidField') -and
    (-not (Test-Path $failEmpty)) -and (-not (Test-Path $failPair))
) "empty=$(Get-FailCode $rawEmpty) pair=$(Get-FailCode $rawPair)"
