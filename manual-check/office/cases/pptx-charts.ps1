# pptx 幻灯片图表：docs/checklist-pptx.md 的 Pc1–Pc7。
# 源文件：PowerPoint 亲手另存，第一页至少一张柱形图。不要用库 create 产物当唯一样例。

$ErrorActionPreference = 'Stop'
. ([IO.Path]::Combine($PSScriptRoot, '..', '..', 'lib', 'bootstrap.ps1'))
if (-not $global:OoxpSuiteName) { Initialize-Suite -Name 'office' }

$S = $global:OoxpStrings
$src = Ensure-PptxPptChartSource
$ppt = Ensure-MsPpt $src
Close-PptPresByPath $ppt $src

function Flatten-Nums($vals) {
    $out = @()
    foreach ($v in @($vals)) {
        if ($null -eq $v) { continue }
        try { $out += [double]$v } catch {}
    }
    return $out
}

function Test-NumsClose($a, $b) {
    $x = @(Flatten-Nums $a)
    $y = @(Flatten-Nums $b)
    if ($x.Count -ne $y.Count) { return $false }
    for ($i = 0; $i -lt $x.Count; $i++) {
        if ([math]::Abs($x[$i] - $y[$i]) -gt 0.01) { return $false }
    }
    return $true
}

function Pick-ValueSeries($chart) {
    foreach ($s in @($chart.series)) {
        if ((@(Flatten-Nums $s.values).Count) -ge 2) { return $s }
    }
    return @($chart.series) | Select-Object -First 1
}

function Get-PptChartCount($slide) {
    $n = 0
    $shapes = $slide.Shapes
    try {
        for ($i = 1; $i -le $shapes.Count; $i++) {
            $sh = $shapes.Item($i)
            try {
                $has = $false
                try { $has = [bool]$sh.HasChart } catch {
                    try { $has = ([int]$sh.Type -eq 3) } catch { $has = $false }
                }
                if ($has) { $n++ }
            }
            finally { Release-Com $sh }
        }
    }
    finally { Release-Com $shapes }
    return $n
}

function Find-PptChart($slide, [string]$name) {
    $shapes = $slide.Shapes
    try {
        for ($i = 1; $i -le $shapes.Count; $i++) {
            $cand = $shapes.Item($i)
            $has = $false
            try { $has = [bool]$cand.HasChart } catch {
                try { $has = ([int]$cand.Type -eq 3) } catch { $has = $false }
            }
            if ($has -and [string]$cand.Name -eq $name) {
                return $cand
            }
            Release-Com $cand
        }
    }
    finally { Release-Com $shapes }
    return $null
}

function Get-PptSeriesNums($chart, [int]$index) {
    $ser = $null
    try { $ser = $chart.SeriesCollection($index) } catch {
        try { $ser = $chart.FullSeriesCollection($index) } catch { return @() }
    }
    try {
        return @(Flatten-Nums $ser.Values)
    }
    finally { if ($ser) { Release-Com $ser } }
}

function Get-FirstPptChart($slide) {
    $shapes = $slide.Shapes
    try {
        for ($i = 1; $i -le $shapes.Count; $i++) {
            $cand = $shapes.Item($i)
            $has = $false
            try { $has = [bool]$cand.HasChart } catch {
                try { $has = ([int]$cand.Type -eq 3) } catch { $has = $false }
            }
            if ($has) { return $cand }
            Release-Com $cand
        }
    }
    finally { Release-Com $shapes }
    return $null
}

function Get-EmbedPartName([string]$pptx) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($pptx)
    try {
        foreach ($e in $zip.Entries) {
            $n = $e.FullName -replace '\\', '/'
            if ($n -like 'ppt/embeddings/*.xlsx') { return $n }
        }
    }
    finally { $zip.Dispose() }
    return $null
}

function Read-ZipBytes([string]$pptx, [string]$name) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($pptx)
    try {
        $entry = $zip.Entries | Where-Object { ($_.FullName -replace '\\', '/') -eq $name } | Select-Object -First 1
        if (-not $entry) { return $null }
        $ms = New-Object System.IO.MemoryStream
        $es = $entry.Open()
        $es.CopyTo($ms)
        $es.Dispose()
        $bytes = $ms.ToArray()
        $ms.Dispose()
        return $bytes
    }
    finally { $zip.Dispose() }
}

