# docx 组合与连接线：docs/checklist-docx.md 的 Dg1–Dg8。
# 用 create 产物再 apply。编号 Dg，不要和 T、Tb、Rv、Rm、Pb、Ph 撞。
# 不实现放映引擎；组合本身不进 inspect 树。

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

function Get-FailCode($raw) {
    $code = $null
    try { $code = ($raw.Stderr | ConvertFrom-Json).error.code } catch { $code = $null }
    return $code
}

function Get-WordShapeStats($doc) {
    $shapes = $null
    $groupN = 0
    $cxnN = 0
    $boxN = 0
    $picN = 0
    $n = 0
    try {
        $shapes = $doc.Shapes
        try { $n = [int]$shapes.Count } catch { $n = 0 }
        for ($i = 1; $i -le $n; $i++) {
            $sh = $null
            try {
                $sh = $shapes.Item($i)
                $ty = 0
                try { $ty = [int]$sh.Type } catch { $ty = 0 }
                $isCxn = $false
                try { $isCxn = [bool]$sh.Connector } catch { $isCxn = $false }
                if ($ty -eq 6) { $groupN++ }
                elseif ($isCxn -or ($ty -eq 9)) { $cxnN++ }
                elseif ($ty -eq 17) { $boxN++ }
                elseif ($ty -eq 13) { $picN++ }
            }
            finally { Release-Com $sh }
        }
    }
    finally { Release-Com $shapes }
    return @{ Count = $n; Group = $groupN; Cxn = $cxnN; Box = $boxN; Pic = $picN }
}

$S = $global:OoxpStrings
$png = Ensure-OoxpPngA

$src = Get-OutPath 'docx-groups-src.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $src.Replace('\', '/')
    format = 'docx'
    document = @{
        paragraphs = @(
            @{ style = 'Heading1'; text = $S.chapter1 }
            @{ text = $S.hello }
        )
    }
} 'docx-dg-create' | Out-Null

$bare = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
} 'docx-dg-inspect-bare'
$bareNoParas = -not ($bare.PSObject.Properties.Name -contains 'paragraphs')
$bareNoList = -not ($bare.PSObject.Properties.Name -contains 'connectors')
Add-Check 'docx-Dg1' (
    ($null -ne $bare.connectorCount) -and ([int]$bare.connectorCount -eq 0) -and $bareNoParas -and $bareNoList
) "connectorCount=$($bare.connectorCount) paragraphs=$bareNoParas list=$bareNoList"

$sliced = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 10 }
} 'docx-dg-inspect-paras'
$p1 = [string]$sliced.paragraphs[0].id
$p2 = [string]$sliced.paragraphs[1].id

$cxnPath = Get-OutPath 'docx-groups-cxn.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $cxnPath.Replace('\', '/')
    ops = @(@{ op = 'insertConnector'; id = $p2 })
} 'docx-dg-insert-cxn' | Out-Null
$treeCxn = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cxnPath.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 10; connectors = $true }
} 'docx-dg-inspect-cxn'
$cxn = $treeCxn.paragraphs[1].connectors[0]
$cxnId = [string]$cxn.id
$cxnXml = Get-ZipPartText $cxnPath 'word/document.xml'
$word = Ensure-MsWord $src
$docCxn = Open-WordDoc $word $cxnPath
$capCxn = [string]$docCxn.ActiveWindow.Caption
$stCxn = Get-WordShapeStats $docCxn
Close-WordDoc $docCxn
Add-Check 'docx-Dg2' (
    ([int]$treeCxn.connectorCount -eq 1) -and
    ([string]$cxn.prst -eq 'straightConnector1') -and
    ($cxnId -match '/cxn:') -and
    ($cxnXml -match 'straightConnector1') -and
    ($capCxn -notmatch 'Repair') -and
    ($stCxn.Cxn -ge 1)
) "count=$($treeCxn.connectorCount) prst=$($cxn.prst) typeCxn=$($stCxn.Cxn) caption=$capCxn"

$delPath = Get-OutPath 'docx-groups-cxn-del.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $cxnPath.Replace('\', '/')
    output = $delPath.Replace('\', '/')
    ops = @(@{ op = 'deleteConnector'; id = $cxnId })
} 'docx-dg-delete-cxn' | Out-Null
$treeDel = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delPath.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 10; connectors = $true }
} 'docx-dg-inspect-cxn-del'
Add-Check 'docx-Dg3' (
    ([int]$treeDel.connectorCount -eq 0) -and ([int]$treeDel.paragraphs[1].connectorCount -eq 0)
) "count=$($treeDel.connectorCount)"

