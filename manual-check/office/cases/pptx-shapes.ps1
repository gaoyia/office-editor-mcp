# pptx 形状增删与几何：docs/checklist-pptx.md 的 Ps1–Ps5。
# 插文本框、改位置尺寸填色、删除。动画绑着用另存样例，只插新框。

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

function Get-PptShapeCount($slide) {
    $shapes = $slide.Shapes
    try { return [int]$shapes.Count }
    finally { Release-Com $shapes }
}

$hostPath = Ensure-PptxCreate
$ppt = Ensure-MsPpt $hostPath

$insertOut = Get-OutPath 'pptx-shapes-insert.pptx'
Close-PptPresByPath $ppt $insertOut
if (Test-Path $insertOut) { Remove-Item -Force $insertOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $insertOut.Replace('\', '/')
    ops = @(@{
        op = 'insertShape'; id = 'slide:1'; text = '新框'
        x = 914400; y = 1371600; cx = 1828800; cy = 914400
    })
} 'pptx-ps-insert' | Out-Null
$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insertOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-ps-insert-inspect'
$newSh = @($treeI.slides[0].shapes) | Where-Object { [string]$_.text -eq '新框' } | Select-Object -First 1
$presI = Open-PptPres $ppt $insertOut
$capI = Get-PptCaption $presI
$sI = Get-PptSlide $presI 1
$nI = Get-PptShapeCount $sI
Release-Com $sI
Close-PptPres $presI
$ps1ok = ($null -ne $newSh) `
    -and ([string]$newSh.id -like 'slide:1/shape:*') `
    -and ([bool]$newSh.stable) `
    -and ($capI -notmatch 'Repair') `
    -and ($nI -ge 3)
Add-Check 'pptx-Ps1' $ps1ok "id=$($newSh.id) stable=$($newSh.stable) n=$nI cap=$capI"

$geoOut = Get-OutPath 'pptx-shapes-geo.pptx'
Close-PptPresByPath $ppt $geoOut
if (Test-Path $geoOut) { Remove-Item -Force $geoOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insertOut.Replace('\', '/')
    output = $geoOut.Replace('\', '/')
    ops = @(
        @{ op = 'setShapePos'; id = $newSh.id; x = 914400; y = 1371600 }
        @{ op = 'setShapeSize'; id = $newSh.id; cx = 1828800; cy = 914400 }
        @{ op = 'setShapeFill'; id = $newSh.id; color = 'FF0000' }
        @{ op = 'setShapeLine'; id = $newSh.id; color = '0000FF'; width = 12700 }
    )
} 'pptx-ps-geo' | Out-Null
$treeG = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $geoOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-ps-geo-inspect'
$gSh = @($treeG.slides[0].shapes) | Where-Object { $_.id -eq $newSh.id } | Select-Object -First 1
$presG = Open-PptPres $ppt $geoOut
$capG = Get-PptCaption $presG
$sG = Get-PptSlide $presG 1
$boxG = Get-PptShape $sG ([string]$gSh.name)
$left = [double]$boxG.Left
$top = [double]$boxG.Top
$wPt = [double]$boxG.Width
$hPt = [double]$boxG.Height
$fill = $boxG.Fill
$fore = $fill.ForeColor
$rgb = [int]$fore.RGB
Release-Com $fore
Release-Com $fill
Release-Com $boxG
Release-Com $sG
Close-PptPres $presG
$posOk = ([math]::Abs($left - 72) -lt 8) -and ([math]::Abs($top - 108) -lt 8)
$sizeOk = ([math]::Abs($wPt - 144) -lt 8) -and ([math]::Abs($hPt - 72) -lt 8)
$inspOk = ([int64]$gSh.x -eq 914400) -and ([int64]$gSh.y -eq 1371600) `
    -and ([int64]$gSh.cx -eq 1828800) -and ([int64]$gSh.cy -eq 914400)
$ps2ok = ($capG -notmatch 'Repair') -and $posOk -and $sizeOk -and $inspOk -and ($rgb -eq 255)
Add-Check 'pptx-Ps2' $ps2ok "cap=$capG left=$left top=$top w=$wPt h=$hPt rgb=$rgb insp=$($gSh.x),$($gSh.y)"

$textOut = Get-OutPath 'pptx-shapes-text.pptx'
Close-PptPresByPath $ppt $textOut
if (Test-Path $textOut) { Remove-Item -Force $textOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $geoOut.Replace('\', '/')
    output = $textOut.Replace('\', '/')
    ops = @(@{ op = 'setShapeText'; id = $newSh.id; text = '改过' })
} 'pptx-ps-text' | Out-Null
$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $textOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-ps-text-inspect'
$tSh = @($treeT.slides[0].shapes) | Where-Object { $_.id -eq $newSh.id } | Select-Object -First 1
$oldKeep = @($treeT.slides[0].shapes) | Where-Object { $_.id -ne $newSh.id } | Select-Object -First 1
$presT = Open-PptPres $ppt $textOut
$capT = Get-PptCaption $presT
Close-PptPres $presT
$ps3ok = ($capT -notmatch 'Repair') -and ([string]$tSh.text -eq '改过') -and ([string]$oldKeep.text -ne '改过')
Add-Check 'pptx-Ps3' $ps3ok "cap=$capT new=$($tSh.text) old=$($oldKeep.text)"

$delOut = Get-OutPath 'pptx-shapes-delete.pptx'
Close-PptPresByPath $ppt $delOut
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $textOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteShape'; id = $newSh.id })
} 'pptx-ps-delete' | Out-Null
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-ps-delete-inspect'
$stillNew = @($treeD.slides[0].shapes) | Where-Object { $_.id -eq $newSh.id } | Select-Object -First 1
$presD = Open-PptPres $ppt $delOut
$capD = Get-PptCaption $presD
$sD = Get-PptSlide $presD 1
$nD = Get-PptShapeCount $sD
Release-Com $sD
Close-PptPres $presD
$ps4ok = ($capD -notmatch 'Repair') -and ($null -eq $stillNew) -and ($nD -eq 2)
Add-Check 'pptx-Ps4' $ps4ok "cap=$capD still=$($stillNew.id) n=$nD"

$src = Ensure-PptxPptSource
$animSrc = Get-OutPath 'pptx-shapes-anim-src.pptx'
Close-PptPresByPath $ppt $animSrc
Copy-Item -Force $src $animSrc
$presA0 = Open-PptPres $ppt $animSrc
$sA0 = Get-PptSlide $presA0 1
$boxA0 = Get-PptShape $sA0 1
$tl = $sA0.TimeLine
$seq = $tl.MainSequence
$eff = $seq.AddEffect($boxA0, 1)
$nEff0 = [int]$seq.Count
$presA0.Save()
Release-Com $eff
Release-Com $seq
Release-Com $tl
Release-Com $boxA0
Release-Com $sA0
Close-PptPres $presA0

$animOut = Get-OutPath 'pptx-shapes-anim.pptx'
Close-PptPresByPath $ppt $animOut
if (Test-Path $animOut) { Remove-Item -Force $animOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $animSrc.Replace('\', '/')
    output = $animOut.Replace('\', '/')
    ops = @(@{ op = 'insertShape'; id = 'slide:1'; text = '旁观' })
} 'pptx-ps-anim' | Out-Null
$presA = Open-PptPres $ppt $animOut
$capA = Get-PptCaption $presA
$sA = Get-PptSlide $presA 1
$tlA = $sA.TimeLine
$seqA = $tlA.MainSequence
$nEff1 = [int]$seqA.Count
Release-Com $seqA
Release-Com $tlA
Release-Com $sA
Close-PptPres $presA

$failShape = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = (Get-OutPath 'pptx-shapes-fail-id.pptx').Replace('\', '/')
    ops = @(@{ op = 'insertShape'; id = 'slide:1/shape:2' })
} 'pptx-ps-fail-id'
$failCx = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = (Get-OutPath 'pptx-shapes-fail-cx.pptx').Replace('\', '/')
    ops = @(@{ op = 'insertShape'; id = 'slide:1'; cx = 100 })
} 'pptx-ps-fail-cx'
$failColor = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = (Get-OutPath 'pptx-shapes-fail-color.pptx').Replace('\', '/')
    ops = @(@{ op = 'setShapeFill'; id = 'slide:1/shape:2' })
} 'pptx-ps-fail-color'
$failIdOut = Get-OutPath 'pptx-shapes-fail-id.pptx'
$failCxOut = Get-OutPath 'pptx-shapes-fail-cx.pptx'
$failColorOut = Get-OutPath 'pptx-shapes-fail-color.pptx'
$ps5ok = ($capA -notmatch 'Repair') `
    -and ($nEff0 -ge 1) `
    -and ($nEff1 -ge 1) `
    -and ($failShape.ExitCode -ne 0) `
    -and ($failCx.ExitCode -ne 0) `
    -and ($failColor.ExitCode -ne 0) `
    -and (-not (Test-Path $failIdOut)) `
    -and (-not (Test-Path $failCxOut)) `
    -and (-not (Test-Path $failColorOut))
Add-Check 'pptx-Ps5' $ps5ok "cap=$capA eff0=$nEff0 eff1=$nEff1 id=$($failShape.ExitCode) cx=$($failCx.ExitCode) color=$($failColor.ExitCode)"
