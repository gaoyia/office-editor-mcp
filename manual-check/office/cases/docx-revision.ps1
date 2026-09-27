# docx 修订痕迹：docs/checklist-docx.md 的 Rv1–Rv6。
# 用 Word 跟踪修订造样例，再用 ooxp 接受 / 拒绝，不调 COM 的 Accept/Reject。

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

function Set-FinalView($doc) {
    try {
        $win = $doc.ActiveWindow
        $view = $win.View
        $view.RevisionsView = 0
        Release-Com $view
        Release-Com $win
    } catch {}
}

$src = Get-OutPath 'docx-revision-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = '原文甲乙' }
            @{ text = '原文丙丁' }
        )
    }
} 'docx-revision-create' | Out-Null

$word = Ensure-MsWord $src
$doc0 = Open-WordDoc $word $src
$doc0.TrackRevisions = $true
$p1 = $doc0.Paragraphs.Item(1)
$r1 = $p1.Range
$r1.Start = $r1.Start + 2
$r1.End = $r1.Start + 1
$r1.Delete() | Out-Null
Release-Com $r1
Release-Com $p1
$p2 = $doc0.Paragraphs.Item(2)
$r2 = $p2.Range
$r2.End = $r2.End - 1
$r2.Collapse(0) | Out-Null
$r2.InsertAfter('插X') | Out-Null
Release-Com $r2
Release-Com $p2
$doc0.TrackRevisions = $false
$doc0.Save()
Close-WordDoc $doc0

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-revision-inspect'
$listed = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ revisions = $true; from = 1; maxParas = 20 }
} 'docx-revision-inspect-list'
$revs = @($listed.revisions)
$del = @($revs | Where-Object { $_.kind -eq 'del' }) | Select-Object -First 1
$ins = @($revs | Where-Object { $_.kind -eq 'ins' }) | Select-Object -First 1
$rv1ok = ($null -ne $tree0.revisionCount) -and ([int]$tree0.revisionCount -ge 2) -and
    ($null -eq $tree0.revisions) -and ($null -ne $del) -and ($null -ne $ins) -and
    ($null -ne $del.host) -and ($null -ne $ins.host)
Add-Check 'docx-Rv1' $rv1ok "count=$($tree0.revisionCount) listed=$($revs.Count) del=$($del.id) ins=$($ins.id)"

$accDel = Get-OutPath 'docx-revision-accept-del.docx'
Close-WordDocByPath $word $accDel
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $accDel.Replace('\', '/')
    ops = @(@{ op = 'acceptRevision'; id = [string]$del.id })
} 'docx-revision-accept-del' | Out-Null
$docD = Open-WordDoc $word $accDel
Set-FinalView $docD
$capD = [string]$docD.ActiveWindow.Caption
$t1 = Get-ParaText $docD.Paragraphs.Item(1)
$t2 = Get-ParaText $docD.Paragraphs.Item(2)
Add-Check 'docx-Rv2' (($capD -notmatch 'Repair') -and ($t1 -notmatch '甲') -and ($t2 -match '插X')) `
    "caption=$capD p1=$t1 p2=$t2"
Close-WordDoc $docD

$rejIns = Get-OutPath 'docx-revision-reject-ins.docx'
Close-WordDocByPath $word $rejIns
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $rejIns.Replace('\', '/')
    ops = @(@{ op = 'rejectRevision'; id = [string]$ins.id })
} 'docx-revision-reject-ins' | Out-Null
$docR = Open-WordDoc $word $rejIns
Set-FinalView $docR
$capR = [string]$docR.ActiveWindow.Caption
$t1r = Get-ParaText $docR.Paragraphs.Item(1)
$t2r = Get-ParaText $docR.Paragraphs.Item(2)
Add-Check 'docx-Rv3' (($capR -notmatch 'Repair') -and ($t2r -notmatch '插X') -and ($t2r -match '原文丙丁')) `
    "caption=$capR p1=$t1r p2=$t2r"
Close-WordDoc $docR

$accIns = Get-OutPath 'docx-revision-accept-ins.docx'
Close-WordDocByPath $word $accIns
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $accIns.Replace('\', '/')
    ops = @(@{ op = 'acceptRevision'; id = [string]$ins.id })
} 'docx-revision-accept-ins' | Out-Null
$docA = Open-WordDoc $word $accIns
Set-FinalView $docA
$capA = [string]$docA.ActiveWindow.Caption
$t2a = Get-ParaText $docA.Paragraphs.Item(2)
Add-Check 'docx-Rv4' (($capA -notmatch 'Repair') -and ($t2a -match '插X')) `
    "caption=$capA p2=$t2a"
Close-WordDoc $docA

$rejDel = Get-OutPath 'docx-revision-reject-del.docx'
Close-WordDocByPath $word $rejDel
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $rejDel.Replace('\', '/')
    ops = @(@{ op = 'rejectRevision'; id = [string]$del.id })
} 'docx-revision-reject-del' | Out-Null
$docN = Open-WordDoc $word $rejDel
Set-FinalView $docN
$capN = [string]$docN.ActiveWindow.Caption
$t1n = Get-ParaText $docN.Paragraphs.Item(1)
Add-Check 'docx-Rv5' (($capN -notmatch 'Repair') -and ($t1n -match '甲')) `
    "caption=$capN p1=$t1n"
Close-WordDoc $docN

$failOut = Get-OutPath 'docx-revision-set.docx'
if (Test-Path $failOut) { Remove-Item -Force $failOut }
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'setParaText'; id = [string]$del.host; text = '不该写' })
} 'docx-revision-set-text'
$failCode = $null
try { $failCode = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$noWrite = -not (Test-Path $failOut)
$rv6ok = ($fail.ExitCode -ne 0) -and ($failCode -eq 'NotImplemented') -and $noWrite
Add-Check 'docx-Rv6' $rv6ok "code=$failCode exit=$($fail.ExitCode) wrote=$(-not $noWrite)"
