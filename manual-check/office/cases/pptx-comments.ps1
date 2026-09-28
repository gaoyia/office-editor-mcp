# pptx 批注：docs/checklist-pptx.md 的 Pcm1–Pcm6、Pik1–Pik4。
# 源文件：PowerPoint 亲手另存，第一页有两条线程化批注，其中一条带回复。不要用库 create 产物当带批注的唯一样例。
# 墨迹：在另存样例的现代批注部件里，按 commentId 给「批注乙」插入本地名为 ink 的空标记。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-PptxPptCommentSource
$ppt = Ensure-MsPpt $src
Close-PptPresByPath $ppt $src

function Find-PptCommentByText($tree, [string]$text) {
    foreach ($c in @($tree.slides[0].comments)) {
        if (([string]$c.text) -eq $text) { return $c }
    }
    return $null
}

function Get-PptCommentTexts($slide) {
    $out = @()
    $comments = $null
    try {
        $comments = $slide.Comments
        $n = [int]$comments.Count
        for ($i = 1; $i -le $n; $i++) {
            $c = $comments.Item($i)
            try {
                $out += ([string]$c.Text).Trim()
                $replies = $null
                try {
                    $replies = $c.Replies
                    $rn = [int]$replies.Count
                    for ($j = 1; $j -le $rn; $j++) {
                        $r = $replies.Item($j)
                        try { $out += ([string]$r.Text).Trim() }
                        finally { Release-Com $r }
                    }
                } finally { Release-Com $replies }
            } finally { Release-Com $c }
        }
    } catch {
        # 线程化批注的 COM 表面不稳定时，交给 inspect 判定。
    } finally {
        Release-Com $comments
    }
    return $out
}

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
} 'pptx-comments-inspect-list'
$s0 = $tree0.slides[0]
$noList = -not ($s0.PSObject.Properties.Name -contains 'comments')
$hasCount = ($tree0.PSObject.Properties.Name -contains 'commentCount') -and ([int]$tree0.commentCount -ge 2)
$slideCount = ($s0.PSObject.Properties.Name -contains 'commentCount') -and ([int]$s0.commentCount -ge 2)
Add-Check 'pptx-Pcm1a' ($hasCount -and $noList -and $slideCount) "root=$($tree0.commentCount) slide=$($s0.commentCount) noList=$noList"

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ comments = $true }
} 'pptx-comments-inspect-expand'
$cA = Find-PptCommentByText $tree1 $S.commentA
$cB = Find-PptCommentByText $tree1 $S.commentB
$idOk = $cA -and $cB -and ([string]$cA.id -like 'slide:1/comment:{*}') -and ([string]$cB.id -like 'slide:1/comment:{*}')
$replyHit = $false
if ($cA) {
    foreach ($r in @($cA.replies)) {
        if (([string]$r.text) -eq $S.commentReply) { $replyHit = $true }
    }
}
if (-not $replyHit -and $cB) {
    foreach ($r in @($cB.replies)) {
        if (([string]$r.text) -eq $S.commentReply) { $replyHit = $true }
    }
}
Add-Check 'pptx-Pcm1' ($idOk -and $replyHit) "a=$($cA.id) b=$($cB.id) reply=$replyHit"

