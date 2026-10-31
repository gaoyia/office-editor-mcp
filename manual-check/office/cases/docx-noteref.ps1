# docx NOTEREF：脚注、尾注各一条。书签套在引用标记上，Word 16.0 更新域后显示编号。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function Wrap-NoteMarks([string]$src, [string]$dest) {
    $py = Join-Path $global:OoxpOutDir 'wrap-noteref.py'
    @'
import sys, zipfile
from io import BytesIO
src, dest = sys.argv[1], sys.argv[2]
marks = [("footnoteReference", "_RefFn1"), ("endnoteReference", "_RefEn1")]
buf = BytesIO()
with zipfile.ZipFile(src, "r") as zin, zipfile.ZipFile(buf, "w") as zout:
    for info in zin.infolist():
        data = zin.read(info.filename)
        if info.filename == "word/document.xml":
            xml = data.decode("utf-8")
            for i, (local, name) in enumerate(marks):
                needle = "<w:" + local
                at = xml.find(needle)
                if at < 0:
                    raise SystemExit("missing " + local)
                head = xml[:at]
                run_at = max(head.rfind("<w:r>"), head.rfind("<w:r "))
                if run_at < 0:
                    raise SystemExit("missing run " + local)
                rel = xml.find("</w:r>", at)
                run_end = rel + len("</w:r>")
                bid = str(90 + i)
                wrapped = (
                    '<w:bookmarkStart w:id="' + bid + '" w:name="' + name + '"/>'
                    + xml[run_at:run_end]
                    + '<w:bookmarkEnd w:id="' + bid + '"/>'
                )
                xml = xml[:run_at] + wrapped + xml[run_end:]
            data = xml.encode("utf-8")
        zout.writestr(info, data)
open(dest, "wb").write(buf.getvalue())
'@ | Set-Content -Path $py -Encoding utf8
    & python -X utf8 $py $src $dest
    if ($LASTEXITCODE -ne 0) { throw "wrap failed $LASTEXITCODE" }
}

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

$marked = Get-OutPath 'docx-noteref-marked.docx'
Wrap-NoteMarks $noted $marked

$out = Get-OutPath 'docx-noteref.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $marked.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(
        @{ op = 'insertCrossRef'; id = $para; bookmark = '_RefFn1'; kind = 'note' }
        @{ op = 'insertCrossRef'; id = $para; bookmark = '_RefEn1'; kind = 'note' }
    )
} 'docx-noteref-xref' | Out-Null

$treeX = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $out.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 5; crossRefs = $true }
} 'docx-noteref-inspect-xref'
$kinds = @($treeX.crossRefs | ForEach-Object { [string]$_.kind })
$two = ([int]$treeX.crossRefCount -eq 2) -and (($kinds | Where-Object { $_ -eq 'note' }).Count -eq 2)

$word = Ensure-MsWord $out
$doc = Open-WordDoc $word $out
$cap = [string]$doc.ActiveWindow.Caption
$text = Get-NoteRefResults $doc
Add-Check 'docx-Nr1' ($two -and ($cap -notmatch 'Repair') -and ($text -eq '1|i')) "count=$($treeX.crossRefCount) kinds=$($kinds -join ',') caption=$cap text=$text"
Close-WordDoc $doc
