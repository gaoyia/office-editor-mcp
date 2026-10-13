# docx 主题剩余：docs/checklist-docx.md 的 Thr1–Thr6。
# Thr1 样例由 Word 空白另存。不要和 Th、Fm、Ea 撞。不要 Stop-Process Word。

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

$tmpDir = Join-Path $env:TEMP 'ooxp-office'
New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null

$word = Ensure-MsWord (Ensure-DocxCreate)

$src = Get-OutPath 'docx-theme-remain-word-blank.docx'
$tmpBlank = Join-Path $tmpDir 'docx-theme-remain-blank.docx'
if (Test-Path $tmpBlank) { Remove-Item -Force $tmpBlank }
$doc = $word.Documents.Add()
$p = $doc.Paragraphs.Item(1)
$r = $p.Range
$r.Text = 'ThemeRemain'
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
} 'docx-thr-inspect'
$word = Ensure-MsWord (Ensure-DocxCreate)
Close-WordDocByPath $word $src
$doc = Open-WordDoc $word $src
$cap0 = [string]$doc.ActiveWindow.Caption
Close-WordDoc $doc
$fontKeys = @()
if ($null -ne $tree0.theme -and $null -ne $tree0.theme.fonts) {
    $fontKeys = @($tree0.theme.fonts.PSObject.Properties.Name)
}
$minorCsAt = [array]::IndexOf($fontKeys, 'minorCs')
$majorAt = [array]::IndexOf($fontKeys, 'majorScripts')
$minorAt = [array]::IndexOf($fontKeys, 'minorScripts')
$extraIsArr = $false
$majorIsArr = $false
$minorIsArr = $false
if ($null -ne $tree0.theme) {
    $extraIsArr = $tree0.theme.extraSchemes -is [System.Array]
    $majorIsArr = $tree0.theme.fonts.majorScripts -is [System.Array]
    $minorIsArr = $tree0.theme.fonts.minorScripts -is [System.Array]
}
Add-Check 'docx-Thr1' (
    ($cap0 -notmatch 'Repair') -and
    $extraIsArr -and
    $majorIsArr -and
    $minorIsArr -and
    ($minorCsAt -ge 0) -and
    ($majorAt -eq ($minorCsAt + 1)) -and
    ($minorAt -eq ($minorCsAt + 2))
) "caption=$cap0 extraArr=$extraIsArr scripts=$majorAt/$minorAt after minorCs=$minorCsAt keys=$($fontKeys -join ',')"

$nameOut = Get-OutPath 'docx-theme-remain-name.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $nameOut.Replace('\', '/')
    ops = @(
        @{
            op = 'setThemeColors'
            id = 'docx:document'
            name = 'Custom'
            themeName = 'Custom Theme'
        }
    )
} 'docx-thr-name' | Out-Null
$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $nameOut.Replace('\', '/')
} 'docx-thr-name-inspect'
Close-WordDocByPath $word $nameOut
$doc = Open-WordDoc $word $nameOut
$capN = [string]$doc.ActiveWindow.Caption
$comName = $null
try { $comName = [string]$doc.DocumentTheme.Name } catch {
    Write-Log ("COM theme name FAIL " + $_.Exception.Message)
}
Close-WordDoc $doc
$themeName = [string]$treeN.theme.name
$clrName = [string]$treeN.theme.colors.name
Add-Check 'docx-Thr2' (
    ($capN -notmatch 'Repair') -and
    ($themeName -eq 'Custom Theme') -and
    ($clrName -eq 'Custom')
) "caption=$capN theme=$themeName colors=$clrName com=$comName"

$scriptOut = Get-OutPath 'docx-theme-remain-script.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $nameOut.Replace('\', '/')
    output = $scriptOut.Replace('\', '/')
    ops = @(
        @{
            op = 'setThemeFonts'
            id = 'docx:document'
            name = 'CustomFonts'
            majorScripts = @(
                @{ script = 'Jpan'; typeface = 'Yu Mincho' }
            )
        }
    )
} 'docx-thr-script' | Out-Null
$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $scriptOut.Replace('\', '/')
} 'docx-thr-script-inspect'
Close-WordDocByPath $word $scriptOut
$doc = Open-WordDoc $word $scriptOut
$capS = [string]$doc.ActiveWindow.Caption
Close-WordDoc $doc
$fontName = [string]$treeS.theme.fonts.name
$sc0 = $treeS.theme.fonts.majorScripts[0]
Add-Check 'docx-Thr3' (
    ($capS -notmatch 'Repair') -and
    ($fontName -eq 'CustomFonts') -and
    ([string]$sc0.script -eq 'Jpan') -and
    ([string]$sc0.typeface -eq 'Yu Mincho')
) "caption=$capS fonts=$fontName script=$($sc0.script) face=$($sc0.typeface)"

