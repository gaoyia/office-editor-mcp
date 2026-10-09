# docx 嵌入 EMF / WMF / TIFF：docs/checklist-docx.md 的 Dem1–Dem6。
# 行内 COM 认 Type=3（wdInlineShapePicture）。探测脚本不进本文件。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }
Add-Type -AssemblyName System.IO.Compression.FileSystem

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

$emf = Ensure-OoxpEmf
$wmf = Ensure-OoxpWmf
$tiff = Ensure-OoxpTiff
$pngB = Ensure-OoxpPngB
$emfFile = $emf.Replace('\', '/')
$wmfFile = $wmf.Replace('\', '/')
$tiffFile = $tiff.Replace('\', '/')

$src = Get-OutPath 'docx-emf-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{ paragraphs = @(@{ text = 'emf-host' }) }
} 'docx-emf-create-host' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; pictures = $true }
} 'docx-emf-inspect-src'
$para = [string]$tree0.paragraphs[0].id

$inserted = Get-OutPath 'docx-emf-inserted.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = $para; file = $emfFile })
} 'docx-emf-insert' | Out-Null

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; pictures = $true }
} 'docx-emf-inspect-insert'
$pic = @($tree1.paragraphs[0].pictures) | Select-Object -First 1
$zipI = [IO.Compression.ZipFile]::OpenRead($inserted)
try {
    $emfPart = @($zipI.Entries | Where-Object { $_.FullName -like '*.emf' }).Count -gt 0
} finally { $zipI.Dispose() }
$word = Ensure-MsWord $inserted
$doc1 = Open-WordDoc $word $inserted
$cap1 = [string]$doc1.ActiveWindow.Caption
$n1 = $doc1.InlineShapes.Count
$t1 = $null
if ($n1 -ge 1) {
    $sh1 = $doc1.InlineShapes.Item(1)
    $t1 = $sh1.Type
    Release-Com $sh1
}
Add-Check 'docx-Dem1' (
    ($pic.contentType -eq 'image/x-emf') -and ($cap1 -notmatch 'Repair') -and ($n1 -ge 1) -and ([int]$t1 -eq 3) -and $emfPart
) "ct=$($pic.contentType) caption=$cap1 inline=$n1 type=$t1 emf=$emfPart"
Close-WordDoc $doc1

$exported = Get-OutPath 'docx-emf-export.emf'
$expJson = (Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $inserted.Replace('\', '/')
    id = [string]$pic.id
    output = $exported.Replace('\', '/')
} 'docx-emf-export') | ConvertFrom-Json
$expCt = [string]$expJson.contentType
$expBytes = [IO.File]::ReadAllBytes($exported)
$srcBytes = [IO.File]::ReadAllBytes($emf)
$same = ($expBytes.Length -eq $srcBytes.Length)
if ($same) {
    for ($i = 0; $i -lt $expBytes.Length; $i++) {
        if ($expBytes[$i] -ne $srcBytes[$i]) { $same = $false; break }
    }
}
Add-Check 'docx-Dem2' (($expCt -eq 'image/x-emf') -and $same) "ct=$expCt len=$($expBytes.Length) same=$same"

$created = Get-OutPath 'docx-emf-created.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $created.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = 'create-wmf'; pictures = @(@{ file = $wmfFile }) }
        )
    }
} 'docx-emf-create' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $created.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; pictures = $true }
} 'docx-emf-inspect-create'
$picC = @($treeC.paragraphs[0].pictures) | Select-Object -First 1
Close-WordDocByPath $word $created
$docC = Open-WordDoc $word $created
$capC = [string]$docC.ActiveWindow.Caption
Add-Check 'docx-Dem3' (
    ($picC.contentType -eq 'image/x-wmf') -and ($capC -notmatch 'Repair')
) "ct=$($picC.contentType) caption=$capC"
Close-WordDoc $docC

$replaced = Get-OutPath 'docx-emf-replaced.docx'
Close-WordDocByPath $word $replaced
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{ op = 'replacePicture'; id = [string]$pic.id; file = $pngB.Replace('\', '/') })
} 'docx-emf-replace' | Out-Null
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; pictures = $true }
} 'docx-emf-inspect-replace'
$picR = @($treeR.paragraphs[0].pictures) | Select-Object -First 1
$hasCt = $picR.PSObject.Properties.Name -contains 'contentType'
$docR = Open-WordDoc $word $replaced
$capR = [string]$docR.ActiveWindow.Caption
Add-Check 'docx-Dem4' ((-not $hasCt) -and ($capR -notmatch 'Repair')) "hasCt=$hasCt caption=$capR"
Close-WordDoc $docR

$hfSrc = Get-OutPath 'docx-emf-hf-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hfSrc.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(@{ text = 'body' })
        headers = @(@{ type = 'default'; paragraphs = @(@{ text = 'hdr' }) })
    }
} 'docx-emf-hf-create' | Out-Null
$treeH0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfSrc.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true; pictures = $true }
} 'docx-emf-inspect-hf-src'
$hPara = [string]$treeH0.headers[0].paragraphs[0].id
$hfOut = Get-OutPath 'docx-emf-hf.docx'
Close-WordDocByPath $word $hfOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hfSrc.Replace('\', '/')
    output = $hfOut.Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = $hPara; file = $tiffFile })
} 'docx-emf-hf-insert' | Out-Null
$treeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfOut.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true; pictures = $true }
} 'docx-emf-inspect-hf'
$hPic = @($treeH.headers[0].pictures) | Select-Object -First 1
$docH = Open-WordDoc $word $hfOut
$capH = [string]$docH.ActiveWindow.Caption
Add-Check 'docx-Dem5' (
    ($hPic.contentType -eq 'image/tiff') -and ($capH -notmatch 'Repair')
) "ct=$($hPic.contentType) caption=$capH"
Close-WordDoc $docH

$junk = Get-OutPath 'docx-emf-not-image.bin'
[IO.File]::WriteAllBytes($junk, [byte[]](1, 2, 3, 4))
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = (Get-OutPath 'docx-emf-should-not.docx').Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = $para; file = $junk.Replace('\', '/') })
} 'docx-emf-reject'
$failCode = $null
try { $failCode = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'docx-Dem6' (($fail.ExitCode -ne 0) -and ($failCode -eq 'InvalidField')) "code=$failCode exit=$($fail.ExitCode)"
