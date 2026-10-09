# pptx create 第四轮：docs/checklist-pptx.md 的 Cq1–Cq5。
# OLE、Zoom、域、连接线、自定义放映、讲义、备注动画、放映设置与修改保护。
# 不算放映引擎。带哈希产物不要 Open-PptPres。

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

function Get-PptOleCount($slide) {
    $n = 0
    $shapes = $slide.Shapes
    try {
        for ($i = 1; $i -le $shapes.Count; $i++) {
            $sh = $shapes.Item($i)
            try {
                if ([int]$sh.Type -eq 7) { $n++ }
            } finally { Release-Com $sh }
        }
    } finally { Release-Com $shapes }
    return $n
}

function Get-PptZoomCount($slide) {
    $n = 0
    $shapes = $slide.Shapes
    try {
        for ($i = 1; $i -le $shapes.Count; $i++) {
            $sh = $shapes.Item($i)
            try {
                $name = [string]$sh.Name
                if (([int]$sh.Type -eq -2) -and ($name.StartsWith('Zoom'))) { $n++ }
            } finally { Release-Com $sh }
        }
    } finally { Release-Com $shapes }
    return $n
}

function Get-ShowSnap($pres) {
    $sss = $pres.SlideShowSettings
    $nss = $sss.NamedSlideShows
    try {
        $n = [int]$nss.Count
        $rows = @()
        for ($i = 1; $i -le $n; $i++) {
            $item = $nss.Item($i)
            try {
                $rows += [pscustomobject]@{
                    Name  = [string]$item.Name
                    Count = [int]$item.Count
                }
            }
            finally { Release-Com $item }
        }
        return @{
            Count = $n
            Rows = $rows
            ShowType = [int]$sss.ShowType
        }
    }
    finally {
        Release-Com $nss
        Release-Com $sss
    }
}

function Get-HandoutHeader($pres) {
    try {
        $hm = $pres.HandoutMaster
        $hfs = $hm.HeadersFooters
        $hdr = $hfs.Header
        try { return [string]$hdr.Text }
        finally {
            Release-Com $hdr
            Release-Com $hfs
            Release-Com $hm
        }
    } catch {
        Write-Log ("HandoutMaster 失败：" + $_.Exception.Message)
        return ''
    }
}

$path = Ensure-PptxCreateRound4 -Force
$ppt = Ensure-MsPpt (Ensure-PptxCreate)
Close-PptPresByPath $ppt $path

$tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'pptx'
    scope = @{
        oleObjects = $true
        customShows = $true
        handout = $true
        zooms = $true
        fields = $true
        connectors = $true
        notes = $true
        timing = $true
        slide = 1
    }
} 'pptx-cq-inspect'

$tree2 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $path.Replace('\', '/'); format = 'pptx'
    scope = @{ zooms = $true; slide = 2 }
} 'pptx-cq-inspect-slide2'

$slideXml = Get-ZipPartText $path 'ppt/slides/slide1.xml'
$notesXml = Get-ZipPartText $path 'ppt/notesSlides/notesSlide1.xml'

$pres = Open-PptPres $ppt $path
$cap = Get-PptCaption $pres
$s1 = Get-PptSlide $pres 1
$s2 = Get-PptSlide $pres 2
$oleN = Get-PptOleCount $s1
$zoomN = Get-PptZoomCount $s2
$snap = Get-ShowSnap $pres
$header = Get-HandoutHeader $pres
Release-Com $s1
Release-Com $s2
Close-PptPres $pres

$show0 = $null
if ($tree.customShows) { $show0 = @($tree.customShows) | Select-Object -First 1 }
$cq1ok = ($cap -notmatch 'Repair') `
    -and ([int]$tree.oleObjectCount -eq 1) `
    -and ([int]$oleN -ge 1) `
    -and ([int]$tree.customShowCount -eq 1) `
    -and ($null -ne $show0) `
    -and ([string]$show0.name -eq '预演') `
    -and ((@($show0.slides) -join ',') -eq 'slide:1,slide:2') `
    -and ([int]$snap.Count -eq 1) `
    -and ([string]$snap.Rows[0].Name -eq '预演') `
    -and ([int]$snap.Rows[0].Count -eq 2) `
    -and ([string]$tree.handout.header -eq '讲义头') `
    -and ($header -eq '讲义头')
Add-Check 'pptx-Cq1' $cq1ok "cap=$cap ole=$($tree.oleObjectCount) comOle=$oleN show=$($show0.name) inspectHdr=$($tree.handout.header) comHdr=$header"

$zoom = $null
if ($tree2.slides[1].zooms) { $zoom = @($tree2.slides[1].zooms) | Select-Object -First 1 }
$cq2ok = ($cap -notmatch 'Repair') `
    -and ([int]$tree2.slides[1].zoomCount -eq 1) `
    -and ($null -ne $zoom) `
    -and ([string]$zoom.type -eq 'slide') `
    -and ([string]$zoom.target -eq 'slide:1') `
    -and ($zoomN -ge 1)