$extraOut = Get-OutPath 'docx-theme-remain-extra.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $scriptOut.Replace('\', '/')
    output = $extraOut.Replace('\', '/')
    ops = @(
        @{
            op = 'setThemeExtraSchemes'
            id = 'docx:document'
            extraSchemes = @(
                @{ name = 'Extra'; colors = @{ accent1 = '0F6FC6' } }
            )
        }
    )
} 'docx-thr-extra' | Out-Null
$treeE = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $extraOut.Replace('\', '/')
} 'docx-thr-extra-inspect'
Close-WordDocByPath $word $extraOut
$doc = Open-WordDoc $word $extraOut
$capE = [string]$doc.ActiveWindow.Caption
Close-WordDoc $doc
$ex0 = $treeE.theme.extraSchemes[0]
$clearOut = Get-OutPath 'docx-theme-remain-extra-clear.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $extraOut.Replace('\', '/')
    output = $clearOut.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeExtraSchemes'; id = 'docx:document'; extraSchemes = $null }
    )
} 'docx-thr-extra-clear' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $clearOut.Replace('\', '/')
} 'docx-thr-extra-clear-inspect'
$cleared = @($treeC.theme.extraSchemes).Count -eq 0
Add-Check 'docx-Thr4' (
    ($capE -notmatch 'Repair') -and
    ([string]$ex0.name -eq 'Extra') -and
    ([string]$ex0.colors.accent1 -eq '0F6FC6') -and
    $cleared
) "caption=$capE extra=$($ex0.name) accent1=$($ex0.colors.accent1) cleared=$cleared"

$langOut = Get-OutPath 'docx-theme-remain-lang.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $langOut.Replace('\', '/')
    ops = @(
        @{
            op = 'setEastAsianSettings'
            id = 'docx:document'
            themeFontLang = @{ val = 'ja-JP' }
        }
    )
} 'docx-thr-lang' | Out-Null
$treeL = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $langOut.Replace('\', '/')
} 'docx-thr-lang-inspect'
Close-WordDocByPath $word $langOut
$doc = Open-WordDoc $word $langOut
$capL = [string]$doc.ActiveWindow.Caption
Close-WordDoc $doc
$langVal = [string]$treeL.eastAsianSettings.themeFontLang.val
Add-Check 'docx-Thr5' (
    ($capL -notmatch 'Repair') -and
    ($langVal -eq 'ja-JP')
) "caption=$capL val=$langVal"

$skel = Get-OutPath 'docx-theme-remain-create.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $skel.Replace('\', '/'); format = 'docx'
    document = @{ paragraphs = @(@{ text = 'no theme' }) }
} 'docx-thr-create' | Out-Null
$failMissing = Get-OutPath 'docx-theme-remain-missing.docx'
$miss = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $skel.Replace('\', '/')
    output = $failMissing.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeColors'; id = 'docx:document'; name = 'Custom' }
    )
} 'docx-thr-missing'
$missExtra = Get-OutPath 'docx-theme-remain-missing-extra.docx'
$miss2 = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $skel.Replace('\', '/')
    output = $missExtra.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeExtraSchemes'; id = 'docx:document'; extraSchemes = @() }
    )
} 'docx-thr-missing-extra'
$xlsxPath = Get-OutPath 'docx-theme-remain.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsxPath.Replace('\', '/')
    format = 'xlsx'
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'docx-thr-xlsx-create' | Out-Null
$failXlsx = Get-OutPath 'docx-theme-remain-xlsx-out.xlsx'
$xlsx = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsxPath.Replace('\', '/')
    output = $failXlsx.Replace('\', '/')
    ops = @(
        @{ op = 'setThemeExtraSchemes'; id = 'xlsx:workbook'; extraSchemes = @() }
    )
} 'docx-thr-xlsx'
Add-Check 'docx-Thr6' (
    ((Get-FailCode $miss) -eq 'IdNotFound') -and
    ((Get-FailCode $miss2) -eq 'IdNotFound') -and
    ((Get-FailCode $xlsx) -eq 'InvalidField') -and
    -not (Test-Path $failMissing) -and
    -not (Test-Path $missExtra) -and
    -not (Test-Path $failXlsx)
) "miss=$(Get-FailCode $miss) extra=$(Get-FailCode $miss2) xlsx=$(Get-FailCode $xlsx)"
