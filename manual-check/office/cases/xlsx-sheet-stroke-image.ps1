# xlsx sheet stroked solid rects. Gold = Excel 16.0 CopyPicture clipboard, no crop.
# Absolute via oneCell@A1 then rewrite to absoluteAnchor (insertShape has no posX).
# oneCell and twoCell kept as independent oracles (twoCell evidence not swapped).

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'office-com.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

function Set-PartText([string]$path, [string]$entryName, [scriptblock]$edit) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [System.IO.Compression.ZipFile]::Open($path, [System.IO.Compression.ZipArchiveMode]::Update)
    try {
        $entry = $zip.GetEntry($entryName)
        $reader = New-Object System.IO.StreamReader($entry.Open())
        try { $text = $reader.ReadToEnd() } finally { $reader.Dispose() }
        $text = & $edit $text
        $entry.Delete()
        $created = $zip.CreateEntry($entryName)
        $writer = New-Object System.IO.StreamWriter($created.Open())
        try { $writer.Write($text) } finally { $writer.Dispose() }
    }
    finally { $zip.Dispose() }
}

function Compare-Px([string]$name, [string]$gold, [string]$lib) {
    $diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
    $diff = Get-OutPath ($name + '-diff.png')
    $line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
    $text = ($line | Out-String).Trim()
    Write-Log ($name + '-px ' + $text)
    $ok = ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
    Add-Check ($name + '-px') $ok $text
}

