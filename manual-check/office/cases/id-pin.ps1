# 同一批 apply 里，段落序号和幻灯片页号按开始时的文件解释。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Read-ZipText([string]$zipPath, [string]$entryName) {
    $zip = [System.IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        $entry = $zip.GetEntry($entryName)
        $stream = $entry.Open()
        try {
            $reader = New-Object System.IO.StreamReader($stream, [System.Text.UTF8Encoding]::new($false))
            try { return $reader.ReadToEnd() } finally { $reader.Dispose() }
        } finally { $stream.Dispose() }
    } finally { $zip.Dispose() }
}

function Write-ZipText([string]$zipPath, [string]$entryName, [string]$text) {
    $zip = [System.IO.Compression.ZipFile]::Open($zipPath, [System.IO.Compression.ZipArchiveMode]::Update)
    try {
        $old = $zip.GetEntry($entryName)
        $old.Delete()
        $created = $zip.CreateEntry($entryName)
        $stream = $created.Open()
        try {
            $writer = New-Object System.IO.StreamWriter($stream, [System.Text.UTF8Encoding]::new($false))
            try { $writer.Write($text) } finally { $writer.Dispose() }
        } finally { $stream.Dispose() }
    } finally { $zip.Dispose() }
}

function Remove-XmlAttr([string]$xml, [string]$name) {
    $key = " ${name}=`""
    $out = ''
    $rest = $xml
    while ($true) {
        $i = $rest.IndexOf($key)
        if ($i -lt 0) { break }
        $out += $rest.Substring(0, $i)
        $after = $rest.Substring($i + $key.Length)
        $end = $after.IndexOf('"')
        $rest = $after.Substring($end + 1)
    }
    return $out + $rest
}

$src = Get-OutPath 'id-pin-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    document = @{ paragraphs = @(
        @{ text = 'P1' }, @{ text = 'P2' }, @{ text = 'P3' }, @{ text = 'P4' }, @{ text = 'P5' }
    ) }
} 'id-pin-create' | Out-Null
$xml = Remove-XmlAttr (Remove-XmlAttr (Read-ZipText $src 'word/document.xml') 'w14:paraId') 'w14:textId'
Write-ZipText $src 'word/document.xml' $xml

$docxOut = Get-OutPath 'id-pin-out.docx'
if (Test-Path $docxOut) { Remove-Item -Force $docxOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $docxOut.Replace('\', '/')
    ops = @(
        @{ op = 'insertPara'; id = 'para:index:1'; text = 'NEW' },
        @{ op = 'setParaText'; id = 'para:index:4'; text = 'EDIT' }
    )
} 'id-pin-apply' | Out-Null

$word = Ensure-MsWord $docxOut
$doc = Open-WordDoc $word $docxOut
$paras = $doc.Paragraphs
$got = @()
try {
    for ($i = 1; $i -le [int]$paras.Count; $i++) {
        $p = $paras.Item($i)
        try { $got += ([string]$p.Range.Text).Trim() } finally { Release-Com $p }
    }
} finally { Release-Com $paras }
$want = @('P1', 'NEW', 'P2', 'P3', 'EDIT', 'P5')
$docxOk = (($got -join '|') -eq ($want -join '|'))
Add-Check 'id-pin-docx' $docxOk ("paras=" + ($got -join '|'))
$doc.Close($false)
Release-Com $doc

$deck = Get-OutPath 'id-pin-src.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $deck.Replace('\', '/')
    presentation = @{ slides = @(
        @{ shapes = @(@{ text = 'S1' }) },
        @{ shapes = @(@{ text = 'S2' }) },
        @{ shapes = @(@{ text = 'S3' }) }
    ) }
} 'id-pin-pptx-create' | Out-Null
$pptxOut = Get-OutPath 'id-pin-out.pptx'
if (Test-Path $pptxOut) { Remove-Item -Force $pptxOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $deck.Replace('\', '/')
    output = $pptxOut.Replace('\', '/')
    ops = @(
        @{ op = 'deleteSlide'; id = 'slide:1' },
        @{ op = 'setShapeText'; id = 'slide:3/shape:2'; text = 'STILL3' }
    )
} 'id-pin-pptx-apply' | Out-Null

$ppt = Ensure-MsPpt $pptxOut
Close-PptPresByPath $ppt $pptxOut
$pres = Open-PptPres $ppt $pptxOut
$pcap = Get-PptCaption $pres
$slide = Get-PptSlide $pres 2
$shape = Get-PptShape $slide 1
$text = ''
try { $text = [string]$shape.TextFrame.TextRange.Text } catch { $text = $_.Exception.Message }
$pptxOk = ($pcap -notmatch 'Repair') -and ($text.Trim() -eq 'STILL3')
Add-Check 'id-pin-pptx' $pptxOk "caption=$pcap text=$text"
Release-Com $shape
Release-Com $slide
Close-PptPres $pres
