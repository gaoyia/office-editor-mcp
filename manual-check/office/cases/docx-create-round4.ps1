# docx create 第四轮：docs/checklist-docx.md 的 Cy1–Cy8。
# 样式、表外观、形状、连接线、拼音、题注 / 索引 / 书目、嵌入字体、DrawingML 链。
# 不算排版引擎。不要和 Cd、Cs、Cf、C、Pq 撞。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function Invoke-OoxpExpectFail($obj, [string]$name) {
    $path = Join-Path $global:OoxpReqDir "$name.json"
    Write-JsonFile $path $obj
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $global:OoxpExe
    $psi.Arguments = "--input `"$path`""
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = [System.Text.Encoding]::UTF8
    $psi.StandardErrorEncoding = [System.Text.Encoding]::UTF8
    $proc = [System.Diagnostics.Process]::Start($psi)
    $stdout = $proc.StandardOutput.ReadToEnd().Trim()
    $stderr = $proc.StandardError.ReadToEnd().Trim()
    $proc.WaitForExit()
    $code = $proc.ExitCode
    $proc.Dispose()
    Write-Log "ooxp $name 退出码 $code stderr=$stderr"
    return @{ ExitCode = $code; Stderr = $stderr; Stdout = $stdout }
}

function Get-OoxpErrorCode($raw) {
    try { return ($raw.Stderr | ConvertFrom-Json).error.code } catch { return $null }
}

$path = Ensure-DocxCreateRound4 -Force
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'docx'
    scope = @{
        from = 1; maxParas = 30
        pictures = $true; shapes = $true; connectors = $true; tables = $true
        styles = $true; rubies = $true; captions = $true; indexes = $true
        indexEntries = $true; sources = $true; citations = $true
        bibliographies = $true; crossRefs = $true; editRanges = $true
        embeddedFonts = $true; docParts = $true
    }
} 'docx-create-round4-inspect'

$style = @($tree.styles) | Where-Object { [string]$_.styleId -eq 'ooxpTbl' } | Select-Object -First 1
$tbl = @($tree.tables) | Select-Object -First 1
$fill = $null
try { $fill = [string]$tbl.rows[0].cells[0].fill } catch {}
Add-Check 'docx-Cy1' (
    ([int]$tree.styleCount -ge 5) -and
    ($style) -and
    ([string]$style.type -eq 'table') -and
    ([int]$tree.tableCount -eq 1) -and
    ([string]$tbl.style -eq 'ooxpTbl') -and
    ([bool]$tbl.look.firstRow -eq $true) -and
    ($fill -eq 'FF0000')
) "styleCount=$($tree.styleCount) tblStyle=$($tbl.style) fill=$fill"

$shape = $null
$cxn = $null
$p2 = @($tree.paragraphs) | Select-Object -Index 1
if ($p2 -and $p2.shapes) { $shape = @($p2.shapes) | Select-Object -First 1 }
if ($p2 -and $p2.connectors) { $cxn = @($p2.connectors) | Select-Object -First 1 }
Add-Check 'docx-Cy2' (
    ([int]$tree.shapeCount -eq 1) -and
    ([string]$shape.prst -eq 'ellipse') -and
    ([int]$tree.connectorCount -eq 1) -and
    ([string]$cxn.prst -eq 'straightConnector1')
) "shape=$($shape.prst) cxn=$($cxn.prst)"

$ruby = @($tree.rubies) | Select-Object -First 1
$er = @($tree.editRanges) | Select-Object -First 1
Add-Check 'docx-Cy3' (
    ([int]$tree.rubyCount -eq 1) -and
    ([string]$ruby.base -eq '汉字') -and
    ([string]$ruby.rt -eq 'hànzì') -and
    ([int]$tree.indexEntryCount -eq 1) -and
    ([int]$tree.citationCount -eq 1) -and
    ([int]$tree.crossRefCount -eq 1) -and
    ([int]$tree.editRangeCount -eq 1) -and
    ([string]$er.edGrp -eq 'everyone')
) "ruby=$($ruby.base)/$($ruby.rt) edGrp=$($er.edGrp)"

Add-Check 'docx-Cy4' (
    ([int]$tree.captionCount -eq 1) -and
    ([int]$tree.indexCount -eq 1) -and
    ([int]$tree.bibliographyCount -eq 1) -and
    ([int]$tree.sourceCount -eq 1) -and
    ([string]$tree.bibliographyStyle.styleName -eq 'MLA') -and
    ([int]$tree.docPartCount -eq 1)
) "caption=$($tree.captionCount) index=$($tree.indexCount) biblio=$($tree.bibliographyCount) style=$($tree.bibliographyStyle.styleName)"

$fontN = [int]$tree.embeddedFontCount
$fontName = $null
try { $fontName = [string]$tree.embeddedFonts[0].name } catch {}
Add-Check 'docx-Cy5' (
    ($fontN -eq 1) -and
    ($fontName -eq 'Calibri') -and
    ([bool]$tree.fontEmbed.trueType -eq $true) -and
    ([string]$tree.eastAsianSettings.characterSpacingControl -eq 'compressPunctuation')
) "fontCount=$fontN name=$fontName trueType=$($tree.fontEmbed.trueType)"

$pic = $null
$p2 = @($tree.paragraphs) | Select-Object -Index 1
if ($p2 -and $p2.pictures) { $pic = @($p2.pictures) | Select-Object -First 1 }
$picUrl = if ($pic) { [string]$pic.hyperlink.url } else { '' }
Add-Check 'docx-Cy6' (
    ([int]$tree.pictureCount -eq 1) -and
    ([int]$tree.hyperlinkCount -eq 0) -and
    ($picUrl -eq 'https://example.com')
) "pictureCount=$($tree.pictureCount) hyperlinkCount=$($tree.hyperlinkCount) url=$picUrl"

$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'create'
    path = (Join-Path $global:OoxpOutDir 'docx-create-round4-wp.docx').Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(@{ text = 'a' })
        protection = @{ password = 'secret' }
    }
} 'docx-create-round4-write-password'
Add-Check 'docx-Cy7' (
    ($fail.ExitCode -ne 0) -and
    ((Get-OoxpErrorCode $fail) -eq 'NotImplemented')
) "code=$(Get-OoxpErrorCode $fail)"

$seed = Ensure-DocxCreate
$word = Ensure-MsWord $seed
try { $word.AskToUpdateLinks = $false } catch {}
try { $word.Options.UpdateLinksAtOpen = $false } catch {}
$doc = Open-WordDoc $word $path
$caption = [string]$doc.ActiveWindow.Caption
$shapeN = 0
$tblFill = $null
$picHl = $null
try {
    $shapeN = [int]$doc.Shapes.Count
} catch {}
try {
    $t = $doc.Tables.Item(1)
    try {
        $cell = $t.Cell(1, 1)
        try {
            $shd = $cell.Shading
            try { $tblFill = [int]$shd.BackgroundPatternColor } finally { Release-Com $shd }
        } finally { Release-Com $cell }
    } finally { Release-Com $t }
} catch {}
try {
    $ils = $doc.InlineShapes
    try {
        if ([int]$ils.Count -ge 1) {
            $il = $ils.Item(1)
            try {
                $hl = $null
                try { $hl = $il.Hyperlink } catch {}
                if ($hl) {
                    try { $picHl = [string]$hl.Address } finally { Release-Com $hl }
                }
            } finally { Release-Com $il }
        }
    } finally { Release-Com $ils }
} catch {}
$biblio = $null
try { $biblio = [string]$doc.Bibliography.BibliographyStyle } catch {}
Add-Check 'docx-Cy8' (
    ($caption -notmatch 'Repair') -and
    ($shapeN -ge 2) -and
    ($tblFill -eq 255) -and
    ($picHl -match '^https://example\.com/?$') -and
    ($biblio -eq 'MLA')
) "caption=$caption shapes=$shapeN fill=$tblFill picHl=$picHl biblio=$biblio"
Close-WordDoc $doc
