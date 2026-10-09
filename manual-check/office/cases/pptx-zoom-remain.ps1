# pptx Zoom 剩余：docs/checklist-pptx.md 的 Pzr1–Pzr5。
# 源文件由 create 再 ooxp insertZoom / setZoom。不要 ExecuteMso。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }
Add-Type -AssemblyName System.IO.Compression.FileSystem

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

function Get-ZipEntryText([string]$pptx, [string]$name) {
    $zip = [IO.Compression.ZipFile]::OpenRead($pptx)
    try {
        $e = $zip.GetEntry($name)
        if (-not $e) { return '' }
        $sr = New-Object IO.StreamReader($e.Open())
        try { return $sr.ReadToEnd() } finally { $sr.Dispose() }
    } finally { $zip.Dispose() }
}

function Get-ZipFirstPngBytes([string]$pptx) {
    $zip = [IO.Compression.ZipFile]::OpenRead($pptx)
    try {
        foreach ($e in $zip.Entries) {
            $n = [string]$e.FullName
            if ($n.StartsWith('ppt/media/') -and $n.EndsWith('.png')) {
                $ms = New-Object IO.MemoryStream
                $s = $e.Open()
                try {
                    $s.CopyTo($ms)
                    return $ms.ToArray()
                } finally {
                    $s.Dispose()
                    $ms.Dispose()
                }
            }
        }
    } finally { $zip.Dispose() }
    return $null
}

$tmpDir = Join-Path $env:TEMP 'ooxp-office'
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
$src = Join-Path $tmpDir 'pptx-zoom-remain-src.pptx'
$req = @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'pptx'
    presentation = @{
        sections = @(
            @{ name = '开场'; start = 1 }
            @{ name = '内容'; start = 2 }
        )
        slides = @(
            @{ shapes = @(@{ text = '页1' }) }
            @{ shapes = @(@{ text = '页2' }) }
            @{ shapes = @(@{ text = '页3' }) }
        )
    }
}
Invoke-OoxpRequest $req 'pptx-zoom-remain-src' | Out-Null

$hostPath = Ensure-PptxCreate
$ppt = Ensure-MsPpt $hostPath

