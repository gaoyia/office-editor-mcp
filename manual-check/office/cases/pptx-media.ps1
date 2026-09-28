# pptx 媒体：docs/checklist-pptx.md 的 Pmd1–Pmd5。
# 源文件：PowerPoint 亲手另存，第一页至少一段视频。

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

function Get-PptErrorCode($raw) {
    try { return ($raw.Stderr | ConvertFrom-Json).error.code } catch { return $null }
}

function Get-PptMediaCount($slide) {
    $n = 0
    $shapes = $slide.Shapes
    try {
        for ($i = 1; $i -le $shapes.Count; $i++) {
            $sh = $shapes.Item($i)
            try {
                if ([int]$sh.Type -eq 16) { $n++ }
            } finally { Release-Com $sh }
        }
    } finally { Release-Com $shapes }
    return $n
}

function Get-ZipEntryBytes([string]$pptx, [string]$part) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $z = [IO.Compression.ZipFile]::OpenRead($pptx)
    try {
        $norm = $part -replace '\\', '/'
        $e = $z.Entries | Where-Object { ($_.FullName -replace '\\', '/') -eq $norm } | Select-Object -First 1
        if (-not $e) { return $null }
        $ms = New-Object IO.MemoryStream
        $s = $e.Open()
        try { $s.CopyTo($ms) } finally { $s.Dispose() }
        return $ms.ToArray()
    } finally { $z.Dispose() }
}

$src = Ensure-PptxPptMediaSource
$mp4 = Ensure-OoxpMp4
$wav = Ensure-OoxpWav
$ppt = Ensure-MsPpt $src
Close-PptPresByPath $ppt $src

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
} 'pptx-media-inspect-list'
$slide0 = $tree0.slides[0]
$noList = -not ($slide0.PSObject.Properties.Name -contains 'videos')
$noAud = -not ($slide0.PSObject.Properties.Name -contains 'audios')
Add-Check 'pptx-Pmd1a' (($tree0.videoCount -ge 1) -and ($slide0.videoCount -ge 1) -and $noList -and $noAud) "wb=$($tree0.videoCount) slide=$($slide0.videoCount)"

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ media = $true; pictures = $true }
} 'pptx-media-inspect-expand'
$videos = @($tree1.slides[0].videos)
$idOk = ($videos.Count -ge 1) -and ($videos[0].id -like 'slide:1/video:*')
$notPic = [int]$tree1.pictureCount -eq 0 -or @($tree1.slides[0].pictures).Count -eq 0
# 源文件可能另有图；视频不得并进 pictures。
$picIds = @()
if ($tree1.slides[0].PSObject.Properties.Name -contains 'pictures') {
    $picIds = @($tree1.slides[0].pictures | ForEach-Object { [string]$_.id })
}
$cnv = [string]$videos[0].cNvPrId
$vidAsPic = 0
if ($cnv) {
    $suffix = "/pic:$cnv"
    $vidAsPic = @($picIds | Where-Object { $_.EndsWith($suffix) }).Count
}
Add-Check 'pptx-Pmd1' (($videos.Count -ge 1) -and $idOk -and ($vidAsPic -eq 0)) "n=$($videos.Count) id=$($videos[0].id) picHit=$vidAsPic"
$srcVid = $videos[0]

$exported = Get-OutPath 'pptx-media-exported.mp4'
if (Test-Path $exported) { Remove-Item $exported -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $src.Replace('\', '/')
    id = $srcVid.id
    output = $exported.Replace('\', '/')
} 'pptx-media-export' | Out-Null
$partName = [string]$srcVid.fileName
$zipBytes = Get-ZipEntryBytes $src ("ppt/media/$partName")
$expBytes = [IO.File]::ReadAllBytes($exported)
$bytesOk = ($null -ne $zipBytes) -and ($expBytes.Length -eq $zipBytes.Length)
if ($bytesOk) {
    for ($i = 0; $i -lt $expBytes.Length; $i++) {
        if ($expBytes[$i] -ne $zipBytes[$i]) { $bytesOk = $false; break }
    }
}
Add-Check 'pptx-Pmd2' $bytesOk "file=$partName exp=$($expBytes.Length) zip=$($zipBytes.Length)"

$created = Ensure-PptxCreate
$inserted = Get-OutPath 'pptx-media-inserted.pptx'
Close-PptPresByPath $ppt $inserted
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $created.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{
        op = 'insertVideo'
        id = 'slide:1'
        file = $mp4.Replace('\', '/')
    })
} 'pptx-media-insert-video' | Out-Null

$presI = Open-PptPres $ppt $inserted
$capI = Get-PptCaption $presI
$sI = Get-PptSlide $presI 1
$nI = Get-PptMediaCount $sI
Add-Check 'pptx-Pmd3a' (($capI -notmatch 'Repair') -and ($nI -ge 1)) "caption=$capI media=$nI"
Release-Com $sI
Close-PptPres $presI

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'pptx'
    scope = @{ media = $true }
} 'pptx-media-inspect-inserted'
$newVid = @($treeI.slides[0].videos) | Select-Object -First 1
Add-Check 'pptx-Pmd3' (($null -ne $newVid) -and ($newVid.id -like 'slide:1/video:*')) "id=$($newVid.id) count=$($treeI.videoCount)"

