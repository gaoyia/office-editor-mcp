# docx 没有字的页。金样是 Word 16.0 导出 PDF，再按 96 dpi 栅格化。
# 像素差用 pixel-diff 标红。红区要回绘制引擎修，不要加大 MaxDelta。
# 2026-11-17 04:32:46–04:40:54 与 04:41:16–04:43:01（Asia/Shanghai）：Word 16.0 对空白 A4 的
# ExportAsFixedFormat(..., 17) 与 SaveAs2(..., 17) 都在写出 PDF 前持续占 CPU、不落盘。
# 本用例仍走该公开金样路径；未改容差、未改金样、未改用私有接口。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'office-com.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'docx-page-image.docx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    document = @{ paragraphs = @(@{ text = '' }) }
} 'docx-page-image-create' | Out-Null

$lib = Get-OutPath 'docx-page-image-lib.png'
if (Test-Path $lib) { Remove-Item -Force $lib }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportPageImage'
    path = $src.Replace('\', '/'); format = 'docx'
    id = 'page:1'
    output = $lib.Replace('\', '/')
} 'docx-page-image-export' | Out-Null

$word = Ensure-MsWord $src
$doc = $null
$pdf = Get-OutPath 'docx-page-image.pdf'
$gold = Get-OutPath 'docx-page-image-gold.png'
Write-Log ("docx-page-image after Ensure docs=" + $word.Documents.Count + " visible=" + $word.Visible + " cap=" + $word.Caption)
try {
    Write-Log ("docx-page-image opening " + $src)
    $doc = Open-WordDoc $word $src
    Write-Log ("docx-page-image opened name=" + $doc.Name + " full=" + $doc.FullName)
    if (Test-Path $pdf) { Remove-Item -Force $pdf }
    Write-Log ("docx-page-image export pdf=" + $pdf)
    $doc.ExportAsFixedFormat($pdf, 17)
    Write-Log ("docx-page-image export done exists=" + (Test-Path $pdf))
    Add-Check 'docx-page-image-pdf' (Test-Path $pdf) "pdf=$pdf"
}
finally {
    Close-WordDoc $doc
}

$pngScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pdf-to-png.ps1'))
if (Test-Path $gold) { Remove-Item -Force $gold }
& $pngScript -Pdf $pdf -Out $gold -Page 1 -Dpi 96
if (-not (Test-Path $gold)) { throw "金样 PNG 没有写出来" }

$diff = Get-OutPath 'docx-page-image-diff.png'
$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
$code = $LASTEXITCODE
$text = ($line | Out-String).Trim()
Write-Log "docx-page-image-px exit=$code $text"
$pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'docx-page-image-px' $pxOk "exit=$code $text"

$many = Get-OutPath 'docx-page-image-52.docx'
if (Test-Path $many) { Remove-Item -Force $many }
$paras = @()
foreach ($i in 1..52) { $paras += @{ text = '' } }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $many.Replace('\', '/')
    document = @{ paragraphs = $paras }
} 'docx-page-image-52-create' | Out-Null

$lib2 = Get-OutPath 'docx-page-image-52-lib.png'
if (Test-Path $lib2) { Remove-Item -Force $lib2 }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportPageImage'
    path = $many.Replace('\', '/'); format = 'docx'
    id = 'page:2'
    output = $lib2.Replace('\', '/')
} 'docx-page-image-52-export' | Out-Null

if (-not ('OoxpWordOpen' -as [type])) {
    Add-Type -TypeDefinition @"
using System;
using System.Reflection;
public static class OoxpWordOpen {
    public static object Open(object app, string path) {
        object docs = app.GetType().InvokeMember("Documents", BindingFlags.GetProperty, null, app, null);
        return docs.GetType().InvokeMember("Open", BindingFlags.InvokeMethod, null, docs, new object[] { path, false });
    }
}
"@
}
$pdf2 = Get-OutPath 'docx-page-image-52.pdf'
$gold2 = Get-OutPath 'docx-page-image-52-gold.png'
$doc2 = $null
Write-Log ("docx-page-image-52 opening " + $many + " docs=" + $word.Documents.Count)
try {
    $doc2 = [OoxpWordOpen]::Open($word, $many)
    Write-Log ("docx-page-image-52 opened name=" + $doc2.Name)
    if (Test-Path $pdf2) { Remove-Item -Force $pdf2 }
    Write-Log ("docx-page-image-52 export pdf=" + $pdf2)
    $doc2.ExportAsFixedFormat($pdf2, 17)
    Write-Log ("docx-page-image-52 export done exists=" + (Test-Path $pdf2))
    Add-Check 'docx-page-image-52-pdf' (Test-Path $pdf2) "pdf=$pdf2"
}
finally { Close-WordDoc $doc2 }

if (Test-Path $gold2) { Remove-Item -Force $gold2 }
& $pngScript -Pdf $pdf2 -Out $gold2 -Page 2 -Dpi 96
$diff2 = Get-OutPath 'docx-page-image-52-diff.png'
$line2 = & $diffScript -Left $gold2 -Right $lib2 -Out $diff2 -MaxDelta 0
$code2 = $LASTEXITCODE
$text2 = ($line2 | Out-String).Trim()
Write-Log "docx-page-image-52-px exit=$code2 $text2"
$px2 = ($code2 -ne 2) -and ($text2 -match 'differ=0\b') -and ($text2 -match 'maxDelta=0\b')
Add-Check 'docx-page-image-52-px' $px2 "exit=$code2 $text2"
