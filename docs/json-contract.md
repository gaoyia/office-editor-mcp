# 领域 JSON 契约

`schemaVersion` 固定为 `1`。库和命令行只认这一层。外部 MCP 也必须整份使用这一层，不许改字段名和含义，不许拆扁成工具参数。

xlsx / docx / pptx 的稳定 id、inspect、create 与专用 apply 分别见 [json-contract-xlsx.md](json-contract-xlsx.md)、
[json-contract-docx.md](json-contract-docx.md)、[json-contract-pptx.md](json-contract-pptx.md)。

机器可读副本入口是 [json-schema/request.schema.json](json-schema/request.schema.json)。
xlsx / docx / pptx 的片段分别是 [xlsx.schema.json](json-schema/xlsx.schema.json)、
[docx.schema.json](json-schema/docx.schema.json)、[pptx.schema.json](json-schema/pptx.schema.json)。与 Markdown 冲突时以本文及分册为准。

xlsx、docx、pptx 的 inspect、create、apply 已经按本文实现。docx 另有 `exportMedia` 与 `exportChartData`。
docx 的 `exportMedia` 认图 id、OLE id、墨迹画布 id、3D 模型 id 与嵌入字体 id：图 id 写出 `/word/media/…`，OLE id 写出 embeddings 部件，
墨迹画布 id 写出 `/word/ink/…` 的 InkML 原字节，
3D 模型 id 写出 `/word/media/…` 的 GLB 原字节，
`font:{n}` 写出 `/word/fonts/…` 的 odttf 原字节（仍是混淆后的，不还原）。
xlsx 的 `exportMedia` 认工作表图 id、页眉页脚图 id、OLE id 与 3D 模型 id：图 id 写出 `/xl/media/…`，OLE id 写出 embeddings 部件，
3D 模型 id 写出 `/xl/media/…` 的 GLB 原字节。
pptx 的 `exportMedia` 认幻灯片图 id、备注图 id、批注内图 id、母版 / 版式图 id、视频 id、音频 id、OLE id 与 3D 模型 id：图 / 视频 / 音频写出 `/ppt/media/…`，OLE 写出 embeddings 部件，
3D 模型 id 写出 `/ppt/media/…` 的 GLB 原字节。
xlsx 的 `scope.charts` 与图表 op 认工作表 / 图表工作表上的图；`scope.headerFooter` 展开工作表上的页眉页脚槽位。
pptx 的 `scope.charts` 与图表 op 认幻灯片、备注页与幻灯片母版 / 版式上的图表 `p:graphicFrame`；`exportChartData` 认这些图表 id，写出 embedding。
pptx 的 `scope.smartArt` 与 SmartArt op 认幻灯片、备注页与幻灯片母版 / 版式上的 diagram `p:graphicFrame`。
pptx 的 `scope.tables` 与表格 op 认幻灯片、备注页与幻灯片母版 / 版式上 DrawingML 表 `p:graphicFrame`（`a:tbl`，ECMA-376 §21.1.3.13），不认 `w:tbl`，也不认工作表表格。
pptx 的 `scope.connectors` 列出该页 `p:cxnSp`；`scope.notes` 与 `scope.connectors` 同开时备注上也列；`scope.masters` 同开时母版 / 版式上也列。
apply 用 `insertConnector` / `deleteConnector` / `setConnector` / `groupShapes` / `ungroup`，
以及 `insertShape` 的 `prst` / `geom`、`setShapeGeom`。
xlsx 的 `groupShapes` / `ungroup` 认工作表文本框、无 `txBox` 形状与连接线。
docx 的 `scope.connectors` 列出正文与页眉页脚里的 `wps` 连接线；
apply 用 `insertConnector` / `deleteConnector` / `groupShapes` / `ungroup`。
docx 的 `scope.inkCanvases` 列出正文与页眉页脚里的墨迹画布（`wpc` 画布加 `w14:contentPart` 加 InkML 部件）；
apply 用 `insertInkCanvas` / `replaceInkCanvas` / `deleteInkCanvas`。xlsx / pptx 拒这些键与 op。
三种格式的 `scope.model3ds` 列出 3D 模型（`am3d:model3d`，GLB 部件）。
apply 用 `insertModel3d` / `replaceModel3d` / `deleteModel3d`。不是图，不进 `pictures`。
xlsx / pptx 的 `scope.maths` 列出 drawing 公式框（`a14:m` 包 OMML）。
apply 复用 `insertMath` / `setMath` / `deleteMath`。不是单元格 `f`，也不把 Word 的 `expr` 搬过去。
docx 的 `scope.maths` 仍是段落 `oMath`，见 [docx 分册](json-contract-docx.md#insertmath)。
xlsx 的 `scope.oleObjects` 与 OLE op 认工作表 `<oleObject>`（ECMA-376 §18.3.1.59），不认 `@link` 与 ActiveX `controls`，不把嵌入字节或预览图塞进树。
pptx 的 `scope.comments` 与批注 op 认该页 2018 线程化批注（`p188:cmLst`），不认 ECMA-376 §19.4 旧式部件。
pptx 的 `scope.timing` 列出该页 `<p:timing>` 效果摘要（含 `delay` / `dur` / `repeat` / 声音名 / 段落 / 点对象）；`scope.notes` 与 `scope.timing` 同开时备注对象带 `timing`；`scope.masters` 与 `scope.timing` 同开时母版 / 版式带 `timing`；`scope.notesMaster` / `scope.handout` 与 `scope.timing` 同开时备注母版 / 讲义带 `timing`。
apply 用 `insertAnim` / `setAnim` / `deleteAnim` / `moveAnim` 按模板增删改预设效果，可写 delay、dur、repeat、效果声音与按段；第一条 `with` / `after` 进片自动播。不接受整棵时间轴树。
`setTransition` 还可写自动进片、方向、`p15:prstTrans`、切换声音、`split` 的 `orient`、`wheel` 的 `spokes`，以及母版 / 版式上的切换。
pptx 的 `scope.masters` 列出幻灯片母版与版式、占位符与装饰形（含该母版 theme 的配色与字体），根上始终带 `slideSize`，apply 可用 `setSlideLayout` 换页的版式关系，
也可用 `insertLayout` / `deleteLayout` / `insertPlaceholder` / `deletePlaceholder` / `insertMaster` / `deleteMaster` 插删版式、占位符与母版，
也可用 `insertShape` / `deleteShape` 插删母版 / 版式上没有 `p:ph` 的装饰形，
也可用 `setSlideSize` / `setThemeColors` / `setThemeFonts` 改画布尺寸与 theme 配色 / 字体，
也可用 `setShapeText` / `setShapeFormat` / `setShapePos` / `setShapeSize` 改占位符与装饰形。
装饰形还可用 `setShapeGeom`。
pptx 的 `scope.handout` 列出讲义母版页眉页脚槽位与占位符，apply 用 `setHandoutText` 改四个槽位的字，也可用 `insertPlaceholder` / `deletePlaceholder` / `setShapeText` 改讲义占位符。
pptx 的 `scope.notesMaster` 列出备注母版占位符，apply 用同一套占位符 op 与预设动画。
pptx 的 `scope.hyperlinks` 与超链接 op 认幻灯片文本形状上的形状级 `p:cNvPr/a:hlinkClick` 与 run 级 `a:rPr/a:hlinkClick`（跳转 / 文件 / URL），不认图、表、图表、SmartArt、备注或母版上的链。
pptx 的 `scope.fields` 列出幻灯片文本形状里的 `a:fld`；apply 用 `insertField` / `setField` / `deleteField` 写页码与日期域，不求值。
xlsx 仍拒 `scope.fields` 与这三条。docx 仍写 `w:fldChar` / `instr`，并可按 `code` 写常用域与窗体 `ffData`。
pptx 的 `scope.media` 与媒体 op 认幻灯片、备注页与幻灯片母版 / 版式上带 `a:videoFile` / `a:audioFile` 的 `p:pic`，与普通图分家。
inspect 带裁剪、书签与字幕轨；apply 用 `setMedia` 改这些字段，不写 fade，不写 timing。
pptx 的 `scope.sections` 与幻灯片节 op 认 presentation 上 `p14:sectionLst`（MS-PPTX §2.3.1.25）的分组元数据，页顺序仍以 `p:sldIdLst` 为准；不要和 docx 的 `w:sectPr` 节混淆。
pptx 的 `scope.customShows` 与自定义放映 op 认 `p:custShowLst`（ECMA-376 §19.2.1.5–19.2.1.7），页引用是 presentation 关系 `r:id`，不实现放映引擎。
pptx 的 `scope.oleObjects` 与 OLE op 认幻灯片上带嵌入关系的 `p:oleObj`（`p:graphicFrame`，ECMA-376 §19.3.2.4），
不认 `p:link` 与 ActiveX `p:control`，不把嵌入字节或预览图塞进树。
pptx 的 `scope.zooms` 与 Zoom op 认幻灯片上的 `sldZm` / `sectionZm` / `summaryZm`（MS-PPTX §2.2.15），Fallback 预览不进 `pictures`，不实现放映跳转。
xlsx 的 `scope.comments` 与批注 op 认工作表上的 MS-XLSX 线程化批注（`threadedComment`），不认作者不以 `tc=` 开头的旧式 VML 批注。
xlsx 的 `scope.textBoxes` 与文本框 op、`setShapeText` / `setShapeFormat` 认工作表 DrawingML 文本框（`xdr:sp` 且 `txBox`）。
xlsx 的 `scope.shapes` 列出工作表上无 `txBox` 的 `xdr:sp`；`scope.connectors` 列出工作表 `xdr:cxnSp`。
apply 用 `insertShape` / `deleteShape` / `insertConnector` / `deleteConnector`。
`setTextBoxSize` / `setTextBoxAnchor` 也认形状 id 与连接线 id。
xlsx 的 `scope.smartArt` 与 SmartArt op 认工作表 diagram `xdr:graphicFrame`。
xlsx 的 `scope.hyperlinks` 与超链接 op 认工作表 `<hyperlinks>`（ECMA-376 §18.3.1.47），不认 `HYPERLINK()` 公式，也不认图或框上的 DrawingML 链。
xlsx 的 `scope.sparklines` 与迷你图 op 认工作表 `extLst` 里的 `x14:sparklineGroups`（MS-XLSX §2.4.8），可读写颜色、标记与坐标轴，不把折线画进树。
xlsx 的 `scope.connections` 与连接 op 认工作簿 `/xl/connections.xml`（ECMA-376 §18.13），不刷新、不把查询结果写进单元格。
xlsx 的 `scope.externalLinks` 与外部引用 op 认工作簿 `<externalReferences>`（ECMA-376 §18.14），不打开目标文件，不求值。
xlsx 的 `scope.customViews` 与自定义视图 op 认工作簿 `<customWorkbookViews>`（ECMA-376 §18.2.3 / §18.2.4），不按视图恢复隐藏行 / 列。
xlsx 可用 `setProtection` 开关无密码的工作表 `sheetProtection` 与工作簿 `workbookProtection`。xlsx 的 `exportChartData` 仍拒绝。
人工打开只认 Microsoft Office，见三份 checklist。

## 信封

每份请求都是一个 JSON 对象，必须含：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `schemaVersion` | 整数 | 必须是 `1` |
| `action` | 字符串 | `inspect`、`create`、`apply`、`exportMedia`、`exportChartData` 之一 |
| `locale` | 字符串 | 可选。错误 `message` 的语言偏好，见 [errors.md](errors.md) |

`locale` 不属于动作参数，五种动作都认。取值宽松匹配：`zh`、`zh-CN`、`zh-Hans`、`zh-SG` 出中文；`en` 及其地区变体出英文；不认识的标签（包括空串）退回英文，不报 `InvalidField`。
这是信封里唯一一处对未知取值不报错的字段：语言只影响呈现，不该挡下文档操作。优先级是请求 `locale` 高于命令行 `--lang`，再高于环境变量 `OOXP_LANG`，都没有时是英文。外部 MCP 没有后两档，只能写请求里的 `locale`。

另外按动作需要 `path`、`format`、`scope`、`workbook`、`document`、`presentation`、`ops`、`id`、`output` 等字段，见下文。
禁止在请求里放一整棵 inspect 树来表示「把文档改成这样」。create 的 xlsx 用 `workbook`，docx 用 `document`，pptx 用 `presentation`，三者不能同时出现。

`format` 取值：`xlsx`、`docx`、`pptx`。不认 `.xlsm`。可省略，省略时从 `path` 的扩展名推断。推断不出就报 `UnsupportedFormat`。

`path` 是本地文件系统路径。Windows 反斜杠和正斜杠都接受。

## 稳定 id

id 是给模型用的字符串，必须能从文件内容稳定算出来，不能依赖本次运行的内存地址或随机数。

解析规则按格式走。id 里的工作表名、段落 id、形状 id 都按文件里的原文，不做 Unicode 正规化。

三种格式的 id 表见 [xlsx](json-contract-xlsx.md#稳定-id)、[docx](json-contract-docx.md#稳定-id)、[pptx](json-contract-pptx.md#稳定-id)。

## inspect

成功响应的对象字段按各分册响应节示例的语义顺序写出，不是按字母排序。命令行打印同一棵树，见 [cli.md](cli.md)。

### 请求

```json
{
  "schemaVersion": 1,
  "action": "inspect",
  "path": "C:\\data\\book.xlsx",
  "format": "xlsx",
  "scope": {
    "sheet": "Sheet1",
    "range": "A1:D20",
    "maxCells": 5000
  }
}
```

xlsx、docx、pptx 共用 `scope` 这个字段名，里面的键按格式分，混用报 `InvalidField`。

xlsx 的 scope 与响应见 [json-contract-xlsx.md](json-contract-xlsx.md#inspect)。
docx 见 [json-contract-docx.md](json-contract-docx.md#inspect)。
pptx 见 [json-contract-pptx.md](json-contract-pptx.md#inspect)。

### 文档属性（三格式共用）

xlsx、docx、pptx 的根上始终带 `props`，不要另给 scope 键。
core / app / custom 是包根关系指向的部件（ECMA-376 §15.2.12，常见 `/docProps/core.xml`、`/docProps/app.xml`、`/docProps/custom.xml`），三种格式同一套字段名。
没有某部件时，对应字段为 `null`，`custom` 为空数组，不报错。

`props` 字段顺序：`title`、`subject`、`creator`、`description`、`keywords`、`category`、`lastModifiedBy`、`created`、`modified`、`company`、`manager`、`custom`。

| 字段 | 来源 | 说明 |
| --- | --- | --- |
| `title` | `dc:title` | 文件标题。没有元素为 `null`；空元素为 `""` |
| `subject` | `dc:subject` | Word / Excel / PowerPoint 文件属性对话框里的「主题」 |
| `creator` | `dc:creator` | 核心属性里的作者。不要跟批注 `author`、修订痕迹上的作者混 |
| `description` | `dc:description` | 备注 / 说明 |
| `keywords` | `cp:keywords` | 关键字 |
| `category` | `cp:category` | 类别 |
| `lastModifiedBy` | `cp:lastModifiedBy` | 只读。apply 不改 |
| `created` | `dcterms:created` | 只读。值为元素文本原文 |
| `modified` | `dcterms:modified` | 只读。值为元素文本原文 |
| `company` | app.xml `Company` | 公司。未点名则留下 |
| `manager` | app.xml `Manager` | 经理 |
| `custom` | custom.xml 的 `property` | 只列名与值，不把整份部件倒进树 |

`custom` 按文档顺序。没有 custom 部件或没有 `property` 时为 `[]`。每条字段顺序：`id`、`name`、`type`、`value`。
`id` 是 `prop:{name}`，`{name}` 取 `property/@name` 原文。`name` 为空的条目不进树。
`type` 是值子元素的 local name，常见 `lpwstr`、`i4`、`r8`、`bool`、`filetime`、`date`。
`value`：`lpwstr` 为字符串；`i4` / `int` 能解析成整数则写整数；`r8` / `r4` 能解析成数字则写数字；`bool` 为 JSON 布尔（`true` / `1` / `on` 为 `true`）；其余把子孙文本拼成字符串。
带 `linkTarget` 的条目仍列出缓存值，树上不另给链接字段。同一 `name` 出现两次时都列出，apply 命中文档顺序第一处。

不把 `Application`、`Pages`、`Words`、`Slides`、`HeadingPairs`、`TitlesOfParts`、`DocSecurity`、数字签名或缩略图放进树。这些未点名时 apply 原样留下。

## create

只有 create 允许从 JSON 生成整包。

xlsx 的 `workbook` 见 [json-contract-xlsx.md](json-contract-xlsx.md#create)。
docx 的 `document` 见 [json-contract-docx.md](json-contract-docx.md#create)。
pptx 的 `presentation` 见 [json-contract-pptx.md](json-contract-pptx.md#create)。

## apply

apply 的入参是操作列表，不是树。下面这种写法非法，必须报 `InvalidField`：

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "book.xlsx",
  "sheets": [],
  "cells": []
}
```

合法请求：

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "output": "C:\\data\\book.patched.xlsx",
  "ops": [
    {
      "op": "setCell",
      "id": "cell:Sheet1!A1",
      "value": 42
    },
    {
      "op": "setCell",
      "id": "cell:Sheet1!B1",
      "formula": "=SUM(A1:A10)"
    },
    {
      "op": "setCell",
      "id": "cell:Sheet1!C1",
      "value": null
    }
  ]
}
```

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `path` | 是 | 源文件 |
| `output` | 否 | 省略则覆盖 `path` |
| `ops` | 是 | 非空数组。按数组顺序执行，中途失败则整次 apply 不写文件。空数组不是原样保存，报 `EmptyArray`。零脏写回只在 OPC 层，见 [opc.md](opc.md) |

create 与 apply 落盘时先写入目标同目录的临时文件，再改名为目标路径。这样写到一半失败时，原文件还在。

只属于某一种格式的 op 正文见 [xlsx](json-contract-xlsx.md#apply)、[docx](json-contract-docx.md#apply)、[pptx](json-contract-pptx.md#apply)。
跨格式 op（图、图表、SmartArt、批注、超链接、文本框、表格插删行列、`setProps`、`setProtection`、`setShapeText` 等）仍写在本文。

不认识的 `op` 报 `UnsupportedOp`，不要忽略。xlsx 认 `setCell`、`setCellStyle`、`setHeaderFooterText`、`replacePicture`、`insertPicture`、
`deletePicture`、`setPictureSize`、`setPictureAnchor`、以及图表 op（`insertChart`、`insertChartSheet`、`deleteChartSheet`、
`deleteChart`、`setChartTitle`、`setChartSeries`、`addChartSeries`、`deleteChartSeries`、`setChartType`、`setChartAxisTitle`、
`setChartLegend`、`setChartDataLabels`、`insertTrendline`、`setTrendline`、`deleteTrendline`、`insertErrorBars`、
`setErrorBars`、`deleteErrorBars`、`setChartSeriesAxis`、`setChartSize`、`setChartAnchor`；
不含 `setChartWrap`）、以及 `setProps` / `setCustomProp` / `deleteCustomProp`、以及工作表管理 op（`insertSheet`、`deleteSheet`、
`renameSheet`、`moveSheet`、`setSheetHidden`）、以及列宽行高与合并 op（`setColWidth`、`setRowHeight`、`mergeRange`、`unmergeRange`）、
以及数据验证 op（`insertDataValidation`、`setDataValidation`、`deleteDataValidation`）、以及条件格式 op（`insertConditionalFormat`、
`setConditionalFormat`、`deleteConditionalFormat`）、以及工作表表格 op（`insertSheetTable`、`setSheetTable`、`deleteSheetTable`）、
以及透视表 op（`insertPivotTable`、`setPivotTable`、`deletePivotTable`）、以及定义名 op（`insertDefinedName`、`setDefinedName`、
`deleteDefinedName`）、以及 `setPrintSetup`、`setSheetView`、`setAutoFilter`、`setSortState`、以及自定义视图 op（`insertCustomView`、
`deleteCustomView`、`showCustomView`）、以及分组 op（`groupRows`、`ungroupRows`、`groupCols`、`ungroupCols`、`setOutline`）、
以及批注 op（`insertComment`、`deleteComment`、`replyComment`、`setCommentText`、`setCommentMeta`、`setCommentDone`）、
以及 `setShapeText`、`setShapeFormat`、以及文本框 op（`insertTextBox`、`deleteTextBox`、`setTextBoxSize`、`setTextBoxAnchor`；
不含 `setTextBoxWrap`、`setTextBoxFill`、`setTextBoxLine`、`setTextBoxDirection`）、以及形状管理 op（`insertShape`、`deleteShape`、
`insertConnector`、`deleteConnector`、`groupShapes`、`ungroup`；
不含 `setShapePos`、`setShapeSize`、`setShapeFill`、`setShapeLine`、`setShapeGeom`、`setConnector`）、以及 SmartArt op（`insertSmartArt`、`deleteSmartArt`、
`setSmartArtText`、`addSmartArtPoint`、`deleteSmartArtPoint`、`setSmartArtSize`、`setSmartArtAnchor`、`setSmartArtColors`、
`setSmartArtStyle`、`setSmartArtLayout`；
不含 `setSmartArtPos`、`setSmartArtWrap`）、以及超链接 op（`insertHyperlink`、`setHyperlink`、`deleteHyperlink`）、
以及迷你图 op（`insertSparkline`、`setSparkline`、`deleteSparkline`）、以及 OLE op（`insertOle`、`deleteOle`）、以及 3D 模型 op（`insertModel3d`、`replaceModel3d`、`deleteModel3d`）、以及公式 op（`insertMath`、`setMath`、`deleteMath`）、以及切片器 op（`insertSlicer`、
`deleteSlicer`）、以及连接 op（`insertConnection`、`deleteConnection`）、以及外部引用 op（`insertExternalLink`、`deleteExternalLink`）、以及 `setProtection`。
docx 认 `setParaText`、`setHeaderFooterDistance`、`setPageMargin`、`setPageSize`、`setColumns`、`setVerticalAlign`、
`setLineNumbering`、`setDocGrid`、`setRtlGutter`、`setMirrorMargins`、`setGutterAtTop`、`setHeaderFooterLink`、
`insertHeaderFooter`、`setTitlePage`、`setEvenAndOddHeaders`、图片 op（含 `setPictureWrap`，
不含 `setPictureAnchor`）、文本框 op、组合与连接线 op（`insertConnector`、`deleteConnector`、`groupShapes`、`ungroup`；
不含 `insertShape`、`deleteShape`、`setShapePos`、`setShapeSize`、`setShapeFill`、`setShapeLine`、`setShapeGeom`、`setConnector`、`setShapeFormat`）、`insertPara` / `deletePara`、
表格 op（`insertTable`、`deleteTable`、`insertRow`、`deleteRow`、`insertCol`、`deleteCol`、`mergeCells`、`splitCells`）、
修订 op（`acceptRevision`、`rejectRevision`）、超链接与书签 op（`insertHyperlink`、`setHyperlink`、`deleteHyperlink`、`insertBookmark`、
`setBookmark`、`deleteBookmark`）、域 op（`insertField`、`setField`、`deleteField`）、目录 op（`insertToc`、`deleteToc`）、
题注与交叉引用 op（`insertCaption`、`deleteCaption`、`insertCrossRef`、`deleteCrossRef`）、索引 op（`insertIndex`、`deleteIndex`、
`insertIndexEntry`、`deleteIndexEntry`）、参考文献 op（`insertSource`、`deleteSource`、`insertCitation`、`deleteCitation`、
`insertBibliography`、`deleteBibliography`）、构建基块 op（`insertDocPart`、`deleteDocPart`、`insertDocPartContent`）、
字体嵌入 op（`insertEmbeddedFont`、`deleteEmbeddedFont`、`setFontEmbed`）、页面背景与边框 op（`setPageBackground`、`setPageBorders`）、
公式 op（`insertMath`、`setMath`、`deleteMath`）、内容控件 op（`insertContentControl`、`setContentControl`、`deleteContentControl`）、
编号 op（`insertNumbering`、`setNumbering`、`setParaNumbering`）、样式 op（`setStyle`、`setParaStyle`）、直接格式 op（`setParaFormat`、
`setRunFormat`）、节结构 op（`insertSection`、`deleteSection`）、脚注尾注 op（`insertFootnote`、`insertEndnote`、`deleteFootnote`、
`deleteEndnote`、`setNoteNumbering`）、批注 op（`insertComment`、`deleteComment`、`replyComment`、`setCommentText`、
`setCommentMeta`、`setCommentDone`）、图表 op（`insertChart`、`deleteChart`、`setChartTitle`、`setChartSeries`、`addChartSeries`、
`deleteChartSeries`、`setChartType`、`setChartAxisTitle`、`setChartLegend`、`setChartDataLabels`、`insertTrendline`、
`setTrendline`、`deleteTrendline`、`insertErrorBars`、`setErrorBars`、`deleteErrorBars`、`setChartSeriesAxis`、`setChartSize`、`setChartWrap`；
不含 `setChartAnchor`、`insertChartSheet`、`deleteChartSheet`）、以及 SmartArt op（`insertSmartArt`、`deleteSmartArt`、
`setSmartArtText`、`addSmartArtPoint`、`deleteSmartArtPoint`、`setSmartArtSize`、`setSmartArtWrap`、`setSmartArtColors`、
`setSmartArtStyle`、`setSmartArtLayout`）、以及 OLE op（`insertOle`、`deleteOle`）、以及墨迹画布 op（`insertInkCanvas`、`replaceInkCanvas`、`deleteInkCanvas`）、以及 3D 模型 op（`insertModel3d`、`replaceModel3d`、`deleteModel3d`）、以及 `setProtection`、
以及 `insertWatermark` / `deleteWatermark`、以及 `setThemeColors` / `setThemeFonts`、
以及 `setProps` / `setCustomProp` / `deleteCustomProp`。
pptx 认 `setShapeText`、`setShapeFormat`、形状管理 op（`insertShape`、`insertConnector`、`deleteConnector`、`setConnector`、`groupShapes`、`ungroup`、`deleteShape`、
`setShapePos`、`setShapeSize`、`setShapeFill`、`setShapeLine`、`setShapeGeom`）、表格 op（`insertTable`、`deleteTable`、`insertRow`、`deleteRow`、
`insertCol`、`deleteCol`、`setTableCellText`、`setTablePos`、`setTableSize`；
不含 `mergeCells`、`splitCells`）、`setNotesText`、批注 op（`insertComment`、`deleteComment`、`replyComment`、`setCommentText`、
`setCommentMeta`、`setCommentDone`）、`replacePicture`、`insertPicture`、`deletePicture`、`setPictureSize`、`setPicturePos`、
以及图表 op（`insertChart`、`deleteChart`、`setChartTitle`、`setChartSeries`、`addChartSeries`、`deleteChartSeries`、
`setChartType`、`setChartAxisTitle`、`setChartLegend`、`setChartDataLabels`、`insertTrendline`、`setTrendline`、
`deleteTrendline`、`insertErrorBars`、`setErrorBars`、`deleteErrorBars`、`setChartSeriesAxis`、`setChartSize`、`setChartPos`；
不含 `setChartWrap`、`setChartAnchor`、`insertChartSheet`、`deleteChartSheet`）、以及 SmartArt op（`insertSmartArt`、
`deleteSmartArt`、`setSmartArtText`、`addSmartArtPoint`、`deleteSmartArtPoint`、`setSmartArtSize`、`setSmartArtPos`、
`setSmartArtColors`、`setSmartArtStyle`、`setSmartArtLayout`；
不含 `setSmartArtWrap`）、以及幻灯片管理 op（`insertSlide`、`deleteSlide`、`moveSlide`、`setSlideHidden`、`setTransition`、
`setSlideLayout`）、以及母版写入 op（`insertLayout`、`deleteLayout`、`insertPlaceholder`、`deletePlaceholder`、`insertMaster`、
`deleteMaster`）、以及主题与尺寸 op（`setSlideSize`、`setThemeColors`、`setThemeFonts`）、以及动画 op（`insertAnim`、`setAnim`、`deleteAnim`、`moveAnim`）、
以及 `setHandoutText`、以及超链接 op（`insertHyperlink`、`setHyperlink`、`deleteHyperlink`）、以及域 op（`insertField`、`setField`、
`deleteField`）、以及媒体 op（`insertVideo`、`insertAudio`、`deleteMedia`、`setMedia`）、以及幻灯片节 op（`insertSlideSection`、`setSlideSectionName`、
`deleteSlideSection`、`setSlideSection`）、以及自定义放映 op（`insertCustomShow`、`setCustomShow`、`deleteCustomShow`）、
以及 OLE op（`insertOle`、`deleteOle`）、以及 3D 模型 op（`insertModel3d`、`replaceModel3d`、`deleteModel3d`）、以及公式 op（`insertMath`、`setMath`、`deleteMath`）、以及 Zoom op（`insertZoom`、`setZoom`、`deleteZoom`）、以及 `setShowPr`、以及 `setProtection`、
以及 `setProps` / `setCustomProp` / `deleteCustomProp`。
不提供把整棵时间轴当 JSON 写入的 op；`setTiming` 仍报 `UnsupportedOp`。
对不上格式的 op（例如对 pptx 发 `setCell` / `setCellStyle` / `setParaText` / `setProtection`，
对 xlsx 发 `setPictureWrap`、`setChartWrap` 或 `setHeaderFooterDistance` / `setPageMargin` / `setPageSize` / `setColumns`
 / `setVerticalAlign` / `setLineNumbering` / `setDocGrid` / `setRtlGutter` / `setPageBackground` / `setPageBorders`
 / `setMirrorMargins` / `setGutterAtTop` / `setHeaderFooterLink` / `insertHeaderFooter` / `setTitlePage` / `setEvenAndOddHeaders`、
`setTextBoxWrap` / `setTextBoxFill` / `setTextBoxLine` / `setTextBoxDirection`、注、`setSmartArtPos` / `setSmartArtWrap`、
`insertWatermark` / `deleteWatermark`、`insertInkCanvas` / `replaceInkCanvas` / `deleteInkCanvas`、表格 op、修订 op、书签 op、域 op、题注与交叉引用 op、索引 op、参考文献 op、构建基块 op、字体嵌入 op、编号 op、样式 op、
直接格式 op、节结构 op 或 `setNotesText`，
对 docx / pptx 发 `setHeaderFooterText`、`insertChartSheet`、`deleteChartSheet`、`insertSheet`、`deleteSheet`、`renameSheet`、
`moveSheet`、`setSheetHidden`、`setCellStyle`、`setColWidth`、`setRowHeight`、`mergeRange` 或 `unmergeRange`、
`insertDataValidation` / `setDataValidation` / `deleteDataValidation`、
`insertConditionalFormat` / `setConditionalFormat` / `deleteConditionalFormat`、
`insertSheetTable` / `setSheetTable` / `deleteSheetTable`、`insertPivotTable` / `setPivotTable` / `deletePivotTable`、
`insertDefinedName` / `setDefinedName` / `deleteDefinedName`、`setPrintSetup`、`setSheetView`、`setAutoFilter`、
`groupRows` / `ungroupRows` / `groupCols` / `ungroupCols` / `setOutline`、`insertSlicer` / `deleteSlicer`、
`insertConnection` / `deleteConnection` / `insertExternalLink` / `deleteExternalLink` / `insertCustomView`
 / `deleteCustomView` / `showCustomView`，
对 docx 发 `setPictureAnchor` / `setChartAnchor` / `setSmartArtPos` / `setNotesText`，
对 pptx 发 `setPictureWrap` / `setPictureAnchor` / `setChartWrap` / `setChartAnchor` / `setSmartArtWrap`，
对 docx / xlsx 发 `setPicturePos` / `setChartPos` / `setSmartArtPos` / `setNotesText` / `insertSlide` / `deleteSlide`
 / `moveSlide` / `setSlideHidden` / `setTransition` / `insertAnim` / `setAnim` / `deleteAnim` / `moveAnim` / `setSlideLayout` / `insertLayout`
 / `deleteLayout` / `insertPlaceholder` / `deletePlaceholder` / `insertMaster` / `deleteMaster` / `setSlideSize`
 / `setThemeColors` / `setThemeFonts` / `setHandoutText` / `setShapePos` / `setShapeSize` / `setShapeFill`
 / `setShapeLine` / `setShapeGeom` / `setConnector` / `setTableCellText` / `setTablePos` / `setTableSize` / `insertVideo` / `insertAudio`
 / `deleteMedia` / `setMedia` / `insertSlideSection` / `setSlideSectionName` / `deleteSlideSection` / `setSlideSection`
 / `insertCustomShow` / `setCustomShow` / `deleteCustomShow` / `insertZoom` / `setZoom` / `deleteZoom`）报 `InvalidField`。
xlsx 上发 `insertTable` / `deleteTable` / `insertRow` / `deleteRow` / `insertCol` / `deleteCol` / `mergeCells` / `splitCells` 仍报 `InvalidField`。
对 docx 发 `insertShape` / `deleteShape` / `setShapeFormat` 报 `InvalidField`。pptx 上发 `mergeCells` / `splitCells` 报 `InvalidField`。

### setProps

- **`op`**
  - 必须是 `"setProps"`
- **`id`**
  - 必须是格式根：`xlsx:workbook`、`docx:document` 或 `pptx:presentation`
- **`title` / `subject` / `creator` / `description` / `keywords` / `category` / `company` / `manager`**
  - 可选
  - 字符串或 `null`
  - `null` 删掉对应 XML 元素
  - 空串写空元素

这八个字段至少给一个，否则报 `InvalidField`。未点名的字段留下，含 `Application`、页数/字数、`HeadingPairs`、`TitlesOfParts`、`Slides`，以及未点名的公司名。
不要写 `DigSig`。不要改 `lastModifiedBy`、`created`、`modified`。出现这些只读键报 `UnknownField`。id 不是格式根报 `InvalidId`。
缺 `core.xml` 或 `app.xml` 时，只要这条 op 点到了对应部件里的字段，就按需新建部件、根关系和 Content Type Override。已经是目标状态则空操作，仍计入 `opsApplied`。

### setCustomProp

- **`op`**
  - 必须是 `"setCustomProp"`
- **`id`**
  - 必须是格式根
- **`name`**
  - 必须
  - 非空字符串，对应 `property/@name`
- **`value`**
  - 必须
  - JSON 字符串、数字或布尔
- **`type`**
  - 可选
  - `lpwstr`、`i4`、`r8` 或 `bool`
  - 省略则按 `value` 的 JSON 类型推断：字符串为 `lpwstr`，布尔为 `bool`，能放进 32 位整数的数字为 `i4`，其余数字为 `r8`

按名 upsert：已有同名 `property` 时命中文档顺序第一处，保留原来的 `pid` / `fmtid`，只换值子元素。
没有则追加，`fmtid` 固定 `{D5CDD505-2E9C-101B-9397-08002B2CF9AE}`，`pid` 取已有最大整数加 1，至少为 2。
`name` 为空、`type` 不认识，或 `type` 与 `value` 对不上（例如 `i4` 却给了字符串或带小数的数字），报 `InvalidField`。id 不是格式根报 `InvalidId`。缺 `custom.xml` 则新建部件、根关系和 Override。

### deleteCustomProp

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteCustomProp"` |
| `id` | 必须是 `prop:{name}`，`{name}` 与树上自定义属性的 `name` 相同 |

找不到该名报 `IdNotFound`。id 不是 `prop:{name}` 报 `InvalidId`。删光后去掉 custom 部件、根关系和 Override。

### setProtection

docx 改 `/word/settings.xml` 的限制编辑；xlsx 改工作表 `sheetProtection` 或工作簿 `workbookProtection`；
pptx 改 `presentation.xml` 的 `p:modifyVerifier`。pptx 的放映设置用 `setShowPr`，不要塞进本 op。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "setProtection",
      "id": "docx:document",
      "enforced": true,
      "edit": "readOnly"
    }
  ]
}
```

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "output": "C:\\data\\book.protect.xlsx",
  "ops": [
    {
      "op": "setProtection",
      "id": "sheet:Sheet1",
      "enforced": true
    },
    {
      "op": "setProtection",
      "id": "xlsx:workbook",
      "lockStructure": true
    }
  ]
}
```

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "setProtection",
      "id": "pptx:presentation",
      "enforced": true,
      "password": "secret"
    }
  ]
}
```

docx：

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setProtection"` |
| `id` | 必须是 `docx:document` |
| `enforced` | 可选。布尔。`true` 写出 `w:documentProtection` 并强制；`false` 删掉该元素 |
| `edit` | 可选。`readOnly`、`comments`、`trackedChanges` 或 `forms`。打开强制且当前没有种类时，缺省 `readOnly` |
| `formatting` | 可选。布尔。对应 `w:formatting` |
| `readOnlyRecommended` | 可选。布尔。`true` 写出 `<w:writeProtection w:recommended="true"/>`；`false` 在没有写保护密码时删掉该元素 |
| `password` | 可选。非空字符串。按 Word 16.0 写限制编辑 SHA-512 哈希 |

这四个可写字段至少给一个，否则报 `InvalidField`。`password` 不算「至少一个可写字段」。未点名的字段留下。`edit` 为 `none` 或不认识的字符串报 `InvalidField`。
关掉强制请用 `enforced: false`，不要写 `edit: none`。空 `password` 报 `InvalidField`。
`password` 只用于 `documentProtection`；只改 `readOnlyRecommended` 却给了 `password` 报 `InvalidField`。
出现 `hasPassword` 报 `InvalidField`。id 不是 `docx:document` 报 `InvalidId`。没有 `settings.xml` 报 `OpcPartMissing`。
对 pptx 发此 op 且 id 为 `pptx:presentation` 时走下面 pptx 那一节。

打开强制且没有哈希时，写出 Transitional 标记：`w:edit` 与 `w:enforcement="1"`。
给了 `password` 时再按 Word 16.0 写 `cryptProviderType="rsaAES"`、`cryptAlgorithmClass="hash"`、`cryptAlgorithmType="typeAny"`、
`cryptAlgorithmSid="14"`、`cryptSpinCount="100000"`、`hash`、`salt`（16 字节随机盐的标准 Base64）。
口令按 UTF-8 取前 15 字节做 ECMA-376 Part 4 §14.8.1 遗留预处理，把 4 字节结果写成大写两位十六进制再按 UTF-16LE；
`H0 = SHA-512(salt || 该 UTF-16LE)`，再对 `i = 0 .. 99999` 做 `H_{i+1} = SHA-512(H_i || i 的 4 字节小端)`。
不写 ISO 的 `algorithmName` / `hashValue` / `saltValue` / `spinCount`，不写 `reservationPassword`。
已有 `cryptAlgorithmSid=14` 且 `hash` / `salt` / `cryptSpinCount` 齐时必须给对得上的 `password`，否则报 `NotImplemented`；口令对不上报 `InvalidField`。
对得上则可改种类，或 `enforced: false` 解开（删掉元素）。改写时换新盐重算。
其它哈希（缺 sid、sid 不是 14、只有 ISO `hashValue`、`reservationPassword` 等）即使给了 `password` 也报 `NotImplemented`。
无哈希时 `enforced: false` 直接删掉元素。`writeProtection` 已有密码哈希时，改 `readOnlyRecommended` 报 `NotImplemented`，即使给了 `password`。
`edit` 为 `trackedChanges` 时同时写出空的 `w:trackRevisions`；关掉保护时不要删已有的 `trackRevisions`。
`edit` 为 `forms` 时给主文档每一个 `w:sectPr` 写 `w:formProt`；关掉窗体保护或关掉强制时删掉各节 `w:formProt`。不读写 `w:permStart` / `w:permEnd`。
已经是目标状态则空操作，仍计入 `opsApplied`。不把哈希放进树。

create 的 `document.protection` 复用同一套规则，但出现 `password` 仍报 `NotImplemented`。
出现 `hasPassword` 报 `InvalidField`（`hasPassword` 是树上的只读字段）。

带密码示例见 `docs/examples/docx-apply-protect-password.json`。

xlsx 工作表（id 为 `sheet:{name}`）：

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setProtection"` |
| `id` | 必须是 `sheet:{name}` |
| `enforced` | 可选。布尔。`true` 写出 `<sheetProtection>` 并打开；`false` 删掉该元素 |
| `objects` 等权限键 | 可选。布尔。与树上 `sheetProtection` 同名同向 |
| `password` | 可选。非空字符串。按 Excel 16.0 写 SHA-512 哈希 |

工作表可写键：`enforced`、`objects`、`scenarios`、`formatCells`、`formatColumns`、`formatRows`、`insertColumns`、`insertRows`、
`insertHyperlinks`、`deleteColumns`、`deleteRows`、`selectLockedCells`、`sort`、`autoFilter`、`pivotTables`、`selectUnlockedCells`。
至少给一个，否则报 `InvalidField`。`password` 可选，非空字符串；空串报 `InvalidField`。`password` 不算「至少一个可写字段」。未点名的属性留下。
出现 `hasPassword` / `edit` / `formatting` / `readOnlyRecommended` / `lockStructure` 报 `InvalidField`。
id 不是工作表 id 报 `InvalidId`。找不到工作表报 `IdNotFound`。对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。

打开保护且该表还没有 `<sheetProtection>` 时，按 Excel 16.0 `Protect()` 缺省写出 `sheet="1" objects="1" scenarios="1"`，其余权限用 schema 缺省（不写属性）。
随后把本次点名的权限键写上：与缺省相同则省略该属性，与缺省不同则写 `"1"` 或 `"0"`。`enforced: true` 时 `@sheet` 为 `"1"`。
给了 `password` 时按 Excel 16.0 写 `algorithmName="SHA-512"`、`hashValue`、`saltValue`（16 字节随机盐的标准 Base64）、`spinCount="100000"`。
口令按 UTF-16LE；`H0 = SHA-512(salt || password)`，再对 `i = 0 .. 99999` 做 `H_{i+1} = SHA-512(H_i || i 的 4 字节小端)`。
不写遗留 `@password`。已有 SHA-512 哈希时必须给对得上的 `password`，否则报 `NotImplemented`；口令对不上报 `InvalidField`。
对得上则可改权限，或 `enforced: false` 解开（删掉元素）。改写时换新盐重算。遗留 XOR 或其它算法即使给了 `password` 也报 `NotImplemented`。
无哈希时 `enforced: false` 删掉整个元素。已经是目标状态则空操作，仍计入 `opsApplied`。不标重算，不新建部件。不把哈希放进树。

图表工作表只认 `enforced` 与 `objects`。`enforced` 写入 `@content`（`CT_ChartsheetProtection`）。出现工作表专用权限键报 `InvalidField`。打开时缺省写出 `content="1" objects="1"`。

xlsx 工作簿（id 为 `xlsx:workbook`）：

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setProtection"` |
| `id` | 必须是 `xlsx:workbook` |
| `lockStructure` | 可选。布尔 |
| `lockWindows` | 可选。布尔 |
| `lockRevision` | 可选。布尔 |
| `password` | 可选。非空字符串。写 `workbook*` SHA-512 哈希 |

这三个至少给一个。出现工作表权限键或 docx 字段报 `InvalidField`。`password` 可选，规则与工作表相同。三项都为假时删掉 `<workbookProtection>`。
为真的项写 `"1"`，为假的省略（schema 缺省即假）。给了 `password` 时写 `workbookAlgorithmName="SHA-512"`、`workbookHashValue`、
`workbookSaltValue`、`workbookSpinCount="100000"`，算法与工作表相同。
不写遗留 `@workbookPassword`。已有 SHA-512 哈希时必须给对得上的 `password`，否则报 `NotImplemented`；口令对不上报 `InvalidField`。
遗留 XOR 或其它算法即使给了 `password` 也报 `NotImplemented`。按 `CT_Workbook` 插在 `workbookPr` 之后、`bookViews` 之前。不新建部件。

示例见 `docs/examples/xlsx-apply-protect.json`。带密码见 `docs/examples/xlsx-apply-protect-password.json`。

pptx（id 为 `pptx:presentation`）：

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setProtection"` |
| `id` | 必须是 `pptx:presentation` |
| `enforced` | 必须。布尔。`true` 写出 `p:modifyVerifier`；`false` 删掉该元素 |
| `password` | 可选。非空字符串。按 PowerPoint 16.0 写修改密码 SHA-512 哈希 |

必须给 `enforced`，否则报 `InvalidField`。`password` 不算「至少一个可写字段」。空 `password` 报 `InvalidField`。
出现 `hasPassword`、docx 字段或 xlsx 权限键报 `InvalidField`。id 不是 `pptx:presentation` 报 `InvalidId`。

`enforced: true` 且不给 `password` 时写出空 `<p:modifyVerifier/>`。已经是空元素则空操作。
给了 `password` 时按 PowerPoint 16.0 写 `cryptProviderType="rsaAES"`、`cryptAlgorithmClass="hash"`、`cryptAlgorithmType="typeAny"`、
`cryptAlgorithmSid="14"`、`spinCount="100000"`、`hashData`、`saltData`（16 字节随机盐的标准 Base64）。
口令按 UTF-16LE；`H0 = SHA-512(salt || password)`，再对 `i = 0 .. 99999` 做 `H_{i+1} = SHA-512(H_i || i 的 4 字节小端)`。
不写 ISO 的 `algorithmName` / `hashValue` / `saltValue` / `spinValue`，不写 Word 的 `hash` / `salt` / `cryptSpinCount`。
已有 `cryptAlgorithmSid=14` 且 `hashData` / `saltData` / `spinCount` 齐时必须给对得上的 `password`，否则报 `NotImplemented`；口令对不上报 `InvalidField`。
对得上则可 `enforced: false` 解开（删掉元素），或 `enforced: true` 换新盐重算。
其它哈希（ISO `hashValue`、Word `hash` / `salt`、缺 sid、sid 不是 14 等）即使给了 `password` 也报 `NotImplemented`。
无哈希时 `enforced: false` 直接删掉元素。没有该元素且 `enforced: false` 则空操作，仍计入 `opsApplied`。
不把哈希放进树。按 `CT_Presentation` 该元素在 `defaultTextStyle` 之后、`extLst` 之前。不新建部件。xlsx / docx 上对本 id 发此 op 仍报 `InvalidId`。

pptx 的放映类型、循环与旁白开关用 `setShowPr`，见 [json-contract-pptx.md](json-contract-pptx.md#setshowpr)。

示例见 `docs/examples/pptx-apply-protect.json`。带密码见 `docs/examples/pptx-apply-protect-password.json`。

### replacePicture

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.pic.docx",
  "ops": [
    {
      "op": "replacePicture",
      "id": "header:default:1/pic:1",
      "file": "C:\\img\\logo.png"
    }
  ]
}
```

- **`op`**
  - 必须是 `"replacePicture"`
- **`id`**
  - 图 id：docx 为 `{容器}/pic:…` 或 `{段落 id}/pic:…`
  - xlsx 为 `sheet:{name}/pic:…`
  - pptx 为 `slide:{n}/pic:…`、`slide:{n}/notes/pic:…` 或 `{批注 id}/pic:{k}`
- **`file`**
  - 本地图片路径，与信封 `path` 一样认正斜杠和反斜杠
  - 不把 base64 放进领域 JSON

找不到该图报 `IdNotFound`。读不到 `file` 报 `Io`。文件不是该格式认的图片报 `InvalidField`。

只换媒体字节与内容类型，不改锚点、绕排和位置。`a:blip/@r:embed` 的关系 id 保持不变。
docx 扩展名变了就写新的 `/word/media/imageN.*`；xlsx 写新的 `/xl/media/imageN.*`；pptx 写新的 `/ppt/media/imageN.*`。
改同一条关系的 `Target`，并更新 `[Content_Types].xml` 的 Default 或 Override；旧媒体若重新解析后无人引用则删掉。
三种格式都按文件头认 PNG、JPEG（`.jpg` / `.jpeg`）、GIF、BMP、SVG、EMF、WMF、TIFF。
扩展名只用来给新媒体取名。不认 SVGZ、WebP。
换成 EMF / WMF / TIFF 时：把原字节写入该格式 media，一条 image 关系。
`a:blip/@r:embed` 指向它。不要 PNG 占位，不要 `svgBlip`。
写出扩展名用 `.emf` / `.wmf` / `.tif`。
`Default Extension` 为 `image/x-emf` / `image/x-wmf` / `image/tiff`。

换成 SVG 时：

- 保留已有 `a:blip` 后备图与关系 id
- 在 `a:blip` 上写或改 `asvg:svgBlip`（Office 2016 SVG 扩展）
- SVG 原字节写入该格式的 media 部件
  - docx：`/word/media/imageN.svg`
  - xlsx：`/xl/media/imageN.svg`
  - pptx：`/ppt/media/imageN.svg`
- `Default Extension="svg"` 为 `image/svg+xml`

从 SVG 换成位图时：按上面换 `a:blip` 媒体，去掉 `svgBlip`；SVG 部件无人引用则删。

### insertPicture

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.ins.docx",
  "ops": [
    {
      "op": "insertPicture",
      "id": "header:default:1/para:3F2A1B00",
      "file": "C:\\img\\logo.png"
    }
  ]
}
```

- **`op`**
  - 必须是 `"insertPicture"`
- **`id`**
  - docx：段落 id（正文用 `para:…` / `para:index:{n}`，页眉页脚用 `{容器}/para:…`）
  - xlsx：工作表 id `sheet:{name}`，或页眉槽位 id `sheet:{name}/hf:{kind}:{side}`
  - pptx：页 id `slide:{n}`、备注容器 id `slide:{n}/notes`、母版 id `master:{n}`、版式 id `master:{n}/layout:{k}`，或批注 / 回复 id
- **`file`**
  - 本地图片路径
- **`cx`**
  - 可选
  - 宽度，EMU，正整数
- **`cy`**
  - 可选
  - 高度，EMU，正整数
- **`x`**
  - pptx 可选
  - 画布横坐标，EMU，非负整数
  - docx / xlsx 上出现报 `InvalidField`
- **`y`**
  - pptx 可选
  - 画布纵坐标，EMU，非负整数
  - docx / xlsx 上出现报 `InvalidField`
- **`from`**
  - xlsx 往工作表插图时必填
  - 左上角单元格，A1
  - 往页眉槽位插图时出现报 `InvalidField`
  - docx / pptx 上出现报 `InvalidField`
- **`to`**
  - xlsx 且 `anchor` 为 `twoCell` 时必填
  - 右下角单元格，A1
  - 往页眉槽位插图时出现报 `InvalidField`
  - docx / pptx 上出现报 `InvalidField`
- **`anchor`**
  - xlsx 往工作表插图时可选
  - `oneCell`（缺省）或 `twoCell`
  - 不认 `absolute`
  - 往页眉槽位插图时出现报 `InvalidField`
  - docx / pptx 上出现报 `InvalidField`
- **`fromOffX` / `fromOffY`**
  - xlsx 往工作表插图时可选
  - 相对 `from` 的 EMU 偏移，缺省 `0`，不得为负
  - 往页眉槽位插图时出现报 `InvalidField`
  - docx / pptx 上出现报 `InvalidField`
- **`toOffX` / `toOffY`**
  - xlsx 且 `twoCell` 时可选
  - 相对 `to` 的 EMU 偏移，缺省 `0`，不得为负
  - 往页眉槽位插图时出现报 `InvalidField`
  - docx / pptx 上出现报 `InvalidField`

docx：找不到该段落报 `IdNotFound`。`id` 不是段落 id 报 `InvalidId`。
`cx` / `cy` 都省略时，位图按像素尺寸、96 dpi 换算成 EMU（`px × 914400 / 96`）；读不出尺寸则用 1 英寸见方（`914400`）。
SVG 按根元素 `width` / `height`（无单位或 `px` 当 CSS 像素，同样 96 dpi）；没有则用 `viewBox`；再没有则 1 英寸见方。
EMF 优先用 `rclFrame`（单位 0.01 mm）换 EMU；读不到再用 `rclBounds` 当像素、96 dpi。
可放置 WMF 用 Aldus 框除以 `inch` 再乘 914400。非放置 WMF 读不到框则 1 英寸见方。
TIFF 用 IFD `ImageWidth` / `ImageLength`，96 dpi；不读分辨率标签。
只给一个时报 `InvalidField`。插入 `wp:inline`，挂在该段末尾一个新的 `w:r` 里。
位图写入该部件一条 image 关系、`/word/media/imageN.*` 与内容类型。
EMF / WMF / TIFF 与位图同一条路径：原字节、一条 image 关系。不要 PNG 占位，不要 `svgBlip`。
SVG 再多写一条 image 关系指向 `/word/media/imageN.svg`。
`a:blip/@r:embed` 指向 1×1 PNG 占位（不栅格化 SVG）。
`asvg:svgBlip/@r:embed` 指向 SVG。
`wp:docPr/@id` 取该部件已有 `docPr` 的最大整数加 1，至少为 1。

xlsx：找不到该表报 `IdNotFound`。`id` 不是 `sheet:{name}` 也不是页眉槽位 id 报 `InvalidId`。该表不是 `worksheet` 报 `InvalidField`。
往工作表插图必须给 `from`，否则报 `MissingField`。`from` / `to` 不是合法 A1 报 `InvalidRange`。
`anchor` 为 `twoCell` 却没有 `to` 报 `InvalidField`。`anchor` 为 `absolute` 或不认识报 `InvalidField`。
`cx` / `cy` 规则与 docx 相同；`twoCell` 也要写 `xdr:pic` 里的 `a:ext`（给 Excel 一个内部尺寸），显示尺寸仍由起止格决定。
没有 drawing 部件则新建 `/xl/drawings/drawingN.xml`、工作表关系、`[Content_Types].xml` 的 Override，并在工作表上写 `<drawing r:id="…"/>`。
往页眉槽位插图时不写 `<drawing>`，改写或新建 `<drawingHF r:id="…"/>`。新媒体写入 `/xl/media/imageN.*`。
EMF / WMF / TIFF 与位图同一条路径：原字节、一条 image 关系。不要 PNG 占位，不要 `svgBlip`。
SVG 再多写一条 image 关系指向 `/xl/media/imageN.svg`。
`a:blip/@r:embed` 指向 1×1 PNG 占位。`asvg:svgBlip/@r:embed` 指向 SVG。
`xdr:cNvPr/@id` 取该表工作表 drawing 与 `drawingHF` 里已有 `cNvPr` 的最大整数加 1。工作表图至少为 1。
页眉图至少为 2：Excel 16.0 若 `drawingHF` 的 `cho` / `lho` 等属性指向 id 1，会拒绝打开该工作簿。锚点必须带空的 `xdr:clientData`（§20.5.2.3）。

pptx：找不到该页 / 该母版 / 该版式报 `IdNotFound`。`id` 必须是 `slide:{n}`、`slide:{n}/notes`、`master:{n}`、`master:{n}/layout:{k}`，
或批注 / 回复 id（`slide:{n}/comment:{guid}` 或 `{批注 id}/reply:{guid}`）。
对图 id、形状 id、批注内图 id 或 `pptx:presentation` 发此 op 报 `InvalidId`。
往幻灯片或备注插时：`cx` / `cy` 规则与 docx 相同；`x` / `y` 必须成对出现或成对省略；只给一个时报 `InvalidField`。
缺坐标则放到该宿主空白处，不要叠在 `(0,0)`：往幻灯片或母版 / 版式插时读取 `presentation.xml` 的 `p:sldSz`；往备注插时读取 `p:notesSz`（没有则用 `6858000` × `9144000`）。
扫描该宿主已有 `p:sp` / `p:pic` / `p:graphicFrame` / 嵌套 `p:grpSp` 的 `a:xfrm`，优先放到最右侧空隙，放不下再放到下方。
`kind` / `wrap` / `from` / `to` / `anchor` 以及 xlsx 偏移字段出现报 `InvalidField`。
写入该宿主 `p:spTree` 一个新的 `p:pic`（带 `a:blip`），关系指向 `/ppt/media/imageN.*`。
EMF / WMF / TIFF 与位图同一条路径：原字节、一条 image 关系。不要 PNG 占位，不要 `svgBlip`。
SVG 再多写一条 image 关系指向 `/ppt/media/imageN.svg`。
`a:blip/@r:embed` 指向 1×1 PNG 占位。`asvg:svgBlip/@r:embed` 指向 SVG。
`p:cNvPr/@id` 取该宿主已有全部 `cNvPr`（`sp` / `pic` / `graphicFrame` / 组合，含 `spTree` 自己的 `id="1"`）的最大整数加 1，至少为 2。
`id` 为 `slide:{n}/notes` 且该页还没有 notesSlide 时，按 `setNotesText` 同一套规则新建最小备注页再插图。
往批注或回复插时：不要 `x` / `y` / `cx` / `cy`（出现报未知字段）。找不到该批注报 `IdNotFound`。
写入该 `p188:cm` 或 `p188:reply`：插在 `txBody` 之后、`extLst` 之前一个 `a:blip r:embed="…"`（没有 `extLst` 则放在该宿主末尾）。
comments 部件 `.rels` 补一条 image 关系，媒体写入 `/ppt/media/imageN.*`。
EMF / WMF / TIFF 仍只写这一条 image 关系，不要 PNG 占位，不要 `svgBlip`。
SVG 时该 `a:blip` 仍指向 1×1 PNG 占位，再补一条 image 关系指向 `/ppt/media/imageN.svg`，并写 `asvg:svgBlip`。
不写 `p:pic`，不写尺寸位置。不并进幻灯片 `pictureCount`。
对 `kind` 为 `ink` 的批注或回复报 `NotImplemented`。PowerPoint 16.0：COM 批注对象没有插图方法；另存接受这种 `a:blip`，标题栏无 Repair。

### deletePicture

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "deletePicture",
      "id": "header:default:1/pic:1"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deletePicture"` |
| `id` | 图 id |

找不到该图报 `IdNotFound`。docx：按图 id 删掉对应的 `w:drawing`。
若该图在 `mc:AlternateContent` 的 Choice 里，整段 `AlternateContent` 删掉（避免 Fallback 的 VML 把图留住）。
所在 `w:r` 若删完后只剩 `w:rPr` 或空，把该 run 也删掉。xlsx：按图 id 删掉包含该 `xdr:pic` 的整个锚点。工作表 drawing 删空后去掉该部件、工作表上的 `<drawing>` 和对应关系。
`drawingHF` 删空后去掉该部件、工作表上的 `<drawingHF>` 和对应关系；同时清掉 `drawingHF` 上指向该 `cNvPr` id 的左/中/右属性。pptx：按图 id 删掉对应的 `p:pic`。
批注内图删掉对应的 `a:blip`，`p188:cm` / `p188:reply` 与可见文字留下。重新解析包内关系之后，若该媒体与该关系再无人引用，删掉媒体部件、关系条目和对应内容类型。

### setPictureSize

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "setPictureSize",
      "id": "header:default:1/pic:1",
      "cx": 2441542,
      "cy": 1828800
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setPictureSize"` |
| `id` | 图 id |
| `cx` | 可选。宽度，EMU，正整数。xlsx `twoCell` 上出现报 `InvalidField` |
| `cy` | 可选。高度，EMU，正整数。xlsx `twoCell` 上出现报 `InvalidField` |
| `to` | xlsx `twoCell` 时必须。右下角 A1。`oneCell` / `absolute` 上出现报 `InvalidField` |
| `toOffX` / `toOffY` | 可选。只随 xlsx `twoCell` 的 `to` 改右下角偏移。省略则留下当前值 |

`cx` 与 `cy` 至少给一个，否则报 `InvalidField`（xlsx `twoCell` 见下）。`cx` / `cy` 与 `to` 不能同时出现。只改被点名的那一侧，另一侧保持原值。
docx：写入 `wp:extent`；若 `pic:spPr/a:xfrm/a:ext` 存在，一并改。
xlsx：`oneCell` 或 `absolute` 时写入 `xdr:ext`；若 `xdr:spPr/a:xfrm/a:ext` 存在，一并改。
`twoCell` 不认 `cx` / `cy`（出现报 `InvalidField`），必须给 `to`（右下角 A1）；可选 `toOffX` / `toOffY`（EMU，非负，省略则留下当前偏移）。不按列宽行高去换算 EMU。
docx / pptx 上出现 `to` / `toOffX` / `toOffY` 报 `InvalidField`。pptx：写入 `p:spPr/a:xfrm/a:ext`。找不到该图报 `IdNotFound`。
批注内图没有画布 `xfrm`，对本 op 报 `InvalidId`。

`setPictureWrap` 见 [docx](json-contract-docx.md#setpicturewrap)。
`setPictureAnchor` 见 [xlsx](json-contract-xlsx.md#setpictureanchor)。
`setPicturePos` 见 [pptx](json-contract-pptx.md#setpicturepos)。

### insertTextBox

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "insertTextBox",
      "id": "para:3F2A1B00",
      "cx": 1828800,
      "cy": 914400
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertTextBox"` |
| `id` | docx：宿主段落 id（正文或页眉页脚段落）。xlsx：工作表 id `sheet:{name}` |
| `cx` | 可选。宽度，EMU，正整数 |
| `cy` | 可选。高度，EMU，正整数 |
| `text` | 可选。框内第一段可见文本，省略则为空段 |
| `from` | xlsx 必须。左上角单元格，A1。docx 上出现报 `InvalidField` |
| `to` / `anchor` / 偏移 | xlsx 可选。规则与 `insertPicture` 相同。docx 上出现报 `InvalidField` |
| `sz` 等格式字段 | xlsx 可选。与 `setShapeFormat` 相同。docx 上出现报未知字段 |

`id` 必须是段落 id，对图 id、框 id、容器 id、框内段落 id 或 `section:{n}` 报 `InvalidId`。找不到该段报 `IdNotFound`。
`cx` / `cy` 都省略时用 2 英寸宽、1 英寸高（`1828800` / `914400`）。只给一个时报 `InvalidField`。
插入浮动 `anchor` + `wrapSquare`，挂在该段末尾一个新的 `w:r` 里，写成 `mc:AlternateContent`（Choice 为 `wps:wsp` / `wps:txbx`，Fallback 为空白 `w:pict`，不编 VML 几何）。
`wp:docPr/@id` 取该部件已有 `docPr` 的最大整数加 1。对框内段落发此 op 报 `InvalidId`（不套框）。
Word 16.0 另存探测：选中框内再 AddTextbox、以框内 Range 作 Anchor、复制框贴进另一框、页眉同样试，写出的都是外层故事的兄弟 `wps:wsp`（`AnchorStory=1`），`w:txbxContent` 里没有嵌套 `wps:wsp`。
对照：框内插图会进 `w:txbxContent`。不强行手写嵌套 XML。对注内或批注内段落发此 op 报 `InvalidField`：Word 不能把图形对象放到文本框、标注、批注、脚注或尾注中。
已有文件里若出现注内或批注内的框，inspect 仍列出，apply 仍可改、删。xlsx：`id` 必须是 `sheet:{name}`，且该表 `kind` 为 `worksheet`。
锚点字段对齐 `insertPicture`：`from` 必填，可选 `anchor` / `to` / `cx` / `cy` 与偏移。缺 `cx` / `cy` 时用 `1828800` × `914400`。
写入 `xdr:sp`，`xdr:cNvSpPr/@txBox="1"`，可见字在 `xdr:txBody`。`xdr:cNvPr/@name` 写成「文本框 {cnv}」。
xlsx 可带 DrawingML 文字格式字段，规则与 `setShapeFormat` 相同。xlsx 的 `anchor` 仍是单元格锚点，框内垂直对齐只用 `textAnchor`。
docx 上这些格式键仍是未知字段。`kind` / `wrap` 出现报 `InvalidField`。pptx 上发此 op 报 `InvalidField`。

### deleteTextBox

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteTextBox"` |
| `id` | 框 id |

找不到该框报 `IdNotFound`。对段落 id、图 id、容器 id 或 `section:{n}` 报 `InvalidId`。
按框 id 删掉对应的 `w:drawing`；若在 `mc:AlternateContent` 的 Choice 里，整段 `AlternateContent` 删掉。
所在 `w:r` 若删完后只剩 `w:rPr` 或空，把该 run 也删掉。链接链从中间删时，把前一框的 `next` 接到后一框、后一框的 `prev` 接到前一框，
并改写 `wps:txbx` / `linkedTxbx` 的 `id` 与 `seq`。
xlsx 的 `id` 必须是文本框 id `sheet:{name}/sp:…`。删掉对应 `xdr:sp`；若在组合里只摘该子节点，空组合再摘组合；锚点只剩 from/to/clientData 再摘整锚点。pptx 上发此 op 报 `InvalidField`。

### setTextBoxSize

字段与 `setPictureSize` 相同。docx 的 `id` 必须是框 id。xlsx 的 `id` 可以是文本框 id、形状 id 或连接线 id。
docx 写入该框 `wp:extent`；若 `wps:spPr/a:xfrm/a:ext` 存在，一并改。
xlsx 规则与图相同：`oneCell` / `absolute` 改 `xdr:ext`，`twoCell` 改右下角 `to`。pptx 上发此 op 报 `InvalidField`。

`setTextBoxAnchor` 见 [xlsx](json-contract-xlsx.md#settextboxanchor)。
`setTextBoxWrap` / `setTextBoxFill` / `setTextBoxLine`
 / `setTextBoxDirection` 见 [docx 分册](json-contract-docx.md#settextboxwrap)。

### insertTable

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "insertTable",
      "id": "docx:document",
      "rows": 2,
      "cols": 2,
      "cells": [
        [
          "A1",
          "B1"
        ],
        [
          "A2",
          "B2"
        ]
      ]
    }
  ]
}
```

- **`op`**
  - 必须是 `"insertTable"`
- **`id`**
  - 见 [docx 表 id](json-contract-docx.md#稳定-id) 与 [pptx 表 id](json-contract-pptx.md#稳定-id)：docx 为段落、`docx:document`、页眉页脚容器、表或单元格
  - pptx 必须是 `slide:{n}`、`slide:{n}/notes`、`master:{n}` 或 `master:{n}/layout:{k}`
- **`rows`**
  - 行数
  - docx：1–500
  - pptx：1–500
- **`cols`**
  - 列数
  - docx：1–63
  - pptx：1–75
- **`cells`**
  - 可选
  - 二维字符串，行数与列数必须与 `rows` / `cols` 一致
  - 省略则每格空文本
- **`x` / `y` / `cx` / `cy`**
  - pptx 可选
  - EMU
  - 成对规则见上
  - docx / xlsx 上出现报 `InvalidField`

写出带单线边框的 `w:tbl`，每列宽 1440 twip，每格至少一段并带新的 `w14:paraId`。插在单元格末尾时，该格末尾仍保留一个 `w:p`（可在新表之后补空段）。
`rows` / `cols` 越界或不是正整数报 `InvalidField`。id 形态不对报 `InvalidId`。找不到锚点报 `IdNotFound`。xlsx 上发此 op 报 `InvalidField`。
pptx：`id` 必须是 `slide:{n}`、`slide:{n}/notes`、`master:{n}` 或 `master:{n}/layout:{k}`；对表 id、形状 id、图 id 或 `pptx:presentation` 发此 op 报 `InvalidId`。
找不到该页报 `IdNotFound`。`rows` 为 1–500，`cols` 为 1–75，越界报 `InvalidField`。
`cells` 可选，必须是 `rows`×`cols` 的字符串数组，否则报 `InvalidField`。`x` / `y` 与 `cx` / `cy` 必须成对；只给一个报 `InvalidField`。
docx 上出现 `x` / `y` / `cx` / `cy` 报 `InvalidField`。pptx 都省略尺寸则用 `5486400` × `3200400`。
缺坐标时在该宿主画布上找空位，规则与 `insertPicture` 相同；往备注插时画布用 `p:notesSz`。
`id` 为 `slide:{n}/notes` 且该页还没有 notesSlide 时，按 `setNotesText` 同一套规则新建最小备注页再插表。
写入该宿主 `p:spTree` 一个含 `a:tbl` 的 `p:graphicFrame`（插在 `extLst` 之前），`a:graphicData/@uri` 为 DrawingML table URI。
`p:cNvPr/@id` 取该宿主已有全部 `cNvPr` 的最大整数加 1，至少为 2。`cNvPr/@name` 写成「表格 {该 id}」。
列宽按 `cx` 均分写入 `a:gridCol/@w`，行高按 `cy` 均分写入 `a:tr/@h`，余数落在最后一列 / 最后一行。
写出 `a:tblPr/@firstRow` 与 `@bandRow` 为 `1`，
并引用内置 `tableStyleId` `{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}`（PowerPoint 中等样式 2 - 强调 1）。
不写复杂主题拷贝，不算自动列宽。不写 `<p:timing>`。docx / xlsx 上不要用本 op 插幻灯片表。

### deleteTable

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteTable"` |
| `id` | 表 id |

删掉该 `w:tbl`。若删的是正文里仅剩的块级内容，在 `sectPr` 前补一个空的 `w:p`。若删的是单元格里的嵌套表，该格至少留下一个 `w:p`。表不存在报 `IdNotFound`。
id 不是表 id 报 `InvalidId`。xlsx 上发此 op 报 `InvalidField`。pptx：删掉对应 `p:graphicFrame`。
若在 `mc:AlternateContent` 的 Choice 里，整段 `AlternateContent` 删掉。组合里删子框后若组合空了，把空 `p:grpSp` 一并摘掉。不改其它对象的 `cNvPr/@id`。
按被删框的 `cNvPr/@id` 清掉本页 `<p:timing>` 里对应的 `p:tgtEl`（`p:spTgt/@spid`），规则与 `deleteShape` 相同。不整节无故删掉仍有内容的 `<p:timing>`。
找不到报表 `IdNotFound`。id 不是 pptx 表 id 报 `InvalidId`。

### insertRow

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertRow"` |
| `id` | 表 id（追加到末尾）或行 id（插到该行之后） |

新行按参照行（表 id 时用最后一行，行 id 时用该行）的 `gridSpan` 布局复制格子，不复制 `vMerge`，每格一段空文本。xlsx 上发此 op 报 `InvalidField`。
pptx：往 `a:tbl` 插入一行 `a:tr`。参照行的 `gridSpan` / `hMerge` 布局复制，不复制 `vMerge` / `rowSpan`，每格空 `a:txBody`。
行高抄参照行的 `a:tr/@h`，缺则均分当前框高。id 不是表 id 也不是行 id 报 `InvalidId`。某行占的列数对不上 `colCount` 报 `NotImplemented`，不写文件。

### deleteRow

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteRow"` |
| `id` | 行 id |

只剩一行时报 `InvalidField`。xlsx 上发此 op 报 `InvalidField`。pptx：删掉该 `a:tr`。只剩一行时报 `InvalidField`。id 不是行 id 报 `InvalidId`。
找不到报 `IdNotFound`。该行含 `rowSpan` 起点或 `vMerge` 继续格时报 `NotImplemented`，不写文件。

### insertCol

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertCol"` |
| `id` | 表 id |
| `after` | 在第几列之后插入。0 表示插到最前，取值 0–`colCount` |

新列宽抄相邻列；没有 `tblGrid` 时先按当前布局补上，缺宽则 1440 twip。插入点落在某格 `gridSpan` 内部时，该格 `gridSpan` 加 1，不新开 `w:tc`。否则每行在该处插入一个新格。
xlsx 上发此 op 报 `InvalidField`。pptx：在 `a:tblGrid` 插入 `a:gridCol`，宽度抄相邻列。
插入点落在某格 `gridSpan` 内部时，该格 `gridSpan` 加 1，并补一个 `hMerge` 继续格。否则每行插入一个空 `a:tc`。`after` 越界报 `InvalidField`。
某行占的列数对不上 `colCount` 报 `NotImplemented`。

### deleteCol

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteCol"` |
| `id` | 表 id |
| `col` | 要删的网格列，从 1 起 |

只剩一列时报 `InvalidField`。该列落在某格 `gridSpan` 内部时，`gridSpan` 减 1；减到 0 则删掉该 `w:tc`。xlsx 上发此 op 报 `InvalidField`。
pptx：删掉对应 `a:gridCol`。该列落在某格 `gridSpan` 内部时，`gridSpan` 减 1；纯 `hMerge` 继续格则删掉该 `a:tc`。只剩一列时报 `InvalidField`。
`col` 越界报 `InvalidField`。某行占的列数对不上 `colCount` 报 `NotImplemented`。

### insertHyperlink

- **`op`**
  - 必须是 `"insertHyperlink"`
- **`id`**
  - docx：段落 id（正文或页眉页脚）
  - xlsx：工作表 id `sheet:{name}`
  - pptx：文本形状、图、表、图表、SmartArt 或表单元格 id（不含 `notesMaster` / `handout`）
- **`text`**
  - docx 必填，显示文本，字符串，可空
  - xlsx 上出现报 `InvalidField`（显示字走 `setCell`）
  - pptx 仅当 `on` 为 `run` 或 `cell` 时可选，写入新 `a:r` 的 `a:t`
  - 框级出现报未知字段
- **`on`**
  - pptx 可选
  - `shape`（文本形状缺省）写到 `p:cNvPr`
  - `run` 写到文本形状 `a:rPr`
  - `pic` / `table` / `chart` / `smartArt` 写到对应框 `cNvPr`
  - `cell` 写到表单元格 `a:rPr`
  - 省略则按 `id` 种类：形状为 `shape`，图 / 表 / 图表 / SmartArt 为对应值，单元格为 `cell`
  - 与 `id` 种类对不上报 `InvalidField`
  - xlsx / docx 上出现报未知字段
- **`event`**
  - pptx 可选。`click`（缺省）或 `hover`
  - `click` 写 `a:hlinkClick`
  - `hover` 在 `cNvPr` 上写 `a:hlinkHover`，在 run 上写 `a:hlinkMouseOver`
  - xlsx / docx 上出现报未知字段
- **`url`**
  - 外部 URL
  - docx 与 `anchor` 必须恰好给一个
  - xlsx 与 `location` 至少给一个，可以同时给
  - pptx 与 `file` / `slide` 必须恰好给一个
- **`anchor`**
  - docx 内部书签名
  - xlsx / pptx 上出现报 `InvalidField`
- **`location`**
  - xlsx 内部位置（本工作簿的表单元格或定义名，或外部靶内位置）
  - docx / pptx 上出现报 `InvalidField`
- **`ref`**
  - xlsx 必填
  - 该链覆盖的 A1 或闭区间
  - docx / pptx 上出现报 `InvalidField`
- **`file`**
  - pptx 外部文件目标
  - xlsx / docx 上出现报 `InvalidField`
- **`slide`**
  - pptx 内部跳转目标，必须是 `slide:{n}`
  - xlsx / docx 上出现报 `InvalidField`
- **`tooltip`**
  - 可选
  - docx 写 `w:tooltip`
  - xlsx 写 `@tooltip`
  - pptx 写在对应 `a:hlinkClick` / `a:hlinkHover` / `a:hlinkMouseOver` 的 `tooltip`
- **`display`**
  - xlsx 可选
  - 写 `@display`
  - docx / pptx 上出现报 `InvalidField`

docx：在该段已有内容之后插入一个 `w:hyperlink`，内含一个带 `text` 的 run。
外部链在该部件 `.rels` 上新建 Type 以 `/hyperlink` 结尾的关系，`TargetMode` 为 `External`，`Target` 为 `url` 原文。内部链写 `w:anchor`，不写关系。
`url` 与 `anchor` 同时出现或一个都不给报 `InvalidField`。`url` 为空串报 `InvalidField`。`anchor` 为空串或以 `_` 开头报 `InvalidField`。
找不到段落报 `IdNotFound`。id 不是段落 id 报 `InvalidId`。

xlsx：在该表 `<hyperlinks>` 末尾追加一条 `<hyperlink>`。没有该元素则按 `CT_Worksheet` 顺序插在 `dataValidations` 之后、`printOptions` 之前。
`ref` 能解析则规范化成 `A1` 或 `A1:B2`；非法报 `InvalidRange`。与同一张表上已有链的单元格相交报 `InvalidField`。
给了 `url`：工作表 `.rels` 新建 Type 以 `/hyperlink` 结尾的关系，`TargetMode` 为 `External`，`Target` 为 `url` 原文，并写 `r:id`。
给了 `location`：写 `@location`。只给 `location` 不写关系。`url` 与 `location` 一个都不给报 `InvalidField`。
`url` 或 `location` 为空串报 `InvalidField`。工作表根上补 `xmlns:r`（若还没有）。不写单元格值，也不改已有格子的样式。id 不是工作表 id 报 `InvalidId`。
找不到工作表报 `IdNotFound`。只能写 `worksheet`；对 `chartsheet` 报 `InvalidField`；对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。
不标重算。`create` 给了 `sheets[].hyperlinks` 时按同一套规则写 `<hyperlinks>`。

pptx：`url`、`file`、`slide` 必须恰好一个，否则报 `InvalidField`。空串报 `InvalidField`。
框级（`on` 为 `shape` / `pic` / `table` / `chart` / `smartArt`，或按 id 种类缺省）：
在该对象 `cNvPr` 下写入 `a:hlinkClick`（`event` 缺省或 `click`）或 `a:hlinkHover`（`event` 为 `hover`）。
`cNvPr` 子顺序为 `hlinkClick`、`hlinkHover`、`extLst`。
同一宿主同一 `event` 的框级链已有时报 `InvalidField`（改地址请用 `setHyperlink`）。不改 `p:txBody`。
`on` 为 `run`（仅文本形状）或 `cell`：写入该框或该格最后一段里 run 的 `a:rPr/a:hlinkClick`（`event` 为 `click`）或 `a:hlinkMouseOver`（`event` 为 `hover`）。
给了 `text`：在 `endParaRPr` 之前追加一个新 `a:r`，抄该段最后一个 `a:r` / `a:fld` 的 `a:rPr`（没有则抄 `endParaRPr`），去掉同事件已有链再写入新链，`a:t` 为 `text`。
没给 `text`：把链写到该段最后一个 `a:r`；该 run 同一 `event` 已有链时报 `InvalidField`；没有 `a:r` 时报 `InvalidField`。不写下划线或主题超链接色。
不认 `a:fld` / `a:br` / `endParaRPr` 上的链。不把 `ppaction://media` 收进树。不收 Zoom。
`on` 与 `id` 种类对不上报 `InvalidField`。`event` 不是 `click` / `hover` 报 `InvalidField`。
id 必须是文本形状、图、表、图表、SmartArt 或表单元格 id。
对备注母版、讲义、批注内图、页 id 或 `pptx:presentation` 报 `InvalidId`。找不到对象报 `IdNotFound`。
给了 `url`：该页 `.rels` 新建 Type 以 `/hyperlink` 结尾的关系，`TargetMode` 为 `External`，`Target` 为 `url` 原文，不写 `action`。
给了 `file`：同样写 External hyperlink 关系，并写 `action="ppaction://hlinkfile"`。
给了 `slide`：必须是已有页的 `slide:{n}`，不能是本页；目标页不存在报 `IdNotFound`；跳到自己报 `InvalidField`。
写入 Type 以 `/slide` 结尾的内部关系，`Target` 为相对目标页部件，并写 `action="ppaction://hlinksldjump"`。幻灯片根上补 `xmlns:r`（若还没有）。不写放映宏动作。
create 给了 `shapes[].url` / `file` / `slide` 时按同一套规则写链，可 `on=run`，可 `event=hover`。
create 上 `slide` 是页序整数，见 [json-contract-pptx.md](json-contract-pptx.md#create)。create 本期不写图 / 表 / 图表 / SmartArt 上的链。

### setHyperlink

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setHyperlink"` |
| `id` | 超链接 id。docx 为 `link:{n}` 或容器前缀；xlsx 为 `sheet:{name}/link:{n}`；pptx 为 `slide:{n}/link:{k}` |
| `url` | 可选。改成外部 URL。xlsx 为 `null` 则去掉 `r:id`。pptx 为 `null` 或空串报 `InvalidField` |
| `file` | pptx 可选。改成外部文件目标。xlsx / docx 上出现报 `InvalidField` |
| `slide` | pptx 可选。改成内部跳转 `slide:{n}`。xlsx / docx 上出现报 `InvalidField` |
| `anchor` | docx 可选。改成内部书签名。xlsx / pptx 上出现报 `InvalidField` |
| `location` | xlsx 可选。写 `@location`；`null` 去掉该属性。docx / pptx 上出现报 `InvalidField` |
| `text` | docx 可选。改显示文本。xlsx / pptx 上出现报 `InvalidField` |
| `tooltip` | 可选。`null` 表示去掉提示 |
| `display` | xlsx 可选。写 `@display`；`null` 去掉该属性。docx / pptx 上出现报 `InvalidField` |
| `ref` | xlsx 可选。改覆盖区域。docx / pptx 上出现报 `InvalidField` |

docx：`url`、`anchor`、`text`、`tooltip` 至少给一个。`url` 与 `anchor` 不能同时出现。
给了 `url`：有已有 `r:id` 则改该关系 `Target`；没有则新建关系并写 `r:id`，去掉 `w:anchor`。给了 `anchor`：写 `w:anchor`，去掉 `r:id`，该关系无人引用则删除。
`text` 只改该 `w:hyperlink` 内可见 `w:t`（规则与 `setParaText` 第 2–4 条相同，范围限于该元素）。`tooltip` 为字符串则写属性；为 `null` 则去掉属性。
找不到报 `IdNotFound`。id 不是超链接 id 报 `InvalidId`。

xlsx：`url`、`location`、`tooltip`、`display`、`ref` 至少给一个。
给了 `url` 字符串：有已有 `r:id` 且该关系 Type 以 `/hyperlink` 结尾则改 `Target`；否则新建关系并写 `r:id`。`url` 为 `null`：去掉 `r:id`，该关系无人引用则删除。
给了 `location` 字符串则写属性；为 `null` 则去掉属性。改完之后必须仍有 `r:id` 或 `location`，否则报 `InvalidField`。
`ref` 非法报 `InvalidRange`；与其它链相交报 `InvalidField`。`tooltip` / `display` 为字符串则写属性；为 `null` 则去掉。不改单元格值。找不到报 `IdNotFound`。
id 不是超链接 id 报 `InvalidId`。只能改 `worksheet`。

pptx：`url`、`file`、`slide`、`tooltip` 至少给一个。`url`、`file`、`slide` 最多给一个，多了报 `InvalidField`。
给了 `url` / `file` / `slide`：改该链元素（`a:hlinkClick` / `a:hlinkHover` / `a:hlinkMouseOver`）的关系与 `action`，
不论宿主是 `cNvPr` 还是 `a:rPr`，规则与 `insertHyperlink` 相同（可从 URL 改成跳转或文件）。
`url` / `file` / `slide` 为 `null` 或空串报 `InvalidField`（不能只拆链；拆链请用 `deleteHyperlink`）。给了 `tooltip` 字符串则写属性；为 `null` 则去掉属性。
不改形状可见文字，也不改 run 的 `a:t`。找不到报 `IdNotFound`。id 不是超链接 id 报 `InvalidId`。目标页不存在报 `IdNotFound`。跳到自己报 `InvalidField`。旧关系在本页 XML 里无人引用则删除。

### deleteHyperlink

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteHyperlink"` |
| `id` | 超链接 id |

docx：拆掉 `w:hyperlink` 外壳，留下内部 `w:r`。该 `r:id` 在本部件 XML 里无人引用则删除关系。找不到报 `IdNotFound`。id 不是超链接 id 报 `InvalidId`。

xlsx：去掉该 `<hyperlink>`。该 `r:id` 在本表其余链里无人引用则删除关系。删光则去掉 `<hyperlinks>`。不改单元格值。找不到报 `IdNotFound`。
id 不是超链接 id 报 `InvalidId`。只能改 `worksheet`。

pptx：去掉该条 `a:hlinkClick`。形状级从 `p:cNvPr` 摘掉；run 级从 `a:rPr` 摘掉，`a:r` 与 `a:t` 留下。该 `r:id` 在本页 XML 里无人引用则删除关系。不改可见文字。
找不到报 `IdNotFound`。id 不是超链接 id 报 `InvalidId`。

示例见 `docs/examples/docx-apply-hyperlink.json`、`docs/examples/pptx-apply-hyperlink.json`。

### insertComment

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertComment"` |
| `start` | docx：范围起始段落 id：正文、页眉页脚、框内或注内。pptx / xlsx 上出现报 `InvalidField` |
| `end` | docx：范围结束段落 id，可与 `start` 相同。pptx / xlsx 上出现报 `InvalidField` |
| `id` | pptx：页 id `slide:{n}`。xlsx：单元格 id `cell:{name}!{A1}`。docx 上出现报 `InvalidField` |
| `x` / `y` | pptx 可选。画布坐标，EMU，非负整数，必须成对。docx / xlsx 上出现报 `InvalidField` |
| `text` | 可选。新批注第一段的可见文本，省略当作 `""` |
| `author` | 可选。省略当作 `"ooxp"` |
| `initials` | 可选。省略当作 `"oo"`。xlsx 接受但不写入 XML |
| `date` | 可选。`xsd:dateTime` 字符串。docx 省略则不写 `w:date`；pptx / xlsx 省略则写入当前 UTC（无尾 `Z`） |

`start` 与 `end` 必须在同一部件，且 `start` 在文档顺序上不得晚于 `end`，否则报 `InvalidField`。不提供字符偏移。
在 `start` 段 `w:pPr` 之后插入 `w:commentRangeStart`；
在 `end` 段已有内容之后插入 `w:commentRangeEnd`，再跟一个含 `w:commentReference` 的 run（`rStyle` 为 `CommentReference`）。跨段时中间段落整段落在范围内，不另写标记。

分配尚未占用的 `w:id`（十进制，常见从 `0` 起）。没有 `/word/comments.xml` 则新建该部件、主文档关系、内容类型 Override。
同时按 Word 16.0 最小集补 `/word/commentsExtended.xml` 与 `/word/people.xml`（已有则改，没有则建）。
新 `w:comment` 至少一段，带新的 `w14:paraId`，第一段含 `w:annotationRef`，段落样式引用 `CommentText`，再跟 `text`。不在 `styles.xml` 里新建这些样式。
`commentEx/@paraId` 写成该批注最后一段的 `paraId`，`done` 为 `0`。`people` 里没有该 `author` 则追加一条。
`commentsIds` / `commentsExtensible` 只在文件里已有时给这条根批注补一项；没有这两份部件则不新建。

对图 id、容器 id、注 id、批注 id 或 `section:{n}` 作 `start` / `end` 报 `InvalidId`。
xlsx 不认 `start` / `end` / `x` / `y`；出现报 `InvalidField`。
xlsx 的 `id` 必须是单元格 id，对工作表 id、图 id 或 `xlsx:workbook` 报 `InvalidId`。
只能写 `worksheet`；对 `chartsheet` 报 `InvalidField`；对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。
写入该表线程化批注部件（MS-XLSX `ThreadedComments`，内容类型 `application/vnd.ms-excel.threadedcomments+xml`），
工作表 `.rels` 的 Type 必须是完整 URI `http://schemas.microsoft.com/office/2017/10/relationships/threadedComment`。
没有则新建 `/xl/threadedComments/threadedCommentN.xml`。
没有 `/xl/persons/person.xml` 则新建（内容类型 `application/vnd.ms-excel.person+xml`），
workbook `.rels` 的 Type 为 `http://schemas.microsoft.com/office/2017/10/relationships/person`。
按 `displayName` 复用已有 `person`；否则新建，`providerId` 为 `None`，`userId` 为 `Name`。`author` 缺省 `"ooxp"`。
`date` 省略则写入当前 UTC `xsd:dateTime`（无尾 `Z`）。新 `threadedComment/@id` 为带花括号的 GUID，`@ref` 为该格 A1。
同时按 Excel 16.0 另存写占位 `/xl/commentsN.xml`（作者 `tc={GUID}`，`xr:uid` 同 GUID）以及 Note `v:shape` 与工作表 `<legacyDrawing>`。
不把旧式 VML 批注当树节点，也不改已有框的几何。同一张表多条批注共用一个 threadedComments 部件。
`create` 给了 `sheets[].comments` 时按同一套规则写线程化批注，字段用 `cell` 代替 `id`，不写回复。pptx 不认 `start` / `end`；出现这两个字段报 `InvalidField`。
pptx 的 `id` 必须是 `slide:{n}`，对备注容器 id、图 id、形状 id 或 `pptx:presentation` 报 `InvalidId`。
可选 `x` / `y` 为 EMU 非负整数，必须成对出现或成对省略；只给一个时报 `InvalidField`。缺坐标则放到该页空白处，规则与 pptx `insertPicture` 相同。
写入该页现代批注部件（MS-PPTX 2018/8 `p188:cmLst`，内容类型 `application/vnd.ms-powerpoint.comments+xml`），
幻灯片 `.rels` 的 Type 为 `http://schemas.microsoft.com/office/2018/10/relationships/comments`（必须比完整 URI，不要把 2006 旧式 comments 关系当成现代批注）。
没有则新建 comments 部件，并在幻灯片 `p:extLst` 写入 `p188:commentRel`（uri `{6950BFC3-D8DA-4A85-94F7-54DA5524770B}`）。
没有 `/ppt/authors.xml` 则新建（内容类型 `application/vnd.ms-powerpoint.authors+xml`），
presentation `.rels` 的 Type 为 `http://schemas.microsoft.com/office/2018/10/relationships/authors`。
按 `name` 加 `initials` 复用已有 `p188:author`；否则新建，`providerId` 为 `None`，`userId` 取新 GUID 的十六进制前 16 位。
`author` / `initials` 缺省 `"ooxp"` / `"oo"`。`date` 省略则写入当前 UTC `xsd:dateTime`（无尾 `Z`，与 PowerPoint 16 另存一致）。
新 `p188:cm/@id` 为带花括号的 GUID。`pc:sldMk/@sldId` 取该页 `p:sldId/@id`（无前缀那一项）。不写 ECMA-376 §19.4 旧式 `commentAuthors`。
同一页多条批注共用一个 comments 部件。`create` 给了 `slides[].comments` 时按同一套规则写现代批注。

### replyComment

- **`op`**
  - 必须是 `"replyComment"`
- **`id`**
  - 父批注 id
  - docx 为 `comment:0`
  - pptx 为 `slide:{n}/comment:{guid}`
  - xlsx 为 `cell:{name}!{A1}/comment:{guid}`
- **`text`**
  - 可选
  - 新回复第一段的可见文本，省略当作 `""`
- **`author` / `initials` / `date`**
  - 与 `insertComment` 相同

新增一条 `w:comment` 和新的 `commentEx`，`paraIdParent` 为父批注当前最后一段的 `paraId`。
在父批注已有的范围上再套一套相同范围的标记（`commentRangeStart` 插在父 start 之后，
`commentRangeEnd` 与 `commentReference` 插在父 `commentReference` 所在 run 之后），与 Word 16 `Replies.Add` 一致；`rStyle` 抄父引用 run。
找不到父批注的范围标记时仍写 `comment` 与 `commentEx`，不报错。inspect 的 `start` / `end` 因此与线程根相同。父批注最后一段没有 `w14:paraId` 时报 `InvalidField`。
父 id 不是批注容器 id 报 `InvalidId`。找不到父批注报 `IdNotFound`。对 `kind` 为 `ink` 的批注发此 op 报 `NotImplemented`。
xlsx 的 `id` 必须是父批注容器 id，不能是回复 id 或单元格 id；对回复 id 报 `InvalidId`。写入同部件一条带 `parentId` 的 `threadedComment`，`ref` 与父相同。
新回复 `id` 为 `{父 id}/reply:{guid}`。只能写 `worksheet`。
pptx 的 `id` 必须是父批注容器 id（`slide:{n}/comment:{guid}`），不能是回复 id；对回复 id 报 `InvalidId`。写入该 `p188:cm` 的 `p188:replyLst`。
新回复 `id` 为 `{父 id}/reply:{guid}`。找不到父批注报 `IdNotFound`。对 `kind` 为 `ink` 的批注发此 op 报 `NotImplemented`。

### deleteComment

- **`op`**
  - 必须是 `"deleteComment"`
- **`id`**
  - 批注 id
  - docx 为 `comment:0`
  - pptx 为 `slide:{n}/comment:{guid}` 或回复 id
  - xlsx 为 `cell:{name}!{A1}/comment:{guid}` 或回复 id

去掉该批注在主文档、页眉、页脚、文本框和注里所有匹配的范围标记与 `commentReference`，再去掉 `comments.xml` 里该条。子回复级联删除（`paraIdParent` 链指向它的都删），清单里验回复不残留。
已经不存在报 `IdNotFound`。id 不是批注容器 id 报 `InvalidId`。批注内图按图片规则收。
删光批注后仍保留 `comments.xml` 空壳和已有的 extended / people 部件，不拆掉整个 part。xlsx 的 `id` 可以是顶层批注或回复。
删顶层即级联丢掉其 `parentId` 指向它的回复，并去掉对应占位 `comment` 与 Note `v:shape`。删回复只去掉那条 `threadedComment`。已经不存在报 `IdNotFound`。
id 不是上述两种报 `InvalidId`。删光后仍保留空的 `ThreadedComments`、`comments.xml` 空壳与 `person.xml`，不拆 part。
pptx 的 `id` 可以是 `slide:{n}/comment:{guid}` 或回复 id。删父 `p188:cm` 即级联丢掉其 `replyLst`。删回复只去掉对应 `p188:reply`。
已经不存在报 `IdNotFound`。id 不是上述两种报 `InvalidId`。删光后仍保留空的 `p188:cmLst` 与 `authors.xml` 空壳，不拆 part。墨迹批注可以删。

### setCommentText

- **`op`**
  - 必须是 `"setCommentText"`
- **`id`**
  - 批注容器 id
  - docx 为 `comment:0`
  - pptx 为 `slide:{n}/comment:{guid}` 或回复 id
  - xlsx 为 `cell:{name}!{A1}/comment:{guid}` 或回复 id
- **`text`**
  - 新的可见纯文本，字符串，可空

只改该 `w:comment` 的可见文本，不改范围标记。留下第一段（含 `annotationRef`），按 `setParaText` 的规则改这段的可见 `w:t`，删掉后面多出来的 `w:p`。
随后把 `commentEx/@paraId` 和指向旧最后一段的 `paraIdParent` 改成留下的那一段。id 不是批注容器 id 报 `InvalidId`。对批注内段落 id 请用 `setParaText`。
对 `kind` 为 `ink` 的批注报 `NotImplemented`，没有可见 `w:t` 时同样拒绝，不写文件。xlsx 的 `id` 可以是批注容器 id 或回复 id。
只改对应 `threadedComment` 的 `<text>`。pptx 的 `id` 可以是批注容器 id 或回复 id。只改对应 `p188:txBody` 的可见文本，规则与 `setShapeText` 相同。
对 `kind` 为 `ink` 的批注报 `NotImplemented`，没有可见 `txBody` 文本时同样拒绝。

### setCommentMeta

- **`op`**
  - 必须是 `"setCommentMeta"`
- **`id`**
  - 批注容器 id
  - docx 为 `comment:0`
  - pptx 为 `slide:{n}/comment:{guid}` 或回复 id
  - xlsx 为 `cell:{name}!{A1}/comment:{guid}` 或回复 id
- **`author`**
  - 可选
- **`initials`**
  - 可选
  - xlsx 接受但不写入 XML
- **`date`**
  - 可选
  - `xsd:dateTime` 字符串

`author`、`initials`、`date` 至少给一个，否则报 `InvalidField`。只改被点名的属性，其余不动。改 `author` 时若 `people.xml` 里还没有这个作者则补一条，不删旧作者。
id 不是批注容器 id 报 `InvalidId`。xlsx 的 `id` 可以是批注容器 id 或回复 id。
改 `author` 时按新的 `displayName` 指向（必要时新建）一条 `person`，只改该条 `personId`，不改共享的那条 person，以免误伤同作者其它批注。`initials` 接受但不写入。
改 `date` 时写 `@dT`。pptx 的 `id` 可以是批注容器 id 或回复 id。
改作者或缩写时按新的 `name` / `initials` 指向（必要时新建）一条 `p188:author`，只改该 `cm` 或 `reply` 的 `authorId`，不改共享的那条 author 记录，以免误伤同作者其它批注。
`kind` 为 `ink` 的批注同样可以改这些字段，不改正文或幻灯片上的墨迹画。

### setCommentDone

- **`op`**
  - 必须是 `"setCommentDone"`
- **`id`**
  - 批注容器 id
  - docx 为 `comment:0`
  - pptx 为 `slide:{n}/comment:{guid}` 或回复 id
  - xlsx 为 `cell:{name}!{A1}/comment:{guid}` 或回复 id
- **`done`**
  - 布尔
  - docx：`true` 写成 `commentEx/@done="1"`，`false` 写成 `"0"`
  - xlsx：`true` 写 `@done="1"`，`false` 去掉 `@done`

只改被点名这一条的 `done`，不自动改回复。没有 `commentsExtended` 则按 Word 16 最小集新建，并为文件里每条批注补 `commentEx`（默认 `done="0"`），再改被点名的那条。
父批注最后一段没有 `w14:paraId` 时报 `InvalidField`。xlsx 的 `id` 可以是批注容器 id 或回复 id。`done: true` 写 `@done="1"`；`false` 去掉 `@done`。
pptx 的 `id` 可以是批注容器 id 或回复 id。`done: true` 写 `status="resolved"`；`false` 去掉 `status`（与另存样例默认无 status、即 active 一致）。`kind` 为 `ink` 的批注同样可以改完成标记。

### 图表 op 的共同限制

`type` 为 `other` 或 `chartex` 时，除 `deleteChart`、`setChartSize`、`setChartPos`、`setChartWrap`、`setChartAnchor` 外一律报 `NotImplemented`，不写文件。
xlsx 没有 `setChartWrap` / `setChartPos`，对应例外是 `deleteChart`、`setChartSize`、`setChartAnchor`。
pptx 没有 `setChartWrap` / `setChartAnchor`，对应例外是 `deleteChart`、`setChartSize`、`setChartPos`。
`flags` 含 `secondaryAxis`、`trendline`、`errorBars` 不再整张锁死。
`flags` 含 `pivot` 时：`setChartSeries`、`addChartSeries`、`deleteChartSeries`、`setChartType` 报 `NotImplemented`；
装饰 op、删图、尺寸、位置、绕排、锚点，以及趋势线 / 误差线 / 次轴 op 可以。
`type` 为 `combo` 不再整张锁死，系列留在原来的包裹里改。`type` 为 `stock`、`surface` 或 `ofPie` 时：`setChartSeries` 与装饰 op 可以。
`addChartSeries`、`deleteChartSeries` 仍报 `NotImplemented`（股价必须保持与 `stockKind` 一致的条数，曲面必须保持三条，复合饼必须保持一条）。
`setChartType` 可以在系列条数对得上时转入或转出这三种（见该 op）。`insertChart` / create 接受 `stock`、`surface`、`ofPie` 与带 `layoutId` 的 `chartex`。
已写入的 `chartex` 仍除删除与尺寸 / 位置 / 绕排 / 锚点外锁死。xlsx 上发 `setChartWrap` / `setChartPos` 报 `InvalidField`。
docx 上发 `setChartAnchor` / `setChartPos` 报 `InvalidField`。pptx 上发 `setChartWrap` / `setChartAnchor` 报 `InvalidField`。`insertChart` 不能插入透视图表。

docx 改系列数据时必须同时改 chart 部件里的 cache（`c:numCache` / `c:strCache` / `c:numLit` / `c:strLit`）以及 embedding 工作簿里公式指向的单元格。
没有 embedding 则只改字面量。不在库里求值公式，不改 `c:f` 里工作表名以外的结构，除非新数据的点数与公式区域长度不同：那时把公式区域扩到或收到新长度，并改 embedding 里对应格子。未点名的系列不动。

xlsx 没有 embedding。有可解析的连续 A1 `c:f` 时：把数组写入这些格子（按 `setCell` 语义），点数变了就按原起点扩或收公式区域，并回写 cache。
`c:f` 是工作簿 `definedNames` 能解析成**一段**连续 A1 的名字（可带表名前缀）时，按解析结果写入格子；
新值长度与该区域格子数不同则报 `NotImplemented`，不改定义名本身，也不把 `c:f` 改成 A1。
`c:f` 是逗号并集的多段连续 A1 时：新值长度与各段格子总数相同则按文档顺序装入，不改 `c:f`；长度不同、或某一段不是连续 A1，报 `NotImplemented`。
三维引用、带运算符或函数调用的定义名报 `NotImplemented`。没有 `c:f`（只有 lit）时只改 lit / cache，不新造单元格区域。写入单元格时走与 `setCell` 同一套重算标记。未点名的系列不动。

### setChartTitle

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setChartTitle"` |
| `id` | 图表 id |
| `title` | 字符串。空串表示去掉标题（写 `c:autoTitleDeleted val="1"`，去掉 `c:title`） |

找不到该图报表 `IdNotFound`。id 不是图表 id 报 `InvalidId`。

### setChartSeries

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setChartSeries"` |
| `id` | 系列 id，例如 `para:3F2A1B00/chart:7/series:0` 或 `sheet:Sheet1/chart:2/series:0` |
| `name` | 可选。系列名 |
| `categories` | 可选。分类数组，元素为字符串或数字。气泡图对应 `c:xVal` |
| `values` | 可选。数值数组，元素为字符串或数字。气泡图对应 `c:yVal` |
| `sizes` | 可选。气泡大小数组，对应 `c:bubbleSize`。只对气泡系列有意义 |

`name`、`categories`、`values`、`sizes` 至少给一个，否则报 `InvalidField`。只给的字段改，其余保留。`categories` 与 `values` 都给时长度可以不同。
对非气泡系列给 `sizes` 报 `InvalidField`。找不到该系列报 `IdNotFound`。id 不是系列 id 报 `InvalidId`。

### addChartSeries

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"addChartSeries"` |
| `id` | 图表 id |
| `name` | 可选。省略当作 `"系列{n}"`，`n` 为从 1 起的新序号 |
| `categories` | 可选。省略则抄已有第一条系列的分类；图表还没有系列且仍省略，报 `InvalidField` |
| `values` | 必须。非空数组 |
| `sizes` | 可选。只对气泡图有意义；省略则写成与 `values` 等长的 `1` |
| `type` | 可选。只对组合图有意义：`bar`、`col`、`line`、`area` 或 `scatter`。省略则抄最后一条系列的包裹。对单类型图给 `type` 报 `InvalidField` |

在 chart 部件末尾追加一条 `c:ser`，`c:idx` / `c:order` 取尚未占用的最小值。样式能从最后一条系列抄则抄，抄不了就用最小默认。docx 有 embedding 则在工作表右侧追加一列。
xlsx 能从已有系列的 `c:f` 推出右侧相邻空列则写入；推不出报 `NotImplemented`。不能用本 op 把单类型图加成组合图。id 不是图表 id 报 `InvalidId`。

### deleteChartSeries

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteChartSeries"` |
| `id` | 系列 id |

删掉该 `c:ser`。docx 若能从公式定位则同步 embedding 里对应列。xlsx 不删单元格。删完后一条都不剩时报 `InvalidField`（要整张去掉请用 `deleteChart`）。
找不到报 `IdNotFound`。id 不是系列 id 报 `InvalidId`。

### setChartType

- **`op`**
  - 必须是 `"setChartType"`
- **`id`**
  - 图表 id
- **`type`**
  - 包括
    - `bar`
    - `col`
    - `line`
    - `pie`
    - `area`
    - `scatter`
    - `doughnut`
    - `radar`
    - `bubble`
    - `stock`
    - `surface`
    - `ofPie`
- **`ofPieType`**
  - 可选
  - 只对 `type` 为 `ofPie` 有意义：`pie`（复合饼）或 `bar`（复合条）
  - 省略则：已是复合饼则保留原值，否则写 `pie`
  - 其它 `type` 上出现报 `InvalidField`
- **`stockKind`**
  - 可选
  - 只对 `type` 为 `stock` 有意义：`hlc` 或 `ohlc`
  - 省略则：三条系列当 `hlc`，四条当 `ohlc`
  - `vhlc` / `vohlc` 只对 `insertChart` 有意义，本 op 上出现报 `InvalidField`
  - 其它 `type` 上出现报 `InvalidField`
- **`surfaceKind`**
  - 可选
  - 只对 `type` 为 `surface` 有意义：`surface3d`、`wireframe`、`contour` 或 `contourWire`
  - 省略则：已是曲面则保留原值，否则写 `surface3d`
  - 其它 `type` 上出现报 `InvalidField`

不认识的枚举、或值为 `combo`、`chartex`，报 `InvalidField`。当前 `type` 为 `other` 或 `chartex` 时报 `NotImplemented`，不写文件。
转成 `stock` 必须恰好三条（`hlc`）或四条（`ohlc`）系列，否则 `NotImplemented`。从组合图转成股价报 `NotImplemented`。
转成 `surface` 必须恰好三条系列，否则 `NotImplemented`。
转成 `ofPie` / `pie` / `doughnut` 必须恰好一条系列，否则 `NotImplemented`。从股价、曲面、复合饼转走时：系列结构按下面同一套规则改包裹；对不上则 `NotImplemented`。
只改系列的包裹元素（`c:barChart`、`c:stockChart`、`c:surface3DChart`、`c:surfaceChart`、`c:ofPieChart` 等）和 `c:barDir` / `c:ofPieType` / `c:wireframe`。
`bar` 与 `col` 只改 `barDir`。`bar` / `col` / `line` / `area` / `radar` 互换时留下 `c:ser`。
转成 `stock` 写 `c:stockChart` 与 `c:hiLowLines`；`ohlc` 再写 `c:upDownBars`。
转成 `surface` 按 `surfaceKind` 写 `c:surface3DChart` 或 `c:surfaceChart`、第三条 `c:serAx` 与 `c:view3D`。
转成 `ofPie` 去掉轴。从曲面转走时去掉 `c:serAx` 与 `c:view3D`。`pie` ↔ `doughnut` 只换包裹，必须恰好一条系列，否则 `NotImplemented`。
转成 `pie` 或 `doughnut` 时必须恰好一条系列，否则 `NotImplemented`。与 `scatter` 互转：分类当 `xVal`、数值当 `yVal`，或反过来；系列结构对不上则 `NotImplemented`。
`scatter` → `bubble`：留下 `xVal` / `yVal`，补上与 `yVal` 等长的 `bubbleSize`（值均为 `1`）。`bubble` → `scatter`：丢掉 `bubbleSize`。
组合图收到单一类型时，把所有包裹收成那一种；对不上（例如多系列转饼）则 `NotImplemented`。单一类型不能经本 op 变成 `combo`，请用 `insertChart`。不要写出 Word 要修复的包。
id 不是图表 id 报 `InvalidId`。找不到报 `IdNotFound`。

### setChartAxisTitle

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setChartAxisTitle"` |
| `id` | 图表 id |
| `axis` | `cat`（主分类轴 / 散点图横轴）或 `val`（主数值轴） |
| `title` | 字符串。空串表示去掉该轴标题 |

`axis` 不认识报 `InvalidField`。找不到对应主轴报 `InvalidField`。id 不是图表 id 报 `InvalidId`。找不到图报表 `IdNotFound`。

### setChartLegend

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setChartLegend"` |
| `id` | 图表 id |
| `show` | 布尔。`false` 时去掉 `c:legend` |
| `pos` | 可选。`b`、`t`、`l`、`r`、`tr`。`show` 为 `true` 且省略时：已有图例则保留原位置，没有则写 `r` |

`show` 为 `false` 时忽略 `pos`。不认识的 `pos` 报 `InvalidField`。id 不是图表 id 报 `InvalidId`。

### setChartDataLabels

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setChartDataLabels"` |
| `id` | 图表 id |
| `show` | 布尔 |

`true`：在 `plotArea` 写 `c:dLbls`，`c:showVal` 为 `1`，不调标签坐标。`false`：去掉 `plotArea` 与各系列上的 `c:dLbls`。id 不是图表 id 报 `InvalidId`。

### insertTrendline

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertTrendline"` |
| `id` | 系列 id |
| `type` | `linear`、`exp`、`log`、`poly`、`power` 或 `movingAvg` |
| `order` | 可选。多项式阶数，正整数。只对 `poly` 有意义；省略则写 `2` |
| `period` | 可选。移动平均周期，正整数。只对 `movingAvg` 有意义；省略则写 `2` |
| `displayRSquared` | 可选。布尔。省略则 `false` |
| `displayEquation` | 可选。布尔。省略则 `false` |
| `name` | 可选。趋势线名称 |

在该 `c:ser` 末尾追加一条 `c:trendline`（插在 `c:cat` / `c:val` / `c:errBars` / `c:extLst` 之前）。
`poly` 以外给 `order`、或 `movingAvg` 以外给 `period`，报 `InvalidField`。饼、圆环、复合饼、曲面、`other`、`chartex` 上报 `InvalidField`。
id 不是系列 id 报 `InvalidId`。找不到系列报 `IdNotFound`。

### setTrendline

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTrendline"` |
| `id` | 趋势线 id |
| `type` / `order` / `period` / `displayRSquared` / `displayEquation` / `name` | 可选。规则与 `insertTrendline` 相同 |

至少给一个可写字段，否则报 `InvalidField`。只改被点名的字段。把 `type` 改成非 `poly` 时去掉 `c:order`；改成非 `movingAvg` 时去掉 `c:period`。
id 不是趋势线 id 报 `InvalidId`。找不到报 `IdNotFound`。

### deleteTrendline

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteTrendline"` |
| `id` | 趋势线 id |

删掉该 `c:trendline`。id 不是趋势线 id 报 `InvalidId`。找不到报 `IdNotFound`。

### insertErrorBars

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertErrorBars"` |
| `id` | 系列 id |
| `dir` | 可选。`x` 或 `y`。省略则 `y` |
| `barType` | 可选。`both`、`plus` 或 `minus`。省略则 `both` |
| `valType` | 可选。`stdErr`、`stdDev`、`fixedVal`、`percentage` 或 `cust`。省略则 `stdErr` |
| `value` | 可选。数字。`stdDev` / `fixedVal` / `percentage` 时必须给；`stdErr` / `cust` 时禁止给 |
| `plus` | 可选。非空数字数组。只对 `cust` 有意义 |
| `minus` | 可选。非空数字数组。只对 `cust` 有意义 |

写出一条 `c:errBars`。`valType` 为 `cust` 时必须给 `plus` 与 `minus`，写成 `c:plus` / `c:minus` 的 `c:numLit`，不必与系列点数相同。
非 `cust` 时出现 `plus` / `minus` 报 `InvalidField`。`lCaps` 不是合法 `valType`，报 `InvalidField`。
`dir` 为 `y` 或缺省时不写 `c:errDir`；`dir` 为 `x` 时写 `c:errDir val="x"`。
该系列已有同 `dir` 的误差线时报 `InvalidField`。
饼、圆环、复合饼、曲面、股价、`other`、`chartex` 上报 `InvalidField`。id 不是系列 id 报 `InvalidId`。

### setErrorBars

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setErrorBars"` |
| `id` | 误差线 id |
| `dir` / `barType` / `valType` / `value` / `plus` / `minus` | 可选。规则与 `insertErrorBars` 相同 |

至少给一个可写字段。改成 `stdErr` 时去掉 `c:val`。改成 `cust` 时必须有 `plus` 与 `minus`（本 op 给的，或原节点已有）。
改成非 `cust` 时去掉 `c:plus` / `c:minus`。`lCaps` 报 `InvalidField`。id 不是误差线 id 报 `InvalidId`。

### deleteErrorBars

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteErrorBars"` |
| `id` | 误差线 id |

删掉该 `c:errBars`。找不到报 `IdNotFound`。

### setChartSeriesAxis

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setChartSeriesAxis"` |
| `id` | 系列 id |
| `axis` | `primary` 或 `secondary` |

把该系列所在包裹的数值轴 `c:axId` 指到主轴或次轴。没有次轴时按主数值轴复制一条，写 `c:overlay val="1"`、`c:crosses val="max"`、新的 `c:axId`。
同一包裹里还有别的系列且目标轴不同时，把被点名系列拆到同类型的新包裹。已经是目标状态则空操作。饼、圆环、复合饼、曲面、股价、`other`、`chartex` 上报 `InvalidField`。
`axis` 不认识报 `InvalidField`。id 不是系列 id 报 `InvalidId`。

### insertChart

- **`op`**
  - 必须是 `"insertChart"`
- **`id`**
  - docx：宿主段落 id（正文、页眉页脚、框内、注内或批注内段落）
  - xlsx：工作表 id `sheet:{name}`
  - pptx：页 id `slide:{n}`、备注容器 id `slide:{n}/notes`、母版 id `master:{n}` 或版式 id `master:{n}/layout:{k}`
- **`type`**
  - 包括
    - `bar`
    - `col`
    - `line`
    - `pie`
    - `area`
    - `scatter`
    - `doughnut`
    - `radar`
    - `bubble`
    - `combo`
    - `stock`
    - `surface`
    - `ofPie`
    - `chartex`
- **`ofPieType`**
  - 可选
  - 只对 `type` 为 `ofPie` 有意义：`pie` 或 `bar`
  - 省略则写 `pie`
  - 其它 `type` 上出现报 `InvalidField`
- **`stockKind`**
  - 可选
  - 只对 `type` 为 `stock` 有意义
    - `hlc`：三条系列，`c:stockChart` 加 `c:hiLowLines`
    - `ohlc`：四条系列，再写 `c:upDownBars`（`gapWidth=150`、空的 `upBars` / `downBars`）
    - `vhlc`：四条系列，一条成交量柱加 HLC，inspect `type` 为 `combo`
    - `vohlc`：五条系列，一条成交量柱加 OHLC，inspect `type` 为 `combo`
  - 省略则：3 条当 `hlc`，4 条当 `ohlc`
  - 其它 `type` 上出现报 `InvalidField`
- **`surfaceKind`**
  - 可选
  - 只对 `type` 为 `surface` 有意义
    - `surface3d`：`c:surface3DChart`，`wireframe=0`，`view3D` 为 `rotX=15` `rotY=20` `rAngAx=0`
    - `wireframe`：同上包裹，`wireframe=1`
    - `contour`：`c:surfaceChart`，`wireframe=0`，`view3D` 为 `rotX=90` `rotY=0` `rAngAx=0` `perspective=0`
    - `contourWire`：俯视包裹，`wireframe=1`
  - 省略则写 `surface3d`
  - 其它 `type` 上出现报 `InvalidField`
- **`layoutId`**
  - `type` 为 `chartex` 时必须
  - 可写：`waterfall`、`treemap`、`sunburst`、`funnel`、`clusteredColumn`、`boxWhisker`、`pareto`
  - `regionMap` 报 `NotImplemented`
  - 其它值或不在 `chartex` 上出现报 `InvalidField`
- **`title`**
  - 可选
  - 图表标题
- **`series`**
  - 必须
  - 非空数组
  - 每项可含 `name`、`categories`、`values`、`sizes`、`type`
  - `values` 必填且非空
- **`kind`**
  - 可选
  - docx：`inline` 或 `anchor`，缺省 `anchor`
  - xlsx / pptx 上出现报 `InvalidField`
- **`wrap`**
  - 可选
  - docx 浮动时的绕排，缺省 `square`
  - 行内必须省略或为 `null`
  - xlsx / pptx 上出现报 `InvalidField`
- **`cx` / `cy`**
  - 可选
  - EMU，正整数
  - docx 都省略则用 `5486400` × `3200400`
  - xlsx 规则与 `insertPicture` 相同
  - pptx 都省略则用同一组缺省
  - 只给一个报 `InvalidField`
- **`x` / `y`**
  - pptx 可选
  - 画布坐标，EMU，非负整数
  - 必须成对
  - 缺则放到该页空白处，规则与 pptx `insertPicture` 相同
  - docx / xlsx 上出现报 `InvalidField`
- **`from`**
  - xlsx 必填
  - 图的左上角单元格，A1
  - docx / pptx 上出现报 `InvalidField`
- **`to`**
  - xlsx 且 `anchor` 为 `twoCell` 时必填
  - docx / pptx 上出现报 `InvalidField`
- **`anchor`**
  - xlsx 可选
  - `oneCell`（缺省）或 `twoCell`
  - 不认 `absolute`
  - docx / pptx 上出现报 `InvalidField`
- **`fromOffX` / `fromOffY` / `toOffX` / `toOffY`**
  - xlsx 可选
  - 与 `insertPicture` 相同
  - docx / pptx 上出现报 `InvalidField`
- **`dataFrom`**
  - xlsx 必填
  - 写到该表上的数据表左上角，A1
  - 第 1 行是系列名，第 1 列是分类，数值向右
  - docx / pptx 上出现报 `InvalidField`

docx：在该段末插入 `w:drawing`，默认浮动。新建 `/word/charts/chartN.xml`（`N` 从 1 起尚未占用）、chart 的 `.rels`、一份最小 embedding xlsx（工作表名 `Sheet1`，
分类从 `A2` 起，系列名在第 1 行，数值从 `B2` 起向右），以及宿主部件关系（主文档、页眉页脚、`comments.xml`、`footnotes.xml` 或 `endnotes.xml`）、`[Content_Types].xml` Override。
chart 部件仍写在 `/word/charts/`。chart 的 Content Type 为 `application/vnd.openxmlformats-officedocument.drawingml.chart+xml`。
关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart`。
embedding 关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/package`。
`wp:docPr/@id` 取该部件已有 `docPr` 的最大整数加 1，至少为 1。不强制写 chart colors / style 部件，也不强制套 `mc:AlternateContent`。
对图 id、图表 id、系列 id、容器 id、框 id、注容器 id、批注容器 id 或 `section:{n}` 发此 op 报 `InvalidId`。找不到段落报 `IdNotFound`。

xlsx：找不到该表报 `IdNotFound`。`id` 不是 `sheet:{name}` 报 `InvalidId`。该表不是 `worksheet` 报 `InvalidField`。
必须给 `from` 与 `dataFrom`，否则报 `MissingField`。`from` / `to` / `dataFrom` 不是合法 A1 报 `InvalidRange`。
`anchor` 规则与 `insertPicture` 相同。按 `setCell` 语义把数据表写到 `dataFrom` 起的格子（已有内容会被覆盖）。
新建 `/xl/charts/chartN.xml`、chart 的 `.rels`（不写 embedding），`c:f` 指向这些格子。
没有 drawing 则按插图同一套规则新建 drawing，并在锚点下写 `xdr:graphicFrame`（须带空的 `xdr:clientData`）。
`xdr:cNvPr/@id` 取该表已有全部 `cNvPr`（图与框）的最大整数加 1，至少为 1。对图 id、图表 id、系列 id 或单元格 id 发此 op 报 `InvalidId`。不要用本 op 新建图表工作表。

pptx：找不到该页 / 该母版 / 该版式报 `IdNotFound`。`id` 必须是 `slide:{n}`、`slide:{n}/notes`、`master:{n}` 或 `master:{n}/layout:{k}`。
对图 id、形状 id、图表 id、批注 id 或 `pptx:presentation` 发此 op 报 `InvalidId`。
写入该宿主 `p:spTree` 一个 `p:graphicFrame`，新建 `/ppt/charts/chartN.xml`、chart 的 `.rels`、
一份最小 embedding（`/ppt/embeddings/Microsoft_Excel_WorksheetN.xlsx`，规则与 docx 相同）。
图表关系写在该宿主 `.rels`（幻灯片或 notesSlide）。`p:cNvPr/@id` 取该宿主已有全部 `cNvPr` 的最大整数加 1，至少为 2。
`x` / `y` 成对规则与 `insertPicture` 相同；往备注插时画布用 `p:notesSz`。
`id` 为 `slide:{n}/notes` 且该页还没有 notesSlide 时，按 `setNotesText` 同一套规则新建最小备注页再插图。不写演示文稿里的工作表。

`type` 不认识或 `series` 为空报 `InvalidField`。`pie`、`doughnut` 与 `ofPie` 只允许恰好一条系列，否则报 `InvalidField`。
`surface` 必须恰好三条系列，否则报 `InvalidField`。
`stock` 必须恰好三条（`hlc`）、四条（`ohlc` / `vhlc`）或五条（`vohlc`），否则报 `InvalidField`。
`stock` 省略 `stockKind` 时：3 条写成 HLC，4 条写成 OHLC。
`vhlc` / `vohlc` 写成共享两条轴的 `c:barChart`（成交量）加 `c:stockChart`；inspect 的 `type` 为 `combo`，不要改成 `stock`。
`surface` 按 `surfaceKind` 写 `c:surface3DChart` 或 `c:surfaceChart`（三条系列、`c:serAx`、空的 `c:bandFmts`、对应 `c:view3D`）。
`ofPie` 写成 `c:ofPieChart`（`c:ofPieType`、`gapWidth=150`、`secondPieSize=75`、`c:serLines`，没有 `c:axId`）。
`chartex` 写成 `cx:chartSpace` 部件（路径 `chartExN.xml`，关系 Type 为 `http://schemas.microsoft.com/office/2014/relationships/chartEx`，
CT `application/vnd.ms-office.chartex+xml`）。宿主 `a:graphicData/@uri` 为 chartex 命名空间，子元是 `cx:chart`。
系列条数：`boxWhisker` 三条，`pareto` 两条，其余可写 layout 一条。xlsx 把数据写到 `dataFrom`，`cx:f` 用工作表 A1，不要抄 `_xlchart.v1.N`。
`bubble` 的 `sizes` 省略则写成与 `values` 等长的 `1`。
`type` 为 `combo` 时，每条系列必须带 `type`（`bar`、`col`、`line`、`area` 或 `scatter`），且至少出现两种不同包裹，否则报 `InvalidField`；
`combo` 的系列 `type` 不能是 `pie`、`doughnut`、`radar`、`bubble`、`stock`、`surface`、`ofPie`、`chartex`。
对非组合图的系列给 `type` 报 `InvalidField`。`chartex` 不带 `layoutId` 报 `InvalidField`。

### deleteChart

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteChart"` |
| `id` | 图表 id |

docx：删掉对应 `w:drawing`。若在 `mc:AlternateContent` 的 Choice 里，整段 `AlternateContent` 删掉。
所在 `w:r` 若删完后只剩 `w:rPr` 或空，把该 run 也删掉。重新解析关系之后，若该 chart 部件、专属 colors / style、embedding 再无人引用，删掉这些部件、关系条目和对应内容类型。

xlsx：按图表 id 删掉包含该 `xdr:graphicFrame` 的整个锚点。不删数据单元格。drawing 部件删空后的清理与 `deletePicture` 相同。chart 部件无人引用则删。
对图表工作表上那张（或仅剩一张）图表发此 op 报 `NotImplemented`，不写文件；要删整张表请用 `deleteChartSheet`。

找不到报 `IdNotFound`。id 不是图表 id 报 `InvalidId`。

### setChartSize

字段与 `setPictureSize` 相同，`id` 为图表 id。docx 改宿主 `wp:extent`（以及 `a:ext` 若有），规则与图相同。
xlsx 规则与图相同：`oneCell` / `absolute` 改 `xdr:ext`，`twoCell` 改右下角 `to`。pptx 写入 graphicFrame 的 `p:xfrm/a:ext`。

`setChartPos` 见 [pptx](json-contract-pptx.md#setchartpos)。`setChartWrap` 见 [docx](json-contract-docx.md#setchartwrap)。
`setChartAnchor` 见 [xlsx](json-contract-xlsx.md#setchartanchor)。

### SmartArt op 的共同限制

xlsx 认同一套 SmartArt op，但几何走工作表锚点：用 `setSmartArtAnchor`，
不用 `setSmartArtPos` / `setSmartArtWrap`（这两条在 xlsx 上报 `InvalidField`）。
pptx 认同一套 SmartArt op。改点只动 data 部件里对应 `dgm:pt` 的 `dgm:t`，以及必要的 `cxnLst`。不自己算版式坐标、连接线或形状几何。
包里若有 diagram drawing 缓存（常见 `/word/diagrams/drawingN.xml` 或 `/xl/diagrams/drawingN.xml`，
关系 Type 含 `diagramDrawing`）：改字时同步改缓存里对应形状的可见 `a:t`；
对不上 modelId 则删掉该缓存部件及其关系，让 Office 打开时重排。插入不写这份缓存。

库内模板写在 `crates/ooxp` 资源里，不从 Office 安装目录抄。三份 layout 短名与 uniqueId：

| 短名 | uniqueId |
| --- | --- |
| `list` | `urn:microsoft.com/office/officeart/2005/8/layout/default` |
| `org` | `urn:microsoft.com/office/officeart/2005/8/layout/orgChart1` |
| `cycle` | `urn:microsoft.com/office/officeart/2005/8/layout/cycle2` |

家族按 uniqueId 最后一个 `/` 之后的末段判定（大小写不敏感）：`default`，或以 `hList` / `vList` 开头，属 `list`；`orgChart1`，或以 `orgChart` 开头，属 `org`；
`cycle2`，或以 `cycle` 开头，属 `cycle`。其余 uniqueId 不属于这三族。

### setSmartArtText

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setSmartArtText"` |
| `id` | 点 id，例如 `para:3F2A1B00/smartdiagram:4/pt:2` |
| `text` | 新的可见纯文本，字符串，可空。`\n` 写成 `a:br` |

只改该点 `dgm:t` 的可见文字。留下第一个含 `a:t` 的 run 收下全部新文本，规则对齐 `setShapeText` 的可见替换（不整段重建 `dgm:t` 的 `a:bodyPr`）。
没有 `dgm:t` 则补一份最小文本体。找不到该点报 `IdNotFound`。id 不是点 id 报 `InvalidId`。
对 `doc` / `pres` / `parTrans` / `sibTrans` 点发此 op 报 `InvalidId`（它们不在树上）。

### addSmartArtPoint

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"addSmartArtPoint"` |
| `id` | 图示 id（挂到文档根点下）或点 id（挂到该点下） |
| `text` | 可选。省略当作 `""` |
| `role` | 可选。`node` 或 `asst`，缺省 `node` |

在 data 部件追加一个可见 `dgm:pt`，并追加一条 `type` 为 `parOf`（或省略 type）的 `cxn`：`srcId` 为父点 `modelId`（图示 id 时父为 `type="doc"` 的点），`destId` 为新点。
不手写 `pres` / `parTrans` / `sibTrans`。现有 `modelId` 都能解析成非负整数时，新点与新连接用尚未占用的最小正整数；
否则新点写新的大写 GUID（带花括号），连接 `modelId` 同样不与已有 pt / cxn 冲突。
`role` 为 `asst` 且当前 layout 属 `list` / `cycle` 家族时报 `InvalidField`。找不到父点或图示报 `IdNotFound`。id 不是图示 id 或点 id 报 `InvalidId`。
缺少 data 部件报 `OpcPartMissing`。

### deleteSmartArtPoint

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteSmartArtPoint"` |
| `id` | 点 id |

删掉该点，并沿 `parOf` 边删掉所有子孙可见点。同时删掉 `srcId` / `destId` 指向这些 `modelId` 的连接，以及 `cxnId` 指向已删连接的 `parTrans` / `sibTrans` 点。
不删 `type="doc"` 的点。删完后可见点一条都不剩时报 `InvalidField`（要整张去掉请用 `deleteSmartArt`）。找不到报 `IdNotFound`。id 不是点 id 报 `InvalidId`。

### insertSmartArt

- **`op`**
  - 必须是 `"insertSmartArt"`
- **`id`**
  - docx：宿主段落 id，正文、页眉页脚、框内、注内或批注内段落
  - xlsx：工作表 id `sheet:{name}`
  - pptx：页 id `slide:{n}`、备注容器 id `slide:{n}/notes`、母版 id `master:{n}` 或版式 id `master:{n}/layout:{k}`
- **`layout`**
  - `list`、`org` 或 `cycle`
- **`points`**
  - 可选
  - 可见点数组
  - 每项可含 `text`、`parent`、`role`
  - `parent` 是本数组里从 0 起的下标，表示父点
  - 省略或 `null` 表示挂到文档根点下
- **`kind`**
  - docx 可选
  - `inline` 或 `anchor`，缺省 `anchor`
  - xlsx / pptx 上出现报 `InvalidField`
- **`wrap`**
  - docx 可选
  - 浮动时的绕排，缺省 `square`
  - 行内必须省略或为 `null`
  - xlsx / pptx 上出现报 `InvalidField`
- **`x` / `y`**
  - pptx 可选
  - 画布坐标，EMU，非负整数
  - 必须成对
  - docx / xlsx 上出现报 `InvalidField`
- **`from` / `to` / `anchor` / 偏移**
  - xlsx 可选（`from` 必填）
  - 规则与 `insertChart` 相同
  - docx / pptx 上出现报 `InvalidField`
- **`cx` / `cy`**
  - 可选
  - EMU，正整数
  - 都省略则用 `5486400` × `3200400`
  - 只给一个报 `InvalidField`

docx：在该段末插入 `w:drawing`，默认浮动。新建 `/word/diagrams/` 下尚未占用的 data / layout / colors / quickStyle 四件套（序号从 1 起），
并把四条关系写在**宿主** `.rels` 上（主文档、页眉页脚、`comments.xml`、`footnotes.xml` 或 `endnotes.xml`）。
diagram 部件仍写在 `/word/diagrams/`。`wp:docPr/@id` 取该部件已有 `docPr` 的最大整数加 1，至少为 1。
对图 id、图表 id、SmartArt id、点 id、容器 id、框 id、注容器 id、批注容器 id、`section:{n}` 或 `slide:{n}` 发此 op 报 `InvalidId`。找不到段落报 `IdNotFound`。

xlsx：在该表 worksheet drawing 插入 `xdr:graphicFrame`。新建 `/xl/diagrams/` 下同样四件套，四条关系写在 **drawing** `.rels`。
`xdr:cNvPr/@id` 取该表已有全部 `cNvPr` 的最大整数加 1。`from` 必填，锚点对齐 `insertChart`。缺省尺寸用 `5486400` × `3200400`。
`kind` / `wrap` / `x` / `y` 出现报 `InvalidField`。
对图 id、图表 id、图示 id、点 id、文本框 id、单元格 id 或 `xlsx:workbook` 发此 op 报 `InvalidId`。找不到工作表报 `IdNotFound`。
对 `chartsheet` 报 `InvalidField`。对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。

pptx：在该宿主 `p:spTree` 插入 `p:graphicFrame`。新建 `/ppt/diagrams/` 下同样四件套，四条关系写在该宿主 `.rels`（幻灯片或 notesSlide）。
`p:cNvPr/@id` 取该宿主已有全部 `cNvPr` 的最大整数加 1，至少为 2。缺 `x` / `y` 则放到该宿主空白处；往备注插时画布用 `p:notesSz`。
`id` 为 `slide:{n}/notes` 且该页还没有 notesSlide 时，按 `setNotesText` 同一套规则新建最小备注页再插入。
对图 id、图表 id、图示 id、点 id、形状 id、批注 id 或 `pptx:presentation` 发此 op 报 `InvalidId`。找不到页报 `IdNotFound`。

`dgm:relIds` 的 `r:dm` / `r:lo` / `r:qs` / `r:cs` 指向这四条。不强制套 `mc:AlternateContent`，不写 Fallback 位图，不写 diagram drawing 缓存。

省略 `points` 时：`list` / `cycle` 写三个可见点，文本为 `文本 1`、`文本 2`、`文本 3`，都挂在文档根点下；`org` 写一个根点 `文本 1`，再写两个子点 `文本 2`、`文本 3`。
给了 `points` 则按数组顺序写；`parent` 越界、指向自身或成环报 `InvalidField`。`layout` 不认识报 `InvalidField`。
`list` / `cycle` 上出现 `role` 为 `asst`，或 `parent` 指向另一个可见点（深度大于 1）报 `InvalidField`。

Content Type：data 为 `application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml`，
layout 为 `…diagramLayout+xml`，colors 为 `…diagramColors+xml`，quickStyle 为 `…diagramQuickStyle+xml`。
关系 Type 分别为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramData`、`…/diagramLayout`、
`…/diagramColors`、`…/diagramQuickStyle`。

### deleteSmartArt

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteSmartArt"` |
| `id` | 图示 id |

删掉对应宿主 drawing。docx 删 `w:drawing`；若在 `mc:AlternateContent` 的 Choice 里，整段 `AlternateContent` 删掉（Fallback 位图一起走）。
所在 `w:r` 若删完后只剩 `w:rPr` 或空，把该 run 也删掉。xlsx 删对应 `xdr:graphicFrame`；若在组合里只摘该子节点，空组合再摘组合；锚点只剩 from/to/clientData 再摘整锚点。
pptx 删 `p:graphicFrame`；若在 Choice 里同样整段 `AlternateContent` 删掉。
重新解析关系之后，若该图示的 data / layout / colors / quickStyle / drawing 缓存再无人引用，删掉这些部件、关系条目和对应内容类型。Fallback 用过的媒体部件若再无人引用也删。
找不到报 `IdNotFound`。id 不是图示 id 报 `InvalidId`。

### setSmartArtSize

字段与 `setPictureSize` 相同，`id` 为图示 id。docx 改宿主 `wp:extent`（以及 `a:ext` 若有），规则与图相同。
xlsx 规则与图相同：`oneCell` / `absolute` 改 `xdr:ext`，`twoCell` 改右下角 `to`。pptx 改 graphicFrame 的 `p:xfrm/a:ext`。

`setSmartArtAnchor` 见 [xlsx](json-contract-xlsx.md#setsmartartanchor)。
`setSmartArtPos` 见 [pptx](json-contract-pptx.md#setsmartartpos)。
`setSmartArtWrap` 见 [docx](json-contract-docx.md#setsmartartwrap)。

### setSmartArtColors

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setSmartArtColors"` |
| `id` | 图示 id |
| `colors` | `accent1`、`accent2`、`accent3`、`accent4`、`accent5` 或 `accent6` |

换成库内对应色板模板。包里已有相同 uniqueId 的 colors 部件则只改 `r:cs` 指向它；否则写入新部件并改 `r:cs`。不认识的取值报 `InvalidField`。找不到图示报 `IdNotFound`。id 不是图示 id 报 `InvalidId`。

### setSmartArtStyle

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setSmartArtStyle"` |
| `id` | 图示 id |
| `style` | `simple`、`subtle` 或 `intense` |

换成库内对应 quickStyle 模板。包里已有相同 uniqueId 的 style 部件则只改 `r:qs`；否则写入新部件并改 `r:qs`。不认识的取值报 `InvalidField`。找不到图示报 `IdNotFound`。id 不是图示 id 报 `InvalidId`。

### setSmartArtLayout

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setSmartArtLayout"` |
| `id` | 图示 id |
| `layout` | `list`、`org` 或 `cycle` |

不认识的短名报 `InvalidField`。当前 uniqueId 已经是目标那一条：空操作，仍计入 `opsApplied`。
当前 uniqueId 属于库内 `list` / `org` / `cycle` 三族之一：把 layout 部件换成库内对应模板，并改 `r:lo`；可见点与 `parOf` 连接不动。
目标是 `list` 或 `cycle` 时，助手点改成普通节点，以免文字在新版式里消失。删掉 diagram drawing 缓存，让 Office 打开时重排。
当前 uniqueId 不在这三族里报 `NotImplemented`，不写文件。不要写出 Word / PowerPoint 要修复的包。找不到图示报 `IdNotFound`。id 不是图示 id 报 `InvalidId`。

### setShapeText

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "output": "C:\\data\\deck.patched.pptx",
  "ops": [
    {
      "op": "setShapeText",
      "id": "slide:1/shape:2",
      "text": "新标题"
    }
  ]
}
```

- **`op`**
  - 必须是 `"setShapeText"`
- **`id`**
  - pptx：
    - `slide:{n}/shape:{shapeId}`
    - `slide:{n}/shape:index:{k}`
    - `slide:{n}/notes/shape:…`
    - `master:{n}/shape:…`
    - `master:{n}/layout:{k}/shape:…`
    - `notesMaster/shape:…`
    - `handout/shape:…`
  - xlsx：文本框 id（`sheet:{name}/sp:{cNvPrId}` 或 `sheet:{name}/sp:index:{k}`）或形状 id（`sheet:{name}/shape:{cNvPrId}` 或 `sheet:{name}/shape:index:{k}`）
- **`text`**
  - 新的可见纯文本，字符串，可空
  - 必填
  - `\\n` 写成 `a:br`，不把字面换行两个字符写进 `a:t`

找不到该形状报 `IdNotFound`。对 `slide:{n}` 或 `slide:{n}/notes`（没有 `/shape:`）发本 op 报 `InvalidId`。
对备注页、备注母版或讲义上占位符 `p:ph/@type` 为 `sldImg` 的形状发本 op 报 `InvalidField`，不写文件。
xlsx 改工作表文本框或无 `txBox` 形状的 `xdr:txBody`，规则与 pptx 相同（把下文的 `p:sp` / `p:txBody` 换成 `xdr:sp` / `xdr:txBody`）。
对工作表 id、图 id、图表 id 或连接线 id 发本 op 报 `InvalidId`。没有 `txBody` 的形状仍报 `IdNotFound`（不补 `txBody`）。docx 上发此 op 报 `InvalidField`。
插删形状请用 `insertShape` / `deleteShape`，不要发 Word / Excel 的 `insertTextBox` / `deleteTextBox`（pptx 上这两条仍是外格式，报 `InvalidField`）。
母版 / 版式占位符与装饰形只改对应 slideMaster / slideLayout 部件，不改各页 XML。没有 `txBody` 的占位符仍报 `IdNotFound`（不补 `txBody`）。
备注母版占位符只改 notesMaster。讲义占位符只改 handoutMaster。对 `handout` 容器发本 op 仍报 `InvalidId`。
没有 `txBody` 的装饰形同样报 `IdNotFound`（不补 `txBody`）。
pptx 含形状级链时：只改 `p:txBody`，不改 `p:cNvPr` 上的 `a:hlinkClick` / `a:hlinkHover`，形状级链必须还在。
第一个收下新文本的 run 若其 `a:rPr` 带 run 级 `a:hlinkClick`，会随第 5 条抄到新 run 上；其余 run 级链随清多余 run 丢掉。
run 级链编进 `hyperlinks`，独立节点用 `insertHyperlink`（`on` 为 `run`）/ `setHyperlink` / `deleteHyperlink`。

替换规则（手术刀，不整页重建 `p:sp`，更不整份重建演示文稿）：

1. 只改命中的那一个 `p:sp` 的 `p:txBody` 可见文字。`p:nvSpPr`、`p:spPr`（几何与位置）、`p:style`、不认识的子元素都留着。禁止删掉该形状再新建一个。
2. 不改同页的 `<p:timing>`、`<p:transition>`，也不改其他形状。重序列化该页时这两节必须作为未点名节点原样保留。
3. 在该 `p:txBody` 里找出含 `a:t` 的 run（`a:r`）。遇到 `mc:AlternateContent` 只进第一个 `mc:Choice`（没有就进 `mc:Fallback`）。
4. 若一个都没有：在现有 `a:bodyPr` / `a:lstStyle` 之后插入 `a:p`/`a:r`/`a:t`，文本放进去；已经有段落时挂在第一段末尾，不凭空多出一段。
   没有 `p:txBody` 则本 op 报 `IdNotFound`（不给无文本框的形状补 `txBody`）。无 `txBody` 的 `p:sp` 不进 inspect 树；用形状 id 找不到就是 `IdNotFound`。
5. 若有：第一个这样的 run 收下全部新文本。第一行写进它的第一个 `a:t`（需要保留首尾空格时加 `xml:space="preserve"`），该 run 里多余的 `a:t` 删掉。
   `text` 里每遇一个 `\n`，就在**段落**里紧接着插一个 `a:br` 加一个新的 `a:r`，新 run 沿用第一个 run 的 `a:rPr`。
6. 换行只能拆在段落这一级：`a:br` 与 `a:r` 平级、同为 `a:p` 的子元素（`EG_TextRun`，§A.4），而 `CT_RegularTextRun` 只允许一个可选 `a:rPr` 加**恰好一个** `a:t`。
   所以 `a:br` 不许塞进 `a:r` 里，一个 `a:r` 也不许带多个 `a:t`，两者都会生成不合 schema 的 run。
7. 该 `p:txBody` 里其余的可见内容一律清掉：别的 `a:t` 删掉，**所有 `a:br` 删掉**，因此变空、且里面再也没有别的 run 内容的 `a:r` 删掉。
   `a:br` 必须一起清，因为 inspect 把它读成 `\n`，留一个下来读回值就比写入值多一个换行，与本节最后一条冲突。收下新文本的那一段原地保留，它自己的 `a:pPr`、`a:endParaRPr` 不许删，也不许换成别段的。
   清完之后，段落里若只剩下 `a:pPr`、`a:endParaRPr`（没有 `a:r`、`a:br`、`a:fld` 以及其它子元素），把这段 `a:p` 删掉。
   `CT_TextBody` 的 `a:p` 至少留一段（§A.4，`minOccurs="1"`）。不许把整棵 `txBody` 的段落删光再重建，也不许把别段的 `a:pPr` 搬到写入段上。
   空段要删，是因为 §21.1.2.2.6 把 `a:p` 定为文本框里最高一层分隔，PowerPoint 会把空段画成空行；inspect 的 `text` 又不在段落之间插入 `\n`。含 `a:fld` 的段按第 8 条不清域，因此也不会被本条删掉。
8. `a:fld` 是应用程序定期更新的生成文本（§21.1.2.2.4），不是用户打的字。本 op 不改、不删未点名的域节点。inspect 的形状 `text` 也不收域里的 `a:t`。
   写进去的用户文本因此能原样读回；屏幕上域的缓存字（页码、日期）仍可能跟在旁边。独立节点用 `insertField` / `setField` / `deleteField`，
   见 [json-contract-pptx.md](json-contract-pptx.md#insertfield)。
9. apply 写进去的文本，随后 inspect 读回来必须与写入值相同。第 9 条只对用户 run 文本成立；域缓存不在此列。

### setShapeFormat

改 DrawingML 文本框的字号、字色、字填充、字轮廓、加粗、斜体、下划线、中英文字体、水平对齐与框内垂直锚点。不改可见字。不要用 Word 的半磅字号。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "setShapeFormat",
      "id": "slide:1/shape:2",
      "sz": 3600,
      "color": "C41E3A",
      "bold": true,
      "align": "ctr"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setShapeFormat"` |
| `id` | pptx 形状 id（与 `setShapeText` 相同）、pptx 表单元格 id，或 xlsx 文本框 / 形状 id |
| `sz` | 可选。百分之一磅，100–400000。`null` 去掉显式字号 |
| `color` | 可选。六位 RGB，或 `null` 去掉显式填色。写入大写。不写 theme 色。不要和 `textFill` 一起给 |
| `textFill` | 可选。字填充。`null` 去掉 `a:rPr` 上的填充子元。对象见下文 |
| `textOutline` | 可选。字轮廓。`null` 去掉 `a:rPr/a:ln`。对象见下文 |
| `bold` / `italic` | 可选。布尔。`false` 写成 `b="0"` / `i="0"` |
| `underline` | 可选。`sng` / `none` / `dbl`，或 `null` 去掉属性 |
| `font` | 可选。西文 `a:latin/@typeface`。空串报错 |
| `eastAsiaFont` | 可选。中文 `a:ea/@typeface`。空串报错 |
| `align` | 可选。`l` / `ctr` / `r` / `just`，或 `null` 去掉属性 |
| `textAnchor` | 可选。`t` / `ctr` / `b`，或 `null` 去掉 `bodyPr/@anchor`。不能叫 `anchor` |

除 `op` 与 `id` 外至少给一个字段，否则报 `InvalidField`。
`sz` 小于 100 报 `InvalidField`；大于 400000 报 `TruncatedLimit`。
`color` 与 `textFill` 同时出现报 `InvalidField`。
`textFill` 为对象时必须有 `type`：`none` 或 `gradient`。不要写 `solid`（实心仍用 `color`）。
`type` 为 `gradient` 时必须有 `stops`，至少两项。每项必须有 `pos`（0–100000）与六位 `color`。
`pos` 重复报 `InvalidField`。可选 `ang`（0–21600000），省略则写 `5400000`（Office 16.0 水平渐变）。
写入 `a:gradFill`：`flip="none"` `rotWithShape="1"`，`a:lin scaled="1"`，空 `a:tileRect`。不写 `a:path`。不写 theme 色。
`textOutline` 为对象时必须有 `color`（六位 RGB）与 `width`（正整数 EMU）。
写入 `a:ln/@w` 与 `a:ln/a:solidFill/a:srgbClr`，插在填充子元之前。不写 `prstDash` / `miter`。不写 `schemeClr`。
不写 `a:prstTxWarp`，不写 `a:effectLst`。功能区艺术字不新开对象。
改该 `txBody` 里每个文本 `a:r` 的 `a:rPr`，并同步 `a:endParaRPr`。不改 `a:fld`。不重写 `a:lstStyle`。
各 run 不一致时 inspect 对应字段为 `null`。
inspect 在 `color` 之后始终带 `textFill`、`textOutline`；在 `textAnchor` 之后、`stable` 之前始终带 `runs`。
`textFill`：各 run 都是 `a:noFill` 则为 `{ "type": "none" }`；都是可读的线性 `a:gradFill`（仅 `srgbClr`）则报该对象；否则为 `null`。
`textOutline`：各 run 都有同一套 `a:ln/@w` 与 `srgbClr` 则报 `{ "color", "width" }`；否则为 `null`。
`runs` 按文档顺序列出每个文本 `a:r`。每条字段顺序：`text`、`sz`、`color`、`textFill`、`textOutline`、
`bold`、`italic`、`underline`、`font`、`eastAsiaFont`。
`text` 取该 run 的 `a:t`。不进 `a:fld`。不算排版引擎。
没有 `txBody` 报 `IdNotFound`。
对连接线 id 报 `InvalidId`。对 `sldImg` 占位符报 `InvalidField`。docx 上发此 op 报 `InvalidField`。
create 的 pptx `shapes[]`、xlsx 的 `textBoxes[]` / `shapes[]`，以及 `insertShape`、xlsx `insertTextBox`，认同一套可选字段。
pptx create 缺省仍是 18 磅黑字 Calibri / 宋体。xlsx 缺省仍是 `sz="1100"`。
xlsx 的 `anchor` 仍是单元格锚点，框内垂直对齐只用 `textAnchor`。
示例见 `docs/examples/pptx-apply-shape-format.json`、`docs/examples/xlsx-apply-shape-format.json`、
`docs/examples/pptx-apply-text-fill.json` 与 `docs/examples/xlsx-apply-text-fill.json`。

## exportMedia

按图 id 把 OPC 里的媒体部件原样写到本地文件。docx 与 pptx 也认 OLE id，写出嵌入部件原字节。docx 也认 `font:{n}`，写出 fontTable 指向的 odttf 原字节（仍是混淆后的，不还原）。
三种格式也认 3D 模型 id，写出 GLB 原字节。docx 也认墨迹画布 id。
不读进 inspect 树，不做转码、缩放、子集化或预览。

```json
{
  "schemaVersion": 1,
  "action": "exportMedia",
  "path": "C:\\data\\note.docx",
  "id": "para:3F2A1B00/pic:1",
  "output": "C:\\out\\logo.png"
}
```

- **`path`**（是）
  - 源 docx、xlsx 或 pptx
- **`id`**（是）
  - 图 id，与 inspect 的 `pictures[].id` 相同
  - docx / xlsx / pptx 也可以是 OLE id，与 `oleObjects[].id` 相同
  - docx / xlsx / pptx 也可以是 3D 模型 id，与 `model3ds[].id` 相同
  - docx 也可以是墨迹画布 id，与 `inkCanvases[].id` 相同
  - pptx 也可以是视频 / 音频 id，与 `videos[].id` / `audios[].id` 相同
  - docx 也可以是嵌入字体 id，与 `embeddedFonts[].id` 相同
- **`output`**（否）
  - 本地目标路径
  - 省略时用命令行 `--output`
  - 两者都缺报 `MissingField`
  - 不得默认写成源 `path`，以免把图片字节盖掉文档
- **`format`**（否）
  - 可省略，从 `path` 扩展名推断

`id` 必须是图 id，或 docx / xlsx / pptx 的 OLE id，或三种格式的 3D 模型 id，或 docx 的墨迹画布 id，或 pptx 的视频 / 音频 id，或 docx 的 `font:{n}`。
对段落 id、工作表 id、单元格 id、页眉页脚容器 id、`section:{n}`、
形状 id 或 `slide:{n}`（没有 `/pic:` / `/video:` / `/audio:` / `/ole:` / `/ink:` / `/model3d:`）报 `InvalidId`。
xlsx / pptx 上发 `font:{n}` 仍按该格式原规则报 `InvalidId`。找不到该图、该嵌入对象、该 3D 模型、该墨迹画布、该媒体或该嵌入字体报 `IdNotFound`。
图 id 优先走 `a:blip/@r:embed`；只有 `@r:link`、没有 embed 时按外部关系处理，报 `OpcPartMissing`，不下载 http。embed 指向外部关系同样报 `OpcPartMissing`。
三种格式的图若带 `asvg:svgBlip/@r:embed`，`exportMedia` 改走这条关系，写出 SVG 原字节。
没有 `svgBlip` 时仍走 `a:blip`。
PowerPoint 另存的 SVG 可以没有 `a:blip/@r:embed`，此时只走 `svgBlip`。
pptx 视频 / 音频 id 优先走 `p14:media/@r:embed`，没有则走 `a:videoFile` / `a:audioFile` / `a:wavAudioFile` 的关系；外链同样报 `OpcPartMissing`。
OLE id 走 oleObject 或 package 关系目标；关系缺失或指向外部同样报 `OpcPartMissing`。

写出的是该媒体部件或嵌入部件的原字节。`contentType` 取自 `[Content_Types].xml` 对该部件的 Default 或 Override；都没有则省略该字段。扩展名由调用方自己定，库不按类型改名。

xlsx 用工作表图 id 或页眉图 id，写出 `/xl/media/…`。xlsx 的 OLE id 写出 `/xl/embeddings/…`，不是预览图。
xlsx 的 3D 模型 id 写出 `/xl/media/…` 的 GLB 原字节，不是海报 PNG。
pptx 用幻灯片图 id、备注图 id、批注内图 id、视频 id 或音频 id，写出 `/ppt/media/…`。视频 / 音频导出的是媒体部件（`mediaN.mp4` / `mediaN.wav`），不是海报 PNG。
pptx 的 OLE id 写出 `/ppt/embeddings/…`，不是预览图。pptx 的 3D 模型 id 写出 `/ppt/media/…` 的 GLB 原字节，不是海报。
docx 文本框里的图用框内段落上的图 id，同一动作能导出。脚注、尾注里的图用注内段落上的图 id，同一动作能导出。
docx 的 OLE id 写出 `/word/embeddings/…`，不是预览图。docx 的墨迹画布 id 写出 `/word/ink/…` 的 InkML 原字节。
docx 的 3D 模型 id 写出 `/word/media/…` 的 GLB 原字节。嵌入图表的工作簿另走 `exportChartData`，不在本动作。

成功时 stdout 见 [errors.md](errors.md)。源包不写回。

## exportChartData

按图表 id 把 embedding 工作簿原字节写到本地。不读进 inspect 树，不转码。

```json
{
  "schemaVersion": 1,
  "action": "exportChartData",
  "path": "C:\\data\\note.docx",
  "id": "para:3F2A1B00/chart:7",
  "output": "C:\\out\\chart.xlsx"
}
```

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `path` | 是 | 源 docx 或 pptx |
| `id` | 是 | 图表 id，与 inspect 的 `charts[].id` 相同 |
| `output` | 否 | 本地目标路径。省略时用命令行 `--output`。两者都缺报 `MissingField`。不得默认写成源 `path` |
| `format` | 否 | 可省略，从 `path` 扩展名推断 |

`id` 必须是图表 id，含正文、页眉页脚、框内、注内与批注内带前缀的 id，以及 pptx 备注图表 id `slide:{n}/notes/chart:…`。对系列 id、段落 id、形状 id、图 id 报 `InvalidId`。
找不到该图报表 `IdNotFound`。没有 embedding 报 `OpcPartMissing`。关系指向外部资源同样报 `OpcPartMissing`，不下载 http。xlsx 本期报 `InvalidField`。

写出的是 embedding 部件的原字节。`contentType` 取自 `[Content_Types].xml`；没有则省略。

成功时 stdout 见 [errors.md](errors.md)。源包不写回。

## 两层 JSON

领域 JSON 就是本文，本仓库只认这一层。外部 MCP 可以把本文整个对象放进自己的信封（例如 RPC 的 `params`）。本仓库不实现、不解析那一层信封。
不许把 JSON-RPC 的 `jsonrpc`、`method`、`id` 塞进领域对象，也不许让单元格或段落字段去迁就 RPC。
