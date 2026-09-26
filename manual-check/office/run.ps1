# Microsoft Office 这一轮的驱动。
#
#   .\run.ps1                      全跑
#   .\run.ps1 -Case docx-apply     只跑一个
#   .\run.ps1 -Case xlsx-*         按通配符挑
#   .\run.ps1 -List                只看有哪些用例
#
# 每个用例套独立的 try/catch，一个塌了不影响后面的。
# 顺序有讲究：inspect 组不用开 Office，先跑；opc-resave 里 Word 可能挂住，放最后。

[CmdletBinding()]
param(
    [string[]]$Case = @('*'),
    [switch]$List
)

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', 'lib', 'bootstrap.ps1'))

$order = @(
    'xlsx-inspect'
    'docx-inspect'
    'xlsx-create'
    'docx-create'
    'xlsx-apply'
    'docx-apply'
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
$failCount = Show-Results
if ($broken.Count) {
    Write-Host ("抛异常的用例：" + ($broken -join ', '))
}
exit ([int]($failCount -gt 0))
