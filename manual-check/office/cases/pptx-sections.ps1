# pptx 幻灯片节：docs/checklist-pptx.md 的 Psc1–Psc5。
# Psc1 用 PowerPoint 另存两节；Psc2–Psc4 用库 create 再 apply。

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

function Get-SectionSnap($pres) {
    $sp = $pres.SectionProperties
    try {
        $n = [int]$sp.Count
        $rows = @()
        for ($i = 1; $i -le $n; $i++) {
            $rows += [pscustomobject]@{
                Name  = [string]$sp.Name($i)
                First = [int]$sp.FirstSlide($i)
                Count = [int]$sp.SlidesCount($i)
            }
        }
        return @{ Count = $n; Rows = $rows }
    }
    finally { Release-Com $sp }
}

function Test-InspectMatchesCom($tree, $snap) {
    if ([int]$tree.sectionCount -ne [int]$snap.Count) { return $false }
    $secs = @($tree.sections)
    if ($secs.Count -ne [int]$snap.Count) { return $false }
    for ($i = 0; $i -lt $secs.Count; $i++) {
        $row = $snap.Rows[$i]
        $want = @()
        for ($s = 0; $s -lt [int]$row.Count; $s++) {
            $want += ("slide:{0}" -f ([int]$row.First + $s))
        }
        $got = @($secs[$i].slides)
        if ([string]$secs[$i].id -ne ("section:{0}" -f ($i + 1))) { return $false }
        if (($got -join ',') -ne ($want -join ',')) { return $false }
    }
    return $true
}

# Psc1：PowerPoint 另存两节以上，inspect 计 sectionCount，展开名单对得上页序。
$tmpDir = Join-Path $env:TEMP 'ooxp-office'
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
$tmpSrc = Join-Path $tmpDir 'pptx-sections-com.pptx'
$req = @{
    schemaVersion = 1; action = 'create'
    path = $tmpSrc.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(
            @{ shapes = @(@{ text = '页1' }) }
            @{ shapes = @(@{ text = '页2' }) }
            @{ shapes = @(@{ text = '页3' }) }
        )
    }
}
Invoke-OoxpRequest $req 'pptx-psc-com-create' | Out-Null
$ppt = Ensure-MsPpt (Ensure-PptxCreate)
Close-PptPresByPath $ppt $tmpSrc
$presC = Open-PptPres $ppt $tmpSrc
$sp = $presC.SectionProperties
try {
    [void]$sp.AddBeforeSlide(2, '内容')
}
finally { Release-Com $sp }
$presC.Save()
$snapC = Get-SectionSnap $presC
Close-PptPres $presC
$comSrc = Get-OutPath 'pptx-sections-com.pptx'
Copy-Item -Force $tmpSrc $comSrc

$treeBare = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $comSrc.Replace('\', '/'); format = 'pptx'
} 'pptx-psc-inspect-bare'
$treeSec = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $comSrc.Replace('\', '/'); format = 'pptx'
    scope = @{ sections = $true }
} 'pptx-psc-inspect-scope'
$hasList = $null -ne $treeBare.PSObject.Properties['sections']
$psc1ok = ([int]$treeBare.sectionCount -ge 2) `
    -and (-not $hasList) `
    -and (Test-InspectMatchesCom $treeSec $snapC)
Add-Check 'pptx-Psc1' $psc1ok "count=$($treeBare.sectionCount) list=$hasList com=$($snapC.Count) id0=$($treeSec.sections[0].id)"

