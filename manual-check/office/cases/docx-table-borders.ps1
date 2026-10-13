# docx 表级边框：docs/checklist-docx.md 的 Tbdr1–Tbdr6。
# 编号 Tbdr，不要和 Tb、Tf、Tw、Pb 撞。不要 Stop-Process Word。

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

$src = Get-OutPath 'docx-table-borders-src.docx'
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
} 'docx-table-borders-create-src' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-table-borders-inspect-src'
$b0 = $tree0.tables[0].borders
$tbdr1ok = ($tree0.tables[0].PSObject.Properties.Name -contains 'borders') -and ([string]$b0.top.val -eq 'single')
Add-Check 'docx-Tbdr1' $tbdr1ok "top=$($b0.top.val)"

$applyOut = Get-OutPath 'docx-table-borders-apply.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $applyOut.Replace('\', '/')
    ops = @(@{
        op = 'setTableBorders'
        id = 'table:1'
        borders = @{ top = @{ val = 'double'; sz = 12; space = 0; color = '000000' } }
    })
} 'docx-table-borders-apply' | Out-Null

$treeA = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $applyOut.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-table-borders-inspect-apply'
$tbdr2tree = [string]$treeA.tables[0].borders.top.val -eq 'double'

$insideOut = Get-OutPath 'docx-table-borders-inside.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $applyOut.Replace('\', '/')
    output = $insideOut.Replace('\', '/')
    ops = @(@{
        op = 'setTableBorders'
        id = 'table:1'
        borders = @{ insideH = @{ val = 'dashed'; sz = 4; space = 0; color = 'auto' } }
    })
} 'docx-table-borders-inside' | Out-Null

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insideOut.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-table-borders-inspect-inside'
$tbdr3tree = ([string]$treeI.tables[0].borders.top.val -eq 'double') -and ([string]$treeI.tables[0].borders.insideH.val -eq 'dashed')

$clearOut = Get-OutPath 'docx-table-borders-clear.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insideOut.Replace('\', '/')
    output = $clearOut.Replace('\', '/')
    ops = @(@{ op = 'setTableBorders'; id = 'table:1'; borders = $null })
} 'docx-table-borders-clear' | Out-Null

$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $clearOut.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-table-borders-inspect-clear'
$xmlN = Get-ZipPartText $clearOut 'word/document.xml'
$tbdr4tree = ($null -eq $treeN.tables[0].borders) -and ($xmlN -notmatch 'tblBorders')

$createOut = Get-OutPath 'docx-table-borders-create.docx'
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
            borders = @{ top = @{ val = 'double'; sz = 8; space = 0; color = 'FF0000' } }
        })
    }
} 'docx-table-borders-create' | Out-Null

$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $createOut.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-table-borders-inspect-create'
$tbdr5tree = [string]$treeC.tables[0].borders.top.val -eq 'double'

$failMissing = Get-OutPath 'docx-table-borders-fail-missing.docx'
$rawM = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failMissing.Replace('\', '/')
    ops = @(@{ op = 'setTableBorders'; id = 'table:1' })
} 'docx-table-borders-fail-missing'
$xlsx = Get-OutPath 'docx-table-borders.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-table-borders-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-table-borders-fail-xlsx.xlsx'
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{
        op = 'setTableBorders'
        id = 'table:1'
        borders = @{ top = @{ val = 'double' } }
    })
} 'docx-table-borders-fail-xlsx'
Add-Check 'docx-Tbdr6' (
    ((Get-FailCode $rawM) -eq 'InvalidField') -and
    ((Get-FailCode $rawX) -eq 'InvalidField')
) "missing=$(Get-FailCode $rawM) xlsx=$(Get-FailCode $rawX)"

$seed = Ensure-DocxCreate
$word = Ensure-MsWord $seed
try { $word.AskToUpdateLinks = $false } catch {}
try { $word.Options.UpdateLinksAtOpen = $false } catch {}

Close-WordDocByPath $word $applyOut
$docA = Open-WordDoc $word $applyOut
$capA = [string]$docA.ActiveWindow.Caption
$comTop = 0
$table = $null
$border = $null
try {
    $table = $docA.Tables.Item(1)
    $border = $table.Borders.Item(-1)
    try { $comTop = [int]$border.LineStyle } catch { $comTop = 0 }
} catch { }
finally {
    Release-Com $border
    Release-Com $table
}
Close-WordDoc $docA
Add-Check 'docx-Tbdr2' (
    $tbdr2tree -and ($capA -notmatch 'Repair') -and ($capA -match 'docx-table-borders-apply') -and ($comTop -ne 0)
) "top=$($treeA.tables[0].borders.top.val) caption=$capA com=$comTop"

Close-WordDocByPath $word $createOut
$docC = Open-WordDoc $word $createOut
$capC = [string]$docC.ActiveWindow.Caption
Close-WordDoc $docC
Add-Check 'docx-Tbdr5' (
    $tbdr5tree -and ($capC -notmatch 'Repair') -and ($capC -match 'docx-table-borders-create')
) "top=$($treeC.tables[0].borders.top.val) caption=$capC"

Close-WordDocByPath $word $insideOut
$docI = Open-WordDoc $word $insideOut
$capI = [string]$docI.ActiveWindow.Caption
Close-WordDoc $docI
Add-Check 'docx-Tbdr3' (
    $tbdr3tree -and ($capI -notmatch 'Repair') -and ($capI -match 'docx-table-borders-inside')
) "top=$($treeI.tables[0].borders.top.val) insideH=$($treeI.tables[0].borders.insideH.val) caption=$capI"

Close-WordDocByPath $word $clearOut
$docN = Open-WordDoc $word $clearOut
$capN = [string]$docN.ActiveWindow.Caption
Close-WordDoc $docN
Add-Check 'docx-Tbdr4' (
    $tbdr4tree -and ($capN -notmatch 'Repair') -and ($capN -match 'docx-table-borders-clear')
) "tree=$($treeN.tables[0].borders) caption=$capN"
