# docx apply：docs/checklist-docx.md 的 A1–A5。
# 源文件由 Word 亲手存出，第二段在 Word 里加过粗，见 lib/fixtures.ps1。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-DocxWordSource
$word = Ensure-MsWord $src

# 段落 id 要现读，不能写死：Word 另存后 paraId 会变。
$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 200 }
} 'docx-source-inspect'
$id2 = $srcTree.paragraphs[1].id
Write-Log ("源文件段落 id：" + (@($srcTree.paragraphs | ForEach-Object { $_.id }) -join ' '))

$applied = Get-OutPath 'docx-applied.docx'
Close-WordDocByPath $word $applied
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $applied.Replace('\', '/')
    ops = @(@{ op = 'setParaText'; id = $id2; text = $S.para2new })
} 'docx-apply' | Out-Null

$doc = Open-WordDoc $word $applied
$q1 = $doc.Paragraphs.Item(1)
$q2 = $doc.Paragraphs.Item(2)
$q3 = $doc.Paragraphs.Item(3)
$t1 = Get-ParaText $q1
$t2 = Get-ParaText $q2
$t3 = Get-ParaText $q3
$caption = [string]$doc.ActiveWindow.Caption
$h1ok = ($q1.Style.NameLocal -eq $S.heading1) -or ($q1.Style.NameLocal -eq 'Heading 1')
$bold = $q2.Range.Font.Bold

Add-Check 'docx-A1' ($caption -notmatch 'Repair') $caption
Add-Check 'docx-A2' ($t2 -eq $S.para2new) $t2
Add-Check 'docx-A3' (($t1 -eq $S.chapter1) -and $h1ok) "text=$t1 style=$($q1.Style.NameLocal)"
# Word 用 -1 表示整段都加粗。
Add-Check 'docx-A4' (($bold -eq -1) -or ($bold -eq $true)) "bold=$bold"
Add-Check 'docx-A5' ($t3 -eq $S.para3keep) $t3

Release-Com $q1
Release-Com $q2
Release-Com $q3
# 文档留着不关，方便肉眼再看一遍。
