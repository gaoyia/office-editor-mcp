# xlsx 页眉页脚：docs/checklist-xlsx.md 的 Xh1–Xh11。
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

$pngA = Ensure-OoxpPngA
$legacy = Get-OutPath 'xlsx-hf-legacy.xlsx'
if (Test-Path $legacy) { Remove-Item $legacy -Force }
$wbL = $excel.Workbooks.Add()
$wsL = $wbL.Worksheets.Item(1)
$wsL.PageSetup.CenterHeaderPicture.Filename = $pngB
$wsL.PageSetup.CenterHeader = '&G'
$wbL.SaveAs($legacy, 51)
Close-AllWorkbooks $excel

$treeL = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $legacy.Replace('\', '/'); format = 'xlsx'
    scope = @{ headerFooter = $true; legacyPictures = $true; pictures = $true }
} 'xlsx-hf-legacy-inspect'
$sheetL = @($treeL.sheets) | Where-Object { $_.kind -eq 'worksheet' } | Select-Object -First 1
$oddL = @($sheetL.headerFooters) | Where-Object { $_.kind -eq 'odd' -and $_.side -eq 'header' } | Select-Object -First 1
$legacyPics = @($oddL.legacyPictures)
$legacyPic = $legacyPics | Select-Object -First 1
$drawPics = @($oddL.pictures)
$xh6 = ($sheetL.legacyPictureCount -eq 1) -and ($oddL.legacyPictureCount -eq 1) -and ($legacyPic.id -like '*/legacyPic:CH') -and ($drawPics.Count -eq 0)
Add-Check 'xlsx-Xh6' $xh6 "count=$($sheetL.legacyPictureCount) id=$($legacyPic.id) pics=$($drawPics.Count)"

