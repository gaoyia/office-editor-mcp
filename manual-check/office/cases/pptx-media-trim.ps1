# pptx 媒体 trim、书签与字幕：docs/checklist-pptx.md 的 Mt1–Mt5。

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

function Get-ZipEntryNames([string]$path) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $z = [IO.Compression.ZipFile]::OpenRead($path)
    try {
        return @($z.Entries | ForEach-Object { ($_.FullName -replace '\\', '/') })
    }
    finally { $z.Dispose() }
}

function Get-PptFirstMediaInfo($slide) {
    $shapes = $slide.Shapes
    $shape = $null
    $fmt = $null
    $bm = $null
    $info = $null
    try {
        $count = [int]$shapes.Count
        for ($i = 1; $i -le $count; $i++) {
            $s = $shapes.Item($i)
            if ([int]$s.Type -eq 16) {
                $shape = $s
                break
            }
            Release-Com $s
        }
        if (-not $shape) { return $null }
        $fmt = $shape.MediaFormat
        $bm = $fmt.MediaBookmarks
        $info = @{
            StartPoint    = [double]$fmt.StartPoint
            EndPoint      = [double]$fmt.EndPoint
            Length        = [double]$fmt.Length
            BookmarkCount = [int]$bm.Count
        }
    }
    finally {
        if ($bm) { Release-Com $bm }
        if ($fmt) { Release-Com $fmt }
        if ($shape) { Release-Com $shape }
        Release-Com $shapes
    }
    return $info
}

$hostPath = Ensure-PptxCreate
$mp4 = Ensure-OoxpMp4
$ppt = Ensure-MsPpt $hostPath
$vtt = Get-OutPath 'pptx-mt-en.vtt'
$utf8 = New-Object System.Text.UTF8Encoding $false
[IO.File]::WriteAllText($vtt, "WEBVTT`n`n00:00:00.000 --> 00:00:00.800`nhello`n", $utf8)

$inserted = Get-OutPath 'pptx-mt-inserted.pptx'
Close-PptPresByPath $ppt $inserted
if (Test-Path $inserted) { Remove-Item -Force $inserted }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{ op = 'insertVideo'; id = 'slide:1'; file = $mp4.Replace('\', '/') })
} 'pptx-mt-insert' | Out-Null

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'pptx'
    scope = @{ media = $true }
} 'pptx-mt-inspect-inserted'
$vidId = [string]$treeI.slides[0].videos[0].id

