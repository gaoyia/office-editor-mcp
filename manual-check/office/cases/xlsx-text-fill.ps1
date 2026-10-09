# xlsx 字填充与轮廓：docs/checklist-xlsx.md 的 Xfl1–Xfl5。
# 工作表文本框渐变字、字轮廓、混排 runs。不算排版引擎。不改 Word。

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

function Get-XlsxErrorCode($raw) {
    try { return ($raw.Stderr | ConvertFrom-Json).error.code } catch { return $null }
}

function Get-ZipEntryText([string]$zipPath, [string]$entryName) {
    $zip = [IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        $e = $zip.GetEntry($entryName)
        if ($null -eq $e) { return $null }
        $sr = New-Object IO.StreamReader($e.Open(), [Text.Encoding]::UTF8)
        try { return $sr.ReadToEnd() }
        finally { $sr.Dispose() }
    }
    finally { $zip.Dispose() }
}

function Set-ZipEntryText([string]$zipPath, [string]$entryName, [string]$text) {
    $zip = [IO.Compression.ZipFile]::Open($zipPath, 'Update')
    try {
        $e = $zip.GetEntry($entryName)
        if ($null -eq $e) { throw "没有 $entryName" }
        $e.Delete()
        $n = $zip.CreateEntry($entryName)
        $enc = New-Object Text.UTF8Encoding $false
        $sw = New-Object IO.StreamWriter($n.Open(), $enc)
        try { $sw.Write($text); $sw.Flush() }
        finally { $sw.Dispose() }
    }
    finally { $zip.Dispose() }
}

function Read-ExcelTextFill($sheet) {
    $shapes = $null
    $shp = $null
    $tf = $null
    $tr = $null
    $font = $null
    $fill = $null
    $line = $null
    $fillType = $null
    $lineVis = $null
    try {
        $shapes = $sheet.Shapes
        $n = 0
        try { $n = [int]$shapes.Count } catch { }
        for ($i = 1; $i -le $n; $i++) {
            $shp = $null
            try { $shp = $shapes.Item($i) } catch { continue }
            $ty = 0
            try { $ty = [int]$shp.Type } catch { }
            if ($ty -ne 17) {
                Release-Com $shp
                $shp = $null
                continue
            }
            $tf = $shp.TextFrame2
            $tr = $tf.TextRange
            $font = $tr.Font
            $fill = $font.Fill
            $fillType = [int]$fill.Type
            $line = $font.Line
            $lineVis = $line.Visible
            break
        }
    }
    finally {
        Release-Com $line
        Release-Com $fill
        Release-Com $font
        Release-Com $tr
        Release-Com $tf
        Release-Com $shp
        Release-Com $shapes
    }
    return @{ FillType = $fillType; LineVisible = $lineVis }
}

$fill = @{
    type = 'gradient'
    stops = @(
        @{ pos = 0; color = 'FF0000' }
        @{ pos = 100000; color = '0000FF' }
    )
}

$hostPath = Get-OutPath 'xlsx-text-fill-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xlsx-xfl-create' | Out-Null

$boxOut = Get-OutPath 'xlsx-text-fill-box.xlsx'
if (Test-Path $boxOut) { Remove-Item -Force $boxOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $boxOut.Replace('\', '/')
    ops = @(@{ op = 'insertTextBox'; id = 'sheet:Sheet1'; from = 'B2'; text = '你好' })
} 'xlsx-xfl-insert' | Out-Null
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $boxOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ textBoxes = $true }
} 'xlsx-xfl-inspect-box'
$boxId = [string]$tree0.sheets[0].textBoxes[0].id

