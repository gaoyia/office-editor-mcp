# 单节、单段、一行的不透明 Calibri 正文。金样是 Word 16.0 导出 PDF，再按 96 dpi 栅格化。
# 红区回绘制引擎修，不要加大 MaxDelta。字形灰度按 C25 第 0 段暂缓，不把灰阶差算成整页通过，
# 也不借用幻灯片文字的布局近似许可。位置和黑框另记，不替代像素门。
# 2026-11-17 04:32:46–04:40:54 与 04:41:16–04:43:01（Asia/Shanghai）：Word 16.0 对空白 A4 的
# ExportAsFixedFormat(..., 17) 与 SaveAs2(..., 17) 都在写出 PDF 前持续占 CPU、不落盘。
# 本用例仍走该公开金样路径；未改容差、未改金样、未改用私有接口。

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
            $minX = $bmp.Width
            $minY = $bmp.Height
            $maxX = -1
            $maxY = -1
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

function New-CalibriDoc($spec) {
    $src = Get-OutPath ('docx-calibri-{0}-src.docx' -f $spec.tag)
    if (Test-Path $src) { Remove-Item -Force $src }
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'create'
        path          = $src.Replace('\', '/')
        document      = @{ paragraphs = @(, @{ text = [string]$spec.text }) }
    } ('docx-calibri-{0}-create' -f $spec.tag) | Out-Null

    $ops = @()
    if ($spec.sz) {
        $tree = Invoke-OoxpTree @{
            schemaVersion = 1
            action        = 'inspect'
            path          = $src.Replace('\', '/')
            format        = 'docx'
            scope         = @{ from = 1; maxParas = 4 }
        } ('docx-calibri-{0}-inspect' -f $spec.tag)
        $pid = [string]$tree.paragraphs[0].id
        $ops += , @{ op = 'setRunFormat'; id = ($pid + '/run:1'); sz = [int]$spec.sz }
    }
    if ($spec.left) {
        if (-not $pid) {
            $tree = Invoke-OoxpTree @{
                schemaVersion = 1
                action        = 'inspect'
                path          = $src.Replace('\', '/')
                format        = 'docx'
                scope         = @{ from = 1; maxParas = 4 }
            } ('docx-calibri-{0}-inspect' -f $spec.tag)
            $pid = [string]$tree.paragraphs[0].id
        }
        $ops += , @{ op = 'setParaFormat'; id = $pid; left = [int]$spec.left }
    }
    if ($spec.before) {
        if (-not $pid) {
            $tree = Invoke-OoxpTree @{
                schemaVersion = 1
                action        = 'inspect'
                path          = $src.Replace('\', '/')
                format        = 'docx'
                scope         = @{ from = 1; maxParas = 4 }
            } ('docx-calibri-{0}-inspect' -f $spec.tag)
            $pid = [string]$tree.paragraphs[0].id
        }
        $ops += , @{ op = 'setParaFormat'; id = $pid; before = [int]$spec.before }
    }
    if ($spec.marginLeft -or $spec.marginTop) {
        $ops += , @{
            op     = 'setPageMargin'
            id     = 'section:1'
            top    = $(if ($spec.marginTop) { [int]$spec.marginTop } else { 1440 })
            left   = $(if ($spec.marginLeft) { [int]$spec.marginLeft } else { 1440 })
            right  = 1440
            bottom = 1440
        }
    }
    if ($ops.Count -eq 0) { return $src }
    $deck = Get-OutPath ('docx-calibri-{0}.docx' -f $spec.tag)
    if (Test-Path $deck) { Remove-Item -Force $deck }
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'apply'
        path          = $src.Replace('\', '/')
        output        = $deck.Replace('\', '/')
        ops           = $ops
    } ('docx-calibri-{0}-apply' -f $spec.tag) | Out-Null
    return $deck
}

$specs = @(
    @{ tag = 'a11'; title = 'Calibri 11pt A on default A4'; text = 'A' }
    @{ tag = 'hello11'; title = 'Calibri 11pt Hello on default A4'; text = 'Hello' }
    @{ tag = 'abc11'; title = 'Calibri 11pt Abc on default A4'; text = 'Abc' }
    @{ tag = 'sz12'; title = 'Calibri 12pt A on default A4'; text = 'A'; sz = 24 }
    @{ tag = 'sz24'; title = 'Calibri 24pt A on default A4'; text = 'A'; sz = 48 }
    @{ tag = 'indent'; title = 'Calibri 11pt A with 720 twip left indent'; text = 'A'; left = 720 }
    @{ tag = 'before'; title = 'Calibri 11pt A with 240 twip spacing before'; text = 'A'; before = 240 }
    @{ tag = 'margin'; title = 'Calibri 11pt A with 2160 twip top/left margin'; text = 'A'; marginTop = 2160; marginLeft = 2160 }
)

$docs = @()
foreach ($spec in $specs) {
    $docs += , (New-CalibriDoc $spec)
}

$word = Ensure-MsWord $docs[0]
Write-Log ("docx-calibri after Ensure docs=" + $word.Documents.Count + " visible=" + $word.Visible + " cap=" + $word.Caption)
$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$pngScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pdf-to-png.ps1'))

for ($i = 0; $i -lt $specs.Count; $i++) {
    $spec = $specs[$i]
    $docPath = $docs[$i]
    $lib = Get-OutPath ('docx-calibri-{0}-lib.png' -f $spec.tag)
    if (Test-Path $lib) { Remove-Item -Force $lib }
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'exportPageImage'
        path          = $docPath.Replace('\', '/')
        format        = 'docx'
        id            = 'page:1'
        output        = $lib.Replace('\', '/')
    } ('docx-calibri-{0}-export' -f $spec.tag) | Out-Null

    $pdf = Get-OutPath ('docx-calibri-{0}.pdf' -f $spec.tag)
    $gold = Get-OutPath ('docx-calibri-{0}-gold.png' -f $spec.tag)
    $doc = $null
    try {
        Write-Log ('docx-calibri-{0} opening {1} docs={2}' -f $spec.tag, $docPath, $word.Documents.Count)
        $doc = Open-WordDoc $word $docPath
        Write-Log ('docx-calibri-{0} opened name={1}' -f $spec.tag, $doc.Name)
        $cap = ''
        if (Get-Command Get-WordCaption -ErrorAction SilentlyContinue) {
            $cap = Get-WordCaption $word
        }
        if (Test-Path $pdf) { Remove-Item -Force $pdf }
        Write-Log ('docx-calibri-{0} export pdf={1} cap={2}' -f $spec.tag, $pdf, $cap)
        $doc.ExportAsFixedFormat($pdf, 17)
        Write-Log ('docx-calibri-{0} export done exists={1}' -f $spec.tag, (Test-Path $pdf))
        $ok = (Test-Path $pdf) -and ($cap -notmatch 'Repair')
        Add-Check ('docx-calibri-{0}-pdf' -f $spec.tag) $ok ('pdf={0} caption={1} {2}' -f $pdf, $cap, $spec.title)
    }
    finally {
        Close-WordDoc $doc
    }

    if (Test-Path $gold) { Remove-Item -Force $gold }
    & $pngScript -Pdf $pdf -Out $gold -Page 1 -Dpi 96
    if (-not (Test-Path $gold)) { throw "金样 PNG 没有写出来: $($spec.tag)" }

    $diff = Get-OutPath ('docx-calibri-{0}-diff.png' -f $spec.tag)
    $line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
    $code = $LASTEXITCODE
    $text = ($line | Out-String).Trim()
    Write-Log ('docx-calibri-{0}-px exit={1} {2}' -f $spec.tag, $code, $text)
    $pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
    Add-Check ('docx-calibri-{0}-px' -f $spec.tag) $pxOk ('exit={0} {1}' -f $code, $text)

    $libBox = Get-InkBox $lib
    $goldBox = Get-InkBox $gold
    $bboxOk = $false
    $bboxMsg = 'missing ink box'
    if ($libBox -and $goldBox) {
        $bboxOk = ($libBox.x -eq $goldBox.x) -and ($libBox.y -eq $goldBox.y) -and ($libBox.w -eq $goldBox.w) -and ($libBox.h -eq $goldBox.h)
        $bboxMsg = ('lib={0},{1} {2}x{3} gold={4},{5} {6}x{7}' -f $libBox.x, $libBox.y, $libBox.w, $libBox.h, $goldBox.x, $goldBox.y, $goldBox.w, $goldBox.h)
    }
    Write-Log ('docx-calibri-{0}-bbox {1}' -f $spec.tag, $bboxMsg)
    Add-Check ('docx-calibri-{0}-bbox' -f $spec.tag) $bboxOk $bboxMsg
}
