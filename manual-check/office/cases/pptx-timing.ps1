# pptx 动画清单：docs/checklist-pptx.md 的 Am1–Am5。
# 源文件：PowerPoint 亲手另存，第一页进入淡入。不要用库 create 产物当带动画的唯一样例。

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

function Test-PptxAnimSource([string]$pptx, [string]$reqName) {
    $tree = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $pptx.Replace('\', '/'); format = 'pptx'
        scope = @{ timing = $true; slide = 1 }
    } $reqName
    $n = [int]$tree.animCount
    $list = @()
    try { $list = @($tree.slides[0].timing) } catch {}
    $target = $null
    $trigger = $null
    if ($list.Count -ge 1) {
        try { $target = [string]$list[0].target } catch {}
        try { $trigger = [string]$list[0].trigger } catch {}
    }
    $shapeId = $null
    try { $shapeId = [string]$tree.slides[0].shapes[0].id } catch {}
    return [pscustomobject]@{
        animCount = $n
        target = $target
        trigger = $trigger
        shapeId = $shapeId
        ok = ($n -ge 1) -and ($target -eq $shapeId) -and ($trigger -eq 'click')
    }
}

function Add-PptFadeIn($slide) {
    $box = Get-PptShape $slide 1
    $tl = $slide.TimeLine
    $seq = $tl.MainSequence
    try {
        # msoAnimEffectFade = 10；msoAnimateLevelNone = 0；msoAnimTriggerOnPageClick = 1
        $eff = $seq.AddEffect($box, 10, 0, 1)
        Release-Com $eff
        return [int]$seq.Count
    }
    finally {
        Release-Com $seq
        Release-Com $tl
        Release-Com $box
    }
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

function Ensure-PptxPptAnimSource {
    $path = Get-OutPath 'pptx-ppt-anim-source.pptx'
    if (Test-Path $path) {
        $cached = Test-PptxAnimSource $path 'pptx-am-source-cache-check'
        if ($cached.ok) { return $path }
        Write-Log "缓存的 pptx anim source 不合格（count=$($cached.animCount) target=$($cached.target) shape=$($cached.shapeId)），重建"
    }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $tmp = Join-Path $tmpDir 'pptx-ppt-anim-source.pptx'
    $hostPath = Ensure-PptxCreate
    Copy-Item -Force $hostPath $tmp

    $ppt = Ensure-MsPpt $hostPath
    Close-PptPresByPath $ppt $tmp
    $pres = Open-PptPres $ppt $tmp
    $s1 = Get-PptSlide $pres 1
    $n = Add-PptFadeIn $s1
    Write-Log "PowerPoint Save() 原地覆盖 TEMP 里的进入动画源文件，MainSequence=$n"
    $pres.Save()
    Release-Com $s1
    Close-PptPres $pres
    Copy-Item -Force $tmp $path
    Write-Log "saved ppt anim source $path"

    $check = Test-PptxAnimSource $path 'pptx-am-source-saved-inspect'
    if (-not $check.ok) {
        throw "PowerPoint 另存的进入动画源不合格：count=$($check.animCount) target=$($check.target) shape=$($check.shapeId) trigger=$($check.trigger)"
    }
    return $path
}

$hostPath = Ensure-PptxCreate
$src = Ensure-PptxPptAnimSource
$ppt = Ensure-MsPpt $hostPath
Close-PptPresByPath $ppt $src

# Am1：无 timing 时有计数、没有名单；create 为 0；另存进入动画 target 对上第一框。
$treeCreate = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
} 'pptx-am-inspect-create'
$treeSrcBare = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
} 'pptx-am-inspect-src-bare'
$treeSrc = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ timing = $true; slide = 1 }
} 'pptx-am-inspect-src'
$createCount = [int]$treeCreate.animCount
$createHasList = $null -ne $treeCreate.slides[0].PSObject.Properties['timing']
$bareCount = [int]$treeSrcBare.animCount
$bareHasList = $null -ne $treeSrcBare.slides[0].PSObject.Properties['timing']
$srcList = @($treeSrc.slides[0].timing)
$srcTarget = [string]$srcList[0].target
$srcShape = [string]$treeSrc.slides[0].shapes[0].id
$am1ok = ($createCount -eq 0) `
    -and (-not $createHasList) `
    -and ($bareCount -ge 1) `
    -and (-not $bareHasList) `
    -and ($srcTarget -eq $srcShape)
