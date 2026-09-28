# pptx 母版写入剩余：docs/checklist-pptx.md 的 Mw1–Mw5。
# 空白版式、正文占位符、第二份母版。不实现母版排版引擎。

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

function Get-CustomLayoutCount($pres) {
    $master = $pres.SlideMaster
    try {
        $cls = $master.CustomLayouts
        try { return [int]$cls.Count }
        finally { Release-Com $cls }
    }
    finally { Release-Com $master }
}

function Get-DesignCount($pres) {
    $des = $pres.Designs
    try { return [int]$des.Count }
    finally { Release-Com $des }
}

function Find-LayoutBodyPlaceholder($pres, [string]$layoutName) {
    $master = $pres.SlideMaster
    try {
        $cls = $master.CustomLayouts
        try {
            $n = [int]$cls.Count
            for ($li = 1; $li -le $n; $li++) {
                $ly = $cls.Item($li)
                $nm = [string]$ly.Name
                if ($nm -ne $layoutName) {
                    Release-Com $ly
                    continue
                }
                try {
                    $shapes = $ly.Shapes
                    try {
                        $sn = [int]$shapes.Count
                        for ($i = 1; $i -le $sn; $i++) {
                            $s = $shapes.Item($i)
                            $ty = 0
                            try { $ty = [int]$s.Type } catch { $ty = 0 }
                            if ($ty -eq 14) { return $s }
                            Release-Com $s
                        }
                        return $null
                    }
                    finally { Release-Com $shapes }
                }
                finally { Release-Com $ly }
            }
            return $null
        }
        finally { Release-Com $cls }
    }
    finally { Release-Com $master }
}

$hostPath = Ensure-PptxCreate
$ppt = Ensure-MsPpt $hostPath

