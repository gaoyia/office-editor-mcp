# pptx 幻灯片图：docs/checklist-pptx.md 的 Pp1–Pp7。
# 源文件：PowerPoint 亲手另存，第一页至少一张 p:pic。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Ensure-PptxPptPictureSource
$pngB = Ensure-OoxpPngB
$ppt = Ensure-MsPpt $src
Close-PptPresByPath $ppt $src

function Get-PptPictureCount($slide) {
    $n = 0
    $shapes = $slide.Shapes
    try {
        for ($i = 1; $i -le $shapes.Count; $i++) {
            $sh = $shapes.Item($i)
            try {
                if ([int]$sh.Type -eq 13) { $n++ }
            } finally { Release-Com $sh }
        }
    } finally { Release-Com $shapes }
    return $n
}

function Find-PptPicture($slide, [string]$name) {
    $shapes = $slide.Shapes
    try {
        for ($i = 1; $i -le $shapes.Count; $i++) {
            $cand = $shapes.Item($i)
            if ([int]$cand.Type -eq 13 -and [string]$cand.Name -eq $name) {
                return $cand
            }
            Release-Com $cand
        }
    } finally { Release-Com $shapes }
    return $null
}

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
} 'pptx-pic-inspect-list'
$slide0 = $tree0.slides[0]
$noList = -not ($slide0.PSObject.Properties.Name -contains 'pictures')
Add-Check 'pptx-Pp1a' (($tree0.pictureCount -ge 1) -and ($slide0.pictureCount -ge 1) -and $noList) "wb=$($tree0.pictureCount) slide=$($slide0.pictureCount)"

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ pictures = $true }
} 'pptx-pic-inspect-expand'
$pics = @($tree1.slides[0].pictures)
$idOk = ($pics.Count -ge 1) -and ($pics[0].id -like 'slide:1/pic:*')
Add-Check 'pptx-Pp1' (($pics.Count -ge 1) -and $idOk) "n=$($pics.Count) id=$($pics[0].id)"
$srcPic = $pics[0]
$srcX = $srcPic.x
$srcY = $srcPic.y
$srcCx = $srcPic.cx
$srcCy = $srcPic.cy

$replaced = Get-OutPath 'pptx-pic-replaced.pptx'
Close-PptPresByPath $ppt $replaced
if (Test-Path $replaced) { Remove-Item $replaced -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{
        op = 'replacePicture'
        id = $srcPic.id
        file = $pngB.Replace('\', '/')
    })
} 'pptx-pic-replace' | Out-Null

$presR = Open-PptPres $ppt $replaced
$capR = Get-PptCaption $presR
$sR = Get-PptSlide $presR 1
$nR = Get-PptPictureCount $sR
Add-Check 'pptx-Pp2' (($capR -notmatch 'Repair') -and ($nR -ge 1)) "caption=$capR pics=$nR"
Release-Com $sR
Close-PptPres $presR

$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'pptx'
    scope = @{ pictures = $true }
} 'pptx-pic-inspect-replaced'
$oneR = @($treeR.slides[0].pictures) | Where-Object { $_.id -eq $srcPic.id } | Select-Object -First 1
Add-Check 'pptx-Pp2b' (($oneR.x -eq $srcX) -and ($oneR.y -eq $srcY) -and ($oneR.cx -eq $srcCx) -and ($oneR.cy -eq $srcCy)) "x=$($oneR.x) y=$($oneR.y) cx=$($oneR.cx)"

$inserted = Get-OutPath 'pptx-pic-inserted.pptx'
Close-PptPresByPath $ppt $inserted
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $replaced.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{
        op = 'insertPicture'
        id = 'slide:1'
        file = $pngB.Replace('\', '/')
        x = 2743200
        y = 2743200
        cx = 1828800
        cy = 1828800
    })
} 'pptx-pic-insert' | Out-Null

$presI = Open-PptPres $ppt $inserted
$capI = Get-PptCaption $presI
$sI = Get-PptSlide $presI 1
$nI = Get-PptPictureCount $sI
Add-Check 'pptx-Pp3' (($capI -notmatch 'Repair') -and ($nI -gt $nR)) "caption=$capI pics=$nI"
Release-Com $sI
Close-PptPres $presI

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'pptx'
    scope = @{ pictures = $true }
} 'pptx-pic-inspect-inserted'
$oldIds = @($pics | ForEach-Object { $_.id })
$newPic = @($treeI.slides[0].pictures) | Where-Object { $oldIds -notcontains $_.id } | Select-Object -First 1
Add-Check 'pptx-Pp3b' (($null -ne $newPic) -and ($newPic.cx -eq 1828800) -and ($newPic.x -eq 2743200)) "id=$($newPic.id) x=$($newPic.x)"

