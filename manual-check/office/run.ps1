# Microsoft Office 这一轮的驱动。
#
#   .\run.ps1                      全跑，跑完把文件留在 Office 里给人看
#   .\run.ps1 -Case docx-apply     只跑一个
#   .\run.ps1 -Case xlsx-*         按通配符挑
#   .\run.ps1 -List                只看有哪些用例
#   .\run.ps1 -CloseOffice         跑完关掉 Word、Excel 和 PowerPoint，不留进程
#
# 每个用例套独立的 try/catch，一个塌了不影响后面的。
# 顺序有讲究：inspect 组不用开 Office，先跑；opc-resave 里 Word 可能挂住，放最后。

[CmdletBinding()]
param(
    [string[]]$Case = @('*'),
    [switch]$List,
    [switch]$CloseOffice
)

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', 'lib', 'bootstrap.ps1'))

$order = @(
    'xlsx-inspect'
    'docx-inspect'
    'pptx-inspect'
    'xlsx-create'
    'xlsx-create-pictures'
    'xlsx-create-charts'
    'xlsx-create-headerfooter'
    'xlsx-create-drawing'
    'xlsx-create-chartsheet'
    'xlsx-create-round4'
    'xlsx-create-round5'
    'docx-create'
    'docx-create-pictures'
    'docx-create-full'
    'docx-create-remain'
    'docx-create-round4'
    'docx-create-round5'
    'pptx-create'
    'pptx-create-pictures'
    'pptx-create-charts'
    'pptx-create-notes-comments'
    'pptx-create-remain'
    'pptx-create-round4'
    'pptx-create-round5'
    'xlsx-apply'
    'xlsx-pictures'
    'xlsx-svg'
    'xlsx-emf'
    'xlsx-model3d'
    'xlsx-model3d-geom'
    'xlsx-model3d-camera'
    'xlsx-omml'
    'xlsx-math-geom'
    'xlsx-charts'
    'chartex-edit'
    'chartex-cache'
    'xlsx-charts-remain'
    'xlsx-charts-rare'
    'xlsx-charts-rare-remain'
    'xlsx-chartsheet'
    'xlsx-headerfooter'
    'docx-apply'
    'docx-body-para'
    'docx-new-header'
    'docx-section'
    'docx-table'
    'docx-table-pin'
    'docx-inline-pin'
    'docx-doc-pin'
    'docx-revision'
    'docx-revision-remain'
    'docx-hyperlink'
    'docx-drawing-hlink'
    'docx-drawing-hlink-remain'
    'docx-field'
    'docx-field-types'
    'docx-numbering'
    'docx-style'
    'docx-style-table'
    'docx-table-width'
    'docx-cell-valign'
    'docx-table-borders'
    'docx-cell-borders'
    'docx-cell-props'
    'docx-box-table'
    'docx-box-fields'
    'docx-format'
    'docx-para-run-remain'
    'docx-run-spacing'
    'docx-run-scale'
    'docx-east-asia'
    'docx-insert-section'
    'docx-toc'
    'docx-math'
    'docx-math-struct'
    'docx-sdt'
    'docx-sdt-types'
    'docx-sdt-bind'
    'docx-headerfooter'
    'docx-distance'
    'docx-link'
    'docx-picture'
    'docx-body-picture'
    'docx-svg'
    'docx-emf'
    'docx-model3d'
    'docx-model3d-geom'
    'docx-model3d-camera'
    'docx-model3d-float'
    'docx-model3d-notes'
    'docx-chartex-save'
    'docx-ink'
    'docx-ink-geom'
    'docx-textbox'
    'docx-textbox-anchor'
    'docx-notes'
    'docx-noteref'
    'docx-comments'
    'docx-charts'
    'docx-charts-ext'
    'docx-charts-remain'
    'docx-charts-rare'
    'docx-charts-rare-remain'
    'docx-smartart'
    'docx-container-drawing'
    'docx-docprops'
    'docx-ole'
    'docx-container-ole'
    'docx-container-toc'
    'docx-protect'
    'docx-protect-hash'
    'docx-create-password'
    'docx-protect-rev-math'
    'docx-watermark'
    'docx-theme'
    'docx-theme-remain'
    'docx-theme-defaults'
    'docx-caption-xref'
    'docx-index'
    'docx-biblio'
    'docx-glossary'
    'docx-font-embed'
    'docx-page-bg'
    'docx-theme-bg-remain'
    'docx-groups'
    'docx-drawing-shape'
    'xlsx-docprops'
    'xlsx-sheets'
    'xlsx-sheet-refs'
    'xlsx-cellstyle'
    'xlsx-named-style'
    'xlsx-layout'
    'xlsx-formula'
    'xlsx-formula-master'
    'xlsx-validation'
    'xlsx-conditional-format'
    'xlsx-id-pin'
    'xlsx-chart-id-pin'
    'xlsx-cf-spark-style'
    'xlsx-table'
    'xlsx-pivot'
    'xlsx-defined-name'
    'xlsx-print'
    'xlsx-view'
    'xlsx-sort'
    'xlsx-hidden-sort'
    'xlsx-comments'
    'xlsx-shapes'
    'xlsx-shape-format'
    'xlsx-shape-spacing'
    'xlsx-text-fill'
    'xlsx-shapes-geom'
    'xlsx-groups'
    'xlsx-ole'
    'xlsx-slicers'
    'xlsx-timelines'
    'xlsx-slicer-anchor'
    'xlsx-slicer-caption'
    'xlsx-slicer-filter'
    'xlsx-theme'
    'xlsx-theme-defaults'
    'xlsx-connections'
    'xlsx-external-link'
    'xlsx-break-link'
    'xlsx-extpath-datatable'
    'xlsx-outline'
    'xlsx-custom-view'
    'xlsx-hyperlinks'
    'xlsx-drawing-hlink'
    'xlsx-sparklines'
    'xlsx-protect'
    'xlsx-protect-hash'
    'xlsx-legacy-password'
    'xlsx-edit-range'
    'xlsx-dialog-macro'
    'xlsx-anchor'
    'pptx-docprops'
    'pptx-apply'
    'pptx-slides'
    'pptx-shapes'
    'pptx-paint'
    'pptx-shape-format'
    'pptx-shape-spacing'
    'pptx-text-fill'
    'pptx-shape-geom'
    'pptx-custgeom'
    'pptx-tables'
    'pptx-id-pin'
    'pptx-table-grid'
    'pptx-table-cell-fill'
    'pptx-table-cell-fill-ex'
    'pptx-table-cell-mar'
    'pptx-table-cell-pr'
    'pptx-table-style'
    'pptx-transition'
    'pptx-transition-remain'
    'pptx-transition-sound'
    'pptx-timing'
    'pptx-anim-write'
    'pptx-masters'
    'pptx-master-write'
    'pptx-placeholder-more'
    'pptx-master-deco'
    'pptx-theme-size'
    'pptx-theme-remain'
    'pptx-theme-defaults'
    'pptx-theme-clrmap'
    'pptx-theme-notes-handout'
    'pptx-notes-size'
    'pptx-custom-show'
    'pptx-ole'
    'pptx-zoom'
    'pptx-zoom-remain'
    'pptx-zoom-geom'
    'pptx-protect'
    'pptx-protect-hash'
    'pptx-fields'
    'pptx-run-links'
    'pptx-hyperlinks-remain'
    'pptx-comment-pics'
    'pptx-handout'
    'pptx-notes-master'
    'pptx-notes-master-objects'
    'pptx-hyperlinks'
    'pptx-pictures'
    'pptx-svg'
    'pptx-emf'
    'pptx-model3d'
    'pptx-model3d-geom'
    'pptx-model3d-camera'
    'pptx-omml'
    'pptx-math-geom'
    'pptx-media'
    'pptx-media-geom'
    'pptx-media-trim'
    'pptx-sections'
    'pptx-charts'
    'pptx-charts-remain'
    'pptx-charts-rare'
    'pptx-charts-rare-remain'
    'pptx-smartart'
    'pptx-notes'
    'pptx-comments'
    'opc-resave'
    'xml-attr-newline'
    'id-pin'
    'pptx-id-pin-refs'
    'strict-docx'
    'docx-tab-br'
    'xlsx-xml-comment'
    'xlsx-implicit-cell'
    'pptx-connector-line'
    'docx-chart-shared'
)

