# pptx 动画写入：docs/checklist-pptx.md 的 Aw1–Aw10。
# Aw2 的源文件：PowerPoint 亲手另存，第一页进入淡入。不要只用库 create 产物当带动画的唯一样例。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
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

function Get-PptMainSeqCount($slide) {
    $tl = $slide.TimeLine
    $seq = $tl.MainSequence
    try { return [int]$seq.Count }
    finally {
        Release-Com $seq
        Release-Com $tl
    }
}

function Add-PptFadeIn($slide) {
    $box = Get-PptShape $slide 1
    $tl = $slide.TimeLine
    $seq = $tl.MainSequence
    try {
        $eff = $seq.AddEffect($box, 10, 0, 1)
        Release-Com $eff
        return [int]$seq.Count
    }
    finally {
        Release-Com $seq
        Release-Com $tl
        Release-Com $box
    }
}

function Test-PptxAnimSource([string]$pptx, [string]$reqName) {
    $tree = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $pptx.Replace('\', '/'); format = 'pptx'
        scope = @{ timing = $true; slide = 1 }
    } $reqName
    $n = [int]$tree.animCount
    $list = @()
    try { $list = @($tree.slides[0].timing) } catch {}
    $target = $null
    if ($list.Count -ge 1) {
        try { $target = [string]$list[0].target } catch {}
    }
    $shapeId = $null
    try { $shapeId = [string]$tree.slides[0].shapes[0].id } catch {}
    return [pscustomobject]@{
        animCount = $n
        target = $target
        shapeId = $shapeId
        ok = ($n -ge 1) -and ($target -eq $shapeId)
    }
}

function Ensure-PptxPptAnimSource {
    $path = Get-OutPath 'pptx-ppt-anim-source.pptx'
    if (Test-Path $path) {
        $cached = Test-PptxAnimSource $path 'pptx-aw-source-cache-check'
        if ($cached.ok) { return $path }
        Write-Log "缓存的 pptx anim source 不合格（count=$($cached.animCount) target=$($cached.target)），重建"
    }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $tmp = Join-Path $tmpDir 'pptx-ppt-anim-write-source.pptx'
    $hostPath = Ensure-PptxCreate
    Copy-Item -Force $hostPath $tmp

    $ppt = Ensure-MsPpt $hostPath
    Close-PptPresByPath $ppt $tmp
    $pres = Open-PptPres $ppt $tmp
    $s1 = Get-PptSlide $pres 1
    $n = Add-PptFadeIn $s1
    Write-Log "PowerPoint Save() 进入动画源，MainSequence=$n"
    $pres.Save()
    Release-Com $s1
    Close-PptPres $pres
    Copy-Item -Force $tmp $path

    $check = Test-PptxAnimSource $path 'pptx-aw-source-saved-inspect'
    if (-not $check.ok) {
        throw "PowerPoint 另存的进入动画源不合格：count=$($check.animCount) target=$($check.target) shape=$($check.shapeId)"
    }
    return $path
}

$hostPath = Ensure-PptxCreate
$src = Ensure-PptxPptAnimSource
$ppt = Ensure-MsPpt $hostPath
Close-PptPresByPath $ppt $hostPath
Close-PptPresByPath $ppt $src

$treeHost = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-aw-host-shapes'
$shape1 = [string]$treeHost.slides[0].shapes[0].id
$shape2 = [string]$treeHost.slides[0].shapes[1].id

