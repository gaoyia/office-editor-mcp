# docx 字符间距：docs/checklist-docx.md 的 Rcs1–Rcs6。
# 用 ooxp 写 run 上 w:spacing / w:kern，再用 Word 打开核对。

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

function Test-PtClose($got, $want) {
    if ($null -eq $got) { return $false }
    return [math]::Abs([double]$got - [double]$want) -le 0.05
}

$src = Get-OutPath 'docx-run-spacing-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = '间距段' }
            @{
                text = '框'
                textBoxes = @(@{ text = '框内字' })
            }
        )
    }
} 'docx-run-spacing-create' | Out-Null

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; runs = $true; textBoxes = $true; textBoxContent = $true }
} 'docx-run-spacing-inspect-src'
$pBody = @($srcTree.paragraphs) | Where-Object { [string]$_.text -eq '间距段' } | Select-Object -First 1
$pBoxHost = @($srcTree.paragraphs) | Where-Object { [string]$_.text -eq '框' } | Select-Object -First 1
$runId = [string]$pBody.runs[0].id
$boxPara = $pBoxHost.textBoxes[0].paragraphs[0]
$boxRunId = [string]$boxPara.runs[0].id

$geomOut = Get-OutPath 'docx-run-spacing-apply.docx'
Close-WordDocByPath (Ensure-MsWord $src) $geomOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $geomOut.Replace('\', '/')
    ops = @(@{
        op = 'setRunFormat'; id = $runId
        spacing = 40; kern = 28
    })
} 'docx-run-spacing-apply' | Out-Null

$treeG = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $geomOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; runs = $true }
} 'docx-run-spacing-inspect-apply'
$pg = @($treeG.paragraphs) | Where-Object { $_.id -eq $pBody.id } | Select-Object -First 1
$rg = $pg.runs[0]
$word = Ensure-MsWord $geomOut
$docG = Open-WordDoc $word $geomOut
$capG = [string]$docG.ActiveWindow.Caption
$comSp = $null
$comKern = $null
$fontG = $null
try {
    $fontG = $docG.Paragraphs.Item(1).Range.Characters.Item(1).Font
    $comSp = [double]$fontG.Spacing
    $comKern = [double]$fontG.Kerning
} catch { }
finally {
    Release-Com $fontG
}
Close-WordDoc $docG
$rcs1ok = ([int]$rg.spacing -eq 40) -and ($capG -notmatch 'Repair') -and
    ($capG -match 'docx-run-spacing-apply') -and (Test-PtClose $comSp 2)
Add-Check 'docx-Rcs1' $rcs1ok "spacing=$($rg.spacing) caption=$capG comSp=$comSp"
$rcs2ok = ([int]$rg.kern -eq 28) -and ($capG -notmatch 'Repair') -and (Test-PtClose $comKern 14)
Add-Check 'docx-Rcs2' $rcs2ok "kern=$($rg.kern) comKern=$comKern"

$negOut = Get-OutPath 'docx-run-spacing-neg.docx'
Close-WordDocByPath $word $negOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $geomOut.Replace('\', '/')
    output = $negOut.Replace('\', '/')
    ops = @(@{
        op = 'setRunFormat'; id = $runId; spacing = -20
    })
} 'docx-run-spacing-neg' | Out-Null

$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $negOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; runs = $true }
} 'docx-run-spacing-inspect-neg'
$pn = @($treeN.paragraphs) | Where-Object { $_.id -eq $pBody.id } | Select-Object -First 1
$rn = $pn.runs[0]
$docN = Open-WordDoc $word $negOut
$capN = [string]$docN.ActiveWindow.Caption
$comNeg = $null
$fontN = $null
try {
    $fontN = $docN.Paragraphs.Item(1).Range.Characters.Item(1).Font
    $comNeg = [double]$fontN.Spacing
} catch { }
finally {
    Release-Com $fontN
}
Close-WordDoc $docN
$rcs3ok = ([int]$rn.spacing -eq -20) -and ($capN -notmatch 'Repair') -and
    ($capN -match 'docx-run-spacing-neg') -and (Test-PtClose $comNeg -1)