# Pzr1：摘要缩放 inspect 与 ZIP。
$sumOut = Get-OutPath 'pptx-zoom-remain-summary.pptx'
Close-PptPresByPath $ppt $sumOut
if (Test-Path $sumOut) { Remove-Item -Force $sumOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $sumOut.Replace('\', '/')
    ops = @(@{
        op = 'insertZoom'; id = 'slide:1'; type = 'summary'
        targets = @('section:1', 'section:2')
    })
} 'pptx-zoom-remain-summary' | Out-Null
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sumOut.Replace('\', '/'); format = 'pptx'
    scope = @{ zooms = $true; pictures = $true }
} 'pptx-zoom-remain-summary-inspect'
$zoomS = $null
if ($treeS.slides[0].zooms) { $zoomS = @($treeS.slides[0].zooms) | Select-Object -First 1 }
$tg = @()
if ($zoomS -and $zoomS.targets) { $tg = @($zoomS.targets) }
$xmlS = Get-ZipEntryText $sumOut 'ppt/slides/slide1.xml'
$pzr1ok = ($null -ne $zoomS) `
    -and ([string]$zoomS.type -eq 'summary') `
    -and ($tg.Count -eq 2) `
    -and ([string]$tg[0] -eq 'section:1') `
    -and ([string]$tg[1] -eq 'section:2') `
    -and ($xmlS -match 'summaryZm') `
    -and ($xmlS -match 'Requires="psuz"')
Add-Check 'pptx-Pzr1' $pzr1ok "type=$($zoomS.type) targets=$($tg -join ',') xml=$($xmlS -match 'summaryZm')"

# Pzr2：打开无 Repair，COM Type=-2。
$presS = Open-PptPres $ppt $sumOut
$capS = Get-PptCaption $presS
$sS = Get-PptSlide $presS 1
$comS = Get-PptZoomCount $sS
Release-Com $sS
Close-PptPres $presS
$pzr2ok = ($capS -notmatch 'Repair') -and ($comS -ge 1)
Add-Check 'pptx-Pzr2' $pzr2ok "cap=$capS com=$comS"

# Pzr3：幻灯片缩放改目标。
$slideSrc = Get-OutPath 'pptx-zoom-remain-slide.pptx'
Close-PptPresByPath $ppt $slideSrc
if (Test-Path $slideSrc) { Remove-Item -Force $slideSrc }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $slideSrc.Replace('\', '/')
    ops = @(@{ op = 'insertZoom'; id = 'slide:1'; type = 'slide'; target = 'slide:2' })
} 'pptx-zoom-remain-slide' | Out-Null
$treeZ = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $slideSrc.Replace('\', '/'); format = 'pptx'
    scope = @{ zooms = $true }
} 'pptx-zoom-remain-slide-inspect'
$zoomId = [string]$treeZ.slides[0].zooms[0].id
$setOut = Get-OutPath 'pptx-zoom-remain-set.pptx'
Close-PptPresByPath $ppt $setOut
if (Test-Path $setOut) { Remove-Item -Force $setOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $slideSrc.Replace('\', '/')
    output = $setOut.Replace('\', '/')
    ops = @(@{ op = 'setZoom'; id = $zoomId; target = 'slide:3' })
} 'pptx-zoom-remain-set' | Out-Null
$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $setOut.Replace('\', '/'); format = 'pptx'
    scope = @{ zooms = $true }
} 'pptx-zoom-remain-set-inspect'
$presT = Open-PptPres $ppt $setOut
$capT = Get-PptCaption $presT
Close-PptPres $presT
$pzr3ok = ([string]$treeT.slides[0].zooms[0].target -eq 'slide:3') -and ($capT -notmatch 'Repair')
Add-Check 'pptx-Pzr3' $pzr3ok "target=$($treeT.slides[0].zooms[0].target) cap=$capT"

# Pzr4：换预览。
$png = Ensure-OoxpPngA
$coverOut = Get-OutPath 'pptx-zoom-remain-cover.pptx'
Close-PptPresByPath $ppt $coverOut
if (Test-Path $coverOut) { Remove-Item -Force $coverOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $setOut.Replace('\', '/')
    output = $coverOut.Replace('\', '/')
    ops = @(@{ op = 'setZoom'; id = $zoomId; file = $png.Replace('\', '/') })
} 'pptx-zoom-remain-cover' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $coverOut.Replace('\', '/'); format = 'pptx'
    scope = @{ zooms = $true; pictures = $true }
} 'pptx-zoom-remain-cover-inspect'
$got = Get-ZipFirstPngBytes $coverOut
$want = [IO.File]::ReadAllBytes($png)
$same = $false
if ($got -and $want) {
    $same = ($got.Length -eq $want.Length)
    if ($same) {
        for ($i = 0; $i -lt $got.Length; $i++) {
            if ($got[$i] -ne $want[$i]) { $same = $false; break }
        }
    }
}
$presC = Open-PptPres $ppt $coverOut
$capCover = Get-PptCaption $presC
Close-PptPres $presC
$pzr4ok = $same -and ([int]$treeC.pictureCount -eq 0) -and ($capCover -notmatch 'Repair')
Add-Check 'pptx-Pzr4' $pzr4ok "same=$same pics=$($treeC.pictureCount) cap=$capCover"

# Pzr5：拒错。
$failEmpty = Get-OutPath 'pptx-zoom-remain-empty.pptx'
$failTargets = Get-OutPath 'pptx-zoom-remain-bad-targets.pptx'
$failId = Get-OutPath 'pptx-zoom-remain-bad-id.pptx'
$failX = Get-OutPath 'pptx-zoom-remain-fail-xlsx.xlsx'
foreach ($p in @($failEmpty, $failTargets, $failId, $failX)) {
    if (Test-Path $p) { Remove-Item -Force $p }
}
$emptyR = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $slideSrc.Replace('\', '/')
    output = $failEmpty.Replace('\', '/')
    ops = @(@{ op = 'setZoom'; id = $zoomId })
} 'pptx-zoom-remain-empty'
$tgtR = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $slideSrc.Replace('\', '/')
    output = $failTargets.Replace('\', '/')
    ops = @(@{ op = 'setZoom'; id = $zoomId; targets = @('section:1') })
} 'pptx-zoom-remain-bad-targets'
$idR = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failId.Replace('\', '/')
    ops = @(@{ op = 'setZoom'; id = 'slide:1'; target = 'slide:2' })
} 'pptx-zoom-remain-bad-id'
$xlsx = Ensure-XlsxCreate
$xR = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'setZoom'; id = 'slide:1/zoom:3'; target = 'slide:2' })
} 'pptx-zoom-remain-fail-xlsx'
$pzr5ok = ((Get-PptErrorCode $emptyR) -eq 'InvalidField') -and (-not (Test-Path $failEmpty)) `
    -and ((Get-PptErrorCode $tgtR) -eq 'InvalidField') -and (-not (Test-Path $failTargets)) `
    -and ((Get-PptErrorCode $idR) -eq 'InvalidId') -and (-not (Test-Path $failId)) `
    -and ((Get-PptErrorCode $xR) -eq 'InvalidField') -and (-not (Test-Path $failX))
Add-Check 'pptx-Pzr5' $pzr5ok "empty=$(Get-PptErrorCode $emptyR) targets=$(Get-PptErrorCode $tgtR) id=$(Get-PptErrorCode $idR) xlsx=$(Get-PptErrorCode $xR)"
