# docx 页面背景与边框：docs/checklist-docx.md 的 Pb1–Pb6。
# 用 ooxp 写入页面颜色与节页面边框，再用 Word 打开核对无修复。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings

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

$side = @{ val = 'single'; sz = 4; space = 24; color = 'auto' }

$src = Get-OutPath 'docx-page-bg-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = $S.chapter1 }
            @{ text = 'hello' }
        )
    }
} 'docx-page-bg-create' | Out-Null

$bare = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-page-bg-inspect-bare'
$bareColorNull = $null -eq $bare.pageBackground.color
$bareDisplay = [bool]$bare.pageBackground.display
$bareBordersNull = $null -eq $bare.sections[0].pageBorders
Add-Check 'docx-Pb1' (
    $bareColorNull -and (-not $bareDisplay) -and $bareBordersNull
) "color=$($bare.pageBackground.color) display=$bareDisplay borders=$($bare.sections[0].pageBorders)"

$bg = Get-OutPath 'docx-page-bg-set.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $bg.Replace('\', '/')
    ops = @(@{
        op = 'setPageBackground'; id = 'docx:document'; color = 'FF0000'
    })
} 'docx-page-bg-set' | Out-Null
$treeBg = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $bg.Replace('\', '/'); format = 'docx'
} 'docx-page-bg-inspect-set'
$word = Ensure-MsWord $src
$docBg = Open-WordDoc $word $bg
$capBg = [string]$docBg.ActiveWindow.Caption
Close-WordDoc $docBg
Add-Check 'docx-Pb2' (
    ([string]$treeBg.pageBackground.color -eq 'FF0000') -and
    ([bool]$treeBg.pageBackground.display -eq $true) -and
    ($capBg -notmatch 'Repair')
) "color=$($treeBg.pageBackground.color) display=$($treeBg.pageBackground.display) caption=$capBg"

$cleared = Get-OutPath 'docx-page-bg-clear.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $bg.Replace('\', '/')
    output = $cleared.Replace('\', '/')
    ops = @(@{
        op = 'setPageBackground'; id = 'docx:document'; color = $null
    })
} 'docx-page-bg-clear' | Out-Null
$treeCl = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cleared.Replace('\', '/'); format = 'docx'
} 'docx-page-bg-inspect-clear'
$docCl = Open-WordDoc $word $cleared
$capCl = [string]$docCl.ActiveWindow.Caption
Close-WordDoc $docCl
Add-Check 'docx-Pb3' (
    ($null -eq $treeCl.pageBackground.color) -and
    ([bool]$treeCl.pageBackground.display -eq $false) -and
    ($capCl -notmatch 'Repair')
) "color=$($treeCl.pageBackground.color) display=$($treeCl.pageBackground.display) caption=$capCl"

$bd = Get-OutPath 'docx-page-bg-borders.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $bd.Replace('\', '/')
    ops = @(@{
        op = 'setPageBorders'; id = 'section:1'; offsetFrom = 'page'
        top = $side; left = $side; bottom = $side; right = $side
    })
} 'docx-page-bg-borders' | Out-Null
$treeBd = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $bd.Replace('\', '/'); format = 'docx'
} 'docx-page-bg-inspect-borders'
$docBd = Open-WordDoc $word $bd
$capBd = [string]$docBd.ActiveWindow.Caption
Close-WordDoc $docBd
Add-Check 'docx-Pb4' (
    ([string]$treeBd.sections[0].pageBorders.top.val -eq 'single') -and
    ([string]$treeBd.sections[0].pageBorders.left.val -eq 'single') -and
    ([string]$treeBd.sections[0].pageBorders.offsetFrom -eq 'page') -and
    ($capBd -notmatch 'Repair')
) "val=$($treeBd.sections[0].pageBorders.top.val) caption=$capBd"

$bdClear = Get-OutPath 'docx-page-bg-borders-clear.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $bd.Replace('\', '/')
    output = $bdClear.Replace('\', '/')
    ops = @(@{ op = 'setPageBorders'; id = 'section:1'; enabled = $false })
} 'docx-page-bg-borders-clear' | Out-Null
$treeBdC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $bdClear.Replace('\', '/'); format = 'docx'
} 'docx-page-bg-inspect-borders-clear'
$docBdC = Open-WordDoc $word $bdClear
$capBdC = [string]$docBdC.ActiveWindow.Caption
Close-WordDoc $docBdC
Add-Check 'docx-Pb5' (
    ($null -eq $treeBdC.sections[0].pageBorders) -and ($capBdC -notmatch 'Repair')
) "borders=$($treeBdC.sections[0].pageBorders) caption=$capBdC"

$xlsx = Get-OutPath 'docx-page-bg-xlsx.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-page-bg-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-page-bg-fail-xlsx.xlsx'
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'setPageBackground'; id = 'docx:document'; color = 'FF0000' })
} 'docx-page-bg-fail-xlsx'
try { $codeX = ($rawX.Stderr | ConvertFrom-Json).error.code } catch { $codeX = $null }
Add-Check 'docx-Pb6' (
    ($rawX.ExitCode -ne 0) -and ($codeX -eq 'InvalidField') -and (-not (Test-Path $failX))
) "xlsx=$codeX wroteX=$([bool](Test-Path $failX))"
