# xlsx 嵌入 SVG：docs/checklist-xlsx.md 的 Xvg1–Xvg6。
# COM 认 Type=28（msoGraphic）。探测脚本不进本文件。

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

function Get-ExcelGraphicCount($sheet) {
    $n = 0
    $shapes = $sheet.Shapes
    try {
        for ($i = 1; $i -le $shapes.Count; $i++) {
            $sh = $shapes.Item($i)
            try {
                if ([int]$sh.Type -eq 28) { $n++ }
            } finally { Release-Com $sh }
        }
    } finally { Release-Com $shapes }
    return $n
}

$svg = Get-OutPath 'xlsx-pic-probe.svg'
[IO.File]::WriteAllText($svg, @'
<svg xmlns="http://www.w3.org/2000/svg" width="96" height="48" viewBox="0 0 96 48">
  <rect width="96" height="48" fill="#3366cc"/>
  <circle cx="48" cy="24" r="16" fill="#ffcc00"/>
</svg>
'@, $global:OoxpUtf8)
$pngB = Ensure-OoxpPngB
$svgFile = $svg.Replace('\', '/')

$src = Get-OutPath 'xlsx-svg-src.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-svg-create-src' | Out-Null

$excel = Ensure-MsExcel $src
Close-AllWorkbooks $excel

$inserted = Get-OutPath 'xlsx-svg-inserted.xlsx'
Close-AllWorkbooks $excel
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{
        op = 'insertPicture'
        id = 'sheet:Sheet1'
        file = $svgFile
        from = 'B2'
    })
} 'xlsx-svg-insert' | Out-Null
$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-svg-inspect-insert'
$pic = @($tree1.sheets[0].pictures) | Select-Object -First 1
$wb1 = Open-ExcelBook $excel $inserted
$cap1 = [string]$wb1.Windows.Item(1).Caption
$s1 = $wb1.Worksheets.Item('Sheet1')
$n1 = Get-ExcelGraphicCount $s1
Release-Com $s1
Add-Check 'xlsx-Xvg1' (
    ($pic.contentType -eq 'image/svg+xml') -and ($cap1 -notmatch 'Repair') -and ($n1 -ge 1)
) "ct=$($pic.contentType) caption=$cap1 n=$n1"
Close-AllWorkbooks $excel

$exported = Get-OutPath 'xlsx-svg-export.svg'
$expJson = (Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $inserted.Replace('\', '/')
    id = [string]$pic.id
    output = $exported.Replace('\', '/')
} 'xlsx-svg-export') | ConvertFrom-Json
$expCt = [string]$expJson.contentType
$expText = [IO.File]::ReadAllText($exported)
$srcText = [IO.File]::ReadAllText($svg)
Add-Check 'xlsx-Xvg2' (($expCt -eq 'image/svg+xml') -and ($expText -eq $srcText)) "ct=$expCt len=$($expText.Length)"

$created = Get-OutPath 'xlsx-svg-created.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $created.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
            pictures = @(@{ file = $svgFile; from = 'B2' })
        })
    }
} 'xlsx-svg-create' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $created.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-svg-inspect-create'
$picC = @($treeC.sheets[0].pictures) | Select-Object -First 1
Close-AllWorkbooks $excel
$wbC = Open-ExcelBook $excel $created
$capC = [string]$wbC.Windows.Item(1).Caption
Add-Check 'xlsx-Xvg3' (
    ($picC.contentType -eq 'image/svg+xml') -and ($capC -notmatch 'Repair')
) "ct=$($picC.contentType) caption=$capC"
Close-AllWorkbooks $excel

$replaced = Get-OutPath 'xlsx-svg-replaced.xlsx'
if (Test-Path $replaced) { Remove-Item $replaced -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{ op = 'replacePicture'; id = [string]$pic.id; file = $pngB.Replace('\', '/') })
} 'xlsx-svg-replace' | Out-Null
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-svg-inspect-replace'
$picR = @($treeR.sheets[0].pictures) | Select-Object -First 1
$hasCt = $picR.PSObject.Properties.Name -contains 'contentType'
$zipR = [IO.Compression.ZipFile]::OpenRead($replaced)
try {
    $svgLeft = @($zipR.Entries | Where-Object { $_.FullName -like '*.svg' }).Count -gt 0
} finally { $zipR.Dispose() }
$wbR = Open-ExcelBook $excel $replaced
$capR = [string]$wbR.Windows.Item(1).Caption
Add-Check 'xlsx-Xvg4' (
    (-not $hasCt) -and (-not $svgLeft) -and ($capR -notmatch 'Repair')
) "hasCt=$hasCt svgLeft=$svgLeft caption=$capR"
Close-AllWorkbooks $excel

$hfOut = Get-OutPath 'xlsx-svg-hf.xlsx'
if (Test-Path $hfOut) { Remove-Item $hfOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $hfOut.Replace('\', '/')
    ops = @(@{
        op = 'insertPicture'
        id = 'sheet:Sheet1/hf:odd:header'
        file = $svgFile
    })
} 'xlsx-svg-hf-insert' | Out-Null
$treeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ headerFooter = $true; pictures = $true }
} 'xlsx-svg-inspect-hf'
$hPic = @($treeH.sheets[0].headerFooters[0].pictures) | Select-Object -First 1
$wbH = Open-ExcelBook $excel $hfOut
$capH = [string]$wbH.Windows.Item(1).Caption
Add-Check 'xlsx-Xvg5' (
    ($hPic.contentType -eq 'image/svg+xml') -and ($capH -notmatch 'Repair')
) "ct=$($hPic.contentType) caption=$capH"
Close-AllWorkbooks $excel

$junk = Get-OutPath 'xlsx-not-image.bin'
[IO.File]::WriteAllBytes($junk, [byte[]](1, 2, 3, 4))
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = (Get-OutPath 'xlsx-svg-should-not.xlsx').Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = 'sheet:Sheet1'; file = $junk.Replace('\', '/'); from = 'B2' })
} 'xlsx-svg-reject'
$failCode = $null
try { $failCode = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'xlsx-Xvg6' (($fail.ExitCode -ne 0) -and ($failCode -eq 'InvalidField')) "code=$failCode exit=$($fail.ExitCode)"
