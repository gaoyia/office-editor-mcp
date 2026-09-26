# pptx inspect：docs/checklist-pptx.md 的 I1–I5。
# 只调 ooxp，不开 PowerPoint。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$path = (Ensure-PptxCreate).Replace('\', '/')

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path; format = 'pptx'
    scope = @{ slide = 1; maxShapes = 200 }
} 'pptx-inspect'

Add-Check 'pptx-I1' (($tree.id -eq 'pptx:presentation') -and ($tree.slideCount -eq 2)) "id=$($tree.id) count=$($tree.slideCount)"

$slides = @($tree.slides)
$idOk = ($slides.Count -eq 2) -and ($slides[0].id -eq 'slide:1') -and ($slides[1].id -eq 'slide:2')
Add-Check 'pptx-I2' $idOk "$($slides[0].id),$($slides[1].id)"

$tree2 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path; format = 'pptx'
} 'pptx-inspect-slides'

$noShapes = -not ($tree2.slides[0].PSObject.Properties.Name -contains 'shapes')
Add-Check 'pptx-I3' ($noShapes -and ($tree2.slideCount -eq 2)) "noShapes=$noShapes count=$($tree2.slideCount)"

$shapes = @($tree.slides[0].shapes)
$idOk2 = ($shapes.Count -eq 2) -and $shapes[0].stable -and $shapes[1].stable -and
         ($shapes[0].id -like 'slide:1/shape:*') -and ($shapes[0].id -notlike 'slide:1/shape:index:*')
Add-Check 'pptx-I4' $idOk2 "$($shapes[0].id),$($shapes[1].id)"
Add-Check 'pptx-I5' (($shapes[0].text -eq $S.title) -and ($shapes[1].text -eq $S.hello)) "$($shapes[0].text)/$($shapes[1].text)"
