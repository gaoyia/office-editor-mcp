# 用例要用的输入文件。谁缺谁造，所以任何一个用例都能单独跑。
# 已经在就直接复用：apply 只读源文件、写到另一个 output，源文件不会被改脏。

function Ensure-XlsxCreate([switch]$Force) {
    $path = Get-OutPath 'xlsx-create.xlsx'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $req = Get-RequestTemplate 'xlsx-create' 'xlsx-create.xlsx'
    Invoke-OoxpRequest $req 'xlsx-create' | Out-Null
    return $path
}

function Ensure-DocxCreate([switch]$Force) {
    $path = Get-OutPath 'docx-create.docx'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $req = Get-RequestTemplate 'docx-create' 'docx-create.docx'
    Invoke-OoxpRequest $req 'docx-create' | Out-Null
    return $path
}

function Ensure-PptxCreate([switch]$Force) {
    $path = Get-OutPath 'pptx-create.pptx'
    if ((Test-Path $path) -and -not $Force) { return $path }
    $req = Get-RequestTemplate 'pptx-create' 'pptx-create.pptx'
    Invoke-OoxpRequest $req 'pptx-create' | Out-Null
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
