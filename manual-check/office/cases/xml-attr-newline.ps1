# 脏部件写回时，属性里的换行仍是字符引用。
# Excel 数据验证提示是两行；PowerPoint 图片替代文字是两行。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Read-ZipText([string]$zipPath, [string]$entryName) {
    $zip = [System.IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        $entry = $zip.GetEntry($entryName)
        if (-not $entry) { throw "找不到 $entryName" }
        $stream = $entry.Open()
        try {
            $reader = New-Object System.IO.StreamReader($stream, [System.Text.UTF8Encoding]::new($false))
            try { return $reader.ReadToEnd() } finally { $reader.Dispose() }
        }
        finally { $stream.Dispose() }
    }
    finally { $zip.Dispose() }
}

function Write-ZipText([string]$zipPath, [string]$entryName, [string]$text) {
    $zip = [System.IO.Compression.ZipFile]::Open(
        $zipPath,
        [System.IO.Compression.ZipArchiveMode]::Update
    )
    try {
        $entry = $zip.GetEntry($entryName)
        if (-not $entry) { throw "找不到 $entryName" }
        $entry.Delete()
        $created = $zip.CreateEntry($entryName)
        $stream = $created.Open()
        try {
            $writer = New-Object System.IO.StreamWriter($stream, [System.Text.UTF8Encoding]::new($false))
            try { $writer.Write($text) } finally { $writer.Dispose() }
        }
        finally { $stream.Dispose() }
    }
    finally { $zip.Dispose() }
}

$hostPath = Get-OutPath 'xml-nl-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'xml-nl-xlsx-create' | Out-Null

$withDv = Get-OutPath 'xml-nl-dv.xlsx'
if (Test-Path $withDv) { Remove-Item -Force $withDv }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $withDv.Replace('\', '/')
    ops = @(@{
        op = 'insertDataValidation'
        id = 'sheet:Sheet1'
        sqref = 'A1'
        type = 'list'
        formula1 = '1,2'
        showInputMessage = $true
        prompt = 'KEEPME'
    })
} 'xml-nl-xlsx-dv' | Out-Null

$sheet = 'xl/worksheets/sheet1.xml'
$xml = (Read-ZipText $withDv $sheet).Replace('prompt="KEEPME"', 'prompt="line1&#10;line2&#9;x"')
if ($xml -notmatch '&#10;') { throw "数据验证 XML 没有写入 &#10;" }
Write-ZipText $withDv $sheet $xml

$xlsxOut = Get-OutPath 'xml-nl-cell.xlsx'
if (Test-Path $xlsxOut) { Remove-Item -Force $xlsxOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $withDv.Replace('\', '/')
    output = $xlsxOut.Replace('\', '/')
    ops = @(@{ op = 'setCell'; id = 'cell:Sheet1!B1'; value = 2 })
} 'xml-nl-xlsx-set' | Out-Null

$excel = Ensure-MsExcel $xlsxOut
Close-AllWorkbooks $excel
$wb = Open-ExcelBook $excel $xlsxOut
$cap = [string]$wb.Windows.Item(1).Caption
$s1 = $wb.Worksheets.Item('Sheet1')
$a1 = $s1.Range('A1')
$msg = ''
try { $msg = [string]$a1.Validation.InputMessage } catch { Write-Log "InputMessage 读失败 $($_.Exception.Message)" }
$twoLines = ($msg -eq "line1`nline2`tx") -or ($msg -eq "line1`r`nline2`tx")
$xlsxOk = ($cap -notmatch 'Repair') -and $twoLines
Add-Check 'xml-nl-xlsx' $xlsxOk "caption=$cap msg=$($msg.Replace("`r", '\r').Replace("`n", '\n').Replace("`t", '\t'))"
Release-Com $a1
Release-Com $s1
Close-AllWorkbooks $excel

$png = Ensure-OoxpPngB
$deck = Get-OutPath 'xml-nl-host.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $deck.Replace('\', '/'); format = 'pptx'
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'xml-nl-pptx-create' | Out-Null

$withPic = Get-OutPath 'xml-nl-pic.pptx'
if (Test-Path $withPic) { Remove-Item -Force $withPic }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $deck.Replace('\', '/')
    output = $withPic.Replace('\', '/')
    ops = @(@{
        op = 'insertPicture'
        id = 'slide:1'
        file = $png.Replace('\', '/')
        x = 914400; y = 685800; cx = 1828800; cy = 914400
    })
} 'xml-nl-pptx-pic' | Out-Null

$slideName = 'ppt/slides/slide1.xml'
$slideXml = Read-ZipText $withPic $slideName
$at = $slideXml.IndexOf('name="图片')
if ($at -lt 0) { throw "幻灯片里没有图片名" }
$end = $slideXml.IndexOf('/>', $at)
if ($end -lt 0) { throw "图片标签没有自闭合" }
$patched = $slideXml.Insert($end, ' descr="line1&#10;line2"')
Write-ZipText $withPic $slideName $patched

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $withPic.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'xml-nl-pptx-inspect'
$shapeId = [string]$tree.slides[0].shapes[0].id

$pptxOut = Get-OutPath 'xml-nl-text.pptx'
if (Test-Path $pptxOut) { Remove-Item -Force $pptxOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $withPic.Replace('\', '/')
    output = $pptxOut.Replace('\', '/')
    ops = @(@{ op = 'setShapeText'; id = $shapeId; text = '改过' })
} 'xml-nl-pptx-text' | Out-Null

$ppt = Ensure-MsPpt $pptxOut
Close-PptPresByPath $ppt $pptxOut
$pres = Open-PptPres $ppt $pptxOut
$pcap = Get-PptCaption $pres
$slide = Get-PptSlide $pres 1
$alts = @()
$shapes = $slide.Shapes
try {
    foreach ($shape in @($shapes)) {
        $alt = ''
        try { $alt = [string]$shape.AlternativeText } catch {}
        if ($alt) { $alts += $alt }
        Release-Com $shape
    }
}
finally { Release-Com $shapes }
$joined = $alts -join ' | '
$pptTwo = @($alts | Where-Object { $_ -eq "line1`nline2" -or $_ -eq "line1`r`nline2" }).Count -gt 0
$pptxOk = ($pcap -notmatch 'Repair') -and $pptTwo
Add-Check 'xml-nl-pptx' $pptxOk "caption=$pcap alt=$($joined.Replace("`r", '\r').Replace("`n", '\n'))"
Release-Com $slide
Close-PptPres $pres
