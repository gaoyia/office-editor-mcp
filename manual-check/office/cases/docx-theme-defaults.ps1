# docx 主题 objectDefaults：docs/checklist-docx.md 的 Tdf1–Tdf5。
# 编号 Tdf，不要和 Th、Thr、Fm 撞。不算主题排版引擎。不要 Stop-Process Word。

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

$word = Ensure-MsWord (Ensure-DocxCreate)

$src = Get-OutPath 'docx-theme-defaults.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(@{ text = 'ThemeDefaults' })
        theme = @{ name = 'Office Theme' }
    }
} 'docx-tdf-create' | Out-Null
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/')
} 'docx-tdf-inspect'
Close-WordDocByPath $word $src
$doc = Open-WordDoc $word $src
$cap0 = [string]$doc.ActiveWindow.Caption
Close-WordDoc $doc
$od0 = $tree0.theme.objectDefaults
Add-Check 'docx-Tdf1' (
    ($cap0 -notmatch 'Repair') -and
    ($null -eq $od0)
) "caption=$cap0 objectDefaults=$od0"

$lnOut = Get-OutPath 'docx-theme-defaults-lnw.docx'
$lnJsonPath = Join-Path $global:OoxpReqDir 'docx-tdf-lnw.json'
$lnJson = '{"schemaVersion":1,"action":"apply","path":"' + $src.Replace('\', '/') + '","output":"' + $lnOut.Replace('\', '/') + '","ops":[{"op":"setThemeObjectDefaults","id":"docx:document","objectDefaults":{"sp":{"lnW":38100}}}]}'
[System.IO.File]::WriteAllText($lnJsonPath, $lnJson, $global:OoxpUtf8)
Invoke-Ooxp $lnJsonPath | Out-Null
$treeL = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $lnOut.Replace('\', '/')
} 'docx-tdf-lnw-inspect'
Close-WordDocByPath $word $lnOut
$doc = Open-WordDoc $word $lnOut
$capL = [string]$doc.ActiveWindow.Caption
Close-WordDoc $doc
$lnW = $treeL.theme.objectDefaults.sp.lnW
Add-Check 'docx-Tdf2' (
    ($capL -notmatch 'Repair') -and
    ($lnW -eq 38100)
) "caption=$capL lnW=$lnW"

$stOut = Get-OutPath 'docx-theme-defaults-style.docx'
$stJsonPath = Join-Path $global:OoxpReqDir 'docx-tdf-style.json'
$stJson = '{"schemaVersion":1,"action":"apply","path":"' + $lnOut.Replace('\', '/') + '","output":"' + $stOut.Replace('\', '/') + '","ops":[{"op":"setThemeObjectDefaults","id":"docx:document","objectDefaults":{"sp":{"style":{"lnRef":1,"fontRef":"minor"}}}}]}'
[System.IO.File]::WriteAllText($stJsonPath, $stJson, $global:OoxpUtf8)
Invoke-Ooxp $stJsonPath | Out-Null
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $stOut.Replace('\', '/')
} 'docx-tdf-style-inspect'
Close-WordDocByPath $word $stOut
$doc = Open-WordDoc $word $stOut
$capS = [string]$doc.ActiveWindow.Caption
Close-WordDoc $doc
$lnRef = $treeS.theme.objectDefaults.sp.style.lnRef
$fontRef = [string]$treeS.theme.objectDefaults.sp.style.fontRef
$lnW2 = $treeS.theme.objectDefaults.sp.lnW
Add-Check 'docx-Tdf3' (
    ($capS -notmatch 'Repair') -and
    ($lnRef -eq 1) -and
    ($fontRef -eq 'minor') -and
    ($lnW2 -eq 38100)
) "caption=$capS lnRef=$lnRef fontRef=$fontRef lnW=$lnW2"

$clOut = Get-OutPath 'docx-theme-defaults-clear.docx'
$clJsonPath = Join-Path $global:OoxpReqDir 'docx-tdf-clear.json'
$clJson = '{"schemaVersion":1,"action":"apply","path":"' + $stOut.Replace('\', '/') + '","output":"' + $clOut.Replace('\', '/') + '","ops":[{"op":"setThemeObjectDefaults","id":"docx:document","objectDefaults":null}]}'
[System.IO.File]::WriteAllText($clJsonPath, $clJson, $global:OoxpUtf8)
Invoke-Ooxp $clJsonPath | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $clOut.Replace('\', '/')
} 'docx-tdf-clear-inspect'
Close-WordDocByPath $word $clOut
$doc = Open-WordDoc $word $clOut
$capC = [string]$doc.ActiveWindow.Caption
Close-WordDoc $doc
$odC = $treeC.theme.objectDefaults
Add-Check 'docx-Tdf4' (
    ($capC -notmatch 'Repair') -and
    ($null -eq $odC)
) "caption=$capC objectDefaults=$odC"

$failId = Get-OutPath 'docx-theme-defaults-bad-id.docx'
$badId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failId.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeObjectDefaults'; id = 'para:index:1'; objectDefaults = @{ sp = @{ lnW = 1 } } }
    )
} 'docx-tdf-bad-id'
$failMiss = Get-OutPath 'docx-theme-defaults-missing.docx'
$miss = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failMiss.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeObjectDefaults'; id = 'docx:document' }
    )
} 'docx-tdf-missing'
$failFont = Get-OutPath 'docx-theme-defaults-font.docx'
$font = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failFont.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeObjectDefaults'; id = 'docx:document'; objectDefaults = @{ sp = @{ style = @{ fontRef = 'latin' } } } }
    )
} 'docx-tdf-font'
$skel = Get-OutPath 'docx-theme-defaults-notheme.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $skel.Replace('\', '/'); format = 'docx'
    document = @{ paragraphs = @(@{ text = 'no theme' }) }
} 'docx-tdf-skel' | Out-Null
$failNone = Get-OutPath 'docx-theme-defaults-notheme-out.docx'
$none = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $skel.Replace('\', '/')
    output = $failNone.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeObjectDefaults'; id = 'docx:document'; objectDefaults = @{ sp = @{ lnW = 1 } } }
    )
} 'docx-tdf-notheme'
Add-Check 'docx-Tdf5' (
    ((Get-FailCode $badId) -eq 'InvalidId') -and
    ((Get-FailCode $miss) -eq 'InvalidField') -and
    ((Get-FailCode $font) -eq 'InvalidField') -and
    ((Get-FailCode $none) -eq 'IdNotFound') -and
    -not (Test-Path $failId) -and
    -not (Test-Path $failMiss) -and
    -not (Test-Path $failFont) -and
    -not (Test-Path $failNone)
) "id=$(Get-FailCode $badId) miss=$(Get-FailCode $miss) font=$(Get-FailCode $font) none=$(Get-FailCode $none)"
