# docx 字符缩放：docs/checklist-docx.md 的 Rsc1–Rsc6。
# 用 ooxp 写 run 上 w:w，再用 Word 打开核对。不要和 Rcs、Rh、Ea 撞。不要 Stop-Process Word。

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

$src = Get-OutPath 'docx-run-scale-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = '缩放段' }
            @{
                text = '框'
                textBoxes = @(@{ text = '框内字' })
            }
        )
    }
} 'docx-run-scale-create-src' | Out-Null

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; runs = $true; textBoxes = $true; textBoxContent = $true }
} 'docx-run-scale-inspect-src'
$pBody = @($srcTree.paragraphs) | Where-Object { [string]$_.text -eq '缩放段' } | Select-Object -First 1
$pBoxHost = @($srcTree.paragraphs) | Where-Object { [string]$_.text -eq '框' } | Select-Object -First 1
$runId = [string]$pBody.runs[0].id
$boxPara = $pBoxHost.textBoxes[0].paragraphs[0]
$boxRunId = [string]$boxPara.runs[0].id

$applyOut = Get-OutPath 'docx-run-scale-apply.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $applyOut.Replace('\', '/')
    ops = @(@{
        op = 'setRunFormat'; id = $runId; scale = 200
    })
} 'docx-run-scale-apply' | Out-Null

$treeA = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $applyOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; runs = $true }
} 'docx-run-scale-inspect-apply'
$pa = @($treeA.paragraphs) | Where-Object { $_.id -eq $pBody.id } | Select-Object -First 1
$ra = $pa.runs[0]
$rsc1tree = [int]$ra.scale -eq 200

$halfOut = Get-OutPath 'docx-run-scale-50.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $applyOut.Replace('\', '/')
    output = $halfOut.Replace('\', '/')
    ops = @(@{
        op = 'setRunFormat'; id = $runId; scale = 50
    })
} 'docx-run-scale-50' | Out-Null

$treeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $halfOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; runs = $true }
} 'docx-run-scale-inspect-50'
$ph = @($treeH.paragraphs) | Where-Object { $_.id -eq $pBody.id } | Select-Object -First 1
$rh = $ph.runs[0]
$rsc2tree = [int]$rh.scale -eq 50

$clrOut = Get-OutPath 'docx-run-scale-clear.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $halfOut.Replace('\', '/')
    output = $clrOut.Replace('\', '/')
    ops = @(@{
        op = 'setRunFormat'; id = $runId; scale = $null
    })
} 'docx-run-scale-clear' | Out-Null

$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $clrOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; runs = $true }
} 'docx-run-scale-inspect-clear'
$pc = @($treeC.paragraphs) | Where-Object { $_.id -eq $pBody.id } | Select-Object -First 1
$rc = $pc.runs[0]
$xmlC = Get-ZipPartText $clrOut 'word/document.xml'
$rsc3tree = ($null -eq $rc.scale) -and ($xmlC -notmatch '<w:w[\s>]')

$boxOut = Get-OutPath 'docx-run-scale-box.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $boxOut.Replace('\', '/')
    ops = @(@{
        op = 'setRunFormat'; id = $boxRunId; scale = 200
    })
} 'docx-run-scale-box' | Out-Null

$treeB = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $boxOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; runs = $true; textBoxes = $true; textBoxContent = $true }
} 'docx-run-scale-inspect-box'
$pb = @($treeB.paragraphs) | Where-Object { $_.id -eq $pBoxHost.id } | Select-Object -First 1
$boxRun = $pb.textBoxes[0].paragraphs[0].runs[0]
$rsc4tree = [int]$boxRun.scale -eq 200

$createOut = Get-OutPath 'docx-run-scale-create.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $createOut.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(@{
            runs = @(@{ text = '字'; scale = 200 })
        })
    }
} 'docx-run-scale-create' | Out-Null

$treeCr = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $createOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 5; runs = $true }
} 'docx-run-scale-inspect-create'
$rsc5tree = [int]$treeCr.paragraphs[0].runs[0].scale -eq 200

