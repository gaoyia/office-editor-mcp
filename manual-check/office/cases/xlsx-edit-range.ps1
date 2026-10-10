# xlsx 允许编辑区域与 xlsm：docs/checklist-xlsx.md 的 Xer1–Xer5。
# 用 create 产物再 apply。编号 Xer，不要和 Xpr、Xph 撞。不执行宏，不破解密码。

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

function Get-FailCode($fail) {
    try { return ($fail.Stderr | ConvertFrom-Json).error.code } catch { return $null }
}

function Get-ZipEntryText([string]$zipPath, [string]$name) {
    $z = [IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        $e = $z.GetEntry($name)
        if (-not $e) { return $null }
        $sr = New-Object IO.StreamReader($e.Open())
        try { return $sr.ReadToEnd() } finally { $sr.Dispose() }
    }
    finally { $z.Dispose() }
}

function Get-ZipEntryBytes([string]$zipPath, [string]$name) {
    $z = [IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        $e = $z.GetEntry($name)
        if (-not $e) { return $null }
        $ms = New-Object IO.MemoryStream
        $es = $e.Open()
        $es.CopyTo($ms)
        $es.Dispose()
        return $ms.ToArray()
    }
    finally { $z.Dispose() }
}

function Add-ZipFile([string]$src, [string]$dst, [string]$name, [byte[]]$payload) {
    if (Test-Path $dst) { Remove-Item -Force $dst }
    Copy-Item -LiteralPath $src -Destination $dst
    $z = [IO.Compression.ZipFile]::Open($dst, 'Update')
    try {
        $e = $z.GetEntry($name)
        if ($e) { $e.Delete() }
        $ne = $z.CreateEntry($name)
        $ws = $ne.Open()
        try { $ws.Write($payload, 0, $payload.Length) } finally { $ws.Dispose() }
    }
    finally { $z.Dispose() }
}

$hostPath = Get-OutPath 'xlsx-edit-range-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
        })
    }
} 'xlsx-xer-create' | Out-Null

$plainOut = Get-OutPath 'xlsx-edit-range-plain.xlsx'
if (Test-Path $plainOut) { Remove-Item -Force $plainOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $plainOut.Replace('\', '/')
    ops = @(@{
        op = 'insertProtectedRange'
        id = 'sheet:Sheet1'
        name = 'Range1'
        sqref = 'A1:C5'
    })
} 'xlsx-xer-insert' | Out-Null

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $plainOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ protectedRanges = $true }
} 'xlsx-xer-inspect-plain'
$pr1 = $tree1.sheets[0].protectedRanges[0]
$excel = Ensure-MsExcel $hostPath
Close-AllWorkbooks $excel
$wb1 = Open-ExcelBook $excel $plainOut
$cap1 = [string]$wb1.Windows.Item(1).Caption
$s1 = $wb1.Worksheets.Item(1)
$aerCount = 0
$aerTitle = ''
$aerAddr = ''
try {
    $aer = $s1.Protection.AllowEditRanges
    $aerCount = [int]$aer.Count
    if ($aerCount -lt 1) {
        $s1.Protect()
        $aer = $s1.Protection.AllowEditRanges
        $aerCount = [int]$aer.Count
    }
    if ($aerCount -ge 1) {
        $item = $aer.Item(1)
        $aerTitle = [string]$item.Title
        $aerAddr = ([string]$item.Range.Address($true, $true)).Replace('$', '')
    }
}
catch {
    Write-Log "AllowEditRanges 失败: $($_.Exception.Message)"
}
$xer1ok = ([int]$tree1.sheets[0].protectedRangeCount -eq 1) `
    -and ([string]$pr1.name -eq 'Range1') `
    -and ([string]$pr1.sqref -eq 'A1:C5') `
    -and (-not [bool]$pr1.hasPassword) `
    -and ($cap1 -notmatch 'Repair') `
    -and ($aerCount -ge 1) `
    -and ($aerTitle -eq 'Range1') `
    -and ($aerAddr -eq 'A1:C5')
Add-Check 'xlsx-Xer1' $xer1ok "count=$($tree1.sheets[0].protectedRangeCount) name=$($pr1.name) sqref=$($pr1.sqref) caption=$cap1 aer=$aerCount/$aerTitle/$aerAddr"
Close-AllWorkbooks $excel

$pwOut = Get-OutPath 'xlsx-edit-range-password.xlsx'
if (Test-Path $pwOut) { Remove-Item -Force $pwOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $pwOut.Replace('\', '/')
    ops = @(@{
        op = 'insertProtectedRange'
        id = 'sheet:Sheet1'
        name = 'Locked'
        sqref = 'A1'
        password = 'secret'
    })
} 'xlsx-xer-password' | Out-Null

$tree2 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $pwOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ protectedRanges = $true }
} 'xlsx-xer-inspect-password'
$pr2 = $tree2.sheets[0].protectedRanges[0]
$dump2 = ($tree2 | ConvertTo-Json -Depth 12 -Compress)
$xml2 = Get-ZipEntryText $pwOut 'xl/worksheets/sheet1.xml'
Close-AllWorkbooks $excel
$wb2 = Open-ExcelBook $excel $pwOut
$cap2 = [string]$wb2.Windows.Item(1).Caption
$xer2ok = ([bool]$pr2.hasPassword) `
    -and ($dump2 -notmatch 'secret') `
    -and ($dump2 -notmatch 'hashValue') `
    -and ($cap2 -notmatch 'Repair') `
    -and ($xml2 -match 'algorithmName="SHA-512"') `
    -and ($xml2 -match 'spinCount="100000"')
