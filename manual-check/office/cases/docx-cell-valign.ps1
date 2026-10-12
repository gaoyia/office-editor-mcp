# docx 单元格垂直对齐：docs/checklist-docx.md 的 Va1–Va5。
# 编号 Va，不要和 Tf、Tw、T、Bx 撞。不要 $pid。

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

$src = Get-OutPath 'docx-cell-valign-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(@{ text = '前' })
        tables = @(@{
            cells = @(
                @('A1', 'B1'),
                @('A2', 'B2')
            )
        })
    }
} 'docx-cell-valign-create-src' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-cell-valign-inspect-src'
$c0 = $tree0.tables[0].rows[0].cells[0]
$va1ok = ($c0.PSObject.Properties.Name -contains 'vAlign') -and ($null -eq $c0.vAlign)
Add-Check 'docx-Va1' $va1ok "vAlign=$($c0.vAlign)"

$applyOut = Get-OutPath 'docx-cell-valign-apply.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $applyOut.Replace('\', '/')
    ops = @(@{ op = 'setCellFormat'; id = 'table:1/r1c1'; vAlign = 'center' })
} 'docx-cell-valign-apply' | Out-Null

$treeA = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $applyOut.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-cell-valign-inspect-apply'
$va2tree = [string]$treeA.tables[0].rows[0].cells[0].vAlign -eq 'center'

$createOut = Get-OutPath 'docx-cell-valign-create.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $createOut.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(@{ text = '前' })
        tables = @(@{
            cells = @(
                @('A1', 'B1'),
                @('A2', 'B2')
            )
            cellFormats = @(@{ r = 1; c = 1; vAlign = 'bottom' })
        })
    }
} 'docx-cell-valign-create' | Out-Null

$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $createOut.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-cell-valign-inspect-create'
$va3tree = [string]$treeC.tables[0].rows[0].cells[0].vAlign -eq 'bottom'

$clearOut = Get-OutPath 'docx-cell-valign-clear.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $applyOut.Replace('\', '/')
    output = $clearOut.Replace('\', '/')
    ops = @(@{ op = 'setCellFormat'; id = 'table:1/r1c1'; vAlign = $null })
} 'docx-cell-valign-clear' | Out-Null

$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $clearOut.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-cell-valign-inspect-clear'
$xmlN = Get-ZipPartText $clearOut 'word/document.xml'
$va4ok = ($null -eq $treeN.tables[0].rows[0].cells[0].vAlign) -and ($xmlN -notmatch 'vAlign')
Add-Check 'docx-Va4' $va4ok "tree=$($treeN.tables[0].rows[0].cells[0].vAlign)"

$failPara = Get-OutPath 'docx-cell-valign-fail-para.docx'
$rawP = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failPara.Replace('\', '/')
    ops = @(@{ op = 'setCellFormat'; id = 'para:nope'; vAlign = 'center' })
} 'docx-cell-valign-fail-para'
$xlsx = Get-OutPath 'docx-cell-valign.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-cell-valign-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-cell-valign-fail-xlsx.xlsx'
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'setCellFormat'; id = 'table:1/r1c1'; vAlign = 'center' })
} 'docx-cell-valign-fail-xlsx'
Add-Check 'docx-Va5' (
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
$comCenter = -1
$table = $null
$cell = $null
try {
    $table = $docA.Tables.Item(1)
    $cell = $table.Cell(1, 1)
    try { $comCenter = [int]$cell.VerticalAlignment } catch { $comCenter = -1 }
} catch { }
finally {
    Release-Com $cell
    Release-Com $table
}
Close-WordDoc $docA
Add-Check 'docx-Va2' (
    $va2tree -and ($capA -notmatch 'Repair') -and ($capA -match 'docx-cell-valign-apply') -and ($comCenter -eq 1)
) "vAlign=$($treeA.tables[0].rows[0].cells[0].vAlign) caption=$capA com=$comCenter"

Close-WordDocByPath $word $createOut
$docC = Open-WordDoc $word $createOut
$capC = [string]$docC.ActiveWindow.Caption
$comBottom = -1
$tableC = $null
$cellC = $null
try {
    $tableC = $docC.Tables.Item(1)
    $cellC = $tableC.Cell(1, 1)
    try { $comBottom = [int]$cellC.VerticalAlignment } catch { $comBottom = -1 }
} catch { }
finally {
    Release-Com $cellC
    Release-Com $tableC
}
Close-WordDoc $docC
Add-Check 'docx-Va3' (
    $va3tree -and ($capC -notmatch 'Repair') -and ($capC -match 'docx-cell-valign-create') -and ($comBottom -eq 3)
) "vAlign=$($treeC.tables[0].rows[0].cells[0].vAlign) caption=$capC com=$comBottom"
