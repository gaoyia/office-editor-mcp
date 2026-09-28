# OPC 层

状态：已实现。零脏保存由库测试覆盖三种格式。Office 打开 create / apply 产物见三份清单。领域 JSON 没有「只保存不改」的动作，清单 O1 已放弃。

OPC 指 Open Packaging Conventions：文件是一个 ZIP，里面有部件、内容类型和关系。ECMA-376 §9 规定 Office 文档用 ZIP 作物理包，且不使用 interleaving。

## 打开

打开时把整个 ZIP 读入内存，为每个条目记住部件名（OPC 规则下的路径）、未压缩字节，以及它在原 ZIP 里的下标。原始字节整份留着，保存时未改条目直接按下标把原压缩流搬过去，压缩方法、时间戳、extra 字段自然跟着一起走，不必逐项抄录。

部件名按 OPC 规则规范化：以 `/` 开头，正斜杠，没有 `.` 或 `..` 段。ZIP 里写成 `xl/workbook.xml` 的，对内统一成 `/xl/workbook.xml`。

必须存在：

- `/[Content_Types].xml`
- `/_rels/.rels`

缺任何一个就返回 `OpcOpen`。其他部件按关系去找，不靠猜固定路径。xlsx 的入口是根关系里 `Type` 以 `/officeDocument` 结尾的那条，目标通常是 `/xl/workbook.xml`。Office 实际文件也允许入口不在这个常见路径，所以必须跟关系走。

## 内容类型

`[Content_Types].xml` 里有 `Default`（按扩展名）和 `Override`（按部件路径）。读部件时用这两张表决定 Content Type。写新部件时必须补上对应条目。没改过的 `[Content_Types].xml` 按原字节写回。

## 关系

关系文件在源部件同级的 `_rels` 目录，文件名为 `源文件名.rels`。包根的关系文件是 `/_rels/.rels`。

读关系时保留每条 `Id`、`Type`、`Target`、可选的 `TargetMode`，以及不认识的属性。apply 若删除某个部件，必须同时删掉指向它的关系。create 只写自己生成的那几条关系，不伪造 Office 专有扩展关系。

## 读与改

按规范化路径取出未压缩字节。调用方（xlsx / docx 模块）改完后把新字节交还给 opc，并标记该路径为脏。同一路径多次标记，以最后一次字节为准。

禁止在 opc 层解析 SpreadsheetML 语义。opc 不认识单元格、段落、形状。

## 保存

写回规则：

1. 未标记为脏的条目：原压缩流整块搬过去，压缩方法、未压缩字节、extra、时间戳都与打开时一致，字节级相同。
2. 脏条目：用新字节重写。压缩方法第一期固定为 deflate，级别不要求与原文件相同。
3. 新建条目：追加到中央目录末尾。
4. 删除条目：从包中去掉，并同步内容类型 Override 与相关关系。
5. 未改条目在中央目录中的相对顺序保持打开时的顺序。新建条目排在它们后面。

不要对未改 XML 做重新缩进、重排属性、改 XML 声明、改换行符。Office 生成的 XML 常带特定声明和空元素写法，重排容易造成「没改的区域看起来不一样」。

## 未知部件与未知关系

ECMA-376 §9.1.4 与 §9.1.7：未知部件和未知关系在消费时不得导致失败。本库打开时保留它们，保存时若未标记删除就原样写回。这是「手术刀」策略在包这一层的体现。

## create 时的最小包（xlsx）

xlsx create 生成 Transitional 包，命名空间用 Office 默认的 2006 那套，不用 Strict 的 `purl.oclc.org`。

必有部件：

| 路径 | 作用 |
| --- | --- |
| `/[Content_Types].xml` | 内容类型 |
| `/_rels/.rels` | 指向 Workbook，以及可选的核心属性 |
| `/xl/workbook.xml` | 工作表名单 |
| `/xl/_rels/workbook.xml.rels` | 指向各 worksheet 和 styles |
| `/xl/worksheets/sheetN.xml` | 每张表 |
| `/xl/styles.xml` | 最小样式表。默认字体颜色写 `<color rgb="FF000000"/>`，不写 `theme`，也不生成 theme 部件 |

