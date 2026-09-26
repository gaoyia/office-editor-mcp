# docx 文本框：docs/checklist-docx.md 的 T1–T9。
# 源文件由 Word 另存：正文有行内框、浮动框、一对链接框，以及组合里的框。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-DocxWordTextBoxSource
$pngB = Ensure-OoxpPngB
$word = Ensure-MsWord $src

function Get-BodyTextBoxes($tree) {
    $boxes = New-Object System.Collections.ArrayList
    foreach ($p in @($tree.paragraphs)) {
        foreach ($b in @($p.textBoxes)) { [void]$boxes.Add($b) }
    }
    return @($boxes)
}

function Get-WordShapeByText($doc, [string]$text) {
    $shapes = $doc.Shapes
    try {
        for ($i = 1; $i -le [int]$shapes.Count; $i++) {
            $sh = $shapes.Item($i)
            $hit = Get-WordShapeByTextIn $sh $text
            if ($hit) { return $hit }
            Release-Com $sh
        }
    } finally { Release-Com $shapes }
    return $null
}

function Get-WordShapeByTextIn($sh, [string]$text) {
    $t = ''
    $tf = $null
    $tr = $null
    try {
        if ([int]$sh.Type -eq 6) {
            $items = $sh.GroupItems
            try {
                for ($j = 1; $j -le [int]$items.Count; $j++) {
                    $g = $items.Item($j)
                    $hit = Get-WordShapeByTextIn $g $text
                    if ($hit) { return $hit }
                    Release-Com $g
                }
            } finally { Release-Com $items }
            return $null
        }
        $tf = $sh.TextFrame
        if ([int]$tf.HasText -ne 0) {
            $tr = $tf.TextRange
            $t = ([string]$tr.Text).Trim()
        }
    } catch {}
    Release-Com $tr
    Release-Com $tf
    if ($t -eq $text) { return $sh }
    return $null
}

# T1：无 scope 根上有 textBoxCount、没有正文 textBoxes；切片 + textBoxes 时有框 id。
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-txbx-inspect-list'
$p1noParas = -not ($tree0.PSObject.Properties.Name -contains 'paragraphs')
$p1noBoxes = -not ($tree0.PSObject.Properties.Name -contains 'textBoxes')
Add-Check 'docx-T1a' (($null -ne $tree0.textBoxCount) -and ($tree0.textBoxCount -ge 4) -and $p1noParas -and $p1noBoxes) "textBoxCount=$($tree0.textBoxCount)"

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true }
} 'docx-txbx-inspect-expand'
$all = Get-BodyTextBoxes $tree1
$kinds = @($all | ForEach-Object { $_.kind })
$hasInline = $kinds -contains 'inline'
$hasAnchor = $kinds -contains 'anchor'
$idOk = ($all.Count -ge 4) -and ($all[0].id -like 'para:*/txbx:*')
Add-Check 'docx-T1' (($all.Count -ge 4) -and $hasInline -and $hasAnchor -and $idOk) "n=$($all.Count) kinds=$($kinds -join ',') id=$($all[0].id)"

# T2：框内段落 id 带框前缀，text 对，不并进宿主 text。
$tree2 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true; textBoxContent = $true }
} 'docx-txbx-inspect-content'
$boxInline = Find-TreeTextBox $tree2 $S.txbxInline
$boxFloat = Find-TreeTextBox $tree2 $S.txbxFloat
$boxLinkA = Find-TreeTextBox $tree2 $S.txbxLinkA
$boxLinkB = $null
if ($boxLinkA -and $boxLinkA.link.next) {
    $boxLinkB = Find-TreeTextBoxById $tree2 ([string]$boxLinkA.link.next)
}
if (-not $boxLinkB) { $boxLinkB = Find-TreeTextBox $tree2 $S.txbxLinkB }
$boxGroup = Find-TreeTextBox $tree2 $S.txbxGroup
$host2 = @($tree2.paragraphs) | Where-Object { $_.text -eq $S.para2src } | Select-Object -First 1
$inner = $null
if ($boxInline) { $inner = @($boxInline.paragraphs) | Select-Object -First 1 }
$innerIdOk = ($null -ne $inner) -and ($inner.id -like '*/txbx:*/para:*')
$hostOk = ($null -ne $host2) -and ([string]$host2.text -notlike "*$($S.txbxInline)*") -and ([string]$host2.text -notlike "*$($S.txbxFloat)*")
Add-Check 'docx-T2' (($null -ne $boxInline) -and $innerIdOk -and $hostOk -and ([string]$inner.text -eq $S.txbxInline)) "innerId=$($inner.id) host=$($host2.text)"