# Psc2：两页 create 连续 insertSlideSection 两节，再把第二页划进第二节。
$hostPath = Ensure-PptxCreate
$splitOut = Get-OutPath 'pptx-sections-split.pptx'
Close-PptPresByPath $ppt $splitOut
if (Test-Path $splitOut) { Remove-Item -Force $splitOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $splitOut.Replace('\', '/')
    ops = @(
        @{ op = 'insertSlideSection'; id = 'pptx:presentation'; name = '开场' }
        @{ op = 'insertSlideSection'; id = 'pptx:presentation'; name = '内容' }
        @{ op = 'setSlideSection'; id = 'slide:2'; section = 'section:2' }
    )
} 'pptx-psc-split' | Out-Null
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $splitOut.Replace('\', '/'); format = 'pptx'
    scope = @{ sections = $true }
} 'pptx-psc-split-inspect'
$presS = Open-PptPres $ppt $splitOut
$capS = Get-PptCaption $presS
$snapS = Get-SectionSnap $presS
Close-PptPres $presS
$s2 = $snapS.Rows[1]
$psc2ok = ($capS -notmatch 'Repair') `
    -and ([int]$treeS.sectionCount -eq 2) `
    -and ([string]$treeS.sections[1].name -eq '内容') `
    -and ($s2.Name -eq '内容') `
    -and ([int]$s2.First -eq 2)
Add-Check 'pptx-Psc2' $psc2ok "cap=$capS name=$($s2.Name) first=$($s2.First) inspect=$($treeS.sections[1].name)"

# Psc3：改名。
$renOut = Get-OutPath 'pptx-sections-rename.pptx'
Close-PptPresByPath $ppt $renOut
if (Test-Path $renOut) { Remove-Item -Force $renOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $splitOut.Replace('\', '/')
    output = $renOut.Replace('\', '/')
    ops = @(@{ op = 'setSlideSectionName'; id = 'section:2'; name = '正文' })
} 'pptx-psc-rename' | Out-Null
$presR = Open-PptPres $ppt $renOut
$capR = Get-PptCaption $presR
$snapR = Get-SectionSnap $presR
Close-PptPres $presR
$psc3ok = ($capR -notmatch 'Repair') -and ($snapR.Rows[1].Name -eq '正文')
Add-Check 'pptx-Psc3' $psc3ok "cap=$capR name=$($snapR.Rows[1].Name)"

# Psc4：删节留页。
$delOut = Get-OutPath 'pptx-sections-delete.pptx'
Close-PptPresByPath $ppt $delOut
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $renOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteSlideSection'; id = 'section:2' })
} 'pptx-psc-delete' | Out-Null
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'pptx'
    scope = @{ sections = $true }
} 'pptx-psc-delete-inspect'
$presD = Open-PptPres $ppt $delOut
$capD = Get-PptCaption $presD
$snapD = Get-SectionSnap $presD
$nSlides = [int]$presD.Slides.Count
Close-PptPres $presD
$namesLeft = @($snapD.Rows | ForEach-Object { $_.Name })
$psc4ok = ($capD -notmatch 'Repair') `
    -and ($nSlides -eq 2) `
    -and ([int]$treeD.slideCount -eq 2) `
    -and ($namesLeft -notcontains '正文')
Add-Check 'pptx-Psc4' $psc4ok "cap=$capD slides=$nSlides names=$($namesLeft -join ',') count=$($snapD.Count)"

# Psc5：拒错，未写文件。
$badIdOut = Get-OutPath 'pptx-sections-bad-id.pptx'
$badBothOut = Get-OutPath 'pptx-sections-bad-both.pptx'
if (Test-Path $badIdOut) { Remove-Item -Force $badIdOut }
if (Test-Path $badBothOut) { Remove-Item -Force $badBothOut }
$failId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badIdOut.Replace('\', '/')
    ops = @(@{ op = 'insertSlideSection'; id = 'slide:1'; name = '内容' })
} 'pptx-psc-bad-id'
$failBoth = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badBothOut.Replace('\', '/')
    ops = @(@{
        op = 'insertSlideSection'; id = 'pptx:presentation'; name = '内容'
        start = 'slide:2'; after = 'section:1'
    })
} 'pptx-psc-bad-both'
$psc5ok = ((Get-PptErrorCode $failId) -eq 'InvalidId') `
    -and ((Get-PptErrorCode $failBoth) -eq 'InvalidField') `
    -and (-not (Test-Path $badIdOut)) `
    -and (-not (Test-Path $badBothOut))
Add-Check 'pptx-Psc5' $psc5ok "id=$(Get-PptErrorCode $failId) both=$(Get-PptErrorCode $failBoth)"
