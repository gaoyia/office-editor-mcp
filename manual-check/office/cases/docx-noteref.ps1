# docx NOTEREF：库按注 id 套书签。Word 16.0 更新域后显示脚注编号 1、尾注编号 i。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function Get-NoteRefResults($doc) {
    $parts = @()
    $fields = $doc.Fields
    try {
        foreach ($f in @($fields)) {
            try {
                $code = $null
                try { $code = $f.Code.Text } catch { $code = '' }
                if ([string]$code -notmatch 'NOTEREF') { continue }
                [void]$f.Update()
                $r = $null
                try {
                    $r = $f.Result
                    $parts += [string]$r.Text
                } finally { Release-Com $r }
            } finally { Release-Com $f }
        }
    } finally { Release-Com $fields }
    return ($parts -join '|')
}

$src = Get-OutPath 'docx-noteref-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{ paragraphs = @(@{ text = '甲' }) }
} 'docx-noteref-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 5 }
} 'docx-noteref-inspect'
$para = [string]$tree.paragraphs[0].id

$noted = Get-OutPath 'docx-noteref-notes.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $noted.Replace('\', '/')
    ops = @(
        @{ op = 'insertFootnote'; id = $para; text = '脚注一' }
        @{ op = 'insertEndnote'; id = $para; text = '尾注一' }
    )
} 'docx-noteref-notes' | Out-Null

$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $noted.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 5; notes = $true }
} 'docx-noteref-inspect-notes'
$fn = [string]$treeN.footnotes[0].id
$en = [string]$treeN.endnotes[0].id

$out = Get-OutPath 'docx-noteref.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $noted.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(
        @{ op = 'insertCrossRef'; id = $para; target = $fn; kind = 'note' }
        @{ op = 'insertCrossRef'; id = $para; target = $en; kind = 'note' }
    )
} 'docx-noteref-xref' | Out-Null

$treeX = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $out.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 5; crossRefs = $true }
} 'docx-noteref-inspect-xref'
$kinds = @($treeX.crossRefs | ForEach-Object { [string]$_.kind })
$marks = @($treeX.crossRefs | ForEach-Object { [string]$_.bookmark })
$two = ([int]$treeX.crossRefCount -eq 2) -and (($kinds | Where-Object { $_ -eq 'note' }).Count -eq 2)
$refs = ($marks.Count -eq 2) -and (@($marks | Where-Object { $_ -like '_Ref*' }).Count -eq 2)

$word = Ensure-MsWord $out
$doc = Open-WordDoc $word $out
$cap = [string]$doc.ActiveWindow.Caption
$text = Get-NoteRefResults $doc
Add-Check 'docx-Nr1' ($two -and $refs -and ($cap -notmatch 'Repair') -and ($text -eq '1|i')) "count=$($treeX.crossRefCount) kinds=$($kinds -join ',') marks=$($marks -join ',') caption=$cap text=$text"
Close-WordDoc $doc
