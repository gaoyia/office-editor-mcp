# 先删第一个书签再改原来第三个，先删第一个公式再改原来第二个。Word 16.0 不报修复。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'docx-inline-pin-src.docx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{ paragraphs = @(@{ text = '宿主' }) }
} 'docx-inline-pin-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 5 }
} 'docx-inline-pin-inspect'
$para = [string]$tree.paragraphs[0].id

$made = Get-OutPath 'docx-inline-pin-made.docx'
if (Test-Path $made) { Remove-Item -Force $made }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $made.Replace('\', '/')
    ops = @(
        @{ op = 'insertBookmark'; id = $para; name = 'Cc' }
        @{ op = 'insertBookmark'; id = $para; name = 'Bb' }
        @{ op = 'insertBookmark'; id = $para; name = 'Aa' }
        @{ op = 'insertMath'; id = $para; text = 'aaa' }
        @{ op = 'insertMath'; id = $para; text = 'bbb' }
        @{ op = 'insertMath'; id = $para; text = 'ccc' }
    )
} 'docx-inline-pin-make' | Out-Null

$out = Get-OutPath 'docx-inline-pin.docx'
if (Test-Path $out) { Remove-Item -Force $out }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $made.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(
        @{ op = 'deleteBookmark'; id = 'bookmark:1' }
        @{ op = 'setBookmark'; id = 'bookmark:3'; name = 'Cc2' }
        @{ op = 'deleteMath'; id = 'math:1' }
        @{ op = 'setMath'; id = 'math:2'; text = 'BBB' }
    )
} 'docx-inline-pin-apply' | Out-Null

$word = Ensure-MsWord (Ensure-DocxCreate)
Close-WordDocByPath $word $out
$doc = Open-WordDoc $word $out
$cap = [string]$doc.ActiveWindow.Caption
$hasAa = $false
$hasBb = $false
$hasCc2 = $false
$bms = $null
try {
    $bms = $doc.Bookmarks
    $hasAa = [bool]$bms.Exists('Aa')
    $hasBb = [bool]$bms.Exists('Bb')
    $hasCc2 = [bool]$bms.Exists('Cc2')
} finally { Release-Com $bms }

$mathN = 0
$omaths = $null
try {
    $omaths = $doc.OMaths
    $mathN = [int]$omaths.Count
} finally { Release-Com $omaths }

Close-WordDoc $doc

$checked = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $out.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 5; maths = $true }
} 'docx-inline-pin-inspect-out'
$m1 = [string]$checked.maths[0].text
$m2 = [string]$checked.maths[1].text
$ok = (-not $hasAa) -and $hasBb -and $hasCc2 -and ($mathN -ge 1) -and ($m1 -eq 'BBB') -and ($m2 -eq 'ccc') -and ($cap -notmatch 'Repair')
Add-Check 'docx-inline-pin' $ok "aa=$hasAa bb=$hasBb cc2=$hasCc2 maths=$mathN m1=$m1 m2=$m2 caption=$cap"
