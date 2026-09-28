# pptx 形状几何剩余：docs/checklist-pptx.md 的 Sg1–Sg5。
# 椭圆、连接线、组合与解开。不实现放映引擎。

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

function Find-PptShapeWhere($slide, [scriptblock]$pred) {
    $shapes = $slide.Shapes
    try {
        $n = [int]$shapes.Count
        for ($i = 1; $i -le $n; $i++) {
            $s = $shapes.Item($i)
            $keep = $false
            try { $keep = [bool](& $pred $s) } catch { $keep = $false }
            if ($keep) { return $s }
            Release-Com $s
        }
        return $null
    }
    finally { Release-Com $shapes }
}

$hostPath = Ensure-PptxCreate
$ppt = Ensure-MsPpt $hostPath

$treeHost = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-sg-host'
$shIds = @($treeHost.slides[0].shapes | ForEach-Object { [string]$_.id })
$a = $shIds[0]
$b = $shIds[1]

$ellOut = Get-OutPath 'pptx-sg-ellipse.pptx'
Close-PptPresByPath $ppt $ellOut
if (Test-Path $ellOut) { Remove-Item -Force $ellOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $ellOut.Replace('\', '/')
    ops = @(@{
        op = 'insertShape'; id = 'slide:1'; prst = 'ellipse'; text = 'oval'
        x = 914400; y = 1371600; cx = 1828800; cy = 914400
    })
} 'pptx-sg-ellipse' | Out-Null
$treeE = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ellOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-sg-ellipse-inspect'
$ell = @($treeE.slides[0].shapes) | Where-Object { [string]$_.prst -eq 'ellipse' } | Select-Object -First 1
$presE = Open-PptPres $ppt $ellOut
$capE = Get-PptCaption $presE
$sE = Get-PptSlide $presE 1
$boxE = Get-PptShape $sE ([string]$ell.name)
$auto = [int]$boxE.AutoShapeType
Release-Com $boxE
Release-Com $sE
Close-PptPres $presE
$sg1ok = ($null -ne $ell) `
    -and ([string]$ell.id -like 'slide:1/shape:*') `
    -and ([string]$ell.prst -eq 'ellipse') `
    -and ([bool]$ell.stable) `
    -and ($auto -eq 9) `
    -and ($capE -notmatch 'Repair')
Add-Check 'pptx-Sg1' $sg1ok "id=$($ell.id) prst=$($ell.prst) auto=$auto cap=$capE"

$cxnOut = Get-OutPath 'pptx-sg-cxn.pptx'
Close-PptPresByPath $ppt $cxnOut
if (Test-Path $cxnOut) { Remove-Item -Force $cxnOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $cxnOut.Replace('\', '/')
    ops = @(@{
        op = 'insertConnector'; id = 'slide:1'
        x = 635000; y = 4064000; cx = 1828800; cy = 762000
    })
} 'pptx-sg-cxn' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cxnOut.Replace('\', '/'); format = 'pptx'
    scope = @{ connectors = $true }
} 'pptx-sg-cxn-inspect'
$cxn = @($treeC.slides[0].connectors) | Select-Object -First 1
$presC = Open-PptPres $ppt $cxnOut
$capC = Get-PptCaption $presC
$sC = Get-PptSlide $presC 1
$lineC = Find-PptShapeWhere $sC { param($s) [int]$s.Connector -ne 0 }
$isCxn = $false
if ($null -ne $lineC) { $isCxn = ([int]$lineC.Connector -ne 0) }
if ($null -ne $lineC) { Release-Com $lineC }
Release-Com $sC
Close-PptPres $presC
$sg2ok = ($null -ne $cxn) `
    -and ([string]$cxn.id -like 'slide:1/cxn:*') `
    -and ([bool]$cxn.stable) `
    -and ([int64]$treeC.connectorCount -eq 1) `
    -and $isCxn `
    -and ($capC -notmatch 'Repair')
Add-Check 'pptx-Sg2' $sg2ok "id=$($cxn.id) stable=$($cxn.stable) isCxn=$isCxn cap=$capC"

$grpOut = Get-OutPath 'pptx-sg-group.pptx'
Close-PptPresByPath $ppt $grpOut
if (Test-Path $grpOut) { Remove-Item -Force $grpOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $grpOut.Replace('\', '/')
    ops = @(@{
        op = 'groupShapes'; id = 'slide:1'
        ids = @($a, $b)
    })
} 'pptx-sg-group' | Out-Null
$treeG = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $grpOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-sg-group-inspect'
$idsG = @($treeG.slides[0].shapes | ForEach-Object { [string]$_.id })
$presG = Open-PptPres $ppt $grpOut
$capG = Get-PptCaption $presG
$sG = Get-PptSlide $presG 1
$grp = Find-PptShapeWhere $sG { param($s) [int]$s.Type -eq 6 }
$typeG = if ($null -ne $grp) { [int]$grp.Type } else { 0 }
if ($null -ne $grp) { Release-Com $grp }
Release-Com $sG
Close-PptPres $presG
$sg3ok = ($typeG -eq 6) `
    -and ($idsG -contains $a) `
    -and ($idsG -contains $b) `
    -and ($capG -notmatch 'Repair')
Add-Check 'pptx-Sg3' $sg3ok "type=$typeG ids=$($idsG -join ',') cap=$capG"

$ungOut = Get-OutPath 'pptx-sg-ungroup.pptx'
Close-PptPresByPath $ppt $ungOut
if (Test-Path $ungOut) { Remove-Item -Force $ungOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $grpOut.Replace('\', '/')
    output = $ungOut.Replace('\', '/')
    ops = @(@{ op = 'ungroup'; id = $a })
} 'pptx-sg-ungroup' | Out-Null
$treeU = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ungOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-sg-ungroup-inspect'
$idsU = @($treeU.slides[0].shapes | ForEach-Object { [string]$_.id })
$presU = Open-PptPres $ppt $ungOut
$capU = Get-PptCaption $presU
$sU = Get-PptSlide $presU 1
$grpU = Find-PptShapeWhere $sU { param($s) [int]$s.Type -eq 6 }
$stillGrp = ($null -ne $grpU)
if ($null -ne $grpU) { Release-Com $grpU }
Release-Com $sU
Close-PptPres $presU
$sg4ok = (-not $stillGrp) `
    -and ($idsU -contains $a) `
    -and ($idsU -contains $b) `
    -and ($capU -notmatch 'Repair')
Add-Check 'pptx-Sg4' $sg4ok "stillGrp=$stillGrp ids=$($idsU -join ',') cap=$capU"

$failPrst = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = (Get-OutPath 'pptx-sg-fail-prst.pptx').Replace('\', '/')
    ops = @(@{ op = 'insertShape'; id = 'slide:1'; prst = 'straightConnector1' })
} 'pptx-sg-fail-prst'
$failOne = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = (Get-OutPath 'pptx-sg-fail-one.pptx').Replace('\', '/')
    ops = @(@{ op = 'groupShapes'; id = 'slide:1'; ids = @($a) })
} 'pptx-sg-fail-one'
$failIns = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = (Get-OutPath 'pptx-sg-fail-ins.pptx').Replace('\', '/')
    ops = @(@{ op = 'insertConnector'; id = $a })
} 'pptx-sg-fail-ins'
$failPrstOut = Get-OutPath 'pptx-sg-fail-prst.pptx'
$failOneOut = Get-OutPath 'pptx-sg-fail-one.pptx'
$failInsOut = Get-OutPath 'pptx-sg-fail-ins.pptx'
$sg5ok = ($failPrst.ExitCode -ne 0) `
    -and ($failOne.ExitCode -ne 0) `
    -and ($failIns.ExitCode -ne 0) `
    -and (-not (Test-Path $failPrstOut)) `
    -and (-not (Test-Path $failOneOut)) `
    -and (-not (Test-Path $failInsOut))
Add-Check 'pptx-Sg5' $sg5ok "prst=$($failPrst.ExitCode) one=$($failOne.ExitCode) ins=$($failIns.ExitCode)"