$withAudio = Get-OutPath 'pptx-media-audio.pptx'
Close-PptPresByPath $ppt $withAudio
if (Test-Path $withAudio) { Remove-Item $withAudio -Force }
$beforeAudio = [int]$treeI.audioCount
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $withAudio.Replace('\', '/')
    ops = @(@{
        op = 'insertAudio'
        id = 'slide:1'
        file = $wav.Replace('\', '/')
    })
} 'pptx-media-insert-audio' | Out-Null

$presA = Open-PptPres $ppt $withAudio
$capA = Get-PptCaption $presA
$sA = Get-PptSlide $presA 1
$nA = Get-PptMediaCount $sA
Release-Com $sA
Close-PptPres $presA
$treeA = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $withAudio.Replace('\', '/'); format = 'pptx'
    scope = @{ media = $true }
} 'pptx-media-inspect-audio'
Add-Check 'pptx-Pmd4' (($capA -notmatch 'Repair') -and ([int]$treeA.audioCount -eq ($beforeAudio + 1))) "caption=$capA audio=$($treeA.audioCount) com=$nA"

$delId = $newVid.id
$deleted = Get-OutPath 'pptx-media-deleted.pptx'
Close-PptPresByPath $ppt $deleted
if (Test-Path $deleted) { Remove-Item $deleted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $deleted.Replace('\', '/')
    ops = @(@{ op = 'deleteMedia'; id = $delId })
} 'pptx-media-delete' | Out-Null

$presD = Open-PptPres $ppt $deleted
$capD = Get-PptCaption $presD
$sD = Get-PptSlide $presD 1
$nD = Get-PptMediaCount $sD
Release-Com $sD
Close-PptPres $presD
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $deleted.Replace('\', '/'); format = 'pptx'
    scope = @{ media = $true }
} 'pptx-media-inspect-deleted'
$gone = [int]$treeD.videoCount -eq 0
Add-Check 'pptx-Pmd5a' (($capD -notmatch 'Repair') -and $gone -and ($nD -eq 0)) "caption=$capD video=$($treeD.videoCount) com=$nD"

$badIdOut = Get-OutPath 'pptx-media-bad-id.pptx'
if (Test-Path $badIdOut) { Remove-Item -Force $badIdOut }
$failId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $created.Replace('\', '/')
    output = $badIdOut.Replace('\', '/')
    ops = @(@{ op = 'insertVideo'; id = 'slide:1/notes'; file = $mp4.Replace('\', '/') })
} 'pptx-media-bad-id'
$codeId = Get-PptErrorCode $failId

$badWavOut = Get-OutPath 'pptx-media-bad-wav.pptx'
if (Test-Path $badWavOut) { Remove-Item -Force $badWavOut }
$failWav = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $created.Replace('\', '/')
    output = $badWavOut.Replace('\', '/')
    ops = @(@{ op = 'insertVideo'; id = 'slide:1'; file = $wav.Replace('\', '/') })
} 'pptx-media-bad-wav'
$codeWav = Get-PptErrorCode $failWav
$pmd5ok = ($codeId -eq 'InvalidId') -and (-not (Test-Path $badIdOut)) `
    -and ($codeWav -eq 'InvalidField') -and (-not (Test-Path $badWavOut))
Add-Check 'pptx-Pmd5' $pmd5ok "id=$codeId wav=$codeWav wroteId=$(Test-Path $badIdOut) wroteWav=$(Test-Path $badWavOut)"
