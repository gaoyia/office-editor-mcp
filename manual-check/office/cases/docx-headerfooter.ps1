# docx 页眉页脚：docs/checklist-docx.md 的 HF1–HF5。
# 源文件由 Word 亲手加页眉页脚后另存，见 lib/fixtures.ps1 的 Ensure-DocxWordHeaderSource。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-DocxWordHeaderSource
$word = Ensure-MsWord $src

# HF1：无 scope 也有 headers/footers，容器不带 paragraphs。
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-hf-inspect-list'

$hasH = $tree0.PSObject.Properties.Name -contains 'headers'
$hasF = $tree0.PSObject.Properties.Name -contains 'footers'
$headers0 = @($tree0.headers)
$footers0 = @($tree0.footers)
$h0 = $headers0 | Where-Object { $_.type -eq 'default' } | Select-Object -First 1
$f0 = $footers0 | Where-Object { $_.type -eq 'default' } | Select-Object -First 1
$h0ok = $null -ne $h0 -and ($h0.id -like 'header:default:*') -and
        ($null -ne $h0.sections) -and ($null -ne $h0.paraCount) -and
        ($null -ne $h0.pictureCount) -and
        -not ($h0.PSObject.Properties.Name -contains 'paragraphs') -and
        -not ($h0.PSObject.Properties.Name -contains 'pictures')
$f0ok = $null -ne $f0 -and ($f0.id -like 'footer:default:*') -and
        -not ($f0.PSObject.Properties.Name -contains 'paragraphs')
Add-Check 'docx-HF1' ($hasH -and $hasF -and $h0ok -and $f0ok) "headers=$($headers0.Count) footers=$($footers0.Count) h0=$($h0.id)"

# HF2：展开后段落 id 带容器前缀。
$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true }
} 'docx-hf-inspect-expand'

$h1 = @($tree1.headers) | Where-Object { $_.type -eq 'default' } | Select-Object -First 1
$paras = @($h1.paragraphs)
$hid = $paras[0].id
$idOk = ($paras.Count -ge 1) -and ($hid -like 'header:default:*/para:*')
$textOk = ($paras[0].text -eq $S.headerSrc) -or ($paras[0].text.Trim() -eq $S.headerSrc)
Add-Check 'docx-HF2' ($idOk -and $textOk) "id=$hid text=$($paras[0].text)"

$bodyBefore = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-hf-body-before'
$bodyTexts = @($bodyBefore.paragraphs | ForEach-Object { $_.text })

$applied = Get-OutPath 'docx-hf-applied.docx'
Close-WordDocByPath $word $applied
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $applied.Replace('\', '/')
    ops = @(@{ op = 'setParaText'; id = $hid; text = $S.headerNew })
} 'docx-hf-apply' | Out-Null

$doc = Open-WordDoc $word $applied
$caption = [string]$doc.ActiveWindow.Caption
Add-Check 'docx-HF3' ($caption -notmatch 'Repair') $caption

$sec = $doc.Sections.Item(1)
$hdr = $sec.Headers.Item(1)
$ftr = $sec.Footers.Item(1)
$hdrText = ([string]$hdr.Range.Text).TrimEnd([char]13, [char]7, [char]10)
$ftrText = ([string]$ftr.Range.Text).TrimEnd([char]13, [char]7, [char]10)
$p1 = Get-ParaText $doc.Paragraphs.Item(1)

Add-Check 'docx-HF4' (($hdrText -eq $S.headerNew) -and ($p1 -eq $S.chapter1)) "hdr=$hdrText body=$p1"
Add-Check 'docx-HF5' ($ftrText -eq $S.footerSrc -or $ftrText.Trim() -eq $S.footerSrc) "footer=$ftrText"

# 正文段落文字应未改。
$bodyAfter = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $applied.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-hf-body-after'
$bodyTexts2 = @($bodyAfter.paragraphs | ForEach-Object { $_.text })
if (@(Compare-Object $bodyTexts $bodyTexts2).Count -ne 0) {
    Add-Check 'docx-HF4b' $false "body changed: before=$($bodyTexts -join '|') after=$($bodyTexts2 -join '|')"
}

Release-Com $hdr
Release-Com $ftr
Release-Com $sec