Add-Check 'xlsx-Xer2' $xer2ok "hasPassword=$($pr2.hasPassword) caption=$cap2 sha=$($xml2 -match 'SHA-512')"
Close-AllWorkbooks $excel

$renamedOut = Get-OutPath 'xlsx-edit-range-renamed.xlsx'
if (Test-Path $renamedOut) { Remove-Item -Force $renamedOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $plainOut.Replace('\', '/')
    output = $renamedOut.Replace('\', '/')
    ops = @(@{
        op = 'setProtectedRange'
        id = 'sheet:Sheet1/editRange:1'
        name = 'Range2'
        sqref = @('B2:D6', 'E10')
    })
} 'xlsx-xer-set' | Out-Null

$tree3 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $renamedOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ protectedRanges = $true }
} 'xlsx-xer-inspect-set'
$pr3 = $tree3.sheets[0].protectedRanges[0]

$deletedOut = Get-OutPath 'xlsx-edit-range-deleted.xlsx'
if (Test-Path $deletedOut) { Remove-Item -Force $deletedOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $renamedOut.Replace('\', '/')
    output = $deletedOut.Replace('\', '/')
    ops = @(@{
        op = 'deleteProtectedRange'
        id = 'sheet:Sheet1/editRange:1'
    })
} 'xlsx-xer-delete' | Out-Null

$treeDel = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $deletedOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ protectedRanges = $true }
} 'xlsx-xer-inspect-delete'
$xmlDel = Get-ZipEntryText $deletedOut 'xl/worksheets/sheet1.xml'
Close-AllWorkbooks $excel
$wb3 = Open-ExcelBook $excel $renamedOut
$cap3 = [string]$wb3.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$wbDel = Open-ExcelBook $excel $deletedOut
$capDel = [string]$wbDel.Windows.Item(1).Caption
$xer3ok = ([string]$pr3.name -eq 'Range2') `
    -and ([string]$pr3.sqref -eq 'B2:D6 E10') `
    -and ([int]$treeDel.sheets[0].protectedRangeCount -eq 0) `
    -and ($xmlDel -notmatch 'protectedRanges') `
    -and ($cap3 -notmatch 'Repair') `
    -and ($capDel -notmatch 'Repair')
Add-Check 'xlsx-Xer3' $xer3ok "name=$($pr3.name) sqref=$($pr3.sqref) count=$($treeDel.sheets[0].protectedRangeCount) capSet=$cap3 capDel=$capDel"
Close-AllWorkbooks $excel

$xlsmPath = Get-OutPath 'xlsx-edit-range.xlsm'
if (Test-Path $xlsmPath) { Remove-Item -Force $xlsmPath }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsmPath.Replace('\', '/'); format = 'xlsm'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
        })
    }
} 'xlsx-xer-xlsm-create' | Out-Null

$treeXlsm = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $xlsmPath.Replace('\', '/'); format = 'xlsm'
} 'xlsx-xer-inspect-xlsm'
$ctXlsm = Get-ZipEntryText $xlsmPath '[Content_Types].xml'
$marker = [Text.Encoding]::UTF8.GetBytes('vba-sentinel-bytes')
$injected = Get-OutPath 'xlsx-edit-range-vba.xlsm'
Add-ZipFile $xlsmPath $injected 'xl/vbaProject.bin' $marker
$appliedXlsm = Get-OutPath 'xlsx-edit-range-vba-applied.xlsm'
if (Test-Path $appliedXlsm) { Remove-Item -Force $appliedXlsm }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $injected.Replace('\', '/')
    output = $appliedXlsm.Replace('\', '/')
    ops = @(@{
        op = 'insertProtectedRange'
        id = 'sheet:Sheet1'
        name = 'Range1'
        sqref = 'A1:C5'
    })
} 'xlsx-xer-xlsm-apply' | Out-Null
$vbaBytes = Get-ZipEntryBytes $appliedXlsm 'xl/vbaProject.bin'
$ctApplied = Get-ZipEntryText $appliedXlsm '[Content_Types].xml'
$vbaSame = $false
if ($null -ne $vbaBytes -and $vbaBytes.Length -eq $marker.Length) {
    $vbaSame = $true
    for ($bi = 0; $bi -lt $marker.Length; $bi++) {
        if ($vbaBytes[$bi] -ne $marker[$bi]) { $vbaSame = $false; break }
    }
}
Close-AllWorkbooks $excel
$wbXlsm = Open-ExcelBook $excel $xlsmPath
$capXlsm = [string]$wbXlsm.Windows.Item(1).Caption
$xer4ok = ([string]$treeXlsm.format -eq 'xlsx') `
    -and ($ctXlsm -match 'application/vnd.ms-excel.sheet.macroEnabled.main\+xml') `
    -and ($ctXlsm -notmatch 'spreadsheetml.sheet.main\+xml') `
    -and $vbaSame `
    -and ($ctApplied -match 'application/vnd.ms-excel.sheet.macroEnabled.main\+xml') `
    -and ($capXlsm -notmatch 'Repair')
