# pptx create 补齐：docs/checklist-pptx.md 的 Cr1–Cr5。
# 节、形状级超链接、媒体、切换、预设进入淡入。不算放映引擎。
# OLE、自定义放映、讲义见 pptx-create-round4.ps1。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function Invoke-OoxpExpectFail($obj, [string]$name) {
    $path = Join-Path $global:OoxpReqDir "$name.json"
    Write-JsonFile $path $obj
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $global:OoxpExe
    $psi.Arguments = "--input `"$path`""
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = [System.Text.Encoding]::UTF8
    $psi.StandardErrorEncoding = [System.Text.Encoding]::UTF8
    $proc = [System.Diagnostics.Process]::Start($psi)
    $stdout = $proc.StandardOutput.ReadToEnd().Trim()
    $stderr = $proc.StandardError.ReadToEnd().Trim()
    $proc.WaitForExit()
    $code = $proc.ExitCode
    $proc.Dispose()
    Write-Log "ooxp $name 退出码 $code stderr=$stderr"
    return @{ ExitCode = $code; Stderr = $stderr; Stdout = $stdout }
}

function Get-PptErrorCode($raw) {
    try { return ($raw.Stderr | ConvertFrom-Json).error.code } catch { return $null }
}

function Get-PptShapeHyperlinkAddress($shape) {
    $settings = $null
    $item = $null
    $hl = $null
    try {
        $settings = $shape.ActionSettings
        $item = $settings.Item(1)
        $hl = $item.Hyperlink
        return [string]$hl.Address
    } catch {
        return ''
    } finally {
        Release-Com $hl
        Release-Com $item
        Release-Com $settings
    }
}

function Get-PptTransitionInfo($slide) {
    $t = $slide.SlideShowTransition
    try {
        return [pscustomobject]@{
            EntryEffect = [int]$t.EntryEffect
            Duration = [double]$t.Duration
        }
    }
    finally { Release-Com $t }
}

function Get-PptMainSeqCount($slide) {
    $tl = $slide.TimeLine
    $seq = $tl.MainSequence
    try { return [int]$seq.Count }
    finally {
        Release-Com $seq
        Release-Com $tl
    }
}

function Get-PptMediaCount($slide) {
    $n = 0
    $shapes = $slide.Shapes
    try {
        for ($i = 1; $i -le $shapes.Count; $i++) {
            $sh = $shapes.Item($i)
            try {
                if ([int]$sh.Type -eq 16) { $n++ }
            } finally { Release-Com $sh }
        }
    } finally { Release-Com $shapes }
    return $n
}

function Get-SectionSnap($pres) {
    $sp = $pres.SectionProperties
    try {
        $n = [int]$sp.Count
        $rows = @()
        for ($i = 1; $i -le $n; $i++) {
            $rows += [pscustomobject]@{
                Name  = [string]$sp.Name($i)
                First = [int]$sp.FirstSlide($i)
                Count = [int]$sp.SlidesCount($i)
            }
        }
        return @{ Count = $n; Rows = $rows }
    }
    finally { Release-Com $sp }
}

$path = Ensure-PptxCreateRemain -Force
$ppt = Ensure-MsPpt (Ensure-PptxCreate)
Close-PptPresByPath $ppt $path

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'pptx'
    scope = @{
        sections = $true
        hyperlinks = $true
        media = $true
        timing = $true
        slide = 1
    }
} 'pptx-cr-inspect'

$pres = Open-PptPres $ppt $path
$cap = Get-PptCaption $pres
$snap = Get-SectionSnap $pres
$slide1 = Get-PptSlide $pres 1
$slide2 = Get-PptSlide $pres 2
$shape1 = Get-PptShape $slide1 1
$addr = Get-PptShapeHyperlinkAddress $shape1
$tr = Get-PptTransitionInfo $slide1
$animN = Get-PptMainSeqCount $slide1
$mediaN = Get-PptMediaCount $slide2
Release-Com $shape1
Release-Com $slide1
Release-Com $slide2
Close-PptPres $pres

$sec0 = $tree.sections[0]
$sec1 = $tree.sections[1]
$cr1ok = ($cap -notmatch 'Repair') `
    -and ([int]$tree.sectionCount -eq 2) `
    -and ([string]$sec0.name -eq '开场') `
    -and ([string]$sec1.name -eq '内容') `
    -and ((@($sec1.slides) -join ',') -eq 'slide:2') `
    -and ([int]$snap.Count -eq 2) `
    -and ([string]$snap.Rows[0].Name -eq '开场') `
    -and ([int]$snap.Rows[0].First -eq 1) `
    -and ([string]$snap.Rows[1].Name -eq '内容') `
    -and ([int]$snap.Rows[1].First -eq 2)
