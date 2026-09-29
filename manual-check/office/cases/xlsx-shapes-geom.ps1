# xlsx 无 txBox 形状与连接线：docs/checklist-xlsx.md 的 Xsg1–Xsg5。
# 用 create 产物再 apply，不另做 Excel 另存样例。

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

function Get-ZipText([string]$zipPath, [string]$part) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        $entry = $zip.Entries | Where-Object { ($_.FullName -replace '\\', '/') -eq $part } | Select-Object -First 1
        if (-not $entry) { return $null }
        $ms = New-Object IO.MemoryStream
        $es = $entry.Open()
        $es.CopyTo($ms)
        $es.Dispose()
        return [Text.Encoding]::UTF8.GetString($ms.ToArray())
    }
    finally { $zip.Dispose() }
}

function Read-SheetGeom($sheet) {
    $shapeN = 0
    $cxnN = 0
    $shapeText = ''
    $shapes = $null
    try { $shapes = $sheet.Shapes } catch { }
    if ($null -eq $shapes) {
        return @{ ShapeCount = 0; CxnCount = 0; ShapeText = '' }
    }
    $n = 0
    try { $n = [int]$shapes.Count } catch { }
    for ($i = 1; $i -le $n; $i++) {
        $shp = $null
        try { $shp = $shapes.Item($i) } catch { }
        if ($null -eq $shp) { continue }
        try {
            $ty = 0
            try { $ty = [int]$shp.Type } catch { }
            if ($ty -eq 17 -or $ty -eq 24) { continue }
            $isCxn = $false
            try { $isCxn = [bool]$shp.Connector } catch { }
            if ($isCxn -or ($ty -eq 9)) {
                $cxnN++
                continue
            }
            $shapeN++
            $tf = $null
            try { $tf = $shp.TextFrame } catch { }
            if ($null -ne $tf) {
                $chars = $null
                try { $chars = $tf.Characters() } catch { }
                if ($null -ne $chars) {
                    try {
                        $t = [string]$chars.Text
                        if ([string]::IsNullOrWhiteSpace($shapeText)) { $shapeText = $t }
                    } finally { Release-Com $chars }
                }
                Release-Com $tf
            }
        } finally { Release-Com $shp }
    }
    Release-Com $shapes
    return @{ ShapeCount = $shapeN; CxnCount = $cxnN; ShapeText = $shapeText }
}

$hostPath = Get-OutPath 'xlsx-shapes-geom-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
        })
    }
} 'xlsx-xsg-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xsg-inspect'
$noShapeList = -not ($tree0.sheets[0].PSObject.Properties.Name -contains 'shapes')
$noCxnList = -not ($tree0.sheets[0].PSObject.Properties.Name -contains 'connectors')
$hasShape = ($tree0.PSObject.Properties.Name -contains 'shapeCount') -and ([int]$tree0.shapeCount -eq 0)
$hasCxn = ($tree0.PSObject.Properties.Name -contains 'connectorCount') -and ([int]$tree0.connectorCount -eq 0)
$sheetShape = ($tree0.sheets[0].PSObject.Properties.Name -contains 'shapeCount') -and ([int]$tree0.sheets[0].shapeCount -eq 0)
$sheetCxn = ($tree0.sheets[0].PSObject.Properties.Name -contains 'connectorCount') -and ([int]$tree0.sheets[0].connectorCount -eq 0)
$treeList = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ shapes = $true; connectors = $true }
} 'xlsx-xsg-inspect-list'
$hasShapeList = $treeList.sheets[0].PSObject.Properties.Name -contains 'shapes'
$hasCxnList = $treeList.sheets[0].PSObject.Properties.Name -contains 'connectors'
$shapes0 = @($treeList.sheets[0].shapes | Where-Object { $_ })
$cxns0 = @($treeList.sheets[0].connectors | Where-Object { $_ })
$listEmpty = $hasShapeList -and $hasCxnList -and ($shapes0.Count -eq 0) -and ($cxns0.Count -eq 0)
$xsg1ok = $hasShape -and $hasCxn -and $sheetShape -and $sheetCxn -and $noShapeList -and $noCxnList -and $listEmpty
Add-Check 'xlsx-Xsg1' $xsg1ok "shape=$($tree0.shapeCount) cxn=$($tree0.connectorCount) empty=$listEmpty"

$shapeOut = Get-OutPath 'xlsx-shapes-geom-rect.xlsx'
if (Test-Path $shapeOut) { Remove-Item -Force $shapeOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $shapeOut.Replace('\', '/')
    ops = @(@{
        op = 'insertShape'
        id = 'sheet:Sheet1'
        from = 'B2'
        text = 'Xsg框'
    })
} 'xlsx-xsg-insert-shape' | Out-Null

$excel = Ensure-MsExcel $shapeOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $shapeOut
$cap = [string]$wb.Windows.Item(1).Caption
$s = $wb.Worksheets.Item(1)
$draw = Read-SheetGeom $s
Close-AllWorkbooks $excel
$treeShape = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $shapeOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ shapes = $true }
} 'xlsx-xsg-inspect-shape'
$shapeNode = @($treeShape.sheets[0].shapes | Where-Object { $_ }) | Select-Object -First 1
$shapeId = [string]$shapeNode.id
$prst = [string]$shapeNode.prst
$drawXml = Get-ZipText $shapeOut 'xl/drawings/drawing1.xml'
$noTxBox = ($null -ne $drawXml) -and ($drawXml -notmatch 'txBox="1"') -and ($drawXml -match '<xdr:sp[\s>]')
$xsg2ok = ($cap -notmatch 'Repair') -and ($draw.ShapeCount -ge 1) -and ($prst -eq 'rect') -and ($shapeId -like 'sheet:*/shape:*') -and $noTxBox
Add-Check 'xlsx-Xsg2' $xsg2ok "caption=$cap n=$($draw.ShapeCount) prst=$prst id=$shapeId noTxBox=$noTxBox"