$boxPath = Get-OutPath 'docx-groups-boxes.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $boxPath.Replace('\', '/')
    ops = @(
        @{ op = 'insertTextBox'; id = $p1; text = '框甲' }
        @{ op = 'insertTextBox'; id = $p2; text = '框乙' }
    )
} 'docx-dg-insert-boxes' | Out-Null
$treeBoxes = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $boxPath.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 10; textBoxes = $true }
} 'docx-dg-inspect-boxes'
$boxA = [string]$treeBoxes.paragraphs[0].textBoxes[0].id
$boxB = [string]$treeBoxes.paragraphs[1].textBoxes[0].id
$grpBox = Get-OutPath 'docx-groups-boxes-group.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $boxPath.Replace('\', '/')
    output = $grpBox.Replace('\', '/')
    ops = @(@{
        op = 'groupShapes'; id = 'docx:document'; ids = @($boxA, $boxB)
    })
} 'docx-dg-group-boxes' | Out-Null
$treeGrp = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $grpBox.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 10; textBoxes = $true }
} 'docx-dg-inspect-group-boxes'
$grpXml = Get-ZipPartText $grpBox 'word/document.xml'
$docGrp = Open-WordDoc $word $grpBox
$capGrp = [string]$docGrp.ActiveWindow.Caption
$stGrp = Get-WordShapeStats $docGrp
Close-WordDoc $docGrp
$memberId = [string]$treeGrp.paragraphs[0].textBoxes[0].id
Add-Check 'docx-Dg4' (
    ([int]$treeGrp.textBoxCount -eq 2) -and
    ([int]$treeGrp.paragraphs[0].textBoxCount -eq 2) -and
    ($grpXml -match 'wpg:wgp') -and
    ($capGrp -notmatch 'Repair') -and
    ($stGrp.Group -ge 1)
) "boxes=$($treeGrp.textBoxCount) group=$($stGrp.Group) caption=$capGrp"

$ung = Get-OutPath 'docx-groups-boxes-ungroup.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $grpBox.Replace('\', '/')
    output = $ung.Replace('\', '/')
    ops = @(@{ op = 'ungroup'; id = $memberId })
} 'docx-dg-ungroup-boxes' | Out-Null
$treeUng = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $ung.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 10; textBoxes = $true }
} 'docx-dg-inspect-ungroup'
$ungXml = Get-ZipPartText $ung 'word/document.xml'
$docUng = Open-WordDoc $word $ung
$capUng = [string]$docUng.ActiveWindow.Caption
$stUng = Get-WordShapeStats $docUng
Close-WordDoc $docUng
Add-Check 'docx-Dg5' (
    ([int]$treeUng.textBoxCount -eq 2) -and
    ($ungXml -notmatch 'wpg:wgp') -and
    ($capUng -notmatch 'Repair') -and
    ($stUng.Group -eq 0) -and
    ($stUng.Box -ge 2)
) "boxes=$($treeUng.textBoxCount) group=$($stUng.Group) box=$($stUng.Box) caption=$capUng"

