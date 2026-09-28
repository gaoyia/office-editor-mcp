# xlsx 超链接：docs/checklist-xlsx.md 的 Xhl1–Xhl5。
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

function Read-SheetLinks($sheet) {
    $n = 0
    $addr = ''
    $text = ''
    $col = $null
    try { $col = $sheet.Hyperlinks } catch { }
    if ($null -eq $col) {
        return @{ Count = 0; Address = ''; Text = '' }
    }
    try { $n = [int]$col.Count } catch { }
    if ($n -ge 1) {
        $h = $null
        try { $h = $col.Item(1) } catch { }
        if ($null -ne $h) {
            try { $addr = [string]$h.Address } catch { }
            try { $text = [string]$h.TextToDisplay } catch { }
            Release-Com $h
        }
    }
    Release-Com $col
    return @{ Count = $n; Address = $addr; Text = $text }
}

$hostPath = Get-OutPath 'xlsx-hyperlinks-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 'Xhl链' })
        })
    }
} 'xlsx-xhl-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xhl-inspect'
$noList = -not ($tree0.sheets[0].PSObject.Properties.Name -contains 'hyperlinks')
$hasCount = ($tree0.PSObject.Properties.Name -contains 'hyperlinkCount') -and ([int]$tree0.hyperlinkCount -eq 0)
$sheetCount = ($tree0.sheets[0].PSObject.Properties.Name -contains 'hyperlinkCount') -and ([int]$tree0.sheets[0].hyperlinkCount -eq 0)
$treeList = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ hyperlinks = $true }
} 'xlsx-xhl-inspect-list'
$hasList = $treeList.sheets[0].PSObject.Properties.Name -contains 'hyperlinks'
$listed = @($treeList.sheets[0].hyperlinks | Where-Object { $_ })
$listEmpty = $hasList -and ($listed.Count -eq 0)
$xhl1ok = $hasCount -and $sheetCount -and $noList -and $listEmpty
Add-Check 'xlsx-Xhl1' $xhl1ok "root=$($tree0.hyperlinkCount) sheet=$($tree0.sheets[0].hyperlinkCount) noList=$noList empty=$listEmpty hasList=$hasList"

$insOut = Get-OutPath 'xlsx-hyperlinks-insert.xlsx'
if (Test-Path $insOut) { Remove-Item -Force $insOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{
        op = 'insertHyperlink'
        id = 'sheet:Sheet1'
        ref = 'A1'
        url = 'https://example.com/xhl'
    })
} 'xlsx-xhl-insert' | Out-Null

$excel = Ensure-MsExcel $insOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $insOut
$cap = [string]$wb.Windows.Item(1).Caption
$s = $wb.Worksheets.Item(1)
$hl = Read-SheetLinks $s
$treeIns = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ hyperlinks = $true }
} 'xlsx-xhl-inspect-inserted'
$insUrl = [string]((@($treeIns.sheets[0].hyperlinks | Where-Object { $_ }) | Select-Object -First 1).url)
$xhl2ok = ($cap -notmatch 'Repair') -and ($hl.Address -match 'example.com/xhl') -and ($insUrl -eq 'https://example.com/xhl')
Add-Check 'xlsx-Xhl2' $xhl2ok "caption=$cap addr=$($hl.Address) n=$($hl.Count) inspect=$insUrl"
Close-AllWorkbooks $excel

$lid = [string]((@($treeIns.sheets[0].hyperlinks | Where-Object { $_ }) | Select-Object -First 1).id)
$setOut = Get-OutPath 'xlsx-hyperlinks-set.xlsx'
if (Test-Path $setOut) { Remove-Item -Force $setOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $setOut.Replace('\', '/')
    ops = @(@{
        op = 'setHyperlink'
        id = $lid
        url = 'https://example.org/xhl2'
    })
} 'xlsx-xhl-set' | Out-Null

Close-AllWorkbooks $excel
$wbS = Open-ExcelBook $excel $setOut
$capS = [string]$wbS.Windows.Item(1).Caption
$sS = $wbS.Worksheets.Item(1)
$hlS = Read-SheetLinks $sS
$xhl3ok = ($capS -notmatch 'Repair') -and ($hlS.Address -match 'example.org/xhl2')
Add-Check 'xlsx-Xhl3' $xhl3ok "caption=$capS addr=$($hlS.Address)"
Close-AllWorkbooks $excel

$delOut = Get-OutPath 'xlsx-hyperlinks-delete.xlsx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $setOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteHyperlink'; id = $lid })
} 'xlsx-xhl-delete' | Out-Null

Close-AllWorkbooks $excel
$wbD = Open-ExcelBook $excel $delOut
$capD = [string]$wbD.Windows.Item(1).Caption
$sD = $wbD.Worksheets.Item(1)
$hlD = Read-SheetLinks $sD
$cellText = ''
try { $cellText = [string]$sD.Range('A1').Text } catch { }
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xhl-inspect-deleted'
$xhl4ok = ($capD -notmatch 'Repair') -and ([int]$hlD.Count -eq 0) -and ($cellText -match 'Xhl链') -and ([int]$treeD.hyperlinkCount -eq 0)
Add-Check 'xlsx-Xhl4' $xhl4ok "caption=$capD n=$($hlD.Count) text=$cellText count=$($treeD.hyperlinkCount)"
Close-AllWorkbooks $excel

$failOut = Get-OutPath 'xlsx-hyperlinks-bad.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'insertHyperlink'; id = 'sheet:Sheet1' })
} 'xlsx-xhl-missing'
$codeEmpty = $null
try { $codeEmpty = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$xhl5ok = ($fail.ExitCode -ne 0) -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xhl5' $xhl5ok "exit=$($fail.ExitCode) code=$codeEmpty wrote=$(Test-Path $failOut)"
