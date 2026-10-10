# docx 段落边框底纹与 run 剩余：docs/checklist-docx.md 的 Bd1–Bd3、Rh1–Rh3。
# 用 ooxp 写 pBdr / shd / tabs 与 highlight / strike / dstrike / vertAlign，再用 Word 打开核对。

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

$src = Get-OutPath 'docx-para-run-remain-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = '边框段' }
            @{ text = 'run段' }
        )
    }
} 'docx-para-run-remain-create' | Out-Null

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; runs = $true }
} 'docx-para-run-remain-inspect-src'
$pBd = @($srcTree.paragraphs) | Where-Object { [string]$_.text -eq '边框段' } | Select-Object -First 1
$pRh = @($srcTree.paragraphs) | Where-Object { [string]$_.text -eq 'run段' } | Select-Object -First 1
$paraId = [string]$pBd.id
$runId = [string]$pRh.runs[0].id

$paraOut = Get-OutPath 'docx-para-run-remain-para.docx'
Close-WordDocByPath (Ensure-MsWord $src) $paraOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $paraOut.Replace('\', '/')
    ops = @(@{
        op = 'setParaFormat'; id = $paraId
        borders = @{
            top = @{ val = 'single'; sz = 12; space = 1; color = '000000' }
        }
        shd = @{ fill = 'ffff00' }
        tabs = @(@{ val = 'left'; pos = 720 })
    })
} 'docx-para-run-remain-para' | Out-Null

$treeP = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $paraOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-para-run-remain-inspect-para'
$pp = @($treeP.paragraphs) | Where-Object { $_.id -eq $paraId } | Select-Object -First 1
$bd1ok = ([string]$pp.borders.top.val -eq 'single') -and
    ([int]$pp.borders.top.sz -eq 12) -and
    ([string]$pp.shd.fill -eq 'FFFF00') -and
    ([string]$pp.tabs[0].val -eq 'left') -and
    ([int]$pp.tabs[0].pos -eq 720)
Add-Check 'docx-Bd1' $bd1ok "val=$($pp.borders.top.val) fill=$($pp.shd.fill) tab=$($pp.tabs[0].val):$($pp.tabs[0].pos)"

$word = Ensure-MsWord $src
$docP = Open-WordDoc $word $paraOut
$capP = [string]$docP.ActiveWindow.Caption
$shdColor = $null
$lineStyle = $null
$tabCount = $null
$pf = $null
$shd = $null
$borders = $null
$top = $null
$tabs = $null
try {
    $pf = $docP.Paragraphs.Item(1).Range.ParagraphFormat
    $shd = $pf.Shading
    $shdColor = [int]$shd.BackgroundPatternColor
    $borders = $pf.Borders
    $top = $borders.Item(-1)
    $lineStyle = [int]$top.LineStyle
    $tabs = $pf.TabStops
    $tabCount = [int]$tabs.Count
} catch { }
finally {
    Release-Com $tabs
    Release-Com $top
    Release-Com $borders
    Release-Com $shd
    Release-Com $pf
}
Close-WordDoc $docP
$bd2ok = ($capP -notmatch 'Repair') -and
    ($capP -match 'docx-para-run-remain-para') -and
    ($shdColor -eq 65535) -and
    ($lineStyle -ne 0) -and
    ($tabCount -ge 1)
Add-Check 'docx-Bd2' $bd2ok "caption=$capP shd=$shdColor line=$lineStyle tabs=$tabCount"

