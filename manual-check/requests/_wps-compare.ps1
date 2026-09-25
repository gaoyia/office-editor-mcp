$ErrorActionPreference = 'Stop'
$root = 'C:\Users\GaoYi-AMD\Desktop\test\manual-check'
$pathA = Join-Path $root '03-applied.docx'
$pathB = Join-Path $root '04-wps-same-edits.docx'
$outCsv = Join-Path $root 'compare-03-vs-04.csv'
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

function Snapshot([string]$path) {
    $word = $null
    $doc = $null
    try {
        $word = New-Object -ComObject Word.Application
        $word.Visible = $false
        $word.DisplayAlerts = 0
        $doc = $word.Documents.Open($path)
        $map = @{}
        $n = [int]$doc.Paragraphs.Count
        $map['count'] = "$n"
        for ($i = 1; $i -le $n; $i++) {
            $p = $doc.Paragraphs.Item($i)
            $text = $p.Range.Text.TrimEnd([char]13)
            $style = [string]$p.Style.NameLocal
            $bold = [string][int]$p.Range.Font.Bold
            $map["p$i.text"] = $text
            $map["p$i.style"] = $style
            $map["p$i.bold"] = $bold
        }
        return $map
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

Write-Log 'snapshot 03'
$a = Snapshot $pathA
Write-Log 'snapshot 04'
$b = Snapshot $pathB

$keys = @($a.Keys + $b.Keys) | Sort-Object -Unique
$lines = New-Object System.Collections.Generic.List[string]
[void]$lines.Add('field,ooxp,wps,same')
$diff = 0
foreach ($k in $keys) {
    $va = [string]$a[$k]
    $vb = [string]$b[$k]
    $same = ($va -eq $vb)
    if (-not $same) { $diff++ }
    [void]$lines.Add("$k,$va,$vb,$same")
}
$utf8 = New-Object System.Text.UTF8Encoding $false
[System.IO.File]::WriteAllLines($outCsv, $lines, $utf8)
Write-Log "diff count=$diff"
$lines | Where-Object { $_ -match ',False$' }
