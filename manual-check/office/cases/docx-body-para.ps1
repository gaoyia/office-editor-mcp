# docx 正文插删段落：docs/checklist-docx.md 的 Ip1–Ip5。
# 正文源文件由 Word 另存（第二段加粗）；页眉用带默认页眉的 Word 另存样例。

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

$S = $global:OoxpStrings
$src = Ensure-DocxWordSource
$hfSrc = Ensure-DocxWordHeaderSource
$word = Ensure-MsWord $src

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 200 }
} 'docx-ip-source-inspect'
$id1 = $srcTree.paragraphs[0].id
$id2 = $srcTree.paragraphs[1].id
$id3 = $srcTree.paragraphs[2].id
Write-Log ("源文件段落 id：$id1 $id2 $id3")

# Ip1：插在第二段之后。
$ins = Get-OutPath 'docx-body-para-insert.docx'
Close-WordDocByPath $word $ins
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $ins.Replace('\', '/')
    ops = @(@{ op = 'insertPara'; id = $id2; text = $S.paraInsert })
} 'docx-ip-insert-after' | Out-Null

$doc = Open-WordDoc $word $ins
$cap1 = [string]$doc.ActiveWindow.Caption
$p1 = Get-ParaText $doc.Paragraphs.Item(1)
$p2 = $doc.Paragraphs.Item(2)
$p2t = Get-ParaText $p2
$p3t = Get-ParaText $doc.Paragraphs.Item(3)
$p4t = Get-ParaText $doc.Paragraphs.Item(4)
$bold = $p2.Range.Font.Bold
$h1ok = ($doc.Paragraphs.Item(1).Style.NameLocal -eq $S.heading1) -or
        ($doc.Paragraphs.Item(1).Style.NameLocal -eq 'Heading 1')
Add-Check 'docx-Ip1' (
    ($cap1 -notmatch 'Repair') -and ($p1 -eq $S.chapter1) -and $h1ok -and
    (($bold -eq -1) -or ($bold -eq $true)) -and
    ($p3t -eq $S.paraInsert) -and ($p4t -eq $S.para3keep)
) "caption=$cap1 p1=$p1 p2=$p2t p3=$p3t p4=$p4t bold=$bold"
Close-WordDoc $doc
Release-Com $p2

# Ip2：插到文档末。
$end = Get-OutPath 'docx-body-para-end.docx'
Close-WordDocByPath $word $end
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $end.Replace('\', '/')
    ops = @(@{ op = 'insertPara'; id = 'docx:document'; text = $S.paraEnd })
} 'docx-ip-insert-end' | Out-Null
$endTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $end.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 200 }
} 'docx-ip-end-inspect'
$lastText = $endTree.paragraphs[-1].text
$docE = Open-WordDoc $word $end
$capE = [string]$docE.ActiveWindow.Caption
Close-WordDoc $docE
Add-Check 'docx-Ip2' (($capE -notmatch 'Repair') -and ($lastText -eq $S.paraEnd)) "caption=$capE last=$lastText"

# Ip3：删掉第二段。
$del = Get-OutPath 'docx-body-para-delete.docx'
Close-WordDocByPath $word $del
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $del.Replace('\', '/')
    ops = @(@{ op = 'deletePara'; id = $id2 })
} 'docx-ip-delete' | Out-Null
$docD = Open-WordDoc $word $del
$capD = [string]$docD.ActiveWindow.Caption
$d1 = Get-ParaText $docD.Paragraphs.Item(1)
$d2 = Get-ParaText $docD.Paragraphs.Item(2)
$h1d = ($docD.Paragraphs.Item(1).Style.NameLocal -eq $S.heading1) -or
       ($docD.Paragraphs.Item(1).Style.NameLocal -eq 'Heading 1')
Add-Check 'docx-Ip3' (
    ($capD -notmatch 'Repair') -and ($d1 -eq $S.chapter1) -and $h1d -and ($d2 -eq $S.para3keep)
) "caption=$capD d1=$d1 d2=$d2"
Close-WordDoc $docD

# Ip4：只剩一段后再删，得 InvalidField，不写文件。
$cur = $del
$round = 0
while ($true) {
    $round++
    $curTree = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $cur.Replace('\', '/'); format = 'docx'
        scope = @{ from = 1; maxParas = 200 }
    } "docx-ip-shrink-$round"
    $n = @($curTree.paragraphs).Count
    if ($n -le 1) { break }
    if ($round -gt 8) { throw "Ip4 收不到只剩一段：还剩 $n" }
    $drop = $curTree.paragraphs[-1].id
    $next = Get-OutPath "docx-body-para-shrink-$round.docx"
    Close-WordDocByPath $word $next
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $cur.Replace('\', '/')
        output = $next.Replace('\', '/')
        ops = @(@{ op = 'deletePara'; id = $drop })
    } "docx-ip-shrink-apply-$round" | Out-Null
    $cur = $next
}
$oneTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cur.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 200 }
} 'docx-ip-one-inspect'
$lastId = $oneTree.paragraphs[0].id
$lastOut = Get-OutPath 'docx-body-para-last.docx'
if (Test-Path $lastOut) { Remove-Item -Force $lastOut }
$raw = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $cur.Replace('\', '/')
    output = $lastOut.Replace('\', '/')
    ops = @(@{ op = 'deletePara'; id = $lastId })
} 'docx-ip-delete-last'
try {
    $errObj = $raw.Stderr | ConvertFrom-Json
    $lastCode = [string]$errObj.error.code
} catch { $lastCode = $raw.Stderr }
$noWrite = -not (Test-Path $lastOut)
Add-Check 'docx-Ip4' (
    ($raw.ExitCode -ne 0) -and ($lastCode -eq 'InvalidField') -and $noWrite
) "code=$lastCode wrote=$(-not $noWrite) left=$($oneTree.paragraphs.Count)"

# Ip5：页眉插一段。
$hfTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfSrc.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true }
} 'docx-ip-hf-inspect'
$h = @($hfTree.headers) | Where-Object { $_.type -eq 'default' } | Select-Object -First 1
$hid = $h.id
$hfOut = Get-OutPath 'docx-body-para-header.docx'
Close-WordDocByPath $word $hfOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hfSrc.Replace('\', '/')
    output = $hfOut.Replace('\', '/')
    ops = @(@{ op = 'insertPara'; id = $hid; text = $S.headerInsert })
} 'docx-ip-header-insert' | Out-Null
$docH = Open-WordDoc $word $hfOut
$capH = [string]$docH.ActiveWindow.Caption
$sec = $docH.Sections.Item(1)
$hdr = $sec.Headers.Item(1)
$hdrText = ([string]$hdr.Range.Text)
$body1 = Get-ParaText $docH.Paragraphs.Item(1)
Add-Check 'docx-Ip5' (
    ($capH -notmatch 'Repair') -and ($hdrText -match [regex]::Escape($S.headerInsert)) -and
    ($hdrText -match [regex]::Escape($S.headerSrc)) -and ($body1 -eq $S.chapter1)
) "caption=$capH hdr=$hdrText body=$body1"
Release-Com $hdr
Release-Com $sec
Close-WordDoc $docH
