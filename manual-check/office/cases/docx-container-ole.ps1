# 文本框里的嵌入对象。Word 16.0 打开没有 Repair，框内能看见 OLE。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'docx-container-ole-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{ paragraphs = @(@{ text = 'host' }) }
} 'docx-container-ole-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 3 }
} 'docx-container-ole-inspect'
$para = [string]$tree.paragraphs[0].id
$boxed = Get-OutPath 'docx-container-ole-box.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $boxed.Replace('\', '/')
    ops = @(@{ op = 'insertTextBox'; id = $para; text = '框' })
} 'docx-container-ole-box' | Out-Null

$innerTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $boxed.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 3; textBoxes = $true; textBoxContent = $true }
} 'docx-container-ole-inner'
$inner = [string]$innerTree.paragraphs[0].textBoxes[0].paragraphs[0].id
$xlsx = Get-OutPath 'docx-container-ole-embed.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/'); format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = '嵌入' }) }) }
} 'docx-container-ole-xlsx' | Out-Null

$out = Get-OutPath 'docx-container-ole.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $boxed.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(@{ op = 'insertOle'; id = $inner; file = $xlsx.Replace('\', '/') })
} 'docx-container-ole-insert' | Out-Null

$word = Ensure-MsWord $out
$doc = Open-WordDoc $word $out
$cap = [string]$doc.ActiveWindow.Caption
$inside = 0
$shapes = $doc.Shapes
for ($i = 1; $i -le $shapes.Count; $i++) {
    $sh = $shapes.Item($i)
    try {
        $rng = $sh.TextFrame.TextRange
        $inside += [int]$rng.InlineShapes.Count
        Release-Com $rng
    } catch {
        Write-Log "shape $i 没有文本框：$($_.Exception.Message)"
    }
    Release-Com $sh
}
Release-Com $shapes
Add-Check 'docx-container-ole' (($cap -notmatch 'Repair') -and ($inside -ge 1)) "caption=$cap inside=$inside"
Close-WordDoc $doc
