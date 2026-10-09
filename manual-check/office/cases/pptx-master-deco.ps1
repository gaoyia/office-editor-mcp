# pptx 母版装饰形：docs/checklist-pptx.md 的 Md1–Md5。
# 母版 / 版式上没有 p:ph 的 p:sp。不把装饰形抄进各页。不算母版排版引擎。

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

function Get-PptErrorCode($raw) {
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

function Get-MasterNonPlaceholderCount($pres) {
    $master = $pres.SlideMaster
    try {
        $shapes = $master.Shapes
        try {
            $n = [int]$shapes.Count
            $c = 0
            for ($i = 1; $i -le $n; $i++) {
                $s = $shapes.Item($i)
                try {
                    $ty = 0
                    try { $ty = [int]$s.Type } catch { $ty = 0 }
                    if ($ty -ne 14) { $c++ }
                }
                finally { Release-Com $s }
            }
            return $c
        }
        finally { Release-Com $shapes }
    }
    finally { Release-Com $master }
}

$hostPath = Ensure-PptxCreate
$ppt = Ensure-MsPpt $hostPath

# Md1：往母版插装饰形，inspect 能看见，不进各页，无 Repair。
$md1 = Get-OutPath 'pptx-md-ins.pptx'
Close-PptPresByPath $ppt $md1
if (Test-Path $md1) { Remove-Item -Force $md1 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $md1.Replace('\', '/')
    ops = @(@{
            op = 'insertShape'; id = 'master:1'; text = '徽标'
            x = 200000; y = 200000
        })
} 'pptx-md-ins' | Out-Null
$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $md1.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true; slide = 1 }
} 'pptx-md-ins-inspect'
$decoN = 0
$decoText = $null
$decoId = $null
try { $decoN = @($tree1.masters[0].shapes).Count } catch {}
try { $decoText = [string]$tree1.masters[0].shapes[0].text } catch {}
try { $decoId = [string]$tree1.masters[0].shapes[0].id } catch {}
$slideN = 0
$slideHasLogo = $false
try { $slideN = @($tree1.slides[0].shapes).Count } catch {}
try {
    foreach ($s in @($tree1.slides[0].shapes)) {
        if ([string]$s.text -eq '徽标') { $slideHasLogo = $true }
    }
} catch {}
$xml1 = Get-ZipEntryText $md1 'ppt/slideMasters/slideMaster1.xml'
$hasPh = $xml1 -match '<p:ph'
$pres1 = Open-PptPres $ppt $md1
$cap1 = Get-PptCaption $pres1
$com1 = Get-MasterNonPlaceholderCount $pres1
Close-PptPres $pres1
$md1ok = ($decoN -eq 1) `
    -and ($decoText -eq '徽标') `
    -and ($decoId -like 'master:1/shape:*') `
    -and (-not $slideHasLogo) `
    -and ($slideN -ge 1) `
    -and (-not $hasPh) `
    -and ($com1 -ge 1) `
    -and ($cap1 -notmatch 'Repair')
Add-Check 'pptx-Md1' $md1ok "n=$decoN text=$decoText slideN=$slideN copied=$slideHasLogo ph=$hasPh com=$com1 cap=$cap1"

# Md2：改字与几何。
$md2 = Get-OutPath 'pptx-md-edit.pptx'
Close-PptPresByPath $ppt $md2
if (Test-Path $md2) { Remove-Item -Force $md2 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $md1.Replace('\', '/')
    output = $md2.Replace('\', '/')
    ops = @(
        @{ op = 'setShapeText'; id = $decoId; text = '改过' }
        @{ op = 'setShapePos'; id = $decoId; x = 200000 }
    )
} 'pptx-md-edit' | Out-Null
$tree2 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $md2.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-md-edit-inspect'
$t2 = $null
$x2 = $null
try { $t2 = [string]$tree2.masters[0].shapes[0].text } catch {}
try { $x2 = [int64]$tree2.masters[0].shapes[0].x } catch {}
$pres2 = Open-PptPres $ppt $md2
$cap2 = Get-PptCaption $pres2
Close-PptPres $pres2
$md2ok = ($t2 -eq '改过') -and ($x2 -eq 200000) -and ($cap2 -notmatch 'Repair')
Add-Check 'pptx-Md2' $md2ok "text=$t2 x=$x2 cap=$cap2"

# Md3：绑 fade。
$md3 = Get-OutPath 'pptx-md-anim.pptx'
Close-PptPresByPath $ppt $md3
if (Test-Path $md3) { Remove-Item -Force $md3 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $md1.Replace('\', '/')
    output = $md3.Replace('\', '/')
    ops = @(@{ op = 'insertAnim'; id = $decoId; type = 'fade' })
} 'pptx-md-anim' | Out-Null
$tree3 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $md3.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true; timing = $true }
} 'pptx-md-anim-inspect'
$tgt3 = $null
$type3 = $null
try { $tgt3 = [string]$tree3.masters[0].timing[0].target } catch {}
try { $type3 = [string]$tree3.masters[0].timing[0].type } catch {}
$pres3 = Open-PptPres $ppt $md3
$cap3 = Get-PptCaption $pres3
$master3 = $pres3.SlideMaster
$tl3 = $master3.TimeLine
$seq3 = $tl3.MainSequence
try { $com3 = [int]$seq3.Count } catch { $com3 = -1 }
Release-Com $seq3
Release-Com $tl3
Release-Com $master3
Close-PptPres $pres3
$md3ok = ($tgt3 -eq $decoId) `
    -and ($type3 -eq 'fade') `
    -and ($com3 -eq 1) `
    -and ($cap3 -notmatch 'Repair')
