# xlsx 工作表公式框事后几何：docs/checklist-xlsx.md 的 Xmg1–Xmg5。
# 编号 Xmg，不要和 Xm、Xom 撞。探测脚本不进本文件。不要 Stop-Process Excel。

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

$src = Get-OutPath 'xlsx-math-geom-src.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-math-geom-create-src' | Out-Null

$excel = Ensure-MsExcel $src
Close-AllWorkbooks $excel

$inserted = Get-OutPath 'xlsx-math-geom-inserted.xlsx'
Close-AllWorkbooks $excel
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{
        op = 'insertMath'
        id = 'sheet:Sheet1'
        text = 'E=mc^2'
        from = 'B2'
    })
} 'xlsx-math-geom-insert' | Out-Null
$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'xlsx'
    scope = @{ maths = $true }
} 'xlsx-math-geom-inspect-insert'
$item = @($tree1.sheets[0].maths) | Select-Object -First 1
$id = [string]$item.id

$sized = Get-OutPath 'xlsx-math-geom-sized.xlsx'
Close-AllWorkbooks $excel
if (Test-Path $sized) { Remove-Item $sized -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $sized.Replace('\', '/')
    ops = @(@{ op = 'setMathSize'; id = $id; cx = 1828800; cy = 914400 })
} 'xlsx-math-geom-size' | Out-Null
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sized.Replace('\', '/'); format = 'xlsx'
    scope = @{ maths = $true }
} 'xlsx-math-geom-inspect-size'
$itemS = @($treeS.sheets[0].maths) | Select-Object -First 1
$wbS = Open-ExcelBook $excel $sized
$capS = [string]$wbS.Windows.Item(1).Caption
Add-Check 'xlsx-Xmg1' (
    ([int]$itemS.cx -eq 1828800) -and ([int]$itemS.cy -eq 914400) -and ($capS -notmatch 'Repair')
) "cx=$($itemS.cx) cy=$($itemS.cy) caption=$capS"
Close-AllWorkbooks $excel

$moved = Get-OutPath 'xlsx-math-geom-moved.xlsx'
Close-AllWorkbooks $excel
if (Test-Path $moved) { Remove-Item $moved -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sized.Replace('\', '/')
    output = $moved.Replace('\', '/')
    ops = @(@{ op = 'setMathAnchor'; id = $id; from = 'C3' })
} 'xlsx-math-geom-anchor' | Out-Null
$treeM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $moved.Replace('\', '/'); format = 'xlsx'
    scope = @{ maths = $true }
} 'xlsx-math-geom-inspect-anchor'
$itemM = @($treeM.sheets[0].maths) | Select-Object -First 1
$wbM = Open-ExcelBook $excel $moved
$capM = [string]$wbM.Windows.Item(1).Caption
Add-Check 'xlsx-Xmg2' (
    ([string]$itemM.from -eq 'C3') -and ([int]$itemM.cx -eq 1828800) -and ($capM -notmatch 'Repair')
) "from=$($itemM.from) cx=$($itemM.cx) caption=$capM"
Close-AllWorkbooks $excel

$tcSrc = Get-OutPath 'xlsx-math-geom-tc.xlsx'
Close-AllWorkbooks $excel
if (Test-Path $tcSrc) { Remove-Item $tcSrc -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $tcSrc.Replace('\', '/')
    ops = @(@{
        op = 'insertMath'
        id = 'sheet:Sheet1'
        text = 'E=mc^2'
        anchor = 'twoCell'
        from = 'A1'
        to = 'C5'
    })
} 'xlsx-math-geom-tc-insert' | Out-Null
$treeT0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tcSrc.Replace('\', '/'); format = 'xlsx'
    scope = @{ maths = $true }
} 'xlsx-math-geom-inspect-tc-ins'
$tcId = [string](@($treeT0.sheets[0].maths) | Select-Object -First 1).id
$tcOut = Get-OutPath 'xlsx-math-geom-tc-to.xlsx'
Close-AllWorkbooks $excel
if (Test-Path $tcOut) { Remove-Item $tcOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $tcSrc.Replace('\', '/')
    output = $tcOut.Replace('\', '/')
    ops = @(@{ op = 'setMathSize'; id = $tcId; to = 'D8' })
} 'xlsx-math-geom-tc-size' | Out-Null
$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tcOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ maths = $true }
} 'xlsx-math-geom-inspect-tc'
$itemT = @($treeT.sheets[0].maths) | Select-Object -First 1
$wbT = Open-ExcelBook $excel $tcOut
$capT = [string]$wbT.Windows.Item(1).Caption
Add-Check 'xlsx-Xmg3' (
    ([string]$itemT.to -eq 'D8') -and ($capT -notmatch 'Repair')
) "to=$($itemT.to) caption=$capT"
Close-AllWorkbooks $excel

$replaced = Get-OutPath 'xlsx-math-geom-replaced.xlsx'
Close-AllWorkbooks $excel
if (Test-Path $replaced) { Remove-Item $replaced -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $moved.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{ op = 'setMath'; id = $id; text = 'a^2+b^2=c^2' })
} 'xlsx-math-geom-set' | Out-Null
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'xlsx'
    scope = @{ maths = $true }
} 'xlsx-math-geom-inspect-set'
$itemR = @($treeR.sheets[0].maths) | Select-Object -First 1
$wbR = Open-ExcelBook $excel $replaced
$capR = [string]$wbR.Windows.Item(1).Caption
$failTb = Get-OutPath 'xlsx-math-geom-fail-tb.xlsx'
$rawTb = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $replaced.Replace('\', '/')
    output = $failTb.Replace('\', '/')
    ops = @(@{ op = 'setTextBoxAnchor'; id = $id; from = 'D4' })
} 'xlsx-math-geom-fail-tb'
Add-Check 'xlsx-Xmg4' (
    ([string]$itemR.text -eq 'a^2+b^2=c^2') -and
    ([int]$itemR.cx -eq 1828800) -and ([int]$itemR.cy -eq 914400) -and
    ([string]$itemR.from -eq 'C3') -and
    ($capR -notmatch 'Repair') -and
    ((Get-FailCode $rawTb) -eq 'InvalidId')
) "text=$($itemR.text) from=$($itemR.from) cx=$($itemR.cx) tb=$(Get-FailCode $rawTb) caption=$capR"
Close-AllWorkbooks $excel

$failCx = Get-OutPath 'xlsx-math-geom-fail-cx.xlsx'
$rawCx = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $tcSrc.Replace('\', '/')
    output = $failCx.Replace('\', '/')
    ops = @(@{ op = 'setMathSize'; id = $tcId; cx = 1000 })
} 'xlsx-math-geom-fail-cx'
$failPos = Get-OutPath 'xlsx-math-geom-fail-pos.xlsx'
$rawPos = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failPos.Replace('\', '/')
    ops = @(@{ op = 'setMathPos'; id = $id; x = 1000; y = 1000 })
} 'xlsx-math-geom-fail-pos'
Add-Check 'xlsx-Xmg5' (
    ((Get-FailCode $rawCx) -eq 'InvalidField') -and
    ((Get-FailCode $rawPos) -eq 'InvalidField') -and
    (-not (Test-Path $failCx)) -and (-not (Test-Path $failPos))
) "cx=$(Get-FailCode $rawCx) pos=$(Get-FailCode $rawPos)"
