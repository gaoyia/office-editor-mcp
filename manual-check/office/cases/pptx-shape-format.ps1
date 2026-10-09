# pptx 形状文字格式：docs/checklist-pptx.md 的 Tf1–Tf5。
# 字号、字色、加粗、对齐。COM Font.Size 是磅；Color.RGB 是 OLE 色（R + G×256 + B×65536）。

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

function Convert-SrgbToOleRgb([string]$hex) {
    $r = [Convert]::ToInt32($hex.Substring(0, 2), 16)
    $g = [Convert]::ToInt32($hex.Substring(2, 2), 16)
    $b = [Convert]::ToInt32($hex.Substring(4, 2), 16)
    return [int]($r + ($g * 256) + ($b * 65536))
}

function Read-PptFont($shape) {
    $frame = $null
    $range = $null
    $font = $null
    $color = $null
    try {
        $frame = $shape.TextFrame
        $range = $frame.TextRange
        $font = $range.Font
        $size = [double]$font.Size
        $bold = $font.Bold
        $italic = $font.Italic
        $color = $font.Color
        $rgb = [int]$color.RGB
        return @{ Size = $size; Bold = $bold; Italic = $italic; Rgb = $rgb }
    }
    finally {
        Release-Com $color
        Release-Com $font
        Release-Com $range
        Release-Com $frame
    }
}

$hostPath = Ensure-PptxCreate
$ppt = Ensure-MsPpt $hostPath

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-tf-inspect-def'
$sh0 = $tree0.slides[0].shapes[0]
$pres0 = Open-PptPres $ppt $hostPath
$cap0 = Get-PptCaption $pres0
Close-PptPres $pres0
$tf1ok = ($cap0 -notmatch 'Repair') `
    -and ([int64]$sh0.sz -eq 1800) `
    -and ([string]$sh0.color -eq '000000') `
    -and ([string]$sh0.font -eq 'Calibri') `
    -and ([string]$sh0.eastAsiaFont -eq '宋体')
Add-Check 'pptx-Tf1' $tf1ok "cap=$cap0 sz=$($sh0.sz) color=$($sh0.color) font=$($sh0.font) ea=$($sh0.eastAsiaFont)"

