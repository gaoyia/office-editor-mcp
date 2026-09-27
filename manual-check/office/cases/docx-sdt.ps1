# docx 内容控件：docs/checklist-docx.md 的 Cc1–Cc6。
# 用 ooxp 插入 w:sdt，再用 Word 核对无修复、仍是内容控件。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings

function Get-SdtInfo($doc) {
    $ccs = $null
    try {
        $ccs = $doc.ContentControls
        $n = [int]$ccs.Count
        if ($n -lt 1) {
            return @{ Count = 0; Type = -1; Text = '' }
        }
        $cc = $null
        try {
            $cc = $ccs.Item(1)
            $typ = [int]$cc.Type
            $rg = $null
            try {
                $rg = $cc.Range
                $txt = [string]$rg.Text
            } finally { Release-Com $rg }
            return @{ Count = $n; Type = $typ; Text = $txt }
        } finally { Release-Com $cc }
    } finally { Release-Com $ccs }
}

$src = Get-OutPath 'docx-sdt-src.docx'
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
} 'docx-sdt-create' | Out-Null

$bare = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-sdt-inspect-bare'
$bareHasList = $null -ne $bare.contentControls

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; headerFooter = $true }
} 'docx-sdt-inspect-src'
$p1 = [string]$srcTree.paragraphs[0].id
$hfPara = [string]$srcTree.headers[0].paragraphs[0].id

$plain = Get-OutPath 'docx-sdt-plain.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $plain.Replace('\', '/')
    ops = @(@{
        op = 'insertContentControl'
        id = $p1
        kind = 'plainText'
        tag = 'name'
        alias = '姓名'
        text = '张三'
    })
} 'docx-sdt-insert-plain' | Out-Null

$treeIn = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $plain.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; contentControls = $true }
} 'docx-sdt-inspect-plain'
$sdts = @($treeIn.contentControls)
$hit = @($sdts | Where-Object { [string]$_.id -eq 'sdt:1' }) | Select-Object -First 1
Add-Check 'docx-Cc1' (
    ([int]$bare.contentControlCount -eq 0) -and (-not $bareHasList) -and
    ([int]$treeIn.contentControlCount -eq 1) -and
    ($null -ne $hit) -and ([string]$hit.kind -eq 'plainText') -and
    ([string]$hit.text -eq '张三')
) "bareCount=$($bare.contentControlCount) bareList=$bareHasList count=$($treeIn.contentControlCount) kind=$($hit.kind) text=$($hit.text)"

$word = Ensure-MsWord $src
$docIn = Open-WordDoc $word $plain
$capIn = [string]$docIn.ActiveWindow.Caption
$ccIn = Get-SdtInfo $docIn
Add-Check 'docx-Cc2' (
    ($capIn -notmatch 'Repair') -and ([int]$ccIn.Count -ge 1) -and ([int]$ccIn.Type -eq 1)
) "caption=$capIn count=$($ccIn.Count) type=$($ccIn.Type)"
Close-WordDoc $docIn

$set = Get-OutPath 'docx-sdt-set.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $plain.Replace('\', '/')
    output = $set.Replace('\', '/')
    ops = @(@{ op = 'setContentControl'; id = 'sdt:1'; text = '李四' })
} 'docx-sdt-set' | Out-Null

$treeSet = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $set.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; contentControls = $true }
} 'docx-sdt-inspect-set'
$docSet = Open-WordDoc $word $set
$capSet = [string]$docSet.ActiveWindow.Caption
$ccSet = Get-SdtInfo $docSet
Add-Check 'docx-Cc3' (
    ([string]$treeSet.contentControls[0].text -eq '李四') -and
    ($capSet -notmatch 'Repair') -and
    ([int]$ccSet.Count -ge 1) -and ([int]$ccSet.Type -eq 1)
) "text=$($treeSet.contentControls[0].text) caption=$capSet count=$($ccSet.Count) type=$($ccSet.Type)"
Close-WordDoc $docSet

$drop = Get-OutPath 'docx-sdt-dropdown.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $drop.Replace('\', '/')
    ops = @(@{
        op = 'insertContentControl'
        id = $p1
        kind = 'dropDown'
        text = 'A'
        items = @('A', 'B')
    })
} 'docx-sdt-insert-dropdown' | Out-Null

$treeDrop = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $drop.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; contentControls = $true }
} 'docx-sdt-inspect-dropdown'
$docDrop = Open-WordDoc $word $drop
$capDrop = [string]$docDrop.ActiveWindow.Caption
$ccDrop = Get-SdtInfo $docDrop
Add-Check 'docx-Cc4' (
    ([string]$treeDrop.contentControls[0].kind -eq 'dropDown') -and
    ($capDrop -notmatch 'Repair') -and
    ([int]$ccDrop.Count -ge 1) -and ([int]$ccDrop.Type -eq 4)
) "kind=$($treeDrop.contentControls[0].kind) caption=$capDrop count=$($ccDrop.Count) type=$($ccDrop.Type)"
Close-WordDoc $docDrop

$gone = Get-OutPath 'docx-sdt-delete.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $plain.Replace('\', '/')
    output = $gone.Replace('\', '/')
    ops = @(@{ op = 'deleteContentControl'; id = 'sdt:1'; keep = 'content' })
} 'docx-sdt-delete' | Out-Null

$treeDel = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $gone.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; contentControls = $true }
} 'docx-sdt-inspect-delete'
$delTexts = @($treeDel.paragraphs | ForEach-Object { [string]$_.text })
$docDel = Open-WordDoc $word $gone
$capDel = [string]$docDel.ActiveWindow.Caption
$ccDel = Get-SdtInfo $docDel
Add-Check 'docx-Cc5' (
    ([int]$treeDel.contentControlCount -eq 0) -and
    (($delTexts | Where-Object { $_ -like "*$($S.chapter1)*" }).Count -ge 1) -and
    (($delTexts | Where-Object { $_ -like '*张三*' }).Count -ge 1) -and
    ($capDel -notmatch 'Repair') -and
    ([int]$ccDel.Count -eq 0)
) "count=$($treeDel.contentControlCount) texts=$($delTexts -join '|') caption=$capDel com=$($ccDel.Count)"
Close-WordDoc $docDel

$hf = Get-OutPath 'docx-sdt-header.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $hf.Replace('\', '/')
    ops = @(@{ op = 'insertContentControl'; id = $hfPara; text = '眉控' })
} 'docx-sdt-insert-header' | Out-Null

$treeHf = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hf.Replace('\', '/'); format = 'docx'
    scope = @{ contentControls = $true; headerFooter = $true }
} 'docx-sdt-inspect-header'
$docHf = Open-WordDoc $word $hf
$capHf = [string]$docHf.ActiveWindow.Caption
Add-Check 'docx-Cc6' (
    ([int]$treeHf.headers[0].contentControlCount -eq 1) -and
    ($capHf -notmatch 'Repair')
) "hfCount=$($treeHf.headers[0].contentControlCount) caption=$capHf"
Close-WordDoc $docHf
