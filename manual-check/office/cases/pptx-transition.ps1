# pptx 幻灯片切换：docs/checklist-pptx.md 的 Tn1–Tn5。
# 源文件：PowerPoint 亲手另存，第一页淡入。不要用库 create 产物当带切换的唯一样例。

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

function Get-PptTransitionInfo($slide) {
    $t = $slide.SlideShowTransition
    try {
        return [pscustomobject]@{
            EntryEffect = [int]$t.EntryEffect
            Duration = [double]$t.Duration
        }
    }
    finally { Release-Com $t }
}

function Set-PptFade($slide) {
    $t = $slide.SlideShowTransition
    try {
        # ppEffectFadeSmoothly = 3849
        $t.EntryEffect = 3849
        $t.Duration = 0.5
    }
    finally { Release-Com $t }
}

function Test-PptxFadeSource([string]$pptx, [string]$reqName) {
    $tree = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $pptx.Replace('\', '/'); format = 'pptx'
    } $reqName
    $tr = $tree.slides[0].transition
    $type = $null
    $dur = $null
    if ($null -ne $tr -and $tr -ne [DBNull]::Value) {
        try { $type = [string]$tr.type } catch {}
        try { $dur = [int]$tr.dur } catch {}
    }
    return [pscustomobject]@{
        type = $type
        dur = $dur
        ok = ($type -eq 'fade') -and ($null -ne $dur)
    }
}

function Ensure-PptxPptFadeSource {
    $path = Get-OutPath 'pptx-ppt-fade-source.pptx'
    if (Test-Path $path) {
        $cached = Test-PptxFadeSource $path 'pptx-tn-source-cache-check'
        if ($cached.ok) { return $path }
        Write-Log "缓存的 pptx fade source 不合格（type=$($cached.type) dur=$($cached.dur)），重建"
    }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $tmp = Join-Path $tmpDir 'pptx-ppt-fade-source.pptx'
    $hostPath = Ensure-PptxCreate
    Copy-Item -Force $hostPath $tmp

    $ppt = Ensure-MsPpt $hostPath
    Close-PptPresByPath $ppt $tmp
    $pres = Open-PptPres $ppt $tmp
    $s1 = Get-PptSlide $pres 1
    Set-PptFade $s1
    Write-Log 'PowerPoint Save() 原地覆盖 TEMP 里的淡入源文件'
    $pres.Save()
    Release-Com $s1
    Close-PptPres $pres
    Copy-Item -Force $tmp $path
    Write-Log "saved ppt fade source $path"

    $check = Test-PptxFadeSource $path 'pptx-tn-source-saved-inspect'
    if (-not $check.ok) {
        throw "PowerPoint 另存的淡入源不合格：type=$($check.type) dur=$($check.dur)"
    }
    return $path
}

function Test-FadeEffect([int]$effect) {
    # ppEffectFade = 1793；ppEffectFadeSmoothly = 3849
    return ($effect -eq 1793) -or ($effect -eq 3849)
}

$hostPath = Ensure-PptxCreate
$src = Ensure-PptxPptFadeSource
$ppt = Ensure-MsPpt $hostPath
Close-PptPresByPath $ppt $src

