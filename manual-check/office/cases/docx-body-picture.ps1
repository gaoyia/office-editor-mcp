# docx 正文图：docs/checklist-docx.md 的 B1–B7。
# 源文件由 Word 往正文插行内图和浮动图后另存。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Ensure-DocxWordBodyPictureSource
$pngB = Ensure-OoxpPngB
$word = Ensure-MsWord $src

# B1：无 scope 根上有 pictureCount；切片 + pictures 时行内 + 浮动都在。
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-body-pic-inspect-list'
$p1noParas = -not ($tree0.PSObject.Properties.Name -contains 'paragraphs')
Add-Check 'docx-B1a' (($null -ne $tree0.pictureCount) -and ($tree0.pictureCount -ge 2) -and $p1noParas) "pictureCount=$($tree0.pictureCount)"

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; pictures = $true }
} 'docx-body-pic-inspect-expand'
$target = @($tree1.paragraphs) | Where-Object { $_.pictureCount -ge 2 } | Select-Object -First 1
$pics = @($target.pictures)
$kinds = @($pics | ForEach-Object { $_.kind })
$hasInline = $kinds -contains 'inline'
$hasAnchor = $kinds -contains 'anchor'
$anchor = $pics | Where-Object { $_.kind -eq 'anchor' } | Select-Object -First 1
$wrapOk = ($null -eq $anchor) -or ($anchor.wrap -eq 'square')
$idOk = ($pics.Count -ge 2) -and ($pics[0].id -like 'para:*/pic:*')
Add-Check 'docx-B1' (($pics.Count -ge 2) -and $hasInline -and $hasAnchor -and $wrapOk -and $idOk) "n=$($pics.Count) kinds=$($kinds -join ',') wrap=$($anchor.wrap) id=$($pics[0].id)"

$inlinePic = $pics | Where-Object { $_.kind -eq 'inline' } | Select-Object -First 1
$paraId = $target.id
if (-not $paraId) { $paraId = 'para:index:2' }

# B2：换图。
$replaced = Get-OutPath 'docx-body-pic-replaced.docx'
Close-WordDocByPath $word $replaced
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{
        op = 'replacePicture'
        id = $inlinePic.id
        file = $pngB.Replace('\', '/')
    })
} 'docx-body-pic-replace' | Out-Null

$docR = Open-WordDoc $word $replaced
$capR = [string]$docR.ActiveWindow.Caption
$still = $docR.InlineShapes.Count + $docR.Shapes.Count
Add-Check 'docx-B2' (($capR -notmatch 'Repair') -and ($still -ge 1)) "caption=$capR shapes=$still"
Close-WordDoc $docR

# B3：插图。
$inserted = Get-OutPath 'docx-body-pic-inserted.docx'
Close-WordDocByPath $word $inserted
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $replaced.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{
        op = 'insertPicture'
        id = $paraId
        file = $pngB.Replace('\', '/')
        cx = 914400
        cy = 914400
    })
} 'docx-body-pic-insert' | Out-Null

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; pictures = $true }
} 'docx-body-pic-inspect-inserted'
$pI = @($treeI.paragraphs) | Where-Object { $_.id -eq $paraId } | Select-Object -First 1
if (-not $pI) { $pI = @($treeI.paragraphs) | Where-Object { $_.pictureCount -ge 1 } | Select-Object -First 1 }
$docI = Open-WordDoc $word $inserted
$capI = [string]$docI.ActiveWindow.Caption
$cntI = $docI.InlineShapes.Count
Add-Check 'docx-B3' (($capI -notmatch 'Repair') -and ($pI.pictureCount -gt $target.pictureCount) -and ($cntI -ge 1)) "caption=$capI count=$($pI.pictureCount) inline=$cntI"
Close-WordDoc $docI

$newPic = @($pI.pictures) | Where-Object { $_.id -notin @($pics | ForEach-Object { $_.id }) } | Select-Object -Last 1
if (-not $newPic) { $newPic = @($pI.pictures) | Select-Object -Last 1 }

