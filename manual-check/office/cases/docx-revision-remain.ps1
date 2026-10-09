# docx 修订剩余类型：docs/checklist-docx.md 的 Rm1–Rm9。
# 用 Word 跟踪修订造样例，再用 ooxp 接受 / 拒绝，不调 COM 的 Accept/Reject。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function Set-FinalView($doc) {
    try {
        $win = $doc.ActiveWindow
        $view = $win.View
        $view.RevisionsView = 0
        Release-Com $view
        Release-Com $win
    } catch {}
}

function Get-KindIds($tree, [string[]]$kinds) {
    $arr = @($tree.revisions | Where-Object { $kinds -contains [string]$_.kind })
    return @($arr | Sort-Object { [int]$_.index } -Descending | ForEach-Object { [string]$_.id })
}

function Copy-Host([string]$src, [string]$name) {
    $dest = Get-OutPath $name
    if (Test-Path $dest) { Remove-Item -Force $dest }
    Copy-Item -Force $src $dest
    return $dest
}

$changeKinds = @(
    'tblPrChange', 'tblGridChange', 'tblPrExChange', 'tcPrChange',
    'trPrChange', 'rPrChange', 'pPrChange', 'sectPrChange'
)

$skel = Get-OutPath 'docx-revision-remain-skel.docx'
if (Test-Path $skel) { Remove-Item -Force $skel }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $skel.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = '甲乙丙丁戊' }
            @{ text = '己庚辛壬' }
        )
        tables = @(@{
            after = 2
            cells = @(
                @('A1', 'B1'),
                @('A2', 'B2')
            )
        })
    }
} 'docx-rm-skel' | Out-Null

$word = Ensure-MsWord $skel

$tblHost = Copy-Host $skel 'docx-revision-remain-tbl.docx'
Close-WordDocByPath $word $tblHost
$doc = Open-WordDoc $word $tblHost
$doc.TrackRevisions = $true
$cell = $doc.Tables.Item(1).Cell(1, 1)
$cell.Shading.BackgroundPatternColor = 255
$cell.Range.Font.Bold = $true
$doc.TrackRevisions = $false
$doc.Save()
Close-WordDoc $doc

$treeBare = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tblHost.Replace('\', '/'); format = 'docx'
} 'docx-rm-inspect-bare'
$listedTbl = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tblHost.Replace('\', '/'); format = 'docx'
    scope = @{ revisions = $true; from = 1; maxParas = 40 }
} 'docx-rm-inspect-tbl'
$chg = @($listedTbl.revisions | Where-Object {
    @('tcPrChange', 'tblPrChange') -contains [string]$_.kind
}) | Select-Object -First 1
$rm1ok = ($null -ne $treeBare.revisionCount) -and ([int]$treeBare.revisionCount -ge 1) -and
    ($null -eq $treeBare.revisions) -and ($null -ne $chg) -and ($null -eq $chg.host)
Add-Check 'docx-Rm1' $rm1ok "count=$($treeBare.revisionCount) kind=$($chg.kind) host=$($chg.host)"

