# xlsx 拆分窗格与自定义视图：docs/checklist-xlsx.md 的 Xsv1–Xsv5。
# 用 create 产物再 apply。不实现放映引擎。不按视图恢复隐藏行 / 列。

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

$hostPath = Get-OutPath 'xlsx-custom-view-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = 1 }
                @{ ref = 'C5'; value = 5 }
            )
        })
    }
} 'xlsx-xsv-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xsv-inspect'
$v0 = $tree0.sheets[0].view
$listed0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ customViews = $true }
} 'xlsx-xsv-inspect-list'
$cvn0 = 0
if ($listed0.customViews) { $cvn0 = @($listed0.customViews).Count }
$xsv1ok = ($null -ne $v0) `
    -and ([int]$v0.splitRows -eq 0) `
    -and ([int]$v0.splitCols -eq 0) `
    -and ([int]$tree0.customViewCount -eq 0) `
    -and ($cvn0 -eq 0)
Add-Check 'xlsx-Xsv1' $xsv1ok "split=$($v0.splitRows)/$($v0.splitCols) count=$($tree0.customViewCount) list=$cvn0"

$splitOut = Get-OutPath 'xlsx-custom-view-split.xlsx'
if (Test-Path $splitOut) { Remove-Item -Force $splitOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $splitOut.Replace('\', '/')
    ops = @(@{
        op = 'setSheetView'
        id = 'sheet:Sheet1'
        splitRows = 4
        splitCols = 2
    })
} 'xlsx-xsv-split' | Out-Null

$excel = Ensure-MsExcel $splitOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $splitOut
$cap = [string]$wb.Windows.Item(1).Caption
$win = $wb.Windows.Item(1)
$split = [bool]$win.Split
$splitRow = [int]$win.SplitRow
$splitCol = [int]$win.SplitColumn
$freeze = [bool]$win.FreezePanes
$xsv2ok = ($cap -notmatch 'Repair') -and $split -and ($splitRow -eq 4) -and ($splitCol -eq 2) -and (-not $freeze)
Add-Check 'xlsx-Xsv2' $xsv2ok "caption=$cap split=$split row=$splitRow col=$splitCol freeze=$freeze"
Close-AllWorkbooks $excel

$groupOut = Get-OutPath 'xlsx-custom-view-group.xlsx'
if (Test-Path $groupOut) { Remove-Item -Force $groupOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $groupOut.Replace('\', '/')
    ops = @(@{
        op = 'insertCustomView'
        id = 'xlsx:workbook'
        name = 'DemoView'
    })
} 'xlsx-xsv-insert' | Out-Null

$treeG = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $groupOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ customViews = $true }
} 'xlsx-xsv-inspect-group'
$cvName = $null
if ($treeG.customViews -and @($treeG.customViews).Count -gt 0) {
    $cvName = [string]$treeG.customViews[0].name
}
$excelG = Ensure-MsExcel $groupOut
Close-AllWorkbooks $excelG
$wbG = Open-ExcelBook $excelG $groupOut
$capG = [string]$wbG.Windows.Item(1).Caption
$cvCount = [int]$wbG.CustomViews.Count
$xsv3ok = ($capG -notmatch 'Repair') -and ($cvCount -eq 1) -and ($cvName -eq 'DemoView') -and ([int]$treeG.customViewCount -eq 1)
Add-Check 'xlsx-Xsv3' $xsv3ok "caption=$capG com=$cvCount name=$cvName json=$($treeG.customViewCount)"
Close-AllWorkbooks $excelG

$ungroupOut = Get-OutPath 'xlsx-custom-view-ungroup.xlsx'
if (Test-Path $ungroupOut) { Remove-Item -Force $ungroupOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $groupOut.Replace('\', '/')
    output = $ungroupOut.Replace('\', '/')
    ops = @(@{ op = 'deleteCustomView'; id = 'customView:1' })
} 'xlsx-xsv-delete' | Out-Null

$treeU = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ungroupOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ customViews = $true }
} 'xlsx-xsv-inspect-ungroup'
$excelU = Ensure-MsExcel $ungroupOut
Close-AllWorkbooks $excelU
$wbU = Open-ExcelBook $excelU $ungroupOut
$capU = [string]$wbU.Windows.Item(1).Caption
$cvCountU = [int]$wbU.CustomViews.Count
$xsv4ok = ($capU -notmatch 'Repair') -and ([int]$treeU.customViewCount -eq 0) -and ($cvCountU -eq 0)
Add-Check 'xlsx-Xsv4' $xsv4ok "caption=$capU json=$($treeU.customViewCount) com=$cvCountU"
Close-AllWorkbooks $excelU

$failMix = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = (Get-OutPath 'xlsx-custom-view-fail-mix.xlsx').Replace('\', '/')
    ops = @(@{ op = 'setSheetView'; id = 'sheet:Sheet1'; frozenRows = 1; splitRows = 4 })
} 'xlsx-xsv-fail-mix'
$failEmpty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = (Get-OutPath 'xlsx-custom-view-fail-empty.xlsx').Replace('\', '/')
    ops = @(@{ op = 'insertCustomView'; id = 'xlsx:workbook'; name = '' })
} 'xlsx-xsv-fail-empty'
$failId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = (Get-OutPath 'xlsx-custom-view-fail-id.xlsx').Replace('\', '/')
    ops = @(@{ op = 'insertCustomView'; id = 'slide:1'; name = 'DemoView' })
} 'xlsx-xsv-fail-id'
$xsv5ok = ($failMix.ExitCode -ne 0) -and ($failEmpty.ExitCode -ne 0) -and ($failId.ExitCode -ne 0) `
    -and -not (Test-Path (Get-OutPath 'xlsx-custom-view-fail-mix.xlsx')) `
    -and -not (Test-Path (Get-OutPath 'xlsx-custom-view-fail-empty.xlsx')) `
    -and -not (Test-Path (Get-OutPath 'xlsx-custom-view-fail-id.xlsx'))
Add-Check 'xlsx-Xsv5' $xsv5ok "mix=$($failMix.ExitCode) empty=$($failEmpty.ExitCode) id=$($failId.ExitCode) stderr=$($failEmpty.Stderr)"
