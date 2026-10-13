# docx DrawingML 链剩余：docs/checklist-docx.md 的 Dhr1–Dhr6。
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

function Get-WordShapeHyperlink($doc) {
    $addr = ''
    $sh = $null
    $h = $null
    try {
        if ([int]$doc.Shapes.Count -ge 1) {
            $sh = $doc.Shapes.Item(1)
            $h = $sh.Hyperlink
            if ($null -ne $h) { $addr = [string]$h.Address }
        }
    } catch {
        Write-Log "Shapes.Hyperlink $($_.Exception.Message)"
    } finally {
        if ($null -ne $h) { Release-Com $h }
        if ($null -ne $sh) { Release-Com $sh }
    }
    return $addr
}

function Get-WordTextBoxPicHyperlink($doc) {
    $addr = ''
    foreach ($sh in @($doc.Shapes)) {
        $tf = $null
        $tr = $null
        $inlines = $null
        $pic = $null
        $h = $null
        try {
            $tf = $sh.TextFrame
            if ($null -ne $tf -and [int]$tf.HasText -ne 0) {
                $tr = $tf.TextRange
                $inlines = $tr.InlineShapes
                if ([int]$inlines.Count -ge 1) {
                    $pic = $inlines.Item(1)
                    $h = $pic.Hyperlink
                    if ($null -ne $h) { $addr = [string]$h.Address }
                }
            }
        } catch {
            Write-Log "TextBox pic Hyperlink $($_.Exception.Message)"
        } finally {
            if ($null -ne $h) { Release-Com $h }
            if ($null -ne $pic) { Release-Com $pic }
            if ($null -ne $inlines) { Release-Com $inlines }
            if ($null -ne $tr) { Release-Com $tr }
            if ($null -ne $tf) { Release-Com $tf }
            Release-Com $sh
        }
        if ($addr) { break }
    }
    return $addr
}

$pngB = Ensure-OoxpPngB

$src = Get-OutPath 'docx-drawing-hlink-remain-src.docx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = '框段' }
            @{ text = '形状段' }
            @{ text = '连接线段' }
        )
    }
} 'docx-dhr-create' | Out-Null

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-dhr-inspect-src'
$paraBox = [string]$srcTree.paragraphs[0].id
$paraShape = [string]$srcTree.paragraphs[1].id
$paraCxn = [string]$srcTree.paragraphs[2].id

$boxFile = Get-OutPath 'docx-drawing-hlink-remain-box.docx'
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
} 'docx-dhr-insert-box' | Out-Null

$treeBox = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $boxFile.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true; textBoxContent = $true }
} 'docx-dhr-inspect-box'
$innerPara = [string]$treeBox.paragraphs[0].textBoxes[0].paragraphs[0].id

$innerPicFile = Get-OutPath 'docx-drawing-hlink-remain-inner-pic.docx'
if (Test-Path $innerPicFile) { Remove-Item -Force $innerPicFile }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $boxFile.Replace('\', '/')
    output = $innerPicFile.Replace('\', '/')
    ops = @(@{
        op = 'insertPicture'
        id = $innerPara
        file = $pngB.Replace('\', '/')
        cx = 457200
        cy = 457200
    })
} 'docx-dhr-inner-pic' | Out-Null

$treeInner = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $innerPicFile.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true; textBoxContent = $true; pictures = $true }
} 'docx-dhr-inspect-inner-pic'
$innerPicId = [string]$treeInner.paragraphs[0].textBoxes[0].paragraphs[0].pictures[0].id

$innerHlink = Get-OutPath 'docx-drawing-hlink-remain-inner-hlink.docx'
if (Test-Path $innerHlink) { Remove-Item -Force $innerHlink }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $innerPicFile.Replace('\', '/')
    output = $innerHlink.Replace('\', '/')
    ops = @(@{
        op = 'insertHyperlink'
        id = $innerPicId
        url = 'https://example.com/inner'
    })
} 'docx-dhr-inner-hlink' | Out-Null

$treeInnerH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $innerHlink.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true; textBoxContent = $true; pictures = $true; hyperlinks = $true }
} 'docx-dhr-inspect-inner-hlink'
$innerUrl = [string]$treeInnerH.paragraphs[0].textBoxes[0].paragraphs[0].pictures[0].hyperlink.url
$listedInner = @($treeInnerH.hyperlinks | Where-Object { $_ })

