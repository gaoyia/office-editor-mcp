# pptx 主题与幻灯片尺寸：docs/checklist-pptx.md 的 Ts1–Ts5。
# 改 p:sldSz 与 theme 配色 / 字体。不按新画布重排，不实现放映引擎。

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

function Get-PptErrorCode($raw) {
    try { return ($raw.Stderr | ConvertFrom-Json).error.code } catch { return $null }
}

$hostPath = Ensure-PptxCreate
$ppt = Ensure-MsPpt $hostPath

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-ts-before'
$x0 = $null; $y0 = $null; $cx0 = $null; $cy0 = $null
try {
    $sp0 = $tree0.slides[0].shapes[0]
    $x0 = $sp0.x; $y0 = $sp0.y; $cx0 = $sp0.cx; $cy0 = $sp0.cy
} catch {}

# Ts1 / Ts2：同一次 setSlideSize。
$sizeOut = Get-OutPath 'pptx-ts-size.pptx'
Close-PptPresByPath $ppt $sizeOut
if (Test-Path $sizeOut) { Remove-Item -Force $sizeOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $sizeOut.Replace('\', '/')
    ops = @(@{
        op = 'setSlideSize'; id = 'pptx:presentation'
        cx = 9144000; cy = 6858000
    })
} 'pptx-ts-size' | Out-Null
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sizeOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-ts-size-inspect'
$cx = $null
try { $cx = [int64]$treeS.slideSize.cx } catch {}
$presS = Open-PptPres $ppt $sizeOut
$capS = Get-PptCaption $presS
$w = $null
$ps = $null
try {
    $ps = $presS.PageSetup
    $w = [double]$ps.SlideWidth
} finally {
    if ($null -ne $ps) { Release-Com $ps }
}
Close-PptPres $presS
$ts1ok = ($cx -eq 9144000) `
    -and ([math]::Abs($w - 720) -lt 0.01) `
    -and ($capS -notmatch 'Repair')
Add-Check 'pptx-Ts1' $ts1ok "cx=$cx comWidth=$w cap=$capS"

$x1 = $null; $y1 = $null; $cx1 = $null; $cy1 = $null
try {
    $sp1 = $treeS.slides[0].shapes[0]
    $x1 = $sp1.x; $y1 = $sp1.y; $cx1 = $sp1.cx; $cy1 = $sp1.cy
} catch {}
$ts2ok = ($null -ne $x0) `
    -and ($x1 -eq $x0) -and ($y1 -eq $y0) `
    -and ($cx1 -eq $cx0) -and ($cy1 -eq $cy0)
Add-Check 'pptx-Ts2' $ts2ok "before=$x0,$y0,$cx0,$cy0 after=$x1,$y1,$cx1,$cy1"

# Ts3：accent1 = FF0000。
$colorOut = Get-OutPath 'pptx-ts-colors.pptx'
Close-PptPresByPath $ppt $colorOut
if (Test-Path $colorOut) { Remove-Item -Force $colorOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $colorOut.Replace('\', '/')
    ops = @(@{ op = 'setThemeColors'; id = 'master:1'; accent1 = 'FF0000' })
} 'pptx-ts-colors' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $colorOut.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-ts-colors-inspect'
$accent = $null
try { $accent = [string]$treeC.masters[0].theme.colors.accent1 } catch {}
$presC = Open-PptPres $ppt $colorOut
$capC = Get-PptCaption $presC
Close-PptPres $presC
$ts3ok = ($accent -eq 'FF0000') -and ($capC -notmatch 'Repair')
Add-Check 'pptx-Ts3' $ts3ok "accent1=$accent cap=$capC"

# Ts4：majorLatin = Arial。
$fontOut = Get-OutPath 'pptx-ts-fonts.pptx'
Close-PptPresByPath $ppt $fontOut
if (Test-Path $fontOut) { Remove-Item -Force $fontOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $fontOut.Replace('\', '/')
    ops = @(@{ op = 'setThemeFonts'; id = 'master:1'; majorLatin = 'Arial' })
} 'pptx-ts-fonts' | Out-Null
$treeF = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $fontOut.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-ts-fonts-inspect'
$face = $null
try { $face = [string]$treeF.masters[0].theme.fonts.majorLatin } catch {}
$presF = Open-PptPres $ppt $fontOut
$capF = Get-PptCaption $presF
Close-PptPres $presF
$ts4ok = ($face -eq 'Arial') -and ($capF -notmatch 'Repair')
Add-Check 'pptx-Ts4' $ts4ok "majorLatin=$face cap=$capF"

# Ts5：页 id / 越界 / 坏 type / xlsx / docx。
$failSlideOut = Get-OutPath 'pptx-ts-fail-slide.pptx'
$failCxOut = Get-OutPath 'pptx-ts-fail-cx.pptx'
$failTypeOut = Get-OutPath 'pptx-ts-fail-type.pptx'
if (Test-Path $failSlideOut) { Remove-Item -Force $failSlideOut }
if (Test-Path $failCxOut) { Remove-Item -Force $failCxOut }
if (Test-Path $failTypeOut) { Remove-Item -Force $failTypeOut }
$failSlide = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failSlideOut.Replace('\', '/')
    ops = @(@{ op = 'setSlideSize'; id = 'slide:1'; cx = 9144000; cy = 6858000 })
} 'pptx-ts-fail-slide'
$failCx = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failCxOut.Replace('\', '/')
    ops = @(@{ op = 'setSlideSize'; id = 'pptx:presentation'; cx = 1; cy = 6858000 })
} 'pptx-ts-fail-cx'
$failType = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failTypeOut.Replace('\', '/')
    ops = @(@{ op = 'setSlideSize'; id = 'pptx:presentation'; cx = 9144000; cy = 6858000; type = 'other' })
} 'pptx-ts-fail-type'
$xlsx = Ensure-XlsxCreate
$docx = Ensure-DocxCreate
$badX = Get-OutPath 'pptx-ts-fail-xlsx.xlsx'
$badD = Get-OutPath 'pptx-ts-fail-docx.docx'
if (Test-Path $badX) { Remove-Item -Force $badX }
if (Test-Path $badD) { Remove-Item -Force $badD }
$failX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $badX.Replace('\', '/')
    ops = @(@{ op = 'setSlideSize'; id = 'pptx:presentation'; cx = 9144000; cy = 6858000 })
} 'pptx-ts-fail-xlsx'
$failD = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $docx.Replace('\', '/')
    output = $badD.Replace('\', '/')
    ops = @(@{ op = 'setSlideSize'; id = 'pptx:presentation'; cx = 9144000; cy = 6858000 })
} 'pptx-ts-fail-docx'
$ts5ok = ((Get-PptErrorCode $failSlide) -eq 'InvalidId') `
    -and ((Get-PptErrorCode $failCx) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failType) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failX) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failD) -eq 'InvalidField') `
    -and (-not (Test-Path $failSlideOut)) `
    -and (-not (Test-Path $failCxOut)) `
    -and (-not (Test-Path $failTypeOut)) `
    -and (-not (Test-Path $badX)) `
    -and (-not (Test-Path $badD))
Add-Check 'pptx-Ts5' $ts5ok "slide=$(Get-PptErrorCode $failSlide) cx=$(Get-PptErrorCode $failCx) type=$(Get-PptErrorCode $failType) x=$(Get-PptErrorCode $failX) d=$(Get-PptErrorCode $failD)"