Add-Check 'pptx-Am1' $am1ok "create=$createCount srcBare=$bareCount target=$srcTarget shape=$srcShape"

# Am2：COM 条数与 animCount 一致，trigger 为 click，无 Repair。
$presSrc = Open-PptPres $ppt $src
$capSrc = Get-PptCaption $presSrc
$sSrc = Get-PptSlide $presSrc 1
$comCount = Get-PptMainSeqCount $sSrc
Release-Com $sSrc
Close-PptPres $presSrc
$srcTrigger = [string]$srcList[0].trigger
$am2ok = ($capSrc -notmatch 'Repair') `
    -and ($comCount -eq [int]$treeSrc.animCount) `
    -and ($srcTrigger -eq 'click')
Add-Check 'pptx-Am2' $am2ok "cap=$capSrc com=$comCount anim=$($treeSrc.animCount) trigger=$srcTrigger"

# Am3：另存样例只改字，名单还在。
$textOut = Get-OutPath 'pptx-timing-text.pptx'
Close-PptPresByPath $ppt $textOut
if (Test-Path $textOut) { Remove-Item -Force $textOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $textOut.Replace('\', '/')
    ops = @(@{ op = 'setShapeText'; id = $srcShape; text = $S.shape2new })
} 'pptx-am-text' | Out-Null
$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $textOut.Replace('\', '/'); format = 'pptx'
    scope = @{ timing = $true; slide = 1 }
} 'pptx-am-text-inspect'
$presT = Open-PptPres $ppt $textOut
$capT = Get-PptCaption $presT
Close-PptPres $presT
$listT = @($treeT.slides[0].timing)
$am3ok = ($capT -notmatch 'Repair') `
    -and ([int]$treeT.animCount -ge 1) `
    -and ([string]$listT[0].target -eq $srcShape) `
    -and ([string]$treeT.slides[0].shapes[0].text -eq $S.shape2new)
Add-Check 'pptx-Am3' $am3ok "cap=$capT count=$($treeT.animCount) text=$($treeT.slides[0].shapes[0].text)"

# Am4：setTiming 得 UnsupportedOp，未写文件。
$badOp = Get-OutPath 'pptx-timing-fail-op.pptx'
Close-PptPresByPath $ppt $badOp
if (Test-Path $badOp) { Remove-Item -Force $badOp }
$failOp = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badOp.Replace('\', '/')
    ops = @(@{ op = 'setTiming'; id = 'slide:1' })
} 'pptx-am-fail-op'
$am4ok = ((Get-PptErrorCode $failOp) -eq 'UnsupportedOp') -and (-not (Test-Path $badOp))
Add-Check 'pptx-Am4' $am4ok "code=$(Get-PptErrorCode $failOp) exists=$(Test-Path $badOp)"

# Am5：xlsx / docx 的 scope.timing 得 InvalidField。
$xlsx = Ensure-XlsxCreate
$docx = Ensure-DocxCreate
$failX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'inspect'
    path = $xlsx.Replace('\', '/'); format = 'xlsx'
    scope = @{ timing = $true }
} 'pptx-am-fail-xlsx'
$failD = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'inspect'
    path = $docx.Replace('\', '/'); format = 'docx'
    scope = @{ timing = $true }
} 'pptx-am-fail-docx'
$am5ok = ((Get-PptErrorCode $failX) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failD) -eq 'InvalidField')
Add-Check 'pptx-Am5' $am5ok "xlsx=$(Get-PptErrorCode $failX) docx=$(Get-PptErrorCode $failD)"
