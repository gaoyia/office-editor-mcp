# 多段/空段/段前后/缩进 layout 矩阵。金样 Word 16.0 PDF→96dpi。
# layout=逐行墨迹几何；px=整页 MaxDelta=0（灰阶仍 FAIL 照实）；页面数独立。
# 显式 line/lineRule 不在本矩阵（库侧 NI）。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'office-com.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.Drawing

function Get-InkBands([string]$path) {
    $bmp = [System.Drawing.Bitmap]::FromFile($path)
    try {
        $w = $bmp.Width; $h = $bmp.Height
        $rect = New-Object System.Drawing.Rectangle 0, 0, $w, $h
        $data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
        try {
            $bytes = New-Object byte[] ($data.Stride * $h)
            [Runtime.InteropServices.Marshal]::Copy($data.Scan0, $bytes, 0, $bytes.Length)
            $rowInk = New-Object bool[] $h
            for ($y = 0; $y -lt $h; $y++) {
                $row = $y * $data.Stride
                for ($x = 0; $x -lt $w; $x++) {
                    $o = $row + ($x * 3)
                    if ($bytes[$o] -ne 255 -or $bytes[($o + 1)] -ne 255 -or $bytes[($o + 2)] -ne 255) {
                        $rowInk[$y] = $true
                        break
                    }
                }
            }
            $bands = New-Object System.Collections.Generic.List[object]
            $start = -1
            for ($y = 0; $y -le $h; $y++) {
                $ink = ($y -lt $h) -and $rowInk[$y]
                if ($ink -and $start -lt 0) { $start = $y }
                elseif (-not $ink -and $start -ge 0) {
                    $gap = 0; $yy = $y
                    while ($yy -lt $h -and -not $rowInk[$yy] -and $gap -lt 3) { $gap++; $yy++ }
                    if ($yy -lt $h -and $rowInk[$yy] -and $gap -lt 3) { continue }
                    $end = $y - 1
                    $minX = $w; $maxX = -1
                    for ($yy = $start; $yy -le $end; $yy++) {
                        $row = $yy * $data.Stride
                        for ($x = 0; $x -lt $w; $x++) {
                            $o = $row + ($x * 3)
                            if ($bytes[$o] -ne 255 -or $bytes[($o + 1)] -ne 255 -or $bytes[($o + 2)] -ne 255) {
                                if ($x -lt $minX) { $minX = $x }
                                if ($x -gt $maxX) { $maxX = $x }
                            }
                        }
                    }
                    $bands.Add([pscustomobject]@{ y0 = $start; y1 = $end; x0 = $minX; x1 = $maxX; w = ($maxX - $minX + 1); h = ($end - $start + 1) })
                    $start = -1
                }
            }
            return $bands
        }
        finally { $bmp.UnlockBits($data) }
    }
    finally { $bmp.Dispose() }
}

function New-MpDoc($tag, $paras, $opsSpec) {
    $src = Get-OutPath ('docx-mp-{0}-src.docx' -f $tag)
    if (Test-Path $src) { Remove-Item -Force $src }
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'create'
        path          = $src.Replace('\', '/')
        document      = @{ paragraphs = $paras }
    } ('docx-mp-{0}-create' -f $tag) | Out-Null
    if (-not $opsSpec -or $opsSpec.Count -eq 0) { return $src }
    $tree = Invoke-OoxpTree @{
        schemaVersion = 1
        action        = 'inspect'
        path          = $src.Replace('\', '/')
        format        = 'docx'
        scope         = @{ from = 1; maxParas = 20 }
    } ('docx-mp-{0}-inspect' -f $tag)
    $ops = @()
    foreach ($spec in $opsSpec) {
        $o = @{}
        foreach ($k in $spec.Keys) { $o[$k] = $spec[$k] }
        if ($o.ContainsKey('paraIndex')) {
            $idx = [int]$o['paraIndex']
            $o.Remove('paraIndex')
            $o['id'] = [string]$tree.paragraphs[$idx - 1].id
            if ($o.ContainsKey('runSz')) {
                $sz = [int]$o['runSz']
                $o.Remove('runSz')
                $ops += , @{ op = 'setRunFormat'; id = ($o['id'] + '/run:1'); sz = $sz }
            }
        }
        $ops += , $o
    }
    $deck = Get-OutPath ('docx-mp-{0}.docx' -f $tag)
    if (Test-Path $deck) { Remove-Item -Force $deck }
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'apply'
        path          = $src.Replace('\', '/')
        output        = $deck.Replace('\', '/')
        ops           = $ops
    } ('docx-mp-{0}-apply' -f $tag) | Out-Null
    return $deck
}

$pngScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pdf-to-png.ps1'))
$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))

