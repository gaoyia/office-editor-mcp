$ErrorActionPreference = 'Stop'
$root = 'C:\Users\GaoYi-AMD\Desktop\test\manual-check'
$logPath = Join-Path $root 'wps-com-log.txt'
$csv = Join-Path $root 'compare-05-vs-06.csv'

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
        for ($i = 1; $i -le $n; $i++) {
            $p = $doc.Paragraphs.Item($i)
            $r = $p.Range
            $r.MoveEnd(1, -1) | Out-Null
            $rows += [pscustomobject]@{
                i = $i
                text = $r.Text
                style = [string]$p.Style.NameLocal
                bold = [string]$r.Font.Bold
                italic = [string]$r.Font.Italic
                color = [string]$r.Font.Color
                size = [string]$r.Font.Size
            }
            Release-Com $r
            Release-Com $p
        }
        $doc.Close($false)
        Release-Com $doc
        $doc = $null
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
    return $rows
}

$before = Dump-Doc (Join-Path $root '05-wps-rich.docx')
$after = Dump-Doc (Join-Path $root '06-rich-applied.docx')
$out = @()
$max = [Math]::Max($before.Count, $after.Count)
for ($i = 0; $i -lt $max; $i++) {
    $b = $before[$i]
    $a = $after[$i]
    $out += [pscustomobject]@{
        i = $i + 1
        beforeText = $b.text
        afterText = $a.text
        beforeStyle = $b.style
        afterStyle = $a.style
        styleSame = ($b.style -eq $a.style)
        beforeBold = $b.bold
        afterBold = $a.bold
        beforeItalic = $b.italic
        afterItalic = $a.italic
        beforeColor = $b.color
        afterColor = $a.color
        beforeSize = $b.size
        afterSize = $a.size
    }
}
$out | Export-Csv -Path $csv -NoTypeInformation -Encoding UTF8
Write-Log "wrote $csv"
$out | Format-Table -AutoSize | Out-String | Write-Output