Add-Check 'pptx-Cq2' $cq2ok "target=$($zoom.target) comZoom=$zoomN cap=$cap"

$fld = $null
if ($tree.slides[0].fields) { $fld = @($tree.slides[0].fields) | Select-Object -First 1 }
$cxn = $null
if ($tree.slides[0].connectors) { $cxn = @($tree.slides[0].connectors) | Select-Object -First 1 }
$startId = if ($cxn) { [string]$cxn.start.id } else { '' }
$endId = if ($cxn) { [string]$cxn.end.id } else { '' }
$cq3ok = ($cap -notmatch 'Repair') `
    -and ([int]$tree.slides[0].fieldCount -eq 1) `
    -and ($null -ne $fld) `
    -and ([string]$fld.type -eq 'slidenum') `
    -and ($slideXml -match '<a:custGeom>') `
    -and ($slideXml -match '<a:fld') `
    -and ($null -ne $cxn) `
    -and ($startId -like 'slide:1/shape:*') `
    -and ($endId -like 'slide:1/shape:*') `
    -and ($startId -ne $endId)
Add-Check 'pptx-Cq3' $cq3ok "fld=$($fld.type) start=$startId end=$endId geom=$($slideXml -match '<a:custGeom>')"

$notesAnim = $null
if ($tree.slides[0].notes.timing) {
    $notesAnim = @($tree.slides[0].notes.timing) | Select-Object -First 1
}
$cq4ok = ($cap -notmatch 'Repair') `
    -and ($notesXml -match '<p:timing>') `
    -and ([int]$tree.slides[0].notes.animCount -eq 1) `
    -and ($null -ne $notesAnim) `
    -and ([string]$notesAnim.type -eq 'fade') `
    -and ([string]$tree.showPr.type -eq 'kiosk') `
    -and ([int]$snap.ShowType -eq 3)
Add-Check 'pptx-Cq4' $cq4ok "notesTiming=$($notesXml -match '<p:timing>') type=$($notesAnim.type) ShowType=$($snap.ShowType)"

$failTiming = Get-OutPath 'pptx-create-round4-fail-timing.pptx'
$failFields = Get-OutPath 'pptx-create-round4-fail-fields.pptx'
$failHl = Get-OutPath 'pptx-create-round4-fail-hyperlinks.pptx'
$failXlsx = Get-OutPath 'pptx-create-round4-fail-xlsx.xlsx'
$hashPath = Get-OutPath 'pptx-create-round4-hash.pptx'
foreach ($p in @($failTiming, $failFields, $failHl, $failXlsx, $hashPath)) {
    if (Test-Path $p) { Remove-Item -Force $p }
}

$rawTiming = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'create'
    path = $failTiming.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(@{ shapes = @(@{ text = 'a' }); timing = @() })
    }
} 'pptx-cq-fail-timing'
$rawFields = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'create'
    path = $failFields.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(@{ shapes = @(@{ text = 'a' }); fields = @() })
    }
} 'pptx-cq-fail-fields'
$rawHl = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'create'
    path = $failHl.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(@{ shapes = @(@{ text = 'a' }); hyperlinks = @() })
    }
} 'pptx-cq-fail-hyperlinks'
$rawXlsx = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'create'
    path = $failXlsx.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        customShows = @(@{ name = 'A'; slides = @(1) })
        sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) })
    }
} 'pptx-cq-fail-xlsx'

Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hashPath.Replace('\', '/'); format = 'pptx'
    presentation = @{
        modifyProtection = @{ enforced = $true; password = 'secret' }
        slides = @(@{ shapes = @(@{ text = '甲' }) })
    }
} 'pptx-cq-hash' | Out-Null
$hashXml = Get-ZipPartText $hashPath 'ppt/presentation.xml'
$hashTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hashPath.Replace('\', '/'); format = 'pptx'
} 'pptx-cq-hash-inspect'

$cq5ok = ((Get-PptErrorCode $rawTiming) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $rawFields) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $rawHl) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $rawXlsx) -eq 'InvalidField') `
    -and (-not (Test-Path $failTiming)) `
    -and (-not (Test-Path $failFields)) `
    -and (-not (Test-Path $failHl)) `
    -and (-not (Test-Path $failXlsx)) `
    -and ([bool]$hashTree.modifyProtection.hasPassword -eq $true) `
    -and ($hashXml -match 'modifyVerifier') `
    -and ($hashXml -match 'hashData=')
Add-Check 'pptx-Cq5' $cq5ok "t=$(Get-PptErrorCode $rawTiming) f=$(Get-PptErrorCode $rawFields) hl=$(Get-PptErrorCode $rawHl) x=$(Get-PptErrorCode $rawXlsx) hash=$($hashTree.modifyProtection.hasPassword)"
