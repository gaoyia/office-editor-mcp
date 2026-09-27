# xlsx 数据透视：docs/checklist-xlsx.md 的 Xpt1–Xpt5。
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

$hostPath = Get-OutPath 'xlsx-pivot-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = '产品' }
                @{ ref = 'B1'; value = '数量' }
                @{ ref = 'C1'; value = '金额' }
                @{ ref = 'A2'; value = '甲' }
                @{ ref = 'B2'; value = 1 }
                @{ ref = 'C2'; value = 10 }
                @{ ref = 'A3'; value = '乙' }
                @{ ref = 'B3'; value = 2 }
                @{ ref = 'C3'; value = 20 }
                @{ ref = 'A4'; value = '甲' }
                @{ ref = 'B4'; value = 3 }
                @{ ref = 'C4'; value = 30 }
                @{ ref = 'A5'; value = '丙' }
                @{ ref = 'B5'; value = 4 }
                @{ ref = 'C5'; value = 40 }
            )
        })
    }
} 'xlsx-xpt-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:C5' }
} 'xlsx-xpt-inspect'
$xpt1ok = ($tree0.sheets[0].pivotCount -eq 0) -and ($null -ne $tree0.sheets[0].pivots) -and ($tree0.sheets[0].pivots.Count -eq 0)
Add-Check 'xlsx-Xpt1' $xpt1ok "count=$($tree0.sheets[0].pivotCount) n=$($tree0.sheets[0].pivots.Count)"

$pivotOut = Get-OutPath 'xlsx-pivot-create.xlsx'
if (Test-Path $pivotOut) { Remove-Item -Force $pivotOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $pivotOut.Replace('\', '/')
    ops = @(@{
        op = 'insertPivotTable'
        id = 'sheet:Sheet1'
        source = @{ sheet = 'Sheet1'; ref = 'A1:C4' }
        location = 'E3'
        name = 'SalesPivot'
        rows = @('产品')
        values = @('金额')
    })
} 'xlsx-xpt-insert' | Out-Null

$excel = Ensure-MsExcel $pivotOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $pivotOut
$cap = [string]$wb.Windows.Item(1).Caption
$s1 = $wb.Worksheets.Item('Sheet1')
$ptCount = $null
$ptName = $null
try {
    $pts = $s1.PivotTables()
    try {
        $ptCount = [int]$pts.Count
        if ($ptCount -ge 1) {
            $pt = $pts.Item(1)
            try { $ptName = [string]$pt.Name } finally { Release-Com $pt }
        }
    } finally { Release-Com $pts }
} catch {
    Write-Log "Xpt2 PivotTables 读失败 $($_.Exception.Message)"
    $ptCount = -1
}
$xpt2ok = ($cap -notmatch 'Repair') -and ($ptCount -ge 1) -and ($ptName -eq 'SalesPivot')
Add-Check 'xlsx-Xpt2' $xpt2ok "caption=$cap count=$ptCount name=$ptName"
Release-Com $s1
Close-AllWorkbooks $excel

$resizeOut = Get-OutPath 'xlsx-pivot-resize.xlsx'
if (Test-Path $resizeOut) { Remove-Item -Force $resizeOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $pivotOut.Replace('\', '/')
    output = $resizeOut.Replace('\', '/')
    ops = @(@{
        op = 'setPivotTable'
        id = 'sheet:Sheet1/pivot:1'
        source = @{ sheet = 'Sheet1'; ref = 'A1:C5' }
    })
} 'xlsx-xpt-resize' | Out-Null

$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $resizeOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:C5' }
} 'xlsx-xpt-resize-inspect'
$wb2 = Open-ExcelBook $excel $resizeOut
$cap2 = [string]$wb2.Windows.Item(1).Caption
$s2 = $wb2.Worksheets.Item('Sheet1')
$rsCount = $null
$rsSrc = $null
try {
    $pts2 = $s2.PivotTables()
    try {
        $rsCount = [int]$pts2.Count
        if ($rsCount -ge 1) {
            $pt2 = $pts2.Item(1)
            try {
                try { $pt2.RefreshTable() | Out-Null } catch {
                    Write-Log "Xpt3 RefreshTable $($_.Exception.Message)"
                }
                $rsSrc = [string]$pt2.SourceData
            } finally { Release-Com $pt2 }
        }
    } finally { Release-Com $pts2 }
} catch {
    Write-Log "Xpt3 PivotTables 读失败 $($_.Exception.Message)"
    $rsCount = -1
}
$srcOk = ($treeR.sheets[0].pivots[0].source.ref -eq 'A1:C5')
$xpt3ok = ($cap2 -notmatch 'Repair') -and ($rsCount -ge 1) -and $srcOk
Add-Check 'xlsx-Xpt3' $xpt3ok "caption=$cap2 count=$rsCount inspect=$($treeR.sheets[0].pivots[0].source.ref) src=$rsSrc"
Release-Com $s2
Close-AllWorkbooks $excel

$delOut = Get-OutPath 'xlsx-pivot-delete.xlsx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $resizeOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deletePivotTable'; id = 'sheet:Sheet1/pivot:1' })
} 'xlsx-xpt-delete' | Out-Null

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:C5' }
} 'xlsx-xpt-delete-inspect'
$wb3 = Open-ExcelBook $excel $delOut
$cap3 = [string]$wb3.Windows.Item(1).Caption
$s3 = $wb3.Worksheets.Item('Sheet1')
$delCount = $null
$a1val = $null
try {
    $pts3 = $s3.PivotTables()
    try { $delCount = [int]$pts3.Count } finally { Release-Com $pts3 }
    $a1 = $s3.Range('A1')
    try { $a1val = [string]$a1.Value2 } finally { Release-Com $a1 }
} catch {
    Write-Log "Xpt4 PivotTables 读失败 $($_.Exception.Message)"
    $delCount = -1
}
$xpt4ok = ($cap3 -notmatch 'Repair') -and ($treeD.sheets[0].pivotCount -eq 0) -and ($delCount -eq 0) -and ($a1val -eq '产品')
Add-Check 'xlsx-Xpt4' $xpt4ok "caption=$cap3 inspect=$($treeD.sheets[0].pivotCount) comCount=$delCount a1=$a1val"
Release-Com $s3
Close-AllWorkbooks $excel

$failOut = Get-OutPath 'xlsx-pivot-bad.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'insertPivotTable'; id = 'sheet:Sheet1' })
} 'xlsx-xpt-missing'
$codeEmpty = $null
try { $codeEmpty = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$xpt5ok = ($fail.ExitCode -ne 0) -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xpt5' $xpt5ok "exit=$($fail.ExitCode) code=$codeEmpty wrote=$(Test-Path $failOut)"
