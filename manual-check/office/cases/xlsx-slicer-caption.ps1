# xlsx 切片器视图：docs/checklist-xlsx.md 的 Xsc1–Xsc5。
# 编号 Xsc，不要和 Xsl、Xsn、Xtl 撞。不执行筛选。不要 Stop-Process Excel。

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

function Get-FailCode($raw) {
    $code = $null
    try { $code = ($raw.Stderr | ConvertFrom-Json).error.code } catch { $code = $null }
    return $code
}

function Get-FirstSlicerCaption($wb) {
    $cap = $null
    $cols = $null
    $caches = $null
    try { $caches = $wb.SlicerCaches } catch { }
    if ($null -eq $caches) { return @{ Caption = $null; Cols = $null } }
    try {
        if ([int]$caches.Count -ge 1) {
            $sc = $null
            $ss = $null
            $s = $null
            try {
                $sc = $caches.Item(1)
                $ss = $sc.Slicers
                if ($null -ne $ss -and [int]$ss.Count -ge 1) {
                    $s = $ss.Item(1)
                    try { $cap = [string]$s.Caption } catch { }
                    try { $cols = [int]$s.NumberOfColumns } catch { }
                }
            } finally {
                Release-Com $s
                Release-Com $ss
                Release-Com $sc
            }
        }
    } finally { Release-Com $caches }
    return @{ Caption = $cap; Cols = $cols }
}

$hostPath = Get-OutPath 'xlsx-slicer-caption-src.xlsx'
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
} 'xlsx-xsc-create' | Out-Null

$tablePath = Get-OutPath 'xlsx-slicer-caption-table.xlsx'
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
} 'xlsx-xsc-table' | Out-Null

$insOut = Get-OutPath 'xlsx-slicer-caption-insert.xlsx'
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
        caption = '类别'
    })
} 'xlsx-xsc-insert' | Out-Null
$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsc-inspect-insert'
$itemI = @($treeI.sheets[0].slicers) | Select-Object -First 1
$id = [string]$itemI.id
$excel = Ensure-MsExcel $hostPath
Close-AllWorkbooks $excel
$wbI = Open-ExcelBook $excel $insOut
$winI = [string]$wbI.Windows.Item(1).Caption
$comI = Get-FirstSlicerCaption $wbI
Add-Check 'xlsx-Xsc1' (
    ([string]$itemI.caption -eq '类别') -and
    ([int]$itemI.rowHeight -eq 381000) -and
    ([int]$itemI.columnCount -eq 1) -and
    ([bool]$itemI.showCaption -eq $true) -and
    ([int]$itemI.startItem -eq 0) -and
    ([bool]$itemI.lockedPosition -eq $false) -and
    ($null -eq $itemI.style) -and
    ($winI -notmatch 'Repair')
) "caption=$($itemI.caption) rowH=$($itemI.rowHeight) cols=$($itemI.columnCount) win=$winI com=$($comI.Caption)"
Close-AllWorkbooks $excel

$viewed = Get-OutPath 'xlsx-slicer-caption-view.xlsx'
if (Test-Path $viewed) { Remove-Item -Force $viewed }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $viewed.Replace('\', '/')
    ops = @(@{
        op = 'setSlicerView'
        id = $id
        caption = '产品线'
        columnCount = 2
        showCaption = $false
    })
} 'xlsx-xsc-view' | Out-Null
$treeV = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $viewed.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsc-inspect-view'
$itemV = @($treeV.sheets[0].slicers) | Select-Object -First 1
$wbV = Open-ExcelBook $excel $viewed
$winV = [string]$wbV.Windows.Item(1).Caption
$comV = Get-FirstSlicerCaption $wbV
Add-Check 'xlsx-Xsc2' (
    ([string]$itemV.caption -eq '产品线') -and
    ([int]$itemV.columnCount -eq 2) -and
    ([bool]$itemV.showCaption -eq $false) -and
    ($winV -notmatch 'Repair') -and
    ([string]$comV.Caption -eq '产品线')
) "caption=$($itemV.caption) cols=$($itemV.columnCount) show=$($itemV.showCaption) com=$($comV.Caption) win=$winV"
Close-AllWorkbooks $excel

