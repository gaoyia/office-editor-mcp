# xlsx 外部引用：docs/checklist-xlsx.md 的 Xel1–Xel5。
# 用 create 产物再 apply。不打开目标文件求值。编号 Xel，不要和 Xl、Xcn 撞。

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

function Get-ExcelLinkSources($wb) {
    $raw = $null
    try { $raw = $wb.LinkSources(1) } catch { return @() }
    if ($null -eq $raw) { return @() }
    return @($raw)
}

$srcPath = Get-OutPath 'xlsx-extlink-source.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $srcPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 42 })
        })
    }
} 'xlsx-xel-source' | Out-Null

$hostPath = Get-OutPath 'xlsx-extlink-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
        })
    }
} 'xlsx-xel-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xel-inspect-bare'
$noList = -not ($tree0.PSObject.Properties.Name -contains 'externalLinks')
$treeList = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ externalLinks = $true }
} 'xlsx-xel-inspect-list'
$listN = 0
if ($treeList.externalLinks) { $listN = @($treeList.externalLinks).Count }
$xel1ok = ([int]$tree0.externalLinkCount -eq 0) `
    -and $noList `
    -and ($listN -eq 0)
Add-Check 'xlsx-Xel1' $xel1ok "wb=$($tree0.externalLinkCount) noList=$noList listed=$listN"

