# docx 表格宽高与对齐：docs/checklist-docx.md 的 Tw1–Tw6。
# 编号 Tw，不要和 Tb、Tf、T、Bx 撞。不要 $pid。

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

$src = Get-OutPath 'docx-table-width-src.docx'
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
} 'docx-table-width-create-src' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-table-width-inspect-src'
$t0 = @($tree0.tables) | Where-Object { $_.id -eq 'table:1' } | Select-Object -First 1
$tw1ok = ($null -ne $t0) -and
    ($t0.PSObject.Properties.Name -contains 'align') -and
    ($t0.PSObject.Properties.Name -contains 'indent') -and
    ($t0.PSObject.Properties.Name -contains 'width') -and
    ($t0.PSObject.Properties.Name -contains 'layout') -and
    ($t0.PSObject.Properties.Name -contains 'colWidths') -and
    ($t0.rows[0].PSObject.Properties.Name -contains 'height')
Add-Check 'docx-Tw1' $tw1ok "align=$($t0.align) width=$($t0.width.type) height=$($t0.rows[0].height)"

$applyOut = Get-OutPath 'docx-table-width-apply.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $applyOut.Replace('\', '/')
    ops = @(
        @{
            op = 'setTableWidth'; id = 'table:1'
            type = 'dxa'; w = 3600; cols = @(1800, 1800)
        }
        @{ op = 'setTableAlign'; id = 'table:1'; align = 'center' }
        @{
            op = 'setTableRowHeight'; id = 'table:1/row:1'
            h = 400; rule = 'atLeast'
        }
    )
} 'docx-table-width-apply' | Out-Null

$treeA = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $applyOut.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-table-width-inspect-apply'
$tA = @($treeA.tables) | Where-Object { $_.id -eq 'table:1' } | Select-Object -First 1
$tw2tree = [string]$tA.align -eq 'center'
$tw3tree = ([int]$tA.width.w -eq 3600) -and ([int]$tA.colWidths[0] -eq 1800) -and ([int]$tA.colWidths[1] -eq 1800)
$tw4tree = [int]$tA.rows[0].height.h -eq 400

$createOut = Get-OutPath 'docx-table-width-create.docx'
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
            align = 'center'
            width = @{ type = 'dxa'; w = 3600 }
            cols = @(1800, 1800)
            rowHeights = @(
                @{ h = 400; rule = 'atLeast' },
                @{ h = $null }
            )
        })
    }
} 'docx-table-width-create' | Out-Null

$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $createOut.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-table-width-inspect-create'
$tC = @($treeC.tables) | Where-Object { $_.id -eq 'table:1' } | Select-Object -First 1
$tw5tree = ([string]$tC.align -eq 'center') -and
    ([int]$tC.width.w -eq 3600) -and
    ([int]$tC.colWidths[0] -eq 1800) -and
    ([int]$tC.rows[0].height.h -eq 400)

$failPara = Get-OutPath 'docx-table-width-fail-para.docx'
$rawP = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failPara.Replace('\', '/')
    ops = @(@{ op = 'setTableWidth'; id = 'para:nope'; w = 1440 })
} 'docx-table-width-fail-para'
$xlsx = Get-OutPath 'docx-table-width.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-table-width-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-table-width-fail-xlsx.xlsx'
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'setTableWidth'; id = 'table:1'; w = 1440 })
} 'docx-table-width-fail-xlsx'
Add-Check 'docx-Tw6' (
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
$align = $null
$pref = $null
$rowH = $null
$table = $null
$row1 = $null
try {
    $table = $docA.Tables.Item(1)
    try { $align = [int]$table.Rows.Alignment } catch { $align = -1 }
    try { $pref = [double]$table.PreferredWidth } catch { $pref = -1 }
    $row1 = $table.Rows.Item(1)
    try { $rowH = [double]$row1.Height } catch { $rowH = -1 }
} catch { }
finally {
    Release-Com $row1
    Release-Com $table
}
Close-WordDoc $docA
Add-Check 'docx-Tw2' (
    $tw2tree -and ($capA -notmatch 'Repair') -and ($capA -match 'docx-table-width-apply') -and ($align -eq 1)
) "align=$($tA.align) caption=$capA comAlign=$align"
Add-Check 'docx-Tw3' (
    $tw3tree -and ($capA -notmatch 'Repair') -and ($capA -match 'docx-table-width-apply') -and ([math]::Abs($pref - 180) -lt 1.5)
) "w=$($tA.width.w) caption=$capA pref=$pref"
Add-Check 'docx-Tw4' (
    $tw4tree -and ($capA -notmatch 'Repair') -and ($capA -match 'docx-table-width-apply') -and ([math]::Abs($rowH - 20) -lt 1.5)
) "h=$($tA.rows[0].height.h) caption=$capA rowH=$rowH"

Close-WordDocByPath $word $createOut
$docC = Open-WordDoc $word $createOut
$capC = [string]$docC.ActiveWindow.Caption
Close-WordDoc $docC
Add-Check 'docx-Tw5' (
    $tw5tree -and ($capC -notmatch 'Repair') -and ($capC -match 'docx-table-width-create')
) "align=$($tC.align) w=$($tC.width.w) caption=$capC"
