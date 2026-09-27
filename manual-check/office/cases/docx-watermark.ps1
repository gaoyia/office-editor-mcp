# docx 页眉 DrawingML 水印：docs/checklist-docx.md 的 Wm1–Wm5。
# 无页眉种子用 create。Ensure-MsWord 只用 Ensure-DocxCreate，不要锁住样例再读 ZIP。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$wdPrimary = 1

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

function Get-HeaderShapeText($hdr) {
    $parts = @()
    try {
        $n = [int]$hdr.Shapes.Count
        for ($i = 1; $i -le $n; $i++) {
            try {
                $parts += [string]$hdr.Shapes.Item($i).TextFrame.TextRange.Text
            } catch {}
        }
    } catch {}
    return ($parts -join '')
}

$hostPath = Get-OutPath 'docx-watermark-host.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = $S.chapter1 }
            @{ text = $S.hello }
        )
    }
} 'docx-watermark-host' | Out-Null

$png = Ensure-OoxpPngB
$word = Ensure-MsWord (Ensure-DocxCreate)
Close-WordDocByPath $word $hostPath

# Wm1：无页眉文件插入文字水印。
$textOut = Get-OutPath 'docx-watermark-text.docx'
Close-WordDocByPath $word $textOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $textOut.Replace('\', '/')
    ops = @(@{ op = 'insertWatermark'; id = 'docx:document'; text = 'CONFIDENTIAL' })
} 'docx-watermark-insert-text' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $textOut.Replace('\', '/'); format = 'docx'
} 'docx-watermark-inspect-count'
$listed = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $textOut.Replace('\', '/'); format = 'docx'
    scope = @{ watermarks = $true }
} 'docx-watermark-inspect-list'
$wm0 = @($listed.watermarks) | Select-Object -First 1
$wmIdOk = ($null -ne $wm0) -and ([string]$wm0.id -like 'header:default:1/watermark:*')
$kindOk = [string]$wm0.kind -eq 'text'
$docT = Open-WordDoc $word $textOut
$capT = [string]$docT.ActiveWindow.Caption
$hdrT = $docT.Sections.Item(1).Headers.Item($wdPrimary)
$shapeN = 0
try { $shapeN = [int]$hdrT.Shapes.Count } catch { $shapeN = 0 }
$shapeText = Get-HeaderShapeText $hdrT
$hdrRange = ''
try { $hdrRange = [string]$hdrT.Range.Text } catch {}
$sawText = ($shapeText -match 'CONFIDENTIAL') -or ($hdrRange -match 'CONFIDENTIAL')
Close-WordDoc $docT
Add-Check 'docx-Wm1' (
    ([int]$tree0.watermarkCount -eq 1) -and
    $wmIdOk -and $kindOk -and
    ($capT -notmatch 'Repair') -and
    $sawText
) "count=$($tree0.watermarkCount) id=$($wm0.id) kind=$($wm0.kind) shapes=$shapeN saw=$sawText caption=$capT"

