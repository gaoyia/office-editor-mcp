# xlsx 条件格式与迷你图字段：docs/checklist-xlsx.md 的 Xfs1–Xfs5。
# 用 create 产物再 apply，不另做 Excel 另存样例。

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

function Read-SheetSparks($sheet, [string]$cell = 'A1') {
    $n = 0
    $src = ''
    $typ = $null
    $rng = $null
    $col = $null
    try { $rng = $sheet.Range($cell) } catch { }
    if ($null -eq $rng) {
        return @{ Count = 0; Source = ''; Type = $null }
    }
    try { $col = $rng.SparklineGroups } catch { }
    if ($null -eq $col) {
        Release-Com $rng
        return @{ Count = 0; Source = ''; Type = $null }
    }
    try { $n = [int]$col.Count } catch { }
    if ($n -ge 1) {
        $g = $null
        try { $g = $col.Item(1) } catch { }
        if ($null -ne $g) {
            try { $src = [string]$g.SourceData } catch { }
            try { $typ = [int]$g.Type } catch { }
            Release-Com $g
        }
    }
    Release-Com $col
    Release-Com $rng
    return @{ Count = $n; Source = $src; Type = $typ }
}

$hostPath = Get-OutPath 'xlsx-xfs-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = 1 }
                @{ ref = 'A2'; value = -1 }
                @{ ref = 'A3'; value = 5 }
                @{ ref = 'A4'; value = 'foo' }
                @{ ref = 'B1'; value = 1 }
                @{ ref = 'C1'; value = 2 }
                @{ ref = 'D1'; value = 3 }
                @{ ref = 'E1'; value = 4 }
                @{ ref = 'F1'; value = 5 }
            )
        })
    }
} 'xlsx-xfs-create' | Out-Null

$topOut = Get-OutPath 'xlsx-xfs-top10.xlsx'
if (Test-Path $topOut) { Remove-Item -Force $topOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $topOut.Replace('\', '/')
    ops = @(@{
        op = 'insertConditionalFormat'
        id = 'sheet:Sheet1'
        sqref = 'A1:A10'
        type = 'top10'
        rank = 3
        dxf = @{ font = @{ bold = $true } }
    })
} 'xlsx-xfs-top10' | Out-Null

$treeTop = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $topOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:A10' }
} 'xlsx-xfs-top10-inspect'
$rank = $null
try { $rank = [int]$treeTop.sheets[0].conditionalFormats[0].rank } catch { }
$excel = Ensure-MsExcel $topOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $topOut
$cap = [string]$wb.Windows.Item(1).Caption
$xfs1ok = ($cap -notmatch 'Repair') -and ($rank -eq 3)
Add-Check 'xlsx-Xfs1' $xfs1ok "caption=$cap rank=$rank"
Close-AllWorkbooks $excel

$textOut = Get-OutPath 'xlsx-xfs-text.xlsx'
if (Test-Path $textOut) { Remove-Item -Force $textOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $textOut.Replace('\', '/')
    ops = @(
        @{
            op = 'insertConditionalFormat'
            id = 'sheet:Sheet1'
            sqref = 'A1:A10'
            type = 'aboveAverage'
            aboveAverage = $false
        }
        @{
            op = 'insertConditionalFormat'
            id = 'sheet:Sheet1'
            sqref = 'A1:A10'
            type = 'containsText'
            text = 'foo'
        }
    )
} 'xlsx-xfs-text' | Out-Null

$treeText = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $textOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:A10' }
} 'xlsx-xfs-text-inspect'
$avg = $null
$textType = $null
$formulas = @()
try {
    $rules = @($treeText.sheets[0].conditionalFormats)
    $avg = $rules[0].aboveAverage
    $textType = [string]$rules[1].type
    $formulas = @($rules[1].formulas)
} catch { }
$hasFormula = $false
foreach ($f in $formulas) {
    if ([string]$f -match 'SEARCH') { $hasFormula = $true }
}
Close-AllWorkbooks $excel
$wb2 = Open-ExcelBook $excel $textOut
$cap2 = [string]$wb2.Windows.Item(1).Caption
$xfs2ok = ($cap2 -notmatch 'Repair') -and ($avg -eq $false) -and ($textType -eq 'containsText') -and $hasFormula
Add-Check 'xlsx-Xfs2' $xfs2ok "caption=$cap2 avg=$avg type=$textType formula=$hasFormula"
Close-AllWorkbooks $excel

