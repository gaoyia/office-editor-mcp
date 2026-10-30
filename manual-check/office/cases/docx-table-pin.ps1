# 先插行再写原第 3 行，先删列再合并原来的第 2、3 列。Word 16.0 不报修复。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

function Get-CellText($table, [int]$r, [int]$c) {
    $cell = $table.Cell($r, $c)
    $rng = $cell.Range
    $t = [string]$rng.Text
    Release-Com $rng
    Release-Com $cell
    return $t.TrimEnd([char]7, [char]13, [char]10)
}

$rowSrc = Get-OutPath 'docx-table-pin-row.docx'
if (Test-Path $rowSrc) { Remove-Item -Force $rowSrc }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $rowSrc.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(@{ text = '前' })
        tables = @(@{ cells = @(@('R1'), @('R2'), @('R3')) })
    }
} 'docx-table-pin-row-create' | Out-Null
$rowOut = Get-OutPath 'docx-table-pin-row-out.docx'
if (Test-Path $rowOut) { Remove-Item -Force $rowOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $rowSrc.Replace('\', '/')
    output = $rowOut.Replace('\', '/')
    ops = @(
        @{ op = 'insertRow'; id = 'table:1/row:1' }
        @{ op = 'setCellFormat'; id = 'table:1/r3c1'; fill = '00FF00' }
    )
} 'docx-table-pin-row-apply' | Out-Null

$word = Ensure-MsWord (Ensure-DocxCreate)
Close-WordDocByPath $word $rowOut
$doc = Open-WordDoc $word $rowOut
$cap = [string]$doc.ActiveWindow.Caption
$table = $doc.Tables.Item(1)
$rows = [int]$table.Rows.Count
$r4 = Get-CellText $table 4 1
$r2 = Get-CellText $table 2 1
Release-Com $table
Close-WordDoc $doc
Add-Check 'docx-table-pin-row' (($rows -eq 4) -and ($r4 -eq 'R3') -and ($r2 -ne 'R3') -and ($cap -notmatch 'Repair')) "rows=$rows r2=$r2 r4=$r4 caption=$cap"

$colSrc = Get-OutPath 'docx-table-pin-col.docx'
if (Test-Path $colSrc) { Remove-Item -Force $colSrc }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $colSrc.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(@{ text = '前' })
        tables = @(@{ cells = @(,@('A', 'B', 'C')) })
    }
} 'docx-table-pin-col-create' | Out-Null
$colOut = Get-OutPath 'docx-table-pin-col-out.docx'
if (Test-Path $colOut) { Remove-Item -Force $colOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $colSrc.Replace('\', '/')
    output = $colOut.Replace('\', '/')
    ops = @(
        @{ op = 'deleteCol'; id = 'table:1'; col = 1 }
        @{
            op = 'mergeCells'; id = 'table:1'
            startRow = 1; startCol = 2; endRow = 1; endCol = 3
        }
    )
} 'docx-table-pin-col-apply' | Out-Null

Close-WordDocByPath $word $colOut
$doc2 = Open-WordDoc $word $colOut
$cap2 = [string]$doc2.ActiveWindow.Caption
$table2 = $doc2.Tables.Item(1)
$cols = [int]$table2.Columns.Count
$text = Get-CellText $table2 1 1
Release-Com $table2
Close-WordDoc $doc2
$merged = ($text -match 'B') -and ($text -match 'C') -and ($text -notmatch 'A')
Add-Check 'docx-table-pin-col' (($cols -eq 1) -and $merged -and ($cap2 -notmatch 'Repair')) "cols=$cols text=$text caption=$cap2"