$textOut = Get-OutPath 'pptx-comments-text.pptx'
Close-PptPresByPath $ppt $textOut
if (Test-Path $textOut) { Remove-Item $textOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $textOut.Replace('\', '/')
    ops = @(@{ op = 'setCommentText'; id = $cA.id; text = $S.commentTextNew })
} 'pptx-comments-set-text' | Out-Null

$presT = Open-PptPres $ppt $textOut
$capT = Get-PptCaption $presT
$slideT = Get-PptSlide $presT 1
$comTexts = Get-PptCommentTexts $slideT
$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $textOut.Replace('\', '/'); format = 'pptx'
    scope = @{ comments = $true }
} 'pptx-comments-text-inspect'
$sawNew = $null -ne (Find-PptCommentByText $treeT $S.commentTextNew)
$comSaw = $comTexts -contains $S.commentTextNew
Add-Check 'pptx-Pcm2' (($capT -notmatch 'Repair') -and $sawNew) "cap=$capT inspect=$sawNew com=$comSaw"
Release-Com $slideT
Close-PptPres $presT

$insOut = Get-OutPath 'pptx-comments-insert.pptx'
Close-PptPresByPath $ppt $insOut
if (Test-Path $insOut) { Remove-Item $insOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{
        op = 'insertComment'
        id = 'slide:1'
        text = $S.commentNew
        author = 'Carol'
        initials = 'C'
        x = 1828800
        y = 914400
    })
} 'pptx-comments-insert' | Out-Null

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'pptx'
    scope = @{ comments = $true }
} 'pptx-comments-insert-inspect'
$cNew = Find-PptCommentByText $treeI $S.commentNew
$presI = Open-PptPres $ppt $insOut
$capI = Get-PptCaption $presI
Add-Check 'pptx-Pcm3' (($capI -notmatch 'Repair') -and ($null -ne $cNew) -and ([int]$treeI.commentCount -ge 3)) "cap=$capI id=$($cNew.id) count=$($treeI.commentCount)"
Close-PptPres $presI

$delOut = Get-OutPath 'pptx-comments-delete.pptx'
Close-PptPresByPath $ppt $delOut
if (Test-Path $delOut) { Remove-Item $delOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteComment'; id = $cB.id })
} 'pptx-comments-delete' | Out-Null

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'pptx'
    scope = @{ comments = $true }
} 'pptx-comments-delete-inspect'
$stillB = $null -ne (Find-PptCommentByText $treeD $S.commentB)
$presD = Open-PptPres $ppt $delOut
$capD = Get-PptCaption $presD
Add-Check 'pptx-Pcm4' (($capD -notmatch 'Repair') -and (-not $stillB) -and ([int]$treeD.commentCount -eq ([int]$tree1.commentCount - 1))) "cap=$capD stillB=$stillB count=$($treeD.commentCount)"
Close-PptPres $presD

$threadOut = Get-OutPath 'pptx-comments-thread.pptx'
Close-PptPresByPath $ppt $threadOut
if (Test-Path $threadOut) { Remove-Item $threadOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $textOut.Replace('\', '/')
    output = $threadOut.Replace('\', '/')
    ops = @(
        @{ op = 'replyComment'; id = (Find-PptCommentByText $treeT $S.commentTextNew).id; text = '手测回复' }
        @{ op = 'setCommentDone'; id = (Find-PptCommentByText $treeT $S.commentTextNew).id; done = $true }
    )
} 'pptx-comments-thread' | Out-Null

$treeTh = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $threadOut.Replace('\', '/'); format = 'pptx'
    scope = @{ comments = $true }
} 'pptx-comments-thread-inspect'
$cTh = Find-PptCommentByText $treeTh $S.commentTextNew
$replyOk = $false
if ($cTh) {
    foreach ($r in @($cTh.replies)) {
        if (([string]$r.text) -eq '手测回复') { $replyOk = $true }
    }
}
$doneOk = $cTh -and ([bool]$cTh.done -eq $true)
$presTh = Open-PptPres $ppt $threadOut
$capTh = Get-PptCaption $presTh
Add-Check 'pptx-Pcm5' (($capTh -notmatch 'Repair') -and $replyOk -and $doneOk) "cap=$capTh reply=$replyOk done=$($cTh.done)"
Close-PptPres $presTh

function Add-ZipBytes([string]$zipPath, [string]$part, [byte[]]$bytes) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::Open($zipPath, 'Update')
    try {
        $n = $part -replace '\\', '/'
        $old = $zip.Entries | Where-Object { ($_.FullName -replace '\\', '/') -eq $n } | Select-Object -First 1
        if ($old) { $old.Delete() }
        $entry = $zip.CreateEntry($n)
        $ws = $entry.Open()
        $ws.Write($bytes, 0, $bytes.Length)
        $ws.Dispose()
    }
    finally { $zip.Dispose() }
}

function Get-ZipBytes([string]$zipPath, [string]$part) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        $n = $part -replace '\\', '/'
        $entry = $zip.Entries | Where-Object { ($_.FullName -replace '\\', '/') -eq $n } | Select-Object -First 1
        if (-not $entry) { return $null }
        $ms = New-Object IO.MemoryStream
        $es = $entry.Open()
        $es.CopyTo($ms)
        $es.Dispose()
        return $ms.ToArray()
    }
    finally { $zip.Dispose() }
}

