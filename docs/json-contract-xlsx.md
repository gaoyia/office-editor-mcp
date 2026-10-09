# xlsx 领域 JSON

本文是 [json-contract.md](json-contract.md) 的 xlsx 分册。这里写稳定 id、inspect 的 scope 与响应、create，以及只属于工作簿的 apply op。

信封、跨格式 op（图、图表、SmartArt、批注、超链接、文本框、表格插删行列、`setProps`、`setProtection`、
`setShapeText` 等）以及 `exportMedia` / `exportChartData` 仍在主文件。

机器可读副本见 [json-schema/xlsx.schema.json](json-schema/xlsx.schema.json)，
入口仍是 [json-schema/request.schema.json](json-schema/request.schema.json)。与 Markdown 冲突时以本文与主契约为准。

## 稳定 id

- **工作簿**
  - id：`xlsx:workbook`
  - 例子：`xlsx:workbook`
- **自定义属性**
  - id：`prop:{name}`
  - 例子：`prop:Editor`
- **工作表**
  - id：`sheet:{name}`
  - 例子：`sheet:Sheet1`、`sheet:销售 2026`
- **单元格**
  - id：`cell:{name}!{A1}`
  - 例子：`cell:Sheet1!A1`、`cell:销售 2026!BC12`
- **工作表批注**
  - id：`cell:{name}!{A1}/comment:{guid}`
  - 例子：`cell:Sheet1!A1/comment:{B6B5C44E-CC35-4DF0-8F43-15D443D3BFDA}`
- **工作表批注回复**
  - id：`{批注 id}/reply:{guid}`
  - 例子：`cell:Sheet1!A1/comment:{B6B5C44E-CC35-4DF0-8F43-15D443D3BFDA}/reply:{973A07B5-6045-4A82-B8DF-002F1A5F07B0}`
- **工作表图**
  - id：`sheet:{name}/pic:{cNvPrId}` 或 `sheet:{name}/pic:index:{k}`
  - 例子：`sheet:Sheet1/pic:2`
- **页眉页脚槽**
  - id：`sheet:{name}/hf:{kind}:{side}`
  - 例子：`sheet:Sheet1/hf:odd:header`
- **页眉页脚图**
  - id：`{槽 id}/pic:{cNvPrId}` 或 `{槽 id}/pic:index:{k}`
  - 例子：`sheet:Sheet1/hf:odd:header/pic:2`
- **工作表图表**
  - id：`sheet:{name}/chart:{cNvPrId}` 或 `sheet:{name}/chart:index:{k}`
  - 例子：`sheet:Sheet1/chart:2`
- **工作表文本框**
  - id：`sheet:{name}/sp:{cNvPrId}` 或 `sheet:{name}/sp:index:{k}`
  - 例子：`sheet:Sheet1/sp:2`
- **工作表公式框**
  - id：`sheet:{name}/math:{cNvPrId}` 或 `sheet:{name}/math:index:{k}`
  - 例子：`sheet:Sheet1/math:2`
- **工作表形状**
  - id：`sheet:{name}/shape:{cNvPrId}` 或 `sheet:{name}/shape:index:{k}`
  - 例子：`sheet:Sheet1/shape:2`
- **工作表连接线**
  - id：`sheet:{name}/cxn:{cNvPrId}` 或 `sheet:{name}/cxn:index:{k}`
  - 例子：`sheet:Sheet1/cxn:3`
- **工作表 SmartArt**
  - id：`sheet:{name}/smartdiagram:{cNvPrId}` 或 `sheet:{name}/smartdiagram:index:{k}`
  - 例子：`sheet:Sheet1/smartdiagram:3`
- **工作表 SmartArt 点**
  - id：`{图示 id}/pt:{modelId}`
  - 例子：`sheet:Sheet1/smartdiagram:3/pt:2`
- **图表系列**
  - id：`{图表 id}/series:{idx}`
  - 例子：`sheet:Sheet1/chart:2/series:0`
- **趋势线**
  - id：`{系列 id}/trendline:{k}`
  - 例子：`sheet:Sheet1/chart:2/series:0/trendline:1`
- **误差线**
  - id：`{系列 id}/errorBars:{k}`
  - 例子：`sheet:Sheet1/chart:2/series:0/errorBars:1`
- **数据验证**
  - id：`sheet:{name}/dv:{n}`
  - 例子：`sheet:Sheet1/dv:1`
- **条件格式**
  - id：`sheet:{name}/cf:{n}`
  - 例子：`sheet:Sheet1/cf:1`
- **工作表表格**
  - id：`sheet:{name}/table:{n}`
  - 例子：`sheet:Sheet1/table:1`
- **工作表透视表**
  - id：`sheet:{name}/pivot:{n}`
  - 例子：`sheet:Sheet1/pivot:1`
- **工作表超链接**
  - id：`sheet:{name}/link:{n}`
  - 例子：`sheet:Sheet1/link:1`
- **工作表迷你图组**
  - id：`sheet:{name}/spark:{n}`
  - 例子：`sheet:Sheet1/spark:1`
- **工作表 OLE**
  - id：`sheet:{name}/ole:{shapeId}` 或 `sheet:{name}/ole:index:{k}`
  - 例子：`sheet:Sheet1/ole:1025`
- **工作表 3D 模型**
  - id：`sheet:{name}/model3d:{cNvPrId}` 或 `sheet:{name}/model3d:index:{k}`
  - 例子：`sheet:Sheet1/model3d:2`
- **工作表切片器**
  - id：`sheet:{name}/slicer:{cNvPrId}` 或 `sheet:{name}/slicer:index:{k}`
  - 例子：`sheet:Sheet1/slicer:2`
- **定义名**
  - id：`name:{n}`
  - 例子：`name:1`
- **外部数据连接**
  - id：`connection:{xmlId}` 或 `connection:index:{k}`
  - 例子：`connection:1`
- **自定义视图**
  - id：`customView:{n}`
  - 例子：`customView:1`

`{name}` 是 `workbook.xml` 里 `<sheet name="...">` 的值，不是 `sheetId`，也不是 `r:id`。查找工作表时大小写不敏感，输出 id 时用文件里的原始大小写。

`{A1}` 是大写列字母加行号，例如 `A1`、`AA100`。请求里若写 `a1`，库先规范成 `A1` 再生成 id。

解析单元格 id：从右侧第一个 `!` 切开。右侧必须是合法 A1，左侧是工作表名。这样工作表名里即使含 `!`，也不会切错。

工作表批注挂在该 `worksheet` 上，不挂在单元格节点上。`{guid}` 取该表线程化批注部件里顶层 `threadedComment/@id` 的原文（MS-XLSX §2.6.205），含花括号。
回复 `{guid}` 取带 `parentId` 的那条 `threadedComment/@id`。按文件原文精确匹配，不因大小写或有无花括号而放宽。同一张表多条批注共用一个 threadedComments 部件。
没有 `parentId` 的是顶层；有 `parentId` 的嵌在对应父批注的 `replies` 里。
旧式 `/xl/commentsN.xml` 里作者不以 `tc=` 开头的条目、以及只有 VML `legacyDrawing` 没有线程化部件的批注，不进树、不接受 apply。
`insertComment` 只写线程化部件，并按 Excel 16.0 另存补占位 `comments.xml` 与 Note `v:shape`。

图表工作表、宏表、对话表出现在 inspect 的表名单里，但没有单元格 id。它们的 id 仍是 `sheet:{name}`。对它们发 `setCell` 报 `InvalidField`。
对它们发 `insertPicture` / `insertChart` 报 `InvalidField`。图表工作表上的那张 `xdr:graphicFrame` 走下面的图表 id，不另编图 id。
新建、删除图表工作表用 `insertChartSheet` / `deleteChartSheet`，不要复用 `insertChart` 往 `chartsheet` 上插第二张。
普通工作表的插、删、改名、重排与隐藏用 `insertSheet` / `deleteSheet` / `renameSheet` / `moveSheet` / `setSheetHidden`，不要复用 `insertChartSheet`。

工作表图挂在该 `worksheet` 上，不挂在单元格上，也不挂在工作簿根上。图的 `{cNvPrId}` 取该 `xdr:pic` 里 `xdr:cNvPr/@id` 的原文（ECMA-376 §20.5.2.8）。
输出用文件里的原始大小写与数字写法。请求里若双方都能解析成十进制整数，按数值匹配（`1` 与 `01` 视为同一处）；否则按原文精确比。同一张表内重复 `cNvPr/@id` 时，inspect 都列出，apply 命中该表文档顺序第一处。
没有 `cNvPr/@id` 时用 `pic:index:{k}`，树上 `"stable": false`。`{k}` 只在该表全部 drawing 部件内从 1 计，计入该表所有被收入的图，不只是缺 id 的那些。

只认锚点（`xdr:twoCellAnchor` / `xdr:oneCellAnchor` / `xdr:absoluteAnchor`）的直接子元素 `xdr:pic` 且带 `a:blip`。
组合 `xdr:grpSp` 里的图、`xdr:graphicFrame`（图表、SmartArt）、`xdr:sp`、连接线不进 `pictures`。VML / `legacyDrawing`（批注框、旧式控件）不编。
工作表图只跟工作表元素 `<drawing r:id>` 指向的 drawing 部件，不跟 `<drawingHF>`。`mc:AlternateContent` 只进第一个 `Choice`。
一张表可以有多个 worksheet drawing 关系，按工作表 XML 里 `<drawing>` 的文档顺序拼接后再编号。文本框与 SmartArt 见下文，不要把它们编进 `pictures`。

xlsx 页眉页脚是工作表上的 `headerFooter`（ECMA-376 §18.3.1.46），不是 Word 那种独立 header 部件，不要套 `header:default:1/para:…`。
`{kind}` 为 `odd` / `even` / `first`，`{side}` 为 `header` / `footer`，一律小写。六个槽位始终按这个顺序：奇数页页眉、奇数页页脚、偶数页页眉、偶数页页脚、首页页眉、首页页脚。
对应 XML 本地名 `oddHeader` / `oddFooter` / `evenHeader` / `evenFooter` / `firstHeader` / `firstFooter`。
文本就是该元素原文，含 `&P` / `&D` / `&A` / `&C` 这类代码，不编成独立域节点。inspect 报原文；apply 整段替换原文。

页眉页脚图只认 `<drawingHF r:id>` 指向的 drawing 里、锚点的直接子元素 `xdr:pic` 且带 `a:blip`。判定与工作表图相同。VML / `legacyDrawingHF` 不进树、不改。
Excel 16.0 另存的页眉图通常是 VML，因此默认样例常常没有 DrawingML 页眉图；树里出现页眉图，只因为包里已有 `drawingHF`，或 `insertPicture` 往槽位新建了 DrawingML。
工作表图名单与页眉图名单不混：`sheet:{name}/pic:…` 只来自 `<drawing>`，`sheet:{name}/hf:…/pic:…` 只来自 `<drawingHF>`。
`drawingHF` 上的 `cho` / `lho` / `rho`（奇数页页眉中/左/右）、`che` / `lhe` / `rhe`（偶数页页眉）、`chf` / `lhf` / `rhf`（首页页眉）、
`cfo` / `lfo` / `rfo`（奇数页页脚）、`cfe` / `lfe` / `rfe`（偶数页页脚）、`cff` / `lff` / `rff`（首页页脚）把 `cNvPr/@id` 分到对应槽；对不上的图归 `odd:header`。
没有 `cNvPr/@id` 时用 `pic:index:{k}`，`{k}` 只在该槽位从 1 计，`stable` 为 `false`。
`chartsheet` / `dialogsheet` / `macrosheet` 本期只读页眉文本，不列出、不接受页眉图。页眉页脚里的图表、形状、OLE 本期不进树。

工作表图表挂在该 `worksheet` 或 `chartsheet` 上，不挂在单元格上，也不挂在工作簿根上。
图表的 `{cNvPrId}` 取该 `xdr:graphicFrame` 里 `xdr:nvGraphicFramePr/xdr:cNvPr/@id` 的原文（§20.5.2.8），匹配规则与工作表图相同。
没有该属性时用 `chart:index:{k}`，树上 `"stable": false`。`{k}` 只在该表全部 drawing 部件内从 1 计，只计入被收入的图表。
系列 `{idx}` 从 0 起，按 `/xl/charts/…` 里 `c:ser` 的文档顺序，不跟 `c:idx/@val`。
趋势线 id 为 `{系列 id}/trendline:{k}`，误差线 id 为 `{系列 id}/errorBars:{k}`，`{k}` 从 1 起，按该系列里对应子元素的文档顺序。

只认锚点的直接子元素 `xdr:graphicFrame`，且 `a:graphicData/@uri` 为 `http://schemas.openxmlformats.org/drawingml/2006/chart`（可带尾斜杠，ECMA-376 §21.2）。
Office 2016 以后的 chartex URI（`http://schemas.microsoft.com/office/drawing/2014/chartex`）也列入，`type` 为 `chartex`。
组合 `xdr:grpSp` 里的框、`xdr:pic`、形状、SmartArt、OLE、VML 不进 `charts`。页眉页脚里的图表本期不进树。`mc:AlternateContent` 只进第一个 `Choice`。
同一张表上同时有 `xdr:pic`、图表、文本框和 SmartArt 的，各走各的名单。diagram URI 的 `graphicFrame` 走 SmartArt，不要算进 `chartCount`。

工作表文本框挂在该 `worksheet` 上。`{cNvPrId}` 取该 `xdr:sp` 里 `xdr:cNvPr/@id` 的原文，匹配规则与工作表图相同。
没有该属性时用 `sp:index:{k}`，树上 `"stable": false`。`{k}` 只在该表全部 worksheet drawing 部件内从 1 计，只计入被收入的文本框。
只认 `xdr:sp` 且 `xdr:cNvSpPr/@txBox` 为 `1` / `true`（Excel 16.0 另存如此）。
连接线 `xdr:cxnSp`、无 `txBox` 的普通 `xdr:sp`、VML 控件不进 `textBoxes`。组合 `xdr:grpSp` 本身不进树，里面符合条件的文本框要展开；几何仍取外层锚点。页眉页脚里的形状本期不进树。
`mc:AlternateContent` 只进第一个 `Choice`。若 Choice `Requires` 含 `a14` 且子孙有 `a14:m` 或 `m:oMath`，整段不进 `textBoxes`，只进 `maths`。
可见字在 `xdr:txBody` 的 `a:t`，规则对齐 pptx 形状 `text`（`a:br` 变成 `\n`，不收 `a:fld`）。不要做成 docx 那种框内 `w:p`。

工作表形状挂在该 `worksheet` 上，与文本框分家。`{cNvPrId}` 取该 `xdr:sp` 里 `xdr:cNvPr/@id` 的原文，匹配规则与工作表图相同。
没有该属性时用 `shape:index:{k}`，树上 `"stable": false`。`{k}` 只在该表全部 worksheet drawing 部件内从 1 计，只计入被收入的形状。
只认 `xdr:sp` 且不是文本框：没有 `xdr:cNvSpPr/@txBox`，或取值不是 `1` / `true` / `True`。有 `xdr:txBody` 则 `text` 规则与文本框相同；没有则省略 `text`。
`prst` 取 `a:prstGeom/@prst`，没有（含只有 `a:custGeom`）则为 `null`。组合里符合条件的形状要展开；几何仍取外层锚点。
与该表 `oleObject/@shapeId` 相同的隐藏 `xdr:sp` 不进 `shapes` / `shapeCount`，也不当 `groupShapes` 成员。页眉页脚与 `chartsheet` 上的形状本期不进树。
VML 不进。`mc:AlternateContent` 只进第一个 `Choice`。
若 Choice `Requires` 含 `a14` 且子孙有 `a14:m` 或 `m:oMath`，整段不进 `shapes`。

工作表连接线挂在该 `worksheet` 上。`{cNvPrId}` 取该 `xdr:cxnSp` 里 `xdr:nvCxnSpPr/xdr:cNvPr/@id` 的原文，匹配规则与工作表图相同。
没有该属性时用 `cxn:index:{k}`，树上 `"stable": false`。`{k}` 只在该表全部 worksheet drawing 部件内从 1 计，只计入被收入的 `xdr:cxnSp`。
只认 `xdr:cxnSp`，不把无 `txBox` 的 `xdr:sp` 算进来。`prst` 取 `a:prstGeom/@prst`，没有则为 `null`。不写 `text`。
空 `cNvCxnSpPr`（未绑 `stCxn` / `endCxn`）仍列入。组合里符合条件的连接线要展开；几何仍取外层锚点。页眉页脚与 `chartsheet` 上的连接线本期不进树。`mc:AlternateContent` 只进第一个 `Choice`。