$accTbl = Get-OutPath 'docx-revision-remain-accept-tbl.docx'
Close-WordDocByPath $word $accTbl
$accIds = Get-KindIds $listedTbl $changeKinds
$accOps = @($accIds | ForEach-Object { @{ op = 'acceptRevision'; id = $_ } })
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $tblHost.Replace('\', '/')
    output = $accTbl.Replace('\', '/')
    ops = $accOps
} 'docx-rm-accept-tbl' | Out-Null
$docA = Open-WordDoc $word $accTbl
Set-FinalView $docA
$capA = [string]$docA.ActiveWindow.Caption
$colorA = [int]$docA.Tables.Item(1).Cell(1, 1).Shading.BackgroundPatternColor
Add-Check 'docx-Rm2' (($capA -notmatch 'Repair') -and ($colorA -eq 255)) "caption=$capA color=$colorA ops=$($accOps.Count)"
Close-WordDoc $docA

$rejTbl = Get-OutPath 'docx-revision-remain-reject-tbl.docx'
Close-WordDocByPath $word $rejTbl
$rejOps = @($accIds | ForEach-Object { @{ op = 'rejectRevision'; id = $_ } })
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $tblHost.Replace('\', '/')
    output = $rejTbl.Replace('\', '/')
    ops = $rejOps
} 'docx-rm-reject-tbl' | Out-Null
$docR = Open-WordDoc $word $rejTbl
Set-FinalView $docR
$capRj = [string]$docR.ActiveWindow.Caption
$colorR = [int]$docR.Tables.Item(1).Cell(1, 1).Shading.BackgroundPatternColor
Add-Check 'docx-Rm3' (($capRj -notmatch 'Repair') -and ($colorR -ne 255)) "caption=$capRj color=$colorR"
Close-WordDoc $docR

$paraHost = Copy-Host $skel 'docx-revision-remain-para.docx'
Close-WordDocByPath $word $paraHost
$doc = Open-WordDoc $word $paraHost
$doc.TrackRevisions = $true
$r = $doc.Paragraphs.Item(1).Range
$r.Collapse(0) | Out-Null
$r.InsertParagraphAfter() | Out-Null
$doc.TrackRevisions = $false
$doc.Save()
Close-WordDoc $doc

$listedPara = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $paraHost.Replace('\', '/'); format = 'docx'
    scope = @{ revisions = $true; from = 1; maxParas = 40 }
} 'docx-rm-inspect-para'
$pmark = @($listedPara.revisions | Where-Object { [string]$_.kind -eq 'paraMarkIns' }) | Select-Object -First 1
$rm4ok = ($null -ne $pmark) -and ($null -ne $pmark.host) -and ($null -eq $pmark.text)
Add-Check 'docx-Rm4' $rm4ok "id=$($pmark.id) host=$($pmark.host) text=$($pmark.text)"
$paraCount0 = [int]$listedPara.paraCount

$accPara = Get-OutPath 'docx-revision-remain-accept-para.docx'
Close-WordDocByPath $word $accPara
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $paraHost.Replace('\', '/')
    output = $accPara.Replace('\', '/')
    ops = @(@{ op = 'acceptRevision'; id = [string]$pmark.id })
} 'docx-rm-accept-para' | Out-Null
$listedAccP = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $accPara.Replace('\', '/'); format = 'docx'
    scope = @{ revisions = $true; from = 1; maxParas = 40 }
} 'docx-rm-inspect-acc-para'
$stillMark = @($listedAccP.revisions | Where-Object { [string]$_.kind -eq 'paraMarkIns' })
$docAp = Open-WordDoc $word $accPara
Set-FinalView $docAp
$capAp = [string]$docAp.ActiveWindow.Caption
Add-Check 'docx-Rm5' (($capAp -notmatch 'Repair') -and ($stillMark.Count -eq 0) -and ([int]$listedAccP.paraCount -eq $paraCount0)) `
    "caption=$capAp paraCount=$($listedAccP.paraCount) marks=$($stillMark.Count)"
Close-WordDoc $docAp

$rejPara = Get-OutPath 'docx-revision-remain-reject-para.docx'
Close-WordDocByPath $word $rejPara
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $paraHost.Replace('\', '/')
    output = $rejPara.Replace('\', '/')
    ops = @(@{ op = 'rejectRevision'; id = [string]$pmark.id })
} 'docx-rm-reject-para' | Out-Null
$listedRejP = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $rejPara.Replace('\', '/'); format = 'docx'
    scope = @{ revisions = $true; from = 1; maxParas = 40 }
} 'docx-rm-inspect-rej-para'
$docRp = Open-WordDoc $word $rejPara
Set-FinalView $docRp
$capRjp = [string]$docRp.ActiveWindow.Caption
$p1 = Get-ParaText $docRp.Paragraphs.Item(1)
Add-Check 'docx-Rm6' (($capRjp -notmatch 'Repair') -and ([int]$listedRejP.paraCount -eq ($paraCount0 - 1)) -and ($p1 -match '甲')) `
    "caption=$capRjp paraCount=$($listedRejP.paraCount) p1=$p1"
Close-WordDoc $docRp

$rowHost = Copy-Host $skel 'docx-revision-remain-row.docx'
Close-WordDocByPath $word $rowHost
$doc = Open-WordDoc $word $rowHost
$doc.TrackRevisions = $true
$doc.Tables.Item(1).Rows.Item(2).Delete()
$doc.TrackRevisions = $false
$doc.Save()
Close-WordDoc $doc

$listedRow = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $rowHost.Replace('\', '/'); format = 'docx'
    scope = @{ revisions = $true; from = 1; maxParas = 40 }
} 'docx-rm-inspect-row'
$rowDel = @($listedRow.revisions | Where-Object { [string]$_.kind -eq 'rowDel' }) | Select-Object -First 1
Add-Check 'docx-Rm7' (($null -ne $rowDel) -and ($null -eq $rowDel.host)) "id=$($rowDel.id) host=$($rowDel.host)"

$accRow = Get-OutPath 'docx-revision-remain-accept-row.docx'
Close-WordDocByPath $word $accRow
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $rowHost.Replace('\', '/')
    output = $accRow.Replace('\', '/')
    ops = @(@{ op = 'acceptRevision'; id = [string]$rowDel.id })
} 'docx-rm-accept-row' | Out-Null
$docAr = Open-WordDoc $word $accRow
Set-FinalView $docAr
$capAr = [string]$docAr.ActiveWindow.Caption
$rowsA = [int]$docAr.Tables.Item(1).Rows.Count
Add-Check 'docx-Rm8' (($capAr -notmatch 'Repair') -and ($rowsA -eq 1)) "caption=$capAr rows=$rowsA"
Close-WordDoc $docAr

$rejRow = Get-OutPath 'docx-revision-remain-reject-row.docx'
Close-WordDocByPath $word $rejRow
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $rowHost.Replace('\', '/')
    output = $rejRow.Replace('\', '/')
    ops = @(@{ op = 'rejectRevision'; id = [string]$rowDel.id })
} 'docx-rm-reject-row' | Out-Null
$docRr = Open-WordDoc $word $rejRow
Set-FinalView $docRr
$capRr = [string]$docRr.ActiveWindow.Caption
$rowsR = [int]$docRr.Tables.Item(1).Rows.Count
$a2 = [string]$docRr.Tables.Item(1).Cell(2, 1).Range.Text
Add-Check 'docx-Rm9' (($capRr -notmatch 'Repair') -and ($rowsR -eq 2) -and ($a2 -match 'A2')) `
    "caption=$capRr rows=$rowsR a2=$a2"
Close-WordDoc $docRr
