# 绑定写回自定义 XML，重复节增一项。样本由 Word 16.0 另存，再交给 ooxp。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'docx-sdt-bind-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{ paragraphs = @(@{ text = 'host' }) }
} 'sdt-bind-create' | Out-Null

$word = Ensure-MsWord $src
$bound = Get-OutPath 'docx-sdt-bind-word.docx'
$doc = Open-WordDoc $word $src
$xml = '<root xmlns="urn:ooxp:probe"><name>old</name></root>'
$part = $doc.CustomXMLParts.Add($xml)
$range = $doc.Paragraphs.Item(1).Range
$range.Collapse(0) | Out-Null
$cc = $doc.ContentControls.Add(1, $range)
[void]$cc.XMLMapping.SetMapping('/ns0:root/ns0:name', "xmlns:ns0='urn:ooxp:probe'", $part)
$cc.Range.Text = 'old'
if (Test-Path $bound) { Remove-Item -Force $bound }
$doc.SaveAs2($bound, 16)
Release-Com $cc
Release-Com $range
Release-Com $part
Close-WordDoc $doc

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $bound.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; contentControls = $true }
} 'sdt-bind-inspect'
$sid = [string]$tree.contentControls[0].id
$edited = Get-OutPath 'docx-sdt-bind-edited.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $bound.Replace('\', '/')
    output = $edited.Replace('\', '/')
    ops = @(@{ op = 'setContentControl'; id = $sid; text = 'new-value' })
} 'sdt-bind-set' | Out-Null

$doc2 = Open-WordDoc $word $edited
$cap = [string]$doc2.ActiveWindow.Caption
$shown = [string]$doc2.ContentControls.Item(1).Range.Text
$xmlNow = ''
foreach ($p in @($doc2.CustomXMLParts)) {
    $raw = [string]$p.XML
    if ($raw -like '*urn:ooxp:probe*') { $xmlNow = $raw }
}
Add-Check 'docx-sdt-bind' (($cap -notmatch 'Repair') -and ($shown -eq 'new-value') -and ($xmlNow -like '*new-value*')) "caption=$cap shown=$shown xml=$xmlNow"
Close-WordDoc $doc2

$repeatSrc = Get-OutPath 'docx-sdt-repeat-word.docx'
$doc3 = Open-WordDoc $word $src
$para = $doc3.Paragraphs.Item(1).Range
$rep = $doc3.ContentControls.Add(9, $para)
if (Test-Path $repeatSrc) { Remove-Item -Force $repeatSrc }
$doc3.SaveAs2($repeatSrc, 16)
Release-Com $rep
Release-Com $para
Close-WordDoc $doc3

$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $repeatSrc.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; contentControls = $true }
} 'sdt-repeat-inspect'
$rid = [string]$treeR.contentControls[0].id
$repeatOut = Get-OutPath 'docx-sdt-repeat-edited.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $repeatSrc.Replace('\', '/')
    output = $repeatOut.Replace('\', '/')
    ops = @(@{ op = 'insertRepeatingItem'; id = $rid })
} 'sdt-repeat-add' | Out-Null

$doc4 = Open-WordDoc $word $repeatOut
$capR = [string]$doc4.ActiveWindow.Caption
$n = $doc4.ContentControls.Item(1).RepeatingSectionItems.Count
Add-Check 'docx-sdt-repeat' (($capR -notmatch 'Repair') -and ($n -eq 2)) "caption=$capR items=$n"
Close-WordDoc $doc4
