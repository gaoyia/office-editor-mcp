# xlsx dialogsheet / macrosheet：docs/checklist-xlsx.md 的 Xdm1–Xdm5。
# 对话框表用 COM 新建再另存 xlsx。宏表注入到 xlsx 包，不强制 Excel 打开。编号 Xdm，不要和 Xm、Xd 撞。

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

function Get-ZipText([IO.Compression.ZipArchive]$z, [string]$name) {
    $e = $z.GetEntry($name)
    if (-not $e) { return $null }
    $sr = New-Object IO.StreamReader ($e.Open())
    try { return $sr.ReadToEnd() } finally { $sr.Dispose() }
}

function Set-ZipText([IO.Compression.ZipArchive]$z, [string]$name, [string]$text) {
    $e = $z.GetEntry($name)
    if ($e) { $e.Delete() }
    $ne = $z.CreateEntry($name)
    $sw = New-Object IO.StreamWriter ($ne.Open())
    $sw.NewLine = "`n"
    try { $sw.Write($text) } finally { $sw.Dispose() }
}

function Find-DialogName($tree) {
    foreach ($s in @($tree.sheets)) {
        if ([string]$s.kind -eq 'dialogsheet') { return [string]$s.name }
    }
    return $null
}

function Get-FailCode($fail) {
    try { return ($fail.Stderr | ConvertFrom-Json).error.code } catch { return $null }
}

$hostPath = Get-OutPath 'xlsx-dialog-macro-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
        })
    }
} 'xlsx-xdm-create' | Out-Null

$dlgPath = Get-OutPath 'xlsx-dialog-macro.xlsx'
if (Test-Path $dlgPath) { Remove-Item -Force $dlgPath }

$excel = Ensure-MsExcel $hostPath
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $hostPath
# xlDialogSheet = -4116
$null = $wb.Sheets.Add([Type]::Missing, [Type]::Missing, 1, -4116)
$wb.SaveAs($dlgPath)
Close-AllWorkbooks $excel

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $dlgPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xdm-inspect-dialog'
$dlgName = Find-DialogName $treeD
$dlg = $null
if ($dlgName) {
    foreach ($s in @($treeD.sheets)) {
        if ([string]$s.name -eq $dlgName) { $dlg = $s; break }
    }
}
$keys = @()
if ($dlg) { $keys = @($dlg.PSObject.Properties.Name) }
$xdm1ok = ($null -ne $dlg) `
    -and ([string]$dlg.kind -eq 'dialogsheet') `
    -and ($keys -contains 'id') `
    -and ($keys -contains 'name') `
    -and ($keys -contains 'kind') `
    -and ($keys -contains 'hidden') `
    -and ($keys -contains 'hasHeaderFooter') `
    -and ($keys -notcontains 'pictureCount') `
    -and ($keys -notcontains 'print') `
    -and ($keys -notcontains 'sheetProtection')
Add-Check 'xlsx-Xdm1' $xdm1ok "name=$dlgName kind=$($dlg.kind) keys=$($keys -join ',')"

$wbD = Open-ExcelBook $excel $dlgPath
$capD = [string]$wbD.Windows.Item(1).Caption
$comNames = @()
$comTypes = @()
$nSheets = [int]$wbD.Sheets.Count
for ($i = 1; $i -le $nSheets; $i++) {
    $s = $wbD.Sheets.Item($i)
    $comNames += [string]$s.Name
    $comTypes += [string]$s.Type
}
$comDlg = $comNames -contains $dlgName
$xdm2ok = ($capD -notmatch 'Repair') -and $comDlg
Add-Check 'xlsx-Xdm2' $xdm2ok "caption=$capD names=$($comNames -join ',') types=$($comTypes -join ',') want=$dlgName"
Close-AllWorkbooks $excel

$macroHost = Get-OutPath 'xlsx-dialog-macro-inject-src.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $macroHost.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(
            @{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) },
            @{ name = 'Macro1'; cells = @(@{ ref = 'A1'; value = 2 }) },
            @{ name = 'Intl1'; cells = @(@{ ref = 'A1'; value = 3 }) }
        )
    }
} 'xlsx-xdm-macro-src' | Out-Null

$macroPath = Get-OutPath 'xlsx-dialog-macro-inject.xlsx'
Copy-Item -Force $macroHost $macroPath

$macroXml = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<xm:macrosheet xmlns:xm="http://schemas.microsoft.com/office/excel/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetViews>
    <sheetView workbookViewId="0" showFormulas="1"/>
  </sheetViews>
  <sheetFormatPr defaultRowHeight="15"/>
  <sheetData/>
  <pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/>
</xm:macrosheet>
"@

$z = [IO.Compression.ZipFile]::Open($macroPath, 'Update')
try {
    $rels = Get-ZipText $z 'xl/_rels/workbook.xml.rels'
    $ct = Get-ZipText $z '[Content_Types].xml'
    $rels = $rels.Replace('Target="worksheets/sheet2.xml"', 'Target="macrosheets/sheet1.xml"')
    $rels = $rels.Replace(
        'Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="macrosheets/sheet1.xml"',
        'Type="http://schemas.microsoft.com/office/2006/relationships/xlMacrosheet" Target="macrosheets/sheet1.xml"'
    )
    $rels = $rels.Replace('Target="worksheets/sheet3.xml"', 'Target="macrosheets/intlsheet1.xml"')
    $rels = $rels.Replace(
        'Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="macrosheets/intlsheet1.xml"',
        'Type="http://schemas.microsoft.com/office/2006/relationships/xlIntlMacrosheet" Target="macrosheets/intlsheet1.xml"'
    )
    $ct = $ct.Replace(
        'PartName="/xl/worksheets/sheet2.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"',
        'PartName="/xl/macrosheets/sheet1.xml" ContentType="application/vnd.ms-excel.macrosheet+xml"'
    )
    $ct = $ct.Replace(
        'PartName="/xl/worksheets/sheet3.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"',
        'PartName="/xl/macrosheets/intlsheet1.xml" ContentType="application/vnd.ms-excel.intlmacrosheet+xml"'
    )
    Set-ZipText $z 'xl/_rels/workbook.xml.rels' $rels
    Set-ZipText $z '[Content_Types].xml' $ct
    $old2 = $z.GetEntry('xl/worksheets/sheet2.xml')
    if ($old2) { $old2.Delete() }
    $old3 = $z.GetEntry('xl/worksheets/sheet3.xml')
    if ($old3) { $old3.Delete() }
    Set-ZipText $z 'xl/macrosheets/sheet1.xml' $macroXml
    Set-ZipText $z 'xl/macrosheets/intlsheet1.xml' $macroXml
} finally { $z.Dispose() }

$treeM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $macroPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xdm-inspect-macro'
$m1 = $null; $m2 = $null
foreach ($s in @($treeM.sheets)) {
    if ([string]$s.name -eq 'Macro1') { $m1 = $s }
    if ([string]$s.name -eq 'Intl1') { $m2 = $s }
}
$m1keys = @()
if ($m1) { $m1keys = @($m1.PSObject.Properties.Name) }
$xdm3ok = ($null -ne $m1) -and ($null -ne $m2) `
    -and ([string]$m1.kind -eq 'macrosheet') `
    -and ([string]$m2.kind -eq 'macrosheet') `
    -and ($m1keys -notcontains 'pictureCount') `
    -and ($m1keys -notcontains 'print') `
    -and ($m1keys -contains 'hasHeaderFooter')
