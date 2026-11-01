# pptx OLE：docs/checklist-pptx.md 的 Po1–Po5。
# 源文件由 PowerPoint Shapes.AddOLEObject 另存嵌入 xlsx。不要用 create 产物当 Po1 唯一样例。
# 不算放映引擎，不激活 OLE 服务器。

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

function Get-PptOleCount($slide) {
    $n = 0
    $shapes = $slide.Shapes
    try {
        for ($i = 1; $i -le $shapes.Count; $i++) {
            $sh = $shapes.Item($i)
            try {
                if ([int]$sh.Type -eq 7) { $n++ }
            } finally { Release-Com $sh }
        }
    } finally { Release-Com $shapes }
    return $n
}

function Get-ZipEntryBytes([string]$pptx, [string]$pattern) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $z = [IO.Compression.ZipFile]::OpenRead($pptx)
    try {
        $entry = $z.Entries | Where-Object { ($_.FullName -replace '\\', '/') -like $pattern } | Select-Object -First 1
        if (-not $entry) { return $null }
        $ms = New-Object IO.MemoryStream
        $s = $entry.Open()
        try { $s.CopyTo($ms) } finally { $s.Dispose() }
        return $ms.ToArray()
    } finally { $z.Dispose() }
}

function Test-PptxOleSource([string]$path, [string]$reqName) {
    $tree = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $path.Replace('\', '/'); format = 'pptx'
        scope = @{ oleObjects = $true; pictures = $true }
    } $reqName
    $slide = $tree.slides[0]
    $item = $null
    if ($slide.oleObjects) { $item = @($slide.oleObjects) | Select-Object -First 1 }
    $picN = 0
    if ($slide.pictures) { $picN = @($slide.pictures).Count }
    $ok = ([int]$tree.oleObjectCount -ge 1) `
        -and ($null -ne $item) `
        -and ([string]$item.id -like 'slide:*/ole:*') `
        -and ($picN -eq 0)
    return @{
        ok = $ok
        oleObjectCount = [int]$tree.oleObjectCount
        id = $(if ($item) { [string]$item.id } else { '' })
        pictureCount = [int]$tree.pictureCount
        tree = $tree
        item = $item
    }
}

function Ensure-PptxPptOleSource {
    $path = Get-OutPath 'pptx-ppt-ole-source.pptx'
    if (Test-Path $path) {
        $cached = Test-PptxOleSource $path 'pptx-ole-source-cache-check'
        if ($cached.ok) { return $path }
        Write-Log "缓存的 pptx ole source 不合格（count=$($cached.oleObjectCount) id=$($cached.id)），重建"
    }

    $xlsx = Get-OutPath 'pptx-ole-embed.xlsx'
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'create'
        path = $xlsx.Replace('\', '/'); format = 'xlsx'
        workbook = @{
            sheets = @(
                @{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 42 }) }
            )
        }
    } 'pptx-ole-embed-xlsx' | Out-Null

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $tmp = Join-Path $tmpDir 'pptx-ppt-ole-source.pptx'
    $hostPath = Ensure-PptxCreate
    Copy-Item -Force $hostPath $tmp

    $ppt = Ensure-MsPpt $hostPath
    Close-PptPresByPath $ppt $tmp
    $pres = Open-PptPres $ppt $tmp
    $s1 = Get-PptSlide $pres 1
    $shapes = $s1.Shapes
    $ole = $null
    try {
        $ole = $shapes.AddOLEObject(72, 72, 288, 216, '', $xlsx)
    } catch {
        Write-Log ("AddOLEObject 报错：" + $_.Exception.Message)
    } finally { Release-Com $shapes }

    if (-not $ole) {
        Release-Com $s1
        Close-PptPres $pres
        throw 'PowerPoint 没有插入嵌入对象'
    }
    Write-Log ("AddOLEObject Type=$([int]$ole.Type)")
    Release-Com $ole
    Write-Log 'PowerPoint Save() 原地覆盖 TEMP 里的 OLE 源文件'
    $pres.Save()
    Release-Com $s1
    Close-PptPres $pres
    Copy-Item -Force $tmp $path
    Write-Log "saved ppt ole source $path"

    $check = Test-PptxOleSource $path 'pptx-ole-source-saved-inspect'
    if (-not $check.ok) {
        throw "PowerPoint 另存的 OLE 源不合格：count=$($check.oleObjectCount) id=$($check.id) pics=$($check.pictureCount)"
    }
    return $path
}

$hostPath = Ensure-PptxCreate
$src = Ensure-PptxPptOleSource
$ppt = Ensure-MsPpt $hostPath
Close-PptPresByPath $ppt $src

# Po1：无名单时只有计数；展开后 id 为 slide:…/ole:…，不并进 pictures。
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
} 'pptx-ole-inspect-list'
$slide0 = $tree0.slides[0]
$noList = -not ($slide0.PSObject.Properties.Name -contains 'oleObjects')
Add-Check 'pptx-Po1a' (($tree0.oleObjectCount -ge 1) -and ($slide0.oleObjectCount -ge 1) -and $noList) "wb=$($tree0.oleObjectCount) slide=$($slide0.oleObjectCount) noList=$noList"

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ oleObjects = $true; pictures = $true }
} 'pptx-ole-inspect-expand'
$oleNode = @($tree1.slides[0].oleObjects) | Select-Object -First 1
$picN = 0
if ($tree1.slides[0].pictures) { $picN = @($tree1.slides[0].pictures).Count }
$po1ok = ($null -ne $oleNode) `
    -and ([string]$oleNode.id -like 'slide:*/ole:*') `
    -and ([int]$tree1.pictureCount -eq 0) `
    -and ($picN -eq 0)
Add-Check 'pptx-Po1' $po1ok "id=$($oleNode.id) pics=$($tree1.pictureCount) picList=$picN"

# Po2：exportMedia 与 ZIP 内 embeddings 字节一致。
$oleId = [string]$oleNode.id
$exportOut = Get-OutPath 'pptx-ole-export.xlsx'
if (Test-Path $exportOut) { Remove-Item -Force $exportOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $src.Replace('\', '/')
    id = $oleId
    output = $exportOut.Replace('\', '/')
} 'pptx-ole-export' | Out-Null
$embed = Get-ZipEntryBytes $src 'ppt/embeddings/*'
$exported = if (Test-Path $exportOut) { [IO.File]::ReadAllBytes($exportOut) } else { $null }
$same = $false
if ($embed -and $exported -and ($embed.Length -eq $exported.Length)) {
    $same = $true
    for ($i = 0; $i -lt $embed.Length; $i++) {
        if ($embed[$i] -ne $exported[$i]) { $same = $false; break }
    }
}
Add-Check 'pptx-Po2' $same "embed=$($embed.Length) export=$($exported.Length)"

# Po3：deleteOle 后 Type=7 没了，无 Repair。
$delOut = Get-OutPath 'pptx-ole-delete.pptx'
Close-PptPresByPath $ppt $delOut
if (Test-Path $delOut) { Remove-Item -Force $delOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteOle'; id = $oleId })
} 'pptx-ole-delete' | Out-Null
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'pptx'
    scope = @{ oleObjects = $true }
} 'pptx-ole-delete-inspect'
$presD = Open-PptPres $ppt $delOut
$capD = Get-PptCaption $presD
$sD = Get-PptSlide $presD 1
$comD = Get-PptOleCount $sD
Release-Com $sD
Close-PptPres $presD
$po3ok = ([int]$treeD.oleObjectCount -eq 0) -and ($comD -eq 0) -and ($capD -notmatch 'Repair')
Add-Check 'pptx-Po3' $po3ok "count=$($treeD.oleObjectCount) com=$comD cap=$capD"

# Po4：create 产物 insertOle 本地 xlsx，无 Repair，COM Type=7。
$insXlsx = Get-OutPath 'pptx-ole-insert-src.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $insXlsx.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(
            @{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 42 }) }
        )
    }
} 'pptx-ole-insert-src' | Out-Null
$insOut = Get-OutPath 'pptx-ole-insert.pptx'
Close-PptPresByPath $ppt $insOut
if (Test-Path $insOut) { Remove-Item -Force $insOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{ op = 'insertOle'; id = 'slide:1'; file = $insXlsx.Replace('\', '/') })
} 'pptx-ole-insert' | Out-Null
$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'pptx'
    scope = @{ oleObjects = $true }
} 'pptx-ole-insert-inspect'
$insNode = @($treeI.slides[0].oleObjects) | Select-Object -First 1
$presI = Open-PptPres $ppt $insOut
$capI = Get-PptCaption $presI
$sI = Get-PptSlide $presI 1
$comI = Get-PptOleCount $sI
Release-Com $sI
Close-PptPres $presI
$insProg = if ($insNode) { [string]$insNode.progId } else { '' }
$po4ok = ([int]$treeI.oleObjectCount -ge 1) `
    -and ($null -ne $insNode) `
    -and ([string]$insNode.id -like 'slide:1/ole:*') `
    -and ($insProg -match 'Excel') `
    -and ($comI -ge 1) `
    -and ($capI -notmatch 'Repair')
Add-Check 'pptx-Po4' $po4ok "id=$($insNode.id) progId=$insProg com=$comI cap=$capI"

# Po5：错 id / xlsx 拒写。
$failDel = Get-OutPath 'pptx-ole-bad-del.pptx'
$failIns = Get-OutPath 'pptx-ole-bad-ins.pptx'
$failX = Get-OutPath 'pptx-ole-fail-xlsx.xlsx'
if (Test-Path $failDel) { Remove-Item -Force $failDel }
if (Test-Path $failIns) { Remove-Item -Force $failIns }
if (Test-Path $failX) { Remove-Item -Force $failX }
$badDel = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failDel.Replace('\', '/')
    ops = @(@{ op = 'deleteOle'; id = 'slide:1' })
} 'pptx-ole-bad-del'
$badIns = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failIns.Replace('\', '/')
    ops = @(@{ op = 'insertOle'; id = $oleId; file = $insXlsx.Replace('\', '/') })
} 'pptx-ole-bad-ins'
$xlsx = Ensure-XlsxCreate
$failXlsx = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'insertOle'; id = 'slide:1'; file = $insXlsx.Replace('\', '/') })
} 'pptx-ole-fail-xlsx'
$po5ok = ((Get-PptErrorCode $badDel) -eq 'InvalidId') `
    -and ((Get-PptErrorCode $badIns) -eq 'InvalidId') `
    -and ((Get-PptErrorCode $failXlsx) -eq 'InvalidId') `
    -and (-not (Test-Path $failDel)) `
    -and (-not (Test-Path $failIns)) `
    -and (-not (Test-Path $failX))
Add-Check 'pptx-Po5' $po5ok "del=$(Get-PptErrorCode $badDel) ins=$(Get-PptErrorCode $badIns) x=$(Get-PptErrorCode $failXlsx)"
