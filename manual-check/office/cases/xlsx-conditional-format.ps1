# xlsx 条件格式：docs/checklist-xlsx.md 的 Xn1–Xn5。
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

# Excel COLORREF：R + G*256 + B*65536。绿底 FFC6EFCE。
$script:GreenFill = 198 + (239 * 256) + (206 * 65536)

$hostPath = Get-OutPath 'xlsx-cf-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = 1 }
                @{ ref = 'A2'; value = -1 }
                @{ ref = 'A3'; value = 5 }
            )
        })
    }
} 'xlsx-xn-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:A10' }
} 'xlsx-xn-inspect'
$xn1ok = ($tree0.sheets[0].conditionalFormatCount -eq 0) -and ($null -ne $tree0.sheets[0].conditionalFormats) -and ($tree0.sheets[0].conditionalFormats.Count -eq 0)
Add-Check 'xlsx-Xn1' $xn1ok "count=$($tree0.sheets[0].conditionalFormatCount) n=$($tree0.sheets[0].conditionalFormats.Count)"

$greenOut = Get-OutPath 'xlsx-cf-green.xlsx'
if (Test-Path $greenOut) { Remove-Item -Force $greenOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $greenOut.Replace('\', '/')
    ops = @(@{
        op = 'insertConditionalFormat'
        id = 'sheet:Sheet1'
        sqref = 'A1:A10'
        type = 'cellIs'
        operator = 'greaterThan'
        formulas = @('0')
        dxf = @{ fill = @{ pattern = 'solid'; fg = @{ rgb = 'FFC6EFCE' } } }
    })
} 'xlsx-xn-green' | Out-Null

$excel = Ensure-MsExcel $greenOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $greenOut
$cap = [string]$wb.Windows.Item(1).Caption
$s1 = $wb.Worksheets.Item('Sheet1')
$a1 = $s1.Range('A1')
$fcCount = $null
$fcType = $null
$fcOp = $null
$fcFill = $null
$dispFill = $null
$a1val = $null
try {
    $a1val = $a1.Value2
    $fcs = $a1.FormatConditions
    $fcCount = [int]$fcs.Count
    if ($fcCount -ge 1) {
        $fc = $fcs.Item(1)
        $fcType = [int]$fc.Type
        try { $fcOp = [int]$fc.Operator } catch { $fcOp = 0 }
        try { $fcFill = [double]$fc.Interior.Color } catch { $fcFill = -1 }
        Release-Com $fc
    }
    try { $dispFill = [double]$a1.DisplayFormat.Interior.Color } catch { $dispFill = -1 }
    Release-Com $fcs
} catch {
    Write-Log "Xn2 FormatConditions 读失败 $($_.Exception.Message)"
}
# Type 1 = xlCellValue，Operator 5 = xlGreater。
$greenSeen = ($null -ne $fcFill -and [math]::Abs($fcFill - $script:GreenFill) -lt 2) -or
    ($null -ne $dispFill -and [math]::Abs($dispFill - $script:GreenFill) -lt 2)
$xn2ok = ($cap -notmatch 'Repair') -and ($fcCount -ge 1) -and ($fcType -eq 1) -and ($fcOp -eq 5) -and $greenSeen -and ([double]$a1val -eq 1)
Add-Check 'xlsx-Xn2' $xn2ok "caption=$cap count=$fcCount type=$fcType op=$fcOp fcFill=$fcFill disp=$dispFill val=$a1val expect=$($script:GreenFill)"
Release-Com $a1
Release-Com $s1
Close-AllWorkbooks $excel

$scaleOut = Get-OutPath 'xlsx-cf-scale.xlsx'
if (Test-Path $scaleOut) { Remove-Item -Force $scaleOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $scaleOut.Replace('\', '/')
    ops = @(@{
        op = 'insertConditionalFormat'
        id = 'sheet:Sheet1'
        sqref = 'A1:A10'
        type = 'colorScale'
        colorScale = @{
            colors = @(
                @{ rgb = 'FFFF0000' }
                @{ rgb = 'FF00B050' }
            )
        }
    })
} 'xlsx-xn-scale' | Out-Null

$wb2 = Open-ExcelBook $excel $scaleOut
$cap2 = [string]$wb2.Windows.Item(1).Caption
$s2 = $wb2.Worksheets.Item('Sheet1')
$a1s = $s2.Range('A1')
$scCount = $null
$scType = $null
try {
    $fcs2 = $a1s.FormatConditions
    $scCount = [int]$fcs2.Count
    if ($scCount -ge 1) {
        $fc2 = $fcs2.Item(1)
        $scType = [int]$fc2.Type
        Release-Com $fc2
    }
    Release-Com $fcs2
} catch {
    Write-Log "Xn3 FormatConditions 读失败 $($_.Exception.Message)"
}
# Type 3 = xlColorScale。
$xn3ok = ($cap2 -notmatch 'Repair') -and ($scCount -ge 1) -and ($scType -eq 3)
Add-Check 'xlsx-Xn3' $xn3ok "caption=$cap2 count=$scCount type=$scType"
Release-Com $a1s
Release-Com $s2
Close-AllWorkbooks $excel

$delOut = Get-OutPath 'xlsx-cf-delete.xlsx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $greenOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteConditionalFormat'; id = 'sheet:Sheet1/cf:1' })
} 'xlsx-xn-delete' | Out-Null

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:A10' }
} 'xlsx-xn-delete-inspect'
$sheetXml = Get-ZipPartText $delOut 'xl/worksheets/sheet1.xml'
$wb3 = Open-ExcelBook $excel $delOut
$cap3 = [string]$wb3.Windows.Item(1).Caption
$s3 = $wb3.Worksheets.Item('Sheet1')
$a1d = $s3.Range('A1')
$delCount = $null
try {
    $fcs3 = $a1d.FormatConditions
    $delCount = [int]$fcs3.Count
    Release-Com $fcs3
} catch {
    Write-Log "Xn4 FormatConditions 读失败 $($_.Exception.Message)"
    $delCount = -1
}
$xn4ok = ($cap3 -notmatch 'Repair') -and ($treeD.sheets[0].conditionalFormatCount -eq 0) -and ($sheetXml -notmatch 'conditionalFormatting') -and ($delCount -eq 0)
Add-Check 'xlsx-Xn4' $xn4ok "caption=$cap3 inspect=$($treeD.sheets[0].conditionalFormatCount) xmlHas=$($sheetXml -match 'conditionalFormatting') comCount=$delCount"
Release-Com $a1d
Release-Com $s3
Close-AllWorkbooks $excel

$failOut = Get-OutPath 'xlsx-cf-bad.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'insertConditionalFormat'; id = 'sheet:Sheet1' })
} 'xlsx-xn-missing'
$codeEmpty = $null
try { $codeEmpty = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$xn5ok = ($fail.ExitCode -ne 0) -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xn5' $xn5ok "exit=$($fail.ExitCode) code=$codeEmpty wrote=$(Test-Path $failOut)"
