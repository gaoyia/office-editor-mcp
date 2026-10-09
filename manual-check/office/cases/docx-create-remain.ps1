# docx create 补齐剩余：docs/checklist-docx.md 的 Cs1–Cs9。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$path = Ensure-DocxCreateRemain -Force
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'docx'
} 'docx-create-remain-inspect-counts'

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'docx'
    scope = @{
        from = 1; maxParas = 20
        hyperlinks = $true; bookmarks = $true; fields = $true; tocs = $true
        maths = $true; contentControls = $true; numbering = $true; comments = $true
    }
} 'docx-create-remain-inspect'

$hl = @($tree.hyperlinks) | Select-Object -First 1
Add-Check 'docx-Cs1' (
    ($tree0.hyperlinkCount -eq 1) -and
    -not ($tree0.PSObject.Properties.Name -contains 'hyperlinks') -and
    ($tree.hyperlinkCount -eq 1) -and
    ($hl) -and
    ([string]$hl.url -eq 'https://example.com') -and
    ([string]$hl.text -eq '点这里')
) "count=$($tree.hyperlinkCount) url=$($hl.url) text=$($hl.text)"

$bm = @($tree.bookmarks) | Select-Object -First 1
Add-Check 'docx-Cs2' (
    ($tree.bookmarkCount -eq 1) -and
    ($bm) -and
    ([string]$bm.name -eq 'ch1')
) "count=$($tree.bookmarkCount) name=$($bm.name)"

$dateField = @($tree.fields) | Where-Object { [string]$_.instr -match 'DATE' } | Select-Object -First 1
Add-Check 'docx-Cs3' (
    ([int]$tree.fieldCount -ge 2) -and
    ($null -ne $dateField)
) "fieldCount=$($tree.fieldCount) date=$([string]$dateField.instr)"

$listed = @($tree.paragraphs) | Where-Object { [string]$_.num -eq 'num:1' } | Select-Object -First 1
$listedText = if ($listed) { [string]$listed.text } else { '' }
Add-Check 'docx-Cs4' (
    ($tree.numberingCount -eq 1) -and
    ($listed) -and
    ([string]$listed.num -eq 'num:1') -and
    ($listedText -match [regex]::Escape('列表项'))
) "numberingCount=$($tree.numberingCount) num=$($listed.num)"

$math = @($tree.maths) | Select-Object -First 1
Add-Check 'docx-Cs5' (
    ($tree.mathCount -eq 1) -and
    ($math) -and
    ([string]$math.text -eq 'x+1') -and
    ([string]$math.kind -eq 'inline')
) "count=$($tree.mathCount) text=$($math.text) kind=$($math.kind)"

$sdt = @($tree.contentControls) | Select-Object -First 1
Add-Check 'docx-Cs6' (
    ($tree.contentControlCount -eq 1) -and
    ($sdt) -and
    ([string]$sdt.text -eq '控件值') -and
    ([string]$sdt.kind -eq 'plainText')
) "count=$($tree.contentControlCount) text=$($sdt.text) kind=$($sdt.kind)"

Add-Check 'docx-Cs7' ($tree.tocCount -eq 1) "tocCount=$($tree.tocCount)"

$reply = @($tree.comments) | Where-Object { [string]$_.parent -eq 'comment:0' } | Select-Object -First 1
$replyText = if ($reply) { [string]@($reply.paragraphs)[0].text } else { '' }
Add-Check 'docx-Cs8' (
    ($tree.commentCount -eq 2) -and
    ($reply) -and
    ($replyText -eq '回复一')
) "count=$($tree.commentCount) parent=$($reply.parent) text=$replyText"

# 不要用带 TOC / 超链接的产物当启动种子：Word 会先弹出「域可能引用了其他文件」，COM 绑不上。
$seed = Ensure-DocxCreate
$word = Ensure-MsWord $seed
try { $word.AskToUpdateLinks = $false } catch {}
try { $word.Options.UpdateLinksAtOpen = $false } catch {}
$doc = Open-WordDoc $word $path
$caption = [string]$doc.ActiveWindow.Caption
$hlN = 0
try { $hlN = [int]$doc.Hyperlinks.Count } catch {}
$bmOk = $false
try { $bmOk = [bool]$doc.Bookmarks.Exists('ch1') } catch {}
$mathN = 0
try { $mathN = [int]$doc.OMaths.Count } catch {}
$sdtN = 0
try { $sdtN = [int]$doc.ContentControls.Count } catch {}
$cmtN = [int]$doc.Comments.Count
$replySaw = $false
$cmtSaw = $false
$comments = $doc.Comments
try {
    $nC = [int]$comments.Count
    for ($i = 1; $i -le $nC; $i++) {
        $c = $comments.Item($i)
        try {
            $t = ([string]$c.Range.Text).Trim()
            if ($t -match [regex]::Escape('批注一')) {
                $cmtSaw = $true
                $rs = $null
                try {
                    $rs = $c.Replies
                    if ([int]$rs.Count -ge 1) { $replySaw = $true }
                } catch {}
                finally { if ($rs) { Release-Com $rs } }
            }
            if ($t -match [regex]::Escape('回复一')) { $replySaw = $true }
        }
        finally { Release-Com $c }
    }
}
finally { Release-Com $comments }
$listType = 0
$p2 = $null
$r2 = $null
$lf = $null
try {
    $p2 = $doc.Paragraphs.Item(2)
    $r2 = $p2.Range
    $lf = $r2.ListFormat
    $listType = [int]$lf.ListType
} catch {}
finally {
    if ($lf) { Release-Com $lf }
    if ($r2) { Release-Com $r2 }
    if ($p2) { Release-Com $p2 }
}
Add-Check 'docx-Cs9' (
    ($caption -notmatch 'Repair') -and
    ($hlN -ge 1) -and
    $bmOk -and
    ($listType -ne 0) -and
    ($mathN -ge 1) -and
    ($sdtN -ge 1) -and
    ($cmtN -ge 2) -and
    $cmtSaw -and
    $replySaw
) "caption=$caption hl=$hlN bm=$bmOk listType=$listType math=$mathN sdt=$sdtN cmt=$cmtN/$cmtSaw reply=$replySaw"

Close-WordDoc $doc
