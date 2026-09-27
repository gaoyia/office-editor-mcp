# 架构

状态：xlsx、docx、pptx 第一期已实现，并已通过 Microsoft Office 16.0 人工打开。验收只认官方 Office，不再要求 WPS。

## 产品形态

产品名是 `ooxp`，全称 Office Open XML Patch。一个 Rust 库，外加一个命令行工具。库只认领域 JSON。命令行把 stdin 或文件里的领域 JSON 交给库，再把结果写到 stdout 或磁盘。

本仓库不实现 MCP 服务。外部 MCP 把领域 JSON 原样交给库即可。不许为了信封去改领域 JSON，也不许把动作拆扁成一套 MCP 专用字段。

对齐 Transitional OOXML，也就是 Office 默认存出来的那种包。ECMA-376 示例里常见的 Strict 命名空间（`http://purl.oclc.org/ooxml/...`）只作对照，create 不按 Strict 生成。

## 三个动作

这三个动作是整套设计的地基，实现时不许改语义。

| 动作 | 真源 | 做什么 |
| --- | --- | --- |
| `inspect` | 已有 ZIP 包里的 XML | 返回带稳定 id 的精简 JSON 树，按调用方给的范围截断 |
| `apply` | 已有 ZIP 包里的 XML | 按 id 打补丁，只改目标节点，其余部件按原字节写回 |
| `create` | 领域 JSON | 从 JSON 生成一份新的完整包。这是唯一允许整包生成的动作 |

禁止把 inspect 树当成完整文档模型，改完之后整包重新生成。apply 的入参只能是操作列表，不能是整棵树。

xlsx 另有只读动作 `exportMedia`：按图 id 把媒体部件原字节写到本地。xlsx 的 `exportMedia` 认工作表图 id 与页眉页脚图 id，写出 `/xl/media/…`。pptx 的 `exportMedia` 认幻灯片图 id 与备注图 id，写出 `/ppt/media/…`。docx 与 pptx 另有 `exportChartData`：按图表 id 把 embedding 工作簿原字节写到本地。它们不改上述三个动作的语义，也不把字节塞进 inspect 树。xlsx 对 `exportChartData` 仍报 `InvalidField`（图表数据就在本工作簿单元格里）。

## 模块

建议用 Cargo workspace，两个 crate。xlsx、docx、pptx 已落地。信封解析在 `json`，按 `format` 分发在 `dispatch`，每种格式自己解析自己的 `scope` 与 `ops`。

```text
crates/ooxp/                 库
  error/                     错误身份、错误码、按语言渲染
    messages/                en.rs、zh_cn.rs；每种语言一个穷尽 match
  opc/                       ZIP、[Content_Types].xml、各级 .rels、按路径读写部件
  json/                      领域 JSON 信封
  dispatch/                  按 format 分发给 xlsx / docx / pptx
  xlsx/                      工作簿 inspect / apply / create
  docx/                      文档 inspect / apply / create
  chart/                     c:chart 部件；embedding 经本模块读写 xlsx
  pptx/                      演示文稿 inspect / apply / create
crates/ooxp-cli/             命令行，只依赖 ooxp。二进制名也是 ooxp
```

依赖方向只能是：`cli` → 库的门面；`dispatch` → `json` 与各格式模块；`xlsx` / `docx` / `pptx` → `opc` 与 `json`。`chart` 可调用 `xlsx` 读写 embedding；pptx 不直接引用 `xlsx` 或 `docx`。`json` 不依赖格式模块。各模块只依赖 `error`，`error` 不反向依赖 `json`、`opc` 或任何格式模块。禁止格式模块直接操作 ZIP 字节而不走 `opc`。`SetCellOp`、工作表 `scope`、`setParaText` 这类类型放在对应格式模块里。

### opc

把 `.docx`、`.xlsx`、`.pptx` 都当成 OPC 包。负责打开、列出部件、按路径读字节、标记脏部件、保存。保存时没改过的部件保持原来的压缩方法和未压缩字节，不重排 ZIP 中央目录里未改条目的相对顺序，也不去美化无关 XML。细节见 [opc.md](opc.md)。

