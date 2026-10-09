# docx 墨迹画布：docs/checklist-docx.md 的 Ic1–Ic6。
# 不要和批注墨迹 Ik 组混跑。探测脚本不进本文件。不要 Stop-Process Word。

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

$ink = Get-OutPath 'ink-probe.xml'
[IO.File]::WriteAllText($ink, @'
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<ink xmlns="http://www.w3.org/2003/InkML">
  <trace>0 0, 10 10, 20 5</trace>
</ink>
'@, $global:OoxpUtf8)
$inkB = Get-OutPath 'ink-probe-b.xml'
[IO.File]::WriteAllText($inkB, @'
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<ink xmlns="http://www.w3.org/2003/InkML">
  <trace>1 1, 30 30</trace>
</ink>
'@, $global:OoxpUtf8)
$inkFile = $ink.Replace('\', '/')
$inkFileB = $inkB.Replace('\', '/')

$src = Get-OutPath 'docx-ink-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{ paragraphs = @(@{ text = 'ink-host' }) }
} 'docx-ink-create-host' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; inkCanvases = $true }
} 'docx-ink-inspect-src'
$para = [string]$tree0.paragraphs[0].id

# Ic1：insertInkCanvas。
$inserted = Get-OutPath 'docx-ink-inserted.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{ op = 'insertInkCanvas'; id = $para; file = $inkFile })
} 'docx-ink-insert' | Out-Null

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; inkCanvases = $true }
} 'docx-ink-inspect-insert'
$canvas = @($tree1.paragraphs[0].inkCanvases) | Select-Object -First 1
$word = Ensure-MsWord $inserted
$doc1 = Open-WordDoc $word $inserted
$cap1 = [string]$doc1.ActiveWindow.Caption
# Word 16.0 把 wp:inline 的 wpc 画布挂在 Shapes 上，InlineShapes.Count 仍是 0。Type=20 是 msoCanvas。
$n1 = $doc1.Shapes.Count
$t1 = $null
$w1 = $null
$h1 = $null
$name1 = $null
if ($n1 -ge 1) {
    $sh1 = $doc1.Shapes.Item(1)
    $t1 = $sh1.Type
    $name1 = [string]$sh1.Name
    $w1 = [math]::Round([double]$sh1.Width, 2)
    $h1 = [math]::Round([double]$sh1.Height, 2)
    Release-Com $sh1
}
Add-Check 'docx-Ic1' (
    ($canvas.contentType -eq 'application/inkml+xml') -and ($cap1 -notmatch 'Repair') -and ($n1 -ge 1) -and ($t1 -eq 20)
) "ct=$($canvas.contentType) caption=$cap1 shapes=$n1 type=$t1 name=$name1 w=$w1 h=$h1"
Close-WordDoc $doc1

# Ic2：exportMedia 是 InkML 原件。
$exported = Get-OutPath 'docx-ink-export.xml'
$expJson = (Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $inserted.Replace('\', '/')
    id = [string]$canvas.id
    output = $exported.Replace('\', '/')
} 'docx-ink-export') | ConvertFrom-Json
$expCt = [string]$expJson.contentType
$expText = [IO.File]::ReadAllText($exported)
$srcText = [IO.File]::ReadAllText($ink)
Add-Check 'docx-Ic2' (($expCt -eq 'application/inkml+xml') -and ($expText -eq $srcText)) "ct=$expCt len=$($expText.Length)"

# Ic3：create 带墨迹画布。
$created = Get-OutPath 'docx-ink-created.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $created.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = 'create-ink'; inkCanvases = @(@{ file = $inkFile }) }
        )
    }
} 'docx-ink-create' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $created.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; inkCanvases = $true }
} 'docx-ink-inspect-create'
$canvasC = @($treeC.paragraphs[0].inkCanvases) | Select-Object -First 1
Close-WordDocByPath $word $created
$docC = Open-WordDoc $word $created
$capC = [string]$docC.ActiveWindow.Caption
Add-Check 'docx-Ic3' (
    ($canvasC.contentType -eq 'application/inkml+xml') -and ($capC -notmatch 'Repair')
) "ct=$($canvasC.contentType) caption=$capC"
Close-WordDoc $docC

# Ic4：换 InkML。
$replaced = Get-OutPath 'docx-ink-replaced.docx'
Close-WordDocByPath $word $replaced
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{ op = 'replaceInkCanvas'; id = [string]$canvas.id; file = $inkFileB })
} 'docx-ink-replace' | Out-Null
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; inkCanvases = $true }
} 'docx-ink-inspect-replace'
$canvasR = @($treeR.paragraphs[0].inkCanvases) | Select-Object -First 1
$exportedR = Get-OutPath 'docx-ink-export-b.xml'
$expR = (Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $replaced.Replace('\', '/')
    id = [string]$canvasR.id
    output = $exportedR.Replace('\', '/')
} 'docx-ink-export-b') | ConvertFrom-Json
$docR = Open-WordDoc $word $replaced
$capR = [string]$docR.ActiveWindow.Caption
$expRText = [IO.File]::ReadAllText($exportedR)
$srcB = [IO.File]::ReadAllText($inkB)
Add-Check 'docx-Ic4' (
    ([string]$expR.contentType -eq 'application/inkml+xml') -and ($expRText -eq $srcB) -and ($capR -notmatch 'Repair')
) "ct=$($expR.contentType) caption=$capR"
Close-WordDoc $docR

# Ic5：页眉。
$hfSrc = Get-OutPath 'docx-ink-hf-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hfSrc.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(@{ text = 'body' })
        headers = @(@{ type = 'default'; paragraphs = @(@{ text = 'hdr' }) })
    }
} 'docx-ink-hf-create' | Out-Null
$treeH0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfSrc.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true; inkCanvases = $true }
} 'docx-ink-inspect-hf-src'
$hPara = [string]$treeH0.headers[0].paragraphs[0].id
$hfOut = Get-OutPath 'docx-ink-hf.docx'
Close-WordDocByPath $word $hfOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hfSrc.Replace('\', '/')
    output = $hfOut.Replace('\', '/')
    ops = @(@{ op = 'insertInkCanvas'; id = $hPara; file = $inkFile })
} 'docx-ink-hf-insert' | Out-Null
$treeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfOut.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true; inkCanvases = $true }
} 'docx-ink-inspect-hf'
$hInk = @($treeH.headers[0].inkCanvases) | Select-Object -First 1
$docH = Open-WordDoc $word $hfOut
$capH = [string]$docH.ActiveWindow.Caption
Add-Check 'docx-Ic5' (
    ($hInk.contentType -eq 'application/inkml+xml') -and ($capH -notmatch 'Repair') -and ([int]$treeH.inkCanvasCount -eq 0)
) "ct=$($hInk.contentType) caption=$capH rootCount=$($treeH.inkCanvasCount)"
Close-WordDoc $docH

# Ic6：拒错，不开 Word。
$junk = Get-OutPath 'not-ink.bin'
[IO.File]::WriteAllBytes($junk, [byte[]](1, 2, 3, 4))
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = (Get-OutPath 'docx-ink-should-not.docx').Replace('\', '/')
    ops = @(@{ op = 'insertInkCanvas'; id = $para; file = $junk.Replace('\', '/') })
} 'docx-ink-reject'
$failCode = $null
try { $failCode = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'docx-Ic6' (($fail.ExitCode -ne 0) -and ($failCode -eq 'InvalidField')) "code=$failCode exit=$($fail.ExitCode)"