# T3：链接框 prev / next 互指。
$linkNext = $null
$linkPrev = $null
if ($boxLinkA) { $linkNext = $boxLinkA.link.next }
if ($boxLinkB) { $linkPrev = $boxLinkB.link.prev }
$linkOk = ($null -ne $boxLinkA) -and ($null -ne $boxLinkB) -and ($linkNext -eq $boxLinkB.id) -and ($linkPrev -eq $boxLinkA.id)
Add-Check 'docx-T3' $linkOk "a=$($boxLinkA.id) next=$linkNext b=$($boxLinkB.id) prev=$linkPrev"

$floatPara = $null
if ($boxFloat) { $floatPara = @($boxFloat.paragraphs) | Select-Object -First 1 }
$heading = @($tree2.paragraphs) | Select-Object -First 1

# T4：对框内段落 setParaText，Word 无修复，框还在。
$edited = Get-OutPath 'docx-txbx-text.docx'
Close-WordDocByPath $word $edited
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $edited.Replace('\', '/')
    ops = @(@{
        op = 'setParaText'
        id = $floatPara.id
        text = $S.txbxInnerNew
    })
} 'docx-txbx-settext' | Out-Null

$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $edited.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true; textBoxContent = $true }
} 'docx-txbx-inspect-text'
$boxT = Find-TreeTextBox $treeT $S.txbxInnerNew
$docT = Open-WordDoc $word $edited
$capT = [string]$docT.ActiveWindow.Caption
$stillT = $docT.InlineShapes.Count + $docT.Shapes.Count
Add-Check 'docx-T4' (($capT -notmatch 'Repair') -and ($null -ne $boxT) -and ($stillT -ge 1)) "caption=$capT still=$stillT"
Close-WordDoc $docT

# T5：插框再删框。
$inserted = Get-OutPath 'docx-txbx-inserted.docx'
Close-WordDocByPath $word $inserted
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{
        op = 'insertTextBox'
        id = $heading.id
        text = '新插框'
        cx = 914400
        cy = 914400
    })
} 'docx-txbx-insert' | Out-Null

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true; textBoxContent = $true }
} 'docx-txbx-inspect-inserted'
$newBox = Find-TreeTextBox $treeI '新插框'
$docI = Open-WordDoc $word $inserted
$capI = [string]$docI.ActiveWindow.Caption
Add-Check 'docx-T5a' (($capI -notmatch 'Repair') -and ($null -ne $newBox) -and ($treeI.textBoxCount -eq ($tree0.textBoxCount + 1))) "caption=$capI count=$($treeI.textBoxCount) id=$($newBox.id)"
Close-WordDoc $docI

$deleted = Get-OutPath 'docx-txbx-deleted.docx'
Close-WordDocByPath $word $deleted
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $deleted.Replace('\', '/')
    ops = @(@{ op = 'deleteTextBox'; id = $newBox.id })
} 'docx-txbx-delete' | Out-Null

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $deleted.Replace('\', '/'); format = 'docx'
} 'docx-txbx-inspect-deleted'
$docD = Open-WordDoc $word $deleted
$capD = [string]$docD.ActiveWindow.Caption
Add-Check 'docx-T5' (($capD -notmatch 'Repair') -and ($treeD.textBoxCount -eq $tree0.textBoxCount)) "caption=$capD count=$($treeD.textBoxCount)"
Close-WordDoc $docD

# T6：尺寸与绕排。2 英寸 = 144 磅 = 1828800 EMU。
$sized = Get-OutPath 'docx-txbx-sized.docx'
Close-WordDocByPath $word $sized
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $sized.Replace('\', '/')
    ops = @(@{
        op = 'setTextBoxSize'
        id = $boxFloat.id
        cx = 1828800
        cy = 1828800
    })
} 'docx-txbx-size' | Out-Null

$docS = Open-WordDoc $word $sized
$capS = [string]$docS.ActiveWindow.Caption
$shS = Get-WordShapeByText $docS $S.txbxFloat
$wPt = $null
if ($shS) {
    $wPt = [double]$shS.Width
    Release-Com $shS
}
Add-Check 'docx-T6a' (($capS -notmatch 'Repair') -and ($null -ne $wPt) -and ([math]::Abs($wPt - 144) -lt 2)) "caption=$capS widthPt=$wPt"
Close-WordDoc $docS

