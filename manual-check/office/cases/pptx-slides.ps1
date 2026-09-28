# pptx 幻灯片管理：docs/checklist-pptx.md 的 Pm1–Pm5。
# 用库 create 产物再 apply。顺序跟 sldIdLst。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings

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

function Get-PptShapeCount($slide) {
    $shapes = $slide.Shapes
    try { return [int]$shapes.Count }
    finally { Release-Com $shapes }
}

function Get-PptSlideHidden($slide) {
    $t = $slide.SlideShowTransition
    try { return [bool]$t.Hidden }
    finally { Release-Com $t }
}

$hostPath = Ensure-PptxCreate
$ppt = Ensure-MsPpt $hostPath

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
} 'pptx-pm-inspect'
$ids = @($tree0.slides | ForEach-Object { [string]$_.id })
$hiddens = @($tree0.slides | ForEach-Object { [bool]$_.hidden })
$pm1ok = ([int]$tree0.slideCount -eq 2) `
    -and ($ids.Count -eq 2) `
    -and ($ids[0] -eq 'slide:1') `
    -and ($ids[1] -eq 'slide:2') `
    -and ($hiddens -notcontains $true)
Add-Check 'pptx-Pm1' $pm1ok "slideCount=$($tree0.slideCount) ids=$($ids -join ',') hidden=$($hiddens -join ',')"

$insertOut = Get-OutPath 'pptx-slides-insert.pptx'
if (Test-Path $insertOut) { Remove-Item -Force $insertOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $insertOut.Replace('\', '/')
    ops = @(@{ op = 'insertSlide'; id = 'pptx:presentation'; after = 'slide:1' })
} 'pptx-pm-insert' | Out-Null
$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insertOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 2 }
} 'pptx-pm-insert-inspect'
$treeI3 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insertOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 3 }
} 'pptx-pm-insert-inspect-3'
$newShapes = @($treeI.slides[1].shapes)
$p3text = [string]$treeI3.slides[2].shapes[0].text
$presI = Open-PptPres $ppt $insertOut
$capI = Get-PptCaption $presI
$s2 = Get-PptSlide $presI 2
$n2 = Get-PptShapeCount $s2
$s3 = Get-PptSlide $presI 3
$box3 = Get-PptShape $s3 '文本框 1'
$t3 = Get-ShapeText $box3
$pm2ok = ([int]$treeI.slideCount -eq 3) `
    -and ($newShapes.Count -eq 0) `
    -and ($p3text -eq $S.page2) `
    -and ($capI -notmatch 'Repair') `
    -and ($n2 -eq 0) `
    -and ($t3 -eq $S.page2)
Add-Check 'pptx-Pm2' $pm2ok "count=$($treeI.slideCount) newShapes=$($newShapes.Count) p3=$p3text caption=$capI comCount=$n2 comT3=$t3"
Release-Com $box3
Release-Com $s3
Release-Com $s2
Close-PptPres $presI

$moveOut = Get-OutPath 'pptx-slides-move.pptx'
if (Test-Path $moveOut) { Remove-Item -Force $moveOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insertOut.Replace('\', '/')
    output = $moveOut.Replace('\', '/')
    ops = @(@{ op = 'moveSlide'; id = 'slide:3' })
} 'pptx-pm-move' | Out-Null
$treeM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $moveOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-pm-move-inspect'
$m1text = [string]$treeM.slides[0].shapes[0].text
$presM = Open-PptPres $ppt $moveOut
$capM = Get-PptCaption $presM
$sM1 = Get-PptSlide $presM 1
$boxM = Get-PptShape $sM1 '文本框 1'
$tM = Get-ShapeText $boxM
$pm3ok = ($m1text -eq $S.page2) -and ($capM -notmatch 'Repair') -and ($tM -eq $S.page2)
Add-Check 'pptx-Pm3' $pm3ok "first=$m1text caption=$capM com=$tM"
Release-Com $boxM
Release-Com $sM1
Close-PptPres $presM

$hideOut = Get-OutPath 'pptx-slides-hidden.pptx'
if (Test-Path $hideOut) { Remove-Item -Force $hideOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $moveOut.Replace('\', '/')
    output = $hideOut.Replace('\', '/')
    ops = @(@{ op = 'setSlideHidden'; id = 'slide:1'; hidden = $true })
} 'pptx-pm-hide' | Out-Null
$treeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hideOut.Replace('\', '/'); format = 'pptx'
} 'pptx-pm-hide-inspect'
$presH = Open-PptPres $ppt $hideOut
$capH = Get-PptCaption $presH
$sH = Get-PptSlide $presH 1
$hidden = Get-PptSlideHidden $sH
$pm4ok = ([bool]$treeH.slides[0].hidden -eq $true) -and ($capH -notmatch 'Repair') -and $hidden
Add-Check 'pptx-Pm4' $pm4ok "inspectHidden=$($treeH.slides[0].hidden) caption=$capH comHidden=$hidden"
Release-Com $sH
Close-PptPres $presH

$onePath = Get-OutPath 'pptx-slides-one.pptx'
if (Test-Path $onePath) { Remove-Item -Force $onePath }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $onePath.Replace('\', '/'); format = 'pptx'
    presentation = @{ slides = @(@{ shapes = @(@{ text = $S.title }) }) }
} 'pptx-pm-one' | Out-Null
$lastOut = Get-OutPath 'pptx-slides-last.pptx'
if (Test-Path $lastOut) { Remove-Item -Force $lastOut }
$rawLast = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $onePath.Replace('\', '/')
    output = $lastOut.Replace('\', '/')
    ops = @(@{ op = 'deleteSlide'; id = 'slide:1' })
} 'pptx-pm-last'
$codeLast = $null
try { $codeLast = ($rawLast.Stderr | ConvertFrom-Json).error.code } catch {}
$noLast = -not (Test-Path $lastOut)

$delOut = Get-OutPath 'pptx-slides-delete.pptx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insertOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteSlide'; id = 'slide:2' })
} 'pptx-pm-delete' | Out-Null
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'pptx'
} 'pptx-pm-delete-inspect'
$presD = Open-PptPres $ppt $delOut
$capD = Get-PptCaption $presD
Close-PptPres $presD

$missOut = Get-OutPath 'pptx-slides-no-hidden.pptx'
if (Test-Path $missOut) { Remove-Item -Force $missOut }
$rawMiss = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $missOut.Replace('\', '/')
    ops = @(@{ op = 'setSlideHidden'; id = 'slide:1' })
} 'pptx-pm-no-hidden'
$codeMiss = $null
try { $codeMiss = ($rawMiss.Stderr | ConvertFrom-Json).error.code } catch {}
$noMiss = -not (Test-Path $missOut)

$pm5ok = ($rawLast.ExitCode -ne 0) -and ($codeLast -eq 'InvalidField') -and $noLast `
    -and ([int]$treeD.slideCount -eq 2) -and ($capD -notmatch 'Repair') `
    -and ($rawMiss.ExitCode -ne 0) -and ($codeMiss -eq 'InvalidField') -and $noMiss
Add-Check 'pptx-Pm5' $pm5ok "last=$codeLast wroteLast=$(-not $noLast) count=$($treeD.slideCount) caption=$capD miss=$codeMiss"