$deleted = Get-OutPath 'pptx-pic-deleted.pptx'
Close-PptPresByPath $ppt $deleted
if (Test-Path $deleted) { Remove-Item $deleted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $deleted.Replace('\', '/')
    ops = @(@{ op = 'deletePicture'; id = $newPic.id })
} 'pptx-pic-delete' | Out-Null

$presD = Open-PptPres $ppt $deleted
$capD = Get-PptCaption $presD
$sD = Get-PptSlide $presD 1
$nD = Get-PptPictureCount $sD
Add-Check 'pptx-Pp4' (($capD -notmatch 'Repair') -and ($nD -eq $nR)) "caption=$capD pics=$nD"
Release-Com $sD
Close-PptPres $presD

$sized = Get-OutPath 'pptx-pic-sized.pptx'
Close-PptPresByPath $ppt $sized
if (Test-Path $sized) { Remove-Item $sized -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $deleted.Replace('\', '/')
    output = $sized.Replace('\', '/')
    ops = @(@{
        op = 'setPictureSize'
        id = $srcPic.id
        cx = 1828800
        cy = 1828800
    })
} 'pptx-pic-size' | Out-Null

$presS = Open-PptPres $ppt $sized
$capS = Get-PptCaption $presS
$sS = Get-PptSlide $presS 1
$shapeS = Find-PptPicture $sS ([string]$srcPic.name)
if (-not $shapeS) {
    $shapesS = $sS.Shapes
    try {
        for ($i = 1; $i -le $shapesS.Count; $i++) {
            $cand = $shapesS.Item($i)
            if ([int]$cand.Type -eq 13) {
                if ($shapeS) { Release-Com $cand } else { $shapeS = $cand }
            } else { Release-Com $cand }
        }
    } finally { Release-Com $shapesS }
}
if (-not $shapeS) { throw "PowerPoint 里找不到图 $($srcPic.name)" }
$wPt = [double]$shapeS.Width
$hPt = [double]$shapeS.Height
$sizeOk = ([math]::Abs($wPt - 144) -lt 8) -and ([math]::Abs($hPt - 144) -lt 8)
Add-Check 'pptx-Pp5' (($capS -notmatch 'Repair') -and $sizeOk) "caption=$capS w=$wPt h=$hPt"
Release-Com $shapeS
Release-Com $sS
Close-PptPres $presS

$moved = Get-OutPath 'pptx-pic-moved.pptx'
Close-PptPresByPath $ppt $moved
if (Test-Path $moved) { Remove-Item $moved -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sized.Replace('\', '/')
    output = $moved.Replace('\', '/')
    ops = @(@{
        op = 'setPicturePos'
        id = $srcPic.id
        x = 914400
        y = 1371600
    })
} 'pptx-pic-pos' | Out-Null

$presM = Open-PptPres $ppt $moved
$capM = Get-PptCaption $presM
$sM = Get-PptSlide $presM 1
$shapeM = Find-PptPicture $sM ([string]$srcPic.name)
if (-not $shapeM) {
    $shapesM = $sM.Shapes
    try {
        for ($i = 1; $i -le $shapesM.Count; $i++) {
            $cand = $shapesM.Item($i)
            if ([int]$cand.Type -eq 13) {
                if ($shapeM) { Release-Com $cand } else { $shapeM = $cand }
            } else { Release-Com $cand }
        }
    } finally { Release-Com $shapesM }
}
if (-not $shapeM) { throw "PowerPoint 里找不到要移位的图" }
$left = [double]$shapeM.Left
$top = [double]$shapeM.Top
# 72 磅 = 914400 EMU，108 磅 = 1371600 EMU。
$posOk = ([math]::Abs($left - 72) -lt 8) -and ([math]::Abs($top - 108) -lt 8)
Add-Check 'pptx-Pp6' (($capM -notmatch 'Repair') -and $posOk) "caption=$capM left=$left top=$top"
Release-Com $shapeM
Release-Com $sM
Close-PptPres $presM

$exported = Get-OutPath 'pptx-pic-export.png'
if (Test-Path $exported) { Remove-Item $exported -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $moved.Replace('\', '/')
    id = $srcPic.id
    output = $exported.Replace('\', '/')
} 'pptx-pic-export' | Out-Null

$outBytes = [IO.File]::ReadAllBytes($exported)
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::OpenRead($moved)
$matched = 0
$mediaLens = @()
try {
    foreach ($entry in @($zip.Entries | Where-Object { $_.FullName -like 'ppt/media/*' })) {
        $ms = New-Object System.IO.MemoryStream
        $es = $entry.Open()
        $es.CopyTo($ms)
        $es.Dispose()
        $mediaBytes = $ms.ToArray()
        $ms.Dispose()
        $mediaLens += $mediaBytes.Length
        if ($mediaBytes.Length -ne $outBytes.Length) { continue }
        $sameEntry = $true
        for ($i = 0; $i -lt $outBytes.Length; $i++) {
            if ($outBytes[$i] -ne $mediaBytes[$i]) { $sameEntry = $false; break }
        }
        if ($sameEntry) { $matched++ }
    }
} finally {
    $zip.Dispose()
}
Add-Check 'pptx-Pp7' ($matched -eq 1) "export=$($outBytes.Length) matched=$matched media=$($mediaLens -join ',')"
