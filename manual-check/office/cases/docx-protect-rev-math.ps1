# docx 保护、修订与公式属性：docs/checklist-docx.md 的 Pq1–Pq8。
# 编号 Pq，不要和 Pr、Ph、Rm、Mm、Ms、Bi 撞。不算排版引擎，不破解密码。

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

function Get-FailCode($fail) {
    try { return ($fail.Stderr | ConvertFrom-Json).error.code } catch { return $null }
}

function Rewrite-ZipPart([string]$zipPath, [string]$part, [scriptblock]$edit) {
    $zip = [IO.Compression.ZipFile]::Open($zipPath, 'Update')
    try {
        $entry = $zip.Entries | Where-Object { ($_.FullName -replace '\\', '/') -eq $part } | Select-Object -First 1
        if (-not $entry) { throw "zip 里没有 $part" }
        $ms = New-Object IO.MemoryStream
        $es = $entry.Open()
        $es.CopyTo($ms)
        $es.Dispose()
        $xml = [Text.Encoding]::UTF8.GetString($ms.ToArray())
        $text2 = & $edit $xml
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

function Open-WordDocWritePassword($word, [string]$path, [string]$writePassword) {
    foreach ($d in @($word.Documents)) {
        if ($d.FullName -and ($d.FullName -ieq $path)) { return $d }
    }
    $missing = [Type]::Missing
    return $word.Documents.Open($path, $false, $false, $false, $missing, $missing, $false, $writePassword)
}

function Get-OMathInfo($doc) {
    $omaths = $null
    try {
        $omaths = $doc.OMaths
        $n = [int]$omaths.Count
        if ($n -lt 1) {
            return @{ Count = 0; Type = -1; Jc = -1; Text = '' }
        }
        $om = $null
        try {
            $om = $omaths.Item(1)
            $typ = [int]$om.Type
            $jc = [int]$om.Justification
            $rg = $null
            try {
                $rg = $om.Range
                $txt = [string]$rg.Text
            } finally { Release-Com $rg }
            return @{ Count = $n; Type = $typ; Jc = $jc; Text = $txt }
        } finally { Release-Com $om }
    } finally { Release-Com $omaths }
}

function Get-KindIds($tree, [string[]]$kinds) {
    $arr = @($tree.revisions | Where-Object { $kinds -contains [string]$_.kind })
    return @($arr | Sort-Object { [int]$_.index } -Descending | ForEach-Object { [string]$_.id })
}

$S = $global:OoxpStrings
$hostPath = Get-OutPath 'docx-protect-rev-math-host.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = $S.chapter1 }
            @{ text = $S.hello }
        )
        headers = @(@{ type = 'default'; paragraphs = @(@{ text = $S.headerSrc }) })
    }
} 'docx-pq-create' | Out-Null

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/')
    scope = @{ from = 1; maxParas = 20; editRanges = $true; headerFooter = $true }
} 'docx-pq-inspect-src'
$pHello = $null
foreach ($p in @($srcTree.paragraphs)) {
    if ([string]$p.text -eq [string]$S.hello) { $pHello = [string]$p.id; break }
}
$hp = [string]$srcTree.headers[0].paragraphs[0].id
$bareCount = $srcTree.editRangeCount
$bareForm = $srcTree.sections[0].formProt
$bareBib = $srcTree.bibliographyStyle

