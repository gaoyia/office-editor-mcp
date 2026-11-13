# xlsx 迷你图：docs/checklist-xlsx.md 的 Xsk1–Xsk5。
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

function Read-SheetSparks($sheet, [string]$cell = 'A1') {
    $n = 0
    $src = ''
    $typ = $null
    $rng = $null
    $col = $null
    try { $rng = $sheet.Range($cell) } catch { }
    if ($null -eq $rng) {
        return @{ Count = 0; Source = ''; Type = $null }
    }
    try { $col = $rng.SparklineGroups } catch { }
    if ($null -eq $col) {
        Release-Com $rng
        return @{ Count = 0; Source = ''; Type = $null }
    }
    try { $n = [int]$col.Count } catch { }
    if ($n -ge 1) {
        $g = $null
        try { $g = $col.Item(1) } catch { }
        if ($null -ne $g) {
            try { $src = [string]$g.SourceData } catch { }
            try { $typ = [int]$g.Type } catch { }
            Release-Com $g
        }
    }
    Release-Com $col
    Release-Com $rng
    return @{ Count = $n; Source = $src; Type = $typ }
}

$hostPath = Get-OutPath 'xlsx-sparklines-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'B1'; value = 1 }
                @{ ref = 'C1'; value = 2 }
                @{ ref = 'D1'; value = 3 }
                @{ ref = 'E1'; value = 4 }
                @{ ref = 'F1'; value = 5 }
            )
        })
    }
} 'xlsx-xsk-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xsk-inspect'
$noList = -not ($tree0.sheets[0].PSObject.Properties.Name -contains 'sparklines')
$hasCount = ($tree0.PSObject.Properties.Name -contains 'sparklineCount') -and ([int]$tree0.sparklineCount -eq 0)
$sheetCount = ($tree0.sheets[0].PSObject.Properties.Name -contains 'sparklineCount') -and ([int]$tree0.sheets[0].sparklineCount -eq 0)
$treeList = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ sparklines = $true }
} 'xlsx-xsk-inspect-list'
$hasList = $treeList.sheets[0].PSObject.Properties.Name -contains 'sparklines'
$listed = @($treeList.sheets[0].sparklines | Where-Object { $_ })
$listEmpty = $hasList -and ($listed.Count -eq 0)
$xsk1ok = $hasCount -and $sheetCount -and $noList -and $listEmpty
Add-Check 'xlsx-Xsk1' $xsk1ok "root=$($tree0.sparklineCount) sheet=$($tree0.sheets[0].sparklineCount) noList=$noList empty=$listEmpty hasList=$hasList"

$insOut = Get-OutPath 'xlsx-sparklines-insert.xlsx'
if (Test-Path $insOut) { Remove-Item -Force $insOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{
        op = 'insertSparkline'
        id = 'sheet:Sheet1'
        ref = 'A1'
        formula = 'B1:F1'
    })
} 'xlsx-xsk-insert' | Out-Null

$excel = Ensure-MsExcel $insOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $insOut
$cap = [string]$wb.Windows.Item(1).Caption
$s = $wb.Worksheets.Item(1)
$sk = Read-SheetSparks $s
$treeIns = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ sparklines = $true }
} 'xlsx-xsk-inspect-inserted'
$g0 = @($treeIns.sheets[0].sparklines | Where-Object { $_ }) | Select-Object -First 1
$insItem = @($g0.sparklines | Where-Object { $_ }) | Select-Object -First 1
$insType = [string]$g0.type
$insFormula = [string]$insItem.formula
$insValues = @($insItem.values | ForEach-Object { [string]$_ }) -join ','
$xsk2ok = ($cap -notmatch 'Repair') -and ($sk.Count -ge 1) -and ($sk.Type -eq 1) -and ($sk.Source -match 'B1:F1') -and ($insType -eq 'line') -and ($insFormula -match 'B1:F1') -and ($insValues -eq '1,2,3,4,5')
Add-Check 'xlsx-Xsk2' $xsk2ok "caption=$cap n=$($sk.Count) type=$($sk.Type) src=$($sk.Source) inspect=$insType $insFormula values=$insValues"
Close-AllWorkbooks $excel

$sid = [string]$g0.id
$setOut = Get-OutPath 'xlsx-sparklines-set.xlsx'
if (Test-Path $setOut) { Remove-Item -Force $setOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $setOut.Replace('\', '/')
    ops = @(@{
        op = 'setSparkline'
        id = $sid
        formula = 'B1:D1'
    })
} 'xlsx-xsk-set' | Out-Null

Close-AllWorkbooks $excel
$wbS = Open-ExcelBook $excel $setOut
$capS = [string]$wbS.Windows.Item(1).Caption
$sS = $wbS.Worksheets.Item(1)
$skS = Read-SheetSparks $sS
$xsk3ok = ($capS -notmatch 'Repair') -and ($skS.Source -match 'B1:D1')
Add-Check 'xlsx-Xsk3' $xsk3ok "caption=$capS src=$($skS.Source)"
Close-AllWorkbooks $excel

$delOut = Get-OutPath 'xlsx-sparklines-delete.xlsx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $setOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteSparkline'; id = $sid })
} 'xlsx-xsk-delete' | Out-Null

Close-AllWorkbooks $excel
$wbD = Open-ExcelBook $excel $delOut
$capD = [string]$wbD.Windows.Item(1).Caption
$sD = $wbD.Worksheets.Item(1)
$skD = Read-SheetSparks $sD
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xsk-inspect-deleted'
$xsk4ok = ($capD -notmatch 'Repair') -and ([int]$skD.Count -eq 0) -and ([int]$treeD.sparklineCount -eq 0)
Add-Check 'xlsx-Xsk4' $xsk4ok "caption=$capD n=$($skD.Count) count=$($treeD.sparklineCount)"
Close-AllWorkbooks $excel

$failOut = Get-OutPath 'xlsx-sparklines-bad.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'insertSparkline'; id = 'sheet:Sheet1' })
} 'xlsx-xsk-missing'
$codeEmpty = $null
try { $codeEmpty = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$xsk5ok = ($fail.ExitCode -ne 0) -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xsk5' $xsk5ok "exit=$($fail.ExitCode) code=$codeEmpty wrote=$(Test-Path $failOut)"
