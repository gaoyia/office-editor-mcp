# pptx 备注母版与讲义形状：docs/checklist-pptx.md 的 Nh1–Nh5。
# 备注母版插删占位符，讲义占位符编成 handout/shape:…，两边写同一套预设动画。不算打印讲义，不算放映引擎。

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

function Find-PhByText($hostObj, [string]$text) {
    foreach ($p in @($hostObj.placeholders)) {
        if ([string]$p.text -eq $text) { return $p }
    }
    return $null
}

function Find-FirstPhType($hostObj, [string]$typeName) {
    foreach ($p in @($hostObj.placeholders)) {
        if ([string]$p.type -eq $typeName) { return $p }
    }
    return $null
}

$hostPath = Ensure-PptxCreate
$ppt = Ensure-MsPpt $hostPath

# Nh1：对 create 产物 insertPlaceholder notesMaster body，inspect 能看见，无 Repair。
$nh1 = Get-OutPath 'pptx-nh-ins.pptx'
Close-PptPresByPath $ppt $nh1
if (Test-Path $nh1) { Remove-Item -Force $nh1 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $nh1.Replace('\', '/')
    ops = @(@{ op = 'insertPlaceholder'; id = 'notesMaster'; type = 'body'; text = '母版备注' })
} 'pptx-nh-ins' | Out-Null
$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $nh1.Replace('\', '/'); format = 'pptx'
    scope = @{ notesMaster = $true }
} 'pptx-nh-ins-inspect'
$ph1 = Find-PhByText $tree1.notesMaster '母版备注'
$pres1 = Open-PptPres $ppt $nh1
$cap1 = Get-PptCaption $pres1
Close-PptPres $pres1
$nh1ok = ($cap1 -notmatch 'Repair') `
    -and ([string]$tree1.notesMaster.id -eq 'notesMaster') `
    -and ($null -ne $ph1) `
    -and ([string]$ph1.id).StartsWith('notesMaster/shape:')
Add-Check 'pptx-Nh1' $nh1ok "cap=$cap1 id=$($tree1.notesMaster.id) ph=$($ph1.id)"

# Nh2：改字后 deletePlaceholder。
$nh2set = Get-OutPath 'pptx-nh-set.pptx'
Close-PptPresByPath $ppt $nh2set
if (Test-Path $nh2set) { Remove-Item -Force $nh2set }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $nh1.Replace('\', '/')
    output = $nh2set.Replace('\', '/')
    ops = @(@{ op = 'setShapeText'; id = [string]$ph1.id; text = '改过' })
} 'pptx-nh-set' | Out-Null
$tree2s = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $nh2set.Replace('\', '/'); format = 'pptx'
    scope = @{ notesMaster = $true }
} 'pptx-nh-set-inspect'
$ph2 = Find-PhByText $tree2s.notesMaster '改过'
$nh2del = Get-OutPath 'pptx-nh-del.pptx'
Close-PptPresByPath $ppt $nh2del
if (Test-Path $nh2del) { Remove-Item -Force $nh2del }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $nh2set.Replace('\', '/')
    output = $nh2del.Replace('\', '/')
    ops = @(@{ op = 'deletePlaceholder'; id = [string]$ph1.id })
} 'pptx-nh-del' | Out-Null
$tree2d = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $nh2del.Replace('\', '/'); format = 'pptx'
    scope = @{ notesMaster = $true }
} 'pptx-nh-del-inspect'
$still = Find-PhByText $tree2d.notesMaster '改过'
$pres2s = Open-PptPres $ppt $nh2set
$cap2s = Get-PptCaption $pres2s
Close-PptPres $pres2s
$pres2d = Open-PptPres $ppt $nh2del
$cap2d = Get-PptCaption $pres2d
Close-PptPres $pres2d
$nh2ok = ($cap2s -notmatch 'Repair') `
    -and ($cap2d -notmatch 'Repair') `
    -and ($null -ne $ph2) `
    -and ($null -eq $still)
Add-Check 'pptx-Nh2' $nh2ok "capSet=$cap2s capDel=$cap2d set=$($ph2.id) gone=$($null -eq $still)"

# Nh3：备注母版占位符 insertAnim fade。
$nh3 = Get-OutPath 'pptx-nh-anim.pptx'
Close-PptPresByPath $ppt $nh3
if (Test-Path $nh3) { Remove-Item -Force $nh3 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $nh1.Replace('\', '/')
    output = $nh3.Replace('\', '/')
    ops = @(@{ op = 'insertAnim'; id = [string]$ph1.id; type = 'fade' })
} 'pptx-nh-anim' | Out-Null
$tree3 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $nh3.Replace('\', '/'); format = 'pptx'
    scope = @{ notesMaster = $true; timing = $true }
} 'pptx-nh-anim-inspect'
$timing3 = @()
try { $timing3 = @($tree3.notesMaster.timing) } catch {}
$pres3 = Open-PptPres $ppt $nh3
$cap3 = Get-PptCaption $pres3
Close-PptPres $pres3
$nh3ok = ($cap3 -notmatch 'Repair') `
    -and ($timing3.Count -ge 1) `
    -and ([string]$timing3[0].target -eq [string]$ph1.id)
Add-Check 'pptx-Nh3' $nh3ok "cap=$cap3 target=$($timing3[0].target) expect=$($ph1.id)"

# Nh4：讲义 insertPlaceholder hdr，setShapeText / insertAnim / setHandoutText。
$nh4 = Get-OutPath 'pptx-nh-handout.pptx'
Close-PptPresByPath $ppt $nh4
if (Test-Path $nh4) { Remove-Item -Force $nh4 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $nh4.Replace('\', '/')
    ops = @(@{ op = 'insertPlaceholder'; id = 'handout'; type = 'hdr'; text = '讲义形' })
} 'pptx-nh-handout' | Out-Null
$tree4a = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $nh4.Replace('\', '/'); format = 'pptx'
    scope = @{ handout = $true }
} 'pptx-nh-handout-inspect'
$ph4new = Find-PhByText $tree4a.handout '讲义形'
$firstHdr = Find-FirstPhType $tree4a.handout 'hdr'
$nh4b = Get-OutPath 'pptx-nh-handout-anim.pptx'
Close-PptPresByPath $ppt $nh4b
if (Test-Path $nh4b) { Remove-Item -Force $nh4b }
$ops4 = @(
    @{ op = 'insertAnim'; id = [string]$ph4new.id; type = 'fade' }
    @{ op = 'setHandoutText'; id = 'handout'; footer = '槽位页脚' }
)
if ($null -ne $firstHdr) {
    $ops4 = @(
        @{ op = 'setShapeText'; id = [string]$firstHdr.id; text = '讲义形' }
    ) + $ops4
}
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $nh4.Replace('\', '/')
    output = $nh4b.Replace('\', '/')
    ops = $ops4
} 'pptx-nh-handout-anim' | Out-Null
$tree4 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $nh4b.Replace('\', '/'); format = 'pptx'
    scope = @{ handout = $true; timing = $true }
} 'pptx-nh-handout-anim-inspect'
$timing4 = @()
try { $timing4 = @($tree4.handout.timing) } catch {}
$pres4a = Open-PptPres $ppt $nh4
$cap4a = Get-PptCaption $pres4a
Close-PptPres $pres4a
$pres4b = Open-PptPres $ppt $nh4b
$cap4b = Get-PptCaption $pres4b
Close-PptPres $pres4b
$nh4ok = ($cap4a -notmatch 'Repair') `
    -and ($cap4b -notmatch 'Repair') `
    -and ($null -ne $ph4new) `
    -and ([string]$ph4new.id).StartsWith('handout/shape:') `
    -and ($timing4.Count -ge 1) `
    -and ([string]$timing4[0].target -eq [string]$ph4new.id) `
    -and ([string]$tree4.handout.footer -eq '槽位页脚') `
    -and ([string]$tree4.handout.header -eq '讲义形')
Add-Check 'pptx-Nh4' $nh4ok "cap=$cap4a/$cap4b ph=$($ph4new.id) target=$($timing4[0].target) header=$($tree4.handout.header)"

# Nh5：拒错，未写文件。
$xlsx = Ensure-XlsxCreate
$failX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'inspect'
    path = $xlsx.Replace('\', '/'); format = 'xlsx'
    scope = @{ notesMaster = $true }
} 'pptx-nh-fail-xlsx'
$badShape = Get-OutPath 'pptx-nh-fail-shape.pptx'
$badDel = Get-OutPath 'pptx-nh-fail-del.pptx'
$badPh = Get-OutPath 'pptx-nh-fail-ph.pptx'
if (Test-Path $badShape) { Remove-Item -Force $badShape }
if (Test-Path $badDel) { Remove-Item -Force $badDel }
if (Test-Path $badPh) { Remove-Item -Force $badPh }
$failIns = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badShape.Replace('\', '/')
    ops = @(@{ op = 'insertShape'; id = 'handout'; text = '甲' })
} 'pptx-nh-fail-insert-shape'
$failDel = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $nh4.Replace('\', '/')
    output = $badDel.Replace('\', '/')
    ops = @(@{ op = 'deleteShape'; id = [string]$ph4new.id })
} 'pptx-nh-fail-delete-shape'
$failPh = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badPh.Replace('\', '/')
    ops = @(@{ op = 'insertPlaceholder'; id = 'pptx:presentation'; type = 'body' })
} 'pptx-nh-fail-insert-ph'
$nh5ok = ((Get-PptErrorCode $failX) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failIns) -eq 'InvalidId') `
    -and ((Get-PptErrorCode $failDel) -eq 'InvalidId') `
    -and ((Get-PptErrorCode $failPh) -eq 'InvalidId') `
    -and (-not (Test-Path $badShape)) `
    -and (-not (Test-Path $badDel)) `
    -and (-not (Test-Path $badPh))
Add-Check 'pptx-Nh5' $nh5ok "xlsx=$(Get-PptErrorCode $failX) ins=$(Get-PptErrorCode $failIns) del=$(Get-PptErrorCode $failDel) ph=$(Get-PptErrorCode $failPh)"
