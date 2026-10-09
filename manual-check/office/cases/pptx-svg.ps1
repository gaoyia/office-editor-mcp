# pptx 嵌入 SVG：docs/checklist-pptx.md 的 Pvg1–Pvg6。
# COM 认 Type=28（msoGraphic）。探测脚本不进本文件。

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

function Get-PptGraphicCount($hostShapes) {
    $n = 0
    try {
        for ($i = 1; $i -le $hostShapes.Count; $i++) {
            $sh = $hostShapes.Item($i)
            try {
                if ([int]$sh.Type -eq 28) { $n++ }
            } finally { Release-Com $sh }
        }
    } finally { Release-Com $hostShapes }
    return $n
}

$svg = Get-OutPath 'pptx-pic-probe.svg'
[IO.File]::WriteAllText($svg, @'
<svg xmlns="http://www.w3.org/2000/svg" width="96" height="48" viewBox="0 0 96 48">
  <rect width="96" height="48" fill="#3366cc"/>
  <circle cx="48" cy="24" r="16" fill="#ffcc00"/>
</svg>
'@, $global:OoxpUtf8)
$pngB = Ensure-OoxpPngB
$svgFile = $svg.Replace('\', '/')

$src = Get-OutPath 'pptx-svg-src.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/'); format = 'pptx'
    presentation = @{ slides = @(@{ shapes = @(@{ text = '标题' }) }) }
} 'pptx-svg-create-src' | Out-Null

$ppt = Ensure-MsPpt $src
Close-PptPresByPath $ppt $src

$inserted = Get-OutPath 'pptx-svg-inserted.pptx'
Close-PptPresByPath $ppt $inserted
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = 'slide:1'; file = $svgFile })
} 'pptx-svg-insert' | Out-Null
$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'pptx'
    scope = @{ pictures = $true }
} 'pptx-svg-inspect-insert'
$pic = @($tree1.slides[0].pictures) | Select-Object -First 1
$pres1 = Open-PptPres $ppt $inserted
$cap1 = Get-PptCaption $pres1
$s1 = Get-PptSlide $pres1 1
$n1 = Get-PptGraphicCount $s1.Shapes
Release-Com $s1
Add-Check 'pptx-Pvg1' (
    ($pic.contentType -eq 'image/svg+xml') -and ($cap1 -notmatch 'Repair') -and ($n1 -ge 1)
) "ct=$($pic.contentType) caption=$cap1 n=$n1"
Close-PptPres $pres1

$exported = Get-OutPath 'pptx-svg-export.svg'
$expJson = (Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $inserted.Replace('\', '/')
    id = [string]$pic.id
    output = $exported.Replace('\', '/')
} 'pptx-svg-export') | ConvertFrom-Json
$expCt = [string]$expJson.contentType
$expText = [IO.File]::ReadAllText($exported)
$srcText = [IO.File]::ReadAllText($svg)
Add-Check 'pptx-Pvg2' (($expCt -eq 'image/svg+xml') -and ($expText -eq $srcText)) "ct=$expCt len=$($expText.Length)"

$created = Get-OutPath 'pptx-svg-created.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $created.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(@{
            shapes = @(@{ text = '标题' })
            pictures = @(@{ file = $svgFile })
        })
    }
} 'pptx-svg-create' | Out-Null
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $created.Replace('\', '/'); format = 'pptx'
    scope = @{ pictures = $true }
} 'pptx-svg-inspect-create'
$picC = @($treeC.slides[0].pictures) | Select-Object -First 1
Close-PptPresByPath $ppt $created
$presC = Open-PptPres $ppt $created
$capC = Get-PptCaption $presC
Add-Check 'pptx-Pvg3' (
    ($picC.contentType -eq 'image/svg+xml') -and ($capC -notmatch 'Repair')
) "ct=$($picC.contentType) caption=$capC"
Close-PptPres $presC

$replaced = Get-OutPath 'pptx-svg-replaced.pptx'
Close-PptPresByPath $ppt $replaced
if (Test-Path $replaced) { Remove-Item $replaced -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $replaced.Replace('\', '/')
    ops = @(@{ op = 'replacePicture'; id = [string]$pic.id; file = $pngB.Replace('\', '/') })
} 'pptx-svg-replace' | Out-Null
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $replaced.Replace('\', '/'); format = 'pptx'
    scope = @{ pictures = $true }
} 'pptx-svg-inspect-replace'
$picR = @($treeR.slides[0].pictures) | Select-Object -First 1
$hasCt = $picR.PSObject.Properties.Name -contains 'contentType'
$zipR = [IO.Compression.ZipFile]::OpenRead($replaced)
try {
    $svgLeft = @($zipR.Entries | Where-Object { $_.FullName -like '*.svg' }).Count -gt 0
} finally { $zipR.Dispose() }
$presR = Open-PptPres $ppt $replaced
$capR = Get-PptCaption $presR
Add-Check 'pptx-Pvg4' (
    (-not $hasCt) -and (-not $svgLeft) -and ($capR -notmatch 'Repair')
) "hasCt=$hasCt svgLeft=$svgLeft caption=$capR"
Close-PptPres $presR

$notesOut = Get-OutPath 'pptx-svg-notes.pptx'
Close-PptPresByPath $ppt $notesOut
if (Test-Path $notesOut) { Remove-Item $notesOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $notesOut.Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = 'slide:1/notes'; file = $svgFile })
} 'pptx-svg-notes-insert' | Out-Null
$treeN = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $notesOut.Replace('\', '/'); format = 'pptx'
    scope = @{ pictures = $true; notes = $true }
} 'pptx-svg-inspect-notes'
$nPic = @($treeN.slides[0].notes.pictures) | Select-Object -First 1
$presN = Open-PptPres $ppt $notesOut
$capN = Get-PptCaption $presN
Add-Check 'pptx-Pvg5' (
    ($nPic.contentType -eq 'image/svg+xml') -and ($capN -notmatch 'Repair')
) "ct=$($nPic.contentType) caption=$capN"
Close-PptPres $presN

$junk = Get-OutPath 'pptx-not-image.bin'
[IO.File]::WriteAllBytes($junk, [byte[]](1, 2, 3, 4))
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = (Get-OutPath 'pptx-svg-should-not.pptx').Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = 'slide:1'; file = $junk.Replace('\', '/') })
} 'pptx-svg-reject'
$failCode = $null
try { $failCode = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
Add-Check 'pptx-Pvg6' (($fail.ExitCode -ne 0) -and ($failCode -eq 'InvalidField')) "code=$failCode exit=$($fail.ExitCode)"