$wrapped = Get-OutPath 'docx-txbx-wrapped.docx'
Close-WordDocByPath $word $wrapped
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $wrapped.Replace('\', '/')
    ops = @(@{
        op = 'setTextBoxWrap'
        id = $boxInline.id
        kind = 'anchor'
        wrap = 'square'
        alignH = 'left'
        alignV = 'top'
    })
} 'docx-txbx-wrap' | Out-Null

$treeW = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $wrapped.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true; textBoxContent = $true }
} 'docx-txbx-inspect-wrapped'
$boxW = Find-TreeTextBox $treeW $S.txbxInline
$docW = Open-WordDoc $word $wrapped
$capW = [string]$docW.ActiveWindow.Caption
Add-Check 'docx-T6' (($capW -notmatch 'Repair') -and ($boxW.kind -eq 'anchor') -and ($boxW.wrap -eq 'square')) "caption=$capW kind=$($boxW.kind) wrap=$($boxW.wrap)"
Close-WordDoc $docW

# T7：填色与描边。
$styled = Get-OutPath 'docx-txbx-styled.docx'
Close-WordDocByPath $word $styled
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $styled.Replace('\', '/')
    ops = @(
        @{ op = 'setTextBoxFill'; id = $boxFloat.id; color = 'FF0000' }
        @{ op = 'setTextBoxLine'; id = $boxFloat.id; color = '0000FF'; width = 19050 }
    )
} 'docx-txbx-style' | Out-Null

$docY = Open-WordDoc $word $styled
$capY = [string]$docY.ActiveWindow.Caption
$shY = Get-WordShapeByText $docY $S.txbxFloat
$fillRgb = $null
$lineRgb = $null
if ($shY) {
    $fill = $shY.Fill
    $fc = $fill.ForeColor
    $fillRgb = [int]$fc.RGB
    $line = $shY.Line
    $lc = $line.ForeColor
    $lineRgb = [int]$lc.RGB
    Release-Com $lc
    Release-Com $line
    Release-Com $fc
    Release-Com $fill
    Release-Com $shY
}
# Word 的 RGB 是 BGR：红=255，蓝=16711680。
Add-Check 'docx-T7' (($capY -notmatch 'Repair') -and ($fillRgb -eq 255) -and ($lineRgb -eq 16711680)) "caption=$capY fill=$fillRgb line=$lineRgb"
Close-WordDoc $docY

# T8：框内插删段、插图。
$innered = Get-OutPath 'docx-txbx-inner.docx'
Close-WordDocByPath $word $innered
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $innered.Replace('\', '/')
    ops = @(@{
        op = 'insertPara'
        id = $boxFloat.id
        text = $S.txbxNewPara
    })
} 'docx-txbx-insert-para' | Out-Null

$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $innered.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true; textBoxContent = $true }
} 'docx-txbx-inspect-inner-para'
$boxN = Find-TreeTextBox $treeN $S.txbxFloat
$newPara = $null
if ($boxN) { $newPara = @($boxN.paragraphs) | Where-Object { $_.text -eq $S.txbxNewPara } | Select-Object -First 1 }
$oldInner = $null
if ($boxN) { $oldInner = @($boxN.paragraphs) | Where-Object { $_.text -eq $S.txbxFloat } | Select-Object -First 1 }

$withPic = Get-OutPath 'docx-txbx-inner-pic.docx'
Close-WordDocByPath $word $withPic
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $innered.Replace('\', '/')
    output = $withPic.Replace('\', '/')
    ops = @(@{
        op = 'insertPicture'
        id = $oldInner.id
        file = $pngB.Replace('\', '/')
        cx = 914400
        cy = 914400
    })
} 'docx-txbx-insert-pic' | Out-Null

$treeP = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $withPic.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true; textBoxContent = $true; pictures = $true }
} 'docx-txbx-inspect-inner-pic'
$boxP = Find-TreeTextBox $treeP $S.txbxFloat
$hostP = @($treeP.paragraphs) | Where-Object { $_.id -eq $host2.id } | Select-Object -First 1
$picOk = ($null -ne $boxP) -and ($boxP.pictureCount -ge 1) -and ($boxP.paraCount -ge 2)
$hostPicsOk = ($null -ne $hostP) -and ($hostP.pictureCount -eq 0)
$docP = Open-WordDoc $word $withPic
$capP = [string]$docP.ActiveWindow.Caption
Add-Check 'docx-T8a' (($capP -notmatch 'Repair') -and $picOk -and $hostPicsOk) "caption=$capP paras=$($boxP.paraCount) pics=$($boxP.pictureCount) hostPics=$($hostP.pictureCount)"
Close-WordDoc $docP