$permOut = Get-OutPath 'docx-protect-rev-math-perm.docx'
if (Test-Path $permOut) { Remove-Item -Force $permOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $permOut.Replace('\', '/')
    ops = @(
        @{ op = 'insertEditRange'; id = $pHello }
        @{ op = 'insertEditRange'; id = $hp; edGrp = 'editors' }
    )
} 'docx-pq-perm' | Out-Null
$permTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $permOut.Replace('\', '/')
    scope = @{ from = 1; maxParas = 20; editRanges = $true; headerFooter = $true }
} 'docx-pq-inspect-perm'
$permXml = Get-ZipPartText $permOut 'word/document.xml'
$word = Ensure-MsWord $hostPath
Close-WordDocByPath $word $permOut
$docPerm = Open-WordDoc $word $permOut
$capPerm = [string]$docPerm.ActiveWindow.Caption
Close-WordDoc $docPerm
$pq1ok = ($bareCount -eq 0) -and ($null -eq $bareForm) -and ($null -eq $bareBib) `
    -and ([int]$permTree.editRangeCount -eq 1) `
    -and ([string]$permTree.editRanges[0].edGrp -eq 'everyone') `
    -and ([string]$permTree.editRanges[0].text -eq [string]$S.hello) `
    -and ([int]$permTree.headers[0].editRangeCount -eq 1) `
    -and ($permXml -match 'permStart') `
    -and ($capPerm -notmatch 'Repair')
Add-Check 'docx-Pq1' $pq1ok "count=$($permTree.editRangeCount) cap=$capPerm"

$formsOut = Get-OutPath 'docx-protect-rev-math-forms.docx'
if (Test-Path $formsOut) { Remove-Item -Force $formsOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $formsOut.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'docx:document'; enforced = $true; edit = 'forms' })
} 'docx-pq-forms' | Out-Null
$excOut = Get-OutPath 'docx-protect-rev-math-formprot.docx'
if (Test-Path $excOut) { Remove-Item -Force $excOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $formsOut.Replace('\', '/')
    output = $excOut.Replace('\', '/')
    ops = @(@{ op = 'setFormProt'; id = 'section:1'; formProt = $false })
} 'docx-pq-formprot' | Out-Null
$fpTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $excOut.Replace('\', '/')
} 'docx-pq-inspect-formprot'
Close-WordDocByPath $word $excOut
$docFp = Open-WordDoc $word $excOut
$capFp = [string]$docFp.ActiveWindow.Caption
$ptypeFp = [int]$docFp.ProtectionType
$sec = $null
try {
    $sec = $docFp.Sections.Item(1)
    $pf = [bool]$sec.ProtectedForForms
} finally { Release-Com $sec }
Close-WordDoc $docFp
$pq2ok = ($fpTree.sections[0].formProt -eq $false) `
    -and ($ptypeFp -eq 2) `
    -and (-not $pf) `
    -and ($capFp -notmatch 'Repair')
Add-Check 'docx-Pq2' $pq2ok "formProt=$($fpTree.sections[0].formProt) type=$ptypeFp pf=$pf cap=$capFp"

$wpOut = Get-OutPath 'docx-protect-rev-math-writepw.docx'
if (Test-Path $wpOut) { Remove-Item -Force $wpOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $wpOut.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'docx:document'; writePassword = 'secret' })
} 'docx-pq-writepw' | Out-Null
$wpTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $wpOut.Replace('\', '/')
} 'docx-pq-inspect-writepw'
$wpDump = ($wpTree | ConvertTo-Json -Depth 12 -Compress)
$wpXml = Get-ZipPartText $wpOut 'word/settings.xml'
Close-WordDocByPath $word $wpOut
$docWp = Open-WordDocWritePassword $word $wpOut 'secret'
$capWp = [string]$docWp.ActiveWindow.Caption
$reserved = $false
try { $reserved = [bool]$docWp.WriteReserved } catch {}
$ro = $false
try { $ro = [bool]$docWp.ReadOnly } catch {}
Close-WordDoc $docWp
$pq3ok = ([bool]$wpTree.protection.hasPassword) `
    -and ($wpDump -notmatch 'secret') `
    -and ($wpXml -match 'cryptAlgorithmSid="14"') `
    -and ($wpXml -match 'writeProtection') `
    -and ($capWp -notmatch 'Repair') `
    -and (-not $ro)
Add-Check 'docx-Pq3' $pq3ok "hash=$($wpTree.protection.hasPassword) reserved=$reserved ro=$ro cap=$capWp"

$failBlank = Get-OutPath 'docx-protect-rev-math-blank.docx'
$failWrong = Get-OutPath 'docx-protect-rev-math-wrong.docx'
$failNone = Get-OutPath 'docx-protect-rev-math-none.docx'
if (Test-Path $failBlank) { Remove-Item -Force $failBlank }
if (Test-Path $failWrong) { Remove-Item -Force $failWrong }
if (Test-Path $failNone) { Remove-Item -Force $failNone }
$badBlank = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failBlank.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'docx:document'; writePassword = '' })
} 'docx-pq-blank'
$badWrong = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $wpOut.Replace('\', '/')
    output = $failWrong.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'docx:document'; writePassword = 'wrong'; readOnlyRecommended = $false })
} 'docx-pq-wrong'
$badNone = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $wpOut.Replace('\', '/')
    output = $failNone.Replace('\', '/')
    ops = @(@{ op = 'setProtection'; id = 'docx:document'; readOnlyRecommended = $true })
} 'docx-pq-none'
$wroteWp = (Test-Path $failBlank) -or (Test-Path $failWrong) -or (Test-Path $failNone)
$pq4ok = ((Get-FailCode $badBlank) -eq 'InvalidField') `
    -and ((Get-FailCode $badWrong) -eq 'InvalidField') `
    -and ((Get-FailCode $badNone) -eq 'NotImplemented') `
    -and (-not $wroteWp)
Add-Check 'docx-Pq4' $pq4ok "blank=$(Get-FailCode $badBlank) wrong=$(Get-FailCode $badWrong) none=$(Get-FailCode $badNone) wrote=$wroteWp"

$mvPath = Get-OutPath 'docx-protect-rev-math-move.docx'
if (Test-Path $mvPath) { Remove-Item -Force $mvPath }
Copy-Item -Force $hostPath $mvPath
Rewrite-ZipPart $mvPath 'word/document.xml' {
    param($xml)
    $from = '<w:moveFromRangeStart w:id="5" w:name="mv1" w:author="A" w:date="2026-01-01T00:00:00Z"/><w:moveFrom w:id="5" w:author="A"><w:r><w:t>' + $S.hello + '</w:t></w:r></w:moveFrom><w:moveFromRangeEnd w:id="5"/>'
    $to = '<w:moveToRangeStart w:id="6" w:name="mv1" w:author="A" w:date="2026-01-01T00:00:00Z"/><w:moveTo w:id="6" w:author="A"><w:r><w:t>' + $S.hello + '</w:t></w:r></w:moveTo><w:moveToRangeEnd w:id="6"/>'
    $pat = '<w:r>(?:(?!</w:r>).)*?<w:t[^>]*>' + [regex]::Escape([string]$S.hello) + '</w:t></w:r>'
    $m = [regex]::Match($xml, $pat)
    if (-not $m.Success) { throw "document.xml 里找不到 $($S.hello) 所在 run" }
    return $xml.Substring(0, $m.Index) + $from + $to + $xml.Substring($m.Index + $m.Length)
}
$mvTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $mvPath.Replace('\', '/')
    scope = @{ revisions = $true; from = 1; maxParas = 20 }
} 'docx-pq-inspect-move'
$mvIds = Get-KindIds $mvTree @('moveFrom', 'moveTo')
$mvAcc = Get-OutPath 'docx-protect-rev-math-move-acc.docx'
if (Test-Path $mvAcc) { Remove-Item -Force $mvAcc }
$mvOps = @()
foreach ($id in $mvIds) { $mvOps += @{ op = 'acceptRevision'; id = $id } }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $mvPath.Replace('\', '/')
    output = $mvAcc.Replace('\', '/')
    ops = $mvOps
} 'docx-pq-move-acc' | Out-Null
Close-WordDocByPath $word $mvAcc
$docMv = Open-WordDoc $word $mvAcc
$capMv = [string]$docMv.ActiveWindow.Caption
$revN = 0
try { $revN = [int]$docMv.Revisions.Count } catch {}
Close-WordDoc $docMv
$kinds = @($mvTree.revisions | ForEach-Object { [string]$_.kind })
$pq5ok = ($kinds -contains 'moveFrom') -and ($kinds -contains 'moveTo') `
    -and ($capMv -notmatch 'Repair')
Add-Check 'docx-Pq5' $pq5ok "kinds=$($kinds -join ',') revN=$revN cap=$capMv"

$mathOut = Get-OutPath 'docx-protect-rev-math-math.docx'
if (Test-Path $mathOut) { Remove-Item -Force $mathOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $mathOut.Replace('\', '/')
    ops = @(
        @{
            op = 'insertMath'; id = $pHello; kind = 'display'; jc = 'center'
            expr = @(
                @{ type = 'phant'; show = $false; e = @(@{ type = 'text'; text = 'x'; brk = $true }) }
                @{ type = 'borderBox'; hideTop = $true; e = @(@{ type = 'text'; text = 'y' }) }
            )
        }
    )
} 'docx-pq-math' | Out-Null
$mathTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $mathOut.Replace('\', '/')
    scope = @{ from = 1; maxParas = 20; maths = $true }
} 'docx-pq-inspect-math'
Close-WordDocByPath $word $mathOut
$docMath = Open-WordDoc $word $mathOut
$capMath = [string]$docMath.ActiveWindow.Caption
$om = Get-OMathInfo $docMath
Close-WordDoc $docMath
$e0 = @($mathTree.maths[0].expr)[0]
$e1 = @($mathTree.maths[0].expr)[1]
$pq6ok = ([string]$mathTree.maths[0].jc -eq 'center') `
    -and ([string]$e0.type -eq 'phant') `
    -and ([string]$e1.type -eq 'borderBox') `
    -and ([bool]$e0.e[0].brk) `
    -and ($om.Count -ge 1) `
    -and ($om.Jc -eq 2) `
    -and ($capMath -notmatch 'Repair')
Add-Check 'docx-Pq6' $pq6ok "jc=$($mathTree.maths[0].jc) omJc=$($om.Jc) n=$($om.Count) cap=$capMath"

$bibOut = Get-OutPath 'docx-protect-rev-math-biblio.docx'
if (Test-Path $bibOut) { Remove-Item -Force $bibOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $bibOut.Replace('\', '/')
    ops = @(@{ op = 'setBibliographyStyle'; id = 'docx:document'; styleName = 'MLA' })
} 'docx-pq-biblio' | Out-Null
$bibTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $bibOut.Replace('\', '/')
} 'docx-pq-inspect-biblio'
Close-WordDocByPath $word $bibOut
$docBib = Open-WordDoc $word $bibOut
$capBib = [string]$docBib.ActiveWindow.Caption
$bibStyle = ''
$bibObj = $null
try {
    $bibObj = $docBib.Bibliography
    $bibStyle = [string]$bibObj.BibliographyStyle
} catch {
    Write-Log ("BibliographyStyle FAIL " + $_.Exception.Message)
} finally { Release-Com $bibObj }
Close-WordDoc $docBib
$pq7ok = ([string]$bibTree.bibliographyStyle.styleName -eq 'MLA') `
    -and ([string]$bibTree.bibliographyStyle.version -eq '7') `
    -and ($capBib -notmatch 'Repair') `
    -and (($bibStyle -eq '') -or ($bibStyle -eq 'MLA'))
Add-Check 'docx-Pq7' $pq7ok "style=$($bibTree.bibliographyStyle.styleName) com=$bibStyle cap=$capBib"

$xlsx = Get-OutPath 'docx-protect-rev-math.xlsx'
$pptx = Get-OutPath 'docx-protect-rev-math.pptx'
$xout = Get-OutPath 'docx-protect-rev-math-x-out.xlsx'
$pout = Get-OutPath 'docx-protect-rev-math-p-out.pptx'
if (Test-Path $xlsx) { Remove-Item -Force $xlsx }
if (Test-Path $pptx) { Remove-Item -Force $pptx }
if (Test-Path $xout) { Remove-Item -Force $xout }
if (Test-Path $pout) { Remove-Item -Force $pout }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1'; cells = @(@{ ref = 'A1'; value = 1 }) }) }
} 'docx-pq-xlsx' | Out-Null
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $pptx.Replace('\', '/')
    presentation = @{ slides = @(@{ shapes = @(@{ text = 'a' }) }) }
} 'docx-pq-pptx' | Out-Null
$badX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $xout.Replace('\', '/')
    ops = @(@{ op = 'insertEditRange'; id = $pHello })
} 'docx-pq-xlsx-op'
$badP = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $pptx.Replace('\', '/')
    output = $pout.Replace('\', '/')
    ops = @(@{ op = 'setBibliographyStyle'; id = 'docx:document'; styleName = 'APA' })
} 'docx-pq-pptx-op'
$pq8ok = ((Get-FailCode $badX) -eq 'InvalidField') `
    -and ((Get-FailCode $badP) -eq 'InvalidField') `
    -and (-not (Test-Path $xout)) `
    -and (-not (Test-Path $pout))
Add-Check 'docx-Pq8' $pq8ok "x=$(Get-FailCode $badX) p=$(Get-FailCode $badP)"