Add-Check 'xlsx-Xer4' $xer4ok "format=$($treeXlsm.format) caption=$capXlsm vba=$vbaSame"
Close-AllWorkbooks $excel

$csHost = Get-OutPath 'xlsx-edit-range-chart.xlsx'
if (Test-Path $csHost) { Remove-Item -Force $csHost }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $csHost.Replace('\', '/')
    ops = @(@{
        op = 'insertChartSheet'
        id = 'xlsx:workbook'
        name = 'Chart1'
        dataFrom = 'A10'
        type = 'col'
        title = '销量'
        series = @(@{
            name = 'Q1'
            categories = @('甲', '乙', '丙')
            values = @(1, 2, 3)
        })
    })
} 'xlsx-xer-chartsheet' | Out-Null

$failCs = Get-OutPath 'xlsx-edit-range-bad-chart.xlsx'
$failCsR = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $csHost.Replace('\', '/')
    output = $failCs.Replace('\', '/')
    ops = @(@{
        op = 'insertProtectedRange'
        id = 'sheet:Chart1'
        name = 'Range1'
        sqref = 'A1'
    })
} 'xlsx-xer-bad-chart'
$codeCs = Get-FailCode $failCsR

$failName = Get-OutPath 'xlsx-edit-range-bad-name.xlsx'
$failNameR = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failName.Replace('\', '/')
    ops = @(@{
        op = 'insertProtectedRange'
        id = 'sheet:Sheet1'
        name = '   '
        sqref = 'A1'
    })
} 'xlsx-xer-bad-name'
$codeName = Get-FailCode $failNameR

$failDup = Get-OutPath 'xlsx-edit-range-dup.xlsx'
$failDupR = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $plainOut.Replace('\', '/')
    output = $failDup.Replace('\', '/')
    ops = @(@{
        op = 'insertProtectedRange'
        id = 'sheet:Sheet1'
        name = 'range1'
        sqref = 'B1'
    })
} 'xlsx-xer-dup'
$codeDup = Get-FailCode $failDupR

$failEmpty = Get-OutPath 'xlsx-edit-range-empty-set.xlsx'
$failEmptyR = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $plainOut.Replace('\', '/')
    output = $failEmpty.Replace('\', '/')
    ops = @(@{
        op = 'setProtectedRange'
        id = 'sheet:Sheet1/editRange:1'
    })
} 'xlsx-xer-empty-set'
$codeEmpty = Get-FailCode $failEmptyR

$failNopw = Get-OutPath 'xlsx-edit-range-nopw.xlsx'
$failNopwR = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $pwOut.Replace('\', '/')
    output = $failNopw.Replace('\', '/')
    ops = @(@{
        op = 'setProtectedRange'
        id = 'sheet:Sheet1/editRange:1'
        name = 'Other'
    })
} 'xlsx-xer-nopw'
$codeNopw = Get-FailCode $failNopwR

$xer5ok = ($failCsR.ExitCode -ne 0) -and ($codeCs -eq 'InvalidField') -and (-not (Test-Path $failCs)) `
    -and ($failNameR.ExitCode -ne 0) -and ($codeName -eq 'InvalidField') -and (-not (Test-Path $failName)) `
    -and ($failDupR.ExitCode -ne 0) -and ($codeDup -eq 'InvalidField') -and (-not (Test-Path $failDup)) `
    -and ($failEmptyR.ExitCode -ne 0) -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failEmpty)) `
    -and ($failNopwR.ExitCode -ne 0) -and ($codeNopw -eq 'NotImplemented') -and (-not (Test-Path $failNopw))
Add-Check 'xlsx-Xer5' $xer5ok "chart=$codeCs name=$codeName dup=$codeDup empty=$codeEmpty nopw=$codeNopw"
