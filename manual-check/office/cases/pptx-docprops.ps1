# pptx 文档属性：docs/checklist-pptx.md 的 Pd1–Pd5。
# 用 create 产物再 apply，不另做 PowerPoint 另存样例。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings

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

$hostPath = Get-OutPath 'pptx-docprops-host.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(@{ shapes = @(@{ text = $S.title }) })
        props = @{ company = '测试公司' }
    }
} 'pptx-pd-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
} 'pptx-pd-inspect'
$hasProps = $tree0.PSObject.Properties.Name -contains 'props'
$pd1ok = $hasProps -and ([string]$tree0.props.creator -eq 'ooxp') -and ([string]$tree0.props.company -eq '测试公司')
Add-Check 'pptx-Pd1' $pd1ok "keys=$($tree0.PSObject.Properties.Name -join ',') creator=$($tree0.props.creator) company=$($tree0.props.company)"

$ppt = Ensure-MsPpt $hostPath
Close-PptPresByPath $ppt $hostPath

$titleOut = Get-OutPath 'pptx-docprops-title.pptx'
Close-PptPresByPath $ppt $titleOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $titleOut.Replace('\', '/')
    ops = @(@{ op = 'setProps'; id = 'pptx:presentation'; title = '新标题' })
} 'pptx-pd-set-title' | Out-Null

$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $titleOut.Replace('\', '/'); format = 'pptx'
} 'pptx-pd-title-inspect'
$comTitle = Get-ShellFileProp $titleOut @('Title', '标题')
$comCompany = Get-ShellFileProp $titleOut @('Company', '公司')
$presT = Open-PptPres $ppt $titleOut
$capT = Get-PptCaption $presT
$pd2ok = ($capT -notmatch 'Repair') -and ($comTitle -eq '新标题') -and ($comCompany -eq '测试公司') -and ([string]$treeT.props.title -eq '新标题')
Add-Check 'pptx-Pd2' $pd2ok "caption=$capT title=$comTitle company=$comCompany"
Close-PptPres $presT

$customOut = Get-OutPath 'pptx-docprops-custom.pptx'
Close-PptPresByPath $ppt $customOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $titleOut.Replace('\', '/')
    output = $customOut.Replace('\', '/')
    ops = @(@{ op = 'setCustomProp'; id = 'pptx:presentation'; name = '部门'; value = '研发' })
} 'pptx-pd-set-custom' | Out-Null

$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $customOut.Replace('\', '/'); format = 'pptx'
} 'pptx-pd-custom-inspect'
$c0 = $null
if ($treeC.props.custom) { $c0 = @($treeC.props.custom) | Select-Object -First 1 }
$comCustom = Get-CustomPropFromZip $customOut '部门'
$presC = Open-PptPres $ppt $customOut
$capC = Get-PptCaption $presC
$pd3ok = ($capC -notmatch 'Repair') -and ($null -ne $c0) -and ([string]$c0.id -eq 'prop:部门') -and ([string]$comCustom -eq '研发')
Add-Check 'pptx-Pd3' $pd3ok "caption=$capC id=$($c0.id) com=$comCustom"
Close-PptPres $presC

$delOut = Get-OutPath 'pptx-docprops-deleted.pptx'
Close-PptPresByPath $ppt $delOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $customOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteCustomProp'; id = 'prop:部门' })
} 'pptx-pd-del-custom' | Out-Null

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'pptx'
} 'pptx-pd-del-inspect'
$left = @($treeD.props.custom).Count
$gone = Get-CustomPropFromZip $delOut '部门'
$presD = Open-PptPres $ppt $delOut
$capD = Get-PptCaption $presD
$pd4ok = ($capD -notmatch 'Repair') -and ($left -eq 0) -and ($null -eq $gone)
Add-Check 'pptx-Pd4' $pd4ok "caption=$capD left=$left gone=$gone"
Close-PptPres $presD

$failOut = Get-OutPath 'pptx-docprops-bad.pptx'
if (Test-Path $failOut) { Remove-Item -Force $failOut }
$raw = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'setProps'; id = 'slide:1/shape:2'; title = 'x' })
} 'pptx-pd-bad-id'
$code = $null
try { $code = ($raw.Stderr | ConvertFrom-Json).error.code } catch {}
$noWrite = -not (Test-Path $failOut)
Add-Check 'pptx-Pd5' (($raw.ExitCode -ne 0) -and ($code -eq 'InvalidId') -and $noWrite) `
    "code=$code exit=$($raw.ExitCode) wrote=$(-not $noWrite)"
