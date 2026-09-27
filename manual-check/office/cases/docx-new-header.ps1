# docx 新建页眉与奇偶首页：docs/checklist-docx.md 的 Nh1–Nh7。
# Nh1 / Nh5 用库 create；Nh4 / Nh7 用 Word 两节另存样例。

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
$wdPrimary = 1
$wdFirst = 2
$wdEven = 3

$blank = Get-OutPath 'docx-nh-blank.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $blank.Replace('\', '/'); format = 'docx'
    document = @{ paragraphs = @(@{ style = 'Heading1'; text = $S.chapter1 }) }
} 'docx-nh-blank-create' | Out-Null

$word = Ensure-MsWord $blank

# Nh1：无页眉文档新建默认页眉并改字。
$ins = Get-OutPath 'docx-nh-insert-default.docx'
Close-WordDocByPath $word $ins
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $blank.Replace('\', '/')
    output = $ins.Replace('\', '/')
    ops = @(@{ op = 'insertHeaderFooter'; id = 'section:1'; kind = 'header'; type = 'default' })
} 'docx-nh-insert-default' | Out-Null
$insTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ins.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true }
} 'docx-nh-insert-inspect'
$hid = $insTree.headers[0].paragraphs[0].id
$ins2 = Get-OutPath 'docx-nh-insert-edit.docx'
Close-WordDocByPath $word $ins2
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $ins.Replace('\', '/')
    output = $ins2.Replace('\', '/')
    ops = @(@{ op = 'setParaText'; id = $hid; text = $S.headerNew })
} 'docx-nh-insert-edit' | Out-Null
$doc1 = Open-WordDoc $word $ins2
$cap1 = [string]$doc1.ActiveWindow.Caption
$hdr1 = $doc1.Sections.Item(1).Headers.Item($wdPrimary)
$hdr1t = [string]$hdr1.Range.Text
$body1 = Get-ParaText $doc1.Paragraphs.Item(1)
Add-Check 'docx-Nh1' (
    ($cap1 -notmatch 'Repair') -and ($hdr1t -match [regex]::Escape($S.headerNew)) -and
    ($body1 -eq $S.chapter1)
) "caption=$cap1 hdr=$hdr1t body=$body1"
Release-Com $hdr1
Close-WordDoc $doc1

# Nh2：首页不同 + first 页眉。
$title = Get-OutPath 'docx-nh-title.docx'
Close-WordDocByPath $word $title
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $blank.Replace('\', '/')
    output = $title.Replace('\', '/')
    ops = @(
        @{ op = 'setTitlePage'; id = 'section:1'; titlePage = $true }
        @{ op = 'insertHeaderFooter'; id = 'section:1'; kind = 'header'; type = 'first' }
    )
} 'docx-nh-title' | Out-Null
$titleTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $title.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true }
} 'docx-nh-title-inspect'
$fid = (@($titleTree.headers) | Where-Object { $_.type -eq 'first' } | Select-Object -First 1).paragraphs[0].id
$title2 = Get-OutPath 'docx-nh-title-edit.docx'
Close-WordDocByPath $word $title2
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $title.Replace('\', '/')
    output = $title2.Replace('\', '/')
    ops = @(@{ op = 'setParaText'; id = $fid; text = $S.headerFirst })
} 'docx-nh-title-edit' | Out-Null
$docT = Open-WordDoc $word $title2
$capT = [string]$docT.ActiveWindow.Caption
$diffFirst = [bool]$docT.Sections.Item(1).PageSetup.DifferentFirstPageHeaderFooter
$firstHdr = $docT.Sections.Item(1).Headers.Item($wdFirst)
$firstT = [string]$firstHdr.Range.Text
Add-Check 'docx-Nh2' (
    ($capT -notmatch 'Repair') -and $diffFirst -and ($firstT -match [regex]::Escape($S.headerFirst))
) "caption=$capT first=$diffFirst text=$firstT"
Release-Com $firstHdr
Close-WordDoc $docT

# Nh3：奇偶页 + even 页眉。
$even = Get-OutPath 'docx-nh-even.docx'
Close-WordDocByPath $word $even
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $blank.Replace('\', '/')
    output = $even.Replace('\', '/')
    ops = @(
        @{ op = 'setEvenAndOddHeaders'; id = 'docx:document'; evenAndOdd = $true }
        @{ op = 'insertHeaderFooter'; id = 'section:1'; kind = 'header'; type = 'even' }
    )
} 'docx-nh-even' | Out-Null
$evenTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $even.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true }
} 'docx-nh-even-inspect'
$eid = (@($evenTree.headers) | Where-Object { $_.type -eq 'even' } | Select-Object -First 1).paragraphs[0].id
$even2 = Get-OutPath 'docx-nh-even-edit.docx'
Close-WordDocByPath $word $even2
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $even.Replace('\', '/')
    output = $even2.Replace('\', '/')
    ops = @(@{ op = 'setParaText'; id = $eid; text = $S.headerEven })
} 'docx-nh-even-edit' | Out-Null
$docE = Open-WordDoc $word $even2
$capE = [string]$docE.ActiveWindow.Caption
$oddEven = [bool]$docE.PageSetup.OddAndEvenPagesHeaderFooter
$evenHdr = $docE.Sections.Item(1).Headers.Item($wdEven)
$evenT = [string]$evenHdr.Range.Text
Add-Check 'docx-Nh3' (
    ($capE -notmatch 'Repair') -and $oddEven -and ($evenT -match [regex]::Escape($S.headerEven))
) "caption=$capE oddEven=$oddEven text=$evenT"
Release-Com $evenHdr
Close-WordDoc $docE

# Nh4：两节，打开奇偶后断链第二节 even 并改字。
$linkSrc = Ensure-DocxWordLinkSource
$oddOn = Get-OutPath 'docx-nh-sec2-odd.docx'
Close-WordDocByPath $word $oddOn
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $linkSrc.Replace('\', '/')
    output = $oddOn.Replace('\', '/')
    ops = @(
        @{ op = 'setEvenAndOddHeaders'; id = 'docx:document'; evenAndOdd = $true }
        @{ op = 'insertHeaderFooter'; id = 'section:1'; kind = 'header'; type = 'even' }
        @{ op = 'setHeaderFooterLink'; id = 'section:2'; kind = 'header'; type = 'even'; linked = $false }
    )
} 'docx-nh-sec2-unlink' | Out-Null
$s2Tree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $oddOn.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true }
} 'docx-nh-sec2-inspect'
$h2 = @($s2Tree.headers) | Where-Object {
    $_.type -eq 'even' -and (@($_.sections) -contains 2)
} | Select-Object -First 1
$h2id = @($h2.paragraphs)[0].id
$s2edit = Get-OutPath 'docx-nh-sec2-edit.docx'
Close-WordDocByPath $word $s2edit
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $oddOn.Replace('\', '/')
    output = $s2edit.Replace('\', '/')
    ops = @(@{ op = 'setParaText'; id = $h2id; text = $S.headerSec2Even })
} 'docx-nh-sec2-edit' | Out-Null
$docS = Open-WordDoc $word $s2edit
$capS = [string]$docS.ActiveWindow.Caption
$s1e = $docS.Sections.Item(1).Headers.Item($wdEven)
$s2e = $docS.Sections.Item(2).Headers.Item($wdEven)
$s1et = [string]$s1e.Range.Text
$s2et = [string]$s2e.Range.Text
Add-Check 'docx-Nh4' (
    ($capS -notmatch 'Repair') -and ($s2et -match [regex]::Escape($S.headerSec2Even)) -and
    ($s1et -notmatch [regex]::Escape($S.headerSec2Even))
) "caption=$capS s1=$s1et s2=$s2et"
Release-Com $s1e
Release-Com $s2e
Close-WordDoc $docS

# Nh5：create 同时写 default / even / first。
$created = Get-OutPath 'docx-nh-create.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $created.Replace('\', '/'); format = 'docx'
    document = @{
        paragraphs = @(@{ style = 'Heading1'; text = $S.chapter1 })
        headers = @(
            @{ type = 'default'; paragraphs = @(@{ text = $S.headerNew }) }
            @{ type = 'even'; paragraphs = @(@{ text = $S.headerEven }) }
            @{ type = 'first'; paragraphs = @(@{ text = $S.headerFirst }) }
        )
    }
} 'docx-nh-create' | Out-Null
$docC = Open-WordDoc $word $created
$capC = [string]$docC.ActiveWindow.Caption
$cOdd = [bool]$docC.PageSetup.OddAndEvenPagesHeaderFooter
$cFirst = [bool]$docC.Sections.Item(1).PageSetup.DifferentFirstPageHeaderFooter
$cDef = [string]$docC.Sections.Item(1).Headers.Item($wdPrimary).Range.Text
$cEv = [string]$docC.Sections.Item(1).Headers.Item($wdEven).Range.Text
$cFi = [string]$docC.Sections.Item(1).Headers.Item($wdFirst).Range.Text
Add-Check 'docx-Nh5' (
    ($capC -notmatch 'Repair') -and $cOdd -and $cFirst -and
    ($cDef -match [regex]::Escape($S.headerNew)) -and
    ($cEv -match [regex]::Escape($S.headerEven)) -and
    ($cFi -match [regex]::Escape($S.headerFirst))
) "caption=$capC odd=$cOdd first=$cFirst d=$cDef e=$cEv f=$cFi"
Close-WordDoc $docC