function Inject-PptxCommentBlip([string]$pptx, [string]$guid, [string]$pngPath) {
    $script:BlipGuid = $guid
    $part = Find-ZipPartContaining $pptx 'ppt/comments/*.xml' $guid
    if (-not $part) { throw "批注部件里没有 $guid" }
    Rewrite-ZipPart $pptx $part {
        param($xml)
        $tIdx = $xml.IndexOf($script:BlipGuid)
        if ($tIdx -lt 0) { throw "部件里没有 $($script:BlipGuid)" }
        $head = $xml.Substring(0, [Math]::Min($tIdx + 20, $xml.Length))
        $cmStart = $head.LastIndexOf('<p188:cm')
        if ($cmStart -lt 0) { $cmStart = $head.LastIndexOf('<p:cm') }
        if ($cmStart -lt 0) { throw '找不到批注元素起点' }
        $cmEnd = $xml.IndexOf('</p188:cm>', $tIdx)
        $endLen = '</p188:cm>'.Length
        if ($cmEnd -lt 0) {
            $cmEnd = $xml.IndexOf('</p:cm>', $tIdx)
            $endLen = '</p:cm>'.Length
        }
        if ($cmEnd -lt 0) { throw '找不到批注元素终点' }
        $cmEnd += $endLen
        $block = $xml.Substring($cmStart, $cmEnd - $cmStart)
        if ($block -notmatch 'a:blip[^>]*r:embed') {
            $close = '</p188:txBody>'
            $p = $block.LastIndexOf($close)
            if ($p -lt 0) {
                $close = '</a:txBody>'
                $p = $block.LastIndexOf($close)
            }
            if ($p -lt 0) { throw '批注里没有 txBody 结束标签' }
            $block = $block.Insert($p + $close.Length, '<a:blip r:embed="rIdOoxpPic"/>')
        }
        return $xml.Substring(0, $cmStart) + $block + $xml.Substring($cmEnd)
    }
    $dir = ($part -replace '\\', '/').Substring(0, ($part -replace '\\', '/').LastIndexOf('/'))
    $file = ($part -replace '\\', '/').Substring(($part -replace '\\', '/').LastIndexOf('/') + 1)
    $rels = "$dir/_rels/$file.rels"
    $relsXml = Get-ZipBytes $pptx $rels
    $relBody = @"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdOoxpPic" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image-ooxp-cm.png"/>
</Relationships>
"@
    if ($null -ne $relsXml) {
        $old = [Text.Encoding]::UTF8.GetString($relsXml)
        if ($old -notmatch 'rIdOoxpPic') {
            $relBody = $old.Replace('</Relationships>', '  <Relationship Id="rIdOoxpPic" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image-ooxp-cm.png"/></Relationships>')
        } else {
            $relBody = $old
        }
    }
    Add-ZipBytes $pptx $rels ([Text.Encoding]::UTF8.GetBytes($relBody))
    Add-ZipBytes $pptx 'ppt/media/image-ooxp-cm.png' ([IO.File]::ReadAllBytes($pngPath))
    Rewrite-ZipPart $pptx '[Content_Types].xml' {
        param($xml)
        if ($xml -match 'Extension="png"') { return $xml }
        return $xml.Replace(
            '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">',
            '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="png" ContentType="image/png"/>'
        )
    }
}

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