可选但 create 要写：

| 路径 | 作用 |
| --- | --- |
| `/docProps/core.xml` | 核心文件属性，dc:title 可空 |
| `/docProps/app.xml` | Application 写成 `ooxp`，Sheet 名单与 workbook 一致 |
| `/docProps/custom.xml` | 仅当 `workbook.props.custom` 非空，或 apply 的 `setCustomProp` 新建了自定义属性 |
| `/xl/drawings/drawingN.xml` | 仅当某张表声明了 `pictures` 或 `charts`，或 `headerFooters[].pictures`，或 apply 插入了文本框 / SmartArt。页眉图走 `drawingHF`，与工作表 drawing 不是同一部件 |
| `/xl/media/imageN.ext` | 仅当某张表声明了 `pictures` 或 `headerFooters[].pictures` |
| `/xl/charts/chartN.xml` | 仅当某张表声明了 `charts` |
| `/xl/diagrams/…` | 仅当 apply 插入了 SmartArt |

Workbook 的 Content Type：

`application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml`

Worksheet：

`application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml`

根关系 `officeDocument` 的 Type：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument`

worksheet 关系 Type：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet`

SpreadsheetML 主命名空间：

`http://schemas.openxmlformats.org/spreadsheetml/2006/main`

apply 的 `setCellStyle` 可能改 `/xl/styles.xml`（复用或追加 `xf`）。apply 的 `setColWidth` / `setRowHeight` / `mergeRange` / `unmergeRange` 只改目标工作表 XML 的 `cols`、`row/@ht` 与 `mergeCells`，不新建部件。apply 的数据验证 op 只改该表 `dataValidations`，不新建部件。apply 的条件格式 op 改该表 `conditionalFormatting`，并可能复用或追加 `/xl/styles.xml` 的 `dxfs`，不新建其它部件。apply 的工作表表格 op 可能新建或删除 `/xl/tables/tableN.xml`（CT `application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml`）。工作表 `.rels` 补一条 table 关系（Type 以 `/table` 结尾），工作表 XML 写 `<tableParts>`。apply 的透视表 op 可能新建或删除 `/xl/pivotTables/pivotTableN.xml`（CT `application/vnd.openxmlformats-officedocument.spreadsheetml.pivotTable+xml`）、`/xl/pivotCache/pivotCacheDefinitionN.xml`（CT `…pivotCacheDefinition+xml`）与 `/xl/pivotCache/pivotCacheRecordsN.xml`（CT `…pivotCacheRecords+xml`）。workbook 写 `<pivotCaches>`，关系 Type 以 `/pivotCacheDefinition` 结尾；缓存定义再指向记录（Type 以 `/pivotCacheRecords` 结尾）；工作表 `.rels` 补一条透视表关系（Type 以 `/pivotTable` 结尾）；透视表部件再写一条指向该缓存定义的关系（Type 同样以 `/pivotCacheDefinition` 结尾，ECMA-376 §12.3.11 / §12.3.12）。不把合计写进 `sheetData`。apply 的定义名 op 只改 `/xl/workbook.xml` 的 `<definedNames>`（ECMA-376 §18.2.5），按 `CT_Workbook` 插在 `externalReferences` 之后、`calcPr` 之前，不新建部件。apply 的 `setPrintSetup` 改目标工作表或图表工作表 XML 的 `pageMargins` / `pageSetup` / `printOptions` / `sheetPr/pageSetUpPr` / `rowBreaks` / `colBreaks`（§18.3.1.62、§18.3.1.63、§18.3.1.70、§18.3.1.3），不新建部件；打印区域仍写 workbook 的表级 `_xlnm.Print_Area`。apply 的 `setSheetView` 改目标表 `sheetViews` 里第一个 `sheetView`（§18.3.1.87、§18.3.1.66），不新建部件。apply 的 `setAutoFilter` 改工作表级 `<autoFilter>`（§18.3.1.2），不改 `/xl/tables/`，不新建部件。apply 插入线程化批注时，可能新建 `/xl/threadedComments/threadedCommentN.xml`（CT `application/vnd.ms-excel.threadedcomments+xml`，根 `ThreadedComments`，命名空间 `http://schemas.microsoft.com/office/spreadsheetml/2018/threadedcomments`）、`/xl/persons/person.xml`（CT `application/vnd.ms-excel.person+xml`，根 `personList`）、占位 `/xl/commentsN.xml`（CT `application/vnd.openxmlformats-officedocument.spreadsheetml.comments+xml`）以及 `/xl/drawings/vmlDrawingN.vml`。工作表 `.rels` 的线程化关系 Type 必须是完整 URI `http://schemas.microsoft.com/office/2017/10/relationships/threadedComment`，不要用 `ends_with("/comments")`。占位 comments 关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments`。VML 关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing`，`[Content_Types].xml` 补 Default `vml` 为 `application/vnd.openxmlformats-officedocument.vmlDrawing`。workbook `.rels` 的 person 关系 Type 为 `http://schemas.microsoft.com/office/2017/10/relationships/person`。工作表 XML 写 `<legacyDrawing r:id="…"/>`。同一张表多条批注共用一个 threadedComments 部件。`create` 本期不写这些部件。apply 插入工作表图时，可能新建 `/xl/drawings/drawingN.xml`（CT `application/vnd.openxmlformats-officedocument.drawing+xml`）、drawing 的 `.rels`，以及 `/xl/media/imageN.*`。工作表 `.rels` 补一条 drawing 关系（Type 以 `/drawing` 结尾），工作表 XML 写 `<drawing r:id="…"/>`。往页眉槽位插图时写 `<drawingHF r:id="…"/>`（ECMA-376 §18.3.1.37），关系 Type 同样以 `/drawing` 结尾，drawing 内容类型相同；不改 `legacyDrawingHF`。drawing 命名空间是 `http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing`（`xdr`）。apply 插入工作表图表时，可能再新建 `/xl/charts/chartN.xml`（CT `application/vnd.openxmlformats-officedocument.drawingml.chart+xml`）及 chart 的 `.rels`；drawing 关系 Type 以 `/chart` 结尾。不写 embedding。apply 插入工作表超链接时，外部链在工作表 `.rels` 上补一条关系（Type `http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink`，`TargetMode` 为 `External`），工作表 XML 写 `<hyperlinks>/<hyperlink r:id="…"/>`（ECMA-376 §18.3.1.47 / §18.3.1.48）。内部位置只写 `@location`，不写关系。删链或改成无 `r:id` 时，该关系无人引用则删除。不新建部件。apply 插入工作表迷你图时，只改该表 XML 的 `extLst` / `x14:sparklineGroups`（MS-XLSX §2.4.8），不新建部件，不写关系。apply 的 `setProtection` 改目标工作表或图表工作表 XML 的 `<sheetProtection>`（§18.3.1.85 / `CT_ChartsheetProtection`），或 `/xl/workbook.xml` 的 `<workbookProtection>`（§18.2.29），按子元素顺序插入，不新建部件，不写哈希。apply 插入工作表文本框时，可能新建或复用 `/xl/drawings/drawingN.xml`；锚点写入 `xdr:sp`（`txBox`）。apply 插入工作表 SmartArt 时，可能再新建 `/xl/diagrams/` 下 data / layout / colors / quickStyle（CT 分别为 `application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml`、`…diagramLayout+xml`、`…diagramColors+xml`、`…diagramQuickStyle+xml`），四条关系写在 **drawing** `.rels` 上（`diagramData` / `diagramLayout` / `diagramColors` / `diagramQuickStyle`），不要写到工作表 `.rels`。改点时若对不上 diagram drawing 缓存，可能删除该缓存部件。`insertChartSheet` 再新建 `/xl/chartsheets/sheetN.xml`（CT `application/vnd.openxmlformats-officedocument.spreadsheetml.chartsheet+xml`），workbook 关系 Type 以 `/chartsheet` 结尾；drawing 上写一张 `absoluteAnchor` 的 `xdr:graphicFrame`。`insertSheet` 新建 `/xl/worksheets/sheetN.xml`（CT `application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml`），workbook 关系 Type 以 `/worksheet` 结尾。`setCellStyle` 改 styles 部件（关系 Type 以 `/styles` 结尾，常见 `/xl/styles.xml`）；没有该部件则按 create 的最小样式表新建，并补 Override。不重排未点名的 `xf`。create 声明了 `sheets[].pictures` 或 `sheets[].charts` 时按同一套规则写 drawing、media 与 chart 部件。create 声明了 `sheets[].headerFooters` 时按 `setHeaderFooterText` / 页眉 `insertPicture` 写工作表 `headerFooter` 与可选 `drawingHF`，不写 `legacyDrawingHF`，也不写图表工作表。`setProps` / `setCustomProp` 可能新建 `/docProps/core.xml`、`/docProps/app.xml`、`/docProps/custom.xml`（三种格式相同）。core 关系 Type 为 `http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties`，CT `application/vnd.openxmlformats-package.core-properties+xml`。app 关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties`，CT `application/vnd.openxmlformats-officedocument.extended-properties+xml`。custom 关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/custom-properties`，CT `application/vnd.openxmlformats-officedocument.custom-properties+xml`。`deleteCustomProp` 删光后去掉 custom 部件、根关系和 Override。定位这些部件用根 `.rels` 的 Type 后缀，不要写死路径。

