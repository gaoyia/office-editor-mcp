# docx SmartArt：docs/checklist-docx.md 的 Sa1–Sa8。
# 源文件由 Word AddSmartArt 另存列表型和循环型。不要用 create 产物当唯一样例。
# 增删点和改尺寸先打在 Word 亲手插的列表上，再插库内列表、删掉 Word 那张（才能看到 Fallback 一起走）。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-DocxWordSmartArtSource
$word = Ensure-MsWord $src

function Test-ListFamily([string]$uid) {
    if ([string]::IsNullOrEmpty($uid)) { return $false }
    $tail = ($uid -split '/')[-1].ToLowerInvariant()
    return ($tail -eq 'default') -or $tail.StartsWith('hlist') -or $tail.StartsWith('vlist')
}

function Test-OrgFamily([string]$uid) {
    if ([string]::IsNullOrEmpty($uid)) { return $false }
    return ($uid -split '/')[-1].ToLowerInvariant().StartsWith('orgchart')
}

function Normalize-SaText([string]$t) {
    if ($null -eq $t) { return '' }
    return $t.Trim().TrimEnd([char]13, [char]10, [char]7, [char]11)
}

function Get-WordSmartArtShapes($doc) {
    $out = New-Object System.Collections.ArrayList
    $shapes = $doc.Shapes
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
    $inlines = $doc.InlineShapes
    try {
        $n = [int]$inlines.Count
        for ($i = 1; $i -le $n; $i++) {
            $ish = $inlines.Item($i)
            $keep = $false
            try {
                $has = $false
                try { $has = [bool]$ish.HasSmartArt } catch {}
                if ($has) { $keep = $true }
            } catch {}
            if ($keep) { [void]$out.Add($ish) } else { Release-Com $ish }
        }
    }
    finally { Release-Com $inlines }
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

function Get-ZipEntryText([string]$docx, [string]$name) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $z = [IO.Compression.ZipFile]::OpenRead($docx)
    try {
        $entry = $z.Entries | Where-Object { ($_.FullName -replace '\\', '/') -eq $name } | Select-Object -First 1
        if (-not $entry) { return $null }
        $sr = New-Object IO.StreamReader($entry.Open(), [Text.Encoding]::UTF8)
        try { return $sr.ReadToEnd() } finally { $sr.Dispose() }
    }
    finally { $z.Dispose() }
}

function Count-Fallback([string]$xml) {
    if ([string]::IsNullOrEmpty($xml)) { return 0 }
    return ([regex]::Matches($xml, '<mc:Fallback[\s>]')).Count
}

function Find-Diagram($tree, [scriptblock]$pred) {
    foreach ($d in @($tree.smartArts)) {
        if (& $pred $d) { return $d }
    }
    return $null
}

# Sa1：无 smartArt 时只有计数；smartArt:true 时 id 与点文本能对上 COM。
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-smartart-inspect-counts'
$noList = -not ($tree0.PSObject.Properties.Name -contains 'smartArts')
Add-Check 'docx-Sa1a' (($tree0.smartArtCount -ge 1) -and $noList) "count=$($tree0.smartArtCount) noList=$noList"

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; smartArt = $true }
} 'docx-smartart-inspect-lists'
$list = Find-Diagram $tree1 { param($d) Test-ListFamily ([string]$d.layout) }
$cycle = Find-Diagram $tree1 { param($d) -not (Test-ListFamily ([string]$d.layout)) -and -not (Test-OrgFamily ([string]$d.layout)) }
if (-not $list) { $list = @($tree1.smartArts) | Select-Object -First 1 }
$idOk = $list -and ([string]$list.id -like 'para:*/smartdiagram:*')
$pts = if ($list) { @($list.points) } else { @() }
$inspectTexts = @($pts | ForEach-Object { Normalize-SaText ([string]$_.text) })
$known = @($S.saItem1, $S.saItem2, $S.saItem3)
$knownInInspect = @($known | Where-Object { $inspectTexts -contains $_ }).Count

