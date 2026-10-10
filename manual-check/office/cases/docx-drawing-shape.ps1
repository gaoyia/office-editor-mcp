# docx 无框形状几何：docs/checklist-docx.md 的 Ds1–Ds8。
# 用 create 产物再 apply。编号 Ds，不要和 Dg、Dh、B、T 撞。
# 不算排版引擎。不编辑 VML。不要 $pid。

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

# Type=1 是 msoAutoShape。矩形 AutoShapeType=1，椭圆 9。
function Get-WordAutoShape($doc) {
    $shapes = $null
    $info = $null
    try {
        $shapes = $doc.Shapes
        $n = 0
        try { $n = [int]$shapes.Count } catch { $n = 0 }
        for ($i = 1; $i -le $n; $i++) {
            $sh = $null
            try {
                $sh = $shapes.Item($i)
                $ty = 0
                try { $ty = [int]$sh.Type } catch { $ty = 0 }
                if ($ty -ne 1) { continue }
                $auto = 0
                try { $auto = [int]$sh.AutoShapeType } catch { $auto = 0 }
                $fillRgb = $null
                $lineRgb = $null
                $fill = $null
                $fc = $null
                $line = $null
                $lc = $null
                try {
                    $fill = $sh.Fill
                    $fc = $fill.ForeColor
                    $fillRgb = [int]$fc.RGB
                } catch { }
                try {
                    $line = $sh.Line
                    $lc = $line.ForeColor
                    $lineRgb = [int]$lc.RGB
                } catch { }
                $info = @{
                    Type = $ty
                    Auto = $auto
                    Fill = $fillRgb
                    Line = $lineRgb
                    Count = $n
                }
            }
            finally {
                Release-Com $lc
                Release-Com $line
                Release-Com $fc
                Release-Com $fill
                Release-Com $sh
            }
            if ($null -ne $info) { break }
        }
    }
    finally { Release-Com $shapes }
    if ($null -eq $info) { return @{ Type = 0; Auto = 0; Fill = $null; Line = $null; Count = 0 } }
    return $info
}

$S = $global:OoxpStrings

$src = Get-OutPath 'docx-drawing-shape-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = $S.chapter1 }
            @{ text = $S.hello }
        )
    }
} 'docx-ds-create' | Out-Null

$sliced = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 10 }
} 'docx-ds-inspect-paras'
$para2 = [string]$sliced.paragraphs[1].id

$rectPath = Get-OutPath 'docx-drawing-shape-rect.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $rectPath.Replace('\', '/')
    ops = @(@{ op = 'insertShape'; id = $para2 })
} 'docx-ds-insert-rect' | Out-Null
$treeRect = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $rectPath.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 10; shapes = $true }
} 'docx-ds-inspect-rect'
$shRect = $treeRect.paragraphs[1].shapes[0]
$shapeId = [string]$shRect.id
$word = Ensure-MsWord $src
$docRect = Open-WordDoc $word $rectPath
$capRect = [string]$docRect.ActiveWindow.Caption
$stRect = Get-WordAutoShape $docRect
Close-WordDoc $docRect
Add-Check 'docx-Ds1' (
    ([int]$treeRect.shapeCount -eq 1) -and
    ([string]$shRect.prst -eq 'rect') -and
    ($shapeId -match '/shape:') -and
    ($capRect -notmatch 'Repair') -and
    ([int]$stRect.Type -eq 1) -and
    ([int]$stRect.Auto -eq 1)
) "count=$($treeRect.shapeCount) prst=$($shRect.prst) auto=$($stRect.Auto) caption=$capRect"

$ellPath = Get-OutPath 'docx-drawing-shape-ellipse.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $ellPath.Replace('\', '/')
    ops = @(@{ op = 'insertShape'; id = $para2; prst = 'ellipse' })
} 'docx-ds-insert-ellipse' | Out-Null
$treeEll = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ellPath.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 10; shapes = $true }
} 'docx-ds-inspect-ellipse'
$shEll = $treeEll.paragraphs[1].shapes[0]
$docEll = Open-WordDoc $word $ellPath
$capEll = [string]$docEll.ActiveWindow.Caption
$stEll = Get-WordAutoShape $docEll
Close-WordDoc $docEll
Add-Check 'docx-Ds2' (
    ([string]$shEll.prst -eq 'ellipse') -and
    ($capEll -notmatch 'Repair') -and
    ([int]$stEll.Type -eq 1) -and
    ([int]$stEll.Auto -eq 9)
) "prst=$($shEll.prst) auto=$($stEll.Auto) caption=$capEll"

$sizePath = Get-OutPath 'docx-drawing-shape-size.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $rectPath.Replace('\', '/')
    output = $sizePath.Replace('\', '/')
    ops = @(@{ op = 'setShapeSize'; id = $shapeId; cx = 914400; cy = 914400 })
} 'docx-ds-size' | Out-Null
$treeSize = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sizePath.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 10; shapes = $true }
} 'docx-ds-inspect-size'
$shSize = $treeSize.paragraphs[1].shapes[0]
$docSize = Open-WordDoc $word $sizePath
$capSize = [string]$docSize.ActiveWindow.Caption
Close-WordDoc $docSize
Add-Check 'docx-Ds3' (
    ([int]$shSize.cx -eq 914400) -and
    ([int]$shSize.cy -eq 914400) -and
    ($capSize -notmatch 'Repair')
) "cx=$($shSize.cx) cy=$($shSize.cy) caption=$capSize"