### json

只处理领域信封：`schemaVersion`、`action`、各动作的必填字段。不认识的字段在第一期直接报错，避免默默丢数据。契约见 [json-contract.md](json-contract.md)。

### xlsx

读 `xl/workbook.xml` 拿工作表名单和关系。按范围读对应 `sheetN.xml`，需要时再读 `xl/sharedStrings.xml`。工作簿与每张 `worksheet` 始终带 `pictureCount`。工作簿、每张 `worksheet` 与每张 `chartsheet` 始终带 `chartCount`。每张表始终带 `hasHeaderFooter`。`scope.pictures` 为 true 时读该表 `<drawing>` 指向的 drawing 部件，列出 `xdr:pic`，不读媒体字节。`scope.charts` 为 true 时列出 `xdr:graphicFrame` 里的图表，系列一次倒出；数值优先读 `c:f` 指向的单元格。`scope.headerFooter` 为 true 时展开六个页眉页脚槽位的原文；同时 `pictures: true` 时再列出 `drawingHF` 里的图。apply 可改目标单元格，也可换、插、删工作表图，并可改 `oneCell` / `absolute` 的尺寸与锚点。也可插、删、改工作表图表：改系列时写本表单元格并回写 cache，插图时在 `dataFrom` 写下数据表。也可新建或删除图表工作表：数据仍写在点名的普通表上，图表工作表本身没有 `sheetData`。也可按槽位改页眉页脚原文，并可对 `worksheet` 的页眉槽位插、换、删 DrawingML 图。插图或插图表可能新建 `/xl/drawings/drawingN.xml`、`/xl/media/…`、`/xl/charts/chartN.xml` 以及工作表上的 `<drawing>`。`insertChartSheet` 再新建 `/xl/chartsheets/sheetN.xml`。往页眉插图可能新建另一份 drawing 并写 `<drawingHF>`。改单元格之后，把工作簿标成打开时需要重算，必要时删掉 `xl/calcChain.xml`。不改 `styles.xml`，不求值公式。不算列宽行高。不写 embedding，不对 xlsx 做 `exportChartData`。不编辑 `legacyDrawingHF` / VML。create 本期不写页眉页脚，也不写 `kind: chartsheet`。

### docx

