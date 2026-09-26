# xlsx 工作表图：docs/checklist-xlsx.md 的 Xp1–Xp7。
# 源文件：Excel 插两张 twoCell，ooxp 补一张 oneCell，再经 Excel 打开（能 Save 则另存）。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Ensure-XlsxExcelPictureSource
$pngB = Ensure-OoxpPngB
$excel = Ensure-MsExcel $src
Close-AllWorkbooks $excel

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'xlsx'
} 'xlsx-pic-inspect-list'
$sheet0 = @($tree0.sheets) | Where-Object { $_.kind -eq 'worksheet' } | Select-Object -First 1
$noList = -not ($sheet0.PSObject.Properties.Name -contains 'pictures')
Add-Check 'xlsx-Xp1a' (($tree0.pictureCount -ge 2) -and ($sheet0.pictureCount -ge 2) -and $noList) "wb=$($tree0.pictureCount) sheet=$($sheet0.pictureCount)"

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-pic-inspect-expand'
$sheet = @($tree1.sheets) | Where-Object { $_.name -eq $sheet0.name } | Select-Object -First 1
$pics = @($sheet.pictures)
$anchors = @($pics | ForEach-Object { $_.anchor })
$hasOne = $anchors -contains 'oneCell'
$hasTwo = $anchors -contains 'twoCell'
$idOk = ($pics.Count -ge 2) -and ($pics[0].id -like 'sheet:*/pic:*')
Add-Check 'xlsx-Xp1' (($pics.Count -ge 2) -and $hasOne -and $hasTwo -and $idOk) "n=$($pics.Count) anchors=$($anchors -join ',') id=$($pics[0].id)"

$onePic = $pics | Where-Object { $_.anchor -eq 'oneCell' } | Select-Object -First 1
$twoPic = $pics | Where-Object { $_.anchor -eq 'twoCell' } | Select-Object -First 1
if (-not $onePic) { throw "源文件没有 oneCell，replace/size/anchor 无法继续" }
if (-not $twoPic) { Add-Check 'xlsx-Xp1c' $false 'missing twoCell in source' }
$sheetId = $sheet.id
if (-not $sheetId) { $sheetId = "sheet:$($sheet.name)" }

$replaced = Get-OutPath 'xlsx-pic-replaced.xlsx'
if (Test-Path $replaced) { Remove-Item $replaced -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{
        op = 'replacePicture'
        id = $onePic.id
        file = $pngB.Replace('\', '/')
    })
} 'xlsx-pic-replace' | Out-Null

$wbR = Open-ExcelBook $excel $replaced
$capR = [string]$wbR.Windows.Item(1).Caption
$sR = $wbR.Worksheets.Item($sheet.name)
$stillR = $sR.Shapes.Count
Add-Check 'xlsx-Xp2' (($capR -notmatch 'Repair') -and ($stillR -ge 2)) "caption=$capR shapes=$stillR"
Close-AllWorkbooks $excel

$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-pic-inspect-replaced'
$sheetR = @($treeR.sheets) | Where-Object { $_.name -eq $sheet.name } | Select-Object -First 1
$oneR = @($sheetR.pictures) | Where-Object { $_.id -eq $onePic.id } | Select-Object -First 1
Add-Check 'xlsx-Xp2b' (($oneR.anchor -eq $onePic.anchor) -and ($oneR.from -eq $onePic.from)) "anchor=$($oneR.anchor) from=$($oneR.from)"

$inserted = Get-OutPath 'xlsx-pic-inserted.xlsx'
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $replaced.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{
        op = 'insertPicture'
        id = $sheetId
        file = $pngB.Replace('\', '/')
        from = 'C12'
        cx = 914400
        cy = 914400
    })
} 'xlsx-pic-insert' | Out-Null

$wbI = Open-ExcelBook $excel $inserted
$capI = [string]$wbI.Windows.Item(1).Caption
$sI = $wbI.Worksheets.Item($sheet.name)
$nI = $sI.Shapes.Count
Add-Check 'xlsx-Xp3' (($capI -notmatch 'Repair') -and ($nI -gt $stillR)) "caption=$capI shapes=$nI"
Close-AllWorkbooks $excel

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-pic-inspect-inserted'
$sheetI = @($treeI.sheets) | Where-Object { $_.name -eq $sheet.name } | Select-Object -First 1
$oldIds = @($pics | ForEach-Object { $_.id })
$newPic = @($sheetI.pictures) | Where-Object { $oldIds -notcontains $_.id } | Select-Object -First 1
Add-Check 'xlsx-Xp3b' (($null -ne $newPic) -and ($newPic.anchor -eq 'oneCell') -and ($newPic.from -eq 'C12')) "id=$($newPic.id) from=$($newPic.from)"

