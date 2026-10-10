# docx 样式增删与表格外观：docs/checklist-docx.md 的 Sy1–Sy6、Tf1–Tf6。
# 编号 Sy / Tf，不要和 St、Sa、Tb、T 撞。不要 $pid。

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

$src = Get-OutPath 'docx-style-table-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = '强调段' }
            @{ text = '前' }
        )
        tables = @(@{
            cells = @(
                @('A1', 'B1'),
                @('A2', 'B2')
            )
        })
    }
} 'docx-style-table-create' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ styles = $true; tables = $true; runs = $true; from = 1; maxParas = 20 }
} 'docx-style-table-inspect-src'
$paraId = [string]$tree0.paragraphs[0].id
$runId = [string]$tree0.paragraphs[0].runs[0].id
$styleCount0 = [int]$tree0.styleCount
$t0 = @($tree0.tables) | Where-Object { $_.id -eq 'table:1' } | Select-Object -First 1
$tf1ok = ($null -ne $t0) -and
    ($t0.PSObject.Properties.Name -contains 'style') -and
    ($t0.PSObject.Properties.Name -contains 'look') -and
    ($t0.PSObject.Properties.Name -contains 'pos') -and
    ($t0.PSObject.Properties.Name -contains 'cellMar') -and
    ($t0.rows[0].cells[0].PSObject.Properties.Name -contains 'fill') -and
    ($t0.rows[0].cells[0].PSObject.Properties.Name -contains 'cellMar') -and
    ($t0.rows[0].cells[0].PSObject.Properties.Name -contains 'tl2br') -and
    ($t0.rows[0].cells[0].PSObject.Properties.Name -contains 'tr2bl')
Add-Check 'docx-Tf1' $tf1ok "keys style=$($null -ne $t0.style) fill=$($t0.rows[0].cells[0].fill)"

$charOut = Get-OutPath 'docx-style-table-char.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $charOut.Replace('\', '/')
    ops = @(
        @{ op = 'insertStyle'; id = 'docx:document'; styleId = 'ooxpChar'; type = 'character'; name = '强调红' }
        @{ op = 'setStyle'; id = 'style:ooxpChar'; color = 'FF0000'; bold = $true }
        @{ op = 'setRunStyle'; id = $runId; style = 'style:ooxpChar' }
    )
} 'docx-style-table-char' | Out-Null

$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $charOut.Replace('\', '/'); format = 'docx'
    scope = @{ styles = $true; runs = $true; from = 1; maxParas = 20 }
} 'docx-style-table-inspect-char'
$listed = @($treeC.styles)
$foundChar = @($listed | Where-Object { [string]$_.id -eq 'style:ooxpChar' }) | Select-Object -First 1
$p0 = @($treeC.paragraphs) | Where-Object { $_.id -eq $paraId } | Select-Object -First 1
$sy1ok = ([int]$treeC.styleCount -eq ($styleCount0 + 1)) -and
    ($null -ne $foundChar) -and ([string]$foundChar.type -eq 'character')
Add-Check 'docx-Sy1' $sy1ok "count=$($treeC.styleCount) type=$($foundChar.type) name=$($foundChar.name)"
$sy3ok = ([string]$p0.runs[0].style -eq 'ooxpChar')
Add-Check 'docx-Sy3' $sy3ok "runStyle=$($p0.runs[0].style)"

$word = Ensure-MsWord $charOut
$docC = Open-WordDoc $word $charOut
$capC = [string]$docC.ActiveWindow.Caption
$rng = $null
$font = $null
$bold = $null
$rgb = $null
try {
    $rng = $docC.Paragraphs.Item(1).Range
    $font = $rng.Font
    $bold = [int]$font.Bold
    $rgb = [int]$font.Color
} catch { }
finally {
    Release-Com $font
    Release-Com $rng
}
Close-WordDoc $docC
$sy2ok = ($capC -notmatch 'Repair') -and ($rgb -eq 255) -and ($bold -ne 0)
Add-Check 'docx-Sy2' $sy2ok "caption=$capC rgb=$rgb bold=$bold"

$delOut = Get-OutPath 'docx-style-table-del.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $charOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteStyle'; id = 'style:ooxpChar' })
} 'docx-style-table-del' | Out-Null
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'docx'
    scope = @{ styles = $true; from = 1; maxParas = 20 }
} 'docx-style-table-inspect-del'
$still = @($treeD.styles | Where-Object { [string]$_.id -eq 'style:ooxpChar' }).Count
$docD = Open-WordDoc $word $delOut
$capD = [string]$docD.ActiveWindow.Caption
Close-WordDoc $docD
$sy4ok = ($still -eq 0) -and ([int]$treeD.styleCount -eq $styleCount0) -and ($capD -notmatch 'Repair')
Add-Check 'docx-Sy4' $sy4ok "still=$still count=$($treeD.styleCount) caption=$capD"

