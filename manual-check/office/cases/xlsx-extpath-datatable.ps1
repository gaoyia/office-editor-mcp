# xlsx 外部引用路径与数据表公式：docs/checklist-xlsx.md 的 Xef1–Xef5。
# 数据表由 Excel 16.0 Range.Table 写入。编号 Xef，不要和 Xel、Xf 撞。

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

function Find-Cell($cells, [string]$id) {
    foreach ($c in $cells) {
        if ($c.id -eq $id) { return $c }
    }
    return $null
}

function Get-ExcelLinkSources($wb) {
    $raw = $null
    try { $raw = $wb.LinkSources(1) } catch { return @() }
    if ($null -eq $raw) { return @() }
    return @($raw)
}

function Get-ZipText([string]$xlsx, [string]$entry) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem | Out-Null
    $z = [IO.Compression.ZipFile]::OpenRead($xlsx)
    try {
        $e = $z.GetEntry($entry)
        if ($null -eq $e) { return '' }
        $sr = New-Object IO.StreamReader($e.Open())
        try { return $sr.ReadToEnd() }
        finally { $sr.Dispose() }
    }
    finally { $z.Dispose() }
}

$hostPath = Get-OutPath 'xlsx-xef-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(
                @{ ref = 'C1'; value = 10 }
                @{ ref = 'D1'; value = 20 }
                @{ ref = 'B1'; formula = '=C1*2' }
                @{ ref = 'A2'; value = 1 }
                @{ ref = 'A3'; value = 2 }
                @{ ref = 'A4'; value = 3 }
            )
        })
    }
} 'xlsx-xef-create' | Out-Null

$srcPath = Get-OutPath 'xlsx-xef-source.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $srcPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 42 })
        })
    }
} 'xlsx-xef-source' | Out-Null

$altPath = Get-OutPath 'xlsx-xef-alt.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $altPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 99 })
        })
    }
} 'xlsx-xef-alt' | Out-Null

$dtPath = Get-OutPath 'xlsx-xef-datatable.xlsx'
if (Test-Path $dtPath) { Remove-Item -Force $dtPath }
Copy-Item $hostPath $dtPath

$excel = Ensure-MsExcel $hostPath
Close-AllWorkbooks $excel
$wbDt = Open-ExcelBook $excel $dtPath
$wsDt = $wbDt.Worksheets.Item(1)
$wsDt.Range('A1:B4').Table($null, $wsDt.Range('C1'))
$wbDt.Save()
Release-Com $wsDt
Close-AllWorkbooks $excel

$treeDt = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $dtPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:D4' }
} 'xlsx-xef-inspect-dt'
$b2 = Find-Cell $treeDt.sheets[0].cells 'cell:Sheet1!B2'
$b3 = Find-Cell $treeDt.sheets[0].cells 'cell:Sheet1!B3'
$xef1ok = ($b2.formula -eq '=TABLE(,C1)') `
    -and ($b2.dataTable -eq $true) `
    -and ($b2.dataTableRef -eq 'B2:B4') `
    -and ($b3.dataTable -eq $true) `
    -and ($null -eq $b3.formula)
Add-Check 'xlsx-Xef1' $xef1ok "b2f=$($b2.formula) ref=$($b2.dataTableRef) b3dt=$($b3.dataTable) b3f=$($b3.formula)"

Close-AllWorkbooks $excel
$wbOpen = Open-ExcelBook $excel $dtPath
$capDt = [string]$wbOpen.Windows.Item(1).Caption
$wsOpen = $wbOpen.Worksheets.Item(1)
$comB2 = [string]$wsOpen.Range('B2').Formula
Release-Com $wsOpen
Close-AllWorkbooks $excel
$xef2ok = ($capDt -notmatch 'Repair') -and ($comB2 -eq '=TABLE(,C1)')
Add-Check 'xlsx-Xef2' $xef2ok "caption=$capDt com=$comB2"

