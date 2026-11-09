# 阶段二第 8 项的 Office 16.0 探测：让 PowerPoint 自己改图层顺序再另存，
# 看 p:spTree 子元素顺序是不是就是 z-order，以及 cNvPr/@id 会不会跟着漂移。
# 结论用来定 index / before / after 的契约，结果回来前不写契约、不改 Rust。
# 只往 out 目录写探测产物，不改仓库里的 pptx，不挂待做序号。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.IO.Compression.FileSystem

$seed = Ensure-PptxCreate
$beforeDeck = Get-OutPath 'pptx-zorder-probe-before.pptx'
$afterDeck = Get-OutPath 'pptx-zorder-probe-after.pptx'
$probeXml = Get-OutPath 'pptx-zorder-probe.xml'
foreach ($p in @($beforeDeck, $afterDeck)) {
    if (Test-Path $p) { Remove-Item $p -Force }
}

# MsoZOrderCmd：msoBringToFront = 0，msoSendToBack = 1。
$zBringFront = 0
$zSendBack = 1

# spTree 自己的 nvGrpSpPr 里也有一个 cNvPr id="1"，那是组本身不是图层，先剔掉。
function Get-SpTreeOrder([string]$deck) {
    $xml = Get-ZipPartText $deck 'ppt/slides/slide1.xml'
    if ([string]::IsNullOrEmpty($xml)) { throw "$deck 里没有 ppt/slides/slide1.xml" }
    $tree = [regex]::Match($xml, '<p:spTree\b.*?</p:spTree>', 'Singleline')
    if (-not $tree.Success) { throw "$deck 里没有 p:spTree" }
    $body = [regex]::Replace($tree.Value, '<p:(nvGrpSpPr|grpSpPr)\b.*?</p:\1>', '', 'Singleline')
    $rows = @([regex]::Matches($body, '<p:cNvPr\b[^>]*>') | ForEach-Object {
        $tag = $_.Value
        $id = [regex]::Match($tag, '\bid="(\d+)"')
        $name = [regex]::Match($tag, '\bname="([^"]*)"')
        if ($id.Success) { "id=$($id.Groups[1].Value) name=$($name.Groups[1].Value)" }
    })
    return @{ Rows = $rows; Xml = $tree.Value }
}

$ppt = Ensure-MsPpt $seed
$pres = Open-PptPres $ppt $seed
$slide = $null
$shapes = $null
$added = New-Object System.Collections.ArrayList
try {
    $slide = Get-PptSlide $pres 1
    $shapes = $slide.Shapes
    # 宿主第一页已有两个文本框，再插两个矩形，凑出四个可辨认的图层。
    foreach ($spec in @(@('probe-a', 72, 72), @('probe-b', 216, 144))) {
        $s = $shapes.AddShape(1, $spec[1], $spec[2], 144, 72)
        $s.Name = $spec[0]
        [void]$added.Add($s)
    }
    Write-Log ("z-order 另存前: " + (($added | ForEach-Object { "$($_.Name)/$($_.ZOrderPosition)" }) -join ' '))
    $pres.SaveCopyAs($beforeDeck)

    # 把 probe-a 提到最前、probe-b 压到最后，制造一次明确的顺序变化。
    $added[0].ZOrder($zBringFront)
    $added[1].ZOrder($zSendBack)
    Write-Log ("z-order 另存后: " + (($added | ForEach-Object { "$($_.Name)/$($_.ZOrderPosition)" }) -join ' '))
    $pres.SaveCopyAs($afterDeck)
}
catch {
    Write-Log "z-order 探测 COM 失败: $($_.Exception.Message)"
    Add-Check 'pptx-zorder-probe' $false $_.Exception.Message
    Close-PptPres $pres
    return
}
finally {
    foreach ($s in $added) { Release-Com $s }
    Release-Com $shapes
    Release-Com $slide
}
Close-PptPres $pres

$first = Get-SpTreeOrder $beforeDeck
$second = Get-SpTreeOrder $afterDeck
$idsBefore = @($first.Rows | ForEach-Object { ($_ -split ' ')[0] } | Sort-Object)
$idsAfter = @($second.Rows | ForEach-Object { ($_ -split ' ')[0] } | Sort-Object)
$orderChanged = (($first.Rows -join '|') -ne ($second.Rows -join '|'))
$idsStable = (($idsBefore -join '|') -eq ($idsAfter -join '|'))

$beforeRows = $first.Rows -join "`r`n"
$afterRows = $second.Rows -join "`r`n"
$body = @(
    '<!-- before: PowerPoint 自己另存，未改 z-order -->',
    $beforeRows,
    '',
    '<!-- after: probe-a 提到最前、probe-b 压到最后 -->',
    $afterRows,
    '',
    '<!-- spTree after -->',
    $second.Xml
) -join "`r`n"
[IO.File]::WriteAllText($probeXml, $body, [Text.UTF8Encoding]::new($false))
Write-Log "z-order 探测片段 -> $probeXml"

# 另存件还要能被 PowerPoint 16.0 打开且标题栏没有 Repair 才算数。
$opened = $false
$caption = ''
$checkPres = $null
try {
    $checkPres = Open-PptPres $ppt $afterDeck
    $opened = $true
    $caption = Get-PptCaption $checkPres
}
catch {
    $caption = "打开失败: $($_.Exception.Message)"
}
finally {
    Close-PptPres $checkPres
}

$ok = $orderChanged -and $idsStable -and ($second.Rows.Count -ge 4) `
    -and $opened -and ($caption -notmatch 'Repair')
Add-Check 'pptx-zorder-probe' $ok "orderChanged=$orderChanged idsStable=$idsStable n=$($second.Rows.Count) cap=$caption"
