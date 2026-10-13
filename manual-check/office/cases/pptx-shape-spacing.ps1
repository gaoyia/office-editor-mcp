# pptx 形状行距与字距：docs/checklist-pptx.md 的 Pln1–Pln5。
# 段前段后与行距用百分之一磅；COM SpaceBefore / Font.Spacing 是磅。

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

function Read-PptSpacing($shape) {
    $tf2 = $null
    $tr = $null
    $font = $null
    $pf = $null
    $spacing = 0
    $kern = 0
    $before = 0
    $within = 0
    try {
        $tf2 = $shape.TextFrame2
        $tr = $tf2.TextRange
        $font = $tr.Font
        $pf = $tr.ParagraphFormat
        try { $spacing = [double]$font.Spacing } catch { }
        try { $kern = [double]$font.Kerning } catch { }
        try { $before = [double]$pf.SpaceBefore } catch { }
        try { $within = [double]$pf.SpaceWithin } catch { }
        return @{ Spacing = $spacing; Kerning = $kern; SpaceBefore = $before; SpaceWithin = $within }
    }
    finally {
        Release-Com $pf
        Release-Com $font
        Release-Com $tr
        Release-Com $tf2
    }
}

$hostPath = Ensure-PptxCreate
$ppt = Ensure-MsPpt $hostPath

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-pln-inspect-def'
$sh0 = $tree0.slides[0].shapes[0]
$targetId = [string]$sh0.id

