# docx 批注：docs/checklist-docx.md 的 Cmt1–Cmt7。
# 源文件由 Word 在前两段插入批注并回复后另存。不要用 create 产物当唯一样例。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-DocxWordCommentsSource
$pngB = Ensure-OoxpPngB
$word = Ensure-MsWord $src

function Find-CommentByText($tree, [string]$text) {
    foreach ($c in @($tree.comments)) {
        $p = @($c.paragraphs)[0]
        if ($p -and ([string]$p.text -eq $text)) { return $c }
    }
    return $null
}

function Test-WordCommentText($doc, [string]$text) {
    $comments = $doc.Comments
    try {
        $n = [int]$comments.Count
        for ($i = 1; $i -le $n; $i++) {
            $c = $comments.Item($i)
            try {
                $t = ([string]$c.Range.Text).Trim()
                if ($t -match [regex]::Escape($text)) { return $true }
            }
            finally { Release-Com $c }
        }
        return $false
    }
    finally { Release-Com $comments }
}

# Cmt1：无 comments 时只有计数；comments:true 时有名单、范围、回复 parent。
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-comments-inspect-counts'
$noList = -not ($tree0.PSObject.Properties.Name -contains 'comments')
Add-Check 'docx-Cmt1a' (($tree0.commentCount -ge 3) -and $noList) "count=$($tree0.commentCount) noList=$noList"

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; comments = $true }
} 'docx-comments-inspect-lists'
$cA = Find-CommentByText $tree1 $S.commentA
$cB = Find-CommentByText $tree1 $S.commentB
$cR = Find-CommentByText $tree1 $S.commentReply
$idOk = $cA -and $cB -and $cR -and ($cA.id -like 'comment:*') -and ($cB.id -like 'comment:*')
$parentOk = $cR -and ($cR.parent -eq $cA.id)
$rangeOk = $cA.start -and $cA.end -and $cB.start -and $cB.end
$doneOk = ($cA.done -eq $false) -or ($cA.done -eq $true)
$p1ids = @($tree1.paragraphs[0].commentIds)
$onP1 = ($p1ids -contains $cA.id) -and ($p1ids -contains $cR.id)
Add-Check 'docx-Cmt1' ($idOk -and $parentOk -and $rangeOk -and $doneOk -and $onP1) "a=$($cA.id) b=$($cB.id) reply=$($cR.id) parent=$($cR.parent) start=$($cA.start)"

# Cmt2：改批注字，正文未改，无修复。
$textOut = Get-OutPath 'docx-comments-text.docx'
Close-WordDocByPath $word $textOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $textOut.Replace('\', '/')
    ops = @(@{ op = 'setCommentText'; id = $cA.id; text = $S.commentTextNew })
} 'docx-comments-set-text' | Out-Null

$docT = Open-WordDoc $word $textOut
$capT = [string]$docT.ActiveWindow.Caption
$sawNew = Test-WordCommentText $docT $S.commentTextNew
$p1 = ([string]$docT.Paragraphs.Item(1).Range.Text).Trim()
Add-Check 'docx-Cmt2' (($capT -notmatch 'Repair') -and $sawNew -and ($p1 -eq $S.chapter1)) "caption=$capT saw=$sawNew body=$p1"
Close-WordDoc $docT

# Cmt3：insertComment 后审阅窗格能看见新批注。
$p3 = $tree1.paragraphs[2].id
$insOut = Get-OutPath 'docx-comments-insert.docx'
Close-WordDocByPath $word $insOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{ op = 'insertComment'; start = $p3; end = $p3; text = $S.commentNew })
} 'docx-comments-insert' | Out-Null

$docI = Open-WordDoc $word $insOut
$capI = [string]$docI.ActiveWindow.Caption
$sawIns = Test-WordCommentText $docI $S.commentNew
Close-WordDoc $docI
$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'docx'
    scope = @{ comments = $true }
} 'docx-comments-insert-inspect'
$newC = Find-CommentByText $treeI $S.commentNew
Add-Check 'docx-Cmt3' (($capI -notmatch 'Repair') -and $sawIns -and ($null -ne $newC) -and ($treeI.commentCount -gt $tree0.commentCount)) "caption=$capI count=$($treeI.commentCount) id=$($newC.id)"

# Cmt4：删掉原线程根，回复一并消失。
$delOut = Get-OutPath 'docx-comments-delete.docx'
Close-WordDocByPath $word $delOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteComment'; id = $cA.id })
} 'docx-comments-delete' | Out-Null

$docD = Open-WordDoc $word $delOut
$capD = [string]$docD.ActiveWindow.Caption
$countD = [int]$docD.Comments.Count
$stillA = Test-WordCommentText $docD $S.commentA
$stillR = Test-WordCommentText $docD $S.commentReply
$stillB = Test-WordCommentText $docD $S.commentB
Add-Check 'docx-Cmt4' (($capD -notmatch 'Repair') -and (-not $stillA) -and (-not $stillR) -and $stillB -and ($countD -lt $tree0.commentCount)) "caption=$capD count=$countD stillA=$stillA stillR=$stillR stillB=$stillB"
Close-WordDoc $docD

