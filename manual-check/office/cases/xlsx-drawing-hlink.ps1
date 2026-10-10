# xlsx 页眉 / 图表工作表 drawing 与 DrawingML 链：docs/checklist-xlsx.md 的 Xdh1–Xdh5。
# 用 create 产物再 apply。不编辑 VML。

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

function Read-ShapeHyperlink($sheet) {
    $addr = ''
    $n = 0
    $shapes = $null
    try { $shapes = $sheet.Shapes } catch { }
    if ($null -eq $shapes) {
        return @{ Count = 0; Address = '' }
    }
    try {
        try { $n = [int]$shapes.Count } catch { $n = 0 }
        if ($n -ge 1) {
            $sh = $null
            $h = $null
            try {
                $sh = $shapes.Item(1)
                $h = $sh.Hyperlink
                if ($null -ne $h) {
                    try { $addr = [string]$h.Address } catch { }
                }
            } catch {
                Write-Log "Shape.Hyperlink $($_.Exception.Message)"
            } finally {
                if ($null -ne $h) { Release-Com $h }
                if ($null -ne $sh) { Release-Com $sh }
            }
        }
    } finally {
        Release-Com $shapes
    }
    return @{ Count = $n; Address = $addr }
}

function Get-ChartShapeCount($wb, [string]$name) {
    $n = 0
    $ch = $null
    try { $ch = $wb.Charts.Item($name) } catch {
        try { $ch = $wb.Sheets.Item($name) } catch { }
    }
    if ($null -eq $ch) { return 0 }
    try {
        $shapes = $null
        try { $shapes = $ch.Shapes } catch { }
        if ($null -ne $shapes) {
            try { $n = [int]$shapes.Count } catch { $n = 0 }
            Release-Com $shapes
        }
    } finally {
        Release-Com $ch
    }
    return $n
}

$pngB = Ensure-OoxpPngB
$hostPath = Get-OutPath 'xlsx-drawing-hlink-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
        })
    }
} 'xlsx-xdh-create' | Out-Null

