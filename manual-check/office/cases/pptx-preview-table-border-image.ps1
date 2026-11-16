# 去掉表样式后的无文字纯色表，外圈和内边灰边。金样是 PowerPoint 16.0 的 Slide.Export。
# 红区回绘制引擎修，不要加大 MaxDelta。不混进合并单元格，也不把文字灰度算进通过。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function New-FillGrid([int]$rows, [int]$cols, [string[]]$colors) {
    $grid = @()
    for ($r = 0; $r -lt $rows; $r++) {
        $line = @()
        for ($c = 0; $c -lt $cols; $c++) {
            $color = $colors[($r * $cols + $c) % $colors.Length]
            $line += , @{ text = ''; fill = $color }
        }
        $grid += , @($line)
    }
    return , @($grid)
}

$specs = @(
    @{ tag = 'g2-aabb'; title = '2x2 192x96 at 384,96 on AABBCC'; x = 3657600; y = 914400; cx = 1828800; cy = 914400; rows = 2; cols = 2; colors = @('00AA00'); bg = 'AABBCC' }
    @{ tag = 'g2-dark'; title = '2x2 192x96 at 384,96 on 334455'; x = 3657600; y = 914400; cx = 1828800; cy = 914400; rows = 2; cols = 2; colors = @('00AA00'); bg = '334455' }
    @{ tag = 'g2-white'; title = '2x2 192x96 at 384,96 on FFFFFF'; x = 3657600; y = 914400; cx = 1828800; cy = 914400; rows = 2; cols = 2; colors = @('00AA00'); bg = 'FFFFFF' }
    @{ tag = 'g2-mix'; title = '2x2 mixed fills at 384,96 on AABBCC'; x = 3657600; y = 914400; cx = 1828800; cy = 914400; rows = 2; cols = 2; colors = @('00AA00', 'EE4455', '0055AA', 'F2C14E'); bg = 'AABBCC' }
    @{ tag = 'g1-64'; title = '1x1 64x64 at 192,96 on AABBCC'; x = 1828800; y = 914400; cx = 609600; cy = 609600; rows = 1; cols = 1; colors = @('00AA00'); bg = 'AABBCC' }
    @{ tag = 'row3'; title = '1x3 288x48 at 192,96 on AABBCC'; x = 1828800; y = 914400; cx = 2743200; cy = 457200; rows = 1; cols = 3; colors = @('00AA00', 'EE4455', '0055AA'); bg = 'AABBCC' }
    @{ tag = 'col3'; title = '3x1 48x144 at 192,96 on AABBCC'; x = 1828800; y = 914400; cx = 457200; cy = 1371600; rows = 3; cols = 1; colors = @('00AA00', 'EE4455', '0055AA'); bg = 'AABBCC' }
    @{ tag = 'xy10'; title = '2x2 192x96 at 10,200 on AABBCC'; x = 95250; y = 1905000; cx = 1828800; cy = 914400; rows = 2; cols = 2; colors = @('00AA00'); bg = 'AABBCC' }
    @{ tag = 'odd'; title = '2x2 192x96 at 193,97 on AABBCC'; x = 1838325; y = 923925; cx = 1828800; cy = 914400; rows = 2; cols = 2; colors = @('00AA00'); bg = 'AABBCC' }
    @{ tag = 'wide'; title = '3x2 256x128 at 192,96 on AABBCC'; x = 1828800; y = 914400; cx = 2438400; cy = 1219200; rows = 2; cols = 3; colors = @('00AA00', 'EE4455', '0055AA', 'F2C14E', '7A3E9D', '2A9D8F'); bg = 'AABBCC' }
    @{ tag = 'red-dark'; title = '2x2 192x96 fill EE4455 on 334455'; x = 3657600; y = 914400; cx = 1828800; cy = 914400; rows = 2; cols = 2; colors = @('EE4455'); bg = '334455' }
)

$src = Get-OutPath 'pptx-preview-table-border-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
$slides = @()
foreach ($s in $specs) {
    $tbl = @{
        rows         = $s.rows
        cols         = $s.cols
        x            = $s.x
        y            = $s.y
        cx           = $s.cx
        cy           = $s.cy
        tableStyleId = $null
        tblLook      = $null
        cells        = (New-FillGrid $s.rows $s.cols $s.colors)
    }
    if ($s.colWidths) { $tbl.colWidths = $s.colWidths }
    if ($s.rowHeights) { $tbl.rowHeights = $s.rowHeights }
    $slides += @{ tables = @($tbl) }
}
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'create'
    path          = $src.Replace('\', '/')
    presentation  = @{ slides = $slides }
} 'pptx-preview-table-border-create' | Out-Null

$ops = @()
for ($i = 0; $i -lt $specs.Count; $i++) {
    $ops += @{ op = 'setPageBackground'; id = ('slide:{0}' -f ($i + 1)); color = $specs[$i].bg }
}
$deck = Get-OutPath 'pptx-preview-table-border.pptx'
if (Test-Path $deck) { Remove-Item -Force $deck }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'apply'
    path          = $src.Replace('\', '/')
    output        = $deck.Replace('\', '/')
    ops           = $ops
} 'pptx-preview-table-border-apply' | Out-Null

$ppt = Ensure-MsPpt $deck
$diffScript = [IO.Path]::Combine($PSScriptRoot, '..', '..', 'pixel-diff.ps1')
for ($i = 0; $i -lt $specs.Count; $i++) {
    $s = $specs[$i]
    $n = $i + 1
    $lib = Get-OutPath ('pptx-preview-table-border-{0}-lib.png' -f $s.tag)
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'exportSlideImage'
        path          = $deck.Replace('\', '/')
        format        = 'pptx'
        id            = ('slide:{0}' -f $n)
        output        = $lib.Replace('\', '/')
    } ('pptx-preview-table-border-{0}-export' -f $s.tag) | Out-Null

    $gold = Get-OutPath ('pptx-preview-table-border-{0}-gold.png' -f $s.tag)
    $pres = $null
    $slide = $null
    try {
        $pres = Open-PptPres $ppt $deck
        $cap = Get-PptCaption $pres
        $slide = Get-PptSlide $pres $n
        if (Test-Path $gold) { Remove-Item -Force $gold }
        $slide.Export($gold, 'PNG')
        $ok = $cap -notmatch 'Repair'
        Add-Check ('pptx-preview-table-border-{0}-open' -f $s.tag) $ok ('caption={0} {1}' -f $cap, $s.title)
    }
    finally {
        Release-Com $slide
        Close-PptPres $pres
    }

    $diff = Get-OutPath ('pptx-preview-table-border-{0}-diff.png' -f $s.tag)
    $line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
    $code = $LASTEXITCODE
    $text = ($line | Out-String).Trim()
    Write-Log ('pptx-preview-table-border-{0}-px exit={1} {2}' -f $s.tag, $code, $text)
    $pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
    Add-Check ('pptx-preview-table-border-{0}-px' -f $s.tag) $pxOk ('exit={0} {1}' -f $code, $text)
}