工作表公式框挂在该 `worksheet` 上。`{cNvPrId}` 取 Choice 里 `xdr:sp` 的 `xdr:cNvPr/@id` 原文，匹配规则与工作表图相同。
没有该属性时用 `math:index:{k}`，树上 `"stable": false`。`{k}` 只在该表全部 worksheet drawing 部件内从 1 计，只计入被收入的公式框。
只认 `mc:AlternateContent` 且 Choice `Requires` 含 `a14`，子孙有 `a14:m` 或 `m:oMath`。整段只进 `maths`。
Choice 与 Fallback 都不进 `textBoxes` / `shapes`，也不计入 `textBoxCount` / `shapeCount`。
组合里符合条件的公式框要展开，组合本身不进树。页眉、`chartsheet` 本期不进树。
不是单元格 `f`。`insertMath` 的 `id` 必须是 `sheet:{name}`。`setMath` / `deleteMath` 的 `id` 必须是公式框 id。
对单元格 id、图 id、文本框 id、形状 id 或 `xlsx:workbook` 发公式 op 报 `InvalidId`。
对 `chartsheet` 报 `InvalidField`。对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。
出现 `expr` / `kind` 报 `InvalidField`。不把 OMML 原文、不把 `expr` 放进树。create 可用 `sheets[].maths`。
docx 见 [docx 分册](json-contract-docx.md#insertmath)。pptx 见 [pptx 分册](json-contract-pptx.md#insertmath)。

`insertTextBox` / `insertShape` 可带 `setShapeFormat` 同一套可选字段。xlsx 的 `anchor` 仍是单元格锚点，框内垂直对齐只用 `textAnchor`。
`insertTextBox` 的 `id` 必须是 `sheet:{name}`，且该表 `kind` 为 `worksheet`。
对文本框 id、形状 id、连接线 id、图 id、图表 id、单元格 id、页眉槽位 id 或 `xlsx:workbook` 发 `insertTextBox` 报 `InvalidId`。
对 `chartsheet` 发此 op 报 `InvalidField`。对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。
`deleteTextBox` 的 `id` 必须是 xlsx 文本框 id。`setTextBoxSize` / `setTextBoxAnchor` 的 `id` 可以是文本框 id、形状 id 或连接线 id。
`setShapeText` / `setShapeFormat` 的 `id` 必须是文本框 id 或形状 id；对连接线 id 报 `InvalidId`。
`setTextBoxWrap` / `setTextBoxFill` / `setTextBoxLine` / `setTextBoxDirection` 是 Word 框属性，xlsx 上报 `InvalidField`。

`insertShape` 的 `id` 必须是 `sheet:{name}`，且该表 `kind` 为 `worksheet`。
对形状 id、文本框 id、连接线 id、图 id、单元格 id 或 `xlsx:workbook` 发 `insertShape` 报 `InvalidId`。对 `chartsheet` 发此 op 报 `InvalidField`。
对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。`deleteShape` 的 `id` 必须是 xlsx 形状 id。
对文本框 id 或连接线 id 发 `deleteShape` 报 `InvalidId`。

`insertConnector` 的 `id` 必须是 `sheet:{name}`，且该表 `kind` 为 `worksheet`。
对连接线 id、形状 id、文本框 id、图 id、单元格 id 或 `xlsx:workbook` 发 `insertConnector` 报 `InvalidId`。
对 `chartsheet` 发此 op 报 `InvalidField`。对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。
`deleteConnector` 的 `id` 必须是 xlsx 连接线 id。`groupShapes` 的 `id` 必须是 `sheet:{name}`。
`ungroup` 的 `id` 必须是该组合内一个文本框 id、形状 id 或连接线 id。
`setShapePos` / `setShapeSize` / `setShapeFill` / `setShapeLine` 在 xlsx 上报 `InvalidField`。
xlsx 改几何请用 `setTextBoxSize` / `setTextBoxAnchor`。出现 `x` / `y` 报 `InvalidField`。

工作表 SmartArt 挂在该 `worksheet` 上。`{cNvPrId}` 取该 `xdr:graphicFrame` 里 `xdr:nvGraphicFramePr/xdr:cNvPr/@id` 的原文，匹配规则与工作表图相同。
没有该属性时用 `smartdiagram:index:{k}`，树上 `"stable": false`。`{k}` 只在该表全部 worksheet drawing 部件内从 1 计，只计入被收入的图示。
点 id 为 `{图示 id}/pt:{modelId}`，规则与 docx / pptx 相同。
只认 `xdr:graphicFrame` 且 `a:graphicData/@uri` 为 `http://schemas.openxmlformats.org/drawingml/2006/diagram`（可带尾斜杠）。
`dgm:relIds` 的四条关系写在 **drawing** `.rels` 上（Excel 16.0 另存如此；不要写到工作表 `.rels`）。组合里符合条件的框要展开；几何仍取外层锚点。页眉页脚里的图示本期不进树。
`mc:AlternateContent` 只进第一个 `Choice`。drawing 缓存按 docx / pptx：对不上 `modelId` 则删缓存。

`insertSmartArt` 的 `id` 必须是 `sheet:{name}`，且该表 `kind` 为 `worksheet`。
对图示 id、点 id、图 id、图表 id、文本框 id、单元格 id 或 `xlsx:workbook` 发 `insertSmartArt` 报 `InvalidId`。
对 `chartsheet` 发此 op 报 `InvalidField`。对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。
`kind` / `wrap` / `x` / `y` 在 xlsx 上出现报 `InvalidField`。
`deleteSmartArt` / `setSmartArtSize` / `setSmartArtAnchor` / `setSmartArtColors` / `setSmartArtStyle` / `setSmartArtLayout` 的 `id` 必须是图示 id。
`setSmartArtText` / `deleteSmartArtPoint` 的 `id` 必须是点 id。`addSmartArtPoint` 的 `id` 是图示 id 或点 id。
`setSmartArtPos` / `setSmartArtWrap` 在 xlsx 上报 `InvalidField`。xlsx 改锚点请用 `setSmartArtAnchor`。

工作表超链接挂在该 `worksheet` 上，不挂在单元格节点上。`{n}` 从 1 起，按该表 `<hyperlinks>` 里 `<hyperlink>` 的文档顺序（ECMA-376 §18.3.1.47）。
增删之后 `{n}` 会变，只够同一次 inspect 之后立刻 apply。只认工作表 XML 里的 `<hyperlink>`。
`HYPERLINK()` 公式、图上 `a:hlinkClick`、文本框 / SmartArt 上的链不进树。
外部链的关系写在**工作表** `.rels`，Type 以 `/hyperlink` 结尾，`TargetMode` 为 `External`。内部位置只写 `@location`，不写关系。`setCell` 改格子显示文本，不改 `<hyperlinks>`。

`insertHyperlink` 的 `id` 必须是 `sheet:{name}`，且该表 `kind` 为 `worksheet`。
对超链接 id、单元格 id、图 id 或 `xlsx:workbook` 发 `insertHyperlink` 报 `InvalidId`。对 `chartsheet` 发此 op 报 `InvalidField`。
对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。`setHyperlink` / `deleteHyperlink` 的 id 必须是 xlsx 超链接 id。
xlsx 上出现 `text` / `anchor` 报 `InvalidField`（Word 字段；内部位置用 `location`，显示字走 `setCell`）。

工作表迷你图挂在该 `worksheet` 上，不挂在单元格节点上。`{n}` 从 1 起，按该表 `extLst` 里全部 `x14:sparklineGroup` 的文档顺序（MS-XLSX §2.4.8 / §2.2.4.12，
URI `{05C60535-1F16-4FD2-B633-F4F36F0B64E0}`，大小写不敏感）。
增删之后 `{n}` 会变，只够同一次 inspect 之后立刻 apply。只认该扩展里的组。图表、条件格式色阶、单元格数字都不算迷你图。数据范围当字符串，不求值。不把折线画进树。

`insertSparkline` 的 `id` 必须是 `sheet:{name}`，且该表 `kind` 为 `worksheet`。
对迷你图组 id、单元格 id、图 id 或 `xlsx:workbook` 发 `insertSparkline` 报 `InvalidId`。对 `chartsheet` 发此 op 报 `InvalidField`。
对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。`setSparkline` / `deleteSparkline` 的 id 必须是迷你图组 id。

工作表嵌入对象挂在该 `worksheet` 上。`{shapeId}` 取 `oleObject/@shapeId` 原文。没有该属性时用 `ole:index:{k}`，树上 `"stable": false`。
`{k}` 只在该表被收入的嵌入对象内从 1 计。只认工作表 `<oleObjects>` 里的 `oleObject`，
且 `r:id` 能对上非 External、Type 后缀为 `oleObject` 或 `package` 的**工作表**关系。
有 `@link` 的链接对象、ActiveX `controls` 不进树。`mc:AlternateContent` 只进第一个 `Choice`。页眉、`chartsheet`、组合里的 OLE 本期不进树。
预览图不进 `pictures`。`insertOle` 的 `id` 必须是 `sheet:{name}`。`deleteOle` 的 `id` 必须是 OLE id。对不上的 id 形态报 `InvalidId`。
不提供改尺寸、位置、图标或 ProgID 的 op。create 本期不写 OLE。docx / pptx 见各自分册。

工作表 3D 模型挂在该 `worksheet` 上。`{cNvPrId}` 取 Choice 里 `xdr:graphicFrame` 的 `xdr:cNvPr/@id` 原文。
没有该属性时用 `model3d:index:{k}`，树上 `"stable": false`。`{k}` 只在该表被收入的 3D 模型内从 1 计。
只认 `mc:AlternateContent` 且 `Choice Requires` 含 `am3d`，Choice 里 `a:graphicData/@uri` 为
`http://schemas.microsoft.com/office/drawing/2017/model3d`（可带尾斜杠）。
只走 `Choice Requires="am3d"`。Fallback 海报 `pic` 与 `am3d:raster` 用的 PNG **不进** `pictures`，也不计入 `pictureCount`。
组合里符合条件的 `graphicFrame` 要展开，组合本身不进树。页眉、`chartsheet` 本期不进树。
`insertModel3d` 的 `id` 必须是 `sheet:{name}`。`replaceModel3d` / `deleteModel3d` 的 `id` 必须是 3D 模型 id。
对图 id、OLE id 或 `xlsx:workbook` 发这些 op 报 `InvalidId`。对 `chartsheet` 报 `InvalidField`。
只认 GLB：文件头小端 `glTF`（`0x46546C67`），version=2。不是 GLB 报 `InvalidImageFile`（对外 `InvalidField`）。
`contentType` 恒为 `model/gltf.binary`。不把 GLB、相机、灯光摊进树。create 可用 `sheets[].model3ds`。
docx / pptx 见各自分册。

工作表切片器挂在该 `worksheet` 上。`{cNvPrId}` 取 drawing 里 slicer `graphicFrame` 的 `cNvPr/@id` 原文。
没有该属性时用 `slicer:index:{k}`，树上 `"stable": false`。`{k}` 只在该表被收入的切片器内从 1 计。
只认 `a:graphicData/@uri` 为 `http://schemas.microsoft.com/office/drawing/2010/slicer`（可带尾斜杠）的 `xdr:graphicFrame`，
且 `sle:slicer/@name` 能对上该表 slicers 部件里的 `slicer/@name`。
时间线 `timeslicer`、OLAP、没有 drawing 的孤立 slicer 部件本期不进树。`insertSlicer` 的 `id` 必须是 `sheet:{name}`。
`deleteSlicer` 的 `id` 必须是切片器 id。对不上的 id 形态报 `InvalidId`。不提供改筛选、样式或标题的 op。create 本期不写切片器。不在库里执行筛选或刷新透视。

xlsx 的 `setProtection` 的 `id` 必须是 `sheet:{name}`（该表 `sheetProtection`）或 `xlsx:workbook`（`workbookProtection`）。
对单元格 id、迷你图 id 或图 id 发此 op 报 `InvalidId`。对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。
docx 的 `setProtection` 仍只用 `docx:document`。
pptx 的 `setProtection` 用 `pptx:presentation` 改 `p:modifyVerifier`，放映设置用 `setShowPr`。

`insertPicture` 的 `id` 必须是 `sheet:{name}`（往工作表 drawing 插图）或页眉槽位 id `sheet:{name}/hf:{kind}:{side}`（往该侧 `drawingHF` 插图），且该表 `kind` 为 `worksheet`。
对图 id、图表 id、系列 id、单元格 id 或 `xlsx:workbook` 发 `insertPicture` 报 `InvalidId`。
对 `chartsheet` / `dialogsheet` 发 `insertPicture`（含往其页眉槽位插图）报 `InvalidField`。对工作表 id 发 `insertPicture` 仍只插工作表图，不进页眉。
往页眉槽位插图时禁止出现 `from` / `to` / `anchor` / `fromOffX` / `fromOffY` / `toOffX` / `toOffY`，出现报 `InvalidField`；只要 `file`，可选 `cx` / `cy`。
写入 `oneCell`、从 A1、偏移 0，并把 `drawingHF` 上该侧中区属性写成新图的 `cNvPr` id（奇数页页眉 `cho`，偶数页页眉 `che`，首页页眉 `chf`，奇数页页脚 `cfo`，偶数页页脚 `cfe`，首页页脚 `cff`）。
该中区已被占用则改写左区（`lh*`），再右区（`rh*`）；三区都占用仍插入 drawing，不再写属性，inspect 把它归到 `odd:header`。
表上只有 `legacyDrawingHF`、没有 `drawingHF` 时，新建 DrawingML 的 `drawingHF`，不动 VML。
`replacePicture` / `deletePicture` / `setPictureSize` / `setPictureAnchor` 的 `id` 必须是工作表图 id 或页眉图 id。
对工作表 id、页眉槽位 id、单元格 id 或图表 id 发这四个 op 报 `InvalidId`。对 `chartsheet` / `dialogsheet` 上的页眉图发这四个 op 报 `InvalidField`。
`setPictureWrap` 是 Word 绕排，xlsx 上报 `InvalidField`。`exportMedia` 认工作表图 id 与页眉图 id，写出 `/xl/media/…`。
`exportMedia` 也认工作表 OLE id，写出 `/xl/embeddings/…`，不是预览图。
`exportMedia` 也认工作表 3D 模型 id，写出 GLB 原字节，不是海报。必须在解析图 id 之前分流 `/model3d:`。

`insertChart` 的 `id` 必须是 `sheet:{name}`，且该表 `kind` 为 `worksheet`。
对图 id、图表 id、系列 id、单元格 id 或 `xlsx:workbook` 发 `insertChart` 报 `InvalidId`。
对 `chartsheet` / `dialogsheet` 发 `insertChart` 报 `InvalidField`。`insertChartSheet` 的 `id` 必须是 `xlsx:workbook`。
`deleteChartSheet` 的 `id` 必须是 `sheet:{name}`，且该表 `kind` 为 `chartsheet`。
`deleteChart` / `setChartTitle` / `setChartType` / `setChartAxisTitle` / `setChartLegend` / `setChartDataLabels`
 / `setChartSize` / `setChartAnchor` / `addChartSeries` 的 `id` 必须是图表 id。
`setChartSeries` / `deleteChartSeries` / `insertTrendline` / `insertErrorBars` / `setChartSeriesAxis` 的 `id` 必须是系列 id。
`setTrendline` / `deleteTrendline` 的 `id` 必须是趋势线 id。`setErrorBars` / `deleteErrorBars` 的 `id` 必须是误差线 id。
对不上的 id 形态报 `InvalidId`。`setChartWrap` 是 Word 绕排，xlsx 上报 `InvalidField`。xlsx 改锚点请用 `setChartAnchor`。

数据验证挂在该 `worksheet` 上，不挂在单元格上。`{n}` 从 1 起，按该表 `<dataValidations>/<dataValidation>` 的文档顺序。
增删之后旧的 `dv:{n}` 会变，只够同一次 inspect 之后立刻 apply。不要套单元格 id，也不要从 `sqref` 编 id。
`insertDataValidation` 的 `id` 必须是工作表 id `sheet:{name}`。`setDataValidation` / `deleteDataValidation` 的 `id` 必须是规则 id。
对单元格 id、图 id 或工作表 id 发后两个 op 报 `InvalidId`。只能改 `worksheet`。`extLst` 里的 `x14:dataValidations` 不进树、不接受 apply，原样留下。

条件格式同样挂在该 `worksheet` 上，不挂在单元格上。`{n}` 从 1 起，按该表全部 `<conditionalFormatting>/<cfRule>` 的文档顺序（先块后规则）。
增删之后旧的 `cf:{n}` 会变，只够同一次 inspect 之后立刻 apply。不要套单元格 id，也不要从 `sqref` 编 id。同一 `<conditionalFormatting>` 里多条规则共用该块的 `sqref`。
`insertConditionalFormat` 的 `id` 必须是工作表 id `sheet:{name}`。
`setConditionalFormat` / `deleteConditionalFormat` 的 `id` 必须是规则 id。对单元格 id、图 id、数据验证 id 或工作表 id 发后两个 op 报 `InvalidId`。
只能改 `worksheet`。`extLst` 里的 `x14:conditionalFormattings` 不进树、不接受 apply，原样留下。不渲染色阶，也不求值公式。

工作表表格是 `/xl/tables/tableN.xml` 加该表 `<tableParts>`（ECMA-376 §18.5.1.2），不是 Word 的 `w:tbl`，也不要套单元格 id。
`{n}` 从 1 起，按该表 `<tableParts>/<tablePart>` 的文档顺序。增删之后旧的 `table:{n}` 会变，只够同一次 inspect 之后立刻 apply。
不要用 XML `id` 或 `displayName` 编树 id；这两项作为字段出现。`insertSheetTable` 的 `id` 必须是工作表 id `sheet:{name}`。
`setSheetTable` / `deleteSheetTable` 的 `id` 必须是表格 id。对单元格 id、图 id、数据验证 id、条件格式 id 或工作表 id 发后两个 op 报 `InvalidId`。
只能改 `worksheet`。表上的 `<autoFilter>` 跟表格部件走；工作表级 `<autoFilter>` 见 `setAutoFilter`，不进表格名单。不实现结构化引用求值，也不渲染色阶或表样式。

工作表透视表是 `/xl/pivotTables/pivotTableN.xml` 加该表关系（Type 以 `/pivotTable` 结尾，ECMA-376 §18.10），缓存在 `/xl/pivotCache/`。
`{n}` 从 1 起，按该表这些关系的文档顺序。增删之后旧的 `pivot:{n}` 会变，只够同一次 inspect 之后立刻 apply。不要用 `cacheId` 或 `name` 编树 id；这两项作为字段出现。
`insertPivotTable` 的 `id` 必须是工作表 id `sheet:{name}`。`setPivotTable` / `deletePivotTable` 的 `id` 必须是透视表 id。
对单元格 id、图 id、数据验证 id、条件格式 id、表格 id 或工作表 id 发后两个 op 报 `InvalidId`。只能改 `worksheet`。
只读写工作表源（`cacheSource/@type="worksheet"`）。OLAP、外部连接与 consolidation 的缓存不进树、不接受 apply，原样留下。
库按源格子抄缓存记录，并标 `refreshOnLoad`，不在库里聚合，也不把合计写进 `sheetData`。透视图表改数仍按已有 `flags.pivot` 拒绝。

定义名是 workbook `<definedNames>/<definedName>`（ECMA-376 §18.2.5）。`{n}` 从 1 起，按该元素的文档顺序。
增删之后旧的 `name:{n}` 会变，只够同一次 inspect 之后立刻 apply。不要用 `@name` 编树 id：同一工作簿里表级同名可以并存。
`insertDefinedName` 的 `id` 必须是 `xlsx:workbook`。`setDefinedName` / `deleteDefinedName` 的 `id` 必须是定义名 id。
对单元格 id、工作表 id、透视表 id 或 `xlsx:workbook` 发后两个 op 报 `InvalidId`。打印区域 `_xlnm.Print_Area` 当普通定义名。公式当字符串，不解析求值。不实现动态数组 spill。

外部数据连接挂在工作簿上，不是某张表。`{xmlId}` 取 `/xl/connections.xml` 里 `connection/@id` 原文（ECMA-376 §18.13.1）。
没有 `@id` 时用 `connection:index:{k}`，树上 `"stable": false`。`{k}` 只在被收入的连接内从 1 计。`deleted="true"` 的连接不进树。
只读 Connections 部件（至多一份）。queryTable、Power Query、ODBC / OLE DB / OLAP 本期不另编 id；inspect 见到其它 `type` 仍列入，`kind` 为 `"other"`。
`insertConnection` 的 `id` 必须是 `xlsx:workbook`。`deleteConnection` 的 `id` 必须是连接 id。对不上的 id 形态报 `InvalidId`。不提供改连接或刷新的 op。
create 本期不写连接。不向数据源发请求，不把查询结果写进 `sheetData`。

外部工作簿引用挂在工作簿上，不是某张表。id 为 `extlink:{n}`，`{n}` 从 1 起，
按 `/xl/workbook.xml` 里 `<externalReferences>/<externalReference>` 的文档顺序（ECMA-376 §18.2.8 / §18.14）。
与单元格公式里的 `[n]` 同一套下标。增删之后旧的 `extlink:{n}` 会变，只够同一次 inspect 之后立刻 apply。不要用关系 `r:id` 或目标文件名编树 id。
`insertExternalLink` 的 `id` 必须是 `xlsx:workbook`。`deleteExternalLink` 的 `id` 必须是 `extlink:{n}`。对不上的 id 形态报 `InvalidId`。
不提供改路径的 op。create 本期不写外部引用。不打开目标文件，不求值。DDE / OLE 外部链只读进树。不要和 `connections.xml` 混用。

自定义视图挂在工作簿上。`{n}` 从 1 起，按 `<customWorkbookView>` 文档顺序（ECMA-376 §18.2.3 / §18.2.4）。
增删之后旧的 `customView:{n}` 会变，只够同一次 inspect 之后立刻 apply。不要用 `@name` 或 guid 编树 id。
`insertCustomView` 的 `id` 必须是 `xlsx:workbook`。`deleteCustomView` / `showCustomView` 的 `id` 必须是自定义视图 id。
对不上的 id 形态报 `InvalidId`。create 本期不写自定义视图。不按视图恢复隐藏行 / 列。


## inspect

xlsx、docx、pptx 共用请求字段名 `scope`，里面的键按格式分，混用报 `InvalidField`。请求信封见 [json-contract.md](json-contract.md#请求)。

### scope

- **`path`**（是）
  - 已有文件
- **`scope`**（否）
  - 省略则只返回工作表名单，不含单元格
- **`scope.sheet`**（要单元格时必填）
  - 工作表名
- **`scope.range`**（要单元格时必填）
  - A1 区域，例如 `A1:D20`
  - 单个单元格写成 `B3` 或 `B3:B3`
- **`scope.maxCells`**（否）
  - 返回单元格上限，默认 `5000`，允许范围 `1` 到 `20000`
- **`scope.pictures`**（否）
  - 为 `true` 时给每张 `worksheet` 填工作表 `pictures`
  - 省略或 `false` 则只有 `pictureCount`，没有工作表图名单
  - 不读媒体字节
  - 不依赖 `sheet` / `range`
  - 单独给此项不会展开页眉图
- **`scope.charts`**（否）
  - 为 `true` 时给每张 `worksheet` 与 `chartsheet` 填 `charts`
  - 省略或 `false` 则只有 `chartCount`，没有名单
  - 不把 chart 部件或媒体字节放进树
  - 不依赖 `sheet` / `range` / `pictures`
- **`scope.headerFooter`**（否）
  - 为 `true` 时给每张 `worksheet` / `chartsheet` / `dialogsheet` / `macrosheet` 填六个页眉页脚槽位（`headerFooters`）
  - 没有的槽位仍列出，`text` 为 `null`
  - 省略或 `false` 则只有 `hasHeaderFooter`，不倒出原文，也没有 `headerFooters`
  - 不依赖 `sheet` / `range` / `pictures` / `charts`
- **`scope.comments`**（否）
  - 为 `true` 时给每张 `worksheet` 填 `comments`
  - 没有线程化批注时为空数组
  - 省略或 `false` 则只有 `commentCount`，没有名单
  - 不依赖 `sheet` / `range` / `pictures` / `charts` / `headerFooter`
  - 不读旧式 VML 批注
- **`scope.textBoxes`**（否）
  - 为 `true` 时给每张 `worksheet` 填 `textBoxes`
  - 没有文本框时为空数组
  - 省略或 `false` 则只有 `textBoxCount`，没有名单
  - 不依赖
    - `sheet`
    - `range`
    - `pictures`
    - `charts`
    - `headerFooter`
    - `comments`
  - 不把框内字做成段落树
- **`scope.maths`**（否）
  - 为 `true` 时给每张 `worksheet` 填 `maths`
  - 没有公式框时为空数组
  - 省略或 `false` 则只有 `mathCount`，没有名单
  - 不依赖 `sheet` / `range` / 其它名单键
  - 不把 OMML 原文、不把 `expr` 放进树
  - 页眉、`chartsheet` 本期不进树
  - 不是单元格 `f`
  - docx 的 `scope.maths` 见 [docx 分册](json-contract-docx.md#inspect)
- **`scope.shapes`**（否）
  - 为 `true` 时给每张 `worksheet` 填 `shapes`
  - 没有形状时为空数组
  - 省略或 `false` 则只有 `shapeCount`，没有名单
  - 不依赖 `sheet` / `range` / 其它名单键
  - 不把无 `txBox` 的形状并进 `textBoxes`
- **`scope.connectors`**（否）
  - 为 `true` 时给每张 `worksheet` 填 `connectors`
  - 没有连接线时为空数组
  - 省略或 `false` 则只有 `connectorCount`，没有名单
  - 不依赖 `sheet` / `range` / 其它名单键
  - docx 也认 `scope.connectors`，见 [docx 分册](json-contract-docx.md#scope)
- **`scope.smartArt`**（否）
  - 为 `true` 时给每张 `worksheet` 填 `smartArts`
  - 没有图示时为空数组
  - 省略或 `false` 则只有 `smartArtCount`，没有名单
  - 不把图示画成图，不读 Fallback 位图
  - 不依赖
    - `sheet`
    - `range`
    - `pictures`
    - `charts`
    - `headerFooter`
    - `comments`
    - `textBoxes`
    - `shapes`
    - `connectors`
- **`scope.hyperlinks`**（否）
  - 为 `true` 时给每张 `worksheet` 填 `hyperlinks`
  - 没有链时为空数组
  - 省略或 `false` 则只有 `hyperlinkCount`，没有名单
  - 不依赖
    - `sheet`
    - `range`
    - `pictures`
    - `charts`
    - `headerFooter`
    - `comments`
    - `textBoxes`
    - `shapes`
    - `connectors`
    - `smartArt`
  - 不把 `HYPERLINK()` 公式或图上的链算进来
- **`scope.sparklines`**（否）
  - 为 `true` 时给每张 `worksheet` 填 `sparklines`
  - 没有迷你图组时为空数组
  - 省略或 `false` 则只有 `sparklineCount`，没有名单
  - 不依赖 `sheet` / `range` / 其它名单键
  - 不把折线画进树
- **`scope.oleObjects`**（否）
  - 为 `true` 时给每张 `worksheet` 填 `oleObjects`
  - 没有嵌入对象时为空数组
  - 省略或 `false` 则只有 `oleObjectCount`，没有名单
  - 不依赖 `sheet` / `range` / 其它名单键
  - 不把嵌入字节或预览图放进树
  - 页眉、`chartsheet`、组合里的 OLE 本期不进树
  - docx / pptx 见各自分册
- **`scope.model3ds`**（否）
  - 为 `true` 时给每张 `worksheet` 填 `model3ds`
  - 没有 3D 模型时为空数组
  - 省略或 `false` 则只有 `model3dCount`，没有名单
  - 不依赖 `sheet` / `range` / 其它名单键
  - 不把 GLB 或海报放进树
  - 页眉、`chartsheet` 本期不进树
  - docx / pptx 见各自分册
- **`scope.slicers`**（否）
  - 为 `true` 时给每张 `worksheet` 填 `slicers`
  - 没有切片器时为空数组
  - 省略或 `false` 则只有 `slicerCount`，没有名单
  - 不依赖 `sheet` / `range` / 其它名单键
  - 不把 cache 选项整表倒进树
  - 时间线、OLAP 本期不进树
- **`scope.connections`**（否）
  - 为 `true` 时给工作簿根填 `connections`
  - 没有连接时为空数组
  - 省略或 `false` 则只有 `connectionCount`，没有名单
  - 不依赖 `sheet` / `range` / 其它名单键
  - 不刷新，不把查询结果放进树
  - docx / pptx 的 `scope` 里出现 `connections` 报 `InvalidField`
- **`scope.externalLinks`**（否）
  - 为 `true` 时给工作簿根填 `externalLinks`
  - 没有外部引用时为空数组
  - 省略或 `false` 则只有 `externalLinkCount`，没有名单
  - 不依赖 `sheet` / `range` / 其它名单键
  - 不打开目标文件
  - docx / pptx 的 `scope` 里出现 `externalLinks` 报 `InvalidField`
- **`scope.customViews`**（否）
  - 为 `true` 时给工作簿根填 `customViews`
  - 没有自定义视图时为空数组
  - 省略或 `false` 则只有 `customViewCount`，没有名单
  - 不依赖 `sheet` / `range` / 其它名单键
  - 不把 guid 或各表 `customSheetView` 细节放进树
  - docx / pptx 的 `scope` 里出现 `customViews` 报 `InvalidField`

省略 `scope`、或只给空对象：返回工作簿节点和表名单，每张表不含 `cells`，
也没有 `pictures` / `charts` / `headerFooters` / `textBoxes` / `maths` / `shapes` / `connectors` / `smartArts` / `comments`
 / `hyperlinks` / `sparklines` / `oleObjects` / `model3ds` / `slicers`。
工作簿与每张 `worksheet` 仍有 `pictureCount`、`textBoxCount`、`mathCount`、`shapeCount`、`connectorCount`、`smartArtCount`。
工作簿、每张 `worksheet` 与每张 `chartsheet` 仍有 `chartCount`。
工作簿与每张 `worksheet` 仍有 `commentCount`、`hyperlinkCount`、`sparklineCount`、`oleObjectCount`、`model3dCount` 与 `slicerCount`。
工作簿仍有 `connectionCount`、`externalLinkCount`、`customViewCount`、`definedNameCount` 与 `definedNames`。
没有 `connections` / `externalLinks` / `customViews` 名单。每张表仍有 `hasHeaderFooter`。
每张 `worksheet` / `chartsheet` 仍有 `print` 与 `view`。每张 `worksheet` 仍有 `autoFilter`。每张 `worksheet` 仍有 `outline`。
工作簿仍有 `workbookProtection`。每张 `worksheet` 与 `chartsheet` 仍有 `sheetProtection`。这是默认截断。

只要想看单元格，就必须同时给 `sheet` 和 `range`。不默认扫第一张表的整张纸。
`scope.sheet` 点名 `chartsheet` / `dialogsheet` / `macrosheet` 报 `InvalidField`。只要想看工作表图名单，就给 `pictures: true`。
只要想看图表名单，就给 `charts: true`。只要想看页眉原文，就给 `headerFooter: true`。只要想看文本框名单，就给 `textBoxes: true`。只要想看公式框名单，就给 `maths: true`。只要想看形状名单，就给 `shapes: true`。
只要想看连接线名单，就给 `connectors: true`。只要想看 SmartArt 名单，就给 `smartArt: true`。只要想看批注名单，就给 `comments: true`。
只要想看超链接名单，就给 `hyperlinks: true`。只要想看迷你图名单，就给 `sparklines: true`。只要想看嵌入对象名单，就给 `oleObjects: true`。
只要想看 3D 模型名单，就给 `model3ds: true`。
只要想看切片器名单，就给 `slicers: true`。只要想看连接名单，就给 `connections: true`。只要想看外部引用名单，就给 `externalLinks: true`。
只要想看自定义视图名单，就给 `customViews: true`。这些键彼此独立，可只开一边。页眉图名单出现在已展开的槽位上，须同时 `headerFooter: true` 与 `pictures: true`。


### 响应

```json
{
  "schemaVersion": 1,
  "format": "xlsx",
  "id": "xlsx:workbook",
  "pictureCount": 0,
  "chartCount": 0,
  "textBoxCount": 0,
  "mathCount": 0,
  "shapeCount": 0,
  "connectorCount": 0,
  "smartArtCount": 0,
  "commentCount": 0,
  "hyperlinkCount": 0,
  "sparklineCount": 0,
  "oleObjectCount": 0,
  "model3dCount": 0,
  "slicerCount": 0,
  "connectionCount": 0,
  "externalLinkCount": 0,
  "customViewCount": 0,
  "definedNameCount": 0,
  "props": {
    "title": "",
    "subject": null,
    "creator": "ooxp",
    "description": null,
    "keywords": null,
    "category": null,
    "lastModifiedBy": null,
    "created": null,
    "modified": null,
    "company": null,
    "manager": null,
    "custom": []
  },
  "workbookProtection": {
    "lockStructure": false,
    "lockWindows": false,
    "lockRevision": false,
    "hasPassword": false
  },
  "sheets": [
    {
      "id": "sheet:Sheet1",
      "name": "Sheet1",
      "kind": "worksheet",
      "hidden": false,
      "pictureCount": 0,
      "chartCount": 0,
      "textBoxCount": 0,
      "mathCount": 0,
      "shapeCount": 0,
      "connectorCount": 0,
      "smartArtCount": 0,
      "dataValidationCount": 0,
      "conditionalFormatCount": 0,
      "tableCount": 0,
      "pivotCount": 0,
      "commentCount": 0,
      "hyperlinkCount": 0,
      "sparklineCount": 0,
      "oleObjectCount": 0,
      "model3dCount": 0,
      "slicerCount": 0,
      "hasHeaderFooter": false,
      "print": {
        "left": 0.7,
        "right": 0.7,
        "top": 0.75,
        "bottom": 0.75,
        "header": 0.3,
        "footer": 0.3,
        "paperSize": 1,
        "orientation": "portrait",
        "scale": 100,
        "fitToPage": false,
        "fitToWidth": 1,
        "fitToHeight": 1,
        "gridLines": false,
        "headings": false,
        "hCenter": false,
        "vCenter": false,
        "printArea": null,
        "rowBreaks": [],
        "colBreaks": []
      },
      "view": {
        "zoom": 100,
        "frozenRows": 0,
        "frozenCols": 0,
        "splitRows": 0,
        "splitCols": 0,
        "activeCell": "A1"
      },
      "autoFilter": null,
      "sortState": null,
      "outline": {
        "summaryBelow": true,
        "summaryRight": true,
        "rows": [],
        "cols": []
      },
      "sheetProtection": {
        "enforced": false,
        "hasPassword": false,
        "objects": false,
        "scenarios": false,
        "formatCells": true,
        "formatColumns": true,
        "formatRows": true,
        "insertColumns": true,
        "insertRows": true,
        "insertHyperlinks": true,
        "deleteColumns": true,
        "deleteRows": true,
        "selectLockedCells": false,
        "sort": true,
        "autoFilter": true,
        "pivotTables": true,
        "selectUnlockedCells": false
      },
      "truncated": true,
      "range": "A1:D20",
      "cells": [
        {
          "id": "cell:Sheet1!A1",
          "ref": "A1",
          "type": "number",
          "value": 1,
          "formula": null,
          "numFmt": "General"
        },
        {
          "id": "cell:Sheet1!B1",
          "ref": "B1",
          "type": "text",
          "value": "hello",
          "formula": null,
          "numFmt": "General"
        },
        {
          "id": "cell:Sheet1!C1",
          "ref": "C1",
          "type": "number",
          "value": 6,
          "formula": "=SUM(A1:A10)",
          "numFmt": "General"
        }
      ],
      "cols": [],
      "rows": [],
      "merges": [],
      "dataValidations": [],
      "conditionalFormats": [],
      "tables": [],
      "pivots": []
    },
    {
      "id": "sheet:Sheet2",
      "name": "Sheet2",
      "kind": "worksheet",
      "hidden": false,
      "pictureCount": 0,
      "chartCount": 0,
      "textBoxCount": 0,
      "mathCount": 0,
      "shapeCount": 0,
      "connectorCount": 0,
      "smartArtCount": 0,
      "dataValidationCount": 0,
      "conditionalFormatCount": 0,
      "tableCount": 0,
      "pivotCount": 0,
      "commentCount": 0,
      "hyperlinkCount": 0,
      "sparklineCount": 0,
      "oleObjectCount": 0,
      "model3dCount": 0,
      "slicerCount": 0,
      "hasHeaderFooter": false,
      "print": {
        "left": 0.7,
        "right": 0.7,
        "top": 0.75,
        "bottom": 0.75,
        "header": 0.3,
        "footer": 0.3,
        "paperSize": 1,
        "orientation": "portrait",
        "scale": 100,
        "fitToPage": false,
        "fitToWidth": 1,
        "fitToHeight": 1,
        "gridLines": false,
        "headings": false,
        "hCenter": false,
        "vCenter": false,
        "printArea": null,
        "rowBreaks": [],
        "colBreaks": []
      },
      "view": {
        "zoom": 100,
        "frozenRows": 0,
        "frozenCols": 0,
        "splitRows": 0,
        "splitCols": 0,
        "activeCell": "A1"
      },
      "autoFilter": null,
      "sortState": null,
      "outline": {
        "summaryBelow": true,
        "summaryRight": true,
        "rows": [],
        "cols": []
      },
      "sheetProtection": {
        "enforced": false,
        "hasPassword": false,
        "objects": false,
        "scenarios": false,
        "formatCells": true,
        "formatColumns": true,
        "formatRows": true,
        "insertColumns": true,
        "insertRows": true,
        "insertHyperlinks": true,
        "deleteColumns": true,
        "deleteRows": true,
        "selectLockedCells": false,
        "sort": true,
        "autoFilter": true,
        "pivotTables": true,
        "selectUnlockedCells": false
      }
    }
  ],
  "definedNames": []
}
```

规则：

- 工作簿字段顺序：`id`、`pictureCount`、`chartCount`、`textBoxCount`、`mathCount`、`shapeCount`、`connectorCount`、`smartArtCount`、`commentCount`、
  `hyperlinkCount`、`sparklineCount`、`oleObjectCount`、`model3dCount`、`slicerCount`、`connectionCount`、`externalLinkCount`、
  `customViewCount`、`definedNameCount`、`props`、`workbookProtection`、`sheets`、`definedNames`，
  有连接名单时再跟 `connections`，有外部引用名单时再跟 `externalLinks`，有自定义视图名单时再跟 `customViews`。
  `pictureCount` 始终出现，等于各 `worksheet` 的 `pictureCount` 之和，不含 `chartsheet` / `dialogsheet` / `macrosheet`。
  `chartCount` 始终出现，等于各 `worksheet` 与 `chartsheet` 的 `chartCount` 之和，不含 `dialogsheet` / `macrosheet`。
  `textBoxCount` 始终出现，等于各 `worksheet` 的 `textBoxCount` 之和。`mathCount` 始终出现，等于各 `worksheet` 的 `mathCount` 之和。`shapeCount` 始终出现，等于各 `worksheet` 的 `shapeCount` 之和。
  `connectorCount` 始终出现，等于各 `worksheet` 的 `connectorCount` 之和。`smartArtCount` 始终出现，等于各 `worksheet` 的 `smartArtCount` 之和。
  `commentCount` 始终出现，等于各 `worksheet` 的 `commentCount` 之和。`hyperlinkCount` 始终出现，等于各 `worksheet` 的 `hyperlinkCount` 之和。
  `sparklineCount` 始终出现，等于各 `worksheet` 的 `sparklineCount` 之和。`oleObjectCount` 始终出现，等于各 `worksheet` 的 `oleObjectCount` 之和。
  `model3dCount` 始终出现，等于各 `worksheet` 的 `model3dCount` 之和。
  `slicerCount` 始终出现，等于各 `worksheet` 的 `slicerCount` 之和。
  `connectionCount` 始终出现，是 `/xl/connections.xml` 里被收入的 `connection` 个数（判定见上文外部数据连接 id 那节）。没有则为 `0`。
  `externalLinkCount` 始终出现，是 workbook `<externalReferences>` 里 `<externalReference>` 的个数。没有则为 `0`。
  `customViewCount` 始终出现，是 workbook `<customWorkbookViews>` 里 `<customWorkbookView>` 的个数；没有则为 `0`。
  `definedNameCount` 始终出现，是 workbook `<definedNames>` 里 `<definedName>` 的个数。没有则为 `0`。
  `definedNames` 始终列出，可为空数组，不另加 scope 键。`props` 始终出现，规则见 [json-contract.md](json-contract.md#文档属性三格式共用)。
  `workbookProtection` 始终出现，规则见下文「工作簿保护」。xlsx 根上不要带 docx 那个 `protection`。
- `definedNames` 每项字段顺序：`id`、`name`、`formula`、`hidden`、`localSheet`。`name` 取 `@name`，没有则为 `""`。
  `formula` 是元素文本，当字符串，不解析求值、不补 `=`。`hidden` 为布尔：`@hidden` 为 `1` / `true` / `TRUE` / `on` 则为 `true`，缺省或其余为 `false`。
  `localSheet` 有 `@localSheetId` 则按 0 基下标解成 `sheets` 里对应表的 `name`；解不出、缺属性或无法解析则为 `null`（簿级）。打印区域 `_xlnm.Print_Area` 与其它内置名同样列出。
- `sheets` 始终列出工作簿里全部表（含隐藏表），便于模型知道还有哪些表。未点名的表不带 `cells`、`range`、`truncated`、`cols`、`rows`、`merges`、`dataValidations`、
  `conditionalFormats`、`tables`、`pivots`。
- `kind`：`worksheet`、`chartsheet`、`dialogsheet`、`macrosheet`。第一期只对 `worksheet` 填单元格与图。
  `worksheet` 与 `chartsheet` 都填 `chartCount`。
  每张表始终填 `hidden`：`sheet/@state` 为 `hidden` 或 `veryHidden` 则为 `true`，缺省、空或 `visible` 为 `false`。
  每张表始终填 `hasHeaderFooter`：该表 `headerFooter` 里任一槽位有非空文本则为 `true`；没有该元素、空的 `<headerFooter/>`、或六个槽位都空则为 `false`。
  不算 VML，也不因为只有 `drawingHF` 图就把此项改成 `true`。每张 `worksheet` 与 `chartsheet` 始终填 `print`（对象，不另编 id）与 `view`（对象，不另编 id）。
  每张 `worksheet` 始终填 `autoFilter`（字符串或 `null`）。每张 `worksheet` 始终填 `sortState`（对象或 `null`，不另编 id）。
  每张 `worksheet` 始终填 `outline`（对象，不另编 id）。每张 `worksheet` 与 `chartsheet` 始终填 `sheetProtection`（对象，不另编 id）。
  `dialogsheet` / `macrosheet` 没有 `print` / `view` / `autoFilter` / `sortState` / `outline` / `sheetProtection`。
  `chartsheet` 没有 `autoFilter`，也没有 `sortState`，也没有 `outline`。
- `worksheet` 字段顺序：`id`、`name`、`kind`、`hidden`、`pictureCount`、`chartCount`、`textBoxCount`、`mathCount`、`shapeCount`、`connectorCount`、
  `smartArtCount`、`dataValidationCount`、`conditionalFormatCount`、`tableCount`、`pivotCount`、`commentCount`、
  `hyperlinkCount`、`sparklineCount`、`oleObjectCount`、`model3dCount`、`slicerCount`、`hasHeaderFooter`、`print`、`view`、`autoFilter`、
  `sortState`、`outline`、`sheetProtection`，
  有页眉名单时再跟 `headerFooters`，有图名单时再跟 `pictures`，有图表名单时再跟 `charts`，有文本框名单时再跟 `textBoxes`，有公式框名单时再跟 `maths`，有形状名单时再跟 `shapes`，
  有连接线名单时再跟 `connectors`，有 SmartArt 名单时再跟 `smartArts`，有批注名单时再跟 `comments`，有超链接名单时再跟 `hyperlinks`，有迷你图名单时再跟 `sparklines`，
  有嵌入对象名单时再跟 `oleObjects`，有 3D 模型名单时再跟 `model3ds`，有切片器名单时再跟 `slicers`，
  有单元格切片时再跟 `truncated`、`range`、`cells`、`cols`、`rows`、`merges`、`dataValidations`、`conditionalFormats`、`tables`、`pivots`。
  `pictureCount` 始终出现，是该表工作表 drawing 里被收入的 `xdr:pic` 个数（判定见上文 xlsx 图 id 那节），不含页眉图。没有图则为 `0`。
  `chartCount` 始终出现，是该表被收入的图表个数（判定见上文 xlsx 图表 id 那节）。没有图表则为 `0`。`textBoxCount` 始终出现，是该表被收入的文本框个数（判定见上文 xlsx 文本框 id 那节）。
  没有则为 `0`。`mathCount` 始终出现，是该表被收入的公式框个数（判定见上文 xlsx 公式框 id 那节）。没有则为 `0`。`shapeCount` 始终出现，是该表被收入的形状个数（判定见上文 xlsx 形状 id 那节）。没有则为 `0`。
  `connectorCount` 始终出现，是该表被收入的连接线个数（判定见上文 xlsx 连接线 id 那节）。没有则为 `0`。
  `smartArtCount` 始终出现，是该表被收入的 SmartArt 个数（判定见上文 xlsx SmartArt id 那节）。没有则为 `0`。
  `dataValidationCount` 始终出现，是该表 `<dataValidations>` 里 `<dataValidation>` 的个数，不含 `extLst` 里的 `x14:dataValidations`。
  没有则为 `0`。`conditionalFormatCount` 始终出现，是该表全部 `<conditionalFormatting>` 里 `<cfRule>` 的个数，
  不含 `extLst` 里的 `x14:conditionalFormattings`。
  没有则为 `0`。`tableCount` 始终出现，是该表 `<tableParts>` 里 `<tablePart>` 的个数。没有则为 `0`。
  `pivotCount` 始终出现，是该表关系里 Type 以 `/pivotTable` 结尾的条目个数。没有则为 `0`。
  `commentCount` 始终出现，是该表线程化批注部件里没有 `parentId` 的 `threadedComment` 个数。没有则为 `0`。
  `hyperlinkCount` 始终出现，是该表 `<hyperlinks>` 里 `<hyperlink>` 的个数。没有则为 `0`。
  `sparklineCount` 始终出现，是该表 `extLst` 里 `x14:sparklineGroup` 的组数。没有则为 `0`。
  `oleObjectCount` 始终出现，是该表被收入的嵌入对象个数（判定见上文 xlsx OLE id 那节）。没有则为 `0`。
  `model3dCount` 始终出现，是该表被收入的 3D 模型个数（判定见上文 xlsx 3D 模型 id 那节）。没有则为 `0`。
  `slicerCount` 始终出现，是该表被收入的切片器个数（判定见上文 xlsx 切片器 id 那节）。没有则为 `0`。`sheetProtection` 始终出现，规则见下文「工作表保护」。
  `chartsheet` 没有 `pictureCount`，也没有 `pictures`，也没有 `textBoxCount` / `textBoxes`，也没有 `mathCount` / `maths`，也没有 `shapeCount` / `shapes`，
  也没有 `connectorCount` / `connectors`，也没有 `smartArtCount` / `smartArts`，也没有 `dataValidationCount`，
  也没有 `conditionalFormatCount`，也没有 `tableCount`，也没有 `pivotCount`，也没有 `commentCount`，也没有 `hyperlinkCount`，
  也没有 `sparklineCount`，也没有 `oleObjectCount`，也没有 `model3dCount`，也没有 `autoFilter`，也没有 `sortState`，也没有 `outline`；
  字段顺序：`id`、`name`、`kind`、`hidden`、`chartCount`、`hasHeaderFooter`、`print`、`view`、`sheetProtection`，
  有页眉名单时再跟 `headerFooters`，有图表名单时再跟 `charts`。
  `dialogsheet` / `macrosheet` 没有 `pictureCount` / `pictures` / `chartCount` / `charts` / `textBoxCount` / `textBoxes`
   / `mathCount` / `maths` / `shapeCount` / `shapes` / `connectorCount` / `connectors` / `smartArtCount` / `smartArts` / `dataValidationCount`
   / `conditionalFormatCount` / `tableCount` / `pivotCount` / `commentCount` / `hyperlinkCount` / `sparklineCount`
   / `oleObjectCount` / `model3dCount` / `print` / `view` / `autoFilter` / `sortState` / `outline` / `sheetProtection`；
  字段顺序：`id`、`name`、`kind`、`hidden`、`hasHeaderFooter`，有页眉名单时再跟 `headerFooters`。
  判定 `kind` 只看 workbook 关系 Type：以 `/chartsheet` 结尾为 `chartsheet`；以 `/dialogsheet` 结尾为 `dialogsheet`；
  以 `/macrosheet`、`/xlMacrosheet` 或 `/xlIntlMacrosheet` 结尾为 `macrosheet`（国际宏表不另开 `kind`）；其余为 `worksheet`。
  不要看部件路径或根元素名。Excel 16.0 另存 `.xlsx` 会把宏表写成普通 `worksheet`，对话框表能留下（常见路径 `/xl/dialogsheets/sheetN.xml`，
  CT `application/vnd.openxmlformats-officedocument.spreadsheetml.dialogsheet+xml`，根 `<dialogsheet>`，ECMA-376 §12.3.7 / §18.3.1.34）。
  宏表常见于 `.xlsm`：路径 `/xl/macrosheets/…`，Type `http://schemas.microsoft.com/office/2006/relationships/xlMacrosheet` 或
  `http://schemas.microsoft.com/office/2006/relationships/xlIntlMacrosheet`，
  CT `application/vnd.ms-excel.macrosheet+xml` / `application/vnd.ms-excel.intlmacrosheet+xml`，
  根 `<xm:macrosheet>`（`xmlns:xm="http://schemas.microsoft.com/office/excel/2006/main"`）。
  本仓库 `format` 只认 `xlsx`，不打开 `.xlsm`；`.xlsx` 包里若已有这些部件，inspect 仍按 Type 列入。
- `print` 字段顺序：`left`、`right`、`top`、`bottom`、`header`、`footer`、`paperSize`、`orientation`、`scale`、`fitToPage`、`fitToWidth`、
  `fitToHeight`、`gridLines`、`headings`、`hCenter`、`vCenter`、`printArea`、`rowBreaks`、`colBreaks`。
  边距是英寸数字，与 XML `pageMargins`（§18.3.1.62）相同；没有该元素时用缺省 `left`/`right` 0.7、`top`/`bottom` 0.75、`header`/`footer` 0.3。
  `paperSize` 是 ECMA 整数（1 为 Letter，9 为 A4）；缺省为 1。
  `orientation` 为 `portrait` 或 `landscape`；XML 为 `default` 或缺省都报 `portrait`。`scale` 缺省 100。
  `fitToPage` 取 `sheetPr/pageSetUpPr/@fitToPage`，缺省 `false`。`fitToWidth` / `fitToHeight` 缺省 1。
  `gridLines` 仅当 `printOptions` 的 `gridLines` 与 `gridLinesSet` 都为真时为 `true`（`gridLinesSet` 缺省为真）。
  `headings` / `hCenter` / `vCenter` 取 `printOptions` 对应属性，缺省 `false`。
  `printArea` 取该表级 `_xlnm.Print_Area` 的公式文本；能解析成单一 A1 时规范化成带表名前缀的绝对引用；没有则为 `null`。
  `rowBreaks` / `colBreaks` 是从 1 计的整数数组，只列入 `man="1"` 的 `<brk>`（XML `@id` 从 0 计，树上加 1）；按升序。
  `chartsheet` 没有 `printOptions` / `rowBreaks` / `colBreaks` / `pageSetUpPr`，也没有 `scale` / `fitToWidth` / `fitToHeight`；这些键仍列出，值为上述缺省。
- `view` 字段顺序：`zoom`、`frozenRows`、`frozenCols`、`splitRows`、`splitCols`、`activeCell`。只读第一个 `sheetView`（§18.3.1.87）。
  `zoom` 取 `@zoomScale`，缺省 100。`frozenRows` / `frozenCols` 仅当 `<pane>/@state` 为 `frozen` 或 `frozenSplit` 时取 `ySplit` / `xSplit`（四舍五入成非负整数）；
  `split`、缺省或没有 `<pane>` 则为 0。
  `splitRows` / `splitCols` 仅当 pane 为拆分（无 `state` 或 `state="split"`，且 `xSplit` / `ySplit` 至少一个大于 0）时，取 `topLeftCell` 的行号减 1 / 列号减 1；
  没有 `topLeftCell` 或不是拆分则为 0。
  不要用 1/20 磅反推行列。拆分与冻结互斥：有拆分则冻结为 0，有冻结则拆分为 0。
  `activeCell` 取与 `activePane` 对应的 `<selection>/@activeCell`，没有则取第一条 `selection` 的 `activeCell` 或 `sqref` 首格；再没有则为 `A1`。
  能解析成 A1 的规范化成列字母加大写行号。`chartsheet` 没有 `<pane>` / `<selection>`；
  `frozenRows` / `frozenCols` / `splitRows` / `splitCols` 仍为 0，`activeCell` 仍为 `A1`，`zoom` 仍取该表 `sheetView/@zoomScale`。
- `autoFilter` 取工作表级 `<autoFilter>/@ref`（§18.3.1.2）。能解析成单一 A1 时规范化成 `A1` 或 `A1:B2`；解析不了的原样留下；没有该元素则为 `null`。
  不算表格部件里的 `<autoFilter>`，也不报 `filterColumn`。
- `sortState` 不要另给 scope 键。来源是该表工作表级 `<sortState>`（ECMA-376 §18.3.1.92）。没有该元素则为 `null`。不算表格部件里的排序，也不报 `x14` 扩展排序。
  对象字段顺序：`ref`、`caseSensitive`、`columnSort`、`columns`。
  `ref` 取 `@ref`，能解析成单一 A1 时规范化成 `A1` 或 `A1:B2`；解析不了的原样留下；缺属性则为 `null`。
  `caseSensitive` / `columnSort` 取同名属性：`1` / `true` / `TRUE` / `on` 为 `true`，缺省或其余为 `false`。不报 `sortMethod`。
  `columns` 按 `<sortCondition>` 文档顺序。每项字段顺序：`ref`、`descending`。`ref` 取 `@ref`，能解析则规范化；缺属性的条件跳过。
  `descending` 规则与上同，缺省 `false`。不报 `customList` / `dxfId` / `iconSet` / `iconId` / `sortBy`。
- `outline` 不要另给 scope 键。来源是该表 `sheetPr/outlinePr`、
  各 `<row>/@outlineLevel` / `@hidden` / `@collapsed` 与 `<cols>/<col>` 的同名属性（ECMA-376 §18.3.1.61 / §18.3.1.73 / §18.3.1.13）。
  字段顺序：`summaryBelow`、`summaryRight`、`rows`、`cols`。
  `summaryBelow` / `summaryRight`：没有 `outlinePr` 或没有对应属性为 `true`；属性为 `0` / `false` / `FALSE` / `off` 则为 `false`。
  `rows` 列出全部 `outlineLevel>0` 或 `collapsed` 为真的 `<row>`，按行号升序，不按 inspect 范围过滤。每项字段顺序：`r`、`level`、`hidden`、`collapsed`。
  `r` 是行号（从 1 计）。`level` 取 `@outlineLevel`，缺省 0。
  `hidden` / `collapsed` 为布尔：属性为 `1` / `true` / `TRUE` / `on` 则为 `true`，缺省或其余为 `false`。
  `cols` 列出全部带 `outlineLevel>0` 或 `collapsed` 为真的 `<col>`，按 `min` 升序。每项字段顺序：`min`、`max`、`level`、`hidden`、`collapsed`。
  `min` / `max` 是列字母。`level` 取 `@outlineLevel`，缺省 0。不报列宽。布局名单 `cols` / `rows` 仍不报隐藏与大纲。
  `groupCols` 新建 `<col>` 时会写缺省 `width`，因此布局 `cols` 可能列出这些列的列宽。
  不把 `sheetFormatPr/@outlineLevelRow` / `@outlineLevelCol` 放进树。
- `sheetProtection` 不要另给 scope 键。来源是该表 `<sheetProtection>`（工作表 ECMA-376 §18.3.1.85 `CT_SheetProtection`；
  图表工作表 §18.3.1.86 `CT_ChartsheetProtection`）。
  没有该元素时按规范缺省填全字段，`enforced` 为 `false`，`hasPassword` 为 `false`。不把哈希、盐、`algorithmName`、`spinCount` 或明文密码放进树。单元格 `locked` 是样式位，不在本对象里。
- 工作表 `sheetProtection` 字段顺序：`enforced`、`hasPassword`、`objects`、`scenarios`、`formatCells`、`formatColumns`、`formatRows`、
  `insertColumns`、`insertRows`、`insertHyperlinks`、`deleteColumns`、`deleteRows`、`selectLockedCells`、`sort`、`autoFilter`、
  `pivotTables`、`selectUnlockedCells`。
  `enforced` 取 `@sheet`：`1` / `true` / `TRUE` / `on` 为 `true`，缺属性或其余为 `false`。
  `hasPassword` 在带非空 `hashValue` / `saltValue` / `algorithmName`，或 `password` 不是空且不是 `0` / `0000` 时为 `true`。
  其余键与 XML 同名同向：`true` 表示该项被锁住（不允许）。缺属性用 schema 缺省：`objects` / `scenarios` / `selectLockedCells` / `selectUnlockedCells` 为 `false`，其余权限键为 `true`。
  这与 Excel「允许」勾选相反。
- 图表工作表 `sheetProtection` 字段顺序：`enforced`、`hasPassword`、`objects`。`enforced` 取 `@content`，规则同上。没有 `formatCells` 这类工作表权限键。
- `workbookProtection` 不要另给 scope 键。来源是 `/xl/workbook.xml` 的 `<workbookProtection>`（§18.2.29）。
  没有该元素时三项锁为 `false`，`hasPassword` 为 `false`。字段顺序：`lockStructure`、`lockWindows`、`lockRevision`、`hasPassword`。
  前三项取同名属性，缺省 `false`。`hasPassword` 在带非空 `workbookHashValue` / `workbookSaltValue` / `workbookAlgorithmName`
   / `revisionsHashValue` / `revisionsSaltValue` / `revisionsAlgorithmName`，
  或 `workbookPassword` / `revisionsPassword` / `password` 不是空且不是 `0` / `0000` 时为 `true`。不把哈希放进树。不报告 IRM 或 `fileSharing`。
- `scope.comments` 为 `true` 时，每张 `worksheet` 带 `comments`。没有线程化批注时为空数组。不依赖单元格切片。
  按该表 `ThreadedComments` 里无 `parentId` 的 `threadedComment` 的文档顺序。
  每条字段顺序：`id`、`commentId`、`author`、`initials`、`date`、`done`、`kind`、`ref`、`text`、`replies`。`commentId` 是 `@id` 原文。
  `author` 按 `personId` 去 `/xl/persons/person.xml` 的 `person/@displayName`，对不上则为 `null`。`initials` 恒为 `null`（person 没有缩写）。
  `date` 取 `@dT` 原文，缺则为 `null`。`done`：`@done` 为 `1` / `true` / `TRUE` / `on` 则为 `true`，缺省或其余为 `false`。`kind` 为 `normal`。
  `ref` 取 `@ref`，能解析成单一 A1 时规范化成列字母加大写行号；解析不了或缺失则为 `null`。`text` 取 `<text>` 的可见文本。回复嵌在 `replies` 里，不单独占 `commentCount`。
  回复字段顺序：`id`、`commentId`、`author`、`initials`、`date`、`done`、`kind`、`text`（无 `ref`）。回复 `id` 为 `{父批注 id}/reply:{guid}`。
  找线程化关系必须比完整 URI `http://schemas.microsoft.com/office/2017/10/relationships/threadedComment`。
- `scope.hyperlinks` 为 `true` 时，每张 `worksheet` 带 `hyperlinks`。没有链时为空数组。不依赖单元格切片。按该表 `<hyperlinks>` 里 `<hyperlink>` 的文档顺序。
  每条字段顺序：`id`、`index`、`kind`、`ref`、`rId`、`url`、`location`、`tooltip`、`display`。`index` 与 id 里的 `{n}` 相同。
  `kind` 为 `url`（有 `r:id`）或 `location`（没有 `r:id`）。`ref` 取 `@ref`，能解析成单一 A1 或闭区间时规范化成 `A1` 或 `A1:B2`；解析不了或缺失则为 `null`。
  `rId` 是关系 id 原文，缺则为 `null`。`url` 是该关系的 `Target` 原文，缺关系则为 `null`。`location` 取 `@location` 原文，缺则为 `null`。
  有 `r:id` 时树上仍可带 `location`（规范说这是靶内位置）。`tooltip` 取 `@tooltip`，缺则为 `null`。`display` 取 `@display`，缺则为 `null`。
  不把单元格可见值抄进这条；显示字仍走单元格。`HYPERLINK()` 公式不进本名单。pptx 的 `scope.hyperlinks` 见 [pptx 响应](json-contract-pptx.md#响应)。
- `scope.sparklines` 为 `true` 时，每张 `worksheet` 带 `sparklines`。没有组时为空数组。不依赖单元格切片。按该表全部 `x14:sparklineGroup` 的文档顺序。
  每条字段顺序：`id`、`index`、`type`、`displayEmptyCellsAs`、`sparklines`。`index` 与 id 里的 `{n}` 相同。
  `type` 取 `@type`：`line` / `column` / `stacked`；缺属性当作 `line`。不认识的原文原样留下。
  `displayEmptyCellsAs` 取同名属性：`gap` / `span` / `zero`；缺属性当作 `zero`。`sparklines` 是组内 `<x14:sparkline>` 数组，按文档顺序。
  每项字段顺序：`ref`、`formula`。`ref` 取 `xm:sqref` 文本，能解析成单格则规范化成列字母加大写行号；解析不了或缺失则为 `null`。`formula` 取 `xm:f` 原文，缺则为 `null`。
  不把颜色、标记、坐标轴或 `xr2:uid` 放进树。不把折线画进树。docx / pptx 的 `scope` 里出现 `sparklines` 报 `InvalidField`。
- `scope.oleObjects` 为 `true` 时，每张 `worksheet` 带 `oleObjects`。没有嵌入对象时为空数组。不依赖单元格切片。
  按该表 `<oleObjects>` 里被收入的 `oleObject` 文档顺序（`mc:AlternateContent` 只进第一个 `Choice`）。
  每条字段顺序：`id`、`shapeId`、`name`、`progId`、`kind`、`contentType`、`showAsIcon`、`anchor`，然后按锚点种类跟位置字段（规则与工作表图相同，几何取 `objectPr/anchor` 的 `from`/`to`；
  没有 `objectPr` 则取与 `shapeId` 相同的隐藏 `xdr:sp` 外层锚点），最后 `stable`。
  `shapeId` 是 `@shapeId` 原文，没有则为 `null`。`name` 取 dummy `xdr:sp` 的 `cNvPr/@name`，没有则为 `""`。`progId` 取 `@progId`，缺则为 `null`。
  `kind`：工作表关系 Type 后缀为 `/package` 时为 `package`，否则为 `embed`。
  `contentType` 取 embeddings 部件在 `[Content_Types].xml` 里的 Default 或 Override，都没有则为 `null`。
  `showAsIcon`：`@dvAspect` 为 `DVASPECT_ICON` 则为 `true`，否则为 `false`。不把嵌入字节或预览图放进树。与 `shapeId` 相同的隐藏 `xdr:sp` 不进 `shapes`。docx / pptx 见各自分册。
- `scope.model3ds` 为 `true` 时，每张 `worksheet` 带 `model3ds`。没有 3D 模型时为空数组。不依赖单元格切片。
  按该表 worksheet drawing 里被收入的 `am3d` AlternateContent 文档顺序。
  每条字段顺序：`id`、`cNvPrId`、`name`、`anchor`，然后按锚点种类跟位置字段（规则与工作表图相同），再跟 `contentType`、`stable`。
  `cNvPrId` 是 Choice 里 `xdr:cNvPr/@id` 原文，没有则为 `null`。`name` 取 `cNvPr/@name`，没有则为 `""`。
  `contentType` 恒为 `model/gltf.binary`。不把 GLB、相机、灯光或海报放进树。
  Fallback 海报不进 `pictures`。docx / pptx 见各自分册。
- `scope.slicers` 为 `true` 时，每张 `worksheet` 带 `slicers`。没有切片器时为空数组。不依赖单元格切片。按该表 drawing 里被收入的 slicer `graphicFrame` 文档顺序。
  每条字段顺序：`id`、`cNvPrId`、`name`、`caption`、`sourceKind`、`sourceName`、`field`、`anchor`，然后按锚点种类跟位置字段（规则与工作表图相同），最后 `stable`。
  `cNvPrId` 是 `xdr:cNvPr/@id` 原文，没有则为 `null`。`name` 取 slicer 部件 `@name`，没有则取 `cNvPr/@name`，都没有则为 `""`。
  `caption` 取 `@caption`，缺则为 `null`。`sourceKind` 为 `table`（cache 带 `x15:tableSlicerCache`）或 `pivot`（带 `tabular` / `pivotTables`）。
  `sourceName` 取表格 `displayName` 或透视表 `@name`，对不上则为 `null`。`field` 取 cache `@sourceName`。不把 cache 选项、筛选状态或样式放进树。
  时间线、OLAP 不进本名单。docx / pptx 的 `scope` 里出现 `slicers` 报 `InvalidField`。
- `scope.connections` 为 `true` 时，工作簿根带 `connections`。没有连接时为空数组。不依赖单元格切片。按 Connections 部件里被收入的 `connection` 文档顺序。
  每条字段顺序：`id`、`xmlId`、`name`、`kind`、`url`、`sourceFile`、`description`、`stable`。`xmlId` 是 `@id` 原文，没有则为 `null`。
  `name` 取 `@name`，没有则为 `""`。`kind`：`@type` 为 `4` 则为 `web`，为 `6` 则为 `text`，其余为 `other`。`url` 取 `webPr/@url`，缺则为 `null`。
  `sourceFile` 取 `textPr/@sourceFile`，缺则为 `null`。`description` 取 `@description`，缺则为 `null`。
  不把连接字符串、密码、queryTable 结果区或 `xr16:uid` 放进树。docx / pptx 的 `scope` 里出现 `connections` 报 `InvalidField`。
- `scope.externalLinks` 为 `true` 时，工作簿根带 `externalLinks`。没有外部引用时为空数组。不依赖单元格切片。按 workbook `<externalReference>` 文档顺序。
  每条字段顺序：`id`、`index`、`kind`、`path`、`sheets`。`index` 与 id 里的 `{n}` 相同。
  `kind`：目标部件根下有 `<externalBook>` 则为 `book`，其余为 `other`。
  `path` 取 `externalBook` 的 `r:id` 对应 `.rels` 的 `Target` 原文，缺关系或没有 `externalBook` 则为 `null`。
  `sheets` 是 `<sheetName>/@val` 组成的字符串数组，按文档顺序；没有则为 `[]`。不把 `sheetDataSet` 缓存格子、`xxl21:alternateUrls` 或 DDE / OLE 项放进树。
  找不到 workbook 关系报 `RelNotFound`，内部部件不在包里报 `PartMissing`。docx / pptx 的 `scope` 里出现 `externalLinks` 报 `InvalidField`。
- `scope.customViews` 为 `true` 时，工作簿根带 `customViews`。没有自定义视图时为空数组。不依赖单元格切片。
  按 `/xl/workbook.xml` 里 `<customWorkbookView>` 的文档顺序（ECMA-376 §18.2.3 / §18.2.4）。
  每条字段顺序：`id`、`name`、`includePrintSettings`、`includeHiddenRowCol`、`activeSheet`。`name` 取 `@name`，没有则为 `""`。
  `includePrintSettings` / `includeHiddenRowCol`：属性为 `0` / `false` / `FALSE` / `off` 则为 `false`，缺属性或其余为 `true`。
  `activeSheet` 按 `@activeSheetId` 对上 `sheets/sheet/@sheetId` 的表 `name`；对不上则为 `null`。
  不把 guid、窗口坐标或各表 `customSheetView` 放进树。docx / pptx 的 `scope` 里出现 `customViews` 报 `InvalidField`。
- `scope.headerFooter` 为 `true` 时，每张表带 `headerFooters`，固定六个对象，顺序见上文 id 表。
  每个槽位字段顺序：`id`、`kind`、`side`、`text`；`worksheet` 再跟 `pictureCount`，若同时 `pictures: true` 再跟 `pictures`。
  `kind` / `side` 与 id 里那两段相同。`text` 为该 XML 元素原文，没有该元素则为 `null`。空串是合法原文，与 `null` 不同。`pictureCount` 是分到该槽的 DrawingML 页眉图个数。
  页眉图字段顺序与工作表图相同，末尾 `contentType` 规则与工作表图相同。
  `chartsheet` / `dialogsheet` / `macrosheet` 的槽位没有 `pictureCount` / `pictures`。
- `scope.pictures` 为 `true` 时，每张 `worksheet` 带工作表 `pictures`。没有图时为空数组。按该表 `<drawing>` 部件顺序、同一部件内按锚点文档顺序。
  每张字段顺序：`id`、`cNvPrId`、`name`、`anchor`，然后按锚点种类跟位置字段，最后 `stable`。
  有 `asvg:svgBlip` 时再跟 `contentType`，值为 `image/svg+xml`。
  没有 `svgBlip` 时，按 image 关系 Target 扩展名：
  `.emf` 为 `image/x-emf`，`.wmf` 为 `image/x-wmf`，`.tif` / `.tiff` 为 `image/tiff`。
  PNG / JPEG / GIF / BMP 没有该字段。不读媒体字节，不把原图放进树。
  `cNvPrId` 是 `xdr:cNvPr/@id` 原文，没有则为 `null`。
  `name` 取 `xdr:cNvPr/@name`，没有则为 `""`。`anchor` 为 `twoCell`、`oneCell` 或 `absolute`。
  单独 `pictures: true`、不开 `headerFooter`：只列工作表图，不含页眉图。
- `twoCell`（§20.5.2.33）再跟 `from`、`fromOffX`、`fromOffY`、`to`、`toOffX`、`toOffY`、`editAs`。
  不报 `cx` / `cy`（显示尺寸由起止格与列宽行高决定，库不算）。`editAs` 取 `twoCellAnchor/@editAs`：`twoCell`、`oneCell` 或 `absolute`；缺省当作 `twoCell`。
- `oneCell`（§20.5.2.24）再跟 `from`、`fromOffX`、`fromOffY`、`cx`、`cy`。`cx` / `cy` 取 `xdr:ext`，没有则为 `null`。
- `absolute`（§20.5.2.1）再跟 `posX`、`posY`、`cx`、`cy`。`posX` / `posY` 取 `xdr:pos`。
- `from` / `to` 是 A1（列字母加大写行号）。XML 里 `xdr:col` / `xdr:row` 从 0 计（§20.5.3.1、§20.5.3.3），树上换成 A1，与单元格 id 同一套写法。
  `fromOffX` / `fromOffY` / `toOffX` / `toOffY` 是 EMU 整数，取 `colOff` / `rowOff`，缺省为 `0`。
- `scope.charts` 为 `true` 时，每张 `worksheet` 与 `chartsheet` 带 `charts`。没有图表时为空数组。按该表 drawing 部件顺序、同一部件内按锚点文档顺序。
  每张字段顺序：`id`、`cNvPrId`、`name`、`anchor`，然后按锚点种类跟位置字段（规则与工作表图相同），再跟 `type`，`type` 为 `ofPie` 时再跟 `ofPieType`，
  `type` 为 `stock` 时再跟 `stockKind`，`type` 为 `surface` 时再跟 `surfaceKind`，
  `type` 为 `chartex` 时再跟 `layoutId`，然后 `title`、`catAxisTitle`、`valAxisTitle`、`series`、`legend`、`dataLabels`、`flags`、`stable`。
  `cNvPrId` / `name` / `anchor` / 位置字段 / `stable` 规则与图相同。
  `type` / `ofPieType` / `stockKind` / `surfaceKind` / `layoutId` / `title` / 轴标题 / `legend` / `dataLabels` / `flags` 规则与 docx 图表相同。
- 每条系列字段顺序：`id`、`index`、`name`、`nameRef`、`categories`、`catRef`、`values`、`valRef`，气泡再跟 `sizes`、`sizeRef`，再跟 `axis`，
  有趋势线时再跟 `trendlines`，有误差线时再跟 `errorBars`，必要时再跟 `cacheStale`。
  `axis` / `trendlines` / `errorBars` 规则与 docx 图表相同。`index` 即 `{idx}`。
  `name` / `categories` / `values` / `sizes` 优先取 `c:f` 指向的单元格缓存值（与 inspect 单元格同一套 `<v>`）；读不到再退回 chart 的 cache / lit。
  `nameRef` / `catRef` / `valRef` / `sizeRef` 取对应 `c:f` 原文；没有公式则为 `null`。单元格值与 chart cache 不一致时该系列 `cacheStale: true`。
- `scope.textBoxes` 为 `true` 时，每张 `worksheet` 带 `textBoxes`。没有文本框时为空数组。
  按该表 worksheet drawing 部件顺序、同一部件内按锚点文档顺序，组合里符合条件的框展开后仍按遇到的顺序。
  每张字段顺序：`id`、`cNvPrId`、`name`、`anchor`，然后按锚点种类跟位置字段（规则与工作表图相同），再跟 `text`，
  然后 `sz`、`color`、`textFill`、`textOutline`、`bold`、`italic`、`underline`、`font`、`eastAsiaFont`、`align`、`textAnchor`、`runs`、`stable`。
  `cNvPrId` / `name` / `anchor` / 位置字段 / `stable` 规则与图相同。`text` 取 `xdr:txBody` 可见 `a:t`，规则对齐 pptx 形状 `text`。不把框内字做成段落树。
  `anchor` 仍是单元格锚点。框内垂直对齐是 `textAnchor`。格式键规则与 pptx 形状相同，见 [json-contract.md](json-contract.md#setshapeformat)。
- `scope.maths` 为 `true` 时，每张 `worksheet` 带 `maths`。没有公式框时为空数组。不依赖单元格切片。
  按该表 worksheet drawing 里被收入的公式框文档顺序。组合里符合条件的展开。
  每条字段顺序：`id`、`cNvPrId`、`name`、`anchor`，然后按锚点种类跟位置字段（规则与工作表图相同），再跟 `text`、`stable`。
  `cNvPrId` 是 Choice 里 `xdr:cNvPr/@id` 原文，没有则为 `null`。`name` 取 `cNvPr/@name`，没有则为 `""`。
  `text` 为该 `oMath` 内 `m:t` 拼接；忽略 `a:fld` 外壳。不把 OMML 原文、不把 `expr` 放进树。
  Fallback 文本框不进 `textBoxes`。docx / pptx 见各自分册。
- `scope.shapes` 为 `true` 时，每张 `worksheet` 带 `shapes`。没有形状时为空数组。顺序与文本框相同。
  每张字段顺序：`id`、`cNvPrId`、`name`、`anchor`，然后按锚点种类跟位置字段（规则与工作表图相同），再跟 `prst`，有 `xdr:txBody` 再跟 `text`，
  然后 `sz`、`color`、`textFill`、`textOutline`、`bold`、`italic`、`underline`、`font`、`eastAsiaFont`、`align`、`textAnchor`、`runs`，最后 `stable`。
  `prst` 取 `a:prstGeom/@prst`，没有则为 `null`。`text` 规则与文本框相同。没有 `txBody` 则没有 `text` 与格式键。
- `scope.connectors` 为 `true` 时，每张 `worksheet` 带 `connectors`。没有连接线时为空数组。顺序与文本框相同。
  每条字段顺序：`id`、`cNvPrId`、`name`、`anchor`，然后按锚点种类跟位置字段（规则与工作表图相同），再跟 `prst`、`stable`。
  `prst` 取 `a:prstGeom/@prst`，没有则为 `null`。不报 `text`。
- `scope.smartArt` 为 `true` 时，每张 `worksheet` 带 `smartArts`。没有图示时为空数组。顺序与文本框相同。
  每张字段顺序：`id`、`cNvPrId`、`name`、`anchor`，然后按锚点种类跟位置字段（规则与工作表图相同），再跟 `layout`、`colors`、`style`、`points`、`stable`。
  `layout` / `colors` / `style` / `points` 规则与 docx / pptx SmartArt 相同。四条关系从 **drawing** `.rels` 解。
- 不把媒体字节、chart 部件或 diagram 四件套放进树。
- `cells` 稀疏：范围内没有 `<c>` 的格子不出现。
- 有单元格切片时始终带 `cols`、`rows`、`merges`、`dataValidations`、`conditionalFormats`、`tables`、`pivots`（可为空数组）。未点名的表、以及没有单元格切片的表不带这七项。
- `cols` 来自工作表 `<cols>/<col>`（§18.3.1.13）。只列出与 inspect 范围列号有交集、且带 `width` 或为真的 `bestFit` 的条目，并把 `min` / `max` 裁到范围内。
  每项字段顺序：`min`、`max`，有 `width` 再跟，`bestFit` 为真时再跟。`min` / `max` 是列字母（与 A1 同一套）。
  JSON 的 `width` 是 Excel 列宽字符数（与 COM `ColumnWidth` 同一套），不是像素，也不是 XML 属性的原样数字。
  从 `col/@width` 按 §18.3.1.13 的公式还原，MDW 固定用该节示例的 Calibri 11 点 7 像素，四舍五入到 2 位小数。不读实际字体、不算排版。没有 `width` 则省略该键。
  `bestFit` 仅在属性为真时出现且为 `true`。不要用 `sheetFormatPr/@defaultColWidth` 去填没有 `<col>` 的列。不报隐藏、大纲、列样式。
- `rows` 来自范围内已有 `<row>` 且带 `ht` 的行。没有 `ht` 的行不出现。每项字段顺序：`r`、`height`。`r` 是行号（从 1 计）。`height` 取 `row/@ht`，单位磅。不报隐藏、大纲。
- `merges` 是与 inspect 范围有交集的 `<mergeCell/@ref>`，规范化成 `A1:B2` 这种闭区间字符串，按左上角行列升序。报完整合并区，不裁到 inspect 范围。没有则为 `[]`。
- `dataValidations` 列出该表全部 `<dataValidation>`（ECMA-376 §18.3.1.32），不按 inspect 范围过滤。条数通常少，点名该表单元格切片时就要能拿到规则 id。
  每项字段顺序：`id`、`sqref`、`type`，然后按需跟 `operator`、`formula1`、`formula2`、`allowBlank`、`dropdown`、`showInputMessage`、
  `showErrorMessage`、`errorStyle`、`errorTitle`、`error`、`promptTitle`、`prompt`。
  `sqref` 是字符串数组：把 XML `sqref`（`ST_Sqref`，空格分隔）拆开，能解析成 A1 的规范化成 `A1` 或 `A1:B2`，解析不了的段落原样留下；按 XML 原文顺序，不排序。
  `type` 取 `ST_DataValidationType`（`none` / `whole` / `decimal` / `list` / `date` / `time` / `textLength` / `custom`），缺属性当作 `none`。
  `operator` 仅在 XML 有该属性时出现。`formula1` / `formula2` 是对应子元素的原文，当字符串，不求值、不补 `=`；没有该子元素则省略。
  `allowBlank` / `showInputMessage` / `showErrorMessage` 始终是布尔：XML 为 `1` / `true` / `TRUE` / `on` 则为 `true`，缺省为 `false`。
  `dropdown` 只在 `type` 为 `list` 时出现：表示是否显示单元格下拉。
  XML 的 `showDropDown="1"` 在 Excel 里是**隐藏**下拉（与 ECMA 属性名相反），因此 `showDropDown` 为真时 JSON `dropdown` 为 `false`，缺省或为假时 `dropdown` 为 `true`。
  `errorStyle` / `errorTitle` / `error` / `promptTitle` / `prompt` 仅在 XML 有对应属性时出现。
  不报 `imeMode`，也不报父元素 `disablePrompts` / `xWindow` / `yWindow`。`extLst` 里的扩展校验不列入。
- `conditionalFormats` 列出该表全部 `<cfRule>`（ECMA-376 §18.3.1.10），按文档顺序，不按 inspect 范围过滤。条数通常少，点名该表单元格切片时就要能拿到规则 id。
  每项字段顺序：`id`、`sqref`、`type`、`priority`，然后按需跟 `operator`、`formulas`、`text`、`timePeriod`、`rank`、`percent`、`bottom`、
  `aboveAverage`、`equalAverage`、`stdDev`、`stopIfTrue`、`dxf`、`colorScale`、`dataBar`、`iconSet`。
  `sqref` 取所属 `<conditionalFormatting>/@sqref`，拆分数组规则与数据验证相同。同一块里多条规则重复同一份 `sqref`。
  `type` 取 `ST_CfType`（§18.18.12）原文；缺属性则省略该键。`priority` 取 XML 整数；缺属性则省略。
  `operator` 仅在 XML 有该属性时出现，取值 `ST_ConditionalFormattingOperator`（§18.18.15）。
  `formulas` 是 `<formula>` 子元素原文组成的字符串数组，不求值、不补 `=`；没有则省略。`text` / `timePeriod` / `rank` 仅在 XML 有对应属性时出现。
  `percent` / `bottom` / `aboveAverage` / `equalAverage` / `stopIfTrue` 仅在属性为真时出现且为 `true`。`stdDev` 仅在 XML 有该属性时出现。
  `dxf` 是 `dxfId` 指向的 `styles.xml` `<dxfs>/<dxf>` 摘要：字段顺序 `font`、`fill`、`border`，形状与单元格样式摘要相同，但按差分本身报（不跟 `fonts[0]` 比）。
  `dxf` 的 `fill` 若缺 `patternType` 却有 `fgColor` / `bgColor`，当作 `solid`。没有 `dxfId` 或越界则省略 `dxf`。
  `colorScale` 字段顺序：`cfvo`、`colors`。`cfvo` 每项字段顺序：`type`，有 `val` 再跟，`gte` 为假时再跟 `false`。
  `colors` 是颜色对象数组，规则与单元格 `color` 相同。`dataBar` 字段顺序：`cfvo`、`color`，然后按需 `minLength`、`maxLength`、`showValue`。
  `showValue` 仅在 XML 为假时出现且为 `false`。`iconSet` 字段顺序：`set`（XML 属性 `iconSet`，缺省当作 `3TrafficLights1`）、`cfvo`，
  然后 `showValue` 为假、`reverse` 为真、`percent` 为假时再跟对应键。
  父块 `pivot="1"` 不进 JSON。`extLst` 里的扩展条件格式不列入。
- `tables` 列出该表全部 `<tablePart>` 指向的表格部件（§18.5.1.2），按 `tableParts` 文档顺序，不按 inspect 范围过滤。点名该表单元格切片时就要能拿到表格 id。
  关系解不出部件报 `XmlBroken`。每项字段顺序：`id`、`name`、`ref`、`columns`。`name` 取 `displayName`，没有则取 `name`，都没有则为 `""`。
  `ref` 取 `@ref`，能解析成 A1 的规范化成 `A1` 或 `A1:B2`，解析不了的原样留下。`columns` 是字符串数组，按 `<tableColumn>` 文档顺序取 `@name`；缺属性则为 `""`。
  不报 XML `id`、`headerRowCount`、`totalsRowCount`、计算列公式、查询连接、`tableStyleInfo` 或切片器。
  工作表级 `<autoFilter>` 不列入此项，见工作表节点上的 `autoFilter`。
- `pivots` 列出该表全部透视表部件（§18.10），按工作表关系里 Type 以 `/pivotTable` 结尾的文档顺序，不按 inspect 范围过滤。点名该表单元格切片时就要能拿到透视表 id。
  关系解不出部件报 `XmlBroken`。每项字段顺序：`id`、`name`、`cacheId`、`location`、`source`、`rows`、`cols`、`filters`、`values`。
  `name` 取 `pivotTableDefinition/@name`，没有则为 `""`。`cacheId` 取 `@cacheId` 的整数；缺属性或无法解析则为 `null`。
  `location` 取 `<location>/@ref`，能解析成 A1 的规范化成 `A1` 或 `A1:B2`，解析不了的原样留下。`source` 字段顺序：`sheet`、`ref`。
  只列出 `cacheSource/@type` 为 `worksheet` 且带 `worksheetSource/@sheet` 与 `@ref` 的源；其它源类型该项为 `null`。
  `rows` / `cols` / `filters` / `values` 是字段名字符串数组，分别来自 `<rowFields>`、`<colFields>`、`<pageFields>`、`<dataFields>`，
  经 `<pivotFields>` 下标解到 `cacheField/@name`（`dataField/@name` 若有则优先）。
  解不出则为 `""`。不报缓存记录、合计数字、`cacheId` 以外的缓存属性、切片器或透视表样式。
- 范围内有内容的格子超过 `maxCells` 时，按行优先、从左到右截断，`truncated` 为 `true`。未截断则为 `false`。
- `type`：`number`、`text`、`boolean`、`error`、`date`、`empty`。对应 SpreadsheetML `ST_CellType`（§18.18.11）的映射见下表。
- `value` 是缓存显示值，不是库算出来的。公式格的 `value` 来自 `<v>`，可能过期。没有 `<v>` 时 `value` 为 `null`。
- `formula`：有公式则返回以 `=` 开头的字符串。XML 里 `<f>` 不含 `=`，由库补上。没有公式则为 `null`。
- 共享公式（`<f t="shared"`，§18.3.1.40）：主格返回公式文本，并带 `"shared": true` 与 `"sharedRef"`（取主格 `f/@ref`，规范化成 `A1` 或 `A1:B2`）。
  从属格带 `"shared": true`；`formula` 按主格到该格的行列差展开成该格的 A1 文本（规则与下文 apply 的相对展开相同）。找不到主格或主格没有公式文本时，从属格 `formula` 仍为 `null`。
- 数组公式（`<f t="array"`）：主格返回公式文本，并带 `"array": true` 与 `"arrayRef"`（取 `f/@ref`，同样规范化；没有 `ref` 则当作该格自己）。
  落在该 `ref` 内、自己没有公式的格子带 `"array": true`，`formula` 为 `null`。
- 数据表公式（`<f t="dataTable"`）：带 `"dataTable": true`。本期不展开，也不在 apply 里改。
- 内联字符串（`t="inlineStr"`）和共享字符串（`t="s"`）在树上都是 `type: "text"`，`value` 为拼接后的可见纯文本，不含 run 级样式。
  `t="s"` 的 `<v>` 必须是共享字符串表里的合法非负整数索引。越界或无法解析报 `XmlBroken`，不要当成空串。
- 单元格字段顺序：`id`、`ref`、`type`、`value`、`formula`，然后始终跟 `numFmt`，有非默认字体 / 填充 / 边框 / 对齐时再跟 `font`、`fill`、`border`、`align`，
  然后按需跟 `array`、`arrayRef`、`shared`、`sharedRef`、`dataTable`。
  只在为真或有值时出现：`array` / `shared` / `dataTable` 为 `true`，`arrayRef` / `sharedRef` 为规范化后的区域字符串。
- `numFmt` 是数字格式代码字符串，不是 `numFmtId`。缺 `<c>/@s` 或 `s` 越界时按 `cellXfs` 第 0 条。
  内置 id 0–49 用 ECMA-376 §18.8.30 的英文代码（例如 `0` → `"General"`，`2` → `"0.00"`，`14` → `"mm-dd-yy"`），不按打开 Excel 的区域设置改写。
  自定义格式取 `numFmts` 里对应 `formatCode`。没有 styles 部件时当作 `"General"`。
- `font` / `fill` / `border` / `align` 是精简摘要，不是完整 `xf` 镜像。与 `fonts[0]` / `fills[0]`（`patternType="none"`）/ 空边框相同则省略该对象。
  渐变填充只报 `{ "pattern": "gradient" }`，不展开 stop。
- `font` 字段顺序：`name`、`sz`、`bold`、`italic`、`underline`、`color`。只出现与默认字体不同的键。`sz` 为磅，整数则 JSON 整数。
  `underline` 为 `single` / `double` / `singleAccounting` / `doubleAccounting`。`color` 见下。
- `fill` 字段顺序：`pattern`，然后 `fg`、`bg`（有才出现）。`pattern` 为 `ST_PatternType`（§18.18.55）。
- `border` 字段顺序：`left`、`right`、`top`、`bottom`。每边是对象：`style`（`ST_BorderStyle`，§18.18.3），有颜色再跟 `color`。没有线型的边省略。
- `align` 字段顺序：`horizontal`、`vertical`、`wrap`。取 `xf/alignment` 上有的属性。
  `horizontal` 为 `ST_HorizontalAlignment`（§18.18.40），`vertical` 为 `ST_VerticalAlignment`（§18.18.88），`wrap` 为布尔。
- 颜色对象恰好给 `rgb`、`theme`、`indexed` 之一；`theme` 可再带 `tint`（数字）。`rgb` 为大写十六进制，6 位或 8 位（ARGB）。主题色原样报索引，不解析成 RGB。不报 `auto`。

类型映射：

| XML `t` | 树上 `type` | `value` JSON 类型 |
| --- | --- | --- |
| 缺省或 `n` | `number` | 数字。解析失败则当 `text` 并把原文放进去 |
| `s` 或 `inlineStr` 或 `str` | `text` | 字符串 |
| `b` | `boolean` | `true` / `false`（XML 里 `1`/`0`） |
| `e` | `error` | 字符串，例如 `"#DIV/0!"` |
| `d` | `date` | 字符串，ISO 8601 原文 |
| 有 `r` 但无 `v`/`is`/`f` | `empty` | `null` |

日期序列值（`t` 缺省、单元格看起来像日期）第一期仍标 `number`，不根据样式猜日期。这是有意的，避免做半套日期引擎。


## create

```json
{
  "schemaVersion": 1,
  "action": "create",
  "path": "C:\\out\\book.xlsx",
  "format": "xlsx",
  "workbook": {
    "sheets": [
      {
        "name": "Sheet1",
        "cells": [
          {
            "ref": "A1",
            "value": 1
          },
          {
            "ref": "A2",
            "value": "hello"
          },
          {
            "ref": "A3",
            "value": true
          },
          {
            "ref": "B1",
            "formula": "=SUM(A1)"
          }
        ],
        "pictures": [
          {
            "file": "C:\\\\img\\\\a.png",
            "from": "B2"
          },
          {
            "file": "C:\\\\img\\\\a.png",
            "from": "D2",
            "to": "F8",
            "anchor": "twoCell"
          }
        ],
        "headerFooters": [
          {
            "kind": "odd",
            "side": "header",
            "text": "&C页眉"
          },
          {
            "kind": "odd",
            "side": "footer",
            "text": "&C页脚",
            "pictures": [
              {
                "file": "C:\\\\img\\\\a.png"
              }
            ]
          }
        ]
      }
    ]
  }
}
```

| 字段 | 含义 |
| --- | --- |
| `path` | 输出路径。已存在则覆盖。省略时必须由调用方另给一个落盘路径，命令行用 `--output` |
| `workbook.sheets` | 至少一张表 |
| `workbook.props` | 可选。文件属性，字段与 [json-contract.md](json-contract.md#setprops) 的 `setProps` / `setCustomProp` 相同 |
| `sheets[].name` | 工作表名。Excel 限制：最长 31 个字符，不含 `:*?/[]\\`，不能叫 `History`，同一工作簿内不重复 |
| `sheets[].kind` | 可选。`worksheet`（省略也是）或 `chartsheet`。`dialogsheet` / `macrosheet` 以及其它取值报 `InvalidField` |
| `sheets[].pictures` | 可选。该表工作表 drawing 上的图列表。省略或空数组则不写工作表图。不进页眉。图表工作表上出现报 `InvalidField` |
| `sheets[].charts` | 可选。该表上的图表列表。省略或空数组则不写图表。图表工作表上出现报 `InvalidField` |
| `sheets[].headerFooters` | 可选。该表页眉页脚槽位列表。省略或空数组则不写 `headerFooter` / `drawingHF` |
| `sheets[].comments` | 可选。该表线程化批注列表。省略或空数组则不写批注部件。不写回复 |
| `sheets[].hyperlinks` | 可选。该表 `<hyperlinks>` 列表。省略或空数组则不写。不写单元格值 |
| `sheets[].textBoxes` | 可选。该表文本框列表。省略或空数组则不写 |
| `sheets[].maths` | 可选。该表 drawing 公式框。省略或空数组则不写。图表工作表上出现报未知字段 |
| `sheets[].shapes` | 可选。该表无 `txBox` 的 `xdr:sp` 列表。省略或空数组则不写 |
| `sheets[].connectors` | 可选。该表 `xdr:cxnSp` 列表。省略或空数组则不写 |
| `sheets[].smartArts` | 可选。该表 SmartArt 列表。省略或空数组则不写 |
| `sheets[].model3ds` | 可选。该表工作表 drawing 上的 3D 模型。省略或空数组则不写。图表工作表上出现报 `InvalidField` |
| `cells[].ref` | A1 地址 |
| `cells[].value` | 数字、字符串、布尔、或 `null`。`null` 表示不写这个格 |
| `cells[].formula` | 以 `=` 开头的公式文本。库不求值 |

| `pictures[].file` | 本地路径，与 `insertPicture` 相同。按文件头认 PNG / JPEG / GIF / BMP / SVG / EMF / WMF / TIFF。缺此字段报 `MissingField` |
| `pictures[].from` | 必须。左上角单元格，A1。缺此字段报 `MissingField`。不是合法 A1 报 `InvalidRange` |
| `pictures[].to` | `anchor` 为 `twoCell` 时必须。右下角单元格，A1 |
| `pictures[].anchor` | 可选。`oneCell`（缺省）或 `twoCell`。不认 `absolute` |
| `pictures[].cx` / `pictures[].cy` | 可选，EMU，正整数。都省则与 `insertPicture` 相同。只给一个时报 `InvalidField`。`twoCell` 也要写 `xdr:pic` 里的 `a:ext`，显示尺寸仍由起止格决定 |
| `pictures[].fromOffX` / `fromOffY` / `toOffX` / `toOffY` | 可选。与 `insertPicture` 相同 |
| `charts[].type` | 必须。与 `insertChart` 相同 |
| `charts[].series` | 必须。非空。字段与 `insertChart` 相同 |
| `charts[].from` | 必须。图的左上角单元格，A1 |
| `charts[].dataFrom` | 必须。数据表左上角，A1。按 `setCell` 语义覆盖已有格子，包括同一张表 `cells[]` 里已声明的值 |
| `charts[].title` | 可选 |
| `charts[].to` / `anchor` / `cx` / `cy` / 偏移 | 与 `insertChart` 相同。`kind` / `wrap` 报 `InvalidField` |
| `headerFooters[].kind` | 必须。`odd` / `even` / `first`。其它取值报 `InvalidField` |
| `headerFooters[].side` | 必须。`header` / `footer`。其它取值报 `InvalidField` |
| `headerFooters[].text` | 可选。该槽位原文，与 `setHeaderFooterText` 相同，含 `&P` / `&C` 这类代码。空串不当作内容 |
| `headerFooters[].pictures` | 可选。往该槽 `drawingHF` 插图。省略或空数组则不写图 |
| `headerFooters[].pictures[].file` | 必须。本地路径，与页眉 `insertPicture` 相同 |
| `headerFooters[].pictures[].cx` / `cy` | 可选，EMU，正整数。都省则与 `insertPicture` 相同。只给一个时报 `InvalidField` |
| `comments[].cell` | 必须。宿主单元格，单格 A1。缺此字段报 `MissingField`。不是合法单格报 `InvalidRange` |
| `comments[].text` | 可选。可见纯文本，省略当作 `""` |
| `comments[].author` | 可选。省略当作 `"ooxp"` |
| `comments[].initials` | 可选。xlsx 接受但不写入 XML |
| `comments[].date` | 可选。`xsd:dateTime` 字符串。省略则写入当前 UTC（无尾 `Z`） |
| `hyperlinks[].ref` | 必须。覆盖的 A1 或闭区间。非法报 `InvalidRange` |
| `hyperlinks[].url` | 外部 URL。与 `location` 至少给一个 |
| `hyperlinks[].location` | 内部位置。与 `url` 至少给一个 |
| `hyperlinks[].tooltip` / `display` | 可选。写入同名属性 |
| `textBoxes[].from` / `shapes[].from` / `connectors[].from` / `smartArts[].from` | 必须。左上角单元格，A1 |
| `model3ds[].file` | 必须。本地 GLB 路径，与 `insertModel3d` 相同 |
| `model3ds[].from` | 可选。左上角单元格，A1。省略当作 `A1` |
| `model3ds[]` 的 `to` / `anchor` / 偏移 / `cx` / `cy` | 可选。规则与 `insertModel3d` 相同 |
| `textBoxes[]` / `shapes[]` / `connectors[]` / `smartArts[]` 的 `to` / `anchor` / 偏移 / `cx` / `cy` | 可选。规则与对应 insert 相同 |
| `textBoxes[].text` / `shapes[].text` | 可选。可见纯文本 |
| `textBoxes[]` / `shapes[]` 的文字格式 | 可选。与 `setShapeFormat` 相同，含 `textFill` / `textOutline` |
| `maths[].text` | 必须。非空。写入一条 `m:r` / `m:t`。出现 `expr` / `kind` 报 `InvalidField` |
| `maths[].from` | 可选。左上角单元格，A1。省略当作 `A1` |
| `maths[]` 的 `to` / `anchor` / 偏移 / `cx` / `cy` | 可选。规则与 `insertMath` 相同 |
| `shapes[].prst` | 可选。省略为 `rect`。必须是 pptx `insertShape` 那份可写预设名。连接线几何名报 `InvalidField` |
| `connectors[].prst` | 可选。缺省 `straightConnector1`。允许取值与 `insertConnector` 相同 |
| `smartArts[].layout` | 必须。`list` / `org` / `cycle` |
| `smartArts[].points` | 可选。字段与 `insertSmartArt` 相同 |
| `kind: chartsheet` 的 `type` / `series` / `dataFrom` | 必须。与 `insertChartSheet` 相同 |
| `kind: chartsheet` 的 `dataSheet` | 可选。承接 `dataFrom` 的普通表名。省略则取 `sheets[]` 里第一张 `worksheet` |
| `kind: chartsheet` 的 `ofPieType` / `title` / `cx` / `cy` | 可选。与 `insertChartSheet` 相同。只给 `cx` 或只给 `cy` 报 `InvalidField` |

同一 `kind` 与 `side` 出现两次报 `InvalidField`。每一项至少要有非空 `text` 或一张图，否则报 `InvalidField`。
页眉图禁止出现 `from` / `to` / `anchor` / `fromOffX` / `fromOffY` / `toOffX` / `toOffY`，出现报 `InvalidField`。
不要把页眉图写进 `sheets[].pictures`。不要用单数 `headerFooter`（那是 inspect 的 scope 键），`sheets[]` 上出现它报 `InvalidField`。
写 `even` 槽位时给 `headerFooter` 设 `differentOddEven="1"`；写 `first` 槽位时设 `differentFirst="1"`。先写各槽原文，再按数组顺序往对应槽插图。不写 VML `legacyDrawingHF`。不解释 `&P`。
带 SVG 图的示例见 `docs/examples/xlsx-create-svg.json`。

`value` 和 `formula` 不能在同一格同时出现。要公式就只写 `formula`。
公式格不写缓存 `<v>`，并在 `workbook.xml` 的 `calcPr` 上设 `fullCalcOnLoad="1"`，让 Excel 打开时重算。

文本写入共享字符串表（`t="s"`），这是 Office 新建文件的常见写法。每个文本值必须先进入该表再写索引；表里找不到对应项时 create 失败，不许退回索引 0。布尔写成 `t="b"`。数字不写 `t`（默认为 `n`）。

行与单元格按行号、列号升序写出。补上 `sheetData` 和合理的 `dimension`。

最小 `styles.xml` 必须生成，见 [opc.md](opc.md)。默认字体颜色写 `<color rgb="FF000000"/>`，不引用 theme，包里也不生成 theme 部件。
给了 `sheets[].pictures` 时按 `insertPicture` 同一套规则写 worksheet drawing 与 media。
给了 `sheets[].charts` 时按 `insertChart` 同一套规则写 `/xl/charts/chartN.xml`、`xdr:graphicFrame` 和 `dataFrom` 一带单元格，不写 embedding。
给了 `sheets[].textBoxes` / `shapes` / `connectors` / `smartArts` 时按对应 insert 同一套规则写入同一份 worksheet drawing：先写 `pictures`，
再写 `model3ds`，再写 `charts`，再按 `textBoxes`、`maths`、`shapes`、`connectors`、`smartArts` 数组顺序写入。
给了 `sheets[].maths` 时按 `insertMath` 同一套规则写 `a14:m` AlternateContent。不写进 `textBoxes[]`。
带公式框的示例见 `docs/examples/xlsx-create-math-drawing.json`。
给了 `sheets[].model3ds` 时按 `insertModel3d` 同一套规则写 GLB、海报 PNG 与 `am3d` AlternateContent。
带 3D 模型的示例见 `docs/examples/xlsx-create-model3d.json`。
给了 `sheets[].comments` 时按 `insertComment` 同一套规则写线程化批注、占位 `comments.xml` 与 Note `v:shape`；不写回复。
给了 `sheets[].hyperlinks` 时按 `insertHyperlink` 同一套规则写 `<hyperlinks>` 与外部链关系。
给了 `sheets[].headerFooters` 时按 `setHeaderFooterText` 写 `headerFooter` 六个槽位，按页眉 `insertPicture` 写 `drawingHF` 与 media；页眉图不进 worksheet drawing。
`xdr:cNvPr/@id` 取该表已有全部 `cNvPr` 的最大整数加 1；页眉图至少为 2。
给了 `kind: chartsheet` 时按 `insertChartSheet` 同一套规则写 `/xl/chartsheets/sheetN.xml`、
drawing 上那一张 `xdr:absoluteAnchor` 框和 `/xl/charts/chartN.xml`，数据写到点名的普通表；`cNvPr/@id` 至少为 2。
图表工作表的 `dataFrom` 在该数据表 `charts[]` 之后写入，会覆盖同一格。`workbook.xml` 的 `<sheet>` 顺序与 `sheets[]` 一致。
`app.xml` 的 HeadingPairs：普通表算 Worksheets，图表工作表算 Charts；TitlesOfParts 按 `sheets[]` 顺序。
至少一张 `worksheet`，否则报 `InvalidField`。不要生成透视表、条件格式。
create 本期不写列宽、行高、合并、数据验证、条件格式、工作表表格、透视表、定义名、打印设置、视图与筛选、排序、分组、拆分窗格、自定义视图、迷你图、OLE、切片器、外部数据连接、保护或组合，
也不写共享公式或数组公式（`cells[]` 出现 `shared` / `sharedRef` / `array` / `arrayRef` 报 `InvalidField`）。
create 本期不写外部引用。`workbook` 根上或 `cells[]` 上出现 `pictures` / `charts` / `headerFooters` / `headerFooter`
 / `definedNames` / `connections` / `externalLinks` / `externalReferences` / `customViews` / `comments`
 / `hyperlinks` / `textBoxes` / `shapes` / `connectors` / `smartArts` / `workbookProtection` / `protection`，
或 `sheets[]` 上出现 `wrap` / `sparklines` / `oleObjects` / `slicers` / `sheetProtection` / `protection` / `headerFooter`
 / `outline` / `sortState` / `sort`，报 `InvalidField`。
`kind` 只能是 `worksheet` 或 `chartsheet`。图表工作表上出现 `cells` / `pictures` / `charts` / `headerFooters` / `comments`
 / `hyperlinks` / `textBoxes` / `maths` / `shapes` / `connectors` / `smartArts` / `model3ds` / `from` / `to` / `anchor` / 偏移 / `wrap` / `id` 报 `InvalidField`。
`comments[]` 上出现 `id` / `replies` / `start` / `end` / `x` / `y`，`hyperlinks[]` 上出现 `text` / `anchor` / `id`，
或 drawing 项上出现 `kind` / `wrap` / `x` / `y` / `id`，报 `InvalidField`。
给了 `workbook.props` 时按 `setProps` / `setCustomProp` 同一套规则写 core / app / custom；
省略则仍写空的 `dc:title` 与 `dc:creator` 为 `ooxp`，不写 custom 部件。


## apply

各格式认哪些 `op` 以 [json-contract.md](json-contract.md#apply) 那一段名单为准。跨格式 op 的字段表也在主文件。

### setCell

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setCell"` |
| `id` | 单元格 id |
| `value` | 与 create 相同。`null` 表示清空该格的值和公式 |
| `formula` | 以 `=` 开头。不能与 `value` 同时出现 |
| `shared` | 可选布尔。`true`：把目标格当共享公式主格，保住、迁主格或新建一组。`false`：先把该格所在共享组拆成独立公式，再按 `value` / `formula` 写主格 |
| `sharedRef` | 可选。共享组区域，A1 闭区间。只能与 `formula` 一起出现。目标格必须是该区域左上角 |
| `array` | 可选布尔。`true`：写成数组公式。`false`：若该格是数组主格，拆成普通公式或值 |
| `arrayRef` | 可选。数组公式区域，A1 闭区间。只能与 `formula` 一起出现。目标格必须是该区域左上角。给了 `arrayRef` 即视为数组公式，不必再写 `array: true` |

`arrayRef` 与 `sharedRef` 不能同时出现。`shared` 与 `array` 不能同时为 `true`。
`value` 不能与 `shared: true`、`array: true`、`sharedRef`、`arrayRef` 同时出现。`shared: false` 不能与 `sharedRef` 同时出现。
`array: false` 不能与 `arrayRef` 同时出现。目标格不是 `arrayRef` / `sharedRef` 的左上角时报 `InvalidField`。
`arrayRef` / `sharedRef` 非法时报 `InvalidRange`。`setCell` 不改工作表 `<hyperlinks>`，也不改 `extLst` 里的迷你图。
示例见 `docs/examples/xlsx-apply-formula.json` 与 `docs/examples/xlsx-apply-shared-master.json`。

共享公式（§18.3.1.40，`t="shared"`）：主格的 `<f>` 带 `ref` 和 `si`，从属格只带 `si`。
对**从属格**发 `setCell`（不带 `shared` / `sharedRef`）仍写成独立 `<f>`，主格不动。
对**主格**只给 `formula`、不给 `shared: false`：保住整组，只改主格 `<f>` 文本，保留 `t` / `si` / `ref`，从属格仍是空的共享 `<f>`。
给了 `sharedRef` 则目标格必须是新区域左上角：新范围内缺的从属格补空的 `<f t="shared" si>`；被踢出的格子按相对位移展开成独立公式；新范围内已有独立公式的格子改成该组从属格。新区域缩成单格则目标格改成普通公式。
对主格给 `shared: false`，或写 `value` / 清空：先拆组（每个从属格写成展开后的独立 `<f>`），再写主格。
给 `formula` 与 `sharedRef`，且新区域与**恰好一组**已有共享组相交（含目标本身属于该组）：沿用该组 `si`，把主格迁到目标格；给了 `formula` 就用这段文本当新主格公式，不从旧主格再平移一遍；旧主格若仍在新区域内则改成从属格。
与任何已有共享组都不相交则新建一组，`si` 取本表未用的下一个非负整数。新 `sharedRef` 同时盖住两组及以上已有共享组，或盖住数组 / 数据表，报 `NotImplemented`。
`arrayRef` 盖住其它共享组、数组或数据表时报 `NotImplemented`。对从属格只给 `shared: true`、不给 `sharedRef` 报 `InvalidField`。

相对展开：拆组与 inspect 从属格时，按主格与目标格的行列差平移公式里的 A1 单元格与 A1 区域（尊重 `$`）。工作表前缀、定义名、结构化表引用、函数名、字符串原样留下。出界写成 `#REF!`。
这不是插行时改其它格子引用的引擎；插行导致的引用漂移仍然要么拒绝、要么只改点名格。

数组公式（`t="array"`）：给 `formula` 与 `arrayRef`（或 `array: true`）时，在左上角写 `<f t="array" ref="…">`。
范围内其它格不写公式；还不存在则插入只有 `r` 的 `<c>`。对已有数组主格只给 `formula`、不给 `array: false`：保留 `t="array"` 与现有 `ref`，只改公式文本。
对数组主格写 `value` / 清空 / `array: false`：主格改成普通值或普通公式，范围内其它格留下缓存值。对数组范围内非主格发 `setCell` 报 `NotImplemented`。

数据表公式（`t="dataTable"`）：对主格或 `ref` 范围内的格子发 `setCell` 报 `NotImplemented`。本期不读写数据表。

create 本期仍只写普通 `<f>`，不写 `t="shared"` / `t="array"`。

### setCellStyle

写单元格的数字格式、字体、填充、边框与对齐。不改值、不改公式。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "output": "C:\\data\\book.style.xlsx",
  "ops": [
    {
      "op": "setCellStyle",
      "id": "cell:Sheet1!A1",
      "numFmt": "0.00",
      "fill": {
        "pattern": "solid",
        "fg": {
          "rgb": "FFFFFF00"
        }
      }
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setCellStyle"` |
| `id` | 单元格 id |
| `numFmt` | 可选。格式代码字符串，或 `null`（回到 `"General"` / `numFmtId="0"`）。空串报 `InvalidField` |
| `font` | 可选。对象或 `null`（回到 `fontId="0"`） |
| `fill` | 可选。对象或 `null`（回到 `fillId="0"`，无填充） |
| `border` | 可选。对象或 `null`（回到 `borderId="0"`，无线） |
| `align` | 可选。对象或 `null`（去掉 `xf` 上的 `alignment`） |

`numFmt`、`font`、`fill`、`border`、`align` 至少给一个，否则报 `InvalidField`。省略的键保持该格当前 `xf` 的对应分量。`null` 把该分量重置为默认。
目标格还不存在时按 `setCell` 同一套规则插入 `<row>` / `<c>`，只写 `r` 与 `s`。找不到工作表报 `IdNotFound`。id 不是单元格 id 报 `InvalidId`。
只能改 `worksheet`；对图表工作表上的格子报 `InvalidField`（与 `setCell` 相同）。docx / pptx 上发此 op 报 `InvalidField`。

与 `setCell` 同一次 `ops`：按数组顺序执行。`setCell` 改值或公式时保留已有 `s`，不改 `styles.xml`。
`setCellStyle` 只改 `s` 与 styles，不改 `<v>` / `<f>` / `<is>`。
先 `setCell` 再 `setCellStyle`，值与样式都在；先 `setCellStyle` 再 `setCell`，`setCell` 仍保留刚写上的 `s`。
`setCell` 清空时保留 `s`，因此先设样式再清空，格式还在。只跑 `setCellStyle` 不要标重算、不要删 `calcChain`。

`font` 对象字段：`name`（非空字符串）、`sz`（磅，大于 0 且不超过 409）、`bold` / `italic`（布尔）、`underline`（`true` 当作 `single`；
`false` 或 `null` 去掉下划线；或 `single` / `double` / `singleAccounting` / `doubleAccounting` / `none`）、`color`（颜色对象）。
给了 `font` 对象时，未点名的子键从当前字体抄来再合并。空 `name`、非法 `sz` / `underline` / `color` 报 `InvalidField`。

`fill` 对象字段：`pattern`（`ST_PatternType`，§18.18.55）、`fg`、`bg`（颜色对象）。只给了 `fg` 且未写 `pattern` 时当作 `solid`。
不写渐变；`pattern` 为 `gradient` 或不认识的值报 `InvalidField`。

`border` 对象：可给顶层 `style` / `color`，作为未点名边的缺省；再给 `left` / `right` / `top` / `bottom` 覆盖单边。
每边 `style` 为 `ST_BorderStyle`（§18.18.3，含 `none`）。不认识的 `style` 报 `InvalidField`。不写对角线。

`align` 对象字段：`horizontal`（`general`、`left`、`center`、`right`、`fill`、`justify`、`centerContinuous`、`distributed`）、
`vertical`（`top`、`center`、`bottom`、`justify`、`distributed`）、`wrap`（布尔）。不认识的对齐报 `InvalidField`。不写缩进、旋转、缩小字体。

颜色对象恰好含 `rgb`、`theme`、`indexed` 之一。`rgb` 为 6 位或 8 位十六进制（不带 `#`），写入时大写；6 位补成 `FF` + 六位 ARGB。`theme` 为非负整数，可再带数字 `tint`。
`indexed` 为非负整数。主题色原样写入，不解析成 RGB。同时给了多种颜色键、或 `tint` 却没有 `theme`，报 `InvalidField`。

写 `styles.xml` 时：能在已有 `numFmts` / `fonts` / `fills` / `borders` / `cellXfs` 里找到相同定义就复用下标，否则追加。不重排、不删、不改写未点名的 `xf` 条目。
未点名格子的 `s` 不变。`s="0"` 可省略该属性。没有 styles 部件则按 create 那份最小表新建，并补 workbook 关系与 Override。不改 `cellStyleXfs`、主题。
`setCellStyle` 不改 `dxfs`；条件格式写 `dxfs` 见 `insertConditionalFormat`。create 本期不写单元格样式。

示例见 `docs/examples/xlsx-apply-cellstyle.json`。

### setColWidth

写工作表列宽。JSON 的 `width` 是 Excel 列宽字符数（与 COM `ColumnWidth` 同一套），不是像素。
写入 `col/@width` 时按 ECMA-376 §18.3.1.13 的公式转换；MDW 固定用该节示例的 Calibri 11 点 7 像素，不读实际字体、不算最适列宽。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "output": "C:\\data\\book.layout.xlsx",
  "ops": [
    {
      "op": "setColWidth",
      "id": "sheet:Sheet1",
      "col": "A",
      "width": 20
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setColWidth"` |
| `id` | 工作表 id `sheet:{name}` |
| `col` | 必须。列字母，或列区间 `A:C`（闭区间，大写小写都认）。不要写单元格地址 |
| `width` | 可选。大于 0 且不超过 255 的数字，或 `null`（去掉这段的自定义列宽） |
| `bestFit` | 可选。布尔。`true` 只写 `bestFit="1"`，不算像素、不改现有 `width`（除非同时给了 `width`） |

`width` 与 `bestFit` 至少给一个，否则报 `InvalidField`。给了 `width` 数字时写 `customWidth="1"`；若未同时 `bestFit: true`，则去掉 `bestFit`。
`width` 为 `null` 时去掉这段的 `width` 与 `customWidth`。只改被点名的列：已有 `<col>` 的 `min`/`max` 盖住更多列时先拆开，未点名列的属性不变。
没有 `<cols>` 则在 `sheetData` 之前插入。相邻且属性相同的 `<col>` 可以合并，不是必须。找不到工作表报 `IdNotFound`。id 不是工作表 id 报 `InvalidId`。
只能改 `worksheet`；对图表工作表报 `InvalidField`。`col` 非法报 `InvalidRange`。docx / pptx 上发此 op 报 `InvalidField`。不标重算，不删 `calcChain`。
不按列宽反推 `twoCell` 图尺寸。create 本期不写列宽。

### setRowHeight

写行高。单位磅，对应 `row/@ht`。不算自动换行后的高度。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setRowHeight"` |
| `id` | 工作表 id `sheet:{name}` |
| `row` | 必须。行号，从 1 计 |
| `toRow` | 可选。闭区间右端行号。省略则只改 `row`。小于 `row` 报 `InvalidField` |
| `height` | 必须。大于 0 且不超过 409 的数字，或 `null`（去掉这些行的 `ht` 与 `customHeight`） |

给了数字时写 `customHeight="1"`。目标行还不存在时插入只有 `r` / `ht` / `customHeight` 的 `<row>`，插在正确行号顺序上。找不到工作表报 `IdNotFound`。
id 不是工作表 id 报 `InvalidId`。只能改 `worksheet`。docx / pptx 上发此 op 报 `InvalidField`。不标重算。create 本期不写行高。

### mergeRange

合并工作表单元格。不要跟 Word 表的 `mergeCells` 混用。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"mergeRange"` |
| `id` | 工作表 id `sheet:{name}` |
| `range` | 必须。A1 区域，例如 `A1:B2`。单格（`A1` 或 `A1:A1`）报 `InvalidField` |

内容留在左上角：范围内其余格子按 `setCell` 清空值和公式，保留 `s`。与新区域相交的已有 `<mergeCell>` 先删掉再写入这一条。
`mergeCells/@count` 写成子元素个数；删光则去掉 `mergeCells`。插入位置在 `sheetData` 之后、`phoneticPr` 等后续元素之前（§A.2 `CT_Worksheet`）。
范围内若有共享公式主格且仍有从属格、或数组公式主格、或数据表公式，报 `NotImplemented`，不写文件。找不到工作表报 `IdNotFound`。id 不是工作表 id 报 `InvalidId`。只能改 `worksheet`。
`range` 非法报 `InvalidRange`。docx / pptx 上发此 op 报 `InvalidField`。xlsx 上仍不要发 Word 的 `mergeCells`。不标重算。create 本期不写合并。

### unmergeRange

取消合并。不恢复合并时清掉的值。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"unmergeRange"` |
| `id` | 工作表 id `sheet:{name}` |
| `range` | 必须。A1 区域或单格。与该区域相交的 `<mergeCell>` 全部去掉 |

没有相交的合并则空操作，仍计入 `opsApplied`。找不到工作表报 `IdNotFound`。id 不是工作表 id 报 `InvalidId`。只能改 `worksheet`。
docx / pptx 上发此 op 报 `InvalidField`。不标重算。

示例见 `docs/examples/xlsx-apply-layout.json`。

### insertDataValidation

往工作表追加一条数据验证（ECMA-376 §18.3.1.32）。公式当字符串，不求值，不展开下拉列表。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "output": "C:\\data\\book.dv.xlsx",
  "ops": [
    {
      "op": "insertDataValidation",
      "id": "sheet:Sheet1",
      "sqref": "A1",
      "type": "list",
      "formula1": "1,2,3"
    }
  ]
}
```

- **`op`**
  - 必须是 `"insertDataValidation"`
- **`id`**
  - 工作表 id `sheet:{name}`
- **`sqref`**
  - 必须
  - A1 区域字符串，或字符串数组
  - 多区域用空格分隔的一条字符串（`A1 C1:C3`）或数组 `["A1","C1:C3"]`
  - 写入时整条替换成 XML `sqref`，不做并集运算
- **`type`**
  - 必须
  - 包括
    - `none`
    - `whole`
    - `decimal`
    - `list`
    - `date`
    - `time`
    - `textLength`
    - `custom`
- **`operator`**
  - 可选
  - 包括
    - `between`
    - `notBetween`
    - `equal`
    - `notEqual`
    - `lessThan`
    - `lessThanOrEqual`
    - `greaterThan`
    - `greaterThanOrEqual`
  - `list` / `custom` / `none` 上出现报 `InvalidField`
- **`formula1` / `formula2`**
  - 可选字符串
  - 写入 `<formula1>` / `<formula2>` 文本
  - 前导 `=` 会去掉
  - 不求值
- **`allowBlank`**
  - 可选布尔
  - 省略当作 `true`，写 `allowBlank="1"`
- **`dropdown`**
  - 可选布尔
  - 只对 `list` 有意义：`true`（省略）显示下拉，不写 XML `showDropDown`
  - `false` 隐藏下拉，写 `showDropDown="1"`
  - 其它 `type` 上出现报 `InvalidField`
- **`showInputMessage` / `showErrorMessage`**
  - 可选布尔
  - 省略当作 `true`
- **`errorStyle`**
  - 可选
  - `stop` / `warning` / `information`
  - 省略不写属性（XML 缺省 `stop`）
- **`errorTitle` / `error` / `promptTitle` / `prompt`**
  - 可选字符串，写入对应属性

`type` 为 `list` / `custom` / `whole` / `decimal` / `date` / `time` / `textLength` 时必须有 `formula1`，否则报 `InvalidField`。
`type` 为 `none` 时可以没有公式（只提示）。`operator` 为 `between` / `notBetween`，
或 `type` 属于 `whole` / `decimal` / `date` / `time` / `textLength` 且省略 `operator`（XML 缺省 `between`）时必须有 `formula2`。
`list` / `custom` / `none` 上出现 `formula2` 报 `InvalidField`。
`list` 的 `formula1` 若不是能解析的 A1 区域、也不含 `(` / `!`、且未以 `"` 开头，写入时加上双引号（Excel 序列字面量的写法）；范围引用如 `$A$1:$A$3` 原样写入。
新规则追加到该表 `<dataValidations>` 末尾；没有该元素则按 `CT_Worksheet` 顺序插在 `conditionalFormatting` 之后、`hyperlinks` 之前。`count` 写成子元素个数。
不检查与其它规则的 `sqref` 是否重叠。找不到工作表报 `IdNotFound`。id 不是工作表 id 报 `InvalidId`。只能改 `worksheet`。`sqref` 非法报 `InvalidRange`。
不认识的 `type` / `operator` / `errorStyle` 报 `InvalidField`。docx / pptx 上发此 op 报 `InvalidField`。不标重算，不删 `calcChain`。
不改 `extLst` 里的扩展校验。create 本期不写数据验证。

### setDataValidation

改已有规则。只改给了的字段。

- **`op`**
  - 必须是 `"setDataValidation"`
- **`id`**
  - 规则 id `sheet:{name}/dv:{n}`
- **其余**
  - 与 `insertDataValidation` 相同，全部可选
  - `formula1` / `formula2` / `operator` / `errorStyle` / 提示类字段可为 `null`，表示去掉该子元素或属性

除 `op` 与 `id` 外至少给一个字段，否则报 `InvalidField`。改完之后的规则仍须满足 insert 那套公式约束（例如改成 `list` 却没有 `formula1`）。`sqref` 一旦给出就整条替换。
未点名的属性、未知属性与未知子元素原样留。找不到该序号报 `IdNotFound`。id 不是规则 id 报 `InvalidId`。只能改 `worksheet`。docx / pptx 上发此 op 报 `InvalidField`。不标重算。

### deleteDataValidation

删掉一条规则。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteDataValidation"` |
| `id` | 规则 id `sheet:{name}/dv:{n}` |

删光则去掉 `<dataValidations>`。找不到该序号报 `IdNotFound`。id 不是规则 id 报 `InvalidId`。只能改 `worksheet`。
docx / pptx 上发此 op 报 `InvalidField`。不标重算。

示例见 `docs/examples/xlsx-apply-validation.json`。

### insertConditionalFormat

往工作表追加一条条件格式规则（ECMA-376 §18.3.1.18 / §18.3.1.10）。公式当字符串，不求值。不渲染色阶、数据条或图标。
始终新建一块 `<conditionalFormatting>`（一块一条规则），追加到已有条件格式之后、`dataValidations` 之前。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "output": "C:\\data\\book.cf.xlsx",
  "ops": [
    {
      "op": "insertConditionalFormat",
      "id": "sheet:Sheet1",
      "sqref": "A1:A10",
      "type": "cellIs",
      "operator": "greaterThan",
      "formulas": [
        "0"
      ],
      "dxf": {
        "fill": {
          "pattern": "solid",
          "fg": {
            "rgb": "FFC6EFCE"
          }
        }
      }
    }
  ]
}
```

- **`op`**
  - 必须是 `"insertConditionalFormat"`
- **`id`**
  - 工作表 id `sheet:{name}`
- **`sqref`**
  - 必须
  - A1 区域字符串，或字符串数组
  - 写入规则与 `insertDataValidation` 的 `sqref` 相同
- **`type`**
  - 必须
  - 本期可写：`cellIs` / `expression` / `colorScale` / `dataBar` / `iconSet`
  - 其余 `ST_CfType` 出现报 `InvalidField`
- **`operator`**
  - `cellIs` 必须
  - 包括
    - `lessThan`
    - `lessThanOrEqual`
    - `equal`
    - `notEqual`
    - `greaterThanOrEqual`
    - `greaterThan`
    - `between`
    - `notBetween`
  - 其它 `type` 上出现报 `InvalidField`
  - `containsText` / `notContains` / `beginsWith` / `endsWith` 本期不写
- **`formulas`**
  - 字符串数组
  - 写入 `<formula>` 文本，前导 `=` 会去掉，不求值
  - `cellIs` 的 `between` / `notBetween` 必须恰好 2 项，其余 `cellIs` 与 `expression` 必须恰好 1 项
  - 其它 `type` 上出现报 `InvalidField`
- **`dxf`**
  - 可选对象
  - 字段 `font` / `fill` / `border`，形状与 `setCellStyle` 相同
  - 写入 `styles.xml` 的 `<dxfs>`：能找到相同 `dxf` 就复用下标，否则追加，不删旧条
  - `fill` 只给了 `fg` 时，差分里同时写 `fgColor` 与 `bgColor`（Excel 条件格式填色常用 `bgColor`）
  - `colorScale` / `dataBar` / `iconSet` 上出现报 `InvalidField`
- **`colorScale`**
  - `type` 为 `colorScale` 时必须
  - 对象：`colors` 必须是 2 或 3 个颜色对象
  - `cfvo` 可选，长度必须与 `colors` 相同
  - 省略 `cfvo` 时两项当作 `min`/`max`，三项当作 `min` / `percentile`（`val` 为 `"50"`）/ `max`
- **`dataBar`**
  - `type` 为 `dataBar` 时可选
  - 省略则写 `min`/`max` 两个 `cfvo` 与颜色 `rgb=FF638EC6`
  - 对象字段：`cfvo`（必须 2 项，省略则用缺省）、`color`（省略则用缺省蓝）、`minLength` / `maxLength`（0–100 的整数
  - 省略不写属性，XML 缺省 10 与 90）、`showValue`（布尔
  - 省略或 `true` 不写属性）
- **`iconSet`**
  - `type` 为 `iconSet` 时可选
  - 省略则写 `set=3TrafficLights1` 与百分位 `0`/`33`/`67`
  - 对象字段：`set`（`ST_IconSetType`，§18.18.42
  - 省略当作 `3TrafficLights1`）、`cfvo`（项数必须等于该套图标个数：3 / 4 / 5
  - 省略则按个数均分 `percent`）、`showValue`、`reverse`、`percent`（布尔）
- **`priority`**
  - 可选正整数
  - 省略则取该表已有 `cfRule/@priority` 的最大整数加 1，没有则 `1`
  - 不检查是否与其它规则重复
- **`stopIfTrue`**
  - 可选布尔
  - `true` 写 `stopIfTrue="1"`
  - 省略或 `false` 不写

`cfvo` 每项：`type` 必须是 `num` / `percent` / `max` / `min` / `formula` / `percentile`。`val` 为字符串；`min` / `max` 可省略。
`gte` 为布尔，`false` 写 `gte="0"`。颜色对象与 `setCellStyle` 相同。
`type` 为 `cellIs` / `expression` 时不要出现 `colorScale` / `dataBar` / `iconSet`。
`type` 为 `colorScale` 时不要出现 `dataBar` / `iconSet` / `operator` / `formulas` / `dxf`，以此类推。找不到工作表报 `IdNotFound`。
id 不是工作表 id 报 `InvalidId`。只能改 `worksheet`。`sqref` 非法报 `InvalidRange`。不认识的枚举报 `InvalidField`。
docx / pptx 上发此 op 报 `InvalidField`。不标重算，不删 `calcChain`。不写 `x14` 扩展数据条（渐变、负值色、轴）或扩展图标集。create 本期不写条件格式。

### setConditionalFormat

改已有规则。只改给了的字段。

- **`op`**
  - 必须是 `"setConditionalFormat"`
- **`id`**
  - 规则 id `sheet:{name}/cf:{n}`
- **其余**
  - 与 `insertConditionalFormat` 相同，全部可选
  - `formulas` / `dxf` / `colorScale` / `dataBar` / `iconSet` / `operator` / `priority` 可为 `null`：`formulas` 为 `null` 去掉全部 `<formula>`
  - `dxf` 为 `null` 去掉 `dxfId`（不删 `styles.xml` 里的 `dxf`）
  - `colorScale` / `dataBar` / `iconSet` 为 `null` 去掉对应子元素
  - `priority` 为 `null` 报 `InvalidField`（XML 必有 `priority`）

除 `op` 与 `id` 外至少给一个字段，否则报 `InvalidField`。改 `sqref` 时改所属 `<conditionalFormatting>/@sqref`，同一块里其它规则一起变范围。
改 `type` 时只允许改成可写的五种；会清掉与新类型冲突的子元素与属性，并按 insert 同一套约束补齐。
对 inspect 列出的其它类型（如 `top10`），可以改 `sqref` / `priority` / `stopIfTrue` / `dxf`；改 `type` 到五种之一则整条按新类型重写；
给 `operator` / `formulas` / `colorScale` / `dataBar` / `iconSet` 却不改 `type` 报 `InvalidField`。
未点名的属性、未知属性与未知子元素（含该规则上的 `extLst`）原样留。改完之后的规则仍须满足 insert 那套约束。找不到该序号报 `IdNotFound`。id 不是规则 id 报 `InvalidId`。
只能改 `worksheet`。docx / pptx 上发此 op 报 `InvalidField`。不标重算。

### deleteConditionalFormat

删掉一条规则。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteConditionalFormat"` |
| `id` | 规则 id `sheet:{name}/cf:{n}` |

该块删光则去掉这块 `<conditionalFormatting>`。全部删光则表上不再有条件格式元素。其它规则的 `priority` 不重排。找不到该序号报 `IdNotFound`。id 不是规则 id 报 `InvalidId`。
只能改 `worksheet`。docx / pptx 上发此 op 报 `InvalidField`。不标重算。不删 `styles.xml` 里未再被引用的 `dxf`。

示例见 `docs/examples/xlsx-apply-conditional-format.json`。

### insertSheetTable

往工作表追加一张 SpreadsheetML 表（ECMA-376 §18.5.1.2）。单元格数据仍在 `sheetData` 里。不实现结构化引用求值，不渲染表样式。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "output": "C:\\data\\book.table.xlsx",
  "ops": [
    {
      "op": "insertSheetTable",
      "id": "sheet:Sheet1",
      "ref": "A1:C3",
      "name": "Sales",
      "columns": [
        "产品",
        "数量",
        "金额"
      ]
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertSheetTable"` |
| `id` | 工作表 id `sheet:{name}` |
| `ref` | 必须。A1 闭区间，至少两行一列（首行表头）。单格或只有一行报 `InvalidField` |
| `name` | 可选。写入 XML `name` 与 `displayName`。省略则取未占用的 `Table{n}`（`n` 从 1 计） |
| `columns` | 可选。列名字符串数组。长度必须等于 `ref` 的列数 |

`ref` 必须是单一矩形，不要多区域。与同一张工作表上已有表格的 `ref` 相交报 `InvalidField`。
`name` 不能为空、不能含空格；必须在整本工作簿的表格 `displayName` / `name` 以及 `definedNames` 里唯一，否则报 `InvalidField`。
省略 `name` 时从 `Table1` 起找第一个未占用名。`columns` 若给出，项不能为空，表内不能重名，并写入表头行对应格子（`t="inlineStr"`）。
省略 `columns` 时从表头行已有单元格取值：共享字符串、内联字符串或 `str` 用可见文本，数字/布尔/错误用缓存文本；空格用 `Column1` 起的未占用名。列名仍须表内唯一，冲突时在原名后加 `2`、`3`…。
始终写 `headerRowCount` 缺省（1）与 `totalsRowShown="0"`。表格部件里写与 `ref` 相同的 `<autoFilter>`。
写 `<tableStyleInfo name="TableStyleMedium2" showFirstColumn="0" showLastColumn="0" showRowStripes="1" showColumnStripes="0"/>`，不把样式摊到单元格。
XML `id` 取整本工作簿已有表格 `id` 的最大整数加 1，至少为 1。部件路径为未占用的 `/xl/tables/tableN.xml`。工作表关系 Type 以 `/table` 结尾。
Content Type 为 `application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml`。
`<tableParts>` 按 `CT_Worksheet` 顺序插在 `extLst` 之前，`count` 写成子元素个数。不改工作表级 `<autoFilter>`。找不到工作表报 `IdNotFound`。
id 不是工作表 id 报 `InvalidId`。只能改 `worksheet`。docx / pptx 上发此 op 报 `InvalidField`。不标重算，不删 `calcChain`。create 本期不写表格。

### setSheetTable

改已有表格。只改给了的字段。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setSheetTable"` |
| `id` | 表格 id `sheet:{name}/table:{n}` |
| `ref` | 可选。新的 A1 闭区间，约束与 insert 相同 |
| `name` | 可选。新的表名，约束与 insert 相同。`null` 报 `InvalidField` |
| `columns` | 可选。列名数组。长度必须等于改完之后 `ref` 的列数 |

除 `op` 与 `id` 外至少给一个字段，否则报 `InvalidField`。改 `ref` 时同步改表格部件 `@ref` 与 `<autoFilter>/@ref`。
列数变多则追加 `tableColumn`（名来自 `columns` 或 `ColumnN`），并写新表头格；列数变少则从末尾删掉多余 `tableColumn`，不删 `sheetData`。行数只改范围，不插删工作表行。
改 `name` 时 `name` 与 `displayName` 写成同一字符串。改 `columns` 时改 `@name` 并写表头格；未点名的 `tableColumn` 子元素（含计算列公式）在列还在时原样留。
未点名的属性与未知子元素原样留。找不到该序号报 `IdNotFound`。id 不是表格 id 报 `InvalidId`。只能改 `worksheet`。docx / pptx 上发此 op 报 `InvalidField`。不标重算。

### deleteSheetTable

删掉一张表格。单元格数据留下。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteSheetTable"` |
| `id` | 表格 id `sheet:{name}/table:{n}` |

去掉对应 `<tablePart>`、工作表关系、表格部件与 Content Type Override。`tableParts` 删光则去掉该元素。
不删 `sheetData`，不删工作表级 `<autoFilter>`，不删 `styles.xml`。找不到该序号报 `IdNotFound`。id 不是表格 id 报 `InvalidId`。只能改 `worksheet`。
docx / pptx 上发此 op 报 `InvalidField`。不标重算。

示例见 `docs/examples/xlsx-apply-table.json`。

### insertPivotTable

往工作表追加一张 SpreadsheetML 透视表（ECMA-376 §18.10）。库按源格子抄缓存记录，并标 `refreshOnLoad="1"`。不在库里聚合，也不把合计写进 `sheetData`。打开后由 Excel 刷新。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "output": "C:\\data\\book.pivot.xlsx",
  "ops": [
    {
      "op": "insertPivotTable",
      "id": "sheet:Sheet1",
      "source": {
        "sheet": "Sheet1",
        "ref": "A1:C4"
      },
      "location": "E1",
      "name": "SalesPivot",
      "rows": [
        "产品"
      ],
      "values": [
        "金额"
      ]
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertPivotTable"` |
| `id` | 工作表 id `sheet:{name}`，透视表写在这张表上 |
| `source` | 必须。对象，字段顺序约定为 `sheet`、`ref`。`sheet` 为源工作表名，`ref` 为源区 A1 闭区间，至少两行一列（首行字段名） |
| `location` | 必须。透视表主体左上角 A1，或一块区域（只用左上角） |
| `name` | 可选。写入 `@name`。省略则取未占用的 `PivotTable{n}`（`n` 从 1 计） |
| `rows` | 可选。行轴字段名字符串数组，须是源表头里的名字 |
| `cols` | 可选。列轴字段名 |
| `filters` | 可选。报表筛选（page）字段名 |
| `values` | 必须。值区字段名，至少一项 |

`source.ref` 必须是单一矩形。非法 A1 报 `InvalidRange`。只有一行或零列报 `InvalidField`。`source.sheet` 省略则与目标表相同，必须是已有 `worksheet`。
同一个字段名不能同时出现在 `rows` / `cols` / `filters` / `values` 里。`values` 缺省或空数组报 `InvalidField`。
`name` 不能为空、不能含空白；整本工作簿透视表名唯一（大小写不敏感）。省略则从 `PivotTable1` 起找第一个未占用名。
始终新建一份工作表源缓存（不与已有透视表共用），XML `cacheId` 取已有最大值加 1，至少为 1。
部件路径为未占用的 `/xl/pivotCache/pivotCacheDefinition{n}.xml`、`/xl/pivotCache/pivotCacheRecords{n}.xml`、`/xl/pivotTables/pivotTable{n}.xml`。
workbook 写 `<pivotCaches>`（`CT_Workbook` 里插在 `customWorkbookViews` 之后、`smartTagPr` 之前）。
缓存关系 Type 以 `/pivotCacheDefinition` 结尾，记录关系 Type 以 `/pivotCacheRecords` 结尾，工作表关系 Type 以 `/pivotTable` 结尾。
透视表部件再写一条指向该缓存定义的关系（同样以 `/pivotCacheDefinition` 结尾，ECMA-376 §12.3.11 / §12.3.12）。
对应 Content Type 为 `spreadsheetml.pivotCacheDefinition+xml`、`pivotCacheRecords+xml`、`pivotTable+xml`。
`<location>/@ref` 写成能盖住主体的最小矩形（含表头行与至少一行数据区），`firstHeaderRow="1"`、`firstDataRow="2"`、`firstDataCol="1"`。
缓存 `refreshOnLoad="1"`。不写 OLAP。找不到工作表报 `IdNotFound`。id 不是工作表 id 报 `InvalidId`。只能改 `worksheet`。
docx / pptx 上发此 op 报 `InvalidField`。不标重算。create 本期不写透视表。

### setPivotTable

改已有透视表。只改给了的字段。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setPivotTable"` |
| `id` | 透视表 id `sheet:{name}/pivot:{n}` |
| `source` | 可选。新的源，约束与 insert 相同 |
| `location` | 可选。新的主体左上角或区域 |
| `name` | 可选。新的表名，约束与 insert 相同。`null` 报 `InvalidField` |
| `rows` / `cols` / `filters` / `values` | 可选。字段名数组。给出则整轴替换。`values` 不能改成空数组 |

除 `op` 与 `id` 外至少给一个字段，否则报 `InvalidField`。
改 `source` 时按新源格子重写该缓存的 `cacheFields` 与 `cacheRecords`，并保持 `refreshOnLoad="1"`。若该缓存还被其它透视表使用，先复制一份新缓存再改，避免牵连。
字段名必须仍在新表头里。改 `location` 时只改 `<location>/@ref` 的左上角并按当前轴重算最小矩形。未点名的属性与未知子元素原样留。找不到该序号报 `IdNotFound`。
id 不是透视表 id 报 `InvalidId`。源不是工作表缓存报 `InvalidField`。只能改 `worksheet`。docx / pptx 上发此 op 报 `InvalidField`。不标重算。

### deletePivotTable

删掉一张透视表。源格子留下。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deletePivotTable"` |
| `id` | 透视表 id `sheet:{name}/pivot:{n}` |

去掉工作表关系、透视表部件与 Content Type Override。若没有其它透视表再用该 `cacheId`，同时去掉 workbook `<pivotCache>`、缓存定义与记录部件及其关系与 Override。
`pivotCaches` 删光则去掉该元素。不删 `sheetData`。找不到该序号报 `IdNotFound`。id 不是透视表 id 报 `InvalidId`。只能改 `worksheet`。
docx / pptx 上发此 op 报 `InvalidField`。不标重算。

示例见 `docs/examples/xlsx-apply-pivot.json`。

### insertDefinedName

往工作簿追加一条定义名（ECMA-376 §18.2.5）。公式当字符串写入元素文本，不解析、不求值。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "output": "C:\\data\\book.named.xlsx",
  "ops": [
    {
      "op": "insertDefinedName",
      "id": "xlsx:workbook",
      "name": "Sales",
      "formula": "Sheet1!$A$1:$A$10"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertDefinedName"` |
| `id` | 必须是 `xlsx:workbook` |
| `name` | 必须。写入 `@name` |
| `formula` | 必须。写入元素文本。不要前导 `=` |
| `hidden` | 可选。布尔。`true` 写 `@hidden="1"`；省略或 `false` 不写该属性 |
| `localSheet` | 可选。工作表名，写成 `@localSheetId`（`sheets` 里 0 基下标）。省略则为簿级 |

`name` 长度 1 到 255 个字符。首字符必须是字母、`_` 或 `\`；其余只能是字母、数字、`.` 或 `_`。不能含空白。不能是 `R` 或 `C`（不分大小写）。
不能整段解析成 A1 单元格或 A1 区域（例如 `A1`、`LOG10`）。不能整段看成 R1C1 引用（例如 `R1C1`、`RC`）。`_xlnm.Print_Area` 合法。
同一作用域内大小写不敏感唯一：簿级互斥；同一 `localSheetId` 互斥；簿级与表级可以同名。还须与整本工作簿表格的 `displayName` / `name` 互斥。
`formula` 去掉首尾空白后不能为空，且不能以 `=` 开头。`localSheet` 必须是已有表名；找不到报 `IdNotFound`。空串或 `null` 报 `InvalidField`。
id 不是 `xlsx:workbook` 报 `InvalidId`。`<definedNames>` 按 `CT_Workbook` 插在 `externalReferences` 之后、`calcPr` 之前；`count` 写成子元素个数。
docx / pptx 上发此 op 报 `InvalidField`。不标重算，不删 `calcChain`。create 本期不写定义名。

### setDefinedName

改已有定义名。只改给了的字段。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setDefinedName"` |
| `id` | 定义名 id `name:{n}` |
| `name` | 可选。新的名，约束与 insert 相同。`null` 报 `InvalidField` |
| `formula` | 可选。新的公式文本，约束与 insert 相同。`null` 报 `InvalidField` |
| `hidden` | 可选。布尔。`true` 写 `@hidden="1"`；`false` 去掉该属性。`null` 报 `InvalidField` |
| `localSheet` | 可选。工作表名则改成该表级；`null` 表示改成簿级（去掉 `@localSheetId`）。省略表示不改 |

除 `op` 与 `id` 外至少给一个字段，否则报 `InvalidField`。改 `name` 或 `localSheet` 时按新作用域做唯一性检查。未点名的属性与未知子元素原样留。找不到该序号报 `IdNotFound`。
id 不是定义名 id 报 `InvalidId`。docx / pptx 上发此 op 报 `InvalidField`。不标重算。

### deleteDefinedName

删掉一条定义名。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteDefinedName"` |
| `id` | 定义名 id `name:{n}` |

去掉该 `<definedName>`。删光则去掉 `<definedNames>`。找不到该序号报 `IdNotFound`。id 不是定义名 id 报 `InvalidId`。
docx / pptx 上发此 op 报 `InvalidField`。不标重算。

示例见 `docs/examples/xlsx-apply-defined-name.json`。

### 重算标记

任意一条 `setCell` 成功之后，必须让打开文件的 Excel 重算。第一期用这两步，两步都做：

1. 在 `/xl/workbook.xml` 的 `<calcPr>` 上设置 `fullCalcOnLoad="1"`。没有 `<calcPr>` 就建一个。其余 `calcPr` 属性保留。依据 ECMA-376 §18.2.2。
2. 若存在 `/xl/calcChain.xml`，删除该部件、内容类型 Override、以及 workbook 关系里指向它的那条。过期计算链会让部分 Excel 版本沿用旧顺序。

不在库里求值，不写新的缓存 `<v>`。

### 改动范围

只允许变脏的部件：

- 目标工作表 XML
- `/xl/workbook.xml`（仅 `calcPr`，以及给隐式编号补 `r` 这类语义等价的改写；`setCellStyle` 单独出现时不要为重算去改它）
- 可能被删除的 `/xl/calcChain.xml` 以及因此修改的关系与内容类型（仅 `setCell` 等改了值或公式时）
- `setCellStyle` 还可脏 styles 部件（常见 `/xl/styles.xml`），必要时补 workbook 关系与 Override

未点名的 `xf` 条目、其他工作表、主题一律按原来的语义留下：不要重排或改写未点名下标。只改单元格值、不改样式、不改图、图表或页眉时，`styles.xml` 与绘图原样写回。
图表 op 还可以脏 drawing、`/xl/charts/…` 及其 `.rels`、`[Content_Types].xml`，以及被写入数据的工作表 XML；写入单元格时走上面同一套重算标记。
`insertChartSheet` / `deleteChartSheet` / `insertSheet` / `deleteSheet` / `renameSheet` / `moveSheet`
 / `setSheetHidden` 还可以脏 `/xl/workbook.xml` 的 `sheets`、workbook 关系，以及新表或被删表的部件。
工作表表格 op 还可以脏 `/xl/tables/…`、工作表 `tableParts`、工作表 `.rels` 与 `[Content_Types].xml`；
改列名或 insert 给出 `columns` 时还可脏该表 `sheetData`。
页眉文本只脏目标表 XML。`setPrintSetup` 脏目标表的 `pageMargins` / `pageSetup` / `printOptions` / `sheetPr` / `rowBreaks` / `colBreaks`；
改打印区域时还脏 `/xl/workbook.xml` 的 `<definedNames>`。
`setSheetView` 脏目标表的 `sheetViews`（含拆分）。`setAutoFilter` 脏目标表的工作表级 `autoFilter`，不改表格部件。
`setSortState` 脏目标表的工作表级 `sortState`，不重排 `sheetData`，不改表格部件。
`insertCustomView` / `deleteCustomView` / `showCustomView` 脏 `/xl/workbook.xml` 的 `customWorkbookViews` 以及各表
`customSheetViews` / 当前 `sheetViews` / 可选 `pageMargins`，不新建部件。
`groupRows` / `ungroupRows` / `groupCols` / `ungroupCols` / `setOutline` 脏目标表的 `sheetPr` / `sheetFormatPr` / `cols` / `sheetData` 行属性，不新建部件。
页眉图还可以脏 `drawingHF` 指向的 drawing、工作表 `.rels`、`/xl/media/…` 与 `[Content_Types].xml`。
文本框、形状与连接线 op、`groupShapes` / `ungroup` 以及 `setShapeText` / `setShapeFormat` 还可以脏 worksheet drawing 及其 `.rels`。
`insertOle` / `deleteOle` 还可以脏工作表 `oleObjects`、worksheet drawing、`legacyDrawing` VML、工作表 `.rels`、VML `.rels`、
`/xl/embeddings/…`、`/xl/media/…` 与 `[Content_Types].xml`。
`insertSlicer` / `deleteSlicer` 还可以脏 `/xl/slicers/…`、`/xl/slicerCaches/…`、worksheet drawing、工作表 `extLst` 的 `slicerList`、
workbook `extLst` 的 `slicerCaches`、workbook 的 `<definedNames>`、工作表 / workbook `.rels` 与 `[Content_Types].xml`；
透视源的 `insertSlicer` 还脏 `/xl/pivotCache/pivotCacheDefinitionN.xml` 的 `extLst`。
`insertConnection` / `deleteConnection` 还可以脏 `/xl/connections.xml`、workbook `.rels` 与 `[Content_Types].xml`。
`insertExternalLink` / `deleteExternalLink` 还可以脏 `/xl/workbook.xml` 的 `<externalReferences>`、
`/xl/externalLinks/…` 及其 `.rels`、workbook `.rels` 与 `[Content_Types].xml`。
SmartArt op 还可以脏 worksheet drawing、drawing `.rels`、`/xl/diagrams/` 四件套与可选 drawing 缓存、以及 `[Content_Types].xml`。
`setProps` / `setCustomProp` / `deleteCustomProp` 还可以脏 `/docProps/core.xml`、`/docProps/app.xml`、`/docProps/custom.xml`、
根 `.rels` 与 `[Content_Types].xml`。
只改 core 字段时不要新建 custom。未点名的 `HeadingPairs` / `TitlesOfParts` 留下。

被改的部件是整份重新序列化的，不是按字节做定点替换。因此保证到这个程度：XML 声明、元素与属性的顺序、未知命名空间的元素与属性、文本内容都保留；注释、处理指令和 DOCTYPE 会丢，CDATA 会变成等价的转义文本。Office 的工作表部件不带这些东西，所以第一期接受这个代价。

### setHeaderFooterText

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "output": "C:\\data\\book.hf.xlsx",
  "ops": [
    {
      "op": "setHeaderFooterText",
      "id": "sheet:Sheet1/hf:odd:header",
      "text": "&C新页眉"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setHeaderFooterText"` |
| `id` | 页眉槽位 id，例如 `sheet:Sheet1/hf:odd:header` |
| `text` | 该槽位原文，字符串，可空。必填 |

找不到该表报 `IdNotFound`。id 不是页眉槽位 id 报 `InvalidId`。对图 id、工作表 id 或单元格 id 发此 op 报 `InvalidId`。
`worksheet` / `chartsheet` / `dialogsheet` 都可以改文本。对 `macrosheet` 本期只读页眉原文，不新开写入。没有 `headerFooter` 则按 CT_Worksheet 顺序新建。
`text` 为空串时去掉该槽位元素；六个槽位都空时仍保留空的 `headerFooter`（若已有 `differentOddEven` / `differentFirst` 等属性则留下）。
写 `even` 槽位时给 `headerFooter` 设 `differentOddEven="1"`；写 `first` 槽位时设 `differentFirst="1"`。不自动清这两项属性。不改 VML。不解释 `&P` 等代码。
docx / pptx 上发此 op 报 `InvalidField`。create 写页眉页脚见上文 `sheets[].headerFooters`。

### setPrintSetup

改工作表或图表工作表的页边距、纸张、方向、缩放、打印网格线、打印区域与手工分页符。不改页眉页脚原文（仍走 `setHeaderFooterText`）。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "output": "C:\\data\\book.print.xlsx",
  "ops": [
    {
      "op": "setPrintSetup",
      "id": "sheet:Sheet1",
      "left": 1,
      "orientation": "landscape",
      "printArea": "A1:A10"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setPrintSetup"` |
| `id` | 工作表 id `sheet:{name}` |
| `left` / `right` / `top` / `bottom` / `header` / `footer` | 可选。英寸数字，须有限且不小于 0 |
| `paperSize` | 可选。ECMA 整数，至少为 1 |
| `orientation` | 可选。`portrait` 或 `landscape` |
| `scale` | 可选。整数，10 到 400 |
| `fitToPage` | 可选。布尔 |
| `fitToWidth` / `fitToHeight` | 可选。非负整数 |
| `gridLines` / `headings` / `hCenter` / `vCenter` | 可选。布尔 |
| `printArea` | 可选。单一 A1（可带本表名前缀），或 `null` 删除该表 `_xlnm.Print_Area` |
| `rowBreaks` / `colBreaks` | 可选。从 1 计的整数数组，只写手工分页符 |

除 `op` / `id` 外至少给一个字段，否则报 `InvalidField`。边距写入 `pageMargins`（六个属性都必填，未点名的用当前值或缺省）。
`pageSetup` 按 `CT_Worksheet` 插在 `pageMargins` 之后、`headerFooter` 之前。
给了 `fitToWidth` / `fitToHeight` 时写 `sheetPr/pageSetUpPr/@fitToPage="1"`。给了 `scale` 且未同时给 fit 时把 `fitToPage` 写成假。
显式给了 `fitToPage` 则以该值为准，但仍遵守「给了 fit 则写成真」这条。`printArea` 写成该表级 `_xlnm.Print_Area`，公式为带表名前缀的绝对引用；`null` 删掉该表这类定义名。
定义名名单里仍能看见它。`rowBreaks` / `colBreaks` 替换手工 `<brk>`（`man="1"`），XML `@id` 为 JSON 减 1；其它 `brk` 留下。找不到该表报 `IdNotFound`。
id 不是工作表 id 报 `InvalidId`。对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。
对 `chartsheet` 发 `scale` / `fitToPage` / `fitToWidth` / `fitToHeight` / `gridLines` / `headings` / `hCenter`
 / `vCenter` / `rowBreaks` / `colBreaks` 报 `InvalidField`（图表工作表没有这些 XML）。
不改 `headerFooter`。docx / pptx 上发此 op 报 `InvalidField`。不标重算。create 本期不写打印设置。

示例见 `docs/examples/xlsx-apply-print.json`。

### setSheetView

改工作表或图表工作表默认视图的缩放、冻结窗格、拆分窗格与活动单元格。不改分页预览 / 页面布局。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "output": "C:\\data\\book.view.xlsx",
  "ops": [
    {
      "op": "setSheetView",
      "id": "sheet:Sheet1",
      "zoom": 80,
      "frozenRows": 1,
      "frozenCols": 0
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setSheetView"` |
| `id` | 工作表 id `sheet:{name}` |
| `zoom` | 可选。整数，10 到 400 |
| `frozenRows` / `frozenCols` | 可选。非负整数。行不超过 1048575，列不超过 16383 |
| `splitRows` / `splitCols` | 可选。非负整数。上限与冻结相同。与 COM `SplitRow` / `SplitColumn` 同一套 |
| `activeCell` | 可选。单格 A1 |

除 `op` / `id` 外至少给一个字段，否则报 `InvalidField`。
只改第一个 `sheetView`；没有 `sheetViews` 则按 `CT_Worksheet` 顺序新建，该视图写 `workbookViewId="0"`。`zoom` 写 `@zoomScale`。
任一冻结字段与任一拆分字段同时出现报 `InvalidField`。给了任一冻结字段时写入 `<pane state="frozen">`：`ySplit` 为行数、`xSplit` 为列数（为 0 则不写该属性），
`topLeftCell` 为冻结区右下角的下一格，`activePane` 按两侧是否非零取 `bottomLeft` / `topRight` / `bottomRight`。
只给一侧冻结时另一侧沿用当前冻结值（拆分 pane 当作当前 0）。两侧冻结都为 0 则去掉 `<pane>`。
给了任一拆分字段时写入拆分 pane：不写 `state`（规范缺省 `split`）；`topLeftCell` 为拆分区右下角的下一格；
`xSplit` / `ySplit` 按缺省列宽（Calibri 11，8.43 字符换成像素再换成 1/20 磅）与 Excel 16.0 金样行高（`SplitRow=4` 对应 `ySplit=1425`）写入，不保证与 Excel 像素级一致。
手测认 COM `SplitRow` / `SplitColumn`。只给一侧拆分时另一侧沿用当前拆分值（冻结 pane 当作当前 0）。两侧拆分都为 0 则去掉 `<pane>`。
改冻结或拆分时重写对应 `activePane` 的 `<selection>`，其它 `selection` 去掉；未点名的 `pivotSelection` 与未知子元素留下。
`activeCell` 须能解析成单格；写成该选区的 `activeCell` 与 `sqref`。
对 `chartsheet` 发 `frozenRows` / `frozenCols` / `splitRows` / `splitCols` / `activeCell` 报 `InvalidField`。
对 `chartsheet` 给了 `zoom` 时写 `@zoomScale` 并把 `@zoomToFit` 写成 `0`。找不到该表报 `IdNotFound`。id 不是工作表 id 报 `InvalidId`。
对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。docx / pptx 上发此 op 报 `InvalidField`。不标重算。create 本期不写视图。

示例见 `docs/examples/xlsx-apply-view.json`。

### setAutoFilter

改工作表级自动筛选范围。不改表格部件里的筛选，不写筛选条件，不隐藏行。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "output": "C:\\data\\book.filter.xlsx",
  "ops": [
    {
      "op": "setAutoFilter",
      "id": "sheet:Sheet1",
      "ref": "A1:A10"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setAutoFilter"` |
| `id` | 工作表 id `sheet:{name}` |
| `ref` | 必须。单一 A1，或 `null` 删除工作表级 `<autoFilter>` |

缺 `ref` 报 `InvalidField`。写入 `<autoFilter>/@ref`，能解析则规范化成 `A1` 或 `A1:B2`。已有该元素时只改 `@ref`，`filterColumn` 等子元素留下。
没有则按 `CT_Worksheet` 插在 `scenarios` 之后、`sortState` 之前。`null` 去掉该元素。与同一张工作表上已有表格的 `ref` 相交报 `InvalidField`。
只能改 `worksheet`。对 `chartsheet` 报 `InvalidField`。找不到该表报 `IdNotFound`。id 不是工作表 id 报 `InvalidId`。
对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。docx / pptx 上发此 op 报 `InvalidField`。不标重算。create 本期不写筛选。

示例见 `docs/examples/xlsx-apply-view.json`。

### setSortState

改工作表级排序条件。不重排 `sheetData`，不隐藏行，不求值。不改表格部件里的排序。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "output": "C:\\data\\book.sort.xlsx",
  "ops": [
    {
      "op": "setSortState",
      "id": "sheet:Sheet1",
      "ref": "A1:A3",
      "columns": [
        {
          "ref": "A1:A3",
          "descending": true
        }
      ]
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setSortState"` |
| `id` | 工作表 id `sheet:{name}` |
| `ref` | 必须。单一 A1，或 `null` 删除工作表级 `<sortState>` |
| `caseSensitive` | 可选布尔。缺省 `false`。为真才写属性 |
| `columnSort` | 可选布尔。缺省 `false`（按列、自上而下）。为真才写属性 |
| `columns` | `ref` 非 `null` 时必须、且不能空。每项必须给 `ref`；`descending` 可选，缺省 `false` |

缺 `ref` 报 `InvalidField`。`ref` 为 `null` 时不要出现 `columns` / `caseSensitive` / `columnSort`，出现报 `InvalidField`；并去掉 `<sortState>`。
`ref` 非 `null` 时缺 `columns` 或空数组报 `InvalidField`。每项 `ref` 必须能解析成单一 A1，且必须落在 `sortState/@ref` 内，否则报 `InvalidField`。
出现 `customList` / `sortBy` / `sortMethod` / `dxfId` / `iconSet` / `iconId` 报 `InvalidField`。
整段替换已有 `<sortState>`，不保留未点名的 `sortCondition` 或 `extLst`。没有则按 `CT_Worksheet` 插在 `autoFilter` 之后、`dataConsolidate` 之前。
为真的 `caseSensitive` / `columnSort` / `descending` 写成 `"1"`。不写 `sortBy`（规范缺省 `value`）。不重排格子。只能改 `worksheet`。
对 `chartsheet` 报 `InvalidField`。找不到该表报 `IdNotFound`。id 不是工作表 id 报 `InvalidId`。
对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。docx / pptx 上发此 op 报 `InvalidField`。不标重算。create 本期不写排序。

示例见 `docs/examples/xlsx-apply-sort.json`。

### insertCustomView

把当前窗口设置存成一条自定义视图。不恢复隐藏行 / 列。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "output": "C:\\data\\book.custom-view.xlsx",
  "ops": [
    {
      "op": "insertCustomView",
      "id": "xlsx:workbook",
      "name": "DemoView",
      "includePrintSettings": true,
      "includeHiddenRowCol": true
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertCustomView"` |
| `id` | 必须是 `xlsx:workbook` |
| `name` | 必须。非空。工作簿内大小写不敏感唯一 |
| `includePrintSettings` | 可选。布尔。省略为 `true` |
| `includeHiddenRowCol` | 可选。布尔。省略为 `true` |

缺 `name` 或空串报 `InvalidField`。重名报 `InvalidField`。id 不是 `xlsx:workbook` 报 `InvalidId`。
写入 workbook `<customWorkbookViews>/<customWorkbookView>`（`CT_Workbook` 插在 `calcPr` 之后、`pivotCaches` 之前）：必填 `name`、
`guid`、`windowWidth`、`windowHeight`、`activeSheetId`。
窗口尺寸从 `bookViews/workbookView` 同名属性除以 15；没有 `bookViews` 则 `windowWidth="1456"`、`windowHeight="1007"`。
`activeSheetId` 取当前 `tabSelected` 的表，没有则取第一张表的 `@sheetId`。
`includePrintSettings` / `includeHiddenRowCol` 为真则不写属性（规范缺省为真），为假则写成 `"0"`。
给每张 `worksheet` 与 `chartsheet` 写一条同一 `guid` 的 `customSheetView`：有冻结或拆分则复制当前第一个 `sheetView` 的 `pane` 与 `selection`；
`includePrintSettings` 为真时再写该表 `pageMargins`（没有则用缺省边距）。
不写 `xr:uid`。不把隐藏行 / 列抄进快照。docx / pptx 上发此 op 报 `InvalidField`。不标重算。create 本期不写自定义视图。

### deleteCustomView

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteCustomView"` |
| `id` | 自定义视图 id `customView:{n}` |

去掉该 `<customWorkbookView>`，以及各表上同一 `guid` 的 `<customSheetView>`。删光则去掉 `<customWorkbookViews>` / 空的 `<customSheetViews>`。
找不到该序号报 `IdNotFound`。id 不是自定义视图 id 报 `InvalidId`。docx / pptx 上发此 op 报 `InvalidField`。不标重算。

### showCustomView

按快照把窗格写回当前默认视图。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"showCustomView"` |
| `id` | 自定义视图 id `customView:{n}` |

对每张带同一 `guid` 的 `customSheetView`：把它的 `pane` / `selection` 写到该表第一个 `sheetView`（没有则新建，`workbookViewId="0"`）。
workbook 上 `includePrintSettings` 为真时，若快照有 `pageMargins` 则写回该表。不恢复隐藏行 / 列，不改筛选。找不到该序号报 `IdNotFound`。
id 不是自定义视图 id 报 `InvalidId`。docx / pptx 上发此 op 报 `InvalidField`。不标重算。

示例见 `docs/examples/xlsx-apply-custom-view.json`。

### groupRows

给指定行的大纲级加 1。不写隐藏、不写折叠、不改汇总方向。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "output": "C:\\data\\book.outline.xlsx",
  "ops": [
    {
      "op": "groupRows",
      "id": "sheet:Sheet1",
      "rows": "2:4"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"groupRows"` |
| `id` | 工作表 id `sheet:{name}` |
| `rows` | 必须。行号或闭区间，例如 `2` 或 `2:4` |

范围内每一行的 `outlineLevel` 加 1。没有 `<row>` 则新建，只写 `r` 与 `outlineLevel`。已是 7 则整条 op 报 `InvalidField`，不改文件。
写完后把 `sheetFormatPr/@outlineLevelRow` 同步成全表最高级；全为 0 则去掉该属性。
没有 `sheetFormatPr` 则新建，并写 `defaultRowHeight`（15，与列宽行高缺省同一套）；
已有该元素却缺 `defaultRowHeight` 则补上，不覆盖已有值（ECMA-376 §18.3.1.81 该属性为必填，缺了 Excel 会修复）。
`rows` 非法报 `InvalidRange`。找不到该表报 `IdNotFound`。id 不是工作表 id 报 `InvalidId`。只能改 `worksheet`。对 `chartsheet` 报 `InvalidField`。
对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。docx / pptx 上发此 op 报 `InvalidField`。不标重算。create 本期不写分组。

示例见 `docs/examples/xlsx-apply-outline.json`。

### ungroupRows

给指定行的大纲级减 1。减到 0 则去掉 `@outlineLevel`。范围内本来就是 0 的行不动。同步 `outlineLevelRow`。字段与错误与 `groupRows` 相同，`op` 必须是 `"ungroupRows"`。不写隐藏、不写折叠。

### groupCols

给指定列的大纲级加 1。不写隐藏、不写折叠。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"groupCols"` |
| `id` | 工作表 id `sheet:{name}` |
| `col` | 必须。列字母或闭区间，例如 `B` 或 `B:C`，与 `setColWidth` 同一套 |

已有 `<col>` 的 `min`/`max` 盖住更多列时先拆开，只改被点名列的 `outlineLevel`。
没有对应 `<col>` 则新建，写 `min` / `max`、缺省 `width`（与 `setColWidth` 同一套 Calibri 11 换算，约 8.43 字符）和 `outlineLevel`。已有 `width` 则不改。
不写 `customWidth`。已是 7 则整条 op 报 `InvalidField`。
写完后同步 `sheetFormatPr/@outlineLevelCol`，并按 `groupRows` 同一条规则保证 `defaultRowHeight`。`col` 非法报 `InvalidRange`。其余错误与 `groupRows` 相同。不标重算。

### ungroupCols

给指定列的大纲级减 1。减到 0 则去掉 `@outlineLevel`。字段与错误与 `groupCols` 相同，`op` 必须是 `"ungroupCols"`。

### setOutline

改汇总方向，或按点名的细节范围折叠 / 展开。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "ops": [
    {
      "op": "setOutline",
      "id": "sheet:Sheet1",
      "summaryBelow": false,
      "collapseRows": "2:4",
      "collapsed": true
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setOutline"` |
| `id` | 工作表 id `sheet:{name}` |
| `summaryBelow` | 可选。布尔。`false` 写 `outlinePr/@summaryBelow="0"`；`true` 去掉该属性（规范缺省为真） |
| `summaryRight` | 可选。布尔。`false` 写 `@summaryRight="0"`；`true` 去掉该属性 |
| `collapseRows` | 可选。细节行范围，写法与 `groupRows` 的 `rows` 相同 |
| `collapseCols` | 可选。细节列范围，写法与 `groupCols` 的 `col` 相同 |
| `collapsed` | 有 `collapseRows` 或 `collapseCols` 时必须。布尔 |

除 `op` 与 `id` 外至少给一个可写字段，否则报 `InvalidField`。先写汇总方向，再按写完后的方向折叠。
`collapsed: true` 时细节行 / 列写 `hidden="1"`；
汇总行（`summaryBelow` 为真则为细节末行加 1，否则为首行减 1）写 `collapsed="1"`，列同理（`summaryRight` 为真则为细节末列加 1）。
汇总行 / 列越界报 `InvalidField`。`collapsed: false` 时去掉这些 `hidden` / `collapsed`。
给了折叠范围却没有 `collapsed`，或只给 `collapsed` 没有范围，报 `InvalidField`。折叠不把细节列改成 `width="0"`。
新建仅含 `collapsed` 的汇总列时，若没有 `width` 则补缺省列宽。`outlinePr` 两个属性都缺省且没有子元素时去掉该元素。只能改 `worksheet`。
对 `chartsheet` 报 `InvalidField`。找不到该表报 `IdNotFound`。id 不是工作表 id 报 `InvalidId`。
对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。docx / pptx 上发此 op 报 `InvalidField`。不标重算。create 本期不写分组。

### setPictureAnchor

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "ops": [
    {
      "op": "setPictureAnchor",
      "id": "sheet:Sheet1/pic:2",
      "anchor": "oneCell",
      "from": "C3"
    }
  ]
}
```

- **`op`**
  - 必须是 `"setPictureAnchor"`
- **`id`**
  - xlsx 图 id
- **`anchor`**
  - 可选
  - `oneCell`、`twoCell` 或 `absolute`
- **`from`**
  - 可选
  - 左上角 A1
  - 从 `absolute` 转出时必须
- **`to`**
  - 可选
  - `twoCell` 的右下角 A1
  - 转到 `twoCell` 时必须（请求给，或原来就是 `twoCell` 已有）
- **`fromOffX` / `fromOffY` / `toOffX` / `toOffY`**
  - 可选
  - EMU，不得为负
- **`posX` / `posY`**
  - 可选
  - EMU，非负
  - 已是 `absolute` 时改位置
  - 转到 `absolute` 时必须成对给出
- **`editAs`**
  - 可选
  - 只对 `twoCell`：`twoCell` / `oneCell` / `absolute`
- **`cx` / `cy`**
  - 转到 `oneCell` 或从 `twoCell` 转到 `absolute` 时必须成对给出（库不算列宽行高）
  - 已经是 `oneCell` / `absolute` 时请用 `setPictureSize`

`anchor`、位置字段、`editAs`、`cx` / `cy` 至少给一类，否则报 `InvalidField`。找不到该图报 `IdNotFound`。`id` 不是 xlsx 图 id 报 `InvalidId`。
docx / pptx 上发此 op 报 `InvalidField`。pptx 改画布坐标请用 `setPicturePos`。

三种锚点可以互转：改锚点元素名，留下 `xdr:pic` 与 `xdr:clientData`。转到 `twoCell` 必须有 `to`（请求给，或原来就是 `twoCell` 已有）。
转到 `oneCell` 必须有 `cx` 与 `cy`（请求给，或原来就是 `oneCell` / `absolute` 已有 `xdr:ext`）。
从 `twoCell` 转出时 `cx` / `cy` 必须出现在请求里，不要用 `twoCell` 锚点上可能存在的 `a:ext` 去猜显示尺寸。
转到 `absolute` 必须给 `posX` 与 `posY`；从 `twoCell` 转入还须给 `cx` 与 `cy`。从 `absolute` 转出必须给 `from`（转到 `twoCell` 还须给 `to`）。
缺字段报 `InvalidField`，不写文件。已有 `absolute` 且不改 `anchor` 时只许改 `posX` / `posY`。`insertPicture` 仍不认 `absolute`。

未点名的位置属性原样留下。`from` / `to` 不是合法 A1 报 `InvalidRange`。

### setTextBoxAnchor

字段与 `setPictureAnchor` 相同。`id` 可以是 xlsx 文本框 id、形状 id 或连接线 id。
改包含该 `xdr:sp` 或 `xdr:cxnSp` 的外层锚点，规则与 `setPictureAnchor` 相同（留下该对象或其所在 `xdr:grpSp`，以及 `xdr:clientData`）。docx / pptx 上发此 op 报 `InvalidField`。

### insertShape

往工作表插入一个无 `txBox` 的 `xdr:sp`。不要和 `insertTextBox` 混用。pptx 的字段与画布坐标见 [pptx 分册](json-contract-pptx.md#insertshape)。

- **`op`**
  - 必须是 `"insertShape"`
- **`id`**
  - 必须是 `sheet:{name}`
- **`from`**
  - 必须
  - 左上角单元格，A1
- **`to` / `anchor` / 偏移 / `cx` / `cy`**
  - 可选
  - 规则与 `insertTextBox` 相同
- **`text`**
  - 可选
  - 可见纯文本，省略则空 `txBody`
- **`sz` / `color` / `textFill` / `textOutline` / `bold` / `italic` / `underline` / `font` / `eastAsiaFont` / `align` / `textAnchor`**
  - 可选
  - 规则与主契约 `setShapeFormat` 相同
  - `anchor` 仍是单元格锚点，不要用来表示框内垂直对齐
- **`prst`**
  - 可选
  - `a:prstGeom/@prst`
  - 省略为 `rect`
  - 必须是 pptx `insertShape` 那份可写预设名
  - 连接线几何名报 `InvalidField`，请用 `insertConnector`
- **`x` / `y`**
  - 禁止
  - 出现报 `InvalidField`

写入 `xdr:sp`：空 `xdr:cNvSpPr`（不写 `txBox`），始终带空或有字的 `xdr:txBody`，默认实心填充 `FFFFFF` 与线宽 `9525` 的黑色描边，与已过 `insertTextBox` 同一套线填。
给了文字格式字段则按 `setShapeFormat` 写入该 `txBody`。省略则缺省 `sz="1100"`。
不写 `xdr:style`，不写 `a16:creationId`。`cNvPr/@name` 写成「形状 {cnv}」。新 `cNvPr/@id` 取该表已有全部 `cNvPr` 的最大整数加 1。
缺 `cx` / `cy` 时用 `1828800` × `914400`。docx 上发此 op 报 `InvalidField`。

### insertConnector

往工作表插入一条未绑连接点的 `xdr:cxnSp`。不写 `a:stCxn` / `a:endCxn`。

- **`op`**
  - 必须是 `"insertConnector"`
- **`id`**
  - 必须是 `sheet:{name}`
- **`from`**
  - 必须
  - 左上角单元格，A1
- **`to` / `anchor` / 偏移 / `cx` / `cy`**
  - 可选
  - 规则与 `insertTextBox` 相同
- **`prst`**
  - 可选
  - 缺省 `straightConnector1`
  - 允许 `line`、`straightConnector1`、`bentConnector2`–`5`、`curvedConnector2`–`5`
- **`x` / `y`**
  - 禁止
  - 出现报 `InvalidField`

写入 `xdr:cxnSp`：`xdr:nvCxnSpPr` / 空 `xdr:cNvCxnSpPr`，无 `txBody`，无填充，线宽 `9525` 的黑色描边。不写 `xdr:style`。
`cNvPr/@name` 写成「连接线 {cnv}」。新 `cNvPr/@id` 规则与 `insertShape` 相同。docx 见 [docx 分册](json-contract-docx.md#insertconnector)。

### deleteShape

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteShape"` |
| `id` | 形状 id `sheet:{name}/shape:…` |

找不到该形状报 `IdNotFound`。id 不是 xlsx 形状 id 报 `InvalidId`。删掉对应 `xdr:sp`；组合里只摘该子节点，空组合再摘组合；锚点只剩几何再摘整锚点。不改其它对象的 `cNvPr/@id`。
docx 上发此 op 报 `InvalidField`。pptx 规则见 [pptx 分册](json-contract-pptx.md#deleteshape)。

### deleteConnector

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteConnector"` |
| `id` | 连接线 id `sheet:{name}/cxn:…` |

找不到该连接线报 `IdNotFound`。id 不是 xlsx 连接线 id 报 `InvalidId`。删掉对应 `xdr:cxnSp`，空组合与空锚点规则与 `deleteShape` 相同。
docx 见 [docx 分册](json-contract-docx.md#deleteconnector)。pptx 规则见 [pptx 分册](json-contract-pptx.md#deleteconnector)。

### groupShapes

把同一张 `worksheet` 上至少两个当前为锚点直接子节点的对象包进一个新 `xdr:grpSp`。组合本身不进 inspect 树，符合条件的子文本框 / 形状 / 连接线仍展开；几何仍取外层锚点。

- **`op`**
  - 必须是 `"groupShapes"`
- **`id`**
  - 必须是 `sheet:{name}`
- **`ids`**
  - 必填
  - 至少两个成员 id
  - 每个必须是该表的文本框 id、形状 id 或连接线 id，且当前是某个 `twoCellAnchor` / `oneCellAnchor` / `absoluteAnchor` 的直接子节点

成员少于 2 个、有重复、不在同一张表、已经在某个 `xdr:grpSp` 里、或不是 `xdr:sp` / `xdr:cxnSp`，报 `InvalidField`。找不到某个成员报 `IdNotFound`。
`id` 不是工作表 id 报 `InvalidId`。对图 id、图表 id、SmartArt id、单元格 id 或 `xlsx:workbook` 发此 op 报 `InvalidId`。
对 `chartsheet` 发此 op 报 `InvalidField`。对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。
新组合 `cNvPr/@id` 取该表已有全部 `cNvPr` 的最大整数加 1；`name` 写成「组合 {该 id}」。不写 `xdr:nvPr`，不写 `a16:creationId`，不写 `xdr:style`。
外层写成一个 `xdr:twoCellAnchor`（不写 `editAs`），`from` / `to` 覆盖成员包围盒。
组合 `xfrm` 的 `off` / `ext` / `chOff` / `chExt` 取该包围盒；
把各成员 `a:off` / `a:ext` 写成工作表空间的绝对 EMU（与 Excel 16.0 `Range.Group()` 另存一致）。
包围盒按该表 `cols` / `row/@ht` / `sheetFormatPr` 换算，缺省列宽 8.43 字符（64 像素）、行高 15 磅；不保证与 Excel 像素级一致。不重算已有组合框。
不把图、图表、SmartArt、OLE 当成员。docx 见 [docx 分册](json-contract-docx.md#groupshapes)。
pptx 见 [pptx 分册](json-contract-pptx.md#groupshapes)。

### ungroup

解开某个成员所在的 `xdr:grpSp`，子节点各自回到独立锚点。组合本身没有 inspect id。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"ungroup"` |
| `id` | 该组合内一个文本框 id、形状 id 或连接线 id |

找不到该对象报 `IdNotFound`。id 不是文本框 / 形状 / 连接线 id 报 `InvalidId`。该对象不在 `xdr:grpSp` 里报 `InvalidField`。
只解开包含该成员的那一层；嵌套组合仍是 `grpSp`。子节点 `cNvPr/@id` 不变。各子对象写成自己的 `xdr:twoCellAnchor`，`from` / `to` 由其 `a:xfrm` 按同一套列宽行高换回单元格。
docx 见 [docx 分册](json-contract-docx.md#ungroup)。pptx 见 [pptx 分册](json-contract-pptx.md#ungroup)。

### insertOle

把本地文件作为嵌入对象写入目标工作表。不在库里启动 OLE 服务器，不生成真实预览图（写入一张最小 PNG 占位）。不进页眉、`chartsheet`、组合。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "ops": [
    {
      "op": "insertOle",
      "id": "sheet:Sheet1",
      "file": "C:\\\\data\\\\book.xlsx",
      "from": "B2"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertOle"` |
| `id` | 必须是 `sheet:{name}` |
| `file` | 必须。本地文件路径 |
| `from` | 必须。左上角单元格，A1 |
| `to` | `anchor` 为 `twoCell` 时必填。右下角单元格，A1 |
| `anchor` | 可选。`oneCell`（缺省）或 `twoCell`。不认 `absolute` |
| `cx` / `cy` | 可选。EMU 正整数，须成对。缺省 `1828800` × `914400` |
| `fromOffX` / `fromOffY` | 可选。相对 `from` 的 EMU 偏移，缺省 `0`，不得为负 |
| `toOffX` / `toOffY` | `twoCell` 时可选。相对 `to` 的 EMU 偏移，缺省 `0`，不得为负 |

`id` 不是工作表 id 报 `InvalidId`。找不到该表报 `IdNotFound`。对 `chartsheet` 报 `InvalidField`。
对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。缺 `from` 报 `MissingField`。`from` / `to` 不是合法 A1 报 `InvalidRange`。
`anchor` 为 `twoCell` 却没有 `to` 报 `InvalidField`。`anchor` 为 `absolute` 或不认识报 `InvalidField`。出现 `x` / `y` 报未知字段。
找不到 `file` 报 `Io`。空路径报 `InvalidField`。

`objectPr/anchor` 始终写无前缀 `from` / `to`（子元素仍是 `xdr:col` 等，与 Excel 16.0 另存一致）。
请求是 `oneCell` 时，按该表列宽行高把 `from` 与 `cx` / `cy` 换成 `to`（只用于写路径，不保证与 Excel 像素级一致）。
新 `shapeId` 取 `max(1024, 已有 oleObject/@shapeId, 已有 VML _x0000_s####)` 加 1。
dummy `xdr:sp` 的 `cNvPr/@id` 与 `shapeId` 相同，`name` 写成「Object {该表已有 OLE 数 + 1}」，并写 `hidden="1"`。不写 `a16:creationId`。

按扩展名决定写入方式：Office 文件（`.xlsx` / `.xlsm` / `.xls` / `.docx` / `.docm` / `.pptx` / `.pptm`）把源文件原字节写入 `/xl/embeddings/…`，
关系 Type 为 `package`，`progId` 用 `FileSpec::ole_obj_name()`（嵌入工作簿为 `Worksheet`，不是 pptx 的 `Excel.Sheet.12`）。
其它扩展名 `progId` 为 `Package`，打成 Ole10Native 写入 `/xl/embeddings/oleObjectN.bin`，关系 Type 为 `oleObject`，并写 `dvAspect="DVASPECT_ICON"`。
预览 PNG 写 `/xl/media/imageN.png`。工作表 `.rels` 同时有 embeddings 与预览 image。
整个 `xdr:twoCellAnchor`（`editAs="oneCell"`）包在 `mc:Choice Requires="a14"` 里，作为 `xdr:wsDr` 的子元素，Fallback 为空；不要把 `AlternateContent` 放进锚点里当图形。
VML 写 `v:shape` Pict 占位（`ObjectType="Pict"`，`type="#_x0000_t75"`，`x:CF` / `x:AutoPict`），预览关系在 VML `.rels`。
不要写批注用的 `_x0000_t202`。这是伴随 markup，不是改表单控件 VML。docx 仍用段落 id，见 [docx 分册](json-contract-docx.md#insertole)。
pptx 见 [pptx 分册](json-contract-pptx.md#insertole)。

### deleteOle

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteOle"` |
| `id` | 必须是 OLE id |

找不到报 `IdNotFound`。id 不是 OLE id 报 `InvalidId`。
摘该条 `mc:AlternateContent`（或裸的 `oleObject`）、对应 dummy `xdr:sp` 所在锚点、以及 VML 里 `_x0000_s{shapeId}` 的 `v:shape`。
`oleObjects` 删光则去掉该元素。清掉无人引用的关系与 embeddings / 预览媒体。docx / pptx 见各自分册。

### insertModel3d

把本地 GLB 作为 3D 模型写入目标工作表。不进 `pictures`。不栅格化模型。海报用 1×1 PNG 占位。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "ops": [
    {
      "op": "insertModel3d",
      "id": "sheet:Sheet1",
      "file": "C:\\\\data\\\\model.glb"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertModel3d"` |
| `id` | 必须是 `sheet:{name}` |
| `file` | 必须。本地 GLB 路径 |
| `from` | 可选。左上角单元格，A1。省略当作 `A1` |
| `to` | `anchor` 为 `twoCell` 时必填。右下角单元格，A1 |
| `anchor` | 可选。`oneCell`（缺省）或 `twoCell`。不认 `absolute` |
| `cx` / `cy` | 可选。EMU 正整数，须成对。缺省 `914400` × `914400` |
| `fromOffX` / `fromOffY` | 可选。相对 `from` 的 EMU 偏移，缺省 `0`，不得为负 |
| `toOffX` / `toOffY` | `twoCell` 时可选。相对 `to` 的 EMU 偏移，缺省 `0`，不得为负 |

`id` 不是工作表 id 报 `InvalidId`。找不到该表报 `IdNotFound`。对 `chartsheet` 报 `InvalidField`。
对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。`from` / `to` 不是合法 A1 报 `InvalidRange`。
`anchor` 为 `twoCell` 却没有 `to` 报 `InvalidField`。`oneCell` 出现 `to` 报未知字段。
`anchor` 为 `absolute` 或不认识报 `InvalidField`。出现 `x` / `y` 报未知字段。
`cx` / `cy` 只给一个报 `InvalidField`。找不到 `file` 报 `Io`。空路径或不是 GLB 报 `InvalidField`。

关系写在 **drawing** `.rels`：一条 `model3d` 指向 `/xl/media/model3dN.glb`，一条 image 指向海报 PNG。
`[Content_Types].xml` 补 Default `glb` 为 `model/gltf.binary`。Choice 里是 `xdr:graphicFrame`，`xdr:xfrm` 的 ext 为 0×0。
整个 `mc:AlternateContent` 放进锚点，Fallback 是海报 `xdr:pic`。不写 `a16:creationId`。
相机、灯光、`am3d:trans` 抄 Office 16.0 默认值，不进 inspect。docx / pptx 见各自分册。

### replaceModel3d

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"replaceModel3d"` |
| `id` | 必须是 3D 模型 id |
| `file` | 必须。本地 GLB 路径 |

换 GLB 字节，保留 graphicFrame 与关系 id。海报仍写成 1×1 PNG。不改位置尺寸。找不到报 `IdNotFound`。
id 不是 3D 模型 id 报 `InvalidId`。不是 GLB 报 `InvalidField`。docx / pptx 见各自分册。

### deleteModel3d

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteModel3d"` |
| `id` | 必须是 3D 模型 id |

找不到报 `IdNotFound`。id 不是 3D 模型 id 报 `InvalidId`。
摘该条 `mc:AlternateContent`；若锚点只剩 `from` / `to` / `ext` / `pos` / `clientData` 则删整个锚点。
GLB 与海报无人引用则删。docx / pptx 见各自分册。

### insertMath

把一条线性公式写入目标工作表 drawing。不进单元格 `f`。不进 `textBoxes`。不实现排版引擎。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "ops": [
    {
      "op": "insertMath",
      "id": "sheet:Sheet1",
      "text": "E=mc^2"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertMath"` |
| `id` | 必须是 `sheet:{name}` |
| `text` | 必须。非空。写入一条 `m:r` / `m:t` |
| `from` | 可选。左上角单元格，A1。省略当作 `A1` |
| `to` | `anchor` 为 `twoCell` 时必填。右下角单元格，A1 |
| `anchor` | 可选。`oneCell`（缺省）或 `twoCell`。不认 `absolute` |
| `cx` / `cy` | 可选。EMU 正整数，须成对。缺省 `914400` × `914400` |
| `fromOffX` / `fromOffY` | 可选。相对 `from` 的 EMU 偏移，缺省 `0`，不得为负 |
| `toOffX` / `toOffY` | `twoCell` 时可选。相对 `to` 的 EMU 偏移，缺省 `0`，不得为负 |

`id` 不是工作表 id 报 `InvalidId`。找不到该表报 `IdNotFound`。对 `chartsheet` 报 `InvalidField`。
对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。`from` / `to` 不是合法 A1 报 `InvalidRange`。
`anchor` 为 `twoCell` 却没有 `to` 报 `InvalidField`。`oneCell` 出现 `to` 报未知字段。
`anchor` 为 `absolute` 或不认识报 `InvalidField`。出现 `x` / `y` 报未知字段。
`cx` / `cy` 只给一个报 `InvalidField`。空 `text` 报 `InvalidField`。出现 `expr` / `kind` 报 `InvalidField`。
出现 `tex` / `unicodemath` / `omml` 报未知字段。

写成 `mc:AlternateContent`，Choice `Requires="a14"`，Choice 里是 `xdr:sp`（`txBox="1"`），
`xdr:txBody/a:p` 里是 `a14:m` 包 `m:oMathPara` / `m:oMath`。不要写 `mathplaceholder` 域。
Fallback 是同一套纯文本框，字为该 `text`。不栅格化，不写 `a:blipFill`。
页眉本期不插。docx / pptx 见各自分册。

### setMath

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setMath"` |
| `id` | 必须是公式框 id |
| `text` | 必须。非空。用一条 `m:r` / `m:t` 替换该 `oMath` 子节点 |

不改锚点与尺寸。出现 `expr` / `kind` 报 `InvalidField`。空 `text` 报 `InvalidField`。
找不到报 `IdNotFound`。id 不是公式框 id 报 `InvalidId`。docx / pptx 见各自分册。

### deleteMath

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteMath"` |
| `id` | 必须是公式框 id |

找不到报 `IdNotFound`。id 不是公式框 id 报 `InvalidId`。
摘该条 `mc:AlternateContent`；若锚点只剩 `from` / `to` / `ext` / `pos` / `clientData` 则删整个锚点。
海报无人引用则删。docx / pptx 见各自分册。

### insertSlicer

把切片器视图写到目标工作表。不在库里执行筛选，不刷新透视合计。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "ops": [
    {
      "op": "insertSlicer",
      "id": "sheet:Sheet1",
      "source": "sheet:Sheet1/table:1",
      "field": "产品",
      "from": "F2"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertSlicer"` |
| `id` | 必须是 `sheet:{name}` |
| `source` | 必须。表格 id 或透视表 id |
| `field` | 必须。表格列名或透视字段名，与 cache `@sourceName` 相同 |
| `from` | 必须。左上角单元格，A1 |
| `to` | `anchor` 为 `twoCell` 时必填 |
| `anchor` | 可选。`oneCell`（缺省）或 `twoCell`。不写 `absoluteAnchor` |
| `cx` / `cy` | 可选。EMU 正整数，须成对。缺省 `1905000` × `2540000` |
| `fromOffX` / `fromOffY` | 可选。缺省 `0`，不得为负 |
| `toOffX` / `toOffY` | `twoCell` 时可选 |

`id` 不是工作表 id 报 `InvalidId`。找不到该表报 `IdNotFound`。对 `chartsheet` 报 `InvalidField`。
对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。`source` 不是该表上的表格 id 或透视表 id 报 `InvalidId`。找不到源报 `IdNotFound`。
`field` 对不上源的列或字段报 `InvalidField`。缺 `from` 报 `MissingField`。

表格源写 `x15:tableSlicerCache` 与工作簿 `x15:slicerCaches`。透视源写 `tabular` 加 `pivotTables`，工作簿走 `x14:slicerCaches`。
`tabular/@pivotCacheId` 必须等于该透视缓存定义 `extLst` 里 `x14:pivotCacheDefinition/@pivotCacheId`（MS-XLSX 2.6.33 / 2.6.83，
URI `{725AE2AE-9491-48BE-B2B4-4EB974FC3084}`）。
已有非 0 值则沿用；没有则写入 workbook `pivotCache/@cacheId`（为 `0` 或空时改写 `1`）。删除切片器时不摘这条扩展。
drawing 写 `xdr:twoCellAnchor`，`mc:Choice` 只包 `sle:slicer` graphicFrame（不要把整段锚点套进 AlternateContent）。
按 MS-XLSX 2.3.2.3 写一条定义名，`@name` 与 cache `@name` 相同，公式为 `#N/A`。不写 `hidden`。不写 `a16:creationId`。不写时间线。不写 OLAP。

### deleteSlicer

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteSlicer"` |
| `id` | 必须是切片器 id |

找不到报 `IdNotFound`。id 不是切片器 id 报 `InvalidId`。摘 drawing 锚点、slicers 部件里对应 `slicer`。
该 cache 已无视图则摘 slicerCache、workbook 关系，以及 `@name` 与 cache 名相同的那条定义名。slicers 部件删光则去掉该部件与工作表 `slicerList` 条目。

### insertConnection

把网页查询或文本连接写入工作簿 Connections 部件。不向数据源发请求，不把查询结果写进单元格，不写 queryTable。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "ops": [
    {
      "op": "insertConnection",
      "id": "xlsx:workbook",
      "name": "WebDemo",
      "type": "web",
      "url": "http://example.com",
      "description": "demo"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertConnection"` |
| `id` | 必须是 `xlsx:workbook` |
| `name` | 必须。非空。未删除连接里大小写不敏感唯一 |
| `type` | 必须。只认 `"web"` 或 `"text"` |
| `url` | `type` 为 `web` 时必须。非空。写入 `webPr/@url` |
| `sourceFile` | `type` 为 `text` 时必须。非空。写入 `textPr/@sourceFile` |
| `description` | 可选。非空则写入 `@description`；省略或空串不写该属性 |

`id` 不是 `xlsx:workbook` 报 `InvalidId`。缺 `name` / `type` 报 `MissingField`。
`type` 不是 `web` / `text`、名为空、未删除连接里重名、`web` 却给了 `sourceFile`、`text` 却给了 `url`，报 `InvalidField`。
`web` 缺 `url` 或 `url` 为空报 `MissingField` / `InvalidField`。`text` 缺 `sourceFile` 或为空同理。

部件路径固定 `/xl/connections.xml`（已有 Connections 部件则沿用其路径）。`@id` 取已有最大整数加 1，至少为 1。
网页连接跟 Excel 16.0 网页另存：`type="4"`、`refreshedVersion="0"`、`background="1"`，`webPr` 写 `htmlTables="1"`、`htmlFormat="all"`。
文本连接跟 Excel 16.0 文本另存：`type="6"`、`refreshedVersion="8"`、`background="1"`、`saveData="1"`，
`textPr` 写 `prompt="0"`、`codePage="65001"`、`comma="1"`，以及一个空 `textField`。
不写 `xr16:uid`。不写 `refreshOnLoad`。不写 queryTable。不写 ODBC / OLE DB / OLAP / Power Query。
docx / pptx 上发此 op 报 `InvalidField`。create 本期不写连接。不标重算。

### deleteConnection

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteConnection"` |
| `id` | 必须是连接 id |

找不到报 `IdNotFound`。id 不是连接 id 报 `InvalidId`。摘该条 `connection`。删光则去掉 Connections 部件、workbook 关系与 Override。
不改 queryTable / `sheetData`。docx / pptx 上发此 op 报 `InvalidField`。

### insertExternalLink

把对其他工作簿的外部引用写入 `/xl/externalLinks/…`。不打开目标文件，不把缓存值写进 `sheetDataSet`，也不改单元格公式。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "output": "C:\\data\\book.extlink.xlsx",
  "ops": [
    {
      "op": "insertExternalLink",
      "id": "xlsx:workbook",
      "path": "C:\\data\\source.xlsx",
      "sheets": [
        "Sheet1"
      ]
    },
    {
      "op": "setCell",
      "id": "cell:Sheet1!B1",
      "formula": "=[1]Sheet1!A1"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertExternalLink"` |
| `id` | 必须是 `xlsx:workbook` |
| `path` | 必须。非空。写入该外部链部件 `.rels` 的 `Target` |
| `sheets` | 可选。目标工作簿表名字符串数组。省略当作 `["Sheet1"]` |

`id` 不是 `xlsx:workbook` 报 `InvalidId`。缺 `path` 报 `MissingField`。`path` 为空串报 `InvalidField`。
`sheets` 为空数组、某项不是非空字符串，报 `InvalidField`。
`path` 已是 `file:` / `http:` / `https:` 则原样当 Target；绝对盘符（`X:\`）或 UNC（`\\`）写成 `file:///` 加原文；其余当相对路径原样写入。
`TargetMode` 为 `External`。追加到 `<externalReferences>` 末尾。部件路径 `/xl/externalLinks/externalLinkN.xml`，N 取未占用的最小正整数。
写出 `externalBook`、`sheetNames` 与每个表一条空的 `<sheetData sheetId>`（从 0 计）。不写 `xxl21` / `x14` / `mc:Ignorable`。不写缓存 `<v>`。
docx / pptx 上发此 op 报 `InvalidField`。create 本期不写外部引用。不标重算。

跨工作簿公式请先 insert，再 `setCell` 写 `=[n]Sheet1!A1`。`[n]` 是 1 基下标，与 `extlink:{n}` 相同。库不把文件路径改写成 `[n]`，也不把 `[n]` 展开成路径。

示例见 `docs/examples/xlsx-apply-external-link.json`。

### deleteExternalLink

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteExternalLink"` |
| `id` | 必须是 `extlink:{n}` |

找不到报 `IdNotFound`。id 不是外部引用 id 报 `InvalidId`。若任一工作表 `<f>` 或定义名文本含 `[k]`（k 为该条下标或更大）报 `NotImplemented`，不写文件。
否则摘该条 `<externalReference>`、对应 `externalLink` 部件及其 `.rels`、workbook 关系与 Override。删光则去掉 `<externalReferences>`。
不重写其余公式的 `[n]`。docx / pptx 上发此 op 报 `InvalidField`。

### insertSparkline

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertSparkline"` |
| `id` | 工作表 id `sheet:{name}` |
| `ref` | 必填。宿主格，恰好一格 |
| `formula` | 必填。数据范围字符串（`xm:f`） |
| `type` | 可选。`line` / `column` / `stacked`。省略当作 `line` |

在该表 `extLst` 里追加一组 `x14:sparklineGroup`，组内一条 `x14:sparkline`。
没有对应 `ext` 则按 `CT_Worksheet` 在末尾写 `extLst` / `ext`（`uri` 写成 Excel 16.0 另存的 `{05C60535-1F16-4fd2-B633-F4F36F0B64E0}`），
`xmlns:x14` 写在该 `ext` 上，`xmlns:xm` 写在 `sparklineGroups` 上。
`ref` 必须是单格；闭区间报 `InvalidField`。非法 A1 报 `InvalidRange`。与同一张表上已有迷你图宿主格相交报 `InvalidField`。`formula` 去掉首尾空白后不能为空。
能解析成无表名前缀的 A1 时写成 `当前表!A1:B1`（不加 `$`）；已含 `!` 或解析不成 A1 的原文写入。能解析成 A1 却既跨行又跨列报 `InvalidField`。
不认识的 `type` 报 `InvalidField`。插入时按 Excel 16.0 另存写 `displayEmptyCellsAs="gap"` 以及一组默认 `color*`（`rgb`），不把这些放进请求。
不写 `xr2:uid`。不写单元格值。id 不是工作表 id 报 `InvalidId`。找不到工作表报 `IdNotFound`。
只能写 `worksheet`；对 `chartsheet` 报 `InvalidField`；对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。不标重算。不新建部件。
其它 `ext` 原样留下。docx / pptx 上发此 op 报 `InvalidField`。create 本期不写迷你图。

### setSparkline

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setSparkline"` |
| `id` | 迷你图组 id `sheet:{name}/spark:{n}` |
| `type` | 可选。`line` / `column` / `stacked` |
| `ref` | 可选。改该组**仅有的那一条**的宿主格 |
| `formula` | 可选。改该组**仅有的那一条**的数据范围 |

`type`、`ref`、`formula` 至少给一个，否则报 `InvalidField`。该组若有多于一条 `sparkline`，给 `ref` 或 `formula` 报 `InvalidField`（仍可只改 `type`）。
`ref` 约束与 insert 相同，含重叠检查（排除自身）。`formula` 空串报 `InvalidField`，写入规则与 insert 相同。不改颜色与其它未点名属性。找不到报 `IdNotFound`。
id 不是迷你图组 id 报 `InvalidId`。只能改 `worksheet`。docx / pptx 上发此 op 报 `InvalidField`。

### deleteSparkline

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteSparkline"` |
| `id` | 迷你图组 id |

去掉该 `x14:sparklineGroup`。组删光则去掉 `sparklineGroups` 与该 `ext`；`extLst` 空则去掉 `extLst`。不改单元格值，也不动其它 `ext`。找不到报 `IdNotFound`。
id 不是迷你图组 id 报 `InvalidId`。只能改 `worksheet`。docx / pptx 上发此 op 报 `InvalidField`。

### insertChartSheet

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "ops": [
    {
      "op": "insertChartSheet",
      "id": "xlsx:workbook",
      "name": "Chart1",
      "dataFrom": "A10",
      "type": "col",
      "series": [
        {
          "name": "S1",
          "categories": [
            "a",
            "b"
          ],
          "values": [
            1,
            2
          ]
        }
      ]
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertChartSheet"` |
| `id` | 必须是 `xlsx:workbook` |
| `name` | 新表名。规则与 create 的 `sheets[].name` 相同 |
| `dataFrom` | 写到普通工作表上的数据表左上角，A1。第 1 行系列名，第 1 列分类，数值向右 |
| `dataSheet` | 可选。承接 `dataFrom` 的 `worksheet` 名。省略则取工作簿里第一张 `worksheet` |
| `type` / `ofPieType` / `title` / `series` | 与 `insertChart` 相同 |
| `cx` / `cy` | 可选。`absoluteAnchor` 的 EMU 尺寸。都省略则用与 `insertChart` 相同的缺省。只给一个报 `InvalidField` |

xlsx：`id` 不是 `xlsx:workbook` 报 `InvalidId`。`name` 非法报 `InvalidField`。与已有表重名（不分大小写）报 `InvalidField`。
找不到 `dataSheet` 报 `IdNotFound`。`dataSheet` 不是 `worksheet`，或省略时工作簿里没有 `worksheet`，报 `InvalidField`。
出现 `from` / `to` / `anchor` / 偏移 / `kind` / `wrap` 报 `InvalidField`。类型与系列限制与 `insertChart` 相同。
按 `setCell` 语义把数据表写到点名的普通表上，不删原有其它格子。新建 `/xl/chartsheets/sheetN.xml`（CT
`application/vnd.openxmlformats-officedocument.spreadsheetml.chartsheet+xml`）、一份 drawing、`/xl/charts/chartN.xml`，
并在 `workbook.xml` 的 `sheets` 末尾加一项。
关系 Type 以 `/chartsheet` 结尾。`chartsheet` 必须带 `<sheetViews>` 与 `<drawing r:id>`（CT_Chartsheet，§18.3.1.12）。
那一张框写 `xdr:absoluteAnchor`，从 (0,0) 起，带空的 `xdr:clientData`。`xdr:cNvPr/@id` 至少为 2。不写 embedding。不在同一张图表工作表上再插第二张。
create 用 `sheets[].kind: "chartsheet"` 声明图表工作表，字段与本 op 相同但不含 `id`；见上文 create。docx / pptx 上发此 op 报 `InvalidField`。

### deleteChartSheet

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteChartSheet"` |
| `id` | `sheet:{name}`，且该表 `kind` 为 `chartsheet` |

找不到该表报 `IdNotFound`。id 不是工作表 id 报 `InvalidId`。对 `worksheet` / `dialogsheet` / `macrosheet` 发此 op 报 `InvalidId`。
删掉工作簿里最后一张表报 `InvalidField`。删掉最后一张可见表报 `InvalidField`。
去掉 workbook 表项、chartsheet 部件及其 drawing / chart（无人引用则删），以及对应关系与内容类型。不删 `dataFrom` 那些单元格。docx / pptx 上发此 op 报 `InvalidField`。

### insertSheet

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "ops": [
    {
      "op": "insertSheet",
      "id": "xlsx:workbook",
      "name": "Sheet3"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertSheet"` |
| `id` | 必须是 `xlsx:workbook` |
| `name` | 新表名。规则与 create 的 `sheets[].name` 相同 |
| `after` | 可选。已有表的 `sheet:{name}`。省略则追加到 `sheets` 末尾；给出则插到该表之后 |

xlsx：`id` 不是 `xlsx:workbook` 报 `InvalidId`。`name` 非法报 `InvalidSheetName`。与已有表重名（不分大小写）报 `DuplicateSheetName`。
`after` 找不到报 `IdNotFound`；不是工作表 id 报 `InvalidId`。不要用本 op 新建图表工作表。
新建最小 `worksheet` 部件（空 `sheetData`，CT 与 create 相同），workbook 关系 Type 以 `/worksheet` 结尾，`sheetId` 取已有最大整数加 1。docx / pptx 上发此 op 报 `InvalidField`。

### deleteSheet

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteSheet"` |
| `id` | `sheet:{name}`，且该表 `kind` 为 `worksheet` |

找不到该表报 `IdNotFound`。id 不是工作表 id 报 `InvalidId`。对 `chartsheet` 发此 op 报 `InvalidId`（请用 `deleteChartSheet`）。
对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。删掉工作簿里最后一张表，或删掉最后一张可见表，报 `InvalidField`。
`definedNames` 或其它表的 `<f>`、图表 `c:f` 仍含该表名时报 `NotImplemented`，不写文件。
去掉 workbook 表项、worksheet 部件及其 drawing / drawingHF（无人引用的媒体一并删），以及对应关系与内容类型。docx / pptx 上发此 op 报 `InvalidField`。

### renameSheet

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"renameSheet"` |
| `id` | `sheet:{name}` |
| `name` | 新表名。规则与 create 的 `sheets[].name` 相同 |

找不到该表报 `IdNotFound`。id 不是工作表 id 报 `InvalidId`。对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。
`name` 非法报 `InvalidSheetName`。与另一张表重名（不分大小写）报 `DuplicateSheetName`。已经是目标名（大小写相同）则空操作，仍计入 `opsApplied`。
只改大小写时仍改 `sheet/@name`。`definedNames` 或其它表 `<f>` / 图表 `c:f` 仍含旧表名时报 `NotImplemented`，不写文件。不改写公式。改名后旧 `sheet:{旧名}` 失效。
docx / pptx 上发此 op 报 `InvalidField`。

### moveSheet

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"moveSheet"` |
| `id` | 要移动的 `sheet:{name}` |
| `after` | 可选。目标前一张表的 `sheet:{name}`。省略或 `null` 表示移到名单最前 |

找不到该表或 `after` 报 `IdNotFound`。id / `after` 不是工作表 id 报 `InvalidId`。`after` 与 `id` 是同一张表报 `InvalidField`。
对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。已经在目标位置则空操作。
只改 `workbook.xml` 里 `sheets` 子元素顺序，不改部件路径、`sheetId` 或关系 id。docx / pptx 上发此 op 报 `InvalidField`。

### setSheetHidden

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setSheetHidden"` |
| `id` | `sheet:{name}` |
| `hidden` | `true` 隐藏，`false` 显示 |

找不到该表报 `IdNotFound`。id 不是工作表 id 报 `InvalidId`。对 `dialogsheet` / `macrosheet` 报 `NotImplemented`。
把最后一张可见表改成隐藏报 `InvalidField`。`true` 写 `state="hidden"`（若原先是 `veryHidden` 也改成 `hidden`）。`false` 去掉 `state`。已经是目标可见性则空操作。
docx / pptx 上发此 op 报 `InvalidField`。

### setChartAnchor

字段与 `setPictureAnchor` 相同，`id` 必须是 xlsx 图表 id。
改包含该 `xdr:graphicFrame` 的锚点，规则与 `setPictureAnchor` 相同（留下 `xdr:graphicFrame` 与 `xdr:clientData`）。docx / pptx 上发此 op 报 `InvalidField`。

### setSmartArtAnchor

字段与 `setChartAnchor` 相同，`id` 必须是 xlsx 图示 id。改包含该 `xdr:graphicFrame` 的外层锚点，规则与 `setChartAnchor` 相同。docx / pptx 上发此 op 报 `InvalidField`。

