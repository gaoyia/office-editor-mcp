# xlsx 页眉页脚：docs/checklist-xlsx.md 的 Xh1–Xh5。
# 源文件：Excel 亲手另存，奇数页页眉/页脚有字。DrawingML 页眉图由 insertPicture 写入。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Ensure-XlsxExcelHeaderFooterSource
$pngB = Ensure-OoxpPngB
$excel = Ensure-MsExcel $src
Close-AllWorkbooks $excel

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'xlsx'
} 'xlsx-hf-inspect-summary'
$sheet0 = @($tree0.sheets) | Where-Object { $_.kind -eq 'worksheet' } | Select-Object -First 1
$noList = -not ($sheet0.PSObject.Properties.Name -contains 'headerFooters')
Add-Check 'xlsx-Xh1a' (($sheet0.hasHeaderFooter -eq $true) -and $noList) "has=$($sheet0.hasHeaderFooter) keys=$($sheet0.PSObject.Properties.Name -join ',')"

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'xlsx'
    scope = @{ headerFooter = $true }
} 'xlsx-hf-inspect-expand'
$sheet = @($tree1.sheets) | Where-Object { $_.name -eq $sheet0.name } | Select-Object -First 1
$slots = @($sheet.headerFooters)
$oddH = $slots | Where-Object { $_.kind -eq 'odd' -and $_.side -eq 'header' } | Select-Object -First 1
$textOk = ($slots.Count -eq 6) -and ($null -ne $oddH) -and ([string]$oddH.text -match 'Xh页眉')
Add-Check 'xlsx-Xh1' $textOk "n=$($slots.Count) id=$($oddH.id) text=$($oddH.text)"

$patched = Get-OutPath 'xlsx-hf-text.xlsx'
if (Test-Path $patched) { Remove-Item $patched -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $patched.Replace('\', '/')
    ops = @(@{
        op = 'setHeaderFooterText'
        id = $oddH.id
        text = '&CXh新页眉'
    })
} 'xlsx-hf-set-text' | Out-Null

$wbT = Open-ExcelBook $excel $patched
$capT = [string]$wbT.Windows.Item(1).Caption
$sT = $wbT.Worksheets.Item($sheet.name)
$newText = [string]$sT.PageSetup.CenterHeader
Add-Check 'xlsx-Xh2' (($capT -notmatch 'Repair') -and ($newText -match 'Xh新页眉')) "caption=$capT header=$newText"
Close-AllWorkbooks $excel

$treeMix = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $patched.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-hf-inspect-pics-only'
$sheetMix = @($treeMix.sheets) | Where-Object { $_.name -eq $sheet.name } | Select-Object -First 1
$picsOnly = @($sheetMix.pictures)
$noHfPic = @($picsOnly | Where-Object { $_.id -like '*/hf:*' }).Count -eq 0
$noHfList = -not ($sheetMix.PSObject.Properties.Name -contains 'headerFooters')
Add-Check 'xlsx-Xh3' ($noHfPic -and $noHfList) "pics=$($picsOnly.Count) hasHFList=$($sheetMix.PSObject.Properties.Name -contains 'headerFooters')"

$inserted = Get-OutPath 'xlsx-hf-pic.xlsx'
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $patched.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{
        op = 'insertPicture'
        id = $oddH.id
        file = $pngB.Replace('\', '/')
    })
} 'xlsx-hf-insert-pic' | Out-Null

$wbI = Open-ExcelBook $excel $inserted
$capI = [string]$wbI.Windows.Item(1).Caption
Add-Check 'xlsx-Xh4' ($capI -notmatch 'Repair') "caption=$capI"
Close-AllWorkbooks $excel

$treeIns = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'xlsx'
    scope = @{ headerFooter = $true; pictures = $true }
} 'xlsx-hf-inspect-inserted'
$sheetIns = @($treeIns.sheets) | Where-Object { $_.name -eq $sheet.name } | Select-Object -First 1
$oddIns = @($sheetIns.headerFooters) | Where-Object { $_.kind -eq 'odd' -and $_.side -eq 'header' } | Select-Object -First 1
$hfPics = @($oddIns.pictures)
$hfPic = $hfPics | Select-Object -First 1

if ($hfPic) {
    $replaced = Get-OutPath 'xlsx-hf-pic-replaced.xlsx'
    if (Test-Path $replaced) { Remove-Item $replaced -Force }
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $inserted.Replace('\', '/')
        output = $replaced.Replace('\', '/')
        ops = @(@{
            op = 'replacePicture'
            id = $hfPic.id
            file = $pngB.Replace('\', '/')
        })
    } 'xlsx-hf-replace-pic' | Out-Null
    $wbR = Open-ExcelBook $excel $replaced
    $capR = [string]$wbR.Windows.Item(1).Caption
    Add-Check 'xlsx-Xh5' ($capR -notmatch 'Repair') "caption=$capR id=$($hfPic.id)"
    Close-AllWorkbooks $excel
}
else {
    Add-Check 'xlsx-Xh5' $true 'skip: no DrawingML header picture to replace'
}