$specs = @(
    @{ tag = 'two'; paras = @(@{ text = 'A' }, @{ text = 'B' }); ops = @(); pages = 1 }
    @{ tag = 'after240'; paras = @(@{ text = 'A' }, @{ text = 'B' }); ops = @(@{ op = 'setParaFormat'; paraIndex = 1; after = 240 }); pages = 1 }
    @{ tag = 'before240b'; paras = @(@{ text = 'A' }, @{ text = 'B' }); ops = @(@{ op = 'setParaFormat'; paraIndex = 2; before = 240 }); pages = 1 }
    @{ tag = 'emptymid'; paras = @(@{ text = 'A' }, @{ text = '' }, @{ text = 'B' }); ops = @(); pages = 1 }
    @{ tag = 'indentb'; paras = @(@{ text = 'A' }, @{ text = 'B' }); ops = @(@{ op = 'setParaFormat'; paraIndex = 2; left = 720 }); pages = 1 }
    @{ tag = 'before240a'; paras = @(@{ text = 'A' }); ops = @(@{ op = 'setParaFormat'; paraIndex = 1; before = 240 }); pages = 1 }
)

$blocked = $false
foreach ($spec in $specs) {
    $tag = $spec.tag
    Write-Host ("===== {0} =====" -f $tag)
    if ($blocked) {
        Add-Result ("docx-mp-{0}-pdf" -f $tag) 'FAIL' 'blocked prior Word PDF'
        continue
    }
    $docPath = New-MpDoc $tag $spec.paras $spec.ops
    $lib = Get-OutPath ('docx-mp-{0}-lib.png' -f $tag)
    $pdf = Get-OutPath ('docx-mp-{0}.pdf' -f $tag)
    $gold = Get-OutPath ('docx-mp-{0}-gold.png' -f $tag)
    $diff = Get-OutPath ('docx-mp-{0}-diff.png' -f $tag)
    foreach ($p in @($lib, $pdf, $gold, $diff)) { if (Test-Path $p) { Remove-Item -Force $p } }

    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'exportPageImage'
        path          = $docPath.Replace('\', '/')
        id            = 'page:1'
        output        = $lib.Replace('\', '/')
    } ('docx-mp-{0}-lib' -f $tag) | Out-Null

    # 页面数：库是否接受 page:N
    $wantPages = [int]$spec.pages
    $pageOk = $true
    for ($pi = 1; $pi -le $wantPages; $pi++) {
        $probe = Get-OutPath ('docx-mp-{0}-p{1}-probe.png' -f $tag, $pi)
        if (Test-Path $probe) { Remove-Item -Force $probe }
        try {
            Invoke-OoxpRequest @{
                schemaVersion = 1
                action        = 'exportPageImage'
                path          = $docPath.Replace('\', '/')
                id            = ('page:{0}' -f $pi)
                output        = $probe.Replace('\', '/')
            } ('docx-mp-{0}-pageprobe-{1}' -f $tag, $pi) | Out-Null
            if (-not (Test-Path $probe)) { $pageOk = $false }
        } catch { $pageOk = $false }
    }
    $over = Get-OutPath ('docx-mp-{0}-over.png' -f $tag)
    if (Test-Path $over) { Remove-Item -Force $over }
    $overRejected = $false
    try {
        Invoke-OoxpRequest @{
            schemaVersion = 1
            action        = 'exportPageImage'
            path          = $docPath.Replace('\', '/')
            id            = ('page:{0}' -f ($wantPages + 1))
            output        = $over.Replace('\', '/')
        } ('docx-mp-{0}-over' -f $tag) | Out-Null
        if (Test-Path $over) { $overRejected = $false } else { $overRejected = $true }
    } catch { $overRejected = $true }
    Add-Result ("docx-mp-{0}-pages" -f $tag) $(if ($pageOk -and $overRejected) { 'PASS' } else { 'FAIL' }) ("want={0} pageOk={1} overRejected={2}" -f $wantPages, $pageOk, $overRejected)

    $exp = Export-WordPdfBounded -DocPath $docPath -PdfPath $pdf -TimeoutSec 60
    if (-not $exp.Ok) {
        $blocked = [bool]$exp.Blocked
        Add-Result ("docx-mp-{0}-pdf" -f $tag) 'FAIL' ("pdf $($exp.Detail)")
        continue
    }
    Add-Result ("docx-mp-{0}-pdf" -f $tag) 'PASS' 'Word PDF'
    & $pngScript -Pdf $pdf -Out $gold -Dpi 96 -Page 1 | Out-Null

    $line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
    $code = $LASTEXITCODE
    $textPx = [string]$line
    $pxOk = ($code -ne 2) -and ($textPx -match 'differ=0\b') -and ($textPx -match 'maxDelta=0\b')
    Add-Result ("docx-mp-{0}-px" -f $tag) $(if ($pxOk) { 'PASS' } else { 'FAIL' }) $textPx

    $libBands = Get-InkBands $lib
    $goldBands = Get-InkBands $gold
    $detail = "lines lib=$($libBands.Count) gold=$($goldBands.Count);"
    $layoutOk = ($libBands.Count -eq $goldBands.Count) -and ($libBands.Count -gt 0)
    $n = [Math]::Max($libBands.Count, $goldBands.Count)
    for ($i = 0; $i -lt $n; $i++) {
        if ($i -ge $libBands.Count -or $i -ge $goldBands.Count) {
            $layoutOk = $false
            $detail += " L$i missing;"
            continue
        }
        $lb = $libBands[$i]
        $gb = $goldBands[$i]
        $same = ([Math]::Abs($lb.x0 - $gb.x0) -le 1) -and ([Math]::Abs($lb.y0 - $gb.y0) -le 1) -and ([Math]::Abs($lb.h - $gb.h) -le 1) -and ([Math]::Abs($lb.w - $gb.w) -le 2)
        if (-not $same) { $layoutOk = $false }
        $detail += " L$i lib=$($lb.x0),$($lb.y0) $($lb.w)x$($lb.h) gold=$($gb.x0),$($gb.y0) $($gb.w)x$($gb.h);"
    }
    Add-Result ("docx-mp-{0}-layout" -f $tag) $(if ($layoutOk) { 'PASS' } else { 'FAIL' }) $detail
    Write-Host ("RESULT {0} pages={1} px={2} layout={3} {4}" -f $tag, $(if ($pageOk -and $overRejected) { 'PASS' } else { 'FAIL' }), $(if ($pxOk) { 'PASS' } else { 'FAIL' }), $(if ($layoutOk) { 'PASS' } else { 'FAIL' }), $detail)
}

# 52 空段：页面数 2（白页，layout 无字带；px 应对齐白底）
Write-Host '===== fifty2 ====='
$emptyParas = @()
1..52 | ForEach-Object { $emptyParas += , @{ text = '' } }
$doc52 = New-MpDoc 'fifty2' $emptyParas @()
$pageOk52 = $true
foreach ($pi in 1, 2) {
    $probe = Get-OutPath ('docx-mp-fifty2-p{0}.png' -f $pi)
    if (Test-Path $probe) { Remove-Item -Force $probe }
    try {
        Invoke-OoxpRequest @{
            schemaVersion = 1
            action        = 'exportPageImage'
            path          = $doc52.Replace('\', '/')
            id            = ('page:{0}' -f $pi)
            output        = $probe.Replace('\', '/')
        } ('docx-mp-fifty2-lib-{0}' -f $pi) | Out-Null
        if (-not (Test-Path $probe)) { $pageOk52 = $false }
    } catch { $pageOk52 = $false }
}
$over52 = Get-OutPath 'docx-mp-fifty2-over.png'
if (Test-Path $over52) { Remove-Item -Force $over52 }
$overRejected52 = $false
try {
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'exportPageImage'
        path          = $doc52.Replace('\', '/')
        id            = 'page:3'
        output        = $over52.Replace('\', '/')
    } 'docx-mp-fifty2-over' | Out-Null
    $overRejected52 = -not (Test-Path $over52)
} catch { $overRejected52 = $true }
Add-Result 'docx-mp-fifty2-pages' $(if ($pageOk52 -and $overRejected52) { 'PASS' } else { 'FAIL' }) ("pageOk={0} overRejected={1}" -f $pageOk52, $overRejected52)

if (-not $blocked) {
    $pdf52 = Get-OutPath 'docx-mp-fifty2.pdf'
    $gold52 = Get-OutPath 'docx-mp-fifty2-gold.png'
    $lib52 = Get-OutPath 'docx-mp-fifty2-p2.png'
    $diff52 = Get-OutPath 'docx-mp-fifty2-diff.png'
    foreach ($p in @($pdf52, $gold52, $diff52)) { if (Test-Path $p) { Remove-Item -Force $p } }
    $exp = Export-WordPdfBounded -DocPath $doc52 -PdfPath $pdf52 -TimeoutSec 90
    if ($exp.Ok) {
        Add-Result 'docx-mp-fifty2-pdf' 'PASS' 'Word PDF'
        & $pngScript -Pdf $pdf52 -Out $gold52 -Dpi 96 -Page 2 | Out-Null
        $line = & $diffScript -Left $gold52 -Right $lib52 -Out $diff52 -MaxDelta 0
        $textPx = [string]$line
        $pxOk = ($LASTEXITCODE -ne 2) -and ($textPx -match 'differ=0\b') -and ($textPx -match 'maxDelta=0\b')
        Add-Result 'docx-mp-fifty2-px' $(if ($pxOk) { 'PASS' } else { 'FAIL' }) $textPx
        Write-Host ("RESULT fifty2 pages={0} px-page2={1} {2}" -f $(if ($pageOk52 -and $overRejected52) { 'PASS' } else { 'FAIL' }), $(if ($pxOk) { 'PASS' } else { 'FAIL' }), $textPx)
    } else {
        Add-Result 'docx-mp-fifty2-pdf' 'FAIL' ("pdf $($exp.Detail)")
    }
}
