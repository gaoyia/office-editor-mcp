# pptx 表单元格边距：docs/checklist-pptx.md 的 Ptm1–Ptm5。
# 编号 Ptm，不要和 Pt、Pg、Ptb 撞。探测脚本不进本文件。不要 Stop-Process PowerPoint。
# 边距是 EMU；COM TextFrame.Margin* 是磅（12700 EMU = 1 磅）。

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

function Test-PptShapeIsTable($sh) {
    try { if ([bool]$sh.HasTable) { return $true } } catch {}
    try { if ([int]$sh.Type -eq 19) { return $true } } catch {}
    return $false
}

function Find-PptTable($slide) {
    $shapes = $slide.Shapes
    try {
        for ($i = 1; $i -le $shapes.Count; $i++) {
            $cand = $shapes.Item($i)
            if (Test-PptShapeIsTable $cand) { return $cand }
            Release-Com $cand
        }
    }
    finally { Release-Com $shapes }
    return $null
}

function Read-PptCellMargins($slide, [int]$row, [int]$col) {
    $top = 0
    $left = 0
    $sh = $null
    $tbl = $null
    $cell = $null
    $shape = $null
    $tf = $null
    try {
        $sh = Find-PptTable $slide
        if ($null -eq $sh) { return @{ Top = 0; Left = 0 } }
        $tbl = $sh.Table
        $cell = $tbl.Cell($row, $col)
        $shape = $cell.Shape
        $tf = $shape.TextFrame
        $top = [double]$tf.MarginTop
        $left = [double]$tf.MarginLeft
    } catch {
        $top = 0
        $left = 0
    } finally {
        if ($tf) { Release-Com $tf }
        if ($shape) { Release-Com $shape }
        if ($cell) { Release-Com $cell }
        if ($tbl) { Release-Com $tbl }
        if ($sh) { Release-Com $sh }
    }
    return @{ Top = $top; Left = $left }
}

