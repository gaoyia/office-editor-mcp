# pptx 备注页尺寸：docs/checklist-pptx.md 的 Nsz1–Nsz5。
# 编号 Nsz，不要和 Ts、Ns 撞。不按新画布重排。不要 Stop-Process PowerPoint。

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

$hostPath = Get-OutPath 'pptx-nsz-src.pptx'
if (Test-Path $hostPath) { Remove-Item -Force $hostPath }
$createReq = Join-Path $global:OoxpReqDir 'pptx-nsz-create.json'
$createJson = '{"schemaVersion":1,"action":"create","path":"' + $hostPath.Replace('\', '/') + '","presentation":{"slides":[{"shapes":[{"text":"标题"}],"notes":"备注"}]}}'
[System.IO.File]::WriteAllText($createReq, $createJson, $global:OoxpUtf8)
Invoke-Ooxp $createReq | Out-Null

$ppt = Ensure-MsPpt $hostPath

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    scope = @{ notesMaster = $true }
} 'pptx-nsz-inspect'
$keys = @(); $cx0 = $null; $cy0 = $null
$phX = $null; $phY = $null; $phCx = $null; $phCy = $null; $sldCx = $null
try {
    $keys = @($tree0.PSObject.Properties.Name)
    $cx0 = [int64]$tree0.notesSize.cx
    $cy0 = [int64]$tree0.notesSize.cy
    $sldCx = [int64]$tree0.slideSize.cx
    $ph = $tree0.notesMaster.placeholders[0]
    $phX = $ph.x; $phY = $ph.y; $phCx = $ph.cx; $phCy = $ph.cy
} catch {}
$slideIdx = [Array]::IndexOf($keys, 'slideSize')
$notesIdx = [Array]::IndexOf($keys, 'notesSize')
$nsz1ok = ($notesIdx -eq ($slideIdx + 1)) -and ($cx0 -eq 6858000) -and ($cy0 -eq 9144000)
Add-Check 'pptx-Nsz1' $nsz1ok "idx=$notesIdx after=$slideIdx cx=$cx0 cy=$cy0"

