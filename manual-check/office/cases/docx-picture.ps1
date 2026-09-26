# docx 页眉页脚图文混排：docs/checklist-docx.md 的 P1–P7。
# 源文件由 Word 亲手往页眉插行内图和浮动图后另存。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Ensure-DocxWordPictureSource
$pngB = Ensure-OoxpPngB
$word = Ensure-MsWord $src

# P1：无 scope 有 pictureCount；pictures:true 时行内 + 浮动都在。
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-pic-inspect-list'
$h0 = @($tree0.headers) | Where-Object { $_.type -eq 'default' } | Select-Object -First 1
$p1noPics = -not ($h0.PSObject.Properties.Name -contains 'pictures')
Add-Check 'docx-P1a' (($null -ne $h0.pictureCount) -and $p1noPics) "pictureCount=$($h0.pictureCount)"

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ pictures = $true }
} 'docx-pic-inspect-expand'
$h1 = @($tree1.headers) | Where-Object { $_.type -eq 'default' } | Select-Object -First 1
$pics = @($h1.pictures)
$kinds = @($pics | ForEach-Object { $_.kind })
$hasInline = $kinds -contains 'inline'
$hasAnchor = $kinds -contains 'anchor'
$anchor = $pics | Where-Object { $_.kind -eq 'anchor' } | Select-Object -First 1
$wrapOk = ($null -eq $anchor) -or ($anchor.wrap -eq 'square')
Add-Check 'docx-P1' (($pics.Count -ge 2) -and $hasInline -and $hasAnchor -and $wrapOk) "n=$($pics.Count) kinds=$($kinds -join ',') wrap=$($anchor.wrap)"

# P2：断开第二节后图还在，无修复。
$unlinked = Get-OutPath 'docx-pic-unlinked.docx'
Close-WordDocByPath $word $unlinked
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $unlinked.Replace('\', '/')
    ops = @(@{
        op = 'setHeaderFooterLink'
        id = 'section:2'
        kind = 'header'
        type = 'default'
        linked = $false
    })
} 'docx-pic-unlink' | Out-Null

$docU = Open-WordDoc $word $unlinked
$capU = [string]$docU.ActiveWindow.Caption
$sec2 = $docU.Sections.Item(2)
$hdr2 = $sec2.Headers.Item(1)
$linked = [bool]$hdr2.LinkToPrevious
$in2 = $hdr2.Range.InlineShapes.Count
$sh2 = $hdr2.Shapes.Count
Add-Check 'docx-P2' (($capU -notmatch 'Repair') -and (-not $linked) -and (($in2 + $sh2) -ge 1)) "caption=$capU linked=$linked inline=$in2 shapes=$sh2"
Release-Com $hdr2
Release-Com $sec2
Close-WordDoc $docU

# 后面的 apply 都以已断开的文件为源，避免改到共享页眉。
$inlinePic = $pics | Where-Object { $_.kind -eq 'inline' } | Select-Object -First 1
$anchorPic = $pics | Where-Object { $_.kind -eq 'anchor' } | Select-Object -First 1

$treeU = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $unlinked.Replace('\', '/'); format = 'docx'
    scope = @{ pictures = $true; headerFooter = $true }
} 'docx-pic-inspect-unlinked'
$hSec2 = @($treeU.headers) | Where-Object {
    $_.type -eq 'default' -and (@($_.sections) -contains 2)
} | Select-Object -First 1
$pics2 = @($hSec2.pictures)
$inline2 = $pics2 | Where-Object { $_.kind -eq 'inline' } | Select-Object -First 1
$paraId = @($hSec2.paragraphs)[0].id
if (-not $inline2) { $inline2 = $inlinePic }
if (-not $paraId) { $paraId = 'header:default:2/para:index:1' }

# P3：换图。
$replaced = Get-OutPath 'docx-pic-replaced.docx'
Close-WordDocByPath $word $replaced
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $unlinked.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{
        op = 'replacePicture'
        id = $inline2.id
        file = $pngB.Replace('\', '/')
    })
} 'docx-pic-replace' | Out-Null

$docR = Open-WordDoc $word $replaced
$capR = [string]$docR.ActiveWindow.Caption
$secR = $docR.Sections.Item(2)
$hdrR = $secR.Headers.Item(1)
$still = $hdrR.Range.InlineShapes.Count + $hdrR.Shapes.Count
Add-Check 'docx-P3' (($capR -notmatch 'Repair') -and ($still -ge 1)) "caption=$capR shapes=$still"
Release-Com $hdrR
Release-Com $secR
Close-WordDoc $docR

