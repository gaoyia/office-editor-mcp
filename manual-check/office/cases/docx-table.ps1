# docx 表格：docs/checklist-docx.md 的 Tb1–Tb8。
# COM 的 Cell.Range.Text 末尾带单元格结束符（ASCII 7）。

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

function Get-CellText($table, [int]$r, [int]$c) {
    $cell = $table.Cell($r, $c)
    $rng = $cell.Range
    $t = [string]$rng.Text
    Release-Com $rng
    Release-Com $cell
    return $t.TrimEnd([char]7, [char]13, [char]10)
}

$src = Get-OutPath 'docx-table-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(@{ text = '前' })
        tables = @(@{
            cells = @(
                @('A1', 'B1'),
                @('A2', 'B2')
            )
        })
    }
} 'docx-table-create' | Out-Null

$word = Ensure-MsWord $src

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-table-inspect'
$listed = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-table-inspect-list'
$t1 = @($listed.tables) | Where-Object { $_.id -eq 'table:1' } | Select-Object -First 1
$cellParas = @()
if ($null -ne $t1) {
    foreach ($row in @($t1.rows)) {
        foreach ($cell in @($row.cells)) {
            $cellParas += @($cell.paragraphs)
        }
    }
}
$tb1ok = ($null -ne $tree0.tableCount) -and ([int]$tree0.tableCount -eq 1) -and
    ($null -eq $tree0.tables) -and ($null -ne $t1) -and
    ([int]$t1.rowCount -eq 2) -and ([int]$t1.colCount -eq 2) -and
    ($cellParas.Count -ge 4) -and ($null -ne $cellParas[0].id)
Add-Check 'docx-Tb1' $tb1ok "count=$($tree0.tableCount) listed=$($t1.id) rows=$($t1.rowCount) cols=$($t1.colCount) paras=$($cellParas.Count)"

$a1id = $t1.rows[0].cells[0].paragraphs[0].id
$b1text = [string]$t1.rows[0].cells[1].paragraphs[0].text
$a2text = [string]$t1.rows[1].cells[0].paragraphs[0].text

$edited = Get-OutPath 'docx-table-text.docx'
Close-WordDocByPath $word $edited
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $edited.Replace('\', '/')
    ops = @(@{ op = 'setParaText'; id = $a1id; text = '新A1' })
} 'docx-table-set-cell' | Out-Null
$docT = Open-WordDoc $word $edited
$capT = [string]$docT.ActiveWindow.Caption
$tblT = $docT.Tables.Item(1)
$gotA1 = Get-CellText $tblT 1 1
$gotB1 = Get-CellText $tblT 1 2
$gotA2 = Get-CellText $tblT 2 1
$p1 = Get-ParaText $docT.Paragraphs.Item(1)
Release-Com $tblT
Add-Check 'docx-Tb2' (($capT -notmatch 'Repair') -and ($gotA1 -eq '新A1') -and ($gotB1 -eq $b1text) -and ($gotA2 -eq $a2text) -and ($p1 -eq '前')) `
    "caption=$capT a1=$gotA1 b1=$gotB1 a2=$gotA2 body=$p1"
Close-WordDoc $docT

$rowOut = Get-OutPath 'docx-table-row.docx'
Close-WordDocByPath $word $rowOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $rowOut.Replace('\', '/')
    ops = @(@{ op = 'insertRow'; id = 'table:1' })
} 'docx-table-insert-row' | Out-Null
$docR = Open-WordDoc $word $rowOut
$capR = [string]$docR.ActiveWindow.Caption
$nRows = [int]$docR.Tables.Item(1).Rows.Count
Add-Check 'docx-Tb3' (($capR -notmatch 'Repair') -and ($nRows -eq 3)) "caption=$capR rows=$nRows"
Close-WordDoc $docR

$colOut = Get-OutPath 'docx-table-col.docx'
Close-WordDocByPath $word $colOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $colOut.Replace('\', '/')
    ops = @(@{ op = 'insertCol'; id = 'table:1'; after = 1 })
} 'docx-table-insert-col' | Out-Null
$docC = Open-WordDoc $word $colOut
$capC = [string]$docC.ActiveWindow.Caption
$nCols = [int]$docC.Tables.Item(1).Columns.Count
Add-Check 'docx-Tb4' (($capC -notmatch 'Repair') -and ($nCols -eq 3)) "caption=$capC cols=$nCols"
Close-WordDoc $docC

$delOut = Get-OutPath 'docx-table-del.docx'
Close-WordDocByPath $word $delOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteTable'; id = 'table:1' })
} 'docx-table-delete' | Out-Null
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'docx'
} 'docx-table-del-inspect'
$docD = Open-WordDoc $word $delOut
$capD = [string]$docD.ActiveWindow.Caption
$nDel = [int]$docD.Tables.Count
Add-Check 'docx-Tb5' (($capD -notmatch 'Repair') -and ([int]$treeD.tableCount -eq 0) -and ($nDel -eq 0)) `
    "caption=$capD count=$($treeD.tableCount) com=$nDel"
Close-WordDoc $docD

$doc6 = Open-WordDoc $word $src
$cap6 = [string]$doc6.ActiveWindow.Caption
$n6 = [int]$doc6.Tables.Count
Add-Check 'docx-Tb6' (($cap6 -notmatch 'Repair') -and ($n6 -eq 1)) "caption=$cap6 tables=$n6"
Close-WordDoc $doc6

$mergeOut = Get-OutPath 'docx-table-merge.docx'
Close-WordDocByPath $word $mergeOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $mergeOut.Replace('\', '/')
    ops = @(@{
        op = 'mergeCells'; id = 'table:1'
        startRow = 1; startCol = 1; endRow = 1; endCol = 2
    })
} 'docx-table-merge' | Out-Null
$docM = Open-WordDoc $word $mergeOut
$capM = [string]$docM.ActiveWindow.Caption
$tblM = $docM.Tables.Item(1)
$nMerge = [int]$tblM.Rows.Item(1).Cells.Count
Release-Com $tblM
Add-Check 'docx-Tb7' (($capM -notmatch 'Repair') -and ($nMerge -eq 1)) "caption=$capM row1cells=$nMerge"
Close-WordDoc $docM

$oneRow = Get-OutPath 'docx-table-onerow.docx'
Close-WordDocByPath $word $oneRow
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $oneRow.Replace('\', '/')
    ops = @(@{ op = 'deleteRow'; id = 'table:1/row:2' })
} 'docx-table-del-row2' | Out-Null
$failOut = Get-OutPath 'docx-table-last-row.docx'
if (Test-Path $failOut) { Remove-Item -Force $failOut }
$raw = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $oneRow.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'deleteRow'; id = 'table:1/row:1' })
} 'docx-table-del-last'
$lastCode = $null
try { $lastCode = ($raw.Stderr | ConvertFrom-Json).error.code } catch {}
$noWrite = -not (Test-Path $failOut)
Add-Check 'docx-Tb8' (($raw.ExitCode -ne 0) -and ($lastCode -eq 'InvalidField') -and $noWrite) `
    "code=$lastCode exit=$($raw.ExitCode) wrote=$(-not $noWrite)"