$styled = Get-OutPath 'xlsx-slicer-caption-style.xlsx'
if (Test-Path $styled) { Remove-Item -Force $styled }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $viewed.Replace('\', '/')
    output = $styled.Replace('\', '/')
    ops = @(@{ op = 'setSlicerView'; id = $id; style = 'SlicerStyleLight1' })
} 'xlsx-xsc-style' | Out-Null
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $styled.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsc-inspect-style'
$itemS = @($treeS.sheets[0].slicers) | Select-Object -First 1
$cleared = Get-OutPath 'xlsx-slicer-caption-style-null.xlsx'
if (Test-Path $cleared) { Remove-Item -Force $cleared }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $styled.Replace('\', '/')
    output = $cleared.Replace('\', '/')
    ops = @(@{ op = 'setSlicerView'; id = $id; style = $null })
} 'xlsx-xsc-style-null' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cleared.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsc-inspect-style-null'
$itemC = @($treeC.sheets[0].slicers) | Select-Object -First 1
$wbS = Open-ExcelBook $excel $styled
$winS = [string]$wbS.Windows.Item(1).Caption
Close-AllWorkbooks $excel
Add-Check 'xlsx-Xsc3' (
    ([string]$itemS.style -eq 'SlicerStyleLight1') -and
    ($null -eq $itemC.style) -and
    ($winS -notmatch 'Repair')
) "style=$($itemS.style) cleared=$($itemC.style) win=$winS"

$dateSrc = Get-OutPath 'xlsx-slicer-caption-date.xlsx'
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
} 'xlsx-xsc-date-create' | Out-Null
$datePt = Get-OutPath 'xlsx-slicer-caption-date-pt.xlsx'
if (Test-Path $datePt) { Remove-Item -Force $datePt }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $dateSrc.Replace('\', '/')
    output = $datePt.Replace('\', '/')
    ops = @(@{
        op = 'insertPivotTable'
        id = 'sheet:Sheet1'
        source = @{ sheet = 'Sheet1'; ref = 'A1:B4' }
        location = 'E3'
        name = 'DatePivot'
        rows = @('日期')
        values = @('数量')
    })
} 'xlsx-xsc-date-pt' | Out-Null
$tlIns = Get-OutPath 'xlsx-slicer-caption-timeline.xlsx'
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
} 'xlsx-xsc-timeline-insert' | Out-Null
$treeTl = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tlIns.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsc-inspect-timeline-ins'
$itemTl = @($treeTl.sheets[0].slicers) | Select-Object -First 1
$tlId = [string]$itemTl.id
$tlView = Get-OutPath 'xlsx-slicer-caption-timeline-view.xlsx'
if (Test-Path $tlView) { Remove-Item -Force $tlView }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $tlIns.Replace('\', '/')
    output = $tlView.Replace('\', '/')
    ops = @(@{
        op = 'setSlicerView'
        id = $tlId
        caption = '时间'
        showHeader = $false
    })
} 'xlsx-xsc-timeline-view' | Out-Null
$treeTlV = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tlView.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsc-inspect-timeline'
$itemTlV = @($treeTlV.sheets[0].slicers) | Select-Object -First 1
$wbTl = Open-ExcelBook $excel $tlView
$winTl = [string]$wbTl.Windows.Item(1).Caption
Add-Check 'xlsx-Xsc4' (
    ([string]$itemTlV.caption -eq '时间') -and
    ([bool]$itemTlV.showHeader -eq $false) -and
    ([string]$itemTlV.sourceKind -eq 'timeline') -and
    ($winTl -notmatch 'Repair')
) "caption=$($itemTlV.caption) header=$($itemTlV.showHeader) kind=$($itemTlV.sourceKind) win=$winTl"
Close-AllWorkbooks $excel

$failEmpty = Get-OutPath 'xlsx-slicer-caption-fail-empty.xlsx'
$rawEmpty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $failEmpty.Replace('\', '/')
    ops = @(@{ op = 'setSlicerView'; id = $id })
} 'xlsx-xsc-fail-empty'
$failKind = Get-OutPath 'xlsx-slicer-caption-fail-kind.xlsx'
$rawKind = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $failKind.Replace('\', '/')
    ops = @(@{ op = 'setSlicerView'; id = $id; showHeader = $false })
} 'xlsx-xsc-fail-kind'
$failId = Get-OutPath 'xlsx-slicer-caption-fail-id.xlsx'
$rawId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $failId.Replace('\', '/')
    ops = @(@{ op = 'setSlicerView'; id = 'sheet:Sheet1/pic:1'; caption = 'x' })
} 'xlsx-xsc-fail-id'
Add-Check 'xlsx-Xsc5' (
    ((Get-FailCode $rawEmpty) -eq 'InvalidField') -and
    ((Get-FailCode $rawKind) -eq 'InvalidField') -and
    ((Get-FailCode $rawId) -eq 'InvalidId') -and
    (-not (Test-Path $failEmpty)) -and
    (-not (Test-Path $failKind)) -and
    (-not (Test-Path $failId))
) "empty=$(Get-FailCode $rawEmpty) kind=$(Get-FailCode $rawKind) id=$(Get-FailCode $rawId)"
