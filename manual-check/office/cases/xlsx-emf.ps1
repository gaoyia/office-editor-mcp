# xlsx 嵌入 EMF / WMF / TIFF：docs/checklist-xlsx.md 的 Xem1–Xem6。
# COM 认 Type=13（msoPicture）。探测脚本不进本文件。

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

function Get-ExcelPictureCount($sheet) {
    $n = 0
    $shapes = $sheet.Shapes
    try {
        for ($i = 1; $i -le $shapes.Count; $i++) {
            $sh = $shapes.Item($i)
            try {
                if ([int]$sh.Type -eq 13) { $n++ }
            } finally { Release-Com $sh }
        }
    } finally { Release-Com $shapes }
    return $n
}

$emf = Ensure-OoxpEmf
$wmf = Ensure-OoxpWmf
$tiff = Ensure-OoxpTiff
$pngB = Ensure-OoxpPngB
$emfFile = $emf.Replace('\', '/')
$wmfFile = $wmf.Replace('\', '/')
$tiffFile = $tiff.Replace('\', '/')

$src = Get-OutPath 'xlsx-emf-src.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-emf-create-src' | Out-Null

$excel = Ensure-MsExcel $src
Close-AllWorkbooks $excel

$inserted = Get-OutPath 'xlsx-emf-inserted.xlsx'
Close-AllWorkbooks $excel
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{
        op = 'insertPicture'
        id = 'sheet:Sheet1'
        file = $emfFile
        from = 'B2'
    })
} 'xlsx-emf-insert' | Out-Null
$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-emf-inspect-insert'
$pic = @($tree1.sheets[0].pictures) | Select-Object -First 1
$zipI = [IO.Compression.ZipFile]::OpenRead($inserted)
try {
    $emfPart = @($zipI.Entries | Where-Object { $_.FullName -like '*.emf' }).Count -gt 0
} finally { $zipI.Dispose() }
$wb1 = Open-ExcelBook $excel $inserted
$cap1 = [string]$wb1.Windows.Item(1).Caption
$s1 = $wb1.Worksheets.Item('Sheet1')
$n1 = Get-ExcelPictureCount $s1
Release-Com $s1
Add-Check 'xlsx-Xem1' (
    ($pic.contentType -eq 'image/x-emf') -and ($cap1 -notmatch 'Repair') -and ($n1 -ge 1) -and $emfPart
) "ct=$($pic.contentType) caption=$cap1 n=$n1 emf=$emfPart"
Close-AllWorkbooks $excel

$exported = Get-OutPath 'xlsx-emf-export.emf'
$expJson = (Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $inserted.Replace('\', '/')
    id = [string]$pic.id
    output = $exported.Replace('\', '/')
} 'xlsx-emf-export') | ConvertFrom-Json
$expCt = [string]$expJson.contentType
$expBytes = [IO.File]::ReadAllBytes($exported)
$srcBytes = [IO.File]::ReadAllBytes($emf)
$same = ($expBytes.Length -eq $srcBytes.Length)
if ($same) {
    for ($i = 0; $i -lt $expBytes.Length; $i++) {
        if ($expBytes[$i] -ne $srcBytes[$i]) { $same = $false; break }
    }
}
Add-Check 'xlsx-Xem2' (($expCt -eq 'image/x-emf') -and $same) "ct=$expCt len=$($expBytes.Length) same=$same"

$created = Get-OutPath 'xlsx-emf-created.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $created.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
            pictures = @(@{ file = $wmfFile; from = 'B2' })
        })
    }
} 'xlsx-emf-create' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $created.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-emf-inspect-create'
$picC = @($treeC.sheets[0].pictures) | Select-Object -First 1
Close-AllWorkbooks $excel
$wbC = Open-ExcelBook $excel $created
$capC = [string]$wbC.Windows.Item(1).Caption
Add-Check 'xlsx-Xem3' (
    ($picC.contentType -eq 'image/x-wmf') -and ($capC -notmatch 'Repair')
) "ct=$($picC.contentType) caption=$capC"
Close-AllWorkbooks $excel

$replaced = Get-OutPath 'xlsx-emf-replaced.xlsx'
if (Test-Path $replaced) { Remove-Item $replaced -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{ op = 'replacePicture'; id = [string]$pic.id; file = $pngB.Replace('\', '/') })
} 'xlsx-emf-replace' | Out-Null
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-emf-inspect-replace'
$picR = @($treeR.sheets[0].pictures) | Select-Object -First 1
$hasCt = $picR.PSObject.Properties.Name -contains 'contentType'
$zipR = [IO.Compression.ZipFile]::OpenRead($replaced)
try {
    $emfLeft = @($zipR.Entries | Where-Object { $_.FullName -like '*.emf' }).Count -gt 0
} finally { $zipR.Dispose() }
$wbR = Open-ExcelBook $excel $replaced
$capR = [string]$wbR.Windows.Item(1).Caption
Add-Check 'xlsx-Xem4' (
    (-not $hasCt) -and (-not $emfLeft) -and ($capR -notmatch 'Repair')
) "hasCt=$hasCt emfLeft=$emfLeft caption=$capR"
Close-AllWorkbooks $excel

$hfOut = Get-OutPath 'xlsx-emf-hf.xlsx'
if (Test-Path $hfOut) { Remove-Item $hfOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $hfOut.Replace('\', '/')
    ops = @(@{
        op = 'insertPicture'
        id = 'sheet:Sheet1/hf:odd:header'
        file = $tiffFile
    })
} 'xlsx-emf-hf-insert' | Out-Null
$treeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ headerFooter = $true; pictures = $true }
} 'xlsx-emf-inspect-hf'
$hPic = @($treeH.sheets[0].headerFooters[0].pictures) | Select-Object -First 1
$wbH = Open-ExcelBook $excel $hfOut
$capH = [string]$wbH.Windows.Item(1).Caption
Add-Check 'xlsx-Xem5' (
    ($hPic.contentType -eq 'image/tiff') -and ($capH -notmatch 'Repair')
) "ct=$($hPic.contentType) caption=$capH"
Close-AllWorkbooks $excel

$junk = Get-OutPath 'xlsx-emf-not-image.bin'
[IO.File]::WriteAllBytes($junk, [byte[]](1, 2, 3, 4))
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = (Get-OutPath 'xlsx-emf-should-not.xlsx').Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = 'sheet:Sheet1'; file = $junk.Replace('\', '/'); from = 'B2' })
} 'xlsx-emf-reject'
$failCode = $null
try { $failCode = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'xlsx-Xem6' (($fail.ExitCode -ne 0) -and ($failCode -eq 'InvalidField')) "code=$failCode exit=$($fail.ExitCode)"
