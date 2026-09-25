$ErrorActionPreference = 'Stop'
$root = 'C:\Users\GaoYi-AMD\Desktop\test\manual-check'
$logPath = Join-Path $root 'wps-com-log.txt'
$csv = Join-Path $root 'compare-07-vs-08.csv'

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

function Dump-Doc([string]$path) {
    $word = $null
    $doc = $null
    $rows = @()
    try {
        $word = New-Object -ComObject Word.Application
        $word.Visible = $false
        $word.DisplayAlerts = 0
        $doc = $word.Documents.Open($path)
        $n = $doc.Paragraphs.Count
        $inline = $doc.InlineShapes.Count
        $shapes = $doc.Shapes.Count
        for ($i = 1; $i -le $n; $i++) {
            $p = $doc.Paragraphs.Item($i)
            $r = $p.Range
            $r.MoveEnd(1, -1) | Out-Null
            $list = ''
            try { $list = [string]$r.ListFormat.ListType } catch {}
            $rows += [pscustomobject]@{
                i = $i
                text = $r.Text
                style = [string]$p.Style.NameLocal
                align = [string]$p.Alignment
                indent = [string]$p.LeftIndent
                bold = [string]$r.Font.Bold
                italic = [string]$r.Font.Italic
                underline = [string]$r.Font.Underline
                strike = [string]$r.Font.StrikeThrough
                color = [string]$r.Font.Color
                size = [string]$r.Font.Size
                shade = [string]$p.Shading.BackgroundPatternColor
                list = $list
            }
            Release-Com $r
            Release-Com $p
        }
        $doc.Close($false)
        Release-Com $doc
        $doc = $null
        return @{ rows = $rows; inline = $inline; shapes = $shapes }
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
}

$exe = "c:\Users\GaoYi-AMD\Desktop\test\target\debug\ooxp.exe"
& $exe --input (Join-Path $root 'requests\08-apply-media.json') | Write-Output

$req = '{"schemaVersion":1,"action":"inspect","path":"C:/Users/GaoYi-AMD/Desktop/test/manual-check/08-media-applied.docx","scope":{"from":1,"maxParas":200}}'
Set-Content -Path (Join-Path $root 'requests\08-inspect.json') -Value $req -Encoding UTF8
cmd /c "`"$exe`" --input `"$root\requests\08-inspect.json`" --pretty > `"$root\inspect-08.json`""

$before = Dump-Doc (Join-Path $root '07-wps-media.docx')
$after = Dump-Doc (Join-Path $root '08-media-applied.docx')
Write-Log ("before inline=" + $before.inline + " shapes=" + $before.shapes)
Write-Log ("after  inline=" + $after.inline + " shapes=" + $after.shapes)

$out = @()
$max = [Math]::Max($before.rows.Count, $after.rows.Count)
for ($i = 0; $i -lt $max; $i++) {
    $b = $before.rows[$i]
    $a = $after.rows[$i]
    $out += [pscustomobject]@{
        i = $i + 1
        beforeText = $b.text
        afterText = $a.text
        styleSame = ($b.style -eq $a.style)
        beforeStyle = $b.style
        afterStyle = $a.style
        alignSame = ($b.align -eq $a.align)
        indentSame = ($b.indent -eq $a.indent)
        italicSame = ($b.italic -eq $a.italic)
        underlineSame = ($b.underline -eq $a.underline)
        strikeSame = ($b.strike -eq $a.strike)
        colorSame = ($b.color -eq $a.color)
        shadeSame = ($b.shade -eq $a.shade)
        listSame = ($b.list -eq $a.list)
        beforeAlign = $b.align
        afterAlign = $a.align
        beforeList = $b.list
        afterList = $a.list
        beforeColor = $b.color
        afterColor = $a.color
    }
}
$out | Export-Csv -Path $csv -NoTypeInformation -Encoding UTF8
Write-Log "wrote $csv"
Write-Log ("shape count same=" + (($before.inline -eq $after.inline) -and ($before.shapes -eq $after.shapes)))