$setOut = Get-OutPath 'pptx-mt-set.pptx'
Close-PptPresByPath $ppt $setOut
if (Test-Path $setOut) { Remove-Item -Force $setOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $setOut.Replace('\', '/')
    ops = @(@{
        op = 'setMedia'
        id = $vidId
        st = 100
        end = 200
        bookmarks = @(@{ name = 'MarkA'; time = 300 })
        tracks = @(@{ label = 'English'; lang = 'en'; file = $vtt.Replace('\', '/') })
    })
} 'pptx-mt-set' | Out-Null

$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $setOut.Replace('\', '/'); format = 'pptx'
    scope = @{ media = $true }
} 'pptx-mt-inspect-set'
$v = $treeS.slides[0].videos[0]
$xmlS = Get-ZipPartText $setOut 'ppt/slides/slide1.xml'
$ctsS = Get-ZipPartText $setOut '[Content_Types].xml'
$namesS = Get-ZipEntryNames $setOut
$hasVtt = @($namesS | Where-Object { $_ -like 'ppt/media/*.vtt' }).Count -ge 1

$presS = Open-PptPres $ppt $setOut
$capS = Get-PptCaption $presS
$sS = Get-PptSlide $presS 1
$infoS = Get-PptFirstMediaInfo $sS
Release-Com $sS
Close-PptPres $presS

$stOk = [int][math]::Round([double]$v.st) -eq 100
$endOk = [int][math]::Round([double]$v.end) -eq 200
$trimXml = $xmlS -match '<p14:trim[^>]*st="100"' -and $xmlS -match '<p14:trim[^>]*end="200"'
$sp = if ($infoS) { [int][math]::Round($infoS.StartPoint) } else { -1 }
$ep = if ($infoS) { [int][math]::Round($infoS.EndPoint) } else { -1 }
$len = if ($infoS) { [int][math]::Round($infoS.Length) } else { -1 }
$mt1ok = ($capS -notmatch 'Repair') -and $stOk -and $endOk -and $trimXml `
    -and ($sp -eq 100) -and ($ep -eq 800) -and ($len -eq 1000)
Add-Check 'pptx-Mt1' $mt1ok "cap=$capS st=$($v.st) end=$($v.end) sp=$sp ep=$ep len=$len"

$bmkOk = ([string]$v.bookmarks[0].name -eq 'MarkA') -and ([int][math]::Round([double]$v.bookmarks[0].time) -eq 300)
$bmkXml = $xmlS -match 'name="MarkA"' -and $xmlS -match 'time="300"'
$bmCount = if ($infoS) { [int]$infoS.BookmarkCount } else { -1 }
$mt2ok = ($capS -notmatch 'Repair') -and $bmkOk -and $bmkXml -and ($bmCount -eq 1)
Add-Check 'pptx-Mt2' $mt2ok "cap=$capS name=$($v.bookmarks[0].name) time=$($v.bookmarks[0].time) bm=$bmCount"

$trkOk = ([string]$v.tracks[0].label -eq 'English') -and ([string]$v.tracks[0].fileName -like '*.vtt')
$mt3ok = ($capS -notmatch 'Repair') -and $trkOk -and ($xmlS -match 'tracksInfo') `
    -and ($ctsS -match 'text/vtt') -and $hasVtt
Add-Check 'pptx-Mt3' $mt3ok "cap=$capS label=$($v.tracks[0].label) file=$($v.tracks[0].fileName) vtt=$hasVtt"

$clrOut = Get-OutPath 'pptx-mt-clear.pptx'
Close-PptPresByPath $ppt $clrOut
if (Test-Path $clrOut) { Remove-Item -Force $clrOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $setOut.Replace('\', '/')
    output = $clrOut.Replace('\', '/')
    ops = @(@{
        op = 'setMedia'
        id = $vidId
        st = 0
        end = 0
        bookmarks = @()
        tracks = @()
    })
} 'pptx-mt-clear' | Out-Null
$xmlC = Get-ZipPartText $clrOut 'ppt/slides/slide1.xml'
$presC = Open-PptPres $ppt $clrOut
$capC = Get-PptCaption $presC
Close-PptPres $presC
$mt4ok = ($capC -notmatch 'Repair') -and ($xmlC -notmatch '<p14:trim') `
    -and ($xmlC -notmatch 'bmkLst') -and ($xmlC -notmatch 'tracksInfo')
Add-Check 'pptx-Mt4' $mt4ok "cap=$capC trim=$($xmlC -match '<p14:trim') bmk=$($xmlC -match 'bmkLst') trk=$($xmlC -match 'tracksInfo')"

$badId = Get-OutPath 'pptx-mt-fail-id.pptx'
$badEmpty = Get-OutPath 'pptx-mt-fail-empty.pptx'
$badDup = Get-OutPath 'pptx-mt-fail-dup.pptx'
Close-PptPresByPath $ppt $badId
Close-PptPresByPath $ppt $badEmpty
Close-PptPresByPath $ppt $badDup
if (Test-Path $badId) { Remove-Item -Force $badId }
if (Test-Path $badEmpty) { Remove-Item -Force $badEmpty }
if (Test-Path $badDup) { Remove-Item -Force $badDup }
$failId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $badId.Replace('\', '/')
    ops = @(@{ op = 'setMedia'; id = 'slide:1'; st = 100 })
} 'pptx-mt-fail-id'
$failEmpty = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $badEmpty.Replace('\', '/')
    ops = @(@{ op = 'setMedia'; id = $vidId })
} 'pptx-mt-fail-empty'
$failDup = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $badDup.Replace('\', '/')
    ops = @(@{
        op = 'setMedia'
        id = $vidId
        bookmarks = @(
            @{ name = 'A'; time = 1 }
            @{ name = 'A'; time = 2 }
        )
    })
} 'pptx-mt-fail-dup'
$codeId = Get-PptErrorCode $failId
$codeEmpty = Get-PptErrorCode $failEmpty
$codeDup = Get-PptErrorCode $failDup
$mt5ok = ($codeId -eq 'InvalidId') -and (-not (Test-Path $badId)) `
    -and ($codeEmpty -eq 'InvalidField') -and (-not (Test-Path $badEmpty)) `
    -and ($codeDup -eq 'InvalidField') -and (-not (Test-Path $badDup))
Add-Check 'pptx-Mt5' $mt5ok "id=$codeId empty=$codeEmpty dup=$codeDup wroteId=$(Test-Path $badId) wroteEmpty=$(Test-Path $badEmpty) wroteDup=$(Test-Path $badDup)"
