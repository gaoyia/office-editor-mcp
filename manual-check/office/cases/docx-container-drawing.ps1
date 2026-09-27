# docx 容器内 drawing：docs/checklist-docx.md 的 Cd1–Cd6。
# 用 create 产物再 apply，不另做 Word 另存样例。

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

function Test-WordCommentText($doc, [string]$text) {
    $comments = $doc.Comments
    try {
        $n = [int]$comments.Count
        for ($i = 1; $i -le $n; $i++) {
            $c = $comments.Item($i)
            try {
                $t = ([string]$c.Range.Text).Trim()
                if ($t -match [regex]::Escape($text)) { return $true }
            }
            finally { Release-Com $c }
        }
        return $false
    }
    finally { Release-Com $comments }
}

$hostPath = Get-OutPath 'docx-container-drawing-host.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = $S.chapter1 }
            @{ text = $S.hello }
        )
    }
} 'docx-cd-create' | Out-Null

$treeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 10 }
} 'docx-cd-host-inspect'
$p1 = $treeH.paragraphs[0].id
$p2 = $treeH.paragraphs[1].id
$rootChart0 = [int]$treeH.chartCount

$word = Ensure-MsWord $hostPath

# Cd1 / Cd2：批注内柱形图。
$cmtOut = Get-OutPath 'docx-container-drawing-comment.docx'
Close-WordDocByPath $word $cmtOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $cmtOut.Replace('\', '/')
    ops = @(@{ op = 'insertComment'; start = $p1; end = $p1; text = $S.commentA })
} 'docx-cd-insert-comment' | Out-Null
$treeC0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cmtOut.Replace('\', '/'); format = 'docx'
    scope = @{ comments = $true }
} 'docx-cd-comment-inspect'
$cmtPara = @($treeC0.comments[0].paragraphs)[0].id
$cmtChartOut = Get-OutPath 'docx-container-drawing-comment-chart.docx'
Close-WordDocByPath $word $cmtChartOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $cmtOut.Replace('\', '/')
    output = $cmtChartOut.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'
        id = $cmtPara
        type = 'col'
        title = $S.chartTitle
        series = @(@{
            name = $S.chartSer1
            categories = @('甲', '乙')
            values = @(1, 2)
        })
    })
} 'docx-cd-comment-chart' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cmtChartOut.Replace('\', '/'); format = 'docx'
    scope = @{ comments = $true; charts = $true }
} 'docx-cd-comment-chart-inspect'
$cmtChart = @($treeC.comments[0].charts)[0]
$cmtIdOk = $cmtChart -and ([string]$cmtChart.id -like 'comment:*')
Add-Check 'docx-Cd1' (
    ($treeC.comments[0].chartCount -eq 1) -and $cmtIdOk -and
    ([int]$treeC.chartCount -eq $rootChart0)
) "cmtChart=$($treeC.comments[0].chartCount) id=$($cmtChart.id) root=$($treeC.chartCount)"

$docC = Open-WordDoc $word $cmtChartOut
$capC = [string]$docC.ActiveWindow.Caption
$sawC = Test-WordCommentText $docC $S.commentA
Add-Check 'docx-Cd2' (($capC -notmatch 'Repair') -and $sawC) "caption=$capC saw=$sawC"
Close-WordDoc $docC