# Tn1：无切换页为 null；另存淡入页 type=fade 且有 dur。
$treeCreate = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
} 'pptx-tn-inspect-create'
$treeSrc = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
} 'pptx-tn-inspect-src'
$tr0 = $treeCreate.slides[0].transition
$trSrc = $treeSrc.slides[0].transition
$tn1ok = ($null -eq $tr0) `
    -and ($null -ne $trSrc) `
    -and ([string]$trSrc.type -eq 'fade') `
    -and ($null -ne $trSrc.dur)
Add-Check 'pptx-Tn1' $tn1ok "create=$tr0 srcType=$($trSrc.type) srcDur=$($trSrc.dur)"

# Tn2：对 create 产物设 fade，放映能看见，inspect 对上，无 Repair。
$fadeOut = Get-OutPath 'pptx-transition-fade.pptx'
Close-PptPresByPath $ppt $fadeOut
if (Test-Path $fadeOut) { Remove-Item -Force $fadeOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $fadeOut.Replace('\', '/')
    ops = @(@{ op = 'setTransition'; id = 'slide:1'; type = 'fade' })
} 'pptx-tn-fade' | Out-Null
$treeF = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $fadeOut.Replace('\', '/'); format = 'pptx'
} 'pptx-tn-fade-inspect'
$presF = Open-PptPres $ppt $fadeOut
$capF = Get-PptCaption $presF
$sF = Get-PptSlide $presF 1
$infoF = Get-PptTransitionInfo $sF
Release-Com $sF
Close-PptPres $presF
$tn2ok = ($capF -notmatch 'Repair') `
    -and ([string]$treeF.slides[0].transition.type -eq 'fade') `
    -and (Test-FadeEffect ([int]$infoF.EntryEffect))
Add-Check 'pptx-Tn2' $tn2ok "cap=$capF type=$($treeF.slides[0].transition.type) effect=$($infoF.EntryEffect) dur=$($infoF.Duration)"

# Tn3：另存样例只改字，切换还在。
$srcShapes = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-tn-src-shapes'
$shapeId = [string]$srcShapes.slides[0].shapes[0].id
$textOut = Get-OutPath 'pptx-transition-text.pptx'
Close-PptPresByPath $ppt $textOut
if (Test-Path $textOut) { Remove-Item -Force $textOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $textOut.Replace('\', '/')
    ops = @(@{ op = 'setShapeText'; id = $shapeId; text = $S.shape2new })
} 'pptx-tn-text' | Out-Null
$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $textOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-tn-text-inspect'
$presT = Open-PptPres $ppt $textOut
$capT = Get-PptCaption $presT
$sT = Get-PptSlide $presT 1
$infoT = Get-PptTransitionInfo $sT
Release-Com $sT
Close-PptPres $presT
$tn3ok = ($capT -notmatch 'Repair') `
    -and ([string]$treeT.slides[0].transition.type -eq 'fade') `
    -and (Test-FadeEffect ([int]$infoT.EntryEffect)) `
    -and ([string]$treeT.slides[0].shapes[0].text -eq $S.shape2new)
Add-Check 'pptx-Tn3' $tn3ok "cap=$capT type=$($treeT.slides[0].transition.type) effect=$($infoT.EntryEffect) text=$($treeT.slides[0].shapes[0].text)"

# Tn4：type null 后 transition 为 null。
$clearOut = Get-OutPath 'pptx-transition-clear.pptx'
Close-PptPresByPath $ppt $clearOut
if (Test-Path $clearOut) { Remove-Item -Force $clearOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $clearOut.Replace('\', '/')
    ops = @(@{ op = 'setTransition'; id = 'slide:1'; type = $null })
} 'pptx-tn-clear' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $clearOut.Replace('\', '/'); format = 'pptx'
} 'pptx-tn-clear-inspect'
$presC = Open-PptPres $ppt $clearOut
$capC = Get-PptCaption $presC
$sC = Get-PptSlide $presC 1
$infoC = Get-PptTransitionInfo $sC
Release-Com $sC
Close-PptPres $presC
$tn4ok = ($capC -notmatch 'Repair') `
    -and ($null -eq $treeC.slides[0].transition) `
    -and ([int]$infoC.EntryEffect -eq 0)
Add-Check 'pptx-Tn4' $tn4ok "cap=$capC tr=$($treeC.slides[0].transition) effect=$($infoC.EntryEffect)"

# Tn5：形状 id 得 InvalidId；type other 得 InvalidField；均未写文件。
$badId = Get-OutPath 'pptx-transition-fail-id.pptx'
$badType = Get-OutPath 'pptx-transition-fail-type.pptx'
Close-PptPresByPath $ppt $badId
Close-PptPresByPath $ppt $badType
if (Test-Path $badId) { Remove-Item -Force $badId }
if (Test-Path $badType) { Remove-Item -Force $badType }
$failId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badId.Replace('\', '/')
    ops = @(@{ op = 'setTransition'; id = 'slide:1/shape:2'; type = 'fade' })
} 'pptx-tn-fail-id'
$failType = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badType.Replace('\', '/')
    ops = @(@{ op = 'setTransition'; id = 'slide:1'; type = 'other' })
} 'pptx-tn-fail-type'
$tn5ok = ((Get-PptErrorCode $failId) -eq 'InvalidId') `
    -and (-not (Test-Path $badId)) `
    -and ((Get-PptErrorCode $failType) -eq 'InvalidField') `
    -and (-not (Test-Path $badType))
Add-Check 'pptx-Tn5' $tn5ok "id=$(Get-PptErrorCode $failId) type=$(Get-PptErrorCode $failType) idExists=$(Test-Path $badId) typeExists=$(Test-Path $badType)"