$setOut = Get-OutPath 'xlsx-extlink-set.xlsx'
if (Test-Path $setOut) { Remove-Item -Force $setOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $setOut.Replace('\', '/')
    ops = @(
        @{
            op = 'insertExternalLink'
            id = 'xlsx:workbook'
            path = $srcPath
            sheets = @('Sheet1')
        }
        @{
            op = 'setCell'
            id = 'cell:Sheet1!B1'
            formula = '=[1]Sheet1!A1'
        }
    )
} 'xlsx-xel-insert' | Out-Null

$excel = Ensure-MsExcel $hostPath
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $setOut
$cap = [string]$wb.Windows.Item(1).Caption
$links = Get-ExcelLinkSources $wb
$linkText = ($links | ForEach-Object { [string]$_ }) -join '|'
$wsSet = $null
$formulaSet = ''
try {
    $wsSet = $wb.Worksheets.Item(1)
    $formulaSet = [string]$wsSet.Range('B1').Formula
} finally {
    if ($wsSet) { Release-Com $wsSet }
}
Close-AllWorkbooks $excel
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $setOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ externalLinks = $true }
} 'xlsx-xel-inspect-set'
$item = $null
if ($treeS.externalLinks) { $item = @($treeS.externalLinks) | Select-Object -First 1 }
$srcName = [IO.Path]::GetFileName($srcPath)
$pathOk = $false
if ($item) { $pathOk = ([string]$item.path) -match [regex]::Escape($srcName) }
$linkOk = ($linkText -match [regex]::Escape($srcName)) `
    -or ($formulaSet -match [regex]::Escape($srcName))
$xel2ok = ($cap -notmatch 'Repair') `
    -and ([int]$treeS.externalLinkCount -eq 1) `
    -and ($null -ne $item) `
    -and ([string]$item.id -eq 'extlink:1') `
    -and ([string]$item.kind -eq 'book') `
    -and $pathOk `
    -and $linkOk
Add-Check 'xlsx-Xel2' $xel2ok "caption=$cap links=$linkText formula=$formulaSet id=$($item.id) kind=$($item.kind) path=$($item.path)"

$formulaOut = Get-OutPath 'xlsx-extlink-formula.xlsx'
if (Test-Path $formulaOut) { Remove-Item -Force $formulaOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $setOut.Replace('\', '/')
    output = $formulaOut.Replace('\', '/')
    ops = @(@{
        op = 'setCell'
        id = 'cell:Sheet1!B1'
        formula = '=[1]Sheet1!A1'
    })
} 'xlsx-xel-formula' | Out-Null

Close-AllWorkbooks $excel
$wbF = Open-ExcelBook $excel $formulaOut
$capF = [string]$wbF.Windows.Item(1).Caption
$ws = $null
$formula = ''
try {
    $ws = $wbF.Worksheets.Item(1)
    $formula = [string]$ws.Range('B1').Formula
} finally {
    if ($ws) { Release-Com $ws }
}
Close-AllWorkbooks $excel
$broke = Get-OutPath 'xlsx-extlink-inuse.xlsx'
if (Test-Path $broke) { Remove-Item -Force $broke }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $formulaOut.Replace('\', '/')
    output = $broke.Replace('\', '/')
    ops = @(@{ op = 'deleteExternalLink'; id = 'extlink:1' })
} 'xlsx-xel-inuse' | Out-Null
$treeB = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $broke.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'B1'; externalLinks = $true }
} 'xlsx-xel-inspect-broke'
$cellB = $null
if ($treeB.sheets) { $cellB = @($treeB.sheets[0].cells) | Select-Object -First 1 }
Close-AllWorkbooks $excel
$wbB = Open-ExcelBook $excel $broke
$capB = [string]$wbB.Windows.Item(1).Caption
$linksB = @(Get-ExcelLinkSources $wbB)
$valB = $null
$formulaB = ''
$wsB = $null
try {
    $wsB = $wbB.Worksheets.Item(1)
    $valB = $wsB.Range('B1').Value2
    $formulaB = [string]$wsB.Range('B1').Formula
} finally {
    if ($wsB) { Release-Com $wsB }
}
Close-AllWorkbooks $excel
$xel3ok = ($capF -notmatch 'Repair') `
    -and ($formula -match "Sheet1'!A1" -or $formula -match 'Sheet1!A1') `
    -and ($capB -notmatch 'Repair') `
    -and ([int]$treeB.externalLinkCount -eq 0) `
    -and ($linksB.Count -eq 0) `
    -and ([string]$valB -eq '0') `
    -and ($formulaB -notmatch '\[1\]')
Add-Check 'xlsx-Xel3' $xel3ok "before=$formula caption=$capB links=$($linksB.Count) count=$($treeB.externalLinkCount) value=$valB formula=$formulaB cell=$($cellB.value)"

$cleared = Get-OutPath 'xlsx-extlink-cleared.xlsx'
if (Test-Path $cleared) { Remove-Item -Force $cleared }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $formulaOut.Replace('\', '/')
    output = $cleared.Replace('\', '/')
    ops = @(@{ op = 'setCell'; id = 'cell:Sheet1!B1'; value = $null })
} 'xlsx-xel-clear' | Out-Null
$delOut = Get-OutPath 'xlsx-extlink-clear.xlsx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $cleared.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteExternalLink'; id = 'extlink:1' })
} 'xlsx-xel-delete' | Out-Null
Close-AllWorkbooks $excel
$wbD = Open-ExcelBook $excel $delOut
$capD = [string]$wbD.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ externalLinks = $true }
} 'xlsx-xel-inspect-deleted'
$xel4ok = ($capD -notmatch 'Repair') `
    -and ([int]$treeD.externalLinkCount -eq 0)
Add-Check 'xlsx-Xel4' $xel4ok "caption=$capD count=$($treeD.externalLinkCount)"

$failOut = Get-OutPath 'xlsx-extlink-bad.xlsx'
$failPath = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'insertExternalLink'
        id = 'xlsx:workbook'
    })
} 'xlsx-xel-missing-path'
$codePath = $null
try { $codePath = ($failPath.Stderr | ConvertFrom-Json).error.code } catch {}
$failSlide = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'insertExternalLink'
        id = 'slide:1'
        path = 'source.xlsx'
    })
} 'xlsx-xel-slide-id'
$codeSlide = $null
try { $codeSlide = ($failSlide.Stderr | ConvertFrom-Json).error.code } catch {}
$xel5ok = ($failPath.ExitCode -ne 0) -and ($codePath -eq 'InvalidField') `
    -and ($failSlide.ExitCode -ne 0) -and ($codeSlide -eq 'InvalidId') `
    -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xel5' $xel5ok "path=$codePath slide=$codeSlide wrote=$(Test-Path $failOut)"
