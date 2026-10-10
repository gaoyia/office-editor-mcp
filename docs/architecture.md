# 架构

状态：xlsx、docx、pptx 第一期已实现，并已通过 Microsoft Office 16.0 人工打开。验收只认官方 Office，不再要求 WPS。

## 产品形态

产品名是 `ooxp`，全称 Office Open XML Patch。一个 Rust 库，外加一个命令行工具。库只认领域 JSON。命令行把 stdin 或文件里的领域 JSON 交给库，再把结果写到 stdout 或磁盘。

本仓库不实现 MCP 服务。外部 MCP 把领域 JSON 原样交给库即可。不许为了信封去改领域 JSON，也不许把动作拆扁成一套 MCP 专用字段。

对齐 Transitional OOXML，也就是 Office 默认存出来的那种包。
ECMA-376 示例里常见的 Strict 命名空间（`http://purl.oclc.org/ooxml/...`）只作对照，create 不按 Strict 生成。

## 三个动作

这三个动作是整套设计的地基，实现时不许改语义。

| 动作 | 真源 | 做什么 |
| --- | --- | --- |
| `inspect` | 已有 ZIP 包里的 XML | 返回带稳定 id 的精简 JSON 树，按调用方给的范围截断 |
| `apply` | 已有 ZIP 包里的 XML | 按 id 打补丁，只改目标节点，其余部件按原字节写回 |
| `create` | 领域 JSON | 从 JSON 生成一份新的完整包。这是唯一允许整包生成的动作 |

禁止把 inspect 树当成完整文档模型，改完之后整包重新生成。apply 的入参只能是操作列表，不能是整棵树。

xlsx 另有只读动作 `exportMedia`：按图 id 把媒体部件原字节写到本地。
xlsx 的 `exportMedia` 认工作表图 id、页眉页脚图 id、OLE id 与 3D 模型 id：图 id 写出 `/xl/media/…`，OLE id 写出 `/xl/embeddings/…`，
3D 模型 id 写出 `/xl/media/…` 的 GLB。
pptx 的 `exportMedia` 认幻灯片图 id、备注图 id、批注内图 id、母版 / 版式图 id、视频 id、音频 id、OLE id 与 3D 模型 id：图 / 视频 / 音频写出 `/ppt/media/…`，OLE 写出 `/ppt/embeddings/…`，
3D 模型 id 写出 `/ppt/media/…` 的 GLB。
docx 的 `exportMedia` 认图 id、OLE id、墨迹画布 id、3D 模型 id 与嵌入字体 id：图写出 `/word/media/…`，OLE 写出 embeddings 部件，
墨迹画布写出 `/word/ink/…`，3D 模型写出 `/word/media/…` 的 GLB，`font:{n}` 写出 `/word/fonts/…`。
docx 与 pptx 另有 `exportChartData`：按图表 id 把 embedding 工作簿原字节写到本地。它们不改上述三个动作的语义，也不把字节塞进 inspect 树。
xlsx 对 `exportChartData` 仍报 `InvalidField`（图表数据就在本工作簿单元格里）。

## 模块

建议用 Cargo workspace，两个 crate。xlsx、docx、pptx 已落地。信封解析在 `json`，按 `format` 分发在 `dispatch`，每种格式自己解析自己的 `scope` 与 `ops`。

```text
crates/ooxp/                 库
  error/                     错误身份、错误码、按语言渲染
    messages/                en.rs、zh_cn.rs；每种语言一个穷尽 match
  opc/                       ZIP、[Content_Types].xml、各级 .rels、按路径读写部件
  json/                      领域 JSON 信封
  dispatch/                  按 format 分发给 xlsx / docx / pptx
  props/                     core / app / custom 文件属性；三格式共用
  xlsx/                      工作簿 inspect / apply / create
  docx/                      文档 inspect / apply / create
  chart/                     c:chart 部件；embedding 经本模块读写 xlsx
  ole/                       嵌入对象分类与 Package 包装；docx / pptx / xlsx 共用
  pptx/                      演示文稿 inspect / apply / create
crates/ooxp-cli/             命令行，只依赖 ooxp。二进制名也是 ooxp
```

依赖方向只能是：`cli` → 库的门面；`dispatch` → `json` 与各格式模块；`xlsx` / `docx` / `pptx` → `opc`、`json` 与 `props`。
`props` 可调用 `opc` 与 `json`；不依赖格式模块。`chart` 可调用 `xlsx` 读写 embedding；pptx 不直接引用 `xlsx` 或 `docx`。
`ole` 可调用 `opc`；xlsx / docx / pptx 调用 `ole`，彼此不引用。`json` 不依赖格式模块。各模块只依赖 `error`，`error` 不反向依赖 `json`、`opc` 或任何格式模块。
禁止格式模块直接操作 ZIP 字节而不走 `opc`。`SetCellOp`、工作表 `scope`、`setParaText` 这类类型放在对应格式模块里。

### opc

把 `.docx`、`.xlsx`、`.pptx` 都当成 OPC 包。负责打开、列出部件、按路径读字节、标记脏部件、保存。
保存时没改过的部件保持原来的压缩方法和未压缩字节，不重排 ZIP 中央目录里未改条目的相对顺序，也不去美化无关 XML。细节见 [opc.md](opc.md)。

### json

只处理领域信封：`schemaVersion`、`action`、各动作的必填字段。不认识的字段在第一期直接报错，避免默默丢数据。
契约见 [json-contract.md](json-contract.md)，xlsx / docx / pptx 分册见同目录 `json-contract-xlsx.md` 等。

### xlsx

