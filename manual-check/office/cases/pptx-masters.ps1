# pptx 母版与版式：docs/checklist-pptx.md 的 Mm1–Mm5。
# create 产物验计数；PowerPoint 默认文稿验版式名、改占位符与换版式。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings

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

function Get-MasterLayouts($tree) {
    $list = @()
    try { $list = @($tree.masters[0].layouts) } catch { $list = @() }
    return $list
}

function Find-TitlePlaceholder($tree) {
    foreach ($ly in @(Get-MasterLayouts $tree)) {
        foreach ($ph in @($ly.placeholders)) {
            if ([string]$ph.type -eq 'title') { return $ph }
        }
    }
    foreach ($ly in @(Get-MasterLayouts $tree)) {
        $phs = @($ly.placeholders)
        if ($phs.Count -ge 1) { return $phs[0] }
    }
    return $null
}

function Find-BlankLayout($tree) {
    foreach ($ly in @(Get-MasterLayouts $tree)) {
        $kind = [string]$ly.kind
        $name = [string]$ly.name
        if ($kind -eq 'blank' -or $name -match '空白|Blank') { return $ly }
    }
    return $null
}

function Ensure-PptxPptDefaultSource {
    $path = Get-OutPath 'pptx-ppt-default-source.pptx'
    if (Test-Path $path) {
        $cached = Invoke-OoxpTree @{
            schemaVersion = 1; action = 'inspect'
            path = $path.Replace('\', '/'); format = 'pptx'
            scope = @{ masters = $true }
        } 'pptx-mm-source-cache-check'
        $n = 0
        try { $n = @($cached.masters[0].layouts).Count } catch {}
        $hasName = $false
        try { $hasName = [string]$cached.masters[0].layouts[0].name -ne '' } catch {}
        $slideN = 0
        try { $slideN = [int]$cached.slideCount } catch {}
        if (($n -ge 1) -and $hasName -and ($slideN -ge 1)) { return $path }
        Write-Log "缓存的默认文稿不合格（layouts=$n name=$hasName slides=$slideN），重建"
    }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $tmp = Join-Path $tmpDir 'pptx-ppt-default-source.pptx'
    if (Test-Path $tmp) { Remove-Item -Force $tmp }

    $ppt = Ensure-MsPpt (Ensure-PptxCreate)
    Close-PptPresByPath $ppt $tmp
    $col = $ppt.Presentations
    try {
        $pres = $col.Add()
        # 本机 Presentations.Add 可能写出没有 sldIdLst 的空文稿。
        # Mm3 要往 slide:1 插用户框，至少留一页。1 = ppLayoutTitle。
        $slides = $pres.Slides
        try {
            if ([int]$slides.Count -lt 1) {
                $sld = $slides.Add(1, 1)
                Release-Com $sld
            }
        }
        finally { Release-Com $slides }
        # 24 = ppSaveAsOpenXMLPresentation
        Write-Log 'PowerPoint Presentations.Add 后 SaveAs 写出默认文稿'
        $pres.SaveAs($tmp, 24)
        Close-PptPres $pres
    }
    finally { Release-Com $col }
    Copy-Item -Force $tmp $path
    Write-Log "saved ppt default source $path"
    return $path
}

$hostPath = Ensure-PptxCreate
$src = Ensure-PptxPptDefaultSource
$ppt = Ensure-MsPpt $hostPath
Close-PptPresByPath $ppt $src

# Mm1：无 masters 时有计数、没有名单；create 为 1 且版式名「空白」；另存默认文稿能列出版式名。
$treeCreateBare = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
} 'pptx-mm-inspect-create-bare'
$treeCreate = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-mm-inspect-create'
$treeSrcBare = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
} 'pptx-mm-inspect-src-bare'
$treeSrc = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ masters = $true }
} 'pptx-mm-inspect-src'
$createCount = [int]$treeCreateBare.masterCount
$createHasList = $null -ne $treeCreateBare.PSObject.Properties['masters']
$createLayoutName = $null
try { $createLayoutName = [string]$treeCreate.masters[0].layouts[0].name } catch {}
$srcBareCount = [int]$treeSrcBare.masterCount
$srcBareHasList = $null -ne $treeSrcBare.PSObject.Properties['masters']
$srcNames = @(Get-MasterLayouts $treeSrc | ForEach-Object { [string]$_.name } | Where-Object { $_ })
$mm1ok = ($createCount -eq 1) `
    -and (-not $createHasList) `
    -and ($createLayoutName -eq '空白') `
    -and ($srcBareCount -ge 1) `
    -and (-not $srcBareHasList) `
    -and ($srcNames.Count -ge 1)
Add-Check 'pptx-Mm1' $mm1ok "create=$createCount name=$createLayoutName srcBare=$srcBareCount srcNames=$($srcNames -join ',')"

# Mm2：对版式标题占位符改字，无 Repair。
$ph = Find-TitlePlaceholder $treeSrc
$textOut = Get-OutPath 'pptx-masters-text.pptx'
Close-PptPresByPath $ppt $textOut
if (Test-Path $textOut) { Remove-Item -Force $textOut }
$mm2ok = $false
if ($null -eq $ph) {
    Add-Check 'pptx-Mm2' $false '没有标题占位符'
} else {
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $src.Replace('\', '/')
        output = $textOut.Replace('\', '/')
        ops = @(@{ op = 'setShapeText'; id = [string]$ph.id; text = $S.shape2new })
    } 'pptx-mm-text' | Out-Null
    $treeT = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $textOut.Replace('\', '/'); format = 'pptx'
        scope = @{ masters = $true }
    } 'pptx-mm-text-inspect'
    $presT = Open-PptPres $ppt $textOut
    $capT = Get-PptCaption $presT
    Close-PptPres $presT
    $got = $null
    foreach ($ly in @(Get-MasterLayouts $treeT)) {
        foreach ($p in @($ly.placeholders)) {
            if ([string]$p.id -eq [string]$ph.id) { $got = [string]$p.text }
        }
    }
    $mm2ok = ($capT -notmatch 'Repair') -and ($got -eq $S.shape2new)
    Add-Check 'pptx-Mm2' $mm2ok "cap=$capT id=$($ph.id) text=$got"
}

# Mm3：插用户框后换成空白版式，用户框还在。
$insertOut = Get-OutPath 'pptx-masters-insert.pptx'
Close-PptPresByPath $ppt $insertOut
if (Test-Path $insertOut) { Remove-Item -Force $insertOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $insertOut.Replace('\', '/')
    ops = @(@{
        op = 'insertShape'; id = 'slide:1'; text = '用户框'
        x = 914400; y = 1371600; cx = 1828800; cy = 914400
    })
} 'pptx-mm-insert' | Out-Null
$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insertOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1; masters = $true }
} 'pptx-mm-insert-inspect'
$blank = Find-BlankLayout $treeI
$layoutOut = Get-OutPath 'pptx-masters-layout.pptx'
Close-PptPresByPath $ppt $layoutOut
if (Test-Path $layoutOut) { Remove-Item -Force $layoutOut }
$mm3ok = $false
if ($null -eq $blank) {
    Add-Check 'pptx-Mm3' $false '没有空白版式'
} else {
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $insertOut.Replace('\', '/')
        output = $layoutOut.Replace('\', '/')
        ops = @(@{ op = 'setSlideLayout'; id = 'slide:1'; layout = [string]$blank.id })
    } 'pptx-mm-layout' | Out-Null
    $treeL = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $layoutOut.Replace('\', '/'); format = 'pptx'
        scope = @{ slide = 1 }
    } 'pptx-mm-layout-inspect'
    $user = @($treeL.slides[0].shapes) | Where-Object { [string]$_.text -eq '用户框' } | Select-Object -First 1
    $presL = Open-PptPres $ppt $layoutOut
    $capL = Get-PptCaption $presL
    Close-PptPres $presL
    $mm3ok = ($null -ne $user) `
        -and ([string]$treeL.slides[0].layoutId -eq [string]$blank.id) `
        -and ($capL -notmatch 'Repair')
    Add-Check 'pptx-Mm3' $mm3ok "cap=$capL layout=$($treeL.slides[0].layoutId) user=$($user.id)"
}

# Mm4：改占位符几何。
$posOut = Get-OutPath 'pptx-masters-pos.pptx'
Close-PptPresByPath $ppt $posOut
if (Test-Path $posOut) { Remove-Item -Force $posOut }
$mm4ok = $false
if ($null -eq $ph) {
    Add-Check 'pptx-Mm4' $false '没有占位符'
} else {
    $oldX = $null
    try { $oldX = [int64]$ph.x } catch {}
    $newX = 200000
    if ($oldX -eq $newX) { $newX = 300000 }
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $src.Replace('\', '/')
        output = $posOut.Replace('\', '/')
        ops = @(@{ op = 'setShapePos'; id = [string]$ph.id; x = $newX; y = 200000 })
    } 'pptx-mm-pos' | Out-Null
    $treeP = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $posOut.Replace('\', '/'); format = 'pptx'
        scope = @{ masters = $true }
    } 'pptx-mm-pos-inspect'
    $gotX = $null
    foreach ($ly in @(Get-MasterLayouts $treeP)) {
        foreach ($p in @($ly.placeholders)) {
            if ([string]$p.id -eq [string]$ph.id) { $gotX = [int64]$p.x }
        }
    }
    $presP = Open-PptPres $ppt $posOut
    $capP = Get-PptCaption $presP
    Close-PptPres $presP
    $mm4ok = ($capP -notmatch 'Repair') -and ($gotX -eq $newX)
    Add-Check 'pptx-Mm4' $mm4ok "cap=$capP oldX=$oldX newX=$newX gotX=$gotX"
}

# Mm5：xlsx / docx 拒 scope.masters 与 setSlideLayout。
$xlsx = Ensure-XlsxCreate
$docx = Ensure-DocxCreate
$failX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'inspect'
    path = $xlsx.Replace('\', '/'); format = 'xlsx'
    scope = @{ masters = $true }
} 'pptx-mm-fail-xlsx'
$failD = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'inspect'
    path = $docx.Replace('\', '/'); format = 'docx'
    scope = @{ masters = $true }
} 'pptx-mm-fail-docx'
$badX = Get-OutPath 'pptx-masters-fail-xlsx.xlsx'
$badD = Get-OutPath 'pptx-masters-fail-docx.docx'
if (Test-Path $badX) { Remove-Item -Force $badX }
if (Test-Path $badD) { Remove-Item -Force $badD }
$failXop = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $badX.Replace('\', '/')
    ops = @(@{ op = 'setSlideLayout'; id = 'slide:1'; layout = 'master:1/layout:1' })
} 'pptx-mm-fail-xlsx-op'
$failDop = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $docx.Replace('\', '/')
    output = $badD.Replace('\', '/')
    ops = @(@{ op = 'setSlideLayout'; id = 'slide:1'; layout = 'master:1/layout:1' })
} 'pptx-mm-fail-docx-op'
$mm5ok = ((Get-PptErrorCode $failX) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failD) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failXop) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failDop) -eq 'InvalidField') `
    -and (-not (Test-Path $badX)) `
    -and (-not (Test-Path $badD))
Add-Check 'pptx-Mm5' $mm5ok "x=$(Get-PptErrorCode $failX)/$(Get-PptErrorCode $failXop) d=$(Get-PptErrorCode $failD)/$(Get-PptErrorCode $failDop)"
