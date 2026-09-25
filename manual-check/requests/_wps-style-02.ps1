$ErrorActionPreference = 'Stop'
$root = 'C:\Users\GaoYi-AMD\Desktop\test\manual-check'
$srcPath = Join-Path $root '02-ooxp-source.docx'
$styledPath = Join-Path $root '02-wps-source.docx'
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
    Write-Log "open $srcPath"
    $doc = $word.Documents.Open($srcPath)
    $n = $doc.Paragraphs.Count
    Write-Log "para count=$n"
    $doc.Paragraphs.Item(2).Range.Font.Bold = $true
    if (Test-Path $styledPath) { Remove-Item $styledPath -Force }
    $doc.SaveAs2($styledPath, 12)
    $doc.Close($false)
    Release-Com $doc
    $doc = $null
    Write-Log "saved $styledPath"
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
