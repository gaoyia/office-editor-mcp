# docx 主题与页面背景剩余：docs/checklist-docx.md 的 Fm1–Fm7。
# Fm1 样例由 Word 空白另存。不要用默认 create 产物当 Fm1 唯一样例。

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

$tmpDir = Join-Path $env:TEMP 'ooxp-office'
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null

$word = Ensure-MsWord (Ensure-DocxCreate)

$src = Get-OutPath 'docx-theme-bg-word-blank.docx'
$tmpBlank = Join-Path $tmpDir 'docx-theme-bg-blank.docx'
if (Test-Path $tmpBlank) { Remove-Item -Force $tmpBlank }
$doc = $word.Documents.Add()
$p = $doc.Paragraphs.Item(1)
$r = $p.Range
$r.Text = 'ThemeBgHost'
Release-Com $r
Release-Com $p
$wdFormatOpenXMLDocument = 16
$doc.SaveAs($tmpBlank, $wdFormatOpenXMLDocument)
Close-WordDoc $doc
Copy-Item -Force $tmpBlank $src
Write-Log "saved word blank $src"

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/')
} 'docx-fm-inspect'
$word = Ensure-MsWord (Ensure-DocxCreate)
Close-WordDocByPath $word $src
$doc = Open-WordDoc $word $src
$cap0 = [string]$doc.ActiveWindow.Caption
$fmtName = [string]$tree0.theme.fmt.name
$fillN = @($tree0.theme.fmt.fills).Count
$lineN = @($tree0.theme.fmt.lines).Count
$ovNull = $null -eq $tree0.themeOverride
Close-WordDoc $doc
Add-Check 'docx-Fm1' (
    ($fmtName -eq 'Office') -and ($fillN -eq 3) -and ($lineN -eq 3) -and $ovNull -and ($cap0 -notmatch 'Repair')
) "caption=$cap0 fmt=$fmtName fills=$fillN lines=$lineN ov=$($tree0.themeOverride)"

$fmtOut = Get-OutPath 'docx-theme-bg-fmt.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $fmtOut.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeFmt'; id = 'docx:document'; line = @{ index = 0; w = 38100 } }
    )
} 'docx-fm-fmt' | Out-Null
$treeF = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $fmtOut.Replace('\', '/')
} 'docx-fm-fmt-inspect'
Close-WordDocByPath $word $fmtOut
$doc = Open-WordDoc $word $fmtOut
$capF = [string]$doc.ActiveWindow.Caption
Close-WordDoc $doc
$w0 = [int64]$treeF.theme.fmt.lines[0].w
Add-Check 'docx-Fm2' (($w0 -eq 38100) -and ($capF -notmatch 'Repair')) "w=$w0 caption=$capF"

$ovOut = Get-OutPath 'docx-theme-bg-override.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $ovOut.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeOverride'; id = 'docx:document'; accent1 = 'FF0000' }
    )
} 'docx-fm-override' | Out-Null
$treeO = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ovOut.Replace('\', '/')
} 'docx-fm-override-inspect'
Close-WordDocByPath $word $ovOut
$doc = Open-WordDoc $word $ovOut
$capO = [string]$doc.ActiveWindow.Caption
Close-WordDoc $doc
$accentO = [string]$treeO.themeOverride.colors.accent1

$ovOff = Get-OutPath 'docx-theme-bg-override-off.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $ovOut.Replace('\', '/')
    output = $ovOff.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeOverride'; id = 'docx:document'; enabled = $false }
    )
} 'docx-fm-override-off' | Out-Null
$treeOff = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ovOff.Replace('\', '/')
} 'docx-fm-override-off-inspect'
Close-WordDocByPath $word $ovOff
$doc = Open-WordDoc $word $ovOff
$capOff = [string]$doc.ActiveWindow.Caption
Close-WordDoc $doc
$offNull = $null -eq $treeOff.themeOverride
Add-Check 'docx-Fm3' (
    ($accentO -eq 'FF0000') -and ($capO -notmatch 'Repair') -and $offNull -and ($capOff -notmatch 'Repair')
) "accent1=$accentO capO=$capO off=$($treeOff.themeOverride) capOff=$capOff"

$createSrc = Get-OutPath 'docx-theme-bg-create-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $createSrc.Replace('\', '/')
    format = 'docx'
    document = @{ paragraphs = @(@{ text = 'hello' }) }
} 'docx-fm-create-src' | Out-Null

