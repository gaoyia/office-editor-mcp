# xlsx 组合：docs/checklist-xlsx.md 的 Xg1–Xg5。
# 用 create 产物再 apply。不按列宽反推已有 twoCell 图尺寸。

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

function Read-SheetGroups($sheet) {
    $groupN = 0
    $childN = 0
    $shapeN = 0
    $cxnN = 0
    $shapes = $null
    try { $shapes = $sheet.Shapes } catch { }
    if ($null -eq $shapes) {
        return @{ GroupCount = 0; ChildCount = 0; ShapeCount = 0; CxnCount = 0 }
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
            if ($ty -eq 6) {
                $groupN++
                try { $childN += [int]$shp.GroupItems.Count } catch { }
            }
            else {
                $isCxn = $false
                try { $isCxn = [bool]$shp.Connector } catch { }
                if ($isCxn -or ($ty -eq 9)) { $cxnN++ } else { $shapeN++ }
            }
        }
        finally { Release-Com $shp }
    }
    Release-Com $shapes
    return @{ GroupCount = $groupN; ChildCount = $childN; ShapeCount = $shapeN; CxnCount = $cxnN }
}

$hostPath = Get-OutPath 'xlsx-groups-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
        })
    }
} 'xlsx-xg-create' | Out-Null

$twoOut = Get-OutPath 'xlsx-groups-two.xlsx'
if (Test-Path $twoOut) { Remove-Item -Force $twoOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $twoOut.Replace('\', '/')
    ops = @(
        @{ op = 'insertShape'; id = 'sheet:Sheet1'; from = 'B2'; text = '甲' }
        @{ op = 'insertShape'; id = 'sheet:Sheet1'; from = 'E4'; text = '乙' }
    )
} 'xlsx-xg-two' | Out-Null
$treeTwo = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $twoOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ shapes = $true }
} 'xlsx-xg-inspect-two'
$ids = @($treeTwo.sheets[0].shapes | ForEach-Object { [string]$_.id })
$a = $ids[0]
$b = $ids[1]

$grpOut = Get-OutPath 'xlsx-groups-grouped.xlsx'
if (Test-Path $grpOut) { Remove-Item -Force $grpOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $twoOut.Replace('\', '/')
    output = $grpOut.Replace('\', '/')
    ops = @(@{ op = 'groupShapes'; id = 'sheet:Sheet1'; ids = @($a, $b) })
} 'xlsx-xg-group' | Out-Null

$excel = Ensure-MsExcel $grpOut
Close-AllWorkbooks $excel
$wbG = Open-ExcelBook $excel $grpOut
$capG = [string]$wbG.Windows.Item(1).Caption
$sG = $wbG.Worksheets.Item(1)
$drawG = Read-SheetGroups $sG
Close-AllWorkbooks $excel
$treeG = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $grpOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ shapes = $true }
} 'xlsx-xg-inspect-grp'
$idsG = @($treeG.sheets[0].shapes | ForEach-Object { [string]$_.id })
$xg1ok = ($capG -notmatch 'Repair') -and ($drawG.GroupCount -eq 1) -and ($drawG.ChildCount -eq 2) `
    -and ($idsG -contains $a) -and ($idsG -contains $b)
Add-Check 'xlsx-Xg1' $xg1ok "caption=$capG groups=$($drawG.GroupCount) children=$($drawG.ChildCount) ids=$($idsG -join ',')"

$ugOut = Get-OutPath 'xlsx-groups-ungroup.xlsx'
if (Test-Path $ugOut) { Remove-Item -Force $ugOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $grpOut.Replace('\', '/')
    output = $ugOut.Replace('\', '/')
    ops = @(@{ op = 'ungroup'; id = $a })
} 'xlsx-xg-ungroup' | Out-Null
Close-AllWorkbooks $excel
$wbU = Open-ExcelBook $excel $ugOut
$capU = [string]$wbU.Windows.Item(1).Caption
$sU = $wbU.Worksheets.Item(1)
$drawU = Read-SheetGroups $sU
Close-AllWorkbooks $excel
$treeU = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ugOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ shapes = $true }
} 'xlsx-xg-inspect-ug'
$idsU = @($treeU.sheets[0].shapes | ForEach-Object { [string]$_.id })
$xg2ok = ($capU -notmatch 'Repair') -and ($drawU.GroupCount -eq 0) -and ($drawU.ShapeCount -ge 2) `
    -and ($idsU -contains $a) -and ($idsU -contains $b)
