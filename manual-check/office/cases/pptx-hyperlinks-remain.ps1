# pptx 超链接剩余：docs/checklist-pptx.md 的 Phx1–Phx5。
# 用 create 产物再 apply。不另做 PowerPoint 另存样例。

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

function Get-PptErrorCode($raw) {
    try { return ($raw.Stderr | ConvertFrom-Json).error.code } catch { return $null }
}

function Get-PptActionAddress($shape, [int]$which) {
    $settings = $null
    $item = $null
    $hl = $null
    try {
        $settings = $shape.ActionSettings
        $item = $settings.Item($which)
        $hl = $item.Hyperlink
        return [string]$hl.Address
    }
    catch {
        return ''
    }
    finally {
        Release-Com $hl
        Release-Com $item
        Release-Com $settings
    }
}

function Find-PptShapeByType($slide, [int]$type) {
    $shapes = $slide.Shapes
    try {
        $n = [int]$shapes.Count
        for ($i = 1; $i -le $n; $i++) {
            $cand = $shapes.Item($i)
            $hit = $false
            try { $hit = ([int]$cand.Type -eq $type) } catch { $hit = $false }
            if ($hit) { return $cand }
            Release-Com $cand
        }
    }
    finally { Release-Com $shapes }
    return $null
}

function Get-PptTableCellRunAddress($tableShape) {
    $tbl = $null
    $cell = $null
    $cellShape = $null
    $tf = $null
    $tr = $null
    $settings = $null
    $item = $null
    $hl = $null
    try {
        $tbl = $tableShape.Table
        $cell = $tbl.Cell(1, 1)
        $cellShape = $cell.Shape
        $tf = $cellShape.TextFrame
        $tr = $tf.TextRange
        $settings = $tr.ActionSettings
        $item = $settings.Item(1)
        $hl = $item.Hyperlink
        return [string]$hl.Address
    }
    catch {
        return ''
    }
    finally {
        Release-Com $hl
        Release-Com $item
        Release-Com $settings
        Release-Com $tr
        Release-Com $tf
        Release-Com $cellShape
        Release-Com $cell
        Release-Com $tbl
    }
}

$png = Ensure-OoxpPngA
$hostPath = Get-OutPath 'pptx-hyperlinks-remain-host.pptx'
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'create'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    presentation = @{
        slides = @(
            @{ shapes = @(@{ text = 'Phx链' }, @{ text = '第二框' }) }
            @{ shapes = @(@{ text = '第二页' }) }
        )
    }
} 'pptx-phx-create' | Out-Null

$ppt = Ensure-MsPpt $hostPath
Close-PptPresByPath $ppt $hostPath

$treeSh = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hostPath.Replace('\', '/'); format = 'pptx'
    scope = @{ slide = 1 }
} 'pptx-phx-shapes'
$shapeId = [string]$treeSh.slides[0].shapes[0].id

