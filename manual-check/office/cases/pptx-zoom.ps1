# pptx Zoom：docs/checklist-pptx.md 的 Pz1–Pz5。
# 源文件由 create 三页再 ooxp insertZoom。不要 ExecuteMso，不要 COM AddZoom。
# 不算放映引擎，不写 timing。

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

$tmpDir = Join-Path $env:TEMP 'ooxp-office'
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
$src3 = Join-Path $tmpDir 'pptx-zoom-src.pptx'
$req = @{
    schemaVersion = 1; action = 'create'
    path = $src3.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(
            @{ shapes = @(@{ text = '页1' }) }
            @{ shapes = @(@{ text = '页2' }) }
            @{ shapes = @(@{ text = '页3' }) }
        )
    }
}
Invoke-OoxpRequest $req 'pptx-zoom-src' | Out-Null

# Pz1：无名单时只有计数；insert 后 id 为 slide:…/zoom:…，不并进 pictures。
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src3.Replace('\', '/'); format = 'pptx'
} 'pptx-zoom-inspect-empty'
$slide0 = $tree0.slides[0]
$noList = -not ($slide0.PSObject.Properties.Name -contains 'zooms')
Add-Check 'pptx-Pz1a' (([int]$tree0.zoomCount -eq 0) -and ([int]$slide0.zoomCount -eq 0) -and $noList) "wb=$($tree0.zoomCount) slide=$($slide0.zoomCount) noList=$noList"

$insOut = Get-OutPath 'pptx-zoom-insert.pptx'
$hostPath = Ensure-PptxCreate
$ppt = Ensure-MsPpt $hostPath
Close-PptPresByPath $ppt $insOut
if (Test-Path $insOut) { Remove-Item -Force $insOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src3.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{ op = 'insertZoom'; id = 'slide:1'; type = 'slide'; target = 'slide:2' })
} 'pptx-zoom-insert' | Out-Null
$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'pptx'
    scope = @{ zooms = $true; pictures = $true }
} 'pptx-zoom-inspect-expand'
$zoomNode = $null
if ($tree1.slides[0].zooms) { $zoomNode = @($tree1.slides[0].zooms) | Select-Object -First 1 }
$picN = 0
if ($tree1.slides[0].pictures) { $picN = @($tree1.slides[0].pictures).Count }
$pz1ok = ($null -ne $zoomNode) `
    -and ([string]$zoomNode.id -like 'slide:1/zoom:*') `
    -and ([string]$zoomNode.type -eq 'slide') `
    -and ([string]$zoomNode.target -eq 'slide:2') `
    -and ([int]$tree1.pictureCount -eq 0) `
    -and ($picN -eq 0)
Add-Check 'pptx-Pz1' $pz1ok "id=$($zoomNode.id) type=$($zoomNode.type) target=$($zoomNode.target) pics=$($tree1.pictureCount)"

# Pz2：create 三页产物 insert 后无 Repair；COM Type=-2 且 Name 以 Zoom 开头。
$presI = Open-PptPres $ppt $insOut
$capI = Get-PptCaption $presI
$sI = Get-PptSlide $presI 1
$comI = Get-PptZoomCount $sI
Release-Com $sI
Close-PptPres $presI
$pz2ok = ($capI -notmatch 'Repair') -and ($comI -ge 1)
Add-Check 'pptx-Pz2' $pz2ok "cap=$capI com=$comI"

# Pz3：插入后 pictureCount 仍为 0。
$pz3ok = ([int]$tree1.pictureCount -eq 0) -and ($picN -eq 0) -and ([int]$tree1.slides[0].pictureCount -eq 0)
Add-Check 'pptx-Pz3' $pz3ok "pictureCount=$($tree1.pictureCount) picList=$picN slide=$($tree1.slides[0].pictureCount)"

# Pz4：delete 后计数 0、COM 不再有 Zoom；无 Repair。
$zoomId = [string]$zoomNode.id
$delOut = Get-OutPath 'pptx-zoom-delete.pptx'
Close-PptPresByPath $ppt $delOut
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteZoom'; id = $zoomId })
} 'pptx-zoom-delete' | Out-Null
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'pptx'
    scope = @{ zooms = $true }
} 'pptx-zoom-delete-inspect'
$presD = Open-PptPres $ppt $delOut
$capD = Get-PptCaption $presD
$sD = Get-PptSlide $presD 1
$comD = Get-PptZoomCount $sD
Release-Com $sD
Close-PptPres $presD
$pz4ok = ([int]$treeD.zoomCount -eq 0) -and ($comD -eq 0) -and ($capD -notmatch 'Repair')
Add-Check 'pptx-Pz4' $pz4ok "count=$($treeD.zoomCount) com=$comD cap=$capD"

# Pz5：错 id / summary / 目标是自己 / xlsx 拒写。
$failDel = Get-OutPath 'pptx-zoom-bad-del.pptx'
$failIns = Get-OutPath 'pptx-zoom-bad-ins.pptx'
$failSum = Get-OutPath 'pptx-zoom-bad-summary.pptx'
$failSelf = Get-OutPath 'pptx-zoom-bad-self.pptx'
$failX = Get-OutPath 'pptx-zoom-fail-xlsx.xlsx'
if (Test-Path $failDel) { Remove-Item -Force $failDel }
if (Test-Path $failIns) { Remove-Item -Force $failIns }
if (Test-Path $failSum) { Remove-Item -Force $failSum }
if (Test-Path $failSelf) { Remove-Item -Force $failSelf }
if (Test-Path $failX) { Remove-Item -Force $failX }
$failDelR = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src3.Replace('\', '/')
    output = $failDel.Replace('\', '/')
    ops = @(@{ op = 'deleteZoom'; id = 'slide:1' })
} 'pptx-zoom-bad-del'
$failInsR = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src3.Replace('\', '/')
    output = $failIns.Replace('\', '/')
    ops = @(@{ op = 'insertZoom'; id = 'slide:1/zoom:3'; type = 'slide'; target = 'slide:2' })
} 'pptx-zoom-bad-ins'
$failSumR = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src3.Replace('\', '/')
    output = $failSum.Replace('\', '/')
    ops = @(@{ op = 'insertZoom'; id = 'slide:1'; type = 'summary'; target = 'slide:2' })
} 'pptx-zoom-bad-summary'
$failSelfR = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src3.Replace('\', '/')
    output = $failSelf.Replace('\', '/')
    ops = @(@{ op = 'insertZoom'; id = 'slide:1'; type = 'slide'; target = 'slide:1' })
} 'pptx-zoom-bad-self'
$xlsx = Ensure-XlsxCreate
$failXR = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'insertZoom'; id = 'slide:1'; type = 'slide'; target = 'slide:2' })
} 'pptx-zoom-fail-xlsx'
$pz5ok = ((Get-PptErrorCode $failDelR) -eq 'InvalidId') `
    -and ((Get-PptErrorCode $failInsR) -eq 'InvalidId') `
    -and ((Get-PptErrorCode $failSumR) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failSelfR) -eq 'InvalidField') `
    -and ((Get-PptErrorCode $failXR) -eq 'InvalidField') `
    -and (-not (Test-Path $failDel)) `
    -and (-not (Test-Path $failIns)) `
    -and (-not (Test-Path $failX))
Add-Check 'pptx-Pz5' $pz5ok "del=$(Get-PptErrorCode $failDelR) ins=$(Get-PptErrorCode $failInsR) sum=$(Get-PptErrorCode $failSumR) self=$(Get-PptErrorCode $failSelfR) xlsx=$(Get-PptErrorCode $failXR)"
