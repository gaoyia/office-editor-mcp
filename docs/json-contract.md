# 领域 JSON 契约

`schemaVersion` 可省略，省略视为 `1`。写了就必须是 `1`。库和命令行只认这一层。`ooxp mcp` 和外部 MCP 也必须整份使用这一层，不许改字段名和含义，不许拆扁成工具参数。

xlsx / docx / pptx 的稳定 id、inspect、create 与专用 apply 分别见 [json-contract-xlsx.md](json-contract-xlsx.md)、
[json-contract-docx.md](json-contract-docx.md)、[json-contract-pptx.md](json-contract-pptx.md)。

机器可读副本是 [json-schema/request.schema.json](json-schema/request.schema.json)。
这份文件由 [json-schema/src/](json-schema/src/) 里的分块拼成，不要手改。
改分块后运行 `python docs/json-schema/build.py`。
与 Markdown 冲突时以本文及分册为准。

xlsx、docx、pptx 的 inspect、create、apply 已经按本文实现。docx 另有 `exportMedia` 与 `exportChartData`。
xlsx 另有 `exportChartImage`：按图表 id 把簇状柱形图、簇状条形图、普通折线图、带标记的散点图、普通面积图或普通饼图画成 PNG。像素不进 inspect。
xlsx 另有 `exportSparklineImage`：按迷你图组 id 把一条折线、非负柱形或盈亏画成 PNG；合法日期轴按日期比例定位。像素不进 inspect。
xlsx 另有 `exportSheetImage`：按工作表 id 把已用区域画成 PNG。像素不进 inspect。画不了的对象让整表失败。
`exportChartImage`、`exportSparklineImage`、`exportSheetImage`、`exportSlideImage` 与 `exportPageImage` 可写可选整数字段 `dpi`（24 到 600）。省略或 `96` 时与现有 96 dpi 出图像素一致。按该 DPI 原生计算坐标、字体、描边和间距，不是先画 96 再拉伸位图。只影响本次 PNG，不改源包，也不改系统 DPI。非默认 DPI 会写入 pHYs（像素/米）；省略或 96 不写该块，以免改既有字节。宽或高超出 1 到 4096 像素仍报 NotImplemented。Word 与 PowerPoint 按请求 DPI 一次换算（twip×dpi/1440 四舍五入、EMU×dpi/914400、磅×dpi/72）；Word 嵌入图 extent 仍按 EMU 截断，与历史 96 dpi 截断一致。不要把 Excel 先按 96 量化再放大的行高等特例套到 Word / PowerPoint。这是每次出图请求上的参数，不是包级或进程全局开关。

示例（库默认仍是 96；对照官方 106 dpi 窗口时显式传入）：

```json
{"schemaVersion":1,"action":"exportSparklineImage","path":"book.xlsx","id":"sheet:Sheet1/spark:1","output":"spark.png","dpi":106}
```

```json
{"schemaVersion":1,"action":"exportChartImage","path":"book.xlsx","id":"sheet:Sheet1/chart:1","output":"chart.png","dpi":106}
```

```json
{"schemaVersion":1,"action":"exportSheetImage","path":"book.xlsx","id":"sheet:Sheet1","output":"sheet.png","dpi":106}
```

```json
{"schemaVersion":1,"action":"exportSlideImage","path":"deck.pptx","id":"slide:1","output":"slide.png","dpi":192}
```

```json
{"schemaVersion":1,"action":"exportPageImage","path":"note.docx","id":"page:1","output":"page.png","dpi":192}
```

命令行用 `--input` 读取同一份领域 JSON（文件路径，或 `-` 表示标准输入）。不要把 JSON 路径当作位置参数，命令行会拒绝并退出码 2。

```text
ooxp --input spark-106.json
```

`spark-106.json` 的内容即上面第一份 `exportSparklineImage` 请求。也可以：