# B4：删刚插入的图。
$deleted = Get-OutPath 'docx-body-pic-deleted.docx'
Close-WordDocByPath $word $deleted
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $deleted.Replace('\', '/')
    ops = @(@{ op = 'deletePicture'; id = $newPic.id })
} 'docx-body-pic-delete' | Out-Null

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $deleted.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; pictures = $true }
} 'docx-body-pic-inspect-deleted'
$pD = @($treeD.paragraphs) | Where-Object { $_.id -eq $paraId } | Select-Object -First 1
if (-not $pD) { $pD = @($treeD.paragraphs) | Where-Object { $_.pictureCount -ge 0 } | Select-Object -First 1 }
$docD = Open-WordDoc $word $deleted
$capD = [string]$docD.ActiveWindow.Caption
Add-Check 'docx-B4' (($capD -notmatch 'Repair') -and ($pD.pictureCount -eq $target.pictureCount)) "caption=$capD count=$($pD.pictureCount)"
Close-WordDoc $docD

# B5：尺寸。2 英寸 = 144 磅 = 1828800 EMU。
$sized = Get-OutPath 'docx-body-pic-sized.docx'
Close-WordDocByPath $word $sized
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $sized.Replace('\', '/')
    ops = @(@{
        op = 'setPictureSize'
        id = $inlinePic.id
        cx = 1828800
        cy = 1828800
    })
} 'docx-body-pic-size' | Out-Null

$docS = Open-WordDoc $word $sized
$capS = [string]$docS.ActiveWindow.Caption
$wPt = $null
if ($docS.InlineShapes.Count -ge 1) {
    $ish = $docS.InlineShapes.Item(1)
    $wPt = [double]$ish.Width
    Release-Com $ish
}
Add-Check 'docx-B5' (($capS -notmatch 'Repair') -and ($null -ne $wPt) -and ([math]::Abs($wPt - 144) -lt 2)) "caption=$capS widthPt=$wPt"
Close-WordDoc $docS

# B6：行内改浮动 square。
$wrapped = Get-OutPath 'docx-body-pic-wrapped.docx'
Close-WordDocByPath $word $wrapped
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $wrapped.Replace('\', '/')
    ops = @(@{
        op = 'setPictureWrap'
        id = $inlinePic.id
        kind = 'anchor'
        wrap = 'square'
        alignH = 'left'
        alignV = 'top'
    })
} 'docx-body-pic-wrap' | Out-Null

$treeW = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $wrapped.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; pictures = $true }
} 'docx-body-pic-inspect-wrapped'
$pW = @($treeW.paragraphs) | Where-Object { $_.id -eq $paraId } | Select-Object -First 1
if (-not $pW) { $pW = @($treeW.paragraphs) | Where-Object { $_.pictureCount -ge 1 } | Select-Object -First 1 }
$pw = @($pW.pictures) | Where-Object { $_.id -eq $inlinePic.id } | Select-Object -First 1
$docW = Open-WordDoc $word $wrapped
$capW = [string]$docW.ActiveWindow.Caption
Add-Check 'docx-B6' (($capW -notmatch 'Repair') -and ($pw.kind -eq 'anchor') -and ($pw.wrap -eq 'square')) "caption=$capW kind=$($pw.kind) wrap=$($pw.wrap)"
Close-WordDoc $docW

# B7：改字，图还在。
$edited = Get-OutPath 'docx-body-pic-text.docx'
Close-WordDocByPath $word $edited
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $edited.Replace('\', '/')
    ops = @(@{
        op = 'setParaText'
        id = $paraId
        text = '改过的正文'
    })
} 'docx-body-pic-text' | Out-Null

$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $edited.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; pictures = $true }
} 'docx-body-pic-inspect-text'
$pT = @($treeT.paragraphs) | Where-Object { $_.id -eq $paraId } | Select-Object -First 1
if (-not $pT) { $pT = @($treeT.paragraphs) | Where-Object { $_.pictureCount -ge 1 } | Select-Object -First 1 }
$docT = Open-WordDoc $word $edited
$capT = [string]$docT.ActiveWindow.Caption
$stillT = $docT.InlineShapes.Count + $docT.Shapes.Count
$textOk = [string]$pT.text -eq '改过的正文'
Add-Check 'docx-B7' (($capT -notmatch 'Repair') -and $textOk -and ($pT.pictureCount -ge 2) -and ($stillT -ge 1)) "caption=$capT text=$($pT.text) pics=$($pT.pictureCount) shapes=$stillT"
Close-WordDoc $docT