# 阶段二探测用例：只往 out 写 XML 对照片段，不改仓库文件，也不挂待做序号。
# 不进默认全跑，只有 -Case 里点名带 probe 才选，免得把没出结果的探测混进日常验收。
$probeOrder = @(
    'pptx-bg-fill-probe'
    'pptx-blip-fill-probe'
    'pptx-zorder-probe'
    'pptx-offslide-geom-probe'
)

$casesDir = Join-Path $PSScriptRoot 'cases'
if ($List) {
    $order | ForEach-Object { "  $_" }
    $probeOrder | ForEach-Object { "  $_ (probe)" }
    return
}

$wantProbe = @($Case | Where-Object { $_ -like '*probe*' }).Count -gt 0
$pool = @($order) + @($probeOrder)
$selected = @($pool | Where-Object {
    $name = $_
    $match = @($Case | Where-Object { $name -like $_ }).Count -gt 0
    $match -and ($wantProbe -or ($probeOrder -notcontains $name))
})
if (-not $selected.Count) {
    throw "没有用例匹配 $($Case -join ',')，可选的有：$($order -join ', ')"
}

Initialize-Suite -Name 'office' -Fresh
Write-Log ("本轮用例：" + ($selected -join ', '))

$zombies = @(Get-ZombieOfficeProcess)
if ($zombies.Count) {
    Write-Log ("上一次跑剩下的无窗口 Office 进程：" +
        (($zombies | ForEach-Object { "$($_.Name)/$($_.Id)" }) -join ' ') +
        "。它们不影响本轮，但会白占内存，可以用 -CloseOffice 收尾避免。")
}

$broken = @()
foreach ($name in $selected) {
    $file = Join-Path $casesDir "$name.ps1"
    if (-not (Test-Path $file)) { throw "找不到用例文件 $file" }
    Write-Log "===== $name ====="
    try {
        & $file
    }
    catch {
        $broken += $name
        Add-Result "$name-CRASHED" 'FAIL' $_.Exception.Message
        Write-Log ("用例 $name 抛异常：" + $_.Exception.Message)
    }
}

Save-Results 'office-results.csv'
Close-OfficeApps -Quit:$CloseOffice
if ($CloseOffice) {
    Start-Sleep -Seconds 2
    $left = @(Get-Process EXCEL, WINWORD, POWERPNT -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -like '*Microsoft Office*' })
    Write-Log ("收尾后剩下的 Office 进程数：" + $left.Count)
}

$failCount = Show-Results
if ($broken.Count) {
    Write-Host ("抛异常的用例：" + ($broken -join ', '))
}
exit ([int]($failCount -gt 0))
