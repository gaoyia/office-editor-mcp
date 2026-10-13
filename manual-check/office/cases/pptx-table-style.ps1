# pptx 表样式引用：docs/checklist-pptx.md 的 Psty1–Psty5。
# 编号 Psty，不要和 Pt、Pg、Ts、Sy 撞。探测脚本不进本文件。不要 Stop-Process PowerPoint。
# COM 不一定暴露 tableStyleId GUID，本脚本用 inspect 与 Repair。

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

$defaultStyle = '{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}'
$specStyle = '{5940675A-B579-460E-94D1-54222C63F5DA}'

$createOut = Get-OutPath 'pptx-table-style-create.pptx'
if (Test-Path $createOut) { Remove-Item -Force $createOut }
$createJsonPath = Join-Path $global:OoxpReqDir 'pptx-psty-create.json'
$createPathJson = $createOut.Replace('\', '/')
$createJson = '{"schemaVersion":1,"action":"create","path":"' + $createPathJson + '","presentation":{"slides":[{"shapes":[{"text":"标题"}],"tables":[{"rows":2,"cols":2,"x":914400,"y":1828800,"cx":5486400,"cy":3200400,"cells":[["甲","乙"],["丙","丁"]]}]}]}}'
[System.IO.File]::WriteAllText($createJsonPath, $createJson, $global:OoxpUtf8)
Invoke-Ooxp $createJsonPath | Out-Null

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $createOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-psty-inspect-create'
$t1 = @($tree1.slides[0].tables) | Select-Object -First 1
$tid = [string]$t1.id
$ppt = Ensure-MsPpt $createOut
$pres1 = Open-PptPres $ppt $createOut
$cap1 = Get-PptCaption $pres1
Close-PptPres $pres1
$psty1ok = ($cap1 -notmatch 'Repair') `
    -and ([string]$t1.tableStyleId -eq $defaultStyle) `
    -and ($t1.tblLook.firstRow -eq $true) `
    -and ($t1.tblLook.bandRow -eq $true) `
    -and ($null -eq $t1.tblLook.firstCol)
Add-Check 'pptx-Psty1' $psty1ok "cap=$cap1 id=$($t1.tableStyleId) firstRow=$($t1.tblLook.firstRow) bandRow=$($t1.tblLook.bandRow)"

$guidOut = Get-OutPath 'pptx-table-style-guid.pptx'
Close-PptPresByPath $ppt $guidOut
if (Test-Path $guidOut) { Remove-Item -Force $guidOut }
$guidJsonPath = Join-Path $global:OoxpReqDir 'pptx-psty-guid.json'
$guidJson = '{"schemaVersion":1,"action":"apply","path":"' + $createOut.Replace('\', '/') + '","output":"' + $guidOut.Replace('\', '/') + '","ops":[{"op":"setTableStyleRef","id":"' + $tid + '","tableStyleId":"' + $specStyle + '"}]}'
[System.IO.File]::WriteAllText($guidJsonPath, $guidJson, $global:OoxpUtf8)
Invoke-Ooxp $guidJsonPath | Out-Null
$tree2 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $guidOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-psty-inspect-guid'
$t2 = @($tree2.slides[0].tables) | Select-Object -First 1
$pres2 = Open-PptPres $ppt $guidOut
$cap2 = Get-PptCaption $pres2
Close-PptPres $pres2
$psty2ok = ($cap2 -notmatch 'Repair') -and ([string]$t2.tableStyleId -eq $specStyle)
Add-Check 'pptx-Psty2' $psty2ok "cap=$cap2 id=$($t2.tableStyleId)"

$lookOut = Get-OutPath 'pptx-table-style-look.pptx'
Close-PptPresByPath $ppt $lookOut
if (Test-Path $lookOut) { Remove-Item -Force $lookOut }
$lookJsonPath = Join-Path $global:OoxpReqDir 'pptx-psty-look.json'
$lookJson = '{"schemaVersion":1,"action":"apply","path":"' + $guidOut.Replace('\', '/') + '","output":"' + $lookOut.Replace('\', '/') + '","ops":[{"op":"setTableStyleRef","id":"' + $tid + '","tblLook":{"firstCol":true}}]}'
[System.IO.File]::WriteAllText($lookJsonPath, $lookJson, $global:OoxpUtf8)
Invoke-Ooxp $lookJsonPath | Out-Null
$tree3 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $lookOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-psty-inspect-look'
$t3 = @($tree3.slides[0].tables) | Select-Object -First 1
$pres3 = Open-PptPres $ppt $lookOut
$cap3 = Get-PptCaption $pres3
Close-PptPres $pres3
$psty3ok = ($cap3 -notmatch 'Repair') `
    -and ($t3.tblLook.firstRow -eq $true) `
    -and ($t3.tblLook.firstCol -eq $true)
Add-Check 'pptx-Psty3' $psty3ok "cap=$cap3 firstRow=$($t3.tblLook.firstRow) firstCol=$($t3.tblLook.firstCol)"

$clearOut = Get-OutPath 'pptx-table-style-clear.pptx'
Close-PptPresByPath $ppt $clearOut
if (Test-Path $clearOut) { Remove-Item -Force $clearOut }
$clearJsonPath = Join-Path $global:OoxpReqDir 'pptx-psty-clear.json'
$clearJson = '{"schemaVersion":1,"action":"apply","path":"' + $lookOut.Replace('\', '/') + '","output":"' + $clearOut.Replace('\', '/') + '","ops":[{"op":"setTableStyleRef","id":"' + $tid + '","tableStyleId":null,"tblLook":null}]}'
[System.IO.File]::WriteAllText($clearJsonPath, $clearJson, $global:OoxpUtf8)
Invoke-Ooxp $clearJsonPath | Out-Null
$tree4 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $clearOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-psty-inspect-clear'
$t4 = @($tree4.slides[0].tables) | Select-Object -First 1
$pres4 = Open-PptPres $ppt $clearOut
$cap4 = Get-PptCaption $pres4
Close-PptPres $pres4
$psty4ok = ($cap4 -notmatch 'Repair') `
    -and ($null -eq $t4.tableStyleId) `
    -and ($null -eq $t4.tblLook.firstRow) `
    -and ($null -eq $t4.tblLook.firstCol) `
    -and ($null -eq $t4.tblLook.lastRow) `
    -and ($null -eq $t4.tblLook.lastCol) `
    -and ($null -eq $t4.tblLook.bandRow) `
    -and ($null -eq $t4.tblLook.bandCol)
Add-Check 'pptx-Psty4' $psty4ok "cap=$cap4 id=$($t4.tableStyleId) firstRow=$($t4.tblLook.firstRow)"

$failId = Get-OutPath 'pptx-table-style-fail-id.pptx'
$failEmpty = Get-OutPath 'pptx-table-style-fail-empty.pptx'
$failGuid = Get-OutPath 'pptx-table-style-fail-guid.pptx'
foreach ($p in @($failId, $failEmpty, $failGuid)) {
    if (Test-Path $p) { Remove-Item -Force $p }
}
$cid = "$tid/r:1/c:1"
$rawId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $failId.Replace('\', '/')
    ops = @(@{ op = 'setTableStyleRef'; id = $cid; tableStyleId = $specStyle })
} 'pptx-psty-fail-id'
$codeId = Get-PptErrorCode $rawId
$rawEmpty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $failEmpty.Replace('\', '/')
    ops = @(@{ op = 'setTableStyleRef'; id = $tid })
} 'pptx-psty-fail-empty'
$codeEmpty = Get-PptErrorCode $rawEmpty
$rawGuid = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $failGuid.Replace('\', '/')
    ops = @(@{ op = 'setTableStyleRef'; id = $tid; tableStyleId = 'not-a-guid' })
} 'pptx-psty-fail-guid'
$codeGuid = Get-PptErrorCode $rawGuid

$xlsx = Get-OutPath 'pptx-table-style.xlsx'
if (Test-Path $xlsx) { Remove-Item -Force $xlsx }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'pptx-psty-xlsx-create' | Out-Null
$failX = Get-OutPath 'pptx-table-style-xlsx-out.xlsx'
if (Test-Path $failX) { Remove-Item -Force $failX }
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'setTableStyleRef'; id = $tid; tableStyleId = $specStyle })
} 'pptx-psty-fail-xlsx'
$codeX = Get-PptErrorCode $rawX

$psty5ok = ($codeId -eq 'InvalidId') -and (-not (Test-Path $failId)) `
    -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failEmpty)) `
    -and ($codeGuid -eq 'InvalidField') -and (-not (Test-Path $failGuid)) `
    -and ($codeX -eq 'InvalidField') -and (-not (Test-Path $failX))
Add-Check 'pptx-Psty5' $psty5ok "id=$codeId empty=$codeEmpty guid=$codeGuid xlsx=$codeX"
