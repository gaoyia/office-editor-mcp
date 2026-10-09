# pptx 字填充与轮廓：docs/checklist-pptx.md 的 Tfl1–Tfl5。
# 渐变字、字轮廓、混排 runs。不算排版引擎。COM Fill.Type：渐变=3。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }
Add-Type -AssemblyName System.IO.Compression.FileSystem

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

function Get-ZipEntryText([string]$zipPath, [string]$entryName) {
    $zip = [IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        $e = $zip.GetEntry($entryName)
        if ($null -eq $e) { return $null }
        $sr = New-Object IO.StreamReader($e.Open(), [Text.Encoding]::UTF8)
        try { return $sr.ReadToEnd() }
        finally { $sr.Dispose() }
    }
    finally { $zip.Dispose() }
}

function Replace-First([string]$s, [string]$old, [string]$neu) {
    $i = $s.IndexOf($old)
    if ($i -lt 0) { return $s }
    return $s.Substring(0, $i) + $neu + $s.Substring($i + $old.Length)
}

function Set-ZipEntryText([string]$zipPath, [string]$entryName, [string]$text) {
    $zip = [IO.Compression.ZipFile]::Open($zipPath, 'Update')
    try {
        $e = $zip.GetEntry($entryName)
        if ($null -eq $e) { throw "没有 $entryName" }
        $e.Delete()
        $n = $zip.CreateEntry($entryName)
        $enc = New-Object Text.UTF8Encoding $false
        $sw = New-Object IO.StreamWriter($n.Open(), $enc)
        try { $sw.Write($text); $sw.Flush() }
        finally { $sw.Dispose() }
    }
    finally { $zip.Dispose() }
}

function Read-PptTextFill($shape) {
    $tf2 = $null
    $tr = $null
    $font = $null
    $fill = $null
    $line = $null
    $fillType = $null
    $lineVis = $null
    try {
        $tf2 = $shape.TextFrame2
        $tr = $tf2.TextRange
        $font = $tr.Font
        $fill = $font.Fill
        $fillType = [int]$fill.Type
        $line = $font.Line
        $lineVis = $line.Visible
        return @{ FillType = $fillType; LineVisible = $lineVis }
    }
    finally {
        Release-Com $line
        Release-Com $fill
        Release-Com $font
        Release-Com $tr
        Release-Com $tf2
    }
}

$fill = @{
    type = 'gradient'
    ang = 5400000
    stops = @(
        @{ pos = 0; color = 'FF0000' }
        @{ pos = 100000; color = '0000FF' }
    )
}

$hostPath = Ensure-PptxCreate
$ppt = Ensure-MsPpt $hostPath
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-tfl-inspect-host'
$targetId = [string]$tree0.slides[0].shapes[0].id

