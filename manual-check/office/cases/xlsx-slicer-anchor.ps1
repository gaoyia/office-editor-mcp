# xlsx 切片器锚点：docs/checklist-xlsx.md 的 Xsn1–Xsn6。
# 编号 Xsn，不要和 Xsl、Xsa、Xtl、Xsk 撞。不执行筛选。不要 Stop-Process Excel。

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

function Get-FirstSlicerSize($wb) {
    $w = $null
    $h = $null
    $caches = $null
    try { $caches = $wb.SlicerCaches } catch { }
    if ($null -eq $caches) { return @{ W = $null; H = $null } }
    try {
        if ([int]$caches.Count -ge 1) {
            $sc = $caches.Item(1)
            $ss = $null
            try {
                $ss = $sc.Slicers
                if ($null -ne $ss -and [int]$ss.Count -ge 1) {
                    $s = $ss.Item(1)
                    try {
                        $w = [math]::Round([double]$s.Width, 2)
                        $h = [math]::Round([double]$s.Height, 2)
                    } finally { Release-Com $s }
                }
            } finally {
                Release-Com $ss
                Release-Com $sc
            }
        }
    } finally { Release-Com $caches }
    return @{ W = $w; H = $h }
}

$hostPath = Get-OutPath 'xlsx-slicer-anchor-src.xlsx'
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
} 'xlsx-xsn-create' | Out-Null

$tablePath = Get-OutPath 'xlsx-slicer-anchor-table.xlsx'
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
} 'xlsx-xsn-table' | Out-Null

$insOut = Get-OutPath 'xlsx-slicer-anchor-insert.xlsx'
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
} 'xlsx-xsn-insert' | Out-Null

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsn-inspect-insert'
$item = @($treeI.sheets[0].slicers) | Select-Object -First 1
$id = [string]$item.id

$excel = Ensure-MsExcel $hostPath
Close-AllWorkbooks $excel

$moved = Get-OutPath 'xlsx-slicer-anchor-moved.xlsx'
if (Test-Path $moved) { Remove-Item -Force $moved }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $moved.Replace('\', '/')
    ops = @(@{ op = 'setSlicerAnchor'; id = $id; from = 'G4' })
} 'xlsx-xsn-anchor' | Out-Null
$treeM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $moved.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsn-inspect-anchor'
$itemM = @($treeM.sheets[0].slicers) | Select-Object -First 1
$wbM = Open-ExcelBook $excel $moved
$capM = [string]$wbM.Windows.Item(1).Caption
Add-Check 'xlsx-Xsn1' (
    ([string]$itemM.from -eq 'G4') -and ($capM -notmatch 'Repair')
) "from=$($itemM.from) caption=$capM"
Close-AllWorkbooks $excel