Add-Check 'pptx-Cr1' $cr1ok "caption=$cap sec=$($tree.sectionCount) com=$($snap.Count) n0=$($snap.Rows[0].Name) n1=$($snap.Rows[1].Name) first1=$($snap.Rows[1].First)"

$hl = @($tree.slides[0].hyperlinks | Where-Object { $_ }) | Select-Object -First 1
$cr2ok = ([int]$tree.slides[0].hyperlinkCount -eq 1) `
    -and ($null -ne $hl) `
    -and ([string]$hl.url -eq 'https://example.com/') `
    -and ($addr -match 'example.com')
Add-Check 'pptx-Cr2' $cr2ok "count=$($tree.slides[0].hyperlinkCount) inspect=$($hl.url) addr=$addr"

$slideMedia = $tree.slides[1]
$cr3ok = ($cap -notmatch 'Repair') `
    -and ([int]$slideMedia.videoCount -eq 1) `
    -and ([int]$slideMedia.audioCount -eq 1) `
    -and ($mediaN -ge 1)
Add-Check 'pptx-Cr3' $cr3ok "caption=$cap video=$($slideMedia.videoCount) audio=$($slideMedia.audioCount) type16=$mediaN"

$trType = $null
if ($null -ne $tree.slides[0].transition -and $tree.slides[0].transition -ne [DBNull]::Value) {
    try { $trType = [string]$tree.slides[0].transition.type } catch { $trType = $null }
}
# ppEffectFadeSmoothly = 3849
$cr4ok = ($trType -eq 'fade') -and ([int]$tr.EntryEffect -eq 3849)
Add-Check 'pptx-Cr4' $cr4ok "type=$trType entry=$($tr.EntryEffect)"

$failHl = Get-OutPath 'pptx-create-remain-fail-hyperlinks.pptx'
$failTiming = Get-OutPath 'pptx-create-remain-fail-timing.pptx'
$failNull = Get-OutPath 'pptx-create-remain-fail-null.pptx'
$failXlsx = Get-OutPath 'pptx-create-remain-fail-xlsx.xlsx'
foreach ($p in @($failHl, $failTiming, $failNull, $failXlsx)) {
    if (Test-Path $p) { Remove-Item -Force $p }
}

$rawHl = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'create'
    path = $failHl.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(@{ shapes = @(@{ text = 'a' }); hyperlinks = @() })
    }
} 'pptx-cr-fail-hyperlinks'
$rawTiming = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'create'
    path = $failTiming.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(@{
            shapes = @(@{ text = 'a' })
            timing = @()
        })
    }
} 'pptx-cr-fail-timing'
$rawNull = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'create'
    path = $failNull.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(@{
            shapes = @(@{ text = 'a' })
            transition = @{ type = $null }
        })
    }
} 'pptx-cr-fail-null'
$rawXlsx = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'create'
    path = $failXlsx.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sections = @(@{ name = 'A'; start = 1 })
        sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) })
    }
} 'pptx-cr-fail-xlsx'

$timing = @($tree.slides[0].timing | Where-Object { $_ }) | Select-Object -First 1
$cr5ok = ([int]$animN -eq 1) `
    -and ([int]$tree.slides[0].animCount -eq 1) `
    -and ([string]$timing.type -eq 'fade') `
    -and ((Get-PptErrorCode $rawHl) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $rawTiming) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $rawNull) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $rawXlsx) -eq 'InvalidField') `
    -and (-not (Test-Path $failHl)) `
    -and (-not (Test-Path $failTiming)) `
    -and (-not (Test-Path $failNull)) `
    -and (-not (Test-Path $failXlsx))
Add-Check 'pptx-Cr5' $cr5ok "anim=$animN type=$($timing.type) hl=$(Get-PptErrorCode $rawHl) timing=$(Get-PptErrorCode $rawTiming) null=$(Get-PptErrorCode $rawNull) x=$(Get-PptErrorCode $rawXlsx)"
