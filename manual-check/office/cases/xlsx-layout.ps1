# xlsx 列宽行高与合并：docs/checklist-xlsx.md 的 Xl1–Xl5。
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

$hostPath = Get-OutPath 'xlsx-layout-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = 1 }
                @{ ref = 'B1'; value = 2 }
                @{ ref = 'A2'; value = 3 }
                @{ ref = 'B2'; value = 4 }
            )
        })
    }
} 'xlsx-xl-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:C3' }
} 'xlsx-xl-inspect'
$xl1ok = ($null -ne $tree0.sheets[0].cols) -and ($tree0.sheets[0].cols.Count -eq 0) -and ($tree0.sheets[0].rows.Count -eq 0) -and ($tree0.sheets[0].merges.Count -eq 0)
Add-Check 'xlsx-Xl1' $xl1ok "cols=$($tree0.sheets[0].cols.Count) rows=$($tree0.sheets[0].rows.Count) merges=$($tree0.sheets[0].merges.Count)"

$layoutOut = Get-OutPath 'xlsx-layout-apply.xlsx'
if (Test-Path $layoutOut) { Remove-Item -Force $layoutOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $layoutOut.Replace('\', '/')
    ops = @(
        @{ op = 'setColWidth'; id = 'sheet:Sheet1'; col = 'A'; width = 20 }
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 1; height = 30 }
        @{ op = 'mergeRange'; id = 'sheet:Sheet1'; range = 'A1:B2' }
    )
} 'xlsx-xl-apply' | Out-Null

$excel = Ensure-MsExcel $layoutOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $layoutOut
$cap = [string]$wb.Windows.Item(1).Caption
$s1 = $wb.Worksheets.Item('Sheet1')
$a1 = $s1.Range('A1')
$colW = [double]$s1.Range('A1').ColumnWidth
$rowH = [double]$s1.Range('A1').RowHeight
$merged = [bool]$s1.Range('A1:B2').MergeCells
$aVal = $a1.Value2
$b1val = $s1.Range('B1').Value2
$xl2ok = ($cap -notmatch 'Repair') -and ([math]::Abs($colW - 20) -lt 0.25)
Add-Check 'xlsx-Xl2' $xl2ok "caption=$cap colW=$colW"
$xl3ok = ($cap -notmatch 'Repair') -and ([math]::Abs($rowH - 30) -lt 0.25)
Add-Check 'xlsx-Xl3' $xl3ok "rowH=$rowH"
$xl4mergeOk = $merged -and ($null -eq $b1val) -and ([math]::Abs([double]$aVal - 1) -lt 0.001)
Release-Com $a1
Release-Com $s1
Close-AllWorkbooks $excel

$unmergeOut = Get-OutPath 'xlsx-layout-unmerge.xlsx'
if (Test-Path $unmergeOut) { Remove-Item -Force $unmergeOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $layoutOut.Replace('\', '/')
    output = $unmergeOut.Replace('\', '/')
    ops = @(@{ op = 'unmergeRange'; id = 'sheet:Sheet1'; range = 'A1' })
} 'xlsx-xl-unmerge' | Out-Null

$excel2 = Ensure-MsExcel $unmergeOut
Close-AllWorkbooks $excel2
$wb2 = Open-ExcelBook $excel2 $unmergeOut
$cap2 = [string]$wb2.Windows.Item(1).Caption
$s1b = $wb2.Worksheets.Item('Sheet1')
$stillMerged = [bool]$s1b.Range('A1:B2').MergeCells
$xl4ok = ($cap2 -notmatch 'Repair') -and (-not $stillMerged) -and $xl4mergeOk
Add-Check 'xlsx-Xl4' $xl4ok "caption=$cap2 stillMerged=$stillMerged mergedFirst=$xl4mergeOk aVal=$aVal"
Release-Com $s1b
Close-AllWorkbooks $excel2

$failOut = Get-OutPath 'xlsx-layout-empty.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'setColWidth'; id = 'sheet:Sheet1'; col = 'A' })
} 'xlsx-xl-empty'
$codeEmpty = $null
try { $codeEmpty = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$xl5ok = ($fail.ExitCode -ne 0) -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xl5' $xl5ok "exit=$($fail.ExitCode) code=$codeEmpty wrote=$(Test-Path $failOut)"