$word = Ensure-MsWord $src
$doc1 = Open-WordDoc $word $innerHlink
$cap1 = [string]$doc1.ActiveWindow.Caption
$hl1 = Get-WordTextBoxPicHyperlink $doc1
Close-WordDoc $doc1
$dhr1ok = ($cap1 -notmatch 'Repair') -and
    ($innerUrl -eq 'https://example.com/inner') -and
    ($hl1 -match 'example.com/inner') -and
    ([int]$treeInnerH.hyperlinkCount -eq 0) -and
    ($listedInner.Count -eq 0)
Add-Check 'docx-Dhr1' $dhr1ok "caption=$cap1 addr=$hl1 inspect=$innerUrl count=$($treeInnerH.hyperlinkCount)"

$shapeFile = Get-OutPath 'docx-drawing-hlink-remain-shape.docx'
if (Test-Path $shapeFile) { Remove-Item -Force $shapeFile }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $shapeFile.Replace('\', '/')
    ops = @(@{ op = 'insertShape'; id = $paraShape; prst = 'ellipse' })
} 'docx-dhr-insert-shape' | Out-Null

$treeShape = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $shapeFile.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; shapes = $true }
} 'docx-dhr-inspect-shape'
$shapeId = [string]$treeShape.paragraphs[1].shapes[0].id

$shapeHlink = Get-OutPath 'docx-drawing-hlink-remain-shape-hlink.docx'
if (Test-Path $shapeHlink) { Remove-Item -Force $shapeHlink }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $shapeFile.Replace('\', '/')
    output = $shapeHlink.Replace('\', '/')
    ops = @(@{
        op = 'insertHyperlink'
        id = $shapeId
        url = 'https://example.com/shape'
    })
} 'docx-dhr-shape-hlink' | Out-Null

$treeShapeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $shapeHlink.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; shapes = $true }
} 'docx-dhr-inspect-shape-hlink'
$shapeUrl = [string]$treeShapeH.paragraphs[1].shapes[0].hyperlink.url
$doc2 = Open-WordDoc $word $shapeHlink
$cap2 = [string]$doc2.ActiveWindow.Caption
$hl2 = Get-WordShapeHyperlink $doc2
Close-WordDoc $doc2
$dhr2ok = ($cap2 -notmatch 'Repair') -and ($shapeUrl -eq 'https://example.com/shape')
Add-Check 'docx-Dhr2' $dhr2ok "caption=$cap2 url=$shapeUrl addr=$hl2"

$cxnFile = Get-OutPath 'docx-drawing-hlink-remain-cxn.docx'
if (Test-Path $cxnFile) { Remove-Item -Force $cxnFile }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $cxnFile.Replace('\', '/')
    ops = @(@{ op = 'insertConnector'; id = $paraCxn })
} 'docx-dhr-insert-cxn' | Out-Null

$treeCxn = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cxnFile.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; connectors = $true }
} 'docx-dhr-inspect-cxn'
$cxnId = [string]$treeCxn.paragraphs[2].connectors[0].id

$cxnHlink = Get-OutPath 'docx-drawing-hlink-remain-cxn-hlink.docx'
if (Test-Path $cxnHlink) { Remove-Item -Force $cxnHlink }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $cxnFile.Replace('\', '/')
    output = $cxnHlink.Replace('\', '/')
    ops = @(@{
        op = 'insertHyperlink'
        id = $cxnId
        url = 'https://example.com/cxn'
    })
} 'docx-dhr-cxn-hlink' | Out-Null

$treeCxnH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cxnHlink.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; connectors = $true }
} 'docx-dhr-inspect-cxn-hlink'
$cxnUrl = [string]$treeCxnH.paragraphs[2].connectors[0].hyperlink.url
$doc3 = Open-WordDoc $word $cxnHlink
$cap3 = [string]$doc3.ActiveWindow.Caption
Close-WordDoc $doc3
$dhr3ok = ($cap3 -notmatch 'Repair') -and ($cxnUrl -eq 'https://example.com/cxn')
Add-Check 'docx-Dhr3' $dhr3ok "caption=$cap3 url=$cxnUrl"