$sized = Get-OutPath 'xlsx-slicer-anchor-sized.xlsx'
if (Test-Path $sized) { Remove-Item -Force $sized }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $moved.Replace('\', '/')
    output = $sized.Replace('\', '/')
    ops = @(@{ op = 'setSlicerSize'; id = $id; to = 'H12' })
} 'xlsx-xsn-size' | Out-Null
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sized.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsn-inspect-size'
$itemS = @($treeS.sheets[0].slicers) | Select-Object -First 1
$wbS = Open-ExcelBook $excel $sized
$capS = [string]$wbS.Windows.Item(1).Caption
Add-Check 'xlsx-Xsn2' (
    ([string]$itemS.to -eq 'H12') -and ([string]$itemS.from -eq 'G4') -and ($capS -notmatch 'Repair')
) "to=$($itemS.to) from=$($itemS.from) caption=$capS"
Close-AllWorkbooks $excel

$one = Get-OutPath 'xlsx-slicer-anchor-one.xlsx'
if (Test-Path $one) { Remove-Item -Force $one }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $one.Replace('\', '/')
    ops = @(@{
        op = 'setSlicerAnchor'
        id = $id
        anchor = 'oneCell'
        cx = 3657600
        cy = 1828800
    })
} 'xlsx-xsn-one' | Out-Null
$treeO = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $one.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsn-inspect-one'
$itemO = @($treeO.sheets[0].slicers) | Select-Object -First 1
$wbO = Open-ExcelBook $excel $one
$capO = [string]$wbO.Windows.Item(1).Caption
$sz = Get-FirstSlicerSize $wbO
Add-Check 'xlsx-Xsn3' (
    ([string]$itemO.anchor -eq 'oneCell') -and
    ([int]$itemO.cx -eq 3657600) -and ([int]$itemO.cy -eq 1828800) -and
    ($capO -notmatch 'Repair') -and
    ($null -ne $sz.W) -and ([math]::Abs($sz.W - 288) -lt 2) -and
    ($null -ne $sz.H) -and ([math]::Abs($sz.H - 144) -lt 2)
) "anchor=$($itemO.anchor) cx=$($itemO.cx) cy=$($itemO.cy) caption=$capO w=$($sz.W) h=$($sz.H)"
Close-AllWorkbooks $excel

$dateSrc = Get-OutPath 'xlsx-slicer-anchor-date.xlsx'
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
} 'xlsx-xsn-date-create' | Out-Null
$datePt = Get-OutPath 'xlsx-slicer-anchor-date-pt.xlsx'
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
} 'xlsx-xsn-date-pivot' | Out-Null
$tlIns = Get-OutPath 'xlsx-slicer-anchor-timeline.xlsx'
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
} 'xlsx-xsn-timeline-insert' | Out-Null
$treeTl = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tlIns.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsn-inspect-timeline-ins'
$itemTl = @($treeTl.sheets[0].slicers) | Select-Object -First 1
$tlId = [string]$itemTl.id
$tlMoved = Get-OutPath 'xlsx-slicer-anchor-timeline-moved.xlsx'
if (Test-Path $tlMoved) { Remove-Item -Force $tlMoved }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $tlIns.Replace('\', '/')
    output = $tlMoved.Replace('\', '/')
    ops = @(@{ op = 'setSlicerAnchor'; id = $tlId; from = 'I5' })
} 'xlsx-xsn-timeline-anchor' | Out-Null
$treeTlM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tlMoved.Replace('\', '/'); format = 'xlsx'
    scope = @{ slicers = $true }
} 'xlsx-xsn-inspect-timeline'
$itemTlM = @($treeTlM.sheets[0].slicers) | Select-Object -First 1
$wbTl = Open-ExcelBook $excel $tlMoved
$capTl = [string]$wbTl.Windows.Item(1).Caption
Add-Check 'xlsx-Xsn4' (
    ([string]$itemTlM.from -eq 'I5') -and
    ([string]$itemTlM.sourceKind -eq 'timeline') -and
    ($capTl -notmatch 'Repair')
) "from=$($itemTlM.from) kind=$($itemTlM.sourceKind) caption=$capTl"
Close-AllWorkbooks $excel

$failPic = Get-OutPath 'xlsx-slicer-anchor-fail-pic.xlsx'
$rawPic = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $failPic.Replace('\', '/')
    ops = @(@{ op = 'setPictureSize'; id = $id; cx = 1000; cy = 1000 })
} 'xlsx-xsn-fail-pic'
Add-Check 'xlsx-Xsn5' (
    ((Get-FailCode $rawPic) -eq 'InvalidId') -and (-not (Test-Path $failPic))
) "pic=$(Get-FailCode $rawPic) wrote=$(Test-Path $failPic)"

$failCx = Get-OutPath 'xlsx-slicer-anchor-fail-cx.xlsx'
$rawCx = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $failCx.Replace('\', '/')
    ops = @(@{ op = 'setSlicerSize'; id = $id; cx = 1000 })
} 'xlsx-xsn-fail-cx'
$failId = Get-OutPath 'xlsx-slicer-anchor-fail-id.xlsx'
$rawId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $failId.Replace('\', '/')
    ops = @(@{ op = 'setSlicerSize'; id = 'sheet:Sheet1/pic:1'; to = 'H12' })
} 'xlsx-xsn-fail-id'
Add-Check 'xlsx-Xsn6' (
    ((Get-FailCode $rawCx) -eq 'InvalidField') -and
    ((Get-FailCode $rawId) -eq 'InvalidId') -and
    (-not (Test-Path $failCx)) -and (-not (Test-Path $failId))
) "cx=$(Get-FailCode $rawCx) id=$(Get-FailCode $rawId)"
