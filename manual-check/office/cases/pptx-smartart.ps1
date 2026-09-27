# pptx 幻灯片 SmartArt：docs/checklist-pptx.md 的 Psa1–Psa7。
# 源文件：PowerPoint 亲手另存列表型。不要用库 create 产物当唯一样例。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-PptxPptSmartArtSource
$ppt = Ensure-MsPpt $src
Close-PptPresByPath $ppt $src

function Test-OrgFamily([string]$uid) {
    if ([string]::IsNullOrEmpty($uid)) { return $false }
    return ($uid -split '/')[-1].ToLowerInvariant().StartsWith('orgchart')
}

function Find-ZipPartContaining([string]$zipPath, [string]$prefix, [string]$needle) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($zipPath)
    try {
        foreach ($e in $zip.Entries) {
            $n = $e.FullName -replace '\\', '/'
            if ($n -like $prefix) {
                $ms = New-Object IO.MemoryStream
                $es = $e.Open()
                $es.CopyTo($ms)
                $es.Dispose()
                $text = [Text.Encoding]::UTF8.GetString($ms.ToArray())
                if ($text.Contains($needle)) { return $n }
            }
        }
    }
    finally { $zip.Dispose() }
    return $null
}

function Rewrite-ZipPart([string]$zipPath, [string]$part, [scriptblock]$edit) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::Open($zipPath, 'Update')
    try {
        $entry = $zip.Entries | Where-Object { ($_.FullName -replace '\\', '/') -eq $part } | Select-Object -First 1
        if (-not $entry) { throw "zip 里没有 $part" }
        $ms = New-Object IO.MemoryStream
        $es = $entry.Open()
        $es.CopyTo($ms)
        $es.Dispose()
        $text = [Text.Encoding]::UTF8.GetString($ms.ToArray())
        $text2 = & $edit $text
        $name = $entry.FullName
        $entry.Delete()
        $newEntry = $zip.CreateEntry($name)
        $bytes = [Text.Encoding]::UTF8.GetBytes($text2)
        $ws = $newEntry.Open()
        $ws.Write($bytes, 0, $bytes.Length)
        $ws.Dispose()
    }
    finally { $zip.Dispose() }
}

function Normalize-SaText([string]$t) {
    if ($null -eq $t) { return '' }
    return $t.Trim().TrimEnd([char]13, [char]10, [char]7, [char]11)
}

function Get-PptSmartArtShapes($slide) {
    $out = New-Object System.Collections.ArrayList
    $shapes = $slide.Shapes
    try {
        $n = [int]$shapes.Count
        for ($i = 1; $i -le $n; $i++) {
            $shp = $shapes.Item($i)
            $keep = $false
            try {
                $has = $false
                try { $has = [bool]$shp.HasSmartArt } catch {
                    $has = ([int]$shp.Type -eq 24)
                }
                if ($has) { $keep = $true }
            } catch {}
            if ($keep) { [void]$out.Add($shp) } else { Release-Com $shp }
        }
    }
    finally { Release-Com $shapes }
    return @($out)
}

function Get-SmartArtTexts($shp) {
    $sa = $null
    try { $sa = $shp.SmartArt } catch { return @() }
    $out = @()
    try {
        $nodes = $sa.AllNodes
        try {
            $n = [int]$nodes.Count
            for ($i = 1; $i -le $n; $i++) {
                $node = $nodes.Item($i)
                try {
                    $tf = $node.TextFrame2
                    try {
                        $tr = $tf.TextRange
                        try { $out += (Normalize-SaText ([string]$tr.Text)) } finally { Release-Com $tr }
                    }
                    finally { Release-Com $tf }
                }
                finally { Release-Com $node }
            }
        }
        finally { Release-Com $nodes }
    }
    finally { Release-Com $sa }
    return $out
}

function Release-SmartArtShapes($items) {
    foreach ($shp in @($items)) { Release-Com $shp }
}

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

function Get-ZipEntryText([string]$pptx, [string]$name) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $z = [IO.Compression.ZipFile]::OpenRead($pptx)
    try {
        $want = $name -replace '\\', '/'
        $e = $z.Entries | Where-Object { ($_.FullName -replace '\\', '/') -eq $want } | Select-Object -First 1
        if (-not $e) { return '' }
        $sr = New-Object IO.StreamReader($e.Open())
        try { return $sr.ReadToEnd() } finally { $sr.Dispose() }
    }
    finally { $z.Dispose() }
}