$createFile = Get-OutPath 'docx-drawing-hlink-remain-create.docx'
if (Test-Path $createFile) { Remove-Item -Force $createFile }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $createFile.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{
                text = '形'
                shapes = @(
                    @{
                        prst = 'ellipse'
                        hyperlink = @{ url = 'https://example.com/create-shape' }
                    }
                )
            }
        )
    }
} 'docx-dhr-create-shape' | Out-Null

$treeCreate = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $createFile.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; shapes = $true }
} 'docx-dhr-inspect-create'
$createUrl = [string]$treeCreate.paragraphs[0].shapes[0].hyperlink.url
$doc4 = Open-WordDoc $word $createFile
$cap4 = [string]$doc4.ActiveWindow.Caption
Close-WordDoc $doc4
$dhr4ok = ($cap4 -notmatch 'Repair') -and ($createUrl -eq 'https://example.com/create-shape')
Add-Check 'docx-Dhr4' $dhr4ok "caption=$cap4 url=$createUrl"

$delOut = Get-OutPath 'docx-drawing-hlink-remain-delete.docx'
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $shapeHlink.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteHyperlink'; id = $shapeId })
} 'docx-dhr-delete' | Out-Null

$treeDel = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; shapes = $true }
} 'docx-dhr-inspect-deleted'
$hDel = $treeDel.paragraphs[1].shapes[0].hyperlink
$shapeStill = [int]$treeDel.shapeCount
$doc5 = Open-WordDoc $word $delOut
$cap5 = [string]$doc5.ActiveWindow.Caption
$stillN = 0
try { $stillN = [int]$doc5.Shapes.Count } catch { $stillN = 0 }
Close-WordDoc $doc5
$dhr5ok = ($cap5 -notmatch 'Repair') -and ($null -eq $hDel) -and ($shapeStill -ge 1) -and ($stillN -ge 1)
Add-Check 'docx-Dhr5' $dhr5ok "caption=$cap5 hyperlink=$hDel shapes=$shapeStill com=$stillN"

$xlsx = Get-OutPath 'docx-drawing-hlink-remain-ole.xlsx'
if (Test-Path $xlsx) { Remove-Item -Force $xlsx }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    format = 'xlsx'
    workbook = @{
        sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) })
    }
} 'docx-dhr-ole-xlsx' | Out-Null

$oleHost = Get-OutPath 'docx-drawing-hlink-remain-ole-host.docx'
if (Test-Path $oleHost) { Remove-Item -Force $oleHost }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $oleHost.Replace('\', '/')
    ops = @(@{
        op = 'insertOle'
        id = $paraShape
        file = $xlsx.Replace('\', '/')
    })
} 'docx-dhr-insert-ole' | Out-Null

$treeOle = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $oleHost.Replace('\', '/'); format = 'docx'
    scope = @{ oleObjects = $true }
} 'docx-dhr-inspect-ole'
$oleId = [string]$treeOle.oleObjects[0].id

$failOle = Get-OutPath 'docx-drawing-hlink-remain-fail-ole.docx'
if (Test-Path $failOle) { Remove-Item -Force $failOle }
$rawOle = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $oleHost.Replace('\', '/')
    output = $failOle.Replace('\', '/')
    ops = @(@{
        op = 'insertHyperlink'
        id = $oleId
        url = 'https://example.com/ole'
    })
} 'docx-dhr-fail-ole'
try { $codeOle = ($rawOle.Stderr | ConvertFrom-Json).error.code } catch { $codeOle = $null }
$dhr6ok = ($rawOle.ExitCode -ne 0) -and ($codeOle -eq 'InvalidId') -and (-not (Test-Path $failOle))
Add-Check 'docx-Dhr6' $dhr6ok "ole=$codeOle wrote=$([bool](Test-Path $failOle))"