$hoverOut = Get-OutPath 'pptx-hyperlinks-remain-hover.pptx'
Close-PptPresByPath $ppt $hoverOut
if (Test-Path $hoverOut) { Remove-Item -Force $hoverOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $hoverOut.Replace('\', '/')
    ops = @(@{
        op = 'insertHyperlink'
        id = $shapeId
        event = 'hover'
        url = 'https://example.com/phx-hover'
    })
} 'pptx-phx-hover' | Out-Null

$treeH = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $hoverOut.Replace('\', '/'); format = 'pptx'
    scope = @{ hyperlinks = $true }
} 'pptx-phx-hover-inspect'
$h0 = @($treeH.slides[0].hyperlinks | Where-Object { $_ }) | Select-Object -First 1
$xmlH = Get-ZipPartText $hoverOut 'ppt/slides/slide1.xml'
$presH = Open-PptPres $ppt $hoverOut
$capH = Get-PptCaption $presH
$slideH = Get-PptSlide $presH 1
$shapeH = Get-PptShape $slideH 1
$addrH = Get-PptActionAddress $shapeH 2
Release-Com $shapeH
Release-Com $slideH
Close-PptPres $presH
$phx1ok = ([string]$h0.event -eq 'hover') `
    -and ($xmlH -match 'hlinkHover') `
    -and ($addrH -match 'example.com/phx-hover') `
    -and ($capH -notmatch 'Repair')
Add-Check 'pptx-Phx1' $phx1ok "event=$($h0.event) addr=$addrH caption=$capH"

$objHost = Get-OutPath 'pptx-hyperlinks-remain-objs.pptx'
Close-PptPresByPath $ppt $objHost
if (Test-Path $objHost) { Remove-Item -Force $objHost }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $objHost.Replace('\', '/')
    ops = @(
        @{ op = 'insertPicture'; id = 'slide:1'; file = $png.Replace('\', '/') }
        @{ op = 'insertTable'; id = 'slide:1'; rows = 2; cols = 2; cells = @(@('甲', '乙'), @('丙', '丁')) }
        @{
            op = 'insertChart'; id = 'slide:1'; type = 'col'
            series = @(@{ name = 's'; categories = @('A', 'B'); values = @(1, 2) })
        }
        @{ op = 'insertSmartArt'; id = 'slide:1'; layout = 'list'; points = @(@{ text = '甲' }, @{ text = '乙' }) }
    )
} 'pptx-phx-insert-objs' | Out-Null

$treeObj = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $objHost.Replace('\', '/'); format = 'pptx'
    scope = @{ pictures = $true; tables = $true; charts = $true; smartArt = $true }
} 'pptx-phx-objs-inspect'
$picId = [string]$treeObj.slides[0].pictures[0].id
$tableId = [string]$treeObj.slides[0].tables[0].id
$cellId = [string]$treeObj.slides[0].tables[0].rows[0].cells[0].id
$chartId = [string]$treeObj.slides[0].charts[0].id
$saId = [string]$treeObj.slides[0].smartArts[0].id

$linked = Get-OutPath 'pptx-hyperlinks-remain-linked.pptx'
Close-PptPresByPath $ppt $linked
if (Test-Path $linked) { Remove-Item -Force $linked }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $objHost.Replace('\', '/')
    output = $linked.Replace('\', '/')
    ops = @(
        @{ op = 'insertHyperlink'; id = $picId; url = 'https://example.com/phx-pic' }
        @{ op = 'insertHyperlink'; id = $tableId; url = 'https://example.com/phx-table' }
        @{ op = 'insertHyperlink'; id = $cellId; on = 'cell'; url = 'https://example.com/phx-cell' }
        @{ op = 'insertHyperlink'; id = $chartId; url = 'https://example.com/phx-chart' }
        @{ op = 'insertHyperlink'; id = $saId; url = 'https://example.com/phx-sa' }
    )
} 'pptx-phx-insert-links' | Out-Null

$treeL = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $linked.Replace('\', '/'); format = 'pptx'
    scope = @{ hyperlinks = $true }
} 'pptx-phx-links-inspect'
$links = @($treeL.slides[0].hyperlinks | Where-Object { $_ })
function Find-LinkOn($on) {
    return @($links | Where-Object { $_.on -eq $on } | Select-Object -First 1)
}

$presL = Open-PptPres $ppt $linked
$capL = Get-PptCaption $presL
$slideL = Get-PptSlide $presL 1
$picSh = Find-PptShapeByType $slideL 13
$tblSh = Find-PptShapeByType $slideL 19
$chartSh = Find-PptShapeByType $slideL 3
$saSh = Find-PptShapeByType $slideL 24
$addrPic = Get-PptActionAddress $picSh 1
$addrTbl = Get-PptActionAddress $tblSh 1
$addrCell = Get-PptTableCellRunAddress $tblSh
$addrChart = Get-PptActionAddress $chartSh 1
$addrSa = Get-PptActionAddress $saSh 1
Release-Com $saSh
Release-Com $chartSh
Release-Com $tblSh
Release-Com $picSh
Release-Com $slideL
Close-PptPres $presL

$picLink = Find-LinkOn 'pic'
$phx2ok = ([string]$picLink.on -eq 'pic') `
    -and ($addrPic -match 'example.com/phx-pic') `
    -and ($capL -notmatch 'Repair')
Add-Check 'pptx-Phx2' $phx2ok "on=$($picLink.on) addr=$addrPic caption=$capL"

$tableLink = Find-LinkOn 'table'
$cellLink = Find-LinkOn 'cell'
$phx3ok = ([string]$tableLink.on -eq 'table') `
    -and ([string]$cellLink.on -eq 'cell') `
    -and ($addrTbl -match 'example.com/phx-table') `
    -and ($addrCell -match 'example.com/phx-cell') `
    -and ($capL -notmatch 'Repair')
Add-Check 'pptx-Phx3' $phx3ok "table=$($tableLink.on) cell=$($cellLink.on) tblAddr=$addrTbl cellAddr=$addrCell"

$chartLink = Find-LinkOn 'chart'
$saLink = Find-LinkOn 'smartArt'
$phx4ok = ([string]$chartLink.on -eq 'chart') `
    -and ([string]$saLink.on -eq 'smartArt') `
    -and ($saId -like '*smartdiagram:*') `
    -and ($addrChart -match 'example.com/phx-chart') `
    -and ($addrSa -match 'example.com/phx-sa') `
    -and ($capL -notmatch 'Repair')
Add-Check 'pptx-Phx4' $phx4ok "chart=$($chartLink.on) sa=$($saLink.on) saId=$saId chartAddr=$addrChart saAddr=$addrSa caption=$capL"

$badEvOut = Get-OutPath 'pptx-hyperlinks-remain-bad-event.pptx'
if (Test-Path $badEvOut) { Remove-Item -Force $badEvOut }
$failEv = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badEvOut.Replace('\', '/')
    ops = @(@{ op = 'insertHyperlink'; id = $shapeId; event = 'foo'; url = 'https://example.com/' })
} 'pptx-phx-bad-event'
$codeEv = Get-PptErrorCode $failEv

$badOnOut = Get-OutPath 'pptx-hyperlinks-remain-pic-run.pptx'
if (Test-Path $badOnOut) { Remove-Item -Force $badOnOut }
$failOn = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $objHost.Replace('\', '/')
    output = $badOnOut.Replace('\', '/')
    ops = @(@{ op = 'insertHyperlink'; id = $picId; on = 'run'; url = 'https://example.com/' })
} 'pptx-phx-pic-run'
$codeOn = Get-PptErrorCode $failOn

$badIdOut = Get-OutPath 'pptx-hyperlinks-remain-bad-id.pptx'
if (Test-Path $badIdOut) { Remove-Item -Force $badIdOut }
$failId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $badIdOut.Replace('\', '/')
    ops = @(@{ op = 'insertHyperlink'; id = 'slide:1'; url = 'https://example.com/' })
} 'pptx-phx-bad-id'
$codeId = Get-PptErrorCode $failId

$phx5ok = ($codeEv -eq 'InvalidField') -and (-not (Test-Path $badEvOut)) `
    -and ($codeOn -eq 'InvalidField') -and (-not (Test-Path $badOnOut)) `
    -and ($codeId -eq 'InvalidId') -and (-not (Test-Path $badIdOut))
Add-Check 'pptx-Phx5' $phx5ok "event=$codeEv on=$codeOn id=$codeId wroteEv=$(Test-Path $badEvOut) wroteOn=$(Test-Path $badOnOut) wroteId=$(Test-Path $badIdOut)"