$deleted = Get-OutPath 'xlsx-pic-deleted.xlsx'
if (Test-Path $deleted) { Remove-Item $deleted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $deleted.Replace('\', '/')
    ops = @(@{ op = 'deletePicture'; id = $newPic.id })
} 'xlsx-pic-delete' | Out-Null

$wbD = Open-ExcelBook $excel $deleted
$capD = [string]$wbD.Windows.Item(1).Caption
$sD = $wbD.Worksheets.Item($sheet.name)
$nD = $sD.Shapes.Count
Add-Check 'xlsx-Xp4' (($capD -notmatch 'Repair') -and ($nD -eq $stillR)) "caption=$capD shapes=$nD"
Close-AllWorkbooks $excel

$sized = Get-OutPath 'xlsx-pic-sized.xlsx'
if (Test-Path $sized) { Remove-Item $sized -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $deleted.Replace('\', '/')
    output = $sized.Replace('\', '/')
    ops = @(@{
        op = 'setPictureSize'
        id = $onePic.id
        cx = 1828800
        cy = 1828800
    })
} 'xlsx-pic-size' | Out-Null

$wbS = Open-ExcelBook $excel $sized
$capS = [string]$wbS.Windows.Item(1).Caption
$sS = $wbS.Worksheets.Item($sheet.name)
$shapesS = $sS.Shapes
$shapeS = $null
for ($i = 1; $i -le $shapesS.Count; $i++) {
    $cand = $shapesS.Item($i)
    if ([string]$cand.Name -eq [string]$onePic.name) {
        $shapeS = $cand
        break
    }
    Release-Com $cand
}
if (-not $shapeS) { throw "Excel 里找不到名为 $($onePic.name) 的图" }
$wPt = [double]$shapeS.Width
$hPt = [double]$shapeS.Height
# 144 磅 = 1828800 EMU；允许换算误差。
$sizeOk = ([math]::Abs($wPt - 144) -lt 8) -and ([math]::Abs($hPt - 144) -lt 8)
Add-Check 'xlsx-Xp5' (($capS -notmatch 'Repair') -and $sizeOk) "caption=$capS w=$wPt h=$hPt"
Release-Com $shapeS
Release-Com $shapesS
Close-AllWorkbooks $excel

$moved = Get-OutPath 'xlsx-pic-moved.xlsx'
if (Test-Path $moved) { Remove-Item $moved -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sized.Replace('\', '/')
    output = $moved.Replace('\', '/')
    ops = @(@{
        op = 'setPictureAnchor'
        id = $onePic.id
        from = 'F12'
    })
} 'xlsx-pic-anchor' | Out-Null

$wbM = Open-ExcelBook $excel $moved
$capM = [string]$wbM.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$treeM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $moved.Replace('\', '/'); format = 'xlsx'
    scope = @{ pictures = $true }
} 'xlsx-pic-inspect-moved'
$sheetM = @($treeM.sheets) | Where-Object { $_.name -eq $sheet.name } | Select-Object -First 1
$oneM = @($sheetM.pictures) | Where-Object { $_.id -eq $onePic.id } | Select-Object -First 1
Add-Check 'xlsx-Xp6' (($capM -notmatch 'Repair') -and ($oneM.from -eq 'F12')) "caption=$capM from=$($oneM.from)"

$exported = Get-OutPath 'xlsx-pic-export.png'
if (Test-Path $exported) { Remove-Item $exported -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $moved.Replace('\', '/')
    id = $onePic.id
    output = $exported.Replace('\', '/')
} 'xlsx-pic-export' | Out-Null

$outBytes = [IO.File]::ReadAllBytes($exported)
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::OpenRead($moved)
$matched = 0
$mediaLens = @()
try {
    foreach ($entry in @($zip.Entries | Where-Object { $_.FullName -like 'xl/media/*' })) {
        $ms = New-Object System.IO.MemoryStream
        $es = $entry.Open()
        $es.CopyTo($ms)
        $es.Dispose()
        $mediaBytes = $ms.ToArray()
        $ms.Dispose()
        $mediaLens += $mediaBytes.Length
        if ($mediaBytes.Length -ne $outBytes.Length) { continue }
        $sameEntry = $true
        for ($i = 0; $i -lt $outBytes.Length; $i++) {
            if ($outBytes[$i] -ne $mediaBytes[$i]) { $sameEntry = $false; break }
        }
        if ($sameEntry) { $matched++ }
    }
} finally {
    $zip.Dispose()
}
Add-Check 'xlsx-Xp7' ($matched -eq 1) "export=$($outBytes.Length) matched=$matched media=$($mediaLens -join ',')"