$picOut = Get-OutPath 'xlsx-drawing-hlink-pic.xlsx'
if (Test-Path $picOut) { Remove-Item -Force $picOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $picOut.Replace('\', '/')
    ops = @(@{
        op = 'insertPicture'
        id = 'sheet:Sheet1'
        file = $pngB.Replace('\', '/')
        from = 'B2'
        cx = 914400
        cy = 914400
    })
} 'xlsx-xdh-pic' | Out-Null

$treePic = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $picOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-xdh-inspect-pic'
$picId = [string]((@($treePic.sheets[0].pictures | Where-Object { $_ }) | Select-Object -First 1).id)

$hlinkOut = Get-OutPath 'xlsx-drawing-hlink-insert.xlsx'
if (Test-Path $hlinkOut) { Remove-Item -Force $hlinkOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $picOut.Replace('\', '/')
    output = $hlinkOut.Replace('\', '/')
    ops = @(@{
        op = 'insertHyperlink'
        id = $picId
        url = 'https://example.com/xdh'
        tooltip = '图链'
    })
} 'xlsx-xdh-hlink' | Out-Null

$excel = Ensure-MsExcel $hlinkOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $hlinkOut
$cap = [string]$wb.Windows.Item(1).Caption
$s = $wb.Worksheets.Item(1)
$hl = Read-ShapeHyperlink $s
$treeIns = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hlinkOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true; hyperlinks = $true }
} 'xlsx-xdh-inspect-inserted'
$insUrl = [string]((@($treeIns.sheets[0].pictures | Where-Object { $_ }) | Select-Object -First 1).hyperlink.url)
$cellLinks = @($treeIns.sheets[0].hyperlinks | Where-Object { $_ })
$xdh1ok = ($cap -notmatch 'Repair') -and ($hl.Address -match 'example.com/xdh') -and ($insUrl -eq 'https://example.com/xdh') -and ([int]$treeIns.hyperlinkCount -eq 0) -and ($cellLinks.Count -eq 0)
Add-Check 'xlsx-Xdh1' $xdh1ok "caption=$cap addr=$($hl.Address) inspect=$insUrl count=$($treeIns.hyperlinkCount)"
Close-AllWorkbooks $excel

$setOut = Get-OutPath 'xlsx-drawing-hlink-set.xlsx'
if (Test-Path $setOut) { Remove-Item -Force $setOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hlinkOut.Replace('\', '/')
    output = $setOut.Replace('\', '/')
    ops = @(@{
        op = 'setHyperlink'
        id = $picId
        url = 'https://example.org/xdh2'
    })
} 'xlsx-xdh-set' | Out-Null

Close-AllWorkbooks $excel
$wbS = Open-ExcelBook $excel $setOut
$capS = [string]$wbS.Windows.Item(1).Caption
$sS = $wbS.Worksheets.Item(1)
$hlS = Read-ShapeHyperlink $sS
Close-AllWorkbooks $excel

$delOut = Get-OutPath 'xlsx-drawing-hlink-delete.xlsx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $setOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteHyperlink'; id = $picId })
} 'xlsx-xdh-delete' | Out-Null

$wbD = Open-ExcelBook $excel $delOut
$capD = [string]$wbD.Windows.Item(1).Caption
$sD = $wbD.Worksheets.Item(1)
$stillPic = 0
try { $stillPic = [int]$sD.Shapes.Count } catch { }
Close-AllWorkbooks $excel
$treeDel = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-xdh-inspect-deleted'
$delHl = (@($treeDel.sheets[0].pictures | Where-Object { $_ }) | Select-Object -First 1).hyperlink
$xdh2ok = ($capS -notmatch 'Repair') -and ($hlS.Address -match 'example.org/xdh2') -and ($capD -notmatch 'Repair') -and ($stillPic -ge 1) -and ($null -eq $delHl) -and ([int]$treeDel.pictureCount -ge 1)
Add-Check 'xlsx-Xdh2' $xdh2ok "setCap=$capS addr=$($hlS.Address) delCap=$capD shapes=$stillPic hl=$delHl pics=$($treeDel.pictureCount)"

$csHost = Get-OutPath 'xlsx-drawing-hlink-chartsheet.xlsx'
if (Test-Path $csHost) { Remove-Item -Force $csHost }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $csHost.Replace('\', '/')
    ops = @(@{
        op = 'insertChartSheet'
        id = 'xlsx:workbook'
        name = 'Chart1'
        dataFrom = 'A10'
        type = 'col'
        title = '销量'
        series = @(@{
            name = 'Q1'
            categories = @('甲', '乙', '丙')
            values = @(1, 2, 3)
        })
    })
} 'xlsx-xdh-chartsheet' | Out-Null

$spOut = Get-OutPath 'xlsx-drawing-hlink-shape.xlsx'
if (Test-Path $spOut) { Remove-Item -Force $spOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $csHost.Replace('\', '/')
    output = $spOut.Replace('\', '/')
    ops = @(@{
        op = 'insertShape'
        id = 'sheet:Chart1'
        prst = 'rect'
        fromX = 0.25
        fromY = 0.25
        toX = 0.75
        toY = 0.75
    })
} 'xlsx-xdh-shape' | Out-Null

Close-AllWorkbooks $excel
$wbSp = Open-ExcelBook $excel $spOut
$capSp = [string]$wbSp.Windows.Item(1).Caption
$shapeN = Get-ChartShapeCount $wbSp 'Chart1'
Close-AllWorkbooks $excel
$treeSp = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $spOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ shapes = $true }
} 'xlsx-xdh-inspect-shape'
$cs = @($treeSp.sheets | Where-Object { $_.name -eq 'Chart1' }) | Select-Object -First 1
$spAnchor = [string]((@($cs.shapes | Where-Object { $_ }) | Select-Object -First 1).anchor)
$xdh3ok = ($capSp -notmatch 'Repair') -and ($shapeN -ge 1) -and ($spAnchor -eq 'relSize') -and ([int]$cs.shapeCount -ge 1)
Add-Check 'xlsx-Xdh3' $xdh3ok "caption=$capSp com=$shapeN anchor=$spAnchor count=$($cs.shapeCount)"

$picCsOut = Get-OutPath 'xlsx-drawing-hlink-cspic.xlsx'
if (Test-Path $picCsOut) { Remove-Item -Force $picCsOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $csHost.Replace('\', '/')
    output = $picCsOut.Replace('\', '/')
    ops = @(@{
        op = 'insertPicture'
        id = 'sheet:Chart1'
        file = $pngB.Replace('\', '/')
        fromX = 0.25
        fromY = 0.25
        toX = 0.75
        toY = 0.75
    })
} 'xlsx-xdh-cspic' | Out-Null

Close-AllWorkbooks $excel
$wbPc = Open-ExcelBook $excel $picCsOut
$capPc = [string]$wbPc.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$treePc = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $picCsOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-xdh-inspect-cspic'
$csPic = @($treePc.sheets | Where-Object { $_.name -eq 'Chart1' }) | Select-Object -First 1
$xdh4ok = ($capPc -notmatch 'Repair') -and ([int]$csPic.pictureCount -ge 1) -and ([int]$treePc.pictureCount -ge 1)
Add-Check 'xlsx-Xdh4' $xdh4ok "caption=$capPc sheet=$($csPic.pictureCount) wb=$($treePc.pictureCount)"

$failFromPath = Get-OutPath 'xlsx-drawing-hlink-bad-from.xlsx'
$failFrom = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $csHost.Replace('\', '/')
    output = $failFromPath.Replace('\', '/')
    ops = @(@{
        op = 'insertShape'
        id = 'sheet:Chart1'
        from = 'B2'
    })
} 'xlsx-xdh-bad-from'
$codeFrom = $null
try { $codeFrom = ($failFrom.Stderr | ConvertFrom-Json).error.code } catch {}

$failRefPath = Get-OutPath 'xlsx-drawing-hlink-bad-ref.xlsx'
$failRefR = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $picOut.Replace('\', '/')
    output = $failRefPath.Replace('\', '/')
    ops = @(@{
        op = 'insertHyperlink'
        id = $picId
        url = 'https://example.com/x'
        ref = 'A1'
    })
} 'xlsx-xdh-bad-ref'
$codeRef = $null
try { $codeRef = ($failRefR.Stderr | ConvertFrom-Json).error.code } catch {}
$xdh5ok = ($failFrom.ExitCode -ne 0) -and ($codeFrom -eq 'InvalidField') -and (-not (Test-Path $failFromPath)) `
    -and ($failRefR.ExitCode -ne 0) -and ($codeRef -eq 'InvalidField') -and (-not (Test-Path $failRefPath))
Add-Check 'xlsx-Xdh5' $xdh5ok "from=$codeFrom wroteFrom=$(Test-Path $failFromPath) ref=$codeRef wroteRef=$(Test-Path $failRefPath)"
