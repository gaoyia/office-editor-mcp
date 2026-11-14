# 把已有 PDF 的一页栅格化成 PNG。
# 用 Windows 自带的 Windows.Data.Pdf。不调用 Word，也不做分页。
# 这一步只把金样 PDF 变成图。库自己画 PNG，不调用本脚本。
#
#   pwsh -NoProfile -File .\manual-check\pdf-to-png.ps1 -Pdf a.pdf -Out a.png
#   pwsh -NoProfile -File .\manual-check\pdf-to-png.ps1 -Pdf a.pdf -Out a.png -Page 2 -Dpi 96

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

$file = Await-Op ([Windows.Storage.StorageFile]::GetFileFromPathAsync($Pdf)) ([Windows.Storage.StorageFile])
$doc = Await-Op ([Windows.Data.Pdf.PdfDocument]::LoadFromFileAsync($file)) ([Windows.Data.Pdf.PdfDocument])
if ($Page -gt $doc.PageCount) { throw "只有 $($doc.PageCount) 页，没有第 $Page 页" }
$pdfPage = $doc.GetPage($Page - 1)
try {
    $sizeW = $pdfPage.Size.Width
    $sizeH = $pdfPage.Size.Height
    $width = [int][Math]::Round($sizeW * $Dpi / 96.0)
    $height = [int][Math]::Round($sizeH * $Dpi / 96.0)
    if ($width -lt 1 -or $height -lt 1) { throw "这一页栅格化后宽高小于 1 像素" }
    $options = New-Object Windows.Data.Pdf.PdfPageRenderOptions
    $options.DestinationWidth = [uint32]$width
    $options.DestinationHeight = [uint32]$height
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
        [System.IO.File]::WriteAllBytes($Out, $memory.ToArray())
    }
    finally { $memory.Dispose() }
    Write-Output "page=$Page/$($doc.PageCount) w=${width}x${height} dpi=$Dpi out=$Out"
}
finally { $pdfPage.Dispose() }
