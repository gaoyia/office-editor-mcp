# 用例要用的输入文件。谁缺谁造，所以任何一个用例都能单独跑。
# 已经在就直接复用：apply 只读源文件、写到另一个 output，源文件不会被改脏。

function Ensure-XlsxCreate([switch]$Force) {
    $path = Get-OutPath 'xlsx-create.xlsx'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $req = Get-RequestTemplate 'xlsx-create' 'xlsx-create.xlsx'
    Invoke-OoxpRequest $req 'xlsx-create' | Out-Null
    return $path
}

# 一张 oneCell（B2）加一张 twoCell（D2:F8）。
function Ensure-XlsxCreatePictures([switch]$Force) {
    $path = Get-OutPath 'xlsx-create-pictures.xlsx'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $png = Ensure-OoxpPngA
    $file = $png.Replace('\', '/')
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $path.Replace('\', '/'); format = 'xlsx'
        workbook = @{
            sheets = @(
                @{
                    name = 'Sheet1'
                    cells = @(@{ ref = 'A1'; value = 1 })
                    pictures = @(
                        @{ file = $file; from = 'B2' }
                        @{ file = $file; from = 'D2'; to = 'F8'; anchor = 'twoCell' }
                    )
                }
            )
        }
    }
    Invoke-OoxpRequest $req 'xlsx-create-pictures' | Out-Null
    return $path
}

function Ensure-XlsxCreateCharts([switch]$Force) {
    $path = Get-OutPath 'xlsx-create-charts.xlsx'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $path.Replace('\', '/'); format = 'xlsx'
        workbook = @{
            sheets = @(
                @{
                    name = 'Sheet1'
                    charts = @(
                        @{
                            type = 'col'
                            title = '销量'
                            from = 'E2'
                            dataFrom = 'A10'
                            series = @(
                                @{
                                    name = 'Q1'
                                    categories = @('甲', '乙', '丙')
                                    values = @(1, 2, 3)
                                }
                            )
                        }
                    )
                }
            )
        }
    }
    Invoke-OoxpRequest $req 'xlsx-create-charts' | Out-Null
    return $path
}

function Ensure-XlsxCreateHeaderFooter([switch]$Force) {
    $path = Get-OutPath 'xlsx-create-headerfooter.xlsx'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $png = Ensure-OoxpPngA
    $file = $png.Replace('\', '/')
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $path.Replace('\', '/'); format = 'xlsx'
        workbook = @{
            sheets = @(
                @{
                    name = 'Sheet1'
                    cells = @(@{ ref = 'A1'; value = 1 })
                    headerFooters = @(
                        @{
                            kind = 'odd'
                            side = 'header'
                            text = '&CC14页眉'
                            pictures = @(@{ file = $file })
                        }
                        @{
                            kind = 'odd'
                            side = 'footer'
                            text = '&CC14页脚'
                        }
                    )
                }
            )
        }
    }
    Invoke-OoxpRequest $req 'xlsx-create-headerfooter' | Out-Null
    return $path
}

function Ensure-DocxCreate([switch]$Force) {
    $path = Get-OutPath 'docx-create.docx'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $req = Get-RequestTemplate 'docx-create' 'docx-create.docx'
    Invoke-OoxpRequest $req 'docx-create' | Out-Null
    return $path
}

# 正文第二段行内图 + 默认页眉浮动 square。
function Ensure-DocxCreatePictures([switch]$Force) {
    $path = Get-OutPath 'docx-create-pictures.docx'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $png = Ensure-OoxpPngA
    $S = $global:OoxpStrings
    $file = $png.Replace('\', '/')
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $path.Replace('\', '/'); format = 'docx'
        document = @{
            paragraphs = @(
                @{ style = 'Heading1'; text = $S.title }
                @{ text = $S.hello; pictures = @(@{ file = $file }) }
            )
            headers = @(
                @{
                    type = 'default'
                    paragraphs = @(
                        @{
                            text = '页眉'
                            pictures = @(@{ file = $file; kind = 'anchor'; wrap = 'square' })
                        }
                    )
                }
            )
        }
    }
    Invoke-OoxpRequest $req 'docx-create-pictures' | Out-Null
    return $path
}

# 两节、饼图、一条脚注、一条尾注、一条批注。内容与 docs/examples/docx-create-full.json 相同。
function Ensure-DocxCreateFull([switch]$Force) {
    $path = Get-OutPath 'docx-create-full.docx'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $path.Replace('\', '/'); format = 'docx'
        document = @{
            paragraphs = @(
                @{
                    style = 'Heading1'
                    text = '第一章'
                    sectionType = 'nextPage'
                    footnote = '脚注一'
                    comments = @(@{ text = '批注一'; author = 'ooxp'; initials = 'oo' })
                }
                @{
                    text = 'hello'
                    endnote = '尾注一'
                    charts = @(
                        @{
                            type = 'pie'
                            title = '份额'
                            series = @(
                                @{
                                    name = 'S1'
                                    categories = @('甲', '乙')
                                    values = @(3, 1)
                                }
                            )
                        }
                    )
                }
            )
        }
    }
    Invoke-OoxpRequest $req 'docx-create-full' | Out-Null
    return $path
}

function Ensure-PptxCreate([switch]$Force) {
    $path = Get-OutPath 'pptx-create.pptx'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $req = Get-RequestTemplate 'pptx-create' 'pptx-create.pptx'
    Invoke-OoxpRequest $req 'pptx-create' | Out-Null
    return $path
}

# 一页一个文本框加两张图。cNvPr：spTree=1，文本框=2，图从 3 起。
function Ensure-PptxCreatePictures([switch]$Force) {
    $path = Get-OutPath 'pptx-create-pictures.pptx'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $png = Ensure-OoxpPngA
    $file = $png.Replace('\', '/')
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $path.Replace('\', '/'); format = 'pptx'
        presentation = @{
            slides = @(
                @{
                    shapes = @(@{ text = '标题' })
                    pictures = @(
                        @{ file = $file; x = 914400; y = 1828800; cx = 1828800; cy = 1828800 }
                        @{ file = $file; x = 3657600; y = 1828800; cx = 1828800; cy = 1828800 }
                    )
                }
            )
        }
    }
    Invoke-OoxpRequest $req 'pptx-create-pictures' | Out-Null
    return $path
}

# 一页一个文本框加一张柱形图。cNvPr：spTree=1，文本框=2，图表从 3 起。
function Ensure-PptxCreateCharts([switch]$Force) {
    $path = Get-OutPath 'pptx-create-charts.pptx'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $path.Replace('\', '/'); format = 'pptx'
        presentation = @{
            slides = @(
                @{
                    shapes = @(@{ text = '标题' })
                    charts = @(
                        @{
                            type = 'col'
                            title = $global:OoxpStrings.chartTitle
                            x = 914400
                            y = 1828800
                            cx = 5486400
                            cy = 3200400
                            series = @(
                                @{
                                    name = $global:OoxpStrings.chartSer1
                                    categories = @($global:OoxpStrings.saItem1, $global:OoxpStrings.saItem2, $global:OoxpStrings.saItem3)
                                    values = @(1, 2, 3)
                                }
                            )
                        }
                    )
                }
            )
        }
    }
    Invoke-OoxpRequest $req 'pptx-create-charts' | Out-Null
    return $path
}

# 一页一个文本框加一张 list。cNvPr：spTree=1，文本框=2，图示从 3 起。
function Ensure-PptxCreateSmartArt([switch]$Force) {
    $path = Get-OutPath 'pptx-create-smartart.pptx'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $S = $global:OoxpStrings
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $path.Replace('\', '/'); format = 'pptx'
        presentation = @{
            slides = @(
                @{
                    shapes = @(@{ text = '标题' })
                    smartArts = @(
                        @{
                            layout = 'list'
                            points = @(
                                @{ text = $S.saItem1 }
                                @{ text = $S.saItem2 }
                                @{ text = $S.saItem3 }
                            )
                        }
                    )
                }
            )
        }
    }
    Invoke-OoxpRequest $req 'pptx-create-smartart' | Out-Null
    return $path
}

# 两页：第一页有演讲者备注和一条现代批注，第二页只有文本框。
function Ensure-PptxCreateNotesComments([switch]$Force) {
    $path = Get-OutPath 'pptx-create-notes-comments.pptx'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $path.Replace('\', '/'); format = 'pptx'
        presentation = @{
            slides = @(
                @{
                    shapes = @(@{ text = '标题' })
                    notes = '演讲者备注'
                    comments = @(
                        @{
                            text = '一条批注'
                            author = 'Alice'
                            initials = 'A'
                            date = '2026-09-01T00:00:00.000'
                            x = 914400
                            y = 914400
                        }
                    )
                }
                @{
                    shapes = @(@{ text = '第二页' })
                }
            )
        }
    }
    Invoke-OoxpRequest $req 'pptx-create-notes-comments' | Out-Null
    return $path
}

# 两页：第一节「开场」、第二节「内容」；第一页带 url、淡入切换与进入淡入；第二页带视频和音频。
function Ensure-PptxCreateRemain([switch]$Force) {
    $path = Get-OutPath 'pptx-create-remain.pptx'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $mp4 = (Ensure-OoxpMp4).Replace('\', '/')
    $wav = (Ensure-OoxpWav).Replace('\', '/')
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $path.Replace('\', '/'); format = 'pptx'
        presentation = @{
            sections = @(
                @{ name = '开场'; start = 1 }
                @{ name = '内容'; start = 2 }
            )
            slides = @(
                @{
                    shapes = @(@{ text = '标题'; url = 'https://example.com/' })
                    transition = @{ type = 'fade' }
                    anims = @(@{ type = 'fade'; shape = 1 })
                }
                @{
                    shapes = @(@{ text = '第二页' })
                    videos = @(@{ file = $mp4 })
                    audios = @(@{ file = $wav })
                }
            )
        }
    }
    Invoke-OoxpRequest $req 'pptx-create-remain' | Out-Null
    return $path
}

# apply 的源文件要由 Excel 亲手存出来，不能拿 ooxp 自己的产物冒充，
# 否则等于自己造自己读，测不到真实文件的写法。
function Ensure-XlsxExcelSource([switch]$Force) {
    $path = Get-OutPath 'xlsx-excel-source.xlsx'
    if ((Test-Path $path) -and -not $Force) { return $path }

    $excel = Ensure-MsExcel (Ensure-XlsxCreate)
    $wb = $excel.Workbooks.Add()
    while ($wb.Worksheets.Count -lt 2) { [void]$wb.Worksheets.Add() }
    $s1 = $wb.Worksheets.Item(1)
    $s2 = $wb.Worksheets.Item(2)
    # 先改成临时名，避免与目标名撞车导致改名失败。
    $s1.Name = '_tmpA'
    $s2.Name = '_tmpB'
    $s1.Name = 'Sheet1'
    $s2.Name = 'Sheet2'
    $s1.Range('A1').Value2 = 1
    $s1.Range('A2').Value2 = 2
    $s1.Range('A3').Value2 = 3
    $s1.Range('B1').Formula = '=SUM(A1)'
    $s1.Range('C1').Value2 = 123.45
    $s1.Range('C1').NumberFormat = '0.00'
    $s1.Range('D1').Formula = '=A1*2'
    $s1.Range('E1').Value2 = 'keep'
    $s1.Range('E1').Font.Bold = $true
    $s1.Range('E1').Font.Color = 255
    $s1.Range('E1').Interior.Color = 65535
    $s1.Columns.Item(5).ColumnWidth = 20
    $s2.Range('A1').Value2 = 'untouched'
    $s2.Range('B1').Value2 = 99
    $s2.Range('A1').Font.Bold = $true
    if (Test-Path $path) { Remove-Item $path -Force }
    $wb.SaveAs($path, 51)
    $wb.Close($false)
    Release-Com $s1
    Release-Com $s2
    Release-Com $wb
    Write-Log "saved excel source $path"
    return $path
}

function Ensure-XlsxExcelHeaderFooterSource([switch]$Force) {
    $path = Get-OutPath 'xlsx-excel-headerfooter-source.xlsx'
    if ((Test-Path $path) -and -not $Force) { return $path }

    $excel = Ensure-MsExcel (Ensure-XlsxCreate)
    $wb = $excel.Workbooks.Add()
    $s1 = $wb.Worksheets.Item(1)
    $s1.Name = 'Sheet1'
    $s1.Range('A1').Value2 = 1
    $ps = $s1.PageSetup
    try {
        $ps.CenterHeader = 'Xh页眉'
        $ps.CenterFooter = 'Xh页脚'
    }
    finally { Release-Com $ps }
    if (Test-Path $path) { Remove-Item $path -Force }
    $wb.SaveAs($path, 51)
    $wb.Close($false)
    Release-Com $s1
    Release-Com $wb
    Write-Log "saved excel headerfooter source $path"
    return $path
}

# 工作表图手测源文件至少各一张 oneCell 与 twoCell。
# Excel 16.0 的 Shapes.AddPicture 只写 twoCellAnchor，Placement 只改 editAs。
# 真 oneCellAnchor 只能由 ooxp 写入，再让 Excel 打开并 Save()，源文件仍算 Excel 另存。
function Test-XlsxPictureSourceAnchors([string]$xlsx, [string]$reqName) {
    $tree = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $xlsx.Replace('\', '/'); format = 'xlsx'
        scope = @{ pictures = $true }
    } $reqName
    $sheet = @($tree.sheets) | Where-Object { $_.kind -eq 'worksheet' } | Select-Object -First 1
    $anchors = @($sheet.pictures | ForEach-Object { $_.anchor })
    return [pscustomobject]@{
        anchors = $anchors
        ok = ($anchors -contains 'oneCell') -and ($anchors -contains 'twoCell')
    }
}

function Ensure-XlsxExcelPictureSource([switch]$Force) {
    $path = Get-OutPath 'xlsx-excel-picture-source.xlsx'
    if ((Test-Path $path) -and -not $Force) {
        $cached = Test-XlsxPictureSourceAnchors $path 'xlsx-picture-source-cache-check'
        if ($cached.ok) { return $path }
        Write-Log "缓存的 picture source 缺 oneCell/twoCell（$($cached.anchors -join ',')），重建"
    }

    $pngA = Ensure-OoxpPngA
    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $excelSaved = Join-Path $tmpDir 'xlsx-excel-picture-excel.xlsx'
    $built = Join-Path $tmpDir 'xlsx-excel-picture-built.xlsx'
    $beforeSave = Join-Path $tmpDir 'xlsx-excel-picture-before-save.xlsx'

    $excel = Ensure-MsExcel (Ensure-XlsxCreate)
    $wb = $excel.Workbooks.Add()
    $s1 = $wb.Worksheets.Item(1)
    $s1.Name = 'Sheet1'
    $s1.Range('A1').Value2 = 1
    $s1.Range('C3').Value2 = 'pic'
    $shapes = $s1.Shapes
    $move = $shapes.AddPicture($pngA, 0, -1, 10, 10, 72, 54)
    $move.Placement = 2
    $size = $shapes.AddPicture($pngA, 0, -1, 200, 10, 72, 54)
    $size.Placement = 1
    if (Test-Path $excelSaved) { Remove-Item $excelSaved -Force }
    $wb.SaveAs($excelSaved, 51)
    $wb.Close($false)
    Release-Com $move
    Release-Com $size
    Release-Com $shapes
    Release-Com $s1
    Release-Com $wb

    if (Test-Path $built) { Remove-Item $built -Force }
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $excelSaved.Replace('\', '/')
        output = $built.Replace('\', '/')
        ops = @(@{
            op = 'insertPicture'
            id = 'sheet:Sheet1'
            file = $pngA.Replace('\', '/')
            from = 'A10'
            cx = 914400
            cy = 914400
        })
    } 'xlsx-picture-source-insert-one' | Out-Null

    $afterInsert = Test-XlsxPictureSourceAnchors $built 'xlsx-picture-source-built-inspect'
    if (-not $afterInsert.ok) {
        throw "ooxp 插入 oneCell 之后锚点不对：$($afterInsert.anchors -join ',')"
    }

    Copy-Item -Force $built $beforeSave
    $wb2 = Open-ExcelBook $excel $built
    $cap = [string]$wb2.Windows.Item(1).Caption
    if ($cap -match 'Repair') { throw "Excel 打开插图后的源文件要修复：$cap" }
    Write-Log 'excel Save() 覆盖 TEMP 里已插入 oneCell 的源文件'
    $wb2.Save()
    $wb2.Close($false)
    Release-Com $wb2

    $afterSave = Test-XlsxPictureSourceAnchors $built 'xlsx-picture-source-saved-inspect'
    if ($afterSave.ok) {
        Copy-Item -Force $built $path
    }
    else {
        Write-Log "Excel Save 之后 oneCell 没了（$($afterSave.anchors -join ',')），改用打开过、未再存的副本"
        Copy-Item -Force $beforeSave $path
    }
    Write-Log "saved excel picture source $path"
    return $path
}

# 工作表图表手测源文件：Excel 亲手插一张柱形图，系列公式指向本表单元格。
# 不要用库 create / insertChart 产物冒充。Excel 16 插入通常是 twoCellAnchor。
function Test-XlsxChartSource([string]$xlsx, [string]$reqName) {
    $tree = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $xlsx.Replace('\', '/'); format = 'xlsx'
        scope = @{ charts = $true }
    } $reqName
    $sheet = @($tree.sheets) | Where-Object { $_.kind -eq 'worksheet' } | Select-Object -First 1
    $charts = @()
    if ($sheet -and ($sheet.PSObject.Properties.Name -contains 'charts')) {
        $charts = @($sheet.charts)
    }
    $col = $charts | Where-Object { [string]$_.type -eq 'col' } | Select-Object -First 1
    $nVal = 0
    $valRef = ''
    if ($col) {
        $ser = @($col.series) | Select-Object -First 1
        if ($ser) {
            $nVal = @($ser.values).Count
            if ($null -ne $ser.valRef) { $valRef = [string]$ser.valRef }
        }
    }
    return [pscustomobject]@{
        chartCount = [int]$tree.chartCount
        n = $charts.Count
        type = $(if ($col) { [string]$col.type } else { '' })
        id = $(if ($col) { [string]$col.id } else { '' })
        nVal = $nVal
        valRef = $valRef
        ok = ($tree.chartCount -ge 1) -and ($null -ne $col) -and ($nVal -ge 2) -and ($valRef.Length -gt 0)
    }
}

function Write-XlsxChartZipHint([string]$xlsx) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $z = [IO.Compression.ZipFile]::OpenRead($xlsx)
    try {
        $hits = @($z.Entries | Where-Object { ($_.FullName -replace '\\', '/') -match 'xl/(drawings|charts)/' })
        Write-Log ("chart zip: " + (($hits | ForEach-Object { $_.FullName }) -join ', '))
        $draw = $hits |
            Where-Object { ($_.FullName -replace '\\', '/') -like 'xl/drawings/drawing*.xml' } |
            Select-Object -First 1
        if (-not $draw) { return }
        $ms = New-Object IO.MemoryStream
        $es = $draw.Open()
        try { $es.CopyTo($ms) } finally { $es.Dispose() }
        $txt = [Text.Encoding]::UTF8.GetString($ms.ToArray())
        $ms.Dispose()
        $snip = if ($txt.Length -gt 500) { $txt.Substring(0, 500) } else { $txt }
        Write-Log ("drawing snip: $snip")
    }
    finally { $z.Dispose() }
}

function Ensure-XlsxExcelChartSource([switch]$Force) {
    $path = Get-OutPath 'xlsx-excel-chart-source.xlsx'
    if ((Test-Path $path) -and -not $Force) {
        $cached = Test-XlsxChartSource $path 'xlsx-chart-source-cache-check'
        if ($cached.ok) { return $path }
        Write-Log "缓存的 chart source 不合格（count=$($cached.chartCount) type=$($cached.type) nVal=$($cached.nVal) valRef=$($cached.valRef)），重建"
    }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $excelSaved = Join-Path $tmpDir 'xlsx-excel-chart-excel.xlsx'

    $excel = Ensure-MsExcel (Ensure-XlsxCreate)
    $wb = $excel.Workbooks.Add()
    $s1 = $wb.Worksheets.Item(1)
    $s1.Name = 'Sheet1'
    $s1.Range('B1').Value2 = 'S1'
    $s1.Range('A2').Value2 = 'A'
    $s1.Range('B2').Value2 = 10
    $s1.Range('A3').Value2 = 'B'
    $s1.Range('B3').Value2 = 20
    $s1.Range('A4').Value2 = 'C'
    $s1.Range('B4').Value2 = 30
    $rng = $s1.Range('A1:B4')
    try { [void]$rng.Select() } catch {}

    # xlColumnClustered = 51；xlColumns = 2。不要用 Charts.Add，那会建成图表工作表。
    $xlColumnClustered = 51
    $xlColumns = 2
    $hostObj = $null
    $shapes = $s1.Shapes
    try {
        try {
            $hostObj = $shapes.AddChart2(-1, $xlColumnClustered, 240, 20, 360, 200)
        }
        catch {
            Write-Log ("AddChart2 报错：" + $_.Exception.Message)
            $cos = $null
            try { $cos = $s1.ChartObjects() } catch { $cos = $s1.ChartObjects }
            try {
                $hostObj = $cos.Add(240, 20, 360, 200)
            }
            finally { Release-Com $cos }
        }
    }
    finally { Release-Com $shapes }

    if (-not $hostObj) { throw 'Excel 没有插入工作表柱形图' }
    $ch = $hostObj.Chart
    try {
        $ch.SetSourceData($rng, $xlColumns)
        try { $ch.ChartType = $xlColumnClustered } catch {}
        try {
            $ch.HasTitle = $true
            $title = $ch.ChartTitle
            try { $title.Text = $global:OoxpStrings.chartTitle } finally { Release-Com $title }
        }
        catch {
            Write-Log ("设图表标题失败：" + $_.Exception.Message)
        }
    }
    finally { Release-Com $ch }

    if (Test-Path $excelSaved) { Remove-Item $excelSaved -Force }
    $wb.SaveAs($excelSaved, 51)
    $wb.Close($false)
    Release-Com $hostObj
    Release-Com $rng
    Release-Com $s1
    Release-Com $wb

    $check = Test-XlsxChartSource $excelSaved 'xlsx-chart-source-saved-inspect'
    if (-not $check.ok) {
        Write-XlsxChartZipHint $excelSaved
        throw "Excel 另存的柱形图不合格：count=$($check.chartCount) type=$($check.type) nVal=$($check.nVal) valRef=$($check.valRef) id=$($check.id)"
    }

    Copy-Item -Force $excelSaved $path
    Write-Log "saved excel chart source $path id=$($check.id) valRef=$($check.valRef)"
    return $path
}

# 同理，docx 的 apply 源文件要经 Word 的手。
# 这里走「%TEMP% 里原地 Save() 再复制回 out」而不是直接 SaveAs2：
# SaveAs2 曾经挂住过一次（见 checklist-docx.md 的 O1），原因没查清，
# 而 Save() 这条路一直稳，造源文件不值得冒风险。
function Ensure-DocxWordSource([switch]$Force) {
    $path = Get-OutPath 'docx-word-source.docx'
    if ((Test-Path $path) -and -not $Force) { return $path }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $tmp = Join-Path $tmpDir 'docx-word-source.docx'
    $S = $global:OoxpStrings
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $tmp.Replace('\', '/'); format = 'docx'
        document = @{
            paragraphs = @(
                @{ style = 'Heading1'; text = $S.chapter1 }
                @{ text = $S.para2src }
                @{ text = $S.para3keep }
            )
        }
    }
    Invoke-OoxpRequest $req 'docx-source-create' | Out-Null

    $word = Ensure-MsWord (Ensure-DocxCreate)
    $doc = Open-WordDoc $word $tmp
    $doc.Paragraphs.Item(2).Range.Font.Bold = $true
    Write-Log 'word Save() 原地覆盖 TEMP 里的源文件'
    $doc.Save()
    Close-WordDoc $doc
    Copy-Item -Force $tmp $path
    Write-Log "saved word source $path"
    return $path
}

# 带页眉页脚的 Word 另存样例。create 不写页眉，必须经 Word 的手。
function Ensure-DocxWordHeaderSource([switch]$Force) {
    $path = Get-OutPath 'docx-word-header-source.docx'
    if ((Test-Path $path) -and -not $Force) { return $path }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $tmp = Join-Path $tmpDir 'docx-word-header-source.docx'
    $S = $global:OoxpStrings
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $tmp.Replace('\', '/'); format = 'docx'
        document = @{
            paragraphs = @(
                @{ style = 'Heading1'; text = $S.chapter1 }
                @{ text = $S.para2src }
            )
        }
    }
    Invoke-OoxpRequest $req 'docx-hf-source-create' | Out-Null

    $word = Ensure-MsWord (Ensure-DocxCreate)
    $doc = Open-WordDoc $word $tmp
    $sec = $doc.Sections.Item(1)
    # wdHeaderFooterPrimary = 1
    $hdr = $sec.Headers.Item(1)
    $ftr = $sec.Footers.Item(1)
    $hdr.Range.Text = $S.headerSrc
    $ftr.Range.Text = $S.footerSrc
    Write-Log 'word Save() 原地覆盖 TEMP 里的页眉样例'
    $doc.Save()
    Release-Com $hdr
    Release-Com $ftr
    Release-Com $sec
    Close-WordDoc $doc
    Copy-Item -Force $tmp $path
    Write-Log "saved word header source $path"
    return $path
}

# 两节且第二节默认页眉勾着「链接到前一节」。create 不写多节，必须经 Word 的手。
function Ensure-DocxWordLinkSource([switch]$Force) {
    $path = Get-OutPath 'docx-word-link-source.docx'
    if ((Test-Path $path) -and -not $Force) { return $path }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $tmp = Join-Path $tmpDir 'docx-word-link-source.docx'
    $S = $global:OoxpStrings
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $tmp.Replace('\', '/'); format = 'docx'
        document = @{
            paragraphs = @(
                @{ style = 'Heading1'; text = $S.chapter1 }
                @{ text = $S.para2src }
            )
        }
    }
    Invoke-OoxpRequest $req 'docx-link-source-create' | Out-Null

    $word = Ensure-MsWord (Ensure-DocxCreate)
    $doc = Open-WordDoc $word $tmp
    $p1 = $doc.Paragraphs.Item(1)
    $r = $p1.Range
    $r.Collapse(0) | Out-Null
    # wdSectionBreakNextPage = 2
    $r.InsertBreak(2)
    Release-Com $r
    Release-Com $p1

    $sec1 = $doc.Sections.Item(1)
    $hdr1 = $sec1.Headers.Item(1)
    $hdr1.Range.Text = $S.headerSrc
    $sec2 = $doc.Sections.Item(2)
    $hdr2 = $sec2.Headers.Item(1)
    if (-not [bool]$hdr2.LinkToPrevious) {
        $hdr2.LinkToPrevious = $true
    }
    Write-Log 'word Save() 原地覆盖 TEMP 里的两节链接样例'
    $doc.Save()
    Release-Com $hdr2
    Release-Com $sec2
    Release-Com $hdr1
    Release-Com $sec1
    Close-WordDoc $doc
    Copy-Item -Force $tmp $path
    Write-Log "saved word link source $path"
    return $path
}

# apply 的 pptx 源文件要经 PowerPoint 的手。同样走 TEMP 里 Save() 再拷回。
function Ensure-PptxPptSource([switch]$Force) {
    $path = Get-OutPath 'pptx-ppt-source.pptx'
    if ((Test-Path $path) -and -not $Force) { return $path }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $tmp = Join-Path $tmpDir 'pptx-ppt-source.pptx'
    $S = $global:OoxpStrings
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $tmp.Replace('\', '/'); format = 'pptx'
        presentation = @{
            slides = @(
                @{ shapes = @(@{ text = $S.chapter1 }, @{ text = $S.shape2src }) }
                @{ shapes = @(@{ text = $S.page2keep }) }
            )
        }
    }
    Invoke-OoxpRequest $req 'pptx-source-create' | Out-Null

    $ppt = Ensure-MsPpt (Ensure-PptxCreate)
    $pres = Open-PptPres $ppt $tmp
    $s1 = Get-PptSlide $pres 1
    $box2 = Get-PptShape $s1 '文本框 2'
    # 中间的 TextFrame / TextRange / Font 都得留住再松手，链式点过去会漏。
    $frame2 = $box2.TextFrame
    $range2 = $frame2.TextRange
    $font2 = $range2.Font
    $font2.Bold = $true
    Write-Log 'PowerPoint Save() 原地覆盖 TEMP 里的源文件'
    $pres.Save()
    Close-PptPres $pres
    Release-Com $font2
    Release-Com $range2
    Release-Com $frame2
    Release-Com $box2
    Release-Com $s1
    Copy-Item -Force $tmp $path
    Write-Log "saved ppt source $path"
    return $path
}

# PowerPoint 亲手另存的带图样例：第一页一张 p:pic，另有文本框。
function Ensure-PptxPptPictureSource([switch]$Force) {
    $path = Get-OutPath 'pptx-ppt-picture-source.pptx'
    if ((Test-Path $path) -and -not $Force) { return $path }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $tmp = Join-Path $tmpDir 'pptx-ppt-picture-source.pptx'
    $S = $global:OoxpStrings
    $pngA = Ensure-OoxpPngA
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $tmp.Replace('\', '/'); format = 'pptx'
        presentation = @{
            slides = @(
                @{ shapes = @(@{ text = $S.chapter1 }, @{ text = $S.shape2src }) }
                @{ shapes = @(@{ text = $S.page2keep }) }
            )
        }
    }
    Invoke-OoxpRequest $req 'pptx-pic-source-create' | Out-Null

    $ppt = Ensure-MsPpt (Ensure-PptxCreate)
    $pres = Open-PptPres $ppt $tmp
    $s1 = Get-PptSlide $pres 1
    $shapes = $s1.Shapes
    # LinkToFile=false、SaveWithDocument=true；单位是磅。
    $pic = $shapes.AddPicture($pngA, 0, -1, 80, 60, 160, 120)
    Write-Log 'PowerPoint Save() 原地覆盖 TEMP 里的带图源文件'
    $pres.Save()
    Close-PptPres $pres
    Release-Com $pic
    Release-Com $shapes
    Release-Com $s1
    Copy-Item -Force $tmp $path
    Write-Log "saved ppt picture source $path"
    return $path
}

function Find-OoxpFfmpeg {
    $cmd = Get-Command ffmpeg -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    foreach ($p in @('C:\bin\ffmpeg.bat', 'C:\bin\ffmpeg.exe', 'C:\ffmpeg\bin\ffmpeg.exe')) {
        if (Test-Path $p) { return $p }
    }
    return $null
}

function Ensure-OoxpMp4([switch]$Force) {
    $path = Get-OutPath 'ooxp-clip.mp4'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $probeOut = Get-OutPath 'probe-media.mp4'
    if ((Test-Path $probeOut) -and -not $Force) {
        Copy-Item -Force $probeOut $path
        return $path
    }
    $probe = Join-Path $env:TEMP 'ooxp-probe-media.mp4'
    if ((Test-Path $probe) -and -not $Force) {
        Copy-Item -Force $probe $path
        return $path
    }
    $ff = Find-OoxpFfmpeg
    if (-not $ff) {
        throw '找不到 ffmpeg，无法生成手测用的 MP4'
    }
    Write-Log "ffmpeg 生成 1 秒黑场 $path"
    & $ff -y -f lavfi -i 'color=c=black:s=160x120:d=1' -c:v libx264 -pix_fmt yuv420p -t 1 $path
    if (-not (Test-Path $path)) { throw "ffmpeg 没有写出 $path" }
    return $path
}

function Ensure-OoxpWav([switch]$Force) {
    $path = Get-OutPath 'ooxp-beep.wav'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $sys = 'C:\Windows\Media\Alarm01.wav'
    if (Test-Path $sys) {
        Copy-Item -Force $sys $path
        return $path
    }
    # 最小 PCM WAV，给 insertAudio 用。
    $data = [byte[]]::new(8)
    $riffSize = [BitConverter]::GetBytes([uint32]44)
    $fmtLen = [BitConverter]::GetBytes([uint32]16)
    $pcm = [BitConverter]::GetBytes([uint16]1)
    $mono = [BitConverter]::GetBytes([uint16]1)
    $rate = [BitConverter]::GetBytes([uint32]8000)
    $byteRate = [BitConverter]::GetBytes([uint32]16000)
    $align = [BitConverter]::GetBytes([uint16]2)
    $bits = [BitConverter]::GetBytes([uint16]16)
    $dataLen = [BitConverter]::GetBytes([uint32]8)
    $bytes = New-Object System.Collections.Generic.List[byte]
    $utf8 = [Text.Encoding]::ASCII
    [void]$bytes.AddRange($utf8.GetBytes('RIFF'))
    [void]$bytes.AddRange($riffSize)
    [void]$bytes.AddRange($utf8.GetBytes('WAVE'))
    [void]$bytes.AddRange($utf8.GetBytes('fmt '))
    [void]$bytes.AddRange($fmtLen)
    [void]$bytes.AddRange($pcm)
    [void]$bytes.AddRange($mono)
    [void]$bytes.AddRange($rate)
    [void]$bytes.AddRange($byteRate)
    [void]$bytes.AddRange($align)
    [void]$bytes.AddRange($bits)
    [void]$bytes.AddRange($utf8.GetBytes('data'))
    [void]$bytes.AddRange($dataLen)
    [void]$bytes.AddRange($data)
    [IO.File]::WriteAllBytes($path, $bytes.ToArray())
    return $path
}

# PowerPoint 亲手另存的带视频样例：第一页至少一段 Type=16 媒体。
function Ensure-PptxPptMediaSource([switch]$Force) {
    $path = Get-OutPath 'pptx-ppt-media-source.pptx'
    if ((Test-Path $path) -and -not $Force) { return $path }

    $mp4 = Ensure-OoxpMp4
    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $tmp = Join-Path $tmpDir 'pptx-ppt-media-source.pptx'
    $S = $global:OoxpStrings
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $tmp.Replace('\', '/'); format = 'pptx'
        presentation = @{
            slides = @(@{ shapes = @(@{ text = $S.chapter1 }) })
        }
    }
    Invoke-OoxpRequest $req 'pptx-media-source-create' | Out-Null

    $ppt = Ensure-MsPpt (Ensure-PptxCreate)
    Close-PptPresByPath $ppt $tmp
    $pres = Open-PptPres $ppt $tmp
    $s1 = Get-PptSlide $pres 1
    $shapes = $s1.Shapes
    $vid = $shapes.AddMediaObject2($mp4, 0, -1, 40, 40, 240, 180)
    Write-Log ("AddMediaObject2 Type=$([int]$vid.Type) Name=$($vid.Name)")
    Write-Log 'PowerPoint Save() 原地覆盖 TEMP 里的带媒体源文件'
    $pres.Save()
    Close-PptPres $pres
    Release-Com $vid
    Release-Com $shapes
    Release-Com $s1
    Copy-Item -Force $tmp $path
    Write-Log "saved ppt media source $path"
    return $path
}

function Test-PptxChartSource([string]$pptx, [string]$reqName) {
    $tree = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $pptx.Replace('\', '/'); format = 'pptx'
        scope = @{ charts = $true }
    } $reqName
    $slide = @($tree.slides) | Select-Object -First 1
    $charts = @()
    if ($slide -and ($slide.PSObject.Properties.Name -contains 'charts')) {
        $charts = @($slide.charts)
    }
    $col = $charts | Where-Object { [string]$_.type -eq 'col' } | Select-Object -First 1
    if (-not $col) { $col = $charts | Select-Object -First 1 }
    $nVal = 0
    $id = ''
    $type = ''
    if ($col) {
        $id = [string]$col.id
        $type = [string]$col.type
        $ser = @($col.series) | Select-Object -First 1
        if ($ser) { $nVal = @($ser.values).Count }
    }
    return [pscustomobject]@{
        chartCount = [int]$tree.chartCount
        n = $charts.Count
        type = $type
        id = $id
        nVal = $nVal
        ok = ($tree.chartCount -ge 1) -and ($id -like 'slide:*/chart:*') -and ($nVal -ge 2)
    }
}

function Write-PptxChartZipHint([string]$pptx) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $z = [IO.Compression.ZipFile]::OpenRead($pptx)
    try {
        $hits = @($z.Entries | Where-Object {
            $n = $_.FullName -replace '\\', '/'
            $n -like 'ppt/charts/*' -or $n -like 'ppt/embeddings/*' -or $n -like 'ppt/slides/*'
        })
        Write-Log ("chart zip: " + (($hits | ForEach-Object { $_.FullName }) -join ', '))
    }
    finally { $z.Dispose() }
}

# PowerPoint 亲手另存的带图表样例：第一页一张柱形图，数据在 embedding 里。
function Ensure-PptxPptChartSource([switch]$Force) {
    $path = Get-OutPath 'pptx-ppt-chart-source.pptx'
    if ((Test-Path $path) -and -not $Force) {
        $cached = Test-PptxChartSource $path 'pptx-chart-source-cache-check'
        if ($cached.ok) { return $path }
        Write-Log "缓存的 pptx chart source 不合格（count=$($cached.chartCount) type=$($cached.type) nVal=$($cached.nVal) id=$($cached.id)），重建"
    }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $tmp = Join-Path $tmpDir 'pptx-ppt-chart-source.pptx'
    $S = $global:OoxpStrings
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $tmp.Replace('\', '/'); format = 'pptx'
        presentation = @{
            slides = @(
                @{ shapes = @(@{ text = $S.chapter1 }, @{ text = $S.shape2src }) }
                @{ shapes = @(@{ text = $S.page2keep }) }
            )
        }
    }
    Invoke-OoxpRequest $req 'pptx-chart-source-create' | Out-Null

    $ppt = Ensure-MsPpt (Ensure-PptxCreate)
    $pres = Open-PptPres $ppt $tmp
    $s1 = Get-PptSlide $pres 1
    $shapes = $s1.Shapes
    $xlColumnClustered = 51
    $hostObj = $null
    try {
        try {
            $hostObj = $shapes.AddChart2(-1, $xlColumnClustered, 80, 40, 360, 220)
        }
        catch {
            Write-Log ("AddChart2 报错：" + $_.Exception.Message)
            try {
                $hostObj = $shapes.AddChart($xlColumnClustered, 80, 40, 360, 220)
            }
            catch {
                Write-Log ("AddChart 报错：" + $_.Exception.Message)
            }
        }
    }
    finally { Release-Com $shapes }

    if (-not $hostObj) { throw 'PowerPoint 没有插入幻灯片柱形图' }
    Start-Sleep -Seconds 1
    $ch = $null
    try { $ch = $hostObj.Chart } catch {
        Write-Log ("取 Chart 失败：" + $_.Exception.Message)
    }
    if ($ch) {
        try {
            $ch.HasTitle = $true
            $title = $ch.ChartTitle
            try { $title.Text = $S.chartTitle } finally { Release-Com $title }
        }
        catch {
            Write-Log ("设图表标题失败：" + $_.Exception.Message)
        }
        Release-Com $ch
    }
    Write-Log 'PowerPoint Save() 原地覆盖 TEMP 里的带图表源文件'
    $pres.Save()
    Close-PptPres $pres
    Release-Com $hostObj
    Release-Com $s1
    Copy-Item -Force $tmp $path
    Write-Log "saved ppt chart source $path"

    $check = Test-PptxChartSource $path 'pptx-chart-source-saved-inspect'
    if (-not $check.ok) {
        Write-PptxChartZipHint $path
        throw "PowerPoint 另存的图表源不合格：count=$($check.chartCount) type=$($check.type) nVal=$($check.nVal) id=$($check.id)"
    }
    return $path
}

function Write-OoxpPng([string]$path, [byte[]]$bytes) {
    $dir = Split-Path -Parent $path
    if ($dir) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
    [IO.File]::WriteAllBytes($path, $bytes)
    return $path
}

# 两张 1×1 PNG，换图用第二张。
function Ensure-OoxpPngA([switch]$Force) {
    $path = Get-OutPath 'pic-a.png'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $bytes = [byte[]]@(
        0x89,0x50,0x4E,0x47,0x0D,0x0A,0x1A,0x0A,0x00,0x00,0x00,0x0D,0x49,0x48,0x44,
        0x52,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x01,0x08,0x02,0x00,0x00,0x00,0x90,
        0x77,0x53,0xDE,0x00,0x00,0x00,0x0C,0x49,0x44,0x41,0x54,0x08,0xD7,0x63,0xF8,
        0xCF,0xC0,0x00,0x00,0x03,0x01,0x01,0x00,0x18,0xDD,0x8D,0xB4,0x00,0x00,0x00,
        0x00,0x49,0x45,0x4E,0x44,0xAE,0x42,0x60,0x82
    )
    return Write-OoxpPng $path $bytes
}

function Ensure-OoxpPngB([switch]$Force) {
    $path = Get-OutPath 'pic-b.png'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $bytes = [byte[]]@(
        0x89,0x50,0x4E,0x47,0x0D,0x0A,0x1A,0x0A,0x00,0x00,0x00,0x0D,0x49,0x48,0x44,
        0x52,0x00,0x00,0x00,0x01,0x00,0x00,0x00,0x01,0x08,0x02,0x00,0x00,0x00,0x90,
        0x77,0x53,0xDE,0x00,0x00,0x00,0x0C,0x49,0x44,0x41,0x54,0x08,0xD7,0x63,0x60,
        0x60,0x60,0x00,0x00,0x00,0x04,0x00,0x01,0xA3,0x0A,0x15,0xE3,0x00,0x00,0x00,
        0x00,0x49,0x45,0x4E,0x44,0xAE,0x42,0x60,0x82
    )
    return Write-OoxpPng $path $bytes
}

# 两节、第二节默认页眉链接前节；页眉里有行内图和浮动图。
function Ensure-DocxWordPictureSource([switch]$Force) {
    $path = Get-OutPath 'docx-word-picture-source.docx'
    if ((Test-Path $path) -and -not $Force) { return $path }

    $pngA = Ensure-OoxpPngA
    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $tmp = Join-Path $tmpDir 'docx-word-picture-source.docx'
    $S = $global:OoxpStrings
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $tmp.Replace('\', '/'); format = 'docx'
        document = @{
            paragraphs = @(
                @{ style = 'Heading1'; text = $S.chapter1 }
                @{ text = $S.para2src }
            )
        }
    }
    Invoke-OoxpRequest $req 'docx-picture-source-create' | Out-Null

    $word = Ensure-MsWord (Ensure-DocxCreate)
    $doc = Open-WordDoc $word $tmp
    $p1 = $doc.Paragraphs.Item(1)
    $r = $p1.Range
    $r.Collapse(0) | Out-Null
    $r.InsertBreak(2)
    Release-Com $r
    Release-Com $p1

    $sec1 = $doc.Sections.Item(1)
    $hdr1 = $sec1.Headers.Item(1)
    $hdr1.Range.Text = $S.headerSrc
    $inlines = $hdr1.Range.InlineShapes
    $inline = $inlines.AddPicture($pngA, $false, $true)
    $shapes = $hdr1.Shapes
    $float = $shapes.AddPicture($pngA, $false, $true, 20, 8, 48, 36)
    $wrap = $float.WrapFormat
    # wdWrapSquare = 0
    $wrap.Type = 0
    $sec2 = $doc.Sections.Item(2)
    $hdr2 = $sec2.Headers.Item(1)
    if (-not [bool]$hdr2.LinkToPrevious) {
        $hdr2.LinkToPrevious = $true
    }
    Write-Log 'word Save() 原地覆盖 TEMP 里的带图页眉样例'
    $doc.Save()
    Release-Com $wrap
    Release-Com $float
    Release-Com $shapes
    Release-Com $inline
    Release-Com $inlines
    Release-Com $hdr2
    Release-Com $sec2
    Release-Com $hdr1
    Release-Com $sec1
    Close-WordDoc $doc
    Copy-Item -Force $tmp $path
    Write-Log "saved word picture source $path"
    return $path
}

# 正文某段同时有行内图和浮动图。
function Ensure-DocxWordBodyPictureSource([switch]$Force) {
    $path = Get-OutPath 'docx-word-body-picture-source.docx'
    if ((Test-Path $path) -and -not $Force) { return $path }

    $pngA = Ensure-OoxpPngA
    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $tmp = Join-Path $tmpDir 'docx-word-body-picture-source.docx'
    $S = $global:OoxpStrings
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $tmp.Replace('\', '/'); format = 'docx'
        document = @{
            paragraphs = @(
                @{ style = 'Heading1'; text = $S.chapter1 }
                @{ text = $S.para2src }
            )
        }
    }
    Invoke-OoxpRequest $req 'docx-body-picture-source-create' | Out-Null

    $word = Ensure-MsWord (Ensure-DocxCreate)
    $doc = Open-WordDoc $word $tmp
    $p2 = $doc.Paragraphs.Item(2)
    $r = $p2.Range
    $inlines = $doc.InlineShapes
    $inline = $inlines.AddPicture($pngA, $false, $true, $r)
    $r2 = $p2.Range
    $inline2 = $inlines.AddPicture($pngA, $false, $true, $r2)
    $float = $inline2.ConvertToShape()
    $wrap = $float.WrapFormat
    $wrap.Type = 0
    Write-Log 'word Save() 原地覆盖 TEMP 里的带图正文样例'
    $doc.Save()
    Release-Com $wrap
    Release-Com $float
    Release-Com $inline2
    Release-Com $inline
    Release-Com $inlines
    Release-Com $r2
    Release-Com $r
    Release-Com $p2
    Close-WordDoc $doc
    Copy-Item -Force $tmp $path
    Write-Log "saved word body picture source $path"
    return $path
}

# 从 inspect 树里按框内第一段文字找框。链接后继框常被 Word 写成空段，对不上文字。
function Find-TreeTextBox($tree, [string]$text) {
    foreach ($p in @($tree.paragraphs)) {
        foreach ($b in @($p.textBoxes)) {
            $paras = @($b.paragraphs)
            if ($paras.Count -and ([string]$paras[0].text -eq $text)) { return $b }
        }
    }
    return $null
}

function Find-TreeTextBoxById($tree, [string]$id) {
    if (-not $id) { return $null }
    foreach ($p in @($tree.paragraphs)) {
        foreach ($b in @($p.textBoxes)) {
            if ($b.id -eq $id) { return $b }
        }
    }
    return $null
}

# 正文有行内框、浮动框、一对链接框，以及组合里的框。
# 框用 ooxp 写成 DrawingML（Word COM 的 AddTextbox 在本机可能落到 VML，inspect 不收）。
# 骨架和收尾都经 Word Save()，组合也由 Word 亲手 Group。
function Ensure-DocxWordTextBoxSource([switch]$Force) {
    $path = Get-OutPath 'docx-word-textbox-source.docx'
    if ((Test-Path $path) -and -not $Force) { return $path }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $skel = Join-Path $tmpDir 'docx-word-textbox-skel.docx'
    $built = Join-Path $tmpDir 'docx-word-textbox-built.docx'
    $linked = Join-Path $tmpDir 'docx-word-textbox-linked.docx'
    $S = $global:OoxpStrings
    $req = @{
        schemaVersion = 1; action = 'create'
        path = $skel.Replace('\', '/'); format = 'docx'
        document = @{
            paragraphs = @(
                @{ style = 'Heading1'; text = $S.chapter1 }
                @{ text = $S.para2src }
                @{ text = $S.txbxLinkPara }
                @{ text = $S.txbxGroupPara }
            )
        }
    }
    Invoke-OoxpRequest $req 'docx-textbox-skel-create' | Out-Null

    $word = Ensure-MsWord (Ensure-DocxCreate)
    $doc = Open-WordDoc $word $skel
    Write-Log 'word Save() 原地覆盖 TEMP 里的文本框骨架'
    $doc.Save()
    Close-WordDoc $doc

    $treeS = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $skel.Replace('\', '/'); format = 'docx'
        scope = @{ from = 1; maxParas = 20 }
    } 'docx-textbox-skel-inspect'
    $paras = @($treeS.paragraphs)
    if ($paras.Count -lt 4) { throw "文本框骨架段落不足 $($paras.Count)" }
    $p2 = $paras[1].id
    $p3 = $paras[2].id
    $p4 = $paras[3].id

    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $skel.Replace('\', '/')
        output = $built.Replace('\', '/')
        ops = @(
            @{ op = 'insertTextBox'; id = $p2; text = $S.txbxInline; cx = 1371600; cy = 548640 }
            @{ op = 'insertTextBox'; id = $p2; text = $S.txbxFloat }
            @{ op = 'insertTextBox'; id = $p3; text = $S.txbxLinkA }
            @{ op = 'insertTextBox'; id = $p3; text = $S.txbxLinkB }
            @{ op = 'insertTextBox'; id = $p4; text = $S.txbxGroup }
        )
    } 'docx-textbox-insert-boxes' | Out-Null

    $treeB = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $built.Replace('\', '/'); format = 'docx'
        scope = @{ from = 1; maxParas = 20; textBoxes = $true; textBoxContent = $true }
    } 'docx-textbox-built-inspect'
    $bInline = Find-TreeTextBox $treeB $S.txbxInline
    $bLinkA = Find-TreeTextBox $treeB $S.txbxLinkA
    $bLinkB = Find-TreeTextBox $treeB $S.txbxLinkB
    if (-not $bInline -or -not $bLinkA -or -not $bLinkB) {
        throw 'insertTextBox 之后找不到行内框或链接框'
    }

    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'apply'
        path = $built.Replace('\', '/')
        output = $linked.Replace('\', '/')
        ops = @(
            @{ op = 'setTextBoxWrap'; id = $bInline.id; kind = 'inline' }
            @{ op = 'setTextBoxLink'; id = $bLinkA.id; next = $bLinkB.id }
        )
    } 'docx-textbox-wrap-link' | Out-Null

    Close-WordDocByPath $word $linked
    $doc2 = Open-WordDoc $word $linked
    $shapes = $doc2.Shapes
    $target = $null
    for ($i = 1; $i -le [int]$shapes.Count; $i++) {
        $sh = $shapes.Item($i)
        $t = ''
        $tf = $null
        $tr = $null
        try {
            $tf = $sh.TextFrame
            if ([int]$tf.HasText -ne 0) {
                $tr = $tf.TextRange
                $t = ([string]$tr.Text).Trim()
            }
        } catch {}
        Release-Com $tr
        Release-Com $tf
        if ($t -eq $S.txbxGroup) {
            $target = $sh
            break
        }
        Release-Com $sh
    }
    if ($target) {
        $rect = $null
        $sr = $null
        $grp = $null
        try {
            $iTarget = $null
            for ($i = 1; $i -le [int]$shapes.Count; $i++) {
                $s = $shapes.Item($i)
                if ($s.Name -eq $target.Name) { $iTarget = $i }
                if ($s -ne $target) { Release-Com $s }
            }
            $rect = $shapes.AddShape(1, 220, 260, 36, 36)
            $arr = New-Object 'System.Object[]' 2
            $arr[0] = $iTarget
            $arr[1] = [int]$shapes.Count
            $sr = $shapes.Range($arr)
            $grp = $sr.Group()
            Write-Log ("word Group 组合框 + 矩形，名=$($grp.Name)")
        } catch {
            Write-Log "word Group 失败：$($_.Exception.Message)"
        }
        Release-Com $grp
        Release-Com $sr
        Release-Com $rect
        Release-Com $target
    } else {
        Write-Log 'word 找不到组合框形状，跳过 Group'
    }
    Write-Log 'word Save() 原地覆盖 TEMP 里的文本框样例'
    $doc2.Save()
    Release-Com $shapes
    Close-WordDoc $doc2
    Copy-Item -Force $linked $path
    Write-Log "saved word textbox source $path"
    return $path
}

# 正文第二段有一条用户脚注和一条用户尾注。骨架用 ooxp create，引用由 Word COM 插入后 Save。
function Ensure-DocxWordNotesSource([switch]$Force) {
    $path = Get-OutPath 'docx-word-notes-source.docx'
    if ((Test-Path $path) -and -not $Force) { return $path }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $skel = Join-Path $tmpDir 'docx-word-notes-skel.docx'
    $S = $global:OoxpStrings
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'create'
        path = $skel.Replace('\', '/'); format = 'docx'
        document = @{
            paragraphs = @(
                @{ style = 'Heading1'; text = $S.chapter1 }
                @{ text = $S.para2src }
                @{ text = $S.para3keep }
            )
        }
    } 'docx-notes-skel-create' | Out-Null

    $word = Ensure-MsWord (Ensure-DocxCreate)
    Close-WordDocByPath $word $skel
    $doc = Open-WordDoc $word $skel
    $missing = [System.Type]::Missing
    $p2 = $doc.Paragraphs.Item(2)
    $rng = $p2.Range
    $rng.Collapse(0)
    [void]$doc.Footnotes.Add($rng, $missing, $S.footnoteSrc)
    Release-Com $rng
    Release-Com $p2
    $p2 = $doc.Paragraphs.Item(2)
    $rng = $p2.Range
    $rng.Collapse(0)
    [void]$doc.Endnotes.Add($rng, $missing, $S.endnoteSrc)
    Release-Com $rng
    Release-Com $p2
    Write-Log 'word Save() 原地覆盖 TEMP 里的脚注尾注样例'
    $doc.Save()
    Close-WordDoc $doc
    Copy-Item -Force $skel $path
    Write-Log "saved word notes source $path"
    return $path
}

function Ensure-DocxWordCommentsSource([switch]$Force) {
    $path = Get-OutPath 'docx-word-comments-source.docx'
    if ((Test-Path $path) -and -not $Force) { return $path }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $skel = Join-Path $tmpDir 'docx-word-comments-skel.docx'
    $S = $global:OoxpStrings
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'create'
        path = $skel.Replace('\', '/'); format = 'docx'
        document = @{
            paragraphs = @(
                @{ style = 'Heading1'; text = $S.chapter1 }
                @{ text = $S.para2src }
                @{ text = $S.para3keep }
            )
        }
    } 'docx-comments-skel-create' | Out-Null

    $word = Ensure-MsWord (Ensure-DocxCreate)
    Close-WordDocByPath $word $skel
    $doc = Open-WordDoc $word $skel
    try { $word.UserName = 'Alice' } catch {}
    try { $word.UserInitials = 'A' } catch {}
    $p1 = $doc.Paragraphs.Item(1)
    $rng1 = $p1.Range
    $c1 = $doc.Comments.Add($rng1, $S.commentA)
    try { $c1.Author = 'Alice' } catch {}
    try { $c1.Initial = 'A' } catch {}
    Release-Com $rng1
    Release-Com $p1
    $p2 = $doc.Paragraphs.Item(2)
    $rng2 = $p2.Range
    $c2 = $doc.Comments.Add($rng2, $S.commentB)
    try { $c2.Author = 'Bob' } catch {}
    try { $c2.Initial = 'B' } catch {}
    Release-Com $rng2
    Release-Com $p2
    $replies = $c1.Replies
    $cr = $c1.Range
    [void]$replies.Add($cr, $S.commentReply)
    Release-Com $cr
    Release-Com $replies
    Release-Com $c2
    Release-Com $c1
    Write-Log 'word Save() 原地覆盖 TEMP 里的批注样例'
    $doc.Save()
    Close-WordDoc $doc
    Copy-Item -Force $skel $path
    Write-Log "saved word comments source $path"
    return $path
}

function Ensure-DocxWordChartsSource([switch]$Force) {
    $path = Get-OutPath 'docx-word-charts-source.docx'
    if ((Test-Path $path) -and -not $Force) { return $path }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $skel = Join-Path $tmpDir 'docx-word-charts-skel.docx'
    $S = $global:OoxpStrings
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'create'
        path = $skel.Replace('\', '/'); format = 'docx'
        document = @{
            paragraphs = @(
                @{ style = 'Heading1'; text = $S.chapter1 }
                @{ text = $S.para2src }
                @{ text = $S.para3keep }
            )
        }
    } 'docx-charts-skel-create' | Out-Null

    $word = Ensure-MsWord (Ensure-DocxCreate)
    Close-WordDocByPath $word $skel
    $doc = Open-WordDoc $word $skel
    $xlColumnClustered = 51
    $xlLine = 4

    function Add-WordChart($document, [int]$paraIndex, [int]$chartType) {
        $p = $document.Paragraphs.Item($paraIndex)
        $rng = $p.Range
        $ish = $null
        $before = [int]$document.InlineShapes.Count
        try {
            $ish = $document.InlineShapes.AddChart2(-1, $chartType, $rng)
        } catch {
            Write-Log ("AddChart2 报错：" + $_.Exception.Message)
            if ([int]$document.InlineShapes.Count -gt $before) {
                $ish = $document.InlineShapes.Item($before + 1)
            } else {
                $ish = $document.InlineShapes.AddChart($chartType, $rng)
            }
        }
        Release-Com $rng
        Release-Com $p
        return $ish
    }

    $ish1 = Add-WordChart $doc 2 $xlColumnClustered
    Start-Sleep -Seconds 1
    $ch1 = $ish1.Chart
    try {
        $ch1.HasTitle = $true
        $ch1.ChartTitle.Text = $S.chartTitle
    } catch {
        Write-Log ("设柱形图标题失败：" + $_.Exception.Message)
    }
    Release-Com $ch1
    Release-Com $ish1

    $nPara = [int]$doc.Paragraphs.Count
    $comboPara = if ($nPara -ge 3) { 3 } else { $nPara }
    $ish2 = Add-WordChart $doc $comboPara $xlColumnClustered
    Start-Sleep -Seconds 1
    $ch2 = $ish2.Chart
    $converted = $false
    try {
        $s2 = $ch2.SeriesCollection(2)
        $s2.ChartType = $xlLine
        $converted = $true
        Release-Com $s2
        Write-Log '已把第二系列改成折线（SeriesCollection）'
    } catch {
        Write-Log ("SeriesCollection(2) 失败：" + $_.Exception.Message)
        try {
            $s2 = $ch2.FullSeriesCollection(2)
            $s2.ChartType = $xlLine
            $converted = $true
            Release-Com $s2
            Write-Log '已把第二系列改成折线（FullSeriesCollection）'
        } catch {
            Write-Log ("FullSeriesCollection(2) 失败：" + $_.Exception.Message)
        }
    }
    if (-not $converted) {
        Write-Log '组合图样例没转成 combo，Ch7 可能失败'
    }
    Release-Com $ch2
    Release-Com $ish2

    Write-Log 'word Save() 原地覆盖 TEMP 里的图表样例'
    $doc.Save()
    Close-WordDoc $doc
    Copy-Item -Force $skel $path
    Write-Log "saved word charts source $path"
    return $path
}

function Inject-PivotSourceOnStandaloneCol([string]$docx) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $tmp = "$docx.inject.zip"
    Copy-Item -Force $docx $tmp
    $z = [IO.Compression.ZipFile]::Open($tmp, 'Update')
    try {
        $targets = @($z.Entries | Where-Object { $_.FullName -replace '\\', '/' -like 'word/charts/chart*.xml' })
        foreach ($entry in $targets) {
            $sr = New-Object IO.StreamReader($entry.Open())
            $xml = $sr.ReadToEnd()
            $sr.Dispose()
            $isCol = $xml.Contains('<c:barChart') -and $xml.Contains('val="col"')
            $isCombo = $xml.Contains('<c:lineChart') -or $xml.Contains('<c:doughnutChart') -or $xml.Contains('<c:radarChart') -or $xml.Contains('<c:bubbleChart') -or $xml.Contains('<c:stockChart')
            if (-not $isCol -or $isCombo -or $xml.Contains('pivotSource')) { continue }
            $patched = $xml -replace '<c:chartSpace([^>]*)>', '<c:chartSpace$1><c:pivotSource><c:name>[0]!Pivot</c:name><c:fmtId val="0"/></c:pivotSource>'
            if ($patched -eq $xml) { continue }
            $name = $entry.FullName
            $entry.Delete()
            $nw = $z.CreateEntry($name)
            $sw = New-Object IO.StreamWriter($nw.Open())
            $sw.Write($patched)
            $sw.Dispose()
            Write-Log "已向 $name 写入 pivotSource"
            break
        }
    }
    finally { $z.Dispose() }
    Move-Item -Force $tmp $docx
}

# 源文件由 Word AddChart2 另存圆环、雷达、气泡、柱线组合、股价，再向一张独立柱形写入 pivotSource。
function Ensure-DocxWordChartsExtSource([switch]$Force) {
    $path = Get-OutPath 'docx-word-charts-ext-source.docx'
    if ((Test-Path $path) -and -not $Force) { return $path }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $skel = Join-Path $tmpDir 'docx-word-charts-ext-skel.docx'
    $S = $global:OoxpStrings
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'create'
        path = $skel.Replace('\', '/'); format = 'docx'
        document = @{
            paragraphs = @(
                @{ style = 'Heading1'; text = $S.chapter1 }
                @{ text = $S.para2src }
                @{ text = $S.para3keep }
                @{ text = $S.chartTitle }
                @{ text = $S.chartSer1 }
                @{ text = $S.chartSer2 }
                @{ text = $S.page2 }
            )
        }
    } 'docx-charts-ext-skel-create' | Out-Null

    $word = Ensure-MsWord (Ensure-DocxCreate)
    Close-WordDocByPath $word $skel
    $doc = Open-WordDoc $word $skel

    $xlDoughnut = -4120
    $xlRadarMarkers = 81
    $xlBubble = 15
    $xlColumnClustered = 51
    $xlLine = 4
    $xlStockHLC = 88

    function Add-WordChart($document, [int]$paraIndex, [int]$chartType) {
        $p = $document.Paragraphs.Item($paraIndex)
        $rng = $p.Range
        $ish = $null
        $before = [int]$document.InlineShapes.Count
        try {
            $ish = $document.InlineShapes.AddChart2(-1, $chartType, $rng)
        } catch {
            Write-Log ("AddChart2($chartType) 报错：" + $_.Exception.Message)
            if ([int]$document.InlineShapes.Count -gt $before) {
                $ish = $document.InlineShapes.Item($before + 1)
            } else {
                try { $ish = $document.InlineShapes.AddChart($chartType, $rng) } catch {
                    Write-Log ("AddChart($chartType) 也失败：" + $_.Exception.Message)
                }
            }
        }
        Release-Com $rng
        Release-Com $p
        return $ish
    }

    $specs = @(
        @{ i = 2; t = $xlDoughnut; n = 'doughnut' }
        @{ i = 3; t = $xlRadarMarkers; n = 'radar' }
        @{ i = 4; t = $xlBubble; n = 'bubble' }
        @{ i = 5; t = $xlColumnClustered; n = 'combo' }
        @{ i = 6; t = $xlStockHLC; n = 'stock' }
        @{ i = 7; t = $xlColumnClustered; n = 'pivot-col' }
    )
    foreach ($spec in $specs) {
        $ish = Add-WordChart $doc ([int]$spec.i) ([int]$spec.t)
        if (-not $ish) {
            Write-Log "未插入 $($spec.n)"
            continue
        }
        Start-Sleep -Seconds 1
        if ($spec.n -eq 'combo') {
            $ch = $ish.Chart
            $converted = $false
            try {
                $s2 = $ch.SeriesCollection(2)
                $s2.ChartType = $xlLine
                $converted = $true
                Release-Com $s2
            } catch {
                try {
                    $s2 = $ch.FullSeriesCollection(2)
                    $s2.ChartType = $xlLine
                    $converted = $true
                    Release-Com $s2
                } catch {
                    Write-Log ("组合图第二系列改折线失败：" + $_.Exception.Message)
                }
            }
            Release-Com $ch
            if (-not $converted) { Write-Log '组合图样例没转成 combo，Cx5/Cx6 可能失败' }
        }
        Release-Com $ish
        Write-Log "已插入 $($spec.n)"
    }

    Write-Log 'word Save() 原地覆盖 TEMP 里的扩展图表样例'
    $doc.Save()
    Close-WordDoc $doc
    Copy-Item -Force $skel $path
    Inject-PivotSourceOnStandaloneCol $path
    Write-Log "saved word charts ext source $path"
    return $path
}

function Find-OfficeSmartArtLayout($app, [string[]]$uniqueIds, [string[]]$nameNeedles) {
    $layouts = $app.SmartArtLayouts
    try {
        foreach ($uid in $uniqueIds) {
            try {
                $lay = $layouts.Item($uid)
                if ($null -ne $lay) {
                    Write-Log "SmartArt 版式按 uniqueId 命中 $uid"
                    return $lay
                }
            } catch {}
        }
        $n = [int]$layouts.Count
        Write-Log "SmartArtLayouts.Count=$n，改按名称找：$($nameNeedles -join ',')"
        for ($i = 1; $i -le $n; $i++) {
            $lay = $layouts.Item($i)
            $name = ''
            try { $name = [string]$lay.Name } catch {}
            $hit = $false
            foreach ($needle in $nameNeedles) {
                if ($name -like "*$needle*") { $hit = $true; break }
            }
            if ($hit) {
                Write-Log "SmartArt 版式按名称命中 i=$i name=$name"
                return $lay
            }
            Release-Com $lay
        }
        throw "找不到 SmartArt 版式（needles=$($nameNeedles -join ','))"
    }
    finally { Release-Com $layouts }
}

function Add-WordSmartArt($document, $layout, [int]$paraIndex, [double]$top) {
    $p = $document.Paragraphs.Item($paraIndex)
    $rng = $p.Range
    $shapes = $document.Shapes
    try {
        return $shapes.AddSmartArt($layout, 50, $top, 400, 200, $rng)
    }
    finally {
        Release-Com $rng
        Release-Com $p
        Release-Com $shapes
    }
}

function Set-WordSmartArtTexts($shp, [string[]]$texts) {
    $sa = $shp.SmartArt
    try {
        $nodes = $sa.AllNodes
        try {
            $n = [int]$nodes.Count
            $limit = [Math]::Min($n, $texts.Count)
            for ($i = 0; $i -lt $limit; $i++) {
                $node = $nodes.Item($i + 1)
                try {
                    $tf = $node.TextFrame2
                    try {
                        $tr = $tf.TextRange
                        try { $tr.Text = $texts[$i] } finally { Release-Com $tr }
                    } finally { Release-Com $tf }
                } finally { Release-Com $node }
            }
        } finally { Release-Com $nodes }
    } finally { Release-Com $sa }
}

# 源文件由 Word AddSmartArt 另存列表型 + 循环型。不要用 create 产物当唯一样例。
function Ensure-DocxWordSmartArtSource([switch]$Force) {
    $path = Get-OutPath 'docx-word-smartart-source.docx'
    if ((Test-Path $path) -and -not $Force) { return $path }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $skel = Join-Path $tmpDir 'docx-word-smartart-skel.docx'
    $S = $global:OoxpStrings
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'create'
        path = $skel.Replace('\', '/'); format = 'docx'
        document = @{
            paragraphs = @(
                @{ style = 'Heading1'; text = $S.chapter1 }
                @{ text = $S.para2src }
                @{ text = $S.para3keep }
            )
        }
    } 'docx-smartart-skel-create' | Out-Null

    $word = Ensure-MsWord (Ensure-DocxCreate)
    Close-WordDocByPath $word $skel
    $doc = Open-WordDoc $word $skel

    $listLay = Find-OfficeSmartArtLayout $word @(
        'urn:microsoft.com/office/officeart/2005/8/layout/default'
    ) @('基本块列表', 'Basic Block List', '块列表', 'Block List')
    $cycleLay = Find-OfficeSmartArtLayout $word @(
        'urn:microsoft.com/office/officeart/2005/8/layout/cycle2'
    ) @('基本循环', 'Basic Cycle', '循环', 'Cycle')

    $ish1 = Add-WordSmartArt $doc $listLay 2 80
    Start-Sleep -Seconds 1
    Set-WordSmartArtTexts $ish1 @($S.saItem1, $S.saItem2, $S.saItem3)
    Release-Com $ish1
    Release-Com $listLay

    $ish2 = Add-WordSmartArt $doc $cycleLay 3 320
    Start-Sleep -Seconds 1
    Release-Com $ish2
    Release-Com $cycleLay

    Write-Log 'word Save() 原地覆盖 TEMP 里的 SmartArt 样例'
    $doc.Save()
    Close-WordDoc $doc
    Copy-Item -Force $skel $path
    Write-Log "saved word smartart source $path"
    return $path
}

function Test-PptxSmartArtSource([string]$pptx, [string]$reqName) {
    $tree = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $pptx.Replace('\', '/'); format = 'pptx'
        scope = @{ smartArt = $true }
    } $reqName
    $slide = @($tree.slides) | Select-Object -First 1
    $arts = @()
    if ($slide -and ($slide.PSObject.Properties.Name -contains 'smartArts')) {
        $arts = @($slide.smartArts)
    }
    $first = $arts | Select-Object -First 1
    $texts = @()
    if ($first) {
        foreach ($p in @($first.points)) {
            $texts += [string]$p.text
        }
    }
    $S = $global:OoxpStrings
    $hit = 0
    foreach ($t in @($S.saItem1, $S.saItem2, $S.saItem3)) {
        if ($texts -contains $t) { $hit++ }
    }
    return [pscustomobject]@{
        smartArtCount = [int]$tree.smartArtCount
        n = $arts.Count
        id = if ($first) { [string]$first.id } else { '' }
        hit = $hit
        ok = ($tree.smartArtCount -ge 1) -and ([string]$first.id -like 'slide:*/smartdiagram:*') -and ($hit -ge 3)
    }
}

# PowerPoint 亲手另存的列表型 SmartArt。不要用库 create 产物当唯一样例。
function Ensure-PptxPptSmartArtSource([switch]$Force) {
    $path = Get-OutPath 'pptx-ppt-smartart-source.pptx'
    if ((Test-Path $path) -and -not $Force) {
        $cached = Test-PptxSmartArtSource $path 'pptx-smartart-source-cache-check'
        if ($cached.ok) { return $path }
        Write-Log "缓存的 pptx SmartArt source 不合格（count=$($cached.smartArtCount) id=$($cached.id) hit=$($cached.hit)），重建"
    }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $tmp = Join-Path $tmpDir 'pptx-ppt-smartart-source.pptx'
    $S = $global:OoxpStrings
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'create'
        path = $tmp.Replace('\', '/'); format = 'pptx'
        presentation = @{
            slides = @(
                @{ shapes = @(@{ text = $S.chapter1 }, @{ text = $S.shape2src }) }
                @{ shapes = @(@{ text = $S.page2keep }) }
            )
        }
    } 'pptx-smartart-source-create' | Out-Null

    $ppt = Ensure-MsPpt (Ensure-PptxCreate)
    $pres = Open-PptPres $ppt $tmp
    $s1 = Get-PptSlide $pres 1
    $listLay = Find-OfficeSmartArtLayout $ppt @(
        'urn:microsoft.com/office/officeart/2005/8/layout/default'
    ) @('基本块列表', 'Basic Block List', '块列表', 'Block List')
    $shapes = $s1.Shapes
    $hostObj = $null
    try {
        $hostObj = $shapes.AddSmartArt($listLay, 80, 40, 400, 220)
    }
    catch {
        Write-Log ("AddSmartArt 报错：" + $_.Exception.Message)
    }
    finally { Release-Com $shapes }
    if (-not $hostObj) { throw 'PowerPoint 没有插入幻灯片 SmartArt' }
    Start-Sleep -Seconds 1
    Set-WordSmartArtTexts $hostObj @($S.saItem1, $S.saItem2, $S.saItem3)
    Write-Log 'PowerPoint Save() 原地覆盖 TEMP 里的 SmartArt 源文件'
    $pres.Save()
    Close-PptPres $pres
    Release-Com $hostObj
    Release-Com $s1
    Release-Com $listLay
    Copy-Item -Force $tmp $path
    Write-Log "saved ppt smartart source $path"
    return $path
}

function Test-PptxNotesSource([string]$pptx, [string]$reqName) {
    $tree = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $pptx.Replace('\', '/'); format = 'pptx'
        scope = @{ notes = $true }
    } $reqName
    $slide = @($tree.slides) | Select-Object -First 1
    $has = $false
    if ($slide -and ($slide.PSObject.Properties.Name -contains 'hasNotes')) {
        $has = [bool]$slide.hasNotes
    }
    $notes = $null
    if ($slide -and ($slide.PSObject.Properties.Name -contains 'notes')) { $notes = $slide.notes }
    $id = ''
    $texts = @()
    if ($notes -and ($notes -ne $null) -and ($notes.GetType().Name -ne 'DBNull')) {
        try { $id = [string]$notes.id } catch { $id = '' }
        foreach ($sh in @($notes.shapes)) {
            $texts += [string]$sh.text
        }
    }
    $S = $global:OoxpStrings
    $hit = $false
    foreach ($t in $texts) {
        if (([string]$t).Trim() -eq $S.notesSrc) { $hit = $true }
    }
    return [pscustomobject]@{
        hasNotes = $has
        id = $id
        hit = $hit
        ok = $has -and ($id -eq 'slide:1/notes') -and $hit
    }
}

# PowerPoint 亲手另存：第一页有演讲者备注。不要用库 create 产物当带备注的唯一样例。
function Ensure-PptxPptNotesSource([switch]$Force) {
    $path = Get-OutPath 'pptx-ppt-notes-source.pptx'
    if ((Test-Path $path) -and -not $Force) {
        $cached = Test-PptxNotesSource $path 'pptx-notes-source-cache-check'
        if ($cached.ok) { return $path }
        Write-Log "缓存的 pptx 备注 source 不合格（hasNotes=$($cached.hasNotes) id=$($cached.id) hit=$($cached.hit)），重建"
    }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $tmp = Join-Path $tmpDir 'pptx-ppt-notes-source.pptx'
    $S = $global:OoxpStrings
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'create'
        path = $tmp.Replace('\', '/'); format = 'pptx'
        presentation = @{
            slides = @(
                @{ shapes = @(@{ text = $S.chapter1 }, @{ text = $S.shape2src }) }
                @{ shapes = @(@{ text = $S.page2keep }) }
            )
        }
    } 'pptx-notes-source-create' | Out-Null

    $ppt = Ensure-MsPpt (Ensure-PptxCreate)
    $pres = Open-PptPres $ppt $tmp
    $s1 = Get-PptSlide $pres 1
    $page = $s1.NotesPage
    $shapes = $page.Shapes
    $written = $false
    try {
        $n = [int]$shapes.Count
        for ($i = 1; $i -le $n; $i++) {
            $shp = $shapes.Item($i)
            try {
                $ph = $null
                try { $ph = $shp.PlaceholderFormat } catch { $ph = $null }
                if ($null -eq $ph) { continue }
                try {
                    if ([int]$ph.Type -eq 2) {
                        $tf = $shp.TextFrame
                        try {
                            $tr = $tf.TextRange
                            try { $tr.Text = $S.notesSrc; $written = $true } finally { Release-Com $tr }
                        } finally { Release-Com $tf }
                    }
                } finally { Release-Com $ph }
            } finally { Release-Com $shp }
            if ($written) { break }
        }
    } finally { Release-Com $shapes }
    Release-Com $page
    if (-not $written) { throw 'PowerPoint 没有写下演讲者备注' }
    Write-Log 'PowerPoint Save() 原地覆盖 TEMP 里的备注源文件'
    $pres.Save()
    Close-PptPres $pres
    Release-Com $s1
    Copy-Item -Force $tmp $path
    Write-Log "saved ppt notes source $path"
    return $path
}

function Test-PptxCommentsSource([string]$pptx, [string]$reqName) {
    $tree = Invoke-OoxpTree @{
        schemaVersion = 1; action = 'inspect'
        path = $pptx.Replace('\', '/'); format = 'pptx'
        scope = @{ comments = $true }
    } $reqName
    $S = $global:OoxpStrings
    $count = 0
    try { $count = [int]$tree.commentCount } catch { $count = 0 }
    $slide = @($tree.slides) | Select-Object -First 1
    $comments = @()
    if ($slide -and ($slide.PSObject.Properties.Name -contains 'comments')) {
        $comments = @($slide.comments)
    }
    $texts = @()
    $replyHit = $false
    foreach ($c in $comments) {
        $texts += [string]$c.text
        $id = [string]$c.id
        if ($id -notlike 'slide:1/comment:{*}') { continue }
        foreach ($r in @($c.replies)) {
            if (([string]$r.text).Trim() -eq $S.commentReply) { $replyHit = $true }
        }
    }
    $hitA = $texts -contains $S.commentA
    $hitB = $texts -contains $S.commentB
    return [pscustomobject]@{
        commentCount = $count
        hitA = $hitA
        hitB = $hitB
        replyHit = $replyHit
        ok = ($count -ge 2) -and $hitA -and $hitB -and $replyHit
    }
}

# PowerPoint 亲手另存：第一页两条线程化批注，其中一条带回复。不要用库 create 产物当带批注的唯一样例。
function Ensure-PptxPptCommentSource([switch]$Force) {
    $path = Get-OutPath 'pptx-ppt-comments-source.pptx'
    if ((Test-Path $path) -and -not $Force) {
        $cached = Test-PptxCommentsSource $path 'pptx-comments-source-cache-check'
        if ($cached.ok) { return $path }
        Write-Log "缓存的 pptx 批注 source 不合格（count=$($cached.commentCount) a=$($cached.hitA) b=$($cached.hitB) reply=$($cached.replyHit)），重建"
    }

    $tmpDir = Join-Path $env:TEMP 'ooxp-office'
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $tmp = Join-Path $tmpDir 'pptx-ppt-comments-source.pptx'
    $S = $global:OoxpStrings
    Invoke-OoxpRequest @{
        schemaVersion = 1; action = 'create'
        path = $tmp.Replace('\', '/'); format = 'pptx'
        presentation = @{
            slides = @(
                @{ shapes = @(@{ text = $S.chapter1 }, @{ text = $S.shape2src }) }
                @{ shapes = @(@{ text = $S.page2keep }) }
            )
        }
    } 'pptx-comments-source-create' | Out-Null

    $ppt = Ensure-MsPpt (Ensure-PptxCreate)
    $pres = Open-PptPres $ppt $tmp
    $s1 = Get-PptSlide $pres 1
    $comments = $s1.Comments
    $c1 = $null
    $c2 = $null
    $replies = $null
    $r1 = $null
    try {
        $c1 = $comments.Add(72, 72, 'Alice', 'A', $S.commentA)
        $c2 = $comments.Add(144, 144, 'Bob', 'B', $S.commentB)
        $replies = $c1.Replies
        $r1 = $replies.Add(72, 90, 'Alice', 'A', $S.commentReply)
    } finally {
        Release-Com $r1
        Release-Com $replies
        Release-Com $c2
        Release-Com $c1
        Release-Com $comments
    }
    Write-Log 'PowerPoint Save() 原地覆盖 TEMP 里的批注源文件'
    $pres.Save()
    Close-PptPres $pres
    Release-Com $s1
    Copy-Item -Force $tmp $path
    Write-Log "saved ppt comments source $path"
    return $path
}