$cleared = Get-OutPath 'docx-para-run-remain-clear.docx'
Close-WordDocByPath $word $cleared
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $paraOut.Replace('\', '/')
    output = $cleared.Replace('\', '/')
    ops = @(@{
        op = 'setParaFormat'; id = $paraId
        borders = $null; shd = $null; tabs = $null
    })
} 'docx-para-run-remain-clear' | Out-Null

$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cleared.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-para-run-remain-inspect-clear'
$pc = @($treeC.paragraphs) | Where-Object { $_.id -eq $paraId } | Select-Object -First 1
$docC = Open-WordDoc $word $cleared
$capC = [string]$docC.ActiveWindow.Caption
Close-WordDoc $docC
$bd3ok = ($null -eq $pc.borders) -and ($null -eq $pc.shd) -and ($null -eq $pc.tabs) -and
    ($capC -notmatch 'Repair') -and ($capC -match 'docx-para-run-remain-clear')
Add-Check 'docx-Bd3' $bd3ok "borders=$($pc.borders) shd=$($pc.shd) tabs=$($pc.tabs) caption=$capC"

$runOut = Get-OutPath 'docx-para-run-remain-run.docx'
Close-WordDocByPath $word $runOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $runOut.Replace('\', '/')
    ops = @(@{
        op = 'setRunFormat'; id = $runId
        highlight = 'yellow'; strike = $true
    })
} 'docx-para-run-remain-run' | Out-Null

$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $runOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; runs = $true }
} 'docx-para-run-remain-inspect-run'
$pr = @($treeR.paragraphs) | Where-Object { $_.id -eq $pRh.id } | Select-Object -First 1
$r0 = $pr.runs[0]
$docR = Open-WordDoc $word $runOut
$capR = [string]$docR.ActiveWindow.Caption
$hl = $null
$strike = $null
$rng = $null
$font = $null
try {
    $rng = $docR.Paragraphs.Item(2).Range
    $hl = [int]$rng.HighlightColorIndex
    $font = $rng.Font
    $strike = [int]$font.StrikeThrough
} catch { }
finally {
    Release-Com $font
    Release-Com $rng
}
Close-WordDoc $docR
$rh1ok = ([string]$r0.highlight -eq 'yellow') -and ($r0.strike -eq $true) -and
    ($capR -notmatch 'Repair') -and ($capR -match 'docx-para-run-remain-run') -and
    ($hl -ne 0) -and ($strike -ne 0)
Add-Check 'docx-Rh1' $rh1ok "hl=$($r0.highlight) strike=$($r0.strike) caption=$capR comHl=$hl comStrike=$strike"

$supOut = Get-OutPath 'docx-para-run-remain-sup.docx'
Close-WordDocByPath $word $supOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $supOut.Replace('\', '/')
    ops = @(@{
        op = 'setRunFormat'; id = $runId
        vertAlign = 'superscript'; dstrike = $true
    })
} 'docx-para-run-remain-sup' | Out-Null

$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $supOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; runs = $true }
} 'docx-para-run-remain-inspect-sup'
$ps = @($treeS.paragraphs) | Where-Object { $_.id -eq $pRh.id } | Select-Object -First 1
$rs = $ps.runs[0]
$docS = Open-WordDoc $word $supOut
$capS = [string]$docS.ActiveWindow.Caption
$sup = $null
$dstrike = $null
$fontS = $null
try {
    $fontS = $docS.Paragraphs.Item(2).Range.Characters.Item(1).Font
    $sup = [int]$fontS.Superscript
    $dstrike = [int]$fontS.DoubleStrikeThrough
} catch { }
finally {
    Release-Com $fontS
}
Close-WordDoc $docS
$rh2ok = ([string]$rs.vertAlign -eq 'superscript') -and ($rs.dstrike -eq $true) -and
    ($capS -notmatch 'Repair') -and ($capS -match 'docx-para-run-remain-sup')
Add-Check 'docx-Rh2' $rh2ok "vert=$($rs.vertAlign) dstrike=$($rs.dstrike) caption=$capS comSup=$sup comDstrike=$dstrike"

$xlsx = Get-OutPath 'docx-para-run-remain.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-para-run-remain-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-para-run-remain-fail-xlsx.xlsx'
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{
        op = 'setRunFormat'; id = $runId; highlight = 'yellow'
    })
} 'docx-para-run-remain-fail-xlsx'
try { $codeX = ($rawX.Stderr | ConvertFrom-Json).error.code } catch { $codeX = $null }

$failHl = Get-OutPath 'docx-para-run-remain-fail-hl.docx'
$rawH = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failHl.Replace('\', '/')
    ops = @(@{
        op = 'setRunFormat'; id = $runId; highlight = 'pink'
    })
} 'docx-para-run-remain-fail-hl'
try { $codeH = ($rawH.Stderr | ConvertFrom-Json).error.code } catch { $codeH = $null }

Add-Check 'docx-Rh3' (
    ($rawX.ExitCode -ne 0) -and ($codeX -eq 'InvalidField') -and (-not (Test-Path $failX)) -and
    ($rawH.ExitCode -ne 0) -and ($codeH -eq 'InvalidField') -and (-not (Test-Path $failHl))
) "xlsx=$codeX wroteX=$([bool](Test-Path $failX)) hl=$codeH wroteH=$([bool](Test-Path $failHl))"