主文档由根关系里 `/officeDocument` 指向，常见路径 `/word/document.xml`。inspect 读该部件里的 `w:p`，返回大纲和按范围截断的段落；并始终列出页眉、页脚容器、各节页面与页眉页脚属性，以及每节是否链接到前一节。文档根始终带正文 `pictureCount`、`textBoxCount`、`footnoteCount`、`endnoteCount`、`commentCount`、`chartCount`、`smartArtCount`、`tableCount`、`revisionCount`、`hyperlinkCount`、`bookmarkCount`、`fieldCount`、`numberingCount`、`styleCount`，以及整篇 `mirrorMargins` / `gutterAtTop`。页眉页脚容器始终带前两项与 `chartCount`、`smartArtCount`、`tableCount`、`revisionCount`、`hyperlinkCount`、`bookmarkCount`、`fieldCount`。`scope.pictures` / `scope.textBoxes` 为 true 时列出页眉页脚图或框；同时给了正文切片则段落上也列出。`scope.textBoxContent` 再展开框内段落。`scope.notes` 为 true 时列出用户脚注、尾注与分隔符；注内段落一次倒出。`scope.comments` 为 true 时列出批注；批注内段落一次倒出。`scope.charts` 为 true 时列出正文与页眉页脚里的嵌入图表；系列一次倒出。`scope.smartArt` 为 true 时列出正文与页眉页脚里的 SmartArt；可见点一次倒出。`scope.tables` 为 true 时列出正文与页眉页脚里的表；单元格段落一次倒出。`scope.revisions` 为 true 时列出正文与页眉页脚里的 run 级插入、删除与属性修订。`scope.hyperlinks` 为 true 时列出正文与页眉页脚里的 `w:hyperlink`。`scope.bookmarks` 为 true 时列出正文与页眉页脚里的配对书签。`scope.fields` 为 true 时列出正文与页眉页脚里的 `w:fldChar` / `w:fldSimple`。`scope.numbering` 为 true 时列出 `numbering.xml` 里的编号实例。`scope.styles` 为 true 时列出 `styles.xml` 里的样式定义。apply 可改被点名段落的可见文本（主文档、页眉页脚、框内、注内或批注内），可在主文档、页眉页脚与表格单元格插删 `w:p`，可插删 `w:tbl` 以及行与列，并可按 `gridSpan` / `vMerge` 合并与拆分，可接受或拒绝被点名的修订，可插、删、改超链接地址与显示文本，可插、删、改书签名，可插、改指令、删域（不求值），可写入编号定义并把段落套上或去掉编号，可给段落换 `pStyle` 并改被点名段落样式的字体、字号、加粗、颜色与段前段后，可改节上 `w:pgMar` / `w:pgSz` / `w:cols` / `w:vAlign` / `w:lnNumType` / `w:docGrid` / `w:rtlGutter`，可改整篇 `w:mirrorMargins` / `w:gutterAtTop`，可改节上页眉/页脚距离，也可断开或重新链接某节某类型的页眉页脚（断链时抄走 header/footer 自己的 `.rels`，媒体仍共用），也可为空白槽位新建页眉页脚部件，并可写本节 `w:titlePg` 与整篇 `w:evenAndOddHeaders`。页眉、页脚、正文、注和批注里的 DrawingML 图片可换、插、删，并可改尺寸与绕排。文本框可插、删，可改尺寸、绕排、填色、描边、横竖排与链接。脚注、尾注可插、删，可改编号格式（整篇同时写 `settings.xml` 与各节 `sectPr`，Word 读节属性）。批注可插、删、回复、改字、改作者与完成标记。嵌入图表可插、删、改标题、改系列数据、改类型与图例，并可改宿主 drawing 的尺寸与绕排。圆环、雷达、气泡与组合图走同一套 op；透视图表只改标题、图例、标签、删除与尺寸绕排，不改数。股价、曲面、chartex、次轴、趋势线、误差线除删除与尺寸绕排外报 `NotImplemented`。SmartArt 可插、删、改各点文字、增删点、改配色与快速样式，并可改宿主 drawing 的尺寸与绕排。`exportMedia` 按图 id 写出 `/word/media/…` 原字节。`exportChartData` 按图表 id 写出 embedding xlsx 原字节。不整篇从 JSON 重建。create 生成最小 Transitional 包，含 `styles.xml`（Normal 与 Heading1–3）和 `settings.xml`（`compatibilityMode` 为 15），不生成多节、不写脚注尾注、不写批注、不写图表、不写修订、不写超链接与书签、不写域、不写编号。段落上可带文本框、DrawingML 图与 SmartArt。给了 `document.tables` 时可在正文写表；页眉页脚可带 `tables`。给了 `document.headers` / `footers` 时可写 `default` / `even` / `first`（给了 even 则写 `evenAndOddHeaders`，给了 first 则写 `titlePg`）。不改注区怎么画，不改批注气泡怎么排，不画图表，不算 SmartArt 版式，不算表列宽。

### pptx

