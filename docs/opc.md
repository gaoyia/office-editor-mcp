# OPC 层

状态：已实现。人工用 Excel / Word / PowerPoint 打开另存文件的检查见 `docs/checklist-xlsx.md` 的 O1、O2。

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
| `/xl/styles.xml` | 最小样式表。Excel 和 WPS 都更认带这份文件的包 |

可选但 create 要写：

| 路径 | 作用 |
| --- | --- |
| `/docProps/core.xml` | 核心文件属性，dc:title 可空 |
| `/docProps/app.xml` | Application 写成 `ooxp`，Sheet 名单与 workbook 一致 |

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

## create 时的最小包（docx）

docx create 同样生成 Transitional 包，命名空间用 2006 那套。

必有部件：

| 路径 | 作用 |
| --- | --- |
| `/[Content_Types].xml` | 内容类型 |
| `/_rels/.rels` | 指向 Main Document，以及可选的核心属性 |
| `/word/document.xml` | 主文档，至少一个 `w:p` |
| `/word/_rels/document.xml.rels` | 指向 styles |
| `/word/styles.xml` | 至少含 Normal、Heading1、Heading2、Heading3 |

可选但 create 要写：

| 路径 | 作用 |
| --- | --- |
| `/docProps/core.xml` | 核心文件属性 |
| `/docProps/app.xml` | Application 写成 `ooxp` |

Main Document 的 Content Type：

`application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml`

Style Definitions：

`application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml`

WordprocessingML 主命名空间：

`http://schemas.openxmlformats.org/wordprocessingml/2006/main`

`w14` 命名空间（paraId）：

`http://schemas.microsoft.com/office/word/2010/wordml`

根元素须把 `w14` 列入 `mc:Ignorable`，依据 MS-DOCX §2.2.4。

## 验收（第 2 项）

打开一份 Excel、一份 Word、一份 PowerPoint 各另存一次，不改任何部件，再交给原软件打开。三份都要能打开，内容看起来与另存前相同。这一项在用户确认文档之后、实现 opc 时再做。