$artOut = Get-OutPath 'docx-theme-bg-apples.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $createSrc.Replace('\', '/')
    output = $artOut.Replace('\', '/')
    ops = @(
        @{
            op = 'setPageBorders'; id = 'section:1'
            top = @{ val = 'apples'; sz = 20; space = 24; color = 'auto' }
            left = @{ val = 'apples'; sz = 20; space = 24; color = 'auto' }
            bottom = @{ val = 'apples'; sz = 20; space = 24; color = 'auto' }
            right = @{ val = 'apples'; sz = 20; space = 24; color = 'auto' }
        }
    )
} 'docx-fm-apples' | Out-Null
$treeA = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $artOut.Replace('\', '/')
} 'docx-fm-apples-inspect'
Close-WordDocByPath $word $artOut
$doc = Open-WordDoc $word $artOut
$capA = [string]$doc.ActiveWindow.Caption
Close-WordDoc $doc
$valA = [string]$treeA.sections[0].pageBorders.top.val
Add-Check 'docx-Fm4' (($valA -eq 'apples') -and ($capA -notmatch 'Repair')) "val=$valA caption=$capA"

$gradOut = Get-OutPath 'docx-theme-bg-grad.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $createSrc.Replace('\', '/')
    output = $gradOut.Replace('\', '/')
    ops = @(
        @{
            op = 'setPageBackground'; id = 'docx:document'
            color = 'FF0000'
            fill = @{ kind = 'gradient'; color = '0000FF' }
        }
    )
} 'docx-fm-grad' | Out-Null
$treeG = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $gradOut.Replace('\', '/')
} 'docx-fm-grad-inspect'
Close-WordDocByPath $word $gradOut
$doc = Open-WordDoc $word $gradOut
$capG = [string]$doc.ActiveWindow.Caption
Close-WordDoc $doc
$kindG = [string]$treeG.pageBackground.fill.kind
Add-Check 'docx-Fm5' (($kindG -eq 'drawing') -and ($capG -notmatch 'Repair')) "kind=$kindG caption=$capG"

$created = Get-OutPath 'docx-theme-bg-create.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $created.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(@{ text = 'hello' })
        theme = @{
            name = 'Office Theme'
            colors = @{ accent1 = 'FF0000' }
        }
        pageBackground = @{ color = 'EEEEEE' }
    }
} 'docx-fm-create' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $created.Replace('\', '/')
} 'docx-fm-create-inspect'
Close-WordDocByPath $word $created
$doc = Open-WordDoc $word $created
$capC = [string]$doc.ActiveWindow.Caption
Close-WordDoc $doc
$treeBare = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $createSrc.Replace('\', '/')
} 'docx-fm-bare-inspect'
$hasTheme = $null -ne $treeC.theme -and $null -ne $treeC.theme.colors
$accentC = [string]$treeC.theme.colors.accent1
$bgC = [string]$treeC.pageBackground.color
$bareNull = $null -eq $treeBare.theme
Add-Check 'docx-Fm6' (
    $hasTheme -and ($accentC -eq 'FF0000') -and ($bgC -eq 'EEEEEE') -and $bareNull -and ($capC -notmatch 'Repair')
) "accent1=$accentC bg=$bgC bare=$($treeBare.theme) caption=$capC"

$failBad = Get-OutPath 'docx-theme-bg-bad.docx'
$badId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failBad.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeFmt'; id = 'para:index:1'; line = @{ index = 0; w = 38100 } }
    )
} 'docx-fm-bad-id'
$xlsx = Get-OutPath 'docx-theme-bg-xlsx.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-fm-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-theme-bg-fail-xlsx.xlsx'
if (Test-Path $failX) { Remove-Item -Force $failX }
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'setThemeOverride'; id = 'slide:1'; accent1 = 'FF0000' })
} 'docx-fm-fail-xlsx'
$codeId = $null
$codeX = $null
try { $codeId = ($badId.Stderr | ConvertFrom-Json).error.code } catch {}
try { $codeX = ($rawX.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'docx-Fm7' (
    ($codeId -eq 'InvalidId') -and ($codeX -eq 'InvalidId') -and
    (-not (Test-Path $failBad)) -and (-not (Test-Path $failX))
) "id=$codeId xlsx=$codeX wrote=$([bool](Test-Path $failBad))"
