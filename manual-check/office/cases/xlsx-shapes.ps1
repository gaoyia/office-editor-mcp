# xlsx 形状与 SmartArt：docs/checklist-xlsx.md 的 Xsa1–Xsa5。
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

# msoTextBox = 17，msoSmartArt = 24。
function Read-SheetDrawing($sheet) {
    $boxN = 0
    $boxText = ''
    $artN = 0
    $shapes = $null
    try { $shapes = $sheet.Shapes } catch { }
    if ($null -eq $shapes) {
        return @{ BoxCount = 0; BoxText = ''; ArtCount = 0 }
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
            $hasArt = $false
            try { $hasArt = [bool]$shp.HasSmartArt } catch { }
            if ($hasArt -or ($ty -eq 24)) {
                $artN++
                continue
            }
            if ($ty -ne 17) { continue }
            $boxN++
            $tf = $null
            try { $tf = $shp.TextFrame } catch { }
            if ($null -ne $tf) {
                $chars = $null
                try { $chars = $tf.Characters() } catch { }
                if ($null -ne $chars) {
                    try {
                        $t = [string]$chars.Text
                        if ([string]::IsNullOrWhiteSpace($boxText)) { $boxText = $t }
                    } finally { Release-Com $chars }
                }
                Release-Com $tf
            }
        } finally { Release-Com $shp }
    }
    Release-Com $shapes
    return @{ BoxCount = $boxN; BoxText = $boxText; ArtCount = $artN }
}

$hostPath = Get-OutPath 'xlsx-shapes-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
        })
    }
} 'xlsx-xsa-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xsa-inspect'
$noBoxList = -not ($tree0.sheets[0].PSObject.Properties.Name -contains 'textBoxes')
$noArtList = -not ($tree0.sheets[0].PSObject.Properties.Name -contains 'smartArts')
$hasBox = ($tree0.PSObject.Properties.Name -contains 'textBoxCount') -and ([int]$tree0.textBoxCount -eq 0)
$hasArt = ($tree0.PSObject.Properties.Name -contains 'smartArtCount') -and ([int]$tree0.smartArtCount -eq 0)
$sheetBox = ($tree0.sheets[0].PSObject.Properties.Name -contains 'textBoxCount') -and ([int]$tree0.sheets[0].textBoxCount -eq 0)
$sheetArt = ($tree0.sheets[0].PSObject.Properties.Name -contains 'smartArtCount') -and ([int]$tree0.sheets[0].smartArtCount -eq 0)
$treeList = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    scope = @{ textBoxes = $true; smartArt = $true }
} 'xlsx-xsa-inspect-list'
$hasBoxList = $treeList.sheets[0].PSObject.Properties.Name -contains 'textBoxes'
$hasArtList = $treeList.sheets[0].PSObject.Properties.Name -contains 'smartArts'
$boxes0 = @($treeList.sheets[0].textBoxes | Where-Object { $_ })
$arts0 = @($treeList.sheets[0].smartArts | Where-Object { $_ })
$listEmpty = $hasBoxList -and $hasArtList -and ($boxes0.Count -eq 0) -and ($arts0.Count -eq 0)
$xsa1ok = $hasBox -and $hasArt -and $sheetBox -and $sheetArt -and $noBoxList -and $noArtList -and $listEmpty
Add-Check 'xlsx-Xsa1' $xsa1ok "box=$($tree0.textBoxCount) art=$($tree0.smartArtCount) empty=$listEmpty"