$linkHost = Get-OutPath 'xlsx-xef-linkhost.xlsx'
if (Test-Path $linkHost) { Remove-Item -Force $linkHost }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $linkHost.Replace('\', '/')
    ops = @(
        @{
            op = 'insertExternalLink'
            id = 'xlsx:workbook'
            path = $srcPath.Replace('\', '/')
            sheets = @('Sheet1')
        }
        @{ op = 'setCell'; id = 'cell:Sheet1!E1'; formula = '=[1]Sheet1!A1' }
    )
} 'xlsx-xef-link' | Out-Null

$repath = Get-OutPath 'xlsx-xef-repath.xlsx'
if (Test-Path $repath) { Remove-Item -Force $repath }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $linkHost.Replace('\', '/')
    output = $repath.Replace('\', '/')
    ops = @(@{
        op = 'setExternalLink'
        id = 'extlink:1'
        path = $altPath.Replace('\', '/')
    })
} 'xlsx-xef-repath' | Out-Null

Close-AllWorkbooks $excel
$wbL = Open-ExcelBook $excel $repath
$capL = [string]$wbL.Windows.Item(1).Caption
$links = Get-ExcelLinkSources $wbL
$linkText = ($links | ForEach-Object { [string]$_ }) -join '|'
Close-AllWorkbooks $excel
$treeL = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $repath.Replace('\', '/'); format = 'xlsx'
    scope = @{ externalLinks = $true; sheet = 'Sheet1'; range = 'E1:E1' }
} 'xlsx-xef-inspect-repath'
$item = $null
if ($treeL.externalLinks) { $item = @($treeL.externalLinks) | Select-Object -First 1 }
$e1 = Find-Cell $treeL.sheets[0].cells 'cell:Sheet1!E1'
$altName = [IO.Path]::GetFileName($altPath)
$pathOk = $false
if ($item) { $pathOk = ([string]$item.path) -match [regex]::Escape($altName) }
$linkOk = $linkText -match [regex]::Escape($altName)
$fOk = $false
if ($e1) { $fOk = ([string]$e1.formula) -match '\[1\]' }
$xef3ok = ($capL -notmatch 'Repair') -and $pathOk -and $linkOk -and $fOk
Add-Check 'xlsx-Xef3' $xef3ok "caption=$capL links=$linkText path=$($item.path) formula=$($e1.formula)"

$tblOut = Get-OutPath 'xlsx-xef-table-set.xlsx'
if (Test-Path $tblOut) { Remove-Item -Force $tblOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $dtPath.Replace('\', '/')
    output = $tblOut.Replace('\', '/')
    ops = @(@{
        op = 'setCell'
        id = 'cell:Sheet1!B2'
        formula = '=TABLE(,D1)'
    })
} 'xlsx-xef-table' | Out-Null

$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tblOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ sheet = 'Sheet1'; range = 'A1:D4' }
} 'xlsx-xef-inspect-table'
$b2t = Find-Cell $treeT.sheets[0].cells 'cell:Sheet1!B2'
Close-AllWorkbooks $excel
$wbT = Open-ExcelBook $excel $tblOut
$capT = [string]$wbT.Windows.Item(1).Caption
$wsT = $wbT.Worksheets.Item(1)
$comT = [string]$wsT.Range('B2').Formula
Release-Com $wsT
Close-AllWorkbooks $excel
$sheetXml = Get-ZipText $tblOut 'xl/worksheets/sheet1.xml'
$noBody = $sheetXml -match 't="dataTable"' -and $sheetXml -notmatch '>TABLE'
$xef4ok = ($b2t.formula -eq '=TABLE(,D1)') `
    -and ($capT -notmatch 'Repair') `
    -and ($comT -eq '=TABLE(,D1)') `
    -and $noBody
Add-Check 'xlsx-Xef4' $xef4ok "insp=$($b2t.formula) caption=$capT com=$comT nobody=$noBody"

$failDep = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $dtPath.Replace('\', '/')
    output = (Get-OutPath 'xlsx-xef-fail-dep.xlsx').Replace('\', '/')
    ops = @(@{ op = 'setCell'; id = 'cell:Sheet1!B3'; formula = '=A3' })
} 'xlsx-xef-fail-dep'
$failMaster = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $dtPath.Replace('\', '/')
    output = (Get-OutPath 'xlsx-xef-fail-master.xlsx').Replace('\', '/')
    ops = @(@{ op = 'setCell'; id = 'cell:Sheet1!B2'; formula = '=A1' })
} 'xlsx-xef-fail-master'
$failPath = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $linkHost.Replace('\', '/')
    output = (Get-OutPath 'xlsx-xef-fail-path.xlsx').Replace('\', '/')
    ops = @(@{ op = 'setExternalLink'; id = 'extlink:1' })
} 'xlsx-xef-fail-path'
$failId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $linkHost.Replace('\', '/')
    output = (Get-OutPath 'xlsx-xef-fail-id.xlsx').Replace('\', '/')
    ops = @(@{
        op = 'setExternalLink'
        id = 'slide:1'
        path = $altPath.Replace('\', '/')
    })
} 'xlsx-xef-fail-id'
$xef5ok = ($failDep.ExitCode -ne 0) `
    -and ($failMaster.ExitCode -ne 0) `
    -and ($failPath.ExitCode -ne 0) `
    -and ($failId.ExitCode -ne 0) `
    -and (-not (Test-Path (Get-OutPath 'xlsx-xef-fail-dep.xlsx'))) `
    -and (-not (Test-Path (Get-OutPath 'xlsx-xef-fail-master.xlsx'))) `
    -and (-not (Test-Path (Get-OutPath 'xlsx-xef-fail-path.xlsx'))) `
    -and (-not (Test-Path (Get-OutPath 'xlsx-xef-fail-id.xlsx')))
Add-Check 'xlsx-Xef5' $xef5ok "dep=$($failDep.ExitCode) master=$($failMaster.ExitCode) path=$($failPath.ExitCode) id=$($failId.ExitCode)"
