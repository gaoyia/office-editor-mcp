# docx 墨迹画布尺寸与绕排：docs/checklist-docx.md 的 Is1–Is5。
# 编号 Is，不要和 Ic、Ik 撞。探测脚本不进本文件。不要 Stop-Process Word。

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

$ink = Get-OutPath 'ink-geom.xml'
[IO.File]::WriteAllText($ink, @'
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<ink xmlns="http://www.w3.org/2003/InkML">
  <trace>0 0, 10 10, 20 5</trace>
</ink>
'@, $global:OoxpUtf8)
$inkB = Get-OutPath 'ink-geom-b.xml'
[IO.File]::WriteAllText($inkB, @'
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<ink xmlns="http://www.w3.org/2003/InkML">
  <trace>1 1, 30 30</trace>
</ink>
'@, $global:OoxpUtf8)
$inkFile = $ink.Replace('\', '/')
$inkFileB = $inkB.Replace('\', '/')

$src = Get-OutPath 'docx-ink-geom-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{ paragraphs = @(@{ text = 'ink-geom-host' }) }
} 'docx-ink-geom-create-host' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; inkCanvases = $true }
} 'docx-ink-geom-inspect-src'
$para = [string]$tree0.paragraphs[0].id

$inserted = Get-OutPath 'docx-ink-geom-inserted.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{ op = 'insertInkCanvas'; id = $para; file = $inkFile })
} 'docx-ink-geom-insert' | Out-Null

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; inkCanvases = $true }
} 'docx-ink-geom-inspect-insert'
$canvas = @($tree1.paragraphs[0].inkCanvases) | Select-Object -First 1
$inkId = [string]$canvas.id

$sized = Get-OutPath 'docx-ink-geom-sized.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $sized.Replace('\', '/')
    ops = @(@{ op = 'setInkCanvasSize'; id = $inkId; cx = 3657600; cy = 1828800 })
} 'docx-ink-geom-size' | Out-Null

$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sized.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; inkCanvases = $true }
} 'docx-ink-geom-inspect-size'
$itemS = @($treeS.paragraphs[0].inkCanvases) | Select-Object -First 1
$xmlS = Get-ZipPartText $sized 'word/document.xml'
$cxHits = ([regex]::Matches($xmlS, 'cx="3657600"')).Count
$word = Ensure-MsWord $sized
$docS = Open-WordDoc $word $sized
$capS = [string]$docS.ActiveWindow.Caption
$wPt = $null
$hPt = $null
if ($docS.Shapes.Count -ge 1) {
    $shS = $docS.Shapes.Item(1)
    $wPt = [math]::Round([double]$shS.Width, 2)
    $hPt = [math]::Round([double]$shS.Height, 2)
    Release-Com $shS
}
Add-Check 'docx-Is1' (
    ([int]$itemS.cx -eq 3657600) -and ([int]$itemS.cy -eq 1828800) -and ($cxHits -ge 2) -and
    ($capS -notmatch 'Repair') -and ($null -ne $wPt) -and ([math]::Abs($wPt - 288) -lt 2) -and
    ($null -ne $hPt) -and ([math]::Abs($hPt - 144) -lt 2)
) "cx=$($itemS.cx) cy=$($itemS.cy) hits=$cxHits caption=$capS w=$wPt h=$hPt"
Close-WordDoc $docS

$wrapped = Get-OutPath 'docx-ink-geom-wrapped.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sized.Replace('\', '/')
    output = $wrapped.Replace('\', '/')
    ops = @(@{
        op = 'setInkCanvasWrap'; id = $inkId; kind = 'anchor'; wrap = 'square'
        alignH = 'left'; alignV = 'top'
    })
} 'docx-ink-geom-wrap' | Out-Null

$treeW = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $wrapped.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; inkCanvases = $true }
} 'docx-ink-geom-inspect-wrap'
$itemW = @($treeW.paragraphs[0].inkCanvases) | Select-Object -First 1
$docW = Open-WordDoc $word $wrapped
$capW = [string]$docW.ActiveWindow.Caption
Add-Check 'docx-Is2' (
    ($itemW.kind -eq 'anchor') -and ($itemW.wrap -eq 'square') -and ($capW -notmatch 'Repair')
) "kind=$($itemW.kind) wrap=$($itemW.wrap) caption=$capW"
Close-WordDoc $docW

$inlined = Get-OutPath 'docx-ink-geom-inline.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $wrapped.Replace('\', '/')
    output = $inlined.Replace('\', '/')
    ops = @(@{ op = 'setInkCanvasWrap'; id = $inkId; kind = 'inline' })
} 'docx-ink-geom-inline' | Out-Null

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inlined.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; inkCanvases = $true }
} 'docx-ink-geom-inspect-inline'
$itemI = @($treeI.paragraphs[0].inkCanvases) | Select-Object -First 1
$docI = Open-WordDoc $word $inlined
$capI = [string]$docI.ActiveWindow.Caption
$wrapNull = ($null -eq $itemI.wrap) -or ([string]$itemI.wrap -eq '')
Add-Check 'docx-Is3' (
    ($itemI.kind -eq 'inline') -and $wrapNull -and ($capI -notmatch 'Repair')
) "kind=$($itemI.kind) wrap=$($itemI.wrap) caption=$capI"
Close-WordDoc $docI

$replaced = Get-OutPath 'docx-ink-geom-replaced.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sized.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{ op = 'replaceInkCanvas'; id = $inkId; file = $inkFileB })
} 'docx-ink-geom-replace' | Out-Null

$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; inkCanvases = $true }
} 'docx-ink-geom-inspect-replace'
$itemR = @($treeR.paragraphs[0].inkCanvases) | Select-Object -First 1
$docR = Open-WordDoc $word $replaced
$capR = [string]$docR.ActiveWindow.Caption
Add-Check 'docx-Is4' (
    ([int]$itemR.cx -eq 3657600) -and ([int]$itemR.cy -eq 1828800) -and ($capR -notmatch 'Repair')
) "cx=$($itemR.cx) cy=$($itemR.cy) caption=$capR"
Close-WordDoc $docR

$failSize = Get-OutPath 'docx-ink-geom-fail-size.docx'
$rawSize = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $failSize.Replace('\', '/')
    ops = @(@{ op = 'setInkCanvasSize'; id = $inkId })
} 'docx-ink-geom-fail-size'
$xlsx = Get-OutPath 'docx-ink-geom.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-ink-geom-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-ink-geom-fail-xlsx.xlsx'
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'setInkCanvasSize'; id = 'sheet:Sheet1/ink:1'; cx = 1000 })
} 'docx-ink-geom-fail-xlsx'
Add-Check 'docx-Is5' (
    ((Get-FailCode $rawSize) -eq 'InvalidField') -and
    ((Get-FailCode $rawX) -eq 'InvalidField')
) "size=$(Get-FailCode $rawSize) xlsx=$(Get-FailCode $rawX)"
