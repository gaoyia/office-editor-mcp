# docx 格属性剩余：docs/checklist-docx.md 的 Tcp1–Tcp6。
# 编号 Tcp，不要和 Va、Tcb、Tf 撞。不要 Stop-Process Word。

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

$src = Get-OutPath 'docx-cell-props-src.docx'
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
} 'docx-cell-props-create-src' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-cell-props-inspect-src'
$c0 = $tree0.tables[0].rows[0].cells[0]
$names = @($c0.PSObject.Properties.Name)
$tcp1ok = (
    ($names -contains 'noWrap') -and ($null -eq $c0.noWrap) -and
    ($names -contains 'textDirection') -and ($null -eq $c0.textDirection) -and
    ($names -contains 'fitText') -and ($null -eq $c0.fitText) -and
    ($names -contains 'hideMark') -and ($null -eq $c0.hideMark) -and
    ($names -contains 'headers') -and (@($c0.headers).Count -eq 0)
)
Add-Check 'docx-Tcp1' $tcp1ok "noWrap=$($c0.noWrap) dir=$($c0.textDirection) headers=$($c0.headers.Count)"

$applyOut = Get-OutPath 'docx-cell-props-apply.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $applyOut.Replace('\', '/')
    ops = @(@{
        op = 'setCellFormat'; id = 'table:1/r1c1'
        noWrap = $true; textDirection = 'tbRl'
    })
} 'docx-cell-props-apply' | Out-Null

$treeA = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $applyOut.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-cell-props-inspect-apply'
$cA = $treeA.tables[0].rows[0].cells[0]
$tcp2tree = ($cA.noWrap -eq $true) -and ([string]$cA.textDirection -eq 'tbRl')

$fitOut = Get-OutPath 'docx-cell-props-fit.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $applyOut.Replace('\', '/')
    output = $fitOut.Replace('\', '/')
    ops = @(@{
        op = 'setCellFormat'; id = 'table:1/r1c1'
        fitText = $true; hideMark = $true
    })
} 'docx-cell-props-fit' | Out-Null

$treeF = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $fitOut.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-cell-props-inspect-fit'
$cF = $treeF.tables[0].rows[0].cells[0]
$xmlF = Get-ZipPartText $fitOut 'word/document.xml'
$tcp3tree = ($cF.fitText -eq $true) -and ($cF.hideMark -eq $true) -and ($xmlF -match 'hideMark')

$hdrOut = Get-OutPath 'docx-cell-props-headers.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $fitOut.Replace('\', '/')
    output = $hdrOut.Replace('\', '/')
    ops = @(@{
        op = 'setCellFormat'; id = 'table:1/r1c1'
        headers = @('A1')
    })
} 'docx-cell-props-headers' | Out-Null

$treeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hdrOut.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-cell-props-inspect-headers'
$cH = $treeH.tables[0].rows[0].cells[0]
$tcp4tree = (@($cH.headers).Count -eq 1) -and ([string]$cH.headers[0] -eq 'A1')

$clearOut = Get-OutPath 'docx-cell-props-clear.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hdrOut.Replace('\', '/')
    output = $clearOut.Replace('\', '/')
    ops = @(@{
        op = 'setCellFormat'; id = 'table:1/r1c1'
        noWrap = $null; textDirection = $null
        fitText = $null; hideMark = $null
        headers = @()
    })
} 'docx-cell-props-clear' | Out-Null

$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $clearOut.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-cell-props-inspect-clear'
$cN = $treeN.tables[0].rows[0].cells[0]
$xmlN = Get-ZipPartText $clearOut 'word/document.xml'
$tcp5ok = (
    ($null -eq $cN.noWrap) -and ($null -eq $cN.textDirection) -and
    ($null -eq $cN.fitText) -and ($null -eq $cN.hideMark) -and
    (@($cN.headers).Count -eq 0) -and
    ($xmlN -notmatch '<w:noWrap') -and ($xmlN -notmatch 'textDirection') -and
    ($xmlN -notmatch 'tcFitText') -and ($xmlN -notmatch 'hideMark') -and
    ($xmlN -notmatch '<w:headers')
)

