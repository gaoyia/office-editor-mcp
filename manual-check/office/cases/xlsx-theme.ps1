# xlsx 主题：docs/checklist-xlsx.md 的 Xth1–Xth5。
# 编号 Xth，不要和 Th、Ts、Xtf 撞。不算主题排版引擎。不要 Stop-Process Excel。

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

$tmpDir = Join-Path $env:TEMP 'ooxp-office'
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null

$excel = Ensure-MsExcel (Ensure-XlsxCreate)
Close-AllWorkbooks $excel

$src = Get-OutPath 'xlsx-theme-excel-blank.xlsx'
$tmpBlank = Join-Path $tmpDir 'xlsx-theme-blank.xlsx'
if (Test-Path $tmpBlank) { Remove-Item -Force $tmpBlank }
$wb = $excel.Workbooks.Add()
$xlWorkbookDefault = 51
$wb.SaveAs($tmpBlank, $xlWorkbookDefault)
$wb.Close($false)
Release-Com $wb
Copy-Item -Force $tmpBlank $src
Write-Log "saved excel blank $src"

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/')
} 'xlsx-th-inspect'
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $src
$cap0 = [string]$wb.Windows.Item(1).Caption
$hasTheme = $null -ne $tree0.theme -and $null -ne $tree0.theme.colors
$accent0 = [string]$tree0.theme.colors.accent1
Close-AllWorkbooks $excel
Add-Check 'xlsx-Xth1' ($hasTheme -and ($accent0.Length -eq 6) -and ($cap0 -notmatch 'Repair')) "caption=$cap0 accent1=$accent0"

$colorOut = Get-OutPath 'xlsx-theme-color.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $colorOut.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeColors'; id = 'xlsx:workbook'; accent1 = 'FF0000' }
    )
} 'xlsx-th-color' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $colorOut.Replace('\', '/')
} 'xlsx-th-color-inspect'
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $colorOut
$capC = [string]$wb.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$accentC = [string]$treeC.theme.colors.accent1
Add-Check 'xlsx-Xth2' (($capC -notmatch 'Repair') -and ($accentC -eq 'FF0000')) "caption=$capC accent1=$accentC"

$fontOut = Get-OutPath 'xlsx-theme-font.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $colorOut.Replace('\', '/')
    output = $fontOut.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeFonts'; id = 'xlsx:workbook'; majorLatin = 'Arial' }
        @{ op = 'setThemeExtraSchemes'; id = 'xlsx:workbook'; extraSchemes = @(@{ name = 'Extra'; colors = @{ accent1 = '0F6FC6' } }) }
    )
} 'xlsx-th-font' | Out-Null
$treeF = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $fontOut.Replace('\', '/')
} 'xlsx-th-font-inspect'
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $fontOut
$capF = [string]$wb.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$latin = [string]$treeF.theme.fonts.majorLatin
$extra = [string]$treeF.theme.extraSchemes[0].name
Add-Check 'xlsx-Xth3' (($capF -notmatch 'Repair') -and ($latin -eq 'Arial') -and ($extra -eq 'Extra')) "caption=$capF latin=$latin extra=$extra"

$ovOut = Get-OutPath 'xlsx-theme-override.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $fontOut.Replace('\', '/')
    output = $ovOut.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeOverride'; id = 'xlsx:workbook'; accent1 = '00FF00' }
    )
} 'xlsx-th-ov' | Out-Null
$treeO = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ovOut.Replace('\', '/')
} 'xlsx-th-ov-inspect'
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $ovOut
$capO = [string]$wb.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$ovAccent = [string]$treeO.themeOverride.colors.accent1

$skel = Get-OutPath 'xlsx-theme-create.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $skel.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) })
        theme = @{
            name = 'Office Theme'
            colors = @{ accent1 = '112233' }
            fonts = @{ majorLatin = 'Calibri' }
        }
    }
} 'xlsx-th-create' | Out-Null
$treeCr = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $skel.Replace('\', '/')
} 'xlsx-th-create-inspect'
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $skel
$capCr = [string]$wb.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$bare = Get-OutPath 'xlsx-theme-bare.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $bare.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-th-bare' | Out-Null
$treeB = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $bare.Replace('\', '/')
} 'xlsx-th-bare-inspect'
$createOk = ($treeCr.theme.colors.accent1 -eq '112233') -and ($null -eq $treeB.theme)
Add-Check 'xlsx-Xth4' (($capO -notmatch 'Repair') -and ($ovAccent -eq '00FF00') -and ($capCr -notmatch 'Repair') -and $createOk) "ov=$ovAccent create=$($treeCr.theme.colors.accent1) bare=$($null -eq $treeB.theme)"

$failMissing = Get-OutPath 'xlsx-theme-missing.xlsx'
$miss = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $bare.Replace('\', '/')
    output = $failMissing.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeColors'; id = 'xlsx:workbook'; accent1 = 'FF0000' }
    )
} 'xlsx-th-missing'
$failBad = Get-OutPath 'xlsx-theme-bad.xlsx'
$badId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failBad.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeColors'; id = 'sheet:Sheet1'; accent1 = 'FF0000' }
    )
} 'xlsx-th-bad-id'
$empty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failBad.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeColors'; id = 'xlsx:workbook' }
    )
} 'xlsx-th-empty'
Add-Check 'xlsx-Xth5' (
    ((Get-FailCode $miss) -eq 'IdNotFound') -and
    ((Get-FailCode $badId) -eq 'InvalidId') -and
    ((Get-FailCode $empty) -eq 'InvalidField') -and
    -not (Test-Path $failMissing) -and
    -not (Test-Path $failBad)
) "miss=$(Get-FailCode $miss) id=$(Get-FailCode $badId) empty=$(Get-FailCode $empty)"
