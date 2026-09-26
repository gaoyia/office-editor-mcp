# docx 脚注尾注：docs/checklist-docx.md 的 N1–N6。
# 源文件由 Word 在第二段插入脚注和尾注后另存。不要用 create 产物当唯一样例。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-DocxWordNotesSource
$pngB = Ensure-OoxpPngB
$word = Ensure-MsWord $src

# N1：无 notes 时只有计数；notes:true 时有名单、host、分隔符。
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-notes-inspect-counts'
$p1noList = -not ($tree0.PSObject.Properties.Name -contains 'footnotes')
$p1noSeps = -not ($tree0.PSObject.Properties.Name -contains 'noteSeparators')
Add-Check 'docx-N1a' (($tree0.footnoteCount -ge 1) -and ($tree0.endnoteCount -ge 1) -and $p1noList -and $p1noSeps) "fn=$($tree0.footnoteCount) en=$($tree0.endnoteCount)"

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; notes = $true }
} 'docx-notes-inspect-lists'
$fn = @($tree1.footnotes) | Select-Object -First 1
$en = @($tree1.endnotes) | Select-Object -First 1
$seps = @($tree1.noteSeparators)
$hostOk = $false
if ($fn -and $fn.host) {
    $hostOk = @($fn.host) -contains $tree1.paragraphs[1].id -or (@($fn.host).Count -ge 1)
}
$sepKinds = @($seps | ForEach-Object { $_.id })
$hasFnSep = ($sepKinds | Where-Object { $_ -like 'footnote:sep:*' }).Count -ge 1
$hasEnSep = ($sepKinds | Where-Object { $_ -like 'endnote:sep:*' }).Count -ge 1
$idOk = ($fn.id -like 'footnote:*') -and ($en.id -like 'endnote:*')
Add-Check 'docx-N1' ($idOk -and $hostOk -and $hasFnSep -and $hasEnSep -and ($fn.type -eq 'normal')) "fn=$($fn.id) host=$($fn.host -join ',') seps=$($sepKinds -join ',')"

# N2：改注文，正文未改，无修复。
$fnPara = @($fn.paragraphs)[0].id
$textOut = Get-OutPath 'docx-notes-text.docx'
Close-WordDocByPath $word $textOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $textOut.Replace('\', '/')
    ops = @(@{ op = 'setParaText'; id = $fnPara; text = $S.footnoteNew })
} 'docx-notes-set-text' | Out-Null

$docT = Open-WordDoc $word $textOut
$capT = [string]$docT.ActiveWindow.Caption
$fnText = ([string]$docT.Footnotes.Item(1).Range.Text).Trim()
$p1 = ([string]$docT.Paragraphs.Item(1).Range.Text).Trim()
Add-Check 'docx-N2' (($capT -notmatch 'Repair') -and ($fnText -match [regex]::Escape($S.footnoteNew)) -and ($p1 -eq $S.chapter1)) "caption=$capT fn=$fnText body=$p1"
Close-WordDoc $docT

# N3：insertFootnote 后 Word 能看见引用与注文。
$p3 = $tree1.paragraphs[2].id
$insOut = Get-OutPath 'docx-notes-insert.docx'
Close-WordDocByPath $word $insOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{ op = 'insertFootnote'; id = $p3; text = $S.footnoteInsert })
} 'docx-notes-insert' | Out-Null

$docI = Open-WordDoc $word $insOut
$capI = [string]$docI.ActiveWindow.Caption
$fnCount = [int]$docI.Footnotes.Count
$sawNew = $false
for ($i = 1; $i -le $fnCount; $i++) {
    $t = ([string]$docI.Footnotes.Item($i).Range.Text).Trim()
    if ($t -match [regex]::Escape($S.footnoteInsert)) { $sawNew = $true }
}
Add-Check 'docx-N3' (($capI -notmatch 'Repair') -and ($fnCount -ge 2) -and $sawNew) "caption=$capI count=$fnCount sawNew=$sawNew"
Close-WordDoc $docI

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'docx'
    scope = @{ notes = $true }
} 'docx-notes-insert-inspect'
$newFn = @($treeI.footnotes) | Where-Object {
    @($_.paragraphs) | Where-Object { $_.text -eq $S.footnoteInsert }
} | Select-Object -First 1

# N4：删掉刚插入的那条。
$delOut = Get-OutPath 'docx-notes-delete.docx'
Close-WordDocByPath $word $delOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteFootnote'; id = $newFn.id })
} 'docx-notes-delete' | Out-Null

$docD = Open-WordDoc $word $delOut
$capD = [string]$docD.ActiveWindow.Caption
$fnCountD = [int]$docD.Footnotes.Count
$stillNew = $false
for ($i = 1; $i -le $fnCountD; $i++) {
    $t = ([string]$docD.Footnotes.Item($i).Range.Text).Trim()
    if ($t -match [regex]::Escape($S.footnoteInsert)) { $stillNew = $true }
}
Add-Check 'docx-N4' (($capD -notmatch 'Repair') -and ($fnCountD -eq ($tree0.footnoteCount)) -and (-not $stillNew)) "caption=$capD count=$fnCountD"
Close-WordDoc $docD

# N5：整篇编号改成大写罗马。
$numOut = Get-OutPath 'docx-notes-numbering.docx'
Close-WordDocByPath $word $numOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $numOut.Replace('\', '/')
    ops = @(@{
        op = 'setNoteNumbering'
        id = 'docx:document'
        kind = 'footnote'
        numFmt = 'upperRoman'
    })
} 'docx-notes-numbering' | Out-Null

$docN = Open-WordDoc $word $numOut
$capN = [string]$docN.ActiveWindow.Caption
$styleN = $null
try { $styleN = [int]$docN.Footnotes.NumberStyle } catch { $styleN = -1 }
# wdNoteNumberStyleUppercaseRoman = 1
Add-Check 'docx-N5' (($capN -notmatch 'Repair') -and ($styleN -eq 1)) "caption=$capN numberStyle=$styleN"
Close-WordDoc $docN

# N6：注内插图。
$picOut = Get-OutPath 'docx-notes-picture.docx'
Close-WordDocByPath $word $picOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $picOut.Replace('\', '/')
    ops = @(@{
        op = 'insertPicture'
        id = $fnPara
        file = $pngB.Replace('\', '/')
    })
} 'docx-notes-insert-pic' | Out-Null

$docP = Open-WordDoc $word $picOut
$capP = [string]$docP.ActiveWindow.Caption
$inFn = 0
try { $inFn = [int]$docP.Footnotes.Item(1).Range.InlineShapes.Count } catch { $inFn = 0 }
Add-Check 'docx-N6' (($capP -notmatch 'Repair') -and ($inFn -ge 1)) "caption=$capP inlineInFootnote=$inFn"
Close-WordDoc $docP
