# docx 3D 事后几何：docs/checklist-docx.md 的 D3g1–D3g6。
# 编号 D3g，不要和 D3d、Dh 撞。探测脚本不进本文件。不要 Stop-Process Word。

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

function Get-FailCode($raw) {
    $code = $null
    try { $code = ($raw.Stderr | ConvertFrom-Json).error.code } catch { $code = $null }
    return $code
}

$glb = Ensure-OoxpGlb
$glbB = Ensure-OoxpGlbB
$glbFile = $glb.Replace('\', '/')
$glbBFile = $glbB.Replace('\', '/')

$src = Get-OutPath 'docx-model3d-geom-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{ paragraphs = @(@{ text = 'model3d-geom-host' }) }
} 'docx-model3d-geom-create-host' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; model3ds = $true }
} 'docx-model3d-geom-inspect-src'
$para = [string]$tree0.paragraphs[0].id

$inserted = Get-OutPath 'docx-model3d-geom-inserted.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{ op = 'insertModel3d'; id = $para; file = $glbFile })
} 'docx-model3d-geom-insert' | Out-Null

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; model3ds = $true }
} 'docx-model3d-geom-inspect-insert'
$item = @($tree1.paragraphs[0].model3ds) | Select-Object -First 1
$id = [string]$item.id

$sized = Get-OutPath 'docx-model3d-geom-sized.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $sized.Replace('\', '/')
    ops = @(@{ op = 'setModel3dSize'; id = $id; cx = 3657600; cy = 1828800 })
} 'docx-model3d-geom-size' | Out-Null

$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sized.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; model3ds = $true }
} 'docx-model3d-geom-inspect-size'
$itemS = @($treeS.paragraphs[0].model3ds) | Select-Object -First 1
$xmlS = Get-ZipPartText $sized 'word/document.xml'
$cxHits = ([regex]::Matches($xmlS, 'cx="3657600"')).Count
$word = Ensure-MsWord $sized
$docS = Open-WordDoc $word $sized
$capS = [string]$docS.ActiveWindow.Caption
$wPt = $null
$hPt = $null
if ($docS.InlineShapes.Count -ge 1) {
    $shS = $docS.InlineShapes.Item(1)
    $wPt = [math]::Round([double]$shS.Width, 2)
    $hPt = [math]::Round([double]$shS.Height, 2)
    Release-Com $shS
}
Add-Check 'docx-D3g1' (
    ([int]$itemS.cx -eq 3657600) -and ([int]$itemS.cy -eq 1828800) -and ($cxHits -ge 2) -and
    ($capS -notmatch 'Repair') -and ($null -ne $wPt) -and ([math]::Abs($wPt - 288) -lt 2) -and
    ($null -ne $hPt) -and ([math]::Abs($hPt - 144) -lt 2)
) "cx=$($itemS.cx) cy=$($itemS.cy) hits=$cxHits caption=$capS w=$wPt h=$hPt"
Close-WordDoc $docS

$wrapped = Get-OutPath 'docx-model3d-geom-wrapped.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sized.Replace('\', '/')
    output = $wrapped.Replace('\', '/')
    ops = @(@{
        op = 'setModel3dWrap'; id = $id; kind = 'anchor'; wrap = 'square'
        alignH = 'left'; alignV = 'top'
    })
} 'docx-model3d-geom-wrap' | Out-Null

$treeW = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $wrapped.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; model3ds = $true }
} 'docx-model3d-geom-inspect-wrap'
$itemW = @($treeW.paragraphs[0].model3ds) | Select-Object -First 1
$docW = Open-WordDoc $word $wrapped
$capW = [string]$docW.ActiveWindow.Caption
Add-Check 'docx-D3g2' (
    ($itemW.kind -eq 'anchor') -and ($itemW.wrap -eq 'square') -and ($capW -notmatch 'Repair')
) "kind=$($itemW.kind) wrap=$($itemW.wrap) caption=$capW"
Close-WordDoc $docW

