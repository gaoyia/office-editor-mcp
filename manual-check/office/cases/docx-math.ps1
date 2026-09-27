# docx 公式：docs/checklist-docx.md 的 Mm1–Mm6。
# 用 ooxp 插入 OMML，再用 Word 核对无修复、仍是公式区。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings

function Get-OMathInfo($doc) {
    $omaths = $null
    try {
        $omaths = $doc.OMaths
        $n = [int]$omaths.Count
        if ($n -lt 1) {
            return @{ Count = 0; Type = -1; Text = '' }
        }
        $om = $null
        try {
            $om = $omaths.Item(1)
            $typ = [int]$om.Type
            $rg = $null
            try {
                $rg = $om.Range
                $txt = [string]$rg.Text
            } finally { Release-Com $rg }
            return @{ Count = $n; Type = $typ; Text = $txt }
        } finally { Release-Com $om }
    } finally { Release-Com $omaths }
}

$src = Get-OutPath 'docx-math-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = $S.chapter1 }
            @{ text = $S.hello }
        )
        headers = @(
            @{ type = 'default'; paragraphs = @(@{ text = $S.headerSrc }) }
        )
    }
} 'docx-math-create' | Out-Null

$bare = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-math-inspect-bare'
$bareHasList = $null -ne $bare.maths

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; headerFooter = $true }
} 'docx-math-inspect-src'
$p1 = [string]$srcTree.paragraphs[0].id
$hfPara = [string]$srcTree.headers[0].paragraphs[0].id

$inline = Get-OutPath 'docx-math-inline.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inline.Replace('\', '/')
    ops = @(@{ op = 'insertMath'; id = $p1; text = 'E=mc^2' })
} 'docx-math-insert-inline' | Out-Null

$treeIn = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inline.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; maths = $true }
} 'docx-math-inspect-inline'
$maths = @($treeIn.maths)
$hit = @($maths | Where-Object { [string]$_.id -eq 'math:1' }) | Select-Object -First 1
Add-Check 'docx-Mm1' (
    ([int]$bare.mathCount -eq 0) -and (-not $bareHasList) -and
    ([int]$treeIn.mathCount -eq 1) -and
    ($null -ne $hit) -and ([string]$hit.kind -eq 'inline') -and
    ([string]$hit.text -eq 'E=mc^2')
) "bareCount=$($bare.mathCount) bareList=$bareHasList count=$($treeIn.mathCount) kind=$($hit.kind) text=$($hit.text)"

$word = Ensure-MsWord $src
$docIn = Open-WordDoc $word $inline
$capIn = [string]$docIn.ActiveWindow.Caption
$omIn = Get-OMathInfo $docIn
Add-Check 'docx-Mm2' (
    ($capIn -notmatch 'Repair') -and ([int]$omIn.Count -ge 1)
) "caption=$capIn omaths=$($omIn.Count) type=$($omIn.Type)"
Close-WordDoc $docIn

$set = Get-OutPath 'docx-math-set.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inline.Replace('\', '/')
    output = $set.Replace('\', '/')
    ops = @(@{ op = 'setMath'; id = 'math:1'; text = 'x=1' })
} 'docx-math-set' | Out-Null

$treeSet = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $set.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; maths = $true }
} 'docx-math-inspect-set'
$docSet = Open-WordDoc $word $set
$capSet = [string]$docSet.ActiveWindow.Caption
$omSet = Get-OMathInfo $docSet
Add-Check 'docx-Mm3' (
    ([string]$treeSet.maths[0].text -eq 'x=1') -and
    ($capSet -notmatch 'Repair') -and
    ([int]$omSet.Count -ge 1)
) "text=$($treeSet.maths[0].text) caption=$capSet omaths=$($omSet.Count)"
Close-WordDoc $docSet

$disp = Get-OutPath 'docx-math-display.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $disp.Replace('\', '/')
    ops = @(@{ op = 'insertMath'; id = $p1; kind = 'display'; text = 'a^2+b^2=c^2' })
} 'docx-math-insert-display' | Out-Null

$treeDisp = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $disp.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; maths = $true }
} 'docx-math-inspect-display'
$docDisp = Open-WordDoc $word $disp
$capDisp = [string]$docDisp.ActiveWindow.Caption
$omDisp = Get-OMathInfo $docDisp
Add-Check 'docx-Mm4' (
    ([string]$treeDisp.maths[0].kind -eq 'display') -and
    ($capDisp -notmatch 'Repair') -and
    ([int]$omDisp.Count -ge 1)
) "kind=$($treeDisp.maths[0].kind) caption=$capDisp omaths=$($omDisp.Count) type=$($omDisp.Type)"
Close-WordDoc $docDisp

$gone = Get-OutPath 'docx-math-delete.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inline.Replace('\', '/')
    output = $gone.Replace('\', '/')
    ops = @(@{ op = 'deleteMath'; id = 'math:1' })
} 'docx-math-delete' | Out-Null

$treeDel = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $gone.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; maths = $true }
} 'docx-math-inspect-delete'
$delTexts = @($treeDel.paragraphs | ForEach-Object { [string]$_.text })
$docDel = Open-WordDoc $word $gone
$capDel = [string]$docDel.ActiveWindow.Caption
Add-Check 'docx-Mm5' (
    ([int]$treeDel.mathCount -eq 0) -and
    ($delTexts -contains $S.chapter1) -and
    ($capDel -notmatch 'Repair')
) "count=$($treeDel.mathCount) texts=$($delTexts -join '|') caption=$capDel"
Close-WordDoc $docDel

$hf = Get-OutPath 'docx-math-header.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $hf.Replace('\', '/')
    ops = @(@{ op = 'insertMath'; id = $hfPara; text = 'h=1' })
} 'docx-math-insert-header' | Out-Null

$treeHf = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hf.Replace('\', '/'); format = 'docx'
    scope = @{ maths = $true; headerFooter = $true }
} 'docx-math-inspect-header'
$docHf = Open-WordDoc $word $hf
$capHf = [string]$docHf.ActiveWindow.Caption
Add-Check 'docx-Mm6' (
    ([int]$treeHf.headers[0].mathCount -eq 1) -and
    ($capHf -notmatch 'Repair')
) "hfCount=$($treeHf.headers[0].mathCount) caption=$capHf"
Close-WordDoc $docHf