$cxnOut = Get-OutPath 'xlsx-shapes-geom-cxn.xlsx'
if (Test-Path $cxnOut) { Remove-Item -Force $cxnOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $cxnOut.Replace('\', '/')
    ops = @(@{
        op = 'insertConnector'
        id = 'sheet:Sheet1'
        from = 'D4'
    })
} 'xlsx-xsg-insert-cxn' | Out-Null

Close-AllWorkbooks $excel
$wbC = Open-ExcelBook $excel $cxnOut
$capC = [string]$wbC.Windows.Item(1).Caption
$sC = $wbC.Worksheets.Item(1)
$drawC = Read-SheetGeom $sC
Close-AllWorkbooks $excel
$treeCxn = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cxnOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ connectors = $true }
} 'xlsx-xsg-inspect-cxn'
$cxnNode = @($treeCxn.sheets[0].connectors | Where-Object { $_ }) | Select-Object -First 1
$cxnId = [string]$cxnNode.id
$cxnPrst = [string]$cxnNode.prst
$xsg3ok = ($capC -notmatch 'Repair') -and ($drawC.CxnCount -ge 1) -and ($cxnId -like 'sheet:*/cxn:*') -and ($cxnPrst -eq 'straightConnector1')
Add-Check 'xlsx-Xsg3' $xsg3ok "caption=$capC n=$($drawC.CxnCount) prst=$cxnPrst id=$cxnId"

$mutOut = Get-OutPath 'xlsx-shapes-geom-mut.xlsx'
if (Test-Path $mutOut) { Remove-Item -Force $mutOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $shapeOut.Replace('\', '/')
    output = $mutOut.Replace('\', '/')
    ops = @(
        @{ op = 'insertConnector'; id = 'sheet:Sheet1'; from = 'D4' }
        @{ op = 'setTextBoxSize'; id = $shapeId; cx = 2000000; cy = 1000000 }
        @{ op = 'setTextBoxAnchor'; id = $shapeId; from = 'C3' }
        @{ op = 'setShapeText'; id = $shapeId; text = 'Xsg改' }
    )
} 'xlsx-xsg-mutate' | Out-Null

Close-AllWorkbooks $excel
$wbM = Open-ExcelBook $excel $mutOut
$capM = [string]$wbM.Windows.Item(1).Caption
$sM = $wbM.Worksheets.Item(1)
$drawM = Read-SheetGeom $sM
Close-AllWorkbooks $excel
$treeM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $mutOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ shapes = $true; connectors = $true }
} 'xlsx-xsg-inspect-mut'
$mutShape = @($treeM.sheets[0].shapes | Where-Object { $_ }) | Select-Object -First 1
$mutCxn = @($treeM.sheets[0].connectors | Where-Object { $_ }) | Select-Object -First 1
$mutShapeId = [string]$mutShape.id
$mutCxnId = [string]$mutCxn.id
$textOk = ([string]$mutShape.text -eq 'Xsg改') -and ($drawM.ShapeText -match 'Xsg改')
$fromOk = [string]$mutShape.from -eq 'C3'

$delOut = Get-OutPath 'xlsx-shapes-geom-del.xlsx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $mutOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(
        @{ op = 'deleteShape'; id = $mutShapeId }
        @{ op = 'deleteConnector'; id = $mutCxnId }
    )
} 'xlsx-xsg-delete' | Out-Null

Close-AllWorkbooks $excel
$wbD = Open-ExcelBook $excel $delOut
$capD = [string]$wbD.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xsg-inspect-del'
$xsg4ok = ($capM -notmatch 'Repair') -and ($capD -notmatch 'Repair') -and $textOk -and $fromOk `
    -and ([int]$treeD.shapeCount -eq 0) -and ([int]$treeD.connectorCount -eq 0)
Add-Check 'xlsx-Xsg4' $xsg4ok "mut=$capM del=$capD text=$($mutShape.text)/$($drawM.ShapeText) from=$($mutShape.from) counts=$($treeD.shapeCount)/$($treeD.connectorCount)"

$failOut = Get-OutPath 'xlsx-shapes-geom-bad.xlsx'
$failFrom = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'insertShape'; id = 'sheet:Sheet1' })
} 'xlsx-xsg-missing-from'
$codeFrom = $null
try { $codeFrom = ($failFrom.Stderr | ConvertFrom-Json).error.code } catch {}
$failSp = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'deleteShape'; id = 'sheet:Sheet1/sp:2' })
} 'xlsx-xsg-del-sp'
$codeSp = $null
try { $codeSp = ($failSp.Stderr | ConvertFrom-Json).error.code } catch {}
$failCxn = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'setShapeText'; id = 'sheet:Sheet1/cxn:3'; text = 'x' })
} 'xlsx-xsg-text-cxn'
$codeCxn = $null
try { $codeCxn = ($failCxn.Stderr | ConvertFrom-Json).error.code } catch {}
$xsg5ok = ($failFrom.ExitCode -ne 0) -and ($codeFrom -eq 'InvalidField') `
    -and ($failSp.ExitCode -ne 0) -and ($codeSp -eq 'InvalidId') `
    -and ($failCxn.ExitCode -ne 0) -and ($codeCxn -eq 'InvalidId') `
    -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xsg5' $xsg5ok "from=$codeFrom sp=$codeSp cxn=$codeCxn wrote=$(Test-Path $failOut)"
