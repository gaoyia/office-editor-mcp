# 阶段二第 6、7 项的 Office 16.0 探测：让 PowerPoint 自己写出 p:bg / gradFill / alpha，
# 再解包取 XML 片段，作为写契约前的对照依据。
# 只往 out 目录写探测产物，不改仓库里的 pptx，不挂待做序号。
# 探测结果回来前不写契约、不改 Rust 代码（roadmap/pptx.md「先探测再进待做」）。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.IO.Compression.FileSystem

$seed = Ensure-PptxCreate
$probeDeck = Get-OutPath 'pptx-bg-fill-probe.pptx'
$probeXml = Get-OutPath 'pptx-bg-fill-probe.xml'
if (Test-Path $probeDeck) { Remove-Item $probeDeck -Force }

$ppt = Ensure-MsPpt $seed
$pres = Open-PptPres $ppt $seed
try {
    $slides = $pres.Slides
    $slide = $slides.Item(1)

    # 第 6 项：页背景。FollowMasterBackground = msoFalse(0) 才让 p:bg 落到本页。
    # RGB 是 OLE BGR，0x2B5CFF 对应 srgbClr 5CFF2B。
    $slide.FollowMasterBackground = 0
    $bg = $slide.Background
    $bgFill = $bg.Fill
    $bgFill.Solid()
    $bgFill.ForeColor.RGB = 0x2B5CFF
    $bgFill.Transparency = 0.35

    # 第 7 项：形状填充的渐变与透明度。
    $shapes = $slide.Shapes
    $shape = $shapes.Item(1)
    $shapeFill = $shape.Fill
    $shapeFill.TwoColorGradient(1, 1)
    $shapeFill.Transparency = 0.5

    $pres.SaveCopyAs($probeDeck)
    Write-Log "bg/fill 探测另存 -> $probeDeck"
}
catch {
    # COM 属性在个别 PowerPoint 版本上名字有差异，留下错误再退出，别拖死整轮。
    Write-Log "bg/fill 探测 COM 失败: $($_.Exception.Message)"
    Add-Check 'pptx-bg-fill-probe' $false $_.Exception.Message
    Close-PptPres $pres
    return
}
finally {
    Release-Com $shapeFill
    Release-Com $shape
    Release-Com $shapes
    Release-Com $bgFill
    Release-Com $bg
    Release-Com $slide
    Release-Com $slides
}
Close-PptPres $pres

# 解包取片段。只保留与本次探测相关的元素，便于写契约时逐项对照。
$zip = [System.IO.Compression.ZipFile]::OpenRead($probeDeck)
try {
    $entry = $zip.Entries |
        Where-Object { $_.FullName -match '^ppt/slides/slide\d+\.xml$' } |
        Select-Object -First 1
    if (-not $entry) { throw '包里没有 ppt/slides/slideN.xml' }
    $reader = New-Object System.IO.StreamReader($entry.Open())
    try { $xml = $reader.ReadToEnd() } finally { $reader.Dispose() }
}
finally {
    $zip.Dispose()
}

$patterns = @(
    '<p:bg\b.*?</p:bg>',
    '<a:gradFill\b.*?</a:gradFill>',
    '<a:blipFill\b.*?</a:blipFill>',
    '<a:solidFill>.*?</a:solidFill>',
    '<a:alpha\s+[^>]*/>'
)
$hits = foreach ($pattern in $patterns) {
    [regex]::Matches($xml, $pattern, 'Singleline') | ForEach-Object { $_.Value }
}
$body = ($hits | Select-Object -Unique) -join "`r`n`r`n"
[IO.File]::WriteAllText($probeXml, $body, [Text.UTF8Encoding]::new($false))
Write-Log "bg/fill 探测片段 $($hits.Count) 条 -> $probeXml"
Add-Check 'pptx-bg-fill-probe' ($hits.Count -gt 0) $probeXml
