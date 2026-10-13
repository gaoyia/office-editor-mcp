# pptx 表单元格 tcPr 剩余：docs/checklist-pptx.md 的 P3c1–P3c5。
# 编号 P3c，不要和 Ptb、Ptm、Pgf、P3g 撞。探测脚本不进本文件。不要 Stop-Process PowerPoint。
# COM 不一定暴露 vert / cell3D，本脚本用 inspect 与 Repair。

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

function Header-Count($v) {
    if ($null -eq $v) { return 0 }
    return @($v).Count
}

function Get-PptErrorCode($raw) {
    try { return ($raw.Stderr | ConvertFrom-Json).error.code } catch { return $null }
}

$createOut = Get-OutPath 'pptx-table-cell-pr-create.pptx'
if (Test-Path $createOut) { Remove-Item -Force $createOut }
$createJsonPath = Join-Path $global:OoxpReqDir 'pptx-p3c-create.json'
$createPathJson = $createOut.Replace('\', '/')
$createJson = '{"schemaVersion":1,"action":"create","path":"' + $createPathJson + '","presentation":{"slides":[{"shapes":[{"text":"标题"}],"tables":[{"rows":2,"cols":2,"x":914400,"y":1828800,"cx":5486400,"cy":3200400,"cells":[[{"text":"甲","vert":"eaVert"},"乙"],["丙","丁"]]}]}]}}'
[System.IO.File]::WriteAllText($createJsonPath, $createJson, $global:OoxpUtf8)
Invoke-Ooxp $createJsonPath | Out-Null

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $createOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-p3c-inspect-create'
$t1 = @($tree1.slides[0].tables) | Select-Object -First 1
$c1 = $t1.rows[0].cells[0]
$cid = [string]$c1.id
$tid = [string]$t1.id
$ppt = Ensure-MsPpt $createOut
$pres1 = Open-PptPres $ppt $createOut
$cap1 = Get-PptCaption $pres1
Close-PptPres $pres1
$hdr1 = Header-Count $c1.headers
$p3c1ok = ($cap1 -notmatch 'Repair') `
    -and ([string]$c1.vert -eq 'eaVert') `
    -and ($null -eq $c1.anchorCtr) `
    -and ($null -eq $c1.horzOverflow) `
    -and ($null -eq $c1.cell3D) `
    -and ($hdr1 -eq 0)
Add-Check 'pptx-P3c1' $p3c1ok "cap=$cap1 vert=$($c1.vert) hdr=$hdr1"

$applyOut = Get-OutPath 'pptx-table-cell-pr-attrs.pptx'
Close-PptPresByPath $ppt $applyOut
if (Test-Path $applyOut) { Remove-Item -Force $applyOut }
$applyJsonPath = Join-Path $global:OoxpReqDir 'pptx-p3c-attrs.json'
$applyJson = '{"schemaVersion":1,"action":"apply","path":"' + $createOut.Replace('\', '/') + '","output":"' + $applyOut.Replace('\', '/') + '","ops":[{"op":"setTableCellPr","id":"' + $cid + '","vert":"vert","anchorCtr":true,"horzOverflow":"overflow"}]}'
[System.IO.File]::WriteAllText($applyJsonPath, $applyJson, $global:OoxpUtf8)
Invoke-Ooxp $applyJsonPath | Out-Null
$tree2 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $applyOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-p3c-inspect-attrs'
$c2 = @($tree2.slides[0].tables)[0].rows[0].cells[0]
$pres2 = Open-PptPres $ppt $applyOut
$cap2 = Get-PptCaption $pres2
Close-PptPres $pres2
$p3c2ok = ($cap2 -notmatch 'Repair') `
    -and ([string]$c2.vert -eq 'vert') `
    -and ([bool]$c2.anchorCtr -eq $true) `
    -and ([string]$c2.horzOverflow -eq 'overflow')
Add-Check 'pptx-P3c2' $p3c2ok "cap=$cap2 vert=$($c2.vert) ctr=$($c2.anchorCtr) ov=$($c2.horzOverflow)"

$threeOut = Get-OutPath 'pptx-table-cell-pr-3d.pptx'
Close-PptPresByPath $ppt $threeOut
if (Test-Path $threeOut) { Remove-Item -Force $threeOut }
$threeJsonPath = Join-Path $global:OoxpReqDir 'pptx-p3c-3d.json'
$threeJson = '{"schemaVersion":1,"action":"apply","path":"' + $applyOut.Replace('\', '/') + '","output":"' + $threeOut.Replace('\', '/') + '","ops":[{"op":"setTableCellPr","id":"' + $cid + '","cell3D":{"prstMaterial":"plastic","bevel":{"prst":"circle","w":76200,"h":76200}}}]}'
[System.IO.File]::WriteAllText($threeJsonPath, $threeJson, $global:OoxpUtf8)
Invoke-Ooxp $threeJsonPath | Out-Null
$tree3 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $threeOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-p3c-inspect-3d'
$c3 = @($tree3.slides[0].tables)[0].rows[0].cells[0]
$pres3 = Open-PptPres $ppt $threeOut
$cap3 = Get-PptCaption $pres3
Close-PptPres $pres3
$p3c3ok = ($cap3 -notmatch 'Repair') `
    -and ([string]$c3.cell3D.prstMaterial -eq 'plastic') `
    -and ([string]$c3.cell3D.bevel.prst -eq 'circle')
Add-Check 'pptx-P3c3' $p3c3ok "cap=$cap3 mat=$($c3.cell3D.prstMaterial) prst=$($c3.cell3D.bevel.prst)"

$headOut = Get-OutPath 'pptx-table-cell-pr-headers.pptx'
Close-PptPresByPath $ppt $headOut
if (Test-Path $headOut) { Remove-Item -Force $headOut }
$headJsonPath = Join-Path $global:OoxpReqDir 'pptx-p3c-headers.json'
$headJson = '{"schemaVersion":1,"action":"apply","path":"' + $threeOut.Replace('\', '/') + '","output":"' + $headOut.Replace('\', '/') + '","ops":[{"op":"setTableCellPr","id":"' + $cid + '","headers":["列甲"]}]}'
[System.IO.File]::WriteAllText($headJsonPath, $headJson, $global:OoxpUtf8)
Invoke-Ooxp $headJsonPath | Out-Null
$tree4 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $headOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-p3c-inspect-headers'
$c4 = @($tree4.slides[0].tables)[0].rows[0].cells[0]
$pres4 = Open-PptPres $ppt $headOut
$cap4 = Get-PptCaption $pres4
Close-PptPres $pres4
$hdr4 = @($c4.headers)
$hdr4n = Header-Count $c4.headers

$clearOut = Get-OutPath 'pptx-table-cell-pr-clear.pptx'
Close-PptPresByPath $ppt $clearOut
if (Test-Path $clearOut) { Remove-Item -Force $clearOut }
$clearJsonPath = Join-Path $global:OoxpReqDir 'pptx-p3c-clear.json'
$clearJson = '{"schemaVersion":1,"action":"apply","path":"' + $headOut.Replace('\', '/') + '","output":"' + $clearOut.Replace('\', '/') + '","ops":[{"op":"setTableCellPr","id":"' + $cid + '","vert":null,"anchorCtr":null,"horzOverflow":null,"cell3D":null,"headers":null}]}'
[System.IO.File]::WriteAllText($clearJsonPath, $clearJson, $global:OoxpUtf8)
Invoke-Ooxp $clearJsonPath | Out-Null
$tree5 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $clearOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-p3c-inspect-clear'
$c5 = @($tree5.slides[0].tables)[0].rows[0].cells[0]
$pres5 = Open-PptPres $ppt $clearOut
$cap5 = Get-PptCaption $pres5
Close-PptPres $pres5
$hdr5n = Header-Count $c5.headers
$p3c4ok = ($cap4 -notmatch 'Repair') -and ($cap5 -notmatch 'Repair') `
    -and ($hdr4n -eq 1) -and ([string]$hdr4[0] -eq '列甲') `
    -and ($null -eq $c5.vert) -and ($null -eq $c5.anchorCtr) `
    -and ($null -eq $c5.horzOverflow) -and ($null -eq $c5.cell3D) `
    -and ($hdr5n -eq 0)
Add-Check 'pptx-P3c4' $p3c4ok "cap4=$cap4 cap5=$cap5 hdr4=$hdr4n vert=$($c5.vert)"

$failId = Get-OutPath 'pptx-table-cell-pr-fail-id.pptx'
$failEmpty = Get-OutPath 'pptx-table-cell-pr-fail-empty.pptx'
$failVert = Get-OutPath 'pptx-table-cell-pr-fail-vert.pptx'
foreach ($p in @($failId, $failEmpty, $failVert)) {
    if (Test-Path $p) { Remove-Item -Force $p }
}
$rawId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $failId.Replace('\', '/')
    ops = @(@{ op = 'setTableCellPr'; id = $tid; vert = 'horz' })
} 'pptx-p3c-fail-id'
$codeId = Get-PptErrorCode $rawId
$rawEmpty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $failEmpty.Replace('\', '/')
    ops = @(@{ op = 'setTableCellPr'; id = $cid })
} 'pptx-p3c-fail-empty'
$codeEmpty = Get-PptErrorCode $rawEmpty
$rawVert = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $failVert.Replace('\', '/')
    ops = @(@{ op = 'setTableCellPr'; id = $cid; vert = 'sideways' })
} 'pptx-p3c-fail-vert'
$codeVert = Get-PptErrorCode $rawVert

$xlsx = Get-OutPath 'pptx-table-cell-pr.xlsx'
if (Test-Path $xlsx) { Remove-Item -Force $xlsx }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'pptx-p3c-xlsx-create' | Out-Null
$failX = Get-OutPath 'pptx-table-cell-pr-xlsx-out.xlsx'
if (Test-Path $failX) { Remove-Item -Force $failX }
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'setTableCellPr'; id = $cid; vert = 'horz' })
} 'pptx-p3c-fail-xlsx'
$codeX = Get-PptErrorCode $rawX

$p3c5ok = ($codeId -eq 'InvalidId') -and (-not (Test-Path $failId)) `
    -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failEmpty)) `
    -and ($codeVert -eq 'InvalidField') -and (-not (Test-Path $failVert)) `
    -and ($codeX -eq 'InvalidField') -and (-not (Test-Path $failX))
Add-Check 'pptx-P3c5' $p3c5ok "id=$codeId empty=$codeEmpty vert=$codeVert xlsx=$codeX"
