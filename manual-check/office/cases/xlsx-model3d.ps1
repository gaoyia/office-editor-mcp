# xlsx 工作表 3D 模型：docs/checklist-xlsx.md 的 X3d1–X3d6。
# COM 认 Type=30（mso3DModel）。探测脚本不进本文件。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }
Add-Type -AssemblyName System.IO.Compression.FileSystem

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

function Get-ExcelModel3dCount($sheet) {
    $n = 0
    $shapes = $sheet.Shapes
    try {
        for ($i = 1; $i -le $shapes.Count; $i++) {
            $sh = $shapes.Item($i)
            try {
                if ([int]$sh.Type -eq 30) { $n++ }
            } finally { Release-Com $sh }
        }
    } finally { Release-Com $shapes }
    return $n
}

function Test-BytesEqual([byte[]]$a, [byte[]]$b) {
    if ($a.Length -ne $b.Length) { return $false }
    for ($i = 0; $i -lt $a.Length; $i++) {
        if ($a[$i] -ne $b[$i]) { return $false }
    }
    return $true
}

$glb = Ensure-OoxpGlb
$glbB = Ensure-OoxpGlbB
$glbFile = $glb.Replace('\', '/')
$glbBFile = $glbB.Replace('\', '/')

$src = Get-OutPath 'xlsx-model3d-src.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-model3d-create-src' | Out-Null

$excel = Ensure-MsExcel $src
Close-AllWorkbooks $excel

$inserted = Get-OutPath 'xlsx-model3d-inserted.xlsx'
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
} 'xlsx-model3d-insert' | Out-Null
$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'xlsx'
    scope = @{ model3ds = $true; pictures = $true }
} 'xlsx-model3d-inspect-insert'
$item = @($tree1.sheets[0].model3ds) | Select-Object -First 1
$picCount = @($tree1.sheets[0].pictures).Count
$zipI = [IO.Compression.ZipFile]::OpenRead($inserted)
try {
    $glbPart = @($zipI.Entries | Where-Object { $_.FullName -like '*.glb' }).Count -gt 0
} finally { $zipI.Dispose() }
$wb1 = Open-ExcelBook $excel $inserted
$cap1 = [string]$wb1.Windows.Item(1).Caption
$s1 = $wb1.Worksheets.Item('Sheet1')
$n1 = Get-ExcelModel3dCount $s1
Release-Com $s1
Add-Check 'xlsx-X3d1' (
    ($item.contentType -eq 'model/gltf.binary') -and ($picCount -eq 0) -and ($cap1 -notmatch 'Repair') -and ($n1 -ge 1) -and $glbPart
) "ct=$($item.contentType) caption=$cap1 n=$n1 pics=$picCount glb=$glbPart"
Close-AllWorkbooks $excel

$exported = Get-OutPath 'xlsx-model3d-export.glb'
$expJson = (Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $inserted.Replace('\', '/')
    id = [string]$item.id
    output = $exported.Replace('\', '/')
} 'xlsx-model3d-export') | ConvertFrom-Json
$expCt = [string]$expJson.contentType
$same = Test-BytesEqual ([IO.File]::ReadAllBytes($exported)) ([IO.File]::ReadAllBytes($glb))
Add-Check 'xlsx-X3d2' (($expCt -eq 'model/gltf.binary') -and $same) "ct=$expCt same=$same"

$created = Get-OutPath 'xlsx-model3d-created.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $created.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
            model3ds = @(@{ file = $glbFile; from = 'B2' })
        })
    }
} 'xlsx-model3d-create' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $created.Replace('\', '/'); format = 'xlsx'
    scope = @{ model3ds = $true; pictures = $true }
} 'xlsx-model3d-inspect-create'
$itemC = @($treeC.sheets[0].model3ds) | Select-Object -First 1
$picC = @($treeC.sheets[0].pictures).Count
Close-AllWorkbooks $excel
$wbC = Open-ExcelBook $excel $created
$capC = [string]$wbC.Windows.Item(1).Caption
Add-Check 'xlsx-X3d3' (
    ($itemC.contentType -eq 'model/gltf.binary') -and ($picC -eq 0) -and ($capC -notmatch 'Repair')
) "ct=$($itemC.contentType) pics=$picC caption=$capC"
Close-AllWorkbooks $excel

$replaced = Get-OutPath 'xlsx-model3d-replaced.xlsx'
if (Test-Path $replaced) { Remove-Item $replaced -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{ op = 'replaceModel3d'; id = [string]$item.id; file = $glbBFile })
} 'xlsx-model3d-replace' | Out-Null
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'xlsx'
    scope = @{ model3ds = $true }
} 'xlsx-model3d-inspect-replace'
$itemR = @($treeR.sheets[0].model3ds) | Select-Object -First 1
$expR = Get-OutPath 'xlsx-model3d-export-b.glb'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $replaced.Replace('\', '/')
    id = [string]$itemR.id
    output = $expR.Replace('\', '/')
} 'xlsx-model3d-export-b' | Out-Null
$sameB = Test-BytesEqual ([IO.File]::ReadAllBytes($expR)) ([IO.File]::ReadAllBytes($glbB))
$wbR = Open-ExcelBook $excel $replaced
$capR = [string]$wbR.Windows.Item(1).Caption
Add-Check 'xlsx-X3d4' (
    ([string]$itemR.id -eq [string]$item.id) -and $sameB -and ($capR -notmatch 'Repair')
) "id=$($itemR.id) sameB=$sameB caption=$capR"
Close-AllWorkbooks $excel

$deleted = Get-OutPath 'xlsx-model3d-deleted.xlsx'
if (Test-Path $deleted) { Remove-Item $deleted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $replaced.Replace('\', '/')
    output = $deleted.Replace('\', '/')
    ops = @(@{ op = 'deleteModel3d'; id = [string]$item.id })
} 'xlsx-model3d-delete' | Out-Null
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $deleted.Replace('\', '/'); format = 'xlsx'
    scope = @{ model3ds = $true }
} 'xlsx-model3d-inspect-delete'
$zipD = [IO.Compression.ZipFile]::OpenRead($deleted)
try {
    $glbLeft = @($zipD.Entries | Where-Object { $_.FullName -like '*.glb' }).Count -gt 0
} finally { $zipD.Dispose() }
$wbD = Open-ExcelBook $excel $deleted
$capD = [string]$wbD.Windows.Item(1).Caption
Add-Check 'xlsx-X3d5' (
    ([int]$treeD.model3dCount -eq 0) -and (-not $glbLeft) -and ($capD -notmatch 'Repair')
) "count=$($treeD.model3dCount) glbLeft=$glbLeft caption=$capD"
Close-AllWorkbooks $excel

$junk = Get-OutPath 'xlsx-model3d-not-glb.bin'
[IO.File]::WriteAllBytes($junk, [byte[]](1, 2, 3, 4))
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = (Get-OutPath 'xlsx-model3d-should-not.xlsx').Replace('\', '/')
    ops = @(@{ op = 'insertModel3d'; id = 'sheet:Sheet1'; file = $junk.Replace('\', '/'); from = 'B2' })
} 'xlsx-model3d-reject'
$failCode = $null
try { $failCode = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'xlsx-X3d6' (($fail.ExitCode -ne 0) -and ($failCode -eq 'InvalidField')) "code=$failCode exit=$($fail.ExitCode)"