function Find-ZipPartContaining([string]$zipPath, [string]$prefix, [string]$needle) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        foreach ($e in $zip.Entries) {
            $n = $e.FullName -replace '\\', '/'
            if ($n -like $prefix) {
                $ms = New-Object IO.MemoryStream
                $es = $e.Open()
                $es.CopyTo($ms)
                $es.Dispose()
                $text = [Text.Encoding]::UTF8.GetString($ms.ToArray())
                if ($text.Contains($needle)) { return $n }
            }
        }
    }
    finally { $zip.Dispose() }
    return $null
}

function Rewrite-ZipPart([string]$zipPath, [string]$part, [scriptblock]$edit) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::Open($zipPath, 'Update')
    try {
        $entry = $zip.Entries | Where-Object { ($_.FullName -replace '\\', '/') -eq $part } | Select-Object -First 1
        if (-not $entry) { throw "zip 里没有 $part" }
        $ms = New-Object IO.MemoryStream
        $es = $entry.Open()
        $es.CopyTo($ms)
        $es.Dispose()
        $xml = [Text.Encoding]::UTF8.GetString($ms.ToArray())
        $text2 = & $edit $xml
        $name = $entry.FullName
        $entry.Delete()
        $newEntry = $zip.CreateEntry($name)
        $bytes = [Text.Encoding]::UTF8.GetBytes($text2)
        $ws = $newEntry.Open()
        $ws.Write($bytes, 0, $bytes.Length)
        $ws.Dispose()
    }
    finally { $zip.Dispose() }
}

$pngA = Ensure-OoxpPngA
$picSrc = Get-OutPath 'pptx-comments-pic-source.pptx'
Close-PptPresByPath $ppt $picSrc
if (Test-Path $picSrc) { Remove-Item $picSrc -Force }
Copy-Item -Force $src $picSrc
Inject-PptxCommentBlip $picSrc ([string]$cA.commentId) $pngA

