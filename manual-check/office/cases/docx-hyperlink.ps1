# docx 超链接与书签：docs/checklist-docx.md 的 Hb1–Hb7。
# 用 Word COM 造链和书签，再用 ooxp 插改删，不调 COM 的 Hyperlinks.Delete 完成改链。

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

$src = Get-OutPath 'docx-hyperlink-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = '链甲' }
            @{ text = '书乙' }
            @{ text = '干净' }
        )
    }
} 'docx-hyperlink-create' | Out-Null

$word = Ensure-MsWord $src
$doc0 = Open-WordDoc $word $src
$p1 = $doc0.Paragraphs.Item(1)
$r1 = $p1.Range
$r1.End = $r1.End - 1
$hls = $doc0.Hyperlinks
$hAdd = $hls.Add($r1, 'https://example.com/')
Release-Com $hAdd
Release-Com $hls
Release-Com $r1
Release-Com $p1
$p2 = $doc0.Paragraphs.Item(2)
$r2 = $p2.Range
$r2.End = $r2.End - 1
$bms = $doc0.Bookmarks
$bAdd = $bms.Add('SrcBm', $r2)
Release-Com $bAdd
Release-Com $bms
Release-Com $r2
Release-Com $p2
$doc0.Save()
Close-WordDoc $doc0

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-hyperlink-inspect'
$listed = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ hyperlinks = $true; bookmarks = $true; from = 1; maxParas = 20 }
} 'docx-hyperlink-inspect-list'
$links = @($listed.hyperlinks)
$bmarks = @($listed.bookmarks)
$urlLink = @($links | Where-Object { $_.kind -eq 'url' }) | Select-Object -First 1
$userBm = @($bmarks | Where-Object { -not ([string]$_.name).StartsWith('_') }) | Select-Object -First 1
$hb1ok = ($null -ne $tree0.hyperlinkCount) -and ([int]$tree0.hyperlinkCount -ge 1) -and
    ($null -eq $tree0.hyperlinks) -and ($null -eq $tree0.bookmarks) -and
    ($null -ne $tree0.bookmarkCount) -and ([int]$tree0.bookmarkCount -ge 1) -and
    ($null -ne $urlLink) -and ($null -ne $urlLink.url) -and
    ($null -ne $userBm) -and ($null -ne $userBm.name)
Add-Check 'docx-Hb1' $hb1ok "hlCount=$($tree0.hyperlinkCount) bmCount=$($tree0.bookmarkCount) link=$($urlLink.id) url=$($urlLink.url) bm=$($userBm.name)"

$setUrl = Get-OutPath 'docx-hyperlink-set.docx'
Close-WordDocByPath $word $setUrl
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $setUrl.Replace('\', '/')
    ops = @(@{ op = 'setHyperlink'; id = [string]$urlLink.id; url = 'https://example.org/' })
} 'docx-hyperlink-set' | Out-Null
$docS = Open-WordDoc $word $setUrl
$capS = [string]$docS.ActiveWindow.Caption
$hlsS = $docS.Hyperlinks
$addr = ''
if ($hlsS.Count -ge 1) {
    $h1 = $hlsS.Item(1)
    $addr = [string]$h1.Address
    Release-Com $h1
}
Release-Com $hlsS
Add-Check 'docx-Hb2' (($capS -notmatch 'Repair') -and ($addr -match 'example.org')) "caption=$capS addr=$addr"
Close-WordDoc $docS

$ins = Get-OutPath 'docx-hyperlink-insert.docx'
Close-WordDocByPath $word $ins
$clean = @($listed.paragraphs) | Where-Object { [string]$_.text -eq '干净' } | Select-Object -First 1
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $ins.Replace('\', '/')
    ops = @(@{
        op = 'insertHyperlink'
        id = [string]$clean.id
        text = '新链'
        url = 'https://new.example/'
    })
} 'docx-hyperlink-insert' | Out-Null
$docI = Open-WordDoc $word $ins
$capI = [string]$docI.ActiveWindow.Caption
$hlsI = $docI.Hyperlinks
$nI = [int]$hlsI.Count
Release-Com $hlsI
$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ins.Replace('\', '/'); format = 'docx'
    scope = @{ hyperlinks = $true }
} 'docx-hyperlink-insert-inspect'
$sawNew = @($treeI.hyperlinks) | Where-Object { $_.text -eq '新链' } | Select-Object -First 1
Add-Check 'docx-Hb3' (($capI -notmatch 'Repair') -and ($nI -ge 2) -and ($null -ne $sawNew)) "caption=$capI wordN=$nI new=$($sawNew.id)"
Close-WordDoc $docI

$del = Get-OutPath 'docx-hyperlink-delete.docx'
Close-WordDocByPath $word $del
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $del.Replace('\', '/')
    ops = @(@{ op = 'deleteHyperlink'; id = [string]$urlLink.id })
} 'docx-hyperlink-delete' | Out-Null
$docD = Open-WordDoc $word $del
$capD = [string]$docD.ActiveWindow.Caption
$hlsD = $docD.Hyperlinks
$nD = [int]$hlsD.Count
Release-Com $hlsD
$t1 = Get-ParaText $docD.Paragraphs.Item(1)
Add-Check 'docx-Hb4' (($capD -notmatch 'Repair') -and ($nD -eq 0) -and ($t1 -match '链甲')) "caption=$capD wordN=$nD p1=$t1"
Close-WordDoc $docD

$insBm = Get-OutPath 'docx-hyperlink-bookmark-ins.docx'
Close-WordDocByPath $word $insBm
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $insBm.Replace('\', '/')
    ops = @(@{ op = 'insertBookmark'; id = [string]$clean.id; name = 'NewBm' })
} 'docx-hyperlink-bookmark-ins' | Out-Null
$docB = Open-WordDoc $word $insBm
$capB = [string]$docB.ActiveWindow.Caption
$bmsB = $docB.Bookmarks
$hasNew = [bool]$bmsB.Exists('NewBm')
Release-Com $bmsB
Add-Check 'docx-Hb5' (($capB -notmatch 'Repair') -and $hasNew) "caption=$capB exists=$hasNew"
Close-WordDoc $docB

$delBm = Get-OutPath 'docx-hyperlink-bookmark-del.docx'
Close-WordDocByPath $word $delBm
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $delBm.Replace('\', '/')
    ops = @(@{ op = 'deleteBookmark'; id = [string]$userBm.id })
} 'docx-hyperlink-bookmark-del' | Out-Null
$docX = Open-WordDoc $word $delBm
$capX = [string]$docX.ActiveWindow.Caption
$bmsX = $docX.Bookmarks
$still = [bool]$bmsX.Exists([string]$userBm.name)
Release-Com $bmsX
Add-Check 'docx-Hb6' (($capX -notmatch 'Repair') -and (-not $still)) "caption=$capX still=$still name=$($userBm.name)"
Close-WordDoc $docX

$failOut = Get-OutPath 'docx-hyperlink-set-text.docx'
if (Test-Path $failOut) { Remove-Item -Force $failOut }
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'setParaText'; id = [string]$urlLink.host; text = '不该写' })
} 'docx-hyperlink-set-text'
$failCode = $null
try { $failCode = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$noWrite = -not (Test-Path $failOut)
$hb7ok = ($fail.ExitCode -ne 0) -and ($failCode -eq 'NotImplemented') -and $noWrite
Add-Check 'docx-Hb7' $hb7ok "code=$failCode exit=$($fail.ExitCode) wrote=$(-not $noWrite)"