# Mw1：insertLayout 后 masterCount 仍为 1，版式名单多一条，COM Count 加 1，无 Repair。
$pres0 = Open-PptPres $ppt $hostPath
$n0 = Get-CustomLayoutCount $pres0
Close-PptPres $pres0
$lyOut = Get-OutPath 'pptx-mw-layout.pptx'
Close-PptPresByPath $ppt $lyOut
if (Test-Path $lyOut) { Remove-Item -Force $lyOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $lyOut.Replace('\', '/')
    ops = @(@{ op = 'insertLayout'; id = 'master:1'; name = '空白 2' })
} 'pptx-mw-layout' | Out-Null
$treeL = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $lyOut.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-mw-layout-inspect'
$layoutN = 0
$kind2 = $null
try { $layoutN = @($treeL.masters[0].layouts).Count } catch {}
try { $kind2 = [string]$treeL.masters[0].layouts[1].kind } catch {}
$presL = Open-PptPres $ppt $lyOut
$capL = Get-PptCaption $presL
$n1 = Get-CustomLayoutCount $presL
Close-PptPres $presL
$mw1ok = ([int]$treeL.masterCount -eq 1) `
    -and ($layoutN -eq ($n0 + 1)) `
    -and ($kind2 -eq 'blank') `
    -and ($n1 -eq ($n0 + 1)) `
    -and ($capL -notmatch 'Repair')
Add-Check 'pptx-Mw1' $mw1ok "masterCount=$($treeL.masterCount) layouts=$layoutN kind=$kind2 com=$n0->$n1 cap=$capL"

# Mw2：对新版式 insertPlaceholder type=body，COM Type=14，无 Repair。
$phOut = Get-OutPath 'pptx-mw-ph.pptx'
Close-PptPresByPath $ppt $phOut
if (Test-Path $phOut) { Remove-Item -Force $phOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $lyOut.Replace('\', '/')
    output = $phOut.Replace('\', '/')
    ops = @(@{ op = 'insertPlaceholder'; id = 'master:1/layout:2'; type = 'body' })
} 'pptx-mw-ph' | Out-Null
$treeP = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $phOut.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-mw-ph-inspect'
$ph = $null
try {
    $phs = @($treeP.masters[0].layouts[1].placeholders)
    $ph = $phs | Where-Object { [string]$_.type -eq 'body' } | Select-Object -First 1
} catch { $ph = $null }
$presP = Open-PptPres $ppt $phOut
$capP = Get-PptCaption $presP
$body = Find-LayoutBodyPlaceholder $presP '空白 2'
$tyP = if ($null -ne $body) { [int]$body.Type } else { 0 }
if ($null -ne $body) { Release-Com $body }
Close-PptPres $presP
$mw2ok = ($null -ne $ph) `
    -and ([string]$ph.type -eq 'body') `
    -and ($tyP -eq 14) `
    -and ($capP -notmatch 'Repair')
Add-Check 'pptx-Mw2' $mw2ok "id=$($ph.id) type=$($ph.type) comType=$tyP cap=$capP"

# Mw3：deletePlaceholder 后该占位符没了，无 Repair。
$delOut = Get-OutPath 'pptx-mw-delph.pptx'
Close-PptPresByPath $ppt $delOut
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $phOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deletePlaceholder'; id = [string]$ph.id })
} 'pptx-mw-delph' | Out-Null
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-mw-delph-inspect'
$still = 0
try { $still = @($treeD.masters[0].layouts[1].placeholders).Count } catch {}
$presDel = Open-PptPres $ppt $delOut
$capDel = Get-PptCaption $presDel
$bodyD = Find-LayoutBodyPlaceholder $presDel '空白 2'
$gone = ($null -eq $bodyD)
if ($null -ne $bodyD) { Release-Com $bodyD }
Close-PptPres $presDel
$mw3ok = ($still -eq 0) -and $gone -and ($capDel -notmatch 'Repair')
Add-Check 'pptx-Mw3' $mw3ok "phs=$still gone=$gone cap=$capDel"

# Mw4：insertMaster 后 masterCount=2，无 Repair；占用时 deleteMaster / deleteLayout 为 InvalidField。
$mOut = Get-OutPath 'pptx-mw-master.pptx'
Close-PptPresByPath $ppt $mOut
if (Test-Path $mOut) { Remove-Item -Force $mOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $mOut.Replace('\', '/')
    ops = @(@{ op = 'insertMaster'; id = 'pptx:presentation' })
} 'pptx-mw-master' | Out-Null
$treeM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $mOut.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-mw-master-inspect'
$presM = Open-PptPres $ppt $mOut
$capM = Get-PptCaption $presM
$desN = Get-DesignCount $presM
Close-PptPres $presM
$failLy = Get-OutPath 'pptx-mw-fail-layout.pptx'
$failMs = Get-OutPath 'pptx-mw-fail-master.pptx'
if (Test-Path $failLy) { Remove-Item -Force $failLy }
if (Test-Path $failMs) { Remove-Item -Force $failMs }
$badLy = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $mOut.Replace('\', '/')
    output = $failLy.Replace('\', '/')
    ops = @(@{ op = 'deleteLayout'; id = 'master:1/layout:1' })
} 'pptx-mw-fail-layout'
$badMs = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $mOut.Replace('\', '/')
    output = $failMs.Replace('\', '/')
    ops = @(@{ op = 'deleteMaster'; id = 'master:1' })
} 'pptx-mw-fail-master'
$mw4ok = ([int]$treeM.masterCount -eq 2) `
    -and ($desN -eq 2) `
    -and ($capM -notmatch 'Repair') `
    -and ((Get-PptErrorCode $badLy) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $badMs) -eq 'InvalidField') `
    -and (-not (Test-Path $failLy)) `
    -and (-not (Test-Path $failMs))
Add-Check 'pptx-Mw4' $mw4ok "masterCount=$($treeM.masterCount) designs=$desN cap=$capM ly=$(Get-PptErrorCode $badLy) ms=$(Get-PptErrorCode $badMs)"

# Mw5：页 id 得 InvalidId；坏 type 得 InvalidField；xlsx / docx 得 InvalidField。
$failSlideOut = Get-OutPath 'pptx-mw-fail-slide.pptx'
$failTypeOut = Get-OutPath 'pptx-mw-fail-type.pptx'
if (Test-Path $failSlideOut) { Remove-Item -Force $failSlideOut }
if (Test-Path $failTypeOut) { Remove-Item -Force $failTypeOut }
$failSlide = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failSlideOut.Replace('\', '/')
    ops = @(@{ op = 'insertLayout'; id = 'slide:1' })
} 'pptx-mw-fail-slide'
$failType = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failTypeOut.Replace('\', '/')
    ops = @(@{ op = 'insertPlaceholder'; id = 'master:1/layout:1'; type = 'obj' })
} 'pptx-mw-fail-type'
$xlsx = Ensure-XlsxCreate
$docx = Ensure-DocxCreate
$badX = Get-OutPath 'pptx-mw-fail-xlsx.xlsx'
$badD = Get-OutPath 'pptx-mw-fail-docx.docx'
if (Test-Path $badX) { Remove-Item -Force $badX }
if (Test-Path $badD) { Remove-Item -Force $badD }
$failX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $badX.Replace('\', '/')
    ops = @(@{ op = 'insertLayout'; id = 'master:1' })
} 'pptx-mw-fail-xlsx'
$failD = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $docx.Replace('\', '/')
    output = $badD.Replace('\', '/')
    ops = @(@{ op = 'insertLayout'; id = 'master:1' })
} 'pptx-mw-fail-docx'
$mw5ok = ((Get-PptErrorCode $failSlide) -eq 'InvalidId') `
    -and ((Get-PptErrorCode $failType) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failX) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failD) -eq 'InvalidField') `
    -and (-not (Test-Path $failSlideOut)) `
    -and (-not (Test-Path $failTypeOut)) `
    -and (-not (Test-Path $badX)) `
    -and (-not (Test-Path $badD))
Add-Check 'pptx-Mw5' $mw5ok "slide=$(Get-PptErrorCode $failSlide) type=$(Get-PptErrorCode $failType) x=$(Get-PptErrorCode $failX) d=$(Get-PptErrorCode $failD)"
