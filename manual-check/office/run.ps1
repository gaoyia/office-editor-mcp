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
    'docx-create'
    'docx-create-pictures'
    'docx-create-full'
    'pptx-create'
    'pptx-create-pictures'
    'pptx-create-charts'
    'xlsx-apply'
    'xlsx-pictures'
    'xlsx-charts'
    'xlsx-charts-remain'
    'xlsx-chartsheet'
    'xlsx-headerfooter'
    'docx-apply'
    'docx-body-para'
    'docx-new-header'
    'docx-section'
    'docx-table'
    'docx-revision'
    'docx-hyperlink'
    'docx-field'
    'docx-numbering'
    'docx-style'
    'docx-format'
    'docx-insert-section'
    'docx-toc'
    'docx-math'
    'docx-sdt'
    'docx-headerfooter'
    'docx-distance'
    'docx-link'
    'docx-picture'
    'docx-body-picture'
    'docx-textbox'
    'docx-notes'
    'docx-comments'
    'docx-charts'
    'docx-charts-ext'
    'docx-charts-remain'
    'docx-smartart'
    'docx-container-drawing'
    'docx-docprops'
    'docx-ole'
    'docx-protect'
    'docx-watermark'
    'xlsx-docprops'
    'xlsx-sheets'
    'xlsx-cellstyle'
    'xlsx-layout'
    'pptx-docprops'
    'pptx-apply'
    'pptx-pictures'
    'pptx-charts'
    'pptx-charts-remain'
    'pptx-smartart'
    'pptx-notes'
    'pptx-comments'
    'opc-resave'
)

$casesDir = Join-Path $PSScriptRoot 'cases'
if ($List) {
    $order | ForEach-Object { "  $_" }
    return
}

$selected = @($order | Where-Object { $name = $_; @($Case | Where-Object { $name -like $_ }).Count -gt 0 })
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