Add-Check 'docx-Rcs3' $rcs3ok "spacing=$($rn.spacing) caption=$capN comSp=$comNeg"

$clrOut = Get-OutPath 'docx-run-spacing-clear.docx'
Close-WordDocByPath $word $clrOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $negOut.Replace('\', '/')
    output = $clrOut.Replace('\', '/')
    ops = @(@{
        op = 'setRunFormat'; id = $runId
        spacing = $null; kern = $null
    })
} 'docx-run-spacing-clear' | Out-Null

$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $clrOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; runs = $true }
} 'docx-run-spacing-inspect-clear'
$pc = @($treeC.paragraphs) | Where-Object { $_.id -eq $pBody.id } | Select-Object -First 1
$rc = $pc.runs[0]
$docC = Open-WordDoc $word $clrOut
$capC = [string]$docC.ActiveWindow.Caption
Close-WordDoc $docC
$rcs4ok = ($null -eq $rc.spacing) -and ($null -eq $rc.kern) -and
    ($capC -notmatch 'Repair') -and ($capC -match 'docx-run-spacing-clear')
Add-Check 'docx-Rcs4' $rcs4ok "spacing=$($rc.spacing) kern=$($rc.kern) caption=$capC"

$boxOut = Get-OutPath 'docx-run-spacing-box.docx'
Close-WordDocByPath $word $boxOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $boxOut.Replace('\', '/')
    ops = @(@{
        op = 'setRunFormat'; id = $boxRunId; spacing = 40
    })
} 'docx-run-spacing-box' | Out-Null

$treeB = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $boxOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; runs = $true; textBoxes = $true; textBoxContent = $true }
} 'docx-run-spacing-inspect-box'
$pb = @($treeB.paragraphs) | Where-Object { $_.id -eq $pBoxHost.id } | Select-Object -First 1
$br = $pb.textBoxes[0].paragraphs[0].runs[0]
$docB = Open-WordDoc $word $boxOut
$capB = [string]$docB.ActiveWindow.Caption
Close-WordDoc $docB
$rcs5ok = ([int]$br.spacing -eq 40) -and ($capB -notmatch 'Repair') -and
    ($capB -match 'docx-run-spacing-box')
Add-Check 'docx-Rcs5' $rcs5ok "boxSpacing=$($br.spacing) caption=$capB"

$xlsx = Get-OutPath 'docx-run-spacing.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-run-spacing-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-run-spacing-fail-xlsx.xlsx'
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{
        op = 'setRunFormat'; id = $runId; spacing = 40
    })
} 'docx-run-spacing-fail-xlsx'
try { $codeX = ($rawX.Stderr | ConvertFrom-Json).error.code } catch { $codeX = $null }

$failS = Get-OutPath 'docx-run-spacing-fail-str.docx'
$rawS = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failS.Replace('\', '/')
    ops = @(@{
        op = 'setRunFormat'; id = $runId; spacing = 'wide'
    })
} 'docx-run-spacing-fail-str'
try { $codeS = ($rawS.Stderr | ConvertFrom-Json).error.code } catch { $codeS = $null }

$failK = Get-OutPath 'docx-run-spacing-fail-kern.docx'
$rawK = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failK.Replace('\', '/')
    ops = @(@{
        op = 'setRunFormat'; id = $runId; kern = -1
    })
} 'docx-run-spacing-fail-kern'
try { $codeK = ($rawK.Stderr | ConvertFrom-Json).error.code } catch { $codeK = $null }

Add-Check 'docx-Rcs6' (
    ($rawX.ExitCode -ne 0) -and ($codeX -eq 'InvalidField') -and (-not (Test-Path $failX)) -and
    ($rawS.ExitCode -ne 0) -and ($codeS -eq 'InvalidField') -and (-not (Test-Path $failS)) -and
    ($rawK.ExitCode -ne 0) -and ($codeK -eq 'InvalidField') -and (-not (Test-Path $failK))
) "xlsx=$codeX str=$codeS kern=$codeK"
