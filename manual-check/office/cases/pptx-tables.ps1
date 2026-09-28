# pptx 幻灯片表格：docs/checklist-pptx.md 的 Pt1–Pt6。
# 源文件：PowerPoint 亲手另存，第一页至少一张表。不要用库 create 产物当带表的唯一样例。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings

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

function Test-PptShapeIsTable($sh) {
    try { if ([bool]$sh.HasTable) { return $true } } catch {}
    try { if ([int]$sh.Type -eq 19) { return $true } } catch {}
    return $false
}

function Get-PptTableCount($slide) {
    $n = 0
    $shapes = $slide.Shapes
    try {
        for ($i = 1; $i -le $shapes.Count; $i++) {
            $sh = $shapes.Item($i)
            try { if (Test-PptShapeIsTable $sh) { $n++ } }
            finally { Release-Com $sh }
        }
    }
    finally { Release-Com $shapes }
    return $n
}

function Find-PptTable($slide) {
    $shapes = $slide.Shapes
    try {
        for ($i = 1; $i -le $shapes.Count; $i++) {
            $cand = $shapes.Item($i)
            if (Test-PptShapeIsTable $cand) { return $cand }
            Release-Com $cand
        }
    }
    finally { Release-Com $shapes }
    return $null
}

function Get-PptTableCellText($table, [int]$row, [int]$col) {
    $cell = $table.Cell($row, $col)
    try {
        $shape = $cell.Shape
        try {
            $text = ''
            try {
                $tf = $shape.TextFrame
                try {
                    $tr = $tf.TextRange
                    try { $text = [string]$tr.Text }
                    finally { Release-Com $tr }
                }
                finally { Release-Com $tf }
            }
            catch {}
            if ([string]::IsNullOrWhiteSpace($text)) {
                try {
                    $tf2 = $shape.TextFrame2
                    try {
                        $tr2 = $tf2.TextRange
                        try { $text = [string]$tr2.Text }
                        finally { Release-Com $tr2 }
                    }
                    finally { Release-Com $tf2 }
                }
                catch {}
            }
            return $text.TrimEnd([char]13, [char]10, [char]32)
        }
        finally { Release-Com $shape }
    }
    finally { Release-Com $cell }
}

function Set-PptTableCellText($table, [int]$row, [int]$col, [string]$text) {
    $cell = $table.Cell($row, $col)
    try {
        $shape = $cell.Shape
        try {
            $tf = $shape.TextFrame
            try {
                $tr = $tf.TextRange
                try { $tr.Text = $text }
                finally { Release-Com $tr }
            }
            finally { Release-Com $tf }
        }
        finally { Release-Com $shape }
    }
    finally { Release-Com $cell }
}

function Test-PptxTableSource([string]$pptx, [string]$reqName) {
    $tree = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $pptx.Replace('\', '/'); format = 'pptx'
        scope = @{ tables = $true }
    } $reqName
    $tables = @()
    if ($tree.slides[0].PSObject.Properties.Name -contains 'tables') {
        $tables = @($tree.slides[0].tables)
    }
    $id = ''
    $text = ''
    if ($tables.Count -ge 1) {
        $id = [string]$tables[0].id
        $text = [string]$tables[0].rows[0].cells[0].text
    }
    return [pscustomobject]@{
        tableCount = [int]$tree.tableCount
        n = $tables.Count
        id = $id
        text = $text
        ok = ($tree.tableCount -ge 1) -and ($id -like 'slide:*/table:*') -and ($text.Length -ge 1)
    }
}

