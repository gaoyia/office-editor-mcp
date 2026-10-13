# pptx 表单元格填充与边框：docs/checklist-pptx.md 的 Ptb1–Ptb5。
# 编号 Ptb，不要和 Pt、Pg、Ptf、Tf 撞。探测脚本不进本文件。不要 Stop-Process PowerPoint。

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

function Read-PptCellFillRgb($slide, [int]$row, [int]$col) {
    $rgb = $null
    $sh = $null
    $tbl = $null
    $cell = $null
    $shape = $null
    $fill = $null
    $fore = $null
    try {
        $sh = Find-PptTable $slide
        if ($null -eq $sh) { return $null }
        $tbl = $sh.Table
        $cell = $tbl.Cell($row, $col)
        $shape = $cell.Shape
        $fill = $shape.Fill
        $fore = $fill.ForeColor
        $rgb = [int64]$fore.RGB
    } catch {
        $rgb = $null
    } finally {
        if ($fore) { Release-Com $fore }
        if ($fill) { Release-Com $fill }
        if ($shape) { Release-Com $shape }
        if ($cell) { Release-Com $cell }
        if ($tbl) { Release-Com $tbl }
        if ($sh) { Release-Com $sh }
    }
    return $rgb
}

$createOut = Get-OutPath 'pptx-table-cell-fill-create.pptx'
if (Test-Path $createOut) { Remove-Item -Force $createOut }
$createJsonPath = Join-Path $global:OoxpReqDir 'pptx-ptb-create.json'
$createPathJson = $createOut.Replace('\', '/')
$createJson = '{"schemaVersion":1,"action":"create","path":"' + $createPathJson + '","presentation":{"slides":[{"shapes":[{"text":"标题"}],"tables":[{"rows":2,"cols":2,"x":914400,"y":1828800,"cx":5486400,"cy":3200400,"cells":[[{"text":"甲","fill":"FF0000","borders":{"left":{"color":"000000","width":12700,"dash":"dash"}}},"乙"],["丙","丁"]]}]}]}}'
[System.IO.File]::WriteAllText($createJsonPath, $createJson, $global:OoxpUtf8)
Invoke-Ooxp $createJsonPath | Out-Null

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $createOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-ptb-inspect-create'
$t1 = @($tree1.slides[0].tables) | Select-Object -First 1
$c1 = $t1.rows[0].cells[0]
$cid = [string]$c1.id
$tid = [string]$t1.id
$ppt = Ensure-MsPpt $createOut
$pres1 = Open-PptPres $ppt $createOut
$cap1 = Get-PptCaption $pres1
$s1 = Get-PptSlide $pres1 1
$rgb1 = Read-PptCellFillRgb $s1 1 1
Release-Com $s1
Close-PptPres $pres1
$ptb1ok = ($cap1 -notmatch 'Repair') `
    -and ([string]$c1.fill.type -eq 'solid') `
    -and ([string]$c1.fill.color -eq 'FF0000') `
    -and ([string]$c1.borders.left.color -eq '000000') `
    -and ([int64]$c1.borders.left.width -eq 12700) `
    -and ([string]$c1.borders.left.dash -eq 'dash') `
    -and ($null -eq $c1.borders.right)
Add-Check 'pptx-Ptb1' $ptb1ok "cap=$cap1 fill=$($c1.fill.type)/$($c1.fill.color) left=$($c1.borders.left.color)/$($c1.borders.left.width)/$($c1.borders.left.dash) rgb=$rgb1"

$fillOut = Get-OutPath 'pptx-table-cell-fill-apply.pptx'
Close-PptPresByPath $ppt $fillOut
if (Test-Path $fillOut) { Remove-Item -Force $fillOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $fillOut.Replace('\', '/')
    ops = @(
        @{ op = 'setTableCellFill'; id = $cid; color = '0000FF' }
    )
} 'pptx-ptb-fill' | Out-Null
$tree2 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $fillOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-ptb-inspect-fill'
$c2 = @($tree2.slides[0].tables)[0].rows[0].cells[0]
$pres2 = Open-PptPres $ppt $fillOut
$cap2 = Get-PptCaption $pres2
$s2 = Get-PptSlide $pres2 1
$rgb2 = Read-PptCellFillRgb $s2 1 1
Release-Com $s2
Close-PptPres $pres2
$ptb2ok = ($cap2 -notmatch 'Repair') `
    -and ([string]$c2.fill.type -eq 'solid') `
    -and ([string]$c2.fill.color -eq '0000FF') `
    -and ($null -ne $rgb2) -and ([int64]$rgb2 -eq 16711680)
Add-Check 'pptx-Ptb2' $ptb2ok "cap=$cap2 fill=$($c2.fill.type)/$($c2.fill.color) rgb=$rgb2"

$borderOut = Get-OutPath 'pptx-table-cell-fill-borders.pptx'
Close-PptPresByPath $ppt $borderOut
if (Test-Path $borderOut) { Remove-Item -Force $borderOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $fillOut.Replace('\', '/')
    output = $borderOut.Replace('\', '/')
    ops = @(
        @{
            op = 'setTableCellBorders'
            id = $cid
            top = @{ color = '00FF00'; width = 25400 }
            right = @{ color = '010101'; width = 12700; dash = $null }
        }
    )
} 'pptx-ptb-borders' | Out-Null
$tree3 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $borderOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-ptb-inspect-borders'
$c3 = @($tree3.slides[0].tables)[0].rows[0].cells[0]
$pres3 = Open-PptPres $ppt $borderOut
$cap3 = Get-PptCaption $pres3
Close-PptPres $pres3
$ptb3ok = ($cap3 -notmatch 'Repair') `
    -and ([string]$c3.borders.left.color -eq '000000') `
    -and ([string]$c3.borders.left.dash -eq 'dash') `
    -and ([string]$c3.borders.top.color -eq '00FF00') `
    -and ([int64]$c3.borders.top.width -eq 25400) `
    -and ($null -eq $c3.borders.top.dash) `
    -and ([string]$c3.borders.right.color -eq '010101') `
    -and ($null -eq $c3.borders.right.dash)
Add-Check 'pptx-Ptb3' $ptb3ok "cap=$cap3 left=$($c3.borders.left.dash) top=$($c3.borders.top.color) rightDash=$($c3.borders.right.dash)"

$clearOut = Get-OutPath 'pptx-table-cell-fill-clear.pptx'
Close-PptPresByPath $ppt $clearOut
if (Test-Path $clearOut) { Remove-Item -Force $clearOut }
$clearJsonPath = Join-Path $global:OoxpReqDir 'pptx-ptb-clear.json'
$clearJson = '{"schemaVersion":1,"action":"apply","path":"' + $borderOut.Replace('\', '/') + '","output":"' + $clearOut.Replace('\', '/') + '","ops":[{"op":"setTableCellFill","id":"' + $cid + '","color":null},{"op":"setTableCellBorders","id":"' + $cid + '","left":null}]}'
[System.IO.File]::WriteAllText($clearJsonPath, $clearJson, $global:OoxpUtf8)
Invoke-Ooxp $clearJsonPath | Out-Null
$tree4 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $clearOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-ptb-inspect-clear'
$c4 = @($tree4.slides[0].tables)[0].rows[0].cells[0]
$pres4 = Open-PptPres $ppt $clearOut
$cap4 = Get-PptCaption $pres4
Close-PptPres $pres4
$ptb4ok = ($cap4 -notmatch 'Repair') `
    -and ($null -eq $c4.fill) `
    -and ($null -eq $c4.borders.left) `
    -and ([string]$c4.borders.top.color -eq '00FF00')
Add-Check 'pptx-Ptb4' $ptb4ok "cap=$cap4 fill=$($c4.fill) left=$($c4.borders.left) top=$($c4.borders.top.color)"

$failId = Get-OutPath 'pptx-table-cell-fill-fail-id.pptx'
if (Test-Path $failId) { Remove-Item -Force $failId }
$rawId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $failId.Replace('\', '/')
    ops = @(@{ op = 'setTableCellFill'; id = $tid; color = 'FF0000' })
} 'pptx-ptb-fail-id'
$codeId = Get-PptErrorCode $rawId

$failColor = Get-OutPath 'pptx-table-cell-fill-fail-color.pptx'
if (Test-Path $failColor) { Remove-Item -Force $failColor }
$rawColor = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $failColor.Replace('\', '/')
    ops = @(@{ op = 'setTableCellFill'; id = $cid })
} 'pptx-ptb-fail-color'
$codeColor = Get-PptErrorCode $rawColor

$failCx = Get-OutPath 'pptx-table-cell-fill-fail-cx.pptx'
if (Test-Path $failCx) { Remove-Item -Force $failCx }
$rawCx = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $failCx.Replace('\', '/')
    ops = @(@{ op = 'setTableCellFill'; id = $cid; color = 'FF0000'; cx = 1 })
} 'pptx-ptb-fail-cx'
$codeCx = Get-PptErrorCode $rawCx

$xlsx = Get-OutPath 'pptx-table-cell-fill-xlsx.xlsx'
if (Test-Path $xlsx) { Remove-Item -Force $xlsx }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'pptx-ptb-xlsx-create' | Out-Null
$failX = Get-OutPath 'pptx-table-cell-fill-xlsx-out.xlsx'
if (Test-Path $failX) { Remove-Item -Force $failX }
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'setTableCellFill'; id = $cid; color = 'FF0000' })
} 'pptx-ptb-fail-xlsx'
$codeX = Get-PptErrorCode $rawX

$failSp = Get-OutPath 'pptx-table-cell-fill-fail-shape.pptx'
if (Test-Path $failSp) { Remove-Item -Force $failSp }
$rawSp = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $failSp.Replace('\', '/')
    ops = @(@{ op = 'setShapeFill'; id = $cid; color = 'FF0000' })
} 'pptx-ptb-fail-shape'
$codeSp = Get-PptErrorCode $rawSp

$ptb5ok = ($codeId -eq 'InvalidId') -and (-not (Test-Path $failId)) `
    -and ($codeColor -eq 'InvalidField') -and (-not (Test-Path $failColor)) `
    -and ($codeCx -eq 'InvalidField') -and (-not (Test-Path $failCx)) `
    -and ($codeX -eq 'InvalidField') -and (-not (Test-Path $failX)) `
    -and ($codeSp -eq 'InvalidId') -and (-not (Test-Path $failSp))
Add-Check 'pptx-Ptb5' $ptb5ok "id=$codeId color=$codeColor cx=$codeCx xlsx=$codeX shape=$codeSp"