$boxOut = Get-OutPath 'xlsx-shapes-textbox.xlsx'
if (Test-Path $boxOut) { Remove-Item -Force $boxOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $boxOut.Replace('\', '/')
    ops = @(@{
        op = 'insertTextBox'
        id = 'sheet:Sheet1'
        from = 'B2'
        text = 'Xsa框'
    })
} 'xlsx-xsa-insert-box' | Out-Null

$excel = Ensure-MsExcel $boxOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $boxOut
$cap = [string]$wb.Windows.Item(1).Caption
$s = $wb.Worksheets.Item(1)
$draw = Read-SheetDrawing $s
Close-AllWorkbooks $excel
$treeBox = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $boxOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ textBoxes = $true }
} 'xlsx-xsa-inspect-box'
$boxId = [string]((@($treeBox.sheets[0].textBoxes | Where-Object { $_ }) | Select-Object -First 1).id)
$textOut = Get-OutPath 'xlsx-shapes-textbox-text.xlsx'
if (Test-Path $textOut) { Remove-Item -Force $textOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $boxOut.Replace('\', '/')
    output = $textOut.Replace('\', '/')
    ops = @(@{
        op = 'setShapeText'
        id = $boxId
        text = 'Xsa改字'
    })
} 'xlsx-xsa-set-text' | Out-Null
$wbT = Open-ExcelBook $excel $textOut
$capT = [string]$wbT.Windows.Item(1).Caption
$sT = $wbT.Worksheets.Item(1)
$drawT = Read-SheetDrawing $sT
$xsa2ok = ($cap -notmatch 'Repair') -and ($capT -notmatch 'Repair') -and ($draw.BoxCount -ge 1) -and ($drawT.BoxText -match 'Xsa改字')
Add-Check 'xlsx-Xsa2' $xsa2ok "caption=$cap/$capT boxN=$($draw.BoxCount) text=$($drawT.BoxText) id=$boxId"
Close-AllWorkbooks $excel

$artOut = Get-OutPath 'xlsx-shapes-smartart.xlsx'
if (Test-Path $artOut) { Remove-Item -Force $artOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $artOut.Replace('\', '/')
    ops = @(@{
        op = 'insertSmartArt'
        id = 'sheet:Sheet1'
        layout = 'list'
        from = 'D4'
    })
} 'xlsx-xsa-insert-art' | Out-Null

Close-AllWorkbooks $excel
$wbA = Open-ExcelBook $excel $artOut
$capA = [string]$wbA.Windows.Item(1).Caption
$sA = $wbA.Worksheets.Item(1)
$drawA = Read-SheetDrawing $sA
$treeArt = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $artOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ smartArt = $true }
} 'xlsx-xsa-inspect-art'
$artNode = @($treeArt.sheets[0].smartArts | Where-Object { $_ }) | Select-Object -First 1
$layout = [string]$artNode.layout
$artId = [string]$artNode.id
$xsa3ok = ($capA -notmatch 'Repair') -and ($drawA.ArtCount -ge 1) -and ($layout -like '*layout/default*')
Add-Check 'xlsx-Xsa3' $xsa3ok "caption=$capA artN=$($drawA.ArtCount) layout=$layout id=$artId"
Close-AllWorkbooks $excel

$delOut = Get-OutPath 'xlsx-shapes-delete.xlsx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
$boxId2 = [string]((@( (Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $textOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ textBoxes = $true }
} 'xlsx-xsa-inspect-box2').sheets[0].textBoxes | Where-Object { $_ } ) | Select-Object -First 1).id)
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $textOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(
        @{ op = 'deleteTextBox'; id = $boxId2 }
    )
} 'xlsx-xsa-delete-box' | Out-Null
$delArtOut = Get-OutPath 'xlsx-shapes-delete-art.xlsx'
if (Test-Path $delArtOut) { Remove-Item -Force $delArtOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $artOut.Replace('\', '/')
    output = $delArtOut.Replace('\', '/')
    ops = @(@{ op = 'deleteSmartArt'; id = $artId })
} 'xlsx-xsa-delete-art' | Out-Null

Close-AllWorkbooks $excel
$wbD = Open-ExcelBook $excel $delOut
$capD = [string]$wbD.Windows.Item(1).Caption
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xsa-inspect-del-box'
Close-AllWorkbooks $excel
$wbDA = Open-ExcelBook $excel $delArtOut
$capDA = [string]$wbDA.Windows.Item(1).Caption
$treeDA = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delArtOut.Replace('\', '/'); format = 'xlsx'
} 'xlsx-xsa-inspect-del-art'
$xsa4ok = ($capD -notmatch 'Repair') -and ($capDA -notmatch 'Repair') -and ([int]$treeD.textBoxCount -eq 0) -and ([int]$treeDA.smartArtCount -eq 0)
Add-Check 'xlsx-Xsa4' $xsa4ok "caption=$capD/$capDA box=$($treeD.textBoxCount) art=$($treeDA.smartArtCount)"
Close-AllWorkbooks $excel

$failOut = Get-OutPath 'xlsx-shapes-bad.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'insertTextBox'; id = 'sheet:Sheet1' })
} 'xlsx-xsa-missing'
$codeEmpty = $null
try { $codeEmpty = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$xsa5ok = ($fail.ExitCode -ne 0) -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xsa5' $xsa5ok "exit=$($fail.ExitCode) code=$codeEmpty wrote=$(Test-Path $failOut)"