# Cmt5：replyComment 后 parent 指向父批注，Word 能看见线程。
$replyOut = Get-OutPath 'docx-comments-reply.docx'
Close-WordDocByPath $word $replyOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $replyOut.Replace('\', '/')
    ops = @(@{
        op = 'replyComment'
        id = $cB.id
        text = $S.commentNew
        date = '2026-08-31T12:00:00Z'
    })
} 'docx-comments-reply' | Out-Null

$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replyOut.Replace('\', '/'); format = 'docx'
    scope = @{ comments = $true }
} 'docx-comments-reply-inspect'
$newReply = Find-CommentByText $treeR $S.commentNew
$parentHit = $newReply -and ($newReply.parent -eq $cB.id)

$docR = Open-WordDoc $word $replyOut
$capR = [string]$docR.ActiveWindow.Caption
$sawReply = Test-WordCommentText $docR $S.commentNew
$replyCount = 0
$ancestorOk = $false
$countR = [int]$docR.Comments.Count
$commentsR = $docR.Comments
try {
    $nR = [int]$commentsR.Count
    for ($i = 1; $i -le $nR; $i++) {
        $c = $commentsR.Item($i)
        try {
            $t = ([string]$c.Range.Text).Trim()
            if ($t -match [regex]::Escape($S.commentB)) {
                $rs = $c.Replies
                try { $replyCount = [int]$rs.Count } finally { Release-Com $rs }
            }
            if ($t -match [regex]::Escape($S.commentNew)) {
                try {
                    $a = $c.Ancestor
                    $ancestorOk = ($null -ne $a)
                    if ($a) { Release-Com $a }
                }
                catch { $ancestorOk = $false }
            }
        }
        finally { Release-Com $c }
    }
}
finally { Release-Com $commentsR }
Add-Check 'docx-Cmt5' (($capR -notmatch 'Repair') -and $parentHit -and $sawReply -and (($replyCount -ge 1) -or $ancestorOk)) "caption=$capR parent=$($newReply.parent) replies=$replyCount ancestor=$ancestorOk saw=$sawReply count=$countR"
Close-WordDoc $docR

# Cmt6：setCommentDone 后 COM Done 为 true。
$doneOut = Get-OutPath 'docx-comments-done.docx'
Close-WordDocByPath $word $doneOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $doneOut.Replace('\', '/')
    ops = @(@{ op = 'setCommentDone'; id = $cA.id; done = $true })
} 'docx-comments-done' | Out-Null

$docN = Open-WordDoc $word $doneOut
$capN = [string]$docN.ActiveWindow.Caption
$doneVal = $false
$commentsN = $docN.Comments
try {
    $nN = [int]$commentsN.Count
    for ($i = 1; $i -le $nN; $i++) {
        $c = $commentsN.Item($i)
        try {
            $t = ([string]$c.Range.Text).Trim()
            if ($t -match [regex]::Escape($S.commentA)) {
                try { $doneVal = [bool]$c.Done } catch { $doneVal = $false }
            }
        }
        finally { Release-Com $c }
    }
}
finally { Release-Com $commentsN }
Add-Check 'docx-Cmt6' (($capN -notmatch 'Repair') -and $doneVal) "caption=$capN done=$doneVal"
Close-WordDoc $docN

# Cmt7：批注内插图。
$cApara = @($cA.paragraphs)[0].id
$picOut = Get-OutPath 'docx-comments-picture.docx'
Close-WordDocByPath $word $picOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $picOut.Replace('\', '/')
    ops = @(@{
        op = 'insertPicture'
        id = $cApara
        file = $pngB.Replace('\', '/')
    })
} 'docx-comments-insert-pic' | Out-Null

$docP = Open-WordDoc $word $picOut
$capP = [string]$docP.ActiveWindow.Caption
$inCmt = 0
$commentsP = $docP.Comments
try {
    $nP = [int]$commentsP.Count
    for ($i = 1; $i -le $nP; $i++) {
        $c = $commentsP.Item($i)
        try {
            $t = ([string]$c.Range.Text).Trim()
            if ($t -match [regex]::Escape($S.commentA)) {
                try { $inCmt = [int]$c.Range.InlineShapes.Count } catch { $inCmt = 0 }
            }
        }
        finally { Release-Com $c }
    }
}
finally { Release-Com $commentsP }
Add-Check 'docx-Cmt7' (($capP -notmatch 'Repair') -and ($inCmt -ge 1)) "caption=$capP inlineInComment=$inCmt"
Close-WordDoc $docP