function Ensure-PptxPptTableSource {
    $path = Get-OutPath 'pptx-ppt-table-source.pptx'
    if (Test-Path $path) {
        $cached = Test-PptxTableSource $path 'pptx-table-source-cache-check'
        if ($cached.ok) { return $path }
        Write-Log "缓存的 pptx table source 不合格（count=$($cached.tableCount) id=$($cached.id) text=$($cached.text)），重建"
    }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $tmp = Join-Path $tmpDir 'pptx-ppt-table-source.pptx'
    $hostPath = Ensure-PptxCreate
    Copy-Item -Force $hostPath $tmp

    $ppt = Ensure-MsPpt $hostPath
    Close-PptPresByPath $ppt $tmp
    $pres = Open-PptPres $ppt $tmp
    $s1 = Get-PptSlide $pres 1
    $shapes = $s1.Shapes
    $hostObj = $null
    try {
        $hostObj = $shapes.AddTable(2, 2, 72, 144, 432, 252)
    }
    catch {
        Write-Log ("AddTable 报错：" + $_.Exception.Message)
    }
    finally { Release-Com $shapes }

    if (-not $hostObj) { throw 'PowerPoint 没有插入幻灯片表格' }
    $tbl = $null
    try { $tbl = $hostObj.Table } catch {
        Write-Log ("取 Table 失败：" + $_.Exception.Message)
    }
    if (-not $tbl) {
        Release-Com $hostObj
        Release-Com $s1
        Close-PptPres $pres
        throw 'PowerPoint 插入的形状没有 Table'
    }
    Set-PptTableCellText $tbl 1 1 $S.saItem1
    Set-PptTableCellText $tbl 1 2 $S.saItem2
    Set-PptTableCellText $tbl 2 1 $S.saItem3
    Set-PptTableCellText $tbl 2 2 $S.saPointNew
    Write-Log 'PowerPoint Save() 原地覆盖 TEMP 里的带表源文件'
    $pres.Save()
    Release-Com $tbl
    Release-Com $hostObj
    Release-Com $s1
    Close-PptPres $pres
    Copy-Item -Force $tmp $path
    Write-Log "saved ppt table source $path"

    $check = Test-PptxTableSource $path 'pptx-table-source-saved-inspect'
    if (-not $check.ok) {
        throw "PowerPoint 另存的表格源不合格：count=$($check.tableCount) id=$($check.id) text=$($check.text)"
    }
    return $path
}

$hostPath = Ensure-PptxCreate
$src = Ensure-PptxPptTableSource
$ppt = Ensure-MsPpt $hostPath
Close-PptPresByPath $ppt $src

# Pt1：无 tables 时只有计数；tables:true 时 id 为 slide:…/table:…，能看见单元格 text。
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
} 'pptx-pt-inspect-list'
$slide0 = $tree0.slides[0]
$noList = -not ($slide0.PSObject.Properties.Name -contains 'tables')
Add-Check 'pptx-Pt1a' (($tree0.tableCount -ge 1) -and ($slide0.tableCount -ge 1) -and $noList) "wb=$($tree0.tableCount) slide=$($slide0.tableCount) noList=$noList"

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-pt-inspect-expand'
$tblNode = @($tree1.slides[0].tables) | Select-Object -First 1
$cell0 = $tblNode.rows[0].cells[0]
$pt1ok = ($null -ne $tblNode) `
    -and ([string]$tblNode.id -like 'slide:*/table:*') `
    -and ([string]$cell0.text -eq $S.saItem1)
Add-Check 'pptx-Pt1' $pt1ok "id=$($tblNode.id) text=$($cell0.text)"

