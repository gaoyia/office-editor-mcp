# 直角矩形描边外角。金样是 PowerPoint 16.0 的 Slide.Export。
# 红区回绘制引擎修，不要加大 MaxDelta。覆盖多种尺寸、位置、线宽、填色、无填充和有色页底，避免只对一个样例。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'pptx-preview-stroke-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'create'
    path          = $src.Replace('\', '/')
    presentation  = @{
        slides = @(
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = 'FFFFFF'; line = @{ color = '000000'; width = 9525 }; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = 'FFFFFF'; line = @{ color = '000000'; width = 9525 }; x = 914400; y = 914400; cx = 1828800; cy = 762000 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = 'FFFFFF'; line = @{ color = '000000'; width = 9525 }; x = 914400; y = 914400; cx = 762000; cy = 1828800 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = 'FFFFFF'; line = @{ color = '000000'; width = 9525 }; x = 457200; y = 609600; cx = 1828800; cy = 762000 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = '0000FF'; line = @{ color = 'FF0000'; width = 9525 }; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = $null; line = @{ color = '000000'; width = 9525 }; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = 'FFFFFF'; line = @{ color = '000000'; width = 9525 }; x = 914400; y = 914400; cx = 190500; cy = 190500 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = 'FFFFFF'; line = @{ color = '000000'; width = 19050 }; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = 'FFFFFF'; line = @{ color = '000000'; width = 19050 }; x = 914400; y = 914400; cx = 1828800; cy = 762000 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = 'FFFFFF'; line = @{ color = '000000'; width = 19050 }; x = 457200; y = 609600; cx = 1828800; cy = 762000 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = '0000FF'; line = @{ color = 'FF0000'; width = 19050 }; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = $null; line = @{ color = '000000'; width = 19050 }; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = 'FFFFFF'; line = @{ color = '000000'; width = 9525 }; x = 961275; y = 923925; cx = 1828800; cy = 1828800 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = 'FFFFFF'; line = @{ color = '000000'; width = 9525 }; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = 'FFFFFF'; line = @{ color = '000000'; width = 19050 }; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = '0000FF'; line = @{ color = 'FF0000'; width = 9525 }; x = 914400; y = 914400; cx = 1828800; cy = 1828800 }) }
        )
    }
} 'pptx-preview-stroke-create' | Out-Null

$deck = Get-OutPath 'pptx-preview-stroke.pptx'
if (Test-Path $deck) { Remove-Item -Force $deck }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'apply'
    path          = $src.Replace('\', '/')
    output        = $deck.Replace('\', '/')
    ops           = @(
        @{ op = 'setPageBackground'; id = 'slide:14'; color = 'AABBCC' }
        @{ op = 'setPageBackground'; id = 'slide:15'; color = 'AABBCC' }
        @{ op = 'setPageBackground'; id = 'slide:16'; color = 'AABBCC' }
    )
} 'pptx-preview-stroke-bg' | Out-Null

$ppt = Ensure-MsPpt $deck
$diffScript = [IO.Path]::Combine($PSScriptRoot, '..', '..', 'pixel-diff.ps1')
$cases = @(
    @{ tag = 's1-sq'; id = 'slide:1'; n = 1; title = '1px square white/black' }
    @{ tag = 's1-wide'; id = 'slide:2'; n = 2; title = '1px wide' }
    @{ tag = 's1-tall'; id = 'slide:3'; n = 3; title = '1px tall' }
    @{ tag = 's1-pos'; id = 'slide:4'; n = 4; title = '1px other origin' }
    @{ tag = 's1-rb'; id = 'slide:5'; n = 5; title = '1px blue/red' }
    @{ tag = 's1-empty'; id = 'slide:6'; n = 6; title = '1px noFill' }
    @{ tag = 's1-small'; id = 'slide:7'; n = 7; title = '1px 20px box' }
    @{ tag = 's2-sq'; id = 'slide:8'; n = 8; title = '2px square white/black' }
    @{ tag = 's2-wide'; id = 'slide:9'; n = 9; title = '2px wide' }
    @{ tag = 's2-pos'; id = 'slide:10'; n = 10; title = '2px other origin' }
    @{ tag = 's2-rb'; id = 'slide:11'; n = 11; title = '2px blue/red' }
    @{ tag = 's2-empty'; id = 'slide:12'; n = 12; title = '2px noFill' }
    @{ tag = 's1-off'; id = 'slide:13'; n = 13; title = '1px rounded EMU origin' }
    @{ tag = 's1-bg'; id = 'slide:14'; n = 14; title = '1px on AABBCC' }
    @{ tag = 's2-bg'; id = 'slide:15'; n = 15; title = '2px on AABBCC' }
    @{ tag = 's1-rb-bg'; id = 'slide:16'; n = 16; title = '1px blue/red on AABBCC' }
)
foreach ($case in $cases) {
    $lib = Get-OutPath ("pptx-preview-stroke-{0}-lib.png" -f $case.tag)
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'exportSlideImage'
        path          = $deck.Replace('\', '/')
        format        = 'pptx'
        id            = $case.id
        output        = $lib.Replace('\', '/')
    } ("pptx-preview-stroke-{0}-export" -f $case.tag) | Out-Null

    $gold = Get-OutPath ("pptx-preview-stroke-{0}-gold.png" -f $case.tag)
    $pres = $null
    $slide = $null
    try {
        $pres = Open-PptPres $ppt $deck
        $cap = Get-PptCaption $pres
        $slide = Get-PptSlide $pres $case.n
        if (Test-Path $gold) { Remove-Item -Force $gold }
        $slide.Export($gold, 'PNG')
        $ok = $cap -notmatch 'Repair'
        Add-Check ("pptx-preview-stroke-{0}-open" -f $case.tag) $ok "caption=$cap $($case.title)"
    }
    finally {
        Release-Com $slide
        Close-PptPres $pres
    }

    $diff = Get-OutPath ("pptx-preview-stroke-{0}-diff.png" -f $case.tag)
    $line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
    $code = $LASTEXITCODE
    $text = ($line | Out-String).Trim()
    Write-Log "pptx-preview-stroke-$($case.tag)-px exit=$code $text"
    $pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
    Add-Check ("pptx-preview-stroke-{0}-px" -f $case.tag) $pxOk "exit=$code $text"
}
