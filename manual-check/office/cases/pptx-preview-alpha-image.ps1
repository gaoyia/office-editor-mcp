# 半透明纯色矩形叠在不透明页底上。金样是 PowerPoint 16.0 的 Slide.Export。
# 红区回绘制引擎修，不要加大 MaxDelta。覆盖多种独立填色、页底、10/25/50/75% 透明、重叠和裁剪，避免只对一个样例。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$src = Get-OutPath 'pptx-preview-alpha-src.pptx'
if (Test-Path $src) { Remove-Item -Force $src }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'create'
    path          = $src.Replace('\', '/')
    presentation  = @{
        slides = @(
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = @{ type = 'solid'; color = 'EE4455'; alpha = 50000 }; x = 1828800; y = 914400; cx = 914400; cy = 457200 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = @{ type = 'solid'; color = 'EE4455'; alpha = 50000 }; x = 1828800; y = 914400; cx = 914400; cy = 457200 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = @{ type = 'solid'; color = 'EE4455'; alpha = 50000 }; x = 1828800; y = 914400; cx = 914400; cy = 457200 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = @{ type = 'solid'; color = '0000FF'; alpha = 50000 }; x = 1828800; y = 914400; cx = 914400; cy = 457200 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = @{ type = 'solid'; color = 'FFFFFF'; alpha = 50000 }; x = 1828800; y = 914400; cx = 914400; cy = 457200 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = @{ type = 'solid'; color = '000000'; alpha = 50000 }; x = 1828800; y = 914400; cx = 914400; cy = 457200 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = @{ type = 'solid'; color = '00FF00'; alpha = 50000 }; x = 1828800; y = 914400; cx = 914400; cy = 457200 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = @{ type = 'solid'; color = 'EE4455'; alpha = 50000 }; x = 1828800; y = 914400; cx = 914400; cy = 457200 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = @{ type = 'solid'; color = 'EE4455'; alpha = 25000 }; x = 1828800; y = 914400; cx = 914400; cy = 457200 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = @{ type = 'solid'; color = 'EE4455'; alpha = 75000 }; x = 1828800; y = 914400; cx = 914400; cy = 457200 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = @{ type = 'solid'; color = 'EE4455'; alpha = 10000 }; x = 1828800; y = 914400; cx = 914400; cy = 457200 }) }
            @{ shapes = @(
                @{ text = ''; prst = 'rect'; fill = 'EE4455'; x = 1828800; y = 914400; cx = 914400; cy = 457200 },
                @{ text = ''; prst = 'rect'; fill = @{ type = 'solid'; color = '0000FF'; alpha = 50000 }; x = 2057400; y = 914400; cx = 914400; cy = 457200 }
            ) }
            @{ shapes = @(
                @{ text = ''; prst = 'rect'; fill = @{ type = 'solid'; color = 'EE4455'; alpha = 50000 }; x = 1828800; y = 914400; cx = 914400; cy = 457200 },
                @{ text = ''; prst = 'rect'; fill = @{ type = 'solid'; color = '0000FF'; alpha = 50000 }; x = 2057400; y = 914400; cx = 914400; cy = 457200 }
            ) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = @{ type = 'solid'; color = 'EE4455'; alpha = 50000 }; x = 1828800; y = 914400; cx = 914400; cy = 457200 }) }
            @{ shapes = @(@{ text = ''; prst = 'rect'; fill = @{ type = 'solid'; color = 'EE4455'; alpha = 50000 }; x = -9525; y = 1828800; cx = 28575; cy = 457200 }) }
        )
    }
} 'pptx-preview-alpha-create' | Out-Null

