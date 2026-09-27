# xlsx 打印设置：docs/checklist-xlsx.md 的 Xps1–Xps5。
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

$hostPath = Get-OutPath 'xlsx-print-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = 1 }
                @{ ref = 'A10'; value = 10 }
            )
        })
    }
} 'xlsx-xps-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xps-inspect'
$p0 = $tree0.sheets[0].print
$xps1ok = ($null -ne $p0) -and ([double]$p0.left -eq 0.7) -and ($p0.orientation -eq 'portrait') -and ($null -eq $p0.printArea)
Add-Check 'xlsx-Xps1' $xps1ok "left=$($p0.left) ori=$($p0.orientation) area=$($p0.printArea)"

$printOut = Get-OutPath 'xlsx-print-setup.xlsx'
if (Test-Path $printOut) { Remove-Item -Force $printOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $printOut.Replace('\', '/')
    ops = @(
        @{
            op = 'setHeaderFooterText'
            id = 'sheet:Sheet1/hf:odd:header'
            text = '&CXps页眉'
        },
        @{
            op = 'setPrintSetup'
            id = 'sheet:Sheet1'
            left = 1
            orientation = 'landscape'
        }
    )
} 'xlsx-xps-margin' | Out-Null

$excel = Ensure-MsExcel $printOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $printOut
$cap = [string]$wb.Windows.Item(1).Caption
$s = $wb.Worksheets.Item(1)
$leftPts = [double]$s.PageSetup.LeftMargin
$wantPts = [double]$excel.InchesToPoints(1)
$ori = [int]$s.PageSetup.Orientation
$header = [string]$s.PageSetup.CenterHeader
$leftOk = [math]::Abs($leftPts - $wantPts) -lt 0.51
$xps2ok = ($cap -notmatch 'Repair') -and $leftOk -and ($ori -eq 2) -and ($header -match 'Xps页眉')
Add-Check 'xlsx-Xps2' $xps2ok "caption=$cap left=$leftPts want=$wantPts ori=$ori header=$header"
Close-AllWorkbooks $excel

$areaOut = Get-OutPath 'xlsx-print-area.xlsx'
if (Test-Path $areaOut) { Remove-Item -Force $areaOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $printOut.Replace('\', '/')
    output = $areaOut.Replace('\', '/')
    ops = @(@{
        op = 'setPrintSetup'
        id = 'sheet:Sheet1'
        printArea = 'A1:A10'
    })
} 'xlsx-xps-area' | Out-Null

Close-AllWorkbooks $excel
$wbA = Open-ExcelBook $excel $areaOut
$capA = [string]$wbA.Windows.Item(1).Caption
$sA = $wbA.Worksheets.Item(1)
$area = [string]$sA.PageSetup.PrintArea
$areaOk = $area -match 'A\$1:\$A\$10'
$xps3ok = ($capA -notmatch 'Repair') -and $areaOk
Add-Check 'xlsx-Xps3' $xps3ok "caption=$capA printArea=$area"
Close-AllWorkbooks $excel

$gridOut = Get-OutPath 'xlsx-print-grid.xlsx'
if (Test-Path $gridOut) { Remove-Item -Force $gridOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $areaOut.Replace('\', '/')
    output = $gridOut.Replace('\', '/')
    ops = @(@{
        op = 'setPrintSetup'
        id = 'sheet:Sheet1'
        gridLines = $true
    })
} 'xlsx-xps-grid' | Out-Null

Close-AllWorkbooks $excel
$wbG = Open-ExcelBook $excel $gridOut
$capG = [string]$wbG.Windows.Item(1).Caption
$sG = $wbG.Worksheets.Item(1)
$grid = [bool]$sG.PageSetup.PrintGridlines
$xps4ok = ($capG -notmatch 'Repair') -and $grid
Add-Check 'xlsx-Xps4' $xps4ok "caption=$capG grid=$grid"
Close-AllWorkbooks $excel

$failOut = Get-OutPath 'xlsx-print-bad.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'setPrintSetup'; id = 'sheet:Sheet1' })
} 'xlsx-xps-missing'
$codeEmpty = $null
try { $codeEmpty = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$xps5ok = ($fail.ExitCode -ne 0) -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xps5' $xps5ok "exit=$($fail.ExitCode) code=$codeEmpty wrote=$(Test-Path $failOut)"
