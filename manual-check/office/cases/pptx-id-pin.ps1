# 先插行再写原来第 3 行；先删第一份母版再改原来第二份母版上的形状。
# PowerPoint 16.0 不报修复。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'pptx-id-pin-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    presentation = @{
        slides = @(
            @{ shapes = @(@{ text = '第一页' }) },
            @{ shapes = @(@{ text = '第二页' }) }
        )
    }
} 'pptx-id-pin-create' | Out-Null

$mid = Get-OutPath 'pptx-id-pin-mid.pptx'
if (Test-Path $mid) { Remove-Item -Force $mid }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $mid.Replace('\', '/')
    ops = @(@{
        op = 'insertTable'; id = 'slide:1'; rows = 3; cols = 3
        x = 914400; y = 1828800; cx = 5486400; cy = 3200400
        cells = @(@('a1','a2','a3'), @('b1','b2','b3'), @('c1','c2','c3'))
    })
} 'pptx-id-pin-table' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $mid.Replace('\', '/'); scope = @{ tables = $true }
} 'pptx-id-pin-inspect'
$tableId = [string]$tree.slides[0].tables[0].id

$out = Get-OutPath 'pptx-id-pin.pptx'
if (Test-Path $out) { Remove-Item -Force $out }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $mid.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(
        @{ op = 'insertRow'; id = "$tableId/row:1" },
        @{ op = 'setTableCellText'; id = "$tableId/r:3/c:1"; text = '原三' }
    )
} 'pptx-id-pin-row' | Out-Null

$ppt = Ensure-MsPpt $out
Close-PptPresByPath $ppt $out
$pres = Open-PptPres $ppt $out
$cap = ''
try { $cap = [string]$pres.Windows.Item(1).Caption } catch { $cap = '' }
$slide = $pres.Slides.Item(1)
$cell3 = ''
$cell4 = ''
foreach ($i in 1..$slide.Shapes.Count) {
    $sh = $slide.Shapes.Item($i)
    if (-not $sh.HasTable) { continue }
    $cell3 = [string]$sh.Table.Cell(3, 1).Shape.TextFrame.TextRange.Text
    $cell4 = [string]$sh.Table.Cell(4, 1).Shape.TextFrame.TextRange.Text
}
$noRepair = $cap -notmatch 'Repair'
$rowOk = ($cell3.Trim() -eq 'b1') -and ($cell4.Trim() -eq '原三')
Add-Check 'pptx-id-pin-row' ($noRepair -and $rowOk) "repair=$noRepair c3=$cell3 c4=$cell4"
Close-PptPres $pres

$cloned = Get-OutPath 'pptx-id-pin-master.pptx'
if (Test-Path $cloned) { Remove-Item -Force $cloned }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $cloned.Replace('\', '/')
    ops = @(
        @{ op = 'insertMaster'; id = 'pptx:presentation' },
        @{
            op = 'insertShape'; id = 'master:2'; text = '母版二'
            x = 914400; y = 914400; cx = 1828800; cy = 914400
        }
    )
} 'pptx-id-pin-master-insert' | Out-Null

$masters = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cloned.Replace('\', '/'); scope = @{ masters = $true }
} 'pptx-id-pin-master-inspect'
$shapeId = ''
foreach ($shape in @($masters.masters[1].shapes)) {
    if ([string]$shape.text -eq '母版二') { $shapeId = [string]$shape.id }
}

$moved = Get-OutPath 'pptx-id-pin-moved.pptx'
if (Test-Path $moved) { Remove-Item -Force $moved }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $cloned.Replace('\', '/')
    output = $moved.Replace('\', '/')
    ops = @(
        @{ op = 'setSlideLayout'; id = 'slide:1'; layout = 'master:2/layout:1' },
        @{ op = 'setSlideLayout'; id = 'slide:2'; layout = 'master:2/layout:1' }
    )
} 'pptx-id-pin-layout' | Out-Null

$masterOut = Get-OutPath 'pptx-id-pin-master-out.pptx'
if (Test-Path $masterOut) { Remove-Item -Force $masterOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $moved.Replace('\', '/')
    output = $masterOut.Replace('\', '/')
    ops = @(
        @{ op = 'deleteMaster'; id = 'master:1' },
        @{ op = 'setShapeText'; id = $shapeId; text = '原二母版' }
    )
} 'pptx-id-pin-master-edit' | Out-Null

$ppt = Ensure-MsPpt $masterOut
Close-PptPresByPath $ppt $masterOut
$pres2 = Open-PptPres $ppt $masterOut
$cap2 = ''
try { $cap2 = [string]$pres2.Windows.Item(1).Caption } catch { $cap2 = '' }
$found = $false
$master = $pres2.SlideMaster
foreach ($i in 1..$master.Shapes.Count) {
    $text = ''
    try { $text = [string]$master.Shapes.Item($i).TextFrame.TextRange.Text } catch { $text = '' }
    if ($text.Trim() -eq '原二母版') { $found = $true }
}
$noRepair2 = $cap2 -notmatch 'Repair'
Add-Check 'pptx-id-pin-master' ($noRepair2 -and $found) "repair=$noRepair2 found=$found"
Close-PptPres $pres2
