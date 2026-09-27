# xlsx 视图与筛选：docs/checklist-xlsx.md 的 Xvw1–Xvw5。
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

$hostPath = Get-OutPath 'xlsx-view-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = 1 }
                @{ ref = 'A10'; value = 10 }
            )
        })
    }
} 'xlsx-xvw-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xvw-inspect'
$v0 = $tree0.sheets[0].view
$xvw1ok = ($null -ne $v0) -and ([int]$v0.zoom -eq 100) -and ([int]$v0.frozenRows -eq 0) -and ([int]$v0.frozenCols -eq 0) -and ($v0.activeCell -eq 'A1') -and ($null -eq $tree0.sheets[0].autoFilter)
Add-Check 'xlsx-Xvw1' $xvw1ok "zoom=$($v0.zoom) rows=$($v0.frozenRows) cols=$($v0.frozenCols) cell=$($v0.activeCell) filter=$($tree0.sheets[0].autoFilter)"

$viewOut = Get-OutPath 'xlsx-view-setup.xlsx'
if (Test-Path $viewOut) { Remove-Item -Force $viewOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $viewOut.Replace('\', '/')
    ops = @(@{
        op = 'setSheetView'
        id = 'sheet:Sheet1'
        zoom = 80
        frozenRows = 1
        frozenCols = 0
    })
} 'xlsx-xvw-freeze' | Out-Null

$excel = Ensure-MsExcel $viewOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $viewOut
$cap = [string]$wb.Windows.Item(1).Caption
$wnd = $excel.ActiveWindow
$zoom = [int]$wnd.Zoom
$frozen = [bool]$wnd.FreezePanes
$splitRow = [int]$wnd.SplitRow
$xvw2ok = ($cap -notmatch 'Repair') -and ($zoom -eq 80) -and $frozen -and ($splitRow -eq 1)
Add-Check 'xlsx-Xvw2' $xvw2ok "caption=$cap zoom=$zoom freeze=$frozen splitRow=$splitRow"
Close-AllWorkbooks $excel

$filterOut = Get-OutPath 'xlsx-view-filter.xlsx'
if (Test-Path $filterOut) { Remove-Item -Force $filterOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $viewOut.Replace('\', '/')
    output = $filterOut.Replace('\', '/')
    ops = @(@{
        op = 'setAutoFilter'
        id = 'sheet:Sheet1'
        ref = 'A1:A10'
    })
} 'xlsx-xvw-filter' | Out-Null

Close-AllWorkbooks $excel
$wbF = Open-ExcelBook $excel $filterOut
$capF = [string]$wbF.Windows.Item(1).Caption
$sF = $wbF.Worksheets.Item(1)
$mode = [bool]$sF.AutoFilterMode
$addr = ''
if ($mode) { $addr = [string]$sF.AutoFilter.Range.Address($true, $true) }
$filterOk = $mode -and ($addr -match 'A\$1:\$A\$10')
$xvw3ok = ($capF -notmatch 'Repair') -and $filterOk
Add-Check 'xlsx-Xvw3' $xvw3ok "caption=$capF mode=$mode addr=$addr"
Close-AllWorkbooks $excel

$unfreezeOut = Get-OutPath 'xlsx-view-unfreeze.xlsx'
if (Test-Path $unfreezeOut) { Remove-Item -Force $unfreezeOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $filterOut.Replace('\', '/')
    output = $unfreezeOut.Replace('\', '/')
    ops = @(@{
        op = 'setSheetView'
        id = 'sheet:Sheet1'
        frozenRows = 0
        frozenCols = 0
    })
} 'xlsx-xvw-unfreeze' | Out-Null

Close-AllWorkbooks $excel
$wbU = Open-ExcelBook $excel $unfreezeOut
$capU = [string]$wbU.Windows.Item(1).Caption
$frozenU = [bool]$excel.ActiveWindow.FreezePanes
$xvw4ok = ($capU -notmatch 'Repair') -and (-not $frozenU)
Add-Check 'xlsx-Xvw4' $xvw4ok "caption=$capU freeze=$frozenU"
Close-AllWorkbooks $excel

$failOut = Get-OutPath 'xlsx-view-bad.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'setSheetView'; id = 'sheet:Sheet1' })
} 'xlsx-xvw-missing'
$codeEmpty = $null
try { $codeEmpty = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$xvw5ok = ($fail.ExitCode -ne 0) -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xvw5' $xvw5ok "exit=$($fail.ExitCode) code=$codeEmpty wrote=$(Test-Path $failOut)"
