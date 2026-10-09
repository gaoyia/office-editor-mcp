# docx 域类型：docs/checklist-docx.md 的 Ft1–Ft7。
# 用 ooxp 按 code 写 PAGE / MERGEFIELD / 窗体 ffData，再用 Word 核对无修复。不求值。

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

function Get-FailCode($result) {
    $code = $null
    try { $code = ($result.Stderr | ConvertFrom-Json).error.code } catch {}
    return $code
}

function Get-FieldCounts($doc) {
    $n = 0
    $fn = 0
    try { $n = [int]$doc.Fields.Count } catch {}
    try { $fn = [int]$doc.FormFields.Count } catch {}
    return @{ Fields = $n; Forms = $fn }
}

function Get-ParaId($tree, [string]$text) {
    $hit = @($tree.paragraphs | Where-Object { [string]$_.text -eq $text }) | Select-Object -First 1
    if ($null -eq $hit) { throw "找不到段落：$text" }
    return [string]$hit.id
}

$src = Get-OutPath 'docx-field-types-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = '页码' }
            @{ text = '合并' }
            @{ text = '窗体' }
            @{ text = '复选' }
            @{ text = '下拉' }
            @{ text = '拒错' }
        )
    }
} 'docx-field-types-create' | Out-Null

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-field-types-inspect-src'
$pPage = Get-ParaId $srcTree '页码'
$pMerge = Get-ParaId $srcTree '合并'
$pForm = Get-ParaId $srcTree '窗体'
$pCheck = Get-ParaId $srcTree '复选'
$pDrop = Get-ParaId $srcTree '下拉'
$pFail = Get-ParaId $srcTree '拒错'

$word = Ensure-MsWord $src

$pageOut = Get-OutPath 'docx-field-types-page.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $pageOut.Replace('\', '/')
    ops = @(@{ op = 'insertField'; id = $pPage; code = 'PAGE' })
} 'docx-field-types-page' | Out-Null
$treePage = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $pageOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; fields = $true }
} 'docx-field-types-inspect-page'
$pageF = @($treePage.fields) | Where-Object { $_.host -eq $pPage } | Select-Object -First 1
$docPage = Open-WordDoc $word $pageOut
$capPage = [string]$docPage.ActiveWindow.Caption
$cntPage = Get-FieldCounts $docPage
Add-Check 'docx-Ft1' (
    ([string]$pageF.code -eq 'PAGE') -and
    ($null -eq $pageF.form) -and
    ($capPage -notmatch 'Repair') -and
    ([int]$cntPage.Fields -ge 1)
) "code=$($pageF.code) form=$($null -eq $pageF.form) caption=$capPage fields=$($cntPage.Fields)"
Close-WordDoc $docPage

$mergeOut = Get-OutPath 'docx-field-types-merge.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $mergeOut.Replace('\', '/')
    ops = @(@{ op = 'insertField'; id = $pMerge; code = 'MERGEFIELD'; arg = 'Name' })
} 'docx-field-types-merge' | Out-Null
$treeMerge = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $mergeOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; fields = $true }
} 'docx-field-types-inspect-merge'
$mergeF = @($treeMerge.fields) | Where-Object { $_.host -eq $pMerge } | Select-Object -First 1
$docMerge = Open-WordDoc $word $mergeOut
$capMerge = [string]$docMerge.ActiveWindow.Caption
Add-Check 'docx-Ft2' (
    ([string]$mergeF.code -eq 'MERGEFIELD') -and
    ([string]$mergeF.instr -match 'Name') -and
    ($capMerge -notmatch 'Repair')
) "code=$($mergeF.code) instr=$($mergeF.instr) caption=$capMerge"
Close-WordDoc $docMerge

$formOut = Get-OutPath 'docx-field-types-form.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $formOut.Replace('\', '/')
    ops = @(@{ op = 'insertField'; id = $pForm; code = 'FORMTEXT' })
} 'docx-field-types-form' | Out-Null
$treeForm = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $formOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; fields = $true }
} 'docx-field-types-inspect-form'
$formF = @($treeForm.fields) | Where-Object { $_.host -eq $pForm } | Select-Object -First 1
$docForm = Open-WordDoc $word $formOut
$capForm = [string]$docForm.ActiveWindow.Caption
$cntForm = Get-FieldCounts $docForm
Add-Check 'docx-Ft3' (
    ([string]$formF.form.type -eq 'text') -and
    ([string]$formF.form.name -eq 'Text1') -and
    ($capForm -notmatch 'Repair') -and
    ([int]$cntForm.Forms -ge 1)
) "type=$($formF.form.type) name=$($formF.form.name) caption=$capForm forms=$($cntForm.Forms)"
Close-WordDoc $docForm

