# xlsx 工作表管理：docs/checklist-xlsx.md 的 Xm1–Xm7。
# 源文件由 Excel 亲手存出，至少两张可见 worksheet。

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

function Get-ComSheetNames($wb) {
    1..$wb.Worksheets.Count | ForEach-Object { [string]$wb.Worksheets.Item($_).Name }
}

function Get-SheetNames($tree) {
    @($tree.sheets | ForEach-Object { [string]$_.name })
}

$src = Ensure-XlsxExcelSource
$excel = Ensure-MsExcel $src
Close-AllWorkbooks $excel

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xm-inspect'
$haveHidden = @($tree0.sheets | ForEach-Object { $_.PSObject.Properties.Name -contains 'hidden' })
$visibleFalse = -not (@($tree0.sheets | ForEach-Object { [bool]$_.hidden }) -contains $true)
$xm1ok = ($haveHidden -notcontains $false) -and $visibleFalse
Add-Check 'xlsx-Xm1' $xm1ok "keys=$($tree0.sheets[0].PSObject.Properties.Name -join ',') hidden=$((@($tree0.sheets | ForEach-Object { $_.hidden })) -join ',')"

$insertOut = Get-OutPath 'xlsx-sheets-insert.xlsx'
if (Test-Path $insertOut) { Remove-Item -Force $insertOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $insertOut.Replace('\', '/')
    ops = @(@{ op = 'insertSheet'; id = 'xlsx:workbook'; name = 'Sheet3' })
} 'xlsx-xm-insert' | Out-Null
$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insertOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xm-insert-inspect'
$namesI = Get-SheetNames $treeI
$wbI = Open-ExcelBook $excel $insertOut
$capI = [string]$wbI.Windows.Item(1).Caption
$comI = Get-ComSheetNames $wbI
$xm2ok = ($capI -notmatch 'Repair') -and ($namesI -contains 'Sheet3') -and ($comI -contains 'Sheet3')
Add-Check 'xlsx-Xm2' $xm2ok "caption=$capI names=$($namesI -join ',') com=$($comI -join ',')"
Close-AllWorkbooks $excel

$renameOut = Get-OutPath 'xlsx-sheets-rename.xlsx'
if (Test-Path $renameOut) { Remove-Item -Force $renameOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insertOut.Replace('\', '/')
    output = $renameOut.Replace('\', '/')
    ops = @(@{ op = 'renameSheet'; id = 'sheet:Sheet3'; name = '附录' })
} 'xlsx-xm-rename' | Out-Null
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $renameOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xm-rename-inspect'
$namesR = Get-SheetNames $treeR
$failOld = Get-OutPath 'xlsx-sheets-old-id.xlsx'
if (Test-Path $failOld) { Remove-Item -Force $failOld }
$rawOld = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $renameOut.Replace('\', '/')
    output = $failOld.Replace('\', '/')
    ops = @(@{ op = 'setCell'; id = 'cell:Sheet3!A1'; value = 9 })
} 'xlsx-xm-old-id'
$codeOld = $null
try { $codeOld = ($rawOld.Stderr | ConvertFrom-Json).error.code } catch {}
$setNew = Get-OutPath 'xlsx-sheets-new-id.xlsx'
if (Test-Path $setNew) { Remove-Item -Force $setNew }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $renameOut.Replace('\', '/')
    output = $setNew.Replace('\', '/')
    ops = @(@{ op = 'setCell'; id = 'cell:附录!A1'; value = 9 })
} 'xlsx-xm-new-id' | Out-Null
$wbR = Open-ExcelBook $excel $setNew
$capR = [string]$wbR.Windows.Item(1).Caption
$comR = Get-ComSheetNames $wbR
$valR = $wbR.Worksheets.Item('附录').Range('A1').Value2
$xm3ok = ($capR -notmatch 'Repair') -and ($namesR -contains '附录') -and ($namesR -notcontains 'Sheet3') -and
    ($codeOld -eq 'IdNotFound') -and (-not (Test-Path $failOld)) -and ($comR -contains '附录') -and ($valR -eq 9)
Add-Check 'xlsx-Xm3' $xm3ok "caption=$capR names=$($namesR -join ',') old=$codeOld val=$valR"
Close-AllWorkbooks $excel

$moveOut = Get-OutPath 'xlsx-sheets-move.xlsx'
if (Test-Path $moveOut) { Remove-Item -Force $moveOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $renameOut.Replace('\', '/')
    output = $moveOut.Replace('\', '/')
    ops = @(@{ op = 'moveSheet'; id = 'sheet:附录' })
} 'xlsx-xm-move' | Out-Null
$treeM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $moveOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xm-move-inspect'
$namesM = Get-SheetNames $treeM
$wbM = Open-ExcelBook $excel $moveOut
$capM = [string]$wbM.Windows.Item(1).Caption
$firstM = [string]$wbM.Worksheets.Item(1).Name
$xm4ok = ($capM -notmatch 'Repair') -and ($namesM[0] -eq '附录') -and ($firstM -eq '附录')
Add-Check 'xlsx-Xm4' $xm4ok "caption=$capM names=$($namesM -join ',') first=$firstM"
Close-AllWorkbooks $excel

$hideOut = Get-OutPath 'xlsx-sheets-hide.xlsx'
if (Test-Path $hideOut) { Remove-Item -Force $hideOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $moveOut.Replace('\', '/')
    output = $hideOut.Replace('\', '/')
    ops = @(@{ op = 'setSheetHidden'; id = 'sheet:Sheet2'; hidden = $true })
} 'xlsx-xm-hide' | Out-Null
$treeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hideOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xm-hide-inspect'
$sheet2 = @($treeH.sheets | Where-Object { $_.name -eq 'Sheet2' })[0]
$wbH = Open-ExcelBook $excel $hideOut
$capH = [string]$wbH.Windows.Item(1).Caption
$visH = [int]$wbH.Worksheets.Item('Sheet2').Visible
$xm5ok = ($capH -notmatch 'Repair') -and ([bool]$sheet2.hidden) -and ($visH -eq 0)
Add-Check 'xlsx-Xm5' $xm5ok "caption=$capH hidden=$($sheet2.hidden) visible=$visH"
Close-AllWorkbooks $excel

$delOut = Get-OutPath 'xlsx-sheets-delete.xlsx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hideOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteSheet'; id = 'sheet:Sheet2' })
} 'xlsx-xm-delete' | Out-Null
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xm-delete-inspect'
$namesD = Get-SheetNames $treeD
$wbD = Open-ExcelBook $excel $delOut
$capD = [string]$wbD.Windows.Item(1).Caption
$comD = Get-ComSheetNames $wbD
$xm6ok = ($capD -notmatch 'Repair') -and ($namesD -notcontains 'Sheet2') -and ($comD -notcontains 'Sheet2')
Add-Check 'xlsx-Xm6' $xm6ok "caption=$capD names=$($namesD -join ',') com=$($comD -join ',')"
Close-AllWorkbooks $excel

$hidLast = Get-OutPath 'xlsx-sheets-last-visible-host.xlsx'
if (Test-Path $hidLast) { Remove-Item -Force $hidLast }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $hidLast.Replace('\', '/')
    ops = @(@{ op = 'setSheetHidden'; id = 'sheet:Sheet2'; hidden = $true })
} 'xlsx-xm-last-host' | Out-Null
$failHide = Get-OutPath 'xlsx-sheets-last-hide.xlsx'
if (Test-Path $failHide) { Remove-Item -Force $failHide }
$rawHide = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hidLast.Replace('\', '/')
    output = $failHide.Replace('\', '/')
    ops = @(@{ op = 'setSheetHidden'; id = 'sheet:Sheet1'; hidden = $true })
} 'xlsx-xm-last-hide'
$codeHide = $null
try { $codeHide = ($rawHide.Stderr | ConvertFrom-Json).error.code } catch {}
$failDel = Get-OutPath 'xlsx-sheets-last-delete.xlsx'
if (Test-Path $failDel) { Remove-Item -Force $failDel }
$rawDel = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hidLast.Replace('\', '/')
    output = $failDel.Replace('\', '/')
    ops = @(@{ op = 'deleteSheet'; id = 'sheet:Sheet1' })
} 'xlsx-xm-last-delete'
$codeDel = $null
try { $codeDel = ($rawDel.Stderr | ConvertFrom-Json).error.code } catch {}
$xm7ok = ($rawHide.ExitCode -ne 0) -and ($codeHide -eq 'InvalidField') -and (-not (Test-Path $failHide)) -and
    ($rawDel.ExitCode -ne 0) -and ($codeDel -eq 'InvalidField') -and (-not (Test-Path $failDel))
Add-Check 'xlsx-Xm7' $xm7ok "hide=$codeHide del=$codeDel wroteHide=$(Test-Path $failHide) wroteDel=$(Test-Path $failDel)"
