# xlsx 工作表公式框：docs/checklist-xlsx.md 的 Xom1–Xom6。
# COM 多半是 Type=17（文本框）。探测脚本不进本文件。

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

$src = Get-OutPath 'xlsx-omml-src.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-omml-create-src' | Out-Null

$excel = Ensure-MsExcel $src
Close-AllWorkbooks $excel

$inserted = Get-OutPath 'xlsx-omml-inserted.xlsx'
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
} 'xlsx-omml-insert' | Out-Null
$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'xlsx'
    scope = @{ maths = $true; textBoxes = $true; shapes = $true }
} 'xlsx-omml-inspect-insert'
$item = @($tree1.sheets[0].maths) | Select-Object -First 1
$tbN = @($tree1.sheets[0].textBoxes).Count
if ($null -eq $tree1.sheets[0].textBoxes) { $tbN = 0 }
$xmlI = Get-ZipPartText $inserted 'xl/drawings/drawing1.xml'
$hasA14 = ($xmlI -like '*a14:m*') -and ($xmlI -like '*Requires="a14"*')
$wb1 = Open-ExcelBook $excel $inserted
$cap1 = [string]$wb1.Windows.Item(1).Caption
Add-Check 'xlsx-Xom1' (
    ([string]$item.text -eq 'E=mc^2') -and ([int]$tree1.textBoxCount -eq 0) -and ($tbN -eq 0) -and $hasA14 -and ($cap1 -notmatch 'Repair')
) "text=$($item.text) tb=$($tree1.textBoxCount) a14=$hasA14 caption=$cap1"
Close-AllWorkbooks $excel

$replaced = Get-OutPath 'xlsx-omml-replaced.xlsx'
if (Test-Path $replaced) { Remove-Item $replaced -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{ op = 'setMath'; id = [string]$item.id; text = 'a^2+b^2=c^2' })
} 'xlsx-omml-set' | Out-Null
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'xlsx'
    scope = @{ maths = $true }
} 'xlsx-omml-inspect-set'
$itemR = @($treeR.sheets[0].maths) | Select-Object -First 1
$wbR = Open-ExcelBook $excel $replaced
$capR = [string]$wbR.Windows.Item(1).Caption
Add-Check 'xlsx-Xom2' (
    ([string]$itemR.id -eq [string]$item.id) -and ([string]$itemR.text -eq 'a^2+b^2=c^2') -and ($capR -notmatch 'Repair')
) "id=$($itemR.id) text=$($itemR.text) caption=$capR"
Close-AllWorkbooks $excel

$created = Get-OutPath 'xlsx-omml-created.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $created.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
            maths = @(@{ text = 'E=mc^2'; from = 'B2' })
        })
    }
} 'xlsx-omml-create' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $created.Replace('\', '/'); format = 'xlsx'
    scope = @{ maths = $true; textBoxes = $true }
} 'xlsx-omml-inspect-create'
$itemC = @($treeC.sheets[0].maths) | Select-Object -First 1
Close-AllWorkbooks $excel
$wbC = Open-ExcelBook $excel $created
$capC = [string]$wbC.Windows.Item(1).Caption
Add-Check 'xlsx-Xom3' (
    ([string]$itemC.text -eq 'E=mc^2') -and ([int]$treeC.textBoxCount -eq 0) -and ($capC -notmatch 'Repair')
) "text=$($itemC.text) tb=$($treeC.textBoxCount) caption=$capC"
Close-AllWorkbooks $excel

$deleted = Get-OutPath 'xlsx-omml-deleted.xlsx'
if (Test-Path $deleted) { Remove-Item $deleted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $replaced.Replace('\', '/')
    output = $deleted.Replace('\', '/')
    ops = @(@{ op = 'deleteMath'; id = [string]$item.id })
} 'xlsx-omml-delete' | Out-Null
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $deleted.Replace('\', '/'); format = 'xlsx'
    scope = @{ maths = $true }
} 'xlsx-omml-inspect-delete'
$wbD = Open-ExcelBook $excel $deleted
$capD = [string]$wbD.Windows.Item(1).Caption
Add-Check 'xlsx-Xom4' (
    ([int]$treeD.mathCount -eq 0) -and ($capD -notmatch 'Repair')
) "count=$($treeD.mathCount) caption=$capD"
Close-AllWorkbooks $excel

Add-Check 'xlsx-Xom5' (
    ([int]$tree1.textBoxCount -eq 0) -and ([int]$tree1.sheets[0].mathCount -eq 1)
) "tb=$($tree1.textBoxCount) math=$($tree1.sheets[0].mathCount)"

$failOut = Get-OutPath 'xlsx-omml-should-not.xlsx'
if (Test-Path $failOut) { Remove-Item $failOut -Force }
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'insertMath'
        id = 'sheet:Sheet1'
        text = 'x'
        expr = @{ type = 'text'; text = 'x' }
    })
} 'xlsx-omml-reject'
$failCode = $null
try { $failCode = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$empty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'insertMath'; id = 'sheet:Sheet1'; text = '' })
} 'xlsx-omml-empty'
$emptyCode = $null
try { $emptyCode = ($empty.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'xlsx-Xom6' (
    ($fail.ExitCode -ne 0) -and ($failCode -eq 'InvalidField') -and (-not (Test-Path $failOut)) -and ($empty.ExitCode -ne 0) -and ($emptyCode -eq 'InvalidField')
) "expr=$failCode empty=$emptyCode exit=$($fail.ExitCode)/$($empty.ExitCode)"