## create 时的最小包（docx）

docx create 同样生成 Transitional 包，命名空间用 2006 那套。

必有部件：

| 路径 | 作用 |
| --- | --- |
| `/[Content_Types].xml` | 内容类型 |
| `/_rels/.rels` | 指向 Main Document，以及可选的核心属性 |
| `/word/document.xml` | 主文档，至少一个 `w:p` |
| `/word/_rels/document.xml.rels` | 指向 styles 与 settings |
| `/word/styles.xml` | 至少含 Normal、Heading1、Heading2、Heading3 |
| `/word/settings.xml` | 最小文档设置。必须含 `compatibilityMode`，`val` 为 `15`（MS-DOCX §2.3.5），否则 Word 按兼容模式打开（默认 12）。给了 `document.protection` 或 apply `setProtection` 时还可含 `documentProtection` / `writeProtection` |

可选但 create 要写：

| 路径 | 作用 |
| --- | --- |
| `/docProps/core.xml` | 核心文件属性 |
| `/docProps/app.xml` | Application 写成 `ooxp` |
| `/docProps/custom.xml` | 仅当 `document.props.custom` 非空，或 apply 的 `setCustomProp` 新建了自定义属性 |
| `/word/header1.xml` | 仅当 `document.headers` 含默认页眉 |
| `/word/footer1.xml` | 仅当 `document.footers` 含默认页脚 |
| `/word/media/imageN.ext` | 仅当段落或页眉页脚声明了 `pictures` |
| `/word/diagrams/…` | 仅当段落或页眉页脚声明了 `smartArts` |