function Count-Fallback([string]$xml) {
    if ([string]::IsNullOrEmpty($xml)) { return 0 }
    return [regex]::Matches($xml, 'Fallback').Count
}

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ smartArt = $true }
} 'pptx-smartart-inspect'
$slide1 = $tree1.slides[0]
$arts = @()
if ($slide1.PSObject.Properties.Name -contains 'smartArts') { $arts = @($slide1.smartArts) }
$list = $arts | Select-Object -First 1
$pts = if ($list) { @($list.points) } else { @() }
$known = @($S.saItem1, $S.saItem2, $S.saItem3)
$knownInInspect = 0
foreach ($p in $pts) {
    $t = Normalize-SaText ([string]$p.text)
    if ($known -contains $t) { $knownInInspect++ }
}
$idOk = ($null -ne $list) -and ([string]$list.id -like 'slide:1/smartdiagram:*')

$presSrc = Open-PptPres $ppt $src
$sSrc = Get-PptSlide $presSrc 1
$comShapes = Get-PptSmartArtShapes $sSrc
$comMatch = $false
$comDetail = "pptN=$(@($comShapes).Count)"
try {
    foreach ($shp in $comShapes) {
        $ct = @(Get-SmartArtTexts $shp)
        $hit = 0
        foreach ($t in $known) { if ($ct -contains $t) { $hit++ } }
        if ($hit -ge 3) { $comMatch = $true }
    }
} catch {
    $comDetail = $_.Exception.Message
}
Release-SmartArtShapes $comShapes
Release-Com $sSrc
Close-PptPres $presSrc
Add-Check 'pptx-Psa1' ($idOk -and ($pts.Count -ge 3) -and ($knownInInspect -ge 3) -and $comMatch) "id=$($list.id) $comDetail inspect=$knownInInspect"

$listId = if ($list) { $list.id } else { 'slide:1/smartdiagram:3' }
$pt2 = if ($pts.Count -ge 2) { $pts[1].id } else { "$listId/pt:2" }