$treePicCm = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $picSrc.Replace('\', '/'); format = 'pptx'
    scope = @{ comments = $true; pictures = $true }
} 'pptx-comments-pic-inspect'
$cPic = Find-PptCommentByText $treePicCm ([string]$cA.text)
$picCountOk = $cPic -and ([int]$cPic.pictureCount -eq 1)
$wantPicId = "$($cA.id)/pic:1"
$picIdOk = $false
if ($cPic -and $cPic.pictures) {
    $picIdOk = ([string]$cPic.pictures[0].id -eq $wantPicId)
}
$expPic = Get-OutPath 'pptx-comments-pic.png'
if (Test-Path $expPic) { Remove-Item $expPic -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $picSrc.Replace('\', '/')
    id = $wantPicId
    output = $expPic.Replace('\', '/')
} 'pptx-comments-pic-export' | Out-Null
$mediaBytes = Get-ZipBytes $picSrc 'ppt/media/image-ooxp-cm.png'
$expBytes = if (Test-Path $expPic) { [IO.File]::ReadAllBytes($expPic) } else { @() }
$bytesOk = ($null -ne $mediaBytes) -and ($expBytes.Length -eq $mediaBytes.Length)
if ($bytesOk) {
    for ($bi = 0; $bi -lt $mediaBytes.Length; $bi++) {
        if ($expBytes[$bi] -ne $mediaBytes[$bi]) { $bytesOk = $false; break }
    }
}
$failPic = Get-OutPath 'pptx-comments-pic-insert.pptx'
if (Test-Path $failPic) { Remove-Item $failPic -Force }
$rawPic = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $picSrc.Replace('\', '/')
    output = $failPic.Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = $cA.id; file = $pngA.Replace('\', '/') })
} 'pptx-comments-pic-insert'
$picCode = ''
try { $picCode = [string](($rawPic.Stderr | ConvertFrom-Json).error.code) } catch { $picCode = $rawPic.Stderr }
$noPicWrite = -not (Test-Path $failPic)
Add-Check 'pptx-Pcm6' (
    $picCountOk -and $picIdOk -and $bytesOk -and ($picCode -eq 'InvalidId') -and $noPicWrite
) "count=$($cPic.pictureCount) id=$($cPic.pictures[0].id) bytes=$bytesOk code=$picCode wrote=$(-not $noPicWrite)"

function Inject-PptxCommentInk([string]$pptx, [switch]$StripText) {
    $script:InkStrip = [bool]$StripText
    $part = Find-ZipPartContaining $pptx 'ppt/comments/*.xml' $script:InkCmId
    if (-not $part) { throw "批注部件里没有 $($script:InkCmId)" }
    Rewrite-ZipPart $pptx $part {
        param($xml)
        $tIdx = $xml.IndexOf($script:InkCmId)
        if ($tIdx -lt 0) { throw "部件里没有 $($script:InkCmId)" }
        $head = $xml.Substring(0, [Math]::Min($tIdx + 20, $xml.Length))
        $cmStart = $head.LastIndexOf('<p188:cm')
        if ($cmStart -lt 0) { $cmStart = $head.LastIndexOf('<p:cm') }
        if ($cmStart -lt 0) { throw '找不到批注元素起点' }
        $cmEnd = $xml.IndexOf('</p188:cm>', $tIdx)
        $endLen = '</p188:cm>'.Length
        if ($cmEnd -lt 0) {
            $cmEnd = $xml.IndexOf('</p:cm>', $tIdx)
            $endLen = '</p:cm>'.Length
        }
        if ($cmEnd -lt 0) { throw '找不到批注元素终点' }
        $cmEnd += $endLen
        $block = $xml.Substring($cmStart, $cmEnd - $cmStart)
        if ($block -notmatch '<ink[\s/>]') {
            $close = '</p188:txBody>'
            $p = $block.LastIndexOf($close)
            if ($p -lt 0) {
                $close = '</a:txBody>'
                $p = $block.LastIndexOf($close)
            }
            if ($p -lt 0) { throw '批注里没有 txBody 结束标签' }
            $block = $block.Insert($p + $close.Length, '<ink/>')
        }
        if ($script:InkStrip) {
            $block = [regex]::Replace($block, '<a:t(\s[^>]*)?>.*?</a:t>', '<a:t></a:t>')
        }
        return $xml.Substring(0, $cmStart) + $block + $xml.Substring($cmEnd)
    }
}

function Find-PptInkComment($tree) {
    foreach ($c in @($tree.slides[0].comments)) {
        if ([string]$c.kind -eq 'ink') { return $c }
    }
    return $null
}

$script:InkCmId = [string]$cB.commentId
$inkSrc = Get-OutPath 'pptx-comments-ink-source.pptx'
Close-PptPresByPath $ppt $inkSrc
if (Test-Path $inkSrc) { Remove-Item $inkSrc -Force }
Copy-Item -Force $src $inkSrc
Inject-PptxCommentInk $inkSrc

$treeInk = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inkSrc.Replace('\', '/'); format = 'pptx'
    scope = @{ comments = $true }
} 'pptx-comments-ink-inspect'
$inkC = Find-PptInkComment $treeInk
$noBmp = $true
if ($inkC) {
    foreach ($n in @($inkC.PSObject.Properties.Name)) {
        if (@('inkBitmap', 'inkImage', 'bitmap', 'inkData', 'inkPng') -contains $n) { $noBmp = $false }
    }
}
$presInk = Open-PptPres $ppt $inkSrc
$capInk = Get-PptCaption $presInk
Close-PptPres $presInk
Add-Check 'pptx-Pik1' (($null -ne $inkC) -and $noBmp -and ($capInk -notmatch 'Repair')) "id=$($inkC.id) kind=$($inkC.kind) noBmp=$noBmp cap=$capInk"

$metaOut = Get-OutPath 'pptx-comments-ink-meta.pptx'
Close-PptPresByPath $ppt $metaOut
if (Test-Path $metaOut) { Remove-Item $metaOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inkSrc.Replace('\', '/')
    output = $metaOut.Replace('\', '/')
    ops = @(@{ op = 'setCommentMeta'; id = $inkC.id; author = 'Ink2'; initials = 'K' })
} 'pptx-comments-ink-meta' | Out-Null

$treeMeta = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $metaOut.Replace('\', '/'); format = 'pptx'
    scope = @{ comments = $true }
} 'pptx-comments-ink-meta-inspect'
$inkMeta = Find-PptInkComment $treeMeta
$presM = Open-PptPres $ppt $metaOut
$capM = Get-PptCaption $presM
Close-PptPres $presM
Add-Check 'pptx-Pik2' (($capM -notmatch 'Repair') -and ($inkMeta.author -eq 'Ink2') -and ($inkMeta.initials -eq 'K')) "cap=$capM author=$($inkMeta.author) initials=$($inkMeta.initials)"

$doneOut = Get-OutPath 'pptx-comments-ink-done.pptx'
Close-PptPresByPath $ppt $doneOut
if (Test-Path $doneOut) { Remove-Item $doneOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inkSrc.Replace('\', '/')
    output = $doneOut.Replace('\', '/')
    ops = @(@{ op = 'setCommentDone'; id = $inkC.id; done = $true })
} 'pptx-comments-ink-done' | Out-Null

$treeDone = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $doneOut.Replace('\', '/'); format = 'pptx'
    scope = @{ comments = $true }
} 'pptx-comments-ink-done-inspect'
$inkDone = Find-PptInkComment $treeDone
$presDn = Open-PptPres $ppt $doneOut
$capDn = Get-PptCaption $presDn
Close-PptPres $presDn
Add-Check 'pptx-Pik3' (($capDn -notmatch 'Repair') -and ([bool]$inkDone.done -eq $true)) "cap=$capDn done=$($inkDone.done)"

$inkBare = Get-OutPath 'pptx-comments-ink-bare.pptx'
Close-PptPresByPath $ppt $inkBare
if (Test-Path $inkBare) { Remove-Item $inkBare -Force }
Copy-Item -Force $inkSrc $inkBare
Inject-PptxCommentInk $inkBare -StripText
$failOut = Get-OutPath 'pptx-comments-ink-text.pptx'
if (Test-Path $failOut) { Remove-Item $failOut -Force }
$rawInk = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inkBare.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'setCommentText'; id = $inkC.id; text = $S.commentTextNew })
} 'pptx-comments-ink-set-text'
$inkCode = ''
try { $inkCode = [string](($rawInk.Stderr | ConvertFrom-Json).error.code) } catch { $inkCode = $rawInk.Stderr }
$noWrite = -not (Test-Path $failOut)

$delOut = Get-OutPath 'pptx-comments-ink-delete.pptx'
Close-PptPresByPath $ppt $delOut
if (Test-Path $delOut) { Remove-Item $delOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inkBare.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteComment'; id = $inkC.id })
} 'pptx-comments-ink-delete' | Out-Null
$treeDel = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'pptx'
    scope = @{ comments = $true }
} 'pptx-comments-ink-delete-inspect'
$stillInk = $null -ne (Find-PptInkComment $treeDel)
$presDel = Open-PptPres $ppt $delOut
$capDel = Get-PptCaption $presDel
Close-PptPres $presDel
Add-Check 'pptx-Pik4' (($inkCode -eq 'NotImplemented') -and $noWrite -and ($capDel -notmatch 'Repair') -and (-not $stillInk)) "code=$inkCode wrote=$(-not $noWrite) cap=$capDel stillInk=$stillInk"
