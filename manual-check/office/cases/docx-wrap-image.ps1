# 单段拉丁换行断点矩阵：wrap11 + 分离变量小样本。金样 Word 16.0 PDF→96dpi。
# 逐行墨迹盒对照；灰阶 MaxDelta=0 仍按 C25 暂缓。layout=逐行几何；px=整页像素。

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

function New-WrapDoc($tag, $text, $left, $sz) {
    $src = Get-OutPath ('docx-wrap-{0}-src.docx' -f $tag)
    if (Test-Path $src) { Remove-Item -Force $src }
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'create'
        path          = $src.Replace('\', '/')
        document      = @{ paragraphs = @(, @{ text = [string]$text }) }
    } ('docx-wrap-{0}-create' -f $tag) | Out-Null

    $ops = @()
    if ($left -or $sz) {
        $tree = Invoke-OoxpTree @{
            schemaVersion = 1
            action        = 'inspect'
            path          = $src.Replace('\', '/')
            format        = 'docx'
            scope         = @{ from = 1; maxParas = 4 }
        } ('docx-wrap-{0}-inspect' -f $tag)
        $paraId = [string]$tree.paragraphs[0].id
        if ($sz) { $ops += , @{ op = 'setRunFormat'; id = ($paraId + '/run:1'); sz = [int]$sz } }
        if ($left) { $ops += , @{ op = 'setParaFormat'; id = $paraId; left = [int]$left } }
    }
    if ($ops.Count -eq 0) { return $src }
    $deck = Get-OutPath ('docx-wrap-{0}.docx' -f $tag)
    if (Test-Path $deck) { Remove-Item -Force $deck }
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'apply'
        path          = $src.Replace('\', '/')
        output        = $deck.Replace('\', '/')
        ops           = $ops
    } ('docx-wrap-{0}-apply' -f $tag) | Out-Null
    return $deck
}

$pngScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pdf-to-png.ps1'))
$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))

$specs = @(
    @{ tag = 'wrap11'; text = 'This line is intentionally long enough that it should not fit on one A4 content width at Calibri eleven point without wrapping in Word.'; left = $null; sz = $null }
    @{ tag = 'words12'; text = 'alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa quebec romeo'; left = $null; sz = $null }
    @{ tag = 'longword'; text = 'short ExtremelyLongWordWithoutSpacesThatStillFitsAlone next words wrap after it on A4 Calibri eleven'; left = $null; sz = $null }
    @{ tag = 'multispace'; text = 'one  two   three    four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen'; left = $null; sz = $null }
    @{ tag = 'indent720'; text = 'This line is intentionally long enough that it should not fit on one A4 content width at Calibri eleven point without wrapping in Word.'; left = 720; sz = $null }
    @{ tag = 'sz18'; text = 'This line is intentionally long enough that it should not fit on one A4 content width at Calibri eighteen point without wrapping in Word.'; left = $null; sz = 36 }
)

$blocked = $false
foreach ($spec in $specs) {
    $tag = $spec.tag
    Write-Host ("===== {0} =====" -f $tag)
    if ($blocked) {
        Add-Result ("docx-wrap-{0}-pdf" -f $tag) 'FAIL' 'blocked prior Word PDF'
        continue
    }
    $docPath = New-WrapDoc $tag $spec.text $spec.left $spec.sz
    $lib = Get-OutPath ('docx-wrap-{0}-lib.png' -f $tag)
    $pdf = Get-OutPath ('docx-wrap-{0}.pdf' -f $tag)
    $gold = Get-OutPath ('docx-wrap-{0}-gold.png' -f $tag)
    $diff = Get-OutPath ('docx-wrap-{0}-diff.png' -f $tag)
    foreach ($p in @($lib, $pdf, $gold, $diff)) { if (Test-Path $p) { Remove-Item -Force $p } }

    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'exportPageImage'
        path          = $docPath.Replace('\', '/')
        id            = 'page:1'
        output        = $lib.Replace('\', '/')
    } ('docx-wrap-{0}-lib' -f $tag) | Out-Null

    $exp = Export-WordPdfBounded -DocPath $docPath -PdfPath $pdf -TimeoutSec 60
    if (-not $exp.Ok) {
        $blocked = [bool]$exp.Blocked
        Add-Result ("docx-wrap-{0}-pdf" -f $tag) 'FAIL' ("pdf $($exp.Detail)")
        continue
    }
    Add-Result ("docx-wrap-{0}-pdf" -f $tag) 'PASS' 'Word PDF'
    & $pngScript -Pdf $pdf -Out $gold -Dpi 96 -Page 1 | Out-Null

    $line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
    $code = $LASTEXITCODE
    $textPx = [string]$line
    $pxOk = ($code -ne 2) -and ($textPx -match 'differ=0\b') -and ($textPx -match 'maxDelta=0\b')
    Add-Result ("docx-wrap-{0}-px" -f $tag) $(if ($pxOk) { 'PASS' } else { 'FAIL' }) $textPx

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
    Add-Result ("docx-wrap-{0}-layout" -f $tag) $(if ($layoutOk) { 'PASS' } else { 'FAIL' }) $detail
    Write-Host ("RESULT {0} px={1} layout={2} {3}" -f $tag, $(if ($pxOk) { 'PASS' } else { 'FAIL' }), $(if ($layoutOk) { 'PASS' } else { 'FAIL' }), $detail)
}