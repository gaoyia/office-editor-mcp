# docx create 带图：docs/checklist-docx.md 的 C5–C7。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$path = Ensure-DocxCreatePictures
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; pictures = $true; headerFooter = $true }
} 'docx-create-pictures-inspect'

$p2 = @($tree.paragraphs) | Select-Object -Index 1
$pics = @($p2.pictures)
$idOk = ($pics.Count -ge 1) -and ($pics[0].id -like 'para:*/pic:1') -and ($pics[0].kind -eq 'inline')
Add-Check 'docx-C5' $idOk "id=$($pics[0].id) kind=$($pics[0].kind) n=$($pics.Count)"

$hdrPics = @($tree.headers[0].pictures)
$hdrOk = ($hdrPics.Count -ge 1) -and ($hdrPics[0].kind -eq 'anchor') -and ($hdrPics[0].wrap -eq 'square')
Add-Check 'docx-C6a' $hdrOk "id=$($hdrPics[0].id) kind=$($hdrPics[0].kind) wrap=$($hdrPics[0].wrap)"

$word = Ensure-MsWord $path
$doc = Open-WordDoc $word $path
$caption = [string]$doc.ActiveWindow.Caption
$bodyN = [int]$doc.InlineShapes.Count
$sec = $doc.Sections.Item(1)
$hdr = $sec.Headers.Item(1)
$hdrN = [int]$hdr.Shapes.Count + [int]$hdr.Range.InlineShapes.Count
Add-Check 'docx-C6' (($caption -notmatch 'Repair') -and ($hdrN -ge 1)) "caption=$caption headerShapes=$hdrN"
Add-Check 'docx-C7' (($caption -notmatch 'Repair') -and ($bodyN -ge 1) -and ($hdrN -ge 1)) "caption=$caption bodyInline=$bodyN headerShapes=$hdrN"

Release-Com $hdr
Release-Com $sec
Close-WordDoc $doc
