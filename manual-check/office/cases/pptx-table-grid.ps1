# pptx 表内列宽与行高：docs/checklist-pptx.md 的 Pg1–Pg5。
# 列宽 / 行高是 EMU；COM Columns.Width / Rows.Height 是磅（12700 EMU = 1 磅）。

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

function Read-PptTableGrid($slide) {
    $w1 = 0
    $w2 = 0
    $h1 = 0
    $sh = $null
    $tbl = $null
    $cols = $null
    $rows = $null
    $c1 = $null
    $c2 = $null
    $r1 = $null
    try {
        $sh = Find-PptTable $slide
        if ($null -eq $sh) { return @{ W1 = 0; W2 = 0; H1 = 0 } }
        $tbl = $sh.Table
        $cols = $tbl.Columns
        $c1 = $cols.Item(1)
        $c2 = $cols.Item(2)
        $w1 = [double]$c1.Width
        $w2 = [double]$c2.Width
        $rows = $tbl.Rows
        $r1 = $rows.Item(1)
        $h1 = [double]$r1.Height
    }
    finally {
        if ($r1) { Release-Com $r1 }
        if ($rows) { Release-Com $rows }
        if ($c2) { Release-Com $c2 }
        if ($c1) { Release-Com $c1 }
        if ($cols) { Release-Com $cols }
        if ($tbl) { Release-Com $tbl }
        if ($sh) { Release-Com $sh }
    }
    return @{ W1 = $w1; W2 = $w2; H1 = $h1 }
}

