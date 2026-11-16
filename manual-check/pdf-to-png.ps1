# 把已有 PDF 的一页栅格化成 PNG。
# 用 Windows 自带的 Windows.Data.Pdf。不调用 Word，也不做分页。
# 这一步只把金样 PDF 变成图。库自己画 PNG，不调用本脚本。
#
#   pwsh -NoProfile -File .\manual-check\pdf-to-png.ps1 -Pdf a.pdf -Out a.png
#   pwsh -NoProfile -File .\manual-check\pdf-to-png.ps1 -Pdf a.pdf -Out a.png -Page 2 -Dpi 96
#
# DestinationWidth/Height 在 WinRT 里是 DIP，不是物理像素。进程若处在
# 非 96 LOGPIXELS（例如 106）下，直接把 Round(Size*Dpi/96) 赋给 Destination*
# 会得到约 ×(LOGPIXELS/96) 的物理 PNG（794→873）。本脚本以目标物理像素为准，
# 必要时二次设置 Destination*（仍走 RenderToStreamAsync，不做 PNG 事后拉伸）。

[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Pdf,
    [Parameter(Mandatory)][string]$Out,
    [int]$Page = 1,
    [int]$Dpi = 96
)

$ErrorActionPreference = 'Stop'

if ($PSVersionTable.PSVersion.Major -ge 6) {
    $hostExe = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
    if (-not (Test-Path -LiteralPath $hostExe)) { throw "需要 Windows PowerShell 5.1 才能调用 Windows.Data.Pdf" }
    $forward = @('-NoProfile', '-File', $PSCommandPath)
    foreach ($name in @('Pdf', 'Out', 'Page', 'Dpi')) {
        if ($PSBoundParameters.ContainsKey($name)) {
            $forward += "-$name"
            $forward += [string]$PSBoundParameters[$name]
        }
    }
    & $hostExe @forward
    exit $LASTEXITCODE
}

if ($Page -lt 1) { throw "-Page 从 1 起" }
if ($Dpi -lt 1 -or $Dpi -gt 600) { throw "-Dpi 要在 1 到 600 之间" }
if (-not (Test-Path -LiteralPath $Pdf)) { throw "找不到 PDF：$Pdf" }
$Pdf = (Resolve-Path -LiteralPath $Pdf).Path
$outDir = Split-Path -Parent $Out
if ($outDir -and -not (Test-Path -LiteralPath $outDir)) {
    New-Item -ItemType Directory -Path $outDir | Out-Null
}

Add-Type -AssemblyName System.Runtime.WindowsRuntime
Add-Type -AssemblyName System.Drawing
$null = [Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime]
$null = [Windows.Data.Pdf.PdfDocument, Windows.Data.Pdf, ContentType = WindowsRuntime]
$null = [Windows.Data.Pdf.PdfPageRenderOptions, Windows.Data.Pdf, ContentType = WindowsRuntime]
$null = [Windows.Storage.Streams.InMemoryRandomAccessStream, Windows.Storage.Streams, ContentType = WindowsRuntime]

$asOp = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
        $_.Name -eq 'AsTask' -and $_.IsGenericMethod -and $_.GetParameters().Count -eq 1 -and
        $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
    })[0]
$asAct = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
        $_.Name -eq 'AsTask' -and -not $_.IsGenericMethod -and $_.GetParameters().Count -eq 1 -and
        $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncAction'
    })[0]

function Await-Op($op, $type) {
    $task = $asOp.MakeGenericMethod($type).Invoke($null, @($op))
    $task.Wait(-1) | Out-Null
    if ($task.IsFaulted) { throw $task.Exception.InnerException }
    $task.Result
}