$checkOut = Get-OutPath 'docx-field-types-check.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $checkOut.Replace('\', '/')
    ops = @(@{ op = 'insertField'; id = $pCheck; code = 'FORMCHECKBOX'; checked = $true })
} 'docx-field-types-check' | Out-Null
$treeCheck = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $checkOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; fields = $true }
} 'docx-field-types-inspect-check'
$checkF = @($treeCheck.fields) | Where-Object { $_.host -eq $pCheck } | Select-Object -First 1
$docCheck = Open-WordDoc $word $checkOut
$capCheck = [string]$docCheck.ActiveWindow.Caption
Add-Check 'docx-Ft4' (
    ([string]$checkF.form.type -eq 'checkBox') -and
    ([bool]$checkF.form.checked -eq $true) -and
    ($capCheck -notmatch 'Repair')
) "type=$($checkF.form.type) checked=$($checkF.form.checked) caption=$capCheck"
Close-WordDoc $docCheck

$dropOut = Get-OutPath 'docx-field-types-drop.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $dropOut.Replace('\', '/')
    ops = @(@{
        op = 'insertField'; id = $pDrop; code = 'FORMDROPDOWN'
        items = @('A', 'B')
    })
} 'docx-field-types-drop' | Out-Null
$treeDrop = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $dropOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; fields = $true }
} 'docx-field-types-inspect-drop'
$dropF = @($treeDrop.fields) | Where-Object { $_.host -eq $pDrop } | Select-Object -First 1
$dropItems = @($dropF.form.items)
$docDrop = Open-WordDoc $word $dropOut
$capDrop = [string]$docDrop.ActiveWindow.Caption
Add-Check 'docx-Ft5' (
    ([string]$dropF.form.type -eq 'dropDown') -and
    ($dropItems -contains 'A') -and
    ($dropItems -contains 'B') -and
    ($capDrop -notmatch 'Repair')
) "type=$($dropF.form.type) items=$($dropItems -join ',') caption=$capDrop"
Close-WordDoc $docDrop

$created = Get-OutPath 'docx-field-types-create-code.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $created.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = '页数'; fields = , @{ code = 'NUMPAGES' } }
        )
    }
} 'docx-field-types-create-numpages' | Out-Null
$treeCreate = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $created.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; fields = $true }
} 'docx-field-types-inspect-create'
$docCreate = Open-WordDoc $word $created
$capCreate = [string]$docCreate.ActiveWindow.Caption
Add-Check 'docx-Ft6' (
    ([string]$treeCreate.fields[0].code -eq 'NUMPAGES') -and
    ($capCreate -notmatch 'Repair')
) "code=$($treeCreate.fields[0].code) caption=$capCreate"
Close-WordDoc $docCreate

$failBoth = Get-OutPath 'docx-field-types-both.docx'
if (Test-Path $failBoth) { Remove-Item -Force $failBoth }
$badBoth = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failBoth.Replace('\', '/')
    ops = @(@{ op = 'insertField'; id = $pFail; instr = ' PAGE '; code = 'PAGE' })
} 'docx-field-types-both'
$failArg = Get-OutPath 'docx-field-types-arg.docx'
if (Test-Path $failArg) { Remove-Item -Force $failArg }
$badArg = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failArg.Replace('\', '/')
    ops = @(@{ op = 'insertField'; id = $pFail; code = 'REF' })
} 'docx-field-types-ref'
$failInc = Get-OutPath 'docx-field-types-inc.docx'
if (Test-Path $failInc) { Remove-Item -Force $failInc }
$badInc = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failInc.Replace('\', '/')
    ops = @(@{ op = 'insertField'; id = $pFail; code = 'INCLUDETEXT' })
} 'docx-field-types-inc'
Add-Check 'docx-Ft7' (
    ($badBoth.ExitCode -ne 0) -and
    ((Get-FailCode $badBoth) -eq 'InvalidField') -and
    (-not (Test-Path $failBoth)) -and
    ($badArg.ExitCode -ne 0) -and
    ((Get-FailCode $badArg) -eq 'InvalidField') -and
    (-not (Test-Path $failArg)) -and
    ($badInc.ExitCode -ne 0) -and
    ((Get-FailCode $badInc) -eq 'InvalidField') -and
    (-not (Test-Path $failInc))
) "both=$(Get-FailCode $badBoth) arg=$(Get-FailCode $badArg) inc=$(Get-FailCode $badInc)"