$fmtOut = Get-OutPath 'pptx-shape-format-set.pptx'
Close-PptPresByPath $ppt $fmtOut
if (Test-Path $fmtOut) { Remove-Item -Force $fmtOut }
$targetId = [string]$sh0.id
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $fmtOut.Replace('\', '/')
    ops = @(@{
        op = 'setShapeFormat'; id = $targetId
        sz = 3600; color = 'C41E3A'; bold = $true
        align = 'ctr'; textAnchor = 'ctr'
    })
} 'pptx-tf-set' | Out-Null
$treeF = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $fmtOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-tf-inspect-set'
$shF = @($treeF.slides[0].shapes) | Where-Object { $_.id -eq $targetId } | Select-Object -First 1
$presF = Open-PptPres $ppt $fmtOut
$capF = Get-PptCaption $presF
$sF = Get-PptSlide $presF 1
$boxF = Get-PptShape $sF ([string]$shF.name)
$fontF = Read-PptFont $boxF
Release-Com $boxF
Release-Com $sF
Close-PptPres $presF
$wantRgb = Convert-SrgbToOleRgb 'C41E3A'
$inspOk = ([int64]$shF.sz -eq 3600) -and ([string]$shF.color -eq 'C41E3A') `
    -and ([bool]$shF.bold) -and ([string]$shF.align -eq 'ctr') -and ([string]$shF.textAnchor -eq 'ctr')
$comOk = ([math]::Abs($fontF.Size - 36) -lt 0.6) `
    -and (($fontF.Bold -eq -1) -or ($fontF.Bold -eq $true)) `
    -and ($fontF.Rgb -eq $wantRgb)
$tf2ok = ($capF -notmatch 'Repair') -and $inspOk -and $comOk
Add-Check 'pptx-Tf2' $tf2ok "cap=$capF sz=$($shF.sz) color=$($shF.color) size=$($fontF.Size) rgb=$($fontF.Rgb) want=$wantRgb"

$insOut = Get-OutPath 'pptx-shape-format-insert.pptx'
Close-PptPresByPath $ppt $insOut
if (Test-Path $insOut) { Remove-Item -Force $insOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $fmtOut.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{
        op = 'insertShape'; id = 'slide:1'; text = '新框'
        sz = 2800; color = '0000FF'; italic = $true
        font = 'Arial'; eastAsiaFont = '黑体'
    })
} 'pptx-tf-insert' | Out-Null
$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-tf-inspect-insert'
$neu = @($treeI.slides[0].shapes) | Where-Object { $_.text -eq '新框' } | Select-Object -First 1
$presI = Open-PptPres $ppt $insOut
$capI = Get-PptCaption $presI
$sI = Get-PptSlide $presI 1
$boxI = Get-PptShape $sI ([string]$neu.name)
$fontI = Read-PptFont $boxI
Release-Com $boxI
Release-Com $sI
Close-PptPres $presI
$wantBlue = Convert-SrgbToOleRgb '0000FF'
$tf3ok = ($capI -notmatch 'Repair') `
    -and ([int64]$neu.sz -eq 2800) `
    -and ([string]$neu.color -eq '0000FF') `
    -and ([bool]$neu.italic) `
    -and ([string]$neu.font -eq 'Arial') `
    -and ([math]::Abs($fontI.Size - 28) -lt 0.6) `
    -and ($fontI.Rgb -eq $wantBlue)
Add-Check 'pptx-Tf3' $tf3ok "cap=$capI sz=$($neu.sz) color=$($neu.color) size=$($fontI.Size) rgb=$($fontI.Rgb)"

$tblHost = Get-OutPath 'pptx-shape-format-table-host.pptx'
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
} 'pptx-tf-table-ins' | Out-Null
$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tblHost.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1; tables = $true }
} 'pptx-tf-inspect-table'
$cellId = [string]$treeT.slides[0].tables[0].rows[0].cells[0].id
$tblOut = Get-OutPath 'pptx-shape-format-table.pptx'
Close-PptPresByPath $ppt $tblOut
if (Test-Path $tblOut) { Remove-Item -Force $tblOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $tblHost.Replace('\', '/')
    output = $tblOut.Replace('\', '/')
    ops = @(@{ op = 'setShapeFormat'; id = $cellId; sz = 2000; color = 'FF0000' })
} 'pptx-tf-table-fmt' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $tblOut.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1; tables = $true }
} 'pptx-tf-inspect-cell'
$cell = $treeC.slides[0].tables[0].rows[0].cells[0]
$presTbl = Open-PptPres $ppt $tblOut
$capTbl = Get-PptCaption $presTbl
Close-PptPres $presTbl
$tf4ok = ($capTbl -notmatch 'Repair') `
    -and ([string]$cell.text -eq '格') `
    -and ([int64]$cell.sz -eq 2000) `
    -and ([string]$cell.color -eq 'FF0000')
Add-Check 'pptx-Tf4' $tf4ok "cap=$capTbl sz=$($cell.sz) color=$($cell.color) text=$($cell.text)"

$failEmpty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = (Get-OutPath 'pptx-shape-format-fail-empty.pptx').Replace('\', '/')
    ops = @(@{ op = 'setShapeFormat'; id = $targetId })
} 'pptx-tf-fail-empty'
$failSz = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = (Get-OutPath 'pptx-shape-format-fail-sz.pptx').Replace('\', '/')
    ops = @(@{ op = 'setShapeFormat'; id = $targetId; sz = 1 })
} 'pptx-tf-fail-sz'
$failEmptyOut = Get-OutPath 'pptx-shape-format-fail-empty.pptx'
$failSzOut = Get-OutPath 'pptx-shape-format-fail-sz.pptx'
$codeEmpty = $null
$codeSz = $null
try { $codeEmpty = ($failEmpty.Stderr | ConvertFrom-Json).error.code } catch {}
try { $codeSz = ($failSz.Stderr | ConvertFrom-Json).error.code } catch {}
$tf5ok = ($failEmpty.ExitCode -ne 0) -and ($failSz.ExitCode -ne 0) `
    -and ($codeEmpty -eq 'InvalidField') -and ($codeSz -eq 'InvalidField') `
    -and (-not (Test-Path $failEmptyOut)) -and (-not (Test-Path $failSzOut))
Add-Check 'pptx-Tf5' $tf5ok "empty=$($failEmpty.ExitCode)/$codeEmpty sz=$($failSz.ExitCode)/$codeSz"
