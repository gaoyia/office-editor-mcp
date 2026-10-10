# docx 框内域、书签与公式控件：docs/checklist-docx.md 的 Bf1–Bf5。
# COM 读形状 TextFrame.TextRange.Hyperlinks。编号 Bf，不要和 Bb、Bx、Bi、Fd 撞。

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

function Get-BoxHyperlinkCount($doc) {
    $n = 0
    $shapes = $null
    try {
        $shapes = $doc.Shapes
        $sc = [int]$shapes.Count
        for ($i = 1; $i -le $sc; $i++) {
            $sh = $null
            $tf = $null
            $tr = $null
            $links = $null
            try {
                $sh = $shapes.Item($i)
                $tf = $sh.TextFrame
                $tr = $tf.TextRange
                $links = $tr.Hyperlinks
                $n += [int]$links.Count
            } catch { }
            finally {
                Release-Com $links
                Release-Com $tr
                Release-Com $tf
                Release-Com $sh
            }
        }
    } catch { }
    finally {
        Release-Com $shapes
    }
    return $n
}

$src = Get-OutPath 'docx-box-fields-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(@{
            text = '外'
            textBoxes = @(@{ text = '框' })
        })
    }
} 'docx-box-fields-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true; textBoxContent = $true }
} 'docx-box-fields-inspect-src'
$box0 = $tree0.paragraphs[0].textBoxes[0]
$boxId = [string]$box0.id
$inner = [string]$box0.paragraphs[0].id
$root0 = [int]$tree0.hyperlinkCount

$ins = Get-OutPath 'docx-box-fields-link.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $ins.Replace('\', '/')
    ops = @(@{
        op = 'insertHyperlink'; id = $inner
        text = '例'; url = 'https://example.com'
    })
} 'docx-box-fields-insert-link' | Out-Null

$listed = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ins.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true; hyperlinks = $true }
} 'docx-box-fields-inspect-link'
$boxL = $listed.paragraphs[0].textBoxes[0]
$l1 = @($boxL.hyperlinks) | Where-Object { [string]$_.id -eq "$boxId/link:1" } | Select-Object -First 1
$rootSame = ([int]$listed.hyperlinkCount -eq $root0)
$bf1inspect = ($null -ne $boxL.hyperlinkCount) -and ([int]$boxL.hyperlinkCount -eq 1) -and $rootSame -and ($null -ne $l1)

$word = Ensure-MsWord $ins
$docL = Open-WordDoc $word $ins
$capL = [string]$docL.ActiveWindow.Caption
$comL = Get-BoxHyperlinkCount $docL
Close-WordDoc $docL
$bf1ok = $bf1inspect -and ($capL -notmatch 'Repair') -and ($capL -match 'docx-box-fields-link') -and ($comL -ge 1)
Add-Check 'docx-Bf1' $bf1ok "root=$($listed.hyperlinkCount) boxCount=$($boxL.hyperlinkCount) id=$($l1.id) caption=$capL comLinks=$comL"

$bm = Get-OutPath 'docx-box-fields-bm.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $ins.Replace('\', '/')
    output = $bm.Replace('\', '/')
    ops = @(
        @{ op = 'insertBookmark'; id = $inner; name = 'BoxBm' }
        @{ op = 'insertField'; id = $inner; code = 'DATE' }
    )
} 'docx-box-fields-bm-field' | Out-Null

$treeB = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $bm.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true; bookmarks = $true; fields = $true }
} 'docx-box-fields-inspect-bm'
$boxB = $treeB.paragraphs[0].textBoxes[0]
$bm1 = @($boxB.bookmarks) | Where-Object { [string]$_.id -eq "$boxId/bookmark:1" } | Select-Object -First 1
$fd1 = @($boxB.fields) | Where-Object { [string]$_.id -eq "$boxId/field:1" } | Select-Object -First 1
$wordB = Ensure-MsWord $bm
$docB = Open-WordDoc $wordB $bm
$capB = [string]$docB.ActiveWindow.Caption
Close-WordDoc $docB
$bf2ok = ([int]$boxB.bookmarkCount -eq 1) -and ([int]$boxB.fieldCount -eq 1) -and ($null -ne $bm1) -and ($null -ne $fd1) -and ($capB -notmatch 'Repair') -and ($capB -match 'docx-box-fields-bm')
Add-Check 'docx-Bf2' $bf2ok "bm=$($boxB.bookmarkCount) field=$($boxB.fieldCount) caption=$capB"

$misc = Get-OutPath 'docx-box-fields-misc.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $misc.Replace('\', '/')
    ops = @(
        @{ op = 'insertMath'; id = $inner; text = 'x' }
        @{ op = 'insertRuby'; id = $inner; base = '汉'; rt = 'hàn' }
        @{ op = 'insertContentControl'; id = $inner; kind = 'plainText'; text = '控' }
    )
} 'docx-box-fields-misc' | Out-Null

$treeM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $misc.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true; maths = $true; rubies = $true; contentControls = $true }
} 'docx-box-fields-inspect-misc'
$boxM = $treeM.paragraphs[0].textBoxes[0]
$wordM = Ensure-MsWord $misc
$docM = Open-WordDoc $wordM $misc
$capM = [string]$docM.ActiveWindow.Caption
Close-WordDoc $docM
$bf3ok = ([int]$boxM.mathCount -eq 1) -and ([int]$boxM.rubyCount -eq 1) -and ([int]$boxM.contentControlCount -eq 1) -and ($capM -notmatch 'Repair') -and ($capM -match 'docx-box-fields-misc')
Add-Check 'docx-Bf3' $bf3ok "math=$($boxM.mathCount) ruby=$($boxM.rubyCount) sdt=$($boxM.contentControlCount) caption=$capM"

$failText = Get-OutPath 'docx-box-fields-text-fail.docx'
if (Test-Path $failText) { Remove-Item -Force $failText }
$rawText = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $ins.Replace('\', '/')
    output = $failText.Replace('\', '/')
    ops = @(@{ op = 'setParaText'; id = $inner; text = '改' })
} 'docx-box-fields-set-text'
$failTextCode = $null
try { $failTextCode = ($rawText.Stderr | ConvertFrom-Json).error.code } catch {}
$noWriteText = -not (Test-Path $failText)
$bf4ok = ($rawText.ExitCode -ne 0) -and ($failTextCode -eq 'NotImplemented') -and $noWriteText
Add-Check 'docx-Bf4' $bf4ok "code=$failTextCode exit=$($rawText.ExitCode) wrote=$(-not $noWriteText)"

$fnSrc = Get-OutPath 'docx-box-fields-fn.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $fnSrc.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(@{ text = '正文'; footnote = '注文' })
    }
} 'docx-box-fields-fn-create' | Out-Null
$treeFn = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $fnSrc.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; notes = $true }
} 'docx-box-fields-fn-inspect'
$fnPara = [string]$treeFn.footnotes[0].paragraphs[0].id
$failOut = Get-OutPath 'docx-box-fields-fn-fail.docx'
if (Test-Path $failOut) { Remove-Item -Force $failOut }
$raw = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $fnSrc.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'insertField'; id = $fnPara; code = 'DATE' })
} 'docx-box-fields-fn-fail'
$failCode = $null
try { $failCode = ($raw.Stderr | ConvertFrom-Json).error.code } catch {}
$noWrite = -not (Test-Path $failOut)
$bf5ok = ($raw.ExitCode -ne 0) -and ($failCode -eq 'InvalidId') -and $noWrite
Add-Check 'docx-Bf5' $bf5ok "code=$failCode exit=$($raw.ExitCode) wrote=$(-not $noWrite)"
