# xlsx 单元格样式：docs/checklist-xlsx.md 的 Xs1–Xs5。
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

function Test-GeneralFmt([string]$fmt) {
    $n = $fmt.Trim()
    $n -eq 'General' -or $n -eq '常规' -or $n -eq '@' -or $n -like 'G/*' -or $n.Contains('通用')
}

$hostPath = Get-OutPath 'xlsx-cellstyle-host.xlsx'
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
} 'xlsx-xs-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:B1' }
} 'xlsx-xs-inspect'
$a1_0 = @($tree0.sheets[0].cells | Where-Object { $_.id -eq 'cell:Sheet1!A1' })[0]
$xs1ok = ([string]$a1_0.numFmt -eq 'General')
Add-Check 'xlsx-Xs1' $xs1ok "numFmt=$($a1_0.numFmt) keys=$($a1_0.PSObject.Properties.Name -join ',')"

$styleOut = Get-OutPath 'xlsx-cellstyle-apply.xlsx'
if (Test-Path $styleOut) { Remove-Item -Force $styleOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $styleOut.Replace('\', '/')
    ops = @(
        @{ op = 'setCell'; id = 'cell:Sheet1!A1'; value = 1.2 }
        @{
            op = 'setCellStyle'
            id = 'cell:Sheet1!A1'
            numFmt = '0.00'
            fill = @{ pattern = 'solid'; fg = @{ rgb = 'FFFFFF00' } }
        }
    )
} 'xlsx-xs-apply' | Out-Null

$treeA = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $styleOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:B1' }
} 'xlsx-xs-apply-inspect'

$excel = Ensure-MsExcel $styleOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $styleOut
$cap = [string]$wb.Windows.Item(1).Caption
$s1 = $wb.Worksheets.Item('Sheet1')
$a1 = $s1.Range('A1')
$b1 = $s1.Range('B1')
$aFmt = [string]$a1.NumberFormat
$aFill = [double]$a1.Interior.Color
$aVal = $a1.Value2
$bFmt = [string]$b1.NumberFormat
$bFill = [double]$b1.Interior.Color
$xs2ok = ($cap -notmatch 'Repair') -and ($aFmt -eq '0.00') -and ([math]::Abs($aFill - 65535) -lt 1) -and ([math]::Abs([double]$aVal - 1.2) -lt 0.001)
Add-Check 'xlsx-Xs2' $xs2ok "caption=$cap fmt=$aFmt fill=$aFill val=$aVal"
$xs3ok = (Test-GeneralFmt $bFmt) -and ([math]::Abs($aFill - $bFill) -gt 1)
Add-Check 'xlsx-Xs3' $xs3ok "B1 fmt=$bFmt fill=$bFill"
$a1tree = @($treeA.sheets[0].cells | Where-Object { $_.id -eq 'cell:Sheet1!A1' })[0]
$xs4ok = ($xs2ok) -and ([string]$a1tree.numFmt -eq '0.00') -and ([string]$a1tree.fill.pattern -eq 'solid')
Add-Check 'xlsx-Xs4' $xs4ok "inspect numFmt=$($a1tree.numFmt) pattern=$($a1tree.fill.pattern)"
Release-Com $a1
Release-Com $b1
Release-Com $s1
Close-AllWorkbooks $excel

$failOut = Get-OutPath 'xlsx-cellstyle-empty.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'setCellStyle'; id = 'cell:Sheet1!A1' })
} 'xlsx-xs-empty'
$codeEmpty = $null
try { $codeEmpty = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$xs5ok = ($fail.ExitCode -ne 0) -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xs5' $xs5ok "exit=$($fail.ExitCode) code=$codeEmpty wrote=$(Test-Path $failOut)"
