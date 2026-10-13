# docx 图与形状超链接：docs/checklist-docx.md 的 Dh1–Dh6。
# 用库 create 产物再 apply。不打开浏览器。

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

function Get-WordDrawingHyperlink($doc) {
    $info = @{
        Address    = ''
        SubAddress = ''
        ScreenTip  = ''
        InlineN    = 0
        ShapeN     = 0
    }
    try { $info.InlineN = [int]$doc.InlineShapes.Count } catch { $info.InlineN = 0 }
    try { $info.ShapeN = [int]$doc.Shapes.Count } catch { $info.ShapeN = 0 }
    $readOne = {
        param($sh)
        try {
            $h = $sh.Hyperlink
            if ($null -ne $h) {
                try { $script:addr = [string]$h.Address } catch { }
                try { $script:sub = [string]$h.SubAddress } catch { }
                try { $script:tip = [string]$h.ScreenTip } catch { }
                Release-Com $h
            }
        } catch {
            Write-Log "Hyperlink $($_.Exception.Message)"
        }
    }
    $script:addr = ''
    $script:sub = ''
    $script:tip = ''
    if ($info.InlineN -ge 1) {
        $sh = $null
        try {
            $sh = $doc.InlineShapes.Item(1)
            & $readOne $sh
        } catch {
            Write-Log "InlineShapes.Item $($_.Exception.Message)"
        } finally {
            if ($null -ne $sh) { Release-Com $sh }
        }
    }
    if ((-not $script:addr) -and (-not $script:sub) -and ($info.ShapeN -ge 1)) {
        $sh = $null
        try {
            $sh = $doc.Shapes.Item(1)
            & $readOne $sh
        } catch {
            Write-Log "Shapes.Item $($_.Exception.Message)"
        } finally {
            if ($null -ne $sh) { Release-Com $sh }
        }
    }
    $info.Address = $script:addr
    $info.SubAddress = $script:sub
    $info.ScreenTip = $script:tip
    return [pscustomobject]$info
}

function Get-HyperlinkRelXml([string]$docxPath) {
    $xml = Get-ZipPartText $docxPath 'word/_rels/document.xml.rels'
    if ([string]::IsNullOrEmpty($xml)) { return '' }
    $m = [regex]::Match($xml, '<Relationship\b[^>]*Type="[^"]*hyperlink"[^>]*(?:/>|>)')
    if ($m.Success) { return $m.Value }
    return ''
}

$pngB = Ensure-OoxpPngB

$src = Get-OutPath 'docx-drawing-hlink-src.docx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = '图段' }
            @{ text = '框段' }
            @{ text = '书签段' }
        )
    }
} 'docx-dh-create' | Out-Null

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-dh-inspect-src'
$paraPic = [string]$srcTree.paragraphs[0].id
$paraBox = [string]$srcTree.paragraphs[1].id
$paraBm = [string]$srcTree.paragraphs[2].id

$picFile = Get-OutPath 'docx-drawing-hlink-pic.docx'
if (Test-Path $picFile) { Remove-Item -Force $picFile }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $picFile.Replace('\', '/')
    ops = @(@{
        op = 'insertPicture'
        id = $paraPic
        file = $pngB.Replace('\', '/')
        cx = 914400
        cy = 914400
    })
} 'docx-dh-insert-pic' | Out-Null

$treePic = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $picFile.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; pictures = $true; hyperlinks = $true }
} 'docx-dh-inspect-pic'
$picId = [string]$treePic.paragraphs[0].pictures[0].id

$hlinkOut = Get-OutPath 'docx-drawing-hlink-insert.docx'
if (Test-Path $hlinkOut) { Remove-Item -Force $hlinkOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $picFile.Replace('\', '/')
    output = $hlinkOut.Replace('\', '/')
    ops = @(@{
        op = 'insertHyperlink'
        id = $picId
        url = 'https://example.com/dh'
        tooltip = '图链'
    })
} 'docx-dh-insert-hlink' | Out-Null

$treeIns = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hlinkOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; pictures = $true; hyperlinks = $true }
} 'docx-dh-inspect-inserted'
$insUrl = [string]$treeIns.paragraphs[0].pictures[0].hyperlink.url
$listedLinks = @($treeIns.hyperlinks | Where-Object { $_ })

$word = Ensure-MsWord $src
$doc1 = Open-WordDoc $word $hlinkOut
$cap1 = [string]$doc1.ActiveWindow.Caption
$hl1 = Get-WordDrawingHyperlink $doc1
Close-WordDoc $doc1
$dh1ok = ($cap1 -notmatch 'Repair') -and
    ($insUrl -eq 'https://example.com/dh') -and
    ($hl1.Address -match 'example.com/dh') -and
    ([int]$treeIns.hyperlinkCount -eq 0) -and
    ($listedLinks.Count -eq 0)
Add-Check 'docx-Dh1' $dh1ok "caption=$cap1 addr=$($hl1.Address) inspect=$insUrl count=$($treeIns.hyperlinkCount)"

$setOut = Get-OutPath 'docx-drawing-hlink-set.docx'
if (Test-Path $setOut) { Remove-Item -Force $setOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hlinkOut.Replace('\', '/')
    output = $setOut.Replace('\', '/')
    ops = @(@{
        op = 'setHyperlink'
        id = $picId
        url = 'https://example.org/dh2'
    })
} 'docx-dh-set' | Out-Null

$doc2 = Open-WordDoc $word $setOut
$cap2 = [string]$doc2.ActiveWindow.Caption
$hl2 = Get-WordDrawingHyperlink $doc2
Close-WordDoc $doc2
$dh2ok = ($cap2 -notmatch 'Repair') -and ($hl2.Address -match 'example.org/dh2')
Add-Check 'docx-Dh2' $dh2ok "caption=$cap2 addr=$($hl2.Address)"

$delOut = Get-OutPath 'docx-drawing-hlink-delete.docx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $setOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteHyperlink'; id = $picId })
} 'docx-dh-delete' | Out-Null

