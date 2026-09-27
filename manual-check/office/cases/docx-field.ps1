# docx 域：docs/checklist-docx.md 的 Fd1–Fd7。
# 用 ooxp 插 PAGE/DATE，再用 Word 打开核对无修复；不调 COM 的 Fields.Update 当验收。

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

$wdPrimary = 1

$src = Get-OutPath 'docx-field-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = '页码' }
            @{ text = '日期' }
            @{ text = '干净' }
            @{ text = '留指令' }
        )
    }
} 'docx-field-create' | Out-Null

$withHf = Get-OutPath 'docx-field-hf.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $withHf.Replace('\', '/')
    ops = @(@{ op = 'insertHeaderFooter'; id = 'section:1'; kind = 'header'; type = 'default' })
} 'docx-field-insert-hf' | Out-Null

$hfTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $withHf.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true; from = 1; maxParas = 20 }
} 'docx-field-hf-inspect'
$hdrPara = $hfTree.headers[0].paragraphs[0].id
$body = @($hfTree.paragraphs)
$pPage = @($body | Where-Object { [string]$_.text -eq '页码' }) | Select-Object -First 1
$pDate = @($body | Where-Object { [string]$_.text -eq '日期' }) | Select-Object -First 1
$pClean = @($body | Where-Object { [string]$_.text -eq '干净' }) | Select-Object -First 1
$pKeep = @($body | Where-Object { [string]$_.text -eq '留指令' }) | Select-Object -First 1

$seed = Get-OutPath 'docx-field-seed.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $withHf.Replace('\', '/')
    output = $seed.Replace('\', '/')
    ops = @(
        @{ op = 'insertField'; id = [string]$hdrPara; instr = ' PAGE '; result = '1' }
        @{ op = 'insertField'; id = [string]$pPage.id; instr = ' PAGE '; result = '1' }
        @{ op = 'insertField'; id = [string]$pDate.id; instr = ' DATE '; result = '2026' }
        @{ op = 'insertField'; id = [string]$pKeep.id; instr = ' PAGE '; result = '9' }
    )
} 'docx-field-seed' | Out-Null

$word = Ensure-MsWord $seed

$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $seed.Replace('\', '/'); format = 'docx'
} 'docx-field-inspect'
$listed = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $seed.Replace('\', '/'); format = 'docx'
    scope = @{ fields = $true; headerFooter = $true; from = 1; maxParas = 20 }
} 'docx-field-inspect-list'
$fields = @($listed.fields)
$pageF = @($fields | Where-Object { $_.code -eq 'PAGE' -and ([string]$_.host) -notlike 'header:*' }) | Select-Object -First 1
$dateF = @($fields | Where-Object { $_.code -eq 'DATE' }) | Select-Object -First 1
$keepF = @($fields | Where-Object { $_.code -eq 'PAGE' -and ([string]$_.host) -eq [string]$pKeep.id }) | Select-Object -First 1
$hdrFields = @($listed.headers[0].fields)
$hdrPage = @($hdrFields | Where-Object { $_.code -eq 'PAGE' }) | Select-Object -First 1
$fd1ok = ($null -ne $tree0.fieldCount) -and ([int]$tree0.fieldCount -ge 3) -and
    ($null -eq $tree0.fields) -and ($null -ne $pageF) -and ($null -ne $pageF.instr) -and
    ($null -ne $pageF.code) -and ([string]$pageF.code -eq 'PAGE')
Add-Check 'docx-Fd1' $fd1ok "count=$($tree0.fieldCount) page=$($pageF.id) instr=$($pageF.instr) code=$($pageF.code)"

