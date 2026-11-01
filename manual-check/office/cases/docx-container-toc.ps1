# 文本框里的目录。Word 16.0 更新域后能看见标题，没有 Repair。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'docx-container-toc-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = '第一章' }
            @{ text = '正文' }
        )
    }
} 'docx-container-toc-create' | Out-Null

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 4 }
} 'docx-container-toc-inspect'
$para = [string]$tree.paragraphs[1].id
$boxed = Get-OutPath 'docx-container-toc-box.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $boxed.Replace('\', '/')
    ops = @(@{ op = 'insertTextBox'; id = $para; text = '框' })
} 'docx-container-toc-box' | Out-Null

$innerTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $boxed.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 4; textBoxes = $true; textBoxContent = $true }
} 'docx-container-toc-inner'
$inner = [string]$innerTree.paragraphs[1].textBoxes[0].paragraphs[0].id
$out = Get-OutPath 'docx-container-toc.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $boxed.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(@{ op = 'insertToc'; id = $inner })
} 'docx-container-toc-insert' | Out-Null

$word = Ensure-MsWord $out
$doc = Open-WordDoc $word $out
$cap = [string]$doc.ActiveWindow.Caption
$text = ''
$updated = $false
$shapes = $doc.Shapes
if ($shapes.Count -ge 1) {
    $sh = $shapes.Item(1)
    try {
        $rng = $sh.TextFrame.TextRange
        $boxFields = $rng.Fields
        try {
            if ($boxFields.Count -ge 1) {
                $f = $boxFields.Item(1)
                try { $updated = [bool]$f.Update() } finally { Release-Com $f }
            }
        } finally { Release-Com $boxFields }
        $text = [string]$rng.Text
        Release-Com $rng
    } finally { Release-Com $sh }
}
Release-Com $shapes
Add-Check 'docx-container-toc' (($cap -notmatch 'Repair') -and $updated -and ($text -like '*第一章*')) "caption=$cap updated=$updated text=$text"
Close-WordDoc $doc