$fail0 = Get-OutPath 'docx-run-scale-fail-0.docx'
$raw0 = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $fail0.Replace('\', '/')
    ops = @(@{ op = 'setRunFormat'; id = $runId; scale = 0 })
} 'docx-run-scale-fail-0'
$fail601 = Get-OutPath 'docx-run-scale-fail-601.docx'
$raw601 = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $fail601.Replace('\', '/')
    ops = @(@{ op = 'setRunFormat'; id = $runId; scale = 601 })
} 'docx-run-scale-fail-601'
$xlsx = Get-OutPath 'docx-run-scale.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-run-scale-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-run-scale-fail-xlsx.xlsx'
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'setRunFormat'; id = 'para:AAAA0001/run:1'; scale = 200 })
} 'docx-run-scale-fail-xlsx'
Add-Check 'docx-Rsc6' (
    ((Get-FailCode $raw0) -eq 'InvalidField') -and
    ((Get-FailCode $raw601) -eq 'InvalidField') -and
    ((Get-FailCode $rawX) -eq 'InvalidField')
) "zero=$(Get-FailCode $raw0) over=$(Get-FailCode $raw601) xlsx=$(Get-FailCode $rawX)"

$seed = Ensure-DocxCreate
$word = Ensure-MsWord $seed
try { $word.AskToUpdateLinks = $false } catch {}
try { $word.Options.UpdateLinksAtOpen = $false } catch {}

Close-WordDocByPath $word $applyOut
$docA = Open-WordDoc $word $applyOut
$capA = [string]$docA.ActiveWindow.Caption
$comA = 0
$fontA = $null
try {
    $fontA = $docA.Paragraphs.Item(1).Range.Characters.Item(1).Font
    $comA = [int]$fontA.Scaling
} catch { }
finally {
    Release-Com $fontA
}
Close-WordDoc $docA
Add-Check 'docx-Rsc1' (
    $rsc1tree -and ($capA -notmatch 'Repair') -and ($capA -match 'docx-run-scale-apply') -and ($comA -eq 200)
) "scale=$($ra.scale) caption=$capA com=$comA"

Close-WordDocByPath $word $halfOut
$docH = Open-WordDoc $word $halfOut
$capH = [string]$docH.ActiveWindow.Caption
$comH = 0
$fontH = $null
try {
    $fontH = $docH.Paragraphs.Item(1).Range.Characters.Item(1).Font
    $comH = [int]$fontH.Scaling
} catch { }
finally {
    Release-Com $fontH
}
Close-WordDoc $docH
Add-Check 'docx-Rsc2' (
    $rsc2tree -and ($capH -notmatch 'Repair') -and ($capH -match 'docx-run-scale-50') -and ($comH -eq 50)
) "scale=$($rh.scale) caption=$capH com=$comH"

Close-WordDocByPath $word $clrOut
$docC = Open-WordDoc $word $clrOut
$capC = [string]$docC.ActiveWindow.Caption
Close-WordDoc $docC
Add-Check 'docx-Rsc3' (
    $rsc3tree -and ($capC -notmatch 'Repair') -and ($capC -match 'docx-run-scale-clear')
) "scale=$($rc.scale) caption=$capC"

Close-WordDocByPath $word $boxOut
$docB = Open-WordDoc $word $boxOut
$capB = [string]$docB.ActiveWindow.Caption
Close-WordDoc $docB
Add-Check 'docx-Rsc4' (
    $rsc4tree -and ($capB -notmatch 'Repair') -and ($capB -match 'docx-run-scale-box')
) "scale=$($boxRun.scale) caption=$capB"

Close-WordDocByPath $word $createOut
$docCr = Open-WordDoc $word $createOut
$capCr = [string]$docCr.ActiveWindow.Caption
Close-WordDoc $docCr
Add-Check 'docx-Rsc5' (
    $rsc5tree -and ($capCr -notmatch 'Repair') -and ($capCr -match 'docx-run-scale-create')
) "scale=$($treeCr.paragraphs[0].runs[0].scale) caption=$capCr"
