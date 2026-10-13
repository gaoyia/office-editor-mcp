# xlsx 形状行距与字距：docs/checklist-xlsx.md 的 Xln1–Xln5。
# 工作表文本框 DrawingML 段落间距与字符间距。COM SpaceBefore / Font.Spacing 是磅。

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

function Read-ExcelBoxSpacing($sheet) {
    $shapes = $null
    $shp = $null
    $tf = $null
    $tr = $null
    $font = $null
    $pf = $null
    $spacing = 0
    $kern = 0
    $before = 0
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
            $pf = $tr.ParagraphFormat
            try { $spacing = [double]$font.Spacing } catch { }
            try { $kern = [double]$font.Kerning } catch { }
            try { $before = [double]$pf.SpaceBefore } catch { }
            break
        }
    }
    finally {
        Release-Com $pf
        Release-Com $font
        Release-Com $tr
        Release-Com $tf
        Release-Com $shp
        Release-Com $shapes
    }
    return @{ Spacing = $spacing; Kerning = $kern; SpaceBefore = $before }
}

$hostPath = Get-OutPath 'xlsx-shape-spacing-host.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(@{
            name = 'Sheet1'
            cells = @(@{ ref = 'A1'; value = 1 })
        })
    }
} 'xlsx-xln-create' | Out-Null

$boxOut = Get-OutPath 'xlsx-shape-spacing-box.xlsx'
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
        lnSpc = @{ type = 'pts'; val = 2400 }
    })
} 'xlsx-xln-insert' | Out-Null
$treeBox = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $boxOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ textBoxes = $true }
} 'xlsx-xln-inspect-box'
$box0 = @($treeBox.sheets[0].textBoxes | Where-Object { $_ }) | Select-Object -First 1
$excel = Ensure-MsExcel $boxOut
Close-AllWorkbooks $excel
$wb0 = Open-ExcelBook $excel $boxOut
$cap0 = [string]$wb0.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$xln1ok = ($cap0 -notmatch 'Repair') `
    -and ([string]$box0.text -eq '你好') `
    -and ([string]$box0.lnSpc.type -eq 'pts') `
    -and ([int64]$box0.lnSpc.val -eq 2400)
Add-Check 'xlsx-Xln1' $xln1ok "cap=$cap0 ln=$($box0.lnSpc.val) id=$($box0.id)"

$fmtOut = Get-OutPath 'xlsx-shape-spacing-set.xlsx'
if (Test-Path $fmtOut) { Remove-Item -Force $fmtOut }
$boxId = [string]$box0.id
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $boxOut.Replace('\', '/')
    output = $fmtOut.Replace('\', '/')
    ops = @(@{
        op = 'setShapeFormat'
        id = $boxId
        spcBef = @{ type = 'pts'; val = 1200 }
        spc = 200
        kern = 1200
    })
} 'xlsx-xln-set' | Out-Null
$treeF = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $fmtOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ textBoxes = $true }
} 'xlsx-xln-inspect-set'
$boxF = @($treeF.sheets[0].textBoxes | Where-Object { $_ }) | Select-Object -First 1
$run0 = $boxF.runs[0]
$xln2ok = ([string]$boxF.spcBef.type -eq 'pts') -and ([int64]$boxF.spcBef.val -eq 1200) `
    -and ([int64]$run0.spc -eq 200) -and ([int64]$run0.kern -eq 1200)
Add-Check 'xlsx-Xln2' $xln2ok "bef=$($boxF.spcBef.val) spc=$($run0.spc) kern=$($run0.kern)"

$wbF = Open-ExcelBook $excel $fmtOut
$capF = [string]$wbF.Windows.Item(1).Caption
$sF = $wbF.Worksheets.Item(1)
$spcF = Read-ExcelBoxSpacing $sF
Close-AllWorkbooks $excel
$xln3ok = ($capF -notmatch 'Repair') `
    -and ([math]::Abs($spcF.SpaceBefore - 12) -lt 0.6) `
    -and ([math]::Abs($spcF.Spacing - 2) -lt 0.6)
Add-Check 'xlsx-Xln3' $xln3ok "cap=$capF before=$($spcF.SpaceBefore) spacing=$($spcF.Spacing)"

$ancOut = Get-OutPath 'xlsx-shape-spacing-anchor.xlsx'
if (Test-Path $ancOut) { Remove-Item -Force $ancOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $fmtOut.Replace('\', '/')
    output = $ancOut.Replace('\', '/')
    ops = @(@{ op = 'setShapeFormat'; id = $boxId; textAnchor = 'just' })
} 'xlsx-xln-anchor' | Out-Null
$treeA = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ancOut.Replace('\', '/'); format = 'xlsx'
    scope = @{ textBoxes = $true }
} 'xlsx-xln-inspect-anchor'
$boxA = @($treeA.sheets[0].textBoxes | Where-Object { $_ }) | Select-Object -First 1
$wbA = Open-ExcelBook $excel $ancOut
$capA = [string]$wbA.Windows.Item(1).Caption
Close-AllWorkbooks $excel
$xln4ok = ($capA -notmatch 'Repair') -and ([string]$boxA.textAnchor -eq 'just')
Add-Check 'xlsx-Xln4' $xln4ok "cap=$capA anchor=$($boxA.textAnchor)"

$failOut = Get-OutPath 'xlsx-shape-spacing-bad.xlsx'
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $fmtOut.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{
        op = 'setShapeFormat'; id = $boxId
        lnSpc = @{ type = 'lines'; val = 1 }
    })
} 'xlsx-xln-fail-type'
$codeBad = $null
try { $codeBad = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$xln5ok = ($fail.ExitCode -ne 0) -and ($codeBad -eq 'InvalidField') -and (-not (Test-Path $failOut))
Add-Check 'xlsx-Xln5' $xln5ok "exit=$($fail.ExitCode) code=$codeBad wrote=$(Test-Path $failOut)"