$createOut = Get-OutPath 'docx-cell-props-create.docx'
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
            cellFormats = @(@{ r = 1; c = 1; noWrap = $true })
        })
    }
} 'docx-cell-props-create' | Out-Null

$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $createOut.Replace('\', '/'); format = 'docx'
    scope = @{ tables = $true; from = 1; maxParas = 20 }
} 'docx-cell-props-inspect-create'
$tcp6tree = $treeC.tables[0].rows[0].cells[0].noWrap -eq $true

$xlsx = Get-OutPath 'docx-cell-props.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-cell-props-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-cell-props-fail-xlsx.xlsx'
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'setCellFormat'; id = 'table:1/r1c1'; noWrap = $true })
} 'docx-cell-props-fail-xlsx'
$tcp6ok = $tcp6tree -and ((Get-FailCode $rawX) -eq 'InvalidField')
Add-Check 'docx-Tcp6' $tcp6ok "create=$tcp6tree xlsx=$(Get-FailCode $rawX)"

$seed = Ensure-DocxCreate
$word = Ensure-MsWord $seed
try { $word.AskToUpdateLinks = $false } catch {}
try { $word.Options.UpdateLinksAtOpen = $false } catch {}

Close-WordDocByPath $word $applyOut
$docA = Open-WordDoc $word $applyOut
$capA = [string]$docA.ActiveWindow.Caption
$comWrap = $true
$comOri = 0
$table = $null
$cell = $null
$rng = $null
try {
    $table = $docA.Tables.Item(1)
    $cell = $table.Cell(1, 1)
    try { $comWrap = [bool]$cell.WordWrap } catch { $comWrap = $true }
    try {
        $rng = $cell.Range
        $comOri = [int]$rng.Orientation
    } catch { $comOri = 0 }
} catch { }
finally {
    Release-Com $rng
    Release-Com $cell
    Release-Com $table
}
Close-WordDoc $docA
Add-Check 'docx-Tcp2' (
    $tcp2tree -and ($capA -notmatch 'Repair') -and ($capA -match 'docx-cell-props-apply') -and
    (-not $comWrap) -and ($comOri -ne 0)
) "noWrap=$($cA.noWrap) dir=$($cA.textDirection) caption=$capA wrap=$comWrap ori=$comOri"

Close-WordDocByPath $word $fitOut
$docF = Open-WordDoc $word $fitOut
$capF = [string]$docF.ActiveWindow.Caption
$comFit = $false
$tableF = $null
$cellF = $null
try {
    $tableF = $docF.Tables.Item(1)
    $cellF = $tableF.Cell(1, 1)
    try { $comFit = [bool]$cellF.FitText } catch { $comFit = $false }
} catch { }
finally {
    Release-Com $cellF
    Release-Com $tableF
}
Close-WordDoc $docF
Add-Check 'docx-Tcp3' (
    $tcp3tree -and ($capF -notmatch 'Repair') -and ($capF -match 'docx-cell-props-fit') -and $comFit
) "fit=$($cF.fitText) hide=$($cF.hideMark) caption=$capF comFit=$comFit"

Close-WordDocByPath $word $hdrOut
$docH = Open-WordDoc $word $hdrOut
$capH = [string]$docH.ActiveWindow.Caption
Close-WordDoc $docH
Add-Check 'docx-Tcp4' (
    $tcp4tree -and ($capH -notmatch 'Repair') -and ($capH -match 'docx-cell-props-headers')
) "headers=$($cH.headers[0]) caption=$capH"

Close-WordDocByPath $word $clearOut
$docN = Open-WordDoc $word $clearOut
$capN = [string]$docN.ActiveWindow.Caption
Close-WordDoc $docN
Add-Check 'docx-Tcp5' (
    $tcp5ok -and ($capN -notmatch 'Repair') -and ($capN -match 'docx-cell-props-clear')
) "noWrap=$($cN.noWrap) caption=$capN"
