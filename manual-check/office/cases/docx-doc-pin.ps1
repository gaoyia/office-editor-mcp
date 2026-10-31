# 先插节，再改原来第 2 节的页边距。Word 16.0 不报修复。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'docx-doc-pin-src.docx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = '一'; sectionType = 'nextPage' }
            @{ text = '二' }
        )
    }
} 'docx-doc-pin-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 5 }
} 'docx-doc-pin-inspect'
$p1 = [string]$tree.paragraphs[0].id

$out = Get-OutPath 'docx-doc-pin.docx'
if (Test-Path $out) { Remove-Item -Force $out }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(
        @{ op = 'insertSection'; id = $p1; type = 'continuous' }
        @{ op = 'setPageMargin'; id = 'section:2'; top = 2222 }
    )
} 'docx-doc-pin-apply' | Out-Null

$word = Ensure-MsWord (Ensure-DocxCreate)
Close-WordDocByPath $word $out
$doc = Open-WordDoc $word $out
$cap = [string]$doc.ActiveWindow.Caption
$n = 0
$top2 = 0.0
$topLast = 0.0
$sections = $null
try {
    $sections = $doc.Sections
    $n = [int]$sections.Count
    if ($n -ge 2) {
        $s2 = $null
        $ps2 = $null
        $sl = $null
        $psl = $null
        try {
            $s2 = $sections.Item(2)
            $ps2 = $s2.PageSetup
            $top2 = [double]$ps2.TopMargin
            $sl = $sections.Item($n)
            $psl = $sl.PageSetup
            $topLast = [double]$psl.TopMargin
        } finally {
            Release-Com $psl
            Release-Com $sl
            Release-Com $ps2
            Release-Com $s2
        }
    }
} finally { Release-Com $sections }
Close-WordDoc $doc

$want = 2222 / 20.0
$ok = ($n -ge 3) -and ([Math]::Abs($topLast - $want) -lt 1) -and ([Math]::Abs($top2 - $want) -gt 1) -and ($cap -notmatch 'Repair')
Add-Check 'docx-doc-pin' $ok "sections=$n top2=$top2 topLast=$topLast caption=$cap"