Add-Check 'xlsx-Xdm3' $xdm3ok "macro=$($m1.kind) intl=$($m2.kind) keys=$($m1keys -join ',')"

$delOut = Get-OutPath 'xlsx-dialog-macro-del.xlsx'
$del = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $dlgPath.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteSheet'; id = ("sheet:" + $dlgName) })
} 'xlsx-xdm-delete'
$codeDel = Get-FailCode $del

$renOut = Get-OutPath 'xlsx-dialog-macro-ren.xlsx'
$ren = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $dlgPath.Replace('\', '/')
    output = $renOut.Replace('\', '/')
    ops = @(@{ op = 'renameSheet'; id = ("sheet:" + $dlgName); name = 'X' })
} 'xlsx-xdm-rename'
$codeRen = Get-FailCode $ren

$hidOut = Get-OutPath 'xlsx-dialog-macro-hid.xlsx'
$hid = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $dlgPath.Replace('\', '/')
    output = $hidOut.Replace('\', '/')
    ops = @(@{ op = 'setSheetHidden'; id = ("sheet:" + $dlgName); hidden = $true })
} 'xlsx-xdm-hide'
$codeHid = Get-FailCode $hid

$xdm4ok = ($del.ExitCode -ne 0) -and ($codeDel -eq 'NotImplemented') -and (-not (Test-Path $delOut)) `
    -and ($ren.ExitCode -ne 0) -and ($codeRen -eq 'NotImplemented') -and (-not (Test-Path $renOut)) `
    -and ($hid.ExitCode -ne 0) -and ($codeHid -eq 'NotImplemented') -and (-not (Test-Path $hidOut))
Add-Check 'xlsx-Xdm4' $xdm4ok "del=$codeDel ren=$codeRen hid=$codeHid"

$createBad = Get-OutPath 'xlsx-dialog-macro-create.xlsx'
$cr = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'create'
    path = $createBad.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; kind = 'dialogsheet' }) }
} 'xlsx-xdm-create-kind'
$codeCr = Get-FailCode $cr

$cell = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'inspect'
    path = $dlgPath.Replace('\', '/')
    format = 'xlsx'
    scope = @{ sheet = $dlgName; range = 'A1' }
} 'xlsx-xdm-cells'
$codeCell = Get-FailCode $cell

$xdm5ok = ($cr.ExitCode -ne 0) -and ($codeCr -eq 'InvalidField') -and (-not (Test-Path $createBad)) `
    -and ($cell.ExitCode -ne 0) -and ($codeCell -eq 'InvalidField')
Add-Check 'xlsx-Xdm5' $xdm5ok "create=$codeCr cells=$codeCell"