$failNormal = Get-OutPath 'docx-style-table-fail-normal.docx'
$rawN = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failNormal.Replace('\', '/')
    ops = @(@{ op = 'deleteStyle'; id = 'style:Normal' })
} 'docx-style-table-fail-normal'
Add-Check 'docx-Sy5' ((Get-FailCode $rawN) -eq 'InvalidField') "code=$(Get-FailCode $rawN)"

$xlsx = Get-OutPath 'docx-style-table.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-style-table-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-style-table-fail-xlsx.xlsx'
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'insertStyle'; id = 'docx:document'; styleId = 'x'; type = 'character' })
} 'docx-style-table-fail-xlsx'
$failNum = Get-OutPath 'docx-style-table-fail-num.docx'
$rawNum = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failNum.Replace('\', '/')
    ops = @(@{ op = 'insertStyle'; id = 'docx:document'; styleId = 'n1'; type = 'numbering' })
} 'docx-style-table-fail-num'
$sy6ok = ((Get-FailCode $rawX) -eq 'InvalidField') -and ((Get-FailCode $rawNum) -eq 'InvalidField')
Add-Check 'docx-Sy6' $sy6ok "xlsx=$(Get-FailCode $rawX) numbering=$(Get-FailCode $rawNum)"

$lookOut = Get-OutPath 'docx-style-table-look.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $lookOut.Replace('\', '/')
    ops = @(
        @{ op = 'insertStyle'; id = 'docx:document'; styleId = 'ooxpTbl'; type = 'table' }
        @{
            op = 'setTableStyle'; id = 'table:1'
            style = 'style:ooxpTbl'
            look = @{ firstRow = $true; bandedRows = $true; bandedColumns = $false }
        }
        @{
            op = 'setTableFloat'; id = 'table:1'
            pos = @{ horzAnchor = 'margin'; vertAnchor = 'text'; x = 1440; y = 720 }
        }
        @{ op = 'setTableCellMar'; id = 'table:1'; top = 120; left = 160; bottom = 120; right = 160 }
        @{
            op = 'setCellFormat'; id = 'table:1/r1c1'
            fill = 'FF0000'
            tl2br = @{ val = 'single'; sz = 12; color = '0000FF' }
        }
    )
} 'docx-style-table-look' | Out-Null

$treeL = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $lookOut.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-style-table-inspect-look'
$t1 = @($treeL.tables) | Where-Object { $_.id -eq 'table:1' } | Select-Object -First 1
$cell = $t1.rows[0].cells[0]
$tf2tree = ([int]$t1.pos.x -eq 1440) -and ([int]$t1.pos.y -eq 720) -and
    ([string]$t1.pos.horzAnchor -eq 'margin')
$tf3tree = ([int]$t1.cellMar.top -eq 120) -and ([int]$t1.cellMar.left -eq 160)
$tf6ok = ([string]$t1.style -eq 'ooxpTbl') -and ($t1.look.bandedRows -eq $true) -and
    ($t1.look.bandedColumns -eq $false)
Add-Check 'docx-Tf6' $tf6ok "style=$($t1.style) bandedRows=$($t1.look.bandedRows) bandedCols=$($t1.look.bandedColumns)"

$docL = Open-WordDoc $word $lookOut
$capL = [string]$docL.ActiveWindow.Caption
$table = $null
$rows = $null
$wrap = $null
$topPad = $null
$c11 = $null
$shd = $null
$bg = $null
$borders = $null
$diag = $null
$ls = $null
try {
    $table = $docL.Tables.Item(1)
    $rows = $table.Rows
    try { $wrap = [bool]$rows.WrapAroundText } catch { $wrap = $false }
    try { $topPad = [double]$table.TopPadding } catch { $topPad = -1 }
    $c11 = $table.Cell(1, 1)
    $shd = $c11.Shading
    $bg = [int]$shd.BackgroundPatternColor
    $borders = $c11.Borders
    $diag = $borders.Item(-7)
    $ls = [int]$diag.LineStyle
} catch { }
finally {
    Release-Com $diag
    Release-Com $borders
    Release-Com $shd
    Release-Com $c11
    Release-Com $rows
    Release-Com $table
}
Close-WordDoc $docL
Add-Check 'docx-Tf2' (
    $tf2tree -and ($capL -notmatch 'Repair') -and ($wrap -eq $true)
) "x=$($t1.pos.x) caption=$capL wrap=$wrap"
Add-Check 'docx-Tf3' (
    $tf3tree -and ($capL -notmatch 'Repair') -and ([math]::Abs($topPad - 6) -lt 0.15)
) "top=$($t1.cellMar.top) caption=$capL topPad=$topPad"
Add-Check 'docx-Tf4' (
    ($capL -notmatch 'Repair') -and ([string]$cell.fill -eq 'FF0000') -and ($bg -eq 255)
) "caption=$capL fill=$($cell.fill) bg=$bg"
Add-Check 'docx-Tf5' (
    ($capL -notmatch 'Repair') -and ([string]$cell.tl2br.val -eq 'single') -and ($ls -ne 0)
) "caption=$capL val=$($cell.tl2br.val) lineStyle=$ls"
