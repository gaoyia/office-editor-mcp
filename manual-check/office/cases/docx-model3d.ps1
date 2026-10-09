# docx 正文 3D 模型：docs/checklist-docx.md 的 D3d1–D3d6。
# 行内 COM 认 Type=19（wdInlineShape3DModel）。探测脚本不进本文件。

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

function Test-BytesEqual([byte[]]$a, [byte[]]$b) {
    if ($a.Length -ne $b.Length) { return $false }
    for ($i = 0; $i -lt $a.Length; $i++) {
        if ($a[$i] -ne $b[$i]) { return $false }
    }
    return $true
}

$glb = Ensure-OoxpGlb
$glbB = Ensure-OoxpGlbB
$glbFile = $glb.Replace('\', '/')
$glbBFile = $glbB.Replace('\', '/')

$src = Get-OutPath 'docx-model3d-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'docx'
    document = @{ paragraphs = @(@{ text = 'model3d-host' }) }
} 'docx-model3d-create-host' | Out-Null

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; model3ds = $true }
} 'docx-model3d-inspect-src'
$para = [string]$tree0.paragraphs[0].id

$inserted = Get-OutPath 'docx-model3d-inserted.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{ op = 'insertModel3d'; id = $para; file = $glbFile })
} 'docx-model3d-insert' | Out-Null

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; model3ds = $true; pictures = $true }
} 'docx-model3d-inspect-insert'
$item = @($tree1.paragraphs[0].model3ds) | Select-Object -First 1
$picCount = @($tree1.paragraphs[0].pictures).Count
$zipI = [IO.Compression.ZipFile]::OpenRead($inserted)
try {
    $glbPart = @($zipI.Entries | Where-Object { $_.FullName -like '*.glb' }).Count -gt 0
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
Add-Check 'docx-D3d1' (
    ($item.contentType -eq 'model/gltf.binary') -and ($picCount -eq 0) -and ($cap1 -notmatch 'Repair') -and ($n1 -ge 1) -and ([int]$t1 -eq 19) -and $glbPart
) "ct=$($item.contentType) caption=$cap1 inline=$n1 type=$t1 pics=$picCount glb=$glbPart"
Close-WordDoc $doc1

$exported = Get-OutPath 'docx-model3d-export.glb'
$expJson = (Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $inserted.Replace('\', '/')
    id = [string]$item.id
    output = $exported.Replace('\', '/')
} 'docx-model3d-export') | ConvertFrom-Json
$expCt = [string]$expJson.contentType
$same = Test-BytesEqual ([IO.File]::ReadAllBytes($exported)) ([IO.File]::ReadAllBytes($glb))
Add-Check 'docx-D3d2' (($expCt -eq 'model/gltf.binary') -and $same) "ct=$expCt same=$same"

$created = Get-OutPath 'docx-model3d-created.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $created.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = 'create-glb'; model3ds = @(@{ file = $glbFile }) }
        )
    }
} 'docx-model3d-create' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $created.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; model3ds = $true; pictures = $true }
} 'docx-model3d-inspect-create'
$itemC = @($treeC.paragraphs[0].model3ds) | Select-Object -First 1
$picC = @($treeC.paragraphs[0].pictures).Count
Close-WordDocByPath $word $created
$docC = Open-WordDoc $word $created
$capC = [string]$docC.ActiveWindow.Caption
Add-Check 'docx-D3d3' (
    ($itemC.contentType -eq 'model/gltf.binary') -and ($picC -eq 0) -and ($capC -notmatch 'Repair')
) "ct=$($itemC.contentType) pics=$picC caption=$capC"
Close-WordDoc $docC

$replaced = Get-OutPath 'docx-model3d-replaced.docx'
Close-WordDocByPath $word $replaced
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{ op = 'replaceModel3d'; id = [string]$item.id; file = $glbBFile })
} 'docx-model3d-replace' | Out-Null
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 8; model3ds = $true }
} 'docx-model3d-inspect-replace'
$itemR = @($treeR.paragraphs[0].model3ds) | Select-Object -First 1
$expR = Get-OutPath 'docx-model3d-export-b.glb'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $replaced.Replace('\', '/')
    id = [string]$itemR.id
    output = $expR.Replace('\', '/')
} 'docx-model3d-export-b' | Out-Null
$sameB = Test-BytesEqual ([IO.File]::ReadAllBytes($expR)) ([IO.File]::ReadAllBytes($glbB))
$docR = Open-WordDoc $word $replaced
$capR = [string]$docR.ActiveWindow.Caption
Add-Check 'docx-D3d4' (
    ([string]$itemR.id -eq [string]$item.id) -and $sameB -and ($capR -notmatch 'Repair')
) "id=$($itemR.id) sameB=$sameB caption=$capR"
Close-WordDoc $docR

$hfSrc = Get-OutPath 'docx-model3d-hf-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hfSrc.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(@{ text = 'body' })
        headers = @(@{ type = 'default'; paragraphs = @(@{ text = 'hdr' }) })
    }
} 'docx-model3d-hf-create' | Out-Null
$treeH0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfSrc.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true; model3ds = $true }
} 'docx-model3d-inspect-hf-src'
$hPara = [string]$treeH0.headers[0].paragraphs[0].id
$hfOut = Get-OutPath 'docx-model3d-hf.docx'
Close-WordDocByPath $word $hfOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hfSrc.Replace('\', '/')
    output = $hfOut.Replace('\', '/')
    ops = @(@{ op = 'insertModel3d'; id = $hPara; file = $glbFile })
} 'docx-model3d-hf-insert' | Out-Null
$treeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfOut.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true; model3ds = $true }
} 'docx-model3d-inspect-hf'
$hItem = @($treeH.headers[0].model3ds) | Select-Object -First 1
$docH = Open-WordDoc $word $hfOut
$capH = [string]$docH.ActiveWindow.Caption
Add-Check 'docx-D3d5' (
    ($hItem.contentType -eq 'model/gltf.binary') -and ([int]$treeH.model3dCount -eq 0) -and ($capH -notmatch 'Repair')
) "ct=$($hItem.contentType) root=$($treeH.model3dCount) caption=$capH"
Close-WordDoc $docH

$junk = Get-OutPath 'docx-model3d-not-glb.bin'
[IO.File]::WriteAllBytes($junk, [byte[]](1, 2, 3, 4))
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = (Get-OutPath 'docx-model3d-should-not.docx').Replace('\', '/')
    ops = @(@{ op = 'insertModel3d'; id = $para; file = $junk.Replace('\', '/') })
} 'docx-model3d-reject'
$failCode = $null
try { $failCode = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'docx-D3d6' (($fail.ExitCode -ne 0) -and ($failCode -eq 'InvalidField')) "code=$failCode exit=$($fail.ExitCode)"
