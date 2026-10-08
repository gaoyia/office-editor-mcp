# docx 主题：docs/checklist-docx.md 的 Th1–Th5。
# Th1 样例由 Word 空白另存。不要用 create 产物当 Th1 唯一样例。

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

$src = Get-OutPath 'docx-theme-word-blank.docx'
$tmpBlank = Join-Path $tmpDir 'docx-theme-blank.docx'
if (Test-Path $tmpBlank) { Remove-Item -Force $tmpBlank }
$doc = $word.Documents.Add()
$p = $doc.Paragraphs.Item(1)
$r = $p.Range
$r.Text = 'ThemeHost'
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
} 'docx-th-inspect'
$word = Ensure-MsWord (Ensure-DocxCreate)
Close-WordDocByPath $word $src
$doc = Open-WordDoc $word $src
$cap0 = [string]$doc.ActiveWindow.Caption
$hasTheme = $null -ne $tree0.theme -and $null -ne $tree0.theme.colors
$accent0 = [string]$tree0.theme.colors.accent1
Close-WordDoc $doc
Add-Check 'docx-Th1' ($hasTheme -and ($accent0.Length -eq 6) -and ($cap0 -notmatch 'Repair')) "caption=$cap0 accent1=$accent0"

$colorOut = Get-OutPath 'docx-theme-color.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $colorOut.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeColors'; id = 'docx:document'; accent1 = 'FF0000' }
    )
} 'docx-th-color' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $colorOut.Replace('\', '/')
} 'docx-th-color-inspect'
Close-WordDocByPath $word $colorOut
$doc = Open-WordDoc $word $colorOut
$capC = [string]$doc.ActiveWindow.Caption
$rgb = $null
try {
    $rgb = [int]$doc.DocumentTheme.ThemeColorScheme.Colors(5).RGB
} catch {
    Write-Log ("COM accent1 RGB FAIL " + $_.Exception.Message)
}
Close-WordDoc $doc
$accentC = [string]$treeC.theme.colors.accent1
Add-Check 'docx-Th2' (($capC -notmatch 'Repair') -and ($accentC -eq 'FF0000') -and ($rgb -eq 255)) "caption=$capC accent1=$accentC rgb=$rgb"

$fontOut = Get-OutPath 'docx-theme-font.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $colorOut.Replace('\', '/')
    output = $fontOut.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeFonts'; id = 'docx:document'; majorLatin = 'Arial' }
    )
} 'docx-th-font' | Out-Null
$treeF = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $fontOut.Replace('\', '/')
} 'docx-th-font-inspect'
Close-WordDocByPath $word $fontOut
$doc = Open-WordDoc $word $fontOut
$capF = [string]$doc.ActiveWindow.Caption
$face = $null
try {
    $face = [string]$doc.DocumentTheme.ThemeFontScheme.MajorFont.Item(1).Name
} catch {
    Write-Log ("COM major latin FAIL " + $_.Exception.Message)
}
Close-WordDoc $doc
$latin = [string]$treeF.theme.fonts.majorLatin
Add-Check 'docx-Th3' (($capF -notmatch 'Repair') -and ($latin -eq 'Arial') -and ($face -eq 'Arial')) "caption=$capF latin=$latin com=$face"

$skel = Get-OutPath 'docx-theme-create.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $skel.Replace('\', '/'); format = 'docx'
    document = @{ paragraphs = @(@{ text = 'no theme' }) }
} 'docx-th-create' | Out-Null
$failMissing = Get-OutPath 'docx-theme-missing.docx'
$miss = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $skel.Replace('\', '/')
    output = $failMissing.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeColors'; id = 'docx:document'; accent1 = 'FF0000' }
    )
} 'docx-th-missing'
$codeMiss = $null
try { $codeMiss = ($miss.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'docx-Th4' (($codeMiss -eq 'IdNotFound') -and -not (Test-Path $failMissing)) "code=$codeMiss wrote=$([bool](Test-Path $failMissing))"

$failBad = Get-OutPath 'docx-theme-bad.docx'
$badId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failBad.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeColors'; id = 'para:index:1'; accent1 = 'FF0000' }
    )
} 'docx-th-bad-id'
$empty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failBad.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeColors'; id = 'docx:document' }
    )
} 'docx-th-empty'
$codeId = $null
$codeEmpty = $null
try { $codeId = ($badId.Stderr | ConvertFrom-Json).error.code } catch {}
try { $codeEmpty = ($empty.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'docx-Th5' (($codeId -eq 'InvalidId') -and ($codeEmpty -eq 'InvalidField') -and -not (Test-Path $failBad)) "id=$codeId empty=$codeEmpty wrote=$([bool](Test-Path $failBad))"
