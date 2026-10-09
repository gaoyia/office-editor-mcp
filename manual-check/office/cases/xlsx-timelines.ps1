# xlsx 时间线切片器：docs/checklist-xlsx.md 的 Xtl1–Xtl5。
# 用 create 产物再 apply 出带日期列的透视表。不执行筛选。

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

function Get-FirstSlicerCacheType($wb) {
    $t = -1
    $caches = $null
    try { $caches = $wb.SlicerCaches } catch { return $t }
    try {
        if ([int]$caches.Count -ge 1) {
            $c = $null
            try {
                $c = $caches.Item(1)
                $t = [int]$c.SlicerCacheType
            } catch {
                Write-Log "SlicerCacheType $($_.Exception.Message)"
            } finally { Release-Com $c }
        }
    } finally { Release-Com $caches }
    return $t
}

$hostPath = Get-OutPath 'xlsx-timelines-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = '日期' }
                @{ ref = 'B1'; value = '数量' }
                @{ ref = 'A2'; value = 44927 }
                @{ ref = 'B2'; value = 1 }
                @{ ref = 'A3'; value = 44958 }
                @{ ref = 'B3'; value = 2 }
                @{ ref = 'A4'; value = 44986 }
                @{ ref = 'B4'; value = 3 }
            )
        })
    }
} 'xlsx-xtl-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xtl-inspect-bare'
$sheet0 = $tree0.sheets[0]
$noList = -not ($sheet0.PSObject.Properties.Name -contains 'slicers')
$treeList = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xtl-inspect-list'
$listN = 0
if ($treeList.sheets[0].slicers) { $listN = @($treeList.sheets[0].slicers).Count }
$xtl1ok = ([int]$tree0.slicerCount -eq 0) `
    -and ([int]$sheet0.slicerCount -eq 0) `
    -and $noList `
    -and ($listN -eq 0)
Add-Check 'xlsx-Xtl1' $xtl1ok "count=$($tree0.slicerCount) list=$listN noList=$noList"

$dated = Get-OutPath 'xlsx-timelines-dated.xlsx'
if (Test-Path $dated) { Remove-Item -Force $dated }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $dated.Replace('\', '/')
    ops = @(
        @{ op = 'setCellStyle'; id = 'cell:Sheet1!A2'; numFmt = 'yyyy-mm-dd' }
        @{ op = 'setCellStyle'; id = 'cell:Sheet1!A3'; numFmt = 'yyyy-mm-dd' }
        @{ op = 'setCellStyle'; id = 'cell:Sheet1!A4'; numFmt = 'yyyy-mm-dd' }
        @{
            op = 'insertPivotTable'
            id = 'sheet:Sheet1'
            source = @{ sheet = 'Sheet1'; ref = 'A1:B4' }
            location = 'E3'
            name = 'DatePivot'
            rows = @('日期')
            values = @('数量')
        }
    )
} 'xlsx-xtl-pivot' | Out-Null

$insOut = Get-OutPath 'xlsx-timelines-insert.xlsx'
if (Test-Path $insOut) { Remove-Item -Force $insOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $dated.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{
        op = 'insertSlicer'
        id = 'sheet:Sheet1'
        source = 'sheet:Sheet1/pivot:1'
        field = '日期'
        kind = 'timeline'
        from = 'H2'
    })
} 'xlsx-xtl-insert' | Out-Null

$excel = Ensure-MsExcel $hostPath
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $insOut
$cap = [string]$wb.Windows.Item(1).Caption
$comN = Get-SlicerCacheCount $wb
$comT = Get-FirstSlicerCacheType $wb
Close-AllWorkbooks $excel
$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xtl-inspect-inserted'
$item = $null
if ($treeI.sheets[0].slicers) { $item = @($treeI.sheets[0].slicers) | Select-Object -First 1 }
$idOk = $false
if ($item) { $idOk = ([string]$item.id).StartsWith('sheet:Sheet1/slicer:') }
$xtl2ok = ($cap -notmatch 'Repair') `
    -and ($comN -ge 1) `
    -and ($comT -eq 2) `
    -and ($null -ne $item) `
    -and $idOk `
    -and ([string]$item.sourceKind -eq 'timeline')
