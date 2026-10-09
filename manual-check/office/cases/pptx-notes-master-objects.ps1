# pptx 备注与母版对象：docs/checklist-pptx.md 的 Nm1–Nm5。
# 备注页扩视频 / 超链接，幻灯片母版扩图与表。复用已有 op。不算放映引擎。

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

function Find-NotesBodyId($tree) {
    foreach ($s in @($tree.slides[0].notes.shapes)) {
        if ([string]$s.text -eq '备注正文') { return [string]$s.id }
    }
    return $null
}

$hostPath = Ensure-PptxCreate
$mp4 = Ensure-OoxpMp4
$png = Ensure-OoxpPngA
$ppt = Ensure-MsPpt $hostPath

# Nm1：备注插视频，根上 videoCount 仍为 0，无 Repair。
$nm1 = Get-OutPath 'pptx-nm-video.pptx'
Close-PptPresByPath $ppt $nm1
if (Test-Path $nm1) { Remove-Item -Force $nm1 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $nm1.Replace('\', '/')
    ops = @(@{ op = 'insertVideo'; id = 'slide:1/notes'; file = $mp4.Replace('\', '/') })
} 'pptx-nm-video' | Out-Null
$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $nm1.Replace('\', '/'); format = 'pptx'
    scope = @{ notes = $true; media = $true }
} 'pptx-nm-video-inspect'
$vid = [string]$tree1.slides[0].notes.videos[0].id
$pres1 = Open-PptPres $ppt $nm1
$cap1 = Get-PptCaption $pres1
Close-PptPres $pres1
$nm1ok = ($cap1 -notmatch 'Repair') `
    -and ([int]$tree1.videoCount -eq 0) `
    -and ([int]$tree1.slides[0].notes.videoCount -eq 1) `
    -and $vid.StartsWith('slide:1/notes/video:')
Add-Check 'pptx-Nm1' $nm1ok "cap=$cap1 root=$($tree1.videoCount) notes=$($tree1.slides[0].notes.videoCount) id=$vid"

# Nm2：备注插超链接。
$noted = Get-OutPath 'pptx-nm-notes.pptx'
Close-PptPresByPath $ppt $noted
if (Test-Path $noted) { Remove-Item -Force $noted }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $noted.Replace('\', '/')
    ops = @(@{ op = 'setNotesText'; id = 'slide:1'; text = '备注正文' })
} 'pptx-nm-notes' | Out-Null
$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $noted.Replace('\', '/'); format = 'pptx'
    scope = @{ notes = $true }
} 'pptx-nm-notes-inspect'
$body = Find-NotesBodyId $treeN
$nm2 = Get-OutPath 'pptx-nm-link.pptx'
Close-PptPresByPath $ppt $nm2
if (Test-Path $nm2) { Remove-Item -Force $nm2 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $noted.Replace('\', '/')
    output = $nm2.Replace('\', '/')
    ops = @(@{ op = 'insertHyperlink'; id = $body; url = 'https://example.com/' })
} 'pptx-nm-link' | Out-Null
$tree2 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $nm2.Replace('\', '/'); format = 'pptx'
    scope = @{ notes = $true; hyperlinks = $true }
} 'pptx-nm-link-inspect'
$lid = [string]$tree2.slides[0].notes.hyperlinks[0].id
$pres2 = Open-PptPres $ppt $nm2
$cap2 = Get-PptCaption $pres2
Close-PptPres $pres2
$nm2ok = ($cap2 -notmatch 'Repair') `
    -and ([int]$tree2.slides[0].notes.hyperlinkCount -eq 1) `
    -and ($lid -eq 'slide:1/notes/link:1')
Add-Check 'pptx-Nm2' $nm2ok "cap=$cap2 count=$($tree2.slides[0].notes.hyperlinkCount) id=$lid body=$body"

# Nm3：母版插图，根上 pictureCount 仍为 0，无 Repair。
$nm3 = Get-OutPath 'pptx-nm-pic.pptx'
Close-PptPresByPath $ppt $nm3
if (Test-Path $nm3) { Remove-Item -Force $nm3 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $nm3.Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = 'master:1'; file = $png.Replace('\', '/') })
} 'pptx-nm-pic' | Out-Null
$tree3 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $nm3.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true; pictures = $true }
} 'pptx-nm-pic-inspect'
$picId = $null
foreach ($p in @($tree3.masters[0].pictures)) {
    if ([string]$p.id -like 'master:1/pic:*') { $picId = [string]$p.id; break }
}
$pres3 = Open-PptPres $ppt $nm3
$cap3 = Get-PptCaption $pres3
Close-PptPres $pres3
$nm3ok = ($cap3 -notmatch 'Repair') `
    -and ([int]$tree3.pictureCount -eq 0) `
    -and ($null -ne $picId)
Add-Check 'pptx-Nm3' $nm3ok "cap=$cap3 root=$($tree3.pictureCount) id=$picId"

# Nm4：母版插表。
$nm4 = Get-OutPath 'pptx-nm-tbl.pptx'
Close-PptPresByPath $ppt $nm4
if (Test-Path $nm4) { Remove-Item -Force $nm4 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $nm4.Replace('\', '/')
    ops = @(@{ op = 'insertTable'; id = 'master:1'; rows = 1; cols = 1 })
} 'pptx-nm-tbl' | Out-Null
$tree4 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $nm4.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true; tables = $true }
} 'pptx-nm-tbl-inspect'
$tid = $null
$nTbl = 0
if ($tree4.masters[0].PSObject.Properties.Name -contains 'tables') {
    $nTbl = @($tree4.masters[0].tables).Count
    $tid = [string]$tree4.masters[0].tables[0].id
}
$pres4 = Open-PptPres $ppt $nm4
$cap4 = Get-PptCaption $pres4
Close-PptPres $pres4
$nm4ok = ($cap4 -notmatch 'Repair') `
    -and ($nTbl -eq 1) `
    -and $tid.StartsWith('master:1/table:')
Add-Check 'pptx-Nm4' $nm4ok "cap=$cap4 n=$nTbl id=$tid"

# Nm5：notesMaster 插图、批注内插视频均 InvalidId，未写文件。
$badPic = Get-OutPath 'pptx-nm-bad-pic.pptx'
Close-PptPresByPath $ppt $badPic
if (Test-Path $badPic) { Remove-Item -Force $badPic }
$fail1 = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badPic.Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = 'notesMaster'; file = $png.Replace('\', '/') })
} 'pptx-nm-bad-pic'
$badVid = Get-OutPath 'pptx-nm-bad-vid.pptx'
Close-PptPresByPath $ppt $badVid
if (Test-Path $badVid) { Remove-Item -Force $badVid }
$fail2 = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badVid.Replace('\', '/')
    ops = @(@{
        op = 'insertVideo'
        id = 'slide:1/comment:00000000-0000-0000-0000-000000000001'
        file = $mp4.Replace('\', '/')
    })
} 'pptx-nm-bad-vid'
$code1 = Get-PptErrorCode $fail1
$code2 = Get-PptErrorCode $fail2
$nm5ok = ($fail1.ExitCode -ne 0) -and ($code1 -eq 'InvalidId') -and (-not (Test-Path $badPic)) `
    -and ($fail2.ExitCode -ne 0) -and ($code2 -eq 'InvalidId') -and (-not (Test-Path $badVid))
Add-Check 'pptx-Nm5' $nm5ok "pic=$code1 existsPic=$(Test-Path $badPic) vid=$code2 existsVid=$(Test-Path $badVid)"