function Render-PdfPageBytes($pdfPage, [uint32]$destW, [uint32]$destH) {
    $options = New-Object Windows.Data.Pdf.PdfPageRenderOptions
    $options.DestinationWidth = $destW
    $options.DestinationHeight = $destH
    $stream = New-Object Windows.Storage.Streams.InMemoryRandomAccessStream
    $render = $pdfPage.RenderToStreamAsync($stream, $options)
    $task = $asAct.Invoke($null, @($render))
    $task.Wait(-1) | Out-Null
    if ($task.IsFaulted) { throw $task.Exception.InnerException }
    $stream.Seek(0) | Out-Null
    $net = [System.IO.WindowsRuntimeStreamExtensions]::AsStream($stream)
    $memory = New-Object System.IO.MemoryStream
    try {
        $net.CopyTo($memory)
        return $memory.ToArray()
    }
    finally { $memory.Dispose() }
}

function Get-PngPixelSize([byte[]]$bytes) {
    $ms = New-Object System.IO.MemoryStream(,$bytes)
    try {
        $bmp = [System.Drawing.Bitmap]::FromStream($ms)
        try {
            return @{ W = $bmp.Width; H = $bmp.Height }
        }
        finally { $bmp.Dispose() }
    }
    finally { $ms.Dispose() }
}

$file = Await-Op ([Windows.Storage.StorageFile]::GetFileFromPathAsync($Pdf)) ([Windows.Storage.StorageFile])
$doc = Await-Op ([Windows.Data.Pdf.PdfDocument]::LoadFromFileAsync($file)) ([Windows.Data.Pdf.PdfDocument])
if ($Page -gt $doc.PageCount) { throw "只有 $($doc.PageCount) 页，没有第 $Page 页" }
$pdfPage = $doc.GetPage($Page - 1)
try {
    $sizeW = $pdfPage.Size.Width
    $sizeH = $pdfPage.Size.Height
    # 目标物理像素：以 PDF Size（DIP）× Dpi/96 为准，禁止事后拉伸 PNG 凑尺寸。
    $targetW = [int][Math]::Round($sizeW * $Dpi / 96.0)
    $targetH = [int][Math]::Round($sizeH * $Dpi / 96.0)
    if ($targetW -lt 1 -or $targetH -lt 1) { throw "这一页栅格化后宽高小于 1 像素" }

    # 首轮：按文档把目标像素直接写入 Destination*（在 96 DPI 进程上即正确物理尺寸）。
    $destW = [uint32]$targetW
    $destH = [uint32]$targetH
    $bytes = Render-PdfPageBytes $pdfPage $destW $destH
    $got = Get-PngPixelSize $bytes

    # 若 WinRT 把 Destination DIP 再乘了进程 DPI 比例，二次用 Destination 反推目标物理像素。
    if ($got.W -ne $targetW -or $got.H -ne $targetH) {
        if ($got.W -lt 1 -or $got.H -lt 1) { throw "首轮栅格化 PNG 尺寸无效：$($got.W)x$($got.H)" }
        $destW2 = [int][Math]::Round($targetW * [double]$destW / $got.W)
        $destH2 = [int][Math]::Round($targetH * [double]$destH / $got.H)
        if ($destW2 -lt 1) { $destW2 = 1 }
        if ($destH2 -lt 1) { $destH2 = 1 }
        $bytes = Render-PdfPageBytes $pdfPage ([uint32]$destW2) ([uint32]$destH2)
        $got2 = Get-PngPixelSize $bytes
        if ($got2.W -ne $targetW -or $got2.H -ne $targetH) {
            throw ("pdf-to-png 物理像素未达标：目标={0}x{1} 首轮dest={2}x{3}->png={4}x{5} 次轮dest={6}x{7}->png={8}x{9}（未做 PNG 拉伸）" -f `
                $targetW, $targetH, $destW, $destH, $got.W, $got.H, $destW2, $destH2, $got2.W, $got2.H)
        }
        $destW = [uint32]$destW2
        $destH = [uint32]$destH2
        $got = $got2
    }

    [System.IO.File]::WriteAllBytes($Out, $bytes)
    Write-Output ("page={0}/{1} target={2}x{3} dest={4}x{5} png={6}x{7} dpi={8} out={9}" -f `
        $Page, $doc.PageCount, $targetW, $targetH, $destW, $destH, $got.W, $got.H, $Dpi, $Out)
}
finally { $pdfPage.Dispose() }
