 横向合并纯色单元格：无 tableStyleId + OOXML 六边显式 noFill，分离合并几何与样式描边。
# JSON borders null 按合同是去掉 ln*，不能表达显式 noFill；本用例直接写合法 OOXML。
# 金样是 PowerPoint 16.0 的 Slide.Export。红区回绘制引擎修，不要加大 MaxDelta。
# 纵向合并仍不画。不把文字灰度算进通过。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'office-com.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Set-HMergeNoFillInSlide1([string]$src, [string]$dst) {
    if (Test-Path $dst) { Remove-Item -Force $dst }
    [IO.File]::Copy($src, $dst, $true)
    $zip = [IO.Compression.ZipFile]::Open($dst, [IO.Compression.ZipArchiveMode]::Update)
    try {
        $entry = $zip.GetEntry('ppt/slides/slide1.xml')
        if (-not $entry) { throw 'missing slide1.xml' }
        $sr = New-Object IO.StreamReader($entry.Open())
        $xml = $sr.ReadToEnd()
        $sr.Close()
        $nofill = '<a:lnL><a:noFill/></a:lnL><a:lnR><a:noFill/></a:lnR><a:lnT><a:noFill/></a:lnT><a:lnB><a:noFill/></a:lnB><a:lnTlToBr><a:noFill/></a:lnTlToBr><a:lnBlToTr><a:noFill/></a:lnBlToTr>'
        $from = 0
        for ($n = 0; $n -lt 2; $n++) {
            $rel = $xml.IndexOf('<a:tcPr>', $from)
            if ($rel -lt 0) { throw "tcPr $n missing" }
            $at = $rel + '<a:tcPr>'.Length
            $xml = $xml.Insert($at, $nofill)
            $from = $at + $nofill.Length
        }
        $first = $xml.IndexOf('<a:tc>')
        if ($first -lt 0) { throw 'first tc missing' }
        $xml = $xml.Insert($first + 5, ' gridSpan="2"')
        $secondRel = $xml.Substring($first + 10).IndexOf('<a:tc>')
        if ($secondRel -lt 0) { throw 'second tc missing' }
        $second = $first + 10 + $secondRel
        $xml = $xml.Insert($second + 5, ' hMerge="1"')
        $entry.Delete()
        $newEntry = $zip.CreateEntry('ppt/slides/slide1.xml')
        $sw = New-Object IO.StreamWriter($newEntry.Open())
        $sw.Write($xml)
        $sw.Dispose()
    }
    finally { $zip.Dispose() }
}

$src = Get-OutPath 'pptx-preview-table-merge-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'create'
    path          = $src.Replace('\', '/')
    presentation  = @{
        slides = @(, @{
            tables = @(, @{
                rows = 1; cols = 2
                tableStyleId = $null
                x = 1828800; y = 914400; cx = 1828800; cy = 914400
                cells = @(, @(
                    @{ text = ''; fill = '00AA00' }
                    @{ text = ''; fill = 'EE4455' }
                ))
            })
        })
    }
} 'pptx-preview-table-merge-create' | Out-Null

$mid = Get-OutPath 'pptx-preview-table-merge-mid.pptx'
Set-HMergeNoFillInSlide1 $src $mid

$deck = Get-OutPath 'pptx-preview-table-merge.pptx'
if (Test-Path $deck) { Remove-Item -Force $deck }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'apply'
    path          = $mid.Replace('\', '/')
    output        = $deck.Replace('\', '/')
    ops           = @(, @{ op = 'setPageBackground'; id = 'slide:1'; color = 'AABBCC' })
} 'pptx-preview-table-merge-apply' | Out-Null

$lib = Get-OutPath 'pptx-preview-table-merge-h2-lib.png'
if (Test-Path $lib) { Remove-Item -Force $lib }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'exportSlideImage'
    path          = $deck.Replace('\', '/')
    format        = 'pptx'
    id            = 'slide:1'
    output        = $lib.Replace('\', '/')
} 'pptx-preview-table-merge-export' | Out-Null

$ppt = Ensure-MsPpt $deck
$pres = Open-PptPres $ppt $deck
$gold = Get-OutPath 'pptx-preview-table-merge-h2-gold.png'
try {
    $slide = Get-PptSlide $pres 1
    try {
        if (Test-Path $gold) { Remove-Item -Force $gold }
        $slide.Export($gold, 'PNG')
        $cap = ''
        if (Get-Command Get-PptCaption -ErrorAction SilentlyContinue) { $cap = Get-PptCaption $ppt }
        $ok = (Test-Path $gold) -and ($cap -notmatch 'Repair')
        Add-Check 'pptx-preview-table-merge-h2-open' $ok ("gold=$gold caption=$cap")
    }
    finally { Release-Com $slide }
}
finally { Close-PptPres $pres }

$diff = Get-OutPath 'pptx-preview-table-merge-h2-diff.png'
$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
$code = $LASTEXITCODE
$text = ($line | Out-String).Trim()
Write-Log "pptx-preview-table-merge-h2-px exit=$code $text"
$pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
Add-Check 'pptx-preview-table-merge-h2-px' $pxOk "exit=$code $text"
