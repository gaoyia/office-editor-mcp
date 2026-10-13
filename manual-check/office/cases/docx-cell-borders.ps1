# docx 格正交边框：docs/checklist-docx.md 的 Tcb1–Tcb6。
# 编号 Tcb，不要和 Tc、Tb、Tf、Tbdr、Va 撞。不要 Stop-Process Word。

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

$src = Get-OutPath 'docx-cell-borders-src.docx'
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
} 'docx-cell-borders-create-src' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-cell-borders-inspect-src'
$cell0 = $tree0.tables[0].rows[0].cells[0]
$tcb1ok = ($cell0.PSObject.Properties.Name -contains 'borders') -and ($null -eq $cell0.borders)
Add-Check 'docx-Tcb1' $tcb1ok "borders=$($cell0.borders)"

$applyOut = Get-OutPath 'docx-cell-borders-apply.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $applyOut.Replace('\', '/')
    ops = @(@{
        op = 'setCellBorders'
        id = 'table:1/r1c1'
        borders = @{ top = @{ val = 'double'; sz = 12; space = 0; color = '000000' } }
    })
} 'docx-cell-borders-apply' | Out-Null

$treeA = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $applyOut.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-cell-borders-inspect-apply'
$tcb2tree = [string]$treeA.tables[0].rows[0].cells[0].borders.top.val -eq 'double'

$insideOut = Get-OutPath 'docx-cell-borders-inside.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $applyOut.Replace('\', '/')
    output = $insideOut.Replace('\', '/')
    ops = @(@{
        op = 'setCellBorders'
        id = 'table:1/r1c1'
        borders = @{ insideH = @{ val = 'dashed'; sz = 4; space = 0; color = 'auto' } }
    })
} 'docx-cell-borders-inside' | Out-Null

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insideOut.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-cell-borders-inspect-inside'
$cellI = $treeI.tables[0].rows[0].cells[0]
$tcb3tree = ([string]$cellI.borders.top.val -eq 'double') -and ([string]$cellI.borders.insideH.val -eq 'dashed')

$clearOut = Get-OutPath 'docx-cell-borders-clear.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insideOut.Replace('\', '/')
    output = $clearOut.Replace('\', '/')
    ops = @(
        @{
            op = 'setCellFormat'
            id = 'table:1/r1c1'
            tl2br = @{ val = 'single'; sz = 12; color = '0000FF' }
        },
        @{ op = 'setCellBorders'; id = 'table:1/r1c1'; borders = $null }
    )
} 'docx-cell-borders-clear' | Out-Null

$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $clearOut.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-cell-borders-inspect-clear'
$cellN = $treeN.tables[0].rows[0].cells[0]
$tcb4tree = ($null -eq $cellN.borders) -and ([string]$cellN.tl2br.val -eq 'single')

$createOut = Get-OutPath 'docx-cell-borders-create.docx'
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
            cellFormats = @(@{
                r = 1
                c = 1
                borders = @{ top = @{ val = 'double'; sz = 8; space = 0; color = 'FF0000' } }
            })
        })
    }
} 'docx-cell-borders-create' | Out-Null

$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $createOut.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-cell-borders-inspect-create'
$tcb5tree = [string]$treeC.tables[0].rows[0].cells[0].borders.top.val -eq 'double'

$failMissing = Get-OutPath 'docx-cell-borders-fail-missing.docx'
$rawM = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failMissing.Replace('\', '/')
    ops = @(@{ op = 'setCellBorders'; id = 'table:1/r1c1' })
} 'docx-cell-borders-fail-missing'
$failTable = Get-OutPath 'docx-cell-borders-fail-table.docx'
$rawT = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failTable.Replace('\', '/')
    ops = @(@{
        op = 'setCellBorders'
        id = 'table:1'
        borders = @{ top = @{ val = 'double' } }
    })
} 'docx-cell-borders-fail-table'
$xlsx = Get-OutPath 'docx-cell-borders.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-cell-borders-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-cell-borders-fail-xlsx.xlsx'
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{
        op = 'setCellBorders'
        id = 'table:1/r1c1'
        borders = @{ top = @{ val = 'double' } }
    })
} 'docx-cell-borders-fail-xlsx'
Add-Check 'docx-Tcb6' (
    ((Get-FailCode $rawM) -eq 'InvalidField') -and
    ((Get-FailCode $rawT) -eq 'InvalidId') -and
    ((Get-FailCode $rawX) -eq 'InvalidField')
) "missing=$(Get-FailCode $rawM) table=$(Get-FailCode $rawT) xlsx=$(Get-FailCode $rawX)"

$seed = Ensure-DocxCreate
$word = Ensure-MsWord $seed
try { $word.AskToUpdateLinks = $false } catch {}
try { $word.Options.UpdateLinksAtOpen = $false } catch {}

Close-WordDocByPath $word $applyOut
$docA = Open-WordDoc $word $applyOut
$capA = [string]$docA.ActiveWindow.Caption
$comTop = 0
$table = $null
$cell = $null
$borders = $null
$border = $null
try {
    $table = $docA.Tables.Item(1)
    $cell = $table.Cell(1, 1)
    $borders = $cell.Borders
    $border = $borders.Item(-1)
    try { $comTop = [int]$border.LineStyle } catch { $comTop = 0 }
} catch { }
finally {
    Release-Com $border
    Release-Com $borders
    Release-Com $cell
    Release-Com $table
}
Close-WordDoc $docA
Add-Check 'docx-Tcb2' (
    $tcb2tree -and ($capA -notmatch 'Repair') -and ($capA -match 'docx-cell-borders-apply') -and ($comTop -ne 0)
) "top=$($treeA.tables[0].rows[0].cells[0].borders.top.val) caption=$capA com=$comTop"

Close-WordDocByPath $word $insideOut
$docI = Open-WordDoc $word $insideOut
$capI = [string]$docI.ActiveWindow.Caption
Close-WordDoc $docI
Add-Check 'docx-Tcb3' (
    $tcb3tree -and ($capI -notmatch 'Repair') -and ($capI -match 'docx-cell-borders-inside')
) "top=$($cellI.borders.top.val) insideH=$($cellI.borders.insideH.val) caption=$capI"

Close-WordDocByPath $word $clearOut
$docN = Open-WordDoc $word $clearOut
$capN = [string]$docN.ActiveWindow.Caption
Close-WordDoc $docN
Add-Check 'docx-Tcb4' (
    $tcb4tree -and ($capN -notmatch 'Repair') -and ($capN -match 'docx-cell-borders-clear')
) "borders=$($cellN.borders) tl2br=$($cellN.tl2br.val) caption=$capN"

Close-WordDocByPath $word $createOut
$docC = Open-WordDoc $word $createOut
$capC = [string]$docC.ActiveWindow.Caption
Close-WordDoc $docC
Add-Check 'docx-Tcb5' (
    $tcb5tree -and ($capC -notmatch 'Repair') -and ($capC -match 'docx-cell-borders-create')
) "top=$($treeC.tables[0].rows[0].cells[0].borders.top.val) caption=$capC"
