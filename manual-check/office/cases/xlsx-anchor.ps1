# xlsx 锚点剩余：docs/checklist-xlsx.md 的 Xa1–Xa5。
# 用 create 带图产物再 apply。不算列宽行高。

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

$hostPath = Ensure-XlsxCreatePictures
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-xa-inspect'
$sheet = @($tree0.sheets) | Where-Object { $_.name -eq 'Sheet1' } | Select-Object -First 1
$pics = @($sheet.pictures)
$one = $pics | Where-Object { $_.anchor -eq 'oneCell' } | Select-Object -First 1
$two = $pics | Where-Object { $_.anchor -eq 'twoCell' } | Select-Object -First 1
if (-not $one) { throw 'create 带图产物没有 oneCell' }
if (-not $two) { throw 'create 带图产物没有 twoCell' }

$toOut = Get-OutPath 'xlsx-anchor-to.xlsx'
if (Test-Path $toOut) { Remove-Item -Force $toOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $toOut.Replace('\', '/')
    ops = @(@{
        op = 'setPictureSize'
        id = $two.id
        to = 'G10'
    })
} 'xlsx-xa-to' | Out-Null

$treeTo = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $toOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-xa-to-inspect'
$twoAfter = @($treeTo.sheets[0].pictures) | Where-Object { $_.id -eq $two.id } | Select-Object -First 1
$excel = Ensure-MsExcel $toOut
Close-AllWorkbooks $excel
$wbTo = Open-ExcelBook $excel $toOut
$capTo = [string]$wbTo.Windows.Item(1).Caption
Close-AllWorkbooks $excel
Add-Check 'xlsx-Xa1' (($twoAfter.to -eq 'G10') -and ($twoAfter.anchor -eq 'twoCell') -and ($capTo -notmatch 'Repair')) "to=$($twoAfter.to) caption=$capTo"

$absOut = Get-OutPath 'xlsx-anchor-abs.xlsx'
if (Test-Path $absOut) { Remove-Item -Force $absOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $absOut.Replace('\', '/')
    ops = @(@{
        op = 'setPictureAnchor'
        id = $one.id
        anchor = 'absolute'
        posX = 914400
        posY = 1828800
    })
} 'xlsx-xa-abs' | Out-Null

$treeAbs = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $absOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-xa-abs-inspect'
$oneAbs = @($treeAbs.sheets[0].pictures) | Where-Object { $_.id -eq $one.id } | Select-Object -First 1
Close-AllWorkbooks $excel
$wbAbs = Open-ExcelBook $excel $absOut
$capAbs = [string]$wbAbs.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$xa2ok = ($oneAbs.anchor -eq 'absolute') -and ([int64]$oneAbs.posX -eq 914400) -and ([int64]$oneAbs.posY -eq 1828800) -and ($capAbs -notmatch 'Repair')
Add-Check 'xlsx-Xa2' $xa2ok "anchor=$($oneAbs.anchor) posX=$($oneAbs.posX) caption=$capAbs"

$movedOut = Get-OutPath 'xlsx-anchor-pos.xlsx'
if (Test-Path $movedOut) { Remove-Item -Force $movedOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $absOut.Replace('\', '/')
    output = $movedOut.Replace('\', '/')
    ops = @(@{
        op = 'setPictureAnchor'
        id = $one.id
        posX = 2000000
    })
} 'xlsx-xa-pos' | Out-Null

$treePos = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $movedOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-xa-pos-inspect'
$onePos = @($treePos.sheets[0].pictures) | Where-Object { $_.id -eq $one.id } | Select-Object -First 1
Close-AllWorkbooks $excel
$wbPos = Open-ExcelBook $excel $movedOut
$capPos = [string]$wbPos.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$xa3ok = ($onePos.anchor -eq 'absolute') -and ([int64]$onePos.posX -eq 2000000) -and ([int64]$onePos.posY -eq 1828800) -and ($capPos -notmatch 'Repair')
Add-Check 'xlsx-Xa3' $xa3ok "posX=$($onePos.posX) caption=$capPos"

$missOut = Get-OutPath 'xlsx-anchor-missing.xlsx'
if (Test-Path $missOut) { Remove-Item -Force $missOut }
$miss = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $missOut.Replace('\', '/')
    ops = @(@{
        op = 'setPictureAnchor'
        id = $one.id
        anchor = 'absolute'
    })
} 'xlsx-xa-missing'
$codeMiss = $null
try { $codeMiss = ($miss.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'xlsx-Xa4' (($codeMiss -eq 'InvalidField') -and -not (Test-Path $missOut)) "code=$codeMiss exists=$(Test-Path $missOut)"

$cxOut = Get-OutPath 'xlsx-anchor-cx.xlsx'
if (Test-Path $cxOut) { Remove-Item -Force $cxOut }
$cxFail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $cxOut.Replace('\', '/')
    ops = @(@{
        op = 'setPictureSize'
        id = $two.id
        cx = 1828800
    })
} 'xlsx-xa-cx'
$codeCx = $null
try { $codeCx = ($cxFail.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'xlsx-Xa5' (($codeCx -eq 'InvalidField') -and -not (Test-Path $cxOut)) "code=$codeCx exists=$(Test-Path $cxOut)"
