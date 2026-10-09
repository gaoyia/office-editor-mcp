# pptx 自定义几何：docs/checklist-pptx.md 的 Pcg1–Pcg5。
# 自由形状路径与连接线绑点。不算自动绕障碍。

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

function Get-PptErrorCode($raw) {
    try { return ($raw.Stderr | ConvertFrom-Json).error.code } catch { return $null }
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

$paths = @(
    @{
        w = 1905001; h = 1270001
        ops = @(
            @{ type = 'moveTo'; x = 0; y = 0 }
            @{ type = 'lnTo'; x = 1905000; y = 0 }
            @{ type = 'lnTo'; x = 1905000; y = 1270000 }
            @{ type = 'lnTo'; x = 0; y = 1270000 }
        )
    }
)
$paths2 = @(
    @{
        w = 1905001; h = 1270001
        ops = @(
            @{ type = 'moveTo'; x = 0; y = 0 }
            @{ type = 'lnTo'; x = 1905000; y = 0 }
            @{ type = 'lnTo'; x = 1905000; y = 635000 }
            @{ type = 'lnTo'; x = 0; y = 1270000 }
            @{ type = 'close' }
        )
    }
)

$hostPath = Ensure-PptxCreate
$ppt = Ensure-MsPpt $hostPath

$treeHost = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-pcg-host'
$shIds = @($treeHost.slides[0].shapes | ForEach-Object { [string]$_.id })
$boxA = $shIds[0]
$boxB = $shIds[1]

$geomOut = Get-OutPath 'pptx-pcg-geom.pptx'
Close-PptPresByPath $ppt $geomOut
if (Test-Path $geomOut) { Remove-Item -Force $geomOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $geomOut.Replace('\', '/')
    ops = @(@{
        op = 'insertShape'; id = 'slide:1'
        geom = @{ paths = $paths }
        x = 914400; y = 914400; cx = 1905001; cy = 1270001
    })
} 'pptx-pcg-insert' | Out-Null
$treeG = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $geomOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-pcg-insert-inspect'
$free = @($treeG.slides[0].shapes) | Where-Object { $null -ne $_.geom } | Select-Object -First 1
$presG = Open-PptPres $ppt $geomOut
$capG = Get-PptCaption $presG
$sG = Get-PptSlide $presG 1
$boxG = Get-PptShape $sG ([string]$free.name)
$typ = [int]$boxG.Type
$auto = [int]$boxG.AutoShapeType
Release-Com $boxG
Release-Com $sG
Close-PptPres $presG
$pcg1ok = ($null -ne $free) `
    -and ($null -eq $free.prst) `
    -and ([int]$free.geom.paths[0].w -eq 1905001) `
    -and ([string]$free.geom.paths[0].ops[0].type -eq 'moveTo') `
    -and ($typ -eq 5) `
    -and ($auto -eq 138) `
    -and ($capG -notmatch 'Repair')
Add-Check 'pptx-Pcg1' $pcg1ok "id=$($free.id) type=$typ auto=$auto cap=$capG"

$cxnOut = Get-OutPath 'pptx-pcg-cxn.pptx'
Close-PptPresByPath $ppt $cxnOut
if (Test-Path $cxnOut) { Remove-Item -Force $cxnOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $cxnOut.Replace('\', '/')
    ops = @(@{
        op = 'insertConnector'; id = 'slide:1'
        start = @{ id = $boxA; site = 1 }
        end = @{ id = $boxB; site = 1 }
    })
} 'pptx-pcg-cxn' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cxnOut.Replace('\', '/'); format = 'pptx'
    scope = @{ connectors = $true; slide = 1 }
} 'pptx-pcg-cxn-inspect'
$cxn = @($treeC.slides[0].connectors) | Select-Object -First 1
$xmlC = Get-ZipPartText $cxnOut 'ppt/slides/slide1.xml'
$hasSt = $xmlC -match '<a:stCxn[^>]*idx="0"'
$hasEnd = $xmlC -match '<a:endCxn[^>]*idx="0"'
$presC = Open-PptPres $ppt $cxnOut
$capC = Get-PptCaption $presC
$sC = Get-PptSlide $presC 1
$lineC = Find-PptShapeWhere $sC { param($s) [int]$s.Connector -ne 0 }
$isCxn = $false
if ($null -ne $lineC) { $isCxn = ([int]$lineC.Connector -ne 0) }
if ($null -ne $lineC) { Release-Com $lineC }
Release-Com $sC
Close-PptPres $presC
$pcg2ok = ($null -ne $cxn) `
    -and ([string]$cxn.start.id -eq $boxA) `
    -and ([string]$cxn.end.id -eq $boxB) `
    -and $hasSt `
    -and $hasEnd `
    -and $isCxn `
    -and ($capC -notmatch 'Repair')
Add-Check 'pptx-Pcg2' $pcg2ok "start=$($cxn.start.id) end=$($cxn.end.id) cap=$capC"

$setOut = Get-OutPath 'pptx-pcg-set.pptx'
Close-PptPresByPath $ppt $setOut
if (Test-Path $setOut) { Remove-Item -Force $setOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $geomOut.Replace('\', '/')
    output = $setOut.Replace('\', '/')
    ops = @(@{
        op = 'setShapeGeom'; id = [string]$free.id
        paths = $paths2
    })
} 'pptx-pcg-set' | Out-Null
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $setOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-pcg-set-inspect'
$freeS = @($treeS.slides[0].shapes) | Where-Object { [string]$_.id -eq [string]$free.id } | Select-Object -First 1
$opsS = @($freeS.geom.paths[0].ops)
$last = $opsS[$opsS.Count - 1]
$presS = Open-PptPres $ppt $setOut
$capSet = Get-PptCaption $presS
Close-PptPres $presS
$pcg3ok = ([string]$last.type -eq 'close') `
    -and ([int]$opsS[2].y -eq 635000) `
    -and ($capSet -notmatch 'Repair')
Add-Check 'pptx-Pcg3' $pcg3ok "last=$($last.type) y=$($opsS[2].y) cap=$capSet"

$bindOut = Get-OutPath 'pptx-pcg-setcxn.pptx'
Close-PptPresByPath $ppt $bindOut
if (Test-Path $bindOut) { Remove-Item -Force $bindOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $cxnOut.Replace('\', '/')
    output = $bindOut.Replace('\', '/')
    ops = @(@{
        op = 'setConnector'; id = [string]$cxn.id
        end = @{ id = $boxA; site = 2 }
    })
} 'pptx-pcg-setcxn' | Out-Null
$xmlB = Get-ZipPartText $bindOut 'ppt/slides/slide1.xml'
$treeB = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $bindOut.Replace('\', '/'); format = 'pptx'
    scope = @{ connectors = $true }
} 'pptx-pcg-setcxn-inspect'
$cxnB = @($treeB.slides[0].connectors) | Select-Object -First 1
$presBind = Open-PptPres $ppt $bindOut
$capBind = Get-PptCaption $presBind
Close-PptPres $presBind
$pcg4ok = ($xmlB -match '<a:endCxn[^>]*idx="1"') `
    -and ([int]$cxnB.end.site -eq 2) `
    -and ([string]$cxnB.end.id -eq $boxA) `
    -and ($capBind -notmatch 'Repair')
Add-Check 'pptx-Pcg4' $pcg4ok "end=$($cxnB.end.id) site=$($cxnB.end.site) cap=$capBind"

$failBoth = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = (Get-OutPath 'pptx-pcg-fail-both.pptx').Replace('\', '/')
    ops = @(@{
        op = 'insertShape'; id = 'slide:1'; prst = 'rect'
        geom = @{ paths = $paths }
    })
} 'pptx-pcg-fail-both'
$failCubic = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = (Get-OutPath 'pptx-pcg-fail-cubic.pptx').Replace('\', '/')
    ops = @(@{
        op = 'setShapeGeom'; id = $boxA
        paths = @(@{ w = 1; h = 1; ops = @(@{ type = 'cubicBezTo'; x = 0; y = 0 }) })
    })
} 'pptx-pcg-fail-cubic'
$failCxn = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = (Get-OutPath 'pptx-pcg-fail-cxn.pptx').Replace('\', '/')
    ops = @(@{ op = 'setShapeGeom'; id = 'slide:1/cxn:4'; paths = $paths })
} 'pptx-pcg-fail-cxn'
$failBothOut = Get-OutPath 'pptx-pcg-fail-both.pptx'
$failCubicOut = Get-OutPath 'pptx-pcg-fail-cubic.pptx'
$failCxnOut = Get-OutPath 'pptx-pcg-fail-cxn.pptx'
$pcg5ok = ($failBoth.ExitCode -ne 0) `
    -and ((Get-PptErrorCode $failBoth) -eq 'InvalidField') `
    -and ($failCubic.ExitCode -ne 0) `
    -and ((Get-PptErrorCode $failCubic) -eq 'InvalidField') `
    -and ($failCxn.ExitCode -ne 0) `
    -and ((Get-PptErrorCode $failCxn) -eq 'InvalidId') `
    -and (-not (Test-Path $failBothOut)) `
    -and (-not (Test-Path $failCubicOut)) `
    -and (-not (Test-Path $failCxnOut))
Add-Check 'pptx-Pcg5' $pcg5ok "both=$(Get-PptErrorCode $failBoth) cubic=$(Get-PptErrorCode $failCubic) cxn=$(Get-PptErrorCode $failCxn)"