$gradOut = Get-OutPath 'pptx-text-fill-grad.pptx'
Close-PptPresByPath $ppt $gradOut
if (Test-Path $gradOut) { Remove-Item -Force $gradOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $gradOut.Replace('\', '/')
    ops = @(@{ op = 'setShapeFormat'; id = $targetId; textFill = $fill })
} 'pptx-tfl-grad' | Out-Null
$treeG = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $gradOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-tfl-inspect-grad'
$shG = @($treeG.slides[0].shapes) | Where-Object { $_.id -eq $targetId } | Select-Object -First 1
$xmlG = Get-ZipEntryText $gradOut 'ppt/slides/slide1.xml'
$presG = Open-PptPres $ppt $gradOut
$capG = Get-PptCaption $presG
$sG = Get-PptSlide $presG 1
$boxG = Get-PptShape $sG ([string]$shG.name)
$comG = Read-PptTextFill $boxG
Release-Com $boxG
Release-Com $sG
Close-PptPres $presG
$tfl1ok = ($capG -notmatch 'Repair') `
    -and ($null -eq $shG.color) `
    -and ([string]$shG.textFill.type -eq 'gradient') `
    -and ([int64]$shG.textFill.ang -eq 5400000) `
    -and ([int64]$shG.textFill.stops[0].pos -eq 0) `
    -and ([string]$shG.textFill.stops[0].color -eq 'FF0000') `
    -and ([int64]$shG.textFill.stops[1].pos -eq 100000) `
    -and ([string]$shG.textFill.stops[1].color -eq '0000FF') `
    -and ($xmlG -match 'gradFill flip="none" rotWithShape="1"') `
    -and ($xmlG -match 'lin ang="5400000" scaled="1"') `
    -and ($xmlG -match '<a:tileRect') `
    -and ($comG.FillType -eq 3)
Add-Check 'pptx-Tfl1' $tfl1ok "cap=$capG type=$($shG.textFill.type) fillType=$($comG.FillType)"

$lnOut = Get-OutPath 'pptx-text-fill-outline.pptx'
Close-PptPresByPath $ppt $lnOut
if (Test-Path $lnOut) { Remove-Item -Force $lnOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $lnOut.Replace('\', '/')
    ops = @(@{
        op = 'setShapeFormat'; id = $targetId
        textOutline = @{ color = 'FF0000'; width = 19050 }
    })
} 'pptx-tfl-outline' | Out-Null
$treeL = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $lnOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-tfl-inspect-outline'
$shL = @($treeL.slides[0].shapes) | Where-Object { $_.id -eq $targetId } | Select-Object -First 1
$xmlL = Get-ZipEntryText $lnOut 'ppt/slides/slide1.xml'
$presL = Open-PptPres $ppt $lnOut
$capL = Get-PptCaption $presL
$sL = Get-PptSlide $presL 1
$boxL = Get-PptShape $sL ([string]$shL.name)
$comL = Read-PptTextFill $boxL
Release-Com $boxL
Release-Com $sL
Close-PptPres $presL
$tfl2ok = ($capL -notmatch 'Repair') `
    -and ([string]$shL.textOutline.color -eq 'FF0000') `
    -and ([int64]$shL.textOutline.width -eq 19050) `
    -and ($xmlL -match 'ln w="19050"') `
    -and (($comL.LineVisible -eq -1) -or ($comL.LineVisible -eq $true))
Add-Check 'pptx-Tfl2' $tfl2ok "cap=$capL w=$($shL.textOutline.width) vis=$($comL.LineVisible)"

$noneOut = Get-OutPath 'pptx-text-fill-none.pptx'
Close-PptPresByPath $ppt $noneOut
if (Test-Path $noneOut) { Remove-Item -Force $noneOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $noneOut.Replace('\', '/')
    ops = @(@{ op = 'setShapeFormat'; id = $targetId; textFill = @{ type = 'none' } })
} 'pptx-tfl-none' | Out-Null
$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $noneOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-tfl-inspect-none'
$shN = @($treeN.slides[0].shapes) | Where-Object { $_.id -eq $targetId } | Select-Object -First 1
$xmlN = Get-ZipEntryText $noneOut 'ppt/slides/slide1.xml'
$presN = Open-PptPres $ppt $noneOut
$capN = Get-PptCaption $presN
Close-PptPres $presN
$tfl3ok = ($capN -notmatch 'Repair') `
    -and ($null -eq $shN.color) `
    -and ([string]$shN.textFill.type -eq 'none') `
    -and ($xmlN -match '<a:noFill')
Add-Check 'pptx-Tfl3' $tfl3ok "cap=$capN color=$($shN.color) type=$($shN.textFill.type)"

$mixOut = Get-OutPath 'pptx-text-fill-runs.pptx'
Close-PptPresByPath $ppt $mixOut
if (Test-Path $mixOut) { Remove-Item -Force $mixOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $mixOut.Replace('\', '/'); format = 'pptx'
    presentation = @{ slides = @(@{ shapes = @(@{ text = "大`n小" }) }) }
} 'pptx-tfl-mix-create' | Out-Null
$xmlM = Get-ZipEntryText $mixOut 'ppt/slides/slide1.xml'
$xmlM2 = Replace-First $xmlM 'sz="1800"' 'sz="2800"'
$xmlM2 = Replace-First $xmlM2 'sz="1800"' 'sz="1200"'
Set-ZipEntryText $mixOut 'ppt/slides/slide1.xml' $xmlM2
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $mixOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-tfl-inspect-runs'
$shR = $treeR.slides[0].shapes[0]
$runs = @($shR.runs)
$tfl4ok = ($null -eq $shR.sz) `
    -and ($runs.Count -eq 2) `
    -and ([int64]$runs[0].sz -eq 2800) `
    -and ([int64]$runs[1].sz -eq 1200)
Add-Check 'pptx-Tfl4' $tfl4ok "sz=$($shR.sz) n=$($runs.Count) a=$($runs[0].sz) b=$($runs[1].sz)"

$failEmpty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = (Get-OutPath 'pptx-text-fill-fail-empty.pptx').Replace('\', '/')
    ops = @(@{ op = 'setShapeFormat'; id = $targetId })
} 'pptx-tfl-fail-empty'
$failBoth = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = (Get-OutPath 'pptx-text-fill-fail-both.pptx').Replace('\', '/')
    ops = @(@{
        op = 'setShapeFormat'; id = $targetId
        color = 'FF0000'
        textFill = @{ type = 'none' }
    })
} 'pptx-tfl-fail-both'
$failEmptyOut = Get-OutPath 'pptx-text-fill-fail-empty.pptx'
$failBothOut = Get-OutPath 'pptx-text-fill-fail-both.pptx'
$codeEmpty = Get-PptErrorCode $failEmpty
$codeBoth = Get-PptErrorCode $failBoth
$tfl5ok = ($failEmpty.ExitCode -ne 0) -and ($failBoth.ExitCode -ne 0) `
    -and ($codeEmpty -eq 'InvalidField') -and ($codeBoth -eq 'InvalidField') `
    -and (-not (Test-Path $failEmptyOut)) -and (-not (Test-Path $failBothOut))
Add-Check 'pptx-Tfl5' $tfl5ok "empty=$($failEmpty.ExitCode)/$codeEmpty both=$($failBoth.ExitCode)/$codeBoth"
