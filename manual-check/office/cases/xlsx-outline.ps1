# xlsx 分组 / 分级显示：docs/checklist-xlsx.md 的 Xon1–Xon5。
# 用 create 产物再 apply。不实现放映引擎。

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

$hostPath = Get-OutPath 'xlsx-outline-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = 1 }
                @{ ref = 'A2'; value = 2 }
                @{ ref = 'A3'; value = 3 }
                @{ ref = 'A4'; value = 4 }
                @{ ref = 'A5'; value = 5 }
                @{ ref = 'B1'; value = 'B' }
                @{ ref = 'C1'; value = 'C' }
            )
        })
    }
} 'xlsx-xon-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xon-inspect'
$o0 = $tree0.sheets[0].outline
$rowN0 = 0
if ($o0.rows) { $rowN0 = @($o0.rows).Count }
$colN0 = 0
if ($o0.cols) { $colN0 = @($o0.cols).Count }
$xon1ok = ($null -ne $o0) `
    -and ([bool]$o0.summaryBelow) `
    -and ([bool]$o0.summaryRight) `
    -and ($rowN0 -eq 0) `
    -and ($colN0 -eq 0)
Add-Check 'xlsx-Xon1' $xon1ok "below=$($o0.summaryBelow) right=$($o0.summaryRight) rows=$rowN0 cols=$colN0"

$groupOut = Get-OutPath 'xlsx-outline-group.xlsx'
if (Test-Path $groupOut) { Remove-Item -Force $groupOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $groupOut.Replace('\', '/')
    ops = @(
        @{ op = 'groupRows'; id = 'sheet:Sheet1'; rows = '2:4' }
        @{ op = 'groupCols'; id = 'sheet:Sheet1'; col = 'B:C' }
    )
} 'xlsx-xon-group' | Out-Null

$treeG = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $groupOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xon-inspect-group'
$og = $treeG.sheets[0].outline
$lvl2 = $null
if ($og.rows) {
    $hit = @($og.rows) | Where-Object { [int]$_.r -eq 2 } | Select-Object -First 1
    if ($hit) { $lvl2 = [int]$hit.level }
}
$colLvl = $null
if ($og.cols -and @($og.cols).Count -gt 0) { $colLvl = [int]$og.cols[0].level }

$excel = Ensure-MsExcel $groupOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $groupOut
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item(1)
$rowLvl = [int]$ws.Rows.Item(2).OutlineLevel
$colBLvl = [int]$ws.Columns.Item(2).OutlineLevel
$xon2ok = ($cap -notmatch 'Repair') -and ($rowLvl -eq 2) -and ($colBLvl -eq 2) -and ($lvl2 -eq 1) -and ($colLvl -eq 1)
Add-Check 'xlsx-Xon2' $xon2ok "caption=$cap comRow=$rowLvl comCol=$colBLvl jsonRow=$lvl2 jsonCol=$colLvl"
Close-AllWorkbooks $excel

$collapseOut = Get-OutPath 'xlsx-outline-collapse.xlsx'
if (Test-Path $collapseOut) { Remove-Item -Force $collapseOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $groupOut.Replace('\', '/')
    output = $collapseOut.Replace('\', '/')
    ops = @(@{
        op = 'setOutline'
        id = 'sheet:Sheet1'
        collapseRows = '2:4'
        collapsed = $true
    })
} 'xlsx-xon-collapse' | Out-Null

$excelC = Ensure-MsExcel $collapseOut
Close-AllWorkbooks $excelC
$wbC = Open-ExcelBook $excelC $collapseOut
$capC = [string]$wbC.Windows.Item(1).Caption
$wsC = $wbC.Worksheets.Item(1)
$hidden2 = [bool]$wsC.Rows.Item(2).Hidden
$hidden4 = [bool]$wsC.Rows.Item(4).Hidden
$xon3ok = ($capC -notmatch 'Repair') -and $hidden2 -and $hidden4
Add-Check 'xlsx-Xon3' $xon3ok "caption=$capC hidden2=$hidden2 hidden4=$hidden4"
Close-AllWorkbooks $excelC

$ungroupOut = Get-OutPath 'xlsx-outline-ungroup.xlsx'
if (Test-Path $ungroupOut) { Remove-Item -Force $ungroupOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $groupOut.Replace('\', '/')
    output = $ungroupOut.Replace('\', '/')
    ops = @(
        @{ op = 'ungroupRows'; id = 'sheet:Sheet1'; rows = '2:4' }
        @{ op = 'ungroupCols'; id = 'sheet:Sheet1'; col = 'B:C' }
    )
} 'xlsx-xon-ungroup' | Out-Null

$treeU = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ungroupOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xon-inspect-ungroup'
$ou = $treeU.sheets[0].outline
$rowNu = 0
if ($ou.rows) { $rowNu = @($ou.rows).Count }
$colNu = 0
if ($ou.cols) { $colNu = @($ou.cols).Count }
$excelU = Ensure-MsExcel $ungroupOut
Close-AllWorkbooks $excelU
$wbU = Open-ExcelBook $excelU $ungroupOut
$capU = [string]$wbU.Windows.Item(1).Caption
$xon4ok = ($capU -notmatch 'Repair') -and ($rowNu -eq 0) -and ($colNu -eq 0)
Add-Check 'xlsx-Xon4' $xon4ok "caption=$capU rows=$rowNu cols=$colNu"
Close-AllWorkbooks $excelU

$failEmpty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = (Get-OutPath 'xlsx-outline-fail-empty.xlsx').Replace('\', '/')
    ops = @(@{ op = 'setOutline'; id = 'sheet:Sheet1' })
} 'xlsx-xon-fail-empty'
$failId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = (Get-OutPath 'xlsx-outline-fail-id.xlsx').Replace('\', '/')
    ops = @(@{ op = 'groupRows'; id = 'slide:1'; rows = '2:4' })
} 'xlsx-xon-fail-id'
$xon5ok = ($failEmpty.ExitCode -ne 0) -and ($failId.ExitCode -ne 0) `
    -and -not (Test-Path (Get-OutPath 'xlsx-outline-fail-empty.xlsx')) `
    -and -not (Test-Path (Get-OutPath 'xlsx-outline-fail-id.xlsx'))
Add-Check 'xlsx-Xon5' $xon5ok "empty=$($failEmpty.ExitCode) id=$($failId.ExitCode) stderr=$($failEmpty.Stderr)"