$sizeOut = Get-OutPath 'pptx-nsz-size.pptx'
Close-PptPresByPath $ppt $sizeOut
if (Test-Path $sizeOut) { Remove-Item -Force $sizeOut }
$sizeReq = Join-Path $global:OoxpReqDir 'pptx-nsz-size.json'
$sizeJson = '{"schemaVersion":1,"action":"apply","path":"' + $hostPath.Replace('\', '/') + '","output":"' + $sizeOut.Replace('\', '/') + '","ops":[{"op":"setNotesSize","id":"pptx:presentation","cx":9144000,"cy":12192000}]}'
[System.IO.File]::WriteAllText($sizeReq, $sizeJson, $global:OoxpUtf8)
Invoke-Ooxp $sizeReq | Out-Null
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sizeOut.Replace('\', '/'); format = 'pptx'
    scope = @{ notesMaster = $true }
} 'pptx-nsz-size-inspect'
$cx = $null; $cy = $null; $sld = $null
try {
    $cx = [int64]$treeS.notesSize.cx
    $cy = [int64]$treeS.notesSize.cy
    $sld = [int64]$treeS.slideSize.cx
} catch {}
$presS = Open-PptPres $ppt $sizeOut
$capS = Get-PptCaption $presS
Close-PptPres $presS
$nsz2ok = ($cx -eq 9144000) -and ($cy -eq 12192000) -and ($sld -eq $sldCx) -and ($capS -notmatch 'Repair')
Add-Check 'pptx-Nsz2' $nsz2ok "cx=$cx cy=$cy sld=$sld cap=$capS"

$ph2x = $null; $ph2y = $null; $ph2cx = $null; $ph2cy = $null
try {
    $ph2 = $treeS.notesMaster.placeholders[0]
    $ph2x = $ph2.x; $ph2y = $ph2.y; $ph2cx = $ph2.cx; $ph2cy = $ph2.cy
} catch {}
$nsz3ok = ($ph2x -eq $phX) -and ($ph2y -eq $phY) -and ($ph2cx -eq $phCx) -and ($ph2cy -eq $phCy) -and ($capS -notmatch 'Repair')
Add-Check 'pptx-Nsz3' $nsz3ok "x=$ph2x y=$ph2y cx=$ph2cx cy=$ph2cy"

$createOut = Get-OutPath 'pptx-nsz-create-size.pptx'
if (Test-Path $createOut) { Remove-Item -Force $createOut }
$createSizeReq = Join-Path $global:OoxpReqDir 'pptx-nsz-create-size.json'
$createSizeJson = '{"schemaVersion":1,"action":"create","path":"' + $createOut.Replace('\', '/') + '","presentation":{"notesSize":{"cx":9144000,"cy":12192000},"slides":[{"shapes":[{"text":"标题"}]}]}}'
[System.IO.File]::WriteAllText($createSizeReq, $createSizeJson, $global:OoxpUtf8)
Invoke-Ooxp $createSizeReq | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $createOut.Replace('\', '/'); format = 'pptx'
} 'pptx-nsz-create-inspect'
$ccx = $null; $ccy = $null
try {
    $ccx = [int64]$treeC.notesSize.cx
    $ccy = [int64]$treeC.notesSize.cy
} catch {}
$presC = Open-PptPres $ppt $createOut
$capC = Get-PptCaption $presC
Close-PptPres $presC
$nsz4ok = ($ccx -eq 9144000) -and ($ccy -eq 12192000) -and ($capC -notmatch 'Repair')
Add-Check 'pptx-Nsz4' $nsz4ok "cx=$ccx cy=$ccy cap=$capC"

$failSlideOut = Get-OutPath 'pptx-nsz-fail-slide.pptx'
$failFieldOut = Get-OutPath 'pptx-nsz-fail-field.pptx'
$failXlsxOut = Get-OutPath 'pptx-nsz-fail.xlsx'
$failDocxOut = Get-OutPath 'pptx-nsz-fail.docx'
foreach ($p in @($failSlideOut, $failFieldOut, $failXlsxOut, $failDocxOut)) {
    if (Test-Path $p) { Remove-Item -Force $p }
}
$failSlide = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failSlideOut.Replace('\', '/')
    ops = @(@{ op = 'setNotesSize'; id = 'slide:1'; cx = 9144000; cy = 12192000 })
} 'pptx-nsz-fail-slide'
$failField = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failFieldOut.Replace('\', '/')
    ops = @(@{ op = 'setNotesSize'; id = 'pptx:presentation'; cx = 9144000 })
} 'pptx-nsz-fail-field'

$xlsxPath = Get-OutPath 'pptx-nsz.xlsx'
if (Test-Path $xlsxPath) { Remove-Item -Force $xlsxPath }
$xlsxReq = Join-Path $global:OoxpReqDir 'pptx-nsz-xlsx.json'
$xlsxJson = '{"schemaVersion":1,"action":"create","path":"' + $xlsxPath.Replace('\', '/') + '","workbook":{"sheets":[{"name":"Sheet1","cells":[{"ref":"A1","value":1}]}]}}'
[System.IO.File]::WriteAllText($xlsxReq, $xlsxJson, $global:OoxpUtf8)
Invoke-Ooxp $xlsxReq | Out-Null
$failXlsx = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsxPath.Replace('\', '/')
    output = $failXlsxOut.Replace('\', '/')
    ops = @(@{ op = 'setNotesSize'; id = 'pptx:presentation'; cx = 9144000; cy = 12192000 })
} 'pptx-nsz-fail-xlsx'

$docxPath = Get-OutPath 'pptx-nsz.docx'
if (Test-Path $docxPath) { Remove-Item -Force $docxPath }
$docxReq = Join-Path $global:OoxpReqDir 'pptx-nsz-docx.json'
$docxJson = '{"schemaVersion":1,"action":"create","path":"' + $docxPath.Replace('\', '/') + '","document":{"paragraphs":[{"text":"甲"}]}}'
[System.IO.File]::WriteAllText($docxReq, $docxJson, $global:OoxpUtf8)
Invoke-Ooxp $docxReq | Out-Null
$failDocx = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $docxPath.Replace('\', '/')
    output = $failDocxOut.Replace('\', '/')
    ops = @(@{ op = 'setNotesSize'; id = 'pptx:presentation'; cx = 9144000; cy = 12192000 })
} 'pptx-nsz-fail-docx'

$nsz5ok = ((Get-PptErrorCode $failSlide) -eq 'InvalidId') `
    -and ((Get-PptErrorCode $failField) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failXlsx) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failDocx) -eq 'InvalidField') `
    -and (-not (Test-Path $failSlideOut)) `
    -and (-not (Test-Path $failFieldOut)) `
    -and (-not (Test-Path $failXlsxOut)) `
    -and (-not (Test-Path $failDocxOut))
Add-Check 'pptx-Nsz5' $nsz5ok "slide=$(Get-PptErrorCode $failSlide) field=$(Get-PptErrorCode $failField) xlsx=$(Get-PptErrorCode $failXlsx) docx=$(Get-PptErrorCode $failDocx)"
