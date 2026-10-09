# xlsx 形状文字格式：docs/checklist-xlsx.md 的 Xtf1–Xtf5。
# 工作表文本框 DrawingML 文字格式。COM Font.Size 是磅；字色 RGB 是 OLE 色（R + G×256 + B×65536）。

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

function Convert-SrgbToOleRgb([string]$hex) {
    $r = [Convert]::ToInt32($hex.Substring(0, 2), 16)
    $g = [Convert]::ToInt32($hex.Substring(2, 2), 16)
    $b = [Convert]::ToInt32($hex.Substring(4, 2), 16)
    return [int]($r + ($g * 256) + ($b * 65536))
}

function Read-ExcelBoxFont($sheet) {
    $shapes = $null
    $shp = $null
    $tf = $null
    $tr = $null
    $font = $null
    $fill = $null
    $fore = $null
    $size = 0
    $rgb = 0
    $bold = $false
    try {
        $shapes = $sheet.Shapes
        $n = 0
        try { $n = [int]$shapes.Count } catch { }
        for ($i = 1; $i -le $n; $i++) {
            $shp = $null
            try { $shp = $shapes.Item($i) } catch { continue }
            $ty = 0
            try { $ty = [int]$shp.Type } catch { }
            if ($ty -ne 17) {
                Release-Com $shp
                $shp = $null
                continue
            }
            $tf = $shp.TextFrame2
            $tr = $tf.TextRange
            $font = $tr.Font
            $size = [double]$font.Size
            try { $bold = [bool]$font.Bold } catch { }
            $fill = $font.Fill
            $fore = $fill.ForeColor
            $rgb = [int]$fore.RGB
            break
        }
    }
    finally {
        Release-Com $fore
        Release-Com $fill
        Release-Com $font
        Release-Com $tr
        Release-Com $tf
        Release-Com $shp
        Release-Com $shapes
    }
    return @{ Size = $size; Rgb = $rgb; Bold = $bold }
}

$hostPath = Get-OutPath 'xlsx-shape-format-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
        })
    }
} 'xlsx-xtf-create' | Out-Null

$boxOut = Get-OutPath 'xlsx-shape-format-box.xlsx'
if (Test-Path $boxOut) { Remove-Item -Force $boxOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $boxOut.Replace('\', '/')
    ops = @(@{
        op = 'insertTextBox'
        id = 'sheet:Sheet1'
        from = 'B2'
        text = '你好'
        sz = 2400
        color = '0000FF'
    })
} 'xlsx-xtf-insert' | Out-Null
$treeBox = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $boxOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ textBoxes = $true }
} 'xlsx-xtf-inspect-box'
$box0 = @($treeBox.sheets[0].textBoxes | Where-Object { $_ }) | Select-Object -First 1
$excel = Ensure-MsExcel $boxOut
Close-AllWorkbooks $excel
$wb0 = Open-ExcelBook $excel $boxOut
$cap0 = [string]$wb0.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$xtf1ok = ($cap0 -notmatch 'Repair') `
    -and ([string]$box0.text -eq '你好') `
    -and ([int64]$box0.sz -eq 2400) `
    -and ([string]$box0.color -eq '0000FF')
Add-Check 'xlsx-Xtf1' $xtf1ok "cap=$cap0 sz=$($box0.sz) color=$($box0.color) id=$($box0.id)"

$fmtOut = Get-OutPath 'xlsx-shape-format-set.xlsx'
if (Test-Path $fmtOut) { Remove-Item -Force $fmtOut }
$boxId = [string]$box0.id
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $boxOut.Replace('\', '/')
    output = $fmtOut.Replace('\', '/')
    ops = @(@{
        op = 'setShapeFormat'
        id = $boxId
        sz = 1800
        color = 'C41E3A'
        bold = $true
        align = 'ctr'
    })
} 'xlsx-xtf-set' | Out-Null
$treeF = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $fmtOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ textBoxes = $true }
} 'xlsx-xtf-inspect-set'
$boxF = @($treeF.sheets[0].textBoxes | Where-Object { $_ }) | Select-Object -First 1
$xtf2ok = ([int64]$boxF.sz -eq 1800) `
    -and ([string]$boxF.color -eq 'C41E3A') `
    -and ([bool]$boxF.bold) `
    -and ([string]$boxF.align -eq 'ctr')
Add-Check 'xlsx-Xtf2' $xtf2ok "sz=$($boxF.sz) color=$($boxF.color) bold=$($boxF.bold) align=$($boxF.align)"

$wbF = Open-ExcelBook $excel $fmtOut
$capF = [string]$wbF.Windows.Item(1).Caption
$sF = $wbF.Worksheets.Item(1)
$fontF = Read-ExcelBoxFont $sF
Close-AllWorkbooks $excel
$wantRgb = Convert-SrgbToOleRgb 'C41E3A'
$xtf3ok = ($capF -notmatch 'Repair') `
    -and ([math]::Abs($fontF.Size - 18) -lt 0.6) `
    -and ($fontF.Rgb -eq $wantRgb)
Add-Check 'xlsx-Xtf3' $xtf3ok "cap=$capF size=$($fontF.Size) rgb=$($fontF.Rgb) want=$wantRgb"

$createOut = Get-OutPath 'xlsx-shape-format-create.xlsx'
if (Test-Path $createOut) { Remove-Item -Force $createOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $createOut.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
            textBoxes = @(@{
                from = 'B2'
                text = 'Create框'
                sz = 2200
                color = '1F4E79'
            })
        })
    }
} 'xlsx-xtf-create-fmt' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $createOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ textBoxes = $true }
} 'xlsx-xtf-inspect-create'
$boxC = @($treeC.sheets[0].textBoxes | Where-Object { $_ }) | Select-Object -First 1
$wbC = Open-ExcelBook $excel $createOut
$capC = [string]$wbC.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$xtf4ok = ($capC -notmatch 'Repair') `
    -and ([string]$boxC.text -eq 'Create框') `
    -and ([int64]$boxC.sz -eq 2200) `
    -and ([string]$boxC.color -eq '1F4E79')
Add-Check 'xlsx-Xtf4' $xtf4ok "cap=$capC sz=$($boxC.sz) color=$($boxC.color)"

$failOut = Get-OutPath 'xlsx-shape-format-bad.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $fmtOut.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'setShapeFormat'; id = $boxId })
} 'xlsx-xtf-missing'
$codeEmpty = $null
try { $codeEmpty = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$xtf5ok = ($fail.ExitCode -ne 0) -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xtf5' $xtf5ok "exit=$($fail.ExitCode) code=$codeEmpty wrote=$(Test-Path $failOut)"
