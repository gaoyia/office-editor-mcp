$ErrorActionPreference = 'Stop'
$root = 'C:\Users\GaoYi-AMD\Desktop\test\manual-check'
$src = Join-Path $root '07-ooxp-media.docx'
$dst = Join-Path $root '07-wps-media.docx'
$picWide = Join-Path $root 'ooxp-pic-wide.png'
$picSq = Join-Path $root 'ooxp-pic-square.png'
$logPath = Join-Path $root 'wps-com-log.txt'

function Write-Log([string]$msg) {
    $line = "$(Get-Date -Format 'HH:mm:ss') $msg"
    Add-Content -Path $logPath -Value $line -Encoding UTF8
    Write-Output $line
}

function Release-Com($obj) {
    if ($null -ne $obj) {
        [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($obj)
    }
}

function Try-SetStyle($para, [string]$name) {
    try {
        $para.Style = $name
        return $true
    } catch {
        return $false
    }
}

# 素材图不入库。缺图就现画一张，免得 AddPicture 抛错把整轮验收卡死。
function Ensure-Pic([string]$path, [int]$w, [int]$h, [string]$label) {
    if (Test-Path $path) { return }
    Add-Type -AssemblyName System.Drawing
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $rect = New-Object System.Drawing.Rectangle 0, 0, $w, $h
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        $rect,
        [System.Drawing.Color]::FromArgb(90, 130, 200),
        [System.Drawing.Color]::FromArgb(200, 220, 240), 45.0)
    $g.FillRectangle($brush, $rect)
    $pen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(40, 60, 100)), 6
    $g.DrawRectangle($pen, 3, 3, ($w - 7), ($h - 7))
    $font = New-Object System.Drawing.Font 'Arial', ([float]($h / 6)), ([System.Drawing.FontStyle]::Bold)
    $g.DrawString($label, $font, [System.Drawing.Brushes]::White, 20, ($h / 2 - $h / 8))
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose(); $brush.Dispose(); $pen.Dispose(); $font.Dispose()
    Write-Log "generated $path ($w x $h)"
}

Ensure-Pic $picWide 1200 300 'ooxp wide'
Ensure-Pic $picSq 600 600 'ooxp square'

$word = $null
$doc = $null
try {
    $word = New-Object -ComObject Word.Application
    $word.Visible = $false
    $word.DisplayAlerts = 0
    Write-Log "open $src"
    $doc = $word.Documents.Open($src)
    Write-Log ("para count before=" + $doc.Paragraphs.Count)

    $p2 = $doc.Paragraphs.Item(2)
    $p2.Alignment = 1
    $p2.Shading.BackgroundPatternColor = 16247773
    $p2.SpaceBefore = 6
    $p2.SpaceAfter = 12
    Release-Com $p2

    $p4 = $doc.Paragraphs.Item(4)
    $quoted = (Try-SetStyle $p4 'Quote') -or (Try-SetStyle $p4 ([string][char]0x5F15 + [char]0x7528))
    Write-Log ("quote style applied=" + $quoted)
    $p4.LeftIndent = 28
    $p4.RightIndent = 14
    $p4.SpaceBefore = 8
    $p4.SpaceAfter = 8
    $p4.Range.Font.Italic = $true
    try {
        $left = $p4.Borders.Item(-2)
        $left.LineStyle = 1
        $left.LineWidth = 12
        $left.Color = 12632256
        Release-Com $left
    } catch {
        Write-Log "left border skipped: $($_.Exception.Message)"
    }
    Release-Com $p4

    $doc.Paragraphs.Item(6).Range.ListFormat.ApplyBulletDefault()
    $doc.Paragraphs.Item(7).Range.ListFormat.ApplyBulletDefault()

    $p9 = $doc.Paragraphs.Item(9)
    try { $p9.Range.Font.NameFarEast = [string][char]0x6977 + [char]0x4F53 } catch {}
    try { $p9.Range.Font.Name = [string][char]0x6977 + [char]0x4F53 } catch {}
    $p9.Range.Font.Underline = 1
    $p9.Range.Font.Color = 16711680
    $p9.Range.Font.Size = 14
    Release-Com $p9

    $p10 = $doc.Paragraphs.Item(10)
    $p10.Range.Font.StrikeThrough = $true
    try { $p10.Range.Font.HighlightColorIndex = 7 } catch {}
    try { $p10.Range.Shading.BackgroundPatternColor = 65535 } catch {}
    Release-Com $p10

    $r12 = $doc.Paragraphs.Item(12).Range
    $r12.Collapse(1) | Out-Null
    $pic1 = $doc.InlineShapes.AddPicture($picWide, $false, $true, $r12)
    $pic1.LockAspectRatio = $true
    $pic1.Width = 380
    Write-Log ("pic1 width=" + $pic1.Width + " height=" + $pic1.Height)
    Release-Com $pic1
    Release-Com $r12

    $r13 = $doc.Paragraphs.Item(13).Range
    $r13.Collapse(1) | Out-Null
    $pic2 = $doc.InlineShapes.AddPicture($picSq, $false, $true, $r13)
    $pic2.LockAspectRatio = $true
    $pic2.Width = 170
    try {
        $pic2.Line.Visible = $true
        $pic2.Line.ForeColor.RGB = 255
        $pic2.Line.Weight = 2.25
    } catch {
        Write-Log "pic2 border skipped: $($_.Exception.Message)"
    }
    $wrapped = $false
    try {
        $sh = $pic2.ConvertToShape()
        $sh.WrapFormat.Type = 0
        $sh.Left = 20
        $wrapped = $true
        Release-Com $sh
    } catch {
        Write-Log "pic2 wrap skipped: $($_.Exception.Message)"
    }
    Write-Log ("pic2 wrap=" + $wrapped)
    Release-Com $pic2
    Release-Com $r13

    Write-Log ("para count after=" + $doc.Paragraphs.Count)
    Write-Log ("inline shapes=" + $doc.InlineShapes.Count)
    Write-Log ("shapes=" + $doc.Shapes.Count)

    if (Test-Path $dst) { Remove-Item $dst -Force }
    $doc.SaveAs2($dst, 12)
    $doc.Close($false)
    Release-Com $doc
    $doc = $null
    Write-Log "saved $dst"
}
catch {
    Write-Log "COM failed: $($_.Exception.Message)"
    throw
}
finally {
    if ($null -ne $doc) {
        try { $doc.Close($false) } catch {}
        Release-Com $doc
    }
    if ($null -ne $word) {
        try { $word.Quit() } catch {}
        Release-Com $word
    }
    [GC]::Collect()
    [GC]::WaitForPendingFinalizers()
}