$treeDel = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; pictures = $true }
} 'docx-dh-inspect-deleted'
$hDel = $treeDel.paragraphs[0].pictures[0].hyperlink
$picStill = [int]$treeDel.pictureCount
$doc3 = Open-WordDoc $word $delOut
$cap3 = [string]$doc3.ActiveWindow.Caption
$stillN = 0
try { $stillN = [int]$doc3.InlineShapes.Count + [int]$doc3.Shapes.Count } catch { $stillN = 0 }
Close-WordDoc $doc3
$dh3ok = ($cap3 -notmatch 'Repair') -and ($null -eq $hDel) -and ($picStill -ge 1) -and ($stillN -ge 1)
Add-Check 'docx-Dh3' $dh3ok "caption=$cap3 hyperlink=$hDel pics=$picStill com=$stillN"

$boxFile = Get-OutPath 'docx-drawing-hlink-box.docx'
if (Test-Path $boxFile) { Remove-Item -Force $boxFile }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $boxFile.Replace('\', '/')
    ops = @(@{
        op = 'insertTextBox'
        id = $paraBox
        text = '框内'
        cx = 1828800
        cy = 914400
    })
} 'docx-dh-insert-box' | Out-Null

$treeBox = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $boxFile.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true }
} 'docx-dh-inspect-box'
$boxId = [string]$treeBox.paragraphs[1].textBoxes[0].id

$boxHlink = Get-OutPath 'docx-drawing-hlink-box-link.docx'
if (Test-Path $boxHlink) { Remove-Item -Force $boxHlink }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $boxFile.Replace('\', '/')
    output = $boxHlink.Replace('\', '/')
    ops = @(@{
        op = 'insertHyperlink'
        id = $boxId
        url = 'https://example.net/box'
    })
} 'docx-dh-box-hlink' | Out-Null

$treeBoxH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $boxHlink.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true; hyperlinks = $true }
} 'docx-dh-inspect-box-hlink'
$boxUrl = [string]$treeBoxH.paragraphs[1].textBoxes[0].hyperlink.url
$doc4 = Open-WordDoc $word $boxHlink
$cap4 = [string]$doc4.ActiveWindow.Caption
Close-WordDoc $doc4
$dh4ok = ($cap4 -notmatch 'Repair') -and ($boxUrl -eq 'https://example.net/box')
Add-Check 'docx-Dh4' $dh4ok "caption=$cap4 url=$boxUrl"

$bmPic = Get-OutPath 'docx-drawing-hlink-bm-pic.docx'
if (Test-Path $bmPic) { Remove-Item -Force $bmPic }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $bmPic.Replace('\', '/')
    ops = @(
        @{
            op = 'insertPicture'
            id = $paraPic
            file = $pngB.Replace('\', '/')
            cx = 914400
            cy = 914400
        }
        @{ op = 'insertBookmark'; id = $paraBm; name = 'SrcBm' }
    )
} 'docx-dh-bm-pic' | Out-Null

$treeBmPic = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $bmPic.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; pictures = $true }
} 'docx-dh-inspect-bm-pic'
$bmPicId = [string]$treeBmPic.paragraphs[0].pictures[0].id

$anchorOut = Get-OutPath 'docx-drawing-hlink-anchor.docx'
if (Test-Path $anchorOut) { Remove-Item -Force $anchorOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $bmPic.Replace('\', '/')
    output = $anchorOut.Replace('\', '/')
    ops = @(@{
        op = 'insertHyperlink'
        id = $bmPicId
        anchor = 'SrcBm'
    })
} 'docx-dh-anchor' | Out-Null

$treeAn = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $anchorOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; pictures = $true }
} 'docx-dh-inspect-anchor'
$kindAn = [string]$treeAn.paragraphs[0].pictures[0].hyperlink.kind
$anchorAn = [string]$treeAn.paragraphs[0].pictures[0].hyperlink.anchor
$relXml = Get-HyperlinkRelXml $anchorOut
$relOk = ($relXml -match 'Target="#SrcBm"') -and ($relXml -notmatch 'TargetMode')
$doc5 = Open-WordDoc $word $anchorOut
$cap5 = [string]$doc5.ActiveWindow.Caption
Close-WordDoc $doc5
$dh5ok = ($cap5 -notmatch 'Repair') -and ($kindAn -eq 'anchor') -and ($anchorAn -eq 'SrcBm') -and $relOk
Add-Check 'docx-Dh5' $dh5ok "caption=$cap5 kind=$kindAn anchor=$anchorAn rel=$relXml"

$failText = Get-OutPath 'docx-drawing-hlink-fail-text.docx'
if (Test-Path $failText) { Remove-Item -Force $failText }
$rawText = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $picFile.Replace('\', '/')
    output = $failText.Replace('\', '/')
    ops = @(@{
        op = 'insertHyperlink'
        id = $picId
        text = '不要'
        url = 'https://example.com'
    })
} 'docx-dh-fail-text'
try { $codeText = ($rawText.Stderr | ConvertFrom-Json).error.code } catch { $codeText = $null }

$dh6ok = ($rawText.ExitCode -ne 0) -and ($codeText -eq 'InvalidField') -and (-not (Test-Path $failText))
Add-Check 'docx-Dh6' $dh6ok "text=$codeText wroteText=$([bool](Test-Path $failText))"
