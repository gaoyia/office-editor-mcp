# xlsx 切片器：docs/checklist-xlsx.md 的 Xsl1–Xsl5。
# 用 create 产物再 apply 出表格 / 透视表。不执行筛选。

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

function Get-SlicerCacheCount($wb) {
    $n = 0
    $caches = $null
    try { $caches = $wb.SlicerCaches } catch { }
    if ($null -eq $caches) { return 0 }
    try {
        try { $n = [int]$caches.Count } catch { $n = 0 }
    } finally { Release-Com $caches }
    return $n
}

$hostPath = Get-OutPath 'xlsx-slicers-host.xlsx'
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
            )
        })
    }
} 'xlsx-xsl-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xsl-inspect-bare'
$sheet0 = $tree0.sheets[0]
$noList = -not ($sheet0.PSObject.Properties.Name -contains 'slicers')
$treeList = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsl-inspect-list'
$listN = 0
if ($treeList.sheets[0].slicers) { $listN = @($treeList.sheets[0].slicers).Count }
$xsl1ok = ([int]$tree0.slicerCount -eq 0) `
    -and ([int]$sheet0.slicerCount -eq 0) `
    -and $noList `
    -and ($listN -eq 0)
Add-Check 'xlsx-Xsl1' $xsl1ok "wb=$($tree0.slicerCount) sheet=$($sheet0.slicerCount) noList=$noList listed=$listN"

$tablePath = Get-OutPath 'xlsx-slicers-table.xlsx'
if (Test-Path $tablePath) { Remove-Item -Force $tablePath }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $tablePath.Replace('\', '/')
    ops = @(@{
        op = 'insertSheetTable'
        id = 'sheet:Sheet1'
        ref = 'A1:C3'
        name = 'Sales'
        columns = @('产品', '数量', '金额')
    })
} 'xlsx-xsl-table' | Out-Null

$insOut = Get-OutPath 'xlsx-slicers-insert.xlsx'
if (Test-Path $insOut) { Remove-Item -Force $insOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $tablePath.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{
        op = 'insertSlicer'
        id = 'sheet:Sheet1'
        source = 'sheet:Sheet1/table:1'
        field = '产品'
        from = 'F2'
    })
} 'xlsx-xsl-insert-table' | Out-Null

$excel = Ensure-MsExcel $hostPath
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $insOut
$cap = [string]$wb.Windows.Item(1).Caption
$comN = Get-SlicerCacheCount $wb
Close-AllWorkbooks $excel
$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsl-inspect-inserted'
$item = $null
if ($treeI.sheets[0].slicers) { $item = @($treeI.sheets[0].slicers) | Select-Object -First 1 }
$idOk = $false
if ($item) { $idOk = ([string]$item.id).StartsWith('sheet:Sheet1/slicer:') }
$xsl2ok = ($cap -notmatch 'Repair') `
    -and ($comN -ge 1) `
    -and ($null -ne $item) `
    -and $idOk `
    -and ([string]$item.sourceKind -eq 'table')
Add-Check 'xlsx-Xsl2' $xsl2ok "caption=$cap com=$comN id=$($item.id) kind=$($item.sourceKind)"

$pivotHost = Get-OutPath 'xlsx-slicers-pivot-host.xlsx'
if (Test-Path $pivotHost) { Remove-Item -Force $pivotHost }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $pivotHost.Replace('\', '/')
    ops = @(@{
        op = 'insertPivotTable'
        id = 'sheet:Sheet1'
        source = @{ sheet = 'Sheet1'; ref = 'A1:C4' }
        location = 'E3'
        name = 'SalesPivot'
        rows = @('产品')
        values = @('金额')
    })
} 'xlsx-xsl-pivot' | Out-Null

$pivotOut = Get-OutPath 'xlsx-slicers-pivot.xlsx'
if (Test-Path $pivotOut) { Remove-Item -Force $pivotOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $pivotHost.Replace('\', '/')
    output = $pivotOut.Replace('\', '/')
    ops = @(@{
        op = 'insertSlicer'
        id = 'sheet:Sheet1'
        source = 'sheet:Sheet1/pivot:1'
        field = '产品'
        from = 'H2'
    })
} 'xlsx-xsl-insert-pivot' | Out-Null

Close-AllWorkbooks $excel
$wbP = Open-ExcelBook $excel $pivotOut
$capP = [string]$wbP.Windows.Item(1).Caption
$comP = Get-SlicerCacheCount $wbP
Close-AllWorkbooks $excel
$treeP = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $pivotOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsl-inspect-pivot'
$itemP = $null
if ($treeP.sheets[0].slicers) { $itemP = @($treeP.sheets[0].slicers) | Select-Object -First 1 }
$xsl3ok = ($capP -notmatch 'Repair') `
    -and ($comP -ge 1) `
    -and ($null -ne $itemP) `
    -and ([string]$itemP.sourceKind -eq 'pivot')
Add-Check 'xlsx-Xsl3' $xsl3ok "caption=$capP com=$comP kind=$($itemP.sourceKind)"

$slicerId = if ($item) { [string]$item.id } else { '' }
$delOut = Get-OutPath 'xlsx-slicers-delete.xlsx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteSlicer'; id = $slicerId })
} 'xlsx-xsl-delete' | Out-Null
Close-AllWorkbooks $excel
$wbD = Open-ExcelBook $excel $delOut
$capD = [string]$wbD.Windows.Item(1).Caption
$comD = Get-SlicerCacheCount $wbD
Close-AllWorkbooks $excel
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsl-inspect-deleted'
$xsl4ok = ($capD -notmatch 'Repair') `
    -and ([int]$treeD.slicerCount -eq 0) `
    -and ($comD -eq 0)
Add-Check 'xlsx-Xsl4' $xsl4ok "caption=$capD count=$($treeD.slicerCount) com=$comD"

$failOut = Get-OutPath 'xlsx-slicers-bad.xlsx'
$failFrom = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $tablePath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'insertSlicer'
        id = 'sheet:Sheet1'
        source = 'sheet:Sheet1/table:1'
        field = '产品'
    })
} 'xlsx-xsl-missing-from'
$codeFrom = $null
try { $codeFrom = ($failFrom.Stderr | ConvertFrom-Json).error.code } catch {}
$failSlide = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $tablePath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'insertSlicer'
        id = 'slide:1'
        source = 'sheet:Sheet1/table:1'
        field = '产品'
        from = 'F2'
    })
} 'xlsx-xsl-slide-id'
$codeSlide = $null
try { $codeSlide = ($failSlide.Stderr | ConvertFrom-Json).error.code } catch {}
$failField = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $tablePath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'insertSlicer'
        id = 'sheet:Sheet1'
        source = 'sheet:Sheet1/table:1'
        field = '不存在的列'
        from = 'F2'
    })
} 'xlsx-xsl-bad-field'
$codeField = $null
try { $codeField = ($failField.Stderr | ConvertFrom-Json).error.code } catch {}
$xsl5ok = ($failFrom.ExitCode -ne 0) -and ($codeFrom -eq 'InvalidField') `
    -and ($failSlide.ExitCode -ne 0) -and ($codeSlide -eq 'InvalidId') `
    -and ($failField.ExitCode -ne 0) -and ($codeField -eq 'InvalidField') `
    -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xsl5' $xsl5ok "from=$codeFrom slide=$codeSlide field=$codeField wrote=$(Test-Path $failOut)"
