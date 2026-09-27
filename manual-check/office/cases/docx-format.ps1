# docx 段落与 run 直接格式：docs/checklist-docx.md 的 Pf1–Pf6。
# 用 ooxp 改对齐、加粗、段距，再用 Word 打开核对无修复、未点名 run 颜色还在。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'docx-format-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = 'AB' }
            @{ text = '间距段' }
        )
    }
} 'docx-format-create' | Out-Null

# Word 给第一段第二个字上色，拆成两个 run，供 Pf4 / Pf5 用。
$word = Ensure-MsWord $src
$docSrc = Open-WordDoc $word $src
$p1src = $docSrc.Paragraphs.Item(1)
$ch2 = $p1src.Range.Characters.Item(2)
$ch2.Font.Color = 255
$docSrc.Save()
Close-WordDoc $docSrc

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-format-inspect'
$pBody = @($tree0.paragraphs) | Where-Object { [string]$_.text -like 'A*' } | Select-Object -First 1
$pSpace = @($tree0.paragraphs) | Where-Object { [string]$_.text -eq '间距段' } | Select-Object -First 1
$noRuns = ($null -eq $pBody.runs)
$hasCount = ($null -ne $pBody.runCount) -and ([int]$pBody.runCount -ge 1)

$treeRuns = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; runs = $true }
} 'docx-format-inspect-runs'
$pListed = @($treeRuns.paragraphs) | Where-Object { $_.id -eq $pBody.id } | Select-Object -First 1
$runs = @($pListed.runs)
$run1 = $runs | Select-Object -First 1
$runIdOk = ([string]$run1.id -eq ("{0}/run:1" -f [string]$pBody.id))
$pf1ok = $noRuns -and $hasCount -and ($runs.Count -ge 1) -and $runIdOk
Add-Check 'docx-Pf1' $pf1ok "runCount=$($pBody.runCount) listed=$($runs.Count) noRuns=$noRuns id=$($run1.id)"

$alignOut = Get-OutPath 'docx-format-align.docx'
Close-WordDocByPath $word $alignOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $alignOut.Replace('\', '/')
    ops = @(@{ op = 'setParaFormat'; id = [string]$pBody.id; align = 'center' })
} 'docx-format-align' | Out-Null

$treeA = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $alignOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-format-align-inspect'
$pa = @($treeA.paragraphs) | Where-Object { $_.id -eq $pBody.id } | Select-Object -First 1
$pf2ok = ([string]$pa.align -eq 'center')
Add-Check 'docx-Pf2' $pf2ok "align=$($pa.align)"

$docA = Open-WordDoc $word $alignOut
$capA = [string]$docA.ActiveWindow.Caption
$alignVal = [int]$docA.Paragraphs.Item(1).Alignment
Close-WordDoc $docA
$pf3ok = ($capA -notmatch 'Repair') -and ($alignVal -eq 1)
Add-Check 'docx-Pf3' $pf3ok "caption=$capA alignment=$alignVal"

$runOut = Get-OutPath 'docx-format-run.docx'
Close-WordDocByPath $word $runOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $runOut.Replace('\', '/')
    ops = @(@{ op = 'setRunFormat'; id = [string]$run1.id; bold = $true })
} 'docx-format-run' | Out-Null

$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $runOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; runs = $true }
} 'docx-format-run-inspect'
$pr = @($treeR.paragraphs) | Where-Object { $_.id -eq $pBody.id } | Select-Object -First 1
$listedR = @($pr.runs)
$r1 = $listedR | Where-Object { $_.id -eq $run1.id } | Select-Object -First 1
$rOther = $listedR | Where-Object { $_.id -ne $run1.id } | Select-Object -First 1

$docR = Open-WordDoc $word $runOut
$capR = [string]$docR.ActiveWindow.Caption
$boldVal = [int]$docR.Paragraphs.Item(1).Range.Characters.Item(1).Font.Bold
$color2 = $null
if ($listedR.Count -ge 2) {
    $color2 = [int64]$docR.Paragraphs.Item(1).Range.Characters.Item(2).Font.Color
}
Close-WordDoc $docR
$pf4ok = ($capR -notmatch 'Repair') -and ($boldVal -eq -1)
Add-Check 'docx-Pf4' $pf4ok "caption=$capR bold=$boldVal"

$colorTree = $null
if ($null -ne $rOther) { $colorTree = [string]$rOther.color }
$pf5ok = ($listedR.Count -ge 2) -and (
    ($colorTree -eq 'FF0000') -or ($color2 -eq 255)
)
Add-Check 'docx-Pf5' $pf5ok "runs=$($listedR.Count) color=$colorTree wordColor=$color2"

$spOut = Get-OutPath 'docx-format-spacing.docx'
Close-WordDocByPath $word $spOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $spOut.Replace('\', '/')
    ops = @(@{ op = 'setParaFormat'; id = [string]$pSpace.id; before = 240; after = 120 })
} 'docx-format-spacing' | Out-Null

$docS = Open-WordDoc $word $spOut
$capS = [string]$docS.ActiveWindow.Caption
$before = [double]$docS.Paragraphs.Item(2).Range.ParagraphFormat.SpaceBefore
$after = [double]$docS.Paragraphs.Item(2).Range.ParagraphFormat.SpaceAfter
Close-WordDoc $docS
$pf6ok = ($capS -notmatch 'Repair') -and
    ([math]::Abs($before - 12) -lt 0.05) -and
    ([math]::Abs($after - 6) -lt 0.05)
Add-Check 'docx-Pf6' $pf6ok "caption=$capS before=$before after=$after"

[void]$r1
