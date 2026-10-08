# xlsx create 带批注、超链接、形状、SmartArt：docs/checklist-xlsx.md 的 C17–C21。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function Read-Threaded($sheet) {
    $text = ''
    $n = 0
    $col = $null
    try { $col = $sheet.CommentsThreaded } catch { }
    if ($null -eq $col) {
        return @{ Count = 0; Text = '' }
    }
    try { $n = [int]$col.Count } catch { }
    if ($n -ge 1) {
        $item = $null
        try { $item = $col.Item(1) } catch { }
        if ($null -ne $item) {
            try { $text = [string]$item.Text } catch { }
            Release-Com $item
        }
    }
    Release-Com $col
    return @{ Count = $n; Text = $text }
}

function Read-SheetLinks($sheet) {
    $n = 0
    $addr = ''
    $col = $null
    try { $col = $sheet.Hyperlinks } catch { }
    if ($null -eq $col) {
        return @{ Count = 0; Address = '' }
    }
    try { $n = [int]$col.Count } catch { }
    if ($n -ge 1) {
        $h = $null
        try { $h = $col.Item(1) } catch { }
        if ($null -ne $h) {
            try { $addr = [string]$h.Address } catch { }
            Release-Com $h
        }
    }
    Release-Com $col
    return @{ Count = $n; Address = $addr }
}

function Read-SheetDrawing($sheet) {
    $artN = 0
    $n = 0
    $shapes = $null
    try { $shapes = $sheet.Shapes } catch { }
    if ($null -eq $shapes) {
        return @{ Count = 0; ArtCount = 0 }
    }
    try { $n = [int]$shapes.Count } catch { }
    for ($i = 1; $i -le $n; $i++) {
        $shp = $null
        try { $shp = $shapes.Item($i) } catch { }
        if ($null -eq $shp) { continue }
        try {
            $hasArt = $false
            try { $hasArt = [bool]$shp.HasSmartArt } catch { }
            $ty = 0
            try { $ty = [int]$shp.Type } catch { }
            if ($hasArt -or ($ty -eq 24)) { $artN++ }
        } finally { Release-Com $shp }
    }
    Release-Com $shapes
    return @{ Count = $n; ArtCount = $artN }
}

$path = Ensure-XlsxCreateDrawing
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'xlsx'
    scope = @{
        comments = $true
        hyperlinks = $true
        shapes = $true
        smartArt = $true
    }
} 'xlsx-create-drawing-inspect'

$sheet = @($tree.sheets) | Where-Object { $_.name -eq 'Sheet1' } | Select-Object -First 1
$cm = @($sheet.comments) | Select-Object -First 1
$c17ok = ($null -ne $cm) -and ($cm.ref -eq 'A1') -and ([string]$cm.text -match 'C17批注')
Add-Check 'xlsx-C17' $c17ok "ref=$($cm.ref) text=$($cm.text)"

$link = @($sheet.hyperlinks) | Select-Object -First 1
$c18ok = ($null -ne $link) -and ($link.id -eq 'sheet:Sheet1/link:1') -and ([string]$link.url -match 'example.com')
Add-Check 'xlsx-C18' $c18ok "id=$($link.id) url=$($link.url)"

$shp = @($sheet.shapes) | Select-Object -First 1
$c19ok = ($null -ne $shp) -and ($shp.from -eq 'B4') -and ($shp.prst -eq 'ellipse')
Add-Check 'xlsx-C19' $c19ok "from=$($shp.from) prst=$($shp.prst) id=$($shp.id)"

$art = @($sheet.smartArts) | Select-Object -First 1
$c20ok = ($null -ne $art) -and ($art.from -eq 'D6') -and ([string]$art.layout -match 'layout/default')
Add-Check 'xlsx-C20' $c20ok "from=$($art.from) layout=$($art.layout)"

$excel = Ensure-MsExcel $path
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $path
$cap = [string]$wb.Windows.Item(1).Caption
$s1 = $wb.Worksheets.Item('Sheet1')
$th = Read-Threaded $s1
$hl = Read-SheetLinks $s1
$dr = Read-SheetDrawing $s1
Release-Com $s1
Close-AllWorkbooks $excel
$c21ok = ($cap -notmatch 'Repair') -and ($th.Count -ge 1) -and ($hl.Count -ge 1) -and ($dr.Count -ge 4) -and ($dr.ArtCount -ge 1)
Add-Check 'xlsx-C21' $c21ok "caption=$cap comments=$($th.Count) links=$($hl.Count) shapes=$($dr.Count) art=$($dr.ArtCount)"