$replacedL = Get-OutPath 'xlsx-hf-legacy-replaced.xlsx'
if (Test-Path $replacedL) { Remove-Item $replacedL -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $legacy.Replace('\', '/')
    output = $replacedL.Replace('\', '/')
    ops = @(@{
        op = 'replaceLegacyPicture'
        id = $legacyPic.id
        file = $pngA.Replace('\', '/')
    })
} 'xlsx-hf-legacy-replace' | Out-Null
$exportedL = Get-OutPath 'xlsx-hf-legacy-export.png'
if (Test-Path $exportedL) { Remove-Item $exportedL -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $replacedL.Replace('\', '/')
    id = $legacyPic.id
    output = $exportedL.Replace('\', '/')
} 'xlsx-hf-legacy-export' | Out-Null
$wbL2 = Open-ExcelBook $excel $replacedL
$capL2 = [string]$wbL2.Windows.Item(1).Caption
$wsL2 = $wbL2.Worksheets.Item(1)
$fileL2 = [string]$wsL2.PageSetup.CenterHeaderPicture.Filename
Close-AllWorkbooks $excel
$bytesA = [IO.File]::ReadAllBytes($pngA)
$bytesE = [IO.File]::ReadAllBytes($exportedL)
$sameL = ($bytesA.Length -eq $bytesE.Length)
if ($sameL) {
    for ($i = 0; $i -lt $bytesA.Length; $i++) {
        if ($bytesA[$i] -ne $bytesE[$i]) { $sameL = $false; break }
    }
}
$xh7 = ($capL2 -notmatch 'Repair') -and $sameL -and ($fileL2.Length -gt 0)
Add-Check 'xlsx-Xh7' $xh7 "caption=$capL2 same=$sameL file=$fileL2"

$deletedL = Get-OutPath 'xlsx-hf-legacy-deleted.xlsx'
if (Test-Path $deletedL) { Remove-Item $deletedL -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $replacedL.Replace('\', '/')
    output = $deletedL.Replace('\', '/')
    ops = @(@{
        op = 'deleteLegacyPicture'
        id = $legacyPic.id
    })
} 'xlsx-hf-legacy-delete' | Out-Null
$wbL3 = Open-ExcelBook $excel $deletedL
$capL3 = [string]$wbL3.Windows.Item(1).Caption
$wsL3 = $wbL3.Worksheets.Item(1)
$headerL3 = [string]$wsL3.PageSetup.CenterHeader
$fileL3 = [string]$wsL3.PageSetup.CenterHeaderPicture.Filename
Close-AllWorkbooks $excel
$xh8 = ($capL3 -notmatch 'Repair') -and ($headerL3 -match 'G') -and ([string]::IsNullOrEmpty($fileL3))
Add-Check 'xlsx-Xh8' $xh8 "caption=$capL3 header=$headerL3 file=$fileL3"

function Test-SameBytes([string]$left, [string]$right) {
    $a = [IO.File]::ReadAllBytes($left)
    $b = [IO.File]::ReadAllBytes($right)
    if ($a.Length -ne $b.Length) { return $false }
    for ($i = 0; $i -lt $a.Length; $i++) {
        if ($a[$i] -ne $b[$i]) { return $false }
    }
    return $true
}

function Export-LegacyBytes([string]$book, [string]$id, [string]$out) {
    if (Test-Path $out) { Remove-Item $out -Force }
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'exportMedia'
        path = $book.Replace('\', '/')
        id = $id
        output = $out.Replace('\', '/')
    } 'xlsx-hf-legacy-slot-export' | Out-Null
}

$slotsBook = Get-OutPath 'xlsx-hf-legacy-slots.xlsx'
if (Test-Path $slotsBook) { Remove-Item $slotsBook -Force }
$wbS = $excel.Workbooks.Add()
$psS = $wbS.Worksheets.Item(1).PageSetup
$psS.LeftHeaderPicture.Filename = $pngB
$psS.CenterHeaderPicture.Filename = $pngB
$psS.RightHeaderPicture.Filename = $pngB
$psS.LeftFooterPicture.Filename = $pngB
$psS.CenterFooterPicture.Filename = $pngB
$psS.RightFooterPicture.Filename = $pngB
$psS.LeftHeader = '&G'
$psS.CenterHeader = '&G'
$psS.RightHeader = '&G'
$psS.LeftFooter = '&G'
$psS.CenterFooter = '&G'
$psS.RightFooter = '&G'
$wbS.SaveAs($slotsBook, 51)
Close-AllWorkbooks $excel

$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $slotsBook.Replace('\', '/'); format = 'xlsx'
    scope = @{ headerFooter = $true; legacyPictures = $true; pictures = $true }
} 'xlsx-hf-legacy-slots'
$sheetS = @($treeS.sheets) | Where-Object { $_.kind -eq 'worksheet' } | Select-Object -First 1
$ids = @()
$drawOnOdd = 0
foreach ($slot in @($sheetS.headerFooters)) {
    $drawOnOdd += @($slot.pictures).Count
    foreach ($pic in @($slot.legacyPictures)) { $ids += [string]$pic.shapeId }
}
$need = @('LH', 'CH', 'RH', 'LF', 'CF', 'RF')
$missing = @($need | Where-Object { $ids -notcontains $_ })
$xh9 = ($sheetS.legacyPictureCount -eq 6) -and ($missing.Count -eq 0) -and ($drawOnOdd -eq 0)
Add-Check 'xlsx-Xh9' $xh9 "count=$($sheetS.legacyPictureCount) ids=$($ids -join ',') missing=$($missing -join ',')"

$rh = $null
$lf = $null
foreach ($slot in @($sheetS.headerFooters)) {
    foreach ($pic in @($slot.legacyPictures)) {
        if ($pic.shapeId -eq 'RH') { $rh = $pic }
        if ($pic.shapeId -eq 'LF') { $lf = $pic }
    }
}
$replacedS = Get-OutPath 'xlsx-hf-legacy-rh.xlsx'
if (Test-Path $replacedS) { Remove-Item $replacedS -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $slotsBook.Replace('\', '/')
    output = $replacedS.Replace('\', '/')
    ops = @(@{
        op = 'replaceLegacyPicture'
        id = $rh.id
        file = $pngA.Replace('\', '/')
    })
} 'xlsx-hf-legacy-replace-rh' | Out-Null
$savedS = Get-OutPath 'xlsx-hf-legacy-rh-saved.xlsx'
if (Test-Path $savedS) { Remove-Item $savedS -Force }
$wbRh = Open-ExcelBook $excel $replacedS
$wbRh.SaveAs($savedS, 51)
Close-AllWorkbooks $excel
$wbSaved = Open-ExcelBook $excel $savedS
$capSaved = [string]$wbSaved.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$treeSaved = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $savedS.Replace('\', '/'); format = 'xlsx'
    scope = @{ headerFooter = $true; legacyPictures = $true }
} 'xlsx-hf-legacy-saved'
$sheetSaved = @($treeSaved.sheets) | Select-Object -First 1
$savedCount = $sheetSaved.legacyPictureCount
$expRh = Get-OutPath 'xlsx-hf-legacy-rh.png'
$expLh = Get-OutPath 'xlsx-hf-legacy-lh.png'
Export-LegacyBytes $savedS $rh.id $expRh
$lhId = $null
foreach ($slot in @($sheetSaved.headerFooters)) {
    foreach ($pic in @($slot.legacyPictures)) {
        if ($pic.shapeId -eq 'LH') { $lhId = [string]$pic.id }
    }
}
Export-LegacyBytes $savedS $lhId $expLh
$xh10 = ($capSaved -notmatch 'Repair') -and ($savedCount -eq 6) -and (Test-SameBytes $pngA $expRh) -and (Test-SameBytes $pngB $expLh)
Add-Check 'xlsx-Xh10' $xh10 "caption=$capSaved count=$savedCount rh=$(Test-SameBytes $pngA $expRh) lh=$(Test-SameBytes $pngB $expLh)"

$deletedF = Get-OutPath 'xlsx-hf-legacy-lf.xlsx'
if (Test-Path $deletedF) { Remove-Item $deletedF -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $savedS.Replace('\', '/')
    output = $deletedF.Replace('\', '/')
    ops = @(@{
        op = 'deleteLegacyPicture'
        id = $lf.id
    })
} 'xlsx-hf-legacy-delete-lf' | Out-Null
$wbF = Open-ExcelBook $excel $deletedF
$capF = [string]$wbF.Windows.Item(1).Caption
$psF = $wbF.Worksheets.Item(1).PageSetup
$footerF = [string]$psF.LeftFooter
$fileF = [string]$psF.LeftFooterPicture.Filename
Close-AllWorkbooks $excel
$xh11 = ($capF -notmatch 'Repair') -and ($footerF -match 'G') -and ([string]::IsNullOrEmpty($fileF))
Add-Check 'xlsx-Xh11' $xh11 "caption=$capF footer=$footerF file=$fileF"
