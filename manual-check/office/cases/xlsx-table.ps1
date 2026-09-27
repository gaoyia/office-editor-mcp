# xlsx 工作表表格：docs/checklist-xlsx.md 的 Xt1–Xt5。
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

$hostPath = Get-OutPath 'xlsx-table-host.xlsx'
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
            )
        })
    }
} 'xlsx-xt-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:C4' }
} 'xlsx-xt-inspect'
$xt1ok = ($tree0.sheets[0].tableCount -eq 0) -and ($null -ne $tree0.sheets[0].tables) -and ($tree0.sheets[0].tables.Count -eq 0)
Add-Check 'xlsx-Xt1' $xt1ok "count=$($tree0.sheets[0].tableCount) n=$($tree0.sheets[0].tables.Count)"

$tableOut = Get-OutPath 'xlsx-table-create.xlsx'
if (Test-Path $tableOut) { Remove-Item -Force $tableOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $tableOut.Replace('\', '/')
    ops = @(@{
        op = 'insertSheetTable'
        id = 'sheet:Sheet1'
        ref = 'A1:C3'
        name = 'Sales'
        columns = @('产品', '数量', '金额')
    })
} 'xlsx-xt-insert' | Out-Null

$excel = Ensure-MsExcel $tableOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $tableOut
$cap = [string]$wb.Windows.Item(1).Caption
$s1 = $wb.Worksheets.Item('Sheet1')
$loCount = $null
$loName = $null
$col1 = $null
$col2 = $null
$col3 = $null
try {
    $los = $s1.ListObjects
    $loCount = [int]$los.Count
    if ($loCount -ge 1) {
        $lo = $los.Item(1)
        $loName = [string]$lo.Name
        $cols = $lo.ListColumns
        try {
            $col1 = [string]$cols.Item(1).Name
            $col2 = [string]$cols.Item(2).Name
            $col3 = [string]$cols.Item(3).Name
        } finally { Release-Com $cols }
        Release-Com $lo
    }
    Release-Com $los
} catch {
    Write-Log "Xt2 ListObjects 读失败 $($_.Exception.Message)"
}
$xt2ok = ($cap -notmatch 'Repair') -and ($loCount -ge 1) -and ($loName -eq 'Sales') -and ($col1 -eq '产品') -and ($col2 -eq '数量') -and ($col3 -eq '金额')
Add-Check 'xlsx-Xt2' $xt2ok "caption=$cap count=$loCount name=$loName cols=$col1,$col2,$col3"
Release-Com $s1
Close-AllWorkbooks $excel

$resizeOut = Get-OutPath 'xlsx-table-resize.xlsx'
if (Test-Path $resizeOut) { Remove-Item -Force $resizeOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $tableOut.Replace('\', '/')
    output = $resizeOut.Replace('\', '/')
    ops = @(@{
        op = 'setSheetTable'
        id = 'sheet:Sheet1/table:1'
        ref = 'A1:C4'
    })
} 'xlsx-xt-resize' | Out-Null

$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $resizeOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:C4' }
} 'xlsx-xt-resize-inspect'
$wb2 = Open-ExcelBook $excel $resizeOut
$cap2 = [string]$wb2.Windows.Item(1).Caption
$s2 = $wb2.Worksheets.Item('Sheet1')
$rsCount = $null
$rsRows = $null
$rsCols = $null
$rsName = $null
try {
    $los2 = $s2.ListObjects
    $rsCount = [int]$los2.Count
    if ($rsCount -ge 1) {
        $lo2 = $los2.Item(1)
        $rsName = [string]$lo2.Name
        $rg = $lo2.Range
        try {
            $rows = $rg.Rows
            try { $rsRows = [int]$rows.Count } finally { Release-Com $rows }
            $cols = $rg.Columns
            try { $rsCols = [int]$cols.Count } finally { Release-Com $cols }
        } finally { Release-Com $rg }
        Release-Com $lo2
    }
    Release-Com $los2
} catch {
    Write-Log "Xt3 ListObjects 读失败 $($_.Exception.Message)"
}
$inspRef = [string]$treeR.sheets[0].tables[0].ref
$xt3ok = ($cap2 -notmatch 'Repair') -and ($rsCount -ge 1) -and ($rsName -eq 'Sales') -and ($rsRows -eq 4) -and ($rsCols -eq 3) -and ($inspRef -eq 'A1:C4')
Add-Check 'xlsx-Xt3' $xt3ok "caption=$cap2 count=$rsCount name=$rsName rows=$rsRows cols=$rsCols inspect=$inspRef"
Release-Com $s2
Close-AllWorkbooks $excel

$delOut = Get-OutPath 'xlsx-table-delete.xlsx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $resizeOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteSheetTable'; id = 'sheet:Sheet1/table:1' })
} 'xlsx-xt-delete' | Out-Null

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:C4' }
} 'xlsx-xt-delete-inspect'
$sheetXml = Get-ZipPartText $delOut 'xl/worksheets/sheet1.xml'
$wb3 = Open-ExcelBook $excel $delOut
$cap3 = [string]$wb3.Windows.Item(1).Caption
$s3 = $wb3.Worksheets.Item('Sheet1')
$delCount = $null
$a1val = $null
try {
    $los3 = $s3.ListObjects
    $delCount = [int]$los3.Count
    Release-Com $los3
    $a1 = $s3.Range('A1')
    try { $a1val = [string]$a1.Value2 } finally { Release-Com $a1 }
} catch {
    Write-Log "Xt4 ListObjects 读失败 $($_.Exception.Message)"
    $delCount = -1
}
$xt4ok = ($cap3 -notmatch 'Repair') -and ($treeD.sheets[0].tableCount -eq 0) -and ($sheetXml -notmatch 'tableParts') -and ($delCount -eq 0) -and ($a1val -eq '产品')
Add-Check 'xlsx-Xt4' $xt4ok "caption=$cap3 inspect=$($treeD.sheets[0].tableCount) xmlHas=$($sheetXml -match 'tableParts') comCount=$delCount a1=$a1val"
Release-Com $s3
Close-AllWorkbooks $excel

$failOut = Get-OutPath 'xlsx-table-bad.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'insertSheetTable'; id = 'sheet:Sheet1' })
} 'xlsx-xt-missing'
$codeEmpty = $null
try { $codeEmpty = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$xt5ok = ($fail.ExitCode -ne 0) -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xt5' $xt5ok "exit=$($fail.ExitCode) code=$codeEmpty wrote=$(Test-Path $failOut)"
