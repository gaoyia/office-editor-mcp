# xlsx 主题 objectDefaults：docs/checklist-xlsx.md 的 Xdf1–Xdf5。
# 编号 Xdf，不要和 Xth、Xtf、Th 撞。不算主题排版引擎。不要 Stop-Process Excel。

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

$excel = Ensure-MsExcel (Ensure-XlsxCreate)
Close-AllWorkbooks $excel

$src = Get-OutPath 'xlsx-theme-defaults.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) })
        theme = @{ name = 'Office Theme' }
    }
} 'xlsx-xdf-create' | Out-Null
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/')
} 'xlsx-xdf-inspect'
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $src
$cap0 = [string]$wb.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$od0 = $tree0.theme.objectDefaults
Add-Check 'xlsx-Xdf1' (
    ($cap0 -notmatch 'Repair') -and
    ($null -eq $od0)
) "caption=$cap0 objectDefaults=$od0"

$lnOut = Get-OutPath 'xlsx-theme-defaults-lnw.xlsx'
$lnJsonPath = Join-Path $global:OoxpReqDir 'xlsx-xdf-lnw.json'
$lnJson = '{"schemaVersion":1,"action":"apply","path":"' + $src.Replace('\', '/') + '","output":"' + $lnOut.Replace('\', '/') + '","ops":[{"op":"setThemeObjectDefaults","id":"xlsx:workbook","objectDefaults":{"sp":{"lnW":38100}}}]}'
[System.IO.File]::WriteAllText($lnJsonPath, $lnJson, $global:OoxpUtf8)
Invoke-Ooxp $lnJsonPath | Out-Null
$treeL = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $lnOut.Replace('\', '/')
} 'xlsx-xdf-lnw-inspect'
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $lnOut
$capL = [string]$wb.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$lnW = $treeL.theme.objectDefaults.sp.lnW
Add-Check 'xlsx-Xdf2' (
    ($capL -notmatch 'Repair') -and
    ($lnW -eq 38100)
) "caption=$capL lnW=$lnW"

$stOut = Get-OutPath 'xlsx-theme-defaults-style.xlsx'
$stJsonPath = Join-Path $global:OoxpReqDir 'xlsx-xdf-style.json'
$stJson = '{"schemaVersion":1,"action":"apply","path":"' + $lnOut.Replace('\', '/') + '","output":"' + $stOut.Replace('\', '/') + '","ops":[{"op":"setThemeObjectDefaults","id":"xlsx:workbook","objectDefaults":{"sp":{"style":{"lnRef":1,"fontRef":"minor"}}}}]}'
[System.IO.File]::WriteAllText($stJsonPath, $stJson, $global:OoxpUtf8)
Invoke-Ooxp $stJsonPath | Out-Null
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $stOut.Replace('\', '/')
} 'xlsx-xdf-style-inspect'
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $stOut
$capS = [string]$wb.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$lnRef = $treeS.theme.objectDefaults.sp.style.lnRef
$fontRef = [string]$treeS.theme.objectDefaults.sp.style.fontRef
$lnW2 = $treeS.theme.objectDefaults.sp.lnW
Add-Check 'xlsx-Xdf3' (
    ($capS -notmatch 'Repair') -and
    ($lnRef -eq 1) -and
    ($fontRef -eq 'minor') -and
    ($lnW2 -eq 38100)
) "caption=$capS lnRef=$lnRef fontRef=$fontRef lnW=$lnW2"

$clOut = Get-OutPath 'xlsx-theme-defaults-clear.xlsx'
$clJsonPath = Join-Path $global:OoxpReqDir 'xlsx-xdf-clear.json'
$clJson = '{"schemaVersion":1,"action":"apply","path":"' + $stOut.Replace('\', '/') + '","output":"' + $clOut.Replace('\', '/') + '","ops":[{"op":"setThemeObjectDefaults","id":"xlsx:workbook","objectDefaults":null}]}'
[System.IO.File]::WriteAllText($clJsonPath, $clJson, $global:OoxpUtf8)
Invoke-Ooxp $clJsonPath | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $clOut.Replace('\', '/')
} 'xlsx-xdf-clear-inspect'
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $clOut
$capC = [string]$wb.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$odC = $treeC.theme.objectDefaults
Add-Check 'xlsx-Xdf4' (
    ($capC -notmatch 'Repair') -and
    ($null -eq $odC)
) "caption=$capC objectDefaults=$odC"

$failId = Get-OutPath 'xlsx-theme-defaults-bad-id.xlsx'
$badId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failId.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeObjectDefaults'; id = 'sheet:Sheet1'; objectDefaults = @{ sp = @{ lnW = 1 } } }
    )
} 'xlsx-xdf-bad-id'
$failMiss = Get-OutPath 'xlsx-theme-defaults-missing.xlsx'
$miss = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failMiss.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeObjectDefaults'; id = 'xlsx:workbook' }
    )
} 'xlsx-xdf-missing'
$failFont = Get-OutPath 'xlsx-theme-defaults-font.xlsx'
$font = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failFont.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeObjectDefaults'; id = 'xlsx:workbook'; objectDefaults = @{ sp = @{ style = @{ fontRef = 'latin' } } } }
    )
} 'xlsx-xdf-font'
$skel = Get-OutPath 'xlsx-theme-defaults-notheme.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $skel.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xdf-skel' | Out-Null
$failNone = Get-OutPath 'xlsx-theme-defaults-notheme-out.xlsx'
$none = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $skel.Replace('\', '/')
    output = $failNone.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeObjectDefaults'; id = 'xlsx:workbook'; objectDefaults = @{ sp = @{ lnW = 1 } } }
    )
} 'xlsx-xdf-notheme'
Add-Check 'xlsx-Xdf5' (
    ((Get-FailCode $badId) -eq 'InvalidId') -and
    ((Get-FailCode $miss) -eq 'InvalidField') -and
    ((Get-FailCode $font) -eq 'InvalidField') -and
    ((Get-FailCode $none) -eq 'IdNotFound') -and
    -not (Test-Path $failId) -and
    -not (Test-Path $failMiss) -and
    -not (Test-Path $failFont) -and
    -not (Test-Path $failNone)
) "id=$(Get-FailCode $badId) miss=$(Get-FailCode $miss) font=$(Get-FailCode $font) none=$(Get-FailCode $none)"
