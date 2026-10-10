# docx 框内表：docs/checklist-docx.md 的 Bx1–Bx5。
# COM 读形状 TextFrame.TextRange.Tables。编号 Bx，不要和 Bb、Tf、T、Tb 撞。

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

function Get-BoxTableCount($doc) {
    $n = 0
    $shapes = $null
    try {
        $shapes = $doc.Shapes
        $sc = [int]$shapes.Count
        for ($i = 1; $i -le $sc; $i++) {
            $sh = $null
            $tf = $null
            $tr = $null
            $tables = $null
            try {
                $sh = $shapes.Item($i)
                $tf = $sh.TextFrame
                $tr = $tf.TextRange
                $tables = $tr.Tables
                $n += [int]$tables.Count
            } catch { }
            finally {
                Release-Com $tables
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

$src = Get-OutPath 'docx-box-table-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(@{
            text = '外'
            textBoxes = @(@{ text = '框' })
        })
    }
} 'docx-box-table-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true }
} 'docx-box-table-inspect-src'
$box0 = $tree0.paragraphs[0].textBoxes[0]
$boxId = [string]$box0.id
$root0 = [int]$tree0.tableCount
$bx1count = ($null -ne $box0.tableCount) -and ([int]$box0.tableCount -eq 0) -and ($null -eq $box0.tables)

$ins = Get-OutPath 'docx-box-table-ins.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $ins.Replace('\', '/')
    ops = @(@{
        op = 'insertTable'; id = $boxId
        rows = 2; cols = 2
        cells = @(@('A1', 'B1'), @('A2', 'B2'))
    })
} 'docx-box-table-insert' | Out-Null

$listed = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ins.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true; tables = $true }
} 'docx-box-table-inspect-ins'
$boxL = $listed.paragraphs[0].textBoxes[0]
$t1 = @($boxL.tables) | Where-Object { [string]$_.id -eq "$boxId/table:1" } | Select-Object -First 1
$rootSame = ([int]$listed.tableCount -eq $root0)
$bx1ok = $bx1count -and $rootSame -and ($null -ne $t1) -and ([int]$boxL.tableCount -eq 1)
Add-Check 'docx-Bx1' $bx1ok "root=$($listed.tableCount) boxCount=$($boxL.tableCount) id=$($t1.id)"

$rowOut = Get-OutPath 'docx-box-table-row.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $ins.Replace('\', '/')
    output = $rowOut.Replace('\', '/')
    ops = @(
        @{ op = 'insertRow'; id = "$boxId/table:1" }
        @{ op = 'setCellFormat'; id = "$boxId/table:1/r1c1"; fill = 'FFFF00' }
    )
} 'docx-box-table-row-fill' | Out-Null

$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $rowOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true; tables = $true }
} 'docx-box-table-inspect-row'
$tR = $treeR.paragraphs[0].textBoxes[0].tables[0]
$fill = [string]$tR.rows[0].cells[0].fill
$bx2ok = ([int]$tR.rowCount -eq 3) -and ($fill -eq 'FFFF00')
Add-Check 'docx-Bx2' $bx2ok "rows=$($tR.rowCount) fill=$fill"

$nest = Get-OutPath 'docx-box-table-nest.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $rowOut.Replace('\', '/')
    output = $nest.Replace('\', '/')
    ops = @(@{
        op = 'insertTable'; id = "$boxId/table:1/r1c1"
        rows = 1; cols = 1
    })
} 'docx-box-table-nest' | Out-Null

$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $nest.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; textBoxes = $true; tables = $true }
} 'docx-box-table-inspect-nest'
$tablesN = @($treeN.paragraphs[0].textBoxes[0].tables)
$t2 = @($tablesN) | Where-Object { [string]$_.id -eq "$boxId/table:2" } | Select-Object -First 1
$bx3ok = ($tablesN.Count -ge 2) -and ($null -ne $t2) -and ($t2.nested -eq $true)
Add-Check 'docx-Bx3' $bx3ok "n=$($tablesN.Count) nestedId=$($t2.id) nested=$($t2.nested)"

$word = Ensure-MsWord $nest
$docN = Open-WordDoc $word $nest
$capN = [string]$docN.ActiveWindow.Caption
$comN = Get-BoxTableCount $docN
Close-WordDoc $docN
# 恢复成「文档1」时标题栏往往没有 Repair 字样，不能只靠这个词。
$bx4ok = ($capN -notmatch 'Repair') -and ($capN -match 'docx-box-table-nest') -and ($comN -ge 1)
Add-Check 'docx-Bx4' $bx4ok "caption=$capN comTables=$comN"

$fnSrc = Get-OutPath 'docx-box-table-fn.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $fnSrc.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(@{ text = '正文'; footnote = '注文' })
    }
} 'docx-box-table-fn-create' | Out-Null
$treeFn = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $fnSrc.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; notes = $true }
} 'docx-box-table-fn-inspect'
$fnPara = [string]$treeFn.footnotes[0].paragraphs[0].id
$failOut = Get-OutPath 'docx-box-table-fn-fail.docx'
if (Test-Path $failOut) { Remove-Item -Force $failOut }
$raw = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $fnSrc.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'insertTable'; id = $fnPara
        rows = 1; cols = 1
    })
} 'docx-box-table-fn-fail'
$failCode = $null
try { $failCode = ($raw.Stderr | ConvertFrom-Json).error.code } catch {}
$noWrite = -not (Test-Path $failOut)
$bx5ok = ($raw.ExitCode -ne 0) -and ($failCode -eq 'InvalidId') -and $noWrite
Add-Check 'docx-Bx5' $bx5ok "code=$failCode exit=$($raw.ExitCode) wrote=$(-not $noWrite)"
