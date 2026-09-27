# pptx 备注：docs/checklist-pptx.md 的 Pn1–Pn4。
# 源文件：PowerPoint 亲手另存，第一页有演讲者备注。不要用库 create 产物当带备注的唯一样例。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-PptxPptNotesSource
$png = Ensure-OoxpPngA
$ppt = Ensure-MsPpt $src
Close-PptPresByPath $ppt $src

function Normalize-Notes([string]$t) {
    if ($null -eq $t) { return '' }
    return $t.Trim().TrimEnd([char]13, [char]10, [char]11, [char]7)
}

function Get-PptNotesText($slide) {
    $page = $slide.NotesPage
    try {
        $shapes = $page.Shapes
        try {
            $n = [int]$shapes.Count
            for ($i = 1; $i -le $n; $i++) {
                $shp = $shapes.Item($i)
                try {
                    $ph = $null
                    try { $ph = $shp.PlaceholderFormat } catch { $ph = $null }
                    if ($null -eq $ph) { continue }
                    try {
                        if ([int]$ph.Type -eq 2) {
                            return (Normalize-Notes (Get-ShapeText $shp))
                        }
                    } finally { Release-Com $ph }
                } finally { Release-Com $shp }
            }
        } finally { Release-Com $shapes }
        return ''
    } finally { Release-Com $page }
}

function Get-PptNotesPictureCount($slide) {
    $page = $slide.NotesPage
    try {
        $nPic = 0
        $shapes = $page.Shapes
        try {
            for ($i = 1; $i -le $shapes.Count; $i++) {
                $sh = $shapes.Item($i)
                try {
                    if ([int]$sh.Type -eq 13) { $nPic++ }
                } finally { Release-Com $sh }
            }
        } finally { Release-Com $shapes }
        return $nPic
    } finally { Release-Com $page }
}

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
} 'pptx-notes-inspect-list'
$s0 = $tree0.slides[0]
$s1bare = $tree0.slides[1]
$noNotesObj = -not ($s0.PSObject.Properties.Name -contains 'notes')
$hasFlag = ($s0.PSObject.Properties.Name -contains 'hasNotes') -and [bool]$s0.hasNotes
$s2flag = ($s1bare.PSObject.Properties.Name -contains 'hasNotes') -and -not [bool]$s1bare.hasNotes
Add-Check 'pptx-Pn1a' ($hasFlag -and $noNotesObj -and $s2flag) "hasNotes=$($s0.hasNotes) noObj=$noNotesObj s2=$($s1bare.hasNotes)"

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ notes = $true }
} 'pptx-notes-inspect-expand'
$notes = $tree1.slides[0].notes
$idOk = ($null -ne $notes) -and ([string]$notes.id -eq 'slide:1/notes')
$texts = @()
if ($notes -and $notes.shapes) {
    foreach ($sh in @($notes.shapes)) { $texts += [string]$sh.text }
}
$hit = $false
foreach ($t in $texts) {
    if ((Normalize-Notes $t) -eq $S.notesSrc) { $hit = $true }
}
Add-Check 'pptx-Pn1' ($idOk -and $hit) "id=$($notes.id) hit=$hit texts=$($texts -join '|')"

$noted = Get-OutPath 'pptx-notes-set.pptx'
Close-PptPresByPath $ppt $noted
if (Test-Path $noted) { Remove-Item $noted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $noted.Replace('\', '/')
    ops = @(@{ op = 'setNotesText'; id = 'slide:1'; text = $S.notesNew })
} 'pptx-notes-set' | Out-Null

$presN = Open-PptPres $ppt $noted
$capN = Get-PptCaption $presN
$slideN = Get-PptSlide $presN 1
$notesText = Get-PptNotesText $slideN
$box1 = Get-PptShape $slideN 1
$slideText = Get-ShapeText $box1
Add-Check 'pptx-Pn2' (($capN -notmatch 'Repair') -and ((Normalize-Notes $notesText) -eq $S.notesNew)) "cap=$capN notes=$notesText"
Add-Check 'pptx-Pn3a' ((Normalize-Notes $slideText) -eq $S.chapter1) $slideText
Release-Com $box1
Release-Com $slideN
Close-PptPres $presN

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $noted.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1; maxShapes = 200 }
} 'pptx-notes-slide-shapes'
$idSlide = $srcTree.slides[0].shapes[0].id
$mixed = Get-OutPath 'pptx-notes-mixed.pptx'
Close-PptPresByPath $ppt $mixed
if (Test-Path $mixed) { Remove-Item $mixed -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $noted.Replace('\', '/')
    output = $mixed.Replace('\', '/')
    ops = @(@{ op = 'setShapeText'; id = $idSlide; text = $S.shape2new })
} 'pptx-notes-set-slide' | Out-Null

$presM = Open-PptPres $ppt $mixed
$capM = Get-PptCaption $presM
$slideM = Get-PptSlide $presM 1
$notesKeep = Get-PptNotesText $slideM
$boxM = Get-PptShape $slideM 1
$slideNew = Get-ShapeText $boxM
Add-Check 'pptx-Pn3' (
    ($capM -notmatch 'Repair') -and
    ((Normalize-Notes $notesKeep) -eq $S.notesNew) -and
    ((Normalize-Notes $slideNew) -eq $S.shape2new)
) "notes=$notesKeep slide=$slideNew cap=$capM"
Release-Com $boxM
Release-Com $slideM
Close-PptPres $presM

$picOut = Get-OutPath 'pptx-notes-pic.pptx'
Close-PptPresByPath $ppt $picOut
if (Test-Path $picOut) { Remove-Item $picOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $picOut.Replace('\', '/')
    ops = @(@{
        op = 'insertPicture'
        id = 'slide:1/notes'
        file = $png.Replace('\', '/')
        x = 914400
        y = 4572000
        cx = 1828800
        cy = 914400
    })
} 'pptx-notes-insert-pic' | Out-Null

$treePic = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $picOut.Replace('\', '/'); format = 'pptx'
    scope = @{ notes = $true; pictures = $true }
} 'pptx-notes-pic-inspect'
$nNotes = $treePic.slides[0].notes
$picIdOk = $false
$nPics = 0
if ($nNotes -and $nNotes.pictures) {
    $nPics = @($nNotes.pictures).Count
    if ($nPics -ge 1) { $picIdOk = ([string]$nNotes.pictures[0].id -like 'slide:1/notes/pic:*') }
}
$rootStill = [int]$treePic.pictureCount -eq 0

$presP = Open-PptPres $ppt $picOut
$capP = Get-PptCaption $presP
$slideP = Get-PptSlide $presP 1
$comPics = Get-PptNotesPictureCount $slideP
Add-Check 'pptx-Pn4' (
    ($capP -notmatch 'Repair') -and $picIdOk -and $rootStill -and ($comPics -ge 1)
) "cap=$capP idOk=$picIdOk rootPc=$($treePic.pictureCount) com=$comPics n=$nPics"
Release-Com $slideP
Close-PptPres $presP