读 `xl/workbook.xml` 拿工作表名单和关系。`dialogsheet` 与 `macrosheet`（含 Type 以 `/xlIntlMacrosheet` 结尾的国际宏表）进 `sheets`，
字段只有 `id`、`name`、`kind`、`hidden`、`hasHeaderFooter`；
给了 `scope.headerFooter` 再跟 `headerFooters`。
不填单元格、图、保护、打印、视图。apply 仍拒写这两类表（`setHeaderFooterText` 对 `dialogsheet` 仍可改文本）。create 不生成这两类。
`format` 认 `.xlsx` 与 `.xlsm`，inspect 树的 `format` 仍为 `xlsx`。不执行宏。
按范围读对应 `sheetN.xml`，需要时再读 `xl/sharedStrings.xml`。工作簿与每张 `worksheet` 始终带 `pictureCount`。
工作簿、每张 `worksheet` 与每张 `chartsheet` 始终带 `chartCount`。每张表始终带 `hasHeaderFooter`。
每张 `worksheet` 与 `chartsheet` 始终带 `print` 与 `view`。每张 `worksheet` 始终带 `autoFilter`。每张 `worksheet` 始终带 `sortState`。
每张 `worksheet` 始终带 `outline`。每张 `worksheet` 始终带 `commentCount`（只计无 `parentId` 的顶层线程化批注）、
`hyperlinkCount`（该表 `<hyperlinks>` 里 `<hyperlink>` 的条数）、`sparklineCount`（该表 `x14:sparklineGroup` 的组数）、
`oleObjectCount`（该表被收入的嵌入对象个数）、`model3dCount`（该表被收入的 3D 模型个数）、
`slicerCount`（该表被收入的切片器个数，含时间线）与 `protectedRangeCount`（该表 `<protectedRange>` 的个数）。
工作簿与每张 `worksheet` 始终带 `textBoxCount`、`mathCount`、`shapeCount`、`connectorCount` 与 `smartArtCount`。
每张 `worksheet` 与 `chartsheet` 始终带 `sheetProtection`。
工作簿根始终带 `commentCount`、`hyperlinkCount`、`sparklineCount`、`oleObjectCount`、`model3dCount` 与 `slicerCount`（各表之和）、`connectionCount`、
`externalLinkCount`、`customViewCount`、`workbookProtection` 与 `props`（core / app / custom，规则见契约「文档属性」）。
`scope.comments` 为 true 时列出该表线程化批注名单，不读作者不以 `tc=` 开头的旧式 VML 批注。
`scope.hyperlinks` 为 true 时列出该表 `<hyperlinks>`，不把 `HYPERLINK()` 公式或图上的链算进来。
`scope.sparklines` 为 true 时列出该表 `x14:sparklineGroups`，数据范围当字符串，不把折线画进树。
`scope.oleObjects` 为 true 时列出该表 `<oleObjects>`，不把嵌入字节或预览图放进树。
`scope.model3ds` 为 true 时列出该表 drawing 里的 `am3d` AlternateContent，不把 GLB 或海报放进树。
`scope.maths` 为 true 时列出该表 drawing 里的 `a14:m` 公式框，不把 OMML 原文放进树，也不进 `textBoxes`。
`scope.slicers` 为 true 时列出该表 drawing 里的 slicer 与时间线 `graphicFrame`，不把 cache 选项或筛选状态倒进树。
`scope.protectedRanges` 为 true 时列出该表 `<protectedRanges>`，不把哈希放进树。
`scope.connections` 为 true 时列出工作簿 `/xl/connections.xml`，不刷新。
`scope.externalLinks` 为 true 时列出 workbook `<externalReferences>`，不打开目标文件。
`scope.pictures` 为 true 时读该表 `<drawing>` 指向的 drawing 部件，列出 `xdr:pic`，不读媒体字节。
`scope.charts` 为 true 时列出 `xdr:graphicFrame` 里的图表，系列一次倒出；数值优先读 `c:f` 指向的单元格。
`scope.textBoxes` 为 true 时列出 `xdr:sp` 且 `txBox` 的文本框，可见字取 `xdr:txBody`。`scope.shapes` 为 true 时列出无 `txBox` 的 `xdr:sp`。
`scope.connectors` 为 true 时列出 `xdr:cxnSp`。
`scope.smartArt` 为 true 时列出 diagram URI 的 `xdr:graphicFrame`；四条关系从 drawing `.rels` 解到 `/xl/diagrams/`。
`scope.headerFooter` 为 true 时展开六个页眉页脚槽位的原文；同时 `pictures: true` 时再列出 `drawingHF` 里的图。
apply 可改目标单元格，也可换、插、删工作表图，并可改 `oneCell` / `absolute` 的尺寸与锚点。
也可插、删、改工作表图表：改系列时写本表单元格并回写 cache（连续 A1、能解析成连续 A1 的命名区域、以及长度对得上的不连续并集），插图时在 `dataFrom` 写下数据表。也可增删趋势线、误差线，并把系列挂到次轴。
也可新建或删除图表工作表：数据仍写在点名的普通表上，图表工作表本身没有 `sheetData`。也可插入、删除、改名、重排与隐藏普通工作表，脏 `workbook.xml` 的 `sheets` 与对应 worksheet 部件。
对 `dialogsheet` / `macrosheet` 这些 op 仍报 `NotImplemented`。
也可用 `setCellStyle` 写单元格数字格式、字体、填充、边框与对齐：复用或追加 `styles.xml` 里的 `xf`，只改被点名格子的 `s`。边框含对角线，对齐可写缩进、旋转与缩小字体。
也可用 `insertCellStyle` / `deleteCellStyle` / `setCellNamedStyle` 读写 `cellStyles`：insert 写入 `cellStyleXfs` 再挂名，套用时把格子的 `s` 指到引用该 `xfId` 的 `cellXfs`。不改主题。
也可插、删、改工作表单元格超链接：脏该表 XML 的 `<hyperlinks>` 与工作表 `.rels`（Type 以 `/hyperlink` 结尾、`TargetMode` 为 `External`），不标重算，不改单元格值。
也可插、删、改图 / 文本框 / 形状 / SmartArt 上的 DrawingML 链：脏 drawing 的 `a:hlinkClick` 与 drawing `.rels`。
也可按槽位改页眉页脚原文，并可对 `worksheet` 与 `chartsheet` 的页眉槽位插、换、删 DrawingML 图。
也可往图表工作表的 Chart Drawing（`c:userShapes`）插图、形状、文本框与连接线，锚点用 0–1 分数。
也可插、删、改工作表文本框（`xdr:sp` `txBox`）、无 `txBox` 的形状、
连接线（`xdr:cxnSp`）与 SmartArt（diagram `graphicFrame`）：改字只动 `xdr:txBody` 或 data 部件可见点；文字格式走 `setShapeFormat`；几何走锚点。
也可插改删工作表 drawing 公式框（`a14:m`），不进 `textBoxes`。
插 SmartArt 再新建 `/xl/diagrams/` 四件套，关系写在 drawing `.rels`。`insertChartSheet` 再新建 `/xl/chartsheets/sheetN.xml`。
往页眉插图可能新建另一份 drawing 并写 `<drawingHF>`。改单元格值或公式之后，把工作簿标成打开时需要重算，必要时删掉 `xl/calcChain.xml`。
`setCellStyle` 可改 `styles.xml`，不求值公式。也可用 `setColWidth` / `setRowHeight` / `mergeRange` / `unmergeRange` 写 `cols`、
`row/@ht` 与 `mergeCells`：只改被点名的列、行与合并区，不算最适列宽，不按列宽反推 `twoCell` 图尺寸。
也可改共享公式主格（保住整组或拆成独立公式）并读写数组公式 `t="array"`，并可改数据表主格的 `TABLE` 输入格，不求值、不新建数据表。
也可用 `insertDataValidation` / `setDataValidation` / `deleteDataValidation` 读写工作表 `dataValidations`：公式当字符串，不求值、不展开下拉列表；`extLst` 里的扩展校验原样留下。
也可用 `insertConditionalFormat` / `setConditionalFormat` / `deleteConditionalFormat` 读写工作表 `conditionalFormatting`：公式当字符串，
颜色写入 `styles.xml` 的 `dxf`；可写 `top10` / `aboveAverage` / 文本类规则；不渲染、不求值；`extLst` 里的扩展条件格式原样留下。
不写 embedding，不对 xlsx 做 `exportChartData`。不编辑 `legacyDrawingHF` / 表单控件 VML。
insert / delete 线程化批注时只追加或去掉对应 Note `v:shape` 与占位 `comments.xml`，不改已有框的几何。
也可插、删、回复、改字、改作者与完成标记线程化批注（MS-XLSX `threadedComment`），
脏 threadedComments、persons、占位 comments、VML、工作表 `legacyDrawing`、关系与内容类型。
也可插、删、改工作表迷你图组：脏该表 `extLst` 里的 `x14:sparklineGroups`，可改颜色、标记与坐标轴，不新建部件，不标重算。
也可用 `setProtection` 开关工作表 `sheetProtection` 与工作簿 `workbookProtection`：脏该表 XML 或 `workbook.xml`；
无密码时不写哈希，给了 `password` 则写 Excel 16.0 的 SHA-512 哈希，不新建部件，不标重算。
也可用 `insertProtectedRange` / `setProtectedRange` / `deleteProtectedRange` 写工作表 `<protectedRanges>`：只改 `worksheet`，不标重算，不新建部件。
create 可写图、图表、图表工作表、页眉页脚、批注、超链接、文本框、公式框、形状、连接线、SmartArt 与 3D 模型，
也可按已有 apply 写出列宽、行高、合并、数据验证、条件格式、表格、透视、定义名、打印、视图、筛选、排序、分组、隐藏行列、拆分、自定义视图、迷你图、OLE、切片器、连接、外部引用、保护、允许编辑区域、组合，以及单元格样式与共享 / 数组公式。
不新建数据表公式。细则见 create。
create 到 `.xlsm` 时 workbook Override 用 macroEnabled Content Type，不建空的 `vbaProject.bin`。