$skOut = Get-OutPath 'xlsx-xfs-spark.xlsx'
if (Test-Path $skOut) { Remove-Item -Force $skOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $skOut.Replace('\', '/')
    ops = @(@{
        op = 'insertSparkline'
        id = 'sheet:Sheet1'
        ref = 'A1'
        formula = 'B1:F1'
        markers = $true
        minAxisType = 'custom'
        manualMin = -2
    })
} 'xlsx-xfs-spark' | Out-Null

Close-AllWorkbooks $excel
$wb3 = Open-ExcelBook $excel $skOut
$cap3 = [string]$wb3.Windows.Item(1).Caption
$s3 = $wb3.Worksheets.Item(1)
$sk = Read-SheetSparks $s3
$treeSk = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $skOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ sparklines = $true }
} 'xlsx-xfs-spark-inspect'
$g0 = @($treeSk.sheets[0].sparklines | Where-Object { $_ }) | Select-Object -First 1
$mk = $false
$minAxis = ''
$manualMin = $null
try { $mk = [bool]$g0.markers } catch { }
try { $minAxis = [string]$g0.minAxisType } catch { }
try { $manualMin = [double]$g0.manualMin } catch { }
$xfs3ok = ($cap3 -notmatch 'Repair') -and ($sk.Count -ge 1) -and $mk -and ($minAxis -eq 'custom') -and ($manualMin -eq -2)
Add-Check 'xlsx-Xfs3' $xfs3ok "caption=$cap3 n=$($sk.Count) markers=$mk axis=$minAxis min=$manualMin"
Release-Com $s3
Close-AllWorkbooks $excel

$stOut = Get-OutPath 'xlsx-xfs-style.xlsx'
if (Test-Path $stOut) { Remove-Item -Force $stOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $stOut.Replace('\', '/')
    ops = @(@{
        op = 'setCellStyle'
        id = 'cell:Sheet1!A1'
        border = @{ diagonal = @{ style = 'thin' } }
        align = @{ indent = 2; rotation = 45 }
    })
} 'xlsx-xfs-style' | Out-Null

$treeSt = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $stOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:A1' }
} 'xlsx-xfs-style-inspect'
$a1 = $null
try { $a1 = $treeSt.sheets[0].cells[0] } catch { }
$diag = $false
$indent = $null
$rot = $null
try { $diag = [bool]$a1.border.diagonalUp } catch { }
try { $indent = [int]$a1.align.indent } catch { }
try { $rot = [int]$a1.align.rotation } catch { }
Close-AllWorkbooks $excel
$wb4 = Open-ExcelBook $excel $stOut
$cap4 = [string]$wb4.Windows.Item(1).Caption
$xfs4ok = ($cap4 -notmatch 'Repair') -and $diag -and ($indent -eq 2) -and ($rot -eq 45)
Add-Check 'xlsx-Xfs4' $xfs4ok "caption=$cap4 diag=$diag indent=$indent rot=$rot"
Close-AllWorkbooks $excel

$fail1 = Get-OutPath 'xlsx-xfs-bad-text.xlsx'
$failText = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $fail1.Replace('\', '/')
    ops = @(@{
        op = 'insertConditionalFormat'
        id = 'sheet:Sheet1'
        sqref = 'A1:A10'
        type = 'containsText'
    })
} 'xlsx-xfs-missing-text'
$code1 = $null
try { $code1 = ($failText.Stderr | ConvertFrom-Json).error.code } catch {}

$fail2 = Get-OutPath 'xlsx-xfs-bad-top10.xlsx'
$failTop = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $fail2.Replace('\', '/')
    ops = @(@{
        op = 'insertConditionalFormat'
        id = 'sheet:Sheet1'
        sqref = 'A1:A10'
        type = 'top10'
        operator = 'greaterThan'
    })
} 'xlsx-xfs-top10-op'
$code2 = $null
try { $code2 = ($failTop.Stderr | ConvertFrom-Json).error.code } catch {}

$fail3 = Get-OutPath 'xlsx-xfs-bad-axis.xlsx'
$failAxis = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $fail3.Replace('\', '/')
    ops = @(@{
        op = 'insertSparkline'
        id = 'sheet:Sheet1'
        ref = 'A1'
        formula = 'B1:F1'
        minAxisType = 'custom'
    })
} 'xlsx-xfs-axis'
$code3 = $null
try { $code3 = ($failAxis.Stderr | ConvertFrom-Json).error.code } catch {}

$xfs5ok = ($failText.ExitCode -ne 0) -and ($code1 -eq 'InvalidField') -and (-not (Test-Path $fail1)) -and
    ($failTop.ExitCode -ne 0) -and ($code2 -eq 'InvalidField') -and (-not (Test-Path $fail2)) -and
    ($failAxis.ExitCode -ne 0) -and ($code3 -eq 'InvalidField') -and (-not (Test-Path $fail3))
Add-Check 'xlsx-Xfs5' $xfs5ok "text=$code1 top=$code2 axis=$code3 wrote=$($(Test-Path $fail1) -or (Test-Path $fail2) -or (Test-Path $fail3))"