$fmtOut = Get-OutPath 'pptx-shape-spacing-set.pptx'
Close-PptPresByPath $ppt $fmtOut
if (Test-Path $fmtOut) { Remove-Item -Force $fmtOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $fmtOut.Replace('\', '/')
    ops = @(@{
        op = 'setShapeFormat'; id = $targetId
        spcBef = @{ type = 'pts'; val = 1200 }
        lnSpc = @{ type = 'pts'; val = 2400 }
    })
} 'pptx-pln-set-para' | Out-Null
$treeP = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $fmtOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-pln-inspect-para'
$shP = @($treeP.slides[0].shapes) | Where-Object { $_.id -eq $targetId } | Select-Object -First 1
$presP = Open-PptPres $ppt $fmtOut
$capP = Get-PptCaption $presP
$sP = Get-PptSlide $presP 1
$boxP = Get-PptShape $sP ([string]$shP.name)
$spcP = Read-PptSpacing $boxP
Release-Com $boxP
Release-Com $sP
Close-PptPres $presP
$pln1ok = ($capP -notmatch 'Repair') `
    -and ([string]$shP.spcBef.type -eq 'pts') -and ([int64]$shP.spcBef.val -eq 1200) `
    -and ([string]$shP.lnSpc.type -eq 'pts') -and ([int64]$shP.lnSpc.val -eq 2400) `
    -and ([math]::Abs($spcP.SpaceBefore - 12) -lt 0.6)
Add-Check 'pptx-Pln1' $pln1ok "cap=$capP bef=$($shP.spcBef.val) ln=$($shP.lnSpc.val) comBefore=$($spcP.SpaceBefore)"

$runOut = Get-OutPath 'pptx-shape-spacing-run.pptx'
Close-PptPresByPath $ppt $runOut
if (Test-Path $runOut) { Remove-Item -Force $runOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $fmtOut.Replace('\', '/')
    output = $runOut.Replace('\', '/')
    ops = @(@{
        op = 'setShapeFormat'; id = $targetId
        spc = 200; kern = 1200
    })
} 'pptx-pln-set-run' | Out-Null
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $runOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-pln-inspect-run'
$shR = @($treeR.slides[0].shapes) | Where-Object { $_.id -eq $targetId } | Select-Object -First 1
$presR = Open-PptPres $ppt $runOut
$capR = Get-PptCaption $presR
$sR = Get-PptSlide $presR 1
$boxR = Get-PptShape $sR ([string]$shR.name)
$spcR = Read-PptSpacing $boxR
Release-Com $boxR
Release-Com $sR
Close-PptPres $presR
$run0 = $shR.runs[0]
$pln2ok = ($capR -notmatch 'Repair') `
    -and ([int64]$run0.spc -eq 200) -and ([int64]$run0.kern -eq 1200) `
    -and ([math]::Abs($spcR.Spacing - 2) -lt 0.6)
Add-Check 'pptx-Pln2' $pln2ok "cap=$capR spc=$($run0.spc) kern=$($run0.kern) comSpc=$($spcR.Spacing)"

$ancOut = Get-OutPath 'pptx-shape-spacing-anchor.pptx'
Close-PptPresByPath $ppt $ancOut
if (Test-Path $ancOut) { Remove-Item -Force $ancOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $runOut.Replace('\', '/')
    output = $ancOut.Replace('\', '/')
    ops = @(@{ op = 'setShapeFormat'; id = $targetId; textAnchor = 'just' })
} 'pptx-pln-anchor' | Out-Null
$treeA = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ancOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-pln-inspect-anchor'
$shA = @($treeA.slides[0].shapes) | Where-Object { $_.id -eq $targetId } | Select-Object -First 1
$presA = Open-PptPres $ppt $ancOut
$capA = Get-PptCaption $presA
Close-PptPres $presA
$pln3ok = ($capA -notmatch 'Repair') -and ([string]$shA.textAnchor -eq 'just')
Add-Check 'pptx-Pln3' $pln3ok "cap=$capA anchor=$($shA.textAnchor)"

$tblHost = Get-OutPath 'pptx-shape-spacing-table-host.pptx'
Close-PptPresByPath $ppt $tblHost
if (Test-Path $tblHost) { Remove-Item -Force $tblHost }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $tblHost.Replace('\', '/')
    ops = @(@{
        op = 'insertTable'; id = 'slide:1'
        rows = 2; cols = 2
        cells = @(@('格', '乙'), @('丙', '丁'))
    })
} 'pptx-pln-table-ins' | Out-Null
$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tblHost.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1; tables = $true }
} 'pptx-pln-inspect-table'
$cellId = [string]$treeT.slides[0].tables[0].rows[0].cells[0].id
$tblOut = Get-OutPath 'pptx-shape-spacing-table.pptx'
Close-PptPresByPath $ppt $tblOut
if (Test-Path $tblOut) { Remove-Item -Force $tblOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $tblHost.Replace('\', '/')
    output = $tblOut.Replace('\', '/')
    ops = @(@{
        op = 'setShapeFormat'; id = $cellId
        lnSpc = @{ type = 'pts'; val = 2400 }
    })
} 'pptx-pln-table-fmt' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tblOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1; tables = $true }
} 'pptx-pln-inspect-cell'
$cell = $treeC.slides[0].tables[0].rows[0].cells[0]
$presTbl = Open-PptPres $ppt $tblOut
$capTbl = Get-PptCaption $presTbl
Close-PptPres $presTbl
$pln4ok = ($capTbl -notmatch 'Repair') `
    -and ([string]$cell.lnSpc.type -eq 'pts') -and ([int64]$cell.lnSpc.val -eq 2400)
Add-Check 'pptx-Pln4' $pln4ok "cap=$capTbl ln=$($cell.lnSpc.val)"

$failOut = Get-OutPath 'pptx-shape-spacing-fail.pptx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'setShapeFormat'; id = $targetId
        lnSpc = @{ type = 'lines'; val = 1 }
    })
} 'pptx-pln-fail-type'
$codeBad = $null
try { $codeBad = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$pln5ok = ($fail.ExitCode -ne 0) -and ($codeBad -eq 'InvalidField') -and (-not (Test-Path $failOut))
Add-Check 'pptx-Pln5' $pln5ok "exit=$($fail.ExitCode) code=$codeBad wrote=$(Test-Path $failOut)"
