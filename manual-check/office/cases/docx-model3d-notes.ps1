# docx 脚注与批注里的行内 3D。docs/checklist-docx.md 的 D3n1。
# Word 16.0 只另存 wp:inline。浮动不在本脚本里写。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$glb = Ensure-OoxpGlb
$glbFile = $glb.Replace('\', '/')

$src = Get-OutPath 'docx-model3d-notes-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{ paragraphs = @(@{ text = 'note-model-host' }) }
} 'docx-model3d-notes-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 4 }
} 'docx-model3d-notes-src'
$para = [string]$tree0.paragraphs[0].id

$hosted = Get-OutPath 'docx-model3d-notes-hosted.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $hosted.Replace('\', '/')
    ops = @(
        @{ op = 'insertFootnote'; id = $para; text = '注' }
        @{ op = 'insertComment'; start = $para; end = $para; text = '批' }
    )
} 'docx-model3d-notes-host' | Out-Null

$treeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hosted.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 4; notes = $true; comments = $true }
} 'docx-model3d-notes-hosted'
$foot = [string]$treeH.footnotes[0].paragraphs[0].id
$comment = [string]$treeH.comments[0].paragraphs[0].id

$inserted = Get-OutPath 'docx-model3d-notes-inserted.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hosted.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(
        @{ op = 'insertModel3d'; id = $foot; file = $glbFile }
        @{ op = 'insertModel3d'; id = $comment; file = $glbFile }
    )
} 'docx-model3d-notes-insert' | Out-Null

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'docx'
    scope = @{ model3ds = $true; pictures = $true }
} 'docx-model3d-notes-tree'
$ids = @($tree1.model3ds | ForEach-Object { [string]$_.id })
$kinds = @($tree1.model3ds | ForEach-Object { [string]$_.kind })
$word = Ensure-MsWord $inserted
$doc = Open-WordDoc $word $inserted
$cap = [string]$doc.ActiveWindow.Caption
$fn = 0
$cm = 0
try { $fn = [int]$doc.Footnotes.Item(1).Range.InlineShapes.Count } catch { $fn = -1 }
try { $cm = [int]$doc.Comments.Item(1).Range.InlineShapes.Count } catch { $cm = -1 }
$ok = ([int]$tree1.model3dCount -eq 2) -and ($ids.Count -eq 2) -and
    ($ids | Where-Object { $_ -like 'footnote:*' }) -and
    ($ids | Where-Object { $_ -like 'comment:*' }) -and
    (@($kinds | Where-Object { $_ -ne 'inline' }).Count -eq 0) -and
    ([int]$tree1.pictureCount -eq 0) -and ($cap -notmatch 'Repair') -and ($fn -ge 1) -and ($cm -ge 1)
Add-Check 'docx-D3n1' $ok "count=$($tree1.model3dCount) ids=$($ids -join ',') kinds=$($kinds -join ',') pics=$($tree1.pictureCount) caption=$cap foot=$fn comment=$cm"
Close-WordDoc $doc
