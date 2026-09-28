# pptx 讲义：docs/checklist-pptx.md 的 Ho1–Ho5。
# create 产物验 hasHandout；setHandoutText 新建最小讲义母版并改页眉页脚。

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

function Get-PptErrorCode($raw) {
    try { return ($raw.Stderr | ConvertFrom-Json).error.code } catch { return $null }
}

$hostPath = Ensure-PptxCreate
$ppt = Ensure-MsPpt $hostPath

# Ho1：无 handout 时有 hasHandout、没有对象；create 为 false；展开后为 null。
$treeBare = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
} 'pptx-ho-inspect-bare'
$treeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    scope = @{ handout = $true }
} 'pptx-ho-inspect-scope'
$hasList = $null -ne $treeBare.PSObject.Properties['handout']
$ho1ok = ([bool]$treeBare.hasHandout -eq $false) `
    -and (-not $hasList) `
    -and ($null -eq $treeH.handout)
Add-Check 'pptx-Ho1' $ho1ok "hasHandout=$($treeBare.hasHandout) list=$hasList scoped=$($treeH.handout)"

# Ho2 / Ho4：对 create 产物写页脚，新建讲义母版，无 Repair。
$footerOut = Get-OutPath 'pptx-handout-footer.pptx'
Close-PptPresByPath $ppt $footerOut
if (Test-Path $footerOut) { Remove-Item -Force $footerOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $footerOut.Replace('\', '/')
    ops = @(@{ op = 'setHandoutText'; id = 'handout'; footer = $S.shape2new })
} 'pptx-ho-footer' | Out-Null
$treeF = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $footerOut.Replace('\', '/'); format = 'pptx'
    scope = @{ handout = $true }
} 'pptx-ho-footer-inspect'
$presF = Open-PptPres $ppt $footerOut
$capF = Get-PptCaption $presF
$comFooter = $null
try {
    $hm = $presF.HandoutMaster
    $hfs = $hm.HeadersFooters
    $ft = $hfs.Footer
    try { $comFooter = [string]$ft.Text }
    finally { Release-Com $ft }
    Release-Com $hfs
    Release-Com $hm
} catch {
    Write-Log "HandoutMaster COM 读页脚失败：$($_.Exception.Message)"
}
Close-PptPres $presF
$ho2ok = ($capF -notmatch 'Repair') `
    -and ([string]$treeF.handout.footer -eq $S.shape2new)
Add-Check 'pptx-Ho2' $ho2ok "cap=$capF footer=$($treeF.handout.footer) com=$comFooter"
$ho4ok = ($capF -notmatch 'Repair') -and ([bool]$treeF.hasHandout)
Add-Check 'pptx-Ho4' $ho4ok "cap=$capF hasHandout=$($treeF.hasHandout)"

# Ho3：再写页眉。
$headerOut = Get-OutPath 'pptx-handout-header.pptx'
Close-PptPresByPath $ppt $headerOut
if (Test-Path $headerOut) { Remove-Item -Force $headerOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $footerOut.Replace('\', '/')
    output = $headerOut.Replace('\', '/')
    ops = @(@{ op = 'setHandoutText'; id = 'handout'; header = $S.chapter1 })
} 'pptx-ho-header' | Out-Null
$treeHd = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $headerOut.Replace('\', '/'); format = 'pptx'
    scope = @{ handout = $true }
} 'pptx-ho-header-inspect'
$presH = Open-PptPres $ppt $headerOut
$capH = Get-PptCaption $presH
Close-PptPres $presH
$ho3ok = ($capH -notmatch 'Repair') `
    -and ([string]$treeHd.handout.header -eq $S.chapter1) `
    -and ([string]$treeHd.handout.footer -eq $S.shape2new)
Add-Check 'pptx-Ho3' $ho3ok "cap=$capH header=$($treeHd.handout.header) footer=$($treeHd.handout.footer)"

# Ho5：xlsx / docx 拒 scope.handout 与 setHandoutText。
$xlsx = Ensure-XlsxCreate
$docx = Ensure-DocxCreate
$failX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'inspect'
    path = $xlsx.Replace('\', '/'); format = 'xlsx'
    scope = @{ handout = $true }
} 'pptx-ho-fail-xlsx'
$failD = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'inspect'
    path = $docx.Replace('\', '/'); format = 'docx'
    scope = @{ handout = $true }
} 'pptx-ho-fail-docx'
$badX = Get-OutPath 'pptx-handout-fail-xlsx.xlsx'
$badD = Get-OutPath 'pptx-handout-fail-docx.docx'
if (Test-Path $badX) { Remove-Item -Force $badX }
if (Test-Path $badD) { Remove-Item -Force $badD }
$failXop = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $badX.Replace('\', '/')
    ops = @(@{ op = 'setHandoutText'; id = 'handout'; footer = $S.shape2new })
} 'pptx-ho-fail-xlsx-op'
$failDop = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $docx.Replace('\', '/')
    output = $badD.Replace('\', '/')
    ops = @(@{ op = 'setHandoutText'; id = 'handout'; footer = $S.shape2new })
} 'pptx-ho-fail-docx-op'
$ho5ok = ((Get-PptErrorCode $failX) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failD) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failXop) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failDop) -eq 'InvalidField') `
    -and (-not (Test-Path $badX)) `
    -and (-not (Test-Path $badD))
Add-Check 'pptx-Ho5' $ho5ok "x=$(Get-PptErrorCode $failX)/$(Get-PptErrorCode $failXop) d=$(Get-PptErrorCode $failD)/$(Get-PptErrorCode $failDop)"