$fillPath = Get-OutPath 'docx-drawing-shape-fill.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $rectPath.Replace('\', '/')
    output = $fillPath.Replace('\', '/')
    ops = @(@{ op = 'setShapeFill'; id = $shapeId; color = 'FF0000' })
} 'docx-ds-fill' | Out-Null
$docFill = Open-WordDoc $word $fillPath
$capFill = [string]$docFill.ActiveWindow.Caption
$stFill = Get-WordAutoShape $docFill
Close-WordDoc $docFill
# Word 的 RGB 是 BGR：红=255。
Add-Check 'docx-Ds4' (
    ($capFill -notmatch 'Repair') -and
    ([int]$stFill.Fill -eq 255)
) "fill=$($stFill.Fill) caption=$capFill"

$linePath = Get-OutPath 'docx-drawing-shape-line.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $rectPath.Replace('\', '/')
    output = $linePath.Replace('\', '/')
    ops = @(@{ op = 'setShapeLine'; id = $shapeId; color = '0000FF' })
} 'docx-ds-line' | Out-Null
$docLine = Open-WordDoc $word $linePath
$capLine = [string]$docLine.ActiveWindow.Caption
$stLine = Get-WordAutoShape $docLine
Close-WordDoc $docLine
# 蓝=16711680。
Add-Check 'docx-Ds5' (
    ($capLine -notmatch 'Repair') -and
    ([int]$stLine.Line -eq 16711680)
) "line=$($stLine.Line) caption=$capLine"

$delPath = Get-OutPath 'docx-drawing-shape-del.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $rectPath.Replace('\', '/')
    output = $delPath.Replace('\', '/')
    ops = @(@{ op = 'deleteShape'; id = $shapeId })
} 'docx-ds-delete' | Out-Null
$treeDel = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delPath.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 10; shapes = $true }
} 'docx-ds-inspect-del'
Add-Check 'docx-Ds6' (
    ([int]$treeDel.shapeCount -eq 0) -and
    ([int]$treeDel.paragraphs[1].shapeCount -eq 0)
) "count=$($treeDel.shapeCount)"

$hfHost = Get-OutPath 'docx-drawing-shape-hf.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $hfHost.Replace('\', '/')
    ops = @(@{
        op = 'insertHeaderFooter'; id = 'section:1'; kind = 'header'; type = 'default'
    })
} 'docx-ds-hf-host' | Out-Null
$treeHf = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfHost.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true }
} 'docx-ds-inspect-hf-host'
$hfPara = [string]$treeHf.headers[0].paragraphs[0].id
$hfSh = Get-OutPath 'docx-drawing-shape-hf-shape.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hfHost.Replace('\', '/')
    output = $hfSh.Replace('\', '/')
    ops = @(@{ op = 'insertShape'; id = $hfPara })
} 'docx-ds-hf-insert' | Out-Null
$treeHfSh = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfSh.Replace('\', '/'); format = 'docx'
    scope = @{ shapes = $true }
} 'docx-ds-inspect-hf-shape'
$docHf = Open-WordDoc $word $hfSh
$capHf = [string]$docHf.ActiveWindow.Caption
Close-WordDoc $docHf
Add-Check 'docx-Ds7' (
    ([int]$treeHfSh.shapeCount -eq 0) -and
    ([int]$treeHfSh.headers[0].shapeCount -eq 1) -and
    ($capHf -notmatch 'Repair')
) "body=$($treeHfSh.shapeCount) header=$($treeHfSh.headers[0].shapeCount) caption=$capHf"

$failPrst = Get-OutPath 'docx-drawing-shape-fail-prst.docx'
$rawPrst = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failPrst.Replace('\', '/')
    ops = @(@{ op = 'insertShape'; id = $para2; prst = 'straightConnector1' })
} 'docx-ds-fail-prst'
$codePrst = Get-FailCode $rawPrst

$failText = Get-OutPath 'docx-drawing-shape-fail-text.docx'
$rawText = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failText.Replace('\', '/')
    ops = @(@{ op = 'insertShape'; id = $para2; text = '框' })
} 'docx-ds-fail-text'
$codeText = Get-FailCode $rawText

$failGeom = Get-OutPath 'docx-drawing-shape-fail-geom.docx'
$rawGeom = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $rectPath.Replace('\', '/')
    output = $failGeom.Replace('\', '/')
    ops = @(@{ op = 'setShapeGeom'; id = $shapeId })
} 'docx-ds-fail-geom'
$codeGeom = Get-FailCode $rawGeom

Add-Check 'docx-Ds8' (
    ($rawPrst.ExitCode -ne 0) -and ($codePrst -eq 'InvalidField') -and (-not (Test-Path $failPrst)) -and
    ($rawText.ExitCode -ne 0) -and ($codeText -eq 'InvalidField') -and (-not (Test-Path $failText)) -and
    ($rawGeom.ExitCode -ne 0) -and ($codeGeom -eq 'InvalidField') -and (-not (Test-Path $failGeom))
) "prst=$codePrst text=$codeText geom=$codeGeom"
