# xlsx 工作表 3D 事后几何：docs/checklist-xlsx.md 的 X3g1–X3g6。
# 编号 X3g，不要和 X3d 撞。探测脚本不进本文件。不要 Stop-Process Excel。

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

function Get-ExcelModel3dShape($sheet) {
    $shapes = $sheet.Shapes
    $found = $null
    try {
        for ($i = 1; $i -le $shapes.Count; $i++) {
            $sh = $shapes.Item($i)
            if ([int]$sh.Type -eq 30) {
                $found = $sh
                break
            }
            Release-Com $sh
        }
    } finally { Release-Com $shapes }
    return $found
}

$glb = Ensure-OoxpGlb
$glbB = Ensure-OoxpGlbB
$glbFile = $glb.Replace('\', '/')
$glbBFile = $glbB.Replace('\', '/')

$src = Get-OutPath 'xlsx-model3d-geom-src.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-model3d-geom-create-src' | Out-Null

$excel = Ensure-MsExcel $src
Close-AllWorkbooks $excel

$inserted = Get-OutPath 'xlsx-model3d-geom-inserted.xlsx'
Close-AllWorkbooks $excel
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{
        op = 'insertModel3d'
        id = 'sheet:Sheet1'
        file = $glbFile
        from = 'B2'
    })
} 'xlsx-model3d-geom-insert' | Out-Null

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'xlsx'
    scope = @{ model3ds = $true }
} 'xlsx-model3d-geom-inspect-insert'
$item = @($tree1.sheets[0].model3ds) | Select-Object -First 1
$id = [string]$item.id