# Cd3 / Cd4：Word 不能往注内或批注内插文本框。
$fnOut = Get-OutPath 'docx-container-drawing-footnote.docx'
Close-WordDocByPath $word $fnOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $fnOut.Replace('\', '/')
    ops = @(@{ op = 'insertFootnote'; id = $p2; text = $S.footnoteInsert })
} 'docx-cd-insert-footnote' | Out-Null
$treeF0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $fnOut.Replace('\', '/'); format = 'docx'
    scope = @{ notes = $true }
} 'docx-cd-footnote-inspect'
$fnPara = @($treeF0.footnotes[0].paragraphs)[0].id
$fnBoxOut = Get-OutPath 'docx-container-drawing-footnote-txbx.docx'
if (Test-Path $fnBoxOut) { Remove-Item -Force $fnBoxOut }
$rawFnBox = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $fnOut.Replace('\', '/')
    output = $fnBoxOut.Replace('\', '/')
    ops = @(@{ op = 'insertTextBox'; id = $fnPara; text = $S.txbxFloat })
} 'docx-cd-footnote-txbx'
$codeFnBox = $null
try { $codeFnBox = ($rawFnBox.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'docx-Cd3' (
    ($rawFnBox.ExitCode -ne 0) -and ($codeFnBox -eq 'InvalidField') -and (-not (Test-Path $fnBoxOut))
) "code=$codeFnBox wrote=$(Test-Path $fnBoxOut)"

$cmtBoxOut = Get-OutPath 'docx-container-drawing-comment-txbx.docx'
if (Test-Path $cmtBoxOut) { Remove-Item -Force $cmtBoxOut }
$rawCmtBox = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $cmtOut.Replace('\', '/')
    output = $cmtBoxOut.Replace('\', '/')
    ops = @(@{ op = 'insertTextBox'; id = $cmtPara; text = $S.txbxFloat })
} 'docx-cd-comment-txbx'
$codeCmtBox = $null
try { $codeCmtBox = ($rawCmtBox.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'docx-Cd4' (
    ($rawCmtBox.ExitCode -ne 0) -and ($codeCmtBox -eq 'InvalidField') -and (-not (Test-Path $cmtBoxOut))
) "code=$codeCmtBox wrote=$(Test-Path $cmtBoxOut)"

# Cd5：框内图表，根计数不含该图。
$boxOut = Get-OutPath 'docx-container-drawing-textbox.docx'
Close-WordDocByPath $word $boxOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $boxOut.Replace('\', '/')
    ops = @(@{ op = 'insertTextBox'; id = $p2; text = $S.txbxFloat })
} 'docx-cd-insert-txbx' | Out-Null
$treeB0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $boxOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 10; textBoxes = $true; textBoxContent = $true }
} 'docx-cd-txbx-inspect'
$inner = $null
foreach ($p in @($treeB0.paragraphs)) {
    foreach ($b in @($p.textBoxes)) {
        $inner = @($b.paragraphs)[0].id
        if ($inner) { break }
    }
    if ($inner) { break }
}
$boxChartOut = Get-OutPath 'docx-container-drawing-textbox-chart.docx'
Close-WordDocByPath $word $boxChartOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $boxOut.Replace('\', '/')
    output = $boxChartOut.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'
        id = $inner
        type = 'col'
        title = $S.chartTitle
        series = @(@{
            name = $S.chartSer1
            categories = @('甲', '乙')
            values = @(1, 2)
        })
    })
} 'docx-cd-txbx-chart' | Out-Null
$treeB = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $boxChartOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 10; textBoxes = $true; textBoxContent = $true; charts = $true }
} 'docx-cd-txbx-chart-inspect'
$boxNode = $null
foreach ($p in @($treeB.paragraphs)) {
    foreach ($b in @($p.textBoxes)) {
        if ([int]$b.chartCount -ge 1) { $boxNode = $b; break }
    }
    if ($boxNode) { break }
}
$docB = Open-WordDoc $word $boxChartOut
$capB = [string]$docB.ActiveWindow.Caption
Close-WordDoc $docB
Add-Check 'docx-Cd5' (
    ($null -ne $boxNode) -and ([int]$boxNode.chartCount -eq 1) -and
    ([int]$treeB.chartCount -eq $rootChart0) -and ($capB -notmatch 'Repair')
) "boxChart=$($boxNode.chartCount) root=$($treeB.chartCount) id=$($boxNode.charts[0].id) caption=$capB"

# Cd6：容器 id。
$failChart = Get-OutPath 'docx-container-drawing-bad-chart.docx'
if (Test-Path $failChart) { Remove-Item -Force $failChart }
$rawChart = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $cmtOut.Replace('\', '/')
    output = $failChart.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'
        id = 'comment:0'
        type = 'col'
        series = @(@{ values = @(1, 2) })
    })
} 'docx-cd-bad-chart'
$codeChart = $null
try { $codeChart = ($rawChart.Stderr | ConvertFrom-Json).error.code } catch {}

$failBox = Get-OutPath 'docx-container-drawing-bad-txbx.docx'
if (Test-Path $failBox) { Remove-Item -Force $failBox }
$boxId = $null
foreach ($p in @($treeB0.paragraphs)) {
    foreach ($b in @($p.textBoxes)) { $boxId = [string]$b.id; if ($boxId) { break } }
    if ($boxId) { break }
}
$rawBox = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $boxOut.Replace('\', '/')
    output = $failBox.Replace('\', '/')
    ops = @(@{ op = 'insertTextBox'; id = $boxId; text = 'x' })
} 'docx-cd-bad-txbx'
$codeBox = $null
try { $codeBox = ($rawBox.Stderr | ConvertFrom-Json).error.code } catch {}
$noWrite = (-not (Test-Path $failChart)) -and (-not (Test-Path $failBox))
Add-Check 'docx-Cd6' (
    ($rawChart.ExitCode -ne 0) -and ($codeChart -eq 'InvalidId') -and
    ($rawBox.ExitCode -ne 0) -and ($codeBox -eq 'InvalidId') -and $noWrite
) "chart=$codeChart box=$codeBox wrote=$(-not $noWrite)"
