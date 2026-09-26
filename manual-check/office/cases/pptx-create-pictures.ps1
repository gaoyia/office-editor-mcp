# pptx create 带图：docs/checklist-pptx.md 的 C5–C7。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$path = Ensure-PptxCreatePictures
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'pptx'
    scope = @{ pictures = $true }
} 'pptx-create-pictures-inspect'

$pics = @($tree.slides[0].pictures)
$first = $pics | Select-Object -First 1
$second = $pics | Select-Object -Skip 1 -First 1
Add-Check 'pptx-C5' (($null -ne $first) -and ($first.id -eq 'slide:1/pic:3')) "id=$($first.id) n=$($pics.Count)"
Add-Check 'pptx-C6' (($pics.Count -eq 2) -and ($null -ne $second) -and ($second.id -like 'slide:1/pic:*')) "id=$($second.id) n=$($pics.Count)"

$ppt = Ensure-MsPpt $path
Close-PptPresByPath $ppt $path
$pres = Open-PptPres $ppt $path
$cap = Get-PptCaption $pres
$s1 = Get-PptSlide $pres 1
$n = 0
$shapes = $s1.Shapes
try {
    for ($i = 1; $i -le $shapes.Count; $i++) {
        $sh = $shapes.Item($i)
        try {
            if ([int]$sh.Type -eq 13) { $n++ }
        } finally { Release-Com $sh }
    }
} finally { Release-Com $shapes }
Release-Com $s1
Close-PptPres $pres
Add-Check 'pptx-C7' (($cap -notmatch 'Repair') -and ($n -ge 2)) "caption=$cap pictures=$n"