# Pt2：另存样例改一格。
$cellOut = Get-OutPath 'pptx-tables-text.pptx'
Close-PptPresByPath $ppt $cellOut
if (Test-Path $cellOut) { Remove-Item -Force $cellOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $cellOut.Replace('\', '/')
    ops = @(@{
        op = 'setTableCellText'
        id = [string]$cell0.id
        text = $S.para2new
    })
} 'pptx-pt-text' | Out-Null
$treeT = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $cellOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-pt-text-inspect'
$tNode = @($treeT.slides[0].tables) | Select-Object -First 1
$keep = [string]$tNode.rows[0].cells[1].text
$presT = Open-PptPres $ppt $cellOut
$capT = Get-PptCaption $presT
$sT = Get-PptSlide $presT 1
$boxT = Find-PptTable $sT
$comCell = ''
$comKeep = ''
if ($boxT) {
    $tblT = $boxT.Table
    $comCell = Get-PptTableCellText $tblT 1 1
    $comKeep = Get-PptTableCellText $tblT 1 2
    Release-Com $tblT
    Release-Com $boxT
}
Release-Com $sT
Close-PptPres $presT
$pt2ok = ($capT -notmatch 'Repair') `
    -and ([string]$tNode.rows[0].cells[0].text -eq $S.para2new) `
    -and ($keep -eq $S.saItem2) `
    -and ($comCell -eq $S.para2new) `
    -and ($comKeep -eq $S.saItem2)
Add-Check 'pptx-Pt2' $pt2ok "cap=$capT insp=$($tNode.rows[0].cells[0].text) keep=$keep com=$comCell comKeep=$comKeep"

# Pt3：增一行。
$rowOut = Get-OutPath 'pptx-tables-row.pptx'
Close-PptPresByPath $ppt $rowOut
if (Test-Path $rowOut) { Remove-Item -Force $rowOut }
$oldRows = [int]$tNode.rowCount
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $cellOut.Replace('\', '/')
    output = $rowOut.Replace('\', '/')
    ops = @(@{ op = 'insertRow'; id = [string]$tNode.id })
} 'pptx-pt-row' | Out-Null
$treeR = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $rowOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-pt-row-inspect'
$rNode = @($treeR.slides[0].tables) | Select-Object -First 1
$presR = Open-PptPres $ppt $rowOut
$capR = Get-PptCaption $presR
$sR = Get-PptSlide $presR 1
$boxR = Find-PptTable $sR
$comRows = 0
if ($boxR) {
    $tblR = $boxR.Table
    $rowsObj = $tblR.Rows
    $comRows = [int]$rowsObj.Count
    Release-Com $rowsObj
    Release-Com $tblR
    Release-Com $boxR
}
Release-Com $sR
Close-PptPres $presR
$pt3ok = ($capR -notmatch 'Repair') `
    -and ([int]$rNode.rowCount -eq ($oldRows + 1)) `
    -and ($comRows -eq ($oldRows + 1))
Add-Check 'pptx-Pt3' $pt3ok "cap=$capR rows=$($rNode.rowCount) com=$comRows old=$oldRows"

# Pt4：对 create 产物 insertTable。
$insertOut = Get-OutPath 'pptx-tables-insert.pptx'
Close-PptPresByPath $ppt $insertOut
if (Test-Path $insertOut) { Remove-Item -Force $insertOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $insertOut.Replace('\', '/')
    ops = @(@{
        op = 'insertTable'; id = 'slide:1'
        rows = 2; cols = 2
        x = 914400; y = 1828800; cx = 5486400; cy = 3200400
        cells = @(@($S.saItem1, $S.saItem2), @($S.saItem3, $S.saPointNew))
    })
} 'pptx-pt-insert' | Out-Null
$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $insertOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true; slide = 1 }
} 'pptx-pt-insert-inspect'
$iNode = @($treeI.slides[0].tables) | Select-Object -First 1
$presI = Open-PptPres $ppt $insertOut
$capI = Get-PptCaption $presI
$sI = Get-PptSlide $presI 1
$nI = Get-PptTableCount $sI
$boxI = Find-PptTable $sI
$comIns = ''
if ($boxI) {
    $tblI = $boxI.Table
    $comIns = Get-PptTableCellText $tblI 1 1
    Release-Com $tblI
    Release-Com $boxI
}
Release-Com $sI
Close-PptPres $presI
$pt4ok = ($null -ne $iNode) `
    -and ([string]$iNode.id -like 'slide:1/table:*') `
    -and ([bool]$iNode.stable) `
    -and ($capI -notmatch 'Repair') `
    -and ($nI -ge 1) `
    -and ($comIns -eq $S.saItem1)
Add-Check 'pptx-Pt4' $pt4ok "id=$($iNode.id) stable=$($iNode.stable) n=$nI cap=$capI com=$comIns"

# Pt5：删表，同页未删形状还在。
$delOut = Get-OutPath 'pptx-tables-delete.pptx'
Close-PptPresByPath $ppt $delOut
if (Test-Path $delOut) { Remove-Item -Force $delOut }
$keepShape = @($treeI.slides[0].shapes) | Select-Object -First 1
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $insertOut.Replace('\', '/')
    output = $delOut.Replace('\', '/')
    ops = @(@{ op = 'deleteTable'; id = [string]$iNode.id })
} 'pptx-pt-delete' | Out-Null
$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $delOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true; slide = 1 }
} 'pptx-pt-delete-inspect'
$stillTbl = @($treeD.slides[0].tables) | Where-Object { $_.id -eq $iNode.id } | Select-Object -First 1
$stillSh = @($treeD.slides[0].shapes) | Where-Object { $_.id -eq $keepShape.id } | Select-Object -First 1
$presD = Open-PptPres $ppt $delOut
$capD = Get-PptCaption $presD
$sD = Get-PptSlide $presD 1
$nD = Get-PptTableCount $sD
Release-Com $sD
Close-PptPres $presD
$pt5ok = ($capD -notmatch 'Repair') `
    -and ($null -eq $stillTbl) `
    -and ([int]$treeD.tableCount -eq 0) `
    -and ($nD -eq 0) `
    -and ($null -ne $stillSh) `
    -and ([string]$stillSh.text -eq [string]$keepShape.text)
Add-Check 'pptx-Pt5' $pt5ok "cap=$capD still=$($stillTbl.id) n=$nD keep=$($stillSh.id)"

# Pt6：形状 id 得 InvalidId；缺 rows、只剩一行时 deleteRow 得 InvalidField；均未写文件。
$failIdOut = Get-OutPath 'pptx-tables-fail-id.pptx'
if (Test-Path $failIdOut) { Remove-Item -Force $failIdOut }
$rawId = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failIdOut.Replace('\', '/')
    ops = @(@{ op = 'insertTable'; id = 'slide:1/shape:2'; rows = 2; cols = 2 })
} 'pptx-pt-fail-id'
$codeId = Get-PptErrorCode $rawId

$failRowsOut = Get-OutPath 'pptx-tables-fail-rows.pptx'
if (Test-Path $failRowsOut) { Remove-Item -Force $failRowsOut }
$rawRows = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $failRowsOut.Replace('\', '/')
    ops = @(@{ op = 'insertTable'; id = 'slide:1'; cols = 2 })
} 'pptx-pt-fail-rows'
$codeRows = Get-PptErrorCode $rawRows

$oneOut = Get-OutPath 'pptx-tables-one-row.pptx'
Close-PptPresByPath $ppt $oneOut
if (Test-Path $oneOut) { Remove-Item -Force $oneOut }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $hostPath.Replace('\', '/')
    output = $oneOut.Replace('\', '/')
    ops = @(@{ op = 'insertTable'; id = 'slide:1'; rows = 1; cols = 2 })
} 'pptx-pt-one-row' | Out-Null
$treeOne = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $oneOut.Replace('\', '/'); format = 'pptx'
    scope = @{ tables = $true }
} 'pptx-pt-one-row-inspect'
$oneId = [string](@($treeOne.slides[0].tables)[0].id)
$failLastOut = Get-OutPath 'pptx-tables-fail-last.pptx'
if (Test-Path $failLastOut) { Remove-Item -Force $failLastOut }
$rawLast = Invoke-OoxpExpectFail @{
    schemaVersion = 1; action = 'apply'
    path = $oneOut.Replace('\', '/')
    output = $failLastOut.Replace('\', '/')
    ops = @(@{ op = 'deleteRow'; id = "$oneId/row:1" })
} 'pptx-pt-fail-last'
$codeLast = Get-PptErrorCode $rawLast

$pt6ok = ($rawId.ExitCode -ne 0) -and ($codeId -eq 'InvalidId') -and (-not (Test-Path $failIdOut)) `
    -and ($rawRows.ExitCode -ne 0) -and ($codeRows -eq 'InvalidField') -and (-not (Test-Path $failRowsOut)) `
    -and ($rawLast.ExitCode -ne 0) -and ($codeLast -eq 'InvalidField') -and (-not (Test-Path $failLastOut))
Add-Check 'pptx-Pt6' $pt6ok "id=$codeId rows=$codeRows last=$codeLast"