### docx

主文档由根关系里 `/officeDocument` 指向，常见路径 `/word/document.xml`。
inspect 读该部件里的 `w:p`，返回大纲和按范围截断的段落；并始终列出页眉、页脚容器、各节页面与页眉页脚属性，以及每节是否链接到前一节。
文档根始终带正文 `pictureCount`、`textBoxCount`、`connectorCount`、`shapeCount`、`inkCanvasCount`、`footnoteCount`、`endnoteCount`、`commentCount`、`chartCount`、
`smartArtCount`、`oleObjectCount`、`model3dCount`、`tableCount`、`revisionCount`、`hyperlinkCount`、`bookmarkCount`、`fieldCount`、`tocCount`、
`captionCount`、`crossRefCount`、`indexCount`、`indexEntryCount`、`sourceCount`、`citationCount`、`bibliographyCount`、
`docPartCount`、`mathCount`、`contentControlCount`、`numberingCount`、`styleCount`、`embeddedFontCount`，
以及整篇 `mirrorMargins` / `gutterAtTop`、`props`、`protection`、`theme`、`themeOverride` 与 `fontEmbed`、`pageBackground`。
页眉页脚容器始终带前两项与 `connectorCount`、`shapeCount`、`inkCanvasCount`、`chartCount`、`smartArtCount`、`oleObjectCount`、`model3dCount`、`tableCount`、`revisionCount`、
`hyperlinkCount`、`bookmarkCount`、`fieldCount`、`mathCount`、`contentControlCount`。
`scope.pictures` / `scope.textBoxes` / `scope.connectors` / `scope.shapes` / `scope.inkCanvases` / `scope.model3ds` 为 true 时列出页眉页脚图、框、连接线、无框形状、墨迹画布或 3D 模型；同时给了正文切片则段落上也列出。
组合本身不进树，组合里的图 / 框 / 连接线 / 无框形状 / 墨迹画布展开。`scope.textBoxContent` 再展开框内段落。`scope.tables` 为 true 时框节点带框内表名单。`scope.notes` 为 true 时列出用户脚注、尾注与分隔符；注内段落一次倒出。
`scope.comments` 为 true 时列出批注；批注内段落一次倒出。`scope.charts` 为 true 时列出正文、页眉页脚、文本框、脚注、尾注与批注里的嵌入图表；系列一次倒出。
`scope.smartArt` 为 true 时列出正文、页眉页脚、文本框、脚注、尾注与批注里的 SmartArt；可见点一次倒出。`scope.oleObjects` 为 true 时列出正文与页眉页脚里的嵌入对象。
`scope.tables` 为 true 时列出正文与页眉页脚里的表；单元格段落一次倒出。`scope.revisions` 为 true 时列出正文与页眉页脚里的 run 级插入、删除与属性修订。
`scope.hyperlinks` 为 true 时列出正文与页眉页脚里的 `w:hyperlink`。`scope.bookmarks` 为 true 时列出正文与页眉页脚里的配对书签。
`scope.fields` 为 true 时列出正文与页眉页脚里的 `w:fldChar` / `w:fldSimple`。`scope.tocs` 为 true 时列出主文档目录块。
`scope.captions` 为 true 时列出主文档题注。`scope.crossRefs` 为 true 时列出主文档交叉引用。`scope.indexes` 为 true 时列出主文档 INDEX 块。
`scope.indexEntries` 为 true 时列出主文档 XE 索引项。`scope.sources` 为 true 时列出书目源。`scope.citations` 为 true 时列出主文档 CITATION。
`scope.bibliographies` 为 true 时列出主文档 BIBLIOGRAPHY 块。`scope.docParts` 为 true 时列出 glossary 构建基块。
`scope.embeddedFonts` 为 true 时列出 fontTable 里的嵌入字体面。`scope.maths` 为 true 时列出正文与页眉页脚里的顶层 `m:oMath`，名单带 `expr`。
`scope.contentControls` 为 true 时列出正文与页眉页脚里被收入的 `w:sdt`。`scope.numbering` 为 true 时列出 `numbering.xml` 里的编号实例。
`scope.styles` 为 true 时列出 `styles.xml` 里的样式定义。`scope.runs` 为 true 时已列出的段落带 run 名单。
apply 可改被点名段落的可见文本（主文档、页眉页脚、框内、注内或批注内），可在主文档、页眉页脚与表格单元格插删 `w:p`，可插删 `w:tbl` 以及行与列（含文本框里的表，不含注内 / 批注内），并可按 `gridSpan` / `vMerge` 合并与拆分，
可接受或拒绝被点名的修订，可插、删、改超链接地址与显示文本，也可给正文与页眉页脚里的图、文本框写 DrawingML `a:hlinkClick`，可插、删、改书签名，可插、改指令、删域（不求值），可写入编号定义并把段落套上或去掉编号，
可增删自定义样式，可给段落换 `pStyle`，可给 run 套字符样式，并可改被点名段落、字符或表格样式的字体、字号、加粗、颜色与段前段后（字符样式不能写段距），
可改表的 `tblStyle`、`tblLook`、浮动 `tblpPr`、表级 `tblCellMar`，以及单元格底纹、格边距与斜线，
可改段上直接对齐、缩进与间距，可改被点名 run 的加粗、斜体、下划线、字号、颜色与字体，
可改节上 `w:pgMar` / `w:pgSz` / `w:cols` / `w:vAlign` / `w:lnNumType` / `w:docGrid` / `w:rtlGutter` / `w:pgBorders`，
可改整篇 `w:mirrorMargins` / `w:gutterAtTop`，可改整篇 `w:background` 与 `w:displayBackgroundShape`，可改节上页眉/页脚距离，
也可断开或重新链接某节某类型的页眉页脚（断链时抄走 header/footer 自己的 `.rels`，媒体仍共用），也可为空白槽位新建页眉页脚部件，并可写本节 `w:titlePg` 与整篇 `w:evenAndOddHeaders`，
也可在正文段后插入分节符或删除非末节，也可插入或删除目录块，也可插入题注与交叉引用（不在库里求 SEQ / REF 的值），也可插入书目源、引文与书目块（不在库里按样式表排版），也可插删构建基块并把基块正文复制进主文档，也可插改删 OMML 公式，也可插改删内容控件。
页眉、页脚、正文、注和批注里的 DrawingML 图片可换、插、删，并可改尺寸与绕排。
文本框可插、删（不含注内、批注内与框内再套框；Word 16.0 不会把框写进另一个 `w:txbxContent`），可改尺寸、绕排、填色、描边、横竖排与链接。连接线可插、删（不含注内、批注内与框内）。
无框形状可插、删、改位置、尺寸、填充与线（不含注内、批注内与框内）。
墨迹画布可插、换、删（不含注内、批注内与框内），并可按 id 导出 InkML。
浮动图 / 框 / 连接线 / 无框形状可组合或解开。脚注、尾注可插、删，可改编号格式（整篇同时写 `settings.xml` 与各节 `sectPr`，Word 读节属性）。批注可插、删、回复、改字、改作者与完成标记。
嵌入图表可插、删、改标题、改系列数据、改类型与图例，并可改宿主 drawing 的尺寸与绕排；宿主可以是正文、页眉页脚、框内、注内或批注内段落。圆环、雷达、气泡与组合图走同一套 op；透视图表只改标题、图例、标签、删除与尺寸绕排，不改数。
股价（含 OHLC）、曲面（含线框与俯视）、复合饼可插、改类型；带成交量的股价按组合图写入。chartex 可 insert / create（须带 `layoutId`），已写入的 chartex 除删除与尺寸绕排外仍锁数据 op。次轴、趋势线、误差线（含 `cust`）可写。SmartArt 可插、删、改各点文字、增删点、改配色与快速样式，并可改宿主 drawing 的尺寸与绕排；宿主范围与图表相同。
三种格式都可改文件属性。docx 还可开关无密码的限制编辑与只读推荐。`exportMedia` 按图 id 写出 `/word/media/…` 原字节，按 OLE id 写出 embeddings 原字节，
按墨迹画布 id 写出 `/word/ink/…` 原字节，按 3D 模型 id 写出 GLB 原字节。
`exportChartData` 按图表 id 写出 embedding xlsx 原字节。不整篇从 JSON 重建。
create 生成最小 Transitional 包，含 `styles.xml`（Normal 与 Heading1–3）和 `settings.xml`（`compatibilityMode` 为 15），
给了 `paragraphs[].sectionType` 时可写多节，给了 `paragraphs[].charts` 时复用 `insertChart`，
给了 `footnote` / `endnote` / `comments` 时复用对应 insert。
给了 `comments[].replies` 时复用 `replyComment`。
给了 `hyperlinks` / `bookmarks` / `fields` / `maths` / `contentControls` 时复用对应 insert。
给了 `document.numberings` 时复用 `insertNumbering`，再按 `paragraphs[].num` 发 `setParaNumbering`。
给了 `document.tocs` 时复用 `insertToc`。不写修订痕迹。段落上可带文本框、DrawingML 图、SmartArt 与图表。
给了 `document.tables` 时可在正文写表；页眉页脚可带 `tables`。
给了 `document.headers` / `footers` 时可写 `default` / `even` / `first`（给了 even 则写 `evenAndOddHeaders`，给了 first 则写 `titlePg`）。
不改注区怎么画，不改批注气泡怎么排，不画图表，不算 SmartArt 版式，不算表列宽。

