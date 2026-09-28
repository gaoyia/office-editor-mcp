# xlsx 保护：docs/checklist-xlsx.md 的 Xpr1–Xpr5。
# 用 create 产物再 apply。带哈希样例在 ZIP 里写 sheetProtection 属性，不靠 Excel 设密码。

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

function Rewrite-ZipPart([string]$zipPath, [string]$part, [scriptblock]$edit) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::Open($zipPath, 'Update')
    try {
        $entry = $zip.Entries | Where-Object { ($_.FullName -replace '\\', '/') -eq $part } | Select-Object -First 1
        if (-not $entry) { throw "zip 里没有 $part" }
        $ms = New-Object IO.MemoryStream
        $es = $entry.Open()
        $es.CopyTo($ms)
        $es.Dispose()
        $text = [Text.Encoding]::UTF8.GetString($ms.ToArray())
        $text2 = & $edit $text
        $name = $entry.FullName
        $entry.Delete()
        $newEntry = $zip.CreateEntry($name)
        $bytes = [Text.Encoding]::UTF8.GetBytes($text2)
        $ws = $newEntry.Open()
        $ws.Write($bytes, 0, $bytes.Length)
        $ws.Dispose()
    }
    finally { $zip.Dispose() }
}

$hostPath = Get-OutPath 'xlsx-protect-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
        })
    }
} 'xlsx-xpr-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xpr-inspect'
$wbProt = $tree0.workbookProtection
$sp = $tree0.sheets[0].sheetProtection
$xpr1ok = ($null -ne $wbProt) -and ($null -ne $sp) `
    -and ([bool]$wbProt.lockStructure -eq $false) `
    -and ([bool]$sp.enforced -eq $false) `
    -and ([bool]$sp.hasPassword -eq $false) `
    -and ([bool]$wbProt.hasPassword -eq $false)
Add-Check 'xlsx-Xpr1' $xpr1ok "lock=$($wbProt.lockStructure) enforced=$($sp.enforced) hash=$($sp.hasPassword)"

$protOut = Get-OutPath 'xlsx-protect-sheet.xlsx'
if (Test-Path $protOut) { Remove-Item -Force $protOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $protOut.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'sheet:Sheet1'; enforced = $true })
} 'xlsx-xpr-protect' | Out-Null

$excel = Ensure-MsExcel $protOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $protOut
$cap = [string]$wb.Windows.Item(1).Caption
$s = $wb.Worksheets.Item(1)
$contents = [bool]$s.ProtectContents
$xpr2ok = ($cap -notmatch 'Repair') -and $contents
Add-Check 'xlsx-Xpr2' $xpr2ok "caption=$cap ProtectContents=$contents"
Close-AllWorkbooks $excel

$openOut = Get-OutPath 'xlsx-protect-unprotect.xlsx'
if (Test-Path $openOut) { Remove-Item -Force $openOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $protOut.Replace('\', '/')
    output = $openOut.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'sheet:Sheet1'; enforced = $false })
} 'xlsx-xpr-unprotect' | Out-Null

Close-AllWorkbooks $excel
$wbU = Open-ExcelBook $excel $openOut
$capU = [string]$wbU.Windows.Item(1).Caption
$sU = $wbU.Worksheets.Item(1)
$contentsU = [bool]$sU.ProtectContents
$wrote = $false
try {
    $sU.Range('A1').Value2 = 99
    $wrote = ([string]$sU.Range('A1').Value2 -eq '99')
} catch {
    $wrote = $false
}
$xpr3ok = ($capU -notmatch 'Repair') -and (-not $contentsU) -and $wrote
Add-Check 'xlsx-Xpr3' $xpr3ok "caption=$capU ProtectContents=$contentsU wrote=$wrote"
Close-AllWorkbooks $excel

$wbOut = Get-OutPath 'xlsx-protect-workbook.xlsx'
if (Test-Path $wbOut) { Remove-Item -Force $wbOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $wbOut.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'xlsx:workbook'; lockStructure = $true })
} 'xlsx-xpr-workbook' | Out-Null

Close-AllWorkbooks $excel
$wbS = Open-ExcelBook $excel $wbOut
$capS = [string]$wbS.Windows.Item(1).Caption
$struct = [bool]$wbS.ProtectStructure
$xpr4ok = ($capS -notmatch 'Repair') -and $struct
Add-Check 'xlsx-Xpr4' $xpr4ok "caption=$capS ProtectStructure=$struct"
Close-AllWorkbooks $excel

$failOut = Get-OutPath 'xlsx-protect-bad.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'sheet:Sheet1' })
} 'xlsx-xpr-missing'
$codeEmpty = $null
try { $codeEmpty = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}

$hashPath = Get-OutPath 'xlsx-protect-hash.xlsx'
Copy-Item -Force $protOut $hashPath
Rewrite-ZipPart $hashPath 'xl/worksheets/sheet1.xml' {
    param($xml)
    return $xml.Replace('<sheetProtection ', '<sheetProtection hashValue="YWJj" ')
}
$treeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hashPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xpr-hash-inspect'
$hashFlag = [bool]$treeH.sheets[0].sheetProtection.hasPassword
$hashOut = Get-OutPath 'xlsx-protect-hash-out.xlsx'
$hashFail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hashPath.Replace('\', '/')
    output = $hashOut.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'sheet:Sheet1'; enforced = $false })
} 'xlsx-xpr-hash-apply'
$codeHash = $null
try { $codeHash = ($hashFail.Stderr | ConvertFrom-Json).error.code } catch {}
$xpr5ok = ($fail.ExitCode -ne 0) -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failOut)) `
    -and $hashFlag -and ($hashFail.ExitCode -ne 0) -and ($codeHash -eq 'NotImplemented') -and (-not (Test-Path $hashOut))
Add-Check 'xlsx-Xpr5' $xpr5ok "empty=$codeEmpty wroteEmpty=$(Test-Path $failOut) hash=$hashFlag hashCode=$codeHash"