$docSrc = Open-WordDoc $word $src
$comShapes = Get-WordSmartArtShapes $docSrc
$comTexts = @()
$comMatch = $false
$comDetail = "wordSmartArt=$($comShapes.Count)"
foreach ($shp in $comShapes) {
    try {
        $ct = @(Get-SmartArtTexts $shp)
        if ($ct.Count -gt $comTexts.Count) { $comTexts = $ct }
        $hit = @($known | Where-Object { $ct -contains $_ }).Count
        if ($hit -ge 3) {
            $comMatch = $true
            $comDetail = "com=$($ct -join ',') inspect=$($inspectTexts -join ',')"
        }
        elseif (-not $comMatch) {
            $comDetail = "com=$($ct -join ',') inspect=$($inspectTexts -join ',')"
        }
    } catch {
        $comDetail = $_.Exception.Message
    }
}
Release-SmartArtShapes $comShapes
Close-WordDoc $docSrc
Add-Check 'docx-Sa1' ($idOk -and ($pts.Count -ge 3) -and ($knownInInspect -ge 3) -and $comMatch) "id=$($list.id) layout=$($list.layout) $comDetail cycle=$($cycle.layout)"

$para1 = $tree1.paragraphs[0].id
$listId = $list.id
$pt2 = if ($pts.Count -ge 2) { $pts[1].id } else { "$listId/pt:2" }
$nPts0 = $pts.Count

# Sa2：改第二个可见点。
$textOut = Get-OutPath 'docx-smartart-text.docx'
Close-WordDocByPath $word $textOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $textOut.Replace('\', '/')
    ops = @(@{ op = 'setSmartArtText'; id = $pt2; text = $S.saItemNew })
} 'docx-smartart-set-text' | Out-Null

$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $textOut.Replace('\', '/'); format = 'docx'
    scope = @{ smartArt = $true }
} 'docx-smartart-text-inspect'
$listT = @($treeT.smartArts) | Where-Object { $_.id -eq $listId } | Select-Object -First 1
$gotT = if ($listT) { Normalize-SaText ([string]@($listT.points)[1].text) } else { '' }
$docT = Open-WordDoc $word $textOut
$capT = [string]$docT.ActiveWindow.Caption
$wordNew = $false
foreach ($shp in (Get-WordSmartArtShapes $docT)) {
    try {
        $ct = @(Get-SmartArtTexts $shp)
        if ($ct -contains $S.saItemNew) { $wordNew = $true }
    } catch {}
    Release-Com $shp
}
Close-WordDoc $docT
Add-Check 'docx-Sa2' (($capT -notmatch 'Repair') -and ($gotT -eq $S.saItemNew) -and $wordNew) "caption=$capT inspect=$gotT word=$wordNew"