### pptx

入口由根关系里 `/officeDocument` 指向，常见路径 `/ppt/presentation.xml`。幻灯片顺序跟 `p:sldIdLst`，不跟文件名。
inspect 默认只返回页名单与各页 `pictureCount`、`mathCount`、`videoCount`、`audioCount`、`chartCount`、`smartArtCount`、`commentCount`、
`hyperlinkCount`、`fieldCount`、`tableCount`、`oleObjectCount`、`model3dCount`、`zoomCount`、`connectorCount`、`animCount`、`layoutId`、`hasNotes`、`transition`，
以及根上 `masterCount`、`hasHandout`、`sectionCount`、`slideSize`、`customShowCount`、`showPr`、`modifyProtection` 与 `props`；
要图名单必须给 `scope.pictures`，要公式框名单必须给 `scope.maths`，要视频 / 音频名单必须给 `scope.media`，要图表名单必须给 `scope.charts`，
要 SmartArt 名单必须给 `scope.smartArt`，要表名单必须给 `scope.tables`，要嵌入对象名单必须给 `scope.oleObjects`，要 3D 模型名单必须给 `scope.model3ds`，要 Zoom 名单必须给 `scope.zooms`，
要连接线名单必须给 `scope.connectors`，要备注形状必须给 `scope.notes`，要批注名单必须给 `scope.comments`，要超链接名单必须给 `scope.hyperlinks`，
要域名单必须给 `scope.fields`，要动画摘要必须给 `scope.timing`，要母版与版式名单必须给 `scope.masters`，要备注母版占位符必须给 `scope.notesMaster`，要讲义母版槽位必须给 `scope.handout`，
要幻灯片节名单必须给 `scope.sections`，要自定义放映名单必须给 `scope.customShows`。
apply 可改被点名形状的 `p:txBody` 可见文字（含备注形状、母版 / 版式占位符与装饰形、备注母版与讲义占位符，缩略图占位符除外），
可插入文本框与非矩形预设几何、写入自定义路径（`a:custGeom`）、插入连接线（可写 `stCxn` / `endCxn`）、组合与解开、删除被点名形状、改画布坐标与尺寸、填充描边与 DrawingML 文字格式。
母版 / 版式装饰形可用 `deleteShape`，占位符请用 `deletePlaceholder`。
删除时清掉 timing 里对应 `tgtEl`，不重建未点名形状，不整节无故丢掉仍有内容的 `<p:timing>`。
可用 `setNotesText` 按页改演讲者备注；也可换、插、删幻灯片或备注页上的 `p:pic` 并改尺寸与画布坐标；也可插、删、改幻灯片或备注页上的嵌入图表（改系列时 cache 与 embedding 双写；
也可增删趋势线、误差线，并把系列挂到次轴）；也可插、删、改幻灯片或备注页上的 SmartArt（改点时写 data 部件，对不上 drawing 缓存则删缓存）；
也可插、删、改幻灯片与备注页上的 DrawingML 表（改单元格字、插删行列、改画布坐标与尺寸；删表时清对应 timing `tgtEl`）；
也可插、删、回复、改字、改作者与完成标记现代批注（MS-PPTX 2018/8 `p188:cmLst`，不写 ECMA-376 §19.4 旧式部件）；
也可插入空白页、删除、按 `sldIdLst` 重排与隐藏（`p:sld/@show`）；也可用 `setTransition` 写或清除幻灯片、母版或版式上的 `<p:transition>`；
也可用 `insertAnim` / `setAnim` / `deleteAnim` / `moveAnim` 按模板增删改预设效果（不接受整棵时间轴树；第一条 `with` / `after` 进片自动播）；也可用 `setSlideLayout` 只改该页 `.rels` 的版式关系（不删用户形状、不抄占位符进页）；
也可用 `insertLayout` / `deleteLayout` / `insertPlaceholder` / `deletePlaceholder` / `insertMaster` / `deleteMaster` 插删空白版式、
带 `p:ph` 的占位符与整份母版（不算母版排版引擎；`insertPlaceholder` 还可写备注母版与讲义）；
也可用 `insertShape` / `deleteShape` 插删母版 / 版式上没有 `p:ph` 的装饰形（不抄进各页）；
也可用 `setSlideSize` 只改 `p:sldSz`（不重排形状）；也可用 `setThemeColors` / `setThemeFonts` 改该母版 theme 的配色与字体（不重写 `fmtScheme`）；
也可用 `setHandoutText` 改讲义母版页眉、页脚、日期与页码占位符（没有 handoutMaster 则新建最小部件）；
讲义与备注母版占位符也可绑同一套预设动画；
也可插删改幻灯片、备注页与幻灯片母版 / 版式上的 `a:hlinkClick` / `a:hlinkHover` / `a:hlinkMouseOver`（URL / 文件 / 跳转到某页，含图、表、图表、SmartArt 与表单元格）；
也可插删改幻灯片、备注页与幻灯片母版 / 版式文本形状里的 `a:fld`（页码与日期域，不求值）；也可往幻灯片、备注页与幻灯片母版 / 版式插、删内嵌视频与音频（`p:pic` 上的 `a:videoFile` / `a:audioFile` 与 `p14:media`，不写 timing），
也可用 `setMedia` 改裁剪、书签与字幕轨（不写 fade）；
也可用 `insertCustomShow` / `setCustomShow` / `deleteCustomShow` 读写 `p:custShowLst`（不算放映引擎）；
也可插、删幻灯片、备注页与幻灯片母版 / 版式上的嵌入对象（`p:oleObj`，不激活 OLE 服务器），也可插、删、换幻灯片、备注页与幻灯片母版 / 版式上的 3D 模型（`am3d:model3d`，不栅格化），
也可插改删幻灯片、备注页与幻灯片母版 / 版式上的 drawing 公式框（`a14:m`，不实现排版引擎），也可插、删、改幻灯片、备注页与幻灯片母版 / 版式上的 Zoom（`sldZm` / `sectionZm` / `summaryZm`，不实现放映跳转）；也可改文件属性。
插删重排不重写未点名页。`exportMedia` 按幻灯片、备注页、批注、母版 / 版式上的图 id、视频 id、音频 id、OLE id 或 3D 模型 id 写出 `/ppt/media/…` 或 `/ppt/embeddings/…`。
`exportChartData` 按图表 id 写出 embedding。create 生成最小 Transitional 包，
必含 theme、slideMaster、slideLayout、presentation、各页 slide、presProps。
给了 `slides[].pictures` 时复用 `insertPicture`，写入 `/ppt/media/…` 与该页 `p:pic`。
给了 `slides[].maths` 时复用 `insertMath`，写入 `a14:m` AlternateContent。
给了 `slides[].model3ds` 时复用 `insertModel3d`，写入 GLB、海报 PNG 与 `am3d` AlternateContent。
给了 `slides[].charts` 时复用 `insertChart`，写入 `/ppt/charts/…`、embedding 与 `p:graphicFrame`。
给了 `slides[].smartArts` 时复用 `insertSmartArt`，写入 `/ppt/diagrams/…` 与图示 `p:graphicFrame`。
给了 `slides[].tables` 时复用 `insertTable`，写入表 `p:graphicFrame`。给了 `slides[].notes` 时复用 `setNotesText`。
给了 `slides[].comments` 时复用 `insertComment`。给了 `slides[].anims` 时复用 `insertAnim`（白名单预设，可写 `delay` / `dur` / `repeat` / 声音 / 按段；第一条 `with` / `after` 进片自动播）。
母版与版式进树（`scope.masters`），含占位符与装饰形。`setSlideLayout` 只换关系，写入用 `insertLayout` / `insertPlaceholder` / `insertMaster` / `insertShape` 等。
动画效果摘要进树（`scope.timing`），写入用 `insertAnim` / `setAnim` / `deleteAnim` / `moveAnim`。切换进树，用 `setTransition` 改。幻灯片尺寸始终进树，写入用 `setSlideSize`。
主题配色与字体挂在 `scope.masters` 的 `theme` 上，写入用 `setThemeColors` / `setThemeFonts`。`mc:AlternateContent` 只走第一个 `Choice`。