# P4：插图。
$inserted = Get-OutPath 'docx-pic-inserted.docx'
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
} 'docx-pic-insert' | Out-Null

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'docx'
    scope = @{ pictures = $true }
} 'docx-pic-inspect-inserted'
$hI = @($treeI.headers) | Where-Object {
    $_.type -eq 'default' -and (@($_.sections) -contains 2)
} | Select-Object -First 1
$docI = Open-WordDoc $word $inserted
$capI = [string]$docI.ActiveWindow.Caption
$secI = $docI.Sections.Item(2)
$hdrI = $secI.Headers.Item(1)
$cntI = $hdrI.Range.InlineShapes.Count
Add-Check 'docx-P4' (($capI -notmatch 'Repair') -and ($hI.pictureCount -gt $hSec2.pictureCount) -and ($cntI -ge 1)) "caption=$capI count=$($hI.pictureCount) inline=$cntI"
Release-Com $hdrI
Release-Com $secI
Close-WordDoc $docI

$newPic = @($hI.pictures) | Where-Object { $_.id -notin @($pics2 | ForEach-Object { $_.id }) } | Select-Object -Last 1
if (-not $newPic) { $newPic = @($hI.pictures) | Select-Object -Last 1 }

# P5：删刚插入的图。
$deleted = Get-OutPath 'docx-pic-deleted.docx'
Close-WordDocByPath $word $deleted
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $deleted.Replace('\', '/')
    ops = @(@{ op = 'deletePicture'; id = $newPic.id })
} 'docx-pic-delete' | Out-Null

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $deleted.Replace('\', '/'); format = 'docx'
    scope = @{ pictures = $true }
} 'docx-pic-inspect-deleted'
$hD = @($treeD.headers) | Where-Object {
    $_.type -eq 'default' -and (@($_.sections) -contains 2)
} | Select-Object -First 1
$docD = Open-WordDoc $word $deleted
$capD = [string]$docD.ActiveWindow.Caption
Add-Check 'docx-P5' (($capD -notmatch 'Repair') -and ($hD.pictureCount -eq $hSec2.pictureCount)) "caption=$capD count=$($hD.pictureCount)"
Close-WordDoc $docD

# P6：尺寸。1 英寸 = 72 磅 = 914400 EMU。
$sizeId = $inline2.id
$sized = Get-OutPath 'docx-pic-sized.docx'
Close-WordDocByPath $word $sized
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $unlinked.Replace('\', '/')
    output = $sized.Replace('\', '/')
    ops = @(@{
        op = 'setPictureSize'
        id = $sizeId
        cx = 1828800
        cy = 1828800
    })
} 'docx-pic-size' | Out-Null

$docS = Open-WordDoc $word $sized
$capS = [string]$docS.ActiveWindow.Caption
$secS = $docS.Sections.Item(2)
$hdrS = $secS.Headers.Item(1)
$wPt = $null
if ($hdrS.Range.InlineShapes.Count -ge 1) {
    $ish = $hdrS.Range.InlineShapes.Item(1)
    $wPt = [double]$ish.Width
    Release-Com $ish
}
Add-Check 'docx-P6' (($capS -notmatch 'Repair') -and ($null -ne $wPt) -and ([math]::Abs($wPt - 144) -lt 2)) "caption=$capS widthPt=$wPt"
Release-Com $hdrS
Release-Com $secS
Close-WordDoc $docS

# P7：行内改浮动 square。
$wrapped = Get-OutPath 'docx-pic-wrapped.docx'
Close-WordDocByPath $word $wrapped
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $unlinked.Replace('\', '/')
    output = $wrapped.Replace('\', '/')
    ops = @(@{
        op = 'setPictureWrap'
        id = $sizeId
        kind = 'anchor'
        wrap = 'square'
        alignH = 'left'
        alignV = 'top'
    })
} 'docx-pic-wrap' | Out-Null

$treeW = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $wrapped.Replace('\', '/'); format = 'docx'
    scope = @{ pictures = $true }
} 'docx-pic-inspect-wrapped'
$hW = @($treeW.headers) | Where-Object {
    $_.type -eq 'default' -and (@($_.sections) -contains 2)
} | Select-Object -First 1
$pw = @($hW.pictures) | Where-Object { $_.id -eq $sizeId } | Select-Object -First 1
$docW = Open-WordDoc $word $wrapped
$capW = [string]$docW.ActiveWindow.Caption
Add-Check 'docx-P7' (($capW -notmatch 'Repair') -and ($pw.kind -eq 'anchor') -and ($pw.wrap -eq 'square')) "caption=$capW kind=$($pw.kind) wrap=$($pw.wrap)"
Close-WordDoc $docW
