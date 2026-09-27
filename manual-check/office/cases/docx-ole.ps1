# docx OLE：docs/checklist-docx.md 的 Ol1–Ol5。
# 源文件由 Word InlineShapes.AddOLEObject 另存嵌入 xlsx。不要用 create 产物当 Ol1 唯一样例。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings

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

function Get-ZipEntryBytes([string]$docx, [string]$pattern) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $z = [IO.Compression.ZipFile]::OpenRead($docx)
    try {
        $entry = $z.Entries | Where-Object { $_.FullName -replace '\\','/' -like $pattern } | Select-Object -First 1
        if (-not $entry) { return $null }
        $ms = New-Object IO.MemoryStream
        $es = $entry.Open()
        try { $es.CopyTo($ms) } finally { $es.Dispose() }
        return $ms.ToArray()
    }
    finally { $z.Dispose() }
}

function Ensure-DocxWordOleSource([switch]$Force) {
    $path = Get-OutPath 'docx-word-ole-source.docx'
    if ((Test-Path $path) -and -not $Force) { return $path }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $skel = Join-Path $tmpDir 'docx-word-ole-skel.docx'
    $xlsx = Join-Path $tmpDir 'docx-word-ole-embed.xlsx'
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'create'
        path = $skel.Replace('\', '/'); format = 'docx'
        document = @{
            paragraphs = @(
                @{ style = 'Heading1'; text = $S.chapter1 }
                @{ text = $S.para2src }
            )
        }
    } 'docx-ole-skel-create' | Out-Null
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'create'
        path = $xlsx.Replace('\', '/'); format = 'xlsx'
        workbook = @{
            sheets = @(
                @{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = '嵌入' }) }
            )
        }
    } 'docx-ole-embed-create' | Out-Null

    $word = Ensure-MsWord (Ensure-DocxCreate)
    Close-WordDocByPath $word $skel
    $doc = Open-WordDoc $word $skel
    $missing = [System.Type]::Missing
    $p2 = $doc.Paragraphs.Item(2)
    $rng = $p2.Range
    $inlines = $doc.InlineShapes
    $before = [int]$inlines.Count
    $ish = $null
    try {
        $ish = $inlines.AddOLEObject($missing, $xlsx, $false, $false, $missing, $missing, $missing, $rng)
        $after = [int]$inlines.Count
        if ($after -le $before) {
            throw "AddOLEObject 后 InlineShapes.Count 仍是 $after"
        }
    } finally {
        if ($ish) { Release-Com $ish }
        Release-Com $inlines
        Release-Com $rng
        Release-Com $p2
    }
    Write-Log 'word Save() 原地覆盖 TEMP 里的 OLE 样例'
    $doc.Save()
    Close-WordDoc $doc
    Copy-Item -Force $skel $path
    Write-Log "saved word ole source $path"
    return $path
}

$src = Ensure-DocxWordOleSource
$word = Ensure-MsWord (Ensure-DocxCreate)
Close-WordDocByPath $word $src

# Ol1：无 scope 有计数无名单；scope.oleObjects 时 progId 含 Excel，id 形如 para:…/ole:…
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-ole-inspect-counts'
$noList = -not ($tree0.PSObject.Properties.Name -contains 'oleObjects')
Add-Check 'docx-Ol1a' (($tree0.oleObjectCount -ge 1) -and $noList) "count=$($tree0.oleObjectCount) noList=$noList"

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ oleObjects = $true }
} 'docx-ole-inspect-lists'
$ole = @($tree1.oleObjects) | Select-Object -First 1
$idOk = $ole -and ([string]$ole.id -like 'para:*/ole:*')
$prog = if ($ole) { [string]$ole.progId } else { '' }
$progOk = $prog -match 'Excel'
Add-Check 'docx-Ol1' ($idOk -and $progOk) "id=$($ole.id) progId=$prog kind=$($ole.kind)"

$oleId = if ($ole) { [string]$ole.id } else { '' }
$paraId = if ($oleId -match '^(.*)/ole:') { $Matches[1] } else { '' }