# Pc1：无 charts 时只有计数；charts:true 时 id 为 slide:…/chart:…，系列值与 embedding 一致。
$tree0 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
} 'pptx-chart-inspect-list'
$slide0 = $tree0.slides[0]
$noList = -not ($slide0.PSObject.Properties.Name -contains 'charts')
Add-Check 'pptx-Pc1a' (($tree0.chartCount -ge 1) -and ($slide0.chartCount -ge 1) -and $noList) "wb=$($tree0.chartCount) slide=$($slide0.chartCount) noList=$noList"

$tree1 = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $src.Replace('\', '/'); format = 'pptx'
    scope = @{ charts = $true }
} 'pptx-chart-inspect-expand'
$charts = @($tree1.slides[0].charts)
$col = $charts | Where-Object { [string]$_.type -eq 'col' } | Select-Object -First 1
if (-not $col) { $col = $charts | Select-Object -First 1 }
$idOk = $col -and ([string]$col.id -like 'slide:1/chart:*')
$ser0 = if ($col) { Pick-ValueSeries $col } else { $null }
$serVals = if ($ser0) { @(Flatten-Nums $ser0.values) } else { @() }
$serId = if ($ser0) { [string]$ser0.id } else { '' }

$embedName = Get-EmbedPartName $src
$embedHit = 0
if ($embedName -and $serVals.Count -gt 0) {
    $tmpXlsx = Get-OutPath 'pptx-chart-src-embed.xlsx'
    if (Test-Path $tmpXlsx) { Remove-Item $tmpXlsx -Force }
    $eb = Read-ZipBytes $src $embedName
    if ($eb) {
        [IO.File]::WriteAllBytes($tmpXlsx, $eb)
        $treeE = Invoke-OoxpTree @{
            schemaVersion = 1; action = 'inspect'
            path = $tmpXlsx.Replace('\', '/'); format = 'xlsx'
            scope = @{ sheet = 'Sheet1'; range = 'A1:Z40' }
        } 'pptx-chart-src-embed-cells'
        $cellNums = @()
        foreach ($c in @($treeE.sheets[0].cells)) {
            try { $cellNums += [double]$c.value } catch {}
        }
        foreach ($nv in $serVals) {
            foreach ($cn in $cellNums) {
                if ([math]::Abs($cn - $nv) -lt 0.01) { $embedHit++; break }
            }
        }
    }
}
Add-Check 'pptx-Pc1' ($idOk -and ($serVals.Count -ge 2) -and ($embedHit -ge $serVals.Count) -and ($null -ne $embedName)) "id=$($col.id) type=$($col.type) vals=$($serVals -join ',') embed=$embedName hit=$embedHit"

$colId = if ($col) { [string]$col.id } else { '' }
$nCat = $serVals.Count
if ($nCat -lt 1) { $nCat = 3 }
$newVals = @()
for ($i = 1; $i -le $nCat; $i++) { $newVals += (100 * $i) }

# Pc2：改系列。数据表是新值，PowerPoint 无修复。
$seriesOut = Get-OutPath 'pptx-chart-series.pptx'
Close-PptPresByPath $ppt $seriesOut
if (Test-Path $seriesOut) { Remove-Item $seriesOut -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $src.Replace('\', '/')
    output = $seriesOut.Replace('\', '/')
    ops = @(@{ op = 'setChartSeries'; id = $serId; values = $newVals })
} 'pptx-chart-set-series' | Out-Null

$treeS = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $seriesOut.Replace('\', '/'); format = 'pptx'
    scope = @{ charts = $true }
} 'pptx-chart-series-inspect'
$colS = @($treeS.slides[0].charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$gotSer = if ($colS) { Pick-ValueSeries $colS } else { $null }
$gotVals = if ($gotSer) { @(Flatten-Nums $gotSer.values) } else { @() }

$presS = Open-PptPres $ppt $seriesOut
$capS = Get-PptCaption $presS
$sS = Get-PptSlide $presS 1
$comNew = $false
$shapeS = Find-PptChart $sS ([string]$colS.name)
if (-not $shapeS) { $shapeS = Get-FirstPptChart $sS }
if ($shapeS) {
    $chS = $null
    try { $chS = $shapeS.Chart } catch {}
    if ($chS) {
        try {
            $cv = Get-PptSeriesNums $chS 1
            if (Test-NumsClose $cv $newVals) { $comNew = $true }
        }
        catch {}
        finally { Release-Com $chS }
    }
    Release-Com $shapeS
}
Release-Com $sS
Close-PptPres $presS
Add-Check 'pptx-Pc2' (($capS -notmatch 'Repair') -and (Test-NumsClose $gotVals $newVals) -and $comNew) "caption=$capS inspect=$($gotVals -join ',') want=$($newVals -join ',') com=$comNew"

# Pc3：插图，能看见且无修复。
$inserted = Get-OutPath 'pptx-chart-inserted.pptx'
Close-PptPresByPath $ppt $inserted
if (Test-Path $inserted) { Remove-Item $inserted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $seriesOut.Replace('\', '/')
    output = $inserted.Replace('\', '/')
    ops = @(@{
        op = 'insertChart'
        id = 'slide:1'
        type = 'col'
        title = $S.chartTitle
        x = 5486400
        y = 2743200
        cx = 3657600
        cy = 1828800
        series = @(@{
            name = $S.chartSer1
            categories = @($S.saItem1, $S.saItem2, $S.saItem3)
            values = @(1, 2, 3)
        })
    })
} 'pptx-chart-insert' | Out-Null

$treeI = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $inserted.Replace('\', '/'); format = 'pptx'
    scope = @{ charts = $true }
} 'pptx-chart-insert-inspect'
$oldIds = @($charts | ForEach-Object { $_.id })
$newChart = @($treeI.slides[0].charts) | Where-Object { $oldIds -notcontains $_.id } | Select-Object -First 1

$presI = Open-PptPres $ppt $inserted
$capI = Get-PptCaption $presI
$sI = Get-PptSlide $presI 1
$nI = Get-PptChartCount $sI
Release-Com $sI
Close-PptPres $presI
Add-Check 'pptx-Pc3' (($capI -notmatch 'Repair') -and ($null -ne $newChart) -and ($newChart.type -eq 'col') -and ($newChart.x -eq 5486400) -and ($nI -ge 2)) "caption=$capI id=$($newChart.id) type=$($newChart.type) ppt=$nI"

# Pc4：删掉刚插的图，无修复。
$deleted = Get-OutPath 'pptx-chart-deleted.pptx'
Close-PptPresByPath $ppt $deleted
if (Test-Path $deleted) { Remove-Item $deleted -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $deleted.Replace('\', '/')
    ops = @(@{ op = 'deleteChart'; id = $newChart.id })
} 'pptx-chart-delete' | Out-Null

$treeD = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $deleted.Replace('\', '/'); format = 'pptx'
    scope = @{ charts = $true }
} 'pptx-chart-delete-inspect'
$stillNew = @($treeD.slides[0].charts) | Where-Object { $_.id -eq $newChart.id } | Select-Object -First 1
$presD = Open-PptPres $ppt $deleted
$capD = Get-PptCaption $presD
$sD = Get-PptSlide $presD 1
$nD = Get-PptChartCount $sD
Release-Com $sD
Close-PptPres $presD
Add-Check 'pptx-Pc4' (($capD -notmatch 'Repair') -and ($null -eq $stillNew) -and ($treeD.slides[0].chartCount -lt $treeI.slides[0].chartCount) -and ($nD -lt $nI)) "caption=$capD count=$($treeD.slides[0].chartCount) ppt=$nD"

# Pc5：柱形改折线。允许样式重置，无修复。
$lined = Get-OutPath 'pptx-chart-line.pptx'
Close-PptPresByPath $ppt $lined
if (Test-Path $lined) { Remove-Item $lined -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $seriesOut.Replace('\', '/')
    output = $lined.Replace('\', '/')
    ops = @(@{ op = 'setChartType'; id = $colId; type = 'line' })
} 'pptx-chart-set-type' | Out-Null

$treeL = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $lined.Replace('\', '/'); format = 'pptx'
    scope = @{ charts = $true }
} 'pptx-chart-line-inspect'
$lineC = @($treeL.slides[0].charts) | Where-Object { $_.id -eq $colId } | Select-Object -First 1
$presL = Open-PptPres $ppt $lined
$capL = Get-PptCaption $presL
$sL = Get-PptSlide $presL 1
$lineTypeOk = $false
$lineTypes = @(4, 63, 64, 65, 66, 67)
$shapeL = Find-PptChart $sL ([string]$lineC.name)
if (-not $shapeL) { $shapeL = Get-FirstPptChart $sL }
if ($shapeL) {
    $chL = $null
    try { $chL = $shapeL.Chart } catch {}
    if ($chL) {
        try {
            if ($lineTypes -contains [int]$chL.ChartType) { $lineTypeOk = $true }
        }
        catch {}
        finally { Release-Com $chL }
    }
    Release-Com $shapeL
}
Release-Com $sL
Close-PptPres $presL
Add-Check 'pptx-Pc5' (($capL -notmatch 'Repair') -and ($lineC.type -eq 'line') -and $lineTypeOk) "caption=$capL type=$($lineC.type) excelLine=$lineTypeOk"

# Pc6：改尺寸和位置。
$moved = Get-OutPath 'pptx-chart-moved.pptx'
Close-PptPresByPath $ppt $moved
if (Test-Path $moved) { Remove-Item $moved -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'apply'
    path = $inserted.Replace('\', '/')
    output = $moved.Replace('\', '/')
    ops = @(
        @{
            op = 'setChartSize'
            id = $newChart.id
            cx = 1828800
            cy = 914400
        }
        @{
            op = 'setChartPos'
            id = $newChart.id
            x = 914400
            y = 1371600
        }
    )
} 'pptx-chart-size-pos' | Out-Null

$treeM = Invoke-OoxpTree @{
    schemaVersion = 1; action = 'inspect'
    path = $moved.Replace('\', '/'); format = 'pptx'
    scope = @{ charts = $true }
} 'pptx-chart-moved-inspect'
$movedC = @($treeM.slides[0].charts) | Where-Object { $_.id -eq $newChart.id } | Select-Object -First 1
$presM = Open-PptPres $ppt $moved
$capM = Get-PptCaption $presM
$sM = Get-PptSlide $presM 1
$shapeM = Find-PptChart $sM ([string]$movedC.name)
if (-not $shapeM) {
    $shapesM = $sM.Shapes
    try {
        for ($i = 1; $i -le $shapesM.Count; $i++) {
            $cand = $shapesM.Item($i)
            $has = $false
            try { $has = [bool]$cand.HasChart } catch { $has = $false }
            if ($has -and [string]$cand.Name -eq [string]$newChart.name) {
                $shapeM = $cand
            }
            elseif ($has -and -not $shapeM) { $shapeM = $cand }
            else { Release-Com $cand }
        }
    }
    finally { Release-Com $shapesM }
}
$wPt = 0; $hPt = 0; $left = 0; $top = 0
if (-not $shapeM) { throw "PowerPoint 里找不到要改位置的图 $($newChart.name)" }
$wPt = [double]$shapeM.Width
$hPt = [double]$shapeM.Height
$left = [double]$shapeM.Left
$top = [double]$shapeM.Top
# 144×72 磅 = 1828800×914400 EMU；72×108 磅 = 914400×1371600 EMU。
$sizeOk = ([math]::Abs($wPt - 144) -lt 8) -and ([math]::Abs($hPt - 72) -lt 8)
$posOk = ([math]::Abs($left - 72) -lt 8) -and ([math]::Abs($top - 108) -lt 8)
$inspectOk = ($movedC.cx -eq 1828800) -and ($movedC.x -eq 914400)
Add-Check 'pptx-Pc6' (($capM -notmatch 'Repair') -and $sizeOk -and $posOk -and $inspectOk) "caption=$capM w=$wPt h=$hPt left=$left top=$top cx=$($movedC.cx)"
Release-Com $shapeM
Release-Com $sM
Close-PptPres $presM

# Pc7：exportChartData 与 embedding 字节一致。
$exported = Get-OutPath 'pptx-chart-export.xlsx'
if (Test-Path $exported) { Remove-Item $exported -Force }
Invoke-OoxpRequest @{
    schemaVersion = 1; action = 'exportChartData'
    path = $moved.Replace('\', '/')
    id = $colId
    output = $exported.Replace('\', '/')
} 'pptx-chart-export' | Out-Null

$outBytes = [IO.File]::ReadAllBytes($exported)
$matched = 0
$mediaLens = @()
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead($moved)
try {
    foreach ($entry in @($zip.Entries | Where-Object { ($_.FullName -replace '\\', '/') -like 'ppt/embeddings/*.xlsx' })) {
        $ms = New-Object System.IO.MemoryStream
        $es = $entry.Open()
        $es.CopyTo($ms)
        $es.Dispose()
        $mediaBytes = $ms.ToArray()
        $ms.Dispose()
        $mediaLens += $mediaBytes.Length
        if ($mediaBytes.Length -ne $outBytes.Length) { continue }
        $sameEntry = $true
        for ($i = 0; $i -lt $outBytes.Length; $i++) {
            if ($outBytes[$i] -ne $mediaBytes[$i]) { $sameEntry = $false; break }
        }
        if ($sameEntry) { $matched++ }
    }
}
finally { $zip.Dispose() }
Add-Check 'pptx-Pc7' ($matched -eq 1) "export=$($outBytes.Length) matched=$matched media=$($mediaLens -join ',')"
