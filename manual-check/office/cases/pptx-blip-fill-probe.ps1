# 阶段二第 7 项补探测：让 PowerPoint 自己把图片填进形状和页背景，另存后看 a:blipFill。
# 只往 out 写探测产物，不改仓库里的 pptx。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.IO.Compression.FileSystem

$png = Ensure-OoxpPngA
$seed = Ensure-PptxCreate
$probeDeck = Get-OutPath 'pptx-blip-fill-probe.pptx'
$probeXml = Get-OutPath 'pptx-blip-fill-probe.xml'
if (Test-Path $probeDeck) { Remove-Item $probeDeck -Force }

$ppt = Ensure-MsPpt $seed
$pres = Open-PptPres $ppt $seed
$slide = $null
$shapes = $null
$shape = $null
$fill = $null
$bg = $null
$bgFill = $null
try {
    $slide = Get-PptSlide $pres 1
    $slide.FollowMasterBackground = 0
    $bg = $slide.Background
    $bgFill = $bg.Fill
    $bgFill.UserPicture($png)

    $shapes = $slide.Shapes
    $shape = $shapes.AddShape(1, 72, 72, 144, 72)
    $shape.Name = 'probe-blip'
    $fill = $shape.Fill
    $fill.UserPicture($png)

    $pres.SaveCopyAs($probeDeck)
    Write-Log "blip 探测另存 -> $probeDeck"
}
catch {
    Write-Log "blip 探测 COM 失败: $($_.Exception.Message)"
    Add-Check 'pptx-blip-fill-probe' $false $_.Exception.Message
    Close-PptPres $pres
    return
}
finally {
    Release-Com $fill
    Release-Com $shape
    Release-Com $shapes
    Release-Com $bgFill
    Release-Com $bg
    Release-Com $slide
}
Close-PptPres $pres

$xml = Get-ZipPartText $probeDeck 'ppt/slides/slide1.xml'
if ([string]::IsNullOrEmpty($xml)) { throw "$probeDeck 里没有 ppt/slides/slide1.xml" }
$hits = @([regex]::Matches($xml, '<[ap]:blipFill\b.*?</[ap]:blipFill>', 'Singleline') | ForEach-Object { $_.Value })
$body = ($hits | Select-Object -Unique) -join "`r`n`r`n"
[IO.File]::WriteAllText($probeXml, $body, [Text.UTF8Encoding]::new($false))
Write-Log "blip 探测片段 $($hits.Count) 条 -> $probeXml"

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

$ok = ($hits.Count -ge 1) -and $opened -and ($caption -notmatch 'Repair')
Add-Check 'pptx-blip-fill-probe' $ok "hits=$($hits.Count) cap=$caption"
