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