```text
ooxp --input - < spark-106.json
```
pptx 另有 `exportSlideImage`：按幻灯片 id 把页底（没有颜色变换的纯色、不透明拉伸铺满 PNG，以及两停点、直角、不透明线性渐变）、PNG、矩形、带 `a:alpha` 的半透明纯色矩形、线宽恰好是 1 或 2 像素的直角矩形描边、圆角矩形、椭圆、两色直线渐变、水平或垂直的直线、直角折线连接符、弯曲连接符、形状文字、簇状柱形图和纯色单元格的表写成图。像素不进 inspect。画不了的对象让整页失败。
两色直线渐变画在没有旋转的矩形、圆角矩形和椭圆上，两档可以带 `alpha`，半透明会叠在已画像素上。形状上的 `blipFill` 会按拉伸铺进这些形状；画不了的变体让整页失败，不再静默跳过。
create 把文本里的换行写成 `a:br` 时，预览器也会画出后续行。
docx 另有 `exportPageImage`：按页 id 把没有字的一页，或 Windows 上单节拉丁或东亚文字多段（可空段、段前段后、左缩进、按空格换行），或单节不透明 `wp:inline` PNG（按 extent 缩放；extent 像素仍按 EMU×dpi/914400 截断），或表（单元格填色、`single`/`nil` 边、`tblGrid`/`tblInd`/`trHeight`、默认单元格边距、表内拉丁或东亚文字），或 default 页眉/页脚拉丁或东亚文字（`pgMar/@header` 顶距、`@footer` 底距到内容底，正文原点不位移），或显式行距（`w:spacing/@w:line|@w:lineRule` auto/exact/atLeast）与分页控制（pageBreakBefore/keepNext/keepLines/widowControl），写成 PNG。像素不进 inspect。右/首行缩进、浮动绕排、裁剪、透明、文图混排、合并/表样式、奇偶/首页页眉页脚、页眉页脚内图/表/域还不画（NotImplemented，不静默丢段）。字形灰度按 C25 第 0 段暂缓，不把灰阶差算成整页通过。
docx 的 `exportMedia` 认图 id、OLE id、墨迹画布 id、3D 模型 id 与嵌入字体 id：图 id 写出 `/word/media/…`，OLE id 写出 embeddings 部件，
墨迹画布 id 写出 `/word/ink/…` 的 InkML 原字节，
3D 模型 id 写出 `/word/media/…` 的 GLB 原字节，
`font:{n}` 写出 `/word/fonts/…` 的 odttf 原字节（仍是混淆后的，不还原）。
xlsx 的 `exportMedia` 认工作表图 id、页眉页脚图 id、工作表背景图 id、OLE id、3D 模型 id 与工作表墨迹 id：图 id 与背景图 id 写出 `/xl/media/…`，OLE id 写出 embeddings 部件，
3D 模型 id 写出 `/xl/media/…` 的 GLB 原字节，墨迹 id 写出 `/xl/ink/…` 的 InkML 原字节。
pptx 的 `exportMedia` 认幻灯片图 id、备注图 id、批注内图 id、母版 / 版式图 id、视频 id、音频 id、OLE id、3D 模型 id 与幻灯片墨迹 id：图 / 视频 / 音频写出 `/ppt/media/…`，OLE 写出 embeddings 部件，
3D 模型 id 写出 `/ppt/media/…` 的 GLB 原字节，墨迹 id 写出 `/ppt/ink/…` 的 InkML 原字节。
xlsx 的 `scope.charts` 与图表 op 认工作表 / 图表工作表上的图；`scope.headerFooter` 展开工作表上的页眉页脚槽位。
pptx 的 `scope.charts` 与图表 op 认幻灯片、备注页与幻灯片母版 / 版式上的图表 `p:graphicFrame`；`exportChartData` 认这些图表 id，写出 embedding。
pptx 的 `scope.smartArt` 与 SmartArt op 认幻灯片、备注页与幻灯片母版 / 版式上的 diagram `p:graphicFrame`。
pptx 的 `scope.tables` 与表格 op 认 DrawingML 表 `p:graphicFrame`（`a:tbl`，ECMA-376 §21.1.3.13），不认 `w:tbl`，也不认工作表表格。
幻灯片上的表进每页 `tables`。备注里的表只在 `scope.notes` 同开时出现在 `notes` 上。
母版与版式上的表只在 `scope.masters` 同开时出现在该母版或版式上。逐键范围以 [pptx 分册](json-contract-pptx.md#scope) 为准。
pptx 的 `scope.connectors` 列出该页 `p:cxnSp`；`scope.notes` 与 `scope.connectors` 同开时备注上也列；`scope.masters` 同开时母版 / 版式上也列。
apply 用 `insertConnector` / `deleteConnector` / `setConnector` / `groupShapes` / `ungroup`，
以及 `insertShape` 的 `prst` / `geom`、`setShapeGeom`。
xlsx 的 `groupShapes` / `ungroup` 认工作表文本框、无 `txBox` 形状与连接线。
docx 的 `scope.connectors` 列出正文与页眉页脚里的 `wps` 连接线；
`scope.shapes` 列出无文本框的 `wps:wsp` 形状。
apply 用 `insertConnector` / `deleteConnector` / `insertShape` / `deleteShape` / `setShapePos` / `setShapeSize` / `setShapeFill` / `setShapeLine` / `groupShapes` / `ungroup`。
docx 的图、文本框、连接线、无框形状、SmartArt、图表、墨迹画布与 3D 节点带 `hyperlink`（DrawingML `a:hlinkClick`），不进 `hyperlinks` 名单，也不计入 `hyperlinkCount`。OLE 不写这条链。
docx 的 `scope.inkCanvases` 列出正文与页眉页脚里的墨迹画布（`wpc` 画布加 `w14:contentPart` 加 InkML 部件）；
apply 用 `insertInkCanvas` / `replaceInkCanvas` / `deleteInkCanvas` / `setInkCanvasSize` / `setInkCanvasWrap`。xlsx / pptx 拒这些键与 op。
三种格式的 `scope.model3ds` 列出 3D 模型（`am3d:model3d`，GLB 部件）。
apply 用 `insertModel3d` / `replaceModel3d` / `deleteModel3d` / `setModel3dSize` / `setModel3dCamera`。
docx 另认 `setModel3dWrap`。xlsx 另认 `setModel3dAnchor`。pptx 另认 `setModel3dPos`。
`setModel3dCamera` 改 `am3d:camera`、`am3d:rot` 与 `spPr` 里的 `a:scene3d/a:lightRig`。不把 `ambientLight` / `ptLight` 放进树。
不是图，不进 `pictures`。对 3D id 发图的 size / pos / wrap / anchor 仍报 `InvalidId`。
pptx 的 `scope.inks` 列出幻灯片墨迹（一笔一个 `p:contentPart`，InkML 在 `/ppt/ink/…`）。位置是画布 EMU，不是单元格。
apply 用 `insertInk` / `replaceInk` / `deleteInk` / `setInkSize` / `setInkPos`。
不是图，不进 `pictures`。不把笔画放进树。xlsx 的 `insertInk` 仍是工作表锚点，字段不同。docx 不认这些 op。
xlsx / pptx 的 `scope.maths` 列出 drawing 公式框（`a14:m` 包 OMML）。
apply 复用 `insertMath` / `setMath` / `deleteMath`。xlsx 另认 `setMathAnchor` / `setMathSize`。pptx 另认 `setMathPos` / `setMathSize`。
`setMath` 仍只改字。不是单元格 `f`，也不把 Word 的 `expr` 搬过去。对公式框 id 发文本框 / 形状的锚点与尺寸 op 仍报 `InvalidId`。
docx 的 `scope.maths` 仍是段落 `oMath`，见 [docx 分册](json-contract-docx.md#insertmath)。
xlsx 的 `scope.oleObjects` 与 OLE op 认工作表 `<oleObject>`（ECMA-376 §18.3.1.59），不认 `@link` 与 ActiveX `controls`，不把嵌入字节或预览图塞进树。
pptx 的 `scope.comments` 与批注 op 认该页 2018 线程化批注（`p188:cmLst`），不认 ECMA-376 §19.4 旧式部件。
pptx 的 `scope.timing` 列出该页 `<p:timing>` 效果摘要（含 `delay` / `dur` / `repeat` / 声音名 / 段落 / 点对象）；`scope.notes` 与 `scope.timing` 同开时备注对象带 `timing`；`scope.masters` 与 `scope.timing` 同开时母版 / 版式带 `timing`；`scope.notesMaster` / `scope.handout` 与 `scope.timing` 同开时备注母版 / 讲义带 `timing`。
apply 用 `insertAnim` / `setAnim` / `deleteAnim` / `moveAnim` 按模板增删改预设效果，可写 delay、dur、repeat、效果声音与按段；第一条 `with` / `after` 进片自动播。不接受整棵时间轴树。
`setTransition` 还可写自动进片、方向、`p15:prstTrans`、切换声音、`split` 的 `orient`、`wheel` 的 `spokes`，以及母版 / 版式上的切换。
pptx 的 `scope.masters` 列出幻灯片母版与版式、占位符与装饰形（含该母版 theme 的配色、字体、`fmtScheme` 常用槽与补充色板），根上始终带 `slideSize` 与 `notesSize`，apply 可用 `setSlideLayout` 换页的版式关系，
也可用 `insertLayout` / `deleteLayout` / `insertPlaceholder` / `deletePlaceholder` / `insertMaster` / `deleteMaster` 插删版式、占位符与母版，
也可用 `insertShape` / `deleteShape` 插删母版 / 版式上没有 `p:ph` 的装饰形，
也可用 `setSlideSize` / `setNotesSize` / `setThemeColors` / `setThemeFonts` / `setThemeFmt` / `setThemeExtraSchemes` / `setThemeObjectDefaults` 改幻灯片画布、备注页画布与 slideMaster / notesMaster / handout 的 theme，
也可用 `setClrMap` / `setClrMapOvr` / `setThemeOverride` 改母版配色映射、页 / 版式覆盖与页级 themeOverride，
也可用 `setShapeText` / `setShapeFormat` / `setShapePos` / `setShapeSize` 改占位符与装饰形。
装饰形还可用 `setShapeGeom`。
pptx 的 `scope.handout` 列出讲义母版 theme、页眉页脚槽位与占位符，apply 用 `setHandoutText` 改四个槽位的字，也可用 `insertPlaceholder` / `deletePlaceholder` / `setShapeText` 改讲义占位符，也可用同一套 theme op 改讲义 theme。
pptx 的 `scope.notesMaster` 列出备注母版 theme 与占位符，apply 用同一套占位符 op、预设动画与 theme op。
pptx 的 `scope.hyperlinks` 与超链接 op 认形状级 `p:cNvPr/a:hlinkClick`、run 级 `a:rPr/a:hlinkClick`、悬停事件，以及图 / 表 / 图表 / SmartArt / 单元格上的链。
`scope.notes` 与 `scope.hyperlinks` 同开时备注上也填，`scope.masters` 同开时母版 / 版式上也填。逐键范围以 [pptx 分册](json-contract-pptx.md#scope) 为准。
pptx 的 `scope.fields` 列出幻灯片文本形状里的 `a:fld`；apply 用 `insertField` / `setField` / `deleteField` 写页码与日期域，不求值。
xlsx 仍拒 `scope.fields` 与这三条。docx 仍写 `w:fldChar` / `instr`，并可按 `code` 写常用域与窗体 `ffData`。
pptx 的 `scope.media` 与媒体 op 认幻灯片、备注页与幻灯片母版 / 版式上带 `a:videoFile` / `a:audioFile` 的 `p:pic`，与普通图分家。
inspect 带裁剪、书签与字幕轨；apply 用 `setMedia` 改这些字段，不写 fade，不写 timing。
pptx 的 `scope.sections` 与幻灯片节 op 认 presentation 上 `p14:sectionLst`（MS-PPTX §2.3.1.25）的分组元数据，页顺序仍以 `p:sldIdLst` 为准；不要和 docx 的 `w:sectPr` 节混淆。
pptx 的 `scope.customShows` 与自定义放映 op 认 `p:custShowLst`（ECMA-376 §19.2.1.5–19.2.1.7），页引用是 presentation 关系 `r:id`，不实现放映引擎。
pptx 的 `scope.oleObjects` 与 OLE op 认幻灯片上带嵌入关系的 `p:oleObj`（`p:graphicFrame`，ECMA-376 §19.3.2.4），
不认 `p:link` 与 ActiveX `p:control`，不把嵌入字节或预览图塞进树。
三家都可用 `setOleSize` 改已有嵌入对象的预览框。不启动 OLE 服务器，不重算嵌入文件。
pptx 的 `scope.zooms` 与 Zoom op 认幻灯片上的 `sldZm` / `sectionZm` / `summaryZm`（MS-PPTX §2.2.15），Fallback 预览不进 `pictures`，不实现放映跳转。
xlsx 的 `scope.comments` 与批注 op 认工作表上的 MS-XLSX 线程化批注（`threadedComment`），不认作者不以 `tc=` 开头的旧式 VML 批注。
xlsx 的 `scope.textBoxes` 与文本框 op、`setShapeText` / `setShapeFormat` 认工作表 DrawingML 文本框（`xdr:sp` 且 `txBox`）。
xlsx 的 `scope.shapes` 列出工作表上无 `txBox` 的 `xdr:sp`；`scope.connectors` 列出工作表 `xdr:cxnSp`。
apply 用 `insertShape` / `deleteShape` / `insertConnector` / `deleteConnector`。
`setTextBoxSize` / `setTextBoxAnchor` 也认形状 id 与连接线 id。
xlsx 的 `scope.smartArt` 与 SmartArt op 认工作表 diagram `xdr:graphicFrame`。
xlsx 的 `scope.hyperlinks` 与单元格超链接 op 认工作表 `<hyperlinks>`（ECMA-376 §18.3.1.47），不认 `HYPERLINK()` 公式。
图 / 文本框 / 形状 / SmartArt 上的 DrawingML 链走对象节点的 `hyperlink` 字段，不进这份名单。
xlsx 的 `scope.sparklines` 与迷你图 op 认工作表 `extLst` 里的 `x14:sparklineGroups`（MS-XLSX §2.4.8），可读写颜色、标记与坐标轴，不把折线画进树。
xlsx 的 `scope.connections` 与连接 op 认工作簿 `/xl/connections.xml`（ECMA-376 §18.13），不刷新、不把查询结果写进单元格。
xlsx 的 `scope.externalLinks` 与外部引用 op 认工作簿 `<externalReferences>`（ECMA-376 §18.14），不打开目标文件，不求值。
xlsx 的 `scope.customViews` 与自定义视图 op 认工作簿 `<customWorkbookViews>`（ECMA-376 §18.2.3 / §18.2.4），不按视图恢复隐藏行 / 列。
xlsx 的 `scope.cellStyles` 与命名单元格样式 op 认 `/xl/styles.xml` 的 `<cellStyles>`（ECMA-376 §18.8.7），不改主题，不把 `cellStyleXfs` 摊进名单。
xlsx 可用 `setProtection` 开关工作表 `sheetProtection` 与工作簿 `workbookProtection`，给了 `password` 则写 SHA-512。
xlsx 也可用 `setThemeColors` / `setThemeFonts` / `setThemeFmt` / `setThemeExtraSchemes` / `setThemeObjectDefaults` / `setThemeOverride` 改工作簿 theme，
id 必须是 `xlsx:workbook`。不把格子上的 theme 指数算成 RGB。
xlsx 也可用 `insertProtectedRange` / `setProtectedRange` / `deleteProtectedRange` 写工作表允许编辑区域。
路径 `.xlsm` 按同一套 SpreadsheetML 打开，inspect 树的 `format` 仍为 `xlsx`。xlsx 的 `exportChartData` 仍拒绝。
人工打开只认 Microsoft Office，见三份 checklist。

## 信封

每份请求都是一个 JSON 对象，必须含：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `schemaVersion` | 整数 | 可选。省略视为 `1`。写了就必须是 `1` |
| `action` | 字符串 | `inspect`、`create`、`apply`、`exportMedia`、`exportChartData`、`exportChartImage`、`exportSparklineImage`、`exportSheetImage`、`exportSlideImage`、`exportPageImage`、`layoutCheck`、`help` 之一 |
| `locale` | 字符串 | 可选。错误 `message` 的语言偏好，见 [errors.md](errors.md) |

`locale` 不属于动作参数，十二种动作都认。取值宽松匹配：`zh`、`zh-CN`、`zh-Hans`、`zh-SG` 出中文；`en` 及其地区变体出英文；不认识的标签（包括空串）退回英文，不报 `InvalidField`。
这是信封里唯一一处对未知取值不报错的字段：语言只影响呈现，不该挡下文档操作。优先级是请求 `locale` 高于命令行 `--lang`，再高于环境变量 `OOXP_LANG`，都没有时是英文。`ooxp mcp --lang` 与这里的 `--lang` 同一档。没带时，错误文案只剩 `locale` 和 `OOXP_LANG`。`help` 在请求和 `--lang` 都没给语言时仍是中文。自己另写的外部 MCP 没有后两档，只能写请求里的 `locale`。

另外按动作需要 `path`、`format`、`scope`、`workbook`、`document`、`presentation`、`ops`、`id`、`output` 等字段，见下文。
禁止在请求里放一整棵 inspect 树来表示「把文档改成这样」。create 的 xlsx 用 `workbook`，docx 用 `document`，pptx 用 `presentation`，三者不能同时出现。

`format` 取值：`xlsx`、`xlsm`、`docx`、`pptx`。可省略，省略时从 `path` 的扩展名推断。推断不出就报 `UnsupportedFormat`。
`xlsm` 与 `xlsx` 走同一套 SpreadsheetML：inspect 树的 `format` 仍为 `xlsx`。不执行宏，不把 `vbaProject.bin` 当对象编辑。不认 `.xls`。
给出 `format` 时，机器可读 Schema 里的 `ops` 只接受该格式的动作。`xlsm` 与 `xlsx` 同一份动作名单。
省略 `format` 时，Schema 仍接受任一已登记动作的形状。库按路径推断格式：别家已登记动作报 `InvalidField`，名单里没有的动作报 `UnsupportedOp`。

`path` 是本地文件系统路径。Windows 反斜杠和正斜杠都接受。

请求里每个字符串，无论是值还是键，都不得含有 XML 1.0 不允许的字符。遇到第一个就拒绝整份请求，不写文件。

- `path` 指向该字段，文案带码位，例如 `U+000B`
- 错误码是 `InvalidField`

- 不允许：U+0000–U+0008、U+000B、U+000C、U+000E–U+001F、U+FFFE、U+FFFF
- 允许：TAB、LF、CR，以及 U+0020 以上的普通字符
- 图片、音视频、OLE、墨迹、3D、字体这些二进制不在这份检查里。它们从路径读入，不把字节放进 JSON

## help

`help` 不读文件，也不改文件。用来查某个操作该怎么填，避免把别的格式的字段套过来。

本工具不支持预览。xlsx 的簇状柱形图、簇状条形图、普通折线图、带标记的散点图、普通面积图和普通饼图可以按 `exportChartImage` 写成 PNG。xlsx 的折线/非负柱形/盈亏迷你图可以按 `exportSparklineImage` 写成 PNG，合法日期轴按日期比例定位。xlsx 的已用区域可以按 `exportSheetImage` 写成 PNG。pptx 可以按 `exportSlideImage` 把页底（没有颜色变换的纯色、不透明拉伸铺满 PNG，以及两停点、直角、不透明线性渐变）、PNG、矩形、带 `a:alpha` 的半透明纯色矩形、线宽恰好是 1 或 2 像素的直角矩形描边、圆角矩形、椭圆、两色直线渐变、水平或垂直的直线、直角折线连接符、弯曲连接符、形状文字、簇状柱形图和纯色单元格的表写成图。两色直线渐变画在没有旋转的矩形、圆角矩形和椭圆上，两档可以带 `alpha`。两停点、直角、不透明的线性页底按同一条 Office 16.0 S 形曲线铺满整页；半透明、路径、非直角、不是 0/100000 两停点，或主题 `bgRef` 渐变仍铺白。形状上的 `blipFill` 会按拉伸铺进这些形状；画不了的变体让整页失败，不再静默跳过。create 把文本里的换行写成 `a:br` 时，预览器也会画出后续行。画不了的对象让整页失败。docx 没有字的页，以及 Windows 上单节、单段（可按空格换行）的不透明拉丁 Calibri 正文，可以按 `exportPageImage` 写成 PNG。第二段、表格、图和页眉还不画。像素不进 inspect。机器上有 Microsoft Office 或 WPS 时，可以自己导出后再查看：

- Word、Excel，以及 WPS 的 `kwps`、`ket`，用 `ExportAsFixedFormat` 写成 PDF
- PowerPoint 和 WPS 的 `kwpp`，用幻灯片的 `Export` 写成 PNG

本工具不调用它们。不写 `format`、`q`、`op`、`name` 时，这句话写在 `help` 这一条的 `limits` 里。搜「预览」或 `preview` 也会命中。

不带 `format`、`q`、`op`、`name` 时，只说明十二个动作各自干什么，以及下一步该带哪些字段。

带 `format` 时，列出该格式的操作。`description` 只写这个操作做什么。填的时候要注意的放在 `limits`，另成一条。`xlsm` 与 `xlsx` 同一份名单。

带 `q` 时，同时搜动作名、定义名、操作名、字段名，以及中英说明和 `limits`。字段名会看进 `properties`、`items`、`oneOf` 里面，所以写在 `runs` 里的 `color` 也算。可以再带 `format`。命中太多时只返回前 20 条，并带上总数。

同一个操作不列两次。`setParaRunsOp` 这种定义名不再单独占一条，只留操作那条。排序是名字精确匹配，然后名字里包含这个词，然后字段名，然后说明，最后才是 `limits`。`limits` 里碰巧出现的词不会排到名字命中前面。字段名这一档里，操作排在定义前面。

每条结果有 `kind` 和 `next`，告诉下一步怎么打开：

- `kind` 为 `action`：这是动作。`next` 形如 `name=createRequest`。用 `name` 打开，不要当成 `op`。
- `kind` 为 `def`：这是定义，例如 `docxDocument`、`docxParaCreate`。`next` 形如 `name=docxParaCreate`。
- `kind` 为 `op`：这是 apply 操作。`next` 形如 `op=setParaText`。多种格式要再带 `format`。

`format` 只过滤操作，以及 `xlsx` / `docx` / `pptx` 开头的定义。新建正文这种共用定义仍然会出来。返回的句子只留一种语言。

带 `op` 时，返回这一个操作的说明和字段。里面的引用只留定义名，不继续展开。同一种名字在多种格式里形状不同时，必须带 `format`；不带就只列出每种格式的说明，不把字段并成一份。

带 `name` 时，打开一个定义，例如 `inspectScope`、`xlsxWorkbook`。新建和查看范围不在操作名单里，走这里。

`name=inspectScope` 是特例，返回形状单独定，不沿用 `op` 那条 `chooseFormat`。

- 不带 `format`：`kind` 为 `chooseFormat`，`name` 为 `inspectScope`，`formats` 为 `["xlsx", "docx", "pptx"]` 三个字符串，不给 `fields`。
  这里的 `formats` 是字符串数组，不带说明对象，与 `op` 那条 `chooseFormat` 的 `formats` 不是一回事。
- 带 `format`：`kind` 仍是 `def`，`fields.properties` 只留该格式认的键。
- `format` 为 `xlsm` 时，先按 `crates/ooxp/src/json.rs` 的 `parse_format` 收成 `xlsx` 再筛。
  `help()` 用 `format_key` 归一后把该值传给 `def_detail`，现有运行结果不会筛出空表。

过滤依据写在 `docs/json-schema/src/shared/envelope.json` 里：`inspectScope.properties` 的每个键带一个 `formats` 数组，`help` 按它筛，不在代码里另抄一份键表。
筛完的 `fields.properties` 里仍带这个 `formats`，值来自 schema，不随 `format` 改写。
请求 schema 里的 `inspectScope` 始终是三格式键的并集。跨格式键由各格式的 `parse_scope` 拒绝，报 `InvalidField`，`help` 过滤不改变这件事。

`op` 与 `name` 不能同时出现。`q` 不能和 `op` 或 `name` 同时出现。不写 `locale` 时，`description` 和 `limits` 是中文。`locale` 为 `en` 时这两条换成英文。一次结果里只出一种语言。动作目录里的 `ask.search.q` 也跟着这一种语言：中文是「单元格」，英文是 `cell`。

## 稳定 id

id 是给模型用的字符串，必须能从文件内容稳定算出来，不能依赖本次运行的内存地址或随机数。

解析规则按格式走。id 里的工作表名、段落 id、形状 id 都按文件里的原文，不做 Unicode 正规化。

三种格式的 id 表见 [xlsx](json-contract-xlsx.md#稳定-id)、[docx](json-contract-docx.md#稳定-id)、[pptx](json-contract-pptx.md#稳定-id)。

一次 apply 里，已经固定的位置 id 按 apply 开始时的文件解释，不随本批前面的增删改序号。

已经固定：

- docx 的 `para:index:{n}`，以及页眉页脚、框内、注内、批注内的 `…/para:index:{k}`
- docx 的 `table:{n}`、`row:{r}`、`r{r}c{c}`，含页眉页脚与框内
- docx 段内的 `bookmark:{n}`、`field:{n}`、`link:{n}`、`math:{n}`、`ruby:{n}`、`sdt:{n}`、`perm:{n}`、`rev:{n}`、`run:{n}`，以及绘图对象的 `:index:{k}`。计数范围见 [docx 分册](json-contract-docx.md#稳定-id)
- docx 的 `section:{n}`、页眉页脚容器、`toc:{n}`、`caption:{n}`、`xref:{n}`、`index:{n}`、`xe:{n}`、`source:{n}`、`cite:{n}`、`biblio:{n}`、`docPart:{n}`、`font:{n}`
- pptx 的 `slide:{n}`，以及挂在它下面、按内容编号的形状、图、图表、表、备注、批注等 id
- xlsx 表内名单、工作簿名单，以及 drawing 对象的 `:index:{k}`。计数范围见 [xlsx 分册](json-contract-xlsx.md#稳定-id)
- pptx 的母版、版式、节、自定义放映，以及页上对象的 `:index:{k}`、链接、域、表的行列。计数范围见 [pptx 分册](json-contract-pptx.md#稳定-id)
- 三种格式共用的 `series:{idx}`、`trendline:{k}`、`errorBars:{k}`。系列按图表里 `c:ser` 的顺序从 0 起，趋势线和误差线在各自系列里从 1 起

被前面的 op 删掉后再点名，报 `IdNotFound`，不落到旁边的对象上。同一批里新建的对象不能用这些位置 id 点名，要等下一次 inspect。

`after`、`start`、`end` 这类字段如果点的是已经固定的位置 id，同样按 apply 开始时解释。

## inspect

成功响应的对象字段按各分册响应节示例的语义顺序写出，不是按字母排序。命令行打印同一棵树，见 [cli.md](cli.md)。

### 请求

``json
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
``

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

只有 create 允许从 JSON 生成整包。`path` 必填。

xlsx 的 `workbook` 见 [json-contract-xlsx.md](json-contract-xlsx.md#create)。
docx 的 `document` 见 [json-contract-docx.md](json-contract-docx.md#create)。
pptx 的 `presentation` 见 [json-contract-pptx.md](json-contract-pptx.md#create)。

## apply

apply 的入参是操作列表，不是树。另可带布尔字段 `dryRun`。

- `dryRun: true` 时各条 op 仍按顺序在内存树上执行，但不生成包字节，也不写盘。
- `dryRun: true` 时 `output` 被忽略，不报 `InvalidField`，仍然不写文件。`dryRun` 省略或为 `false` 时，`output` 照旧是另存路径。
- 成功响应不带 `path`，改带 `dryRun: true`、`opsApplied` 与 `parts`。`parts` 是本批标脏的部件名，排序去重。
- 任一 op 失败仍按现有错误形态返回，源包不变。不新增事务或回滚类型。
- 省略或 `false` 时行为不变。响应形状见 [errors.md](errors.md#成功形态)。

下面这种写法非法，必须报 `InvalidField`：

``json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "book.xlsx",
  "sheets": [],
  "cells": []
}
``

合法请求：

``json
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
``

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `path` | 是 | 源文件 |
| `output` | 否 | 省略则覆盖 `path` |
| `ops` | 是 | 非空数组。按数组顺序执行，中途失败则整次 apply 不写文件。空数组不是原样保存，报 `EmptyArray`。零脏写回只在 OPC 层，见 [opc.md](opc.md) |

create 与 apply 落盘时先写入目标同目录的临时文件，再改名覆盖目标路径。改名失败时删掉临时文件，原文件不动，报 `Io`，错误路径是目标文件。目录里不留 `.ooxp.bak`。

只属于某一种格式的 op 正文见 [xlsx](json-contract-xlsx.md#apply)、[docx](json-contract-docx.md#apply)、[pptx](json-contract-pptx.md#apply)。
跨格式 op（图、图表、SmartArt、批注、超链接、文本框、表格插删行列、`setProps`、`setProtection`、`setShapeText` 等）仍写在本文。

不认识的 `op` 报 `UnsupportedOp`，不要忽略。xlsx 认 `setCell`、`setCellStyle`、`setHeaderFooterText`、`replacePicture`、`replaceLegacyPicture`、`deleteLegacyPicture`、`insertPicture`、
`deletePicture`、`setPictureSize`、`setPictureAnchor`、以及图表 op（`insertChart`、`insertChartSheet`、`deleteChartSheet`、
`deleteChart`、`setChartTitle`、`setChartSeries`、`addChartSeries`、`deleteChartSeries`、`setChartType`、`setChartAxisTitle`、
`setChartLegend`、`setChartDataLabels`、`insertTrendline`、`setTrendline`、`deleteTrendline`、`insertErrorBars`、
`setErrorBars`、`deleteErrorBars`、`setChartSeriesAxis`、`setChartSize`、`setChartAnchor`；
不含 `setChartWrap`）、以及 `setProps` / `setCustomProp` / `deleteCustomProp`、以及工作表管理 op（`insertSheet`、`deleteSheet`、
`renameSheet`、`moveSheet`、`setSheetHidden`）、以及列宽行高与合并 op（`setColWidth`、`setRowHeight`、`mergeRange`、`unmergeRange`）、以及隐藏行列 op（`setRowHidden`、`setColHidden`）、
以及数据验证 op（`insertDataValidation`、`setDataValidation`、`deleteDataValidation`）、以及条件格式 op（`insertConditionalFormat`、
`setConditionalFormat`、`deleteConditionalFormat`）、以及工作表表格 op（`insertSheetTable`、`setSheetTable`、`deleteSheetTable`）、
以及透视表 op（`insertPivotTable`、`setPivotTable`、`deletePivotTable`）、以及定义名 op（`insertDefinedName`、`setDefinedName`、
`deleteDefinedName`）、以及 `setPrintSetup`、`setSheetView`、`setAutoFilter`、`setSortState`、以及自定义视图 op（`insertCustomView`、
`deleteCustomView`、`showCustomView`）、以及分组 op（`groupRows`、`ungroupRows`、`groupCols`、`ungroupCols`、`setOutline`）、
以及批注 op（`insertComment`、`deleteComment`、`replyComment`、`setCommentText`、`setCommentBox`、`setCommentMeta`、`setCommentDone`）、
以及 `setShapeText`、`setShapeFormat`、以及文本框 op（`insertTextBox`、`deleteTextBox`、`setTextBoxSize`、`setTextBoxAnchor`；
不含 `setTextBoxWrap`、`setTextBoxFill`、`setTextBoxLine`、`setTextBoxDirection`、`setTextBoxTextAnchor`）、以及形状管理 op（`insertShape`、`deleteShape`、
`insertConnector`、`deleteConnector`、`groupShapes`、`ungroup`；
不含 `setShapePos`、`setShapeSize`、`setShapeFill`、`setShapeLine`、`setShapeGeom`、`setConnector`）、以及 SmartArt op（`insertSmartArt`、`deleteSmartArt`、
`setSmartArtText`、`addSmartArtPoint`、`deleteSmartArtPoint`、`setSmartArtSize`、`setSmartArtAnchor`、`setSmartArtColors`、
`setSmartArtStyle`、`setSmartArtLayout`；
不含 `setSmartArtPos`、`setSmartArtWrap`）、以及超链接 op（`insertHyperlink`、`setHyperlink`、`deleteHyperlink`）、
以及迷你图 op（`insertSparkline`、`setSparkline`、`deleteSparkline`）、以及 OLE op（`insertOle`、`deleteOle`、`setOleSize`）、以及 3D 模型 op（`insertModel3d`、`replaceModel3d`、`deleteModel3d`、`setModel3dSize`、`setModel3dAnchor`、`setModel3dCamera`；不含 `setModel3dWrap`、`setModel3dPos`）、以及公式 op（`insertMath`、`setMath`、`deleteMath`、`setMathSize`、`setMathAnchor`；不含 `setMathPos`）、以及切片器 op（`insertSlicer`、
`deleteSlicer`、`setSlicerSize`、`setSlicerAnchor`、`setSlicerView`、`setSlicerFilter`）、以及连接 op（`insertConnection`、`deleteConnection`）、以及外部引用 op（`insertExternalLink`、`setExternalLink`、`deleteExternalLink`）、以及 `setProtection`、以及允许编辑区域 op（`insertProtectedRange`、`setProtectedRange`、`deleteProtectedRange`）、以及命名单元格样式 op（`insertCellStyle`、`deleteCellStyle`、`setCellNamedStyle`）、
以及主题 op（`setThemeColors`、`setThemeFonts`、`setThemeFmt`、`setThemeExtraSchemes`、`setThemeObjectDefaults`、`setThemeOverride`）。
docx 认 `setParaText`、`setHeaderFooterDistance`、`setPageMargin`、`setPageSize`、`setColumns`、`setVerticalAlign`、
`setLineNumbering`、`setDocGrid`、`setRtlGutter`、`setMirrorMargins`、`setGutterAtTop`、`setHeaderFooterLink`、
`insertHeaderFooter`、`setTitlePage`、`setEvenAndOddHeaders`、图片 op（含 `setPictureWrap`，
不含 `setPictureAnchor`）、文本框 op、组合与连接线 op（`insertConnector`、`deleteConnector`、`groupShapes`、`ungroup`）、
无框形状 op（`insertShape`、`deleteShape`、`setShapePos`、`setShapeSize`、`setShapeFill`、`setShapeLine`；
不含 `setShapeGeom`、`setConnector`、`setShapeFormat`）、`insertPara` / `deletePara`、
表格 op（`insertTable`、`deleteTable`、`insertRow`、`deleteRow`、`insertCol`、`deleteCol`、`mergeCells`、`splitCells`、
`setTableStyle`、`setTableFloat`、`setTableWidth`、`setTableAlign`、`setTableRowHeight`、`setTableCellMar`、`setTableBorders`、`setCellFormat`、`setCellBorders`）、
修订 op（`acceptRevision`、`rejectRevision`）、超链接与书签 op（`insertHyperlink`、`setHyperlink`、`deleteHyperlink`、`insertBookmark`、
`setBookmark`、`deleteBookmark`）、例外编辑区 op（`insertEditRange`、`deleteEditRange`）、域 op（`insertField`、`setField`、`deleteField`）、目录 op（`insertToc`、`deleteToc`）、
题注与交叉引用 op（`insertCaption`、`deleteCaption`、`insertCrossRef`、`deleteCrossRef`）、索引 op（`insertIndex`、`deleteIndex`、
`insertIndexEntry`、`deleteIndexEntry`）、参考文献 op（`insertSource`、`deleteSource`、`setBibliographyStyle`、`insertCitation`、`deleteCitation`、
`insertBibliography`、`deleteBibliography`）、构建基块 op（`insertDocPart`、`deleteDocPart`、`insertDocPartContent`）、
字体嵌入 op（`insertEmbeddedFont`、`deleteEmbeddedFont`、`setFontEmbed`）、页面背景与边框 op（`setPageBackground`、`setPageBorders`、`setFormProt`）、
公式 op（`insertMath`、`setMath`、`deleteMath`）、拼音 op（`insertRuby`、`setRuby`、`deleteRuby`）、`setEastAsianSettings`、内容控件 op（`insertContentControl`、`setContentControl`、`deleteContentControl`）、
编号 op（`insertNumbering`、`setNumbering`、`setParaNumbering`）、
样式 op（`insertStyle`、`deleteStyle`、`setStyle`、`setParaStyle`、`setRunStyle`）、直接格式 op（`setParaFormat`、
`setRunFormat`）、节结构 op（`insertSection`、`deleteSection`）、脚注尾注 op（`insertFootnote`、`insertEndnote`、`deleteFootnote`、
`deleteEndnote`、`setNoteNumbering`）、批注 op（`insertComment`、`deleteComment`、`replyComment`、`setCommentText`、
`setCommentMeta`、`setCommentDone`）、图表 op（`insertChart`、`deleteChart`、`setChartTitle`、`setChartSeries`、`addChartSeries`、
`deleteChartSeries`、`setChartType`、`setChartAxisTitle`、`setChartLegend`、`setChartDataLabels`、`insertTrendline`、
`setTrendline`、`deleteTrendline`、`insertErrorBars`、`setErrorBars`、`deleteErrorBars`、`setChartSeriesAxis`、`setChartSize`、`setChartWrap`；
不含 `setChartAnchor`、`insertChartSheet`、`deleteChartSheet`）、以及 SmartArt op（`insertSmartArt`、`deleteSmartArt`、
`setSmartArtText`、`addSmartArtPoint`、`deleteSmartArtPoint`、`setSmartArtSize`、`setSmartArtWrap`、`setSmartArtColors`、
`setSmartArtStyle`、`setSmartArtLayout`）、以及 OLE op（`insertOle`、`deleteOle`、`setOleSize`）、以及墨迹画布 op（`insertInkCanvas`、`replaceInkCanvas`、`deleteInkCanvas`、`setInkCanvasSize`、`setInkCanvasWrap`）、以及 3D 模型 op（`insertModel3d`、`replaceModel3d`、`deleteModel3d`、`setModel3dSize`、`setModel3dWrap`、`setModel3dCamera`；不含 `setModel3dAnchor`、`setModel3dPos`）、以及 `setProtection`、
以及 `insertWatermark` / `deleteWatermark`、以及 `setThemeColors` / `setThemeFonts` / `setThemeFmt` / `setThemeExtraSchemes` / `setThemeObjectDefaults` / `setThemeOverride`、
以及 `setProps` / `setCustomProp` / `deleteCustomProp`。
pptx 认 `setShapeText`、`setShapeFormat`、形状管理 op（`insertShape`、`insertConnector`、`deleteConnector`、`setConnector`、`groupShapes`、`ungroup`、`deleteShape`、
`setShapePos`、`setShapeSize`、`setShapeFill`、`setShapeLine`、`setShapeGeom`）、`setPageBackground`、表格 op（`insertTable`、`deleteTable`、`insertRow`、`deleteRow`、
`insertCol`、`deleteCol`、`setTableCellText`、`setTableCellFill`、`setTableCellBorders`、`setTableCellInset`、`setTableCellPr`、`setTableStyleRef`、`setTablePos`、`setTableSize`、`setTableColWidths`、`setTableTrHeight`；
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
`deleteMaster`）、以及主题与尺寸 op（`setSlideSize`、`setNotesSize`、`setThemeColors`、`setThemeFonts`、`setThemeFmt`、`setThemeExtraSchemes`、`setThemeObjectDefaults`、`setClrMap`、`setClrMapOvr`、`setThemeOverride`）、以及动画 op（`insertAnim`、`setAnim`、`deleteAnim`、`moveAnim`）、
以及 `setHandoutText`、以及超链接 op（`insertHyperlink`、`setHyperlink`、`deleteHyperlink`）、以及域 op（`insertField`、`setField`、
`deleteField`）、以及媒体 op（`insertVideo`、`insertAudio`、`deleteMedia`、`setMedia`）、以及幻灯片节 op（`insertSlideSection`、`setSlideSectionName`、
`deleteSlideSection`、`setSlideSection`）、以及自定义放映 op（`insertCustomShow`、`setCustomShow`、`deleteCustomShow`）、
以及 OLE op（`insertOle`、`deleteOle`、`setOleSize`）、以及 3D 模型 op（`insertModel3d`、`replaceModel3d`、`deleteModel3d`、`setModel3dSize`、`setModel3dPos`、`setModel3dCamera`；不含 `setModel3dWrap`、`setModel3dAnchor`）、以及墨迹 op（`insertInk`、`replaceInk`、`deleteInk`、`setInkSize`、`setInkPos`；不含 `setInkAnchor`）、以及公式 op（`insertMath`、`setMath`、`deleteMath`、`setMathSize`、`setMathPos`；不含 `setMathAnchor`）、以及 Zoom op（`insertZoom`、`setZoom`、`deleteZoom`）、以及 `setShowPr`、以及 `setProtection`、
以及 `setProps` / `setCustomProp` / `deleteCustomProp`。
不提供把整棵时间轴当 JSON 写入的 op；`setTiming` 仍报 `UnsupportedOp`。
对不上格式的 op（例如对 pptx 发 `setCell` / `setCellStyle` / `setParaText` / `setProtection`，
对 xlsx 发 `setPictureWrap`、`setChartWrap` 或 `setHeaderFooterDistance` / `setPageMargin` / `setPageSize` / `setColumns`
 / `setVerticalAlign` / `setLineNumbering` / `setDocGrid` / `setRtlGutter` / `setPageBackground` / `setPageBorders` / `setFormProt`
 / `setMirrorMargins` / `setGutterAtTop` / `setHeaderFooterLink` / `insertHeaderFooter` / `setTitlePage` / `setEvenAndOddHeaders`、
`setTextBoxWrap` / `setTextBoxFill` / `setTextBoxLine` / `setTextBoxDirection` / `setTextBoxTextAnchor`、注、`setSmartArtPos` / `setSmartArtWrap`、
`insertWatermark` / `deleteWatermark`、`insertInkCanvas` / `replaceInkCanvas` / `deleteInkCanvas` / `setInkCanvasSize` / `setInkCanvasWrap`、`setModel3dWrap`、表格 op、修订 op、书签 op、例外编辑区 op、域 op、题注与交叉引用 op、索引 op、参考文献 op、`setBibliographyStyle`、构建基块 op、字体嵌入 op、编号 op、样式 op、
直接格式 op、拼音 op、`setEastAsianSettings`、节结构 op 或 `setNotesText`，
对 docx / pptx 发 `setHeaderFooterText`、`insertChartSheet`、`deleteChartSheet`、`insertSheet`、`deleteSheet`、`renameSheet`、
`moveSheet`、`setSheetHidden`、`setCellStyle`、`setColWidth`、`setRowHeight`、`mergeRange` 或 `unmergeRange`、
`setRowHidden` / `setColHidden`、
`insertDataValidation` / `setDataValidation` / `deleteDataValidation`、
`insertConditionalFormat` / `setConditionalFormat` / `deleteConditionalFormat`、
`insertSheetTable` / `setSheetTable` / `deleteSheetTable`、`insertPivotTable` / `setPivotTable` / `deletePivotTable`、
`insertDefinedName` / `setDefinedName` / `deleteDefinedName`、`setPrintSetup`、`setSheetView`、`setAutoFilter`、
`groupRows` / `ungroupRows` / `groupCols` / `ungroupCols` / `setOutline`、`insertSlicer` / `deleteSlicer` / `setSlicerSize` / `setSlicerAnchor` / `setSlicerView` / `setSlicerFilter`、
`insertConnection` / `deleteConnection` / `insertExternalLink` / `setExternalLink` / `deleteExternalLink` / `insertCustomView`
 / `deleteCustomView` / `showCustomView` / `insertProtectedRange` / `setProtectedRange` / `deleteProtectedRange`
 / `insertCellStyle` / `deleteCellStyle` / `setCellNamedStyle`，
对 docx 发 `setPictureAnchor` / `setChartAnchor` / `setSmartArtPos` / `setNotesText` / `setModel3dAnchor` / `setSlicerAnchor` / `setSlicerSize` / `setSlicerView` / `setSlicerFilter` / `setMathAnchor` / `setMathPos` / `setMathSize`，
对 pptx 发 `setPictureWrap` / `setPictureAnchor` / `setChartWrap` / `setChartAnchor` / `setSmartArtWrap` / `setModel3dWrap` / `setModel3dAnchor` / `setSlicerAnchor` / `setSlicerSize` / `setSlicerView` / `setSlicerFilter` / `setMathAnchor`，
对 docx / xlsx 发 `setPicturePos` / `setChartPos` / `setSmartArtPos` / `setModel3dPos` / `setMathPos` / `setNotesText` / `insertSlide` / `deleteSlide`
 / `moveSlide` / `setSlideHidden` / `setTransition` / `insertAnim` / `setAnim` / `deleteAnim` / `moveAnim` / `setSlideLayout` / `insertLayout`
 / `deleteLayout` / `insertPlaceholder` / `deletePlaceholder` / `insertMaster` / `deleteMaster` / `setSlideSize` / `setNotesSize`
 / `setClrMap` / `setClrMapOvr` / `setHandoutText` / `setShapePos` / `setShapeSize` / `setShapeFill`
 / `setShapeLine` / `setShapeGeom` / `setConnector` / `setTableCellText` / `setTableCellFill` / `setTableCellBorders` / `setTableCellInset` / `setTableCellPr` / `setTableStyleRef` / `setTablePos` / `setTableSize` / `setTableColWidths` / `setTableTrHeight` / `insertVideo` / `insertAudio`
 / `deleteMedia` / `setMedia` / `insertSlideSection` / `setSlideSectionName` / `deleteSlideSection` / `setSlideSection`
 / `insertCustomShow` / `setCustomShow` / `deleteCustomShow` / `insertZoom` / `setZoom` / `deleteZoom`）报 `InvalidField`。
xlsx 上发 `insertTable` / `deleteTable` / `insertRow` / `deleteRow` / `insertCol` / `deleteCol` / `mergeCells` / `splitCells` 仍报 `InvalidField`。
对 docx 发 `setShapeGeom` / `setConnector` / `setShapeFormat` 报 `InvalidField`。pptx 上发 `mergeCells` / `splitCells` 报 `InvalidField`。
xlsx / pptx 上发 `insertStyle` / `deleteStyle` / `setRunStyle` / `setTableStyle` / `setTableFloat` / `setTableWidth` / `setTableAlign` / `setTableRowHeight` / `setTableCellMar` / `setTableBorders` / `setCellFormat` / `setCellBorders` 报 `InvalidField`。
pptx 的 `setTablePos` / `setTableSize` / `setTableColWidths` / `setTableTrHeight` 单位仍是 EMU，不要和 docx 的 `setTableFloat` / `setTableWidth` / `setTableRowHeight`、xlsx 的 `setColWidth` / `setRowHeight` 混。`setTableSize` 仍只改框，不重分配格网。

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

``json
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
``

``json
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
``

``json
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
``

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
| `writePassword` | 可选。非空字符串。按 Word 16.0 写修改密码 SHA-512 哈希 |

`enforced` / `edit` / `formatting` / `readOnlyRecommended` / `writePassword` 至少给一个，否则报 `InvalidField`。`password` 不算「至少一个可写字段」。未点名的字段留下。`edit` 为 `none` 或不认识的字符串报 `InvalidField`。
关掉强制请用 `enforced: false`，不要写 `edit: none`。空 `password` / `writePassword` 报 `InvalidField`。
`password` 只用于 `documentProtection`；只改 `readOnlyRecommended` 却给了 `password` 报 `InvalidField`。
`writePassword` 只用于 `writeProtection`。xlsx / pptx 出现 `writePassword` 报未知字段。
出现 `hasPassword` 报 `InvalidField`。id 不是 `docx:document` 报 `InvalidId`。没有 `settings.xml` 报 `OpcPartMissing`。
对 pptx 发此 op 且 id 为 `pptx:presentation` 时走下面 pptx 那一节。

打开强制且没有哈希时，写出 Transitional 标记：`w:edit` 与 `w:enforcement="1"`。
给了 `password` 时再按 Word 16.0 写 `cryptProviderType="rsaAES"`、`cryptAlgorithmClass="hash"`、`cryptAlgorithmType="typeAny"`、
`cryptAlgorithmSid="14"`、`cryptSpinCount="100000"`、`hash`、`salt`（16 字节随机盐的标准 Base64）。
口令按 UTF-8 取前 15 字节做 ECMA-376 Part 4 §14.8.1 遗留预处理，把 4 字节结果写成大写两位十六进制再按 UTF-16LE；
`H0 = SHA-512(salt || 该 UTF-16LE)`，再对 `i = 0 .. 99999` 做 `H_{i+1} = SHA-512(H_i || i 的 4 字节小端)`。
不写 ISO 的 `algorithmName` / `hashValue` / `saltValue` / `spinCount`，不写 `reservationPassword`。
已有哈希时必须给 `password`，否则报 `NotImplemented`；口令对不上报 `InvalidField`。
能核的算法：`cryptAlgorithmSid` 4 / 12 / 13 / 14（SHA-1 / SHA-256 / SHA-384 / SHA-512，限制编辑先做遗留预处理）、ISO `algorithmName` + `hashValue`、以及 `reservationPassword`（ECMA-376 Part 4 §14.8.1）。
对得上则可改种类，或 `enforced: false` 解开（删掉元素）。改写时换成上面的 SHA-512，换新盐。认不出的算法仍报 `NotImplemented`。
无哈希时 `enforced: false` 直接删掉元素。
给了 `writePassword` 时写 `writeProtection`：`crypt*` 属性组与限制编辑相同（`sid=14`、`spin=100000`、`hash`/`salt` 标准 Base64）。
口令按 UTF-16LE；`H0 = SHA-512(salt || password)`，再对 `i = 0 .. 99999` 做 SHA-512 迭代。不要做限制编辑那套遗留预处理。
已有哈希时，改或删 `writeProtection` 必须给对得上的 `writePassword`，否则报 `NotImplemented`；口令对不上报 `InvalidField`。
修改密码不做遗留预处理。能核 SHA-1 / SHA-256 / SHA-384 / SHA-512（`cryptAlgorithmSid` 或 ISO `hashValue`）和 `reservationPassword`。
`writePassword` 且 `readOnlyRecommended: false`：校验后删掉该元素。只给 `writePassword`：换成 SHA-512 并换新盐。有哈希时规范要求忽略 `recommended`，树上 `readOnlyRecommended` 仍为 `false`。
认不出的算法仍报 `NotImplemented`。只改限制编辑时不要动已有 `writeProtection` 哈希。
`edit` 为 `trackedChanges` 时同时写出空的 `w:trackRevisions`；关掉保护时不要删已有的 `trackRevisions`。
`edit` 为 `forms` 时给主文档每一个 `w:sectPr` 写 `w:formProt`；关掉窗体保护或关掉强制时删掉各节 `w:formProt`。
某节例外见 [docx 分册](json-contract-docx.md#setformprot) 的 `setFormProt`。`w:permStart` / `w:permEnd` 见该分册例外编辑区。
已经是目标状态则空操作，仍计入 `opsApplied`。不把哈希放进树。

create 的 `document.protection` 复用同一套规则，包括 `password` 与 `writePassword`。
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
不写遗留 `@password`。已有哈希时必须给对得上的 `password`，否则报 `NotImplemented`；口令对不上报 `InvalidField`。
能核 SHA-512，以及 SHA-1 / SHA-256 / SHA-384。Excel 的 ISO 哈希还会再试一次：把口令先做成 16 位 XOR 校验值，当作一个 UTF-16 字符再哈希。遗留 `@password` 用 MS-OFFCRYPTO 的 16 位 XOR。
对得上则可改权限，或 `enforced: false` 解开（删掉元素）。改写时换成 SHA-512，换新盐。认不出的算法仍报 `NotImplemented`。
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
不写遗留 `@workbookPassword`。已有哈希时必须给对得上的 `password`，否则报 `NotImplemented`；口令对不上报 `InvalidField`。
算法与工作表相同，包括遗留 `@workbookPassword` 的 16 位 XOR。对得上则换成 SHA-512。认不出的算法仍报 `NotImplemented`。按 `CT_Workbook` 插在 `workbookPr` 之后、`bookViews` 之前。不新建部件。

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
已有哈希时必须给对得上的 `password`，否则报 `NotImplemented`；口令对不上报 `InvalidField`。
能核 `cryptAlgorithmSid` 4 / 12 / 13 / 14 的 `hashData`，ISO `hashValue`，以及 Word 风格的 `hash` / `salt`（先按原文，再按遗留预处理）。
对得上则可 `enforced: false` 解开（删掉元素），或 `enforced: true` 换成 SHA-512 并换新盐。认不出的算法仍报 `NotImplemented`。
无哈希时 `enforced: false` 直接删掉元素。没有该元素且 `enforced: false` 则空操作，仍计入 `opsApplied`。
不把哈希放进树。按 `CT_Presentation` 该元素在 `defaultTextStyle` 之后、`extLst` 之前。不新建部件。xlsx / docx 上对本 id 发此 op 仍报 `InvalidId`。

pptx 的放映类型、循环与旁白开关用 `setShowPr`，见 [json-contract-pptx.md](json-contract-pptx.md#setshowpr)。

示例见 `docs/examples/pptx-apply-protect.json`。带密码见 `docs/examples/pptx-apply-protect-password.json`。

### replacePicture

``json
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
``

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

``json
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
``

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
- **`index` / `before` / `after`**
  - 仅 pptx
  - 三者至多一个，规则与 `insertShape` 的图层位置相同
  - 批注内图带这三项报 `InvalidField`
  - docx / xlsx 上出现报未知字段
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

``json
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
``

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

``json
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
``

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setPictureSize"` |
| `id` | 图 id。pptx 还可以是视频 id 或音频 id |
| `cx` | 可选。宽度，EMU，正整数。xlsx `twoCell` 上出现报 `InvalidField` |
| `cy` | 可选。高度，EMU，正整数。xlsx `twoCell` 上出现报 `InvalidField` |
| `to` | xlsx `twoCell` 时必须。右下角 A1。`oneCell` / `absolute` 上出现报 `InvalidField` |
| `toOffX` / `toOffY` | 可选。只随 xlsx `twoCell` 的 `to` 改右下角偏移。省略则留下当前值 |

`cx` 与 `cy` 至少给一个，否则报 `InvalidField`（xlsx `twoCell` 见下）。`cx` / `cy` 与 `to` 不能同时出现。只改被点名的那一侧，另一侧保持原值。
docx：写入 `wp:extent`；若 `pic:spPr/a:xfrm/a:ext` 存在，一并改。
xlsx：`oneCell` 或 `absolute` 时写入 `xdr:ext`；若 `xdr:spPr/a:xfrm/a:ext` 存在，一并改。
`twoCell` 不认 `cx` / `cy`（出现报 `InvalidField`），必须给 `to`（右下角 A1）；可选 `toOffX` / `toOffY`（EMU，非负，省略则留下当前偏移）。不按列宽行高去换算 EMU。
docx / pptx 上出现 `to` / `toOffX` / `toOffY` 报 `InvalidField`。pptx：写入 `p:spPr/a:xfrm/a:ext`。`id` 可以是图 id、视频 id 或音频 id。找不到该对象报 `IdNotFound`。
批注内图没有画布 `xfrm`，对本 op 报 `InvalidId`。`setMedia` 仍不改几何。

### setOleSize

改已有嵌入对象的预览框尺寸。不启动 OLE 服务器，不重算嵌入文件，不改 embeddings 字节。

字段与 `setPictureSize` 相同，`id` 必须是 OLE id。`cx` / `cy` 与 `to` 不能同时出现。只改被点名的那一侧，另一侧保持原值。
docx 写入 `v:shape` 的 `style` 宽高（EMU 按 12700 换成 pt）。若有 `wp:extent` 或带 `cx` 的 `a:ext`，一并改。`w:dxaOrig` / `w:dyaOrig` 若有，按 635 EMU 一缇同步。
xlsx 的 `objectPr/anchor` 是 `twoCell` 时必须给 `to`，可选 `toOffX` / `toOffY`。同时改 dummy 锚点的 `to`，以及 VML `x:Anchor` 的右下角单元格。不按列宽行高换算 EMU。`oneCell` / `absolute` 改 `xdr:ext`。
pptx 写入 graphicFrame 的 `p:xfrm/a:ext`、两支 `p:oleObj` 的 `imgW` / `imgH`，以及 Fallback 预览 `p:pic` 的 `a:ext`。不改 `a:off`。
对图 id 发此 op、对 OLE id 发 `setPictureSize` 报 `InvalidId`。
docx / pptx 上出现 `to` / `toOffX` / `toOffY` 报 `InvalidField`。
找不到该对象报 `IdNotFound`。

`setPictureWrap` 见 [docx](json-contract-docx.md#setpicturewrap)。
`setPictureAnchor` 见 [xlsx](json-contract-xlsx.md#setpictureanchor)。
`setPicturePos` 见 [pptx](json-contract-pptx.md#setpicturepos)。
`setInkCanvasSize` / `setInkCanvasWrap` 见 [docx](json-contract-docx.md#setinkcanvassize)。
`setModel3dSize` 字段与 `setPictureSize` 相同，`id` 必须是 3D 模型 id。
`setModel3dCamera` 见 [docx](json-contract-docx.md#setmodel3dcamera)。三家同一套字段。
docx 改宿主 `wp:extent`（以及 `a:ext` 若有），规则与图相同；Fallback 海报 drawing 同步改尺寸。
xlsx 规则与图相同：`oneCell` / `absolute` 改 `xdr:ext`，`twoCell` 改右下角 `to`。
pptx 写入 graphicFrame 的 `p:xfrm/a:ext`；Fallback 海报 `p:pic` 同步改尺寸。
`replaceModel3d` 仍不改几何。对图 id 发此 op、对 3D id 发 `setPictureSize` 仍报 `InvalidId`。
`setModel3dWrap` 见 [docx](json-contract-docx.md#setmodel3dwrap)。
`setModel3dAnchor` 见 [xlsx](json-contract-xlsx.md#setmodel3danchor)。
`setModel3dPos` 见 [pptx](json-contract-pptx.md#setmodel3dpos)。
`setSlicerSize` 字段与 `setPictureSize` 的 xlsx 一侧相同，`id` 必须是切片器 id（含时间线）。
`setSlicerAnchor` 字段与 `setPictureAnchor` 相同。docx / pptx 上发这两条报 `InvalidField`。
`setSlicerView` 改 slicer / timeline 视图部件上的标题、行高、列数、显示开关与样式名。docx / pptx 上发此 op 报 `InvalidField`。
`setSlicerFilter` 改切片器勾选或时间线筛选区间。docx / pptx 上发此 op 报 `InvalidField`。
见 [xlsx](json-contract-xlsx.md#setslicersize)。
xlsx 的 `setMathSize` 字段与 `setPictureSize` 的 xlsx 一侧相同；`setMathAnchor` 字段与 `setPictureAnchor` 相同。
pptx 的 `setMathSize` 字段与 `setShapeSize` 相同；`setMathPos` 字段与 `setShapePos` 相同。
id 必须是公式框 id。`setMath` 仍禁止 `x` / `y` / `cx` / `cy` / `from` / `to`。
docx 上 `setMathSize` / `setMathAnchor` / `setMathPos` 报 `InvalidField`。
见 [xlsx](json-contract-xlsx.md#setmathsize)、[pptx](json-contract-pptx.md#setmathsize)。

### insertTextBox

``json
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
``

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
 / `setTextBoxDirection` / `setTextBoxTextAnchor` 见 [docx 分册](json-contract-docx.md#settextboxwrap)。

### insertTable

``json
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
``

- **`op`**
  - 必须是 `"insertTable"`
- **`id`**
  - 见 [docx 表 id](json-contract-docx.md#稳定-id) 与 [pptx 表 id](json-contract-pptx.md#稳定-id)：docx 为段落（含框内）、`docx:document`、页眉页脚容器、文本框、表或单元格
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

写出带单线边框的 `w:tbl`，每列宽 1440 twip，每格至少一段并带新的 `w14:paraId`。
docx 插在单元格时，嵌套表放在该格 `tcPr` 之后、已有段落之前。格末仍保留一个 `w:p`。
Word 16.0 在文本框单元格里插表另存也是这个顺序；把等宽嵌套表接到格尾会报无法读取。
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
  - docx：段落 id（正文、页眉页脚或框内）写 `w:hyperlink`；或图 / 文本框 / 连接线 / 无框形状 / SmartArt / 图表 / 墨迹画布 / 3D 的 id 写 DrawingML `a:hlinkClick`（含框内图、注内 / 批注内的图或框）
  - xlsx：工作表 id `sheet:{name}`（单元格链），或图 / 文本框 / 形状 / SmartArt 图示 id（DrawingML 链）
  - pptx：文本形状、图、表、图表、SmartArt 或表单元格 id（不含 `notesMaster` / `handout`）
- **`text`**
  - docx 段落链必填，显示文本，字符串，可空
  - docx DrawingML 对象上出现报未知字段
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
  - xlsx 单元格链必填
  - DrawingML 链禁止出现
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
  - docx 段落链写 `w:tooltip`；DrawingML 链写 `a:hlinkClick/@tooltip`
  - xlsx 写 `@tooltip`
  - pptx 写在对应 `a:hlinkClick` / `a:hlinkHover` / `a:hlinkMouseOver` 的 `tooltip`
- **`display`**
  - xlsx 可选
  - 写 `@display`
  - docx / pptx 上出现报 `InvalidField`

docx：段落链在该段已有内容之后插入一个 `w:hyperlink`，内含一个带 `text` 的 run。
外部链在该部件 `.rels` 上新建 Type 以 `/hyperlink` 结尾的关系，`TargetMode` 为 `External`，`Target` 为 `url` 原文。内部链写 `w:anchor`，不写关系。
`url` 与 `anchor` 同时出现或一个都不给报 `InvalidField`。`url` 为空串报 `InvalidField`。`anchor` 为空串或以 `_` 开头报 `InvalidField`。
找不到段落报 `IdNotFound`。id 不是段落 id 且不是本期 DrawingML 对象 id 报 `InvalidId`。
DrawingML 链：`id` 是图、文本框、连接线、无框形状、SmartArt、图表、墨迹画布或 3D 的 id（含框内图、注内 / 批注内的图或框）。禁止 `text`。`url` 与 `anchor` 必须恰好一个。
图写入 `wp:docPr/a:hlinkClick`（没有 `docPr` 则写该 `pic` 的 `cNvPr`；组合子图写该 `pic` 的 `cNvPr`）。
文本框、连接线、无框形状写入 `wps:cNvPr/a:hlinkClick`（没有则写外层 `wp:docPr`）。
SmartArt、图表、墨迹画布、3D 写入该 drawing 的 `wp:docPr/a:hlinkClick`（没有 `docPr` 则写该对象的 `cNvPr`）。
关系写在该**故事部件** `.rels`。外部链 `TargetMode=External`。内部书签 `Target` 为 `#{name}`，不写 `TargetMode`。
同一对象已有 `hlinkClick` 时报 `InvalidField`（改地址请用 `setHyperlink`）。
对 OLE id 报 `InvalidId`。
xlsx / pptx 字段 `location` / `ref` / `on` / `event` 在 docx 上出现报未知字段或 `InvalidField`。

xlsx：单元格链在该表 `<hyperlinks>` 末尾追加一条 `<hyperlink>`。没有该元素则按 `CT_Worksheet` 顺序插在 `dataValidations` 之后、`printOptions` 之前。
`ref` 能解析则规范化成 `A1` 或 `A1:B2`；非法报 `InvalidRange`。与同一张表上已有链的单元格相交报 `InvalidField`。
给了 `url`：工作表 `.rels` 新建 Type 以 `/hyperlink` 结尾的关系，`TargetMode` 为 `External`，`Target` 为 `url` 原文，并写 `r:id`。
给了 `location`：写 `@location`。只给 `location` 不写关系。`url` 与 `location` 一个都不给报 `InvalidField`。
`url` 或 `location` 为空串报 `InvalidField`。工作表根上补 `xmlns:r`（若还没有）。不写单元格值，也不改已有格子的样式。id 不是工作表 id 且不是图 / 文本框 / 形状 / SmartArt id 报 `InvalidId`。
找不到工作表报 `IdNotFound`。单元格链只能写 `worksheet`；对 `chartsheet` 发工作表 id 报 `InvalidField`；对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。
DrawingML 链：`id` 是图 / 文本框 / 形状 / SmartArt 图示 id。禁止 `ref` / `display`。`url` 与 `location` 必须恰好一个。
写入该对象 `cNvPr/a:hlinkClick`，关系写在 **drawing** `.rels`。外部链 `TargetMode=External`。内部位置 `Target` 为 `#{location}`，不写 `TargetMode`。
同一对象已有 `hlinkClick` 时报 `InvalidField`（改地址请用 `setHyperlink`）。
不标重算。`create` 给了 `sheets[].hyperlinks` 时按单元格链同一套规则写 `<hyperlinks>`。

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
create 上 `slide` 是页序整数，见 [json-contract-pptx.md](json-contract-pptx.md#create)。
create 给了 `pictures[].hyperlink` / `tables[].hyperlink` / `charts[].hyperlink` / `smartArts[].hyperlink` 时按同一套规则写到对应对象上。表单元格上的 `url` / `file` / `slide` 同样写入。

### setHyperlink

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setHyperlink"` |
| `id` | 超链接 id。docx 为 `link:{n}`、容器前缀，或图 / 文本框 id；xlsx 为 `sheet:{name}/link:{n}` 或图 / 文本框 / 形状 / SmartArt 图示 id；pptx 为 `slide:{n}/link:{k}` |
| `url` | 可选。改成外部 URL。xlsx 为 `null` 则去掉 `r:id`。pptx 为 `null` 或空串报 `InvalidField` |
| `file` | pptx 可选。改成外部文件目标。xlsx / docx 上出现报 `InvalidField` |
| `slide` | pptx 可选。改成内部跳转 `slide:{n}`。xlsx / docx 上出现报 `InvalidField` |
| `anchor` | docx 可选。改成内部书签名。xlsx / pptx 上出现报 `InvalidField` |
| `location` | xlsx 可选。写 `@location`；`null` 去掉该属性。docx / pptx 上出现报 `InvalidField` |
| `text` | docx 可选。改显示文本。xlsx / pptx 上出现报 `InvalidField` |
| `tooltip` | 可选。`null` 表示去掉提示 |
| `display` | xlsx 可选。写 `@display`；`null` 去掉该属性。docx / pptx 上出现报 `InvalidField` |
| `ref` | xlsx 可选。改覆盖区域。docx / pptx 上出现报 `InvalidField` |

docx：段落链：`url`、`anchor`、`text`、`tooltip` 至少给一个。`url` 与 `anchor` 不能同时出现。
给了 `url`：有已有 `r:id` 则改该关系 `Target`；没有则新建关系并写 `r:id`，去掉 `w:anchor`。给了 `anchor`：写 `w:anchor`，去掉 `r:id`，该关系无人引用则删除。
`text` 只改该 `w:hyperlink` 内可见 `w:t`（规则与 `setParaText` 第 2–4 条相同，范围限于该元素）。`tooltip` 为字符串则写属性；为 `null` 则去掉属性。
找不到报 `IdNotFound`。id 不是超链接 id 且不是本期 DrawingML 对象 id 报 `InvalidId`。
DrawingML 链：禁止 `text`。`url`、`anchor`、`tooltip` 至少给一个。`url` 与 `anchor` 不能同时出现。
改 `a:hlinkClick` 的关系与 `tooltip`。`url` 与 `anchor` 都去掉后必须 `deleteHyperlink`。找不到对象或该对象没有链报 `IdNotFound`。
对 OLE id 报 `InvalidId`。

xlsx：单元格链：`url`、`location`、`tooltip`、`display`、`ref` 至少给一个。
给了 `url` 字符串：有已有 `r:id` 且该关系 Type 以 `/hyperlink` 结尾则改 `Target`；否则新建关系并写 `r:id`。`url` 为 `null`：去掉 `r:id`，该关系无人引用则删除。
给了 `location` 字符串则写属性；为 `null` 则去掉属性。改完之后必须仍有 `r:id` 或 `location`，否则报 `InvalidField`。
`ref` 非法报 `InvalidRange`；与其它链相交报 `InvalidField`。`tooltip` / `display` 为字符串则写属性；为 `null` 则去掉。不改单元格值。找不到报 `IdNotFound`。
id 不是单元格超链接 id 且不是图 / 文本框 / 形状 / SmartArt 图示 id 报 `InvalidId`。单元格链只能改 `worksheet`。
DrawingML 链：禁止 `ref` / `display`。`url` 与 `location` 最多给一个。改 `a:hlinkClick` 的关系与 `tooltip`。
`url` 与 `location` 都去掉后必须 `deleteHyperlink`。找不到对象或该对象没有链报 `IdNotFound`。

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

docx：段落链拆掉 `w:hyperlink` 外壳，留下内部 `w:r`。该 `r:id` 在本部件 XML 里无人引用则删除关系。
DrawingML 链去掉 `a:hlinkClick` 和无人引用的 hyperlink 关系，不删对象。
找不到报 `IdNotFound`。id 不是超链接 id 且不是带链的本期 DrawingML 对象 id 报 `InvalidId`。
对 OLE id 报 `InvalidId`。

xlsx：单元格链去掉该 `<hyperlink>`。该 `r:id` 在本表其余链里无人引用则删除关系。删光则去掉 `<hyperlinks>`。不改单元格值。
DrawingML 链去掉 `a:hlinkClick`，drawing `.rels` 无人引用则删除关系。不删图或形状。
找不到报 `IdNotFound`。id 不是单元格超链接 id 且不是带链的图 / 文本框 / 形状 / SmartArt id 报 `InvalidId`。
单元格链只能改 `worksheet`。

pptx：去掉该条链元素（`a:hlinkClick`、`a:hlinkHover` 或 `a:hlinkMouseOver`）。形状级从 `p:cNvPr` 摘掉；run 级从 `a:rPr` 摘掉，`a:r` 与 `a:t` 留下。该 `r:id` 在本页 XML 里无人引用则删除关系。不改可见文字。
找不到报 `IdNotFound`。id 不是超链接 id 报 `InvalidId`。

示例见 `docs/examples/docx-apply-hyperlink.json`、`docs/examples/docx-apply-drawing-hlink.json`、`docs/examples/pptx-apply-hyperlink.json`。

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
`create` 给了 `sheets[].comments` 时按同一套规则写线程化批注，字段用 `cell` 代替 `id`；给了 `replies` 再按 `replyComment` 写一层回复，不要再套 `replies`。pptx 不认 `start` / `end`；出现这两个字段报 `InvalidField`。
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
Word 16.0 把空的 `wpi:ink` 插进另存批注后再保存，标记会被丢掉。`Replies.Add` 的另存同样丢掉，所以不写回复。
xlsx 的 `id` 必须是父批注容器 id，不能是回复 id 或单元格 id；对回复 id 报 `InvalidId`。写入同部件一条带 `parentId` 的 `threadedComment`，`ref` 与父相同。
新回复 `id` 为 `{父 id}/reply:{guid}`。只能写 `worksheet`。
pptx 的 `id` 必须是父批注容器 id（`slide:{n}/comment:{guid}`），不能是回复 id；对回复 id 报 `InvalidId`。写入该 `p188:cm` 的 `p188:replyLst`。
新回复 `id` 为 `{父 id}/reply:{guid}`。找不到父批注报 `IdNotFound`。对 `kind` 为 `ink` 的批注发此 op 报 `NotImplemented`。
PowerPoint 16.0 保存时丢掉注入的空 `ink`。`Replies.Add` 能写出普通回复，父批注里的 `ink` 不在另存结果里，所以不写回复。

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
对 `kind` 为 `ink` 的批注报 `NotImplemented`，没有可见 `w:t` 时同样拒绝，不写文件。
Word 16.0 改 `Range.Text` 后再另存，空的 `wpi:ink` 也会被丢掉，所以不改字。
xlsx 的 `id` 可以是批注容器 id 或回复 id。
只改对应 `threadedComment` 的 `<text>`。pptx 的 `id` 可以是批注容器 id 或回复 id。只改对应 `p188:txBody` 的可见文本，规则与 `setShapeText` 相同。
对 `kind` 为 `ink` 的批注报 `NotImplemented`，没有可见 `txBody` 文本时同样拒绝。
PowerPoint 16.0 的批注 `Text` 只读，另存也会丢掉空的 `ink`，所以不改字。

### setCommentBox

- **`op`**
  - 必须是 `"setCommentBox"`
- **`id`**
  - xlsx 顶层批注 id，`cell:{name}!{A1}/comment:{guid}`
- **`visible`**
  - 可选布尔。`true` 写成 `visibility:visible`，`false` 写成 `visibility:hidden`
- **`width`**
  - 可选正数，磅
- **`height`**
  - 可选正数，磅

只用于 xlsx。按批注格的行、列找到 `ObjectType="Note"` 的 `v:shape`，不看 `shapeId`。
`visible`、`width`、`height` 至少给一个，否则报 `InvalidField`。宽或高不是正数报 `InvalidField`。
`visible` 为真时写上 `x:Visible`，为假时去掉。改宽高时改样式，并按 72 像素一列、20 像素一行挪 `x:Anchor` 的右下角。
回复 id 报 `InvalidId`。批注在、框不在，报 `IdNotFound`。不改文字、作者和线程。
docx 与 pptx 不认这个 op。

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

`type` 为 `other` 时，除 `deleteChart`、`setChartSize`、`setChartPos`、`setChartWrap`、`setChartAnchor` 外一律报 `NotImplemented`，不写文件。
`type` 为 `chartex` 时，可以改标题、图例和系列，也可以增删系列：

- `setChartSeries` 改 `cx:pt`，并按 `cx:f` 写 xlsx 单元格或 docx / pptx 的 embedding
- `addChartSeries` 再写一条同样 `layoutId` 的系列，数值落到下一列
- 帕累托和箱线的条数固定，增删报 `NotImplemented`
- 删到只剩一条报 `LastChartSeries`
- 瀑布图汇总点、树状图层级留在同一条系列里
- 趋势线、误差线、换类型、轴标题、数据标签仍报 `NotImplemented`
- `regionMap` 不插入。Excel 16.0 的图表类型接口不接受地图图表，没有另存出地理缓存
xlsx 没有 `setChartWrap` / `setChartPos`，对应例外是 `deleteChart`、`setChartSize`、`setChartAnchor`。
pptx 没有 `setChartWrap` / `setChartAnchor`，对应例外是 `deleteChart`、`setChartSize`、`setChartPos`。
`flags` 含 `secondaryAxis`、`trendline`、`errorBars` 不再整张锁死。
`flags` 含 `pivot` 时：`setChartSeries`、`addChartSeries`、`deleteChartSeries`、`setChartType` 报 `NotImplemented`；
装饰 op、删图、尺寸、位置、绕排、锚点，以及趋势线 / 误差线 / 次轴 op 可以。
`type` 为 `combo` 不再整张锁死，系列留在原来的包裹里改。`type` 为 `stock`、`surface` 或 `ofPie` 时：`setChartSeries` 与装饰 op 可以。
`addChartSeries`、`deleteChartSeries` 仍报 `NotImplemented`（股价必须保持与 `stockKind` 一致的条数，曲面必须保持三条，复合饼必须保持一条）。
`setChartType` 可以在系列条数对得上时转入或转出这三种（见该 op）。`insertChart` / create 接受 `stock`、`surface`、`ofPie` 与带 `layoutId` 的 `chartex`。
已写入的 `chartex` 可以改标题、图例和系列数据。xlsx 上发 `setChartWrap` / `setChartPos` 报 `InvalidField`。
docx 上发 `setChartAnchor` / `setChartPos` 报 `InvalidField`。pptx 上发 `setChartWrap` / `setChartAnchor` 报 `InvalidField`。`insertChart` 不能插入透视图表。

docx 改系列数据时必须同时改 chart 部件里的 cache（`c:numCache` / `c:strCache` / `c:numLit` / `c:strLit`）以及 embedding 工作簿里公式指向的单元格。
没有 embedding 则只改字面量。不在库里求值公式，不改 `c:f` 里工作表名以外的结构，除非新数据的点数与公式区域长度不同：那时把公式区域扩到或收到新长度，并改 embedding 里对应格子。未点名的系列不动。

xlsx 没有 embedding。有可解析的连续 A1 `c:f` 时：把数组写入这些格子（按 `setCell` 语义），点数变了就按原起点扩或收公式区域，并回写 cache。
`c:f` 是工作簿 `definedNames` 能解析成**一段**连续 A1 的名字（可带表名前缀）时，按解析结果写入格子；
新值长度与该区域格子数不同则报 `NotImplemented`，不改定义名本身，也不把 `c:f` 改成 A1。
`c:f` 是逗号并集的多段连续 A1 时：新值长度与各段格子总数相同则按文档顺序装入，不改 `c:f`；长度不同、或某一段不是连续 A1，报 `NotImplemented`。
`c:f` 是三维引用 `表1:表2!区域` 时：两端按工作簿里工作表的顺序，区域内先行后列。新值个数与这些格子数相同则写入格子，`c:f` 保持原样，并回写旁边的缓存。个数不同报 `NotImplemented`，不写格子。
`c:f` 是定义名，且公式能求成一块或几块格子时同样写入，包括 `OFFSET`、`INDEX`，以及名字里面的运算。`c:f` 仍是这个名字，不改定义名。求出来是一个数、不是格子区域的，报 `NotImplemented`。
重算时，上述公式也会按格子重写旁边的 `numCache` 或 `strCache`。三维引用里整张空表按 0 补进缓存。求出来是一个数的名字不改缓存。
没有 `c:f`（只有 lit）时只改 lit / cache，不新造单元格区域。写入单元格时走与 `setCell` 同一套重算标记。未点名的系列不动。

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
  - 单个单元格地址，例如 `A1`
  - 不要写范围（`A1:E19`），不要写表名（`Sheet!A1`）
  - 不是图表去引用哪一块已有单元格
  - `series` 里的分类和数值按这个格子当左上角写进工作表，挡住的格子会被盖掉
  - 分类写在这一列的下一行起
  - 每个系列的名字写在这一行、向右逐列
  - 数值写在名字下面
  - 气泡图每个系列占两列
  - 已经手写了表格时，指到空区域，不要指到那张表的表头
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
inspect 先按 `cx:f` 读单元格，含 `_xlchart.v1.N` 这类定义名；读不到再用 `cx:lvl`。
`refs` 为真时，`catRef` / `valRef` / `nameRef` 给 `cx:f` 原文。缓存和单元格都有且对不上时给 `cacheStale`，规则与普通图表相同。
Word 与 PowerPoint 16.0 改数据后另存仍是直接区域并保留 `cx:lvl`，内嵌工作簿不写定义名。Excel 16.0 会改成隐藏定义名并去掉 `cx:lvl`。
Excel 另存时，瀑布、树状、旭日、箱线的 Choice 是 `Requires="cx1"`。漏斗是 `cx2`，直方和帕累托是 `cx7`。
Word 另存漏斗也是 `cx2`。PowerPoint 另存直方和帕累托是 `cx7`。
库进入 `cx1`、`cx2`、`cx7`。`cx4`（地图）不进。
Fallback 里的海报形状或图片不进 `shapes` / `pictures`。
箱线系列上 Excel 会补 `cx:statistics quartileMethod="exclusive"`。`setChartSeries` 留着这段，也留下 `AlternateContent` 外壳。
docx 写 chartex 时，`w:drawing` 包在 `mc:AlternateContent` 的 Choice 里，前缀在 Choice 上声明。
漏斗用 `Requires="cx2"`，其余布局用 `cx1`，与 Word 16.0 粘贴 Excel 原生图后另存的前缀一致。Fallback 写成空的 `mc:Fallback`，不写海报。
不包外壳时 Word 16.0 一另存就崩溃。只有 Choice、没有 Fallback 时，Word 另存会拆掉外壳，下一次另存同样崩溃。
xlsx、pptx 仍写裸图形框，Excel、PowerPoint 另存正常。
本库以前写出、没有外壳的 docx 图，apply 不去补。要让 Word 另存不崩，先 `deleteChart` 再重新 `insertChart`。
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

``json
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
``

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

改 DrawingML 文本框的字号、字色、字填充、字轮廓、加粗、斜体、下划线、中英文字体、水平对齐、框内垂直锚点、段落间距与字符间距。不改可见字。不要用 Word 的半磅字号。

``json
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
``

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
| `textAnchor` | 可选。`t` / `ctr` / `b` / `just` / `dist`（`ST_TextAnchoringType`），或 `null` 去掉 `bodyPr/@anchor`。不能叫 `anchor` |
| `spcBef` / `spcAft` / `lnSpc` | 可选。段前、段后、行距。对象 `{ "type": "pts" 或 "pct", "val": 整数 }`，或 `null` 去掉对应子元。`pts` 写 `a:spcPts`（百分之一磅，0–158400）。`pct` 写 `a:spcPct`（整数，0 起；100000 为 100%） |
| `spc` | 可选。字符间距 `a:rPr/@spc`，百分之一磅，可负（-400000–400000）。`null` 去掉属性 |
| `kern` | 可选。字距调整 `a:rPr/@kern`，百分之一磅非负整数（0–400000）。`null` 去掉属性 |

除 `op` 与 `id` 外至少给一个字段，否则报 `InvalidField`。
`sz` 小于 100 报 `InvalidField`；大于 400000 报 `TruncatedLimit`。
`color` 与 `textFill` 同时出现报 `InvalidField`。
`textFill` 为对象时必须有 `type`：`none` 或 `gradient`。不要写 `solid`（实心仍用 `color`）。
`type` 为 `gradient` 时必须有 `stops`，至少两项。每项必须有 `pos`（0–100000）与六位 `color`。
`pos` 重复报 `InvalidField`。可选 `ang`（0–21600000），省略则写 `5400000`（Office 16.0 水平渐变）。
写入 `a:gradFill`：`flip="none"` `rotWithShape="1"`，`a:lin scaled="1"`，空 `a:tileRect`。不写 `a:path`。不写 theme 色。
`textOutline` 为对象时必须有 `color`（六位 RGB）与 `width`（正整数 EMU）。
写入 `a:ln/@w` 与 `a:ln/a:solidFill/a:srgbClr`，插在填充子元之前。不写 `prstDash` / `miter`。不写 `schemeClr`。
`spcBef` / `spcAft` / `lnSpc` 为对象时必须有 `type` 与 `val`。`type` 不是 `pts` / `pct`、或 `pts` 的 `val` 越界，报 `InvalidField`。
`spc` / `kern` 越界报 `InvalidField`（超出 schema 上限报 `TruncatedLimit`）。
不写 `a:prstTxWarp`，不写 `a:effectLst`。功能区艺术字不新开对象。
改该 `txBody` 里每个文本 `a:r` 的 `a:rPr`，并同步 `a:endParaRPr`。段落间距写每个 `a:pPr`。不改 `a:fld`。不重写 `a:lstStyle`。
各 run 不一致时 inspect 对应字段为 `null`。各段间距不一致时 `spcBef` / `spcAft` / `lnSpc` 为 `null`。
inspect 在 `color` 之后始终带 `textFill`、`textOutline`；在 `textAnchor` 之后始终带 `spcBef`、`spcAft`、`lnSpc`，再跟 `runs`。
`textFill`：各 run 都是 `a:noFill` 则为 `{ "type": "none" }`；都是可读的线性 `a:gradFill`（仅 `srgbClr`）则报该对象；否则为 `null`。
`textOutline`：各 run 都有同一套 `a:ln/@w` 与 `srgbClr` 则报 `{ "color", "width" }`；否则为 `null`。
`runs` 按文档顺序列出每个文本 `a:r`。每条字段顺序：`text`、`sz`、`color`、`textFill`、`textOutline`、
`bold`、`italic`、`underline`、`font`、`eastAsiaFont`、`spc`、`kern`。
`text` 取该 run 的 `a:t`。不进 `a:fld`。不算排版引擎。
没有 `txBody` 报 `IdNotFound`。
对连接线 id 报 `InvalidId`。对 `sldImg` 占位符报 `InvalidField`。docx 上发此 op 报 `InvalidField`。
Word 行距仍走 `setParaFormat`。字距与 kern 见 docx 待做 62。
create 的 pptx `shapes[]`、xlsx 的 `textBoxes[]` / `shapes[]`，以及 `insertShape`、xlsx `insertTextBox`，认同一套可选字段。
pptx create 缺省仍是 18 磅黑字 Calibri / 宋体。xlsx 缺省仍是 `sz="1100"`。
xlsx 的 `anchor` 仍是单元格锚点，框内垂直对齐只用 `textAnchor`。
示例见 `docs/examples/pptx-apply-shape-format.json`、`docs/examples/xlsx-apply-shape-format.json`、
`docs/examples/pptx-apply-text-fill.json` 与 `docs/examples/xlsx-apply-text-fill.json`、
`docs/examples/pptx-apply-shape-spacing.json` 与 `docs/examples/xlsx-apply-shape-spacing.json`。

### setShapeRuns

按原文把形状或文本框里的可见文字拆成多个 run。`id` 与 `setShapeText` 相同。不改没点名的字，不重建形状。短的用 `match`。中间很长时不写 `match`，只写两个切点：`after` 是左切点，这一截从它后面开始；`before` 是右切点，这一截到它前面结束。两切点中间的原文单独成一截，中间不用抄出来。

``json
{
  "op": "setShapeRuns",
  "id": "slide:1/shape:2",
  "runs": [
    { "after": "前半", "before": "结尾", "color": "FF0000", "underline": "sng" }
  ]
}
``

xlsx 文本框和带文字的形状用同一条，id 例如 `sheet:Sheet1/sp:1`。docx 上发此 op 报 `InvalidField`。单元格、备注整段、讲义槽位、图表标题、SmartArt 点文字不走这条。

- **`runs`**
  - 至少一条
  - 要么每条都是 `match` 或一对切点，要么每条都是 `text`，要么每条都是 `start` 与 `end`
  - 三种不要混用
- **`match`**
  - 可见文字里的原文，不能是空串
  - 找不到报 `InvalidField`
- **`after` / `before`**
  - 不写 `match` 时必须成对
  - 这一截是二者中间的全部原文
- **`nth`**
  - 可选，从 1 起，不写就是接下来的第一次
- **`text`**
  - 全部连起来必须等于当前可见文字，否则报 `InvalidField`
- **`start` / `end`**
  - 字符下标，按 Unicode 标量值，含 `start` 不含 `end`
  - 必须从 0 起首尾相接
- **格式字段**
  - `bold`、`italic`、`color`、`underline`、`sz`、`font`、`eastAsiaFont`
  - 规则与 `setShapeFormat` 的 run 字段相同
  - `underline` 是 `sng` / `none` / `dbl`
  - 对齐、框内垂直、段间距仍用 `setShapeFormat`

`a:pPr`、`a:endParaRPr`、`a:fld` 留下。换行仍是 `\n`，对应 `a:br`。对不上报 `InvalidField`，一个字都不改。

## exportMedia

按图 id 把 OPC 里的媒体部件原样写到本地文件。docx 与 pptx 也认 OLE id，写出嵌入部件原字节。docx 也认 `font:{n}`，写出 fontTable 指向的 odttf 原字节（仍是混淆后的，不还原）。
三种格式也认 3D 模型 id，写出 GLB 原字节。docx 也认墨迹画布 id。
不读进 inspect 树，不做转码、缩放、子集化或预览。

``json
{
  "schemaVersion": 1,
  "action": "exportMedia",
  "path": "C:\\data\\note.docx",
  "id": "para:3F2A1B00/pic:1",
  "output": "C:\\out\\logo.png"
}
``

- **`path`**（是）
  - 源 docx、xlsx 或 pptx
- **`id`**（是）
  - 图 id，与 inspect 的 `pictures[].id` 相同
  - docx / xlsx / pptx 也可以是 OLE id，与 `oleObjects[].id` 相同
  - docx / xlsx / pptx 也可以是 3D 模型 id，与 `model3ds[].id` 相同
  - docx 也可以是墨迹画布 id，与 `inkCanvases[].id` 相同
  - pptx 也可以是视频 / 音频 id，与 `videos[].id` / `audios[].id` 相同
  - docx 也可以是嵌入字体 id，与 `embeddedFonts[].id` 相同
- **`output`**（是）
  - 本地目标路径
  - 缺了报 `MissingField`
  - 不得默认写成源 `path`，以免把图片字节盖掉文档
- **`format`**（否）
  - 可省略，从 `path` 扩展名推断

`id` 必须是图 id，或 docx / xlsx / pptx 的 OLE id，或三种格式的 3D 模型 id，或 docx 的墨迹画布 id，或 pptx 的视频 / 音频 id，或 docx 的 `font:{n}`。
对段落 id、工作表 id、单元格 id、表单元格 id、页眉页脚容器 id、`section:{n}`、
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

``json
{
  "schemaVersion": 1,
  "action": "exportChartData",
  "path": "C:\\data\\note.docx",
  "id": "para:3F2A1B00/chart:7",
  "output": "C:\\out\\chart.xlsx"
}
``

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `path` | 是 | 源 docx 或 pptx |
| `id` | 是 | 图表 id，与 inspect 的 `charts[].id` 相同 |
| `output` | 是 | 本地目标路径。缺了报 `MissingField`。不得默认写成源 `path` |
| `format` | 否 | 可省略，从 `path` 扩展名推断 |

`id` 必须是图表 id，含正文、页眉页脚、框内、注内与批注内带前缀的 id，以及 pptx 备注图表 id `slide:{n}/notes/chart:…`。对系列 id、段落 id、形状 id、图 id 报 `InvalidId`。
找不到该图报表 `IdNotFound`。没有 embedding 报 `OpcPartMissing`。关系指向外部资源同样报 `OpcPartMissing`，不下载 http。xlsx 本期报 `InvalidField`。

写出的是 embedding 部件的原字节。`contentType` 取自 `[Content_Types].xml`；没有则省略。

成功时 stdout 见 [errors.md](errors.md)。源包不写回。

## exportChartImage

按图表 id 把工作表或图表工作表上的簇状柱形图、簇状条形图、普通（非堆积、非百分比）折线图、带标记的散点图、普通（非堆积、非百分比）面积图或普通饼图画成 PNG，写到本地。只读。不改源包，不把像素放进 inspect。

``json
{
  "schemaVersion": 1,
  "action": "exportChartImage",
  "path": "C:\\data\\book.xlsx",
  "id": "sheet:Sheet1/chart:2",
  "output": "C:\\out\\chart.png"
}
``

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `path` | 是 | 源 xlsx 或 xlsm |
| `id` | 是 | 工作表图表 id，与 inspect 的 `charts[].id` 相同 |
| `output` | 是 | 本地目标路径。缺了报 `MissingField`。不得默认写成源 `path` |
| `format` | 否 | 可省略，从 `path` 扩展名推断 |

- 只画簇状柱形图，簇状条形图（`barChart` 且 `barDir` 为 `bar` 或省略、grouping 为 clustered 或省略），grouping 为 standard 或省略的 `lineChart`，scatterStyle 为 marker 或 lineMarker（缺省按 lineMarker）的 `scatterChart`，grouping 为 standard 或省略的 `areaChart`，以及恰好一条系列的 `pieChart`。系列、类目和系列名优先用图里的 `numCache`、`strCache` 或字面量。没有这些缓存时，公式必须是本簿一张表上的连续区域或单个格子，点用格子里已有的缓存。不算公式。区域对不上，或两边都没有缓存，报 `NotImplemented`
- 宽高跟 Excel 16.0 的 `Chart.Export`。`twoCell` 按起止格的列宽和行高累加，不用占位的 `a:ext`
- 每一列、每一行先换成 96 dpi 的像素，再乘工作表 `zoomScale`（缺省 100）后四舍五入，然后相加
- `oneCell` 和 `absolute` 仍用 `a:ext` 的像素，再按同一个缩放四舍五入
- 没有 `defaultRowHeight` 时，行高用 14.25 磅（19 像素）。写了就用写下的磅值
- 宽或高小于 1 或大于 4096 报 `NotImplemented`
- 没有系列填充时，包里有 theme 就用 theme 的 accent。没有 theme 时用 Excel 16.0 的 Office 主题 accent。系列上的 `solidFill` 优先。不套用 `c:style`
- 轴、网格、图例按图里的位置画。图里没写字体时，字号用 10 磅。标题加粗，轴、类目和图例不加粗
- 外框是 1 像素圆角，半径 14，颜色 `898989`。圆角外面留白
- 类目轴主刻度在类目分界上，向上 3 像素、向下 4 像素。次刻度在类目正中，向上 2 像素、向下 3 像素
- 类目刻度在柱之后画。向上的那一截落在柱里时，盖住柱
- 类目槽从数值轴起到网格右端的下一列。这一段按类目等分，正好落在两个像素中间时取偶数
- 柱用同一段宽度。缺省空隙是柱宽的 150%，一组柱在类目槽里居中
- 数值轴主刻度是网格伸出轴线的 3 像素。相邻主刻度之间再画 4 道次刻度，向轴线两侧各 2 像素
- Windows 上用 GDI+ 的 AntiAliasGridFit 画「等线」。没有这套字时依次试 DengXian、微软雅黑、Calibri。别处用 fontdue 的 Calibri
- 字形的灰度可以和 Excel 不同。字的位置和黑框大小跟 Excel 对齐
- 纵轴自动刻度用 1、2、5 的步长。数据超过这一档上限的 95% 时，再加一个步长。正数且能含 0 时，下限钉在 0
- 数值轴的横坐标是 9，加上最宽刻度的字宽，再加上 11。字宽是 10 磅的 GDI 前进宽度，向上取整
- 缩放 100 时，一位数刻度的轴在 x=27，「4.5」在 x=37，「1400」在 x=48。不设固定下限
- 图例修复：有效 `r/l/t/b/tr` 位置不静默丢失。没有 legend 表示关闭；缺失或畸形的可选位置、数值按缺省处理，不强制生成图例。
- 图例继承 chartSpace 与 legend 的普通 `txPr` 字体、字号、粗体、斜体和不透明 sRGB/accent 文字色（lumMod/lumOff/shade/tint）；非 accent 主题色、未支持的透明/颜色变换、段落/文本框配置、命名字体不可用、未解析主题字族、富文本/装饰等有效但未实现的配置明确报未实现，不伪装成异常缺省。
- `overlay` 决定是否预留绘图区；普通 manualLayout 支持 chart 分数尺寸与 edge/factor 坐标。factor 是相对默认位置，不是绝对坐标；手工框按原生 Office 行为保留自动布局的预留量。
- 图例 key 与间距随文字 em/空间度量，条目按可用宽度排布，宽手动框把剩余空间分给外侧与条目间隙，不把同一窄组仅搬到中央；普通 legendEntry.delete 可隐藏条目。有效复杂背景/entry文字覆盖等不静默忽略，暂报未实现。
- 旧缺省右图例仍保留已有像素基线。新样式布局修复不代表 106 DPI 官方图与库 96 DPI 整图像素验收已过。
- 右侧图例的系列名整段留在图内。名字比缺省右边距宽时，绘图区右缘向左让。短名字仍用原来的边距
- 这段留白用 GDI+ 默认排版宽度，截掉小数。字形前进宽度更窄，用它会把类目槽撑宽，柱越往右越偏
- 面积图按与折线相同的类目槽公式定位，用不透明纯色填到数值轴交叉（`autoZero` 为零，零在轴外时取较近端）。正值在交叉之上、负值在下，相邻异号在交叉处剖开。系列按 `order` 先画后盖。缺值按缺口断开填充，不跨空点相连。有写出的实线描边才画上沿线，不默认画标记。填充带透明、堆积、百分比堆积、三维、可见数据标签、空值按零或连线、标记、组合图报 `NotImplemented`。官方 Excel 16.0 `Chart.Export` / `Slide.Export` 对照尚未做，不把本片自测当成官方通过。
- 普通饼图只画单系列 `pieChart`，没有坐标轴。`firstSliceAng` 缺省为 0，按从正上方起顺时针计量，360 与 0 相同。只给有限正值扇区，角度按正值之和比例分配；空值跳过扇区与图例项，零值不占角度但仍可出现在图例。`varyColors` 省略时视为开启，扇区按 accent 循环，`dPt` 上的不透明纯色可覆盖；关闭时用系列纯色。图例只画缺省右侧类目名，或没有图例。饼是绘图区内接圆。负值或没有有限正值、多系列、三维饼、圆环、复合饼、非零爆炸、可见数据标签、空值按零或连线、半透明或非纯色扇区、可见扇区描边、非缺省右侧类目图例报 `NotImplemented`。官方 Excel 16.0 `Chart.Export` / `Slide.Export` 对照尚未做，不把本片自测当成官方通过。
- 不是上述已支持的类型，或类目图的 `crossBetween` 不是 `between`，报 `NotImplemented`
- docx 与 pptx 报 `InvalidField`
- 找不到图报 `IdNotFound`。id 不是图表 id 报 `InvalidId`
- 成功时 `contentType` 为 `image/png`。源包不写回
- 像素差在绘制引擎里修。对照只标红区，不靠放宽容差过关

## exportSparklineImage

按迷你图组 id 把这一组里的一条折线、非负柱形或盈亏（`stacked`）画成 PNG，写到本地。只读。不改源包，不把像素放进 inspect。图里只有这条线或这些柱，没有格子的文字、填充和边框。

``json
{
  "schemaVersion": 1,
  "action": "exportSparklineImage",
  "path": "C:\\data\\book.xlsx",
  "id": "sheet:Sheet1/spark:1",
  "output": "C:\\out\\spark.png"
}
``

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `path` | 是 | 源 xlsx 或 xlsm |
| `id` | 是 | 迷你图组 id，与 inspect 的 `sparklines[].id` 相同 |
| `output` | 是 | 本地目标路径。缺了报 `MissingField`。不得默认写成源 `path` |
| `format` | 否 | 可省略，从 `path` 扩展名推断 |

- 一组里只能有一条迷你图，落点只能是一个格子。落在合并单元格里报 `NotImplemented`
- 宽高是这个格子的列宽和行高，缺省 96 dpi。列宽与工作表共用同一套几何：按请求 `dpi`（缺省 96）重测 MDW 后套公式，Calibri 11 缺省列在 96 仍是 64、在 106 是 72，不再把 96 像素按 `round(64*dpi/96)` 放大成 71。迷你图宿主行高仍用布局的 `round(emu*dpi/914400)`（与工作表 `excel_row_px` 的先 96 截断再放大不是同一条路径，20 磅等自定义行可能差 1 像素）；未写行高的 Calibri 11 与 等线 11 均为 14.25 磅（96 下 19 像素），不再把 Calibri 11 当成 15 磅/20 像素
- 宽或高小于 1 或大于 4096 报 `NotImplemented`
- 点用包里已有的格子缓存，不算公式。公式不是本簿一块连续区域时报 `NotImplemented`
- 空格子按 `displayEmptyCellsAs`。缺省当 `zero`。`gap` 断开，`span` 跨过空格连线，`zero` 当 0
- 横轴把绘图区左右各留 2 像素，剩下的宽度按点数等分，点在每一份的正中
- 纵轴上沿是 5 像素，下沿是高度减 4 的那一行。最小值和最大值取这条线自己的数。两者相同，线画在上沿
- `minAxisType` / `maxAxisType` 缺省是 `individual`。`group` 在这一组只有一条时与它相同。`custom` 用 `manualMin` / `manualMax`，缺了或最小值不小于最大值时报 `NotImplemented`。超出轴端的数收到轴端
- 线色用 `colorSeries` 的 `rgb`。没有这层时用 `376092`。只有主题色、没有 `rgb`，或带透明，报 `NotImplemented`
- 线宽缺省 0.75 磅，按请求 DPI（缺省 96）换成像素
- 柱形在每个点都是有限非负数、没有空格子时可以画。柱是纯色，没有抗锯齿。色用 `colorSeries` 的 `rgb`，没有这层时用 `376092`。只有主题色、没有 `rgb`，或带透明，报 `NotImplemented`
- 柱形横向：格子宽减去 5 后按点数等分，槽从第 3 列像素起。第 i 根柱（从 0 起）占这一槽正中的三分之二。左缘是 `floor(3 + (i + 1/6) × 槽宽)`，右缘含端点是 `ceil(3 + (i + 5/6) × 槽宽) - 1`。右缘正好落在整数上时，这一列不画
- 柱形纵向和折线同一段：上沿是 5，下沿含端点是高度减 4。绘图高是高度减 8。最小值和最大值取这些数自己。全部相同且大于 0 时每根柱都铺满。全部是 0 时不画。其余柱高是 `floor((值 - 最小) / (最大 - 最小) × (绘图高 - 1)) + 1`，底边贴下沿。值是 0 且最小值也是 0 时这一根不画，槽仍占着
- 柱形不读自定义轴。有负数、空格子、横坐标、从右到左，或宽小于 6、高小于 9，报 `NotImplemented`
- 柱形的高点、低点、首点和末点换成对应颜色的纯色。同一根柱同时命中时，按低点、高点、首点、末点的顺序留第一种。并列的极值每根都换色。没有写出颜色时用 `D00000`。只有主题色、没有 `rgb`，或带透明，报 `NotImplemented`
- 柱形上的 `markers` 不改变像素。负值着色打开但每个点都不是负数时，也不改变像素
- 盈亏（`type=stacked`）：横向与柱形同一套槽公式；纵轴上沿 5、下沿 `height-4`，绘图高对半，正值铺上半、负值铺下半；`0` 与空槽不画。正色用 `colorSeries`（元素缺省时回落 Excel 新建缺省 `376092`；有元素无 `rgb` 则 NI），`negative` 为真时负色用 `colorNegative`（元素缺省回落 `D00000`；有元素无 `rgb` 则 NI），否则负值仍用系列色。paint 不写死样本色。`displayEmptyCellsAs` 只认 `gap`/`zero`（空都不画）；`span`、自定义轴、`displayXAxis`、`rightToLeft`、首末高低标记报 `NotImplemented`。日期轴与折线、柱形共用下面这条规则。Excel 16.0 CopyPicture 金样（2026-11-17 Asia/Shanghai）对 `+/-` 实心色与半高分界有独立证据；整图 73×22@~106 vs lib 64×19（本机 defaultRowHeight 14.3）sizeDiffer→pxBlocked，不改金样/DPI
- `dateAxis` 打开时，组上的 `xm:f` 必须是与数值等长的单行或单列有限日期序列（Excel 序列数）。等距日期仍按分类轴同一套槽公式定位，与关闭日期轴像素一致；不等距日期按 `(date-min)/(max-min)` 把最小与最大放到连续左右沿（96 dpi 各留 12 像素，再按请求 dpi 四舍五入，右沿为 `width-1-left`），不再映射到分类槽。不排序；反向日期可以从右画到左。`n=1` 时即使日期相同也落在第 0 槽。至少两点但全部日期相同、日期缺失或非数值、长度不合、二维区域报 `NotImplemented`，不回退成等距分类轴。`dateAxis` 关闭时忽略组上的日期公式。隐藏日期格子与数值同一套 `displayHidden`。折线、非负柱形和盈亏在原有子集内都按此定位；柱宽仍按点数和格子宽计算，日期很近时柱可以重叠，不凭单图把官方 15px 柱宽写成通用公式。`insertSparkline` / `setSparkline` 本片不写 `dateAxis` 或日期公式。
- 折线上的标记、高点、低点、首点、末点、负值、横坐标、从右到左，报 `NotImplemented`
- 源格子落在隐藏的行或列上，且没有 `displayHidden`，报 `NotImplemented`。`displayHidden` 为真时，隐藏格子的缓存照样画
- docx 与 pptx 报 `InvalidField`
- 找不到组报 `IdNotFound`。id 不是迷你图组 id 报 `InvalidId`
- 成功时 `contentType` 为 `image/png`。源包不写回
- 折线的抗锯齿可以和 Excel 差几个像素。差多少记在手测里，不靠加大容差算通过

## exportSheetImage

按工作表 id 把已用区域画成 PNG，写到本地。只读。不改源包，不把像素放进 inspect。图里是格子。没有描边或仅有 1/2 像素实线描边、没有字的纯色矩形可以画。不透明 PNG 在目标框正好等于要贴的像素时可以画，整像素裁剪（含裁后再放大到目标）也画。同一单元格里、变换是恒等的组合，里面只有这种矩形时也画。其它压到已用区域上的图形让整表失败。

``json
{
  "schemaVersion": 1,
  "action": "exportSheetImage",
  "path": "C:\\data\\book.xlsx",
  "id": "sheet:总览",
  "output": "C:\\out\\sheet.png"
}
``

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `path` | 是 | 源 xlsx 或 xlsm |
| `id` | 是 | 工作表 id，与 inspect 的 `sheet:{name}` 相同 |
| `output` | 是 | 本地目标路径。缺了报 `MissingField`。不得默认写成源 `path` |
| `format` | 否 | 可省略，从 `path` 扩展名推断 |

- 范围取 `dimension` 和有格子的外框的并集。只有一边时用那一边。两边都空报 `NotImplemented`
- 可选 `tilePx`（1–4096）：按该边长切块导出，**不**先分配整表再裁。未超限时仍一张 PNG，像素与不加 `tilePx` 一致；超限时写出多张 `{stem}-r{row}c{col}.png`，响应 `paths` 列出全部路径，单块边长不超过 `tilePx`。不设 `tilePx` 时一边超过 4096 仍整表 `NotImplemented`（默认不绕上限）。`rightToLeft` 切块报 `NotImplemented` 切块张数上限 1024、全图合计像素上限 64Mi；超限在编码/落盘前 NotImplemented（`tilePx=1` 不留下海量文件）。中途落盘失败时已写出的分块文件可能残留。 超限切块的官方对照（2026-11-18）：有界 65 列×16 行、网格关、Calibri 缺省列。Excel 16.0 窗口 DPI 106、Zoom=100 的全表 `Range.CopyPicture`（`xlScreen=1, xlBitmap=2`）成功为 4681×337（`1+65×72` 宽、`1+16×21` 高），未超内存。当时库 `tilePx=1024` 缝合 96=4161×321（`1+65×64`、`1+16×20`）、106=4616×353（`1+65×71`、`1+16×22`），反映旧的「先按 96 测 MDW 再比例放大」以及 Calibri 11 未写行高按 15 磅。本次改为按请求 dpi 重测 MDW、未写宽度用 `8×MDW+8`、行高先按 96 截断再放大、Calibri 11 未写行高改 14.25 磅后，库 106 期望与当时官方全表 4681×337（`1+65×72`、`1+16×21`）同尺寸，库 96 高度由 `1+16×20=321` 变为 `1+16×19=305`（宽度 4161 不变）。这是授权的缺省 96 几何修正，不是把旧失败改写成早已通过。须用当时合格源包做库回归，不以缝合差 0 冒充官方验收。内部块只在全表官方图上取窗才与库瓦片同一坐标系；对内部子范围再 `CopyPicture` 会在该子范围原点新画 `leading_frame`，不得与全表窗口或库内部块直接逐像素等同。库内部块不再画左框，与全表窗口一致。不以库内缝合差 0 冒充官方验收，也不把本对照写成整图通过。
- `iconSet` 条件格式：cfvo/reverse/showValue/gte **选择语义**在库内可测；**出图仍 NotImplemented**（无公开可合法复现标准图标字形前不绘制）。最小锁定：`3Arrows` / `3ArrowsGray` / `3TrafficLights1`。不声称完整图标集或与 Excel 差 0。
- 公式差分边框 `dashed` / `slantDashDot`：与格子边框共用绘制；`expression`+dxf 可出图。`cellIs` 带边框、差分对角线仍 NotImplemented。整图与 Excel CopyPicture 若屏 DPI≠96 可能 sizeDiffer，不以缩金样宣称差 0。
- 像素对齐 Excel 16.0 的 `Range.CopyPicture`（`xlScreen`、`xlBitmap`）。不乘工作表 `zoomScale`
- 工作表、切块、迷你图及图表 twoCell 锚点共用同一套列几何。MDW 按请求 `dpi`（缺省 96）用公开 GDI 重测，再套列宽公式，不把 96 dpi 像素按 `dpi/96` 放大；不读取本机显示 DPI，也不改系统 DPI。非 Windows 无法重测时，仅普通 Calibri 11 在非 96 dpi 下退回旧的 96 像素再 `scale_px` 路径，不把本机 106 写成常量。
- 先按 `builtinId=0` 找 Normal 样式，再回退英文名 Normal 和字体 0。列宽的 MDW 是这个字体、字号及粗斜体在该导出 dpi 下数字 0–9 的最大前进宽，不取格子的局部字体。
- 显式 `col/@width` 和 `defaultColWidth` 都直接按 `floor(((256 * width + floor(128 / MDW)) / 256) * MDW)` 换像素，不经过 UI 两位小数字符宽，也不重复加 5 像素。这里的 MDW 是导出 dpi 下的值。
- 没写上述宽度时，缺省是 `8×MDW+8` 像素（8 个字符宽再加 8 像素）。Calibri 11 / MDW=7 在 96 dpi 仍为 64；MDW=8 为 72；MDW=10 为 88（不再用 `floor(8.43×MDW+5)` 的 89）；MDW=11 为 96（不是 97）。`16.7109375` 在 MDW=7 仍为 117 像素。写入路径的 `setColWidth` 仍按规范示例 MDW=7 存 OOXML，导出按实际 MDW 解释已存值。
- Windows 用公开 GDI 按该 dpi 的像素字号测量，并核对实际字体名；缺字体或测量失败报 `NotImplemented`，不静默改成 MDW=7。非 Windows 仅对规范已知的普通 Calibri 11 使用 MDW=7，其它 Normal 字体暂不出图。
- 图表 zoomScale 与行高四舍五入、工作表行高先 96 截断再放大的既有路径区别仍在。请求 `dpi: 106` 后，图表 `a:ext` 按 `round(EMU*106/914400)` 取画布（例如 5486400×3200400 EMU 为 636×371），twoCell 锚点的列宽改用该 dpi 的 MDW（Calibri 11 缺省 72，不再是 71）。官方对照以尽力接近的几何/颜色/抗锯齿及实测尺寸报告，不再把 differ=0 当唯一完成门。这是 2026-11-18 用户明确放宽，不是把旧失败改写成已通过。
- 写了行高时，先按 96 dpi 截断 `floor(round(磅 * 12700) / 9525)`，再按请求 dpi 四舍五入放大。96 dpi 时 20 磅是 26，不是 27；14.25 磅在 106 dpi 是 21 不是 20，23 磅在 106 dpi 是 33 不是 34。
- 没写行高时，先取这一张表的缺省，再用格子里的字加高
- 没写 `defaultRowHeight` 时，缺省是 14.25 磅，19 像素（96 dpi）
- 写了 `defaultRowHeight` 但 `customHeight` 不是真时，不采用这个数字。缺省改用常规样式的行高。常规样式是 Calibri 11 或 等线 11 时，这一档都是 14.25 磅
- `customHeight` 为真并且写了 `defaultRowHeight` 时，缺省就是这个磅值。这一行的字如果还是常规字体，不再加高
- 要加高时，Calibri 11 磅与 等线/DengXian 11 磅一样按 14.25 磅（官方 2026-11-18 COM StandardHeight 与未写行高的 CopyPicture 均为 14.25；96 dpi 为 19 像素，106 dpi 为 21 像素）。这会改变缺省 96 出图里原先按 15 磅/20 像素计算的 Calibri 11 自动行高，属授权修正，不把历史失败改写成早已通过。微软雅黑、宋体、黑体、楷体、仿宋（及 Microsoft YaHei / SimSun / SimHei）仍按字号乘 1.65。等线/DengXian 其它字号仍用字号×15/11（COM 12→15.75、18→23 不单例拟合）。其余按字号乘 15/11。一行里取最高的。不把雅黑改离 1.65×
- 合并盖住的行没有自己的格子时，仍按锚点的字加高。锚点被隐藏也一样。没被合并盖住、也没有格子的行保持上面的缺省
- 宽和高是各格像素之和再加 1。这一像素是外框
- 网格色是 `E0E0E0`。纯色填充盖住格子右缘和下缘那条网格。右边和下边的格子后画，盖住交界。上边和左边的外框最后仍画成网格色
- `sheetView` 的 `showGridLines` 为 `0` 或 `false` 时不画这些网格。上边和左边的外框留白。宽高不变。没写这个属性，或写成真，仍画网格。边框仍画在原来的像素上
- 行 `hidden` 为真时，这一行不占像素，填充、文字和边框都不画。写了行高也一样。列 `hidden` 为真时，这一列不占像素。隐藏列里的字仍参与这一行的自动行高
- 已用区域里每一行都隐藏时，高度是 1，整行留白，不画网格。每一列都隐藏时，宽度是 1，整列留白。行和列都隐藏时是 1×1 的白
- 合并跨过隐藏行或隐藏列时，按剩下的可见格子铺满锚点。锚点自己被隐藏、合并仍伸进可见格子时，照样铺
- 条件格式的区域含隐藏行或隐藏列时，整表报 `NotImplemented`
- `sheetView` 的 `rightToLeft` 为真，且没有条件格式、没有自动筛选时：无文字则整图水平翻转；有拉丁/CJK 文字时先翻转非文字层，再在翻转后的格子坐标用同一套 `Face::width`/`draw_cells` 正写（不 Unicode 反序、不镜像字形）。`alignment/@readingOrder` 为 `2`，或文本含强 RTL 文种（阿拉伯/希伯来等）时仍整表 `NotImplemented`（与列镜像不同语义；逐字前进宽不能冒充 bidi shaping）。条件格式或自动筛选在 RTL 下仍 `NotImplemented`
- 文字认左、中、右和常规。常规时数字靠右，文字靠左，布尔与点名错误值居中。垂直认上、中、下。左缘留白是该字体平均字宽的一半，右缘留 2 像素。字与字的间距用 GDI 前进宽度，不跟整段字符串的排版步进
- `wrapText` 为真或单元格文本含硬换行 `\n` 时按单元格内宽软断多行；`indent` 0–15，每级缩进为 3 个空格宽（与绘制同一套 `Face::width(" ") * 3`，不用 Word 排版步进）。多行块按垂直对齐整体摆放。多行行距用该字体自动行高像素（等线/Calibri 11 在 96 dpi 为 19、106 dpi 为 21），不是 GDI `tmHeight`；行内基线按 `sheetFormatPr` 的 `dyDescent`（缺省 0.25）落在行盒内 `1-dyDescent` 处。顶对齐在行顶加 2 像素；底对齐的行盒贴齐下一行原点，不再额外减 1。这是布局合同，不要求与 ClearType 灰阶逐像素一致。水平 `justify`：无软换行时按左对齐（含数字/布尔）；软换行后同一段非末行按词间等间隙拉齐，末行仍左对齐。水平 `distributed`：至少两词按等间隙左右顶齐。`justify` 上的 `indent`、`justifyLastLine`、一词 `distributed`、`fill`、`shrinkToFit` 仍 `NotImplemented`。有字格子上的非 0 `textRotation`：1–90 按逆时针、91–180 按顺时针（`90-value，180 为 -90°`）旋转 typographic 框，把转完后的外接框按水平/垂直对齐放到与未旋转相同的左内缘和顶对齐 typographic 顶；255 堆积把每个正立字水平居中在 `tmHeight` 宽的列里，第一字与未旋转顶对齐同一行盒，后续字按自动行高向下排并裁切到格子。带换行、缩进、`distributed` 或 `justifyLastLine` 的旋转仍 `NotImplemented`。这是按官方 `Range.CopyPicture`（`pr10-official-review-20261118-001710`）锁定的布局合同，不要求与 ClearType 灰阶逐像素一致。不照搬 Word 行距或断词。
- 单元格 `t="e"` 且 `<v>` 为 `#NULL!` / `#DIV/0!` / `#VALUE!` / `#REF!` / `#NAME?` / `#NUM!` / `#N/A` 时按该串画字（常规对齐居中，同布尔）。其它错误码报 `NotImplemented`。共享弦里的同形文字仍是文字、仍靠左，不是错误值。
- 数字格式认常规、`0`、`0.0`、`0.00`、`#,##0`，以及同样位数的百分数。格式码 `General` 不区分大小写。内置 14–22 与点名 `yyyy/mm/dd`（及去转义后的同义码 `m/d/yy`、`d-mmm-yy`、`h:mm AM/PM` 等）按 Excel 1900/1904 序列与英文月缩写画显示串；未点名日期码与方括号 locale/颜色仍 `NotImplemented`
- 格式码只有分号、分成两段到四段且每段都空时，数字不画。四段时文字和布尔也不画。两段、三段仍画文字和布尔。无方括号的货币/多段（含 OOXML `\$` `\(` `\)` 转义、会计 `_` 跳宽、正/负/零选段、引号字面量段）按 Excel 选段规则画成与 `Range.Text` 一致的串；带方括号的颜色/条件格式仍报 `NotImplemented`。未知骨架不静默当 General
- 公式有 `<v>` 或内联结果时用这份缓存，共享公式的从属格也一样。没有缓存时用库里的求值，包括 `IF` 的文字。算不了报 `NotImplemented`
- 合并单元格按锚点的样式铺满。合并区域跨出已用区域报 `NotImplemented`
- 数据条：无 x14 时只认最小/最大两档，长度从列内可用宽度的 10% 到 90% 并向白色淡出，区域内有负数报 `NotImplemented`。有 x14 `dataBar` 且 `gradient=0`、`axisPosition=middle`、`minLength=0`/`maxLength=100`、cfvo 仅 min/max、无 border、方向缺省或 LTR 时画实心条：格内中轴，正右负左，长度按 `|v|/max(|values|)` 占半宽；正色取 ECMA `color/@rgb`；`negativeBarColorSameAsPositive` 为假（或省略属性但仍有 `negativeFillColor`）时负色取 `negativeFillColor/@rgb`，否则同正色；轴色取 `axisColor/@rgb`（缺省黑）；零值只画轴。其它 axisPosition/cfvo/border/direction、主题色-only、带负数的 x14 渐变条报 `NotImplemented`。不把 PPT/单元格渐变参数搬进数据条
- 色阶认两档或三档。百分位用 `PERCENTILE.INC`，颜色按 RGB 混合
- `cellIs` 没有 `dxfId` 时不改像素。有纯色 `dxf` 填充时盖住格子
- `expression` 的公式按规则区域左上角平移后再求值。结果非 0 时，有纯色 `dxf` 填充就盖住格子，区域内没有 `<c>` 的空位也铺。结果为 0、文字或错误时不铺。求不了报 `NotImplemented`
- `timePeriod` 用规则里的 `<formula>`，按 `expression` 同一套平移后再求值。命中且差分是纯色时铺上该色，区域内没有 `<c>` 的空位也铺。没有公式或求不了报 `NotImplemented`。这条规则的差分边框仍报 `NotImplemented`
- 同一条命中的公式规则，差分边框是 `thin`、`medium`、`thick`、`dotted`、`hair`、`double`、`dashDot`、`dashDotDot`、`mediumDashed`、`mediumDashDot`、`mediumDashDotDot` 时，按格子边框的像素画。没写的一侧仍用格子自己的边。优先级数字更小的那条差分边框生效，不把两条差分的边拼在一起。区域内没有 `<c>` 的空位也画。`expression` 的 `dashed` 与 `slantDashDot` 差分边框按格子边框画；`cellIs` 带边框和斜线的差分边框仍报 `NotImplemented`
- `aboveAverage` 只数区域内的数字，空格不算。高于平均是严格大于。`equalAverage` 为真时含等于。`aboveAverage` 为 0 时看低于平均。`stdDev` 用总体标准差，平方和除以个数。数字少于两个又写了 `stdDev` 时不铺。命中且差分是纯色时铺上该色
- `top10` 的 `rank` 缺省是 10。`percent` 为真时，个数乘 `rank` 再除以 100，向上取整，至少 1。并列时阈值上的格子都算。`bottom` 为真时取最小的那些。命中且差分是纯色时铺上该色
- `containsText`、`notContainsText`、`beginsWith`、`endsWith` 忽略大小写。`notContainsText` 也铺到区域内没有 `<c>` 的空位。命中且差分是纯色时铺上该色。这几类规则的差分边框仍报 `NotImplemented`
- `duplicateValues` 只数有值的格子，没有 `<v>` 的格子不参与。数字按数值相等，格式不影响。文字忽略大小写。能看成数字的文字按数值并进数字，包括前后空格、千分位逗号、小数、科学计数、百分号和括号里的负数。百分号前面可以有空格。含有数字但对不上这套写法的文字报 `NotImplemented`。空字符串和空格按文字比，互不相等。布尔只和布尔比。命中且差分是纯色时铺上该色。差分填充没写 `patternType`、只有 `bgColor` 时也当纯色。这条规则的差分边框仍报 `NotImplemented`
- `uniqueValues` 用同一套比较。这个值只出现一次时铺纯色。没有值的格子不参与，也不铺。对不上写法的数字文字同样报 `NotImplemented`。差分边框仍报 `NotImplemented`
- `containsBlanks` 的公式必须是区域左上角的 `LEN(TRIM(锚点))=0`。`notContainsBlanks` 的公式必须是同一锚点的 `LEN(TRIM(锚点))>0`。函数名和单元格引用不区分大小写，公式可以有前导等号。`=0` 把没有 `<c>`、没有值，以及去掉首尾普通空格后为空的文字铺成纯色。数字和布尔不算空白。`>0` 铺剩下的格子，没有 `<c>` 的空位不铺。错误值上这条公式得到错误，两种都不铺。没有公式时不铺。公式是别的写法时报 `NotImplemented`。这两条的差分边框仍报 `NotImplemented`
- `containsErrors` 的公式必须是区域左上角的 `ISERROR(锚点)`。`notContainsErrors` 的公式必须是 `NOT(ISERROR(锚点))`。函数名和单元格引用不区分大小写，可以有前导等号。`ISERROR` 只铺 `t="e"` 的格子。`NOT(ISERROR)` 铺其余格子，没有 `<c>` 的空位也铺。没有公式时不铺。公式是别的写法时报 `NotImplemented`。错误文字的字色和底色不同时，整表报 `NotImplemented`。字色和底色相同则不画这几个字。这两条的差分边框仍报 `NotImplemented`
- 文字能按上面这套规则看成数字时，格子左上角内侧画 6×6 的直角三角，颜色是 `008000`。三角盖住填充和边框。没有数字的文字不画。含有数字但对不上写法时，没有重复值规则就不画三角，整表照常画
- 自动筛选在表头右侧画 17×17 的按钮。箭头水平居中，尖端落在按钮正中。表头文字停在按钮左边
- 字色、填充色和边框色认 `rgb`、主题色和索引色。主题色下标依次是 `lt1`、`dk1`、`lt2`、`dk2`、六个强调色、超链接、已访问的超链接，取工作簿主题里的颜色。没有主题部件时用 Office 默认色。`auto` 当黑色。下标对不上报 `NotImplemented`
- `tint` 从 -1 到 1。负值变暗，正值变亮。按 Excel 的 240 级 HLS 改亮度，亮度先截成整数再转回 RGB。离整百分比不超过 `1/32767` 的值按那个百分比算
- 边框认 `thin`、`medium`、`thick`，宽度是 1、2、3 像素。也认 `dashed`、`dotted`、`hair`、`double`、`dashDot`、`dashDotDot`、`mediumDashed`、`mediumDashDot`、`mediumDashDotDot` 和 `slantDashDot`
- 没写颜色时用黑色。线以格子边界为中线。宽度是偶数时，多出的一像素落在坐标较小的一侧。线只伸到实际有边的那一端，不沿合并区扩开。落在合并区内部的边不画。同一条边界上更粗的线盖住更细的线。同样粗时，右边缘盖住左边缘，其余保持格子的先后
- 底边比左边那一格的底边更细时，左端才让开。左边是虚线就让开它的全部厚度，左边是实线就让开一半。2 像素虚线在边界那一行再向右涂 1 像素。上面那一格有底边，或格子自己有上边时，竖线不涂上边界。两边都没有时，竖线从上边界涂起
- 虚线按整张图的坐标取余。`dashed` 用 x 加 y 取余，余数 3 时空一像素，周期 4；横线和竖线相同，不再对竖线做 y 减 1。其它线型仍是横线用 x、竖线用 y。`dotted` 在余数 0 和 3 上涂，周期 4。`hair` 在偶数坐标上涂
- `dashDot` 的周期是 18，余数 0 到 8、12 到 14 涂。`dashDotDot` 的周期是 24，余数 0 到 8、12 到 14、18 到 20 涂
- `mediumDashed`、`mediumDashDot`、`mediumDashDotDot` 和 `slantDashDot` 是 2 像素，占边界和坐标较小的那一像素。前三种两行用同一套余数。`mediumDashed` 的周期是 12，余数 0 到 8 涂。后两种分别用 `dashDot` 和 `dashDotDot` 的余数
- `slantDashDot` 的边界那一行按周期 18，余数 0 到 9、12 到 15 涂。紧挨的较小坐标那一行，余数 1 到 11、13 到 17 涂
- `double` 不涂边界那一像素，涂它两侧各 1 像素
- 图案和纯色铺在同一块矩形上，盖住格子右缘和下缘。点的位置用整张图的像素坐标，含左边和上边那一像素外框，不从格子左上角重算
- 从右到左的翻转发生在铺完图案之后，点跟着整张图一起翻
- 图案类型认 `gray125`、`gray0625`、`lightHorizontal`、`darkHorizontal`、`lightVertical` 和 `darkVertical`
- 图案类型也认 `lightGrid`、`darkGrid`、`mediumGray`、`lightGray`、`darkGray`、`darkDown`、`lightDown`、`darkUp`、`lightUp`、`darkTrellis` 和 `lightTrellis`
- `fgColor` 是点，`bgColor` 是底。没写前景时用黑色，没写背景时用白色。颜色仍认 `rgb`、主题色和索引色
- `rgb` 写成 8 位、前两位又不是 `FF` 时，这一格图案不认，整表报 `NotImplemented`
- 没有描边、没有字的纯色矩形可以画。`prst` 是 `rect`，`a:ln` 只有 `noFill`，正文没有文字。填充是不透明的 `srgbClr`，或没有颜色变换的 `schemeClr`。`schemeClr` 的颜色取主题部件里这一档，`sysClr` 用 `lastClr`。没有主题部件，或带了 `lumMod`、`tint`、`shade`、`alpha`，报 `NotImplemented`
- 锚点的 EMU 除以 9525 后四舍五入。`oneCell` 的终点是起点单元格加上偏移和 `ext`。`twoCell` 的终点是 `to` 标记。图形含起点，不含终点
- 单元格锚点（`oneCell` / `twoCell`）的 `colOff` / `rowOff` 超出当前格的像素宽高时，原点钳在该格远边，不把超出的 EMU 累加到后续列或行。未超出时仍按请求 dpi 把该格内偏移四舍五入为像素。`oneCell` 的目标框尺寸按未钳位的 EMU 差计算，可以从钳位后的原点伸出格外。`twoCell` 的 `to` 偏移同样钳在目标格内；同一格内 from/to 都顶到远边时只留下该边上的 1 像素，不按 oneCell 的 ext 伸出。若未钳位框仍与不透明 PNG 1:1，官方把该图缩小进残余框（不是裁掉多出来的原图像素）；一般缩小仍失败整表。`absoluteAnchor` 不受格尺寸钳位：原点按 `pos` 四舍五入，远边按 `pos+ext` 四舍五入。15 磅行上 `rowOff=381000` EMU 在 106 dpi 落在 y=22，而不是 44
- 上边和左边的外框盖住图形。图形盖住格子、网格和边框。后画的盖住先画的
- 不透明 PNG 可以画。没有 `a:srcRect` 时，目标框宽高必须等于源图像素，按原像素贴。`a:srcRect` 四边落到整像素时：裁完宽高等于目标则原样贴；裁完宽高均不超过目标（可再放大）时，Windows 上用公开 GDI+（与幻灯片同源的 `scale_device_rgba_window`：源矩形画在原图上，相位用真实目标原点）放大后再贴。半像素百分比、缩小、一边放大一边缩小、无裁纯放大、带透明通道、`fillRect` 有偏移、旋转、翻转、描边、不是 PNG，报 `NotImplemented`。本机屏 DPI≠96 时 CopyPicture 可能 sizeDiffer，不以缩金样宣称差 0
- 组合可以绕中心转 90 度的倍数，也可以水平或垂直翻转。子矩形先绕自己的中心转 90 度的倍数，再按组的外框和子坐标框做整数比例，除法去掉余数，再翻转，再旋转。转完的子矩形要落在组的子坐标里。坐标从工作表左上角量起。组外框与子矩形的 EMU 按请求 `dpi` 换算（省略或 96 时与除以 9525 四舍五入相同）。锚点外接矩形要和转完、翻完之后的组外框按像素重合。里面的这种矩形可以跨格。子矩形的水平或垂直翻转不改变这块纯色，也可以和直角叠在一起。组合可以再套一层，换算从内到外。里层也可以绕中心转 90 度。里层和外层可以同时绕中心转 90 度。锚点组合还可以绕中心转非直角：先按同一套整数比例缩放并翻转，把未旋转外框对到 twoCell 外接框（官方把该外接框当作未旋转目的框，因此比旋转原始 xfrm 外框更肥），再绕中心顺时针旋转；在 Windows 上用公开 GDI+ `TranslateTransform` / `RotateTransform` / `FillRectangle`（抗锯齿和半像素偏移）画真实旋转矩形，不拿转完后的外接框铺色。直角倍数仍走已差 0 的整数路径。子矩形自己的非直角、里层组合的非直角、组合内描边或其它对象，以及 GDI+ 不可用，报 `NotImplemented`。工作表上未编组纯色矩形的其它角度仍报 `NotImplemented`。墨迹可以超出已写入的 twoCell。官方尺寸与库默认列宽行高仍不合，不可 raw 零差，不把本片自测当成用户验收
- 没有描边的纯色矩形可以绕中心转 90 度的倍数，也可以水平或垂直翻转。锚点上的外接矩形就是要铺的那一块。线写成 `noFill` 时，旁边的虚线、斜接和隐藏线不画。其它角度、有字、不是矩形和其它图形，报 `NotImplemented`。隐藏行或隐藏列碰到这种图形也报 `NotImplemented`
- 线宽恰好是 1 或 2 像素（9525 / 19050 EMU）的不透明实线描边也会画：`prst` 为 `rect`，轴对齐、无旋转无翻转；用与幻灯片相同的 GDI+ `DrawRectangle`（1 像素无像素偏移，2 像素半像素偏移，圆角接缝，平头线帽）叠在已画格子上。填充可以是不透明 `srgbClr` / 没有颜色变换的 `schemeClr`，或 `noFill`。`absoluteAnchor` 的原点按 `pos` 在请求 dpi 下四舍五入，远边按 `pos+ext` 四舍五入（尺寸是两者之差），与 `oneCell` 的未钳位 EMU 差相同，不是把 `ext` 单独四舍五入后再加到原点。组合内描边、其它线宽（含缺省非 9525/19050）、虚线、箭头、显式 `miter`/`bevel`/`round`、`algn` 不是 `ctr`、半透明填或线，整块形状仍 `NotImplemented`。本机屏 DPI≠96 时 CopyPicture 可能 sizeDiffer，不以缩金样宣称差 0
- 格子 `diagonalUp` / `diagonalDown` 且 `<diagonal style>` 为 `thin`（1 像素）或 `medium`（中心加四邻，约 3 像素）实线时画斜线，色取 `diagonal/color`。细线从格子上角网格 LineTo 到远边网格（down：`(x0,y0)→(x1,y1)`，up：`(x1,y0)→(x0,y1)`），含起点不含终点；medium 含终点后再加粗。允许落在远边网格 `x=x1` / `y=y1`；不得朝左/上渗进邻格；整表左/上外框 `x=0` 或 `y=0` 不落斜线墨。其它斜线线型、只有方向标志没有 style、条件格式斜线仍报 `NotImplemented`
- `fills` 里 `gradientFill`：`type` 缺省或 `linear`、恰好两停点、每停不透明 `rgb`（8 位且 A=`FF`，或 6 位）时，按 `degree`（缺省 0=左→右，90=上→下）做通道直线插值（与条件格式 colorScale 同 `mix`）。这是 SpreadsheetML 单元格渐变，**不是** DrawingML/PPT 形状的 `gamma_mix`/S 曲线；PPT 渐变证伪不拿来挡本档，也不照抄 PPT 曲线。Excel 16.0 ExportAsFixedFormat PDF 对 FF92D050→FF0070C0 条带 RMSE(sRGB)≈0.68、gamma2.2≈18.5。`type=path`、theme/indexed/auto/tint、A≠FF、停点不是两个，仍报 `NotImplemented`
- 认不出的图案、条件格式里的图案、图标集和其它还没画的条件格式，报 `NotImplemented`
- 宽或高小于 1 或大于 4096 报 `NotImplemented`
- docx 与 pptx 报 `InvalidField`
- 找不到工作表报 `IdNotFound`。id 不是 `sheet:{name}` 报 `InvalidId`
- 成功时 `contentType` 为 `image/png`。源包不写回
- 字形灰度可以和 Excel 不同。差多少记在手测里，不靠加大容差算通过

## exportSlideImage

按幻灯片 id 把这一页写成 PNG，写到本地。只读。不改源包，不把像素放进 inspect。

``json
{
  "schemaVersion": 1,
  "action": "exportSlideImage",
  "path": "C:\\data\\deck.pptx",
  "id": "slide:1",
  "output": "C:\\out\\slide.png"
}
``

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `path` | 是 | 源 pptx |
| `id` | 是 | 幻灯片 id，与 inspect 的 `slides[].id` 相同，形如 `slide:1` |
| `output` | 是 | 本地目标路径。缺了报 `MissingField`。不得默认写成源 `path` |
| `format` | 否 | 可省略，从 `path` 扩展名推断 |
| `dpi` | 否 | 本次栅格化的整数 DPI，24 到 600。省略或 96 时与现有 96 dpi 像素一致 |

- 宽高按 `p:sldSz` 的 EMU，按请求 DPI 四舍五入（`EMU × dpi / 914400`）。省略或 96 时与现有 96 dpi 像素一致。这是原生换算，不是先按 96 量化再放大，也不套用 Excel 的行高特例
- 宽或高小于 1 或大于 4096 报 `NotImplemented`
- 页底先用本页的 `p:bg`。本页没有时用版式，版式也没有时用母版
- `showMasterSp` 不改变页底。它只决定画不画母版和版式上的形状
- 不透明 `srgbClr` 直接铺色。`a:alpha` 不是 `100000` 时，这一层页底不认
- `schemeClr` 没有 `tint`、`shade`、`lumMod` 这类变换时，先按本页的 `overrideClrMapping` 取色
- 本页没有这张表时用版式的，再没有就用母版 `clrMap`。`bg1` 先换成表里的名字，再到主题里取 `srgbClr` 或 `sysClr/@lastClr`
- `bgRef` 的 `idx` 为 `0` 或 `1000` 时铺白，不再继承更下层
- `idx` 从 `1001` 起指向 `bgFillStyleLst`，`1` 到 `999` 指向 `fillStyleLst`。那一档必须是没有变换的 `phClr` 纯色，再用 `bgRef` 上的颜色代替
- 半透明、带颜色变换的主题色、渐变页底仍不认。这一层已经写了页底但画不成时铺白，不改用更下层
- 没有旋转、没有裁剪、没有平铺、没有 `fillRect` 偏移、拉伸铺满的不透明嵌入 PNG 页底，在 Windows 上按与图片放大同一套公开 GDI+（`InterpolationMode.HighQualityBicubic`、`WrapMode.TileFlipXY`、`PixelOffsetMode.Half`）画到整页，**包括源与页同尺寸的 1:1**，不走 `p:pic` 的逐字节拷贝。关系从拥有该页底的部件（本页、版式或母版）解析。官方 `pptx-preview-page-bg-image` 覆盖 1280×720 与 960×720 页、1:1 / 2× / 4×、版式与母版继承、矩形与图片遮盖，均为 `differ=0`、`maxDelta=0`（2026-11-17 03:59:14–04:00:04 Asia/Shanghai）。透明、平铺、裁剪、缩小或不透明条件不满足的图片页底铺白，不把 t93u 残差接受扩到页底。非 Windows 没有这条 GDI+ 对照路径，不声称与 Office 差 0
- 位置和尺寸用 `a:off` 与 `a:ext`，与页画布同一套请求 DPI。后画的盖住先画的，超出页面的部分裁掉
- `p:pic` 和形状 `blipFill` 里的嵌入 PNG。目标宽高都等于源像素时原样拷贝。没有 `srcRect`、且目标宽和高都不小于源时，Windows 上按 GDI+ `InterpolationMode.HighQualityBicubic`、`WrapMode.TileFlipXY`、`PixelOffsetMode.Half` 放大，与 PowerPoint 16.0 对齐。不透明图（全部 alpha=255）官方对照多种源尺寸、整数与非整数倍率、不均匀拉伸、形状填充和左缘裁切，均为 `differ=0`、`maxDelta=0`
- 带透明通道的放大在 Windows 上先按同一套插值，再叠到已画内容上。用户 t93u 验收范围只限当前图片放大及其透明度：位置、目标尺寸和边界必须与官方一致，色彩/alpha 混合可以有残差。官方 `pptx-preview-scale-image` 透明 2×2→4×4 页记录为 `differ=13`、`maxDelta=2`，几何锚点通过；不把这页写成像素差 0，也不放宽全局 MaxDelta。t93u 的残差接受不扩到渐变、半透明纯色矩形或其它绘制
- 非 Windows 没有这条 GDI+ 对照路径：不透明放大沿用历史最近邻，不声称与 Office 差 0；带透明像素的放大整页失败，避免静默用最近邻冒充成功
- `a:srcRect` 四边是从原图边缘向内的百分比。四边都落到整像素，并且裁完的宽高正好等于目标像素时，这块原图原样拷贝
- 四边落到整像素后，若裁完的宽和高都不大于目标（两边都不缩小），Windows 上把源矩形画在原图上，按同一套 GDI+ `InterpolationMode.HighQualityBicubic`、`WrapMode.TileFlipXY`、`PixelOffsetMode.Half` 放大，并按页面上的真实 `dest_x` / `dest_y` 取相位。这档是部分实现，尚未整体与 PowerPoint 16.0 差 0。不透明图官方对照多种源尺寸、裁剪边（左/右/上/下/中心）、整数与非整数倍率、不均匀放大、形状 `blipFill`，以及 `dest_y=0/64/66/96`，均为 `differ=0`、`maxDelta=0`。等像素整像素裁剪仍走原来的拷贝路径。未裁的放大仍走原来的紧凑目标位图，不改已验收路径
- 顶裁 4×3→8×6 在 `dest_y=65` 时，官方已切到高相位、本库 GDI+ 仍是低相位。官方 `pptx-preview-crop-scale-image` 该页原文为 `differ=34`、`maxDelta=2`（921600 像素）。这是未接受的不透明插值相位残差，不在 t93u 透明度放宽范围，不能当作本切片已通过，也不能靠套件 PASS 汇总掩盖。裁剪边界、定位和目标尺寸与官方一致，几何锚点通过。不把这一页写成像素差 0，也不放宽全局 MaxDelta=0。有限对照表明该残差只出现在这一几何的 `dest_y=65`，不是普遍奇偶 dest_y，也不随 dest_x 或其它顶裁倍率一起出现；不加坐标特判
- 透明裁剪后再放大沿用同一套插值后再 `over()` 的机制，只验收几何，不把 t93u 自动扩成零差声明。官方同一用例透明左上 2×2→4×4 页记录为 `differ=15`、`maxDelta=2`，几何锚点通过
- 百分比落在像素中间，或任一目标边小于裁完的源（缩小，或一边放大一边缩小），整页失败，未对照官方。平铺和 `fillRect` 偏移也不画。非 Windows 不透明裁剪放大沿用历史最近邻，不声称与 Office 差 0；带透明像素的裁剪放大整页失败。xlsx 工作表出图未接这条路径，目标尺寸不等于源时仍整页失败
- 实心矩形：`prst` 为 `rect`。没有描边，或线写成 `noFill` 时，旁边的虚线和空的效果列表不画。填充是不透明 `srgbClr`，或没有颜色变换的 `schemeClr`。`srgbClr` 带不是缺省、也不是 `100000` 的 `a:alpha` 时，按 8 位覆盖率把源和目标各自四舍五入后再相加，叠在已有像素上，与 PowerPoint 16.0 / GDI+ 默认 SourceOver 对齐；官方对照覆盖 10000 / 25000 / 50000 / 75000，以及多种独立填色、页底、重叠和左缘裁剪。`alpha` 为 0 时不涂色。线宽恰好是 1 或 2 像素（9525 / 19050 EMU）的不透明实线描边也会画，外角与 PowerPoint 16.0 对齐：用 GDI+ `DrawRectangle` 抗锯齿绘制，1 像素无像素偏移，2 像素半像素偏移，圆角接缝，平头线帽，叠在已有页底上。填充也可以是 `noFill`。其它线宽（包括默认 12700）、虚线、箭头、显式的 `miter` / `bevel` / `round`、`algn` 不是 `ctr`、带旋转的描边、半透明描边，或半透明填充再加描边，整块形状仍不画，页面继续成功
- `schemeClr` 按页底同一套配色表和主题取色。`sysClr` 用 `lastClr`。带了颜色变换，或主题里没有这一档，整页失败
- `flipH` 和 `flipV` 不改变这块纯色
- 矩形可以绕中心转 90 度的倍数。90 和 270 画成宽高对调后的外接矩形。没有描边、没有翻转、没有可见文字的不透明纯色矩形（`srgbClr` 或没有颜色变换的 `schemeClr`）在 Windows 上也可以按任意非直角角度绕中心顺时针旋转：走公开 GDI+ 平移到中心、`RotateTransform`、`FillRectangle`，加上抗锯齿和半像素偏移，画在真实页面坐标上，与 PowerPoint 16.0 差 0。官方对照覆盖 15 / 22.5 / 30 / 45 / 60 / 120 / 315 度、正方形与不等边、不同原点和填色，以及 90 度对照页。带翻转、描边、可见文字、半透明、渐变或图片填充的非直角矩形，以及组合和其它几何的非直角旋转，仍整页失败。非 Windows 没有这条 GDI+ 路径，这种矩形整块留空。t93u 不扩到旋转形状
- 两色 `srgbClr` 直线渐变画在没有旋转的矩形、圆角矩形和椭圆上。角度只认 90 度的倍数。这一档能画出，但尚未与 PowerPoint 16.0 差 0，保持 ⚠️，见 `.plan/2026年-11月-16日-⚠️-pptx-形状渐变收口.md`。t93u 不适用于渐变，对照仍是 `MaxDelta=0`，不把几何对齐写成像素通过
- 两档必须落在 0 和 100%。不透明两档的混合走现有 Office 曲线：通道先按伽马 2.2 展开，第二档的份额是一条对称的 S 形（17 点表），再编码回 sRGB，不是通道上的直线。官方 `pptx-preview-fill-shape-grad-px` 原文为 `differ=21103`、`maxDelta=6`（921600 像素）。独立无描边矩形样本的 `maxDelta` 在 1 到 6 之间，框外几何为 0。2026-11-17 有限核查了 GDI+ `LinearGradientBrush.SetSigmaBellShape(1, 1)` 和 ChristopherVR/ooxml `native-gradient-curve.ts` 的 256 项 sigma 表（社区容差 2–2.5），都不是差 0，未改现有曲线，未改金样，未放宽容差
- 两档可以带 `a:alpha`。半透明两色渐变不走上面那条不透明 S 曲线，而是在 sRGB 里按像素中心线性插值，alpha 按同一 `t` 线性混合，再一次叠到已经画好的页面像素上。官方 `pptx-preview-fill-alpha-grad-px` 原文为 `differ=480`、`maxDelta=1`。本轮纯色矩形的两步 SourceOver 不能通用修掉这个 max 1（在该官方页上差点更多）。独立宽度、alpha、颜色、方向和页底样本同样是 `maxDelta=1`，没有差 0 的子范围
- 矩形上的渐变仍不能带描边。圆角矩形和椭圆上的渐变可以带不透明描边。不要用渐变调色去补覆盖率；圆角纯色已差 0 的 `FillPath` 路径不扩展成渐变的零差声明
- 档位不在两端、不是直线 `a:lin`、颜色不是 `srgbClr`，或形状带了旋转，整页失败
- `p:cxnSp` 或形状上的 `line`、`straightConnector1`，在没有翻转、并且宽或高有一边的 EMU 是 0 时，画水平或垂直直线。描边是不透明 `srgbClr`，或 `p:style` 里 `lnRef` 的主题色
- 描边宽度四舍五入到 1 或 2 像素。2 像素是实心，不含终点那一像素，并往坐标较小的一侧加一像素。1 像素横线两端按约 75% 覆盖。1 像素竖线只涂一列，起点（坐标较小的一端）按约 5/8 覆盖，终点按约 7/8 覆盖。宽和高都不为 0 的不透明实线斜线，笔画像素四舍五入为 1 或 2 时也可以画，并可带 `flipH` / `flipV` 表示方向；抗锯齿与 PowerPoint 16.0 对齐。`bentConnector2` 与 `bentConnector3` 的不透明实线直角折线，笔画像素四舍五入为 1 或 2 时也可以画，并可带 `flipH` / `flipV`。`bentConnector3` 的转折取 `adj1`（单位是宽度的十万分之一，缺省 50000），转折点可以不是整像素。`bentConnector4` 与 `bentConnector5` 同样按 ECMA-376 预设多段直角折线来画：`adj1` / `adj2` 缺省 50000（分别相对宽度和高度），`bentConnector5` 再取 `adj3`（相对宽度，缺省 50000）；官方 `pptx-preview-elbow45-image` 多种尺寸、方向、`adj`、翻转和 1 / 2 像素均为 `differ=0`、`maxDelta=0`。`curvedConnector2` 与 `curvedConnector3` 的不透明实线弯曲连接符，笔画像素四舍五入为 1 或 2 时也可以画，并可带 `flipH` / `flipV`。几何按预设三次贝塞尔；`curvedConnector3` 的控制点取 `adj1`（单位是宽度的十万分之一，缺省 50000），控制点可以不是整像素。`curvedConnector4` 与 `curvedConnector5` 同样按 ECMA-376 预设多段三次贝塞尔来画：`adj1` / `adj2` 缺省 50000（分别相对宽度和高度），`curvedConnector5` 再取 `adj3`（相对宽度，缺省 50000）；官方 `pptx-preview-curve45-image` 多种尺寸、方向、`adj`、翻转和 1 / 2 像素均为 `differ=0`、`maxDelta=0`。其它宽度、虚线、箭头、非平头线帽或带旋转的直线整页失败
- 形状上的 `blipFill`（图片填充）按拉伸铺进没有旋转的矩形、圆角矩形和椭圆。`p:pic` 图片会画
- 平铺、`fillRect` 偏移、对不齐或还要缩放的裁剪、非 PNG、`a:blip` 上带颜色变换，或矩形上的图片填充还带描边，整页失败，不再静默跳过
- create 把文本里的 `\n` 写成段落里的 `a:br` 加后续 run。预览器按这些换行画出后续行。同一行里多个 run 目前共用第一个 run 的字号和颜色
- 椭圆可以带不透明描边和 `srgbClr` 透明度。圆角矩形用预设圆角，填充可以半透明。没有描边、没有旋转或翻转的不透明纯色圆角矩形（官方对照为 `srgbClr`）在 Windows 上按真实页面坐标走公开 GDI+ `FillPath`，加上抗锯齿和半像素偏移，叠在已有页像素上，直边和拐角与 PowerPoint 16.0 差 0。官方对照覆盖多种尺寸、半径（`adj` 0 / 1000 / 5000 / 16667 / 33333 / 50000）、页底、填色和原点。带描边、半透明或渐变的圆角仍走原来的路径，不在这次官方零差范围内。非 Windows 没有这条页面坐标路径。t93u 不扩到圆角形状
- 椭圆的左上角和右下角按 96 dpi 四舍五入。有描边时，外接矩形的宽和高再各加 1 像素
- 椭圆边缘用抗锯齿，并按半像素偏移。填充和描边同色时，可见轮廓就是这个外接矩形
- 半透明填充叠在已经画好的像素上。描边颜色不同时，只留外接矩形外侧的一圈，不先把整圆涂成描边色
- 圆角矩形的不透明 `srgbClr` 描边，直边按 96 dpi 收成 1 像素。右边和下边用起点加长度之后再取整
- 拐角是和直边相切的圆弧。覆盖的峰在半径内侧 0.15 像素，向内 0.9 像素、向外 0.65 像素收到 0。直边从圆弧收束之后才画实色
- 形状里的文字按 ECMA-376 21.1.2.3 逐字选字体槽。表内拉丁区间用 `a:latin`，复杂文种用 `a:cs`，`U+F000`–`U+F0FF` 用 `a:sym`，其余用 `a:ea`
- `U+2018`–`U+201F` 在 `lang` 为 `ii-CN`、`ja-JP`、`ko-KR`、`zh-CN`、`zh-HK`、`zh-MO`、`zh-SG`、`zh-TW` 时用 `a:ea`
- 一行的宽度是各字在自己字体里的前进宽度之和。没有本机字体时这页字不画
- 同一行的几种字体共用一条基线。基线取这些字体里最大的上升高度
- 形状和表单元格文字的竖直位置按 Windows 单元格模型排：基线落在内边距顶边加 `tmAscent`。未写行距时按 1.2 倍字号累加。这是布局近似，不保证与 PowerPoint 16.0 `Slide.Export` 的字形灰阶逐像素一致
- 对 Calibri 12/18/24 磅，首墨与金图对齐；36/48/72 磅首墨会低 1 像素。灰阶差记在手测里。不靠加大渐变或整页容差把差不为 0 的图算成通过。图表文字仍走 typographic 路径
- 簇状柱形图嵌在幻灯片上时，按图表框的像素大小来画
- 幻灯片上不画 Excel 的圆角外框。有标题时，绘图区比 Excel 再低 7 像素
- 幻灯片上的右侧图例按 GDI+ 默认排版宽度留空。字形前进宽度更窄，用它会把类目槽撑宽，柱越往右越偏
- 表先画纯色单元格。横向合并：锚格 `gridSpan`、后续 `hMerge` 时锚格填色铺满跨列（有表样式或显式边线、不走无样式默认灰边）；合并且会走默认灰边，或纵向 `rowSpan`/`vMerge`，整页 `NotImplemented`。去掉表样式且单元格没有显式边线（`lnL` / `lnR` / `lnT` / `lnB` 等）时，在文字之前画默认 2 像素、50% 黑 source-over 网格；每条外框和内线单独描，交点两次叠色。列、行边缘按累计 EMU 取整。有 `tableStyleId`、`tableStyle` 或单元格显式边线的表不加这套默认网格。中段和整数内角已与 PowerPoint 16.0 对齐；交点抗锯齿尚未差 0，见 `.plan/2026年-11月-16日-⚠️-pptx-表单元格灰边.md`。然后画单元格文字。合并单元格仍让整页失败
- 单元格文字跟形状文字同一套字体槽和基线。run 没写 `sz` 时用 18 磅
- 水平默认靠左，垂直默认靠上。边距没写时左右各 91440 EMU，上下各 45720 EMU
- `a:tcPr` 写了 `marL`、`marR`、`marT`、`marB` 或 `anchor` 就用写下的值
- run 没写字体时，拉丁用主题 minor 的 `a:latin`。主题里没有这个名字就用 Calibri
- 汉字用主题 minor 的 `a:ea`。`typeface` 为空时用微软雅黑
- 内置样式 `{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}` 且 `firstRow` 打开时，首行在 run 没写粗体和字色的情况下加粗，字用白色
- 同一内置样式的最小外观子合同（公开官方多样本锁定，含默认 Office 主题 accent1=4F81BD 与 accent1=FF0000）：无显式不透明单元格 solidFill 时画样式填色。
irstRow 打开时首行填主题 ccent1 本色；andRow 打开时正文行从首行之后（或 
irstRow 关闭时从第 0 行）起算，偶数行把 ccent1 在线性 sRGB（IEC 61966-2-1）里向白做 60% tint，奇数行做 80%。显式不透明 solidFill 覆盖样式填色。横向合并的样式填色铺满跨列，不在合并格内画竖向内线。
irstRow 打开时，在 96dpi 幻灯片画布上沿首行底边画 4 像素实心白条（上下各 2 像素），横向铺满表宽。其它 	ableStyleId、未打开的 andRow 带色、以及样式细内外描边（含交点抗锯齿）仍不画；灰边交点计划不并入本条通过结论。纵向合并仍整页 NotImplemented
- 其余单元格没写字色时用黑色。别的表样式不套这套首行规则
- 组合里已经会画的对象，子矩形先绕自己的中心转 90 度的倍数，再按 `layoutCheck` 的组合换算落到幻灯片 EMU，然后按上面的像素规则取整。
  子矩形的直角可以和组合自己的直角或翻转叠在一起。子矩形自己的水平或垂直翻转不改变这块纯色。组合可以再套一层，换算从内到外。里层也可以绕中心转 90 度。里层和外层可以同时绕中心转 90 度。组合可以绕中心转 90 度的倍数，也可以水平或垂直翻转。其它角度整页失败
- 母版和版式上已经会画的对象画在本页内容下面，盖在页底上面。本页 `showMasterSp` 为假时不画这两层。版式 `showMasterSp` 为假时不画母版上的对象。没写这个属性时照样画
- 旋转、翻转的图，对不齐的裁剪，视频和音频，报 `NotImplemented`，整页不画
- 还有认不出的形状时，整页失败，不交出缺一块的图
- xlsx 与 docx 报 `InvalidField`
- id 不是 `slide:N` 报 `InvalidId`。页不存在报 `IdNotFound`
- 成功时 `contentType` 为 `image/png`。源包不写回
- 像素差在绘制引擎里修。对照只标红区，不靠放宽容差过关

## exportPageImage

按页 id 把这一页写成 PNG，写到本地。只读。不改源包，不把像素放进 inspect。

``json
{
  "schemaVersion": 1,
  "action": "exportPageImage",
  "path": "C:\\data\\note.docx",
  "id": "page:1",
  "output": "C:\\out\\page.png"
}
``

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `path` | 是 | 源 docx |
| `id` | 是 | 页 id，形如 `page:1`，从 1 起 |
| `output` | 是 | 本地目标路径。缺了报 `MissingField`。不得默认写成源 `path` |
| `format` | 否 | 可省略，从 `path` 扩展名推断 |
| `dpi` | 否 | 本次栅格化的整数 DPI，24 到 600。省略或 96 时与现有 96 dpi 像素一致 |

- 可以画没有可见内容的页，或 Windows 上单节、单段的不透明拉丁（ASCII）Calibri 正文（同一段可按空格换行（GDI+ GenericTypographic 测宽；可排宽扣左缩进），行距与空段分页同一档并按半磅字号比例）。一节，节属性在 `w:body` 末尾
- 没有页眉、页脚、页面边框、行号、文档网格，也没有多栏。有字页不允许非零 `gutter`，垂直对齐只接受 top
- 文档默认字体必须是 Calibri，默认字号 11 磅（`w:sz` 为 22）。默认段落样式没有额外段属性
- 空段行距按 Calibri 的 2500/2048 × 11 磅。一页放不下就换页。有字页只接受恰好一段；同一段拉丁按空格贪心换行（`Face::typo_width`，与绘制同套 StringFormat；不按逐字 GetGlyphOutline 前进宽之和）。可排宽度为页左右边距之间减去左/start 缩进。单词本身宽于可排区或换行后总高度超出本页剩余正文高时报 `NotImplemented`
- 有字页可带左/start 缩进和段前间距；段后、首行/悬挂/右缩进必须为 0 或缺省；对齐只接受 left/start。run 可带 `sz` / 匹配的 `szCs`，`rFonts` 的 ascii/hAnsi 须为 Calibri 或沿用默认，颜色只接受 auto 或 000000
- 有字页在 Windows 上走已有公开 GDI+ `AntiAliasGridFit` 路径。非 Windows 对有字页报 `NotImplemented`。字形灰度按 C25 第 0 段暂缓，不把灰阶差算成整页通过，也不借用幻灯片文字的布局近似许可
- 表：按 `tblGrid` 累计列宽与行高（空自动行 = Calibri 11 行进 + 11 twip；显式 `trHeight` exact/atLeast）、`tblInd`、默认单元格边距、单元格 `shd` 纯色填色、表内拉丁或东亚文字；`tblBorders` 的 `single`（sz 2/4/8、color auto/000000）画 1px 黑，`nil`/`none` 或去掉边框节点为无边。合并、`tblStyle`、表内图、非默认 `tblCellMar`、非上述边色/粗细报 `NotImplemented`。不等宽且 `tblW=auto` 时 Word 可能 autofit，本实现跟 OOXML 网格，不建模 autofit
- 单节不透明 `wp:inline` PNG 按 extent 缩放绘制（extent 像素仍按 EMU×dpi/914400 截断）；浮动/裁剪/透明/混排报 `NotImplemented`
- 宽高按该节 `w:pgSz` 的 twip，按请求 DPI 将 `twip × dpi / 1440` 四舍五入成像素（96 dpi 时与现有 `twip × 96 / 1440` 一致）。磅按 `磅 × dpi / 72`。嵌入图 extent 按 `EMU × dpi / 914400` 截断，与历史 96 dpi 截断一致。不要套用 Excel 先按 96 量化再放大的行高规则
- 宽或高小于 1 或大于 4096 报 `NotImplemented`
- 页边距缺了按 1440 twip。正文区放不下一行报 `NotImplemented`
- 合并/表样式/表内图、浮动图、分节符、或空段行距不是上面这一档，报 `NotImplemented`。plain 表（含表内拉丁或东亚文字）与按 extent 缩放的不透明 inline PNG 已画
- xlsx 与 pptx 报 `InvalidField`
- id 不是 `page:N` 报 `InvalidId`。页不存在报 `IdNotFound`
- 成功时 `contentType` 为 `image/png`。源包不写回
- 像素差在绘制引擎里修。对照只标红区，不靠放宽容差过关

## layoutCheck

`layoutCheck` 是第七个动作，工具名就是 `layoutCheck`。参数仍是整份领域 JSON，或 MCP 里单独给 `requestPath`。
它只读几何：不写文件、不生成图片、不调用 Office、不按新画布重排。本期只对 pptx 实现，xlsx / docx 报 `InvalidField`。

请求只认 `action`、`schemaVersion`、`path`、`format`、`locale`。没有 `scope`，也不接受 `ops` 或 `output`。

成功时 stdout 是一份 JSON 树，不带 `ok`、不带 `path`：

``json
{
  "schemaVersion": 1,
  "format": "pptx",
  "slideSize": { "cx": 12192000, "cy": 6858000 },
  "slides": [
    {
      "id": "slide:1",
      "shapes": [
        {
          "id": "slide:1/shape:2",
          "name": "Title Box",
          "box": { "x": 914400, "y": 457200, "cx": 3000000, "cy": 800000 },
          "outOfBounds": false
        }
      ],
      "outOfBounds": ["slide:1/shape:9"],
      "overlaps": [
        { "a": "slide:1/shape:2", "b": "slide:1/shape:3", "area": 123456 }
      ],
      "textOverflow": [
        { "id": "slide:1/shape:4", "estimated": true }
      ]
    }
  ]
}
``

形状集合与坐标系：

- `shapes` 与 `inspect` 的 `shapes` 同集合：只收带 `p:txBody` 的 `p:sp`，沿 `p:grpSp` 递归收子形状，组合本身不产生条目。
  `mc:AlternateContent` 与 `a14` 公式框的例外按 [pptx 分册](json-contract-pptx.md) 第 422 到 428 行，不在本文重复。
- id 只用子形状的 `inspect` 稳定 id，可直接拿去写 apply 的 op。`name` 取 `p:cNvPr/@name`，没有就省略。
- `box` 是幻灯片 EMU。形状自身 `a:xfrm` 给的是组内坐标，要沿祖先组合链换算。
- 一层组合取 `p:grpSpPr/a:xfrm` 的 `off=(gx,gy)`、`ext=(gcx,gcy)`、`chOff=(cx0,cy0)`、`chExt=(ccx,ccy)`。
  换算为 `sx = gx + (x - cx0) × gcx / ccx`，`sy = gy + (y - cy0) × gcy / ccy`，`scx = cx × gcx / ccx`，`scy = cy × gcy / ccy`。
- 四个元素都按属性补缺省，已有属性原样保留，不因同伴缺失被重置；元素整体缺失视作两个属性都缺。
- `a:off/@x` 缺取 `gx = 0`，`/@y` 缺取 `gy = 0`；`a:ext/@cx` 缺取 `gcx = 0`，`/@cy` 缺取 `gcy = 0`。
- `a:chOff/@x` 缺取 `cx0 = 0`，`/@y` 缺取 `cy0 = 0`。
- `a:chExt/@cx` 缺取 `ccx = gcx`，`/@cy` 缺取 `ccy = gcy`；这里取同一层 `a:ext` 补完缺省后的值。
- 例：`<a:off x="914400"/>` 得 `gx = 914400`、`gy = 0`；`<a:chExt cy="100"/>` 得 `ccx = gcx`、`ccy = 100`。
- 这几个是本契约的缺省值，不依赖 Office 补齐。
- 嵌套组合按从最内层到最外层逐层套用。中间按有理数算，最后四舍五入到最近 EMU 整数，半值远离零。
  负坐标会出现，`-1.5` 收成 `-2`，`1.5` 收成 `2`。
- 恒等判定看补完缺省后的值：`ccx` 或 `ccy` 为 0 时该层按恒等处理。
  缺失本身不判 0，先按上面补缺省；补完仍为 0 才恒等。缺 `a:xfrm` 的形状不给 `box`。
- `box` 与同一 id 在 `inspect` 里的 `x` / `y` / `cx` / `cy` 不是同一坐标系，不能直接比相等。
  后者按 [pptx 分册](json-contract-pptx.md) 第 1220 行取自身 `a:xfrm` 的原始 EMU，不换算。第 914 行是图的规则，管不到形状。
- `layoutCheck` 的包围盒和 `exportSlideImage` 的像素都按这一套换算。出图换完再按 96 dpi 取整。组合可以绕中心转 90 度的倍数，也可以水平或垂直翻转。其它角度这一页不出图。

判定式都用整数算，长度量取 EMU：

- `slideSize` 取 `p:sldSz` 的 `cx` / `cy`。
- 越出条件：`x < 0`、`y < 0`、`x + cx > slideSize.cx`、`y + cy > slideSize.cy`，四条有一条成立即越出。
  相等不算。越出的 id 进该页 `outOfBounds`，形状上的同名布尔为 `true`。
  缺 `box` 的形状不进名单，布尔为 `false`。
- 相交宽 `iw = min(x1+cx1, x2+cx2) - max(x1, x2)`，高 `ih = min(y1+cy1, y2+cy2) - max(y1, y2)`。
  只有 `iw > 0` 且 `ih > 0` 才列入 `overlaps`，`area = iw × ih`。边界相接时 `area` 为 0，不列入。
  每对只列一次，`spTree` 文档序在前的放 `a`。缺 `box` 的形状不参与。
- 文字溢出的 `estimated` 恒为 `true`，表示下面这个可复现的阈值，不表示排版准确，也不保证与 PowerPoint 一致。
- 文本按 `p:txBody` 的 `a:p` 分段，段内再按 `a:br` 拆成片段。每个片段取 `a:r/a:t` 按文档序拼接，字符数按 Unicode 标量数计。
- 字号 `sz` 取各 run 的 `a:rPr/@sz` 最大值；取不到或数值为 0 时按 1800 处理，避免除零。`szEMU = sz × 127`。
- 可用宽 `w = box.cx - lIns - rIns`，可用高 `h = box.cy - tIns - bIns`。四个 inset 取 `a:bodyPr`，`lIns` / `rIns` 缺省 91440，`tIns` / `bIns` 缺省 45720。
- 每行容量 `n = max(1, floor(2 × w / szEMU))`。`rows` 为各片段 `max(1, ceil(片段字符数 / n))` 之和，不再另加换行数；空片段计 1 行。
- `rows × sz × 762 > h × 5` 时该形状 id 进 `textOverflow`。
  这是纯整数比较：`szEMU × 6/5` 展开就是 `sz × 762 / 5`，不引入浮点。系数 `1/2` 与 `6/5` 是固定估算值，不读字体度量、不排文字。
  无 `p:txBody`、文本为空或缺 `box` 的形状不进名单。

`layoutCheck` 不改 `inspect` / `apply` / `create` 的语义，也不进 `opc` 的写回路径。失败时走 [errors.md](errors.md) 的错误形态。

## 两层 JSON

领域 JSON 就是本文。库只认这一层。命令行的 `ooxp mcp` 会解析 JSON-RPC，但交给库之前只留下领域 JSON，`action` 用工具名补上。工具参数若只有 `requestPath`，命令行先读该文件，再把文件内容当作领域 JSON。`requestPath` 不是领域字段。外部调用方也可以自己包信封，把本文整个对象放进 `params`。
不许把 JSON-RPC 的 `jsonrpc`、`method`、`id` 塞进领域对象，也不许让单元格或段落字段去迁就 RPC。


### 诊断语料与出图验收边界（2026-11-17 P0）

`geometry-matrix.py` 的 100705 旧39项仅历史尺寸诊断，其中13个chart探针删除了公式引用源而保留缓存，不能据其Office空图验收内容。修复后的17组×3=51项在出图前核对actual cells、formula、cache、dimension、Normal/styleXF/font索引，并记录输入SHA。Office实际17个Series.Values为1/4/2、XValues为A/B/C；50个可出图满足固定96维度，但严格0 PASS/50 sizeDiffer/1既有隐藏源spark NI。此P0只修测试与展示，不改变renderer缓存优先、API支持范围、96dpi或像素容差；P0交付时工作表内嵌spark与圆角alpha仍待修；P1内嵌遗漏修复范围见下节，圆角alpha仍未修。解释图白底合成不改变raw gold的RGBA严格比较。


### 工作表/tiles内嵌迷你图（既有painter接线，2026-11-17 P1）

导出工作表的共同paint路径合成既有独立spark API支持的单宿主组：line、非负column及win-loss既有子集（合法日期轴按同一套槽公式比例定位；不扩标记/主题、不合法日期轴等支持）。空、未合并、LTR可见宿主按工作表实际格w/h直接采样；宿主sqref计入used-range，无需物化c。每宿主保留原f32覆盖率并只采样一次，格底合成后再画边框/浮动对象；tiles按绝对坐标裁取，禁止PNG白色key或缩放。每宿主沿用4096边长限制，合计宿主raster像素64Mi上限。

隐藏/零尺寸宿主不绘制，也不因其不可见复杂配置拒整表。可见且已有独立接口不支持的隐藏源（displayHidden未开启）、多宿主组、不合法日期轴、标记/样式等返回NI；sheet RTL、合并宿主、非空宿主文字的层序尚未官方锁定，同样诚实NI，不把合法配置叫畸形/异常。displayHidden=true沿用原能力。旧独立API行高round及chart/zoom合同不在本片盲统一。

17组同SHA公平矩阵中16个支持sheet宿主对同尺寸独立painter像素差0、宿主外变化0；16个独立spark和17个chart PNG字节不变；实际CLI九块拼接与mono像素差0。完整51项49成功/2 NI（新增NI是原来成功却漏图的隐藏源sheet），严格0 PASS/49 sizeDiffer/2 NI；库内像素一致不是官方整图PASS。gold复用P0相同输入SHA/同range/chart的原生图，未缩放，P1未重复打开Office。


### 2026-11-17：既有Excel圆角chart PNG的alpha

默认白底圆角chart使用RGBA，透明来自生产边框圆弧coverage，不以白色key透明化。内部白柱/白字/白底仍不透明；半覆盖边缘保存未预混白的RGB及coverage alpha，避免叠在深底出现白晕。原RGB白底合成像素保持不变；Slide/未mask的Canvas仍为原RGB。固定96、anchor/zoom/轴/legend布局不变。
本片不是自定义chart背景/描边能力：显式关闭圆角、noFill、自定义底色/线/效果诚实NI（有效但未支持），不按“异常缺省”吞掉；默认/显式纯白背景与显式圆角true为本片范围。原生Office整图仍严格比较尺寸和RGBA；四角透明恢复不等于AA/像素PASS。


### 2026-11-17：既有簇状柱图线性值轴

nice-number候选容量使用固定96完整字体行框和图表可用高度，不再所有图固定5..10区间。大图原4.5/.5合同保留，小图可为5/1。显式valAx scaling min/max及majorUnit/minorUnit优先，默认minor=major/5；非整除范围按数值位置画刻度，循环有界。min/max/major/minor的26组标量由Office公开属性验证，不等于PNG PASS。
窄范围单侧非恒值的非零自动界限尚未实现，返回NI（有效配置，不是畸形缺省）。多值轴、log/reversed、非默认crosses/crossesAt、非General格式不伪造支持。本片不声称Office所有title/布局/字体组合自动算法已完全匹配。默认autoZero交点另核对26项COM crossesAt并接入类别轴线/刻度/标签；非默认交点仍NI。详自动值轴计划；96几何、raw gold、容差和P3 AA残差不改。


## 2026-11-17 t127：DPI差异分层验收（非新功能）

用户允许**已证实仅DPI引起的像素差**通过；raw严格结果保留但不再一票否决。库固定96 contract不变，原始gold不缩放、不改色、不放宽正常配置。验收必须绑定source SHA/实际exe，核实内容、尺度、字体及合法继承、位置/anchor后才可豁免命名子范围；sizeDiffer、同尺寸或逻辑标量通过不能直接推出整图通过。曲角AA、spark端点、默认边距或布局若未证因，写待确认，真实缺内容/样式/标签位置差仍须修。

当前原51几何子证书：**38通过（DPI豁免）/11待确认/2 NI**；11中6 Normal字体继承、5 chart导出1px边界舍入。轴26/27 case的130逻辑值、P1接线16/16、P3四角17/17分别是已通过子范围，不合并张数。完整图原51=49待确认/2NI；新27=26待确认/1NI；图例7待确认。raw严格49/26/7 sizeDiffer保留，不能当最终失败统计。

本轮只验收/文档，renderer基线a81c49e6；原1868 Rust和8 Python日志保留，新策略再加10 Python测试。证据：桌面 `office-editor-PR10-DPI验收复核-20261117-145216`。详 `manual-check/office/dpi_acceptance.py`、`dpi-acceptance-report.py` 和DPI验收复核计划。COM图例中心-4pt尚未校准原点，禁止直接硬补；explicit类别标签侧此前读图误报已撤回：COM=4 NextToAxis且双方原生bbox均在零轴下/绘图区底之上，不修改renderer。其它十项暂停新功能不恢复。


2026-11-17 类别标签反证复核：1ec6eac0中的explicit“Office标签在绘图区底”是读图错误，已明确撤回。当前1ec实际exe重渲染与a81图片SHA相同；COM公开TickLabelPosition=4（NextToAxis），原生bbox证实双方均跟随0轴，原有130逻辑值成果不动。修正验收器/CSV，不制造renderer补丁。38项几何DPI通过不变；曲角/端点/其它布局待确认不变。详类别标签误报复核计划及桌面150736新证据，旧快照不覆盖。


## 2026-11-17 Normal继承6路径有限核实（非renderer修复）

2个合法样式族×3路径=6记录、4份源，库按builtin0→styleXF→font的规范链96几何6/6验证；Office实际Styles.Normal与独立SaveCopy后的Normal均为Calibri11，而源声明Arial11，已证明不只是name/xf编号归一化。此探针族中导入改变字体语义，不能算仅DPI，也不证明库应改font0。17组有限控制、实际cell字体与Normal分开、列/Chart点数、原始/保存副本XML及SHA均保存；canonical从未重开、不当新gold。

DPI几何分子**38/49不变**，6 Normal路径仍需Office同语义可比证据，另5导出边界/2 NI不扩本片。6条规范链证书不是6功能/6整图通过；保留builtinId优先、非零xf及MDW96生产合同，不加7/8/font0回退。23 Python/语法/fmt/diff检查；无Rust改动，不冒称重跑1868。证据：桌面office-editor-PR10-Normal继承核实-20261117-151855；详Normal继承核实计划及normal-inheritance-report.py。


## 2026-11-17 Chart.Export宽度边界5条核实（仅验收器修复）

5个零off twoCell簇状柱图的旧ceil(ChartObject.Width*106/72)比原PNG多1，原因是将与Single ShapeRange相同的点数读回当作无限精确像素边界。公开Double列宽与同Worksheet左右端点通过IEEE754半ULP精确区间认证设备栅格，分别320/288/320/288/243px，与原PNG严格一致；不加epsilon、不容差，真多1px仍拒绝。

五个新frame-ext一致性单变量控制Office PNG字节不变；十个新原生PNG均与旧gold字节相同，十个库输出与旧after字节相同。公开DPI均106，pHYs4173/m是整数米量化，非新显示DPI。ChartArea/PlotArea内侧原点未当PNG外框。只更新宽度责任链，未宣称通用导出舍入算法，未改renderer/96/576336zoom/高度/alpha。

**最新几何DPI证书43/49（此前38历史值由此更新）**；本片5/5子范围通过，不代表5功能/43整图。Normal6仍待同语义oracle、2NI不变；其它AA/内容/样式残差仍分别保留，raw严格列仅诊断。29 Python/语法/fmt/diff验证，无Rust改动不冒称重跑全套。证据：桌面office-editor-PR10-导出边界核实-20261117-153629；chart-boundary-report.py/原始API/哈希/更新CSV。十个暂停新功能未恢复。


## 2026-11-17 迷你图端点/coverage有限核实（不改renderer）

原几何DPI证书仍43/49；本片未增加端点/整图PASS。原dengxian11-absent同SHA及6个预声明单变量控制，原Office和CLI图与141413像素不变；七组历史右/下1px裁剪均只去白边、无原点平移。72宽端点bbox左右各1px差、81宽bbox相同，尚不足认证逻辑端点，不硬改PAD2/round/端帽。

笔画coverage存在不能仅归DPI的实现差异：0.75pt平线Office106原色中心/原生EMF alpha255；库当前核clamp(weight/2+0.25-distance)即数学代入106笔宽，上限仍0.802083，不能产生该中心。此为现有AA近似机制反证，不是修改输出DPI或拟合0.25；尚未修生产核/plot clip。EMF内部为71×21位图置于(1,1)，不是逻辑路径；不混局部与PNG坐标。补充EMF seed已打开才prehash导致一次错误changed断言，最终7源SHA不变/独占读/own锁0，第二源未重开。

公开COM实际笔宽/源色/markers/数据与宿主点数已保留；两个错误属性名null强转字段明确不采信，不能以其证明默认语义。完整36 Python（29+7）、语法/fmt/diff验证；无Rust改不冒称重跑全套。证据：桌面office-editor-PR10-迷你图端点核实-20261117-155155。仅当前PR既有迷你图问题，暂停的十项未启动。


## 2026-11-17 迷你图笔画欠覆盖修复（既有line子范围）

19c已证实的距离核欠覆盖现已修复。不是将0.25试成0.5：以单位像素沿线法向的一维box滤波定义，线带区间[-r,r]与像素区间[d-0.5,d+0.5]的交叠长度为clamp(r+0.5-d,0,min(1,2r))。半像素来自像素宽度1的两半；min(1,2r)保证小于1px细线不虚增覆盖，水平/垂直线体在不同相位积分守恒。斜线/圆端帽仍是既有距离几何的滤波近似，不声称精确二维方像素面积或Office全域AA。多段仍max合并coverage后只blend一次，不增join模式。

固定96、点数转像素、PAD2、point_xy/round、host clip、数据和几何未改。默认0.75pt平线3个不同宿主控制中心由RGB105/136/173恢复55/96/146，与同SHA Office原图源色一致；含1.5/3pt在内5个水平控制中心色5/5符合。只收口本中心色/核欠覆盖子范围，不能把较粗线Office plot clip差或端点位置也称已修。

实际新CLI在原17×3矩阵重绘：49成功/2NI，固定维度不变，raw严格49sizeDiff/2NI仍保留；额外7控制×2路径另列不加分母。23个宿主=独立spark像素差0、23宿主外变化0，17chart PNG字节不变；实际CLI非A1九tiles拼接与mono像素差0。几何证书仍43/49，端点/斜线与端帽AA和plot clip缺证仍待核，不用t127吞未证实差异。

完整cargo test -p ooxp为1873 passed/0failed/0ignored（新增5数学/颜色/对称/重复段/单点回归）；workspace clippy --all-targets --all-features -D warnings、fmt、真实cargo build -p ooxp-cli通过。全部38 Python通过；旧核反证及19c原图保留为明确historical，不删除旧test假通过。新exeSHA 28ddb1f2811f2ae8c20d6c580ff3c5577bdcced33f92eaf1096295074c499782；旧exe63beeb5539353f3b62f3de8e0b4e641f0ff063b87e945132348289a87c0beb69在新证据目录完整保留。

证据：桌面office-editor-PR10-迷你图笔画修复-20261117-160458。原始before/after/gold及SHA、数学解释、metrics/host/tiles/日志；本片未重开Office或修改旧快照。仅修本PR既有迷你图笔画，不恢复暂停十项。


## 2026-11-17 图例坐标与cc6收尾（按t128停止扩展）

只用原7源，4源必要COM校准，0新源；同一baseline ChartObject在worksheet平移72/约36pt后，ChartArea.Left/Top跟随而Legend/LegendKey数值不变，Chart.Export PNG字节完全不变。说明这些属性不在同一worksheet坐标系；ChartArea.Width426/Height246亦不等于ChartObject432/252。PNG内部原点尚无完整证书，key纯色支持框/文字ink不当Legend.Rect；此前“中心-4pt是真库bug”不成立为证据，不硬补4pt。手动edge/factor组合合法，styled-right仍是style+manual+overlay，不按异常吞默认。

已验证子范围分列：显式Arial18粗红2/2、手动w/h=172.8×75.6pt比例语义3/3、overlay不预留布尔语义2/2、系列色语义7/7；这些不同分母不相加，7整图仍缺绝对原点/自动布局细节证书。636×371与576×336外框由106/96解释，不用sizeDiff否决已验证子范围，也不豁免未证实布局。

收尾中只修一处cc6确定回归：仅改overlay却使库缺省右图例key从8×8变7×7并改变文字排版，而Office两源key/legend属性完全相同。将锁定default-right ink与overlay绘图区预留解耦；保持原默认PNG，overlay仍释放plot空间，不改坐标/字体度量/手动布局、不增功能。7当前图仅overlay-only一张变化，6张字节不变；修后蓝key64像素与baseline相同，原baseline20个不透明字形点保留，overlay后方plot黑边不误当字形差。原17×3路径49 PNG字节不变/2同NI，sheet/spark/其它chart未回退。

实际完整Rust1874 passed/0failed/0ignored；44 Python通过，fmt、workspace clippy全targets/features-Dwarnings、真实CLI build、语法/diff通过。新exeSHA见桌面minimal-fix-results.json，旧28ddb1f2完整exe保留。原1873与38是前片历史，不冒充本次。几何43/49不变，未称7功能/7整图PASS。

证据：桌面office-editor-PR10-图例坐标核实-20261117-162104。四自有wb全部Close(false)、ownwb0、源SHA不变/独占读/锁0；无Save/Quit/kill/删锁/系统设置。当前必要检查与小回归修复已结束，按t128不继续追4pt、自动布局或其它项目；余绝对原点/自动边距/key与ink细节记录为限制，暂停十项不恢复。


## 2026-11-20 t160u/t164u：迷你折线与列宽审核通过

用户 t160 将迷你折线角度审核标为通过（同 106 画布角度 / 抗锯齿残差已接受，无唯一通用公式，未写入代码）。用户 t164 通过 `fonts[0]` 列宽修复：Excel 16.0 列宽 MDW 取 stylesheet `fonts[0]` 而非 Normal，仍按请求 DPI 重测，不是「永远 96 MDW」。修后 106 下工作表 325×23、迷你图 82×23 已与官方齐，图表 324×220 对官方 324×221，已接受 1px 高度残差（未硬插 +1）。对应计划已改名为 ✅：`.plan/2026年-11月-17日-✅-xlsx-Normal继承核实.md`、`.plan/2026年-11月-17日-✅-迷你图端点与笔画核实.md`、`.plan/2026年-11月-17日-✅-xlsx-DPI验收复核.md`、`.plan/2026年-11月-20日-✅-pr12-出图对照收束.md`。准备由用户合并 PR12。不改绿 11 月 14 日总计划或 pr10 / pr11 交接。
