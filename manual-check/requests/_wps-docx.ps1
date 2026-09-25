# WPS Writer COM (Word.Application). ASCII only.
$ErrorActionPreference = 'Stop'
$root = 'C:\Users\GaoYi-AMD\Desktop\test\manual-check'
$createPath = Join-Path $root '01-create.docx'
$resavePath = Join-Path $root '01-create-wps-resaved.docx'
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

if (Test-Path $logPath) { Remove-Item $logPath -Force }

$word = $null
$doc = $null
try {
    $word = New-Object -ComObject Word.Application
    $word.Visible = $false
    $word.DisplayAlerts = 0

    Write-Log "open $createPath"
    $doc = $word.Documents.Open($createPath)
    $p1 = $doc.Paragraphs.Item(1)
    $p2 = $doc.Paragraphs.Item(2)
    $s1 = [string]$p1.Style.NameLocal
    $t1 = $p1.Range.Text.TrimEnd([char]13)
    $t2 = $p2.Range.Text.TrimEnd([char]13)
    Write-Log "create p1.style=$s1 p1.text=$t1 p2.text=$t2"
    if (Test-Path $resavePath) { Remove-Item $resavePath -Force }
    $doc.SaveAs2($resavePath, 12)
    $doc.Close($false)
    Release-Com $p1
    Release-Com $p2
    Release-Com $doc
    $doc = $null
    Write-Log "saved $resavePath"

    Write-Log "open $srcPath"
    $doc = $word.Documents.Open($srcPath)
    $doc.Paragraphs.Item(2).Range.Font.Bold = $true
    if (Test-Path $styledPath) { Remove-Item $styledPath -Force }
    $doc.SaveAs2($styledPath, 12)
    $doc.Close($false)
    Release-Com $doc
    $doc = $null
    Write-Log "saved $styledPath bold para2"
    Write-Log 'COM style pass done'
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
