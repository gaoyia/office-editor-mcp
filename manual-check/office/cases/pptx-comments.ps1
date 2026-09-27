# pptx 批注：docs/checklist-pptx.md 的 Pcm1–Pcm5。
# 源文件：PowerPoint 亲手另存，第一页有两条线程化批注，其中一条带回复。不要用库 create 产物当带批注的唯一样例。

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