# Sa5：在 Word 列表上加点（必须在删整张之前）。
$addOut = Get-OutPath 'docx-smartart-addpt.docx'
Close-WordDocByPath $word $addOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $textOut.Replace('\', '/')
    output = $addOut.Replace('\', '/')
    ops = @(@{ op = 'addSmartArtPoint'; id = $listId; text = $S.saPointNew })
} 'docx-smartart-add-point' | Out-Null

$treeA = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $addOut.Replace('\', '/'); format = 'docx'
    scope = @{ smartArt = $true }
} 'docx-smartart-add-inspect'
$listA = @($treeA.smartArts) | Where-Object { $_.id -eq $listId } | Select-Object -First 1
$ptsA = if ($listA) { @($listA.points) } else { @() }
$addedId = $null
foreach ($p in $ptsA) {
    if ((Normalize-SaText ([string]$p.text)) -eq $S.saPointNew) { $addedId = $p.id; break }
}
$docA = Open-WordDoc $word $addOut
$capA = [string]$docA.ActiveWindow.Caption
$wordAdd = $false
$wordAddN = 0
foreach ($shp in (Get-WordSmartArtShapes $docA)) {
    try {
        $ct = @(Get-SmartArtTexts $shp)
        if ($ct -contains $S.saPointNew) {
            $wordAdd = $true
            $wordAddN = $ct.Count
        }
    } catch {}
    Release-Com $shp
}
Close-WordDoc $docA
Add-Check 'docx-Sa5' (($capA -notmatch 'Repair') -and ($ptsA.Count -eq ($nPts0 + 1)) -and $wordAdd) "caption=$capA inspectN=$($ptsA.Count) want=$($nPts0 + 1) wordN=$wordAddN saw=$wordAdd"

# Sa6：删掉刚加的点。
$delPtOut = Get-OutPath 'docx-smartart-delpt.docx'
Close-WordDocByPath $word $delPtOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $addOut.Replace('\', '/')
    output = $delPtOut.Replace('\', '/')
    ops = @(@{ op = 'deleteSmartArtPoint'; id = $addedId })
} 'docx-smartart-del-point' | Out-Null

$treeP = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delPtOut.Replace('\', '/'); format = 'docx'
    scope = @{ smartArt = $true }
} 'docx-smartart-delpt-inspect'
$listP = @($treeP.smartArts) | Where-Object { $_.id -eq $listId } | Select-Object -First 1
$ptsP = if ($listP) { @($listP.points) } else { @() }
$stillNew = @($ptsP | Where-Object { (Normalize-SaText ([string]$_.text)) -eq $S.saPointNew }).Count
$docP = Open-WordDoc $word $delPtOut
$capP = [string]$docP.ActiveWindow.Caption
$wordGone = $true
foreach ($shp in (Get-WordSmartArtShapes $docP)) {
    try {
        $ct = @(Get-SmartArtTexts $shp)
        if ($ct -contains $S.saPointNew) { $wordGone = $false }
    } catch {}
    Release-Com $shp
}
Close-WordDoc $docP
Add-Check 'docx-Sa6' (($capP -notmatch 'Repair') -and ($ptsP.Count -eq $nPts0) -and ($stillNew -eq 0) -and $wordGone) "caption=$capP inspectN=$($ptsP.Count) still=$stillNew wordGone=$wordGone"

# Sa8：改宿主尺寸。
$sizeOut = Get-OutPath 'docx-smartart-size.docx'
Close-WordDocByPath $word $sizeOut
$cx = 2000000
$cy = 1000000
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $delPtOut.Replace('\', '/')
    output = $sizeOut.Replace('\', '/')
    ops = @(@{ op = 'setSmartArtSize'; id = $listId; cx = $cx; cy = $cy })
} 'docx-smartart-set-size' | Out-Null

$treeZ = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $sizeOut.Replace('\', '/'); format = 'docx'
    scope = @{ smartArt = $true }
} 'docx-smartart-size-inspect'
$listZ = @($treeZ.smartArts) | Where-Object { $_.id -eq $listId } | Select-Object -First 1
$docZ = Open-WordDoc $word $sizeOut
$capZ = [string]$docZ.ActiveWindow.Caption
$wantW = $cx / 12700.0
$wantH = $cy / 12700.0
$sizeOk = $false
$sizeDetail = ''
foreach ($shp in (Get-WordSmartArtShapes $docZ)) {
    try {
        $ct = @(Get-SmartArtTexts $shp)
        if ($ct -contains $S.saItem1 -or $ct -contains $S.saItemNew) {
            $w = [double]$shp.Width
            $h = [double]$shp.Height
            $sizeOk = ([math]::Abs($w - $wantW) -lt 3) -and ([math]::Abs($h - $wantH) -lt 3)
            $sizeDetail = "w=$w h=$h wantW=$wantW wantH=$wantH"
        }
    } catch { $sizeDetail = $_.Exception.Message }
    Release-Com $shp
}
Close-WordDoc $docZ
$inspectSize = $listZ -and ([int64]$listZ.cx -eq $cx) -and ([int64]$listZ.cy -eq $cy)
Add-Check 'docx-Sa8' (($capZ -notmatch 'Repair') -and $inspectSize -and $sizeOk) "caption=$capZ inspectCx=$($listZ.cx) inspectCy=$($listZ.cy) $sizeDetail"

# Sa3：插入一张 list。
$insOut = Get-OutPath 'docx-smartart-insert.docx'
Close-WordDocByPath $word $insOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $sizeOut.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{
        op = 'insertSmartArt'
        id = $para1
        layout = 'list'
        points = @(@{ text = $S.saInsert1 }, @{ text = $S.saInsert2 })
    })
} 'docx-smartart-insert' | Out-Null

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'docx'
    scope = @{ smartArt = $true }
} 'docx-smartart-insert-inspect'
$inserted = Find-Diagram $treeI { param($d) @($d.points | ForEach-Object { Normalize-SaText ([string]$_.text) }) -contains $S.saInsert1 }
$docI = Open-WordDoc $word $insOut
$capI = [string]$docI.ActiveWindow.Caption
$shpsI = Get-WordSmartArtShapes $docI
$nI = @($shpsI).Count
$wordIns = $false
foreach ($shp in $shpsI) {
    try {
        $ct = @(Get-SmartArtTexts $shp)
        if ($ct -contains $S.saInsert1) { $wordIns = $true }
    } catch {}
    Release-Com $shp
}
Close-WordDoc $docI
Add-Check 'docx-Sa3' (($capI -notmatch 'Repair') -and ($null -ne $inserted) -and ($treeI.smartArtCount -gt $tree0.smartArtCount) -and $wordIns) "caption=$capI inserted=$($inserted.id) count=$($treeI.smartArtCount) wordN=$nI wordIns=$wordIns"