$deck = Get-OutPath 'pptx-preview-alpha.pptx'
if (Test-Path $deck) { Remove-Item -Force $deck }
Invoke-OoxpRequest @{
    schemaVersion = 1
    action        = 'apply'
    path          = $src.Replace('\', '/')
    output        = $deck.Replace('\', '/')
    ops           = @(
        @{ op = 'setPageBackground'; id = 'slide:1'; color = 'AABBCC' }
        @{ op = 'setPageBackground'; id = 'slide:2'; color = 'FFFFFF' }
        @{ op = 'setPageBackground'; id = 'slide:3'; color = '000000' }
        @{ op = 'setPageBackground'; id = 'slide:4'; color = 'AABBCC' }
        @{ op = 'setPageBackground'; id = 'slide:5'; color = 'AABBCC' }
        @{ op = 'setPageBackground'; id = 'slide:6'; color = 'AABBCC' }
        @{ op = 'setPageBackground'; id = 'slide:7'; color = 'AABBCC' }
        @{ op = 'setPageBackground'; id = 'slide:8'; color = '112233' }
        @{ op = 'setPageBackground'; id = 'slide:9'; color = 'AABBCC' }
        @{ op = 'setPageBackground'; id = 'slide:10'; color = 'AABBCC' }
        @{ op = 'setPageBackground'; id = 'slide:11'; color = 'AABBCC' }
        @{ op = 'setPageBackground'; id = 'slide:12'; color = 'AABBCC' }
        @{ op = 'setPageBackground'; id = 'slide:13'; color = 'AABBCC' }
        @{ op = 'setPageBackground'; id = 'slide:15'; color = 'AABBCC' }
    )
} 'pptx-preview-alpha-bg' | Out-Null

$ppt = Ensure-MsPpt $deck
$diffScript = [IO.Path]::Combine($PSScriptRoot, '..', '..', 'pixel-diff.ps1')
$cases = @(
    @{ tag = 's50-ee-aabbcc'; id = 'slide:1'; n = 1; title = '50% EE4455 on AABBCC' }
    @{ tag = 's50-ee-white'; id = 'slide:2'; n = 2; title = '50% EE4455 on white' }
    @{ tag = 's50-ee-black'; id = 'slide:3'; n = 3; title = '50% EE4455 on black' }
    @{ tag = 's50-blue-aabbcc'; id = 'slide:4'; n = 4; title = '50% 0000FF on AABBCC' }
    @{ tag = 's50-white-aabbcc'; id = 'slide:5'; n = 5; title = '50% white on AABBCC' }
    @{ tag = 's50-black-aabbcc'; id = 'slide:6'; n = 6; title = '50% black on AABBCC' }
    @{ tag = 's50-green-aabbcc'; id = 'slide:7'; n = 7; title = '50% 00FF00 on AABBCC' }
    @{ tag = 's50-ee-112233'; id = 'slide:8'; n = 8; title = '50% EE4455 on 112233' }
    @{ tag = 's25-ee-aabbcc'; id = 'slide:9'; n = 9; title = '25% EE4455 on AABBCC' }
    @{ tag = 's75-ee-aabbcc'; id = 'slide:10'; n = 10; title = '75% EE4455 on AABBCC' }
    @{ tag = 's10-ee-aabbcc'; id = 'slide:11'; n = 11; title = '10% EE4455 on AABBCC' }
    @{ tag = 'overlap-op-50'; id = 'slide:12'; n = 12; title = 'opaque then 50% overlap' }
    @{ tag = 'overlap-two-50'; id = 'slide:13'; n = 13; title = 'two 50% overlap' }
    @{ tag = 's50-ee-defaultwhite'; id = 'slide:14'; n = 14; title = '50% EE4455 on default white' }
    @{ tag = 's50-ee-clip'; id = 'slide:15'; n = 15; title = '50% EE4455 clipped at left' }
)
foreach ($case in $cases) {
    $lib = Get-OutPath ("pptx-preview-alpha-{0}-lib.png" -f $case.tag)
    Invoke-OoxpRequest @{
        schemaVersion = 1
        action        = 'exportSlideImage'
        path          = $deck.Replace('\', '/')
        format        = 'pptx'
        id            = $case.id
        output        = $lib.Replace('\', '/')
    } ("pptx-preview-alpha-{0}-export" -f $case.tag) | Out-Null

    $gold = Get-OutPath ("pptx-preview-alpha-{0}-gold.png" -f $case.tag)
    $pres = $null
    $slide = $null
    try {
        $pres = Open-PptPres $ppt $deck
        $cap = Get-PptCaption $pres
        $slide = Get-PptSlide $pres $case.n
        if (Test-Path $gold) { Remove-Item -Force $gold }
        $slide.Export($gold, 'PNG')
        $ok = $cap -notmatch 'Repair'
        Add-Check ("pptx-preview-alpha-{0}-open" -f $case.tag) $ok "caption=$cap $($case.title)"
    }
    finally {
        Release-Com $slide
        Close-PptPres $pres
    }

    $diff = Get-OutPath ("pptx-preview-alpha-{0}-diff.png" -f $case.tag)
    $line = & $diffScript -Left $gold -Right $lib -Out $diff -MaxDelta 0
    $code = $LASTEXITCODE
    $text = ($line | Out-String).Trim()
    Write-Log "pptx-preview-alpha-$($case.tag)-px exit=$code $text"
    $pxOk = ($code -ne 2) -and ($text -match 'differ=0\b') -and ($text -match 'maxDelta=0\b')
    Add-Check ("pptx-preview-alpha-{0}-px" -f $case.tag) $pxOk "exit=$code $text"
}