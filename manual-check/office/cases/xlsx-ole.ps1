# xlsx OLE：docs/checklist-xlsx.md 的 Xol1–Xol5。
# 用 create 产物再 apply。不激活 OLE 服务器。

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

function Get-SheetOleCount($sheet) {
    $n = 0
    $shapes = $null
    try { $shapes = $sheet.Shapes } catch { }
    if ($null -eq $shapes) { return 0 }
    try {
        $c = 0
        try { $c = [int]$shapes.Count } catch { }
        for ($i = 1; $i -le $c; $i++) {
            $sh = $null
            try { $sh = $shapes.Item($i) } catch { }
            if ($null -eq $sh) { continue }
            try {
                if ([int]$sh.Type -eq 7) { $n++ }
            } finally { Release-Com $sh }
        }
    } finally { Release-Com $shapes }
    return $n
}

function Get-ZipEntryBytes([string]$xlsx, [string]$pattern) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $z = [IO.Compression.ZipFile]::OpenRead($xlsx)
    try {
        $entry = $z.Entries | Where-Object { ($_.FullName -replace '\\', '/') -like $pattern } | Select-Object -First 1
        if (-not $entry) { return $null }
        $ms = New-Object IO.MemoryStream
        $s = $entry.Open()
        try { $s.CopyTo($ms) } finally { $s.Dispose() }
        return $ms.ToArray()
    } finally { $z.Dispose() }
}

$hostPath = Get-OutPath 'xlsx-ole-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
        })
    }
} 'xlsx-xol-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xol-inspect-bare'
$sheet0 = $tree0.sheets[0]
$noList = -not ($sheet0.PSObject.Properties.Name -contains 'oleObjects')
$treeList = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ oleObjects = $true }
} 'xlsx-xol-inspect-list'
$listN = 0
if ($treeList.sheets[0].oleObjects) { $listN = @($treeList.sheets[0].oleObjects).Count }
$xol1ok = ([int]$tree0.oleObjectCount -eq 0) `
    -and ([int]$sheet0.oleObjectCount -eq 0) `
    -and $noList `
    -and ($listN -eq 0)
Add-Check 'xlsx-Xol1' $xol1ok "wb=$($tree0.oleObjectCount) sheet=$($sheet0.oleObjectCount) noList=$noList listed=$listN"

$embedPath = Get-OutPath 'xlsx-ole-embed.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $embedPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 42 })
        })
    }
} 'xlsx-xol-embed' | Out-Null

$insOut = Get-OutPath 'xlsx-ole-insert.xlsx'
if (Test-Path $insOut) { Remove-Item -Force $insOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{
        op = 'insertOle'
        id = 'sheet:Sheet1'
        file = $embedPath.Replace('\', '/')
        from = 'B2'
    })
} 'xlsx-xol-insert' | Out-Null

$excel = Ensure-MsExcel $hostPath
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $insOut
$cap = [string]$wb.Windows.Item(1).Caption
$s = $wb.Worksheets.Item(1)
$comN = Get-SheetOleCount $s
Close-AllWorkbooks $excel
$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ oleObjects = $true; shapes = $true }
} 'xlsx-xol-inspect-inserted'
$item = $null
if ($treeI.sheets[0].oleObjects) { $item = @($treeI.sheets[0].oleObjects) | Select-Object -First 1 }
$shapeN = 0
if ($treeI.sheets[0].shapes) { $shapeN = @($treeI.sheets[0].shapes).Count }
$xol2ok = ($cap -notmatch 'Repair') `
    -and ($comN -eq 1) `
    -and ($null -ne $item) `
    -and ([string]$item.id -eq 'sheet:Sheet1/ole:1025') `
    -and ([string]$item.progId -eq 'Worksheet') `
    -and ($shapeN -eq 0)
Add-Check 'xlsx-Xol2' $xol2ok "caption=$cap com=$comN id=$($item.id) progId=$($item.progId) shapes=$shapeN"

$oleId = if ($item) { [string]$item.id } else { '' }
$exportOut = Get-OutPath 'xlsx-ole-export.xlsx'
if (Test-Path $exportOut) { Remove-Item -Force $exportOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $insOut.Replace('\', '/')
    id = $oleId
    output = $exportOut.Replace('\', '/')
} 'xlsx-xol-export' | Out-Null
$embed = Get-ZipEntryBytes $insOut 'xl/embeddings/*'
$exported = if (Test-Path $exportOut) { [IO.File]::ReadAllBytes($exportOut) } else { $null }
$same = $false
if ($embed -and $exported -and ($embed.Length -eq $exported.Length)) {
    $same = $true
    for ($i = 0; $i -lt $embed.Length; $i++) {
        if ($embed[$i] -ne $exported[$i]) { $same = $false; break }
    }
}
Add-Check 'xlsx-Xol3' $same "embed=$($embed.Length) export=$($exported.Length)"

$delOut = Get-OutPath 'xlsx-ole-delete.xlsx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteOle'; id = $oleId })
} 'xlsx-xol-delete' | Out-Null
Close-AllWorkbooks $excel
$wbD = Open-ExcelBook $excel $delOut
$capD = [string]$wbD.Windows.Item(1).Caption
$sD = $wbD.Worksheets.Item(1)
$comD = Get-SheetOleCount $sD
Close-AllWorkbooks $excel
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ oleObjects = $true }
} 'xlsx-xol-inspect-deleted'
$xol4ok = ($capD -notmatch 'Repair') `
    -and ([int]$treeD.oleObjectCount -eq 0) `
    -and ($comD -eq 0)
Add-Check 'xlsx-Xol4' $xol4ok "caption=$capD count=$($treeD.oleObjectCount) com=$comD"

$failOut = Get-OutPath 'xlsx-ole-bad.xlsx'
$failFrom = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'insertOle'
        id = 'sheet:Sheet1'
        file = $embedPath.Replace('\', '/')
    })
} 'xlsx-xol-missing-from'
$codeFrom = $null
try { $codeFrom = ($failFrom.Stderr | ConvertFrom-Json).error.code } catch {}
$failSlide = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'insertOle'
        id = 'slide:1'
        file = $embedPath.Replace('\', '/')
        from = 'B2'
    })
} 'xlsx-xol-slide-id'
$codeSlide = $null
try { $codeSlide = ($failSlide.Stderr | ConvertFrom-Json).error.code } catch {}
$failShape = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'insertOle'
        id = 'sheet:Sheet1/shape:2'
        file = $embedPath.Replace('\', '/')
        from = 'B2'
    })
} 'xlsx-xol-shape-id'
$codeShape = $null
try { $codeShape = ($failShape.Stderr | ConvertFrom-Json).error.code } catch {}
$xol5ok = ($failFrom.ExitCode -ne 0) -and ($codeFrom -eq 'InvalidField') `
    -and ($failSlide.ExitCode -ne 0) -and ($codeSlide -eq 'InvalidId') `
    -and ($failShape.ExitCode -ne 0) -and ($codeShape -eq 'InvalidId') `
    -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xol5' $xol5ok "from=$codeFrom slide=$codeSlide shape=$codeShape wrote=$(Test-Path $failOut)"
