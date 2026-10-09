# docx 内容控件类型：docs/checklist-docx.md 的 Ct1–Ct8。
# 用 ooxp 写入 checkbox / picture / group / equation，再用 Word 核对无修复。

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

function Get-SdtInfo($doc) {
    $ccs = $null
    try {
        $ccs = $doc.ContentControls
        $n = [int]$ccs.Count
        if ($n -lt 1) {
            return @{ Count = 0; Type = -1 }
        }
        $cc = $null
        try {
            $cc = $ccs.Item(1)
            return @{ Count = $n; Type = [int]$cc.Type }
        } finally { Release-Com $cc }
    } finally { Release-Com $ccs }
}

$pngA = Ensure-OoxpPngA
$pngB = Ensure-OoxpPngB

$src = Get-OutPath 'docx-sdt-types-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ text = '复选' }
            @{ text = '图片' }
            @{ text = '组合' }
            @{ text = '公式' }
        )
    }
} 'docx-sdt-types-create' | Out-Null

$srcTree = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20 }
} 'docx-sdt-types-inspect-src'
$pCheck = [string](@($srcTree.paragraphs) | Where-Object { [string]$_.text -eq '复选' } | Select-Object -First 1).id
$pPic = [string](@($srcTree.paragraphs) | Where-Object { [string]$_.text -eq '图片' } | Select-Object -First 1).id
$pGroup = [string](@($srcTree.paragraphs) | Where-Object { [string]$_.text -eq '组合' } | Select-Object -First 1).id
$pEq = [string](@($srcTree.paragraphs) | Where-Object { [string]$_.text -eq '公式' } | Select-Object -First 1).id

$word = Ensure-MsWord $src

$cbOut = Get-OutPath 'docx-sdt-types-checkbox.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $cbOut.Replace('\', '/')
    ops = @(, @{
        op = 'insertContentControl'
        id = $pCheck
        kind = 'checkbox'
        checked = $true
    })
} 'docx-sdt-types-checkbox' | Out-Null
$treeCb = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cbOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; contentControls = $true }
} 'docx-sdt-types-inspect-checkbox'
$cb = @($treeCb.contentControls) | Select-Object -First 1
$docCb = Open-WordDoc $word $cbOut
$capCb = [string]$docCb.ActiveWindow.Caption
$ccCb = Get-SdtInfo $docCb
Add-Check 'docx-Ct1' (
    ([string]$cb.kind -eq 'checkbox') -and
    ([bool]$cb.checked -eq $true) -and
    ($capCb -notmatch 'Repair') -and
    ([int]$ccCb.Count -ge 1) -and
    ([int]$ccCb.Type -eq 8)
) "kind=$($cb.kind) checked=$($cb.checked) caption=$capCb count=$($ccCb.Count) type=$($ccCb.Type)"
Close-WordDoc $docCb

$picOut = Get-OutPath 'docx-sdt-types-picture.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $picOut.Replace('\', '/')
    ops = @(, @{
        op = 'insertContentControl'
        id = $pPic
        kind = 'picture'
        file = $pngA.Replace('\', '/')
    })
} 'docx-sdt-types-picture' | Out-Null
$treePic = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $picOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; contentControls = $true }
} 'docx-sdt-types-inspect-picture'
$pic = @($treePic.contentControls) | Select-Object -First 1
$docPic = Open-WordDoc $word $picOut
$capPic = [string]$docPic.ActiveWindow.Caption
$ccPic = Get-SdtInfo $docPic
Add-Check 'docx-Ct2' (
    ([string]$pic.kind -eq 'picture') -and
    ($capPic -notmatch 'Repair') -and
    ([int]$ccPic.Count -ge 1) -and
    ([int]$ccPic.Type -eq 2)
) "kind=$($pic.kind) caption=$capPic count=$($ccPic.Count) type=$($ccPic.Type)"
Close-WordDoc $docPic

$grpOut = Get-OutPath 'docx-sdt-types-group.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $grpOut.Replace('\', '/')
    ops = @(, @{
        op = 'insertContentControl'
        id = $pGroup
        kind = 'group'
        text = '组'
    })
} 'docx-sdt-types-group' | Out-Null
$treeGrp = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $grpOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; contentControls = $true }
} 'docx-sdt-types-inspect-group'
$grp = @($treeGrp.contentControls) | Select-Object -First 1
$docGrp = Open-WordDoc $word $grpOut
$capGrp = [string]$docGrp.ActiveWindow.Caption
$ccGrp = Get-SdtInfo $docGrp
Add-Check 'docx-Ct3' (
    ([string]$grp.kind -eq 'group') -and
    ($capGrp -notmatch 'Repair') -and
    ([int]$ccGrp.Count -ge 1) -and
    ([int]$ccGrp.Type -eq 7)
) "kind=$($grp.kind) caption=$capGrp count=$($ccGrp.Count) type=$($ccGrp.Type)"
Close-WordDoc $docGrp