# Nh6：未开奇偶页时插 even。
$fail6 = Get-OutPath 'docx-nh-even-fail.docx'
if (Test-Path $fail6) { Remove-Item -Force $fail6 }
$raw6 = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $blank.Replace('\', '/')
    output = $fail6.Replace('\', '/')
    ops = @(@{ op = 'insertHeaderFooter'; id = 'section:1'; kind = 'header'; type = 'even' })
} 'docx-nh-even-fail'
try { $code6 = [string](($raw6.Stderr | ConvertFrom-Json).error.code) } catch { $code6 = $raw6.Stderr }
Add-Check 'docx-Nh6' (
    ($raw6.ExitCode -ne 0) -and ($code6 -eq 'InvalidField') -and (-not (Test-Path $fail6))
) "code=$code6 wrote=$(Test-Path $fail6)"

# Nh7：第二节仍链接时 insertHeaderFooter。
$fail7 = Get-OutPath 'docx-nh-linked-fail.docx'
if (Test-Path $fail7) { Remove-Item -Force $fail7 }
$raw7 = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $linkSrc.Replace('\', '/')
    output = $fail7.Replace('\', '/')
    ops = @(@{ op = 'insertHeaderFooter'; id = 'section:2'; kind = 'header'; type = 'default' })
} 'docx-nh-linked-fail'
try { $code7 = [string](($raw7.Stderr | ConvertFrom-Json).error.code) } catch { $code7 = $raw7.Stderr }
Add-Check 'docx-Nh7' (
    ($raw7.ExitCode -ne 0) -and ($code7 -eq 'InvalidField') -and (-not (Test-Path $fail7))
) "code=$code7 wrote=$(Test-Path $fail7)"
