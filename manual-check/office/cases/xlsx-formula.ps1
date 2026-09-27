# xlsx 公式结构：docs/checklist-xlsx.md 的 Xf1–Xf5。
# 用 create 产物再 apply 写出共享组与数组，不另做 Excel 另存样例。

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

function Find-Cell($cells, [string]$id) {
    foreach ($c in $cells) {
        if ($c.id -eq $id) { return $c }
    }
    return $null
}

$hostPath = Get-OutPath 'xlsx-formula-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'A1'; value = 10 }
                @{ ref = 'A2'; value = 11 }
            )
        })
    }
} 'xlsx-xf-create' | Out-Null

$sharedPath = Get-OutPath 'xlsx-formula-shared.xlsx'
if (Test-Path $sharedPath) { Remove-Item -Force $sharedPath }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $sharedPath.Replace('\', '/')
    ops = @(@{
        op = 'setCell'; id = 'cell:Sheet1!B1'; formula = '=A1*2'; sharedRef = 'B1:B2'
    })
} 'xlsx-xf-shared' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sharedPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:C3' }
} 'xlsx-xf-inspect'
$b1 = Find-Cell $tree0.sheets[0].cells 'cell:Sheet1!B1'
$b2 = Find-Cell $tree0.sheets[0].cells 'cell:Sheet1!B2'
$xf1ok = ($b1.formula -eq '=A1*2') -and ($b1.shared -eq $true) -and ($b1.sharedRef -eq 'B1:B2') -and ($b2.formula -eq '=A2*2') -and ($b2.shared -eq $true)
Add-Check 'xlsx-Xf1' $xf1ok "b1f=$($b1.formula) b1s=$($b1.shared) ref=$($b1.sharedRef) b2f=$($b2.formula)"

$keepPath = Get-OutPath 'xlsx-formula-keep.xlsx'
if (Test-Path $keepPath) { Remove-Item -Force $keepPath }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sharedPath.Replace('\', '/')
    output = $keepPath.Replace('\', '/')
    ops = @(@{ op = 'setCell'; id = 'cell:Sheet1!B1'; formula = '=A1*3' })
} 'xlsx-xf-keep' | Out-Null

$excel = Ensure-MsExcel $keepPath
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $keepPath
$cap = [string]$wb.Windows.Item(1).Caption
$s1 = $wb.Worksheets.Item('Sheet1')
$b1v = [double]$s1.Range('B1').Value2
$b2v = [double]$s1.Range('B2').Value2
$b1f = [string]$s1.Range('B1').Formula
$b2f = [string]$s1.Range('B2').Formula
$xf2ok = ($cap -notmatch 'Repair') -and ([math]::Abs($b1v - 30) -lt 0.001) -and ([math]::Abs($b2v - 33) -lt 0.001)
Add-Check 'xlsx-Xf2' $xf2ok "caption=$cap b1v=$b1v b2v=$b2v b1f=$b1f b2f=$b2f"
Release-Com $s1
Close-AllWorkbooks $excel

$expPath = Get-OutPath 'xlsx-formula-explode.xlsx'
if (Test-Path $expPath) { Remove-Item -Force $expPath }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $keepPath.Replace('\', '/')
    output = $expPath.Replace('\', '/')
    ops = @(@{ op = 'setCell'; id = 'cell:Sheet1!B1'; formula = '=A1*4'; shared = $false })
} 'xlsx-xf-explode' | Out-Null

$excel2 = Ensure-MsExcel $expPath
Close-AllWorkbooks $excel2
$wb2 = Open-ExcelBook $excel2 $expPath
$cap2 = [string]$wb2.Windows.Item(1).Caption
$s1b = $wb2.Worksheets.Item('Sheet1')
$e1v = [double]$s1b.Range('B1').Value2
$e2v = [double]$s1b.Range('B2').Value2
$e1f = [string]$s1b.Range('B1').Formula
$e2f = [string]$s1b.Range('B2').Formula
$xf3ok = ($cap2 -notmatch 'Repair') -and ([math]::Abs($e1v - 40) -lt 0.001) -and ([math]::Abs($e2v - 33) -lt 0.001) -and ($e1f -eq '=A1*4') -and ($e2f -eq '=A2*3')
Add-Check 'xlsx-Xf3' $xf3ok "caption=$cap2 e1v=$e1v e2v=$e2v e1f=$e1f e2f=$e2f"
Release-Com $s1b
Close-AllWorkbooks $excel2

$arrayPath = Get-OutPath 'xlsx-formula-array.xlsx'
if (Test-Path $arrayPath) { Remove-Item -Force $arrayPath }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $arrayPath.Replace('\', '/')
    ops = @(@{
        op = 'setCell'; id = 'cell:Sheet1!C1'; formula = '=A1:A2'; arrayRef = 'C1:C2'
    })
} 'xlsx-xf-array' | Out-Null

$excel3 = Ensure-MsExcel $arrayPath
Close-AllWorkbooks $excel3
$wb3 = Open-ExcelBook $excel3 $arrayPath
$cap3 = [string]$wb3.Windows.Item(1).Caption
$s1c = $wb3.Worksheets.Item('Sheet1')
$c1 = $s1c.Range('C1')
$c2 = $s1c.Range('C2')
$hasArr = [bool]$c1.HasArray
$c1v = [double]$c1.Value2
$c2v = [double]$c2.Value2
$xf4ok = ($cap3 -notmatch 'Repair') -and $hasArr -and ([math]::Abs($c1v - 10) -lt 0.001) -and ([math]::Abs($c2v - 11) -lt 0.001)
Add-Check 'xlsx-Xf4' $xf4ok "caption=$cap3 hasArr=$hasArr c1v=$c1v c2v=$c2v"
Release-Com $c1
Release-Com $c2
Release-Com $s1c
Close-AllWorkbooks $excel3

$failOut = Get-OutPath 'xlsx-formula-part.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $arrayPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'setCell'; id = 'cell:Sheet1!C2'; value = 1 })
} 'xlsx-xf-part'
$codePart = $null
try { $codePart = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$xf5ok = ($fail.ExitCode -ne 0) -and ($codePart -eq 'NotImplemented') -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xf5' $xf5ok "exit=$($fail.ExitCode) code=$codePart wrote=$(Test-Path $failOut)"