$setOut = Get-OutPath 'docx-field-set.docx'
Close-WordDocByPath $word $setOut
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $seed.Replace('\', '/')
    output = $setOut.Replace('\', '/')
    ops = @(@{ op = 'setField'; id = [string]$pageF.id; instr = ' DATE ' })
} 'docx-field-set' | Out-Null
$docS = Open-WordDoc $word $setOut
$capS = [string]$docS.ActiveWindow.Caption
$afterSet = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $setOut.Replace('\', '/'); format = 'docx'
    scope = @{ fields = $true }
} 'docx-field-set-inspect'
$setHit = @($afterSet.fields) | Where-Object { $_.id -eq $pageF.id } | Select-Object -First 1
Add-Check 'docx-Fd2' (($capS -notmatch 'Repair') -and ([string]$setHit.code -eq 'DATE')) "caption=$capS code=$($setHit.code)"
Close-WordDoc $docS

$ins = Get-OutPath 'docx-field-insert.docx'
Close-WordDocByPath $word $ins
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $seed.Replace('\', '/')
    output = $ins.Replace('\', '/')
    ops = @(@{
        op = 'insertField'
        id = [string]$pClean.id
        instr = ' DATE '
        result = '插'
    })
} 'docx-field-insert' | Out-Null
$docI = Open-WordDoc $word $ins
$capI = [string]$docI.ActiveWindow.Caption
$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ins.Replace('\', '/'); format = 'docx'
    scope = @{ fields = $true; from = 1; maxParas = 20 }
} 'docx-field-insert-inspect'
$sawNew = @($treeI.fields) | Where-Object { $_.host -eq $pClean.id } | Select-Object -First 1
Add-Check 'docx-Fd3' (($capI -notmatch 'Repair') -and ($null -ne $sawNew) -and ([string]$sawNew.code -eq 'DATE')) "caption=$capI new=$($sawNew.id)"
Close-WordDoc $docI

$delRes = Get-OutPath 'docx-field-del-result.docx'
Close-WordDocByPath $word $delRes
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $seed.Replace('\', '/')
    output = $delRes.Replace('\', '/')
    ops = @(@{ op = 'deleteField'; id = [string]$dateF.id })
} 'docx-field-del-result' | Out-Null
$docR = Open-WordDoc $word $delRes
$capR = [string]$docR.ActiveWindow.Caption
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delRes.Replace('\', '/'); format = 'docx'
    scope = @{ fields = $true; from = 1; maxParas = 20 }
} 'docx-field-del-result-inspect'
$stillDate = @($treeR.fields) | Where-Object { $_.host -eq $dateF.host } | Select-Object -First 1
$pDateAfter = @($treeR.paragraphs) | Where-Object { $_.id -eq $pDate.id } | Select-Object -First 1
Add-Check 'docx-Fd4' (
    ($capR -notmatch 'Repair') -and ($null -eq $stillDate) -and
    (([string]$pDateAfter.text) -match '2026')
) "caption=$capR gone=$($null -eq $stillDate) text=$($pDateAfter.text)"
Close-WordDoc $docR

$delInstr = Get-OutPath 'docx-field-del-instr.docx'
Close-WordDocByPath $word $delInstr
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $seed.Replace('\', '/')
    output = $delInstr.Replace('\', '/')
    ops = @(@{ op = 'deleteField'; id = [string]$keepF.id; keep = 'instr' })
} 'docx-field-del-instr' | Out-Null
$docK = Open-WordDoc $word $delInstr
$capK = [string]$docK.ActiveWindow.Caption
$treeK = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delInstr.Replace('\', '/'); format = 'docx'
    scope = @{ fields = $true; from = 1; maxParas = 20 }
} 'docx-field-del-instr-inspect'
$stillKeep = @($treeK.fields) | Where-Object { $_.host -eq $keepF.host } | Select-Object -First 1
$pKeepAfter = @($treeK.paragraphs) | Where-Object { $_.id -eq $pKeep.id } | Select-Object -First 1
Add-Check 'docx-Fd5' (
    ($capK -notmatch 'Repair') -and ($null -eq $stillKeep) -and
    (([string]$pKeepAfter.text) -match 'PAGE')
) "caption=$capK gone=$($null -eq $stillKeep) text=$($pKeepAfter.text)"
Close-WordDoc $docK

$docH = Open-WordDoc $word $seed
$capH = [string]$docH.ActiveWindow.Caption
$hdr = $docH.Sections.Item(1).Headers.Item($wdPrimary)
$hdrText = [string]$hdr.Range.Text
Release-Com $hdr
$fd6ok = ($capH -notmatch 'Repair') -and ($null -ne $hdrPage) -and
    (([string]$hdrPage.id).StartsWith('header:')) -and ([string]$hdrPage.code -eq 'PAGE')
Add-Check 'docx-Fd6' $fd6ok "caption=$capH id=$($hdrPage.id) hdrText=$hdrText"
Close-WordDoc $docH

$failOut = Get-OutPath 'docx-field-set-text.docx'
if (Test-Path $failOut) { Remove-Item -Force $failOut }
$fail = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $seed.Replace('\', '/')
    output = $failOut.Replace('\', '/')
    ops = @(@{ op = 'setParaText'; id = [string]$pageF.host; text = '不该写' })
} 'docx-field-set-text'
$failCode = $null
try { $failCode = ($fail.Stderr | ConvertFrom-Json).error.code } catch {}
$noWrite = -not (Test-Path $failOut)
$fd7ok = ($fail.ExitCode -ne 0) -and ($failCode -eq 'NotImplemented') -and $noWrite
Add-Check 'docx-Fd7' $fd7ok "code=$failCode exit=$($fail.ExitCode) wrote=$(-not $noWrite)"