# Sa4：删掉 Word 亲手插的那张（带 Fallback）。
$srcXml = Get-ZipEntryText $insOut 'word/document.xml'
$fb0 = Count-Fallback $srcXml
$delOut = Get-OutPath 'docx-smartart-delete.docx'
Close-WordDocByPath $word $delOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteSmartArt'; id = $listId })
} 'docx-smartart-delete' | Out-Null

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'docx'
    scope = @{ smartArt = $true }
} 'docx-smartart-delete-inspect'
$stillList = @($treeD.smartArts) | Where-Object { $_.id -eq $listId } | Select-Object -First 1
$delXml = Get-ZipEntryText $delOut 'word/document.xml'
$fb1 = Count-Fallback $delXml
$docD = Open-WordDoc $word $delOut
$capD = [string]$docD.ActiveWindow.Caption
$wordStill = $false
foreach ($shp in (Get-WordSmartArtShapes $docD)) {
    try {
        $ct = @(Get-SmartArtTexts $shp)
        if ($ct -contains $S.saItem1 -or $ct -contains $S.saItemNew) { $wordStill = $true }
    } catch {}
    Release-Com $shp
}
Close-WordDoc $docD
# Word COM AddSmartArt 另存常常没有 Fallback；有则必须一起少。
$fbOk = ($fb0 -eq 0) -or ($fb1 -lt $fb0)
Add-Check 'docx-Sa4' (($capD -notmatch 'Repair') -and ($null -eq $stillList) -and $fbOk -and (-not $wordStill)) "caption=$capD still=$($stillList.id) fb=$fb0->$fb1 wordStill=$wordStill"

# Sa7：循环图改成 list 得到 NotImplemented，不写坏文件。
$comboOut = Get-OutPath 'docx-smartart-layout.docx'
if (Test-Path $comboOut) { Remove-Item -Force $comboOut }
$cycleId = if ($cycle) { $cycle.id } else { $null }
$cycleFail = $false
$cycleCode = ''
if ($cycleId) {
    Close-WordDocByPath $word $comboOut
    $raw = Invoke-OoxpExpectFail @{
        schemaVersion = 1; action = 'apply'
        path = $src.Replace('\', '/')
        output = $comboOut.Replace('\', '/')
        ops = @(@{ op = 'setSmartArtLayout'; id = $cycleId; layout = 'list' })
    } 'docx-smartart-layout-cycle'
    try {
        $errObj = $raw.Stderr | ConvertFrom-Json
        $cycleCode = [string]$errObj.error.code
    } catch { $cycleCode = $raw.Stderr }
    $cycleFail = ($raw.ExitCode -ne 0) -and ($cycleCode -eq 'NotImplemented')
}
$noWrite = -not (Test-Path $comboOut)
$docC = Open-WordDoc $word $src
$capC = [string]$docC.ActiveWindow.Caption
Close-WordDoc $docC
Add-Check 'docx-Sa7' ($cycleFail -and $noWrite -and ($capC -notmatch 'Repair')) "code=$cycleCode wrote=$(-not $noWrite) caption=$capC cycleId=$cycleId"