$inlined = Get-OutPath 'docx-model3d-geom-inline.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $wrapped.Replace('\', '/')
    output = $inlined.Replace('\', '/')
    ops = @(@{ op = 'setModel3dWrap'; id = $id; kind = 'inline' })
} 'docx-model3d-geom-inline' | Out-Null

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inlined.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; model3ds = $true }
} 'docx-model3d-geom-inspect-inline'
$itemI = @($treeI.paragraphs[0].model3ds) | Select-Object -First 1
$docI = Open-WordDoc $word $inlined
$capI = [string]$docI.ActiveWindow.Caption
$wrapNull = ($null -eq $itemI.wrap) -or ([string]$itemI.wrap -eq '')
Add-Check 'docx-D3g3' (
    ($itemI.kind -eq 'inline') -and $wrapNull -and ($capI -notmatch 'Repair')
) "kind=$($itemI.kind) wrap=$($itemI.wrap) caption=$capI"
Close-WordDoc $docI

$replaced = Get-OutPath 'docx-model3d-geom-replaced.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sized.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{ op = 'replaceModel3d'; id = $id; file = $glbBFile })
} 'docx-model3d-geom-replace' | Out-Null

$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; model3ds = $true }
} 'docx-model3d-geom-inspect-replace'
$itemR = @($treeR.paragraphs[0].model3ds) | Select-Object -First 1
$docR = Open-WordDoc $word $replaced
$capR = [string]$docR.ActiveWindow.Caption
Add-Check 'docx-D3g4' (
    ([int]$itemR.cx -eq 3657600) -and ([int]$itemR.cy -eq 1828800) -and ($capR -notmatch 'Repair')
) "cx=$($itemR.cx) cy=$($itemR.cy) caption=$capR"
Close-WordDoc $docR

$hfSrc = Get-OutPath 'docx-model3d-geom-hf-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hfSrc.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(@{ text = 'body' })
        headers = @(@{ type = 'default'; paragraphs = @(@{ text = 'hdr' }) })
    }
} 'docx-model3d-geom-hf-create' | Out-Null
$treeH0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfSrc.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true; model3ds = $true }
} 'docx-model3d-geom-inspect-hf-src'
$hPara = [string]$treeH0.headers[0].paragraphs[0].id
$hfIns = Get-OutPath 'docx-model3d-geom-hf-ins.docx'
Close-WordDocByPath $word $hfIns
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hfSrc.Replace('\', '/')
    output = $hfIns.Replace('\', '/')
    ops = @(@{ op = 'insertModel3d'; id = $hPara; file = $glbFile })
} 'docx-model3d-geom-hf-insert' | Out-Null
$treeH1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfIns.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true; model3ds = $true }
} 'docx-model3d-geom-inspect-hf-ins'
$hId = [string](@($treeH1.headers[0].model3ds) | Select-Object -First 1).id
$hfOut = Get-OutPath 'docx-model3d-geom-hf.docx'
Close-WordDocByPath $word $hfOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hfIns.Replace('\', '/')
    output = $hfOut.Replace('\', '/')
    ops = @(@{ op = 'setModel3dSize'; id = $hId; cx = 3657600; cy = 1828800 })
} 'docx-model3d-geom-hf-size' | Out-Null
$treeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfOut.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true; model3ds = $true }
} 'docx-model3d-geom-inspect-hf'
$hItem = @($treeH.headers[0].model3ds) | Select-Object -First 1
$docH = Open-WordDoc $word $hfOut
$capH = [string]$docH.ActiveWindow.Caption
Add-Check 'docx-D3g5' (
    ([int]$hItem.cx -eq 3657600) -and ([int]$hItem.cy -eq 1828800) -and ($capH -notmatch 'Repair')
) "cx=$($hItem.cx) cy=$($hItem.cy) caption=$capH"
Close-WordDoc $docH

$failSize = Get-OutPath 'docx-model3d-geom-fail-size.docx'
$rawSize = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failSize.Replace('\', '/')
    ops = @(@{ op = 'setModel3dSize'; id = $id })
} 'docx-model3d-geom-fail-size'
$failPic = Get-OutPath 'docx-model3d-geom-fail-pic.docx'
$rawPic = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failPic.Replace('\', '/')
    ops = @(@{ op = 'setPictureSize'; id = $id; cx = 1000; cy = 1000 })
} 'docx-model3d-geom-fail-pic'
Add-Check 'docx-D3g6' (
    ((Get-FailCode $rawSize) -eq 'InvalidField') -and
    ((Get-FailCode $rawPic) -eq 'InvalidId')
) "size=$(Get-FailCode $rawSize) pic=$(Get-FailCode $rawPic)"
