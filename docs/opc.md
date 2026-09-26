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
| `/xl/drawings/drawingN.xml` | 仅当某张表声明了 `pictures` |
| `/xl/media/imageN.ext` | 仅当某张表声明了 `pictures` |

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

apply 插入工作表图时，可能新建 `/xl/drawings/drawingN.xml`（CT `application/vnd.openxmlformats-officedocument.drawing+xml`）、drawing 的 `.rels`，以及 `/xl/media/imageN.*`。工作表 `.rels` 补一条 drawing 关系（Type 以 `/drawing` 结尾），工作表 XML 写 `<drawing r:id="…"/>`。drawing 命名空间是 `http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing`（`xdr`）。apply 插入工作表图表时，可能再新建 `/xl/charts/chartN.xml`（CT `application/vnd.openxmlformats-officedocument.drawingml.chart+xml`）及 chart 的 `.rels`；drawing 关系 Type 以 `/chart` 结尾。不写 embedding。create 声明了 `sheets[].pictures` 时按同一套规则写 drawing 与 media，仍不写 chart 部件。

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
| `/word/settings.xml` | 最小文档设置。必须含 `compatibilityMode`，`val` 为 `15`（MS-DOCX §2.3.5），否则 Word 按兼容模式打开（默认 12） |

可选但 create 要写：

| 路径 | 作用 |
| --- | --- |
| `/docProps/core.xml` | 核心文件属性 |
| `/docProps/app.xml` | Application 写成 `ooxp` |
| `/word/header1.xml` | 仅当 `document.headers` 含默认页眉 |
| `/word/footer1.xml` | 仅当 `document.footers` 含默认页脚 |
| `/word/media/imageN.ext` | 仅当段落或页眉页脚声明了 `pictures` |
| `/word/diagrams/…` | 仅当段落或页眉页脚声明了 `smartArts` |

apply 插入脚注或尾注时，可能新建 `/word/footnotes.xml` 或 `/word/endnotes.xml`（CT 分别为 `…footnotes+xml` / `…endnotes+xml`），并在主文档 `.rels` 与 `[Content_Types].xml` 上补关系与 Override。apply 插入批注时，可能新建 `/word/comments.xml`（CT `…comments+xml`）、`/word/commentsExtended.xml`（CT `application/vnd.ms-word.commentsExtended+xml`）、`/word/people.xml`（CT `application/vnd.openxmlformats-officedocument.wordprocessingml.people+xml`）。`commentsIds` / `commentsExtensible` 只在原包已有时补条目，create 与默认插入都不新建这两份。`replyComment` 还会在父批注范围上再套一套范围标记，因此可能脏主文档或页眉页脚、注部件。apply 插入图表时，可能新建 `/word/charts/chartN.xml`（CT `application/vnd.openxmlformats-officedocument.drawingml.chart+xml`）、chart 的 `.rels`，以及 `/word/embeddings/….xlsx`（CT `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`），并在宿主 `.rels` 与 `[Content_Types].xml` 上补关系与 Override。apply 或 create 插入 SmartArt 时，可能新建 `/word/diagrams/` 下的 data / layout / colors / quickStyle（CT 分别为 `application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml`、`…diagramLayout+xml`、`…diagramColors+xml`、`…diagramQuickStyle+xml`），四条关系写在宿主 `.rels` 上（`diagramData` / `diagramLayout` / `diagramColors` / `diagramQuickStyle`）。改点时若对不上 diagram drawing 缓存，可能删除该缓存部件。create 不写脚注、尾注、批注、图表部件；create 可写 SmartArt 四件套。

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
| `/ppt/slides/_rels/slideN.xml.rels` | 指向 slideLayout1 |
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

第一期不写备注、讲义、`viewProps`、`tableStyles`、动画。

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

## 验收（第 2 项）

零脏保存：`open` 之后不调用 `set` / `remove`，再 `save`。每个部件的未压缩字节、以及 ZIP 里该条目的 CRC，都与打开时相同。库测试对 Excel 另存的 xlsx、以及 create 写出的 docx / pptx 各跑一遍。

领域 JSON 的三个动作不含原样保存。`apply` 的 `ops` 必须非空，见 [json-contract.md](json-contract.md)。命令行不能做「打开再原样写出」。这一条第一期放弃，原因写在三份清单的 O1。

Office 打开 create / apply 产物、以及 Office 自己另存 create 产物，见 `docs/checklist-xlsx.md`、`docs/checklist-docx.md`、`docs/checklist-pptx.md` 与 `manual-check/office/cases/opc-resave.ps1`。那是打开冒烟，不是零脏保存。
