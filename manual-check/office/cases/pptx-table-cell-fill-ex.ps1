# pptx 表单元格非实心填充：docs/checklist-pptx.md 的 Pgf1–Pgf5。
# 编号 Pgf，不要和 Ptb、Tfl 撞。探测脚本不进本文件。不要 Stop-Process PowerPoint。

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

$png = Ensure-OoxpPngA
$pngJson = $png.Replace('\', '/')

$createOut = Get-OutPath 'pptx-table-cell-fill-ex-create.pptx'
if (Test-Path $createOut) { Remove-Item -Force $createOut }
$createJsonPath = Join-Path $global:OoxpReqDir 'pptx-pgf-create.json'
$createPathJson = $createOut.Replace('\', '/')
$createJson = '{"schemaVersion":1,"action":"create","path":"' + $createPathJson + '","presentation":{"slides":[{"shapes":[{"text":"标题"}],"tables":[{"rows":2,"cols":2,"x":914400,"y":1828800,"cx":5486400,"cy":3200400,"cells":[[{"text":"甲","fill":"FF0000"},"乙"],["丙","丁"]]}]}]}}'
[System.IO.File]::WriteAllText($createJsonPath, $createJson, $global:OoxpUtf8)
Invoke-Ooxp $createJsonPath | Out-Null

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $createOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-pgf-inspect-create'
$t1 = @($tree1.slides[0].tables) | Select-Object -First 1
$c1 = $t1.rows[0].cells[0]
$c2 = $t1.rows[0].cells[1]
$c3 = $t1.rows[1].cells[0]
$cid1 = [string]$c1.id
$cid2 = [string]$c2.id
$cid3 = [string]$c3.id
$tid = [string]$t1.id
$ppt = Ensure-MsPpt $createOut
$pres1 = Open-PptPres $ppt $createOut
$cap1 = Get-PptCaption $pres1
Close-PptPres $pres1
$pgf1ok = ($cap1 -notmatch 'Repair') `
    -and ([string]$c1.fill.type -eq 'solid') `
    -and ([string]$c1.fill.color -eq 'FF0000')
Add-Check 'pptx-Pgf1' $pgf1ok "cap=$cap1 type=$($c1.fill.type) color=$($c1.fill.color)"

$fillOut = Get-OutPath 'pptx-table-cell-fill-ex-apply.pptx'
Close-PptPresByPath $ppt $fillOut
if (Test-Path $fillOut) { Remove-Item -Force $fillOut }
$applyJsonPath = Join-Path $global:OoxpReqDir 'pptx-pgf-apply.json'
$fillPathJson = $fillOut.Replace('\', '/')
$srcPathJson = $createOut.Replace('\', '/')
$applyJson = '{"schemaVersion":1,"action":"apply","path":"' + $srcPathJson + '","output":"' + $fillPathJson + '","ops":[{"op":"setTableCellFill","id":"' + $cid1 + '","fill":{"type":"grad","ang":5400000,"stops":[{"pos":0,"color":"FF0000"},{"pos":100000,"color":"0000FF"}]}},{"op":"setTableCellFill","id":"' + $cid2 + '","fill":{"type":"patt","prst":"smCheck","fg":"FF0000","bg":"FFFFFF"}},{"op":"setTableCellFill","id":"' + $cid3 + '","fill":{"type":"blip"},"file":"' + $pngJson + '"}]}'
[System.IO.File]::WriteAllText($applyJsonPath, $applyJson, $global:OoxpUtf8)
Invoke-Ooxp $applyJsonPath | Out-Null

$tree2 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $fillOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-pgf-inspect-apply'
$g = @($tree2.slides[0].tables)[0].rows[0].cells[0].fill
$p = @($tree2.slides[0].tables)[0].rows[0].cells[1].fill
$b = @($tree2.slides[0].tables)[0].rows[1].cells[0].fill
$pres2 = Open-PptPres $ppt $fillOut
$cap2 = Get-PptCaption $pres2
Close-PptPres $pres2

$gAng = $null; $gPos0 = $null; $gCol0 = $null; $gPos1 = $null; $gCol1 = $null
try {
    $gAng = [int64]$g.ang
    $gPos0 = [int64]$g.stops[0].pos
    $gCol0 = [string]$g.stops[0].color
    $gPos1 = [int64]$g.stops[1].pos
    $gCol1 = [string]$g.stops[1].color
} catch {}
$pgf2ok = ($cap2 -notmatch 'Repair') `
    -and ([string]$g.type -eq 'grad') `
    -and ($gAng -eq 5400000) `
    -and ($gPos0 -eq 0) -and ($gCol0 -eq 'FF0000') `
    -and ($gPos1 -eq 100000) -and ($gCol1 -eq '0000FF')
Add-Check 'pptx-Pgf2' $pgf2ok "cap=$cap2 type=$($g.type) ang=$gAng stops=$gPos0/$gCol0,$gPos1/$gCol1"

$pgf3ok = ($cap2 -notmatch 'Repair') `
    -and ([string]$p.type -eq 'patt') `
    -and ([string]$p.prst -eq 'smCheck') `
    -and ([string]$p.fg -eq 'FF0000') `
    -and ([string]$p.bg -eq 'FFFFFF')
Add-Check 'pptx-Pgf3' $pgf3ok "cap=$cap2 type=$($p.type) prst=$($p.prst) fg=$($p.fg) bg=$($p.bg)"

$expOut = Get-OutPath 'pptx-table-cell-fill-ex-export.png'
if (Test-Path $expOut) { Remove-Item -Force $expOut }
$rawExp = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'exportMedia'
    path = $fillOut.Replace('\', '/')
    id = $cid3
    output = $expOut.Replace('\', '/')
} 'pptx-pgf-export'
$codeExp = Get-PptErrorCode $rawExp
$pgf4ok = ($cap2 -notmatch 'Repair') `
    -and ([string]$b.type -eq 'blip') `
    -and ([string]$b.rId -like 'rId*') `
    -and ([string]$b.contentType -eq 'image/png') `
    -and ($codeExp -eq 'InvalidId') `
    -and (-not (Test-Path $expOut))
Add-Check 'pptx-Pgf4' $pgf4ok "cap=$cap2 type=$($b.type) rId=$($b.rId) ct=$($b.contentType) exp=$codeExp"

$failType = Get-OutPath 'pptx-table-cell-fill-ex-fail-type.pptx'
if (Test-Path $failType) { Remove-Item -Force $failType }
$rawType = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $failType.Replace('\', '/')
    ops = @(@{ op = 'setTableCellFill'; id = $cid1; fill = @{ color = 'FF0000' } })
} 'pptx-pgf-fail-type'
$codeType = Get-PptErrorCode $rawType

$failBoth = Get-OutPath 'pptx-table-cell-fill-ex-fail-both.pptx'
if (Test-Path $failBoth) { Remove-Item -Force $failBoth }
$rawBoth = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $failBoth.Replace('\', '/')
    ops = @(@{ op = 'setTableCellFill'; id = $cid1; color = 'FF0000'; fill = @{ type = 'solid'; color = '00FF00' } })
} 'pptx-pgf-fail-both'
$codeBoth = Get-PptErrorCode $rawBoth

$failId = Get-OutPath 'pptx-table-cell-fill-ex-fail-id.pptx'
if (Test-Path $failId) { Remove-Item -Force $failId }
$rawId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $failId.Replace('\', '/')
    ops = @(@{ op = 'setTableCellFill'; id = $tid; fill = @{ type = 'solid'; color = 'FF0000' } })
} 'pptx-pgf-fail-id'
$codeId = Get-PptErrorCode $rawId

$xlsx = Get-OutPath 'pptx-table-cell-fill-ex-xlsx.xlsx'
if (Test-Path $xlsx) { Remove-Item -Force $xlsx }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'pptx-pgf-xlsx-create' | Out-Null
$failX = Get-OutPath 'pptx-table-cell-fill-ex-xlsx-out.xlsx'
if (Test-Path $failX) { Remove-Item -Force $failX }
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'setTableCellFill'; id = $cid1; fill = @{ type = 'solid'; color = 'FF0000' } })
} 'pptx-pgf-fail-xlsx'
$codeX = Get-PptErrorCode $rawX

$pgf5ok = ($codeType -eq 'InvalidField') -and (-not (Test-Path $failType)) `
    -and ($codeBoth -eq 'InvalidField') -and (-not (Test-Path $failBoth)) `
    -and (($codeId -eq 'InvalidId') -or ($codeId -eq 'InvalidField')) -and (-not (Test-Path $failId)) `
    -and ($codeX -eq 'InvalidField') -and (-not (Test-Path $failX))
Add-Check 'pptx-Pgf5' $pgf5ok "type=$codeType both=$codeBoth id=$codeId xlsx=$codeX"
