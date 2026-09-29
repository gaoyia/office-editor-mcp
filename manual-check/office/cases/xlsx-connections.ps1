# xlsx 外部数据连接：docs/checklist-xlsx.md 的 Xcn1–Xcn5。
# 用 create 产物再 apply。不刷新连接。

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

function Get-WorkbookConnectionCount($wb) {
    $n = 0
    $conns = $null
    try { $conns = $wb.Connections } catch { }
    if ($null -eq $conns) { return 0 }
    try {
        try { $n = [int]$conns.Count } catch { $n = 0 }
    } finally { Release-Com $conns }
    return $n
}

function Get-WorkbookConnectionName($wb, [int]$index) {
    $conns = $null
    $item = $null
    try {
        $conns = $wb.Connections
        if ($null -eq $conns) { return '' }
        $item = $conns.Item($index)
        return [string]$item.Name
    } catch {
        return ''
    } finally {
        if ($item) { Release-Com $item }
        if ($conns) { Release-Com $conns }
    }
}

$hostPath = Get-OutPath 'xlsx-connections-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
        })
    }
} 'xlsx-xcn-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xcn-inspect-bare'
$noList = -not ($tree0.PSObject.Properties.Name -contains 'connections')
$treeList = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ connections = $true }
} 'xlsx-xcn-inspect-list'
$listN = 0
if ($treeList.connections) { $listN = @($treeList.connections).Count }
$xcn1ok = ([int]$tree0.connectionCount -eq 0) `
    -and $noList `
    -and ($listN -eq 0)
Add-Check 'xlsx-Xcn1' $xcn1ok "wb=$($tree0.connectionCount) noList=$noList listed=$listN"

$webOut = Get-OutPath 'xlsx-connections-web.xlsx'
if (Test-Path $webOut) { Remove-Item -Force $webOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $webOut.Replace('\', '/')
    ops = @(@{
        op = 'insertConnection'
        id = 'xlsx:workbook'
        name = 'WebDemo'
        type = 'web'
        url = 'http://example.com'
        description = 'demo'
    })
} 'xlsx-xcn-insert-web' | Out-Null

$excel = Ensure-MsExcel $hostPath
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $webOut
$cap = [string]$wb.Windows.Item(1).Caption
$comN = Get-WorkbookConnectionCount $wb
$comName = Get-WorkbookConnectionName $wb 1
Close-AllWorkbooks $excel
$treeW = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $webOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ connections = $true }
} 'xlsx-xcn-inspect-web'
$item = $null
if ($treeW.connections) { $item = @($treeW.connections) | Select-Object -First 1 }
$xcn2ok = ($cap -notmatch 'Repair') `
    -and ($comN -ge 1) `
    -and ($comName -eq 'WebDemo') `
    -and ($null -ne $item) `
    -and ([string]$item.id -eq 'connection:1') `
    -and ([string]$item.kind -eq 'web') `
    -and ([string]$item.url -eq 'http://example.com')
Add-Check 'xlsx-Xcn2' $xcn2ok "caption=$cap com=$comN name=$comName id=$($item.id) kind=$($item.kind)"

$csvPath = Get-OutPath '_xlsx-connections-src.csv'
[IO.File]::WriteAllText($csvPath, "a,b`r`n1,2`r`n", [Text.UTF8Encoding]::new($false))
$textOut = Get-OutPath 'xlsx-connections-text.xlsx'
if (Test-Path $textOut) { Remove-Item -Force $textOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $textOut.Replace('\', '/')
    ops = @(@{
        op = 'insertConnection'
        id = 'xlsx:workbook'
        name = 'CsvSrc'
        type = 'text'
        sourceFile = $csvPath
    })
} 'xlsx-xcn-insert-text' | Out-Null

Close-AllWorkbooks $excel
$wbT = Open-ExcelBook $excel $textOut
$capT = [string]$wbT.Windows.Item(1).Caption
$comT = Get-WorkbookConnectionCount $wbT
Close-AllWorkbooks $excel
$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $textOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ connections = $true }
} 'xlsx-xcn-inspect-text'
$itemT = $null
if ($treeT.connections) { $itemT = @($treeT.connections) | Select-Object -First 1 }
$srcOk = $false
if ($itemT) { $srcOk = ([string]$itemT.sourceFile) -eq $csvPath }
$xcn3ok = ($capT -notmatch 'Repair') `
    -and ($comT -ge 1) `
    -and ($null -ne $itemT) `
    -and ([string]$itemT.kind -eq 'text') `
    -and $srcOk
Add-Check 'xlsx-Xcn3' $xcn3ok "caption=$capT com=$comT kind=$($itemT.kind) src=$($itemT.sourceFile)"

$connId = if ($item) { [string]$item.id } else { '' }
$delOut = Get-OutPath 'xlsx-connections-delete.xlsx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $webOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteConnection'; id = $connId })
} 'xlsx-xcn-delete' | Out-Null
Close-AllWorkbooks $excel
$wbD = Open-ExcelBook $excel $delOut
$capD = [string]$wbD.Windows.Item(1).Caption
$comD = Get-WorkbookConnectionCount $wbD
Close-AllWorkbooks $excel
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ connections = $true }
} 'xlsx-xcn-inspect-deleted'
$xcn4ok = ($capD -notmatch 'Repair') `
    -and ([int]$treeD.connectionCount -eq 0) `
    -and ($comD -eq 0)
Add-Check 'xlsx-Xcn4' $xcn4ok "caption=$capD count=$($treeD.connectionCount) com=$comD"

$failOut = Get-OutPath 'xlsx-connections-bad.xlsx'
$failType = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'insertConnection'
        id = 'xlsx:workbook'
        name = 'WebDemo'
    })
} 'xlsx-xcn-missing-type'
$codeType = $null
try { $codeType = ($failType.Stderr | ConvertFrom-Json).error.code } catch {}
$failSlide = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'insertConnection'
        id = 'slide:1'
        name = 'WebDemo'
        type = 'web'
        url = 'http://example.com'
    })
} 'xlsx-xcn-slide-id'
$codeSlide = $null
try { $codeSlide = ($failSlide.Stderr | ConvertFrom-Json).error.code } catch {}
$failKind = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'insertConnection'
        id = 'xlsx:workbook'
        name = 'WebDemo'
        type = 'odbc'
        url = 'http://example.com'
    })
} 'xlsx-xcn-bad-type'
$codeKind = $null
try { $codeKind = ($failKind.Stderr | ConvertFrom-Json).error.code } catch {}
$xcn5ok = ($failType.ExitCode -ne 0) -and ($codeType -eq 'InvalidField') `
    -and ($failSlide.ExitCode -ne 0) -and ($codeSlide -eq 'InvalidId') `
    -and ($failKind.ExitCode -ne 0) -and ($codeKind -eq 'InvalidField') `
    -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xcn5' $xcn5ok "type=$codeType slide=$codeSlide kind=$codeKind wrote=$(Test-Path $failOut)"