Add-Check 'pptx-Md3' $md3ok "target=$tgt3 type=$type3 com=$com3 cap=$cap3"

# Md4：删装饰形；对占位符 deleteShape 为 InvalidId。
$withPh = Get-OutPath 'pptx-md-with-ph.pptx'
Close-PptPresByPath $ppt $withPh
if (Test-Path $withPh) { Remove-Item -Force $withPh }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $md1.Replace('\', '/')
    output = $withPh.Replace('\', '/')
    ops = @(@{ op = 'insertPlaceholder'; id = 'master:1'; type = 'title'; text = '母版标题' })
} 'pptx-md-ph' | Out-Null
$treePh = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $withPh.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-md-ph-inspect'
$phId = $null
$decoId4 = $null
try { $phId = [string]$treePh.masters[0].placeholders[0].id } catch {}
try { $decoId4 = [string]$treePh.masters[0].shapes[0].id } catch {}
$failPh = Get-OutPath 'pptx-md-fail-ph.pptx'
Close-PptPresByPath $ppt $failPh
if (Test-Path $failPh) { Remove-Item -Force $failPh }
$failDel = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $withPh.Replace('\', '/')
    output = $failPh.Replace('\', '/')
    ops = @(@{ op = 'deleteShape'; id = $phId })
} 'pptx-md-fail-ph'
$md4 = Get-OutPath 'pptx-md-del.pptx'
Close-PptPresByPath $ppt $md4
if (Test-Path $md4) { Remove-Item -Force $md4 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $withPh.Replace('\', '/')
    output = $md4.Replace('\', '/')
    ops = @(@{ op = 'deleteShape'; id = $decoId4 })
} 'pptx-md-del' | Out-Null
$tree4 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $md4.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-md-del-inspect'
$left = -1
$phLeft = $null
try { $left = @($tree4.masters[0].shapes).Count } catch {}
try { $phLeft = [string]$tree4.masters[0].placeholders[0].text } catch {}
$pres4 = Open-PptPres $ppt $md4
$cap4 = Get-PptCaption $pres4
Close-PptPres $pres4
$md4ok = ((Get-PptErrorCode $failDel) -eq 'InvalidId') `
    -and (-not (Test-Path $failPh)) `
    -and ($left -eq 0) `
    -and ($phLeft -eq '母版标题') `
    -and ($cap4 -notmatch 'Repair')
Add-Check 'pptx-Md4' $md4ok "fail=$(Get-PptErrorCode $failDel) left=$left ph=$phLeft cap=$cap4"

# Md5：版式装饰形；拒 pptx:presentation 与形状 id。
$md5 = Get-OutPath 'pptx-md-layout.pptx'
Close-PptPresByPath $ppt $md5
if (Test-Path $md5) { Remove-Item -Force $md5 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $md5.Replace('\', '/')
    ops = @(@{ op = 'insertShape'; id = 'master:1/layout:1'; text = '版式徽标' })
} 'pptx-md-layout' | Out-Null
$tree5 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $md5.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-md-layout-inspect'
$lyN = 0
$lyText = $null
$masterN = 0
try { $lyN = @($tree5.masters[0].layouts[0].shapes).Count } catch {}
try { $lyText = [string]$tree5.masters[0].layouts[0].shapes[0].text } catch {}
try { $masterN = @($tree5.masters[0].shapes).Count } catch {}
$pres5 = Open-PptPres $ppt $md5
$cap5 = Get-PptCaption $pres5
Close-PptPres $pres5
$failPres = Get-OutPath 'pptx-md-fail-pres.pptx'
$failSp = Get-OutPath 'pptx-md-fail-sp.pptx'
if (Test-Path $failPres) { Remove-Item -Force $failPres }
if (Test-Path $failSp) { Remove-Item -Force $failSp }
$badPres = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failPres.Replace('\', '/')
    ops = @(@{ op = 'insertShape'; id = 'pptx:presentation'; text = 'x' })
} 'pptx-md-fail-pres'
$badSp = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $md1.Replace('\', '/')
    output = $failSp.Replace('\', '/')
    ops = @(@{ op = 'insertShape'; id = $decoId; text = 'x' })
} 'pptx-md-fail-sp'
$md5ok = ($lyN -eq 1) `
    -and ($lyText -eq '版式徽标') `
    -and ($masterN -eq 0) `
    -and ($cap5 -notmatch 'Repair') `
    -and ((Get-PptErrorCode $badPres) -eq 'InvalidId') `
    -and ((Get-PptErrorCode $badSp) -eq 'InvalidId') `
    -and (-not (Test-Path $failPres)) `
    -and (-not (Test-Path $failSp))
Add-Check 'pptx-Md5' $md5ok "lyN=$lyN text=$lyText masterN=$masterN pres=$(Get-PptErrorCode $badPres) sp=$(Get-PptErrorCode $badSp) cap=$cap5"
