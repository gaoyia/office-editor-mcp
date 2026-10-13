# xlsx 切片器筛选项：docs/checklist-xlsx.md 的 Xsi1–Xsi5。
# 编号 Xsi，不要和 Xsl、Xsn、Xsc、Xtl 撞。不执行筛选。不要 Stop-Process Excel。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.IO.Compression.FileSystem

$S = $global:OoxpStrings

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

function Get-FailCode($raw) {
    $code = $null
    try { $code = ($raw.Stderr | ConvertFrom-Json).error.code } catch { $code = $null }
    return $code
}

function Get-ZipText([string]$xlsx, [string]$entry) {
    $z = [IO.Compression.ZipFile]::OpenRead($xlsx)
    try {
        $e = $z.GetEntry($entry)
        if ($null -eq $e) { return '' }
        $sr = New-Object IO.StreamReader($e.Open())
        try { return $sr.ReadToEnd() }
        finally { $sr.Dispose() }
    }
    finally { $z.Dispose() }
}

function Selected-Of($item) {
    if ($null -eq $item.selected) { return @() }
    return @($item.selected)
}

$hostPath = Get-OutPath 'xlsx-slicer-filter-src.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = $S.saProduct }
                @{ ref = 'B1'; value = '数量' }
                @{ ref = 'C1'; value = '金额' }
                @{ ref = 'A2'; value = $S.saItem1 }
                @{ ref = 'B2'; value = 1 }
                @{ ref = 'C2'; value = 10 }
                @{ ref = 'A3'; value = $S.saItem2 }
                @{ ref = 'B3'; value = 2 }
                @{ ref = 'C3'; value = 20 }
                @{ ref = 'A4'; value = $S.saItem1 }
                @{ ref = 'B4'; value = 3 }
                @{ ref = 'C4'; value = 30 }
            )
        })
    }
} 'xlsx-xsi-create' | Out-Null

$tablePath = Get-OutPath 'xlsx-slicer-filter-table.xlsx'
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
        columns = @($S.saProduct, '数量', '金额')
    })
} 'xlsx-xsi-table' | Out-Null

$tblIns = Get-OutPath 'xlsx-slicer-filter-table-insert.xlsx'
if (Test-Path $tblIns) { Remove-Item -Force $tblIns }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $tablePath.Replace('\', '/')
    output = $tblIns.Replace('\', '/')
    ops = @(@{
        op = 'insertSlicer'
        id = 'sheet:Sheet1'
        source = 'sheet:Sheet1/table:1'
        field = $S.saProduct
        from = 'F2'
    })
} 'xlsx-xsi-table-insert' | Out-Null
$treeTbl = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tblIns.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsi-inspect-table'
$itemTbl = @($treeTbl.sheets[0].slicers) | Select-Object -First 1
$tblId = [string]$itemTbl.id
$selTbl0 = Selected-Of $itemTbl

$pivotHost = Get-OutPath 'xlsx-slicer-filter-pivot-host.xlsx'
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
        rows = @($S.saProduct)
        values = @('金额')
    })
} 'xlsx-xsi-pivot-host' | Out-Null

