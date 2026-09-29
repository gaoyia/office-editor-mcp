# pptx 批注内插图：docs/checklist-pptx.md 的 Pcip1–Pcip5。
# 不算放映引擎。不写 p:pic。create 不写批注内图。

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

function Get-PptErrorCode($raw) {
    try { return ($raw.Stderr | ConvertFrom-Json).error.code } catch { return $null }
}

function Get-ZipEntryText([string]$zipPath, [string]$entryName) {
    $zip = [IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        $e = $zip.GetEntry($entryName.Replace('\', '/'))
        if ($null -eq $e) { return $null }
        $sr = New-Object IO.StreamReader($e.Open(), [Text.Encoding]::UTF8)
        try { return $sr.ReadToEnd() }
        finally { $sr.Dispose() }
    }
    finally { $zip.Dispose() }
}

function List-Zip([string]$zipPath) {
    $zip = [IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        return @($zip.Entries | ForEach-Object { $_.FullName.Replace('\', '/') })
    }
    finally { $zip.Dispose() }
}

function Find-CommentPart([string]$pptx) {
    foreach ($n in (List-Zip $pptx)) {
        if ($n -like 'ppt/comments/*.xml' -and $n -notlike '*_rels*') { return $n }
    }
    return $null
}

$pngA = Ensure-OoxpPngA
$pngB = Ensure-OoxpPngB

$hostPath = Get-OutPath 'pptx-comment-pics-host.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(@{ shapes = @(@{ text = '页1' }) })
    }
} 'pptx-pcip-create' | Out-Null

$ppt = Ensure-MsPpt $hostPath
Close-PptPresByPath $ppt $hostPath

$cmOut = Get-OutPath 'pptx-comment-pics-comment.pptx'
Close-PptPresByPath $ppt $cmOut
if (Test-Path $cmOut) { Remove-Item -Force $cmOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $cmOut.Replace('\', '/')
    ops = @(@{ op = 'insertComment'; id = 'slide:1'; text = '批注甲'; author = 'Alice'; initials = 'A' })
} 'pptx-pcip-comment' | Out-Null

$treeCm = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cmOut.Replace('\', '/'); format = 'pptx'
    scope = @{ comments = $true }
} 'pptx-pcip-inspect-cm'
$cid = [string]$treeCm.slides[0].comments[0].id

$insOut = Get-OutPath 'pptx-comment-pics-insert.pptx'
Close-PptPresByPath $ppt $insOut
if (Test-Path $insOut) { Remove-Item -Force $insOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $cmOut.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = $cid; file = $pngA.Replace('\', '/') })
} 'pptx-pcip-insert' | Out-Null

$treeIns = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'pptx'
    scope = @{ comments = $true; pictures = $true }
} 'pptx-pcip-inspect-ins'
$cm = $treeIns.slides[0].comments[0]
$picId = [string]$cm.pictures[0].id
$pcip1ok = ([int]$treeIns.pictureCount -eq 0) -and ([int]$treeIns.slides[0].pictureCount -eq 0) `
    -and ([int]$cm.pictureCount -eq 1) -and ($picId -eq "$cid/pic:1")
Add-Check 'pptx-Pcip1' $pcip1ok "root=$($treeIns.pictureCount) slide=$($treeIns.slides[0].pictureCount) cm=$($cm.pictureCount) id=$picId"

$presIns = Open-PptPres $ppt $insOut
$capIns = Get-PptCaption $presIns
Close-PptPres $presIns
$part = Find-CommentPart $insOut
$xml = Get-ZipEntryText $insOut $part
$blipAfterTx = $xml -match '</p188:txBody><a:blip r:embed="'
$hasMedia = @((List-Zip $insOut) | Where-Object { $_ -like 'ppt/media/*' }).Count -ge 1
$pcip2ok = ($capIns -notmatch 'Repair') -and $blipAfterTx -and $hasMedia
Add-Check 'pptx-Pcip2' $pcip2ok "cap=$capIns blip=$blipAfterTx media=$hasMedia"

$repOut = Get-OutPath 'pptx-comment-pics-replace.pptx'
Close-PptPresByPath $ppt $repOut
if (Test-Path $repOut) { Remove-Item -Force $repOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $repOut.Replace('\', '/')
    ops = @(@{ op = 'replacePicture'; id = $picId; file = $pngB.Replace('\', '/') })
} 'pptx-pcip-replace' | Out-Null
$expB = Get-OutPath 'pptx-comment-pics-export.png'
if (Test-Path $expB) { Remove-Item -Force $expB }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $repOut.Replace('\', '/')
    id = $picId
    output = $expB.Replace('\', '/')
} 'pptx-pcip-export' | Out-Null
$bBytes = [IO.File]::ReadAllBytes($pngB)
$expBytes = if (Test-Path $expB) { [IO.File]::ReadAllBytes($expB) } else { @() }
$bytesOk = ($expBytes.Length -eq $bBytes.Length)
if ($bytesOk) {
    for ($i = 0; $i -lt $bBytes.Length; $i++) {
        if ($expBytes[$i] -ne $bBytes[$i]) { $bytesOk = $false; break }
    }
}
$presRep = Open-PptPres $ppt $repOut
$capRep = Get-PptCaption $presRep
Close-PptPres $presRep
$pcip3ok = ($capRep -notmatch 'Repair') -and $bytesOk
Add-Check 'pptx-Pcip3' $pcip3ok "cap=$capRep bytes=$bytesOk"

$delOut = Get-OutPath 'pptx-comment-pics-delete.pptx'
Close-PptPresByPath $ppt $delOut
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $repOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deletePicture'; id = $picId })
} 'pptx-pcip-delete' | Out-Null
$treeDel = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'pptx'
    scope = @{ comments = $true; pictures = $true }
} 'pptx-pcip-inspect-del'
$cmDel = $treeDel.slides[0].comments[0]
$xmlDel = Get-ZipEntryText $delOut (Find-CommentPart $delOut)
$noBlip = $xmlDel -notmatch '<a:blip\b'
$presDel = Open-PptPres $ppt $delOut
$capDel = Get-PptCaption $presDel
Close-PptPres $presDel
$pcip4ok = ($capDel -notmatch 'Repair') -and ([int]$cmDel.pictureCount -eq 0) `
    -and ([string]$cmDel.text -eq '批注甲') -and $noBlip
Add-Check 'pptx-Pcip4' $pcip4ok "cap=$capDel count=$($cmDel.pictureCount) text=$($cmDel.text) noBlip=$noBlip"

$xyOut = Get-OutPath 'pptx-comment-pics-xy.pptx'
if (Test-Path $xyOut) { Remove-Item -Force $xyOut }
$rawXy = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $cmOut.Replace('\', '/')
    output = $xyOut.Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = $cid; file = $pngA.Replace('\', '/'); x = 1; y = 1 })
} 'pptx-pcip-xy'
$codeXy = Get-PptErrorCode $rawXy

$szOut = Get-OutPath 'pptx-comment-pics-size.pptx'
if (Test-Path $szOut) { Remove-Item -Force $szOut }
$rawSz = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $szOut.Replace('\', '/')
    ops = @(@{ op = 'setPictureSize'; id = $picId; cx = 100; cy = 100 })
} 'pptx-pcip-size'
$codeSz = Get-PptErrorCode $rawSz

$xlsx = Get-OutPath 'pptx-comment-pics-fail.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'pptx-pcip-xlsx-create' | Out-Null
$xlsxOut = Get-OutPath 'pptx-comment-pics-xlsx-out.xlsx'
if (Test-Path $xlsxOut) { Remove-Item -Force $xlsxOut }
$rawXlsx = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $xlsxOut.Replace('\', '/')
    ops = @(@{ op = 'insertPicture'; id = $cid; file = $pngA.Replace('\', '/') })
} 'pptx-pcip-xlsx'
$codeXlsx = Get-PptErrorCode $rawXlsx

$pcip5ok = ($codeXy -eq 'InvalidField') -and (-not (Test-Path $xyOut)) `
    -and ($codeSz -eq 'InvalidId') -and (-not (Test-Path $szOut)) `
    -and ($codeXlsx -eq 'InvalidId') -and (-not (Test-Path $xlsxOut))
Add-Check 'pptx-Pcip5' $pcip5ok "xy=$codeXy size=$codeSz xlsx=$codeXlsx"