function Export-And-Gold([string]$xlsx, [string]$range, [string]$stem) {
    $png = Get-OutPath ($stem + '-lib.png')
    if (Test-Path $png) { Remove-Item -Force $png }
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'exportSheetImage'
        path = $xlsx.Replace('\', '/'); format = 'xlsx'
        id = 'sheet:Sheet1'
        output = $png.Replace('\', '/')
    } ($stem + '-export') | Out-Null

    $excel = Ensure-MsExcel $xlsx
    $wb = Open-ExcelBook $excel $xlsx
    $gold = Get-OutPath ($stem + '-gold.png')
    try {
        $null = $wb.Activate()
        $ws = $wb.Worksheets.Item(1)
        $null = $ws.Activate()
        $excel.ActiveWindow.Zoom = 100
        $excel.ActiveWindow.DisplayGridlines = $false
        $ws.Range($range).CopyPicture(1, 2)
        Start-Sleep -Milliseconds 200
        $img = [System.Windows.Forms.Clipboard]::GetImage()
        if ($null -eq $img) { throw 'clipboard empty' }
        try { $img.Save($gold, [System.Drawing.Imaging.ImageFormat]::Png) }
        finally { $img.Dispose() }
        Release-Com $ws
    }
    finally {
        if ($wb) { $wb.Close($false); Release-Com $wb }
    }
    Compare-Px $stem $gold $png
}

function Convert-OneCellToAbsolute([string]$drawingXml) {
    # oneCell from A1: absolute pos = fromOff (sheet origin = A1 corner).
    $rx = [regex]'<xdr:oneCellAnchor>\s*<xdr:from>\s*<xdr:col>0</xdr:col>\s*<xdr:colOff>(\d+)</xdr:colOff>\s*<xdr:row>0</xdr:row>\s*<xdr:rowOff>(\d+)</xdr:rowOff>\s*</xdr:from>\s*<xdr:ext cx="(\d+)" cy="(\d+)"/>\s*(.*?)\s*<xdr:clientData/></xdr:oneCellAnchor>'
    $script:converted = 0
    $out = $rx.Replace($drawingXml, {
        param($m)
        $script:converted++
        $x = $m.Groups[1].Value
        $y = $m.Groups[2].Value
        $cx = $m.Groups[3].Value
        $cy = $m.Groups[4].Value
        $body = $m.Groups[5].Value
        "<xdr:absoluteAnchor><xdr:pos x=`"$x`" y=`"$y`"/><xdr:ext cx=`"$cx`" cy=`"$cy`"/>$body<xdr:clientData/></xdr:absoluteAnchor>"
    })
    if ($script:converted -lt 1) { throw 'no oneCell converted to absolute' }
    $out
}

$px = 9525

# --- absolute ---
$src = Get-OutPath 'xlsx-sheet-stroke-abs-src.xlsx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(
        @{ ref = 'A1'; value = '' }
        @{ ref = 'A2'; value = '' }
        @{ ref = 'A3'; value = '' }
        @{ ref = 'A4'; value = '' }
        @{ ref = 'A5'; value = '' }
        @{ ref = 'A6'; value = '' }
        @{ ref = 'A7'; value = '' }
        @{ ref = 'A8'; value = '' }
    ) }) }
} 'xlsx-sheet-stroke-abs-create' | Out-Null

$abs = Get-OutPath 'xlsx-sheet-stroke-abs.xlsx'
if (Test-Path $abs) { Remove-Item -Force $abs }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $abs.Replace('\', '/')
    ops = @(
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 1; height = 15 }
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 2; height = 15 }
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 3; height = 15 }
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 4; height = 15 }
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 5; height = 15 }
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 6; height = 15 }
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 7; height = 15 }
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 8; height = 15 }
        @{ op = 'insertShape'; id = 'sheet:Sheet1'; prst = 'rect'; anchor = 'oneCell'; from = 'A1'; fromOffX = (2 * $px); fromOffY = (2 * $px); cx = (12 * $px); cy = (12 * $px) }
        @{ op = 'insertShape'; id = 'sheet:Sheet1'; prst = 'rect'; anchor = 'oneCell'; from = 'A1'; fromOffX = (20 * $px); fromOffY = (3 * $px); cx = (24 * $px); cy = (8 * $px) }
        @{ op = 'insertShape'; id = 'sheet:Sheet1'; prst = 'rect'; anchor = 'oneCell'; from = 'A1'; fromOffX = (4 * $px); fromOffY = (20 * $px); cx = (8 * $px); cy = (20 * $px) }
        @{ op = 'insertShape'; id = 'sheet:Sheet1'; prst = 'rect'; anchor = 'oneCell'; from = 'A1'; fromOffX = (40 * $px); fromOffY = (4 * $px); cx = (16 * $px); cy = (16 * $px) }
        @{ op = 'insertShape'; id = 'sheet:Sheet1'; prst = 'rect'; anchor = 'oneCell'; from = 'A1'; fromOffX = (60 * $px); fromOffY = (22 * $px); cx = (20 * $px); cy = (10 * $px) }
        @{ op = 'insertShape'; id = 'sheet:Sheet1'; prst = 'rect'; anchor = 'oneCell'; from = 'A1'; fromOffX = (80 * $px + 4763); fromOffY = (6 * $px); cx = (10 * $px); cy = (10 * $px) }
    )
} 'xlsx-sheet-stroke-abs-apply' | Out-Null

Set-PartText $abs 'xl/worksheets/sheet1.xml' {
    param($text)
    if ($text -notmatch '<dimension [^>]*/>') { throw 'dimension missing' }
    $views = '<sheetViews><sheetView showGridLines="0" workbookViewId="0"/></sheetViews>'
    [regex]::Replace($text, '(<dimension [^>]*/>)', ('$1' + $views), 1)
}
Set-PartText $abs 'xl/drawings/drawing1.xml' {
    param($text)
    $text = $text.Replace('val="FFFFFF"', 'val="EE4455"')
    $script:i = 0
    $text = [regex]::Replace($text, 'w="9525"', {
        param($m)
        $script:i++
        if ($script:i -eq 4 -or $script:i -eq 5) { 'w="19050"' } else { $m.Value }
    })
    $text = Convert-OneCellToAbsolute $text
    # 6th absolute: blue fill red stroke
    $blocks = [regex]::Split($text, '(?=<xdr:absoluteAnchor>)')
    $out = New-Object System.Collections.Generic.List[string]
    $ai = 0
    foreach ($b in $blocks) {
        if ($b.StartsWith('<xdr:absoluteAnchor>')) {
            $ai++
            if ($ai -eq 6) {
                $b = $b.Replace('val="EE4455"', 'val="3366CC"')
                $b = $b.Replace('<a:solidFill><a:srgbClr val="000000"/></a:solidFill>', '<a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>')
            }
            $out.Add($b) | Out-Null
        } else { $out.Add($b) | Out-Null }
    }
    $text = [string]::Join('', $out)
    $extra = '<xdr:absoluteAnchor><xdr:pos x="' + [string](10 * $px) + '" y="' + [string](90 * $px) + '"/><xdr:ext cx="' + [string](14 * $px) + '" cy="' + [string](14 * $px) + '"/><xdr:sp macro="" textlink=""><xdr:nvSpPr><xdr:cNvPr id="99" name="strokeHollow"/><xdr:cNvSpPr/></xdr:nvSpPr><xdr:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="' + [string](14 * $px) + '" cy="' + [string](14 * $px) + '"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/><a:ln w="9525"><a:solidFill><a:srgbClr val="000000"/></a:solidFill></a:ln></xdr:spPr><xdr:style/><xdr:txBody><a:bodyPr/><a:lstStyle/><a:p/></xdr:txBody></xdr:sp><xdr:clientData/></xdr:absoluteAnchor>'
    $text.Replace('</xdr:wsDr>', ($extra + '</xdr:wsDr>'))
}

Export-And-Gold $abs 'A1:A8' 'xlsx-sheet-stroke-abs'

# --- oneCell ---
$src2 = Get-OutPath 'xlsx-sheet-stroke-onecell-src.xlsx'
if (Test-Path $src2) { Remove-Item -Force $src2 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src2.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(
        @{ ref = 'A1'; value = '' }
        @{ ref = 'A2'; value = '' }
        @{ ref = 'A3'; value = '' }
        @{ ref = 'A4'; value = '' }
    ) }) }
} 'xlsx-sheet-stroke-onecell-create' | Out-Null

$one = Get-OutPath 'xlsx-sheet-stroke-onecell.xlsx'
if (Test-Path $one) { Remove-Item -Force $one }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src2.Replace('\', '/')
    output = $one.Replace('\', '/')
    ops = @(
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 1; height = 15 }
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 2; height = 15 }
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 3; height = 15 }
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 4; height = 15 }
        @{ op = 'insertShape'; id = 'sheet:Sheet1'; prst = 'rect'; anchor = 'oneCell'; from = 'A1'; fromOffX = (2 * $px); fromOffY = (2 * $px); cx = (12 * $px); cy = (10 * $px) }
        @{ op = 'insertShape'; id = 'sheet:Sheet1'; prst = 'rect'; anchor = 'oneCell'; from = 'A2'; fromOffX = (5 * $px); fromOffY = (3 * $px); cx = (20 * $px); cy = (8 * $px) }
        @{ op = 'insertShape'; id = 'sheet:Sheet1'; prst = 'rect'; anchor = 'oneCell'; from = 'A3'; fromOffX = 4763; fromOffY = (2 * $px); cx = (16 * $px); cy = (8 * $px) }
    )
} 'xlsx-sheet-stroke-onecell-apply' | Out-Null

Set-PartText $one 'xl/worksheets/sheet1.xml' {
    param($text)
    if ($text -notmatch '<dimension [^>]*/>') { throw 'dimension missing' }
    $views = '<sheetViews><sheetView showGridLines="0" workbookViewId="0"/></sheetViews>'
    [regex]::Replace($text, '(<dimension [^>]*/>)', ('$1' + $views), 1)
}
Set-PartText $one 'xl/drawings/drawing1.xml' {
    param($text)
    $text = $text.Replace('val="FFFFFF"', 'val="EE4455"')
    $script:i = 0
    [regex]::Replace($text, 'w="9525"', {
        param($m)
        $script:i++
        if ($script:i -eq 2) { 'w="19050"' } else { $m.Value }
    })
}

Export-And-Gold $one 'A1:A4' 'xlsx-sheet-stroke-onecell'

# --- twoCell evidence ---
$src3 = Get-OutPath 'xlsx-sheet-stroke-twocell-src.xlsx'
if (Test-Path $src3) { Remove-Item -Force $src3 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src3.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(
        @{ ref = 'A1'; value = '' }
        @{ ref = 'B1'; value = '' }
        @{ ref = 'A2'; value = '' }
        @{ ref = 'B2'; value = '' }
        @{ ref = 'A3'; value = '' }
        @{ ref = 'B3'; value = '' }
    ) }) }
} 'xlsx-sheet-stroke-twocell-create' | Out-Null

$two = Get-OutPath 'xlsx-sheet-stroke-twocell.xlsx'
if (Test-Path $two) { Remove-Item -Force $two }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src3.Replace('\', '/')
    output = $two.Replace('\', '/')
    ops = @(
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 1; height = 15 }
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 2; height = 15 }
        @{ op = 'setRowHeight'; id = 'sheet:Sheet1'; row = 3; height = 15 }
        @{ op = 'insertShape'; id = 'sheet:Sheet1'; prst = 'rect'; anchor = 'twoCell'; from = 'A1'; to = 'B2'; fromOffX = (2 * $px); fromOffY = (2 * $px); toOffX = (-2 * $px); toOffY = (-2 * $px) }
    )
} 'xlsx-sheet-stroke-twocell-apply' | Out-Null

Set-PartText $two 'xl/worksheets/sheet1.xml' {
    param($text)
    if ($text -notmatch '<dimension [^>]*/>') { throw 'dimension missing' }
    $views = '<sheetViews><sheetView showGridLines="0" workbookViewId="0"/></sheetViews>'
    [regex]::Replace($text, '(<dimension [^>]*/>)', ('$1' + $views), 1)
}
Set-PartText $two 'xl/drawings/drawing1.xml' {
    param($text)
    $text = $text.Replace('val="FFFFFF"', 'val="EE4455"')
    if ($text -notmatch 'w="9525"') { throw 'stroke missing' }
    $text
}

Export-And-Gold $two 'A1:B3' 'xlsx-sheet-stroke-twocell'