$trimmed = Get-OutPath 'docx-txbx-inner-trim.docx'
Close-WordDocByPath $word $trimmed
$delPara = $null
if ($boxP) { $delPara = @($boxP.paragraphs) | Where-Object { $_.text -eq $S.txbxNewPara } | Select-Object -First 1 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $withPic.Replace('\', '/')
    output = $trimmed.Replace('\', '/')
    ops = @(@{ op = 'deletePara'; id = $delPara.id })
} 'docx-txbx-delete-para' | Out-Null

$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $trimmed.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true; textBoxContent = $true; pictures = $true }
} 'docx-txbx-inspect-inner-trim'
$boxR = Find-TreeTextBox $treeR $S.txbxFloat
$docR = Open-WordDoc $word $trimmed
$capR = [string]$docR.ActiveWindow.Caption
Add-Check 'docx-T8' (($capR -notmatch 'Repair') -and ($boxR.paraCount -eq 1) -and ($boxR.pictureCount -ge 1)) "caption=$capR paras=$($boxR.paraCount) pics=$($boxR.pictureCount)"
Close-WordDoc $docR

# T9：断开链接再接上，Word 能打开。
$unlinked = Get-OutPath 'docx-txbx-unlinked.docx'
Close-WordDocByPath $word $unlinked
# ConvertTo-Json 会丢掉 $null，setTextBoxLink 的 next 必须显式写出 null。
$unlinkPath = Join-Path $global:OoxpReqDir 'docx-txbx-unlink.json'
$unlinkJson = '{"schemaVersion":1,"action":"apply","path":"' + ($src.Replace('\', '/')) + '","output":"' + ($unlinked.Replace('\', '/')) + '","ops":[{"op":"setTextBoxLink","id":"' + $boxLinkA.id + '","next":null}]}'
[System.IO.File]::WriteAllText($unlinkPath, $unlinkJson, $global:OoxpUtf8)
Invoke-Ooxp $unlinkPath | Out-Null

$treeU = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $unlinked.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true; textBoxContent = $true }
} 'docx-txbx-inspect-unlinked'
$boxUA = Find-TreeTextBoxById $treeU $boxLinkA.id
if (-not $boxUA) { $boxUA = Find-TreeTextBox $treeU $S.txbxLinkA }
$boxUB = Find-TreeTextBoxById $treeU $boxLinkB.id
$unlinkedOk = ($null -ne $boxUA) -and ($null -ne $boxUB) -and ($null -eq $boxUA.link.next) -and ($null -eq $boxUB.link.prev)

$relinked = Get-OutPath 'docx-txbx-relinked.docx'
Close-WordDocByPath $word $relinked
$relinkErr = $null
try {
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $unlinked.Replace('\', '/')
        output = $relinked.Replace('\', '/')
        ops = @(@{ op = 'setTextBoxLink'; id = $boxUA.id; next = $boxUB.id })
    } 'docx-txbx-relink' | Out-Null
} catch {
    $relinkErr = $_.Exception.Message
}

$boxLA = $null
$boxLB = $null
$treeL = $null
$capL = ''
if (-not $relinkErr) {
    $treeL = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $relinked.Replace('\', '/'); format = 'docx'
        scope = @{ from = 1; maxParas = 20; textBoxes = $true; textBoxContent = $true }
    } 'docx-txbx-inspect-relinked'
    $boxLA = Find-TreeTextBoxById $treeL $boxUA.id
    $boxLB = Find-TreeTextBoxById $treeL $boxUB.id
    $docL = Open-WordDoc $word $relinked
    $capL = [string]$docL.ActiveWindow.Caption
    Close-WordDoc $docL
}
$relinkOk = (-not $relinkErr) -and ($null -ne $boxLA) -and ($null -ne $boxLB) -and ($boxLA.link.next -eq $boxLB.id) -and ($boxLB.link.prev -eq $boxLA.id)
Add-Check 'docx-T9' (($capL -notmatch 'Repair') -and $unlinkedOk -and $relinkOk) "caption=$capL unlink=$unlinkedOk relink=$relinkOk err=$relinkErr"