$createOut = Get-OutPath 'pptx-table-cell-mar-create.pptx'
if (Test-Path $createOut) { Remove-Item -Force $createOut }
$createJsonPath = Join-Path $global:OoxpReqDir 'pptx-ptm-create.json'
$createPathJson = $createOut.Replace('\', '/')
$createJson = '{"schemaVersion":1,"action":"create","path":"' + $createPathJson + '","presentation":{"slides":[{"shapes":[{"text":"标题"}],"tables":[{"rows":2,"cols":2,"x":914400,"y":1828800,"cx":5486400,"cy":3200400,"cells":[[{"text":"甲","cellMar":{"top":12700,"left":25400,"bottom":0}},"乙"],["丙","丁"]]}]}]}}'
[System.IO.File]::WriteAllText($createJsonPath, $createJson, $global:OoxpUtf8)
Invoke-Ooxp $createJsonPath | Out-Null

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $createOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-ptm-inspect-create'
$t1 = @($tree1.slides[0].tables) | Select-Object -First 1
$c1 = $t1.rows[0].cells[0]
$cid = [string]$c1.id
$tid = [string]$t1.id
$ppt = Ensure-MsPpt $createOut
$pres1 = Open-PptPres $ppt $createOut
$cap1 = Get-PptCaption $pres1
$s1 = Get-PptSlide $pres1 1
$m1 = Read-PptCellMargins $s1 1 1
Release-Com $s1
Close-PptPres $pres1
$ptm1ok = ($cap1 -notmatch 'Repair') `
    -and ([int64]$c1.cellMar.top -eq 12700) `
    -and ([int64]$c1.cellMar.left -eq 25400) `
    -and ([int64]$c1.cellMar.bottom -eq 0) `
    -and ($null -eq $c1.cellMar.right)
Add-Check 'pptx-Ptm1' $ptm1ok "cap=$cap1 top=$($c1.cellMar.top) left=$($c1.cellMar.left) comT=$($m1.Top) comL=$($m1.Left)"

$applyOut = Get-OutPath 'pptx-table-cell-mar-apply.pptx'
Close-PptPresByPath $ppt $applyOut
if (Test-Path $applyOut) { Remove-Item -Force $applyOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $applyOut.Replace('\', '/')
    ops = @(@{ op = 'setTableCellInset'; id = $cid; right = 38100 })
} 'pptx-ptm-apply' | Out-Null
$tree2 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $applyOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-ptm-inspect-apply'
$c2 = @($tree2.slides[0].tables)[0].rows[0].cells[0]
$pres2 = Open-PptPres $ppt $applyOut
$cap2 = Get-PptCaption $pres2
Close-PptPres $pres2
$ptm2ok = ($cap2 -notmatch 'Repair') `
    -and ([int64]$c2.cellMar.top -eq 12700) `
    -and ([int64]$c2.cellMar.right -eq 38100)
Add-Check 'pptx-Ptm2' $ptm2ok "cap=$cap2 top=$($c2.cellMar.top) right=$($c2.cellMar.right)"

$pres3 = Open-PptPres $ppt $applyOut
$cap3 = Get-PptCaption $pres3
$s3 = Get-PptSlide $pres3 1
$m3 = Read-PptCellMargins $s3 1 1
Release-Com $s3
Close-PptPres $pres3
$ptm3ok = ($cap3 -notmatch 'Repair') `
    -and ([math]::Abs($m3.Top - 1) -lt 0.6) `
    -and ([math]::Abs($m3.Left - 2) -lt 0.6)
Add-Check 'pptx-Ptm3' $ptm3ok "cap=$cap3 comT=$($m3.Top) comL=$($m3.Left)"

$clearOut = Get-OutPath 'pptx-table-cell-mar-clear.pptx'
Close-PptPresByPath $ppt $clearOut
if (Test-Path $clearOut) { Remove-Item -Force $clearOut }
$clearJsonPath = Join-Path $global:OoxpReqDir 'pptx-ptm-clear.json'
$clearJson = '{"schemaVersion":1,"action":"apply","path":"' + $applyOut.Replace('\', '/') + '","output":"' + $clearOut.Replace('\', '/') + '","ops":[{"op":"setTableCellInset","id":"' + $cid + '","top":null,"left":null,"bottom":null,"right":null}]}'
[System.IO.File]::WriteAllText($clearJsonPath, $clearJson, $global:OoxpUtf8)
Invoke-Ooxp $clearJsonPath | Out-Null
$tree4 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $clearOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-ptm-inspect-clear'
$c4 = @($tree4.slides[0].tables)[0].rows[0].cells[0]
$pres4 = Open-PptPres $ppt $clearOut
$cap4 = Get-PptCaption $pres4
Close-PptPres $pres4
$ptm4ok = ($cap4 -notmatch 'Repair') -and ($null -eq $c4.cellMar)
Add-Check 'pptx-Ptm4' $ptm4ok "cap=$cap4 cellMar=$($c4.cellMar)"

$failId = Get-OutPath 'pptx-table-cell-mar-fail-id.pptx'
if (Test-Path $failId) { Remove-Item -Force $failId }
$rawId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $failId.Replace('\', '/')
    ops = @(@{ op = 'setTableCellInset'; id = $tid; top = 1 })
} 'pptx-ptm-fail-id'
$codeId = Get-PptErrorCode $rawId

$failEmpty = Get-OutPath 'pptx-table-cell-mar-fail-empty.pptx'
if (Test-Path $failEmpty) { Remove-Item -Force $failEmpty }
$rawEmpty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $failEmpty.Replace('\', '/')
    ops = @(@{ op = 'setTableCellInset'; id = $cid })
} 'pptx-ptm-fail-empty'
$codeEmpty = Get-PptErrorCode $rawEmpty

$failNeg = Get-OutPath 'pptx-table-cell-mar-fail-neg.pptx'
if (Test-Path $failNeg) { Remove-Item -Force $failNeg }
$rawNeg = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $failNeg.Replace('\', '/')
    ops = @(@{ op = 'setTableCellInset'; id = $cid; top = -1 })
} 'pptx-ptm-fail-neg'
$codeNeg = Get-PptErrorCode $rawNeg

$xlsx = Get-OutPath 'pptx-table-cell-mar-xlsx.xlsx'
if (Test-Path $xlsx) { Remove-Item -Force $xlsx }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'pptx-ptm-xlsx-create' | Out-Null
$failX = Get-OutPath 'pptx-table-cell-mar-xlsx-out.xlsx'
if (Test-Path $failX) { Remove-Item -Force $failX }
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'setTableCellInset'; id = $cid; top = 1 })
} 'pptx-ptm-fail-xlsx'
$codeX = Get-PptErrorCode $rawX

$ptm5ok = ($codeId -eq 'InvalidId') -and (-not (Test-Path $failId)) `
    -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failEmpty)) `
    -and ($codeNeg -eq 'InvalidField') -and (-not (Test-Path $failNeg)) `
    -and ($codeX -eq 'InvalidField') -and (-not (Test-Path $failX))
Add-Check 'pptx-Ptm5' $ptm5ok "id=$codeId empty=$codeEmpty neg=$codeNeg xlsx=$codeX"