### cli

从文件或 stdin 读入一份领域 JSON，按 `action` 分发。inspect 把树打到 stdout。create 和 apply 写出目标 Office 文件。`exportMedia` 写出媒体文件。
`exportChartData` 写出图表 embedding。契约见 [cli.md](cli.md)。

## 数据流

### inspect

调用方给出路径和范围。库打开 OPC 包，定位格式入口部件（xlsx 是 Workbook，docx 是 Main Document，pptx 是 Presentation）。按范围读必要的 XML，生成带 id 的树。
xlsx 默认不读完全部单元格，也不展开 `pictures`、`charts`、`textBoxes`、`maths`、`shapes`、`connectors`、`smartArts`、页眉原文、数据验证名单、条件格式名单、工作表表格名单、透视表名单、
批注名单、超链接名单、迷你图名单、嵌入对象名单、切片器名单、连接名单、外部引用名单或自定义视图名单（仍返回 `pictureCount`、`chartCount`、`textBoxCount`、`mathCount`、`shapeCount`、
`connectorCount`、`smartArtCount`、`dataValidationCount`、`conditionalFormatCount`、`tableCount`、`pivotCount`、
`commentCount`、`hyperlinkCount`、`sparklineCount`、`oleObjectCount`、`model3dCount`、`slicerCount`、`connectionCount`、`externalLinkCount`、
`customViewCount`、`hasHeaderFooter`、`print`、`view`、`sheetProtection` 与根上 `props`、`workbookProtection`、`definedNameCount`、`definedNames`）。
xlsx 的 `worksheet` 还返回 `autoFilter` 与 `sortState`。
docx 默认不返回正文 `paragraphs` 也不展开页眉页脚段落、任何 `pictures`、`textBoxes`、`connectors`、注名单、批注名单、图表名单、SmartArt 名单、OLE 名单、水印名单、表名单、
修订名单、超链接名单、书签名单、域名单、目录名单、题注名单、交叉引用名单、构建基块名单、公式名单、内容控件名单、编号名单、样式名单或 run 名单（仍返回根上 `props` 与 `protection`、`theme`、`themeOverride`、`fontEmbed`、`pageBackground`，
以及 `watermarkCount`），pptx 默认不返回 `shapes`，
也不展开 `pictures` / `maths` / `charts` / `smartArts` / `tables` / `oleObjects` / `model3ds` / `zooms` / `connectors` / `notes` / `comments`
 / `hyperlinks` / `fields` / `timing` / `masters` / `notesMaster` / `handout` / `sections` / `customShows`（仍返回根与各页 `pictureCount`、`mathCount`、
`chartCount`、`smartArtCount`、`commentCount`、`hyperlinkCount`、`fieldCount`、`tableCount`、`oleObjectCount`、`model3dCount`、`zoomCount`、
`connectorCount`、`animCount`、`masterCount`、`hasHandout`、`sectionCount`、`slideSize`、`customShowCount`，
以及各页 `layoutId`、`hasNotes`、`transition` 与根上 `props`）。包不写回。

### apply

