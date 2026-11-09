# 阶段二第 9、10 项的 Office 16.0 探测：让 PowerPoint 自己把形状放到画布外，
# 另存后看 a:off 的负值、a:ext，以及完整 p:sp 里的 xfrm / prstGeom / solidFill / ln。
# 第 9 项据此判断负 EMU 能否直接进树；第 10 项据此对照 create 一步写几何的 XML。
# 只往 out 目录写探测产物，不改仓库里的 pptx，不挂待做序号。
# 探测结果回来前不写契约、不改 Rust 代码（roadmap/pptx.md「先探测再进待做」）。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.IO.Compression.FileSystem

$seed = Ensure-PptxCreate
$probeDeck = Get-OutPath 'pptx-offslide-geom-probe.pptx'
$probeXml = Get-OutPath 'pptx-offslide-geom-probe.xml'
$spXml = Get-OutPath 'pptx-shape-create-geom-probe.xml'
if (Test-Path $probeDeck) { Remove-Item $probeDeck -Force }

$ppt = Ensure-MsPpt $seed
$pres = Open-PptPres $ppt $seed
$slide = $null
$shapes = $null
$shape = $null
$fill = $null
$fillFore = $null
$line = $null
$lineFore = $null
$left = 0
$top = 0
try {
    $slide = Get-PptSlide $pres 1
    $shapes = $slide.Shapes
    # PowerPoint 的 Left / Top 是磅，1 磅 = 12700 EMU。负 36 磅就是 -457200 EMU。
    $shape = $shapes.AddShape(1, 72, 72, 144, 72)
    $shape.Name = 'probe-offslide'
    $shape.Left = -36
    $shape.Top = -18

    # 填满色与描边，让 p:sp 里同时出现 a:solidFill 与 a:ln，供第 10 项对照。
    $fill = $shape.Fill
    $fill.Solid()
    $fillFore = $fill.ForeColor
    $fillFore.RGB = 0xFF5C2B
    $line = $shape.Line
    $line.Weight = 2
    $lineFore = $line.ForeColor
    $lineFore.RGB = 0x554433

    $left = [double]$shape.Left
    $top = [double]$shape.Top
    $pres.SaveCopyAs($probeDeck)
    Write-Log "负坐标探测另存 -> $probeDeck（COM 读回 left=$left top=$top 磅）"
}
catch {
    Write-Log "负坐标探测 COM 失败: $($_.Exception.Message)"
    Add-Check 'pptx-offslide-geom-probe' $false $_.Exception.Message
    Add-Check 'pptx-shape-create-geom-probe' $false 'COM 失败，未产出 p:sp 片段'
    Close-PptPres $pres
    return
}
finally {
    Release-Com $lineFore
    Release-Com $line
    Release-Com $fillFore
    Release-Com $fill
    Release-Com $shape
    Release-Com $shapes
    Release-Com $slide
}
Close-PptPres $pres

$xml = Get-ZipPartText $probeDeck 'ppt/slides/slide1.xml'
if ([string]::IsNullOrEmpty($xml)) { throw "$probeDeck 里没有 ppt/slides/slide1.xml" }

$offs = @([regex]::Matches($xml, '<a:off\b[^>]*/>') | ForEach-Object { $_.Value })
$exts = @([regex]::Matches($xml, '<a:ext\b[^>]*/>') | ForEach-Object { $_.Value })
$negOff = @($offs | Where-Object { $_ -match '\b[xy]="-' })
$offRows = ($offs + $exts) -join "`r`n"
$body = @(
    '<!-- 画布外形状的 a:off / a:ext；负号就是第 9 项要确认的写法 -->',
    $offRows
) -join "`r`n"
[IO.File]::WriteAllText($probeXml, $body, [Text.UTF8Encoding]::new($false))

$sps = @([regex]::Matches($xml, '<p:sp\b.*?</p:sp>', 'Singleline') | ForEach-Object { $_.Value })
$target = @($sps | Where-Object { $_ -match 'name="probe-offslide"' }) | Select-Object -First 1
$geomOk = ($null -ne $target) `
    -and ($target -match '<a:off\b') -and ($target -match '<a:ext\b') `
    -and ($target -match '<a:prstGeom\b') -and ($target -match '<a:solidFill\b') `
    -and ($target -match '<a:ln\b')
[IO.File]::WriteAllText($spXml, [string]$target, [Text.UTF8Encoding]::new($false))
Write-Log "完整 p:sp 片段 -> $spXml"
Add-Check 'pptx-shape-create-geom-probe' $geomOk "spCount=$($sps.Count) targetFound=$($null -ne $target)"

# 另存件还要能被 PowerPoint 16.0 打开且标题栏没有 Repair 才算数。
$opened = $false
$caption = ''
$checkPres = $null
try {
    $checkPres = Open-PptPres $ppt $probeDeck
    $opened = $true
    $caption = Get-PptCaption $checkPres
}
catch {
    $caption = "打开失败: $($_.Exception.Message)"
}
finally {
    Close-PptPres $checkPres
}

$offOk = ($negOff.Count -gt 0) -and ($left -lt 0) -and ($top -lt 0) `
    -and $opened -and ($caption -notmatch 'Repair')
Add-Check 'pptx-offslide-geom-probe' $offOk "negOff=$($negOff.Count) left=$left top=$top cap=$caption"
