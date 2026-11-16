# exportSlideImage 预览补全：形状 blipFill、半透明两色渐变、圆角矩形/椭圆渐变、以及 a:br 多行文本。
# 金样是 PowerPoint 16.0 的 Slide.Export。像素差用 pixel-diff 标红，MaxDelta 为 0。
# Linux 上的自动化测试只锁住“确实画出来”，这一份才做 Office 像素对照。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

Add-Type -AssemblyName System.Drawing
$png = Get-OutPath 'pptx-preview-fill-dot.png'
$bmp = New-Object System.Drawing.Bitmap 8, 8
$color = [System.Drawing.Color]::FromArgb(255, 0x22, 0x88, 0x44)
foreach ($x in 0..7) {
    foreach ($y in 0..7) { $bmp.SetPixel($x, $y, $color) }
}
$bmp.Save($png, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()

$src = Get-OutPath 'pptx-preview-fill-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'create'
    path          = $src.Replace('\', '/')
    presentation  = @{
        slides = @(
            @{
                shapes = @(
                    @{
                        text = ''
                        fill = @{ type = 'blip'; file = $png.Replace('\', '/') }
                        x    = 1828800
                        y    = 914400
                        cx   = 914400
                        cy   = 914400
                    }
                )
            }
            @{
                shapes = @(
                    @{
                        text = ''
                        fill = @{
                            type  = 'grad'
                            ang   = 0
                            alpha = 50000
                            stops = @(
                                @{ pos = 0; color = 'FF0000' }
                                @{ pos = 100000; color = '0000FF' }
                            )
                        }
                        x    = 1828800
                        y    = 914400
                        cx   = 914400
                        cy   = 457200
                    }
                )
            }
            @{
                shapes = @(
                    @{
                        text = ''
                        prst = 'ellipse'
                        line = @{ color = $null }
                        fill = @{
                            type  = 'grad'
                            ang   = 0
                            stops = @(
                                @{ pos = 0; color = 'FF0000' }
                                @{ pos = 100000; color = '0000FF' }
                            )
                        }
                        x    = 914400
                        y    = 914400
                        cx   = 1828800
                        cy   = 1828800
                    }
                    @{
                        text = ''
                        prst = 'roundRect'
                        line = @{ color = $null }
                        fill = @{
                            type  = 'grad'
                            ang   = 0
                            stops = @(
                                @{ pos = 0; color = '00AA00' }
                                @{ pos = 100000; color = 'AA00AA' }
                            )
                        }
                        x    = 3657600
                        y    = 914400
                        cx   = 1828800
                        cy   = 914400
                    }
                )
            }
            @{
                shapes = @(
                    @{
                        text = "AAAA`nBBBB"
                        sz   = 3600
                        x    = 914400
                        y    = 914400
                        cx   = 5486400
                        cy   = 3657600
                    }
                )
            }
        )
    }
} 'pptx-preview-fill-create' | Out-Null

$deck = Get-OutPath 'pptx-preview-fill.pptx'
if (Test-Path $deck) { Remove-Item -Force $deck }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'apply'
    path          = $src.Replace('\', '/')
    output        = $deck.Replace('\', '/')
    ops           = @(
        @{ op = 'setPageBackground'; id = 'slide:1'; color = 'AABBCC' }
        @{ op = 'setPageBackground'; id = 'slide:2'; color = 'AABBCC' }
        @{ op = 'setPageBackground'; id = 'slide:3'; color = 'AABBCC' }
        @{ op = 'setPageBackground'; id = 'slide:4'; color = 'FFFFFF' }
    )
} 'pptx-preview-fill-bg' | Out-Null

$ppt = Ensure-MsPpt $deck
$diffScript = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..\pixel-diff.ps1'))
$cases = @(
    @{ id = 'slide:1'; tag = 'blip'; title = '形状图片填充' }
    @{ id = 'slide:2'; tag = 'alpha-grad'; title = '半透明两色渐变' }
    @{ id = 'slide:3'; tag = 'shape-grad'; title = '圆角矩形与椭圆渐变' }
    @{ id = 'slide:4'; tag = 'multiline'; title = '多行文本' }
)

foreach ($case in $cases) {
    $n = [int]($case.id -replace 'slide:', '')
    $lib = Get-OutPath ("pptx-preview-fill-{0}-lib.png" -f $case.tag)
    if (Test-Path $lib) { Remove-Item -Force $lib }
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'exportSlideImage'
        path          = $deck.Replace('\', '/')
        format        = 'pptx'
        id            = $case.id
        output        = $lib.Replace('\', '/')
    } ("pptx-preview-fill-{0}-export" -f $case.tag) | Out-Null

    $gold = Get-OutPath ("pptx-preview-fill-{0}-gold.png" -f $case.tag)
    $pres = $null
    $slide = $null
    try {
        $pres = Open-PptPres $ppt $deck
        $cap = Get-PptCaption $pres
        $slide = Get-PptSlide $pres $n
        if (Test-Path $gold) { Remove-Item -Force $gold }
        $slide.Export($gold, 'PNG')
        $ok = $cap -notmatch 'Repair'
        Add-Check ("pptx-preview-fill-{0}-open" -f $case.tag) $ok "caption=$cap $($case.title)"
    }
    finally {
        Release-Com $slide
        Close-PptPres $pres
    }

    $diff = Get-OutPath ("pptx-preview-fill-{0}-diff.png" -f $case.tag)
    $line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
    $code = $LASTEXITCODE
    $text = ($line | Out-String).Trim()
    Write-Log "pptx-preview-fill-$($case.tag)-px exit=$code $text"
    $pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
    Add-Check ("pptx-preview-fill-{0}-px" -f $case.tag) $pxOk "exit=$code $text"
}