# Aw1：create 后 insertAnim fade，名单一条，COM Count=1，无 Repair。
$aw1 = Get-OutPath 'pptx-anim-write-insert.pptx'
Close-PptPresByPath $ppt $aw1
if (Test-Path $aw1) { Remove-Item -Force $aw1 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $aw1.Replace('\', '/')
    ops = @(@{ op = 'insertAnim'; id = $shape1; type = 'fade' })
} 'pptx-aw-insert' | Out-Null
$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $aw1.Replace('\', '/'); format = 'pptx'
    scope = @{ timing = $true; slide = 1 }
} 'pptx-aw-insert-inspect'
$list1 = @($tree1.slides[0].timing)
$pres1 = Open-PptPres $ppt $aw1
$cap1 = Get-PptCaption $pres1
$s1 = Get-PptSlide $pres1 1
$com1 = Get-PptMainSeqCount $s1
Release-Com $s1
Close-PptPres $pres1
$aw1ok = ($cap1 -notmatch 'Repair') `
    -and ($list1.Count -eq 1) `
    -and ([string]$list1[0].id -eq 'slide:1/anim:1') `
    -and ([string]$list1[0].target -eq $shape1) `
    -and ([string]$list1[0].type -eq 'fade') `
    -and ($com1 -eq 1)
Add-Check 'pptx-Aw1' $aw1ok "cap=$cap1 count=$($list1.Count) target=$($list1[0].target) com=$com1"

# Aw2：另存已有动画的页再插一条，旧的还在，新的在末尾。
$aw2 = Get-OutPath 'pptx-anim-write-append.pptx'
Close-PptPresByPath $ppt $aw2
if (Test-Path $aw2) { Remove-Item -Force $aw2 }
$treeSrc = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ timing = $true; slide = 1 }
} 'pptx-aw-src-inspect'
$oldTarget = [string]$treeSrc.slides[0].timing[0].target
$oldCount = [int]$treeSrc.animCount
$srcShape2 = [string]$treeSrc.slides[0].shapes[1].id
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $aw2.Replace('\', '/')
    ops = @(@{ op = 'insertAnim'; id = $srcShape2; type = 'fade' })
} 'pptx-aw-append' | Out-Null
$tree2 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $aw2.Replace('\', '/'); format = 'pptx'
    scope = @{ timing = $true; slide = 1 }
} 'pptx-aw-append-inspect'
$list2 = @($tree2.slides[0].timing)
$pres2 = Open-PptPres $ppt $aw2
$cap2 = Get-PptCaption $pres2
$s2 = Get-PptSlide $pres2 1
$com2 = Get-PptMainSeqCount $s2
Release-Com $s2
Close-PptPres $pres2
$last = $list2[$list2.Count - 1]
$aw2ok = ($cap2 -notmatch 'Repair') `
    -and ($list2.Count -eq ($oldCount + 1)) `
    -and ([string]$list2[0].target -eq $oldTarget) `
    -and ([string]$last.target -eq $srcShape2) `
    -and ($com2 -eq $list2.Count)
Add-Check 'pptx-Aw2' $aw2ok "cap=$cap2 old=$oldCount new=$($list2.Count) last=$($last.target) com=$com2"

# Aw3：deleteAnim 后 Count 少 1；删光没有 p:timing。
$aw3a = Get-OutPath 'pptx-anim-write-delete-one.pptx'
$aw3b = Get-OutPath 'pptx-anim-write-delete-all.pptx'
Close-PptPresByPath $ppt $aw3a
Close-PptPresByPath $ppt $aw3b
if (Test-Path $aw3a) { Remove-Item -Force $aw3a }
if (Test-Path $aw3b) { Remove-Item -Force $aw3b }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $aw2.Replace('\', '/')
    output = $aw3a.Replace('\', '/')
    ops = @(@{ op = 'deleteAnim'; id = 'slide:1/anim:1' })
} 'pptx-aw-delete-one' | Out-Null
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $aw1.Replace('\', '/')
    output = $aw3b.Replace('\', '/')
    ops = @(@{ op = 'deleteAnim'; id = 'slide:1/anim:1' })
} 'pptx-aw-delete-all' | Out-Null
$tree3a = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $aw3a.Replace('\', '/'); format = 'pptx'
    scope = @{ timing = $true }
} 'pptx-aw-delete-one-inspect'
$tree3b = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $aw3b.Replace('\', '/'); format = 'pptx'
    scope = @{ timing = $true }
} 'pptx-aw-delete-all-inspect'
$xml3 = Get-ZipEntryText $aw3b 'ppt/slides/slide1.xml'
$pres3a = Open-PptPres $ppt $aw3a
$cap3a = Get-PptCaption $pres3a
$s3a = Get-PptSlide $pres3a 1
$com3a = Get-PptMainSeqCount $s3a
Release-Com $s3a
Close-PptPres $pres3a
$pres3b = Open-PptPres $ppt $aw3b
$cap3b = Get-PptCaption $pres3b
$s3b = Get-PptSlide $pres3b 1
$com3b = Get-PptMainSeqCount $s3b
Release-Com $s3b
Close-PptPres $pres3b
$aw3ok = ($cap3a -notmatch 'Repair') -and ($cap3b -notmatch 'Repair') `
    -and ([int]$tree3a.animCount -eq $oldCount) `
    -and ($com3a -eq [int]$tree3a.animCount) `
    -and ([int]$tree3b.animCount -eq 0) `
    -and ($com3b -eq 0) `
    -and ($xml3 -notmatch '<p:timing')
Add-Check 'pptx-Aw3' $aw3ok "cap=$cap3a/$cap3b one=$($tree3a.animCount) all=$($tree3b.animCount) hasTiming=$($xml3 -match '<p:timing')"

# Aw4：只改字，名单还在。
$aw4 = Get-OutPath 'pptx-anim-write-text.pptx'
Close-PptPresByPath $ppt $aw4
if (Test-Path $aw4) { Remove-Item -Force $aw4 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $aw1.Replace('\', '/')
    output = $aw4.Replace('\', '/')
    ops = @(@{ op = 'setShapeText'; id = $shape1; text = $S.shape2new })
} 'pptx-aw-text' | Out-Null
$tree4 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $aw4.Replace('\', '/'); format = 'pptx'
    scope = @{ timing = $true; slide = 1 }
} 'pptx-aw-text-inspect'
$list4 = @($tree4.slides[0].timing)
$pres4 = Open-PptPres $ppt $aw4
$cap4 = Get-PptCaption $pres4
Close-PptPres $pres4
$aw4ok = ($cap4 -notmatch 'Repair') `
    -and ($list4.Count -eq 1) `
    -and ([string]$list4[0].target -eq $shape1) `
    -and ([string]$tree4.slides[0].shapes[0].text -eq $S.shape2new)
Add-Check 'pptx-Aw4' $aw4ok "cap=$cap4 count=$($list4.Count) text=$($tree4.slides[0].shapes[0].text)"

# Aw5：错 id / type；xlsx / docx 拒 insertAnim；setTiming 仍 UnsupportedOp。
$badId = Get-OutPath 'pptx-anim-write-fail-id.pptx'
$badType = Get-OutPath 'pptx-anim-write-fail-type.pptx'
$badOp = Get-OutPath 'pptx-anim-write-fail-op.pptx'
$failId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/'); output = $badId.Replace('\', '/')
    ops = @(@{ op = 'insertAnim'; id = 'slide:1'; type = 'fade' })
} 'pptx-aw-fail-id'
$failType = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/'); output = $badType.Replace('\', '/')
    ops = @(@{ op = 'insertAnim'; id = $shape1; type = 'blinds' })
} 'pptx-aw-fail-type'
$failOp = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/'); output = $badOp.Replace('\', '/')
    ops = @(@{ op = 'setTiming'; id = 'slide:1' })
} 'pptx-aw-fail-op'
$xlsx = Ensure-XlsxCreate
$docx = Ensure-DocxCreate
$failX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/'); output = (Get-OutPath 'pptx-anim-write-fail.xlsx').Replace('\', '/')
    ops = @(@{ op = 'insertAnim'; id = $shape1; type = 'fade' })
} 'pptx-aw-fail-xlsx'
$failD = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $docx.Replace('\', '/'); output = (Get-OutPath 'pptx-anim-write-fail.docx').Replace('\', '/')
    ops = @(@{ op = 'insertAnim'; id = $shape1; type = 'fade' })
} 'pptx-aw-fail-docx'
$aw5ok = ((Get-PptErrorCode $failId) -eq 'InvalidId') -and (-not (Test-Path $badId)) `
    -and ((Get-PptErrorCode $failType) -eq 'InvalidField') -and (-not (Test-Path $badType)) `
    -and ((Get-PptErrorCode $failOp) -eq 'UnsupportedOp') -and (-not (Test-Path $badOp)) `
    -and ((Get-PptErrorCode $failX) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failD) -eq 'InvalidField')
Add-Check 'pptx-Aw5' $aw5ok "id=$(Get-PptErrorCode $failId) type=$(Get-PptErrorCode $failType) op=$(Get-PptErrorCode $failOp) xlsx=$(Get-PptErrorCode $failX) docx=$(Get-PptErrorCode $failD)"

# Aw6：delay / dur 写入并能 inspect 回来。
$aw6 = Get-OutPath 'pptx-anim-write-delay.pptx'
Close-PptPresByPath $ppt $aw6
if (Test-Path $aw6) { Remove-Item -Force $aw6 }
$shape2 = [string]$treeHost.slides[0].shapes[1].id
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $aw6.Replace('\', '/')
    ops = @(
        @{ op = 'insertAnim'; id = $shape1; type = 'fade' }
        @{ op = 'insertAnim'; id = $shape2; type = 'fade'; trigger = 'with'; delay = 380; dur = 800 }
    )
} 'pptx-aw-delay' | Out-Null
$tree6 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $aw6.Replace('\', '/'); format = 'pptx'
    scope = @{ timing = $true; slide = 1 }
} 'pptx-aw-delay-inspect'
$list6 = @($tree6.slides[0].timing)
$xml6 = Get-ZipEntryText $aw6 'ppt/slides/slide1.xml'
$pres6 = Open-PptPres $ppt $aw6
$cap6 = Get-PptCaption $pres6
$s6 = Get-PptSlide $pres6 1
$com6 = Get-PptMainSeqCount $s6
Release-Com $s6
Close-PptPres $pres6
$aw6ok = ($cap6 -notmatch 'Repair') `
    -and ($list6.Count -eq 2) `
    -and ([int]$list6[1].delay -eq 380) `
    -and ([int]$list6[1].dur -eq 800) `
    -and ($xml6 -match 'delay="380"') `
    -and ($xml6 -match 'dur="800"') `
    -and ($com6 -eq 2)
Add-Check 'pptx-Aw6' $aw6ok "cap=$cap6 delay=$($list6[1].delay) dur=$($list6[1].dur) com=$com6"

# Aw7：setAnim 改已有第二条的 trigger / delay / dur。
$aw7 = Get-OutPath 'pptx-anim-write-set.pptx'
Close-PptPresByPath $ppt $aw7
if (Test-Path $aw7) { Remove-Item -Force $aw7 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $aw6.Replace('\', '/')
    output = $aw7.Replace('\', '/')
    ops = @(@{ op = 'setAnim'; id = 'slide:1/anim:2'; trigger = 'after'; delay = 200; dur = 700 })
} 'pptx-aw-set' | Out-Null
$tree7 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $aw7.Replace('\', '/'); format = 'pptx'
    scope = @{ timing = $true; slide = 1 }
} 'pptx-aw-set-inspect'
$list7 = @($tree7.slides[0].timing)
$pres7 = Open-PptPres $ppt $aw7
$cap7 = Get-PptCaption $pres7
$s7 = Get-PptSlide $pres7 1
$com7 = Get-PptMainSeqCount $s7
Release-Com $s7
Close-PptPres $pres7
$aw7ok = ($cap7 -notmatch 'Repair') `
    -and ($list7.Count -eq 2) `
    -and ([string]$list7[1].trigger -eq 'after') `
    -and ([int]$list7[1].delay -eq 200) `
    -and ([int]$list7[1].dur -eq 700) `
    -and ($com7 -eq 2)
Add-Check 'pptx-Aw7' $aw7ok "cap=$cap7 trigger=$($list7[1].trigger) delay=$($list7[1].delay) dur=$($list7[1].dur) com=$com7"

# Aw8：第一条 with 进片自动播：外层有 onBegin，仍有 delay=indefinite。
$aw8 = Get-OutPath 'pptx-anim-write-onbegin.pptx'
Close-PptPresByPath $ppt $aw8
if (Test-Path $aw8) { Remove-Item -Force $aw8 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $aw8.Replace('\', '/')
    ops = @(@{ op = 'insertAnim'; id = $shape1; type = 'fade'; trigger = 'with' })
} 'pptx-aw-onbegin' | Out-Null
$tree8 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $aw8.Replace('\', '/'); format = 'pptx'
    scope = @{ timing = $true; slide = 1 }
} 'pptx-aw-onbegin-inspect'
$list8 = @($tree8.slides[0].timing)
$xml8 = Get-ZipEntryText $aw8 'ppt/slides/slide1.xml'
$pres8 = Open-PptPres $ppt $aw8
$cap8 = Get-PptCaption $pres8
$s8 = Get-PptSlide $pres8 1
$com8 = Get-PptMainSeqCount $s8
Release-Com $s8
Close-PptPres $pres8
$aw8ok = ($cap8 -notmatch 'Repair') `
    -and ($list8.Count -eq 1) `
    -and ([string]$list8[0].trigger -eq 'with') `
    -and ($xml8 -match 'evt="onBegin"') `
    -and ($xml8 -match 'delay="indefinite"') `
    -and ($com8 -eq 1)
Add-Check 'pptx-Aw8' $aw8ok "cap=$cap8 trigger=$($list8[0].trigger) com=$com8"

# Aw9：备注页动画。不要用 NotesPage COM Count。
$aw9 = Get-OutPath 'pptx-anim-write-notes.pptx'
Close-PptPresByPath $ppt $aw9
if (Test-Path $aw9) { Remove-Item -Force $aw9 }
$noted = Get-OutPath 'pptx-anim-write-notes-src.pptx'
Close-PptPresByPath $ppt $noted
if (Test-Path $noted) { Remove-Item -Force $noted }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $noted.Replace('\', '/')
    ops = @(@{ op = 'setNotesText'; id = 'slide:1'; text = '演讲者备注' })
} 'pptx-aw-notes-src' | Out-Null
$treeNotes = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $noted.Replace('\', '/'); format = 'pptx'
    scope = @{ notes = $true; slide = 1 }
} 'pptx-aw-notes-shapes'
$notesBody = $null
try {
    foreach ($sp in @($treeNotes.slides[0].notes.shapes)) {
        if ([string]$sp.text -eq '演讲者备注') { $notesBody = [string]$sp.id; break }
    }
} catch {}
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $noted.Replace('\', '/')
    output = $aw9.Replace('\', '/')
    ops = @(@{ op = 'insertAnim'; id = $notesBody; type = 'fade' })
} 'pptx-aw-notes' | Out-Null
$tree9 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $aw9.Replace('\', '/'); format = 'pptx'
    scope = @{ notes = $true; timing = $true }
} 'pptx-aw-notes-inspect'
$notesList = @()
try { $notesList = @($tree9.slides[0].notes.timing) } catch {}
$xml9 = Get-ZipEntryText $aw9 'ppt/notesSlides/notesSlide1.xml'
$pres9 = Open-PptPres $ppt $aw9
$cap9 = Get-PptCaption $pres9
Close-PptPres $pres9
$aw9ok = ($cap9 -notmatch 'Repair') `
    -and ([int]$tree9.animCount -eq 0) `
    -and ($notesList.Count -eq 1) `
    -and ([string]$notesList[0].id -eq 'slide:1/notes/anim:1') `
    -and ($xml9 -match '<p:timing')
Add-Check 'pptx-Aw9' $aw9ok "cap=$cap9 host=$notesBody count=$($notesList.Count) root=$($tree9.animCount) hasTiming=$($xml9 -match '<p:timing')"

# Aw10：母版占位符动画。COM 读 SlideMaster.TimeLine。
$aw10 = Get-OutPath 'pptx-anim-write-master.pptx'
Close-PptPresByPath $ppt $aw10
if (Test-Path $aw10) { Remove-Item -Force $aw10 }
$masterPh = Get-OutPath 'pptx-anim-write-master-ph.pptx'
Close-PptPresByPath $ppt $masterPh
if (Test-Path $masterPh) { Remove-Item -Force $masterPh }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $masterPh.Replace('\', '/')
    ops = @(@{ op = 'insertPlaceholder'; id = 'master:1'; type = 'title'; text = '母版标题' })
} 'pptx-aw-master-ph' | Out-Null
$treePh = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $masterPh.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-aw-master-ph-inspect'
$phId = $null
try { $phId = [string]$treePh.masters[0].placeholders[0].id } catch {}
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $masterPh.Replace('\', '/')
    output = $aw10.Replace('\', '/')
    ops = @(@{ op = 'insertAnim'; id = $phId; type = 'fade' })
} 'pptx-aw-master' | Out-Null
$tree10 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $aw10.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true; timing = $true }
} 'pptx-aw-master-inspect'
$masterList = @()
try { $masterList = @($tree10.masters[0].timing) } catch {}
$xml10 = Get-ZipEntryText $aw10 'ppt/slideMasters/slideMaster1.xml'
$pres10 = Open-PptPres $ppt $aw10
$cap10 = Get-PptCaption $pres10
$master10 = $pres10.SlideMaster
$tl10 = $master10.TimeLine
$seq10 = $tl10.MainSequence
try { $com10 = [int]$seq10.Count } catch { $com10 = -1 }
Release-Com $seq10
Release-Com $tl10
Release-Com $master10
Close-PptPres $pres10
$aw10ok = ($cap10 -notmatch 'Repair') `
    -and ([int]$tree10.animCount -eq 0) `
    -and ($masterList.Count -eq 1) `
    -and ([string]$masterList[0].id -eq 'master:1/anim:1') `
    -and ($xml10 -match '<p:timing') `
    -and ($com10 -eq 1)
Add-Check 'pptx-Aw10' $aw10ok "cap=$cap10 ph=$phId count=$($masterList.Count) com=$com10 hasTiming=$($xml10 -match '<p:timing')"