# Wm2：整篇删掉。
$offOut = Get-OutPath 'docx-watermark-off.docx'
Close-WordDocByPath $word $offOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $textOut.Replace('\', '/')
    output = $offOut.Replace('\', '/')
    ops = @(@{ op = 'deleteWatermark'; id = 'docx:document' })
} 'docx-watermark-delete-all' | Out-Null
$treeOff = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $offOut.Replace('\', '/'); format = 'docx'
} 'docx-watermark-inspect-off'
$docOff = Open-WordDoc $word $offOut
$capOff = [string]$docOff.ActiveWindow.Caption
$hdrOff = $docOff.Sections.Item(1).Headers.Item($wdPrimary)
$shapeOff = 0
try { $shapeOff = [int]$hdrOff.Shapes.Count } catch { $shapeOff = 0 }
$offText = Get-HeaderShapeText $hdrOff
Close-WordDoc $docOff
Add-Check 'docx-Wm2' (
    ([int]$treeOff.watermarkCount -eq 0) -and
    ($capOff -notmatch 'Repair') -and
    ($offText -notmatch 'CONFIDENTIAL')
) "count=$($treeOff.watermarkCount) shapes=$shapeOff caption=$capOff"

# Wm3：图片水印。
$picOut = Get-OutPath 'docx-watermark-picture.docx'
Close-WordDocByPath $word $picOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $picOut.Replace('\', '/')
    ops = @(@{ op = 'insertWatermark'; id = 'docx:document'; file = $png.Replace('\', '/') })
} 'docx-watermark-insert-pic' | Out-Null
$treePic = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $picOut.Replace('\', '/'); format = 'docx'
    scope = @{ watermarks = $true }
} 'docx-watermark-inspect-pic'
$wmP = @($treePic.watermarks) | Select-Object -First 1
$docP = Open-WordDoc $word $picOut
$capP = [string]$docP.ActiveWindow.Caption
$hdrP = $docP.Sections.Item(1).Headers.Item($wdPrimary)
$shapeP = 0
try { $shapeP = [int]$hdrP.Shapes.Count } catch { $shapeP = 0 }
Close-WordDoc $docP
Add-Check 'docx-Wm3' (
    ([int]$treePic.watermarkCount -eq 1) -and
    ([string]$wmP.kind -eq 'picture') -and
    ($capP -notmatch 'Repair') -and
    ($shapeP -ge 1)
) "count=$($treePic.watermarkCount) kind=$($wmP.kind) shapes=$shapeP caption=$capP"

# Wm4：页眉普通图不算水印。
$hfOut = Get-OutPath 'docx-watermark-hf.docx'
Close-WordDocByPath $word $hfOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $hfOut.Replace('\', '/')
    ops = @(@{ op = 'insertHeaderFooter'; id = 'section:1'; kind = 'header'; type = 'default' })
} 'docx-watermark-insert-hf' | Out-Null
$treeHf = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfOut.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true }
} 'docx-watermark-inspect-hf'
$paraId = [string]$treeHf.headers[0].paragraphs[0].id
$plainOut = Get-OutPath 'docx-watermark-plain-pic.docx'
Close-WordDocByPath $word $plainOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hfOut.Replace('\', '/')
    output = $plainOut.Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = $paraId; file = $png.Replace('\', '/') })
} 'docx-watermark-insert-plain-pic' | Out-Null
$treePlain = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $plainOut.Replace('\', '/'); format = 'docx'
    scope = @{ pictures = $true; watermarks = $true }
} 'docx-watermark-inspect-plain'
$hPlain = @($treePlain.headers) | Where-Object { $_.type -eq 'default' } | Select-Object -First 1
$pics = @($hPlain.pictures)
Add-Check 'docx-Wm4' (
    ([int]$treePlain.watermarkCount -eq 0) -and
    ([int]$hPlain.watermarkCount -eq 0) -and
    ($pics.Count -ge 1)
) "wm=$($treePlain.watermarkCount) pics=$($pics.Count) headerWm=$($hPlain.watermarkCount)"

# Wm5：拒错。不强制开 Word。
$failId = Get-OutPath 'docx-watermark-bad-id.docx'
$failBoth = Get-OutPath 'docx-watermark-both.docx'
if (Test-Path $failId) { Remove-Item -Force $failId }
if (Test-Path $failBoth) { Remove-Item -Force $failBoth }
$badId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failId.Replace('\', '/')
    ops = @(@{ op = 'insertWatermark'; id = 'para:00000001'; text = 'CONFIDENTIAL' })
} 'docx-watermark-bad-id'
$badBoth = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failBoth.Replace('\', '/')
    ops = @(@{
        op = 'insertWatermark'; id = 'docx:document'
        text = 'CONFIDENTIAL'; file = $png.Replace('\', '/')
    })
} 'docx-watermark-both'
$codeId = $null
$codeBoth = $null
try { $codeId = ($badId.Stderr | ConvertFrom-Json).error.code } catch {}
try { $codeBoth = ($badBoth.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'docx-Wm5' (
    ($codeId -eq 'InvalidId') -and
    ($codeBoth -eq 'InvalidField') -and
    (-not (Test-Path $failId)) -and
    (-not (Test-Path $failBoth))
) "id=$codeId both=$codeBoth wroteId=$([bool](Test-Path $failId)) wroteBoth=$([bool](Test-Path $failBoth))"