调用方给出路径和 `ops`。库打开包，按每条 op 的 id 找到 XML 节点，只改这些节点。xlsx 在改了单元格之后，把工作簿标成打开时需要重算。
xlsx 也可改被点名的工作表图（换、插、删、改尺寸、改锚点），也可插删改工作表图表，脏 drawing、chart 部件、工作表 XML、关系与媒体。
xlsx 也可新建或删除图表工作表，脏 workbook 的 `sheets`、chartsheet 部件及其 drawing / chart。
xlsx 也可插入、删除、改名、重排与隐藏普通工作表，脏 workbook 的 `sheets`、worksheet 部件与关系。
对 `dialogsheet` / `macrosheet` 这些 op 仍报 `NotImplemented`。xlsx 也可用 `setCellStyle` 改被点名格子的样式，脏该表 XML 与 `styles.xml`。
xlsx 也可设列宽、行高、合并与取消合并，脏该表 `cols` / `sheetData` / `mergeCells`，不标重算。xlsx 也可改共享公式主格、把主格迁到新区域左上角，或写成数组公式，仍标重算、仍不求值。
xlsx 也可增删改数据验证，脏该表 `dataValidations`，不标重算。xlsx 也可增删改条件格式，脏该表 `conditionalFormatting` 与必要时 `styles.xml` 的 `dxfs`，不标重算。
xlsx 也可增删改工作表表格，脏 `/xl/tables/…`、工作表 `tableParts`、关系与内容类型，必要时脏 `sheetData`，不标重算。
xlsx 也可增删改透视表，脏 `/xl/pivotTables/…`、`/xl/pivotCache/…`、workbook 的 `pivotCaches`、关系与内容类型，不标重算，也不把合计写进 `sheetData`。
xlsx 也可增删改定义名，脏 workbook 的 `definedNames`，不新建部件，不标重算。
xlsx 也可用 `setPrintSetup` 改工作表页边距、纸张、方向、缩放、网格线、打印区域与手工分页符，脏该表 XML，打印区域还脏 workbook 的 `definedNames`，不标重算。
xlsx 也可用 `setSheetView` 改默认视图的缩放、冻结与拆分，脏该表 `sheetViews`，不标重算。
xlsx 也可列出、插入、删除自定义视图，并按快照把窗格写回当前视图，脏 workbook 的 `customWorkbookViews` 与各表 `customSheetViews` / 当前 `sheetViews`，不标重算，不恢复隐藏行 / 列。
xlsx 也可用 `groupRows` / `ungroupRows` / `groupCols` / `ungroupCols` / `setOutline` 改行 / 列大纲与折叠，
也可用 `setRowHidden` / `setColHidden` 写没有大纲级的隐藏行 / 列，
脏该表 `sheetPr/outlinePr`、`sheetFormatPr`、`cols` 与 `sheetData` 行属性，不标重算。
xlsx 也可用 `setAutoFilter` 改工作表级筛选范围，脏该表 `autoFilter`，不改表格部件，不隐藏行，不标重算。
xlsx 也可用 `setSortState` 改工作表级排序条件（含自定义序列、按颜色 / 图标、`sortMethod`），脏该表 `sortState`，给了 `color` 时还可脏 styles 的 `dxfs`，不重排 `sheetData`，不标重算。
xlsx 也可插删回复并改线程化批注，脏 `/xl/threadedComments/`、`/xl/persons/person.xml`、占位 `/xl/commentsN.xml`、VML、工作表 `legacyDrawing`、关系与内容类型，不标重算。
xlsx 也可插删改工作表文本框、形状、连接线与 SmartArt，可用 `setShapeFormat` 改 DrawingML 文字格式，脏 worksheet drawing、drawing `.rels`、`/xl/diagrams/` 与内容类型，不标重算。
xlsx 也可插删改工作表单元格超链接，脏该表 XML 与工作表 `.rels`，不标重算。xlsx 也可插删改工作表迷你图组，脏该表 `extLst`，不新建部件，不标重算。
xlsx 也可列出、导出、插入并删除工作表嵌入对象，脏 `oleObjects`、worksheet drawing、VML `legacyDrawing`、工作表 `.rels`、embeddings 与预览媒体，不把字节塞进 inspect 树，不标重算。
xlsx 也可插、删、换工作表 3D 模型，脏 worksheet drawing、drawing `.rels`、GLB 与海报 PNG，不把 GLB 塞进 inspect 树，不标重算。
xlsx 也可插改删工作表 drawing 公式框，脏 worksheet drawing，写 `a14:m` AlternateContent，不标重算。
xlsx 也可列出、插入并删除工作表切片器与时间线，脏 `/xl/slicers/…`、`/xl/slicerCaches/…`、`/xl/timelines/…`、`/xl/timelineCaches/…`、worksheet drawing、工作表 `extLst` 的 `slicerList` / `timelineRefs`、
workbook `extLst` 的 `slicerCaches` / `timelineCacheRefs`、workbook 的 `<definedNames>`、工作表 / workbook `.rels` 与内容类型；
透视源的插入还脏 `/xl/pivotCache/pivotCacheDefinitionN.xml` 的 `extLst`（`x14:pivotCacheDefinition/@pivotCacheId`）。
时间线还会把该透视表 `updatedVersion` 升到至少 8，不在库里执行筛选，不标重算。
xlsx 也可列出、插入并删除工作簿外部数据连接，脏 `/xl/connections.xml`、workbook `.rels` 与内容类型，不向数据源发请求，不把查询结果写进 `sheetData`，不标重算。
xlsx 也可列出、插入、删除并改路径对其他工作簿的外部引用，脏 workbook `<externalReferences>`、`/xl/externalLinks/…` 及其 `.rels`、关系与内容类型；改路径只动该链 `.rels` 的 `Target`，不写 `xxl21:alternateUrls`，不打开目标文件，不求值，不标重算。
xlsx 也可用 `setProtection` 改工作表 `sheetProtection` 或 workbook `workbookProtection`，可写 SHA-512 哈希，不标重算。
xlsx 也可插、改、删工作表允许编辑区域，脏该表 `<protectedRanges>`，可写同一套 SHA-512，不标重算。
xlsx 也可改工作表上的页眉页脚原文，并可对 `worksheet` 与 `chartsheet` 页眉槽位插换删 DrawingML 图，脏 `headerFooter` / `drawingHF` 与对应 drawing。
xlsx 也可改文件属性，脏 `docProps`。docx 可改被点名的段落，可在主文档、页眉页脚与表格单元格插删段落，可插删表以及行与列并合并拆分单元格，可接受或拒绝 run 级、段落标记、表行与表属性修订，可插删改超链接与书签，可插改删域指令（不求值；
可按 code 写常用域与窗体 ffData），可插入或删除目录块（不在库里算页码），可插入题注与交叉引用（不在库里求 SEQ / REF），可插删构建基块并把基块正文复制进主文档，
可嵌入或删除 TrueType 字体并改 `embedTrueTypeFonts` / `embedSystemFonts` / `saveSubsetFonts`，可改页面背景色与节上页面边框，可插改删 OMML 公式（不实现排版引擎），可插改删拼音（不实现注音排版），可改段上禁则与 run 强调 / 压缩排版，可改整篇压缩与自定义禁则字符，
可插改删内容控件（含复选、图片、组合外壳与公式外壳；
不实现开发者模式 UI），可写入编号定义并把段落套上或去掉编号，可给段落换样式并改样式定义上的一批常见属性，可改段与 run 上的直接格式，可改节上页边距、纸张、分栏、垂直对齐、行号、文档网格与装订线侧，可改整篇镜像边距与装订线在上，
可改节上页眉页脚距边界，也可改节上页眉页脚是否链接前节，也可为空白槽位新建页眉页脚，也可写 `titlePg` 与 `evenAndOddHeaders`，也可插入或删除节，
也可改页眉、页脚、正文、注或批注里被点名的 DrawingML 图与文本框，也可插删连接线与无框形状并把浮动图 / 框 / 连接线 / 无框形状组合或解开，也可插删脚注尾注并改编号属性，也可插删回复批注并改完成标记，也可插删改嵌入图表（含框内、注内、批注内；
含趋势线、误差线与次轴），也可插删改 SmartArt（宿主范围相同）。
docx 也可插删嵌入对象，脏宿主 XML、`.rels`、embeddings 与占位预览图。
docx 也可插删换 3D 模型，脏宿主 XML、`.rels`、GLB 与海报 PNG。
docx 也可用 `setProtection` 改 `settings.xml` 的保护标记（限制编辑哈希走遗留预处理，修改密码走 UTF-16LE SHA-512），窗体保护还可脏主文档各节 `w:formProt`，
也可用 `setFormProt` 把某节改成例外，也可用 `insertEditRange` / `deleteEditRange` 读写 `w:permStart` / `w:permEnd`。
docx 也可插删页眉 DrawingML 水印，无 default 页眉时会新建，并可脏页眉部件与媒体。
docx 也可用 `setThemeColors` / `setThemeFonts` / `setThemeFmt` 改主文档 theme 部件的配色、字体与 `fmtScheme` 常用槽，只脏该 theme 部件；
也可用 `setThemeOverride` 新建或改主文档 themeOverride 部件。
pptx 可改被点名形状的可见文字（含备注形状与母版 / 版式占位符），可插入文本框与非矩形预设几何、写入自定义路径、插入连接线（幻灯片、备注页与母版 / 版式，可写 `stCxn` / `endCxn`）、组合与解开、删除被点名形状并清对应 timing `tgtEl`、改画布坐标与尺寸、填充描边与 DrawingML 文字格式，
可用 `setNotesText` 按页改或新建演讲者备注，也可改被点名的幻灯片图、备注图与母版 / 版式图（换、插、删、改尺寸、改画布坐标），也可插删改幻灯片、备注页与母版 / 版式图表，也可插删改幻灯片、备注页与母版 / 版式 SmartArt，
也可插删改幻灯片、备注页与母版 / 版式 DrawingML 表，也可插删回复并改现代批注，也可对批注或回复 id 插图并换、删批注内图（不改尺寸位置），也可按批注内图 id `exportMedia`，也可插入、删除、重排与隐藏幻灯片，
也可用 `setTransition` 写或清除该页、母版或版式上的 `<p:transition>`（含自动进片、方向、切换声音、`split` 的 `orient`、`wheel` 的 `spokes` 与 `p15:prstTrans`），
也可用 `insertAnim` / `setAnim` / `deleteAnim` / `moveAnim` 按模板增删改该宿主 `<p:timing>` 里的预设效果（不接受整棵时间轴树），也可用 `setSlideLayout` 只改该页 `.rels` 的版式关系，
也可用 `insertLayout` / `deleteLayout` / `insertPlaceholder` / `deletePlaceholder` / `insertMaster` / `deleteMaster` 插删版式、占位符与母版，
也可用 `setSlideSize` 只改 `p:sldSz`，也可用 `setThemeColors` / `setThemeFonts` 改该母版 theme，也可用 `setHandoutText` 改或新建讲义母版，
也可用 `insertField` / `setField` / `deleteField` 插删改幻灯片、备注页与母版 / 版式文本形状里的 `a:fld`（不求值），
也可用 `insertSlideSection` / `setSlideSectionName` / `deleteSlideSection` / `setSlideSection` 读写 `p14:sectionLst`（删节时页还在；
把页划进某节时可重排 `sldIdLst`），也可用 `insertCustomShow` / `setCustomShow`
 / `deleteCustomShow` 读写 `p:custShowLst`（删页时按 `r:id` 清引用），
