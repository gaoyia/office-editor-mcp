# pptx 备注：docs/checklist-pptx.md 的 Pn1–Pn6。
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

$chartOut = Get-OutPath 'pptx-notes-chart.pptx'
Close-PptPresByPath $ppt $chartOut
if (Test-Path $chartOut) { Remove-Item $chartOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $chartOut.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'
        id = 'slide:1/notes'
        type = 'col'
        title = $S.chartTitle
        series = @(@{
            name = $S.chartSer1
            categories = @($S.saItem1, $S.saItem2, $S.saItem3)
            values = @(1, 2, 3)
        })
    })
} 'pptx-notes-insert-chart' | Out-Null

$treeCh = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $chartOut.Replace('\', '/'); format = 'pptx'
    scope = @{ notes = $true; charts = $true }
} 'pptx-notes-chart-inspect'
$nNotesCh = $treeCh.slides[0].notes
$chId = $null
if ($nNotesCh -and $nNotesCh.charts -and @($nNotesCh.charts).Count -ge 1) {
    $chId = [string]$nNotesCh.charts[0].id
}
$chIdOk = ($null -ne $chId) -and ($chId -like 'slide:1/notes/chart:*')
$slideChStill = [int]$treeCh.chartCount -eq 0 -and [int]$treeCh.slides[0].chartCount -eq 0
$serId = $null
if ($chIdOk) { $serId = "$chId/series:0" }

$serOut = Get-OutPath 'pptx-notes-chart-series.pptx'
Close-PptPresByPath $ppt $serOut
if (Test-Path $serOut) { Remove-Item $serOut -Force }
if ($serId) {
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $chartOut.Replace('\', '/')
        output = $serOut.Replace('\', '/')
        ops = @(@{ op = 'setChartSeries'; id = $serId; values = @(10, 20, 30) })
    } 'pptx-notes-chart-series' | Out-Null
}
$treeSer = $null
$valsOk = $false
if (Test-Path $serOut) {
    $treeSer = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $serOut.Replace('\', '/'); format = 'pptx'
        scope = @{ notes = $true; charts = $true }
    } 'pptx-notes-chart-series-inspect'
    $got = @($treeSer.slides[0].notes.charts[0].series[0].values)
    $valsOk = ($got.Count -eq 3) -and ([double]$got[0] -eq 10) -and ([double]$got[1] -eq 20) -and ([double]$got[2] -eq 30)
}

$openChart = if (Test-Path $serOut) { $serOut } else { $chartOut }
$presC = Open-PptPres $ppt $openChart
$capC = Get-PptCaption $presC
Close-PptPres $presC
Add-Check 'pptx-Pn5' (
    ($capC -notmatch 'Repair') -and $chIdOk -and $slideChStill -and $valsOk
) "cap=$capC id=$chId slideCc=$($treeCh.chartCount) vals=$valsOk"

$saOut = Get-OutPath 'pptx-notes-smartart.pptx'
Close-PptPresByPath $ppt $saOut
if (Test-Path $saOut) { Remove-Item $saOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $saOut.Replace('\', '/')
    ops = @(@{
        op = 'insertSmartArt'
        id = 'slide:1/notes'
        layout = 'list'
        points = @(
            @{ text = $S.saItem1 }
            @{ text = $S.saItem2 }
            @{ text = $S.saItem3 }
        )
    })
} 'pptx-notes-insert-smartart' | Out-Null

$treeSa = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $saOut.Replace('\', '/'); format = 'pptx'
    scope = @{ notes = $true; smartArt = $true }
} 'pptx-notes-smartart-inspect'
$nNotesSa = $treeSa.slides[0].notes
$saId = $null
if ($nNotesSa -and $nNotesSa.smartArts -and @($nNotesSa.smartArts).Count -ge 1) {
    $saId = [string]$nNotesSa.smartArts[0].id
}
$saIdOk = ($null -ne $saId) -and ($saId -like 'slide:1/notes/smartdiagram:*')
$slideSaStill = [int]$treeSa.smartArtCount -eq 0 -and [int]$treeSa.slides[0].smartArtCount -eq 0
$presSa = Open-PptPres $ppt $saOut
$capSa = Get-PptCaption $presSa
Close-PptPres $presSa
Add-Check 'pptx-Pn6' (
    ($capSa -notmatch 'Repair') -and $saIdOk -and $slideSaStill
) "cap=$capSa id=$saId slideSa=$($treeSa.smartArtCount)"
