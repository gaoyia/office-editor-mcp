# xlsx 命名单元格样式：docs/checklist-xlsx.md 的 Xns1–Xns5。
# 用 create 产物再 apply。不改主题。不要和 Xs、Xcs 撞。

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

function Test-HasNamedStyle($wb, [string]$name) {
    $styles = $wb.Styles
    $found = $false
    try {
        $n = [int]$styles.Count
        for ($i = 1; $i -le $n; $i++) {
            $st = $null
            try { $st = $styles.Item($i) } catch { continue }
            if ($null -eq $st) { continue }
            try {
                if ([string]$st.Name -eq $name) { $found = $true }
            } finally { Release-Com $st }
            if ($found) { break }
        }
    } finally { Release-Com $styles }
    return $found
}

$hostPath = Get-OutPath 'xlsx-named-style-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = 1.2 }
                @{ ref = 'B1'; value = 3 }
            )
        })
    }
} 'xlsx-xns-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xns-inspect'
$listed0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ cellStyles = $true }
} 'xlsx-xns-inspect-list'
$normal = @($listed0.cellStyles | Where-Object { $_.id -eq 'cellStyle:1' })[0]
$xns1ok = ([int]$tree0.cellStyleCount -eq 1) `
    -and ($null -eq $tree0.cellStyles) `
    -and ([int]$listed0.cellStyleCount -eq 1) `
    -and ([string]$normal.name -eq 'Normal') `
    -and ([int]$normal.builtinId -eq 0)
Add-Check 'xlsx-Xns1' $xns1ok "count=$($tree0.cellStyleCount) name=$($normal.name) builtin=$($normal.builtinId)"

$insOut = Get-OutPath 'xlsx-named-style-insert.xlsx'
if (Test-Path $insOut) { Remove-Item -Force $insOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{
        op = 'insertCellStyle'
        id = 'xlsx:workbook'
        name = 'OoxpNamed'
        font = @{ bold = $true }
        fill = @{ pattern = 'solid'; fg = @{ rgb = 'FFFFFF00' } }
    })
} 'xlsx-xns-insert' | Out-Null

$treeIns = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ cellStyles = $true }
} 'xlsx-xns-insert-inspect'

$excel = Ensure-MsExcel $insOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $insOut
$capIns = [string]$wb.Windows.Item(1).Caption
$hasNamed = Test-HasNamedStyle $wb 'OoxpNamed'
$xns2ok = ($capIns -notmatch 'Repair') `
    -and $hasNamed `
    -and ([int]$treeIns.cellStyleCount -eq 2)
Add-Check 'xlsx-Xns2' $xns2ok "caption=$capIns hasNamed=$hasNamed count=$($treeIns.cellStyleCount)"
Close-AllWorkbooks $excel

$applyOut = Get-OutPath 'xlsx-named-style-apply.xlsx'
if (Test-Path $applyOut) { Remove-Item -Force $applyOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $applyOut.Replace('\', '/')
    ops = @(@{
        op = 'setCellNamedStyle'
        id = 'cell:Sheet1!A1'
        style = 'cellStyle:2'
    })
} 'xlsx-xns-apply' | Out-Null

$wb = Open-ExcelBook $excel $applyOut
$capApply = [string]$wb.Windows.Item(1).Caption
$s1 = $wb.Worksheets.Item('Sheet1')
$a1 = $s1.Range('A1')
$st = $a1.Style
$styleName = [string]$st.Name
Release-Com $st
$aFill = [double]$a1.Interior.Color
$aVal = $a1.Value2
$xns3ok = ($capApply -notmatch 'Repair') `
    -and ($styleName -eq 'OoxpNamed') `
    -and ([math]::Abs($aFill - 65535) -lt 1) `
    -and ([math]::Abs([double]$aVal - 1.2) -lt 0.001)
Add-Check 'xlsx-Xns3' $xns3ok "caption=$capApply style=$styleName fill=$aFill val=$aVal"
Release-Com $a1
Release-Com $s1
Close-AllWorkbooks $excel

$delOut = Get-OutPath 'xlsx-named-style-delete.xlsx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $applyOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteCellStyle'; id = 'cellStyle:2' })
} 'xlsx-xns-delete' | Out-Null

$treeDel = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ cellStyles = $true; sheet = 'Sheet1'; range = 'A1' }
} 'xlsx-xns-delete-inspect'
$stillNamed = @($treeDel.cellStyles | Where-Object { $_.name -eq 'OoxpNamed' }).Count
$a1tree = @($treeDel.sheets[0].cells | Where-Object { $_.id -eq 'cell:Sheet1!A1' })[0]

$wb = Open-ExcelBook $excel $delOut
$capDel = [string]$wb.Windows.Item(1).Caption
$hasNamedAfter = Test-HasNamedStyle $wb 'OoxpNamed'
$s1 = $wb.Worksheets.Item('Sheet1')
$a1 = $s1.Range('A1')
$aFillDel = [double]$a1.Interior.Color
Release-Com $a1
Release-Com $s1
Close-AllWorkbooks $excel

$failBuiltin = Get-OutPath 'xlsx-named-style-del-normal.xlsx'
$failDel = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $delOut.Replace('\', '/')
    output = $failBuiltin.Replace('\', '/')
    ops = @(@{ op = 'deleteCellStyle'; id = 'cellStyle:1' })
} 'xlsx-xns-del-normal'
$codeBuiltin = $null
try { $codeBuiltin = ($failDel.Stderr | ConvertFrom-Json).error.code } catch {}
$xns4ok = ($capDel -notmatch 'Repair') `
    -and ([int]$treeDel.cellStyleCount -eq 1) `
    -and ($stillNamed -eq 0) `
    -and (-not $hasNamedAfter) `
    -and ([string]$a1tree.fill.pattern -eq 'solid') `
    -and ([math]::Abs($aFillDel - 65535) -lt 1) `
    -and ($failDel.ExitCode -ne 0) `
    -and ($codeBuiltin -eq 'InvalidField') `
    -and (-not (Test-Path $failBuiltin))
Add-Check 'xlsx-Xns4' $xns4ok "caption=$capDel count=$($treeDel.cellStyleCount) named=$stillNamed fill=$aFillDel code=$codeBuiltin"

$failOut = Get-OutPath 'xlsx-named-style-empty.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'insertCellStyle'; id = 'xlsx:workbook' })
} 'xlsx-xns-empty'
$codeEmpty = $null
try { $codeEmpty = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$xns5ok = ($fail.ExitCode -ne 0) -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xns5' $xns5ok "exit=$($fail.ExitCode) code=$codeEmpty wrote=$(Test-Path $failOut)"