也可插、删幻灯片、备注页与母版 / 版式上的嵌入对象（`p:oleObj`，不激活 OLE 服务器），也可插、删、换幻灯片、备注页与母版 / 版式上的 3D 模型（`am3d:model3d`，不栅格化），也可插改删幻灯片、备注页与母版 / 版式上的 drawing 公式框（`a14:m`，不实现排版引擎），也可插、删幻灯片、备注页与母版 / 版式上的 Zoom（`sldZm` / `sectionZm`，不写摘要缩放，不实现放映跳转），
也可用 `setShowPr` 改 `presProps` 的 `p:showPr`，也可用 `setProtection` 写出或删掉 `p:modifyVerifier`（给了 `password` 时写 PowerPoint 16.0 的 SHA-512 哈希），
脏 `presentation.xml` 的 `sldIdLst`、presentation 关系、新页或被删页及其备注 / 现代批注、内容类型，以及若有则同步 `p14:sectionLst` 与 `p:custShowLst`。
未点名页原字节写回。插删改图、图表、SmartArt、备注与批注时仍脏该页 XML、该页 `.rels`、notesSlide 及其 `.rels`、必要时 notesMaster 与 presentation、
`/ppt/charts/`、embedding、`/ppt/diagrams/`、`/ppt/comments/` 与 `/ppt/authors.xml`。
表格 op 只脏该宿主 XML（幻灯片、notesSlide、slideMaster 或 slideLayout）。形状管理 op 只脏目标宿主 XML（含被点名的 slideMaster / slideLayout）；
`insertShape` 往备注插时与 `setNotesText` 相同，还可脏 notesMaster 与 presentation。
超链接 op 脏该宿主 XML（幻灯片、notesSlide、slideMaster 或 slideLayout）与该宿主 `.rels`。域 op（`insertField` / `setField` / `deleteField`）只脏该宿主 XML。`setSlideLayout` 只脏该页 `.rels`。
`insertLayout` / `deleteLayout` / `insertMaster` / `deleteMaster` 还可脏母版 / 版式 / theme 部件、presentation 与内容类型；
`insertPlaceholder` / `deletePlaceholder` 只脏目标母版或版式 XML。
`setSlideSize` 只脏 `presentation.xml` 的 `p:sldSz`。`setThemeColors` / `setThemeFonts` 只脏该母版指向的 theme 部件。
三种格式都可用 `setProps` / `setCustomProp` / `deleteCustomProp` 改 core / app / custom，脏对应 `docProps` 部件与必要时的根关系、内容类型。保存时只重写脏部件。

### create