Add-Check 'xlsx-Xg2' $xg2ok "caption=$capU groups=$($drawU.GroupCount) shapes=$($drawU.ShapeCount) ids=$($idsU -join ',')"

$mixOut = Get-OutPath 'xlsx-groups-mix.xlsx'
if (Test-Path $mixOut) { Remove-Item -Force $mixOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $mixOut.Replace('\', '/')
    ops = @(
        @{ op = 'insertShape'; id = 'sheet:Sheet1'; from = 'B2' }
        @{ op = 'insertConnector'; id = 'sheet:Sheet1'; from = 'D4' }
    )
} 'xlsx-xg-mix-ins' | Out-Null
$treeMix = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $mixOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ shapes = $true; connectors = $true }
} 'xlsx-xg-inspect-mix'
$sid = [string]$treeMix.sheets[0].shapes[0].id
$cid = [string]$treeMix.sheets[0].connectors[0].id
$mixGrp = Get-OutPath 'xlsx-groups-mix-grouped.xlsx'
if (Test-Path $mixGrp) { Remove-Item -Force $mixGrp }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $mixOut.Replace('\', '/')
    output = $mixGrp.Replace('\', '/')
    ops = @(@{ op = 'groupShapes'; id = 'sheet:Sheet1'; ids = @($sid, $cid) })
} 'xlsx-xg-mix-group' | Out-Null
Close-AllWorkbooks $excel
$wbM = Open-ExcelBook $excel $mixGrp
$capM = [string]$wbM.Windows.Item(1).Caption
$sM = $wbM.Worksheets.Item(1)
$drawM = Read-SheetGroups $sM
Close-AllWorkbooks $excel
$treeMG = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $mixGrp.Replace('\', '/'); format = 'xlsx'
    scope = @{ shapes = $true; connectors = $true }
} 'xlsx-xg-inspect-mixg'
$xg3ok = ($capM -notmatch 'Repair') -and ($drawM.GroupCount -eq 1) `
    -and ([int]$treeMG.shapeCount -eq 1) -and ([int]$treeMG.connectorCount -eq 1)
Add-Check 'xlsx-Xg3' $xg3ok "caption=$capM groups=$($drawM.GroupCount) shape=$($treeMG.shapeCount) cxn=$($treeMG.connectorCount)"

$failAgain = Get-OutPath 'xlsx-groups-again.xlsx'
$failNested = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $grpOut.Replace('\', '/')
    output = $failAgain.Replace('\', '/')
    ops = @(@{ op = 'groupShapes'; id = 'sheet:Sheet1'; ids = @($a, $b) })
} 'xlsx-xg-nested'
$codeNested = $null
try { $codeNested = ($failNested.Stderr | ConvertFrom-Json).error.code } catch {}
$xg4ok = ($failNested.ExitCode -ne 0) -and ($codeNested -eq 'InvalidField') -and (-not (Test-Path $failAgain))
Add-Check 'xlsx-Xg4' $xg4ok "code=$codeNested wrote=$(Test-Path $failAgain)"

$failOut = Get-OutPath 'xlsx-groups-bad.xlsx'
$failOne = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $twoOut.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'groupShapes'; id = 'sheet:Sheet1'; ids = @($a) })
} 'xlsx-xg-one'
$codeOne = $null
try { $codeOne = ($failOne.Stderr | ConvertFrom-Json).error.code } catch {}
$failSlide = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $twoOut.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'groupShapes'; id = 'slide:1'; ids = @($a, $b) })
} 'xlsx-xg-slide'
$codeSlide = $null
try { $codeSlide = ($failSlide.Stderr | ConvertFrom-Json).error.code } catch {}
$failBare = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $twoOut.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'ungroup'; id = $a })
} 'xlsx-xg-bare'
$codeBare = $null
try { $codeBare = ($failBare.Stderr | ConvertFrom-Json).error.code } catch {}
$xg5ok = ($failOne.ExitCode -ne 0) -and ($codeOne -eq 'InvalidField') `
    -and ($failSlide.ExitCode -ne 0) -and ($codeSlide -eq 'InvalidId') `
    -and ($failBare.ExitCode -ne 0) -and ($codeBare -eq 'InvalidField') `
    -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xg5' $xg5ok "one=$codeOne slide=$codeSlide bare=$codeBare wrote=$(Test-Path $failOut)"