Add-Check 'xlsx-Xtl2' $xtl2ok "caption=$cap com=$comN type=$comT id=$($item.id) kind=$($item.sourceKind)"

$tlXml = Get-ZipPartText $insOut 'xl/timelines/timeline1.xml'
$cacheXml = Get-ZipPartText $insOut 'xl/timelineCaches/timelineCache1.xml'
$drawXml = Get-ZipPartText $insOut 'xl/drawings/drawing1.xml'
$slicerPart = Get-ZipPartText $insOut 'xl/slicers/slicer1.xml'
$slicerCachePart = Get-ZipPartText $insOut 'xl/slicerCaches/slicerCache1.xml'
$xtl3ok = ($null -ne $tlXml) `
    -and ($null -ne $cacheXml) `
    -and ($null -eq $slicerPart) `
    -and ($null -eq $slicerCachePart) `
    -and ($drawXml -match 'timeslicer')
Add-Check 'xlsx-Xtl3' $xtl3ok "timeline=$($null -ne $tlXml) cache=$($null -ne $cacheXml) draw=$($drawXml -match 'timeslicer')"

$slicerId = if ($item) { [string]$item.id } else { '' }
$delOut = Get-OutPath 'xlsx-timelines-delete.xlsx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteSlicer'; id = $slicerId })
} 'xlsx-xtl-delete' | Out-Null
Close-AllWorkbooks $excel
$wbD = Open-ExcelBook $excel $delOut
$capD = [string]$wbD.Windows.Item(1).Caption
$comD = Get-SlicerCacheCount $wbD
Close-AllWorkbooks $excel
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xtl-inspect-deleted'
$delTl = Get-ZipPartText $delOut 'xl/timelines/timeline1.xml'
$xtl4ok = ($capD -notmatch 'Repair') `
    -and ([int]$treeD.slicerCount -eq 0) `
    -and ($comD -eq 0) `
    -and ($null -eq $delTl)
Add-Check 'xlsx-Xtl4' $xtl4ok "caption=$capD count=$($treeD.slicerCount) com=$comD"

$tablePath = Get-OutPath 'xlsx-timelines-table.xlsx'
if (Test-Path $tablePath) { Remove-Item -Force $tablePath }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $tablePath.Replace('\', '/')
    ops = @(@{
        op = 'insertSheetTable'
        id = 'sheet:Sheet1'
        ref = 'A1:B4'
        name = 'Dates'
        columns = @('日期', '数量')
    })
} 'xlsx-xtl-table' | Out-Null

$failOut = Get-OutPath 'xlsx-timelines-bad.xlsx'
$failTable = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $tablePath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'insertSlicer'
        id = 'sheet:Sheet1'
        source = 'sheet:Sheet1/table:1'
        field = '日期'
        kind = 'timeline'
        from = 'F2'
    })
} 'xlsx-xtl-table-source'
$codeTable = $null
try { $codeTable = ($failTable.Stderr | ConvertFrom-Json).error.code } catch {}
$failFrom = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $dated.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'insertSlicer'
        id = 'sheet:Sheet1'
        source = 'sheet:Sheet1/pivot:1'
        field = '日期'
        kind = 'timeline'
    })
} 'xlsx-xtl-missing-from'
$codeFrom = $null
try { $codeFrom = ($failFrom.Stderr | ConvertFrom-Json).error.code } catch {}
$failSlide = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $dated.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'insertSlicer'
        id = 'slide:1'
        source = 'sheet:Sheet1/pivot:1'
        field = '日期'
        kind = 'timeline'
        from = 'H2'
    })
} 'xlsx-xtl-slide-id'
$codeSlide = $null
try { $codeSlide = ($failSlide.Stderr | ConvertFrom-Json).error.code } catch {}
$xtl5ok = ($failTable.ExitCode -ne 0) -and ($codeTable -eq 'InvalidField') `
    -and ($failFrom.ExitCode -ne 0) -and ($codeFrom -eq 'InvalidField') `
    -and ($failSlide.ExitCode -ne 0) -and ($codeSlide -eq 'InvalidId') `
    -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xtl5' $xtl5ok "table=$codeTable from=$codeFrom slide=$codeSlide wrote=$(Test-Path $failOut)"