调用方给出格式、输出路径和内容对象。库在内存里生成一套最小可打开的 Transitional 部件，打成新 ZIP。不读取任何已有文件当模板，除非以后另开「从模板复制」的动作。
xlsx 的 `sheets[].pictures` 复用 `insertPicture`，`sheets[].charts` 复用 `insertChart`（数据写本表单元格，不写 embedding），
`sheets[].kind: chartsheet` 复用 `insertChartSheet`（数据写点名的普通表，不写 embedding），
`sheets[].headerFooters` 复用 `setHeaderFooterText` 与页眉 `insertPicture`（不写 VML），
`sheets[].comments` 复用 `insertComment`，`comments[].replies` 复用 `replyComment`（只一层），`sheets[].hyperlinks` 复用 `insertHyperlink`，
`sheets[].textBoxes` / `maths` / `shapes` / `connectors` / `smartArts` / `model3ds` 复用对应 insert。
`workbook.cellStyles` 复用 `insertCellStyle`，`cells[].namedStyle` 复用 `setCellNamedStyle`。
其余第四轮字段（列宽、表格、保护、迷你图、OLE、切片器等）复用对应 apply，见 [json-contract-xlsx.md](json-contract-xlsx.md#create)。
docx 段落与页眉页脚上的 `pictures` 同样复用插图，`charts` 复用 `insertChart`（数据写 embedding），`oleObjects` 复用 `insertOle`，
`inkCanvases` 复用 `insertInkCanvas`，
`footnote` / `endnote` / `comments` 复用对应 insert，`comments[].replies` 复用 `replyComment`，
`hyperlinks` / `bookmarks` / `fields` / `maths` / `contentControls` 复用对应 insert。
`document.numberings` 复用 `insertNumbering`，`paragraphs[].num` 复用 `setParaNumbering`。`document.tocs` 复用 `insertToc`。
docx create 第四轮同样复用已有 apply：`styles` / `sources` / `bibliographyStyle` / `bibliographies` / `captions` / `indexes` / `docParts` / `embeddedFonts` / `fontEmbed` / `eastAsianSettings`，以及段上 `shapes` / `connectors` / `rubies` / `citations` / `indexEntries` / `crossRefs` / `editRange`、图与框的 DrawingML 链、表外观。不写修订痕迹，也不写 `writePassword`。
create 不写修订痕迹。pptx 的 `slides[].pictures` 复用 `insertPicture`，
`slides[].maths` 复用 `insertMath`，`slides[].model3ds` 复用 `insertModel3d`，
`slides[].videos` / `slides[].audios` 复用 `insertVideo` / `insertAudio`，
`slides[].charts` 复用 `insertChart`（数据写 embedding），`slides[].smartArts` 复用 `insertSmartArt`，
`slides[].tables` 复用 `insertTable`，`slides[].oleObjects` 复用 `insertOle`，
`slides[].connectors` 复用 `insertConnector`，`slides[].zooms` 复用 `insertZoom`，
`slides[].notes` 复用 `setNotesText`（对象形态的 `anims` 复用 `insertAnim` 写到备注页，
`maths` / `model3ds` 复用 `insertMath` / `insertModel3d` 写到备注页），
`slides[].comments` 复用 `insertComment`，
形状上的 `url` / `file` / `slide` 复用 `insertHyperlink`（可 `on=run`），
`pictures[].hyperlink` / `tables[].hyperlink` / `charts[].hyperlink` / `smartArts[].hyperlink` 以及表单元格链字段同样复用 `insertHyperlink`，
`shapes[].geom` 复用 `setShapeGeom`，`shapes[].fields` 复用 `insertField`，
`shapes[].textFill` / `textOutline` 复用 `setShapeFormat`，
`slides[].transition` 复用 `setTransition`，
`slides[].anims` 复用 `insertAnim`（白名单预设，可写 `delay` / `dur` / `repeat` / 声音 / 按段，第一条 `with` / `after` 进片自动播，不接受整棵时间轴树），
`presentation.sections` 复用 `insertSlideSection`，`presentation.customShows` 复用 `insertCustomShow`，
`presentation.handout` 复用 `setHandoutText`，`presentation.showPr` 复用 `setShowPr`，
`presentation.modifyProtection` 复用 `setProtection`。
create 本期不写批注内图。
三种格式的 `props` 复用 `setProps` / `setCustomProp` 同一套字段写 core / app / custom。
docx 的 `document.protection` 复用 `setProtection`。docx 的 `document.watermarks` 复用 `insertWatermark`。
docx 的 `document.theme` 写入 theme 部件；`document.pageBackground` / `document.pageBorders` 复用对应 apply。省略 `theme` 则不写该部件。

### exportMedia

调用方给出源 docx、xlsx 或 pptx、图 id、docx / xlsx / pptx OLE id、三种格式 3D 模型 id、docx 墨迹画布 id、pptx 视频 / 音频 id 或 docx 嵌入字体 id 和本地目标路径。
库打开包，图 id 按 `a:blip` 的关系写出媒体部件；带 `asvg:svgBlip` 时改走 SVG 关系。
OLE id 按 oleObject 或 package 关系写出 embeddings 部件；
3D 模型 id 按 model3d 关系写出 GLB 部件；
docx 墨迹画布 id 按 customXml 关系写出 InkML 部件；
docx `font:{n}` 按 fontTable 关系写出 odttf 原字节。包不写回。

### exportChartData

调用方给出源 docx 或 pptx、图表 id 和本地目标路径。库打开包，按图表关系找到 embedding 工作簿，把该部件原字节写出。包不写回。没有 embedding 则失败。xlsx 本期直接拒绝。

## 硬约束

- 自己实现 OPC。不要把 `office_oxide` 写进 `Cargo.toml`，不要加它的 git 子模块，不要在仓库里放它的源码目录。
- 查规范用 `standards/md/` 的章节索引。xlsx 单元格见 ECMA-376 §18.3.1.4，公式见 §18.3.1.96 与 §18.17，共享字符串见 §18.4，计算属性见 §18.2.2，
  工作表图与 `graphicFrame` 见 §20.5，图表部件见 §21.2，透视表见 §18.10，线程化批注见 MS-XLSX §2.1.17 / §2.1.18 / §2.3.7 / §2.6.203–2.6.207，占位批注见 ECMA-376 §18.7。
  docx 主文档见 §11.3.10，段落见 §17.3.1.22，样式见 §17.7，页眉页脚见 §17.10，批注见 §17.13，编号见 §17.9，域见 §17.16，图表见 §21.2，
  SmartArt / diagram 见 §21.4，`w14:paraId` 见 MS-DOCX §2.6.2.3，`commentEx` 见 MS-DOCX §2.5.3.1。
  pptx Presentation 部件见 §13.3.6，幻灯片见 §13.3.8 与 §19.3，页名单见 §19.2.1.34，自定义放映见 §19.2.1.5–19.2.1.7，
  形状与文本框见 §19.3.1.45 / §19.3.1.51，图片 `p:pic` 见 §19.3.1.37，`cNvPr` 见 §19.3.1.12，`p:graphicFrame` 见 §19.3.1.21，
  嵌入对象 `p:oleObj` 见 §19.3.2.4，Zoom 见 MS-PPTX §2.2.15 / §2.9–2.11，备注页见 §13.3.5 与 §19.3.2，讲义母版见 §19.3.1.24，
  现代批注见 MS-PPTX 2018/8（`p188:cmLst` / `p188:authorLst`）。包结构见 §9 与 §12.2。
- 不实现 Excel 计算引擎、Word 排版引擎、PowerPoint 放映引擎，也不实现图表渲染或 SmartArt 版式算法。不接受把整棵 `<p:timing>` 当 JSON 写入。
- 不支持 `.doc`、`.xls`、`.ppt`。
- XML 编辑是手术刀：不认识的命名空间、属性、子元素，只要没被当前 op 点到，就原样保留。

## 实现时允许的依赖（文档约定，代码阶段再写入 Cargo.toml）

库可以依赖常见的 ZIP 与 XML crate，例如 `zip`、`quick-xml`、`serde`、`serde_json`。xlsx / docx 写保护哈希可用 `sha2`、`getrandom`、`base64`。
禁止依赖任何把整份 Office 文档建成自己那套对象模型、再整包写出的高层 Office crate。

## 阶段边界

xlsx 与 docx 第一期已做完，人工打开见 [checklist-xlsx.md](checklist-xlsx.md)、[checklist-docx.md](checklist-docx.md)。
pptx 第一期实现已落地，人工打开见 [checklist-pptx.md](checklist-pptx.md)。