apply 插入脚注或尾注时，可能新建 `/word/footnotes.xml` 或 `/word/endnotes.xml`（CT 分别为 `…footnotes+xml` / `…endnotes+xml`），并在主文档 `.rels` 与 `[Content_Types].xml` 上补关系与 Override。apply 插入批注时，可能新建 `/word/comments.xml`（CT `…comments+xml`）、`/word/commentsExtended.xml`（CT `application/vnd.openxmlformats-officedocument.wordprocessingml.commentsExtended+xml`）、`/word/people.xml`（CT `application/vnd.openxmlformats-officedocument.wordprocessingml.people+xml`）。`commentsExtended` 必须用这条 CT：Word 16.0 把旧的 `application/vnd.ms-word.commentsExtended+xml` 当成损坏，打开失败。`commentsIds` / `commentsExtensible` 只在原包已有时补条目，create 与默认插入都不新建这两份。`replyComment` 还会在父批注范围上再套一套范围标记，因此可能脏主文档或页眉页脚、注部件。apply 插入图表时，可能新建 `/word/charts/chartN.xml`（CT `application/vnd.openxmlformats-officedocument.drawingml.chart+xml`）、chart 的 `.rels`，以及 `/word/embeddings/….xlsx`（CT `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`），并在宿主 `.rels` 与 `[Content_Types].xml` 上补关系与 Override。apply 或 create 插入 SmartArt 时，可能新建 `/word/diagrams/` 下的 data / layout / colors / quickStyle（CT 分别为 `application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml`、`…diagramLayout+xml`、`…diagramColors+xml`、`…diagramQuickStyle+xml`），四条关系写在宿主 `.rels` 上（`diagramData` / `diagramLayout` / `diagramColors` / `diagramQuickStyle`）。改点时若对不上 diagram drawing 缓存，可能删除该缓存部件。apply 或 create 插入 OLE 时，可能新建 `/word/embeddings/…`（Office 文件用对应格式的 Content Type；Package 为 `application/vnd.openxmlformats-officedocument.oleObject`）以及占位预览图 `/word/media/imageN.png`。Office 文件的关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/package`（与 Word 16.0 另存一致）；Package 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject`。inspect 两种关系都认。create 给了 `footnote` / `endnote` / `comments` / `charts` / `oleObjects` 时，按对应 insert 同一套规则新建这些部件；也可写 SmartArt 四件套。create 或 apply 给了水印时，可能新建 `/word/headerN.xml`，并把 DrawingML 写入该页眉；图片水印还可新建 `/word/media/…`。

