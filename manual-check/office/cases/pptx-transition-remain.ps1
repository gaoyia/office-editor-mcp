# pptx 切换剩余：docs/checklist-pptx.md 的 Tr1–Tr5。

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

function Get-PptErrorCode($raw) {
    try { return ($raw.Stderr | ConvertFrom-Json).error.code } catch { return $null }
}

function Get-PptTransitionRemain($slide) {
    $t = $slide.SlideShowTransition
    try {
        return [pscustomobject]@{
            EntryEffect = [int]$t.EntryEffect
            AdvanceOnClick = [int]$t.AdvanceOnClick
            AdvanceOnTime = [int]$t.AdvanceOnTime
            AdvanceTime = [double]$t.AdvanceTime
        }
    }
    finally { Release-Com $t }
}

$hostPath = Ensure-PptxCreate
$ppt = Ensure-MsPpt $hostPath

# Tr1：定时进片。
$autoOut = Get-OutPath 'pptx-tr-remain-auto-apply.pptx'
Close-PptPresByPath $ppt $autoOut
if (Test-Path $autoOut) { Remove-Item -Force $autoOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $autoOut.Replace('\', '/')
    ops = @(@{ op = 'setTransition'; id = 'slide:1'; type = 'fade'; advTm = 2000 })
} 'pptx-tr-auto' | Out-Null
$treeA = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $autoOut.Replace('\', '/'); format = 'pptx'
} 'pptx-tr-auto-inspect'
$presA = Open-PptPres $ppt $autoOut
$capA = Get-PptCaption $presA
$sA = Get-PptSlide $presA 1
$infoA = Get-PptTransitionRemain $sA
Release-Com $sA
Close-PptPres $presA
$trA = $treeA.slides[0].transition
$tr1ok = ($capA -notmatch 'Repair') `
    -and ([string]$trA.type -eq 'fade') `
    -and ([int]$trA.advTm -eq 2000) `
    -and ([bool]$trA.advClick -eq $true) `
    -and ([int]$infoA.AdvanceOnTime -ne 0) `
    -and ([math]::Abs([double]$infoA.AdvanceTime - 2) -lt 0.05)
Add-Check 'pptx-Tr1' $tr1ok "cap=$capA type=$($trA.type) advTm=$($trA.advTm) click=$($trA.advClick) comTime=$($infoA.AdvanceOnTime) seconds=$($infoA.AdvanceTime)"

# Tr2：wipe 向右。
$wipeOut = Get-OutPath 'pptx-tr-remain-wipe.pptx'
Close-PptPresByPath $ppt $wipeOut
if (Test-Path $wipeOut) { Remove-Item -Force $wipeOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $wipeOut.Replace('\', '/')
    ops = @(@{ op = 'setTransition'; id = 'slide:1'; type = 'wipe'; dir = 'r' })
} 'pptx-tr-wipe' | Out-Null
$treeW = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $wipeOut.Replace('\', '/'); format = 'pptx'
} 'pptx-tr-wipe-inspect'
$presW = Open-PptPres $ppt $wipeOut
$capW = Get-PptCaption $presW
$sW = Get-PptSlide $presW 1
$infoW = Get-PptTransitionRemain $sW
Release-Com $sW
Close-PptPres $presW
$trW = $treeW.slides[0].transition
$tr2ok = ($capW -notmatch 'Repair') `
    -and ([string]$trW.type -eq 'wipe') `
    -and ([string]$trW.dir -eq 'r') `
    -and ([int]$infoW.EntryEffect -eq 2819)
Add-Check 'pptx-Tr2' $tr2ok "cap=$capW type=$($trW.type) dir=$($trW.dir) effect=$($infoW.EntryEffect)"

# Tr3：fallOver。
$fallOut = Get-OutPath 'pptx-tr-remain-fall.pptx'
Close-PptPresByPath $ppt $fallOut
if (Test-Path $fallOut) { Remove-Item -Force $fallOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $fallOut.Replace('\', '/')
    ops = @(@{ op = 'setTransition'; id = 'slide:1'; type = 'fallOver' })
} 'pptx-tr-fall' | Out-Null
$treeF = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $fallOut.Replace('\', '/'); format = 'pptx'
} 'pptx-tr-fall-inspect'
$presF = Open-PptPres $ppt $fallOut
$capF = Get-PptCaption $presF
$sF = Get-PptSlide $presF 1
$infoF = Get-PptTransitionRemain $sF
Release-Com $sF
Close-PptPres $presF
$trF = $treeF.slides[0].transition
$tr3ok = ($capF -notmatch 'Repair') `
    -and ([string]$trF.type -eq 'fallOver') `
    -and ([int]$infoF.EntryEffect -eq 3934)
Add-Check 'pptx-Tr3' $tr3ok "cap=$capF type=$($trF.type) effect=$($infoF.EntryEffect)"

# Tr4：先写 wipe+定时，再改字。
$keepOut = Get-OutPath 'pptx-tr-remain-keep.pptx'
Close-PptPresByPath $ppt $keepOut
if (Test-Path $keepOut) { Remove-Item -Force $keepOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $keepOut.Replace('\', '/')
    ops = @(@{ op = 'setTransition'; id = 'slide:1'; type = 'wipe'; dir = 'r'; advTm = 2000 })
} 'pptx-tr-keep-set' | Out-Null
$shapes = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $keepOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-tr-keep-shapes'
$shapeId = [string]$shapes.slides[0].shapes[0].id
$textOut = Get-OutPath 'pptx-tr-remain-text.pptx'
Close-PptPresByPath $ppt $textOut
if (Test-Path $textOut) { Remove-Item -Force $textOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $keepOut.Replace('\', '/')
    output = $textOut.Replace('\', '/')
    ops = @(@{ op = 'setShapeText'; id = $shapeId; text = $S.shape2new })
} 'pptx-tr-keep-text' | Out-Null
$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $textOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-tr-keep-inspect'
$presT = Open-PptPres $ppt $textOut
$capT = Get-PptCaption $presT
Close-PptPres $presT
$trK = $treeT.slides[0].transition
$tr4ok = ($capT -notmatch 'Repair') `
    -and ([string]$trK.type -eq 'wipe') `
    -and ([string]$trK.dir -eq 'r') `
    -and ([int]$trK.advTm -eq 2000) `
    -and ([string]$treeT.slides[0].shapes[0].text -eq $S.shape2new)
Add-Check 'pptx-Tr4' $tr4ok "cap=$capT type=$($trK.type) dir=$($trK.dir) advTm=$($trK.advTm) text=$($treeT.slides[0].shapes[0].text)"

# Tr5：fade 上 dir / invX。
$badDir = Get-OutPath 'pptx-tr-remain-fail-dir.pptx'
$badInv = Get-OutPath 'pptx-tr-remain-fail-inv.pptx'
Close-PptPresByPath $ppt $badDir
Close-PptPresByPath $ppt $badInv
if (Test-Path $badDir) { Remove-Item -Force $badDir }
if (Test-Path $badInv) { Remove-Item -Force $badInv }
$failDir = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badDir.Replace('\', '/')
    ops = @(@{ op = 'setTransition'; id = 'slide:1'; type = 'fade'; dir = 'r' })
} 'pptx-tr-fail-dir'
$failInv = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badInv.Replace('\', '/')
    ops = @(@{ op = 'setTransition'; id = 'slide:1'; type = 'fade'; invX = $true })
} 'pptx-tr-fail-inv'
$tr5ok = ((Get-PptErrorCode $failDir) -eq 'InvalidField') `
    -and (-not (Test-Path $badDir)) `
    -and ((Get-PptErrorCode $failInv) -eq 'InvalidField') `
    -and (-not (Test-Path $badInv))
Add-Check 'pptx-Tr5' $tr5ok "dir=$(Get-PptErrorCode $failDir) inv=$(Get-PptErrorCode $failInv) dirExists=$(Test-Path $badDir) invExists=$(Test-Path $badInv)"
