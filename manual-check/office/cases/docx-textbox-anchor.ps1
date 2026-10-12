# docx 文本框内垂直锚点：docs/checklist-docx.md 的 Bta1–Bta5。
# 编号 Bta，不要和 Tb、T、Bx 撞。不要 $pid。

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

$src = Get-OutPath 'docx-textbox-anchor-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(@{
            text = '外'
            textBoxes = @(@{ text = '框内' })
        })
    }
} 'docx-textbox-anchor-create-src' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true }
} 'docx-textbox-anchor-inspect-src'
$box0 = $tree0.paragraphs[0].textBoxes[0]
$boxId = [string]$box0.id
$bta1ok = ($box0.PSObject.Properties.Name -contains 'textAnchor') -and ($null -eq $box0.textAnchor)
Add-Check 'docx-Bta1' $bta1ok "id=$boxId textAnchor=$($box0.textAnchor)"

$applyOut = Get-OutPath 'docx-textbox-anchor-apply.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $applyOut.Replace('\', '/')
    ops = @(@{ op = 'setTextBoxTextAnchor'; id = $boxId; textAnchor = 'ctr' })
} 'docx-textbox-anchor-apply' | Out-Null

$treeA = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $applyOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true }
} 'docx-textbox-anchor-inspect-apply'
$xmlA = Get-ZipPartText $applyOut 'word/document.xml'
$bta2tree = ([string]$treeA.paragraphs[0].textBoxes[0].textAnchor -eq 'ctr') -and ($xmlA -match 'anchor="ctr"')

$clearOut = Get-OutPath 'docx-textbox-anchor-clear.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $applyOut.Replace('\', '/')
    output = $clearOut.Replace('\', '/')
    ops = @(@{ op = 'setTextBoxTextAnchor'; id = $boxId; textAnchor = $null })
} 'docx-textbox-anchor-clear' | Out-Null

$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $clearOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true }
} 'docx-textbox-anchor-inspect-clear'
$xmlN = Get-ZipPartText $clearOut 'word/document.xml'
$bta3ok = ($null -eq $treeN.paragraphs[0].textBoxes[0].textAnchor) -and ($xmlN -notmatch 'anchor=')
Add-Check 'docx-Bta3' $bta3ok "tree=$($treeN.paragraphs[0].textBoxes[0].textAnchor)"

$justOut = Get-OutPath 'docx-textbox-anchor-just.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $justOut.Replace('\', '/')
    ops = @(@{ op = 'setTextBoxTextAnchor'; id = $boxId; textAnchor = 'just' })
} 'docx-textbox-anchor-just' | Out-Null
$distOut = Get-OutPath 'docx-textbox-anchor-dist.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $distOut.Replace('\', '/')
    ops = @(@{ op = 'setTextBoxTextAnchor'; id = $boxId; textAnchor = 'dist' })
} 'docx-textbox-anchor-dist' | Out-Null
$treeJ = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $justOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true }
} 'docx-textbox-anchor-inspect-just'
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $distOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true }
} 'docx-textbox-anchor-inspect-dist'
$bta4tree = ([string]$treeJ.paragraphs[0].textBoxes[0].textAnchor -eq 'just') -and
    ([string]$treeD.paragraphs[0].textBoxes[0].textAnchor -eq 'dist')

$failPara = Get-OutPath 'docx-textbox-anchor-fail-para.docx'
$rawP = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failPara.Replace('\', '/')
    ops = @(@{ op = 'setTextBoxTextAnchor'; id = 'para:nope'; textAnchor = 'ctr' })
} 'docx-textbox-anchor-fail-para'
$xlsx = Get-OutPath 'docx-textbox-anchor.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-textbox-anchor-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-textbox-anchor-fail-xlsx.xlsx'
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'setTextBoxTextAnchor'; id = 'sheet:Sheet1/sp:1'; textAnchor = 'ctr' })
} 'docx-textbox-anchor-fail-xlsx'
Add-Check 'docx-Bta5' (
    ((Get-FailCode $rawP) -eq 'InvalidId') -and
    ((Get-FailCode $rawX) -eq 'InvalidField')
) "para=$(Get-FailCode $rawP) xlsx=$(Get-FailCode $rawX)"

$seed = Ensure-DocxCreate
$word = Ensure-MsWord $seed
try { $word.AskToUpdateLinks = $false } catch {}
try { $word.Options.UpdateLinksAtOpen = $false } catch {}

Close-WordDocByPath $word $applyOut
$docA = Open-WordDoc $word $applyOut
$capA = [string]$docA.ActiveWindow.Caption
Close-WordDoc $docA
Add-Check 'docx-Bta2' (
    $bta2tree -and ($capA -notmatch 'Repair') -and ($capA -match 'docx-textbox-anchor-apply')
) "textAnchor=$($treeA.paragraphs[0].textBoxes[0].textAnchor) caption=$capA"

Close-WordDocByPath $word $justOut
$docJ = Open-WordDoc $word $justOut
$capJ = [string]$docJ.ActiveWindow.Caption
Close-WordDoc $docJ
Close-WordDocByPath $word $distOut
$docD = Open-WordDoc $word $distOut
$capD = [string]$docD.ActiveWindow.Caption
Close-WordDoc $docD
Add-Check 'docx-Bta4' (
    $bta4tree -and ($capJ -notmatch 'Repair') -and ($capD -notmatch 'Repair') -and
    ($capJ -match 'docx-textbox-anchor-just') -and ($capD -match 'docx-textbox-anchor-dist')
) "just=$($treeJ.paragraphs[0].textBoxes[0].textAnchor) dist=$($treeD.paragraphs[0].textBoxes[0].textAnchor) capJ=$capJ capD=$capD"
