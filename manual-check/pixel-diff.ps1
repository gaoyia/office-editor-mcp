# 两张 PNG 的像素对照。
# 底图是左图的灰度，通道差超过 -MaxDelta 的像素涂成纯红。
# 宽高不同时不写图，退出码 2。有差异也退出码 0，差异记在标准输出。
# 本脚本只标红区，不改两张原图。红区要回到绘制引擎修，不要靠加大 -MaxDelta 算成通过。
# 现有手测不出成对 PNG，不要把本脚本套进那些用例。
#
#   pwsh -NoProfile -File .\manual-check\pixel-diff.ps1 -Left a.png -Right b.png
#   pwsh -NoProfile -File .\manual-check\pixel-diff.ps1 -Left a.png -Right b.png -Out diff.png -MaxDelta 0

[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Left,
    [Parameter(Mandatory)][string]$Right,
    [string]$Out,
    [int]$MaxDelta = 0
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

if (-not (Test-Path -LiteralPath $Left)) { throw "找不到左图：$Left" }
if (-not (Test-Path -LiteralPath $Right)) { throw "找不到右图：$Right" }
$Left = (Resolve-Path -LiteralPath $Left).Path
$Right = (Resolve-Path -LiteralPath $Right).Path
if (-not $Out) {
    $dir = Split-Path -Parent $Left
    $stem = [IO.Path]::GetFileNameWithoutExtension($Left)
    $Out = Join-Path $dir ($stem + '-diff.png')
}

function New-Bgra32([System.Drawing.Image]$src) {
    $bmp = New-Object System.Drawing.Bitmap $src.Width, $src.Height, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    try {
        $g.DrawImage($src, 0, 0, $src.Width, $src.Height)
    }
    finally { $g.Dispose() }
    return $bmp
}

function Read-Bgra([System.Drawing.Bitmap]$bmp) {
    $rect = New-Object System.Drawing.Rectangle 0, 0, $bmp.Width, $bmp.Height
    $data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, $bmp.PixelFormat)
    try {
        $n = $data.Stride * $bmp.Height
        $buf = New-Object byte[] $n
        [Runtime.InteropServices.Marshal]::Copy($data.Scan0, $buf, 0, $n)
        return @{ Buf = $buf; Stride = $data.Stride }
    }
    finally { $bmp.UnlockBits($data) }
}

$srcL = [System.Drawing.Bitmap]::FromFile($Left)
$srcR = [System.Drawing.Bitmap]::FromFile($Right)
$leftBmp = $null
$rightBmp = $null
$diffBmp = $null
try {
    if ($srcL.Width -ne $srcR.Width -or $srcL.Height -ne $srcR.Height) {
        Write-Output ("size-differ left=$($srcL.Width)x$($srcL.Height) right=$($srcR.Width)x$($srcR.Height)")
        exit 2
    }
    $w = $srcL.Width
    $h = $srcL.Height
    $leftBmp = New-Bgra32 $srcL
    $rightBmp = New-Bgra32 $srcR
    $la = Read-Bgra $leftBmp
    $rb = Read-Bgra $rightBmp
    $stride = $la.Stride
    $outBuf = New-Object byte[] ($stride * $h)
    $differ = 0
    $max = 0
    for ($y = 0; $y -lt $h; $y++) {
        $row = $y * $stride
        for ($x = 0; $x -lt $w; $x++) {
            $i = $row + ($x * 4)
            $db = [int]$la.Buf[$i] - [int]$rb.Buf[$i]
            $dg = [int]$la.Buf[$i + 1] - [int]$rb.Buf[$i + 1]
            $dr = [int]$la.Buf[$i + 2] - [int]$rb.Buf[$i + 2]
            if ($db -lt 0) { $db = -$db }
            if ($dg -lt 0) { $dg = -$dg }
            if ($dr -lt 0) { $dr = -$dr }
            $d = $db
            if ($dg -gt $d) { $d = $dg }
            if ($dr -gt $d) { $d = $dr }
            if ($d -gt $max) { $max = $d }
            if ($d -gt $MaxDelta) {
                $differ++
                $outBuf[$i] = 0
                $outBuf[$i + 1] = 0
                $outBuf[$i + 2] = 255
                $outBuf[$i + 3] = 255
            }
            else {
                $gray = [byte]((0.114 * $la.Buf[$i]) + (0.587 * $la.Buf[$i + 1]) + (0.299 * $la.Buf[$i + 2]))
                $outBuf[$i] = $gray
                $outBuf[$i + 1] = $gray
                $outBuf[$i + 2] = $gray
                $outBuf[$i + 3] = 255
            }
        }
    }
    $diffBmp = New-Object System.Drawing.Bitmap $w, $h, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $rect = New-Object System.Drawing.Rectangle 0, 0, $w, $h
    $locked = $diffBmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::WriteOnly, $diffBmp.PixelFormat)
    try {
        [Runtime.InteropServices.Marshal]::Copy($outBuf, 0, $locked.Scan0, $outBuf.Length)
    }
    finally { $diffBmp.UnlockBits($locked) }
    $diffBmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
    Write-Output ("w=${w}x${h} differ=$differ maxDelta=$max maxKept=$MaxDelta pixels=$($w * $h) out=$Out")
}
finally {
    if ($diffBmp) { $diffBmp.Dispose() }
    if ($leftBmp) { $leftBmp.Dispose() }
    if ($rightBmp) { $rightBmp.Dispose() }
    $srcL.Dispose()
    $srcR.Dispose()
}