Main Document 的 Content Type：

`application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml`

Style Definitions：

`application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml`

Document Settings：

`application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml`

settings 关系 Type：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings`

`compatibilityMode` 的 `w:uri` 必须是 `http://schemas.microsoft.com/office/word`。

WordprocessingML 主命名空间：

`http://schemas.openxmlformats.org/wordprocessingml/2006/main`

`w14` 命名空间（paraId）：

`http://schemas.microsoft.com/office/word/2010/wordml`

根元素须把 `w14` 列入 `mc:Ignorable`，依据 MS-DOCX §2.2.4。

## create 时的最小包（pptx）

pptx create 同样生成 Transitional 包，命名空间用 2006 那套。不能只写 ECMA-376 §13.3.6 示例里那个仅含 `sldSz` / `notesSz` 的 presentation：PowerPoint 实际打开还要 theme、slideMaster、slideLayout。

必有部件：

| 路径 | 作用 |
| --- | --- |
| `/[Content_Types].xml` | 内容类型 |
| `/_rels/.rels` | 指向 Presentation，以及可选的核心属性 |
| `/ppt/presentation.xml` | 页名单、母版关系、幻灯片尺寸 |
| `/ppt/_rels/presentation.xml.rels` | 指向 slideMaster、各 slide、presProps |
| `/ppt/presProps.xml` | Presentation Properties（§13.3.7，包内应当恰好一份） |
| `/ppt/slides/slideN.xml` | 每一页 |
| `/ppt/slides/_rels/slideN.xml.rels` | 指向 slideLayout1；有图时再加 image 关系 |
| `/ppt/slideLayouts/slideLayout1.xml` | 一张空白版式 |
| `/ppt/slideLayouts/_rels/slideLayout1.xml.rels` | 指向 slideMaster1 |
| `/ppt/slideMasters/slideMaster1.xml` | 一张母版 |
| `/ppt/slideMasters/_rels/slideMaster1.xml.rels` | 指向 theme1 与 slideLayout1 |
| `/ppt/theme/theme1.xml` | 最小主题 |

