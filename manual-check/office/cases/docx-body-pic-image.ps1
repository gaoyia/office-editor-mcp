# 正文嵌入不透明 inline PNG（extent 像素=源像素）矩阵。金样 Word 16.0 PDF→96dpi。
# px=整页 MaxDelta=0；layout=墨迹盒。不套 PPT bicubic。浮动/裁剪/透明/混排/缩放 NI。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'office-com.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$pngScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pdf-to-png.ps1'))
$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$repo = $global:OoxpRepoRoot

function New-Pic($tag, $png, $cx, $cy, $left, $margin) {
    $src = Get-OutPath ("docx-pic-{0}-src.docx" -f $tag)
    if (Test-Path $src) { Remove-Item -Force $src }
    Invoke-OoxpRequest @{ schemaVersion = 1; action = 'create'; path = $src.Replace('\', '/'); document = @{ paragraphs = @(, @{ text = '' }) } } ("docx-pic-{0}-create" -f $tag) | Out-Null
    $tree = Invoke-OoxpTree @{ schemaVersion = 1; action = 'inspect'; path = $src.Replace('\', '/'); format = 'docx'; scope = @{ from = 1; maxParas = 4; pictures = $true } } ("docx-pic-{0}-ins" -f $tag)
    $paraId = [string]$tree.paragraphs[0].id
    $ops = @()
    if ($margin) { $ops += , @{ op = 'setPageMargin'; id = 'section:1'; top = [int]$margin; bottom = [int]$margin; left = [int]$margin; right = [int]$margin } }
    if ($left) { $ops += , @{ op = 'setParaFormat'; id = $paraId; left = [int]$left } }
    $file = (Join-Path $repo ('manual-check\office\out\' + $png)).Replace('\', '/')
    $ops += , @{ op = 'insertPicture'; id = $paraId; file = $file; cx = [int64]$cx; cy = [int64]$cy }
    $deck = Get-OutPath ("docx-pic-{0}.docx" -f $tag)
    if (Test-Path $deck) { Remove-Item -Force $deck }
    Invoke-OoxpRequest @{ schemaVersion = 1; action = 'apply'; path = $src.Replace('\', '/'); output = $deck.Replace('\', '/'); ops = $ops } ("docx-pic-{0}-apply" -f $tag) | Out-Null
    return $deck
}

$specs = @(
    @{ tag = 'r64'; png = 'pic-64x32-red.png'; cx = 609600; cy = 304800; left = $null; margin = $null }
    @{ tag = 'b96'; png = 'pic-96x96-blue.png'; cx = 914400; cy = 914400; left = $null; margin = $null }
    @{ tag = 'g200'; png = 'pic-200x50-green.png'; cx = 1905000; cy = 476250; left = $null; margin = $null }
    @{ tag = 'r64-indent'; png = 'pic-64x32-red.png'; cx = 609600; cy = 304800; left = 720; margin = $null }
    @{ tag = 'r64-margin'; png = 'pic-64x32-red.png'; cx = 609600; cy = 304800; left = $null; margin = 2160 }
)

$blocked = $false
foreach ($spec in $specs) {
    $tag = $spec.tag
    if ($blocked) { Add-Result ("docx-pic-{0}-pdf" -f $tag) 'FAIL' 'blocked'; continue }
    $doc = New-Pic $tag $spec.png $spec.cx $spec.cy $spec.left $spec.margin
    $lib = Get-OutPath ("docx-pic-{0}-lib.png" -f $tag)
    $pdf = Get-OutPath ("docx-pic-{0}.pdf" -f $tag)
    $gold = Get-OutPath ("docx-pic-{0}-gold.png" -f $tag)
    $diff = Get-OutPath ("docx-pic-{0}-diff.png" -f $tag)
    foreach ($p in @($lib, $pdf, $gold, $diff)) { if (Test-Path $p) { Remove-Item -Force $p } }
    Invoke-OoxpRequest @{ schemaVersion = 1; action = 'exportPageImage'; path = $doc.Replace('\', '/'); id = 'page:1'; output = $lib.Replace('\', '/') } ("docx-pic-{0}-lib" -f $tag) | Out-Null
    $exp = Export-WordPdfBounded -DocPath $doc -PdfPath $pdf -TimeoutSec 60
    if (-not $exp.Ok) { $blocked = [bool]$exp.Blocked; Add-Result ("docx-pic-{0}-pdf" -f $tag) 'FAIL' $exp.Detail; continue }
    Add-Result ("docx-pic-{0}-pdf" -f $tag) 'PASS' 'Word PDF'
    & $pngScript -Pdf $pdf -Out $gold -Dpi 96 -Page 1 | Out-Null
    $line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
    $pxOk = ($LASTEXITCODE -ne 2) -and ("$line" -match 'differ=0\b') -and ("$line" -match 'maxDelta=0\b')
    Add-Result ("docx-pic-{0}-px" -f $tag) $(if ($pxOk) { 'PASS' } else { 'FAIL' }) "$line"
    Write-Host ("RESULT {0} px={1} {2}" -f $tag, $(if ($pxOk) { 'PASS' } else { 'FAIL' }), $line)
}