$picPath = Get-OutPath 'docx-groups-pics.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $picPath.Replace('\', '/')
    ops = @(
        @{ op = 'insertPicture'; id = $p2; file = $png.Replace('\', '/') }
        @{ op = 'insertPicture'; id = $p2; file = $png.Replace('\', '/') }
    )
} 'docx-dg-insert-pics' | Out-Null
$treePics = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $picPath.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 10; pictures = $true }
} 'docx-dg-inspect-pics'
$picA = [string]$treePics.paragraphs[1].pictures[0].id
$picB = [string]$treePics.paragraphs[1].pictures[1].id
$floatPath = Get-OutPath 'docx-groups-pics-float.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $picPath.Replace('\', '/')
    output = $floatPath.Replace('\', '/')
    ops = @(
        @{ op = 'setPictureWrap'; id = $picA; kind = 'anchor'; wrap = 'square' }
        @{ op = 'setPictureWrap'; id = $picB; kind = 'anchor'; wrap = 'square' }
    )
} 'docx-dg-float-pics' | Out-Null
$grpPic = Get-OutPath 'docx-groups-pics-group.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $floatPath.Replace('\', '/')
    output = $grpPic.Replace('\', '/')
    ops = @(@{
        op = 'groupShapes'; id = 'docx:document'; ids = @($picA, $picB)
    })
} 'docx-dg-group-pics' | Out-Null
$treeGrpPic = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $grpPic.Replace('\', '/'); format = 'docx'
    scope = @{ from = 1; maxParas = 10; pictures = $true }
} 'docx-dg-inspect-group-pics'
$picXml = Get-ZipPartText $grpPic 'word/document.xml'
$docPic = Open-WordDoc $word $grpPic
$capPic = [string]$docPic.ActiveWindow.Caption
$stPic = Get-WordShapeStats $docPic
Close-WordDoc $docPic
Add-Check 'docx-Dg6' (
    ([int]$treeGrpPic.pictureCount -eq 2) -and
    ([int]$treeGrpPic.paragraphs[1].pictureCount -eq 2) -and
    ($picXml -match 'wpg:wgp') -and
    ($capPic -notmatch 'Repair') -and
    ($stPic.Group -ge 1)
) "pics=$($treeGrpPic.pictureCount) group=$($stPic.Group) caption=$capPic"

$hfHost = Get-OutPath 'docx-groups-hf.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $hfHost.Replace('\', '/')
    ops = @(@{
        op = 'insertHeaderFooter'; id = 'section:1'; kind = 'header'; type = 'default'
    })
} 'docx-dg-insert-hf' | Out-Null
$treeHf = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfHost.Replace('\', '/'); format = 'docx'
    scope = @{ headerFooter = $true }
} 'docx-dg-inspect-hf'
$hfPara = [string]$treeHf.headers[0].paragraphs[0].id
$hfCxn = Get-OutPath 'docx-groups-hf-cxn.docx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hfHost.Replace('\', '/')
    output = $hfCxn.Replace('\', '/')
    ops = @(@{ op = 'insertConnector'; id = $hfPara })
} 'docx-dg-insert-hf-cxn' | Out-Null
$treeHfCxn = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hfCxn.Replace('\', '/'); format = 'docx'
    scope = @{ connectors = $true }
} 'docx-dg-inspect-hf-cxn'
$docHf = Open-WordDoc $word $hfCxn
$capHf = [string]$docHf.ActiveWindow.Caption
Close-WordDoc $docHf
Add-Check 'docx-Dg7' (
    ([int]$treeHfCxn.connectorCount -eq 0) -and
    ([int]$treeHfCxn.headers[0].connectorCount -eq 1) -and
    ($capHf -notmatch 'Repair')
) "body=$($treeHfCxn.connectorCount) header=$($treeHfCxn.headers[0].connectorCount) caption=$capHf"

$failInline = Get-OutPath 'docx-groups-fail-inline.docx'
$rawInline = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $picPath.Replace('\', '/')
    output = $failInline.Replace('\', '/')
    ops = @(@{
        op = 'groupShapes'; id = 'docx:document'; ids = @($picA, $picB)
    })
} 'docx-dg-fail-inline'
$codeInline = Get-FailCode $rawInline

$failId = Get-OutPath 'docx-groups-fail-id.docx'
$rawId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failId.Replace('\', '/')
    ops = @(@{ op = 'deleteConnector'; id = "$p2/cxn:9" })
} 'docx-dg-fail-id'
$codeId = Get-FailCode $rawId

$failShape = Get-OutPath 'docx-groups-fail-shape.docx'
$rawShape = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $failShape.Replace('\', '/')
    ops = @(@{ op = 'setShapeGeom'; id = $p2 })
} 'docx-dg-fail-shape'
$codeShape = Get-FailCode $rawShape

$xlsx = Get-OutPath 'docx-groups-xlsx.xlsx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $xlsx.Replace('\', '/')
    workbook = @{ sheets = @(@{ name = 'Sheet1' }) }
} 'docx-dg-xlsx-create' | Out-Null
$failX = Get-OutPath 'docx-groups-fail-xlsx.xlsx'
$rawX = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $xlsx.Replace('\', '/')
    output = $failX.Replace('\', '/')
    ops = @(@{ op = 'insertConnector'; id = $p2 })
} 'docx-dg-fail-xlsx'
$codeX = Get-FailCode $rawX

Add-Check 'docx-Dg8' (
    ($rawInline.ExitCode -ne 0) -and ($codeInline -eq 'InvalidField') -and (-not (Test-Path $failInline)) -and
    ($rawId.ExitCode -ne 0) -and ($codeId -eq 'IdNotFound') -and (-not (Test-Path $failId)) -and
    ($rawShape.ExitCode -ne 0) -and ($codeShape -eq 'InvalidField') -and (-not (Test-Path $failShape)) -and
    ($rawX.ExitCode -ne 0) -and ($codeX -eq 'InvalidId') -and (-not (Test-Path $failX))
) "inline=$codeInline id=$codeId shape=$codeShape xlsx=$codeX"
