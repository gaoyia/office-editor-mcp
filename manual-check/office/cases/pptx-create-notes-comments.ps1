# pptx create 备注与批注：docs/checklist-pptx.md 的 C11–C13。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

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

$path = Ensure-PptxCreateNotesComments -Force

$bare = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'pptx'
} 'pptx-create-notes-comments-inspect-bare'
$s0 = $bare.slides[0]
$s1 = $bare.slides[1]
$noNotesObj = -not ($s0.PSObject.Properties.Name -contains 'notes')
$noCommentsObj = -not ($s0.PSObject.Properties.Name -contains 'comments')
$hasNotes = ($s0.PSObject.Properties.Name -contains 'hasNotes') -and [bool]$s0.hasNotes
$s2flag = ($s1.PSObject.Properties.Name -contains 'hasNotes') -and -not [bool]$s1.hasNotes
Add-Check 'pptx-C11a' ($hasNotes -and $s2flag -and $noNotesObj) "hasNotes=$($s0.hasNotes) s2=$($s1.hasNotes) noObj=$noNotesObj"

$notesTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'pptx'
    scope = @{ notes = $true }
} 'pptx-create-notes-comments-inspect-notes'
$notes = $notesTree.slides[0].notes
$body = ''
if ($notes -and $notes.shapes) {
    foreach ($sh in @($notes.shapes)) {
        if ([string]$sh.id -eq 'slide:1/notes/shape:3') { $body = [string]$sh.text }
    }
}
$idOk = ($null -ne $notes) -and ([string]$notes.id -eq 'slide:1/notes')
Add-Check 'pptx-C11' ($idOk -and ($body -eq '演讲者备注')) "id=$($notes.id) body=$body"

$cmBareOk = ($noCommentsObj) -and ([int]$bare.commentCount -eq 1) -and ([int]$s0.commentCount -eq 1) -and ([int]$s1.commentCount -eq 0)
$cmTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'pptx'
    scope = @{ comments = $true }
} 'pptx-create-notes-comments-inspect-comments'
$c = @($cmTree.slides[0].comments) | Select-Object -First 1
$cmOk = ($null -ne $c) -and ([string]$c.text -eq '一条批注') -and ([string]$c.author -eq 'Alice') -and ([string]$c.id -like 'slide:1/comment:{*}')
Add-Check 'pptx-C12' ($cmBareOk -and $cmOk) "root=$($bare.commentCount) text=$($c.text) author=$($c.author) id=$($c.id)"

$ppt = Ensure-MsPpt (Ensure-PptxCreate)
Close-PptPresByPath $ppt $path
$pres = Open-PptPres $ppt $path
$cap = Get-PptCaption $pres
$slide = Get-PptSlide $pres 1
$notesText = Get-PptNotesText $slide
$comCount = 0
try {
    $comments = $slide.Comments
    try { $comCount = [int]$comments.Count } catch { $comCount = 0 }
    finally { Release-Com $comments }
} catch { $comCount = 0 }
Release-Com $slide
Close-PptPres $pres
Add-Check 'pptx-C13' (($cap -notmatch 'Repair') -and ($notesText -eq '演讲者备注')) "caption=$cap notes=$notesText com=$comCount"
