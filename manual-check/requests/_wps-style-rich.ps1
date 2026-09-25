$ErrorActionPreference = 'Stop'
$root = 'C:\Users\GaoYi-AMD\Desktop\test\manual-check'
$src = Join-Path $root '05-ooxp-rich.docx'
$dst = Join-Path $root '05-wps-rich.docx'
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

$word = $null
$doc = $null
try {
    $word = New-Object -ComObject Word.Application
    $word.Visible = $false
    $word.DisplayAlerts = 0
    Write-Log "open $src"
    $doc = $word.Documents.Open($src)
    Write-Log ("para count=" + $doc.Paragraphs.Count)
    $doc.Paragraphs.Item(4).Range.Font.Bold = $true
    $doc.Paragraphs.Item(4).Range.Font.Color = 255
    $doc.Paragraphs.Item(5).Range.Font.Italic = $true
    $doc.Paragraphs.Item(5).Range.Font.Size = 14
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
