# pptx 嵌入 EMF / WMF / TIFF：docs/checklist-pptx.md 的 Pem1–Pem6。
# COM 认 Type=13（msoPicture）。探测脚本不进本文件。

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

function Get-PptPictureCount($hostShapes) {
    $n = 0
    try {
        for ($i = 1; $i -le $hostShapes.Count; $i++) {
            $sh = $hostShapes.Item($i)
            try {
                if ([int]$sh.Type -eq 13) { $n++ }
            } finally { Release-Com $sh }
        }
    } finally { Release-Com $hostShapes }
    return $n
}

$emf = Ensure-OoxpEmf
$wmf = Ensure-OoxpWmf
$tiff = Ensure-OoxpTiff
$pngB = Ensure-OoxpPngB
$emfFile = $emf.Replace('\', '/')
$wmfFile = $wmf.Replace('\', '/')
$tiffFile = $tiff.Replace('\', '/')

$src = Get-OutPath 'pptx-emf-src.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'pptx'
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'pptx-emf-create-src' | Out-Null

$ppt = Ensure-MsPpt $src
Close-PptPresByPath $ppt $src

$inserted = Get-OutPath 'pptx-emf-inserted.pptx'
Close-PptPresByPath $ppt $inserted
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = 'slide:1'; file = $emfFile })
} 'pptx-emf-insert' | Out-Null
$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'pptx'
    scope = @{ pictures = $true }
} 'pptx-emf-inspect-insert'
$pic = @($tree1.slides[0].pictures) | Select-Object -First 1
$pres1 = Open-PptPres $ppt $inserted
$cap1 = Get-PptCaption $pres1
$s1 = Get-PptSlide $pres1 1
$n1 = Get-PptPictureCount $s1.Shapes
Release-Com $s1
Add-Check 'pptx-Pem1' (
    ($pic.contentType -eq 'image/x-emf') -and ($cap1 -notmatch 'Repair') -and ($n1 -ge 1)
) "ct=$($pic.contentType) caption=$cap1 n=$n1"
Close-PptPres $pres1

$exported = Get-OutPath 'pptx-emf-export.emf'
$expJson = (Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $inserted.Replace('\', '/')
    id = [string]$pic.id
    output = $exported.Replace('\', '/')
} 'pptx-emf-export') | ConvertFrom-Json
$expCt = [string]$expJson.contentType
$expBytes = [IO.File]::ReadAllBytes($exported)
$srcBytes = [IO.File]::ReadAllBytes($emf)
$same = ($expBytes.Length -eq $srcBytes.Length)
if ($same) {
    for ($i = 0; $i -lt $expBytes.Length; $i++) {
        if ($expBytes[$i] -ne $srcBytes[$i]) { $same = $false; break }
    }
}
Add-Check 'pptx-Pem2' (($expCt -eq 'image/x-emf') -and $same) "ct=$expCt len=$($expBytes.Length) same=$same"

$created = Get-OutPath 'pptx-emf-created.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $created.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(@{
            shapes = @(@{ text = '标题' })
            pictures = @(@{ file = $wmfFile })
        })
    }
} 'pptx-emf-create' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $created.Replace('\', '/'); format = 'pptx'
    scope = @{ pictures = $true }
} 'pptx-emf-inspect-create'
$picC = @($treeC.slides[0].pictures) | Select-Object -First 1
Close-PptPresByPath $ppt $created
$presC = Open-PptPres $ppt $created
$capC = Get-PptCaption $presC
Add-Check 'pptx-Pem3' (
    ($picC.contentType -eq 'image/x-wmf') -and ($capC -notmatch 'Repair')
) "ct=$($picC.contentType) caption=$capC"
Close-PptPres $presC

$replaced = Get-OutPath 'pptx-emf-replaced.pptx'
Close-PptPresByPath $ppt $replaced
if (Test-Path $replaced) { Remove-Item $replaced -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{ op = 'replacePicture'; id = [string]$pic.id; file = $pngB.Replace('\', '/') })
} 'pptx-emf-replace' | Out-Null
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'pptx'
    scope = @{ pictures = $true }
} 'pptx-emf-inspect-replace'
$picR = @($treeR.slides[0].pictures) | Select-Object -First 1
$hasCt = $picR.PSObject.Properties.Name -contains 'contentType'
$zipR = [IO.Compression.ZipFile]::OpenRead($replaced)
try {
    $emfLeft = @($zipR.Entries | Where-Object { $_.FullName -like '*.emf' }).Count -gt 0
} finally { $zipR.Dispose() }
$presR = Open-PptPres $ppt $replaced
$capR = Get-PptCaption $presR
Add-Check 'pptx-Pem4' (
    (-not $hasCt) -and (-not $emfLeft) -and ($capR -notmatch 'Repair')
) "hasCt=$hasCt emfLeft=$emfLeft caption=$capR"
Close-PptPres $presR

$notesOut = Get-OutPath 'pptx-emf-notes.pptx'
Close-PptPresByPath $ppt $notesOut
if (Test-Path $notesOut) { Remove-Item $notesOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $notesOut.Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = 'slide:1/notes'; file = $tiffFile })
} 'pptx-emf-notes-insert' | Out-Null
$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $notesOut.Replace('\', '/'); format = 'pptx'
    scope = @{ pictures = $true; notes = $true }
} 'pptx-emf-inspect-notes'
$nPic = @($treeN.slides[0].notes.pictures) | Select-Object -First 1
$presN = Open-PptPres $ppt $notesOut
$capN = Get-PptCaption $presN
Add-Check 'pptx-Pem5' (
    ($nPic.contentType -eq 'image/tiff') -and ($capN -notmatch 'Repair')
) "ct=$($nPic.contentType) caption=$capN"
Close-PptPres $presN

$junk = Get-OutPath 'pptx-emf-not-image.bin'
[IO.File]::WriteAllBytes($junk, [byte[]](1, 2, 3, 4))
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = (Get-OutPath 'pptx-emf-should-not.pptx').Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = 'slide:1'; file = $junk.Replace('\', '/') })
} 'pptx-emf-reject'
$failCode = $null
try { $failCode = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'pptx-Pem6' (($fail.ExitCode -ne 0) -and ($failCode -eq 'InvalidField')) "code=$failCode exit=$($fail.ExitCode)"