$ptIns = Get-OutPath 'xlsx-slicer-filter-pivot-insert.xlsx'
if (Test-Path $ptIns) { Remove-Item -Force $ptIns }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $pivotHost.Replace('\', '/')
    output = $ptIns.Replace('\', '/')
    ops = @(@{
        op = 'insertSlicer'
        id = 'sheet:Sheet1'
        source = 'sheet:Sheet1/pivot:1'
        field = $S.saProduct
        from = 'H2'
    })
} 'xlsx-xsi-pivot-insert' | Out-Null
$treePt = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ptIns.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsi-inspect-pivot'
$itemPt = @($treePt.sheets[0].slicers) | Select-Object -First 1
$ptId = [string]$itemPt.id
$selPt0 = Selected-Of $itemPt

$dateSrc = Get-OutPath 'xlsx-slicer-filter-date.xlsx'
if (Test-Path $dateSrc) { Remove-Item -Force $dateSrc }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $dateSrc.Replace('\', '/'); format = 'xlsx'
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
} 'xlsx-xsi-date-create' | Out-Null
$datePt = Get-OutPath 'xlsx-slicer-filter-date-pt.xlsx'
if (Test-Path $datePt) { Remove-Item -Force $datePt }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $dateSrc.Replace('\', '/')
    output = $datePt.Replace('\', '/')
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
} 'xlsx-xsi-date-pt' | Out-Null
$tlIns = Get-OutPath 'xlsx-slicer-filter-timeline.xlsx'
if (Test-Path $tlIns) { Remove-Item -Force $tlIns }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $datePt.Replace('\', '/')
    output = $tlIns.Replace('\', '/')
    ops = @(@{
        op = 'insertSlicer'
        id = 'sheet:Sheet1'
        source = 'sheet:Sheet1/pivot:1'
        field = '日期'
        kind = 'timeline'
        from = 'H2'
    })
} 'xlsx-xsi-timeline-insert' | Out-Null
$treeTl = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tlIns.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsi-inspect-timeline'
$itemTl = @($treeTl.sheets[0].slicers) | Select-Object -First 1
$tlId = [string]$itemTl.id

$excel = Ensure-MsExcel $hostPath
Close-AllWorkbooks $excel
$wbT = Open-ExcelBook $excel $tblIns
$winT = [string]$wbT.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$wbP = Open-ExcelBook $excel $ptIns
$winP = [string]$wbP.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$wbL = Open-ExcelBook $excel $tlIns
$winL = [string]$wbL.Windows.Item(1).Caption
Close-AllWorkbooks $excel

Add-Check 'xlsx-Xsi1' (
    ($selTbl0.Count -eq 0) -and
    ($selPt0 -contains $S.saItem1) -and
    ($selPt0 -contains $S.saItem2) -and
    ($null -eq $itemTl.filter) -and
    ($winT -notmatch 'Repair') -and
    ($winP -notmatch 'Repair') -and
    ($winL -notmatch 'Repair')
) "tblSel=$($selTbl0 -join ',') ptSel=$($selPt0 -join ',') filter=$($itemTl.filter) winT=$winT winP=$winP winL=$winL"

$tblSel = Get-OutPath 'xlsx-slicer-filter-table-selected.xlsx'
if (Test-Path $tblSel) { Remove-Item -Force $tblSel }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $tblIns.Replace('\', '/')
    output = $tblSel.Replace('\', '/')
    ops = @(@{
        op = 'setSlicerFilter'
        id = $tblId
        selected = @($S.saItem2)
    })
} 'xlsx-xsi-table-selected' | Out-Null
$treeTblS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tblSel.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsi-inspect-table-sel'
$itemTblS = @($treeTblS.sheets[0].slicers) | Select-Object -First 1
$selTblS = Selected-Of $itemTblS
$tableXml = Get-ZipText $tblSel 'xl/tables/table1.xml'
$wbTS = Open-ExcelBook $excel $tblSel
$winTS = [string]$wbTS.Windows.Item(1).Caption
Close-AllWorkbooks $excel
Add-Check 'xlsx-Xsi2' (
    ($selTblS.Count -eq 1) -and
    ($selTblS[0] -eq $S.saItem2) -and
    ($tableXml -match [regex]::Escape($S.saItem2)) -and
    ($tableXml -match 'filterColumn') -and
    ($winTS -notmatch 'Repair')
) "sel=$($selTblS -join ',') xmlHas=$($tableXml -match 'filterColumn') win=$winTS"

$ptSel = Get-OutPath 'xlsx-slicer-filter-pivot-selected.xlsx'
if (Test-Path $ptSel) { Remove-Item -Force $ptSel }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $ptIns.Replace('\', '/')
    output = $ptSel.Replace('\', '/')
    ops = @(@{
        op = 'setSlicerFilter'
        id = $ptId
        selected = @($S.saItem2)
    })
} 'xlsx-xsi-pivot-selected' | Out-Null
$treePtS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ptSel.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsi-inspect-pivot-sel'
$itemPtS = @($treePtS.sheets[0].slicers) | Select-Object -First 1
$selPtS = Selected-Of $itemPtS
$wbPS = Open-ExcelBook $excel $ptSel
$winPS = [string]$wbPS.Windows.Item(1).Caption
Close-AllWorkbooks $excel
Add-Check 'xlsx-Xsi3' (
    ($selPtS.Count -eq 1) -and
    ($selPtS[0] -eq $S.saItem2) -and
    ($winPS -notmatch 'Repair')
) "sel=$($selPtS -join ',') win=$winPS"

$tlSet = Get-OutPath 'xlsx-slicer-filter-timeline-set.xlsx'
if (Test-Path $tlSet) { Remove-Item -Force $tlSet }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $tlIns.Replace('\', '/')
    output = $tlSet.Replace('\', '/')
    ops = @(@{
        op = 'setSlicerFilter'
        id = $tlId
        filter = @{
            filterType = 'dateBetween'
            startDate = '2023-01-01T00:00:00'
            endDate = '2023-03-01T00:00:00'
        }
    })
} 'xlsx-xsi-timeline-set' | Out-Null
$treeTlS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tlSet.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsi-inspect-timeline-set'
$itemTlS = @($treeTlS.sheets[0].slicers) | Select-Object -First 1
$tlClr = Get-OutPath 'xlsx-slicer-filter-timeline-null.xlsx'
if (Test-Path $tlClr) { Remove-Item -Force $tlClr }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $tlSet.Replace('\', '/')
    output = $tlClr.Replace('\', '/')
    ops = @(@{
        op = 'setSlicerFilter'
        id = $tlId
        filter = $null
    })
} 'xlsx-xsi-timeline-null' | Out-Null
$treeTlC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tlClr.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsi-inspect-timeline-null'
$itemTlC = @($treeTlC.sheets[0].slicers) | Select-Object -First 1
$wbTlS = Open-ExcelBook $excel $tlSet
$winTlS = [string]$wbTlS.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$wbTlC = Open-ExcelBook $excel $tlClr
$winTlC = [string]$wbTlC.Windows.Item(1).Caption
Close-AllWorkbooks $excel
Add-Check 'xlsx-Xsi4' (
    ([string]$itemTlS.filter.filterType -eq 'dateBetween') -and
    (([datetime]$itemTlS.filter.startDate) -eq ([datetime]'2023-01-01')) -and
    (([datetime]$itemTlS.filter.endDate) -eq ([datetime]'2023-03-01')) -and
    ($null -eq $itemTlC.filter) -and
    ($winTlS -notmatch 'Repair') -and
    ($winTlC -notmatch 'Repair')
) "type=$($itemTlS.filter.filterType) start=$($itemTlS.filter.startDate) end=$($itemTlS.filter.endDate) cleared=$($null -eq $itemTlC.filter) winS=$winTlS winC=$winTlC"

$failEmpty = Get-OutPath 'xlsx-slicer-filter-fail-empty.xlsx'
$rawEmpty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $tblIns.Replace('\', '/')
    output = $failEmpty.Replace('\', '/')
    ops = @(@{ op = 'setSlicerFilter'; id = $tblId })
} 'xlsx-xsi-fail-empty'
$failName = Get-OutPath 'xlsx-slicer-filter-fail-name.xlsx'
$rawName = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $tblIns.Replace('\', '/')
    output = $failName.Replace('\', '/')
    ops = @(@{ op = 'setSlicerFilter'; id = $tblId; selected = @('不存在') })
} 'xlsx-xsi-fail-name'
$failId = Get-OutPath 'xlsx-slicer-filter-fail-id.xlsx'
$rawId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $tblIns.Replace('\', '/')
    output = $failId.Replace('\', '/')
    ops = @(@{ op = 'setSlicerFilter'; id = 'sheet:Sheet1/pic:1'; selected = @($S.saItem2) })
} 'xlsx-xsi-fail-id'
Add-Check 'xlsx-Xsi5' (
    ((Get-FailCode $rawEmpty) -eq 'InvalidField') -and
    ((Get-FailCode $rawName) -eq 'InvalidField') -and
    ((Get-FailCode $rawId) -eq 'InvalidId') -and
    (-not (Test-Path $failEmpty)) -and
    (-not (Test-Path $failName)) -and
    (-not (Test-Path $failId))
) "empty=$(Get-FailCode $rawEmpty) name=$(Get-FailCode $rawName) id=$(Get-FailCode $rawId)"