入口由根关系里 `/officeDocument` 指向，常见路径 `/ppt/presentation.xml`。幻灯片顺序跟 `p:sldIdLst`，不跟文件名。inspect 默认只返回页名单与各页 `pictureCount`、`chartCount`、`smartArtCount`、`commentCount`、`hasNotes`；要形状必须给 `scope.slide`，要图名单必须给 `scope.pictures`，要图表名单必须给 `scope.charts`，要 SmartArt 名单必须给 `scope.smartArt`，要备注形状必须给 `scope.notes`，要批注名单必须给 `scope.comments`。apply 可改被点名形状的 `p:txBody` 可见文字（含备注形状，缩略图占位符除外），可用 `setNotesText` 按页改演讲者备注；也可换、插、删幻灯片或备注页上的 `p:pic` 并改尺寸与画布坐标；也可插、删、改幻灯片上的嵌入图表（改系列时 cache 与 embedding 双写）；也可插、删、改幻灯片上的 SmartArt（改点时写 data 部件，对不上 drawing 缓存则删缓存）；也可插、删、回复、改字、改作者与完成标记现代批注（MS-PPTX 2018/8 `p188:cmLst`，不写 ECMA-376 §19.4 旧式部件）。不删形状重建，不改 `<p:timing>`。`exportMedia` 按图 id 写出 `/ppt/media/…`。`exportChartData` 按图表 id 写出 embedding。create 生成最小 Transitional 包，必含 theme、slideMaster、slideLayout、presentation、各页 slide、presProps。给了 `slides[].pictures` 时复用 `insertPicture`，写入 `/ppt/media/…` 与该页 `p:pic`。给了 `slides[].charts` 时复用 `insertChart`，写入 `/ppt/charts/…`、embedding 与 `p:graphicFrame`。给了 `slides[].smartArts` 时复用 `insertSmartArt`，写入 `/ppt/diagrams/…` 与图示 `p:graphicFrame`。create 本期不写备注、不写批注。母版内容、动画第一期不进树也不改。`mc:AlternateContent` 只走第一个 `Choice`。

### cli

从文件或 stdin 读入一份领域 JSON，按 `action` 分发。inspect 把树打到 stdout。create 和 apply 写出目标 Office 文件。`exportMedia` 写出媒体文件。`exportChartData` 写出图表 embedding。契约见 [cli.md](cli.md)。

## 数据流

### inspect

调用方给出路径和范围。库打开 OPC 包，定位格式入口部件（xlsx 是 Workbook，docx 是 Main Document，pptx 是 Presentation）。按范围读必要的 XML，生成带 id 的树。xlsx 默认不读完全部单元格，也不展开 `pictures`、`charts` 或页眉原文（仍返回 `pictureCount`、`chartCount` 与 `hasHeaderFooter`）。docx 默认不返回正文 `paragraphs` 也不展开页眉页脚段落、任何 `pictures`、`textBoxes`、注名单、批注名单、图表名单、SmartArt 名单、表名单、修订名单、超链接名单、书签名单、域名单、编号名单或样式名单，pptx 默认不返回 `shapes`，也不展开 `pictures` / `charts` / `smartArts` / `notes` / `comments`（仍返回根与各页 `pictureCount`、`chartCount`、`smartArtCount`、`commentCount`，以及各页 `hasNotes`）。包不写回。

### apply

调用方给出路径和 `ops`。库打开包，按每条 op 的 id 找到 XML 节点，只改这些节点。xlsx 在改了单元格之后，把工作簿标成打开时需要重算。xlsx 也可改被点名的工作表图（换、插、删、改尺寸、改锚点），也可插删改工作表图表，脏 drawing、chart 部件、工作表 XML、关系与媒体。xlsx 也可新建或删除图表工作表，脏 workbook 的 `sheets`、chartsheet 部件及其 drawing / chart。xlsx 也可改工作表上的页眉页脚原文，并可对 `worksheet` 页眉槽位插换删 DrawingML 图，脏 `headerFooter` / `drawingHF` 与对应 drawing。docx 可改被点名的段落，可在主文档、页眉页脚与表格单元格插删段落，可插删表以及行与列并合并拆分单元格，可接受或拒绝 run 级修订，可插删改超链接与书签，可插改删域指令（不求值），可写入编号定义并把段落套上或去掉编号，可给段落换样式并改样式定义上的一批常见属性，可改节上页边距、纸张、分栏、垂直对齐、行号、文档网格与装订线侧，可改整篇镜像边距与装订线在上，可改节上页眉页脚距边界，也可改节上页眉页脚是否链接前节，也可为空白槽位新建页眉页脚，也可写 `titlePg` 与 `evenAndOddHeaders`，也可改页眉、页脚、正文、注或批注里被点名的 DrawingML 图与文本框，也可插删脚注尾注并改编号属性，也可插删回复批注并改完成标记，也可插删改嵌入图表，也可插删改 SmartArt。pptx 可改被点名形状的可见文字（含备注形状），可用 `setNotesText` 按页改或新建演讲者备注，也可改被点名的幻灯片图与备注图（换、插、删、改尺寸、改画布坐标），也可插删改幻灯片图表，也可插删改幻灯片 SmartArt，也可插删回复并改现代批注，脏该页 XML、该页 `.rels`、notesSlide、必要时 notesMaster 与 presentation、`/ppt/charts/`、embedding、`/ppt/diagrams/`、`/ppt/comments/` 与 `/ppt/authors.xml`。保存时只重写脏部件。