$createOut = Get-OutPath 'pptx-table-grid-create.pptx'
if (Test-Path $createOut) { Remove-Item -Force $createOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $createOut.Replace('\', '/')
    presentation = @{
        slides = @(@{
            shapes = @(@{ text = '标题' })
            tables = @(@{
                rows = 2; cols = 2
                x = 914400; y = 1828800
                cx = 5486400; cy = 3200400
                colWidths = @(1270000, 2540000)
                rowHeights = @(635000, 1270000)
                cells = @(@('甲', '乙'), @('丙', '丁'))
            })
        })
    }
} 'pptx-pg-create' | Out-Null

$ppt = Ensure-MsPpt $createOut
Close-PptPresByPath $ppt $createOut

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $createOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-pg-inspect-create'
$t1 = @($tree1.slides[0].tables) | Select-Object -First 1
$pres1 = Open-PptPres $ppt $createOut
$cap1 = Get-PptCaption $pres1
$s1 = Get-PptSlide $pres1 1
$g1 = Read-PptTableGrid $s1
Release-Com $s1
Close-PptPres $pres1
$pg1ok = ($cap1 -notmatch 'Repair') `
    -and ([int64]$t1.colWidths[0] -eq 1270000) -and ([int64]$t1.colWidths[1] -eq 2540000) `
    -and ([int64]$t1.rows[0].height -eq 635000) `
    -and ([math]::Abs($g1.W1 - 100) -lt 0.6) -and ([math]::Abs($g1.W2 - 200) -lt 0.6) `
    -and ([math]::Abs($g1.H1 - 50) -lt 0.6)
Add-Check 'pptx-Pg1' $pg1ok "cap=$cap1 w=$($t1.colWidths[0]),$($t1.colWidths[1]) h=$($t1.rows[0].height) comW=$($g1.W1),$($g1.W2) comH=$($g1.H1)"

$colOut = Get-OutPath 'pptx-table-grid-cols.pptx'
Close-PptPresByPath $ppt $colOut
if (Test-Path $colOut) { Remove-Item -Force $colOut }
$tid = [string]$t1.id
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $colOut.Replace('\', '/')
    ops = @(@{
        op = 'setTableColWidths'; id = $tid
        cols = @(1905000, 2540000)
    })
} 'pptx-pg-cols' | Out-Null
$tree2 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $colOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-pg-inspect-cols'
$t2 = @($tree2.slides[0].tables) | Select-Object -First 1
$pres2 = Open-PptPres $ppt $colOut
$cap2 = Get-PptCaption $pres2
$s2 = Get-PptSlide $pres2 1
$g2 = Read-PptTableGrid $s2
Release-Com $s2
Close-PptPres $pres2
$pg2ok = ($cap2 -notmatch 'Repair') `
    -and ([int64]$t2.colWidths[0] -eq 1905000) -and ([int64]$t2.colWidths[1] -eq 2540000) `
    -and ([int64]$t2.cx -eq 5486400) `
    -and ([math]::Abs($g2.W1 - 150) -lt 0.6) -and ([math]::Abs($g2.W2 - 200) -lt 0.6)
Add-Check 'pptx-Pg2' $pg2ok "cap=$cap2 w=$($t2.colWidths[0]),$($t2.colWidths[1]) cx=$($t2.cx) comW=$($g2.W1),$($g2.W2)"

$rowOut = Get-OutPath 'pptx-table-grid-row.pptx'
Close-PptPresByPath $ppt $rowOut
if (Test-Path $rowOut) { Remove-Item -Force $rowOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $colOut.Replace('\', '/')
    output = $rowOut.Replace('\', '/')
    ops = @(@{
        op = 'setTableTrHeight'; id = "$tid/row:1"
        h = 1270000
    })
} 'pptx-pg-row' | Out-Null
$tree3 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $rowOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-pg-inspect-row'
$t3 = @($tree3.slides[0].tables) | Select-Object -First 1
$pres3 = Open-PptPres $ppt $rowOut
$cap3 = Get-PptCaption $pres3
$s3 = Get-PptSlide $pres3 1
$g3 = Read-PptTableGrid $s3
Release-Com $s3
Close-PptPres $pres3
$pg3ok = ($cap3 -notmatch 'Repair') `
    -and ([int64]$t3.rows[0].height -eq 1270000) `
    -and ([math]::Abs($g3.H1 - 100) -lt 0.6)
Add-Check 'pptx-Pg3' $pg3ok "cap=$cap3 h=$($t3.rows[0].height) comH=$($g3.H1)"

$sizeOut = Get-OutPath 'pptx-table-grid-size.pptx'
Close-PptPresByPath $ppt $sizeOut
if (Test-Path $sizeOut) { Remove-Item -Force $sizeOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $rowOut.Replace('\', '/')
    output = $sizeOut.Replace('\', '/')
    ops = @(
        @{ op = 'setTableSize'; id = $tid; cx = 4000000; cy = 2000000 }
    )
} 'pptx-pg-size' | Out-Null
$tree4 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sizeOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-pg-inspect-size'
$t4 = @($tree4.slides[0].tables) | Select-Object -First 1
$pres4 = Open-PptPres $ppt $sizeOut
$cap4 = Get-PptCaption $pres4
Close-PptPres $pres4
$h4 = $t4.rows[0].height
$pg4ok = ($cap4 -notmatch 'Repair') `
    -and ([int64]$t4.colWidths[0] -eq 1905000) -and ([int64]$t4.colWidths[1] -eq 2540000) `
    -and ([int64]$t4.cx -eq 4000000) -and ([int64]$t4.cy -eq 2000000) `
    -and ([int64]$h4 -eq 1270000)
Add-Check 'pptx-Pg4' $pg4ok "cap=$cap4 w=$($t4.colWidths[0]),$($t4.colWidths[1]) cx=$($t4.cx) h=$h4"

$failLen = Get-OutPath 'pptx-table-grid-fail-len.pptx'
if (Test-Path $failLen) { Remove-Item -Force $failLen }
$rawLen = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $failLen.Replace('\', '/')
    ops = @(@{ op = 'setTableColWidths'; id = $tid; cols = @(1270000) })
} 'pptx-pg-fail-len'
$codeLen = Get-PptErrorCode $rawLen

$failId = Get-OutPath 'pptx-table-grid-fail-id.pptx'
if (Test-Path $failId) { Remove-Item -Force $failId }
$rawId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $failId.Replace('\', '/')
    ops = @(@{ op = 'setTableTrHeight'; id = $tid; h = 1 })
} 'pptx-pg-fail-id'
$codeId = Get-PptErrorCode $rawId

$failNull = Get-OutPath 'pptx-table-grid-fail-null.pptx'
if (Test-Path $failNull) { Remove-Item -Force $failNull }
$rawNull = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $createOut.Replace('\', '/')
    output = $failNull.Replace('\', '/')
    ops = @(@{ op = 'setTableTrHeight'; id = "$tid/row:1"; h = $null })
} 'pptx-pg-fail-null'
$codeNull = Get-PptErrorCode $rawNull

$xlsx = Get-OutPath 'pptx-table-grid-xlsx.xlsx'
if (Test-Path $xlsx) { Remove-Item -Force $xlsx }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'pptx-pg-xlsx-create' | Out-Null
$failX = Get-OutPath 'pptx-table-grid-xlsx-out.xlsx'
if (Test-Path $failX) { Remove-Item -Force $failX }
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'setTableColWidths'; id = $tid; cols = @(1, 1) })
} 'pptx-pg-fail-xlsx'
$codeX = Get-PptErrorCode $rawX
$pg5ok = ($codeLen -eq 'InvalidField') -and (-not (Test-Path $failLen)) `
    -and ($codeId -eq 'InvalidId') -and (-not (Test-Path $failId)) `
    -and ($codeNull -eq 'InvalidField') -and (-not (Test-Path $failNull)) `
    -and ($codeX -eq 'InvalidField') -and (-not (Test-Path $failX))
Add-Check 'pptx-Pg5' $pg5ok "len=$codeLen id=$codeId null=$codeNull xlsx=$codeX"