$eqOut = Get-OutPath 'docx-sdt-types-equation.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $eqOut.Replace('\', '/')
    ops = @(, @{
        op = 'insertContentControl'
        id = $pEq
        kind = 'equation'
        text = 'x'
    })
} 'docx-sdt-types-equation' | Out-Null
$treeEq = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $eqOut.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; contentControls = $true }
} 'docx-sdt-types-inspect-equation'
$eq = @($treeEq.contentControls) | Select-Object -First 1
$eqXml = Get-ZipPartText $eqOut 'word/document.xml'
$docEq = Open-WordDoc $word $eqOut
$capEq = [string]$docEq.ActiveWindow.Caption
Add-Check 'docx-Ct4' (
    ([string]$eq.kind -eq 'equation') -and
    ($eqXml -match '<w:equation') -and
    ($capEq -notmatch 'Repair')
) "kind=$($eq.kind) hasEquation=$($eqXml -match '<w:equation') caption=$capEq"
Close-WordDoc $docEq

$setCb = Get-OutPath 'docx-sdt-types-set-checkbox.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $cbOut.Replace('\', '/')
    output = $setCb.Replace('\', '/')
    ops = @(, @{ op = 'setContentControl'; id = 'sdt:1'; checked = $false })
} 'docx-sdt-types-set-checkbox' | Out-Null
$treeSetCb = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $setCb.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 20; contentControls = $true }
} 'docx-sdt-types-inspect-set-checkbox'
$docSetCb = Open-WordDoc $word $setCb
$capSetCb = [string]$docSetCb.ActiveWindow.Caption
$ccSetCb = Get-SdtInfo $docSetCb
Add-Check 'docx-Ct5' (
    ([bool]$treeSetCb.contentControls[0].checked -eq $false) -and
    ($capSetCb -notmatch 'Repair') -and
    ([int]$ccSetCb.Type -eq 8)
) "checked=$($treeSetCb.contentControls[0].checked) caption=$capSetCb type=$($ccSetCb.Type)"
Close-WordDoc $docSetCb

$setPic = Get-OutPath 'docx-sdt-types-set-picture.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $picOut.Replace('\', '/')
    output = $setPic.Replace('\', '/')
    ops = @(, @{
        op = 'setContentControl'
        id = 'sdt:1'
        file = $pngB.Replace('\', '/')
    })
} 'docx-sdt-types-set-picture' | Out-Null
$docSetPic = Open-WordDoc $word $setPic
$capSetPic = [string]$docSetPic.ActiveWindow.Caption
$ccSetPic = Get-SdtInfo $docSetPic
Add-Check 'docx-Ct6' (
    ($capSetPic -notmatch 'Repair') -and
    ([int]$ccSetPic.Count -ge 1) -and
    ([int]$ccSetPic.Type -eq 2)
) "caption=$capSetPic count=$($ccSetPic.Count) type=$($ccSetPic.Type)"
Close-WordDoc $docSetPic

$failGrp = Get-OutPath 'docx-sdt-types-set-group.docx'
if (Test-Path $failGrp) { Remove-Item -Force $failGrp }
$badGrp = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $grpOut.Replace('\', '/')
    output = $failGrp.Replace('\', '/')
    ops = @(, @{ op = 'setContentControl'; id = 'sdt:1'; text = '改' })
} 'docx-sdt-types-set-group'
Add-Check 'docx-Ct7' (
    ($badGrp.ExitCode -ne 0) -and
    ((Get-FailCode $badGrp) -eq 'NotImplemented') -and
    (-not (Test-Path $failGrp))
) "code=$(Get-FailCode $badGrp) exists=$(Test-Path $failGrp)"

$failPic = Get-OutPath 'docx-sdt-types-no-file.docx'
if (Test-Path $failPic) { Remove-Item -Force $failPic }
$badPic = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failPic.Replace('\', '/')
    ops = @(, @{ op = 'insertContentControl'; id = $pPic; kind = 'picture' })
} 'docx-sdt-types-no-file'
Add-Check 'docx-Ct8' (
    ($badPic.ExitCode -ne 0) -and
    ((Get-FailCode $badPic) -eq 'InvalidField') -and
    (-not (Test-Path $failPic))
) "code=$(Get-FailCode $badPic) exists=$(Test-Path $failPic)"
