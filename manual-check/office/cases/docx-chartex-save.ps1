# docx 里七种 chartex 让 Word 16.0 另存。docs/checklist-docx.md 的 Cxs1–Cxs7。
# 没包 AlternateContent 时 Word 打开不报错，一另存进程就崩，所以这里必须另存。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'docx-chartex-save-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{ paragraphs = @(@{ text = '甲' }) }
} 'docx-chartex-save-create' | Out-Null
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 2 }
} 'docx-chartex-save-inspect'
$para = [string]$tree.paragraphs[0].id

$cats = @('甲', '乙', '丙')
$one = @(@{ name = '值'; categories = $cats; values = @(10, 4, 2) })
$cases = @(
    @{ n = 1; layout = 'waterfall'; prefix = 'cx1'; series = $one }
    @{ n = 2; layout = 'funnel'; prefix = 'cx2'; series = $one }
    @{ n = 3; layout = 'treemap'; prefix = 'cx1'; series = $one }
    @{ n = 4; layout = 'sunburst'; prefix = 'cx1'; series = $one }
    @{ n = 5; layout = 'clusteredColumn'; prefix = 'cx1'; series = $one }
    @{ n = 6; layout = 'pareto'; prefix = 'cx1'; series = @(
            @{ name = '值'; categories = $cats; values = @(10, 4, 2) }
            @{ name = '线'; categories = $cats; values = @(10, 14, 16) }
        ) }
    @{ n = 7; layout = 'boxWhisker'; prefix = 'cx1'; series = @(
            @{ name = 'a'; categories = $cats; values = @(1, 2, 3) }
            @{ name = 'b'; categories = $cats; values = @(2, 3, 4) }
            @{ name = 'c'; categories = $cats; values = @(3, 4, 5) }
        ) }
)

foreach ($c in $cases) {
    $made = Get-OutPath "docx-chartex-save-$($c.layout).docx"
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $src.Replace('\', '/')
        output = $made.Replace('\', '/')
        ops = @(@{ op = 'insertChart'; id = $para; type = 'chartex'; layoutId = $c.layout; title = $c.layout; series = $c.series })
    } "docx-chartex-save-$($c.layout)" | Out-Null
    $before = Get-ZipPartText $made 'word/document.xml'
    $wrapped = $before -match ('Requires="' + $c.prefix + '"')

    $word = Ensure-MsWord $made
    $doc = Open-WordDoc $word $made
    $cap = [string]$doc.ActiveWindow.Caption
    $saved = Get-OutPath "docx-chartex-save-$($c.layout)-saved.docx"
    $r = Save-WordDocChecked $doc $saved
    if ($global:OoxpWord) { Close-WordDoc $doc }
    $after = ''
    if (Test-Path $saved) { $after = Get-ZipPartText $saved 'word/document.xml' }
    $kept = $after -match 'Requires="cx\d"'
    Add-Check "docx-Cxs$($c.n)" ($wrapped -and ($cap -notmatch 'Repair') -and $r.Ok -and $kept) `
        "layout=$($c.layout) wrapped=$wrapped caption=$cap $($r.Detail) keptShell=$kept"
}