### create

调用方给出格式、输出路径和内容对象。库在内存里生成一套最小可打开的 Transitional 部件，打成新 ZIP。不读取任何已有文件当模板，除非以后另开「从模板复制」的动作。xlsx 的 `sheets[].pictures` 复用 `insertPicture`，`sheets[].charts` 复用 `insertChart`（数据写本表单元格，不写 embedding）。docx 段落与默认页眉页脚上的 `pictures` 同样复用插图。pptx 的 `slides[].pictures` 复用 `insertPicture`，`slides[].charts` 复用 `insertChart`（数据写 embedding），`slides[].smartArts` 复用 `insertSmartArt`。docx create 仍不写图表。

### exportMedia

调用方给出源 docx、xlsx 或 pptx、图 id 和本地目标路径。库打开包，按图 id 找到 `a:blip` 的关系，把目标媒体部件原字节写出。包不写回。

### exportChartData

调用方给出源 docx 或 pptx、图表 id 和本地目标路径。库打开包，按图表关系找到 embedding 工作簿，把该部件原字节写出。包不写回。没有 embedding 则失败。xlsx 本期直接拒绝。

## 硬约束

- 自己实现 OPC。不要把 `office_oxide` 写进 `Cargo.toml`，不要加它的 git 子模块，不要在仓库里放它的源码目录。
- 查规范用 `standards/md/` 的章节索引。xlsx 单元格见 ECMA-376 §18.3.1.4，公式见 §18.3.1.96 与 §18.17，共享字符串见 §18.4，计算属性见 §18.2.2，工作表图与 `graphicFrame` 见 §20.5，图表部件见 §21.2。docx 主文档见 §11.3.10，段落见 §17.3.1.22，样式见 §17.7，页眉页脚见 §17.10，批注见 §17.13，编号见 §17.9，域见 §17.16，图表见 §21.2，SmartArt / diagram 见 §21.4，`w14:paraId` 见 MS-DOCX §2.6.2.3，`commentEx` 见 MS-DOCX §2.5.3.1。pptx Presentation 部件见 §13.3.6，幻灯片见 §13.3.8 与 §19.3，页名单见 §19.2.1.34，形状与文本框见 §19.3.1.45 / §19.3.1.51，图片 `p:pic` 见 §19.3.1.37，`cNvPr` 见 §19.3.1.12，`p:graphicFrame` 见 §19.3.1.21，备注页见 §13.3.5 与 §19.3.2，现代批注见 MS-PPTX 2018/8（`p188:cmLst` / `p188:authorLst`）。包结构见 §9 与 §12.2。
- 不实现 Excel 计算引擎、Word 排版引擎、PowerPoint 完整动画时间轴，也不实现图表渲染或 SmartArt 版式算法。
- 不支持 `.doc`、`.xls`、`.ppt`。
- XML 编辑是手术刀：不认识的命名空间、属性、子元素，只要没被当前 op 点到，就原样保留。

## 实现时允许的依赖（文档约定，代码阶段再写入 Cargo.toml）

库可以依赖常见的 ZIP 与 XML crate，例如 `zip`、`quick-xml`、`serde`、`serde_json`。禁止依赖任何把整份 Office 文档建成自己那套对象模型、再整包写出的高层 Office crate。

## 阶段边界

xlsx 与 docx 第一期已做完，人工打开见 [checklist-xlsx.md](checklist-xlsx.md)、[checklist-docx.md](checklist-docx.md)。pptx 第一期实现已落地，人工打开见 [checklist-pptx.md](checklist-pptx.md)。