# Ol2：exportMedia 字节与 ZIP embeddings 一致。
$expOut = Get-OutPath 'docx-ole-export.bin'
Close-WordDocByPath $word $expOut
Close-WordDocByPath $word $src
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportMedia'
    path = $src.Replace('\', '/')
    id = $oleId
    output = $expOut.Replace('\', '/')
} 'docx-ole-export' | Out-Null
$embed = Get-ZipEntryBytes $src 'word/embeddings/*'
$exported = if (Test-Path $expOut) { [IO.File]::ReadAllBytes($expOut) } else { @() }
$same = $false
if ($embed -and $exported -and ($embed.Length -eq $exported.Length)) {
    $same = $true
    for ($i = 0; $i -lt $embed.Length; $i++) {
        if ($embed[$i] -ne $exported[$i]) { $same = $false; break }
    }
}
Add-Check 'docx-Ol2' $same "embed=$($embed.Length) export=$($exported.Length)"

# Ol3：deleteOle 后计数 0，Word 无修复。
$delOut = Get-OutPath 'docx-ole-delete.docx'
Close-WordDocByPath $word $delOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteOle'; id = $oleId })
} 'docx-ole-delete' | Out-Null
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'docx'
    scope = @{ oleObjects = $true }
} 'docx-ole-delete-inspect'
$docD = Open-WordDoc $word $delOut
$capD = [string]$docD.ActiveWindow.Caption
Close-WordDoc $docD
$delCount = [int]$treeD.oleObjectCount
Add-Check 'docx-Ol3' (($delCount -eq 0) -and ($capD -notmatch 'Repair')) "count=$delCount caption=$capD"

# Ol4：insertOle 写入本地 xlsx 后 Word 无修复。
$hostPath = Get-OutPath 'docx-ole-insert-host.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = $S.chapter1 }
            @{ text = $S.hello }
        )
    }
} 'docx-ole-insert-host' | Out-Null
$treeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 10 }
} 'docx-ole-insert-host-inspect'
$insPara = [string]$treeH.paragraphs[1].id
$insXlsx = Get-OutPath 'docx-ole-insert-src.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $insXlsx.Replace('\', '/'); format = 'xlsx'
    workbook = @{
        sheets = @(
            @{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 42 }) }
        )
    }
} 'docx-ole-insert-src' | Out-Null
$insOut = Get-OutPath 'docx-ole-insert.docx'
Close-WordDocByPath $word $insOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $insOut.Replace('\', '/')
    ops = @(@{ op = 'insertOle'; id = $insPara; file = $insXlsx.Replace('\', '/') })
} 'docx-ole-insert' | Out-Null
$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insOut.Replace('\', '/'); format = 'docx'
    scope = @{ oleObjects = $true }
} 'docx-ole-insert-inspect'
$docI = Open-WordDoc $word $insOut
$capI = [string]$docI.ActiveWindow.Caption
Close-WordDoc $docI
$insCount = [int]$treeI.oleObjectCount
$insProg = if ($treeI.oleObjects) { [string]$treeI.oleObjects[0].progId } else { '' }
Add-Check 'docx-Ol4' (($insCount -ge 1) -and ($insProg -match 'Excel') -and ($capI -notmatch 'Repair')) "count=$insCount progId=$insProg caption=$capI"

# Ol5：错 id 得 InvalidId，不强制开 Word。
$failDel = Get-OutPath 'docx-ole-bad-del.docx'
$failIns = Get-OutPath 'docx-ole-bad-ins.docx'
if (Test-Path $failDel) { Remove-Item -Force $failDel }
if (Test-Path $failIns) { Remove-Item -Force $failIns }
$badDel = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failDel.Replace('\', '/')
    ops = @(@{ op = 'deleteOle'; id = $paraId })
} 'docx-ole-bad-del'
$badIns = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failIns.Replace('\', '/')
    ops = @(@{ op = 'insertOle'; id = $oleId; file = $insXlsx.Replace('\', '/') })
} 'docx-ole-bad-ins'
$codeDel = $null
$codeIns = $null
try { $codeDel = ($badDel.Stderr | ConvertFrom-Json).error.code } catch {}
try { $codeIns = ($badIns.Stderr | ConvertFrom-Json).error.code } catch {}
$noWrite = (-not (Test-Path $failDel)) -and (-not (Test-Path $failIns))
Add-Check 'docx-Ol5' (($badDel.ExitCode -ne 0) -and ($codeDel -eq 'InvalidId') -and ($badIns.ExitCode -ne 0) -and ($codeIns -eq 'InvalidId') -and $noWrite) "del=$codeDel ins=$codeIns wrote=$(-not $noWrite)"
