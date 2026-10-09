# pptx 幻灯片 3D 模型：docs/checklist-pptx.md 的 P3d1–P3d6。
# COM 认 Type=30（mso3DModel）。探测脚本不进本文件。

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

function Get-PptModel3dCount($hostShapes) {
    $n = 0
    try {
        for ($i = 1; $i -le $hostShapes.Count; $i++) {
            $sh = $hostShapes.Item($i)
            try {
                if ([int]$sh.Type -eq 30) { $n++ }
            } finally { Release-Com $sh }
        }
    } finally { Release-Com $hostShapes }
    return $n
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

$src = Get-OutPath 'pptx-model3d-src.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'pptx'
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'pptx-model3d-create-src' | Out-Null

$ppt = Ensure-MsPpt $src
Close-PptPresByPath $ppt $src

$inserted = Get-OutPath 'pptx-model3d-inserted.pptx'
Close-PptPresByPath $ppt $inserted
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{ op = 'insertModel3d'; id = 'slide:1'; file = $glbFile })
} 'pptx-model3d-insert' | Out-Null
$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'pptx'
    scope = @{ model3ds = $true; pictures = $true }
} 'pptx-model3d-inspect-insert'
$item = @($tree1.slides[0].model3ds) | Select-Object -First 1
$picCount = @($tree1.slides[0].pictures).Count
$pres1 = Open-PptPres $ppt $inserted
$cap1 = Get-PptCaption $pres1
$s1 = Get-PptSlide $pres1 1
$n1 = Get-PptModel3dCount $s1.Shapes
Release-Com $s1
Add-Check 'pptx-P3d1' (
    ($item.contentType -eq 'model/gltf.binary') -and ($picCount -eq 0) -and ($cap1 -notmatch 'Repair') -and ($n1 -ge 1)
) "ct=$($item.contentType) caption=$cap1 n=$n1 pics=$picCount"
Close-PptPres $pres1

$exported = Get-OutPath 'pptx-model3d-export.glb'
$expJson = (Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $inserted.Replace('\', '/')
    id = [string]$item.id
    output = $exported.Replace('\', '/')
} 'pptx-model3d-export') | ConvertFrom-Json
$expCt = [string]$expJson.contentType
$same = Test-BytesEqual ([IO.File]::ReadAllBytes($exported)) ([IO.File]::ReadAllBytes($glb))
Add-Check 'pptx-P3d2' (($expCt -eq 'model/gltf.binary') -and $same) "ct=$expCt same=$same"

$created = Get-OutPath 'pptx-model3d-created.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $created.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(@{
            shapes = @(@{ text = '标题' })
            model3ds = @(@{ file = $glbFile })
        })
    }
} 'pptx-model3d-create' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $created.Replace('\', '/'); format = 'pptx'
    scope = @{ model3ds = $true; pictures = $true }
} 'pptx-model3d-inspect-create'
$itemC = @($treeC.slides[0].model3ds) | Select-Object -First 1
$picC = @($treeC.slides[0].pictures).Count
Close-PptPresByPath $ppt $created
$presC = Open-PptPres $ppt $created
$capC = Get-PptCaption $presC
Add-Check 'pptx-P3d3' (
    ($itemC.contentType -eq 'model/gltf.binary') -and ($picC -eq 0) -and ($capC -notmatch 'Repair')
) "ct=$($itemC.contentType) pics=$picC caption=$capC"
Close-PptPres $presC

$replaced = Get-OutPath 'pptx-model3d-replaced.pptx'
Close-PptPresByPath $ppt $replaced
if (Test-Path $replaced) { Remove-Item $replaced -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{ op = 'replaceModel3d'; id = [string]$item.id; file = $glbBFile })
} 'pptx-model3d-replace' | Out-Null
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'pptx'
    scope = @{ model3ds = $true }
} 'pptx-model3d-inspect-replace'
$itemR = @($treeR.slides[0].model3ds) | Select-Object -First 1
$expR = Get-OutPath 'pptx-model3d-export-b.glb'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $replaced.Replace('\', '/')
    id = [string]$itemR.id
    output = $expR.Replace('\', '/')
} 'pptx-model3d-export-b' | Out-Null
$sameB = Test-BytesEqual ([IO.File]::ReadAllBytes($expR)) ([IO.File]::ReadAllBytes($glbB))
$presR = Open-PptPres $ppt $replaced
$capR = Get-PptCaption $presR
Add-Check 'pptx-P3d4' (
    ([string]$itemR.id -eq [string]$item.id) -and $sameB -and ($capR -notmatch 'Repair')
) "id=$($itemR.id) sameB=$sameB caption=$capR"
Close-PptPres $presR

$notesOut = Get-OutPath 'pptx-model3d-notes.pptx'
Close-PptPresByPath $ppt $notesOut
if (Test-Path $notesOut) { Remove-Item $notesOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $notesOut.Replace('\', '/')
    ops = @(@{ op = 'insertModel3d'; id = 'slide:1/notes'; file = $glbFile })
} 'pptx-model3d-notes-insert' | Out-Null
$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $notesOut.Replace('\', '/'); format = 'pptx'
    scope = @{ model3ds = $true; notes = $true }
} 'pptx-model3d-inspect-notes'
$nItem = @($treeN.slides[0].notes.model3ds) | Select-Object -First 1
$presN = Open-PptPres $ppt $notesOut
$capN = Get-PptCaption $presN
Add-Check 'pptx-P3d5' (
    ($nItem.contentType -eq 'model/gltf.binary') -and ([int]$treeN.model3dCount -eq 0) -and ($capN -notmatch 'Repair')
) "ct=$($nItem.contentType) root=$($treeN.model3dCount) caption=$capN"
Close-PptPres $presN

$junk = Get-OutPath 'pptx-model3d-not-glb.bin'
[IO.File]::WriteAllBytes($junk, [byte[]](1, 2, 3, 4))
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = (Get-OutPath 'pptx-model3d-should-not.pptx').Replace('\', '/')
    ops = @(@{ op = 'insertModel3d'; id = 'slide:1'; file = $junk.Replace('\', '/') })
} 'pptx-model3d-reject'
$failCode = $null
try { $failCode = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'pptx-P3d6' (($fail.ExitCode -ne 0) -and ($failCode -eq 'InvalidField')) "code=$failCode exit=$($fail.ExitCode)"
