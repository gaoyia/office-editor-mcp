# docx 文档属性：docs/checklist-docx.md 的 Dp1–Dp5。
# 用 create 产物再 apply，不另做 Word 另存样例。

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

$hostPath = Get-OutPath 'docx-docprops-host.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = $S.chapter1 }
            @{ text = $S.hello }
        )
        props = @{ company = '测试公司' }
    }
} 'docx-dp-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'docx'
} 'docx-dp-inspect'
$hasProps = $tree0.PSObject.Properties.Name -contains 'props'
$dp1ok = $hasProps -and ([string]$tree0.props.creator -eq 'ooxp') -and ([string]$tree0.props.title -eq '') -and ([string]$tree0.props.company -eq '测试公司')
Add-Check 'docx-Dp1' $dp1ok "keys=$($tree0.PSObject.Properties.Name -join ',') creator=$($tree0.props.creator) title='$($tree0.props.title)' company=$($tree0.props.company)"

$titleOut = Get-OutPath 'docx-docprops-title.docx'
$word = Ensure-MsWord $hostPath
Close-WordDocByPath $word $titleOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $titleOut.Replace('\', '/')
    ops = @(@{ op = 'setProps'; id = 'docx:document'; title = '新标题' })
} 'docx-dp-set-title' | Out-Null

$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $titleOut.Replace('\', '/'); format = 'docx'
} 'docx-dp-title-inspect'
$comTitle = Get-ShellFileProp $titleOut @('Title', '标题')
$comCompany = Get-ShellFileProp $titleOut @('Company', '公司')
$docT = Open-WordDoc $word $titleOut
$capT = [string]$docT.ActiveWindow.Caption
$dp2ok = ($capT -notmatch 'Repair') -and ($comTitle -eq '新标题') -and ($comCompany -eq '测试公司') -and ([string]$treeT.props.title -eq '新标题') -and ([string]$treeT.props.company -eq '测试公司')
Add-Check 'docx-Dp2' $dp2ok "caption=$capT title=$comTitle company=$comCompany treeTitle=$($treeT.props.title)"
Close-WordDoc $docT

$customOut = Get-OutPath 'docx-docprops-custom.docx'
Close-WordDocByPath $word $customOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $titleOut.Replace('\', '/')
    output = $customOut.Replace('\', '/')
    ops = @(@{ op = 'setCustomProp'; id = 'docx:document'; name = '部门'; value = '研发' })
} 'docx-dp-set-custom' | Out-Null

$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $customOut.Replace('\', '/'); format = 'docx'
} 'docx-dp-custom-inspect'
$c0 = $null
if ($treeC.props.custom) { $c0 = @($treeC.props.custom) | Select-Object -First 1 }
$comCustom = Get-CustomPropFromZip $customOut '部门'
$docC = Open-WordDoc $word $customOut
$capC = [string]$docC.ActiveWindow.Caption
$dp3ok = ($capC -notmatch 'Repair') -and ($null -ne $c0) -and ([string]$c0.id -eq 'prop:部门') -and ([string]$comCustom -eq '研发')
Add-Check 'docx-Dp3' $dp3ok "caption=$capC id=$($c0.id) com=$comCustom"
Close-WordDoc $docC

$delOut = Get-OutPath 'docx-docprops-deleted.docx'
Close-WordDocByPath $word $delOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $customOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteCustomProp'; id = 'prop:部门' })
} 'docx-dp-del-custom' | Out-Null

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'docx'
} 'docx-dp-del-inspect'
$left = @($treeD.props.custom).Count
$gone = Get-CustomPropFromZip $delOut '部门'
$docD = Open-WordDoc $word $delOut
$capD = [string]$docD.ActiveWindow.Caption
$dp4ok = ($capD -notmatch 'Repair') -and ($left -eq 0) -and ($null -eq $gone)
Add-Check 'docx-Dp4' $dp4ok "caption=$capD left=$left gone=$gone"
Close-WordDoc $docD

$failOut = Get-OutPath 'docx-docprops-bad.docx'
if (Test-Path $failOut) { Remove-Item -Force $failOut }
$rawId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'setProps'; id = 'para:1'; title = 'x' })
} 'docx-dp-bad-id'
$codeId = $null
try { $codeId = ($rawId.Stderr | ConvertFrom-Json).error.code } catch {}
$rawEmpty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'setProps'; id = 'docx:document' })
} 'docx-dp-empty'
$codeEmpty = $null
try { $codeEmpty = ($rawEmpty.Stderr | ConvertFrom-Json).error.code } catch {}
$noWrite = -not (Test-Path $failOut)
Add-Check 'docx-Dp5' (($rawId.ExitCode -ne 0) -and ($codeId -eq 'InvalidId') -and ($rawEmpty.ExitCode -ne 0) -and ($codeEmpty -eq 'InvalidField') -and $noWrite) `
    "id=$codeId empty=$codeEmpty exitId=$($rawId.ExitCode) wrote=$(-not $noWrite)"