$gradOut = Get-OutPath 'xlsx-text-fill-grad.xlsx'
if (Test-Path $gradOut) { Remove-Item -Force $gradOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $boxOut.Replace('\', '/')
    output = $gradOut.Replace('\', '/')
    ops = @(@{ op = 'setShapeFormat'; id = $boxId; textFill = $fill })
} 'xlsx-xfl-grad' | Out-Null
$treeG = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $gradOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ textBoxes = $true }
} 'xlsx-xfl-inspect-grad'
$boxG = $treeG.sheets[0].textBoxes[0]
$xmlG = Get-ZipEntryText $gradOut 'xl/drawings/drawing1.xml'
$excel = Ensure-MsExcel $gradOut
Close-AllWorkbooks $excel
$wbG = Open-ExcelBook $excel $gradOut
$capG = [string]$wbG.Windows.Item(1).Caption
$sG = $wbG.Worksheets.Item(1)
$comG = Read-ExcelTextFill $sG
Release-Com $sG
Close-AllWorkbooks $excel
$xfl1ok = ($capG -notmatch 'Repair') `
    -and ($null -eq $boxG.color) `
    -and ([string]$boxG.textFill.type -eq 'gradient') `
    -and ([int64]$boxG.textFill.ang -eq 5400000) `
    -and ($xmlG -match 'gradFill flip="none" rotWithShape="1"') `
    -and ($comG.FillType -eq 3)
Add-Check 'xlsx-Xfl1' $xfl1ok "cap=$capG type=$($boxG.textFill.type) fillType=$($comG.FillType)"

$lnOut = Get-OutPath 'xlsx-text-fill-outline.xlsx'
if (Test-Path $lnOut) { Remove-Item -Force $lnOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $boxOut.Replace('\', '/')
    output = $lnOut.Replace('\', '/')
    ops = @(@{
        op = 'setShapeFormat'; id = $boxId
        textOutline = @{ color = 'FF0000'; width = 19050 }
    })
} 'xlsx-xfl-outline' | Out-Null
$treeL = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $lnOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ textBoxes = $true }
} 'xlsx-xfl-inspect-outline'
$boxL = $treeL.sheets[0].textBoxes[0]
$xmlL = Get-ZipEntryText $lnOut 'xl/drawings/drawing1.xml'
$wbL = Open-ExcelBook $excel $lnOut
$capL = [string]$wbL.Windows.Item(1).Caption
$sL = $wbL.Worksheets.Item(1)
$comL = Read-ExcelTextFill $sL
Release-Com $sL
Close-AllWorkbooks $excel
$xfl2ok = ($capL -notmatch 'Repair') `
    -and ([int64]$boxL.textOutline.width -eq 19050) `
    -and ([string]$boxL.textOutline.color -eq 'FF0000') `
    -and ($xmlL -match 'ln w="19050"') `
    -and (($comL.LineVisible -eq -1) -or ($comL.LineVisible -eq $true))
Add-Check 'xlsx-Xfl2' $xfl2ok "cap=$capL w=$($boxL.textOutline.width) vis=$($comL.LineVisible)"

$noneOut = Get-OutPath 'xlsx-text-fill-none.xlsx'
if (Test-Path $noneOut) { Remove-Item -Force $noneOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $boxOut.Replace('\', '/')
    output = $noneOut.Replace('\', '/')
    ops = @(@{ op = 'setShapeFormat'; id = $boxId; textFill = @{ type = 'none' } })
} 'xlsx-xfl-none' | Out-Null
$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $noneOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ textBoxes = $true }
} 'xlsx-xfl-inspect-none'
$boxN = $treeN.sheets[0].textBoxes[0]
$xmlN = Get-ZipEntryText $noneOut 'xl/drawings/drawing1.xml'
$xfl3ok = ($null -eq $boxN.color) `
    -and ([string]$boxN.textFill.type -eq 'none') `
    -and ($xmlN -match '<a:noFill')
Add-Check 'xlsx-Xfl3' $xfl3ok "color=$($boxN.color) type=$($boxN.textFill.type)"

$mixOut = Get-OutPath 'xlsx-text-fill-runs.xlsx'
if (Test-Path $mixOut) { Remove-Item -Force $mixOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $mixOut.Replace('\', '/')
    ops = @(@{ op = 'insertTextBox'; id = 'sheet:Sheet1'; from = 'B2'; text = 'Aa' })
} 'xlsx-xfl-mix-ins' | Out-Null
$xmlM = Get-ZipEntryText $mixOut 'xl/drawings/drawing1.xml'
$oldRun = '<a:r><a:rPr sz="1100"/><a:t>Aa</a:t></a:r>'
$newRun = '<a:r><a:rPr sz="2800"/><a:t>A</a:t></a:r><a:r><a:rPr sz="1200"/><a:t>a</a:t></a:r>'
if ($xmlM -notlike "*$oldRun*") { throw "drawing 没有缺省 run：$xmlM" }
Set-ZipEntryText $mixOut 'xl/drawings/drawing1.xml' ($xmlM.Replace($oldRun, $newRun))
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $mixOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ textBoxes = $true }
} 'xlsx-xfl-inspect-runs'
$boxR = $treeR.sheets[0].textBoxes[0]
$runs = @($boxR.runs)
$xfl4ok = ($null -eq $boxR.sz) `
    -and ($runs.Count -eq 2) `
    -and ([int64]$runs[0].sz -eq 2800) `
    -and ([int64]$runs[1].sz -eq 1200)
Add-Check 'xlsx-Xfl4' $xfl4ok "sz=$($boxR.sz) n=$($runs.Count) a=$($runs[0].sz) b=$($runs[1].sz)"

$failBoth = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $boxOut.Replace('\', '/')
    output = (Get-OutPath 'xlsx-text-fill-fail-both.xlsx').Replace('\', '/')
    ops = @(@{
        op = 'setShapeFormat'; id = $boxId
        color = 'FF0000'
        textFill = @{ type = 'none' }
    })
} 'xlsx-xfl-fail-both'
$failBothOut = Get-OutPath 'xlsx-text-fill-fail-both.xlsx'
$codeBoth = Get-XlsxErrorCode $failBoth
$xfl5ok = ($failBoth.ExitCode -ne 0) -and ($codeBoth -eq 'InvalidField') `
    -and (-not (Test-Path $failBothOut))
Add-Check 'xlsx-Xfl5' $xfl5ok "both=$($failBoth.ExitCode)/$codeBoth"