可选但 create 要写：

| 路径 | 作用 |
| --- | --- |
| `/docProps/core.xml` | 核心文件属性 |
| `/docProps/app.xml` | Application 写成 `ooxp`，Slides 与页数一致 |
| `/docProps/custom.xml` | 仅当 `presentation.props.custom` 非空，或 apply 的 `setCustomProp` 新建了自定义属性 |
| `/ppt/media/imageN.*` | 给了 `slides[].pictures` 时写入 |

第一期 create 不写备注、批注、讲义、`viewProps`、`tableStyles`、动画。apply 的 `setNotesText` 或往备注插图时，可能新建 `/ppt/notesMasters/notesMaster1.xml` 与 `/ppt/notesSlides/notesSlideN.xml`。apply 的 `setHandoutText` 可能新建 `/ppt/handoutMasters/handoutMaster1.xml`。apply 插入现代批注时，可能新建 `/ppt/comments/modernComment_{sldId}_N.xml` 与 `/ppt/authors.xml`。

`presentation.xml` 里：`p:sldMasterIdLst` 一条、`p:sldIdLst` 按页顺序、`p:sldSz` 为 16:9（`cx="12192000"` `cy="6858000"`）、`p:notesSz` 为 `cx="6858000"` `cy="9144000"`。`p:sldId/@id` 从 256 起递增，与领域 id 的 `slide:{n}` 不是一回事。

Presentation 的 Content Type：

`application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml`

Slide：

`application/vnd.openxmlformats-officedocument.presentationml.slide+xml`

Slide Master：

`application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml`

Slide Layout：

`application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml`

Presentation Properties：

`application/vnd.openxmlformats-officedocument.presentationml.presProps+xml`

Theme：

`application/vnd.openxmlformats-officedocument.theme+xml`

根关系 `officeDocument` 的 Type 与 xlsx / docx 相同：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument`

幻灯片关系 Type：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide`

幻灯片母版：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster`

版式：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout`

主题：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme`

Presentation Properties：

`http://schemas.openxmlformats.org/officeDocument/2006/relationships/presProps`

PresentationML 主命名空间：

`http://schemas.openxmlformats.org/presentationml/2006/main`

DrawingML 主命名空间：

`http://schemas.openxmlformats.org/drawingml/2006/main`

apply 插入幻灯片图时，可能新建 `/ppt/media/imageN.*`，并在该页 `.rels` 补一条 image 关系（Type 以 `/image` 结尾）。`p:pic` 写在该页 `p:spTree` 里，位置与尺寸在 `p:spPr/a:xfrm`。create 给了 `slides[].pictures` 时同样写入。

