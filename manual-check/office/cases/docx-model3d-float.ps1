# 3D 模型按图片绕排写成浮动。Word 16.0 打开没有 Repair，形状是四周型。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$glb = (Ensure-OoxpGlb).Replace('\', '/')
$src = Get-OutPath 'docx-model3d-float-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{ paragraphs = @(@{ text = 'host' }) }
} 'docx-3d-float-create' | Out-Null
$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 3 }
} 'docx-3d-float-inspect'
$para = [string]$tree.paragraphs[0].id
$out = Get-OutPath 'docx-model3d-float.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $out.Replace('\', '/')
    ops = @(@{
        op = 'insertModel3d'; id = $para; file = $glb
        kind = 'anchor'; wrap = 'square'; alignH = 'left'; alignV = 'top'
    })
} 'docx-3d-float-insert' | Out-Null

$word = Ensure-MsWord $out
$doc = Open-WordDoc $word $out
$cap = [string]$doc.ActiveWindow.Caption
$n = $doc.Shapes.Count
$wrap = $null
$ty = $null
if ($n -ge 1) {
    $sh = $doc.Shapes.Item(1)
    $ty = $sh.Type
    $wrap = $sh.WrapFormat.Type
    Release-Com $sh
}
Add-Check 'docx-3d-float' (($cap -notmatch 'Repair') -and ($n -ge 1) -and ($wrap -eq 0)) "caption=$cap shapes=$n type=$ty wrap=$wrap"
Close-WordDoc $doc