# Psa2：改第二个可见点。
$textOut = Get-OutPath 'pptx-smartart-text.pptx'
Close-PptPresByPath $ppt $textOut
if (Test-Path $textOut) { Remove-Item $textOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $textOut.Replace('\', '/')
    ops = @(@{ op = 'setSmartArtText'; id = $pt2; text = $S.saItemNew })
} 'pptx-smartart-set-text' | Out-Null

$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $textOut.Replace('\', '/'); format = 'pptx'
    scope = @{ smartArt = $true }
} 'pptx-smartart-text-inspect'
$listT = @($treeT.slides[0].smartArts) | Where-Object { $_.id -eq $listId } | Select-Object -First 1
$gotT = if ($listT) { Normalize-SaText ([string]@($listT.points)[1].text) } else { '' }
$presT = Open-PptPres $ppt $textOut
$capT = Get-PptCaption $presT
$sT = Get-PptSlide $presT 1
$wordNew = $false
foreach ($shp in (Get-PptSmartArtShapes $sT)) {
    try {
        $ct = @(Get-SmartArtTexts $shp)
        if ($ct -contains $S.saItemNew) { $wordNew = $true }
    } catch {}
    Release-Com $shp
}
Release-Com $sT
Close-PptPres $presT
Add-Check 'pptx-Psa2' (($capT -notmatch 'Repair') -and ($gotT -eq $S.saItemNew) -and $wordNew) "caption=$capT inspect=$gotT ppt=$wordNew"

# Psa3：增删点后条数对。
$addOut = Get-OutPath 'pptx-smartart-addpt.pptx'
Close-PptPresByPath $ppt $addOut
if (Test-Path $addOut) { Remove-Item $addOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $textOut.Replace('\', '/')
    output = $addOut.Replace('\', '/')
    ops = @(@{ op = 'addSmartArtPoint'; id = $listId; text = $S.saPointNew })
} 'pptx-smartart-add-point' | Out-Null

$treeA = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $addOut.Replace('\', '/'); format = 'pptx'
    scope = @{ smartArt = $true }
} 'pptx-smartart-add-inspect'
$listA = @($treeA.slides[0].smartArts) | Where-Object { $_.id -eq $listId } | Select-Object -First 1
$ptsA = if ($listA) { @($listA.points) } else { @() }
$addedId = $null
foreach ($p in $ptsA) {
    if ((Normalize-SaText ([string]$p.text)) -eq $S.saPointNew) { $addedId = $p.id; break }
}
$nA = $ptsA.Count

$delPtOut = Get-OutPath 'pptx-smartart-delpt.pptx'
Close-PptPresByPath $ppt $delPtOut
if (Test-Path $delPtOut) { Remove-Item $delPtOut -Force }
if ($addedId) {
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $addOut.Replace('\', '/')
        output = $delPtOut.Replace('\', '/')
        ops = @(@{ op = 'deleteSmartArtPoint'; id = $addedId })
    } 'pptx-smartart-del-point' | Out-Null
} else {
    Copy-Item $addOut $delPtOut
}
$treeP = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delPtOut.Replace('\', '/'); format = 'pptx'
    scope = @{ smartArt = $true }
} 'pptx-smartart-delpt-inspect'
$listP = @($treeP.slides[0].smartArts) | Where-Object { $_.id -eq $listId } | Select-Object -First 1
$ptsP = if ($listP) { @($listP.points) } else { @() }
$stillNew = @($ptsP | Where-Object { (Normalize-SaText ([string]$_.text)) -eq $S.saPointNew }).Count
$presP = Open-PptPres $ppt $delPtOut
$capP = Get-PptCaption $presP
$sP = Get-PptSlide $presP 1
$pptN = 0
foreach ($shp in (Get-PptSmartArtShapes $sP)) {
    try { $pptN = @(Get-SmartArtTexts $shp).Count } catch {}
    Release-Com $shp
}
Release-Com $sP
Close-PptPres $presP
Add-Check 'pptx-Psa3' (($capP -notmatch 'Repair') -and ($nA -eq ($pts.Count + 1)) -and ($ptsP.Count -eq $pts.Count) -and ($stillNew -eq 0)) "caption=$capP add=$nA del=$($ptsP.Count) pptN=$pptN"

# Psa4：delete 整张后 Fallback 也不在。
$srcXml = Get-ZipEntryText $textOut 'ppt/slides/slide1.xml'
$fb0 = Count-Fallback $srcXml
$delOut = Get-OutPath 'pptx-smartart-delete.pptx'
Close-PptPresByPath $ppt $delOut
if (Test-Path $delOut) { Remove-Item $delOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $textOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteSmartArt'; id = $listId })
} 'pptx-smartart-delete' | Out-Null

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'pptx'
    scope = @{ smartArt = $true }
} 'pptx-smartart-delete-inspect'
$stillList = @($treeD.slides[0].smartArts) | Where-Object { $_.id -eq $listId } | Select-Object -First 1
$delXml = Get-ZipEntryText $delOut 'ppt/slides/slide1.xml'
$fb1 = Count-Fallback $delXml
$presD = Open-PptPres $ppt $delOut
$capD = Get-PptCaption $presD
$sD = Get-PptSlide $presD 1
$pptStill = $false
foreach ($shp in (Get-PptSmartArtShapes $sD)) {
    try {
        $ct = @(Get-SmartArtTexts $shp)
        if ($ct -contains $S.saItem1 -or $ct -contains $S.saItemNew) { $pptStill = $true }
    } catch {}
    Release-Com $shp
}
Release-Com $sD
Close-PptPres $presD
$fbOk = ($fb0 -eq 0) -or ($fb1 -lt $fb0)
Add-Check 'pptx-Psa4' (($capD -notmatch 'Repair') -and ($null -eq $stillList) -and $fbOk -and (-not $pptStill)) "caption=$capD still=$($stillList.id) fb=$fb0->$fb1 pptStill=$pptStill"

# Psa5：create 带 list，打开无修复，能看见 SmartArt。
$createPath = Ensure-PptxCreateSmartArt
Close-PptPresByPath $ppt $createPath
$treeC = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $createPath.Replace('\', '/'); format = 'pptx'
    scope = @{ smartArt = $true }
} 'pptx-create-smartart-inspect'
$cArt = @($treeC.slides[0].smartArts) | Select-Object -First 1
$presC = Open-PptPres $ppt $createPath
$capC = Get-PptCaption $presC
$sC = Get-PptSlide $presC 1
$shpsC = Get-PptSmartArtShapes $sC
$nC = @($shpsC).Count
Release-SmartArtShapes $shpsC
Release-Com $sC
Close-PptPres $presC
Add-Check 'pptx-Psa5' (($capC -notmatch 'Repair') -and ($null -ne $cArt) -and ($cArt.id -eq 'slide:1/smartdiagram:3') -and ($nC -ge 1)) "caption=$capC id=$($cArt.id) ppt=$nC"

# Psa6：列表改成组织图，文字还在，无修复。
$comboOut = Get-OutPath 'pptx-smartart-layout.pptx'
if (Test-Path $comboOut) { Remove-Item -Force $comboOut }
Close-PptPresByPath $ppt $comboOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $comboOut.Replace('\', '/')
    ops = @(@{ op = 'setSmartArtLayout'; id = $listId; layout = 'org' })
} 'pptx-smartart-layout-org' | Out-Null
$treeL = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $comboOut.Replace('\', '/'); format = 'pptx'
    scope = @{ smartArt = $true }
} 'pptx-smartart-layout-inspect'
$after = @($treeL.slides[0].smartArts) | Where-Object { $_.id -eq $listId } | Select-Object -First 1
$layOk = $after -and (Test-OrgFamily ([string]$after.layout))
$ptsAfter = if ($after) { @($after.points) } else { @() }
$textOk = $ptsAfter.Count -ge 3
$inspectKeep = @($known | Where-Object { @($ptsAfter | ForEach-Object { Normalize-SaText ([string]$_.text) }) -contains $_ }).Count -ge 3
$presF = Open-PptPres $ppt $comboOut
$capF = Get-PptCaption $presF
$sF = Get-PptSlide $presF 1
$comKeep = $false
foreach ($shp in (Get-PptSmartArtShapes $sF)) {
    try {
        $ct = @(Get-SmartArtTexts $shp)
        if ((@($known | Where-Object { $ct -contains $_ }).Count) -ge 3) { $comKeep = $true }
    } catch {}
    Release-Com $shp
}
Release-Com $sF
Close-PptPres $presF
Add-Check 'pptx-Psa6' (($capF -notmatch 'Repair') -and $layOk -and $textOk -and $inspectKeep -and $comKeep) "caption=$capF layout=$($after.layout) pts=$($ptsAfter.Count) inspectKeep=$inspectKeep comKeep=$comKeep"

# Psa7：库外 uniqueId 仍拒绝。
$unk = Get-OutPath 'pptx-smartart-unknown.pptx'
Copy-Item -Force $src $unk
$unkOut = Get-OutPath 'pptx-smartart-unknown-out.pptx'
if (Test-Path $unkOut) { Remove-Item -Force $unkOut }
$layUid = [string]$list.layout
$layPart = Find-ZipPartContaining $unk '*diagrams/layout*.xml' $layUid
if ($layPart) {
    Rewrite-ZipPart $unk $layPart {
        param($t)
        [regex]::Replace($t, 'uniqueId="[^"]+"', 'uniqueId="urn:microsoft.com/office/officeart/2005/8/layout/pyramid1"', 1)
    }
}
$raw = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $unk.Replace('\', '/')
    output = $unkOut.Replace('\', '/')
    ops = @(@{ op = 'setSmartArtLayout'; id = $listId; layout = 'org' })
} 'pptx-smartart-layout-unknown'
$unkCode = ''
try { $unkCode = [string](($raw.Stderr | ConvertFrom-Json).error.code) } catch { $unkCode = $raw.Stderr }
$noWrite = -not (Test-Path $unkOut)
Add-Check 'pptx-Psa7' (($null -ne $layPart) -and ($unkCode -eq 'NotImplemented') -and $noWrite) "part=$layPart code=$unkCode wrote=$(-not $noWrite)"