apply 插入幻灯片图表时，可能新建 `/ppt/charts/chartN.xml`（CT `application/vnd.openxmlformats-officedocument.drawingml.chart+xml`）、chart 的 `.rels`，以及 `/ppt/embeddings/Microsoft_Excel_WorksheetN.xlsx`（CT `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`）。该页 `.rels` 补一条 chart 关系（Type 以 `/chart` 结尾）。chart 部件写 `c:externalData r:id="rId1"`，关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/package`。宿主是 `p:graphicFrame`，位置与尺寸在 `p:xfrm`。create 给了 `slides[].charts` 时同样写入。

apply 插入幻灯片 SmartArt 时，可能新建 `/ppt/diagrams/` 下的 data / layout / colors / quickStyle（CT 分别为 `application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml`、`…diagramLayout+xml`、`…diagramColors+xml`、`…diagramQuickStyle+xml`）。该页 `.rels` 补四条 diagram 关系。宿主是 `p:graphicFrame`，`a:graphicData/@uri` 为 diagram URI，`dgm:relIds` 指向这四条。create 给了 `slides[].smartArts` 时同样写入。不写 Fallback 位图，不写 diagram drawing 缓存。改点对不上缓存时删掉该缓存。

apply 新建备注页时，可能写入 `/ppt/notesSlides/notesSlideN.xml`（CT `application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml`）和 `/ppt/notesMasters/notesMaster1.xml`（CT `…notesMaster+xml`）。幻灯片 `.rels` 补一条 notesSlide 关系（Type 以 `/notesSlide` 结尾）。备注页 `.rels` 指向 notesMaster 与对应幻灯片（Type 分别以 `/notesMaster`、`/slide` 结尾）。若 presentation 还没有 notesMaster，补 presentation 关系，并在 `presentation.xml` 的 `p:sldMasterIdLst` 之后写入 `p:notesMasterIdLst`。notesMaster 的 theme 关系指向包内已有 theme，不另建 theme2。备注页结构对照 PowerPoint 16.0 另存样例：`sldImg`、`body`、`sldNum` 三个占位符。往备注插的 `p:pic` 写在 notesSlide 的 `p:spTree` 里，媒体仍在 `/ppt/media/…`。

apply 的 `setHandoutText` 可能写入 `/ppt/handoutMasters/handoutMaster1.xml`（CT `application/vnd.openxmlformats-officedocument.presentationml.handoutMaster+xml`）。presentation `.rels` 补一条 handoutMaster 关系（Type 以 `/handoutMaster` 结尾）。若还没有 `p:handoutMasterIdLst`，写在 `notesMasterIdLst` 之后（没有则写在 `sldMasterIdLst` 之后）。handoutMaster 的 theme 关系指向新复制的 theme 部件（内容与包内已有 theme 相同，常见为 `/ppt/theme/theme2.xml`），不和 slideMaster 共用同一部件。新建部件含 `hdr`、`dt`、`ftr`、`sldNum` 四个占位符，不写 `sldImg`。

apply 插入现代批注时，可能写入 `/ppt/comments/modernComment_{sldId}_N.xml`（CT `application/vnd.ms-powerpoint.comments+xml`，根 `p188:cmLst`，命名空间 `http://schemas.microsoft.com/office/powerpoint/2018/8/main`）和 `/ppt/authors.xml`（CT `application/vnd.ms-powerpoint.authors+xml`，根 `p188:authorLst`）。幻灯片 `.rels` 补一条 comments 关系，Type 必须是完整 URI `http://schemas.microsoft.com/office/2018/10/relationships/comments`，不要用 `ends_with("/comments")`（会误伤 2006 旧式）。presentation `.rels` 补一条 authors 关系，Type 为 `http://schemas.microsoft.com/office/2018/10/relationships/authors`。幻灯片 XML 在 `p:clrMapOvr` 之后写入 `p:extLst` / `p:ext`（uri `{6950BFC3-D8DA-4A85-94F7-54DA5524770B}`）/ `p188:commentRel`。同一页多条批注共用一个 comments 部件。`p188:cm` 子元素顺序为锚点（`pc:sldMkLst`）、`p188:pos`、可选 `p188:replyLst`、`p188:txBody`。`pc:sldMk/@sldId` 取 presentation 该页 `p:sldId` 的无前缀 `id`。不写 ECMA-376 §19.4 的 `/ppt/comments/commentN.xml` 与 `commentAuthors`。

## 验收（第 2 项）

零脏保存：`open` 之后不调用 `set` / `remove`，再 `save`。每个部件的未压缩字节、以及 ZIP 里该条目的 CRC，都与打开时相同。库测试对 Excel 另存的 xlsx、以及 create 写出的 docx / pptx 各跑一遍。

领域 JSON 的三个动作不含原样保存。`apply` 的 `ops` 必须非空，见 [json-contract.md](json-contract.md)。命令行不能做「打开再原样写出」。这一条第一期放弃，原因写在三份清单的 O1。

Office 打开 create / apply 产物、以及 Office 自己另存 create 产物，见 `docs/checklist-xlsx.md`、`docs/checklist-docx.md`、`docs/checklist-pptx.md` 与 `manual-check/office/cases/opc-resave.ps1`。那是打开冒烟，不是零脏保存。