$sized = Get-OutPath 'xlsx-model3d-geom-sized.xlsx'
Close-AllWorkbooks $excel
if (Test-Path $sized) { Remove-Item $sized -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $sized.Replace('\', '/')
    ops = @(@{ op = 'setModel3dSize'; id = $id; cx = 3657600; cy = 1828800 })
} 'xlsx-model3d-geom-size' | Out-Null

$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sized.Replace('\', '/'); format = 'xlsx'
    scope = @{ model3ds = $true }
} 'xlsx-model3d-geom-inspect-size'
$itemS = @($treeS.sheets[0].model3ds) | Select-Object -First 1
$wbS = Open-ExcelBook $excel $sized
$capS = [string]$wbS.Windows.Item(1).Caption
$shS = $wbS.Worksheets.Item(1)
$modelS = Get-ExcelModel3dShape $shS
$wPt = $null
$hPt = $null
if ($null -ne $modelS) {
    $wPt = [math]::Round([double]$modelS.Width, 2)
    $hPt = [math]::Round([double]$modelS.Height, 2)
    Release-Com $modelS
}
Release-Com $shS
Add-Check 'xlsx-X3g1' (
    ([int]$itemS.cx -eq 3657600) -and ([int]$itemS.cy -eq 1828800) -and
    ($capS -notmatch 'Repair') -and ($null -ne $wPt) -and ([math]::Abs($wPt - 288) -lt 2) -and
    ($null -ne $hPt) -and ([math]::Abs($hPt - 144) -lt 2)
) "cx=$($itemS.cx) cy=$($itemS.cy) caption=$capS w=$wPt h=$hPt"
Close-AllWorkbooks $excel

$moved = Get-OutPath 'xlsx-model3d-geom-moved.xlsx'
Close-AllWorkbooks $excel
if (Test-Path $moved) { Remove-Item $moved -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sized.Replace('\', '/')
    output = $moved.Replace('\', '/')
    ops = @(@{ op = 'setModel3dAnchor'; id = $id; from = 'C3' })
} 'xlsx-model3d-geom-anchor' | Out-Null

$treeM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $moved.Replace('\', '/'); format = 'xlsx'
    scope = @{ model3ds = $true }
} 'xlsx-model3d-geom-inspect-anchor'
$itemM = @($treeM.sheets[0].model3ds) | Select-Object -First 1
$wbM = Open-ExcelBook $excel $moved
$capM = [string]$wbM.Windows.Item(1).Caption
Add-Check 'xlsx-X3g2' (
    ([string]$itemM.from -eq 'C3') -and ($capM -notmatch 'Repair')
) "from=$($itemM.from) caption=$capM"
Close-AllWorkbooks $excel

$tcSrc = Get-OutPath 'xlsx-model3d-geom-tc.xlsx'
Close-AllWorkbooks $excel
if (Test-Path $tcSrc) { Remove-Item $tcSrc -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $tcSrc.Replace('\', '/')
    ops = @(@{
        op = 'insertModel3d'
        id = 'sheet:Sheet1'
        file = $glbFile
        anchor = 'twoCell'
        from = 'A1'
        to = 'C5'
    })
} 'xlsx-model3d-geom-tc-insert' | Out-Null
$treeT0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tcSrc.Replace('\', '/'); format = 'xlsx'
    scope = @{ model3ds = $true }
} 'xlsx-model3d-geom-inspect-tc-ins'
$tcId = [string](@($treeT0.sheets[0].model3ds) | Select-Object -First 1).id
$tcOut = Get-OutPath 'xlsx-model3d-geom-tc-to.xlsx'
Close-AllWorkbooks $excel
if (Test-Path $tcOut) { Remove-Item $tcOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $tcSrc.Replace('\', '/')
    output = $tcOut.Replace('\', '/')
    ops = @(@{ op = 'setModel3dSize'; id = $tcId; to = 'D8' })
} 'xlsx-model3d-geom-tc-size' | Out-Null
$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tcOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ model3ds = $true }
} 'xlsx-model3d-geom-inspect-tc'
$itemT = @($treeT.sheets[0].model3ds) | Select-Object -First 1
$wbT = Open-ExcelBook $excel $tcOut
$capT = [string]$wbT.Windows.Item(1).Caption
Add-Check 'xlsx-X3g3' (
    ([string]$itemT.to -eq 'D8') -and ($capT -notmatch 'Repair')
) "to=$($itemT.to) caption=$capT"
Close-AllWorkbooks $excel

$replaced = Get-OutPath 'xlsx-model3d-geom-replaced.xlsx'
Close-AllWorkbooks $excel
if (Test-Path $replaced) { Remove-Item $replaced -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sized.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{ op = 'replaceModel3d'; id = $id; file = $glbBFile })
} 'xlsx-model3d-geom-replace' | Out-Null
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'xlsx'
    scope = @{ model3ds = $true }
} 'xlsx-model3d-geom-inspect-replace'
$itemR = @($treeR.sheets[0].model3ds) | Select-Object -First 1
$wbR = Open-ExcelBook $excel $replaced
$capR = [string]$wbR.Windows.Item(1).Caption
Add-Check 'xlsx-X3g4' (
    ([int]$itemR.cx -eq 3657600) -and ([int]$itemR.cy -eq 1828800) -and ($capR -notmatch 'Repair')
) "cx=$($itemR.cx) cy=$($itemR.cy) caption=$capR"
Close-AllWorkbooks $excel

$failPic = Get-OutPath 'xlsx-model3d-geom-fail-pic.xlsx'
$rawPic = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failPic.Replace('\', '/')
    ops = @(@{ op = 'setPictureSize'; id = $id; cx = 1000; cy = 1000 })
} 'xlsx-model3d-geom-fail-pic'
Add-Check 'xlsx-X3g5' (
    (Get-FailCode $rawPic) -eq 'InvalidId'
) "pic=$(Get-FailCode $rawPic)"

$failCx = Get-OutPath 'xlsx-model3d-geom-fail-cx.xlsx'
$rawCx = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $tcSrc.Replace('\', '/')
    output = $failCx.Replace('\', '/')
    ops = @(@{ op = 'setModel3dSize'; id = $tcId; cx = 1000 })
} 'xlsx-model3d-geom-fail-cx'
$failWrap = Get-OutPath 'xlsx-model3d-geom-fail-wrap.xlsx'
$rawWrap = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failWrap.Replace('\', '/')
    ops = @(@{ op = 'setModel3dWrap'; id = $id; kind = 'inline' })
} 'xlsx-model3d-geom-fail-wrap'
Add-Check 'xlsx-X3g6' (
    ((Get-FailCode $rawCx) -eq 'InvalidField') -and
    ((Get-FailCode $rawWrap) -eq 'InvalidField')
) "cx=$(Get-FailCode $rawCx) wrap=$(Get-FailCode $rawWrap)"
