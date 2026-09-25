$ErrorActionPreference = 'Stop'
$root = 'C:\Users\GaoYi-AMD\Desktop\test\manual-check'
$src = Join-Path $root '02-wps-source.docx'
$dst = Join-Path $root '04-wps-same-edits.docx'
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

function U([int[]]$codes) {
    -join ($codes | ForEach-Object { [char]$_ })
}

$newText = U @(0x6539, 0x8FC7, 0x7684, 0x6B63, 0x6587)

$word = $null
$doc = $null
try {
    $word = New-Object -ComObject Word.Application
    $word.Visible = $false
    $word.DisplayAlerts = 0
    Write-Log "open $src"
    $doc = $word.Documents.Open($src)
    $rng = $doc.Paragraphs.Item(2).Range
    $wdCharacter = 1
    [void]$rng.MoveEnd($wdCharacter, -1)
    $rng.Text = $newText
    $doc.Paragraphs.Item(2).Range.Font.Bold = $true
    if (Test-Path $dst) { Remove-Item $dst -Force }
    $doc.SaveAs2($dst, 12)
    $doc.Close($false)
    Release-Com $rng
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
