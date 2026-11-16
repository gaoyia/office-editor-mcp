# 单段拉丁空格换行几何手测。金样 Word 16.0 PDF→96dpi PNG。
# 灰阶 MaxDelta=0 整页仍按 C25 暂缓；本脚本主要记墨迹盒/行高几何原文，不把灰阶差算通过。
# Export-WordPdfBounded 有界等待；超时 FAIL blocked，不杀 WINWORD。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'office-com.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.Drawing

function Get-InkBox([string]$path) {
    $bmp = [System.Drawing.Bitmap]::FromFile($path)
    try {
        $rect = New-Object System.Drawing.Rectangle 0, 0, $bmp.Width, $bmp.Height
        $data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
        try {
            $bytes = New-Object byte[] ($data.Stride * $bmp.Height)
            [Runtime.InteropServices.Marshal]::Copy($data.Scan0, $bytes, 0, $bytes.Length)
            $minX = $bmp.Width; $minY = $bmp.Height; $maxX = -1; $maxY = -1
            for ($y = 0; $y -lt $bmp.Height; $y++) {
                $row = $y * $data.Stride
                for ($x = 0; $x -lt $bmp.Width; $x++) {
                    $o = $row + ($x * 3)
                    if ($bytes[$o] -ne 255 -or $bytes[($o + 1)] -ne 255 -or $bytes[($o + 2)] -ne 255) {
                        if ($x -lt $minX) { $minX = $x }
                        if ($y -lt $minY) { $minY = $y }
                        if ($x -gt $maxX) { $maxX = $x }
                        if ($y -gt $maxY) { $maxY = $y }
                    }
                }
            }
            if ($maxX -lt 0) { return $null }
            return @{ x = $minX; y = $minY; w = ($maxX - $minX + 1); h = ($maxY - $minY + 1) }
        }
        finally { $bmp.UnlockBits($data) }
    }
    finally { $bmp.Dispose() }
}

$wrapText = 'This line is intentionally long enough that it should not fit on one A4 content width at Calibri eleven point without wrapping in Word.'
$tag = 'wrap11'
$src = Get-OutPath ('docx-wrap-{0}-src.docx' -f $tag)
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'create'
    path          = $src.Replace('\', '/')
    document      = @{ paragraphs = @(, @{ text = $wrapText }) }
} ('docx-wrap-{0}-create' -f $tag) | Out-Null

$docPath = $src
$pngScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pdf-to-png.ps1'))
$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$outDir = Split-Path $src -Parent
$pdf = Join-Path $outDir ('docx-wrap-{0}.pdf' -f $tag)
$gold = Join-Path $outDir ('docx-wrap-{0}-gold.png' -f $tag)
$lib = Join-Path $outDir ('docx-wrap-{0}-lib.png' -f $tag)
$diff = Join-Path $outDir ('docx-wrap-{0}-diff.png' -f $tag)
foreach ($p in @($pdf, $gold, $lib, $diff)) { if (Test-Path $p) { Remove-Item -Force $p } }

Write-Host ("[{0}] exportPageImage lib..." -f $tag)
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'exportPageImage'
    path          = $docPath.Replace('\', '/')
    id            = 'page:1'
    output        = $lib.Replace('\', '/')
} ('docx-wrap-{0}-lib' -f $tag) | Out-Null

Write-Host ("[{0}] Word PDF bounded..." -f $tag)
$exp = Export-WordPdfBounded -DocPath $docPath -PdfPath $pdf -TimeoutSec 60
if (-not $exp.Ok) {
    Write-Host ("FAIL blocked wrap Word PDF: {0}" -f $exp.Reason)
    exit 2
}

Write-Host ("[{0}] pdf-to-png..." -f $tag)
& $pngScript -Pdf $pdf -Out $gold -Dpi 96 -Page 1
$line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
$code = $LASTEXITCODE
$textPx = [string]$line
$pxOk = ($code -ne 2) -and ($textPx -match 'differ=0\b') -and ($textPx -match 'maxDelta=0\b')
$libBox = Get-InkBox $lib
$goldBox = Get-InkBox $gold
$bboxOk = $false
if ($libBox -and $goldBox) {
    $bboxOk = ($libBox.x -eq $goldBox.x) -and ($libBox.y -eq $goldBox.y) -and ($libBox.w -eq $goldBox.w) -and ($libBox.h -eq $goldBox.h)
}

Write-Host ("RESULT tag={0} pdf=PASS px={1} bbox={2}" -f $tag, $(if ($pxOk) { 'PASS' } else { 'FAIL' }), $(if ($bboxOk) { 'PASS' } else { 'FAIL' }))
Write-Host ("PX {0}" -f $textPx)
if ($libBox -and $goldBox) {
    Write-Host ("BBOX lib={0},{1} {2}x{3} gold={4},{5} {6}x{7}" -f $libBox.x,$libBox.y,$libBox.w,$libBox.h,$goldBox.x,$goldBox.y,$goldBox.w,$goldBox.h)
} else {
    Write-Host "BBOX missing"
}
# geometry note: wrapped ink should be multi-line tall
if ($libBox) { Write-Host ("LIB_INK_H={0}" -f $libBox.h) }
if ($goldBox) { Write-Host ("GOLD_INK_H={0}" -f $goldBox.h) }