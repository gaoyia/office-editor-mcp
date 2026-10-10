# xlsx 隐藏行列与排序字段：docs/checklist-xlsx.md 的 Xhd1–Xhd5。
# 用 create 产物再 apply。不按值重排格子，不算排序引擎。

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

$hostPath = Get-OutPath 'xlsx-hidden-sort-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = 'Low' }
                @{ ref = 'A2'; value = 'Medium' }
                @{ ref = 'A3'; value = 'High' }
                @{ ref = 'A4'; value = 'Low' }
                @{ ref = 'B1'; value = 'B' }
            )
        })
    }
} 'xlsx-xhd-create' | Out-Null

$hideOut = Get-OutPath 'xlsx-hidden-sort-hide.xlsx'
if (Test-Path $hideOut) { Remove-Item -Force $hideOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $hideOut.Replace('\', '/')
    ops = @(
        @{ op = 'setRowHidden'; id = 'sheet:Sheet1'; rows = '3'; hidden = $true }
        @{ op = 'setColHidden'; id = 'sheet:Sheet1'; col = 'B'; hidden = $true }
    )
} 'xlsx-xhd-hide' | Out-Null

$treeHide = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hideOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xhd-inspect-hide'
$ol = $treeHide.sheets[0].outline
$row3 = $ol.rows | Where-Object { $_.r -eq 3 } | Select-Object -First 1
$colB = $ol.cols | Where-Object { $_.min -eq 'B' } | Select-Object -First 1
$xhd1ok = ($null -ne $row3) -and ([bool]$row3.hidden) -and ([int]$row3.level -eq 0) -and ($null -ne $colB) -and ([bool]$colB.hidden) -and ([int]$colB.level -eq 0)
Add-Check 'xlsx-Xhd1' $xhd1ok "row3=$($row3 | ConvertTo-Json -Compress) colB=$($colB | ConvertTo-Json -Compress)"

$excel = Ensure-MsExcel $hideOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $hideOut
$cap = [string]$wb.Windows.Item(1).Caption
$ws = $wb.Worksheets.Item(1)
$rowHidden = $false
$colHidden = $false
try { $rowHidden = [bool]$ws.Rows.Item(3).Hidden } catch { $rowHidden = $false }
try { $colHidden = [bool]$ws.Columns.Item(2).Hidden } catch { $colHidden = $false }
$xhd2ok = ($cap -notmatch 'Repair') -and $rowHidden -and $colHidden
Add-Check 'xlsx-Xhd2' $xhd2ok "caption=$cap rowHidden=$rowHidden colHidden=$colHidden"
Close-AllWorkbooks $excel

$listOut = Get-OutPath 'xlsx-hidden-sort-list.xlsx'
if (Test-Path $listOut) { Remove-Item -Force $listOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $listOut.Replace('\', '/')
    ops = @(@{
        op = 'setSortState'
        id = 'sheet:Sheet1'
        ref = 'A1:A4'
        columns = @(@{ ref = 'A1:A4'; customList = 'Low,Medium,High' })
    })
} 'xlsx-xhd-list' | Out-Null

$treeList = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $listOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:A4' }
} 'xlsx-xhd-inspect-list'
$stList = $treeList.sheets[0].sortState
$a1cell = $treeList.sheets[0].cells | Where-Object { $_.ref -eq 'A1' } | Select-Object -First 1
$a1val = if ($null -ne $a1cell) { [string]$a1cell.value } else { '' }
$listInspectOk = ($null -ne $stList) -and ($stList.columns[0].customList -eq 'Low,Medium,High') -and ($a1val -eq 'Low')
Close-AllWorkbooks $excel
$wbL = Open-ExcelBook $excel $listOut
$capL = [string]$wbL.Windows.Item(1).Caption
$xhd3ok = ($capL -notmatch 'Repair') -and $listInspectOk
Add-Check 'xlsx-Xhd3' $xhd3ok "caption=$capL customList=$($stList.columns[0].customList) a1=$a1val"
Close-AllWorkbooks $excel

$colorOut = Get-OutPath 'xlsx-hidden-sort-color.xlsx'
if (Test-Path $colorOut) { Remove-Item -Force $colorOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $colorOut.Replace('\', '/')
    ops = @(@{
        op = 'setSortState'
        id = 'sheet:Sheet1'
        ref = 'A1:A4'
        sortMethod = 'stroke'
        columns = @(@{
            ref = 'A1:A4'
            sortBy = 'cellColor'
            color = @{ rgb = 'FFFF0000' }
        })
    })
} 'xlsx-xhd-color' | Out-Null

$treeColor = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $colorOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xhd-inspect-color'
$stColor = $treeColor.sheets[0].sortState
$colorInspectOk = ($null -ne $stColor) -and ($stColor.sortMethod -eq 'stroke') -and ($stColor.columns[0].sortBy -eq 'cellColor') -and ($stColor.columns[0].color.rgb -eq 'FFFF0000')
Close-AllWorkbooks $excel
$wbC = Open-ExcelBook $excel $colorOut
$capC = [string]$wbC.Windows.Item(1).Caption
$xhd4ok = ($capC -notmatch 'Repair') -and $colorInspectOk
Add-Check 'xlsx-Xhd4' $xhd4ok "caption=$capC method=$($stColor.sortMethod) sortBy=$($stColor.columns[0].sortBy) rgb=$($stColor.columns[0].color.rgb)"
Close-AllWorkbooks $excel

$failColor = Get-OutPath 'xlsx-hidden-sort-bad-color.xlsx'
$fail1 = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failColor.Replace('\', '/')
    ops = @(@{
        op = 'setSortState'
        id = 'sheet:Sheet1'
        ref = 'A1:A4'
        columns = @(@{ ref = 'A1:A4'; sortBy = 'cellColor' })
    })
} 'xlsx-xhd-bad-color'
$code1 = $null
try { $code1 = ($fail1.Stderr | ConvertFrom-Json).error.code } catch {}

$failIcon = Get-OutPath 'xlsx-hidden-sort-bad-icon.xlsx'
$fail2 = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failIcon.Replace('\', '/')
    ops = @(@{
        op = 'setSortState'
        id = 'sheet:Sheet1'
        ref = 'A1:A4'
        columns = @(@{ ref = 'A1:A4'; sortBy = 'icon'; customList = 'Low,Medium,High' })
    })
} 'xlsx-xhd-bad-icon'
$code2 = $null
try { $code2 = ($fail2.Stderr | ConvertFrom-Json).error.code } catch {}

$failSlide = Get-OutPath 'xlsx-hidden-sort-bad-id.xlsx'
$fail3 = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failSlide.Replace('\', '/')
    ops = @(@{
        op = 'setRowHidden'
        id = 'slide:1'
        rows = '3'
        hidden = $true
    })
} 'xlsx-xhd-bad-id'
$code3 = $null
try { $code3 = ($fail3.Stderr | ConvertFrom-Json).error.code } catch {}

$xhd5ok = ($fail1.ExitCode -ne 0) -and ($code1 -eq 'InvalidField') -and (-not (Test-Path $failColor)) `
    -and ($fail2.ExitCode -ne 0) -and ($code2 -eq 'InvalidField') -and (-not (Test-Path $failIcon)) `
    -and ($fail3.ExitCode -ne 0) -and ($code3 -eq 'InvalidId') -and (-not (Test-Path $failSlide))
Add-Check 'xlsx-Xhd5' $xhd5ok "exit=$($fail1.ExitCode)/$($fail2.ExitCode)/$($fail3.ExitCode) code=$code1/$code2/$code3 wrote=$(Test-Path $failColor)/$(Test-Path $failIcon)/$(Test-Path $failSlide)"
