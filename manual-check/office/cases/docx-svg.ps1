# docx 嵌入 SVG：docs/checklist-docx.md 的 Sv1–Sv6。
# 不要和正文图 B 组混跑依赖。探测脚本不进本文件。

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

$svg = Get-OutPath 'pic-probe.svg'
[IO.File]::WriteAllText($svg, @'
<svg xmlns="http://www.w3.org/2000/svg" width="96" height="48" viewBox="0 0 96 48">
  <rect width="96" height="48" fill="#3366cc"/>
  <circle cx="48" cy="24" r="16" fill="#ffcc00"/>
</svg>
'@, $global:OoxpUtf8)
$pngB = Ensure-OoxpPngB
$svgFile = $svg.Replace('\', '/')

$src = Get-OutPath 'docx-svg-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{ paragraphs = @(@{ text = 'svg-host' }) }
} 'docx-svg-create-host' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; pictures = $true }
} 'docx-svg-inspect-src'
$para = [string]$tree0.paragraphs[0].id

# Sv1：insertPicture SVG。
$inserted = Get-OutPath 'docx-svg-inserted.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = $para; file = $svgFile })
} 'docx-svg-insert' | Out-Null

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; pictures = $true }
} 'docx-svg-inspect-insert'
$pic = @($tree1.paragraphs[0].pictures) | Select-Object -First 1
$word = Ensure-MsWord $inserted
$doc1 = Open-WordDoc $word $inserted
$cap1 = [string]$doc1.ActiveWindow.Caption
$n1 = $doc1.InlineShapes.Count
$t1 = $null
$w1 = $null
$h1 = $null
if ($n1 -ge 1) {
    $sh1 = $doc1.InlineShapes.Item(1)
    $t1 = $sh1.Type
    $w1 = [math]::Round([double]$sh1.Width, 2)
    $h1 = [math]::Round([double]$sh1.Height, 2)
    Release-Com $sh1
}
Add-Check 'docx-Sv1' (
    ($pic.contentType -eq 'image/svg+xml') -and ($cap1 -notmatch 'Repair') -and ($n1 -ge 1)
) "ct=$($pic.contentType) caption=$cap1 inline=$n1 type=$t1 w=$w1 h=$h1"
Close-WordDoc $doc1

# Sv2：exportMedia 是 SVG 原件。
$exported = Get-OutPath 'docx-svg-export.svg'
$expJson = (Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $inserted.Replace('\', '/')
    id = [string]$pic.id
    output = $exported.Replace('\', '/')
} 'docx-svg-export') | ConvertFrom-Json
$expCt = [string]$expJson.contentType
$expText = [IO.File]::ReadAllText($exported)
$srcText = [IO.File]::ReadAllText($svg)
Add-Check 'docx-Sv2' (($expCt -eq 'image/svg+xml') -and ($expText -eq $srcText)) "ct=$expCt len=$($expText.Length)"

# Sv3：create 带 SVG。
$created = Get-OutPath 'docx-svg-created.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $created.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = 'create-svg'; pictures = @(@{ file = $svgFile }) }
        )
    }
} 'docx-svg-create' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $created.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; pictures = $true }
} 'docx-svg-inspect-create'
$picC = @($treeC.paragraphs[0].pictures) | Select-Object -First 1
Close-WordDocByPath $word $created
$docC = Open-WordDoc $word $created
$capC = [string]$docC.ActiveWindow.Caption
Add-Check 'docx-Sv3' (
    ($picC.contentType -eq 'image/svg+xml') -and ($capC -notmatch 'Repair')
) "ct=$($picC.contentType) caption=$capC"
Close-WordDoc $docC

# Sv4：换成 PNG。
$replaced = Get-OutPath 'docx-svg-replaced.docx'
Close-WordDocByPath $word $replaced
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{ op = 'replacePicture'; id = [string]$pic.id; file = $pngB.Replace('\', '/') })
} 'docx-svg-replace' | Out-Null
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; pictures = $true }
} 'docx-svg-inspect-replace'
$picR = @($treeR.paragraphs[0].pictures) | Select-Object -First 1
$hasCt = $picR.PSObject.Properties.Name -contains 'contentType'
$docR = Open-WordDoc $word $replaced
$capR = [string]$docR.ActiveWindow.Caption
Add-Check 'docx-Sv4' ((-not $hasCt) -and ($capR -notmatch 'Repair')) "hasCt=$hasCt caption=$capR"
Close-WordDoc $docR

# Sv5：页眉 SVG。
$hfSrc = Get-OutPath 'docx-svg-hf-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hfSrc.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(@{ text = 'body' })
        headers = @(@{ type = 'default'; paragraphs = @(@{ text = 'hdr' }) })
    }
} 'docx-svg-hf-create' | Out-Null
$treeH0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfSrc.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true; pictures = $true }
} 'docx-svg-inspect-hf-src'
$hPara = [string]$treeH0.headers[0].paragraphs[0].id
$hfOut = Get-OutPath 'docx-svg-hf.docx'
Close-WordDocByPath $word $hfOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hfSrc.Replace('\', '/')
    output = $hfOut.Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = $hPara; file = $svgFile })
} 'docx-svg-hf-insert' | Out-Null
$treeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfOut.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true; pictures = $true }
} 'docx-svg-inspect-hf'
$hPic = @($treeH.headers[0].pictures) | Select-Object -First 1
$docH = Open-WordDoc $word $hfOut
$capH = [string]$docH.ActiveWindow.Caption
Add-Check 'docx-Sv5' (
    ($hPic.contentType -eq 'image/svg+xml') -and ($capH -notmatch 'Repair')
) "ct=$($hPic.contentType) caption=$capH"
Close-WordDoc $docH

# Sv6：拒错，不开 Word。
$junk = Get-OutPath 'not-image.bin'
[IO.File]::WriteAllBytes($junk, [byte[]](1, 2, 3, 4))
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = (Get-OutPath 'docx-svg-should-not.docx').Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = $para; file = $junk.Replace('\', '/') })
} 'docx-svg-reject'
$failCode = $null
try { $failCode = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'docx-Sv6' (($fail.ExitCode -ne 0) -and ($failCode -eq 'InvalidField')) "code=$failCode exit=$($fail.ExitCode)"
