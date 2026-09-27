# 领域 JSON 契约

`schemaVersion` 固定为 `1`。库和命令行只认这一层。外部 MCP 也必须整份使用这一层，不许改字段名和含义，不许拆扁成工具参数。机器可读副本见 [json-schema/request.schema.json](json-schema/request.schema.json)，与本文冲突时以本文为准。

xlsx、docx、pptx 的 inspect、create、apply 已经按本文实现。docx 另有 `exportMedia` 与 `exportChartData`。docx 的 `exportMedia` 认图 id 与 OLE id：图 id 写出 `/word/media/…`，OLE id 写出 embeddings 部件。xlsx 的 `exportMedia` 认工作表图 id 与页眉页脚图 id。pptx 的 `exportMedia` 认幻灯片图 id 与备注图 id，写出 `/ppt/media/…`。xlsx / pptx 的 `exportMedia` 不认 OLE id。xlsx 的 `scope.charts` 与图表 op 认工作表 / 图表工作表上的图；`scope.headerFooter` 展开工作表上的页眉页脚槽位。pptx 的 `scope.charts` 与图表 op 认幻灯片上的 `p:graphicFrame`；`exportChartData` 认幻灯片图表 id，写出 embedding。pptx 的 `scope.smartArt` 与 SmartArt op 认幻灯片上的 diagram `p:graphicFrame`。pptx 的 `scope.comments` 与批注 op 认该页 2018 线程化批注（`p188:cmLst`），不认 ECMA-376 §19.4 旧式部件。xlsx 的 `exportChartData` 仍拒绝。xlsx 的 `scope.smartArt` 与 SmartArt op 仍拒绝。xlsx 的 `scope.comments` 与批注 op 仍拒绝。人工打开只认 Microsoft Office，见三份 checklist。

## 信封

每份请求都是一个 JSON 对象，必须含：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `schemaVersion` | 整数 | 必须是 `1` |
| `action` | 字符串 | `inspect`、`create`、`apply`、`exportMedia`、`exportChartData` 之一 |
| `locale` | 字符串 | 可选。错误 `message` 的语言偏好，见 [errors.md](errors.md) |

`locale` 不属于动作参数，五种动作都认。取值宽松匹配：`zh`、`zh-CN`、`zh-Hans`、`zh-SG` 出中文；`en` 及其地区变体出英文；不认识的标签（包括空串）退回英文，不报 `InvalidField`。这是信封里唯一一处对未知取值不报错的字段：语言只影响呈现，不该挡下文档操作。优先级是请求 `locale` 高于命令行 `--lang`，再高于环境变量 `OOXP_LANG`，都没有时是英文。外部 MCP 没有后两档，只能写请求里的 `locale`。

另外按动作需要 `path`、`format`、`scope`、`workbook`、`document`、`presentation`、`ops`、`id`、`output` 等字段，见下文。禁止在请求里放一整棵 inspect 树来表示「把文档改成这样」。create 的 xlsx 用 `workbook`，docx 用 `document`，pptx 用 `presentation`，三者不能同时出现。

`format` 取值：`xlsx`、`docx`、`pptx`。可省略，省略时从 `path` 的扩展名推断。推断不出就报 `UnsupportedFormat`。

`path` 是本地文件系统路径。Windows 反斜杠和正斜杠都接受。

## 稳定 id

id 是给模型用的字符串，必须能从文件内容稳定算出来，不能依赖本次运行的内存地址或随机数。

解析规则按格式走。id 里的工作表名、段落 id、形状 id 都按文件里的原文，不做 Unicode 正规化。

### xlsx

| 节点 | id | 例子 |
| --- | --- | --- |
| 工作簿 | `xlsx:workbook` | `xlsx:workbook` |
| 自定义属性 | `prop:{name}` | `prop:Editor` |
| 工作表 | `sheet:{name}` | `sheet:Sheet1`、`sheet:销售 2026` |
| 单元格 | `cell:{name}!{A1}` | `cell:Sheet1!A1`、`cell:销售 2026!BC12` |
| 工作表图 | `sheet:{name}/pic:{cNvPrId}` 或 `sheet:{name}/pic:index:{k}` | `sheet:Sheet1/pic:2` |
| 页眉页脚槽 | `sheet:{name}/hf:{kind}:{side}` | `sheet:Sheet1/hf:odd:header` |
| 页眉页脚图 | `{槽 id}/pic:{cNvPrId}` 或 `{槽 id}/pic:index:{k}` | `sheet:Sheet1/hf:odd:header/pic:2` |
| 工作表图表 | `sheet:{name}/chart:{cNvPrId}` 或 `sheet:{name}/chart:index:{k}` | `sheet:Sheet1/chart:2` |
| 图表系列 | `{图表 id}/series:{idx}` | `sheet:Sheet1/chart:2/series:0` |

`{name}` 是 `workbook.xml` 里 `<sheet name="...">` 的值，不是 `sheetId`，也不是 `r:id`。查找工作表时大小写不敏感，输出 id 时用文件里的原始大小写。

`{A1}` 是大写列字母加行号，例如 `A1`、`AA100`。请求里若写 `a1`，库先规范成 `A1` 再生成 id。

解析单元格 id：从右侧第一个 `!` 切开。右侧必须是合法 A1，左侧是工作表名。这样工作表名里即使含 `!`，也不会切错。

图表工作表、宏表出现在 inspect 的表名单里，但没有单元格 id。它们的 id 仍是 `sheet:{name}`。对它们发 `setCell` 报 `InvalidField`。对它们发 `insertPicture` / `insertChart` 报 `InvalidField`。图表工作表上的那张 `xdr:graphicFrame` 走下面的图表 id，不另编图 id。新建、删除图表工作表用 `insertChartSheet` / `deleteChartSheet`，不要复用 `insertChart` 往 `chartsheet` 上插第二张。

工作表图挂在该 `worksheet` 上，不挂在单元格上，也不挂在工作簿根上。图的 `{cNvPrId}` 取该 `xdr:pic` 里 `xdr:cNvPr/@id` 的原文（ECMA-376 §20.5.2.8）。输出用文件里的原始大小写与数字写法。请求里若双方都能解析成十进制整数，按数值匹配（`1` 与 `01` 视为同一处）；否则按原文精确比。同一张表内重复 `cNvPr/@id` 时，inspect 都列出，apply 命中该表文档顺序第一处。没有 `cNvPr/@id` 时用 `pic:index:{k}`，树上 `"stable": false`。`{k}` 只在该表全部 drawing 部件内从 1 计，计入该表所有被收入的图，不只是缺 id 的那些。

只认锚点（`xdr:twoCellAnchor` / `xdr:oneCellAnchor` / `xdr:absoluteAnchor`）的直接子元素 `xdr:pic` 且带 `a:blip`。组合 `xdr:grpSp` 里的图、`xdr:graphicFrame`（图表、SmartArt）、`xdr:sp`、连接线不进 `pictures`。VML / `legacyDrawing`（批注框、旧式控件）不编。工作表图只跟工作表元素 `<drawing r:id>` 指向的 drawing 部件，不跟 `<drawingHF>`。`mc:AlternateContent` 只进第一个 `Choice`。一张表可以有多个 worksheet drawing 关系，按工作表 XML 里 `<drawing>` 的文档顺序拼接后再编号。

xlsx 页眉页脚是工作表上的 `headerFooter`（ECMA-376 §18.3.1.46），不是 Word 那种独立 header 部件，不要套 `header:default:1/para:…`。`{kind}` 为 `odd` / `even` / `first`，`{side}` 为 `header` / `footer`，一律小写。六个槽位始终按这个顺序：奇数页页眉、奇数页页脚、偶数页页眉、偶数页页脚、首页页眉、首页页脚。对应 XML 本地名 `oddHeader` / `oddFooter` / `evenHeader` / `evenFooter` / `firstHeader` / `firstFooter`。文本就是该元素原文，含 `&P` / `&D` / `&A` / `&C` 这类代码，不编成独立域节点。inspect 报原文；apply 整段替换原文。

页眉页脚图只认 `<drawingHF r:id>` 指向的 drawing 里、锚点的直接子元素 `xdr:pic` 且带 `a:blip`。判定与工作表图相同。VML / `legacyDrawingHF` 不进树、不改。Excel 16.0 另存的页眉图通常是 VML，因此默认样例常常没有 DrawingML 页眉图；树里出现页眉图，只因为包里已有 `drawingHF`，或 `insertPicture` 往槽位新建了 DrawingML。工作表图名单与页眉图名单不混：`sheet:{name}/pic:…` 只来自 `<drawing>`，`sheet:{name}/hf:…/pic:…` 只来自 `<drawingHF>`。`drawingHF` 上的 `cho` / `lho` / `rho`（奇数页页眉中/左/右）、`che` / `lhe` / `rhe`（偶数页页眉）、`chf` / `lhf` / `rhf`（首页页眉）、`cfo` / `lfo` / `rfo`（奇数页页脚）、`cfe` / `lfe` / `rfe`（偶数页页脚）、`cff` / `lff` / `rff`（首页页脚）把 `cNvPr/@id` 分到对应槽；对不上的图归 `odd:header`。没有 `cNvPr/@id` 时用 `pic:index:{k}`，`{k}` 只在该槽位从 1 计，`stable` 为 `false`。`chartsheet` / `dialogsheet` 本期只读页眉文本，不列出、不接受页眉图。页眉页脚里的图表、形状、OLE 本期不进树。

工作表图表挂在该 `worksheet` 或 `chartsheet` 上，不挂在单元格上，也不挂在工作簿根上。图表的 `{cNvPrId}` 取该 `xdr:graphicFrame` 里 `xdr:nvGraphicFramePr/xdr:cNvPr/@id` 的原文（§20.5.2.8），匹配规则与工作表图相同。没有该属性时用 `chart:index:{k}`，树上 `"stable": false`。`{k}` 只在该表全部 drawing 部件内从 1 计，只计入被收入的图表。系列 `{idx}` 从 0 起，按 `/xl/charts/…` 里 `c:ser` 的文档顺序，不跟 `c:idx/@val`。

只认锚点的直接子元素 `xdr:graphicFrame`，且 `a:graphicData/@uri` 为 `http://schemas.openxmlformats.org/drawingml/2006/chart`（可带尾斜杠，ECMA-376 §21.2）。Office 2016 以后的 chartex URI（`http://schemas.microsoft.com/office/drawing/2014/chartex`）也列入，`type` 为 `chartex`。组合 `xdr:grpSp` 里的框、`xdr:pic`、形状、SmartArt、OLE、VML 不进 `charts`。页眉页脚里的图表本期不进树。`mc:AlternateContent` 只进第一个 `Choice`。同一张表上同时有 `xdr:pic` 和图表的，各走各的名单。

`insertPicture` 的 `id` 必须是 `sheet:{name}`（往工作表 drawing 插图）或页眉槽位 id `sheet:{name}/hf:{kind}:{side}`（往该侧 `drawingHF` 插图），且该表 `kind` 为 `worksheet`。对图 id、图表 id、系列 id、单元格 id 或 `xlsx:workbook` 发 `insertPicture` 报 `InvalidId`。对 `chartsheet` / `dialogsheet` 发 `insertPicture`（含往其页眉槽位插图）报 `InvalidField`。对工作表 id 发 `insertPicture` 仍只插工作表图，不进页眉。往页眉槽位插图时禁止出现 `from` / `to` / `anchor` / `fromOffX` / `fromOffY` / `toOffX` / `toOffY`，出现报 `InvalidField`；只要 `file`，可选 `cx` / `cy`。写入 `oneCell`、从 A1、偏移 0，并把 `drawingHF` 上该侧中区属性写成新图的 `cNvPr` id（奇数页页眉 `cho`，偶数页页眉 `che`，首页页眉 `chf`，奇数页页脚 `cfo`，偶数页页脚 `cfe`，首页页脚 `cff`）。该中区已被占用则改写左区（`lh*`），再右区（`rh*`）；三区都占用仍插入 drawing，不再写属性，inspect 把它归到 `odd:header`。表上只有 `legacyDrawingHF`、没有 `drawingHF` 时，新建 DrawingML 的 `drawingHF`，不动 VML。`replacePicture` / `deletePicture` / `setPictureSize` / `setPictureAnchor` 的 `id` 必须是工作表图 id 或页眉图 id。对工作表 id、页眉槽位 id、单元格 id 或图表 id 发这四个 op 报 `InvalidId`。对 `chartsheet` / `dialogsheet` 上的页眉图发这四个 op 报 `InvalidField`。`setPictureWrap` 是 Word 绕排，xlsx 上报 `InvalidField`。`exportMedia` 认工作表图 id 与页眉图 id，写出 `/xl/media/…`。

`insertChart` 的 `id` 必须是 `sheet:{name}`，且该表 `kind` 为 `worksheet`。对图 id、图表 id、系列 id、单元格 id 或 `xlsx:workbook` 发 `insertChart` 报 `InvalidId`。对 `chartsheet` / `dialogsheet` 发 `insertChart` 报 `InvalidField`。`insertChartSheet` 的 `id` 必须是 `xlsx:workbook`。`deleteChartSheet` 的 `id` 必须是 `sheet:{name}`，且该表 `kind` 为 `chartsheet`。`deleteChart` / `setChartTitle` / `setChartType` / `setChartAxisTitle` / `setChartLegend` / `setChartDataLabels` / `setChartSize` / `setChartAnchor` / `addChartSeries` 的 `id` 必须是图表 id。`setChartSeries` / `deleteChartSeries` 的 `id` 必须是系列 id。对不上的 id 形态报 `InvalidId`。`setChartWrap` 是 Word 绕排，xlsx 上报 `InvalidField`。xlsx 改锚点请用 `setChartAnchor`。

### docx

文档根节点 id 固定为 `docx:document`。

段落 id 优先用 `w14:paraId`（十六进制，MS-DOCX §2.6.2.3，命名空间 `http://schemas.microsoft.com/office/word/2010/wordml`）。有该属性时：`para:{paraId}`，例如 `para:3F2A1B00`。输出用文件里的原始大小写。请求里的十六进制按大小写不敏感匹配。

没有 `w14:paraId` 时：`para:index:{n}`，`n` 从 1 起，按主文档部件里 `w:p` 的文档顺序（含表格单元格里的段落）。树上同时给 `"stable": false`。这类 id 在增删段落后会变，只够同一次 inspect 之后立刻 apply。

标题和正文都是段落，不另编一套 id。脚注、尾注、批注都是独立故事：根上始终有计数；给了对应 `scope` 才列出名单，并可 apply。分隔符三条不进用户名单，禁止 apply。文本框是独立节点：框本身、框内段落与框内图按下面规则进树，并可 apply。页眉、页脚按下面规则进树，可按 id 改字；节上的页眉页脚距边界、页面边距、纸张、分栏、垂直对齐、行号、文档网格、装订线侧，以及是否链接到前一节，都可以读写。整篇的镜像边距与装订线在上，写在 `settings.xml`。页眉、页脚、正文、注和批注里的 DrawingML 图片可查、换、插、删，并可改尺寸与绕排。正文与页眉页脚里的嵌入对象（OLE / 包）可查、插、删，并可按 id 导出嵌入部件；框内、注内、批注内的 OLE 本期不编。正文与页眉页脚里的 `w:tbl` 可查、插、删，并可插删行与列、合并与拆分单元格。正文与页眉页脚里的 run 级插入、删除与段落/run 属性修订可按范围列出，并可接受或拒绝被点名的一条。apply 可用 `insertHeaderFooter` 为某节某类型新建页眉或页脚部件，可用 `setTitlePage` 写本节 `w:titlePg`，可用 `setEvenAndOddHeaders` 写整篇 `w:evenAndOddHeaders`，可用 `insertSection` 在某段之后插入分节符，可用 `deleteSection` 删掉非末节并把内容并进下一节，可用 `insertToc` 插入目录域、用 `deleteToc` 删掉目录块，可用 `insertMath` / `setMath` / `deleteMath` 插改删 OMML 公式，可用 `insertContentControl` / `setContentControl` / `deleteContentControl` 插改删内容控件。docx 可用 `setProtection` 开关无密码的限制编辑与只读推荐。三种格式都可用 `setProps` / `setCustomProp` / `deleteCustomProp` 改文件属性。`create` 可在正文和页眉页脚上写 DrawingML 图，也可写文本框、SmartArt、图表、脚注、尾注与批注；页眉页脚允许 `default` / `even` / `first`；不写修订。字段见下文 create。嵌入图表不是 `pic:pic`：根上始终有 `chartCount`；给了 `scope.charts` 才列出名单，并可 apply。SmartArt 不是 `pic:pic`：根上始终有 `smartArtCount`；给了 `scope.smartArt` 才列出名单，并可 apply。正文与页眉页脚、文本框、脚注、尾注、批注里的图表与 SmartArt 走同一套 op，id 带所在段落前缀。注内与批注内的文本框同样进树，并可 apply。框内再套框仍拒绝。

页眉、页脚 id 按部件编，不按节编。`w:sectPr` 里的 `w:headerReference` / `w:footerReference`（§17.10.2、§17.10.5）经主文档关系解到部件路径。同一部件只出一个容器节点；多个节引用同一部件时，`sections` 列出所有节号。后一节某类型引用省略时按规范继承前节（§17.10.2）；`even` 是否继承还看 `settings.xml` 的 `w:evenAndOddHeaders`（§17.10.1），`first` 是否继承还看该节的 `w:titlePg`（§17.10.6）。OOXML 没有「链接到前一节」元素；Word UI 的勾选就是这种省略。inspect 在每个节上用 `headerLink` / `footerLink` 标出该类型是显式引用、继承前节，还是对本节不生效。apply 用 `setHeaderFooterLink` 断开或重新链接；用 `insertHeaderFooter` 在空白槽位新建部件。两节显式指向同一部件不算链接，两边都是 `false`。

| 节点 | id | 例子 |
| --- | --- | --- |
| 节 | `section:{n}` | `section:1` |
| 页眉容器 | `header:{type}:{n}` | `header:default:1`、`header:first:1` |
| 页脚容器 | `footer:{type}:{n}` | `footer:default:1`、`footer:even:1` |
| 页眉/页脚段落 | `{容器 id}/para:{paraId}` 或 `{容器 id}/para:index:{k}` | `header:default:1/para:3F2A1B00` |
| 页眉/页脚图 | `{容器 id}/pic:{docPrId}` 或 `{容器 id}/pic:index:{k}` | `header:default:1/pic:1` |
| 正文图 | `{段落 id}/pic:{docPrId}` 或 `{段落 id}/pic:index:{k}` | `para:3F2A1B00/pic:1` |
| 正文文本框 | `{段落 id}/txbx:{docPrId}` 或 `{段落 id}/txbx:index:{k}` | `para:3F2A1B00/txbx:8` |
| 页眉/页脚文本框 | `{页眉或页脚段落 id}/txbx:{docPrId}` | `header:default:1/para:AA/txbx:8` |
| 框内段落 | `{框 id}/para:{paraId}` 或 `{框 id}/para:index:{k}` | `para:3F2A1B00/txbx:8/para:00FF00AA` |
| 框内图 | `{框内段落 id}/pic:{docPrId}` | `para:3F2A1B00/txbx:8/para:00FF00AA/pic:1` |
| 框内图表 | `{框内段落 id}/chart:{docPrId}` | `para:3F2A1B00/txbx:8/para:00FF00AA/chart:1` |
| 框内 SmartArt | `{框内段落 id}/smartdiagram:{docPrId}` | `para:3F2A1B00/txbx:8/para:00FF00AA/smartdiagram:1` |
| 脚注 | `footnote:{id}` | `footnote:2` |
| 尾注 | `endnote:{id}` | `endnote:2` |
| 注内段落 | `{注 id}/para:{paraId}` 或 `{注 id}/para:index:{k}` | `footnote:2/para:3F2A1B00` |
| 注内图 | `{注内段落 id}/pic:{docPrId}` | `footnote:2/para:3F2A1B00/pic:1` |
| 注内文本框 | `{注内段落 id}/txbx:{docPrId}` | `footnote:2/para:3F2A1B00/txbx:8` |
| 注内图表 | `{注内段落 id}/chart:{docPrId}` | `footnote:2/para:3F2A1B00/chart:1` |
| 注内 SmartArt | `{注内段落 id}/smartdiagram:{docPrId}` | `footnote:2/para:3F2A1B00/smartdiagram:1` |
| 分隔符（只读） | `footnote:sep:{type}` / `endnote:sep:{type}` | `footnote:sep:separator` |
| 批注 | `comment:{id}` | `comment:0` |
| 批注内段落 | `{批注 id}/para:{paraId}` 或 `{批注 id}/para:index:{k}` | `comment:0/para:3F2A1B00` |
| 批注内图 | `{批注内段落 id}/pic:{docPrId}` | `comment:0/para:3F2A1B00/pic:1` |
| 批注内文本框 | `{批注内段落 id}/txbx:{docPrId}` | `comment:0/para:3F2A1B00/txbx:8` |
| 批注内图表 | `{批注内段落 id}/chart:{docPrId}` | `comment:0/para:3F2A1B00/chart:1` |
| 批注内 SmartArt | `{批注内段落 id}/smartdiagram:{docPrId}` | `comment:0/para:3F2A1B00/smartdiagram:1` |
| 正文 OLE | `{段落 id}/ole:{token}` 或 `{段落 id}/ole:index:{k}` | `para:3F2A1B00/ole:_1795770305` |
| 页眉/页脚 OLE | `{页眉或页脚段落 id}/ole:{token}` | `header:default:1/para:AA/ole:_1795770305` |
| 正文图表 | `{段落 id}/chart:{docPrId}` 或 `{段落 id}/chart:index:{k}` | `para:3F2A1B00/chart:7` |
| 页眉/页脚图表 | `{页眉或页脚段落 id}/chart:{docPrId}` | `header:default:1/para:AA/chart:7` |
| 图表系列 | `{图表 id}/series:{idx}` | `para:3F2A1B00/chart:7/series:0` |
| 正文 SmartArt | `{段落 id}/smartdiagram:{docPrId}` 或 `{段落 id}/smartdiagram:index:{k}` | `para:3F2A1B00/smartdiagram:4` |
| 页眉/页脚 SmartArt | `{页眉或页脚段落 id}/smartdiagram:{docPrId}` | `header:default:1/para:AA/smartdiagram:4` |
| SmartArt 点 | `{图示 id}/pt:{modelId}` | `para:3F2A1B00/smartdiagram:4/pt:2` |
| 正文表 | `table:{n}` | `table:1` |
| 正文表行 | `table:{n}/row:{r}` | `table:1/row:2` |
| 正文单元格 | `table:{n}/r{r}c{c}` | `table:1/r2c3` |
| 页眉/页脚表 | `{容器 id}/table:{n}` | `header:default:1/table:1` |
| 页眉/页脚表行 | `{容器 id}/table:{n}/row:{r}` | `header:default:1/table:1/row:1` |
| 页眉/页脚单元格 | `{容器 id}/table:{n}/r{r}c{c}` | `header:default:1/table:1/r1c1` |
| 正文修订 | `rev:{n}` | `rev:1` |
| 页眉/页脚修订 | `{容器 id}/rev:{n}` | `header:default:1/rev:1` |
| 正文超链接 | `link:{n}` | `link:1` |
| 页眉/页脚超链接 | `{容器 id}/link:{n}` | `header:default:1/link:1` |
| 正文书签 | `bookmark:{n}` | `bookmark:1` |
| 页眉/页脚书签 | `{容器 id}/bookmark:{n}` | `header:default:1/bookmark:1` |
| 正文域 | `field:{n}` | `field:1` |
| 页眉/页脚域 | `{容器 id}/field:{n}` | `header:default:1/field:1` |
| 正文目录 | `toc:{n}` | `toc:1` |
| 正文公式 | `math:{n}` | `math:1` |
| 页眉/页脚公式 | `{容器 id}/math:{n}` | `header:default:1/math:1` |
| 正文内容控件 | `sdt:{n}` | `sdt:1` |
| 页眉/页脚内容控件 | `{容器 id}/sdt:{n}` | `header:default:1/sdt:1` |
| 编号实例 | `num:{numId}` | `num:1` |
| 抽象编号 | `abstractNum:{abstractNumId}` | `abstractNum:0` |
| 样式 | `style:{styleId}` | `style:Normal` |
| 段落内 run | `{段落 id}/run:{n}` | `para:3F2A1B00/run:1`、`header:default:1/para:AA/run:2` |
| 自定义属性 | `prop:{name}` | `prop:Editor` |

节号 `{n}` 从 1 起，按主文档里 `w:sectPr` 的文档顺序（与页眉目录相同）。`insertSection` / `deleteSection` 之后旧的 `section:{n}` 会变，只够同一次 inspect 之后立刻 apply。距边界挂在节上，不挂在页眉部件节点上。页眉页脚图挂在容器上，不挂在节上。正文图挂在它所在的主文档段落上，不挂在文档根上。对 `section:{n}` 发 `setParaText` 或图片 op 报 `InvalidId`。`insertSection` 的 `id` 必须是主文档 `w:body` 的直接子段落 id，或 `docx:document`。`deleteSection` 的 `id` 必须是 `section:{n}`。对页眉页脚、框内、注内、批注内、表格单元格内的段落 id，或容器 id、表 id、run id 发 `insertSection` 报 `InvalidId`。

`{type}` 取引用上 `w:type` 的原文，只允许 `default`、`even`、`first`（`ST_HdrFtr`，§17.18.36）。缺失或不是这三项报 `XmlBroken`。`{n}` 在同一 `kind`（header 或 footer）与 `{type}` 下，按该部件首次出现的顺序从 1 起。页眉页脚段落没有 `w14:paraId` 时用 `para:index:{k}`，`k` 只在该部件内从 1 计。

正文段落 id 仍是 `para:{paraId}` / `para:index:{n}`，只表示主文档部件。对容器 id（没有 `/para:`）发 `setParaText` 报 `InvalidId`。拿正文 `para:` 去改页眉页脚、注或批注里同 hex 的段落报 `IdNotFound`。

脚注、尾注的 `{id}` 取 `w:footnote/@w:id` / `w:endnote/@w:id` 的原文（ECMA-376 §17.11.2）。输出用文件里的原始数字写法。请求里若双方都能解析成十进制整数，按数值匹配（`2` 与 `02` 视为同一条）；否则按原文精确比。同一文件里脚注 id 与尾注 id 可以都是 `2`，靠前缀区分。只把 `w:type` 为 `normal` 或未写 type 的当作用户注（§17.18.33 `ST_FtnEdn`）。`separator`、`continuationSeparator`、`continuationNotice` 是分隔符，id 写成 `footnote:sep:{type}` / `endnote:sep:{type}`，只出现在 `noteSeparators`，对它们发任何 apply 报 `InvalidId`。同一侧重复 `w:id` 时 inspect 都列出，apply 命中该部件文档顺序第一处。

批注的 `{id}` 取 `w:comment/@w:id` 的原文（ECMA-376 §17.13.4.2）。匹配规则与脚注 `w:id` 相同。同一文件里批注 id 与脚注 id 可以都是 `0`，靠前缀区分。重复 `w:id` 时 inspect 都列出，apply 命中 `comments.xml` 文档顺序第一处。范围标记 `w:commentRangeStart` / `w:commentRangeEnd` / `w:commentReference` 的 `w:id` 与 `w:comment/@w:id` 按数值对应（§17.13.4.4、§17.13.4.3、§17.13.4.5）。线程关系靠 `commentsExtended` 的 `paraIdParent` 挂到父批注（MS-DOCX §2.5.3.1）。Word 16 给回复也写一套范围标记，套在父批注范围上：`commentRangeStart` 紧跟父 start 之后，`commentRangeEnd` 与 `commentReference` 紧跟父 `commentReference` 所在 run 之后。inspect 读到回复自己的标记就用；没有标记时沿用线程根。`commentEx/@paraId` 是该批注**最后一段**的 `w14:paraId`，不是第一段，也不是宿主段落。

图表的 `{docPrId}` 取该 `w:drawing` 里 `wp:docPr/@id` 的原文，匹配规则与图相同。没有该属性时用 `chart:index:{k}`，`stable` 为 `false`。正文的 `{k}` 只在该段内从 1 计；页眉页脚的 `{k}` 只在该部件内从 1 计。系列 `{idx}` 从 0 起，按 chart 部件里 `c:ser` 的文档顺序，不跟 `c:idx/@val`。只认 `a:graphicData/@uri` 为 `http://schemas.openxmlformats.org/drawingml/2006/chart`（可带尾斜杠）的 drawing（ECMA-376 §21.2）。Office 2016 以后的 chartex URI（`http://schemas.microsoft.com/office/drawing/2014/chartex`）也列入，`type` 为 `chartex`。VML 图表不编。`w:drawing` 里同时有 `pic:pic` 和图表的，各走各的名单。正文图表只收主文档段落的子孙，遇 `w:txbxContent` 停住；表格单元格里的段落与正文段落同一套规则。页眉页脚图表挂在它所在的页眉或页脚段落上，名单出现在该容器的 `charts` 上。框内图表挂在框节点上，id 带框内段落前缀。注内、批注内图表挂在该注或批注节点上，id 带注内或批注内段落前缀。框内图表不算进注或批注自己的 `chartCount`。根上 `chartCount` 仍只计正文，不含页眉页脚，也不含框内、注内、批注内。

`insertChart` 的 `id` 必须是段落 id：正文、页眉页脚、框内、注内或批注内段落。对图 id、图表 id、系列 id、SmartArt id、点 id、容器 id、框 id、注容器 id、批注容器 id 或 `section:{n}` 发 `insertChart` 报 `InvalidId`。`deleteChart` / `setChartTitle` / `setChartType` / `setChartAxisTitle` / `setChartLegend` / `setChartDataLabels` / `setChartSize` / `setChartWrap` / `addChartSeries` 的 `id` 必须是图表 id。`setChartSeries` / `deleteChartSeries` 的 `id` 必须是系列 id。对不上的 id 形态报 `InvalidId`。

SmartArt 的 `{docPrId}` 取该 `w:drawing` 里 `wp:docPr/@id` 的原文，匹配规则与图相同。没有该属性时用 `smartdiagram:index:{k}`，`stable` 为 `false`。正文的 `{k}` 只在该段内从 1 计；页眉页脚的 `{k}` 只在该部件内从 1 计。点的 `{modelId}` 取 data 部件里 `dgm:pt/@modelId` 的原文，输出用文件里的原始大小写。请求按精确字符串匹配（GUID 的花括号和连字符都按原文）。同一 data 部件里重复 `modelId` 时 inspect 都列出，apply 命中 `ptLst` 文档顺序第一处。只认 `a:graphicData/@uri` 为 `http://schemas.openxmlformats.org/drawingml/2006/diagram`（可带尾斜杠）的 drawing（ECMA-376 §21.4）。`dgm:relIds` 的 `r:dm` / `r:lo` / `r:qs` / `r:cs` 指向**宿主部件**关系里的 data / layout / quickStyle / colors（§21.4.2.22）。VML 图示不编。`w:drawing` 里同时有 `pic:pic`、图表和 SmartArt 的，各走各的名单。正文 SmartArt 只收主文档段落的子孙，遇 `w:txbxContent` 停住；表格单元格里的段落与正文段落同一套规则。页眉页脚 SmartArt 挂在它所在的页眉或页脚段落上，名单出现在该容器的 `smartArts` 上。框内 SmartArt 挂在框节点上，id 带框内段落前缀。注内、批注内 SmartArt 挂在该注或批注节点上，id 带注内或批注内段落前缀。框内图示不算进注或批注自己的 `smartArtCount`。根上 `smartArtCount` 仍只计正文，不含页眉页脚，也不含框内、注内、批注内。

可见点：`dgm:pt` 的 `type` 缺省（规范默认 `node`，§21.4.7.51）或为 `node` / `asst` 的点进 `points`。`doc`、`pres`、`parTrans`、`sibTrans` 不进树。没有 `dgm:t` 时 `text` 为 `""`。层次看 `cxnLst` 里 `type` 缺省或为 `parOf` 的连接：`srcId` 是父、`destId` 是子。父点是 `doc` 或不在可见名单里时 `parent` 为 `null`；否则 `parent` 是父点的点 id。

`insertSmartArt` 的 `id` 必须是段落 id：正文、页眉页脚、框内、注内或批注内段落。对图 id、图表 id、SmartArt id、点 id、容器 id、框 id、注容器 id、批注容器 id、表 id、行 id、单元格 id 或 `section:{n}` 发 `insertSmartArt` 报 `InvalidId`。`deleteSmartArt` / `setSmartArtSize` / `setSmartArtWrap` / `setSmartArtColors` / `setSmartArtStyle` / `setSmartArtLayout` 的 `id` 必须是图示 id。`setSmartArtText` / `deleteSmartArtPoint` 的 `id` 必须是点 id。`addSmartArtPoint` 的 `id` 是图示 id（挂到文档根点下）或点 id（挂到该点下）。对不上的 id 形态报 `InvalidId`。

OLE 的 `{token}` 优先取 `o:OLEObject/@ObjectID` 原文（常带前导 `_`），否则 `w:objectEmbed/@shapeId`，否则该 `w:object` 里 `wp:docPr/@id`。输出用文件里的原始大小写。请求里若双方都能解析成十进制整数，按数值匹配；否则按原文精确比。同一归属范围内重复 token 时 inspect 都列出，apply 命中该范围内文档顺序第一处。正文的范围是该段；页眉页脚的范围是该部件。都没有上述属性时用 `ole:index:{k}`，树上 `"stable": false`。正文的 `{k}` 只在该段内从 1 计；页眉页脚的 `{k}` 只在该部件内从 1 计，计入该范围内所有被收入的 OLE，不只是缺 token 的那些。只认 `w:object`（ECMA-376 §17.3.3.19）且带嵌入关系：子孙里有 `o:OLEObject`（Office 命名空间 `urn:schemas-microsoft-com:office:office`）且 `Type` 不是 `Link`，或有 `w:objectEmbed`。关系 Type 后缀为 `oleObject` 或 `package`（Transitional `http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject` 与 `…/package`）。Word 16.0 把嵌入的 xlsx 等 Office 文件写成 `package`；ECMA 与部分文件写成 `oleObject`。两者都进树。`objectLink`、`Type="Link"`、`w:control`、`w:movie` 不进树。预览图（VML `v:imagedata` 或 `w:object` 里的 `w:drawing`）不进 `pictures`，也不进 `charts` / `smartArts`。正文 OLE 只收主文档段落的子孙，遇 `w:txbxContent` 停住；表格单元格里的段落与正文段落同一套规则。页眉页脚 OLE 挂在它所在的页眉或页脚段落上，名单出现在该容器的 `oleObjects` 上。框内、注内、批注内的 `w:object` 本期不编；对这些容器发 OLE op 报 `InvalidId`。根上 `oleObjectCount` 仍只计正文，不含页眉页脚。xlsx / pptx 本期不编 OLE。

`insertOle` 的 `id` 必须是段落 id：正文或页眉页脚。对 OLE id、图 id、容器 id、框 id、注容器 id、批注容器 id、框内 / 注内 / 批注内段落 id 或 `section:{n}` 发 `insertOle` 报 `InvalidId`。`deleteOle` 的 `id` 必须是 OLE id。对段落 id 发 `deleteOle` 报 `InvalidId`。

表的 `{n}` 从 1 起，按该故事里 `w:tbl` 的文档顺序（遇 `w:txbxContent` 停住；`mc:AlternateContent` 只进 Choice）。嵌套在单元格里的表也编号，不另起前缀。OOXML 没有表级稳定 id，因此 `table:{n}` 在增删表之后会变，只够同一次 inspect 之后立刻 apply。行号 `{r}` 是该表内 `w:tr` 的文档顺序，从 1 起。列号 `{c}` 是该 `w:tc` 在 `w:tblGrid` 上的起始列，从 1 起；`w:gridSpan` 缺省为 1（§17.4.17）。水平合并只出一个单元格节点，`gridSpan` 大于 1。垂直合并的继续格仍有 `w:tc`，树上 `vMerge` 为 `continue`。没有 `tblGrid` 时，`colCount` 取第一行各格 `gridSpan` 之和。框内、注内、批注内的 `w:tbl` 本期不编、不改。

`insertTable` 的 `id` 可以是：正文或页眉页脚段落 id（插到该 `w:p` 之后，与该段同父）、`docx:document` 或页眉页脚容器 id（插到该故事末尾、`sectPr` 之前）、已有表 id（插到该 `w:tbl` 之后）、或单元格 id（作为嵌套表插到该格末尾，格末仍保留至少一个 `w:p`）。`deleteTable` 的 `id` 必须是表 id。`insertRow` 的 `id` 是表 id（追加一行）或行 id（插到该行之后）。`deleteRow` 的 `id` 必须是行 id。`insertCol` / `deleteCol` / `mergeCells` 的 `id` 必须是表 id。`splitCells` 的 `id` 必须是单元格 id。对不上的 id 形态报 `InvalidId`。

修订的 `{n}` 从 1 起，按该故事里被收入节点的文档顺序（遇 `w:txbxContent` 停住；`mc:AlternateContent` 只进 Choice）。OOXML 的 `w:id` 在拆 run 时会重复，因此不用它当树上的唯一键；树上另给 `xmlId` 存原文。`rev:{n}` 在增删修订之后会变，只够同一次 inspect 之后立刻 apply。同一 `w:id` 的多个节点是多条修订，`acceptRevision` / `rejectRevision` 只动被点名的那一个。本期只收：不在 `pPr` / `rPr` / `trPr` / `tblPr` / `tcPr` / `sectPr` / `numPr` 里、且直接子元素不是 `w:p` / `w:tbl` 的 `w:ins` / `w:del`（run 级插入与删除，ECMA-376 §17.13.5.18、§17.13.5.14）；以及 `w:rPrChange`（§17.13.5.31）与 `w:pPrChange`（§17.13.5.27）。属性容器上的 `ins`/`del`（段落标记、表行）、块级包段落的 `ins`/`del`、`w:moveFrom` / `w:moveTo`、表上 `tblPrChange` / `trPrChange` / `tcPrChange` / `tblGridChange`、`sectPrChange` 本期不进树。框内、注内、批注内的修订本期不编；对这些容器发 `acceptRevision` / `rejectRevision` 报 `InvalidId`。`w:id` 缺失时仍列出，`xmlId` 为 `null`。

`acceptRevision` / `rejectRevision` 的 `id` 必须是修订 id。对段落 id、表 id、容器 id 或 `section:{n}` 发这两个 op 报 `InvalidId`。

超链接的 `{n}` 从 1 起，按该故事里 `w:hyperlink` 的文档顺序（遇 `w:txbxContent` 停住；`mc:AlternateContent` 只进 Choice）。关系 `r:id` 不当树上的唯一键，只进 `rId`。`link:{n}` 在增删链之后会变，只够同一次 inspect 之后立刻 apply。本期只收 `w:hyperlink` 元素（ECMA-376 §17.16.22）。HYPERLINK 域、图上 `a:hlinkClick`、框内 / 注内 / 批注内的链不进树；对这些容器发超链接 op 报 `InvalidId`。有 `r:id` 时为外部链（规范说此时忽略 `w:anchor`）；只有 `w:anchor` 时为内部链，指向书签名。两者都没有仍列出，`kind` 为 `anchor`，`anchor` 为 `null`。

书签的 `{n}` 从 1 起，按该故事里被收入的 `w:bookmarkStart` 的文档顺序（同一套边界）。只收起止都在同一故事里、且 `w:name` 不以 `_` 开头的配对（Word 隐藏书签含 `_GoBack` 不进树）。配对按 `w:id`（§17.13.6.1、§17.13.6.2）。找不到配对的不进树。`w:id` 进 `xmlId`，缺则为 `null`。`w:name` 进 `name`。框内、注内、批注内的书签本期不编；对这些容器发书签 op 报 `InvalidId`。

域的 `{n}` 从 1 起，按该故事里被收入域起点的文档顺序（遇 `w:txbxContent` 停住；`mc:AlternateContent` 只进 Choice）。起点是 `w:fldSimple`，或配对完整的 `w:fldChar/@w:fldCharType="begin"`（§17.16.18、§17.16.19）。嵌套域各算一条：外层 begin 先于内层 begin，因此外层 `{n}` 更小。故事结束时尚未配对 end 的 begin 不进树。框内、注内、批注内的域本期不编；对这些容器发域 op 报 `InvalidId`。HYPERLINK 域进本名单，不进 `hyperlinks`。主文档里 `code` 为 `TOC` 或 `TOA` 的域另外编目录 id，它们仍出现在 `fields` 里。

目录的 `{n}` 从 1 起，按主文档里被收入的 TOC / TOA 域起点的文档顺序，与域名单同一套边界。`toc:{n}` 在插删目录之后会变，只够同一次 inspect 之后立刻 apply。`kind` 为 `toc`（`TOC` 且指令里没有 `\c` 开关）、`tof`（`TOC` 且有 `\c`，图表目录）或 `toa`（`TOA`）。页眉页脚、框内、注内、批注内的 TOC 本期不编 `toc:`；对这些容器发 `insertToc` / `deleteToc` 报 `InvalidId`。嵌套在 TOC 结果里的 `PAGEREF` / `HYPERLINK` 域不算目录块。

公式的 `{n}` 从 1 起，按该故事里顶层 `m:oMath` 的文档顺序（遇 `w:txbxContent` 停住；`mc:AlternateContent` 只进 Choice）。嵌在另一个 `oMath` 里的不算一条。`oMathPara` 里的每个 `oMath` 各算一条。`math:{n}` 在插删公式之后会变，只够同一次 inspect 之后立刻 apply。父节点是 `oMathPara` 时 `kind` 为 `display`，否则为 `inline`。正文用 `math:{n}`，页眉页脚用 `{容器}/math:{n}`。框内、注内、批注内的公式本期不编；对这些容器发公式 op 报 `InvalidId`。Equation Editor 3.0 的 OLE / `w:object` 不编。只认 OMML，不把公式编成结构化 JSON 树，也不实现排版引擎。

内容控件的 `{n}` 从 1 起，按该故事里被收入的 `w:sdt` 的文档顺序（遇 `w:txbxContent` 停住；`mc:AlternateContent` 只进 Choice，ECMA-376 §17.5.2）。嵌套控件各算一条，外层 `{n}` 更小。`sdt:{n}` 在插删控件之后会变，只够同一次 inspect 之后立刻 apply。正文用 `sdt:{n}`，页眉页脚用 `{容器}/sdt:{n}`。框内、注内、批注内的控件本期不编；对这些容器发内容控件 op 报 `InvalidId`。本期收入：`plainText`（`w:text`）、`richText`（`w:richText`，或 `sdtPr` 未写类型元素，规范缺省为富文本）、`dropDown`（`w:dropDownList`）、`comboBox`（`w:comboBox`）、`date`（`w:date`）、`repeatingSection`（`repeatingSection` / `repeatingSectionItem`）。不收 `docPartObj`（目录等构建基块库仍走 `toc:`）、`bibliography`、`citation`、`picture`、`group`、`equation`、`checkbox`。祖先里有 `w:p` 时 `level` 为 `inline`，否则为 `block`。控件内段落仍走段落 id。`w:sdtPr/w:id/@w:val` 进 `xmlId`，不是树上的唯一键。带 `w:dataBinding` 的仍列出，`bound` 为 `true`。不实现 Word 开发者模式 UI，也不写自定义 XML 部件。

编号定义在 `/word/numbering.xml`（关系 `/numbering`，ECMA-376 §17.9）。实例 id 用 `w:num/@w:numId`：`num:{numId}`。抽象定义 id 用 `w:abstractNum/@w:abstractNumId`：`abstractNum:{abstractNumId}`。`numId` 为 0 表示无编号，不进树。这与 `table:{n}` 不同：编号 XML 本身有稳定数字 id，增删定义后已有实例的 id 不变。不实现编号计算引擎，树上不出现「显示为 3.」这类结果。页眉页脚容器不挂编号计数（整篇共用一份定义）。框内、注内、批注内的段若自带 `w:numPr` 仍能看见 `num` / `ilvl`，但编号 op 只接受正文或页眉页脚段落。

样式定义在 `/word/styles.xml`（关系 `/styles`，ECMA-376 §17.7）。实例 id 用 `w:style/@w:styleId`：`style:{styleId}`。`styleId` 为空的不进树。这与 `table:{n}` 不同：样式 XML 本身有稳定字符串 id。不实现排版引擎，树上不出现算出来的字号或颜色。页眉页脚容器不挂样式计数（整篇共用一份定义）。段落上的 `style` 仍只读自己的 `w:pStyle/@w:val`，不解析 `basedOn`，也不把样式定义摊到段上。框内、注内、批注内的段若自带 `pStyle` 仍能看见 `style`，但样式 op 只接受正文或页眉页脚段落。

直接格式写在该段自己的 `w:pPr` 或该 run 自己的 `w:rPr` 上，不改 `styles.xml`。run 的 `{n}` 从 1 起，按该段内被收入的 `w:r` 文档顺序。遇 `w:txbxContent` 停住；`mc:AlternateContent` 只进 Choice；`w:del` 里的 run 不编。`w:ins`、`w:hyperlink` 里的 run 要编。增删 run 之后 `{n}` 会变，只够同一次 inspect 之后立刻 apply。对 run id 发 `setParaText` / `setParaFormat` 报 `InvalidId`。对段落 id 发 `setRunFormat` 报 `InvalidId`。

`insertHyperlink` 的 `id` 必须是段落 id（正文或页眉页脚）。`setHyperlink` / `deleteHyperlink` 的 `id` 必须是超链接 id。`insertBookmark` 的 `id` 必须是起始段落 id；可选 `end` 为同一故事里的结束段落 id。`setBookmark` / `deleteBookmark` 的 `id` 必须是书签 id。`insertField` 的 `id` 必须是段落 id（正文或页眉页脚）。`setField` / `deleteField` 的 `id` 必须是域 id。`insertNumbering` 的 `id` 必须是 `docx:document`。`setNumbering` 的 `id` 必须是抽象编号 id。`setParaNumbering` 的 `id` 必须是段落 id（正文或页眉页脚）。`setStyle` 的 `id` 必须是样式 id。`setParaStyle` 的 `id` 必须是段落 id（正文或页眉页脚）。`setParaFormat` 的 `id` 必须是段落 id（正文、页眉页脚、框内、注内或批注内）。`setRunFormat` 的 `id` 必须是 run id。`insertSection` 的 `id` 必须是主文档 `w:body` 的直接子段落 id 或 `docx:document`。`deleteSection` 的 `id` 必须是 `section:{n}`。`insertToc` 的 `id` 必须是主文档 `w:body` 的直接子段落 id 或 `docx:document`。`deleteToc` 的 `id` 必须是 `toc:{n}`。`insertMath` 的 `id` 必须是段落 id（正文或页眉页脚）或 `docx:document`。`setMath` / `deleteMath` 的 `id` 必须是公式 id。`insertContentControl` 的 `id` 必须是段落 id（正文或页眉页脚）或 `docx:document`。`setContentControl` / `deleteContentControl` 的 `id` 必须是内容控件 id。对不上的 id 形态报 `InvalidId`。

图的 `{docPrId}` 取该 `w:drawing` 里 `wp:docPr/@id` 的原文（ECMA-376 §20.4）。输出用文件里的原始大小写与数字写法。请求里若双方都能解析成十进制整数，按数值匹配（`1` 与 `01` 视为同一处）；否则按原文精确比。同一归属范围内重复 `docPr/@id` 时，inspect 都列出，apply 命中该范围内文档顺序第一处。页眉页脚的范围是该容器部件；正文的范围是该段。没有 `docPr/@id` 时用 `pic:index:{k}`，树上 `"stable": false`。页眉页脚的 `{k}` 只在该部件内从 1 计；正文的 `{k}` 只在该段内从 1 计。两种 `{k}` 都计入该范围内所有被收入的图，不只是缺 id 的那些。只认 DrawingML 的 `pic:pic` 且带 `a:blip` 的 `w:drawing`。VML / `w:pict`、图表、SmartArt、文本框不进宿主段落的 `pictures`。框内图走框内段落上的 `pictures`，id 带框前缀。`mc:AlternateContent` 仍只进 Choice。正文图只收主文档段落的子孙，遇 `w:txbxContent` 停住，表格单元格里的段落与正文段落同一套规则。

`insertPicture` 的 `id` 必须是段落 id：正文用 `para:…` / `para:index:{n}`，页眉页脚用 `{容器}/para:…`，框内用 `{框 id}/para:…`，注内用 `{注 id}/para:…`，批注内用 `{批注 id}/para:…`。对图 id、容器 id、框 id、注容器 id、批注容器 id 或 `section:{n}` 发 `insertPicture` 报 `InvalidId`。`replacePicture` / `deletePicture` / `setPictureSize` / `setPictureWrap` 的 `id` 必须是图 id。对段落 id、容器 id、框 id、注容器 id、批注容器 id 或 `section:{n}` 发这四个 op 报 `InvalidId`。

文本框只认 DrawingML：`w:drawing` 里有 `w:txbxContent`，或 `wps:wsp` / `wps:txbx`（MS-DOCX WordprocessingShape）。常包在 `mc:AlternateContent` 里，只进 Choice。VML `v:textbox` / `w:pict` 不编、不改。组合 `wpg:wgp` 本身不进树，只展开里面符合条件的框。框的 `{docPrId}` 取该 `w:drawing` 里 `wp:docPr/@id`，规则与图相同。组合里的框取该 `wps:wsp` 的 `cNvPr/@id`；再没有则 `txbx:index:{k}`。`k` 只在该宿主段内从 1 计，`stable` 为 `false`。页眉页脚框挂在它所在的页眉或页脚段落上，不挂在节上。

链接框：`wps:txbx` 是链的第一框（§20.4.2.37），后续框用 `linkedTxbx`（§20.4.2.34），同一 `id` 为同一故事，`seq` 为顺序。树上 `link.prev` / `link.next` 是相邻框的图 id 风格框 id；未链接则为 `null`。不把后继框的字并进当前框。

「宿主段落遇到框就停住」仍成立，否则外层 `paraCount` 会把框内段算进去。边界按下面两条划：

- 遍历宿主段落（正文或页眉页脚）时遇到 `w:txbxContent` 就停住，不再往下走。框内 `w:p` 不计入宿主 `paraCount`，不出现在 `outline` 与宿主 `paragraphs` 里，它们的 `w:t` 也不并进外层段落的 `text`。拿框内 `paraId` 当正文 `para:` 发 apply，报 `IdNotFound`。框内段落必须用带框前缀的 id。
- 遇到 `mc:AlternateContent` 只走第一个 `mc:Choice`；没有 `mc:Choice` 才走 `mc:Fallback`。两支都走会把同一份内容读出两遍，也会让 apply 只改到其中一支、另一支不同步。这条对 `paraCount`、`outline`、`paragraphs`、`text`、`runCount`、`runs`、页眉页脚、`pictureCount`、`pictures`、`textBoxCount`、`textBoxes`、`chartCount`、`charts`、`smartArtCount`、`smartArts`、`oleObjectCount`、`oleObjects` 和 apply 一律适用。

Word 2010 以后插入文本框、图表、SmartArt 都是 `mc:AlternateContent` 套 `w:txbxContent` 的写法，所以这两条不是边角情况。

同一部件里若出现重复的 `w14:paraId`（违反 MS-DOCX，但现实文件可能有），inspect 照样列出每一段；apply 命中该部件文档顺序中的第一段。

### pptx

演示文稿根节点 id 固定为 `pptx:presentation`。

| 节点 | id | 例子 |
| --- | --- | --- |
| 页 | `slide:{n}` | `slide:1` |
| 形状 | `slide:{n}/shape:{shapeId}` | `slide:1/shape:4` |
| 无 `cNvPr/@id` 的形状 | `slide:{n}/shape:index:{k}` | `slide:1/shape:index:2` |
| 图 | `slide:{n}/pic:{cNvPrId}` | `slide:1/pic:4` |
| 无 `cNvPr/@id` 的图 | `slide:{n}/pic:index:{k}` | `slide:1/pic:index:1` |
| 图表 | `slide:{n}/chart:{cNvPrId}` | `slide:1/chart:5` |
| 无 `cNvPr/@id` 的图表 | `slide:{n}/chart:index:{k}` | `slide:1/chart:index:1` |
| 系列 | `{图表 id}/series:{idx}` | `slide:1/chart:5/series:0` |
| SmartArt | `slide:{n}/smartdiagram:{cNvPrId}` | `slide:1/smartdiagram:4` |
| 无 `cNvPr/@id` 的 SmartArt | `slide:{n}/smartdiagram:index:{k}` | `slide:1/smartdiagram:index:1` |
| SmartArt 点 | `{图示 id}/pt:{modelId}` | `slide:1/smartdiagram:4/pt:2` |
| 备注容器 | `slide:{n}/notes` | `slide:1/notes` |
| 备注形状 | `slide:{n}/notes/shape:{shapeId}` | `slide:1/notes/shape:3` |
| 无 `cNvPr/@id` 的备注形状 | `slide:{n}/notes/shape:index:{k}` | `slide:1/notes/shape:index:1` |
| 备注图 | `slide:{n}/notes/pic:{cNvPrId}` | `slide:1/notes/pic:5` |
| 无 `cNvPr/@id` 的备注图 | `slide:{n}/notes/pic:index:{k}` | `slide:1/notes/pic:index:1` |
| 批注 | `slide:{n}/comment:{guid}` | `slide:1/comment:{A1B2C3D4-E5F6-4789-ABCD-EF1234567890}` |
| 批注回复 | `{批注 id}/reply:{guid}` | `slide:1/comment:{A1B2C3D4-E5F6-4789-ABCD-EF1234567890}/reply:{11111111-2222-4333-8444-555555555555}` |
| 自定义属性 | `prop:{name}` | `prop:Editor` |

`{n}` 从 1 起，按 Presentation 部件（§13.3.6）里 `p:sldIdLst` / `p:sldId` 的出现顺序（§19.2.1.34），再跟 `r:id` 找到幻灯片部件。`n` 不是文件名里的数字，也不是 `p:sldId/@id`（那是 `ST_SlideId`，常见从 256 起）。隐藏页仍占序号，仍可 inspect 和 apply。

`{shapeId}` 取该形状 `p:nvSpPr/p:cNvPr/@id` 的原文（§19.3.1.12）。输出用文件里的原始数字字符串。请求按十进制数值匹配，不因前导零而失败。

没有 `cNvPr/@id` 时用 `slide:{n}/shape:index:{k}`，`k` 从 1 起，按该页列入树的形状顺序。树上同时给 `"stable": false`。这类 id 在增删形状后会变，只够同一次 inspect 之后立刻 apply。Office 亲手存的文件几乎都有 `cNvPr/@id`。

图 id 的 `{cNvPrId}` 取该 `p:pic` 的 `p:nvPicPr/p:cNvPr/@id` 原文。没有该属性时用 `slide:{n}/pic:index:{k}`，`stable` 为 `false`。`{k}` 只在该页全部被收入的 `p:pic` 内从 1 计，与形状的 `shape:index` 分家，互不影响。只认带 `a:blip` 的 `p:pic`。`p:sp`、连接线、`p:graphicFrame`（图表 / 表 / SmartArt）不进 `pictures`。组合本身不进树，里面符合条件的 `p:pic` 要展开。`mc:AlternateContent` 只进第一个 `Choice`，规则与形状相同。

图表 id 的 `{cNvPrId}` 取该 `p:graphicFrame` 的 `p:nvGraphicFramePr/p:cNvPr/@id` 原文。没有则用 `slide:{n}/chart:index:{k}`，`stable` 为 `false`。`{k}` 只在该页全部被收入的图表内从 1 计，与 `pic:index` / `shape:index` 分家。只认 `p:graphicFrame` 且 `a:graphicData/@uri` 为 chart URI（可带尾斜杠）。chartex URI 列入，`type` 为 `chartex`。`p:pic`、表格、SmartArt 不进 `charts`。组合里符合条件的框要展开。系列 `{idx}` 从 0 起，按 chart 部件里 `c:ser` 的文档顺序，不跟 `c:idx/@val`。数据在 embedding 工作簿与 cache 里；以 embedding 为准，不一致则该系列 `cacheStale: true`。

SmartArt id 的 `{cNvPrId}` 取该 `p:graphicFrame` 的 `p:nvGraphicFramePr/p:cNvPr/@id` 原文。没有则用 `slide:{n}/smartdiagram:index:{k}`，`stable` 为 `false`。`{k}` 只在该页全部被收入的图示内从 1 计，与 `pic:index` / `chart:index` / `shape:index` 分家。只认 `p:graphicFrame` 且 `a:graphicData/@uri` 为 `http://schemas.openxmlformats.org/drawingml/2006/diagram`（可带尾斜杠，ECMA-376 §21.4）。`dgm:relIds` 的四条关系写在该幻灯片 `.rels` 上。`p:pic`、图表、表格不进 `smartArts`。组合里符合条件的框要展开。点 id 规则与 docx 相同。备注、批注、母版、版式上的图示本期不进树。

同一页里若出现重复的 `cNvPr/@id`，inspect 照样列出每一处；apply 命中该页文档顺序中的第一处。

批注 `{guid}` 取该页现代批注部件里 `p188:cm/@id` 的原文（MS-PPTX 2018/8），含花括号。回复 `{guid}` 取同一条 `p188:cm` 里 `p188:reply/@id` 的原文。按文件原文精确匹配，不因大小写或有无花括号而放宽。同一页多条批注共用一个 comments 部件。ECMA-376 §19.4 旧式 `/ppt/comments/commentN.xml` 与 `commentAuthors` 不进树、不接受 apply；只有旧式时 `commentCount` 为 `0`，`scope.comments` 为空数组。insert 只写现代部件。

apply 不得删形状再重建，以免 `cNvPr/@id` 变化导致绑在形状上的动画失效。不改 `<p:timing>`。

讲义、幻灯片母版、版式第一期不进这棵树，也不接受 apply。备注页始终给出 `hasNotes`；`scope.notes: true` 时展开该页 `notesSlide` 上的文本形状。xlsx 的批注另开，不走 docx 这套 `comment:` id。pptx 批注走 `slide:{n}/comment:{guid}`，不走 `comment:{w:id}`。

「文本形状」要有可执行的判定：

- 只收带 `p:txBody` 的 `p:sp`（§19.3.1.45、§19.3.1.51），含嵌在 `p:grpSp` 里的。标题占位符、正文占位符、自由文本框都算。
- 不收 `p:pic`、`p:cxnSp`、`p:graphicFrame`（表格、图表、SmartArt 等）、`p:contentPart`。组合本身不进树，只展开里面符合条件的 `p:sp`。
- 遇到 `mc:AlternateContent` 只走第一个 `mc:Choice`；没有 `mc:Choice` 才走 `mc:Fallback`。两支都走会把同一份内容读出两遍，也会让 apply 只改到其中一支。这条对 `shapes`、`text` 和 apply 一律适用。

## inspect

成功响应的对象字段按下面各节示例的语义顺序写出，不是按字母排序。命令行打印同一棵树，见 [cli.md](cli.md)。

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

#### xlsx 的 scope

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `path` | 是 | 已有文件 |
| `scope` | 否 | 省略则只返回工作表名单，不含单元格 |
| `scope.sheet` | 要单元格时必填 | 工作表名 |
| `scope.range` | 要单元格时必填 | A1 区域，例如 `A1:D20`。单个单元格写成 `B3` 或 `B3:B3` |
| `scope.maxCells` | 否 | 返回单元格上限，默认 `5000`，允许范围 `1` 到 `20000` |
| `scope.pictures` | 否 | 为 `true` 时给每张 `worksheet` 填工作表 `pictures`。省略或 `false` 则只有 `pictureCount`，没有工作表图名单。不读媒体字节。不依赖 `sheet` / `range`。单独给此项不会展开页眉图 |
| `scope.charts` | 否 | 为 `true` 时给每张 `worksheet` 与 `chartsheet` 填 `charts`。省略或 `false` 则只有 `chartCount`，没有名单。不把 chart 部件或媒体字节放进树。不依赖 `sheet` / `range` / `pictures` |
| `scope.headerFooter` | 否 | 为 `true` 时给每张 `worksheet` / `chartsheet` / `dialogsheet` 填六个页眉页脚槽位（`headerFooters`）。没有的槽位仍列出，`text` 为 `null`。省略或 `false` 则只有 `hasHeaderFooter`，不倒出原文，也没有 `headerFooters`。不依赖 `sheet` / `range` / `pictures` / `charts` |

省略 `scope`、或只给空对象：返回工作簿节点和表名单，每张表不含 `cells`，也没有 `pictures` / `charts` / `headerFooters`。工作簿与每张 `worksheet` 仍有 `pictureCount`。工作簿、每张 `worksheet` 与每张 `chartsheet` 仍有 `chartCount`。每张表仍有 `hasHeaderFooter`。这是默认截断。

只要想看单元格，就必须同时给 `sheet` 和 `range`。不默认扫第一张表的整张纸。只要想看工作表图名单，就给 `pictures: true`。只要想看图表名单，就给 `charts: true`。只要想看页眉原文，就给 `headerFooter: true`。四者独立，可只开一边。页眉图名单出现在已展开的槽位上，须同时 `headerFooter: true` 与 `pictures: true`。

#### docx 的 scope

```json
{
  "schemaVersion": 1,
  "action": "inspect",
  "path": "C:\\data\\note.docx",
  "format": "docx",
      "scope": { "from": 1, "maxParas": 200, "headerFooter": true, "pictures": true, "textBoxes": true, "notes": true, "comments": true, "charts": true, "smartArt": true, "oleObjects": true, "tables": true, "revisions": true, "hyperlinks": true, "bookmarks": true, "fields": true, "tocs": true, "maths": true, "contentControls": true, "numbering": true, "styles": true, "runs": true }
}
```

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `path` | 是 | 已有文件 |
| `scope` | 否 | 省略或空对象：返回 `outline`、`paraCount`、`pictureCount`、`textBoxCount`、`footnoteCount`、`endnoteCount`、`commentCount`、`chartCount`、`smartArtCount`、`oleObjectCount`、`tableCount`、`revisionCount`、`hyperlinkCount`、`bookmarkCount`、`fieldCount`、`tocCount`、`mathCount`、`contentControlCount`、`numberingCount`、`styleCount`、`headers`、`footers`、`sections`；正文没有 `paragraphs`；页眉页脚容器不带 `paragraphs`；没有 `footnotes` / `endnotes` / `noteSeparators` / `comments` / `charts` / `smartArts` / `oleObjects` / `tables` / `revisions` / `hyperlinks` / `bookmarks` / `fields` / `tocs` / `maths` / `contentControls` / `numberings` / `styles` |
| `scope.from` | 要正文段落时必填 | 从第几段开始，从 1 起，按主文档 `w:p` 顺序 |
| `scope.maxParas` | 要正文段落时必填 | 本次最多返回多少段，允许范围 `1` 到 `5000` |
| `scope.headerFooter` | 否 | 为 `true` 时给每个页眉、页脚节点填 `paragraphs`。省略或 `false` 则容器上没有 `paragraphs` |
| `scope.pictures` | 否 | 为 `true` 时给每个页眉、页脚节点填 `pictures`；若同时给了 `from` 与 `maxParas`，切片里每段也填 `pictures`；若同时 `notes: true`，每条用户注也填 `pictures`；若同时 `comments: true`，每条批注也填 `pictures`。省略或 `false` 则这些节点上都没有 `pictures`。不读媒体字节 |
| `scope.textBoxes` | 否 | 为 `true` 时给每个页眉、页脚节点填 `textBoxes`；若同时给了 `from` 与 `maxParas`，切片里每段也填 `textBoxes`；若同时 `notes: true`，每条用户注也填 `textBoxes`；若同时 `comments: true`，每条批注也填 `textBoxes`。省略或 `false` 则这些节点上都没有 `textBoxes` |
| `scope.textBoxContent` | 否 | 为 `true` 且该节点已有 `textBoxes` 时，每个框再带全部 `paragraphs`；若同时 `pictures: true`，框内段再带 `pictures`。省略或 `false` 则框上只有计数，没有框内名单。不与 `from` / `maxParas` 抢正文切片 |
| `scope.notes` | 否 | 为 `true` 时列出全部用户脚注、尾注（`footnotes` / `endnotes`）以及分隔符（`noteSeparators`）。每条用户注带全部注内段落，不再用 `from` / `maxParas` 切。省略或 `false` 则只有两个计数，没有这三份名单 |
| `scope.comments` | 否 | 为 `true` 时列出全部批注（`comments`）。每条带全部批注内段落，不再用 `from` / `maxParas` 切。省略或 `false` 则只有 `commentCount`，没有名单 |
| `scope.charts` | 否 | 为 `true` 时列出正文图表（根上 `charts`）以及每个页眉、页脚容器上的 `charts`；若同时 `notes: true` / `comments: true`，每条用户注或批注也填 `charts`；已列出的框在给了 `textBoxes` 时也填 `charts`。不依赖 `from` / `maxParas`。省略或 `false` 则只有 `chartCount`，没有名单。不把 embedding 字节放进树 |
| `scope.smartArt` | 否 | 为 `true` 时列出正文 SmartArt（根上 `smartArts`）以及每个页眉、页脚容器上的 `smartArts`；若同时 `notes: true` / `comments: true`，每条用户注或批注也填 `smartArts`；已列出的框在给了 `textBoxes` 时也填 `smartArts`。不依赖 `from` / `maxParas`。省略或 `false` 则只有 `smartArtCount`，没有名单。不把图示画成图，不读 Fallback 位图 |
| `scope.oleObjects` | 否 | 为 `true` 时列出正文嵌入对象（根上 `oleObjects`）以及每个页眉、页脚容器上的 `oleObjects`。不依赖 `from` / `maxParas`。省略或 `false` 则只有 `oleObjectCount`，没有名单。不把嵌入字节或预览图放进树。框内、注内、批注内本期不列出 |
| `scope.tables` | 否 | 为 `true` 时列出正文表（根上 `tables`）以及每个页眉、页脚容器上的 `tables`。不依赖 `from` / `maxParas`。省略或 `false` 则只有 `tableCount`，没有名单 |
| `scope.revisions` | 否 | 为 `true` 时列出正文修订（根上 `revisions`）以及每个页眉、页脚容器上的 `revisions`。不依赖 `from` / `maxParas`。省略或 `false` 则只有 `revisionCount`，没有名单 |
| `scope.hyperlinks` | 否 | 为 `true` 时列出正文超链接（根上 `hyperlinks`）以及每个页眉、页脚容器上的 `hyperlinks`。不依赖 `from` / `maxParas`。省略或 `false` 则只有 `hyperlinkCount`，没有名单 |
| `scope.bookmarks` | 否 | 为 `true` 时列出正文书签（根上 `bookmarks`）以及每个页眉、页脚容器上的 `bookmarks`。不依赖 `from` / `maxParas`。省略或 `false` 则只有 `bookmarkCount`，没有名单 |
| `scope.fields` | 否 | 为 `true` 时列出正文域（根上 `fields`）以及每个页眉、页脚容器上的 `fields`。不依赖 `from` / `maxParas`。省略或 `false` 则只有 `fieldCount`，没有名单 |
| `scope.tocs` | 否 | 为 `true` 时列出主文档目录块（根上 `tocs`）。不依赖 `from` / `maxParas`。省略或 `false` 则只有 `tocCount`，没有名单。页眉页脚里的 TOC 仍只出现在该容器 `fields` 里 |
| `scope.maths` | 否 | 为 `true` 时列出正文公式（根上 `maths`）以及每个页眉、页脚容器上的 `maths`。不依赖 `from` / `maxParas`。省略或 `false` 则只有 `mathCount`，没有名单 |
| `scope.contentControls` | 否 | 为 `true` 时列出正文内容控件（根上 `contentControls`）以及每个页眉、页脚容器上的 `contentControls`。不依赖 `from` / `maxParas`。省略或 `false` 则只有 `contentControlCount`，没有名单 |
| `scope.numbering` | 否 | 为 `true` 时列出编号实例（根上 `numberings`）。不依赖 `from` / `maxParas`。省略或 `false` 则只有 `numberingCount`，没有名单。页眉页脚容器不挂这份名单 |
| `scope.styles` | 否 | 为 `true` 时列出样式定义（根上 `styles`）。不依赖 `from` / `maxParas`。省略或 `false` 则只有 `styleCount`，没有名单。页眉页脚容器不挂这份名单 |
| `scope.runs` | 否 | 为 `true` 时，已列出的段落（正文切片、已展开的页眉页脚、框内、注内、批注内、表格单元格）带 `runs`。省略或 `false` 则这些段只有 `runCount`，没有名单 |

`from` / `maxParas` 只切正文，`headerFooter` 只管页眉页脚段落，`pictures` 只展开图名单，`textBoxes` 只展开框名单，`notes` 只展开脚注尾注，`comments` 只展开批注，`charts` 只展开图表名单，`smartArt` 只展开图示名单，`oleObjects` 只展开嵌入对象名单，`tables` 只展开表名单，`revisions` 只展开修订名单，`hyperlinks` 只展开超链接名单，`bookmarks` 只展开书签名单，`fields` 只展开域名单，`maths` 只展开公式名单，`contentControls` 只展开内容控件名单，`numbering` 只展开编号实例名单，`styles` 只展开样式定义名单，`runs` 只展开已列出段落上的 run 名单。这些键彼此独立，可只开一边。只要想看正文段落，就必须同时给 `from` 和 `maxParas`。不默认把全文倒出来。正文图与正文框名单只出现在已切出的段落上，不挂到文档根。正文图表名单、正文 SmartArt 名单、正文 OLE 名单、正文表名单、正文修订名单、正文超链接名单、正文书签名单、正文域名单、目录名单、公式名单、内容控件名单、编号实例名单与样式定义名单挂在文档根上，不挂在段落上。`from` 超出 `paraCount` 时正文 `paragraphs` 为空数组，`truncated` 为 `false`，不报错。页眉页脚通常很短，给了对应键就返回该部件全部段落、图、框、图表、SmartArt、嵌入对象、表、修订、超链接、书签、域、公式或内容控件，都不再单独分页。框通常短，给了 `textBoxContent: true` 就返回该框全部段落，不再单独分页。用户注和批注通常不多，给了 `notes: true` 或 `comments: true` 就返回全部名单，每条倒出全部段落，不再单独分页。图表通常不多，给了 `charts: true` 就返回全部名单，每张带全部系列，不再单独分页。SmartArt 通常不多，给了 `smartArt: true` 就返回全部名单，每张带全部可见点，不再单独分页。嵌入对象通常不多，给了 `oleObjects: true` 就返回全部名单。表通常不多，给了 `tables: true` 就返回全部名单。修订通常不多，给了 `revisions: true` 就返回全部名单。超链接、书签与域通常不多，给了对应键就返回全部名单。编号定义通常不多，给了 `numbering: true` 就返回全部实例。样式定义通常不多，给了 `styles: true` 就返回全部样式。已列出的段落始终有 `runCount`；给了 `runs: true` 才带 `runs`。xlsx 的 `scope` 里出现 `textBoxes`、`textBoxContent`、`notes`、`comments`、`smartArt`、`oleObjects`、`tables`、`revisions`、`hyperlinks`、`bookmarks`、`fields`、`tocs`、`maths`、`contentControls`、`numbering`、`styles` 或 `runs` 报 `InvalidField`。pptx 的 `scope` 里出现 `headerFooter`、`textBoxes`、`textBoxContent`、`tables`、`revisions`、`hyperlinks`、`bookmarks`、`fields`、`tocs`、`maths`、`contentControls`、`numbering`、`styles`、`oleObjects` 或 `runs` 报 `InvalidField`。pptx 认 `scope.pictures`、`scope.charts`、`scope.smartArt`、`scope.notes` 与 `scope.comments`。只给 `textBoxContent`、不给 `textBoxes` 报 `InvalidField`。

#### pptx 的 scope

```json
{
  "schemaVersion": 1,
  "action": "inspect",
  "path": "C:\\data\\deck.pptx",
  "format": "pptx",
  "scope": { "slide": 1, "maxShapes": 200 }
}
```

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `path` | 是 | 已有文件 |
| `scope` | 否 | 省略或空对象：返回 `slideCount`、`pictureCount`、`chartCount`、`smartArtCount`、`commentCount` 和页名单；各页有这四项计数和 `hasNotes`，没有 `shapes`，也没有 `pictures` / `charts` / `smartArts` 名单，也没有 `notes` 对象，也没有 `comments` 名单 |
| `scope.slide` | 要形状时必填 | 页序号，从 1 起，按 `p:sldIdLst` 顺序 |
| `scope.maxShapes` | 否 | 该页最多返回多少个文本形状，默认 `2000`，允许范围 `1` 到 `5000`。不管备注形状 |
| `scope.pictures` | 否 | 为 `true` 时给每页填 `pictures`。若同时 `notes: true` 且该页有备注对象，备注上也填 `pictures`。省略或 `false` 则只有计数，没有名单。不读媒体字节。不依赖 `scope.slide` |
| `scope.charts` | 否 | 为 `true` 时给每页填 `charts`。省略或 `false` 则只有 `chartCount`，没有名单。不把 chart 部件或 embedding 字节放进树。不依赖 `scope.slide` / `pictures`。备注里的图表本期不进树 |
| `scope.smartArt` | 否 | 为 `true` 时给每页填 `smartArts`。省略或 `false` 则只有 `smartArtCount`，没有名单。不把图示画成图，不读 Fallback 位图。不依赖 `scope.slide` / `pictures` / `charts`。备注里的 SmartArt 本期不进树 |
| `scope.notes` | 否 | 为 `true` 时给每页填 `notes`。有备注页则为对象，没有则为 `null`。省略或 `false` 则只有 `hasNotes`，不倒出备注形状。不依赖 `scope.slide` |
| `scope.comments` | 否 | 为 `true` 时给每页填 `comments`。没有现代批注时为空数组。省略或 `false` 则只有 `commentCount`，没有名单。不依赖 `scope.slide`。不读 ECMA-376 §19.4 旧式 `commentN.xml` |

只要想看形状，就必须给 `slide`。不默认把每一页的文本框都倒出来。只给 `maxShapes`、不给 `slide` 报 `ScopeRequired`。只给 `pictures: true`、`charts: true`、`smartArt: true`、`notes: true` 或 `comments: true`、不给 `slide` 合法，各页都带对应名单、都不带幻灯片 `shapes`。`slide` 小于 `1` 报 `InvalidField`。`slide` 超出 `slideCount` 时各页都不带 `shapes`，不报错；图名单、图表名单、SmartArt 名单、备注对象与批注名单仍按各自键决定。

### 响应（xlsx）

```json
{
  "schemaVersion": 1,
  "format": "xlsx",
  "id": "xlsx:workbook",
  "pictureCount": 0,
  "chartCount": 0,
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
  "sheets": [
    {
      "id": "sheet:Sheet1",
      "name": "Sheet1",
      "kind": "worksheet",
      "pictureCount": 0,
      "chartCount": 0,
      "hasHeaderFooter": false,
      "truncated": true,
      "range": "A1:D20",
      "cells": [
        {
          "id": "cell:Sheet1!A1",
          "ref": "A1",
          "type": "number",
          "value": 1,
          "formula": null
        },
        {
          "id": "cell:Sheet1!B1",
          "ref": "B1",
          "type": "text",
          "value": "hello",
          "formula": null
        },
        {
          "id": "cell:Sheet1!C1",
          "ref": "C1",
          "type": "number",
          "value": 6,
          "formula": "=SUM(A1:A10)"
        }
      ]
    },
    {
      "id": "sheet:Sheet2",
      "name": "Sheet2",
      "kind": "worksheet",
      "pictureCount": 0,
      "chartCount": 0,
      "hasHeaderFooter": false
    }
  ]
}
```

规则：

- 工作簿字段顺序：`id`、`pictureCount`、`chartCount`、`props`、`sheets`。`pictureCount` 始终出现，等于各 `worksheet` 的 `pictureCount` 之和，不含 `chartsheet` / `dialogsheet`。`chartCount` 始终出现，等于各 `worksheet` 与 `chartsheet` 的 `chartCount` 之和，不含 `dialogsheet`。`props` 始终出现，规则见下文「文档属性」。
- `sheets` 始终列出工作簿里全部表（含隐藏表），便于模型知道还有哪些表。未点名的表不带 `cells`、`range`、`truncated`。
- `kind`：`worksheet`、`chartsheet`、`dialogsheet`。第一期只对 `worksheet` 填单元格与图。`worksheet` 与 `chartsheet` 都填 `chartCount`。每张表始终填 `hasHeaderFooter`：该表 `headerFooter` 里任一槽位有非空文本则为 `true`；没有该元素、空的 `<headerFooter/>`、或六个槽位都空则为 `false`。不算 VML，也不因为只有 `drawingHF` 图就把此项改成 `true`。
- `worksheet` 字段顺序：`id`、`name`、`kind`、`pictureCount`、`chartCount`、`hasHeaderFooter`，有页眉名单时再跟 `headerFooters`，有图名单时再跟 `pictures`，有图表名单时再跟 `charts`，有单元格切片时再跟 `truncated`、`range`、`cells`。`pictureCount` 始终出现，是该表工作表 drawing 里被收入的 `xdr:pic` 个数（判定见上文 xlsx 图 id 那节），不含页眉图。没有图则为 `0`。`chartCount` 始终出现，是该表被收入的图表个数（判定见上文 xlsx 图表 id 那节）。没有图表则为 `0`。`chartsheet` 没有 `pictureCount`，也没有 `pictures`；字段顺序：`id`、`name`、`kind`、`chartCount`、`hasHeaderFooter`，有页眉名单时再跟 `headerFooters`，有图表名单时再跟 `charts`。`dialogsheet` 没有 `pictureCount` / `pictures` / `chartCount` / `charts`；字段顺序：`id`、`name`、`kind`、`hasHeaderFooter`，有页眉名单时再跟 `headerFooters`。
- `scope.headerFooter` 为 `true` 时，每张表带 `headerFooters`，固定六个对象，顺序见上文 id 表。每个槽位字段顺序：`id`、`kind`、`side`、`text`；`worksheet` 再跟 `pictureCount`，若同时 `pictures: true` 再跟 `pictures`。`kind` / `side` 与 id 里那两段相同。`text` 为该 XML 元素原文，没有该元素则为 `null`。空串是合法原文，与 `null` 不同。`pictureCount` 是分到该槽的 DrawingML 页眉图个数。`chartsheet` / `dialogsheet` 的槽位没有 `pictureCount` / `pictures`。
- `scope.pictures` 为 `true` 时，每张 `worksheet` 带工作表 `pictures`。没有图时为空数组。按该表 `<drawing>` 部件顺序、同一部件内按锚点文档顺序。每张字段顺序：`id`、`cNvPrId`、`name`、`anchor`，然后按锚点种类跟位置字段，最后 `stable`。`cNvPrId` 是 `xdr:cNvPr/@id` 原文，没有则为 `null`。`name` 取 `xdr:cNvPr/@name`，没有则为 `""`。`anchor` 为 `twoCell`、`oneCell` 或 `absolute`。单独 `pictures: true`、不开 `headerFooter`：只列工作表图，不含页眉图。
- `twoCell`（§20.5.2.33）再跟 `from`、`fromOffX`、`fromOffY`、`to`、`toOffX`、`toOffY`、`editAs`。不报 `cx` / `cy`（显示尺寸由起止格与列宽行高决定，库不算）。`editAs` 取 `twoCellAnchor/@editAs`：`twoCell`、`oneCell` 或 `absolute`；缺省当作 `twoCell`。
- `oneCell`（§20.5.2.24）再跟 `from`、`fromOffX`、`fromOffY`、`cx`、`cy`。`cx` / `cy` 取 `xdr:ext`，没有则为 `null`。
- `absolute`（§20.5.2.1）再跟 `posX`、`posY`、`cx`、`cy`。`posX` / `posY` 取 `xdr:pos`。
- `from` / `to` 是 A1（列字母加大写行号）。XML 里 `xdr:col` / `xdr:row` 从 0 计（§20.5.3.1、§20.5.3.3），树上换成 A1，与单元格 id 同一套写法。`fromOffX` / `fromOffY` / `toOffX` / `toOffY` 是 EMU 整数，取 `colOff` / `rowOff`，缺省为 `0`。
- `scope.charts` 为 `true` 时，每张 `worksheet` 与 `chartsheet` 带 `charts`。没有图表时为空数组。按该表 drawing 部件顺序、同一部件内按锚点文档顺序。每张字段顺序：`id`、`cNvPrId`、`name`、`anchor`，然后按锚点种类跟位置字段（规则与工作表图相同），再跟 `type`、`title`、`catAxisTitle`、`valAxisTitle`、`series`、`legend`、`dataLabels`、`flags`、`stable`。`cNvPrId` / `name` / `anchor` / 位置字段 / `stable` 规则与图相同。`type` / `title` / 轴标题 / `legend` / `dataLabels` / `flags` 规则与 docx 图表相同。
- 每条系列字段顺序：`id`、`index`、`name`、`nameRef`、`categories`、`catRef`、`values`、`valRef`，气泡再跟 `sizes`、`sizeRef`，必要时再跟 `cacheStale`。`index` 即 `{idx}`。`name` / `categories` / `values` / `sizes` 优先取 `c:f` 指向的单元格缓存值（与 inspect 单元格同一套 `<v>`）；读不到再退回 chart 的 cache / lit。`nameRef` / `catRef` / `valRef` / `sizeRef` 取对应 `c:f` 原文；没有公式则为 `null`。单元格值与 chart cache 不一致时该系列 `cacheStale: true`。
- 不把媒体字节或 chart 部件放进树。
- `cells` 稀疏：范围内没有 `<c>` 的格子不出现。
- 范围内有内容的格子超过 `maxCells` 时，按行优先、从左到右截断，`truncated` 为 `true`。未截断则为 `false`。
- `type`：`number`、`text`、`boolean`、`error`、`date`、`empty`。对应 SpreadsheetML `ST_CellType`（§18.18.11）的映射见下表。
- `value` 是缓存显示值，不是库算出来的。公式格的 `value` 来自 `<v>`，可能过期。没有 `<v>` 时 `value` 为 `null`。
- `formula`：有公式则返回以 `=` 开头的字符串。XML 里 `<f>` 不含 `=`，由库补上。没有公式则为 `null`。
- 共享公式（`<f t="shared">`）：主单元格返回其公式文本；从属单元格若 XML 里没有文本，第一期 `formula` 为 `null`，并多一个 `"shared": true`。以后可以改成展开成该格的 A1 公式，不改 id 规则。
- 内联字符串（`t="inlineStr"`）和共享字符串（`t="s"`）在树上都是 `type: "text"`，`value` 为拼接后的可见纯文本，不含 run 级样式。`t="s"` 的 `<v>` 必须是共享字符串表里的合法非负整数索引。越界或无法解析报 `XmlBroken`，不要当成空串。

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

### 响应（docx）

```json
{
  "schemaVersion": 1,
  "format": "docx",
  "id": "docx:document",
  "paraCount": 4,
  "pictureCount": 0,
  "textBoxCount": 0,
  "footnoteCount": 0,
  "endnoteCount": 0,
  "commentCount": 0,
  "chartCount": 0,
  "smartArtCount": 0,
  "oleObjectCount": 0,
  "tableCount": 0,
  "revisionCount": 0,
  "hyperlinkCount": 0,
  "bookmarkCount": 0,
  "fieldCount": 0,
  "tocCount": 0,
  "mathCount": 0,
  "contentControlCount": 0,
  "numberingCount": 0,
  "styleCount": 4,
  "outline": [
    {
      "id": "para:3F2A1B00",
      "index": 1,
      "level": 1,
      "style": "Heading1",
      "text": "标题"
    }
  ],
  "headers": [
    {
      "id": "header:default:1",
      "type": "default",
      "sections": [1],
      "paraCount": 1,
      "pictureCount": 0,
      "textBoxCount": 0,
      "chartCount": 0,
      "smartArtCount": 0,
      "oleObjectCount": 0,
      "tableCount": 0,
      "revisionCount": 0,
      "hyperlinkCount": 0,
      "bookmarkCount": 0,
      "fieldCount": 0,
      "mathCount": 0,
      "contentControlCount": 0
    }
  ],
  "footers": [
    {
      "id": "footer:default:1",
      "type": "default",
      "sections": [1],
      "paraCount": 1,
      "pictureCount": 0,
      "textBoxCount": 0,
      "chartCount": 0,
      "smartArtCount": 0,
      "oleObjectCount": 0,
      "tableCount": 0,
      "revisionCount": 0,
      "hyperlinkCount": 0,
      "bookmarkCount": 0,
      "fieldCount": 0,
      "mathCount": 0,
      "contentControlCount": 0,
      "paragraphs": [
        {
          "id": "footer:default:1/para:00FF00AA",
          "index": 1,
          "style": null,
          "text": "页脚",
          "stable": true
        }
      ]
    }
  ],
  "sections": [
    {
      "id": "section:1",
      "index": 1,
      "headerDistance": 708,
      "footerDistance": 708,
      "top": 1440,
      "bottom": 1440,
      "left": 1440,
      "right": 1440,
      "gutter": 0,
      "pageWidth": 11906,
      "pageHeight": 16838,
      "orient": null,
      "columns": null,
      "vAlign": null,
      "lineNumbering": null,
      "docGrid": null,
      "rtlGutter": null,
      "headerLink": { "default": false, "even": null, "first": null },
      "footerLink": { "default": false, "even": null, "first": null }
    }
  ],
  "mirrorMargins": null,
  "gutterAtTop": null,
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
  "from": 1,
  "truncated": false,
  "paragraphs": [
    {
      "id": "para:3F2A1B00",
      "index": 1,
      "style": "Heading1",
      "level": 1,
      "text": "标题",
      "stable": true,
      "pictureCount": 0,
      "textBoxCount": 0
    },
    {
      "id": "para:index:2",
      "index": 2,
      "style": "Normal",
      "text": "hello",
      "stable": false,
      "pictureCount": 0,
      "textBoxCount": 0
    }
  ]
}
```

规则：

- 主文档部件（根关系里 `/officeDocument` 那条，常见路径 `/word/document.xml`）始终读。页眉、页脚部件按节引用与继承解析后读。脚注、尾注、批注部件按主文档关系去读（常见 `/word/footnotes.xml`、`/word/endnotes.xml`、`/word/comments.xml`）；没有对应关系则对应计数为 0。`commentsExtended`、`people`、`commentsIds`、`commentsExtensible` 有则读，没有不报错。
- 文档根字段顺序：`id`、`paraCount`、`pictureCount`、`textBoxCount`、`footnoteCount`、`endnoteCount`、`commentCount`、`chartCount`、`smartArtCount`、`oleObjectCount`、`tableCount`、`revisionCount`、`hyperlinkCount`、`bookmarkCount`、`fieldCount`、`tocCount`、`mathCount`、`contentControlCount`、`numberingCount`、`styleCount`、`outline`、`headers`、`footers`、`sections`、`mirrorMargins`、`gutterAtTop`、`props`、`protection`，有注名单时再跟 `footnotes`、`endnotes`、`noteSeparators`，有批注名单时再跟 `comments`，有图表名单时再跟 `charts`，有 SmartArt 名单时再跟 `smartArts`，有 OLE 名单时再跟 `oleObjects`，有表名单时再跟 `tables`，有修订名单时再跟 `revisions`，有超链接名单时再跟 `hyperlinks`，有书签名单时再跟 `bookmarks`，有域名单时再跟 `fields`，有目录名单时再跟 `tocs`，有公式名单时再跟 `maths`，有内容控件名单时再跟 `contentControls`，有编号名单时再跟 `numberings`，有样式名单时再跟 `styles`，有正文切片时再跟 `from`、`truncated`、`paragraphs`。`props` 始终出现，规则见下文「文档属性」。`protection` 始终出现，规则见下文「文档保护」。xlsx / pptx 根上不要带 `protection`。
- `paraCount` 是主文档里 `w:p` 的总数，含表格单元格里的段落，不含文本框里的段落，始终出现。
- 文档根上的 `pictureCount` 始终出现，只计主文档里被收入的 DrawingML 图片个数（判定见上文 docx 图 id 那节），不含页眉页脚，也不含框内图、注内图、批注内图。根上不挂正文 `pictures`。
- 文档根上的 `textBoxCount` 始终出现，只计主文档里被收入的文本框个数，不含页眉页脚，也不含注内、批注内的框。根上不挂正文 `textBoxes`。
- 文档根上的 `footnoteCount` / `endnoteCount` 始终出现，只计用户注（`normal` 或未写 type），不含分隔符。没有注部件则为 `0`。根上不挂注名单，除非给了 `scope.notes`。
- 文档根上的 `commentCount` 始终出现，计 `comments.xml` 里全部 `w:comment`（含回复与墨迹）。没有批注部件则为 `0`。根上不挂批注名单，除非给了 `scope.comments`。
- 文档根上的 `chartCount` 始终出现，只计主文档里被收入的图表个数（判定见上文 docx 图表 id 那节），不含页眉页脚，也不含框内、注内、批注内的图表。没有图表则为 `0`。根上不挂正文 `charts`，除非给了 `scope.charts`。
- 文档根上的 `smartArtCount` 始终出现，只计主文档里被收入的 SmartArt 个数（判定见上文 docx SmartArt id 那节），不含页眉页脚，也不含框内、注内、批注内的图示。没有图示则为 `0`。根上不挂正文 `smartArts`，除非给了 `scope.smartArt`。
- 文档根上的 `oleObjectCount` 始终出现，只计主文档里被收入的嵌入对象个数（判定见上文 docx OLE id 那节），不含页眉页脚，也不含框内、注内、批注内。没有则为 `0`。根上不挂正文 `oleObjects`，除非给了 `scope.oleObjects`。每条字段顺序：`id`、`progId`、`name`、`kind`、`contentType`、`stable`。`progId` 取 `o:OLEObject/@ProgID` 或 `w:objectEmbed/@progId`，缺则为 `null`。`name` 取该 `w:object` 里 `wp:docPr/@name`，没有则为 `null`。`kind`：ProgID 为 `Package`（大小写不敏感）时为 `package`，否则为 `embed`。`contentType` 取 embeddings 部件在 `[Content_Types].xml` 里的 Default 或 Override，都没有则为 `null`。不把嵌入字节放进树。
- 文档根上的 `tableCount` 始终出现，只计主文档里被收入的 `w:tbl` 个数（含嵌套表，判定见上文表 id 那节），不含页眉页脚，也不含框内、注内、批注内的表。没有表则为 `0`。根上不挂正文 `tables`，除非给了 `scope.tables`。
- 文档根上的 `revisionCount` 始终出现，只计主文档里被收入的修订节点个数（判定见上文修订 id 那节），不含页眉页脚，也不含框内、注内、批注内的修订。没有修订则为 `0`。根上不挂正文 `revisions`，除非给了 `scope.revisions`。
- 文档根上的 `hyperlinkCount` 始终出现，只计主文档里被收入的 `w:hyperlink` 个数（判定见上文超链接 id 那节），不含页眉页脚，也不含框内、注内、批注内的链。没有链则为 `0`。根上不挂正文 `hyperlinks`，除非给了 `scope.hyperlinks`。
- 文档根上的 `bookmarkCount` 始终出现，只计主文档里被收入的书签个数（判定见上文书签 id 那节），不含页眉页脚，也不含框内、注内、批注内的书签。没有书签则为 `0`。根上不挂正文 `bookmarks`，除非给了 `scope.bookmarks`。
- 文档根上的 `fieldCount` 始终出现，只计主文档里被收入的域个数（判定见上文域 id 那节），不含页眉页脚，也不含框内、注内、批注内的域。没有域则为 `0`。根上不挂正文 `fields`，除非给了 `scope.fields`。
- 文档根上的 `tocCount` 始终出现，只计主文档里被收入的目录块个数（判定见上文目录 id 那节）。没有目录则为 `0`。根上不挂 `tocs`，除非给了 `scope.tocs`。页眉页脚容器不出现 `tocCount`。
- 文档根上的 `mathCount` 始终出现，只计主文档里被收入的顶层 `m:oMath` 个数（判定见上文公式 id 那节），不含页眉页脚，也不含框内、注内、批注内的公式。没有公式则为 `0`。根上不挂 `maths`，除非给了 `scope.maths`。
- 文档根上的 `contentControlCount` 始终出现，只计主文档里被收入的 `w:sdt` 个数（判定见上文内容控件 id 那节），不含页眉页脚，也不含框内、注内、批注内的控件。没有控件则为 `0`。根上不挂 `contentControls`，除非给了 `scope.contentControls`。
- 文档根上的 `numberingCount` 始终出现，计 `numbering.xml` 里 `w:num` 且 `numId` 不为 0 的个数。没有 numbering 部件则为 `0`。根上不挂 `numberings`，除非给了 `scope.numbering`。页眉页脚容器不出现 `numberingCount`。
- 文档根上的 `styleCount` 始终出现，计 `styles.xml` 里 `w:style` 且 `styleId` 非空的个数。没有 styles 部件则为 `0`。根上不挂 `styles`，除非给了 `scope.styles`。页眉页脚容器不出现 `styleCount`。
- `outline` 始终出现，只含主文档标题段。收入满足下面任一条件的段落：`w:pStyle/@w:val` 为 `Heading1`–`Heading9`（大小写不敏感）或 `标题1`–`标题9`；或者 `w:outlineLvl/@w:val` 为 `0`–`8`。树上 `level` 从 1 到 9。`Heading1` / `标题1` / `outlineLvl=0` 都是 `level: 1`。不是标题的段落不进 `outline`。页眉页脚段落不进 `outline`。
- `headers`、`footers` 始终出现。没有引用则为空数组。容器字段顺序：`id`、`type`、`sections`、`paraCount`、`pictureCount`、`textBoxCount`、`chartCount`、`smartArtCount`、`oleObjectCount`、`tableCount`、`revisionCount`、`hyperlinkCount`、`bookmarkCount`、`fieldCount`、`mathCount`、`contentControlCount`，有图名单时再跟 `pictures`，有框名单时再跟 `textBoxes`，有图表名单时再跟 `charts`，有 SmartArt 名单时再跟 `smartArts`，有 OLE 名单时再跟 `oleObjects`，有表名单时再跟 `tables`，有修订名单时再跟 `revisions`，有超链接名单时再跟 `hyperlinks`，有书签名单时再跟 `bookmarks`，有域名单时再跟 `fields`，有公式名单时再跟 `maths`，有内容控件名单时再跟 `contentControls`，有段落时再跟 `paragraphs`。`sections` 是从 1 起的节号数组，按主文档里 `w:sectPr` 的文档顺序。容器上的 `paraCount` 是该部件内 `w:p` 的个数（边界与正文相同）。`pictureCount` 始终出现，是该部件内被收入的 DrawingML 图片个数（不含框内图）。`textBoxCount` 始终出现，是该部件内被收入的文本框个数。`chartCount` 始终出现，是该部件内被收入的图表个数（不含框内图表）。`smartArtCount` 始终出现，是该部件内被收入的 SmartArt 个数（不含框内图示）。`oleObjectCount` 始终出现，是该部件内被收入的嵌入对象个数（不含框内）。`tableCount` 始终出现，是该部件内被收入的 `w:tbl` 个数（含嵌套表，不含框内表）。`revisionCount` 始终出现，是该部件内被收入的修订节点个数（不含框内修订）。`hyperlinkCount` 始终出现，是该部件内被收入的 `w:hyperlink` 个数（不含框内链）。`bookmarkCount` 始终出现，是该部件内被收入的书签个数（不含框内书签）。`fieldCount` 始终出现，是该部件内被收入的域个数（不含框内域）。`mathCount` 始终出现，是该部件内被收入的顶层 `m:oMath` 个数（不含框内公式）。`contentControlCount` 始终出现，是该部件内被收入的内容控件个数（不含框内控件）。
- 文档根上的 `sections` 始终出现。每个 `w:sectPr` 一项，字段顺序：`id`、`index`、`headerDistance`、`footerDistance`、`top`、`bottom`、`left`、`right`、`gutter`、`pageWidth`、`pageHeight`、`orient`、`columns`、`vAlign`、`lineNumbering`、`docGrid`、`rtlGutter`、`headerLink`、`footerLink`。
- `headerDistance` / `footerDistance` 取该节 `w:pgMar` 的 `w:header` / `w:footer`（§17.6.11），单位 twip，JSON 非负整数。`left` / `right` / `gutter` 同属 `w:pgMar`，同样非负。`top` / `bottom` 是 `ST_SignedTwipsMeasure`，可为负整数。缺 `w:pgMar` 或缺对应属性则为 `null`，不猜默认值。属性存在但不是整数，或本该非负却为负，报 `XmlBroken`。
- `pageWidth` / `pageHeight` 取该节 `w:pgSz` 的 `w` / `h`（§17.6.13），单位 twip，非负整数。`orient` 取 `w:orient`，值为 `portrait` 或 `landscape`。缺 `w:pgSz` 或缺对应属性则为 `null`；规范说省略 `orient` 视为纵向，树上仍给 `null`，不猜。属性存在但不是非负整数，或 `orient` 不是这两个词，报 `XmlBroken`。不读写 `w:code`。改 `orient` 时不自动对调宽高。
- `columns` 取 `w:cols`（§17.6.4）。没有该元素则为 `null`。有则对象字段顺序：`num`、`space`、`equalWidth`、`sep`、`cols`。`num` / `space` 缺则为 `null`（`space` 单位 twip，非负）。`equalWidth` / `sep` 取 OnOff：`1` / `true` / `on` 为 `true`，`0` / `false` / `off` 为 `false`，缺属性为 `null`。`cols` 是子元素 `w:col` 的数组，按文档顺序；没有子元素则为 `[]`。每项字段顺序：`width`、`space`，对应 `w:w` / `w:space`，缺则为 `null`。不等宽时列数以子元素为准，树上仍原样给出 `num`。
- `vAlign` 取 `w:vAlign/@w:val`（§17.6.23），值为 `top` / `center` / `both` / `bottom`。没有元素则为 `null`。有元素却没有 `val` 或不认识，报 `XmlBroken`。
- `lineNumbering` 取 `w:lnNumType`（§17.6.8）。没有该元素则为 `null`。有则对象字段顺序：`countBy`、`start`、`distance`、`restart`。前三项缺则为 `null`（`distance` 单位 twip，非负；`countBy` / `start` 为整数）。`restart` 为 `newPage` / `newSection` / `continuous`，缺则为 `null`。不认识的 `restart` 或不是整数的数字属性报 `XmlBroken`。
- `docGrid` 取 `w:docGrid`（§17.6.5）。没有该元素则为 `null`。有则对象字段顺序：`type`、`linePitch`、`charSpace`。`type` 为 `default` / `lines` / `linesAndChars` / `snapToChars`，缺则为 `null`。`linePitch` / `charSpace` 为整数，缺则为 `null`。不认识的 `type` 或不是整数报 `XmlBroken`。
- `rtlGutter` 取本节 `w:rtlGutter`（§17.6.16）：没有元素为 `null`；有且 OnOff 为开为 `true`，为关为 `false`。
- 文档根上的 `mirrorMargins` / `gutterAtTop` 始终出现，取 `settings.xml` 的 `w:mirrorMargins` / `w:gutterAtTop`（§17.15.1.64、§17.15.1.50）。没有 settings 部件、或没有对应元素，则为 `null`。有则按 OnOff 写成布尔。这两项是整篇的，不出现在节节点上。
- `headerLink` / `footerLink` 始终出现，键顺序为 `default`、`even`、`first`。每个键的取值：`true` 表示该类型对本节生效，且本节 `sectPr` 没有对应引用（继承前节；前节也没有部件、规范要求空白时仍是 `true`）；`false` 表示该类型生效，且本节有显式引用；`null` 表示该类型对本节不生效。`even` 是否生效看整篇 `w:evenAndOddHeaders`；`first` 是否生效看本节 `w:titlePg`。第一节的 `default` 只能是 `false`（没有前节可链）。第一节生效的 `even` / `first` 若无引用也是 `false`（空白，不是链接）。两节显式指向同一部件时两边都是 `false`。`headers` / `footers` 仍按部件去重；断开链接后会出现新的容器 id。
- 省略 `scope`、或没有 `from`/`maxParas` 时，没有正文 `paragraphs`、`from`、`truncated`。
- 给了 `from` 和 `maxParas` 时，正文 `paragraphs` 按文档顺序切片。范围内段落数超过 `maxParas` 则 `truncated` 为 `true`。切片里每段始终有 `pictureCount`、`textBoxCount` 与 `runCount`。段落字段顺序：`id`、`index`、`style`、`level`（标题才有）、该段自己的 `w:pPr/w:numPr` 且 `numId` 非 0 时再跟 `num`、`ilvl`、然后始终跟 `align`、`indent`、`spacing`、`text`、`stable`、`pictureCount`、`textBoxCount`、`runCount`，有图名单时再跟 `pictures`，有框名单时再跟 `textBoxes`，该段有脚注或尾注引用时再跟 `footnoteRef` / `endnoteRef`（注 id 数组，按引用在该段内的文档顺序），该段落落在批注范围内时再跟 `commentIds`（批注 id 数组，按范围开始在文档中的顺序；同一段重叠多条都列出），给了 `scope.runs` 时再跟 `runs`。没有引用或范围则没有这些键。已展开的页眉页脚、框内、注内段落同样可以带 `commentIds`。不解析 `styles.xml` 继承的编号：样式上有 `numPr`、段上没有的，树上没有 `num` / `ilvl`。缺 `ilvl` 当作 `0`。`num` 是 `num:{numId}`。
- `scope.headerFooter` 为 `true` 时，每个页眉、页脚节点带 `paragraphs`；否则容器上没有该字段。页眉页脚段落节点不带 `pictureCount` / `pictures` / `textBoxCount` / `textBoxes` / `chartCount` / `charts` / `smartArtCount` / `smartArts` / `oleObjectCount` / `oleObjects`，图、框、图表、SmartArt 与嵌入对象仍挂在容器上。页眉页脚段落仍有 `align`、`indent`、`spacing`、`runCount`；给了 `scope.runs` 时再带 `runs`。
- `scope.pictures` 为 `true` 时，每个页眉、页脚节点带 `pictures`；若同时给了 `from` 与 `maxParas`，切片里每段也带 `pictures`。否则这些节点上都没有该字段。不把媒体字节放进树。每张图的字段顺序：`id`、`docPrId`、`name`、`kind`、`cx`、`cy`、`wrap`、`stable`。
- `scope.textBoxes` 为 `true` 时，每个页眉、页脚节点带 `textBoxes`；若同时给了 `from` 与 `maxParas`，切片里每段也带 `textBoxes`。否则这些节点上都没有该字段。只给 `textBoxes`、不给 `from` / `maxParas`：只展开页眉页脚容器上的框。每个框的字段顺序：`id`、`docPrId`、`name`、`kind`、`cx`、`cy`、`wrap`、`link`、`paraCount`、`pictureCount`、`chartCount`、`smartArtCount`、`stable`，有框内段名单时再跟 `paragraphs`，有框内图名单时再跟 `pictures`，给了 `scope.charts` 时再跟 `charts`，给了 `scope.smartArt` 时再跟 `smartArts`。`chartCount` / `smartArtCount` 始终出现，只计该框 `w:txbxContent` 内、再遇嵌套 `w:txbxContent` 停住之后的图表与 SmartArt。框内图表与 SmartArt 不挂到文档根，也不挂到宿主段落。
- `scope.notes` 为 `true` 时，根上带 `footnotes`、`endnotes`、`noteSeparators`。没有用户注时前两个为空数组；没有分隔符时 `noteSeparators` 为空数组。用户注按部件内文档顺序。每条用户注字段顺序：`id`、`noteId`、`type`、`host`、`paraCount`、`pictureCount`、`textBoxCount`、`chartCount`、`smartArtCount`，有图名单时再跟 `pictures`，有框名单时再跟 `textBoxes`，有图表名单时再跟 `charts`，有 SmartArt 名单时再跟 `smartArts`，然后 `paragraphs`。`noteId` 是 `w:id` 原文。`type` 恒为 `normal`。`host` 是引用所在段落 id 的数组：在主文档、页眉、页脚和文本框里找 `w:footnoteReference` / `w:endnoteReference`，按文档顺序，id 规则与该段落本身相同。一条注被引用多次则 `host` 有多项；没有任何引用则为 `[]`，注仍列出。`paraCount` / `pictureCount` / `textBoxCount` / `chartCount` / `smartArtCount` 只计该注元素内、遇 `w:txbxContent` 停住之后的段落、图、框、图表与 SmartArt，规则与正文相同。框内的图、图表与 SmartArt 不算进该注自己的这几项计数。注内段落字段顺序与正文段落相同，id 带注前缀。注内图、框、图表与 SmartArt 的字段顺序与宿主相同，id 带注前缀。
- `noteSeparators` 每项字段顺序：`id`、`kind`、`type`、`noteId`。`kind` 为 `footnote` 或 `endnote`。`type` 为 `separator`、`continuationSeparator` 或 `continuationNotice`。对 `noteSeparators` 里的 id 发 apply 报 `InvalidId`。
- `scope.comments` 为 `true` 时，根上带 `comments`。没有批注时为空数组。按 `comments.xml` 里 `w:comment` 的文档顺序。每条字段顺序：`id`、`commentId`、`author`、`initials`、`date`、`done`、`parent`、`kind`、`start`、`end`、`paraCount`、`pictureCount`、`textBoxCount`、`chartCount`、`smartArtCount`，有图名单时再跟 `pictures`，有框名单时再跟 `textBoxes`，有图表名单时再跟 `charts`，有 SmartArt 名单时再跟 `smartArts`，然后 `paragraphs`。`commentId` 是 `w:id` 原文。`author` / `initials` / `date` 取对应属性原文，缺则为 `null`。`done` 取 `commentEx/@done`：`1` / `true` / `on` 为 `true`，其余或没有 `commentEx` 为 `false`。`parent` 用 `paraIdParent` 对上父批注最后一段的 `w14:paraId`，再写成 `comment:{id}`；对不上或没有则为 `null`。根批注的 `parent` 为 `null`。`kind` 为 `normal`；批注内出现 `inkml:ink` 或等价墨迹标记则为 `ink`。`start` / `end` 是范围落在的段落 id（正文、页眉、页脚、框内或注内，规则与该段落本身相同）。从配对的 `commentRangeStart` 走到 `commentRangeEnd`，路径上经过的每个 `w:p` 都算在范围内；`start` 是第一个，`end` 是最后一个。回复有自己的范围标记时按自己的标记算；没有时沿用线程根的 `start` / `end`，并出现在那些段的 `commentIds` 里。找不到对应段（残缺标记）则该侧为 `null`，该批注仍列出，不报 `XmlBroken`。`paraCount` / `pictureCount` / `textBoxCount` / `chartCount` / `smartArtCount` 只计该 `w:comment` 内、遇 `w:txbxContent` 停住之后的段落、图、框、图表与 SmartArt，规则与正文相同。框内的图、图表与 SmartArt 不算进该批注自己的这几项计数。批注内段落字段顺序与正文段落相同，id 带批注前缀。批注内图、框、图表与 SmartArt 的字段顺序与宿主相同，id 带批注前缀。
- `scope.charts` 为 `true` 时，根上带 `charts`（正文），每个页眉、页脚容器带 `charts`；若同时给了 `scope.notes` / `scope.comments`，每条用户注或批注也带 `charts`（只含该注或批注段落上的，不含框内）。没有图表时为空数组。正文按主文档段落文档顺序，同一段内按 drawing 顺序。每张字段顺序：`id`、`docPrId`、`name`、`kind`、`cx`、`cy`、`wrap`、`type`、`title`、`catAxisTitle`、`valAxisTitle`、`series`、`legend`、`dataLabels`、`flags`、`stable`。`kind` / `cx` / `cy` / `wrap` / `docPrId` / `name` / `stable` 规则与图相同。`type` 为 `bar`、`col`、`line`、`pie`、`area`、`scatter`、`doughnut`、`radar`、`bubble`、`combo`、`chartex` 或 `other`。`barChart` / `bar3DChart` 看 `c:barDir`：`bar` 为 `bar`，`col` 或缺省为 `col`。`lineChart` / `line3DChart` 为 `line`，`pieChart` / `pie3DChart` 为 `pie`，`areaChart` / `area3DChart` 为 `area`，`scatterChart` 为 `scatter`，`doughnutChart` 为 `doughnut`，`radarChart` 为 `radar`，`bubbleChart` 为 `bubble`。`plotArea` 里出现两种及以上系列包裹（例如 `barChart` 加 `lineChart`）则为 `combo`。其余（股价、曲面、复合饼图、chartex 以外的扩展）为 `other`。`title` 取 `c:title` 可见文本（`c:tx` 的 `c:rich` 里 `a:t` 拼接，或 `c:strRef` 的 cache）；没有标题、或 `c:autoTitleDeleted/@val` 为真则为 `null`。`catAxisTitle` / `valAxisTitle` 取主分类轴、主数值轴的 `c:title`，规则与图表标题相同；没有则为 `null`。散点图的横轴当分类轴。`series` 按 `c:ser` 顺序。每条字段顺序：`id`、`index`、`name`、`categories`、`values`，气泡图再跟 `sizes`，cache 与 embedding 不一致时再跟 `cacheStale: true`。`index` 是从 0 起的出现序号。`name` 取系列 `c:tx` 的 cache 或字面量，缺则为 `null`。`categories` / `values` 取 `c:cat` / `c:val`（散点图、气泡图为 `c:xVal` / `c:yVal`）的 cache 或字面量；没有 cache 再读 embedding 里公式指向的单元格。气泡图的 `sizes` 取 `c:bubbleSize`，规则与 `values` 相同；没有则为 `[]`。非气泡系列不出现 `sizes`。全是数字则 JSON 数字，否则字符串。没有数据则为 `[]`。cache 与 embedding 都有且对不上时以 embedding 为准，并标 `cacheStale`。`legend` 为 `{ "show": bool, "pos": ... }`。没有 `c:legend` 则 `show` 为 `false`、`pos` 为 `null`；有则 `show` 为 `true`，`pos` 取 `c:legendPos/@val`（`b` / `t` / `l` / `r` / `tr`），缺属性则为 `null`。`dataLabels` 为 `{ "show": bool }`：`plotArea` 或任一系列带 `c:dLbls`，且其中至少一个 `c:showVal` / `c:showCatName` / `c:showSerName` / `c:showPercent` 为 `1` / `true` / `on` 时为 `true`，否则 `false`。`flags` 是字符串数组，可能含 `secondaryAxis`（出现 `c:overlay` 为 1 的轴，或系列 `c:axId` 指向次轴）、`trendline`（`c:trendline`）、`errorBars`（`c:errBars`）、`pivot`（`c:pivotSource` 或等价透视引用）。没有这些则为 `[]`。不把图表画成图，不读图表的渲染缓存图。xlsx / pptx 的 `scope` 里出现 `charts` 报 `InvalidField`。
- `scope.smartArt` 为 `true` 时，根上带 `smartArts`（正文），每个页眉、页脚容器带 `smartArts`；若同时给了 `scope.notes` / `scope.comments`，每条用户注或批注也带 `smartArts`（只含该注或批注段落上的，不含框内）。没有图示时为空数组。正文按主文档段落文档顺序，同一段内按 drawing 顺序。每张字段顺序：`id`、`docPrId`、`name`、`kind`、`cx`、`cy`、`wrap`、`layout`、`colors`、`style`、`points`、`stable`。`kind` / `cx` / `cy` / `wrap` / `docPrId` / `name` / `stable` 规则与图相同。`layout` 取 layout 部件根元素 `dgm:layoutDef/@uniqueId` 的原文；没有该部件或没有该属性则为 `null`。`colors` 取 colors 部件 `dgm:colorsDef/@uniqueId`；`style` 取 quickStyle 部件 `dgm:styleDef/@uniqueId`；缺则为 `null`。`points` 按 data 部件 `ptLst` 文档顺序，只含可见点（判定见上文 SmartArt id 那节）。每条字段顺序：`id`、`modelId`、`role`、`text`、`parent`。`modelId` 是 `dgm:pt/@modelId` 原文。`role` 为 `node` 或 `asst`（`type` 缺省当作 `node`）。`text` 取该点 `dgm:t` 里 `a:t` 按文档顺序拼接；`a:br` 变成 `\n`；没有 `dgm:t` 则为 `""`。`parent` 规则见上文。缺少 `r:dm` 或 data 部件解析失败时该图示仍列出，`points` 为 `[]`，`layout` / `colors` / `style` 能读则读、不能则为 `null`，不报 `XmlBroken`。不读 Fallback 位图，不把图示画成图。xlsx 的 `scope` 里出现 `smartArt` 报 `InvalidField`。
- `scope.tables` 为 `true` 时，根上带 `tables`（正文），每个页眉、页脚容器带 `tables`。没有表时为空数组。按该故事里 `w:tbl` 的文档顺序，嵌套表也列出。每张字段顺序：`id`、`index`、`rowCount`、`colCount`、`nested`、`rows`。`index` 与 id 里的 `{n}` 相同。`rowCount` 是 `w:tr` 个数。`colCount` 优先取 `tblGrid` 的 `gridCol` 个数；没有 `tblGrid` 则取第一行各格 `gridSpan` 之和（缺省 1）。`nested` 在该表的直接父元素是 `w:tc` 时为 `true`，否则为 `false`。`rows` 按 `w:tr` 顺序。每行字段顺序：`id`、`index`、`cells`。`cells` 按该行 `w:tc` 顺序，不是按空网格补格。每格字段顺序：`id`、`row`、`col`、`gridSpan`、`vMerge`、`paragraphs`。`row` / `col` 与 id 里的数字相同。`gridSpan` 为整数，缺元素则为 `1`。`vMerge` 为 `restart`、`continue` 或 `null`。`paragraphs` 是该格内、遇嵌套 `w:tbl` 停住之后的 `w:p`（嵌套表自己的段挂在那张嵌套表上）。段落字段顺序与正文段落相同，id 仍是 `para:…` / `para:index:…`（页眉页脚带容器前缀），`index` 是该故事全部 `w:p` 的序号。给了 `scope.pictures` 时，这些段落同样带 `pictureCount` / `pictures`。xlsx / pptx 的 `scope` 里出现 `tables` 报 `InvalidField`。`gridSpan` 或 `vMerge/@val` 不是正整数 / 不是 `restart` 或 `continue` 时报 `XmlBroken`。某行各格占的网格列之和对不上 `colCount` 时仍列出，结构 op 报 `NotImplemented`。
- `scope.revisions` 为 `true` 时，根上带 `revisions`（正文），每个页眉、页脚容器带 `revisions`。没有修订时为空数组。按该故事里被收入节点的文档顺序。每条字段顺序：`id`、`index`、`kind`、`xmlId`、`author`、`date`、`host`、`text`。`index` 与 id 里的 `{n}` 相同。`kind` 为 `ins`、`del`、`rPrChange` 或 `pPrChange`。`xmlId` 是 `w:id` 原文，缺则为 `null`。`author` / `date` 取对应属性原文，缺则为 `null`。`host` 是所在段落 id（正文 `para:…`，页眉页脚带容器前缀）；修订不在任何 `w:p` 里则为 `null`。`text`：`ins` 为该节点内可见 `w:t` 拼接；`del` 为 `w:delText`（以及其中若有的 `w:t`）拼接；属性修订为 `null`。xlsx / pptx 的 `scope` 里出现 `revisions` 报 `InvalidField`。
- `scope.hyperlinks` 为 `true` 时，根上带 `hyperlinks`（正文），每个页眉、页脚容器带 `hyperlinks`。没有链时为空数组。按该故事里被收入节点的文档顺序。每条字段顺序：`id`、`index`、`kind`、`rId`、`url`、`anchor`、`tooltip`、`host`、`text`。`kind` 为 `url`（有 `r:id`）或 `anchor`（没有 `r:id`）。`rId` 是关系 id 原文，缺则为 `null`。`url` 是该关系的 `Target` 原文，缺关系则为 `null`。`anchor` 是 `w:anchor` 原文，缺则为 `null`。有 `r:id` 时树上仍可带 `anchor`，但规范说跳转时忽略它。`tooltip` 取 `w:tooltip`，缺则为 `null`。`host` 是所在段落 id；不在任何 `w:p` 里则为 `null`。`text` 为该节点内可见 `w:t` 拼接。xlsx / pptx 的 `scope` 里出现 `hyperlinks` 报 `InvalidField`。
- `scope.bookmarks` 为 `true` 时，根上带 `bookmarks`（正文），每个页眉、页脚容器带 `bookmarks`。没有书签时为空数组。按该故事里被收入的 `w:bookmarkStart` 的文档顺序。每条字段顺序：`id`、`index`、`name`、`xmlId`、`start`、`end`。`name` 是 `w:name` 原文。`xmlId` 是 `w:id` 原文，缺则为 `null`。`start` / `end` 是起止所在段落 id；标记不在任何 `w:p` 里则为 `null`。xlsx / pptx 的 `scope` 里出现 `bookmarks` 报 `InvalidField`。
- `scope.fields` 为 `true` 时，根上带 `fields`（正文），每个页眉、页脚容器带 `fields`。没有域时为空数组。按该故事里被收入域起点的文档顺序。每条字段顺序：`id`、`index`、`kind`、`code`、`instr`、`host`、`end`、`text`。`kind` 为 `simple`（`w:fldSimple`）或 `complex`（配对 `w:fldChar`）。`instr` 是本层指令原文：简单域取 `w:instr`；复杂域拼接 begin 与 separate 之间（没有 separate 则到 end）本层的 `w:instrText`，不含 `w:delInstrText`，也不含嵌套域的指令。`code` 是 `instr` 去掉首尾空白后第一个词，按 ASCII 大写；公式域以 `=` 开头时 `code` 为 `"="`；没有词则为 `""`。`host` 是起点所在段落 id；`end` 是终点所在段落 id（简单域与 `host` 相同）；不在任何 `w:p` 里则为 `null`。`text` 是缓存结果的可见文本：简单域为元素内可见 `w:t`；复杂域为 separate 与 end 之间的可见 `w:t`（没有 separate 则为 `""`）。xlsx / pptx 的 `scope` 里出现 `fields` 报 `InvalidField`。
- `scope.tocs` 为 `true` 时，根上带 `tocs`。没有目录时为空数组。按主文档里被收入的 TOC / TOA 域起点的文档顺序。每条字段顺序：`id`、`index`、`kind`、`field`、`instr`、`host`、`end`、`text`。`id` 是 `toc:{n}`。`kind` 为 `toc`、`tof` 或 `toa`。`field` 是对应的正文域 id（`field:{m}`）。`instr` / `host` / `end` / `text` 与该域在 `fields` 里的同名字段相同。xlsx / pptx 的 `scope` 里出现 `tocs` 报 `InvalidField`。
- `scope.maths` 为 `true` 时，根上带 `maths`（正文），每个页眉、页脚容器带 `maths`。没有公式时为空数组。按该故事里顶层 `m:oMath` 的文档顺序。每条字段顺序：`id`、`index`、`kind`、`host`、`text`。`id` 是 `math:{n}` 或 `{容器}/math:{n}`。`kind` 为 `inline` 或 `display`。`host` 是所在段落 id；不在任何 `w:p` 里则为 `null`。`text` 为该 `oMath` 子孙里所有 `t`（即 `m:t`）按文档顺序拼接，不做 UnicodeMath 全量互转。xlsx / pptx 的 `scope` 里出现 `maths` 报 `InvalidField`。
- `scope.contentControls` 为 `true` 时，根上带 `contentControls`（正文），每个页眉、页脚容器带 `contentControls`。没有控件时为空数组。按该故事里被收入的 `w:sdt` 的文档顺序。每条字段顺序：`id`、`index`、`kind`、`level`、`host`、`tag`、`alias`、`xmlId`、`bound`、`parent`、`text`；`kind` 为 `dropDown` 或 `comboBox` 时再跟 `items`；`kind` 为 `date` 时再跟 `fullDate`。`id` 是 `sdt:{n}` 或 `{容器}/sdt:{n}`。`kind` 为 `plainText`、`richText`、`dropDown`、`comboBox`、`date` 或 `repeatingSection`。`level` 为 `inline` 或 `block`。`host` 是所在段落 id（行内为祖先 `w:p`；块级为 `sdtContent` 里第一个 `w:p`）；对不上则为 `null`。`tag` / `alias` 取 `w:tag` / `w:alias` 的 `w:val`，缺则为 `null`。`xmlId` 是 `w:sdtPr/w:id/@w:val` 原文，缺则为 `null`。`bound` 在有 `w:dataBinding` 时为 `true`，否则为 `false`。`parent` 是最近一层被收入的祖先控件 id，没有则为 `null`。`text` 为 `sdtContent` 子孙里所有 `w:t` 按文档顺序拼接。`items` 是 `w:listItem` 数组，每项字段顺序：`display`、`value`，取 `w:displayText` / `w:value`，缺则为 `null`。`fullDate` 取 `w:date/@w:fullDate` 原文，缺则为 `null`。xlsx / pptx 的 `scope` 里出现 `contentControls` 报 `InvalidField`。
- `scope.numbering` 为 `true` 时，根上带 `numberings`。没有实例时为空数组。按 `numbering.xml` 里 `w:num`（`numId` 非 0）的文档顺序。每条字段顺序：`id`、`numId`、`abstractId`、`abstractNumId`、`multiLevelType`、`levels`。`id` 是 `num:{numId}`。`abstractId` 是 `abstractNum:{abstractNumId}`。`multiLevelType` 取该实例指向的抽象定义上 `w:multiLevelType/@w:val`，缺则为 `null`。`levels` 按 `ilvl` 升序。每一级先取抽象定义上的 `w:lvl`，该实例上同 `ilvl` 的 `w:lvlOverride/w:lvl` 盖住对应级。每级字段顺序：`ilvl`、`numFmt`、`lvlText`、`start`、`lvlJc`。`numFmt` / `lvlText` / `lvlJc` 取对应子元素 `@w:val` 原文，缺则为 `null`。`start` 能解析成整数则写整数，否则为 `null`。inspect 列出 XML 原文，不把 `numFmt` 收成白名单。xlsx / pptx 的 `scope` 里出现 `numbering` 报 `InvalidField`。
- `scope.styles` 为 `true` 时，根上带 `styles`。没有样式时为空数组。按 `styles.xml` 里 `w:style`（`styleId` 非空）的文档顺序。每条字段顺序：`id`、`styleId`、`type`、`name`、`basedOn`、`default`。`id` 是 `style:{styleId}`。`styleId` 是 `w:styleId` 原文。`type` 取 `w:type` 原文，缺则为 `null`（规范缺省为段落样式，树上不猜）。`name` 取 `w:name/@w:val`，缺则为 `null`。`basedOn` 取 `w:basedOn/@w:val`，缺则为 `null`。`default` 看 `w:default`：`1` / `true` / `on` 为 `true`，其余或没有为 `false`。`w:latentStyles` 与 `w:docDefaults` 不进名单。xlsx / pptx 的 `scope` 里出现 `styles` 报 `InvalidField`。
- `align` 取该段自己的 `w:pPr/w:jc/@w:val`（§17.3.1.13），没有 `jc` 则为 `null`。不把 `styles.xml` 摊进来。
- `indent` 取该段自己的 `w:pPr/w:ind`（§17.3.1.12）。没有该元素则为 `null`。有则对象字段顺序：`left`、`right`、`firstLine`、`hanging`。`left` 取 `w:left`，没有则取 `w:start`。`right` 取 `w:right`，没有则取 `w:end`。能解析成非负整数则写整数，否则为 `null`。缺属性为 `null`。单位 twip。
- `spacing` 取该段自己的 `w:pPr/w:spacing`（§17.3.1.33）。没有该元素则为 `null`。有则对象字段顺序：`before`、`after`、`line`、`lineRule`。前三项能解析成非负整数则写整数，否则为 `null`。`lineRule` 取 `w:lineRule` 原文，缺则为 `null`。单位 twip。不计算自动行距。
- `runCount` 始终出现在已列出的段落上（正文切片、页眉页脚段落、框内、注内、批注内、表格单元格）。计该段内被收入的 `w:r` 个数，规则与 run id 相同。
- `scope.runs` 为 `true` 时，已列出的段落带 `runs`。每条字段顺序：`id`、`index`、`text`、`bold`、`italic`、`underline`、`sz`、`color`、`font`、`eastAsiaFont`。`id` 是 `{段落 id}/run:{n}`。`index` 与 `{n}` 相同。`text` 是该 run 内可见纯文本，规则与段上 `text` 相同，范围限于这个 `w:r`。`bold` / `italic` 只看该 run 自己的 `w:b` / `w:i`：没有元素为 `null`；有且 OnOff 为关为 `false`，否则为 `true`。`underline` 取 `w:u/@w:val`，没有 `u` 为 `null`，有元素却没有 `val` 则当作 `single`。`sz` 取 `w:sz/@w:val` 能解析成整数则写整数，否则为 `null`。`color` 取 `w:color/@w:val` 原文，缺则为 `null`；不解析 theme。`font` 取 `w:rFonts` 的 `ascii`，没有则取 `hAnsi`，再没有为 `null`。`eastAsiaFont` 取 `eastAsia`，缺则为 `null`。xlsx / pptx 的 `scope` 里出现 `runs` 报 `InvalidField`。
- `link` 始终出现。未链接则为 `{ "prev": null, "next": null }`。链接时 `prev` / `next` 是相邻框的 id，没有那一端则为 `null`。
- 框内 `paragraphs` 的字段顺序与正文段落相同（含 `pictureCount`）。框内图的字段顺序与宿主图相同，id 带框前缀。
- `kind`：`inline`（`wp:inline`）或 `anchor`（`wp:anchor`）。
- `cx` / `cy`：`wp:extent` 的值，单位 EMU，JSON 整数。缺则为 `null`。属性存在但不是整数报 `XmlBroken`。
- `wrap`：行内为 `null`；浮动取子元素 `wrapSquare` / `wrapTight` / `wrapThrough` / `wrapTopAndBottom` / `wrapNone`，对应 `square` / `tight` / `through` / `topAndBottom` / `none`。没有这些子元素则为 `null`。
- `docPrId` 取 `wp:docPr/@id` 的原文；没有该属性则为 `null`。`name` 取 `wp:docPr/@name` 的原文；没有则为 `null`。
- `stable`：有 `wp:docPr/@id` 为 `true`，否则为 `false`。
- `style` 取 `w:pStyle/@w:val` 的原文。没有 `pStyle` 则为 `null`（Word 此时按默认段落样式显示，树上不猜）。不把 `styles.xml` 里的字体、字号摊到段上。
- `text` 是可见纯文本：该 `w:p` 内、不在 `w:del` 里的 `w:t` 按顺序拼接；`w:tab` 变成 `\t`，`w:br` 变成 `\n`。`w:instrText`（域指令）不进入。不含 run 级样式。嵌在 `w:txbxContent` 里的 `w:t` 不算，`mc:AlternateContent` 只取一支，判定见上文 docx 那节的边界两条。
- `stable`：有 `w14:paraId` 为 `true`，否则为 `false`。
- 无 `scope.revisions` 时不返回修订名单，仍返回 `revisionCount`。无 `scope.hyperlinks` 时不返回超链接名单，仍返回 `hyperlinkCount`。无 `scope.bookmarks` 时不返回书签名单，仍返回 `bookmarkCount`。无 `scope.fields` 时不返回域名单，仍返回 `fieldCount`。无 `scope.tocs` 时不返回目录名单，仍返回 `tocCount`。无 `scope.maths` 时不返回公式名单，仍返回 `mathCount`。无 `scope.contentControls` 时不返回内容控件名单，仍返回 `contentControlCount`。无 `scope.numbering` 时不返回编号名单，仍返回 `numberingCount`。无 `scope.styles` 时不返回样式名单，仍返回 `styleCount`。无 `scope.runs` 时不返回 run 名单，已列出的段落仍返回 `runCount`。TOC / TOA 域同时出现在 `fields` 与 `tocs` 里。文本框里的段落不并进外层段落的 `text`；它们只出现在对应框的 `paragraphs` 里（需 `textBoxContent`）。含超链接或域或公式或行内内容控件的段落仍出现在树上，`text` 只含可见结果（不含 `w:instrText`）。apply 的 `setParaText` 验收只用无域、无超链接、无公式、无行内内容控件的页眉页脚段落。

### 响应（pptx）

```json
{
  "schemaVersion": 1,
  "format": "pptx",
  "id": "pptx:presentation",
  "slideCount": 2,
  "pictureCount": 0,
  "chartCount": 0,
  "smartArtCount": 0,
  "commentCount": 0,
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
  "slides": [
    {
      "id": "slide:1",
      "index": 1,
      "name": null,
      "hidden": false,
      "hasNotes": false,
      "pictureCount": 0,
      "chartCount": 0,
      "smartArtCount": 0,
      "commentCount": 0,
      "truncated": false,
      "shapes": [
        {
          "id": "slide:1/shape:2",
          "shapeId": "2",
          "name": "文本框 1",
          "text": "标题",
          "stable": true
        },
        {
          "id": "slide:1/shape:3",
          "shapeId": "3",
          "name": "文本框 2",
          "text": "hello",
          "stable": true
        }
      ]
    },
    {
      "id": "slide:2",
      "index": 2,
      "name": null,
      "hidden": false,
      "hasNotes": false,
      "pictureCount": 0,
      "chartCount": 0,
      "smartArtCount": 0,
      "commentCount": 0
    }
  ]
}
```

规则：

- 读 Presentation 部件、各页幻灯片及其 `.rels`（用 `.rels` 判断 `hasNotes`，用现代 comments 关系判断批注）。`scope.notes` 为 `true` 且该页有 notesSlide 关系时再读 `/ppt/notesSlides/…`。`scope.comments` 为 `true` 且该页有现代 comments 关系时再读该 comments 部件与 `/ppt/authors.xml`。讲义、母版、版式不读。备注里的图表与 SmartArt 本期不进树。批注内图本期不进树。
- `slideCount` 是 `p:sldIdLst` 里 `p:sldId` 的个数，始终出现。
- `pictureCount` 在演示文稿根上始终出现，等于各页幻灯片 `p:pic`（带 `a:blip`）之和，不含备注图。每页始终有自己的 `pictureCount`，同样只计该页幻灯片。
- `chartCount` 在演示文稿根上始终出现，等于各页被收入的图表之和。每页始终有自己的 `chartCount`。
- `smartArtCount` 在演示文稿根上始终出现，等于各页被收入的 SmartArt 之和。每页始终有自己的 `smartArtCount`。
- `commentCount` 在演示文稿根上始终出现，等于各页 `p188:cm` 之和，不含 `p188:reply`。每页始终有自己的 `commentCount`，只计该页。旧式 §19.4 批注不计。
- `hasNotes` 在每页始终出现：该页 `.rels` 有 Type 以 `/notesSlide` 结尾的关系则为 `true`，否则为 `false`。
- `slides` 始终列出全部页（含隐藏页）。未点名的页不带 `shapes`、`truncated`。
- `name` 取 `p:cSld/@name` 的原文。没有该属性则为 `null`。
- `hidden`：`p:sld/@show` 为 `0` 或 `false` 时为 `true`，其余为 `false`。
- 省略 `scope` 或只给空对象时，没有 `shapes`、`truncated`，也没有 `pictures` / `charts` / `smartArts` 名单，也没有 `notes`，也没有 `comments`。
- 给了 `scope.slide` 时，只给那一页填 `shapes`。该页带 `p:txBody` 的 `p:sp` 超过 `maxShapes` 则 `truncated` 为 `true`，按该页文档顺序截断。未截断则为 `false`。备注形状不算进 `maxShapes`。
- `scope.slide` 大于 `slideCount` 时，各页都不带 `shapes`，不报错。
- `scope.pictures` 为 `true` 时，每页带 `pictures`。没有图时为空数组。不依赖 `scope.slide`。每张字段顺序：`id`、`cNvPrId`、`name`、`x`、`y`、`cx`、`cy`、`stable`。`cNvPrId` 是 `p:cNvPr/@id` 原文，没有则为 `null`。`name` 取 `p:cNvPr/@name`，没有则为 `""`。`x` / `y` 取 `p:spPr/a:xfrm/a:off` 的 EMU；`cx` / `cy` 取 `a:ext`。缺对应属性则为 `null`。不读媒体字节。幻灯片图 id 为 `slide:{n}/pic:…`。
- `scope.charts` 为 `true` 时，每页带 `charts`。没有图表时为空数组。不依赖 `scope.slide`。每张字段顺序：`id`、`cNvPrId`、`name`、`x`、`y`、`cx`、`cy`、`type`、`title`、`catAxisTitle`、`valAxisTitle`、`series`、`legend`、`dataLabels`、`flags`、`stable`。`cNvPrId` / `name` / `stable` 规则与图相同。`x` / `y` / `cx` / `cy` 取 graphicFrame 的 `p:xfrm`。`type` / `title` / 轴标题 / `series` / `legend` / `dataLabels` / `flags` 规则与 docx 图表相同；系列值优先读 embedding 单元格，读不到再退回 cache。
- `scope.smartArt` 为 `true` 时，每页带 `smartArts`。没有图示时为空数组。不依赖 `scope.slide`。每张字段顺序：`id`、`cNvPrId`、`name`、`x`、`y`、`cx`、`cy`、`layout`、`colors`、`style`、`points`、`stable`。`cNvPrId` / `name` / `stable` 规则与图相同。`x` / `y` / `cx` / `cy` 取 graphicFrame 的 `p:xfrm`。`layout` / `colors` / `style` / `points` 规则与 docx SmartArt 相同。pptx 没有 `kind` / `wrap`。
- `scope.notes` 为 `true` 时，每页带 `notes`。没有 notesSlide 关系则为 `null`。有则为对象，字段顺序：`id`、`pictureCount`，有图名单时再跟 `pictures`，然后 `shapes`。`id` 为 `slide:{n}/notes`。`pictureCount` 只计该备注页上被收入的 `p:pic`，不并进幻灯片 `pictures`。备注形状判定与幻灯片形状相同（带 `p:txBody` 的 `p:sp`），id 为 `slide:{n}/notes/shape:…`。字段顺序与幻灯片形状相同：`id`、`shapeId`、`name`、`text`、`stable`。备注形状一次倒出，不用 `maxShapes`。
- `scope.comments` 为 `true` 时，每页带 `comments`。没有现代批注时为空数组。不依赖 `scope.slide`。按该页 `p188:cmLst` 里 `p188:cm` 的文档顺序。每条字段顺序：`id`、`commentId`、`author`、`initials`、`date`、`done`、`kind`、`x`、`y`、`text`、`replies`。`commentId` 是 `p188:cm/@id` 原文。`author` / `initials` 按 `authorId` 去 `/ppt/authors.xml` 的 `p188:author` 取 `name` / `initials`，对不上则为 `null`。`date` 取 `created` 原文，缺则为 `null`。`done`：`status` 为 `resolved` 或 `closed` 为 `true`；缺 `status`（即 active）为 `false`。`kind` 为 `normal`；该 `cm` 内出现墨迹标记（本地名为 `ink`）则为 `ink`。`x` / `y` 取 `p188:pos` 的 EMU，缺则为 `null`。`text` 取 `p188:txBody` 可见纯文本，规则与形状 `text` 相同。回复嵌在 `replies` 里，不单独占 `commentCount`。回复字段顺序：`id`、`commentId`、`author`、`initials`、`date`、`done`、`kind`、`text`（无 `x` / `y`）。回复 `id` 为 `{父批注 id}/reply:{guid}`。
- 页对象字段顺序：`id`、`index`、`name`、`hidden`、`hasNotes`、`pictureCount`、`chartCount`、`smartArtCount`、`commentCount`，然后按需 `pictures`，再按需 `charts`，再按需 `smartArts`，再按需 `notes`，再按需 `comments`，再按需 `truncated` 与 `shapes`。根对象字段顺序：`schemaVersion`、`format`、`id`、`slideCount`、`pictureCount`、`chartCount`、`smartArtCount`、`commentCount`、`props`、`slides`。`props` 始终出现，规则见下文「文档属性」。
- `shapeId` 是 `cNvPr/@id` 的原文。走 `shape:index:{k}` 时为 `null`。
- 形状上的 `name` 取 `cNvPr/@name` 的原文。没有则为 `null`。
- `text` 是可见纯文本：该形状 `p:txBody` 内、位于 `a:r` 里的 `a:t` 按文档顺序拼接；`a:br` 变成 `\n`。不含 run 级样式。`a:fld` 是应用程序更新的生成文本（§21.1.2.2.4），域里的 `a:t` 是缓存，不进入。`mc:AlternateContent` 只取一支，判定见上文 pptx 那节。
- `stable`：有 `cNvPr/@id` 为 `true`，否则为 `false`。
- 第一期不返回动画、切换、讲义、母版、版式。批注内图本期不进树。对批注 id 发 `insertPicture` 报 `InvalidId`。

### 文档属性（三格式共用）

xlsx、docx、pptx 的根上始终带 `props`，不要另给 scope 键。core / app / custom 是包根关系指向的部件（ECMA-376 §15.2.12，常见 `/docProps/core.xml`、`/docProps/app.xml`、`/docProps/custom.xml`），三种格式同一套字段名。没有某部件时，对应字段为 `null`，`custom` 为空数组，不报错。

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

`custom` 按文档顺序。没有 custom 部件或没有 `property` 时为 `[]`。每条字段顺序：`id`、`name`、`type`、`value`。`id` 是 `prop:{name}`，`{name}` 取 `property/@name` 原文。`name` 为空的条目不进树。`type` 是值子元素的 local name，常见 `lpwstr`、`i4`、`r8`、`bool`、`filetime`、`date`。`value`：`lpwstr` 为字符串；`i4` / `int` 能解析成整数则写整数；`r8` / `r4` 能解析成数字则写数字；`bool` 为 JSON 布尔（`true` / `1` / `on` 为 `true`）；其余把子孙文本拼成字符串。带 `linkTarget` 的条目仍列出缓存值，树上不另给链接字段。同一 `name` 出现两次时都列出，apply 命中文档顺序第一处。

不把 `Application`、`Pages`、`Words`、`Slides`、`HeadingPairs`、`TitlesOfParts`、`DocSecurity`、数字签名或缩略图放进树。这些未点名时 apply 原样留下。

### 文档保护（仅 docx）

docx 根上始终带 `protection`，不要另给 scope 键。来源是 `/word/settings.xml` 的 `w:documentProtection`（ECMA-376 §17.15.1.29）与 `w:writeProtection`（§17.15.1.93）。没有 settings 部件时按全关处理，不报错。xlsx / pptx 不要带此对象。

`protection` 字段顺序：`enforced`、`edit`、`formatting`、`hasPassword`、`readOnlyRecommended`。

| 字段 | 来源 | 说明 |
| --- | --- | --- |
| `enforced` | `documentProtection/@enforcement` | `ST_OnOff` 为 on / 1 / true 时为 `true`，缺元素或未强制为 `false` |
| `edit` | `documentProtection/@edit` | `readOnly`、`comments`、`trackedChanges` 或 `forms`。缺元素、省略或 `none` 时为 `null`。不认识的取值报 `XmlBroken` |
| `formatting` | `documentProtection/@formatting` | 限制样式与直接格式。缺或未开为 `false` |
| `hasPassword` | 两元素上的哈希属性 | `documentProtection` 或 `writeProtection` 带非空 `hash` / `hashValue` / `salt` / `saltValue` / `reservationPassword` 时为 `true`。树上不出现哈希、盐或明文 |
| `readOnlyRecommended` | `writeProtection/@recommended` | 为 on 且该元素没有密码哈希时为 `true`。有写保护密码时规范要求忽略 recommended，树上为 `false` |

不把 `algorithmName`、`cryptProviderType`、`cryptAlgorithmSid`、`spinCount` 放进树。不列出 `w:permStart` / `w:permEnd`。不报告 IRM。

## create

只有 create 允许从 JSON 生成整包。

### xlsx

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
          { "ref": "A1", "value": 1 },
          { "ref": "A2", "value": "hello" },
          { "ref": "A3", "value": true },
          { "ref": "B1", "formula": "=SUM(A1)" }
        ],
        "pictures": [
          { "file": "C:\\\\img\\\\a.png", "from": "B2" },
          { "file": "C:\\\\img\\\\a.png", "from": "D2", "to": "F8", "anchor": "twoCell" }
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
| `workbook.props` | 可选。文件属性，字段与下文 `setProps` / `setCustomProp` 相同 |
| `sheets[].name` | 工作表名。Excel 限制：最长 31 个字符，不含 `:*?/[]\\`，不能叫 `History`，同一工作簿内不重复 |
| `sheets[].pictures` | 可选。该表上的图列表。省略或空数组则不写图 |
| `sheets[].charts` | 可选。该表上的图表列表。省略或空数组则不写图表 |
| `cells[].ref` | A1 地址 |
| `cells[].value` | 数字、字符串、布尔、或 `null`。`null` 表示不写这个格 |
| `cells[].formula` | 以 `=` 开头的公式文本。库不求值 |

| `pictures[].file` | 本地路径，与 `insertPicture` 相同，按文件头认 PNG / JPEG / GIF / BMP。缺此字段报 `MissingField` |
| `pictures[].from` | 必须。左上角单元格，A1。缺此字段报 `MissingField`。不是合法 A1 报 `InvalidRange` |
| `pictures[].to` | `anchor` 为 `twoCell` 时必须。右下角单元格，A1 |
| `pictures[].anchor` | 可选。`oneCell`（缺省）或 `twoCell`。不认 `absolute` |
| `pictures[].cx` / `pictures[].cy` | 可选，EMU，正整数。都省则按图片像素以 96 dpi 换算。只给一个时报 `InvalidField`。`twoCell` 也要写 `xdr:pic` 里的 `a:ext`，显示尺寸仍由起止格决定 |
| `pictures[].fromOffX` / `fromOffY` / `toOffX` / `toOffY` | 可选。与 `insertPicture` 相同 |
| `charts[].type` | 必须。与 `insertChart` 相同 |
| `charts[].series` | 必须。非空。字段与 `insertChart` 相同 |
| `charts[].from` | 必须。图的左上角单元格，A1 |
| `charts[].dataFrom` | 必须。数据表左上角，A1。按 `setCell` 语义覆盖已有格子，包括同一张表 `cells[]` 里已声明的值 |
| `charts[].title` | 可选 |
| `charts[].to` / `anchor` / `cx` / `cy` / 偏移 | 与 `insertChart` 相同。`kind` / `wrap` 报 `InvalidField` |

`value` 和 `formula` 不能在同一格同时出现。要公式就只写 `formula`。公式格不写缓存 `<v>`，并在 `workbook.xml` 的 `calcPr` 上设 `fullCalcOnLoad="1"`，让 Excel 打开时重算。

文本写入共享字符串表（`t="s"`），这是 Office 新建文件的常见写法。每个文本值必须先进入该表再写索引；表里找不到对应项时 create 失败，不许退回索引 0。布尔写成 `t="b"`。数字不写 `t`（默认为 `n`）。

行与单元格按行号、列号升序写出。补上 `sheetData` 和合理的 `dimension`。

最小 `styles.xml` 必须生成，见 [opc.md](opc.md)。默认字体颜色写 `<color rgb="FF000000"/>`，不引用 theme，包里也不生成 theme 部件。给了 `sheets[].pictures` 时按 `insertPicture` 同一套规则写 drawing 与 media。给了 `sheets[].charts` 时按 `insertChart` 同一套规则写 `/xl/charts/chartN.xml`、`xdr:graphicFrame` 和 `dataFrom` 一带单元格，不写 embedding。一张表上的图与图表共用一份 drawing：先写 `pictures`，再写 `charts`。`xdr:cNvPr/@id` 取该表已有全部 `cNvPr` 的最大整数加 1。不要生成透视表、条件格式、图表工作表。`workbook` 根上或 `cells[]` 上出现 `pictures` / `charts`，或 `sheets[]` 上出现 `kind` / `wrap` / `smartArts`，报 `InvalidField`。`sheets[].kind` 仍拒绝。给了 `workbook.props` 时按 `setProps` / `setCustomProp` 同一套规则写 core / app / custom；省略则仍写空的 `dc:title` 与 `dc:creator` 为 `ooxp`，不写 custom 部件。

### docx

```json
{
  "schemaVersion": 1,
  "action": "create",
  "path": "C:\\out\\note.docx",
  "format": "docx",
  "document": {
    "paragraphs": [
      { "style": "Heading1", "text": "标题" },
      { "text": "hello", "pictures": [{ "file": "C:\\\\img\\\\a.png" }] }
    ],
    "headers": [
      {
        "type": "default",
        "paragraphs": [
          { "text": "页眉", "pictures": [{ "file": "C:\\\\img\\\\a.png", "kind": "anchor", "wrap": "square" }] }
        ]
      }
    ]
  }
}
```

带图表、注、批注与两节的示例见 `docs/examples/docx-create-full.json`。

| 字段 | 含义 |
| --- | --- |
| `path` | 输出路径。已存在则覆盖。省略时必须由调用方另给落盘路径，命令行用 `--output` |
| `document.paragraphs` | 段落列表。可空；空则生成一个空的 `w:p`，否则 Word 可能打不开 |
| `paragraphs[].text` | 可见文本，可空字符串。省略当作 `""` |
| `paragraphs[].style` | 段落样式 id。省略或 `Normal` 表示正文。第一期只允许 `Normal`、`Heading1`、`Heading2`、`Heading3`（大小写不敏感，写出时用这里的 canonical 形式） |
| `paragraphs[].sectionType` | 可选。该段是非末节的末段，取值与 `insertSection` 的 `type` 相同。在该段 `pPr` 写出 `w:sectPr`（含 `w:type` 与默认 `pgSz` / `pgMar`）。最后一段禁止带此字段，否则报 `InvalidField`。省略则该段不是分节末段。页眉、页脚段落不允许此字段 |
| `paragraphs[].textBoxes` | 可选。该段上的文本框列表。省略则不写框 |
| `paragraphs[].pictures` | 可选。该段上的图列表。省略则不写图 |
| `paragraphs[].smartArts` | 可选。该段上的 SmartArt 列表。省略则不写图示。字段与 `insertSmartArt` 相同，但不含 `id` |
| `paragraphs[].charts` | 可选。该段上的图表列表。省略则不写图表。字段与 `insertChart` 相同，但不含 `op` / `id`，也不含 `from` / `to` / `anchor` / `dataFrom` / `x` / `y`。出现这些 xlsx / pptx 字段报 `InvalidField`。缺省与 `insertChart` 相同：`kind` 为 `anchor`，`wrap` 为 `square`。页眉页脚段落同样允许 |
| `paragraphs[].oleObjects` | 可选。该段上的嵌入对象列表。省略则不写。每项只要 `file`，规则与 `insertOle` 相同。页眉页脚段落同样允许 |
| `paragraphs[].footnote` | 可选。字符串。在该段末插入一条脚注，规则与 `insertFootnote` 的 `text` 相同，可空。省略则不写。页眉页脚段落同样允许 |
| `paragraphs[].endnote` | 可选。字符串。在该段末插入一条尾注，规则与 `insertEndnote` 相同。可与 `footnote` 同时出现。页眉页脚段落同样允许 |
| `paragraphs[].comments` | 可选。该段上的批注列表。每条范围的 start 与 end 都是该段自身，不跨段。不写回复。页眉页脚段落同样允许 |
| `document.headers` / `document.footers` | 可选。页眉或页脚。省略则不写对应部件 |
| `document.props` | 可选。文件属性，字段与下文 `setProps` / `setCustomProp` 相同 |
| `document.protection` | 可选。文档保护，字段与下文 `setProtection` 相同（不要 `id`，不要 `hasPassword`） |
| `document.tables` | 可选。正文表列表。省略或空数组则不写 `w:tbl` |
| `headers[].type` / `footers[].type` | `default`、`even` 或 `first`；省略当作 `default`。同一侧同一 `type` 只能出现一次，否则报 `InvalidField`。给了 `even` 则写入整篇 `w:evenAndOddHeaders`；给了 `first` 则第一节写入 `w:titlePg` |
| `headers[].paragraphs` / `footers[].paragraphs` | 可选。空或省略则写一个空的 `w:p`。字段与正文段落相同，但不允许 `style`、不允许 `textBoxes`、不允许 `sectionType`；允许 `pictures`、`smartArts`、`charts`、`oleObjects`、`footnote`、`endnote`、`comments` |
| `headers[].tables` / `footers[].tables` | 可选。该部件里的表，规则与 `document.tables` 相同，`after` 相对该部件的 `paragraphs` |

| `tables[].cells` | 必须。二维字符串数组。至少一行一列，至多 500 行、63 列（Word 列上限）。每一行的长度必须相同，否则报 `InvalidField`。每个字符串是该格第一段可见文本，可空 |
| `tables[].after` | 可选。非负整数。表插在第 `after` 个段落之后（段落按下标从 1 计）；`0` 表示插在第一段之前。省略则插在全部段落之后。超过段落个数报 `InvalidField`。多张表 `after` 相同则按数组顺序相邻 |

| `textBoxes[].text` | 框内第一段可见文本，可空。省略当作 `""` |
| `textBoxes[].cx` | 可选。宽度，EMU。省略为 `1828800` |
| `textBoxes[].cy` | 可选。高度，EMU。省略为 `914400` |

| `pictures[].file` | 本地路径，与 `insertPicture` 相同，按文件头认 PNG / JPEG / GIF / BMP。缺此字段报 `MissingField` |
| `pictures[].cx` / `pictures[].cy` | 可选，EMU，正整数。都省则按图片像素以 96 dpi 换算。只给一个时报 `InvalidField` |
| `pictures[].kind` | 可选。`inline` 或 `anchor`，省略为 `inline` |
| `pictures[].wrap` | `kind` 为 `anchor` 时必填，枚举与 `setPictureWrap` 相同。`kind` 为 `inline` 时必须省略 |
| `pictures[].alignH` / `alignV` / `offsetX` / `offsetY` | 仅浮动，规则与 `setPictureWrap` 相同 |

| `smartArts[].layout` | 必须。`list` 或 `org`，与 `insertSmartArt` 相同 |
| `smartArts[].points` | 可选。与 `insertSmartArt` 相同；省略则用该版式的缺省占位点 |
| `smartArts[].kind` / `wrap` / `cx` / `cy` | 可选。与 `insertSmartArt` 相同 |

| `charts[].type` | 必须。与 `insertChart` 相同 |
| `charts[].series` | 必须。非空。字段与 `insertChart` 相同 |
| `charts[].title` | 可选 |
| `charts[].kind` / `wrap` / `cx` / `cy` | 可选。与 docx `insertChart` 相同 |

| `comments[].text` | 可选。新批注第一段可见文本，省略当作 `""` |
| `comments[].author` | 可选。省略当作 `"ooxp"` |
| `comments[].initials` | 可选。省略当作 `"oo"` |
| `comments[].date` | 可选。`xsd:dateTime` 字符串。省略则不写 `w:date` |

不把 base64 写进 create JSON。xlsx 的图只认 `workbook.sheets[].pictures`，图表只认 `workbook.sheets[].charts`。pptx 的图只认 `presentation.slides[].pictures`，图表只认 `presentation.slides[].charts`，SmartArt 只认 `presentation.slides[].smartArts`。`presentation` 根上或 `shapes[]` 上出现 `pictures` / `charts` / `smartArts` 报 `InvalidField`。xlsx 上出现 `smartArts` 同样报 `InvalidField`。

create 写出 Transitional 最小包，见 [opc.md](opc.md)。每个 `w:p` 都带唯一的 `w14:paraId` 和 `w14:textId`（取值大于 0、小于 `0x80000000`，MS-DOCX §2.6.2.3 / §2.6.2.4），并在根元素上把 `w14` 列入 `mc:Ignorable`。这样随后 inspect 得到的 id 都是 `para:{paraId}`，`stable` 为 `true`。图的 `wp:docPr/@id` 在该部件内从 `1` 起递增，与 `insertPicture` 同一套写入。

标题段写出 `<w:pStyle w:val="Heading1"/>` 这类引用，样式定义放在 `/word/styles.xml`。同时写 `/word/settings.xml`，含 `<w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/>`（MS-DOCX §2.3.5）。缺这项时 Word 按兼容模式打开（默认 12）。`textBoxes` 写成浮动 `square` 的 `wps:wsp`，与 `insertTextBox` 同一套最小 Choice。`smartArts` 复用 `insertSmartArt` 的四件套生成。给了 `headers` / `footers` 时按出现顺序写 `/word/headerN.xml` 或 `/word/footerN.xml`，并在第一节 `sectPr` 上挂对应 `w:type` 的引用。若第一节在某段上结束，引用写在该段的 `sectPr` 上，不要留在 `w:body`。给了 `even` 则 `settings.xml` 含 `<w:evenAndOddHeaders/>`。给了 `first` 则第一节 `sectPr` 含 `<w:titlePg/>`。给了 `paragraphs[].sectionType` 时写出对应中间节；末节的 `sectPr` 仍在 `w:body` 末尾。给了 `paragraphs[].charts` 时按 `insertChart` 同一套规则写 `/word/charts/…`、embedding 与宿主 `w:drawing`。给了 `paragraphs[].oleObjects` 时按 `insertOle` 同一套规则写 embeddings、预览图与 `w:object`。给了 `footnote` / `endnote` 时按对应 insert 同一套规则写注部件、分隔符与引用 run。给了 `comments` 时按 `insertComment` 同一套规则写 `comments.xml`、`commentsExtended.xml`、`people.xml` 与范围标记。同一段上先写骨架里的文字 / 图 / 框 / SmartArt，再图表，再 OLE，再脚注，再尾注，再批注。不要生成目录、修订、超链接、书签、域、编号、公式、内容控件，也不写批注回复。给了 `document.props` 时按 `setProps` / `setCustomProp` 同一套规则写 core / app / custom；省略则仍写空的 `dc:title` 与 `dc:creator` 为 `ooxp`，不写 custom 部件。给了 `document.protection` 时按 `setProtection` 同一套规则写 `settings.xml`，窗体保护还可写各节 `w:formProt`。默认页眉页脚带框不在本期 create。给了 `tables` 时写出带单线边框的 `w:tbl`，每列 `gridCol` 宽 1440 twip，每格一段，带新的 `w14:paraId`。不写 `tblStyle`。不算列宽。框内、表内、注内本期 create 不写图表或注。

### pptx

```json
{
  "schemaVersion": 1,
  "action": "create",
  "path": "C:\\out\\deck.pptx",
  "format": "pptx",
  "presentation": {
    "slides": [
      {
        "shapes": [
          { "text": "标题" },
          { "text": "hello" }
        ]
      },
      {
        "shapes": [
          { "text": "第二页" }
        ]
      }
    ]
  }
}
```

| 字段 | 含义 |
| --- | --- |
| `path` | 输出路径。已存在则覆盖。省略时必须由调用方另给落盘路径，命令行用 `--output` |
| `presentation.slides` | 页列表。可空；空则生成一页，页上有一个空文本框，否则 PowerPoint 可能打不开 |
| `presentation.props` | 可选。文件属性，字段与下文 `setProps` / `setCustomProp` 相同 |
| `slides[].shapes` | 该页文本框列表。可空；空则生成一个空文本框 |
| `slides[].pictures` | 可选。该页上的图列表。省略或空数组则不写 `p:pic` |
| `slides[].charts` | 可选。该页上的图表列表。省略或空数组则不写图表 `p:graphicFrame` |
| `slides[].smartArts` | 可选。该页上的 SmartArt 列表。省略或空数组则不写图示 `p:graphicFrame` |
| `shapes[].text` | 可见文本，可空字符串。省略当作 `""`。`\n` 写成 `a:br` |
| `pictures[].file` | 本地路径，与 `insertPicture` 相同，按文件头认 PNG / JPEG / GIF / BMP。缺此字段报 `MissingField` |
| `pictures[].x` / `pictures[].y` | 可选，EMU，非负整数。必须成对出现或成对省略；只给一个时报 `InvalidField`。缺则放到该页空白处，规则与 pptx `insertPicture` 相同 |
| `pictures[].cx` / `pictures[].cy` | 可选，EMU，正整数。都省则按图片像素以 96 dpi 换算。只给一个时报 `InvalidField` |
| `charts[].type` | 必须。与 pptx `insertChart` 相同 |
| `charts[].series` | 必须。非空。字段与 `insertChart` 相同 |
| `charts[].title` | 可选 |
| `charts[].x` / `charts[].y` / `charts[].cx` / `charts[].cy` | 可选。规则与 pptx `insertChart` 相同 |
| `smartArts[].layout` | 必须。`list` 或 `org`，与 pptx `insertSmartArt` 相同 |
| `smartArts[].points` | 可选。与 `insertSmartArt` 相同；省略则用该版式的缺省占位点 |
| `smartArts[].x` / `smartArts[].y` / `smartArts[].cx` / `smartArts[].cy` | 可选。规则与 pptx `insertSmartArt` 相同 |

带图示例见 `docs/examples/pptx-create-pictures.json`。带图表示例见 `docs/examples/pptx-create-charts.json`。带 SmartArt 示例见 `docs/examples/pptx-create-smartart.json`。`kind` / `wrap` / `from` / `to` / `anchor` / `dataFrom` 以及 xlsx 偏移字段出现报 `InvalidField`。

`workbook`、`document`、`presentation` 三者必须恰好出现一个。xlsx 路径必须用 `workbook`，docx 路径必须用 `document`，pptx 路径必须用 `presentation`，对不上报 `InvalidField`。

create 写出 Transitional 最小包，见 [opc.md](opc.md)。必须带 theme、slideMaster、slideLayout、presentation、各页 slide、presProps，不能只写 ECMA 示例里那个仅含 `notesSz` 的 presentation。幻灯片尺寸用 16:9（`sldSz` 为 `cx="12192000"` `cy="6858000"`）。不要生成动画、切换、备注、批注。给了 `slides[].pictures` 时按 `insertPicture` 同一套规则写 `/ppt/media/…`、该页 `.rels` 与 `p:pic`。给了 `slides[].charts` 时按 `insertChart` 同一套规则写 `/ppt/charts/…`、embedding 与图表 `p:graphicFrame`。给了 `slides[].smartArts` 时按 `insertSmartArt` 同一套规则写 `/ppt/diagrams/…` 与图示 `p:graphicFrame`。先写该页 `shapes`，再按数组顺序写 `pictures`，再写 `charts`，再写 `smartArts`。`p:cNvPr/@id` 取该页已有全部 `cNvPr` 的最大整数加 1，至少为 2。`presentation` 根上或 `shapes[]` 上出现 `pictures` / `charts` / `smartArts` 报 `InvalidField`。给了 `presentation.props` 时按 `setProps` / `setCustomProp` 同一套规则写 core / app / custom；省略则仍写空的 `dc:title` 与 `dc:creator` 为 `ooxp`，不写 custom 部件。

每个文本框写成带 `p:txBody` 的 `p:sp`，`p:cNvPr/@id` 从 `2` 起在该页内递增（`1` 留给 `spTree`）。随后 inspect 得到的 id 都是 `slide:{n}/shape:{id}`，`stable` 为 `true`。形状纵向错开，避免叠在同一位置。颜色用 RGB，不依赖主题色语义。

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
    { "op": "setCell", "id": "cell:Sheet1!A1", "value": 42 },
    { "op": "setCell", "id": "cell:Sheet1!B1", "formula": "=SUM(A1:A10)" },
    { "op": "setCell", "id": "cell:Sheet1!C1", "value": null }
  ]
}
```

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `path` | 是 | 源文件 |
| `output` | 否 | 省略则覆盖 `path` |
| `ops` | 是 | 非空数组。按数组顺序执行，中途失败则整次 apply 不写文件。空数组不是原样保存，报 `EmptyArray`。零脏写回只在 OPC 层，见 [opc.md](opc.md) |

create 与 apply 落盘时先写入目标同目录的临时文件，再改名为目标路径。这样写到一半失败时，原文件还在。

### setCell

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setCell"` |
| `id` | 单元格 id |
| `value` | 与 create 相同。`null` 表示清空该格的值和公式 |
| `formula` | 以 `=` 开头。不能与 `value` 同时出现 |

不认识的 `op` 报 `UnsupportedOp`，不要忽略。xlsx 认 `setCell`、`setHeaderFooterText`、`replacePicture`、`insertPicture`、`deletePicture`、`setPictureSize`、`setPictureAnchor`、以及图表 op（`insertChart`、`insertChartSheet`、`deleteChartSheet`、`deleteChart`、`setChartTitle`、`setChartSeries`、`addChartSeries`、`deleteChartSeries`、`setChartType`、`setChartAxisTitle`、`setChartLegend`、`setChartDataLabels`、`setChartSize`、`setChartAnchor`；不含 `setChartWrap`）、以及 `setProps` / `setCustomProp` / `deleteCustomProp`。docx 认 `setParaText`、`setHeaderFooterDistance`、`setPageMargin`、`setPageSize`、`setColumns`、`setVerticalAlign`、`setLineNumbering`、`setDocGrid`、`setRtlGutter`、`setMirrorMargins`、`setGutterAtTop`、`setHeaderFooterLink`、`insertHeaderFooter`、`setTitlePage`、`setEvenAndOddHeaders`、图片 op（含 `setPictureWrap`，不含 `setPictureAnchor`）、文本框 op、`insertPara` / `deletePara`、表格 op（`insertTable`、`deleteTable`、`insertRow`、`deleteRow`、`insertCol`、`deleteCol`、`mergeCells`、`splitCells`）、修订 op（`acceptRevision`、`rejectRevision`）、超链接与书签 op（`insertHyperlink`、`setHyperlink`、`deleteHyperlink`、`insertBookmark`、`setBookmark`、`deleteBookmark`）、域 op（`insertField`、`setField`、`deleteField`）、目录 op（`insertToc`、`deleteToc`）、公式 op（`insertMath`、`setMath`、`deleteMath`）、内容控件 op（`insertContentControl`、`setContentControl`、`deleteContentControl`）、编号 op（`insertNumbering`、`setNumbering`、`setParaNumbering`）、样式 op（`setStyle`、`setParaStyle`）、直接格式 op（`setParaFormat`、`setRunFormat`）、节结构 op（`insertSection`、`deleteSection`）、脚注尾注 op（`insertFootnote`、`insertEndnote`、`deleteFootnote`、`deleteEndnote`、`setNoteNumbering`）、批注 op（`insertComment`、`deleteComment`、`replyComment`、`setCommentText`、`setCommentMeta`、`setCommentDone`）、图表 op（`insertChart`、`deleteChart`、`setChartTitle`、`setChartSeries`、`addChartSeries`、`deleteChartSeries`、`setChartType`、`setChartAxisTitle`、`setChartLegend`、`setChartDataLabels`、`setChartSize`、`setChartWrap`；不含 `setChartAnchor`、`insertChartSheet`、`deleteChartSheet`）、以及 SmartArt op（`insertSmartArt`、`deleteSmartArt`、`setSmartArtText`、`addSmartArtPoint`、`deleteSmartArtPoint`、`setSmartArtSize`、`setSmartArtWrap`、`setSmartArtColors`、`setSmartArtStyle`、`setSmartArtLayout`）、以及 OLE op（`insertOle`、`deleteOle`）、以及 `setProtection`、以及 `setProps` / `setCustomProp` / `deleteCustomProp`。pptx 认 `setShapeText`、`setNotesText`、批注 op（`insertComment`、`deleteComment`、`replyComment`、`setCommentText`、`setCommentMeta`、`setCommentDone`）、`replacePicture`、`insertPicture`、`deletePicture`、`setPictureSize`、`setPicturePos`、以及图表 op（`insertChart`、`deleteChart`、`setChartTitle`、`setChartSeries`、`addChartSeries`、`deleteChartSeries`、`setChartType`、`setChartAxisTitle`、`setChartLegend`、`setChartDataLabels`、`setChartSize`、`setChartPos`；不含 `setChartWrap`、`setChartAnchor`、`insertChartSheet`、`deleteChartSheet`）、以及 SmartArt op（`insertSmartArt`、`deleteSmartArt`、`setSmartArtText`、`addSmartArtPoint`、`deleteSmartArtPoint`、`setSmartArtSize`、`setSmartArtPos`、`setSmartArtColors`、`setSmartArtStyle`、`setSmartArtLayout`；不含 `setSmartArtWrap`）、以及 `setProps` / `setCustomProp` / `deleteCustomProp`。对不上格式的 op（例如对 pptx 发 `setCell` / `setParaText`，对 xlsx 发 `setPictureWrap`、`setChartWrap` 或 `setHeaderFooterDistance` / `setPageMargin` / `setPageSize` / `setColumns` / `setVerticalAlign` / `setLineNumbering` / `setDocGrid` / `setRtlGutter` / `setMirrorMargins` / `setGutterAtTop` / `setHeaderFooterLink` / `insertHeaderFooter` / `setTitlePage` / `setEvenAndOddHeaders`、文本框、注、批注、SmartArt 的 op、OLE 的 op、`setProtection`、表格 op、修订 op、超链接与书签 op、域 op、编号 op、样式 op、直接格式 op、节结构 op 或 `setNotesText`，对 docx / pptx 发 `setHeaderFooterText`、`insertChartSheet` 或 `deleteChartSheet`，对 docx 发 `setPictureAnchor` / `setChartAnchor` / `setSmartArtPos` / `setNotesText`，对 pptx 发 `setPictureWrap` / `setPictureAnchor` / `setChartWrap` / `setChartAnchor` / `setSmartArtWrap`，或对 docx / xlsx 发 `setPicturePos` / `setChartPos` / `setSmartArtPos` / `setNotesText`）报 `InvalidField`。

### setProps

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setProps"` |
| `id` | 必须是格式根：`xlsx:workbook`、`docx:document` 或 `pptx:presentation` |
| `title` / `subject` / `creator` / `description` / `keywords` / `category` / `company` / `manager` | 可选。字符串或 `null`。`null` 删掉对应 XML 元素；空串写空元素 |

这八个字段至少给一个，否则报 `InvalidField`。未点名的字段留下，含 `Application`、页数/字数、`HeadingPairs`、`TitlesOfParts`、`Slides`，以及未点名的公司名。不要写 `DigSig`。不要改 `lastModifiedBy`、`created`、`modified`。出现这些只读键报 `UnknownField`。id 不是格式根报 `InvalidId`。缺 `core.xml` 或 `app.xml` 时，只要这条 op 点到了对应部件里的字段，就按需新建部件、根关系和 Content Type Override。已经是目标状态则空操作，仍计入 `opsApplied`。

### setCustomProp

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setCustomProp"` |
| `id` | 必须是格式根 |
| `name` | 必须。非空字符串，对应 `property/@name` |
| `value` | 必须。JSON 字符串、数字或布尔 |
| `type` | 可选。`lpwstr`、`i4`、`r8` 或 `bool`。省略则按 `value` 的 JSON 类型推断：字符串为 `lpwstr`，布尔为 `bool`，能放进 32 位整数的数字为 `i4`，其余数字为 `r8` |

按名 upsert：已有同名 `property` 时命中文档顺序第一处，保留原来的 `pid` / `fmtid`，只换值子元素。没有则追加，`fmtid` 固定 `{D5CDD505-2E9C-101B-9397-08002B2CF9AE}`，`pid` 取已有最大整数加 1，至少为 2。`name` 为空、`type` 不认识，或 `type` 与 `value` 对不上（例如 `i4` 却给了字符串或带小数的数字），报 `InvalidField`。id 不是格式根报 `InvalidId`。缺 `custom.xml` 则新建部件、根关系和 Override。

### deleteCustomProp

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteCustomProp"` |
| `id` | 必须是 `prop:{name}`，`{name}` 与树上自定义属性的 `name` 相同 |

找不到该名报 `IdNotFound`。id 不是 `prop:{name}` 报 `InvalidId`。删光后去掉 custom 部件、根关系和 Override。

### setProtection

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    { "op": "setProtection", "id": "docx:document", "enforced": true, "edit": "readOnly" }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setProtection"` |
| `id` | 必须是 `docx:document` |
| `enforced` | 可选。布尔。`true` 写出 `w:documentProtection` 并强制；`false` 删掉该元素 |
| `edit` | 可选。`readOnly`、`comments`、`trackedChanges` 或 `forms`。打开强制且当前没有种类时，缺省 `readOnly` |
| `formatting` | 可选。布尔。对应 `w:formatting` |
| `readOnlyRecommended` | 可选。布尔。`true` 写出 `<w:writeProtection w:recommended="true"/>`；`false` 在没有写保护密码时删掉该元素 |

这四个字段至少给一个，否则报 `InvalidField`。未点名的字段留下。`edit` 为 `none` 或不认识的字符串报 `InvalidField`。关掉强制请用 `enforced: false`，不要写 `edit: none`。

出现 `password` 报 `NotImplemented`，不写哈希。`documentProtection` 已有密码哈希时，改 `enforced` / `edit` / `formatting` 报 `NotImplemented`，不删哈希。`writeProtection` 已有密码哈希时，改 `readOnlyRecommended` 报 `NotImplemented`。id 不是 `docx:document` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。没有 `settings.xml` 报 `OpcPartMissing`。

打开强制时写出 Transitional 标记：`w:edit` 与 `w:enforcement="1"`，不要写 `hash` / `salt` / `crypt*`。`edit` 为 `trackedChanges` 时同时写出空的 `w:trackRevisions`；关掉保护时不要删已有的 `trackRevisions`。`edit` 为 `forms` 时给主文档每一个 `w:sectPr` 写 `w:formProt`；关掉窗体保护或关掉强制时删掉各节 `w:formProt`。不读写 `w:permStart` / `w:permEnd`。已经是目标状态则空操作，仍计入 `opsApplied`。

create 的 `document.protection` 复用同一套规则。出现 `hasPassword` 或 `password` 报 `InvalidField` / `NotImplemented`（`hasPassword` 是树上的只读字段，create 出现即 `InvalidField`）。

空单元格：目标格子在 XML 里还不存在时，`setCell` 按需插入 `<row>` 和 `<c>`，插在正确的行列顺序上。找不到工作表才报 `IdNotFound`。

省略 `r` 的行和单元格：SpreadsheetML 允许 `<row>` 与 `<c>` 不写 `r`，编号由出现顺序推出。apply 在改这张表之前，先把这类隐式编号按顺序算出来写成显式 `r`。不这样做，后面插入新行会让它们的编号整体漂移，等于悄悄改了别的数据。这是语义等价的改写，Office 打开看不出区别。

共享公式：主单元格的 `<f>` 带 `ref` 和 `si`，从属单元格只带 `si`。对**主单元格**发 `setCell`（写值、写公式、清空都算）会让从属单元格的 `si` 失去定义，Office 打开时报需要修复，因此第一期直接报 `NotImplemented`，不写文件。想改主单元格，先把同组的从属单元格逐个改成独立公式。对**从属单元格**发 `setCell` 是允许的，主单元格不受影响。库不做公式引用平移，这属于计算引擎的活。

清空：去掉 `<f>`、`<v>`、`<is>`，保留 `<c>` 上的 `r`、`s` 以及不认识的属性。这样常见数字格式和字体还能在。若清空后 `<c>` 只剩 `r`（没有 `s` 也没有其他属性），可以把整个 `<c>` 删掉。目标格在 XML 里还不存在时，清空是空操作，仍计入 `opsApplied`；这不是错误，因为 `setCell` 对空格子本来就不报 `IdNotFound`。

改已有格子：保留 `s` 和未知属性。把文本从共享字符串改成新文本时，写成 `t="inlineStr"`，不要去改 `sharedStrings.xml` 里的旧项。改共享字符串会牵动所有引用同一索引的格子。

数字、布尔按类型改 `t` 和 `<v>`。写入公式时：`<f>` 存去掉前导 `=` 的文本，`t` 若原先是 `s`/`inlineStr`/`b` 则去掉，删除 `<v>` 和 `<is>`。

### 重算标记

任意一条 `setCell` 成功之后，必须让打开文件的 Excel 重算。第一期用这两步，两步都做：

1. 在 `/xl/workbook.xml` 的 `<calcPr>` 上设置 `fullCalcOnLoad="1"`。没有 `<calcPr>` 就建一个。其余 `calcPr` 属性保留。依据 ECMA-376 §18.2.2。
2. 若存在 `/xl/calcChain.xml`，删除该部件、内容类型 Override、以及 workbook 关系里指向它的那条。过期计算链会让部分 Excel 版本沿用旧顺序。

不在库里求值，不写新的缓存 `<v>`。

### 改动范围

只允许变脏的部件：

- 目标工作表 XML
- `/xl/workbook.xml`（仅 `calcPr`，以及给隐式编号补 `r` 这类语义等价的改写）
- 可能被删除的 `/xl/calcChain.xml` 以及因此修改的关系与内容类型

`styles.xml`、其他工作表、主题一律原样写回，字节不变。只改单元格、不改图、图表或页眉时，绘图原样写回。图表 op 还可以脏 drawing、`/xl/charts/…` 及其 `.rels`、`[Content_Types].xml`，以及被写入数据的工作表 XML；写入单元格时走上面同一套重算标记。`insertChartSheet` / `deleteChartSheet` 还可以脏 `/xl/workbook.xml` 的 `sheets`、workbook 关系、`/xl/chartsheets/…`。页眉文本只脏目标表 XML。页眉图还可以脏 `drawingHF` 指向的 drawing、工作表 `.rels`、`/xl/media/…` 与 `[Content_Types].xml`。`setProps` / `setCustomProp` / `deleteCustomProp` 还可以脏 `/docProps/core.xml`、`/docProps/app.xml`、`/docProps/custom.xml`、根 `.rels` 与 `[Content_Types].xml`。只改 core 字段时不要新建 custom。未点名的 `HeadingPairs` / `TitlesOfParts` 留下。

被改的部件是整份重新序列化的，不是按字节做定点替换。因此保证到这个程度：XML 声明、元素与属性的顺序、未知命名空间的元素与属性、文本内容都保留；注释、处理指令和 DOCTYPE 会丢，CDATA 会变成等价的转义文本。Office 的工作表部件不带这些东西，所以第一期接受这个代价。

### setHeaderFooterText

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "output": "C:\\data\\book.hf.xlsx",
  "ops": [
    { "op": "setHeaderFooterText", "id": "sheet:Sheet1/hf:odd:header", "text": "&C新页眉" }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setHeaderFooterText"` |
| `id` | 页眉槽位 id，例如 `sheet:Sheet1/hf:odd:header` |
| `text` | 该槽位原文，字符串，可空。必填 |

找不到该表报 `IdNotFound`。id 不是页眉槽位 id 报 `InvalidId`。对图 id、工作表 id 或单元格 id 发此 op 报 `InvalidId`。`worksheet` / `chartsheet` / `dialogsheet` 都可以改文本。没有 `headerFooter` 则按 CT_Worksheet 顺序新建。`text` 为空串时去掉该槽位元素；六个槽位都空时仍保留空的 `headerFooter`（若已有 `differentOddEven` / `differentFirst` 等属性则留下）。写 `even` 槽位时给 `headerFooter` 设 `differentOddEven="1"`；写 `first` 槽位时设 `differentFirst="1"`。不自动清这两项属性。不改 VML。不解释 `&P` 等代码。docx / pptx 上发此 op 报 `InvalidField`。`create` 本期不写页眉页脚。

### setParaText

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.patched.docx",
  "ops": [
    { "op": "setParaText", "id": "para:3F2A1B00", "text": "新标题" }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setParaText"` |
| `id` | 段落 id：正文用 `para:…` / `para:index:n`；页眉页脚用 `header:…/para:…` 或 `footer:…/para:…`；框内用 `{框 id}/para:…`；注内用 `{注 id}/para:…` |
| `text` | 新的可见纯文本，字符串，可空。必填 |

找不到该段落报 `IdNotFound`。不插入、不删除段落。对页眉或页脚容器 id（没有 `/para:`）报 `InvalidId`。该段里的 `w:drawing` 必须留下，包括只剩绘图、已经被清掉 `w:t` 的 run。

替换规则（手术刀，不整篇重建目标部件）：

1. 只改命中的那一个 `w:p`。`w:pPr`、书签、绘图、`w:hyperlink` 外壳、不认识的子元素都留着。不删 `w:drawing`。
2. 在该段里找出不位于 `w:del` 中的、含 `w:t` 的 run。
3. 若一个都没有：在 `w:pPr` 之后插入一个新的 `w:r`/`w:t`，文本放进去。
4. 若有：把完整新文本写入其中第一个 `w:t`（需要保留首尾空格时加 `xml:space="preserve"`）；其余这类 `w:t` 删掉；因此变空、且里面再也没有别的 run 内容的 `w:r` 删掉。第一个 run 上已有的 `w:rPr` 保留。
5. 段内若落在本期收入的域范围内，走第 9 条，不按本条拆域。
6. 不往文本框里写字。第 2 步找 run 的时候，遇到 `w:txbxContent` 停住，遇到 `mc:AlternateContent` 只进第一个 `mc:Choice`（没有就进 `mc:Fallback`），判定与 inspect 那两条边界一致。某一段除文本框之外没有可改的 `w:t` 时走第 3 步插新 run，插在 `w:pPr` 之后、该段其余内容之前。这样 apply 写进去的文本，随后 inspect 读回来必须与写入值相同。
7. 段内存在本期收入的修订节点（run 级 `w:ins` / `w:del`、`w:rPrChange`、`w:pPrChange`）时报 `NotImplemented`，不改 XML。先对该段上的修订发 `acceptRevision` / `rejectRevision`。属性容器上的 `ins`/`del`（例如段落标记删除）本期不进树，不挡 `setParaText`。
8. 段内存在本期收入的 `w:hyperlink` 时报 `NotImplemented`，不改 XML。先对该链发 `setHyperlink` / `deleteHyperlink`。书签不挡 `setParaText`。
9. 目标段落落在本期收入的域范围内（该域的起点段、终点段、或跨段域中间的段）时报 `NotImplemented`，不改 XML。先对该域发 `setField` / `deleteField`。框内 / 注内 / 批注内的域本期不进树，不挡 `setParaText`。

xlsx / pptx 上发此 op 报 `InvalidField`。

### setHeaderFooterDistance

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.dist.docx",
  "ops": [
    { "op": "setHeaderFooterDistance", "id": "section:1", "headerDistance": 720, "footerDistance": 1440 }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setHeaderFooterDistance"` |
| `id` | 节 id，必须是 `section:{n}` |
| `headerDistance` | 可选。页眉到页顶的距离，非负整数，单位 twip |
| `footerDistance` | 可选。页脚到页底的距离，非负整数，单位 twip |

两个距离至少给一个，否则报 `InvalidField`。节不存在报 `IdNotFound`。id 不是 `section:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

只改目标节 `w:pgMar` 上被点名的属性。没有 `w:pgMar` 时插入一个，只写本次给出的字段，不补 `top` / `left` / `right` / `bottom` / `gutter`。已有的其它边距属性原样留下。四边距与装订线请用 `setPageMargin`，不要塞进本 op。

### setPageMargin

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.margin.docx",
  "ops": [
    { "op": "setPageMargin", "id": "section:1", "top": 1440, "left": 1800, "right": 1800, "bottom": 1440 }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setPageMargin"` |
| `id` | 节 id，必须是 `section:{n}` |
| `top` | 可选。上边距，整数，单位 twip，可为负 |
| `bottom` | 可选。下边距，整数，单位 twip，可为负 |
| `left` | 可选。左边距，非负整数，单位 twip |
| `right` | 可选。右边距，非负整数，单位 twip |
| `gutter` | 可选。装订线，非负整数，单位 twip |

五个边距至少给一个，否则报 `InvalidField`。不要出现 `headerDistance` / `footerDistance`。节不存在报 `IdNotFound`。id 不是 `section:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

只改目标节 `w:pgMar` 上被点名的属性。没有 `w:pgMar` 时插入一个，只写本次给出的字段。已有的 `header` / `footer` 以及未点名的边距原样留下。

### setPageSize

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.size.docx",
  "ops": [
    { "op": "setPageSize", "id": "section:1", "pageWidth": 16838, "pageHeight": 11906, "orient": "landscape" }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setPageSize"` |
| `id` | 节 id，必须是 `section:{n}` |
| `pageWidth` | 可选。页宽，非负整数，单位 twip，写入 `w:pgSz/@w:w` |
| `pageHeight` | 可选。页高，非负整数，单位 twip，写入 `w:pgSz/@w:h` |
| `orient` | 可选。`portrait` 或 `landscape`，写入 `w:orient` |

三个字段至少给一个，否则报 `InvalidField`。不认识的 `orient` 报 `InvalidField`。只改被点名的属性。没有 `w:pgSz` 时插入一个，只写本次给出的字段，不补宽高，也不猜纵向。改 `orient` 时不自动对调 `pageWidth` / `pageHeight`。不读写 `w:code`。

### setColumns

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.cols.docx",
  "ops": [
    { "op": "setColumns", "id": "section:1", "num": 2, "space": 720, "equalWidth": true }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setColumns"` |
| `id` | 节 id，必须是 `section:{n}` |
| `num` | 可选。等宽时的列数，正整数，写入 `w:num` |
| `space` | 可选。等宽列间距，非负整数，单位 twip，写入 `w:space` |
| `equalWidth` | 可选。布尔，写入 `w:equalWidth` |
| `sep` | 可选。布尔，写入 `w:sep` |
| `cols` | 可选。不等宽列数组。每项可含 `width` / `space`（非负 twip）。出现时替换全部 `w:col` 子元素；空数组清掉子元素，不删 `w:cols` 本身 |
| `enabled` | 可选。只能是 `false`：删掉本节 `w:cols`。不要写 `true` |

`enabled: false` 与其它字段不能同时出现，否则报 `InvalidField`。其余字段至少给一个，否则报 `InvalidField`。`num` 小于 1 报 `InvalidField`。只改被点名的属性。没有 `w:cols` 且不是删除时插入一个。给了 `cols` 却同时 `equalWidth` 为 `true` 报 `InvalidField`。

### setVerticalAlign

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setVerticalAlign"` |
| `id` | 节 id，必须是 `section:{n}` |
| `vAlign` | `top` / `center` / `both` / `bottom`，或 `null` 表示删掉 `w:vAlign` |

必填。不认识的字符串报 `InvalidField`。

### setLineNumbering

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setLineNumbering"` |
| `id` | 节 id，必须是 `section:{n}` |
| `countBy` | 可选。非负整数，写入 `w:countBy` |
| `start` | 可选。非负整数，写入 `w:start` |
| `distance` | 可选。非负整数，单位 twip，写入 `w:distance` |
| `restart` | 可选。`newPage` / `newSection` / `continuous` |
| `enabled` | 可选。只能是 `false`：删掉本节 `w:lnNumType` |

`enabled: false` 与其它字段不能同时出现。其余字段至少给一个。没有 `w:lnNumType` 且不是删除时插入一个，只写本次给出的属性。

### setDocGrid

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setDocGrid"` |
| `id` | 节 id，必须是 `section:{n}` |
| `type` | 可选。`default` / `lines` / `linesAndChars` / `snapToChars` |
| `linePitch` | 可选。整数，写入 `w:linePitch` |
| `charSpace` | 可选。整数，写入 `w:charSpace` |
| `enabled` | 可选。只能是 `false`：删掉本节 `w:docGrid` |

`enabled: false` 与其它字段不能同时出现。其余字段至少给一个。没有 `w:docGrid` 且不是删除时插入一个。

### setRtlGutter

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setRtlGutter"` |
| `id` | 节 id，必须是 `section:{n}` |
| `rtlGutter` | 布尔。`true` 写入本节 `w:rtlGutter`，`false` 删掉该元素 |

必填。规则与 `setTitlePage` 相同。

### setMirrorMargins

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setMirrorMargins"` |
| `id` | 必须是 `docx:document` |
| `mirrorMargins` | 布尔。`true` 写入 `settings.xml` 的 `w:mirrorMargins`，`false` 删掉该元素 |

id 不是 `docx:document` 报 `InvalidId`。没有 `settings.xml` 报 `OpcPartMissing`。xlsx / pptx 上发此 op 报 `InvalidField`。

### setGutterAtTop

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setGutterAtTop"` |
| `id` | 必须是 `docx:document` |
| `gutterAtTop` | 布尔。`true` 写入 `settings.xml` 的 `w:gutterAtTop`，`false` 删掉该元素 |

id 与缺 settings 的报错与 `setMirrorMargins` 相同。关掉本标志不改各节 `w:pgMar/@w:gutter`。

### setHeaderFooterLink

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.link.docx",
  "ops": [
    { "op": "setHeaderFooterLink", "id": "section:2", "kind": "header", "type": "default", "linked": false }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setHeaderFooterLink"` |
| `id` | 节 id，必须是 `section:{n}` |
| `kind` | `header` 或 `footer` |
| `type` | `default`、`even` 或 `first` |
| `linked` | `true` 重新链接（删掉本节该类型引用）；`false` 断开 |

第一节上 `linked: true` 报 `InvalidField`。该类型对本节不生效时报 `InvalidField`（不替用户打开首页不同或奇偶页）。节不存在报 `IdNotFound`。id 不是 `section:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。已经是目标状态则空操作，仍计入 `opsApplied`，不脏任何部件。

`linked: false`：若前节有可继承的该类型部件，把该部件字节复制到新路径（`/word/headerN.xml` 或 `/word/footerN.xml`，`N` 取未占用序号）；写入主文档关系、`[Content_Types].xml` 的 Override，以及本节一条引用。若源部件有自己的 `.rels`，把该关系文件原样抄到新部件对应路径（`Target` 仍指向同一 `/word/media/…` 等，不复制媒体字节）。若前节没有部件（空白），写一份最小 `w:hdr` / `w:ftr`（一个空 `w:p`），再挂引用。

`linked: true`：删掉本节该类型引用。重新按继承解析之后，若该部件没有任何节再用，删掉部件、对应关系、该部件自己的 `.rels` 和 Override。媒体部件若仍被其它页眉页脚引用则留下。

不写入 `titlePg` / `evenAndOddHeaders`（那是 `setTitlePage` / `setEvenAndOddHeaders`）。未点名的其它页眉页脚字节仍不变。

### insertHeaderFooter

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.hf.docx",
  "ops": [
    { "op": "insertHeaderFooter", "id": "section:1", "kind": "header", "type": "default" }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertHeaderFooter"` |
| `id` | 节 id，必须是 `section:{n}` |
| `kind` | `header` 或 `footer` |
| `type` | `default`、`even` 或 `first` |

该类型对本节不生效时报 `InvalidField`（不替用户打开首页不同或奇偶页）。本节该类型已经链接前节（inspect 的 `headerLink` / `footerLink` 为 `true`）时报 `InvalidField`；要独立部件走 `setHeaderFooterLink` 且 `linked: false`。节不存在报 `IdNotFound`。id 不是 `section:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

本节已有该类型显式引用：空操作，仍计入 `opsApplied`，不脏任何部件。

否则写一份最小 `w:hdr` / `w:ftr`（一个空 `w:p`），路径为未占用的 `/word/headerN.xml` 或 `/word/footerN.xml`，写入主文档关系、`[Content_Types].xml` 的 Override，以及本节一条引用。不要从母版或前节抄内容。

### setTitlePage

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.title.docx",
  "ops": [
    { "op": "setTitlePage", "id": "section:1", "titlePage": true }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTitlePage"` |
| `id` | 节 id，必须是 `section:{n}` |
| `titlePage` | `true` 写入本节 `w:titlePg`；`false` 去掉该元素 |

不新建 first 页眉页脚。关掉时不删除已有 first 部件。已经是目标状态则空操作，仍计入 `opsApplied`，不脏任何部件。节不存在报 `IdNotFound`。id 不是 `section:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### setEvenAndOddHeaders

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.even.docx",
  "ops": [
    { "op": "setEvenAndOddHeaders", "id": "docx:document", "evenAndOdd": true }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setEvenAndOddHeaders"` |
| `id` | 必须是 `docx:document` |
| `evenAndOdd` | `true` 在 `settings.xml` 写入 `w:evenAndOddHeaders`；`false` 去掉该元素 |

整篇一份，不按节。不新建 even 页眉页脚。关掉时不删除已有 even 部件。已经是目标状态则空操作，仍计入 `opsApplied`，不脏任何部件。没有 `settings.xml` 时报 `OpcPartMissing`。id 不是 `docx:document` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### replacePicture

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.pic.docx",
  "ops": [
    { "op": "replacePicture", "id": "header:default:1/pic:1", "file": "C:\\img\\logo.png" }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"replacePicture"` |
| `id` | 图 id：docx 为 `{容器}/pic:…` 或 `{段落 id}/pic:…`；xlsx 为 `sheet:{name}/pic:…`；pptx 为 `slide:{n}/pic:…` 或 `slide:{n}/notes/pic:…` |
| `file` | 本地图片路径，与信封 `path` 一样认正斜杠和反斜杠。不把 base64 放进领域 JSON |

找不到该图报 `IdNotFound`。读不到 `file` 报 `Io`。文件不是本库认的位图（见下）报 `InvalidField`。

只换媒体字节与内容类型，不改锚点、绕排和位置。`a:blip/@r:embed` 的关系 id 保持不变。docx 扩展名变了就写新的 `/word/media/imageN.*`；xlsx 写新的 `/xl/media/imageN.*`；pptx 写新的 `/ppt/media/imageN.*`。改同一条关系的 `Target`，并更新 `[Content_Types].xml` 的 Default 或 Override；旧媒体若重新解析后无人引用则删掉。认 PNG、JPEG（`.jpg` / `.jpeg`）、GIF、BMP；按文件头判断，扩展名只用来给新媒体取名。

### insertPicture

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.ins.docx",
  "ops": [
    { "op": "insertPicture", "id": "header:default:1/para:3F2A1B00", "file": "C:\\img\\logo.png" }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertPicture"` |
| `id` | docx：段落 id（正文用 `para:…` / `para:index:{n}`，页眉页脚用 `{容器}/para:…`）。xlsx：工作表 id `sheet:{name}`，或页眉槽位 id `sheet:{name}/hf:{kind}:{side}`。pptx：页 id `slide:{n}`，或备注容器 id `slide:{n}/notes` |
| `file` | 本地图片路径 |
| `cx` | 可选。宽度，EMU，正整数 |
| `cy` | 可选。高度，EMU，正整数 |
| `x` | pptx 可选。画布横坐标，EMU，非负整数。docx / xlsx 上出现报 `InvalidField` |
| `y` | pptx 可选。画布纵坐标，EMU，非负整数。docx / xlsx 上出现报 `InvalidField` |
| `from` | xlsx 往工作表插图时必填。左上角单元格，A1。往页眉槽位插图时出现报 `InvalidField`。docx / pptx 上出现报 `InvalidField` |
| `to` | xlsx 且 `anchor` 为 `twoCell` 时必填。右下角单元格，A1。往页眉槽位插图时出现报 `InvalidField`。docx / pptx 上出现报 `InvalidField` |
| `anchor` | xlsx 往工作表插图时可选。`oneCell`（缺省）或 `twoCell`。不认 `absolute`。往页眉槽位插图时出现报 `InvalidField`。docx / pptx 上出现报 `InvalidField` |
| `fromOffX` / `fromOffY` | xlsx 往工作表插图时可选。相对 `from` 的 EMU 偏移，缺省 `0`，不得为负。往页眉槽位插图时出现报 `InvalidField`。docx / pptx 上出现报 `InvalidField` |
| `toOffX` / `toOffY` | xlsx 且 `twoCell` 时可选。相对 `to` 的 EMU 偏移，缺省 `0`，不得为负。往页眉槽位插图时出现报 `InvalidField`。docx / pptx 上出现报 `InvalidField` |

docx：找不到该段落报 `IdNotFound`。`id` 不是段落 id 报 `InvalidId`。`cx` / `cy` 都省略时，按 PNG / JPEG / GIF / BMP 的像素尺寸、96 dpi 换算成 EMU（`px × 914400 / 96`）；读不出尺寸则用 1 英寸见方（`914400`）。只给一个时报 `InvalidField`。插入 `wp:inline`，挂在该段末尾一个新的 `w:r` 里。写入该部件关系、`/word/media/imageN.*` 与内容类型。`wp:docPr/@id` 取该部件已有 `docPr` 的最大整数加 1，至少为 1。

xlsx：找不到该表报 `IdNotFound`。`id` 不是 `sheet:{name}` 也不是页眉槽位 id 报 `InvalidId`。该表不是 `worksheet` 报 `InvalidField`。往工作表插图必须给 `from`，否则报 `MissingField`。`from` / `to` 不是合法 A1 报 `InvalidRange`。`anchor` 为 `twoCell` 却没有 `to` 报 `InvalidField`。`anchor` 为 `absolute` 或不认识报 `InvalidField`。`cx` / `cy` 规则与 docx 相同；`twoCell` 也要写 `xdr:pic` 里的 `a:ext`（给 Excel 一个内部尺寸），显示尺寸仍由起止格决定。没有 drawing 部件则新建 `/xl/drawings/drawingN.xml`、工作表关系、`[Content_Types].xml` 的 Override，并在工作表上写 `<drawing r:id="…"/>`。往页眉槽位插图时不写 `<drawing>`，改写或新建 `<drawingHF r:id="…"/>`。新媒体写入 `/xl/media/imageN.*`。`xdr:cNvPr/@id` 取该表工作表 drawing 与 `drawingHF` 里已有 `cNvPr` 的最大整数加 1。工作表图至少为 1。页眉图至少为 2：Excel 16.0 若 `drawingHF` 的 `cho` / `lho` 等属性指向 id 1，会拒绝打开该工作簿。锚点必须带空的 `xdr:clientData`（§20.5.2.3）。

pptx：找不到该页报 `IdNotFound`。`id` 必须是 `slide:{n}` 或 `slide:{n}/notes`。对图 id、形状 id 或 `pptx:presentation` 发此 op 报 `InvalidId`。`cx` / `cy` 规则与 docx 相同。`x` / `y` 必须成对出现或成对省略；只给一个时报 `InvalidField`。缺坐标则放到该宿主空白处，不要叠在 `(0,0)`：往幻灯片插时读取 `presentation.xml` 的 `p:sldSz`；往备注插时读取 `p:notesSz`（没有则用 `6858000` × `9144000`）。扫描该宿主已有 `p:sp` / `p:pic` / `p:graphicFrame` / 嵌套 `p:grpSp` 的 `a:xfrm`，优先放到最右侧空隙，放不下再放到下方。`kind` / `wrap` / `from` / `to` / `anchor` 以及 xlsx 偏移字段出现报 `InvalidField`。写入该宿主 `p:spTree` 一个新的 `p:pic`（带 `a:blip`），关系指向 `/ppt/media/imageN.*`。`p:cNvPr/@id` 取该宿主已有全部 `cNvPr`（`sp` / `pic` / `graphicFrame` / 组合，含 `spTree` 自己的 `id="1"`）的最大整数加 1，至少为 2。`id` 为 `slide:{n}/notes` 且该页还没有 notesSlide 时，按 `setNotesText` 同一套规则新建最小备注页再插图。

### deletePicture

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    { "op": "deletePicture", "id": "header:default:1/pic:1" }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deletePicture"` |
| `id` | 图 id |

找不到该图报 `IdNotFound`。docx：按图 id 删掉对应的 `w:drawing`。若该图在 `mc:AlternateContent` 的 Choice 里，整段 `AlternateContent` 删掉（避免 Fallback 的 VML 把图留住）。所在 `w:r` 若删完后只剩 `w:rPr` 或空，把该 run 也删掉。xlsx：按图 id 删掉包含该 `xdr:pic` 的整个锚点。工作表 drawing 删空后去掉该部件、工作表上的 `<drawing>` 和对应关系。`drawingHF` 删空后去掉该部件、工作表上的 `<drawingHF>` 和对应关系；同时清掉 `drawingHF` 上指向该 `cNvPr` id 的左/中/右属性。pptx：按图 id 删掉对应的 `p:pic`。重新解析包内关系之后，若该媒体与该关系再无人引用，删掉媒体部件、关系条目和对应内容类型。

### setPictureSize

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    { "op": "setPictureSize", "id": "header:default:1/pic:1", "cx": 2441542, "cy": 1828800 }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setPictureSize"` |
| `id` | 图 id |
| `cx` | 可选。宽度，EMU，正整数 |
| `cy` | 可选。高度，EMU，正整数 |

`cx` 与 `cy` 至少给一个，否则报 `InvalidField`。只改被点名的那一侧，另一侧保持原值。docx：写入 `wp:extent`；若 `pic:spPr/a:xfrm/a:ext` 存在，一并改。xlsx：`anchor` 为 `oneCell` 或 `absolute` 时写入 `xdr:ext`；若 `xdr:spPr/a:xfrm/a:ext` 存在，一并改。`anchor` 为 `twoCell` 时报 `NotImplemented`，不写文件。pptx：写入 `p:spPr/a:xfrm/a:ext`。找不到该图报 `IdNotFound`。

### setPictureWrap

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "setPictureWrap",
      "id": "header:default:1/pic:1",
      "kind": "anchor",
      "wrap": "square",
      "alignH": "left",
      "alignV": "top"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setPictureWrap"` |
| `id` | 图 id |
| `kind` | `inline` 或 `anchor` |
| `wrap` | `kind` 为 `anchor` 时必填：`square` / `tight` / `through` / `topAndBottom` / `none`。`kind` 为 `inline` 时必须省略或为 `null` |
| `alignH` | 可选。相对页边水平对齐：`left` / `center` / `right`。只在 `kind` 为 `anchor` 时有意义 |
| `alignV` | 可选。相对页边垂直对齐：`top` / `center` / `bottom` |
| `offsetX` | 可选。相对页边水平偏移，EMU 整数（可负）。与 `alignH` 不能同时出现 |
| `offsetY` | 可选。相对页边垂直偏移，EMU 整数（可负）。与 `alignV` 不能同时出现 |

行内改浮动或反过来。位置只支持相对页边（`wp:positionH/@relativeFrom` 与 `wp:positionV/@relativeFrom` 都写 `margin`），不做任意 Dist，也不写其它 `relativeFrom`。改成浮动且没给对齐或偏移时，水平默认 `alignH: "left"`，垂直默认 `alignV: "top"`。已经是浮动、只改 `wrap` 时，未点名的位置属性原样留下。`behindDoc`：`wrap` 为 `none` 时写成 `1`（衬于文字下方），其余为 `0`。xlsx / pptx 上发此 op 报 `InvalidField`。xlsx 改锚点请用 `setPictureAnchor`。pptx 改画布坐标请用 `setPicturePos`。

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

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setPictureAnchor"` |
| `id` | xlsx 图 id |
| `anchor` | 可选。`oneCell` 或 `twoCell`。写成 `absolute` 报 `NotImplemented` |
| `from` | 可选。左上角 A1 |
| `to` | 可选。`twoCell` 的右下角 A1 |
| `fromOffX` / `fromOffY` / `toOffX` / `toOffY` | 可选。EMU，不得为负 |
| `posX` / `posY` | 可选。仅当现有锚点已是 `absolute` 时改位置，EMU |
| `editAs` | 可选。只对 `twoCell`：`twoCell` / `oneCell` / `absolute` |
| `cx` / `cy` | 从 `twoCell` 转到 `oneCell` 时必填（库不算列宽行高）。已经是 `oneCell` 时请用 `setPictureSize` |

`anchor`、位置字段、`editAs`、`cx` / `cy` 至少给一类，否则报 `InvalidField`。找不到该图报 `IdNotFound`。`id` 不是 xlsx 图 id 报 `InvalidId`。docx / pptx 上发此 op 报 `InvalidField`。pptx 改画布坐标请用 `setPicturePos`。

`oneCell` ↔ `twoCell` 可以互转：改锚点元素名，留下 `xdr:pic` 与 `xdr:clientData`。转到 `twoCell` 必须有 `to`（请求给，或原来就是 `twoCell` 已有）。转到 `oneCell` 必须有 `cx` 与 `cy`（请求给，或原来就是 `oneCell` 已有）。与 `absolute` 互转报 `NotImplemented`，不写文件。已有 `absolute` 只许改 `posX` / `posY`；改 `from` / `to` / `anchor` 报 `NotImplemented`。

未点名的位置属性原样留下。`from` / `to` 不是合法 A1 报 `InvalidRange`。

### setPicturePos

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    { "op": "setPicturePos", "id": "slide:1/pic:4", "x": 914400, "y": 914400 }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setPicturePos"` |
| `id` | pptx 图 id `slide:{n}/pic:…` 或 `slide:{n}/notes/pic:…` |
| `x` | 可选。画布横坐标，EMU，非负整数 |
| `y` | 可选。画布纵坐标，EMU，非负整数 |

`x` 与 `y` 至少给一个，否则报 `InvalidField`。只改被点名的那一侧。写入 `p:spPr/a:xfrm/a:off`。找不到该图报 `IdNotFound`。`id` 不是 pptx 图 id 报 `InvalidId`。docx / xlsx 上发此 op 报 `InvalidField`。

### insertTextBox

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    { "op": "insertTextBox", "id": "para:3F2A1B00", "cx": 1828800, "cy": 914400 }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertTextBox"` |
| `id` | 宿主段落 id（正文或页眉页脚段落） |
| `cx` | 可选。宽度，EMU，正整数 |
| `cy` | 可选。高度，EMU，正整数 |
| `text` | 可选。框内第一段可见文本，省略则为空段 |

`id` 必须是段落 id，对图 id、框 id、容器 id、框内段落 id 或 `section:{n}` 报 `InvalidId`。找不到该段报 `IdNotFound`。`cx` / `cy` 都省略时用 2 英寸宽、1 英寸高（`1828800` / `914400`）。只给一个时报 `InvalidField`。插入浮动 `anchor` + `wrapSquare`，挂在该段末尾一个新的 `w:r` 里，写成 `mc:AlternateContent`（Choice 为 `wps:wsp` / `wps:txbx`，Fallback 为空白 `w:pict`，不编 VML 几何）。`wp:docPr/@id` 取该部件已有 `docPr` 的最大整数加 1。对框内段落发此 op 报 `InvalidId`（不套框）。对注内或批注内段落发此 op 报 `InvalidField`：Word 不能把图形对象放到文本框、标注、批注、脚注或尾注中。已有文件里若出现注内或批注内的框，inspect 仍列出，apply 仍可改、删。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteTextBox

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteTextBox"` |
| `id` | 框 id |

找不到该框报 `IdNotFound`。对段落 id、图 id、容器 id 或 `section:{n}` 报 `InvalidId`。按框 id 删掉对应的 `w:drawing`；若在 `mc:AlternateContent` 的 Choice 里，整段 `AlternateContent` 删掉。所在 `w:r` 若删完后只剩 `w:rPr` 或空，把该 run 也删掉。链接链从中间删时，把前一框的 `next` 接到后一框、后一框的 `prev` 接到前一框，并改写 `wps:txbx` / `linkedTxbx` 的 `id` 与 `seq`。xlsx / pptx 上发此 op 报 `InvalidField`。

### setTextBoxSize

字段与 `setPictureSize` 相同，`id` 必须是框 id。写入该框 `wp:extent`；若 `wps:spPr/a:xfrm/a:ext` 存在，一并改。

### setTextBoxWrap

字段与 `setPictureWrap` 相同，`id` 必须是框 id。行内 / 浮动与绕排规则同图片。

### setTextBoxFill

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTextBoxFill"` |
| `id` | 框 id |
| `color` | RGB 六位十六进制，如 `"FF0000"`；`null` 表示无填充 |

写入 `wps:spPr` 的 `a:solidFill/a:srgbClr`；`color` 为 `null` 时写成 `a:noFill`。不要 theme 色。xlsx / pptx 上发此 op 报 `InvalidField`。

### setTextBoxLine

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTextBoxLine"` |
| `id` | 框 id |
| `color` | RGB 六位十六进制；`null` 表示无边框 |
| `width` | 可选。线宽，EMU，正整数。无边框时必须省略 |

`color` 为 `null` 时写成 `a:ln/a:noFill`。有颜色时写成 `a:ln` 加 `a:solidFill`。xlsx / pptx 上发此 op 报 `InvalidField`。

### setTextBoxDirection

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTextBoxDirection"` |
| `id` | 框 id |
| `direction` | `horizontal` 或 `vertical` |

`horizontal` 对应横排（去掉或写成从左到右）；`vertical` 对应竖排，写入 `wps:bodyPr/@vert` 为 `eaVert`。xlsx / pptx 上发此 op 报 `InvalidField`。

### insertPara / deletePara

| 字段 | 含义 |
| --- | --- |
| `op` | `"insertPara"` 或 `"deletePara"` |
| `id` | 见下 |
| `text` | 仅 `insertPara`。可选，新段可见文本 |

`insertPara` 的 `id`：

- 正文段落 id：插到该段之后（该段的直接父节点里，紧挨着它）
- `docx:document`：插到主文档故事末尾（`w:body` 里、`w:sectPr` 之前）
- 页眉或页脚容器 id（如 `header:default:1`）：插到该部件末尾
- 页眉或页脚段落 id：插到该段之后
- 框 id、注 id、批注 id：插到该容器末尾（原规则）
- 框内 / 注内 / 批注内段落 id：插到该段之后（原规则）

`deletePara` 的 `id` 必须是段落 id（正文、页眉页脚、表格单元格内、框内、注内或批注内）。对 `docx:document`、页眉页脚容器、框 id、注容器 id 或批注容器 id 发 `deletePara` 报 `InvalidId`。

只插删 `w:p`，不插删 `w:tbl` / `w:tr` / `w:tc`。表格单元格内的段落跟本 op 走。插删表、行、列见 `insertTable` 等结构 op。

直接父节点里只剩一个 `w:p` 时 `deletePara` 报 `InvalidField`。正文、页眉、页脚、表格单元格、`w:txbxContent`、一条用户注、一条 `w:comment` 都适用。不允许删空文档或删空页眉页脚。

新段带新的 `w14:paraId`。批注内插删段后必须把该批注 `commentEx/@paraId` 改成新的最后一段 `paraId`；若有回复的 `paraIdParent` 指向旧的最后一段，一并改成新值。

删主文档里收集顺序的最后一段，且该段 `pPr` 带 `sectPr`（文档末节属性）时：把 `sectPr` 挪到删完后仍留下的最后一段上，再删。该段不是收集顺序的最后一段、却带 `sectPr`（分节符）时，报 `NotImplemented`。

`deletePara` 时，若该段里的书签、批注范围或许可范围起止不成对（跨到段外），报 `NotImplemented`。段内有 `w:ins`、`w:del`、`w:moveFrom` 或 `w:moveTo` 时同样报 `NotImplemented`。不要拆坏跨段标记。框内、注内、批注内的既有规则不变。

xlsx / pptx 上发此 op 报 `InvalidField`。

### insertSection

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.split.docx",
  "ops": [
    { "op": "insertSection", "id": "para:3F2A1B00", "type": "nextPage" }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertSection"` |
| `id` | 主文档 `w:body` 的直接子段落 id（插在该段之后），或 `docx:document`（在文档末尾插入新的空节） |
| `type` | 可选。`nextPage`、`continuous`、`evenPage`、`oddPage`、`nextColumn`（`ST_SectionMark`）。省略当作 `nextPage`。写在新分节符所在的那个 `w:sectPr` 上 |

插在点名段之后：该段成为当前节的最后一段。该段还没有 `pPr/sectPr` 时，把分节符写在该段上。该段已经带 `sectPr` 时，在它后面插入一个空的 `w:p`（新 `w14:paraId`），把新分节符放在这个空段上。不要拆 run，不要拆段落中间。

节属性副本克隆该段当前所属节的 `w:sectPr`（含 `pgSz` / `pgMar` / `cols` / `titlePg` / `footnotePr` 等）。分节符之前的那一节保留原来的页眉、页脚引用。分节符之后的那一节省略 `headerReference` / `footerReference`，inspect 的 `headerLink` / `footerLink` 按已有规则为 `true`（第一节除外）。不要把引用抄到新节上。

点名段是当前最后一节的最后一段时：该段写上带引用的 `sectPr`，并在 `w:body` 的 `sectPr` 之前再插一个空的 `w:p`，保证末节至少有一段。`id` 为 `docx:document` 时：已有正文留在原末节，新节至少有一个空段。

该段之后、下一 `sectPr` 之前的块级内容（含 `w:tbl`）属于新节。最后一节的 `sectPr` 仍在 `w:body` 末尾，中间节在段末，不要写反。插完之后 `section:{n}` 按新的文档顺序重编。

表格单元格内、页眉页脚、框内、注内、批注内的段落，以及 `section:{n}`、容器 id、表 id、run id，一律报 `InvalidId`。找不到段落报 `IdNotFound`。不认识的 `type` 报 `InvalidField`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteSection

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.merge.docx",
  "ops": [
    { "op": "deleteSection", "id": "section:1" }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteSection"` |
| `id` | 节 id，必须是 `section:{n}`。`n` 不能是最后一节 |

删掉结束第 `n` 节的那个 `w:sectPr`（一定在某段 `pPr` 上，不会是 `w:body` 末尾那一份）。第 `n` 节的正文并进第 `n+1` 节，沿用第 `n+1` 节的节属性。这与 Word 删除分节符的方向相同。

只剩一节，或对最后一节发此 op，报 `InvalidField`。节不存在报 `IdNotFound`。id 不是 `section:{n}` 报 `InvalidId`。

删完后，若某个 header/footer 部件没有任何节的 `headerReference` / `footerReference` 再用，按 `setHeaderFooterLink` 且 `linked: true` 的既有逻辑删部件、主文档关系、该部件 `.rels`、Override。媒体仍被其它页眉引用则留下。xlsx / pptx 上发此 op 报 `InvalidField`。

`deletePara` 现有规则不变：非末段带 `sectPr` 仍报 `NotImplemented`。要拆掉分节符请用 `deleteSection`。

示例见 `docs/examples/docx-apply-insert-section.json`。create 多节见 `docs/examples/docx-create-sections.json`。

### insertTable

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    { "op": "insertTable", "id": "docx:document", "rows": 2, "cols": 2, "cells": [["A1","B1"],["A2","B2"]] }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertTable"` |
| `id` | 见上文表 id 那节：段落、`docx:document`、页眉页脚容器、表或单元格 |
| `rows` | 行数，1–500 的整数 |
| `cols` | 列数，1–63 的整数 |
| `cells` | 可选。二维字符串，行数与列数必须与 `rows` / `cols` 一致。省略则每格一段空文本 |

写出带单线边框的 `w:tbl`，每列宽 1440 twip，每格至少一段并带新的 `w14:paraId`。插在单元格末尾时，该格末尾仍保留一个 `w:p`（可在新表之后补空段）。`rows` / `cols` 越界或不是正整数报 `InvalidField`。id 形态不对报 `InvalidId`。找不到锚点报 `IdNotFound`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteTable

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteTable"` |
| `id` | 表 id |

删掉该 `w:tbl`。若删的是正文里仅剩的块级内容，在 `sectPr` 前补一个空的 `w:p`。若删的是单元格里的嵌套表，该格至少留下一个 `w:p`。表不存在报 `IdNotFound`。id 不是表 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### insertRow

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertRow"` |
| `id` | 表 id（追加到末尾）或行 id（插到该行之后） |

新行按参照行（表 id 时用最后一行，行 id 时用该行）的 `gridSpan` 布局复制格子，不复制 `vMerge`，每格一段空文本。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteRow

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteRow"` |
| `id` | 行 id |

只剩一行时报 `InvalidField`。xlsx / pptx 上发此 op 报 `InvalidField`。

### insertCol

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertCol"` |
| `id` | 表 id |
| `after` | 在第几列之后插入。0 表示插到最前，取值 0–`colCount` |

新列宽抄相邻列；没有 `tblGrid` 时先按当前布局补上，缺宽则 1440 twip。插入点落在某格 `gridSpan` 内部时，该格 `gridSpan` 加 1，不新开 `w:tc`。否则每行在该处插入一个新格。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteCol

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteCol"` |
| `id` | 表 id |
| `col` | 要删的网格列，从 1 起 |

只剩一列时报 `InvalidField`。该列落在某格 `gridSpan` 内部时，`gridSpan` 减 1；减到 0 则删掉该 `w:tc`。xlsx / pptx 上发此 op 报 `InvalidField`。

### mergeCells

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"mergeCells"` |
| `id` | 表 id |
| `startRow` / `startCol` / `endRow` / `endCol` | 闭区间，从 1 起。起大于止报 `InvalidField` |

范围内已有合并、或格子对不上网格时报 `NotImplemented`。幸存格是左上角：水平方向写 `gridSpan`，垂直方向左上为 `vMerge=restart`、其余行为 `continue`。被吃掉的格子里的段落（遇嵌套表停住）按先行后列挪进幸存格，然后删掉那些 `w:tc`。`continue` 格留下一个空的 `w:p`。xlsx / pptx 上发此 op 报 `InvalidField`。

### splitCells

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"splitCells"` |
| `id` | 单元格 id |

`gridSpan` 为 1 且没有 `vMerge` 时报 `InvalidField`。对 `continue` 格发此 op 报 `InvalidField`，必须点名 `restart` 那一格。水平拆分后补出的新格各有一段空文本；垂直拆分去掉该列向下连续的 `vMerge`。xlsx / pptx 上发此 op 报 `InvalidField`。

表、行或单元格上出现 `w:ins` / `w:del` / `w:moveFrom` / `w:moveTo`，或 `tblPrChange` / `trPrChange` / `tcPrChange` / `tblGridChange` 时，上述结构 op 报 `NotImplemented`，不写文件。某行占的网格列之和对不上 `colCount` 时，`insertCol` / `deleteCol` / `mergeCells` / `splitCells` / `insertRow` 同样报 `NotImplemented`。

### acceptRevision

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    { "op": "acceptRevision", "id": "rev:1" }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"acceptRevision"` |
| `id` | 修订 id |

只改被点名的那一个 XML 节点，不调 Word COM。`ins`：拆掉 `w:ins` 外壳，留下内部 `w:r`，并去掉这些 run 的属性容器上的 `ins` / `del`（Word 会另写在 `rPr` 里，本期不进树）。`del`：删掉整个 `w:del`（含 `delText`）。`rPrChange` / `pPrChange`：删掉 Change 元素，留下当前属性。找不到报 `IdNotFound`。id 不是修订 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### rejectRevision

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"rejectRevision"` |
| `id` | 修订 id |

`ins`：删掉整个 `w:ins`。`del`：拆掉 `w:del` 外壳，把其中 `w:delText` 改回 `w:t`，并去掉内部属性容器上的 `ins` / `del`。`rPrChange`：用 Change 里的旧 `w:rPr` 替换当前 run 属性。`pPrChange`：用 Change 里的旧 `w:pPr` 替换当前段落属性。找不到报 `IdNotFound`。id 不是修订 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/docx-apply-revision.json`。

### insertHyperlink

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertHyperlink"` |
| `id` | 段落 id（正文或页眉页脚） |
| `text` | 显示文本，字符串，可空。必填 |
| `url` | 外部目标。与 `anchor` 必须恰好给一个 |
| `anchor` | 内部书签名。与 `url` 必须恰好给一个 |
| `tooltip` | 可选。`w:tooltip` |

在该段已有内容之后插入一个 `w:hyperlink`，内含一个带 `text` 的 run。外部链在该部件 `.rels` 上新建 Type 以 `/hyperlink` 结尾的关系，`TargetMode` 为 `External`，`Target` 为 `url` 原文。内部链写 `w:anchor`，不写关系。`url` 与 `anchor` 同时出现或一个都不给报 `InvalidField`。`url` 为空串报 `InvalidField`。`anchor` 为空串或以 `_` 开头报 `InvalidField`。找不到段落报 `IdNotFound`。id 不是段落 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### setHyperlink

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setHyperlink"` |
| `id` | 超链接 id |
| `url` | 可选。改成外部目标 |
| `anchor` | 可选。改成内部书签名 |
| `text` | 可选。改显示文本 |
| `tooltip` | 可选。`null` 表示去掉 `w:tooltip` |

`url`、`anchor`、`text`、`tooltip` 至少给一个。`url` 与 `anchor` 不能同时出现。给了 `url`：有已有 `r:id` 则改该关系 `Target`；没有则新建关系并写 `r:id`，去掉 `w:anchor`。给了 `anchor`：写 `w:anchor`，去掉 `r:id`，该关系无人引用则删除。`text` 只改该 `w:hyperlink` 内可见 `w:t`（规则与 `setParaText` 第 2–4 条相同，范围限于该元素）。`tooltip` 为字符串则写属性；为 `null` 则去掉属性。找不到报 `IdNotFound`。id 不是超链接 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteHyperlink

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteHyperlink"` |
| `id` | 超链接 id |

拆掉 `w:hyperlink` 外壳，留下内部 `w:r`。该 `r:id` 在本部件 XML 里无人引用则删除关系。找不到报 `IdNotFound`。id 不是超链接 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### insertBookmark

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertBookmark"` |
| `id` | 起始段落 id（正文或页眉页脚） |
| `name` | 可选。书签名。省略则用尚未占用的 `ooxp{n}`（n 从 1 起） |
| `end` | 可选。结束段落 id。省略则与 `id` 相同 |

在起始段 `w:pPr` 之后插入 `w:bookmarkStart`，在结束段末尾插入配对 `w:bookmarkEnd`。`w:id` 取该部件已有书签 id 的最大整数加 1，至少为 0。`name` 不能为空、不能含空白、不能以 `_` 开头，否则报 `InvalidField`。与已有书签名（含隐藏书签）冲突报 `InvalidField`。`end` 必须与 `id` 在同一故事，且文档顺序不得早于起始段，否则报 `InvalidField`。找不到段落报 `IdNotFound`。id / `end` 不是段落 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### setBookmark

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setBookmark"` |
| `id` | 书签 id |
| `name` | 新书签名 |

改 `w:bookmarkStart/@w:name`。同时把本部件里 `w:hyperlink/@w:anchor` 等于旧名的改成新名。新名规则与 `insertBookmark` 相同。与其他书签重名报 `InvalidField`。已经是该名则空操作，仍计入 `opsApplied`。找不到报 `IdNotFound`。id 不是书签 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteBookmark

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteBookmark"` |
| `id` | 书签 id |

去掉配对的 `w:bookmarkStart` 与 `w:bookmarkEnd`。不改指向该书签的 `w:hyperlink/@w:anchor`。找不到报 `IdNotFound`。id 不是书签 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/docx-apply-hyperlink.json`。

### insertField

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertField"` |
| `id` | 段落 id（正文或页眉页脚） |
| `instr` | 域指令字符串，必填。去掉首尾空白后不能为空 |
| `result` | 可选。写入的缓存结果。省略则没有结果 run（仍写 `separate`） |

在该段已有内容之后插入一组复杂域：`fldChar begin`（`w:dirty="true"`）、一条 `instrText`（`xml:space="preserve"`）、`fldChar separate`、若给了非空 `result` 则再写一个带 `w:t` 的 run、`fldChar end`。不在库里求值。`instr` 缺或只含空白报 `InvalidField`。找不到段落报 `IdNotFound`。id 不是段落 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### setField

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setField"` |
| `id` | 域 id |
| `instr` | 新的指令字符串，必填。去掉首尾空白后不能为空 |

只改被点名那一域的指令，不改缓存结果。简单域写 `w:instr`。复杂域删掉本层指令区里的旧 `instrText` 以及落在指令区里的嵌套域，在 begin 之后插入一条新的 `instrText`。结果区（separate 与 end 之间）不动。不在库里求值。`instr` 缺或只含空白报 `InvalidField`。找不到报 `IdNotFound`。id 不是域 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteField

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteField"` |
| `id` | 域 id |
| `keep` | 可选。`result`（缺省）或 `instr`。其他值报 `InvalidField` |

`keep` 为 `result`：拆掉该域外壳与指令，留下缓存结果。指令区里的嵌套域一并删掉；结果区里的嵌套域留下。简单域拆掉 `w:fldSimple`，留下子节点。`keep` 为 `instr`：把本层指令变成普通可见文本（`instrText` 改成 `w:t`，简单域则写成一个带指令原文的 run），删掉 begin / separate / end 与结果区。指令区里的嵌套域留下。找不到报 `IdNotFound`。id 不是域 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/docx-apply-field.json`。

### insertToc

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertToc"` |
| `id` | 主文档 `w:body` 的直接子段落 id（插在该段之后），或 `docx:document`（插到正文末尾、`sectPr` 之前） |
| `kind` | 可选。`toc`（目录）、`tof`（图表目录）、`toa`（引文目录）。省略且没有 `instr` 时当作 `toc` |
| `instr` | 可选。域指令原文。给了则按此写入，第一个词必须是 `TOC` 或 `TOA`（大小写不敏感）。省略时：`toc` 为 ` TOC \o "1-3" \h \z \u`，`tof` 为 ` TOC \c "Figure" \h \z \u`，`toa` 为 ` TOA \h` |

在点名段之后插入一个新的 `w:p`（新 `w14:paraId`），里面写一组复杂域：`fldChar begin`（`w:dirty="true"`）、一条 `instrText`、`fldChar separate`、`fldChar end`。不写结果 run，不在库里根据标题算页码。不包 `w:sdt`。表格单元格内、页眉页脚、框内、注内、批注内的段落，以及 `section:{n}`、`toc:{n}`、容器 id、表 id、run id，一律报 `InvalidId`。找不到段落报 `IdNotFound`。不认识的 `kind`、空白 `instr`、或 `instr` 不是 TOC/TOA，报 `InvalidField`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteToc

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteToc"` |
| `id` | 目录 id，必须是 `toc:{n}` |

删掉该目录块。若该 TOC 域落在 `w:sdt` 里，且 `sdtPr` 的 `docPartGallery` 为 `Table of Contents` / `Table of Figures` / `List of Figures` / `Table of Authorities`，并且该 `sdt` 是 `w:body` 的直接子节点，则删掉整个 `sdt`（含「目录」标题段）。否则删掉该域从 begin 到 end 的全部 run（含结果区里的嵌套域），再删掉因此变空、且没有 `sectPr` 的正文段落。不要留下空的目录段。删完后若 `w:body` 里没有 `w:p`，在 `sectPr` 之前补一个空段。某段带分节符且不能整段删掉时，报 `NotImplemented`。找不到报 `IdNotFound`。id 不是 `toc:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

`deleteField` 现有规则不变：对 TOC 域发 `deleteField` 仍只拆外壳。要整块去掉请用 `deleteToc`。

示例见 `docs/examples/docx-apply-insert-toc.json`。

### insertMath

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertMath"` |
| `id` | 段落 id（正文或页眉页脚），或 `docx:document` |
| `text` | 必填，非空。写入一条 `m:r` / `m:t` |
| `kind` | 可选。`inline`（缺省）或 `display` |

写入前在该部件根上补 `xmlns:m`（`http://schemas.openxmlformats.org/officeDocument/2006/math`）。元素名写成 `m:oMath`，不要用宿主段落的 `w:` 前缀。

`kind` 为 `inline`：把 `m:oMath`（内含一条 `m:r`/`m:t`）追加到该段。`id` 为 `docx:document` 时在正文末尾、`sectPr` 之前新插一个只含该 `oMath` 的 `w:p`。

`kind` 为 `display`：在点名段之后新插一个 `w:p`（新 `w14:paraId`），里面是 `m:oMathPara`/`m:oMath`。`id` 为 `docx:document` 时插到正文末尾、`sectPr` 之前。

不把 `text` 解析成分数、根号等结构化 OMML。框内、注内、批注内的段落，以及公式 id、容器 id、表 id、run id、`section:{n}`，一律报 `InvalidId`。找不到段落报 `IdNotFound`。空 `text` 或不认识的 `kind` 报 `InvalidField`。xlsx / pptx 上发此 op 报 `InvalidField`。create 本期不写公式。

### setMath

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setMath"` |
| `id` | 公式 id |
| `text` | 必填，非空 |

用一条新的 `m:r`/`m:t` 整棵替换该 `oMath` 的子节点。结构化分数等会摊平。不改 `kind`，不拆 `oMathPara` 外壳。空 `text` 报 `InvalidField`。找不到报 `IdNotFound`。id 不是公式 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteMath

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteMath"` |
| `id` | 公式 id |

删掉该 `oMath`；若父 `oMathPara` 里已经没有 `oMath`，连 `oMathPara` 一起删。因此变空且无 `sectPr` 的正文段可删掉。表格单元格内的空段留下。删完后若 `w:body` 里没有 `w:p`，在 `sectPr` 之前补一个空段。某段带分节符且不能整段删掉时，报 `NotImplemented`。找不到报 `IdNotFound`。id 不是公式 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

对含本期收入的 `oMath` 的段落发 `setParaText` 报 `NotImplemented`。

示例见 `docs/examples/docx-apply-math.json`。

### insertContentControl

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertContentControl"` |
| `id` | 段落 id（正文或页眉页脚），或 `docx:document` |
| `text` | 必填，非空。写入控件当前值 |
| `kind` | 可选。`plainText`（缺省）、`richText`、`dropDown`、`comboBox`、`date` |
| `tag` | 可选。写入 `w:tag/@w:val` |
| `alias` | 可选。写入 `w:alias/@w:val` |
| `items` | 可选。字符串数组。只用于 `dropDown` / `comboBox`。省略时用 `text` 作为唯一列表项 |
| `fullDate` | 可选。写入 `w:date/@w:fullDate`。只用于 `date` |

在点名段末尾追加一个行内 `w:sdt`（`sdtPr` 含 `w:id`，值为本故事里尚未占用的十进制数字；有 `tag` / `alias` 则写入；类型元素按 `kind`；`sdtContent` 里一条 `w:r`/`w:t`）。元素名跟宿主段落同一 `w:` 前缀。`id` 为 `docx:document` 时在正文末尾、`sectPr` 之前新插一个只含该控件的 `w:p`。不插入块级控件。不写 `w:dataBinding`。不写占位符 `showingPlcHdr`。

框内、注内、批注内的段落，以及控件 id、容器 id、表 id、run id、`section:{n}`，一律报 `InvalidId`。找不到段落报 `IdNotFound`。空 `text`、不认识的 `kind`、给了 `items` 却不是 `dropDown`/`comboBox`、给了 `fullDate` 却不是 `date`、或 `items` 不是非空字符串数组，报 `InvalidField`。xlsx / pptx 上发此 op 报 `InvalidField`。create 本期不写内容控件。

### setContentControl

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setContentControl"` |
| `id` | 内容控件 id |
| `text` | 必填，非空 |
| `tag` | 可选。改 `w:tag/@w:val`；给空串则删掉 `w:tag` |
| `alias` | 可选。改 `w:alias/@w:val`；给空串则删掉 `w:alias` |
| `items` | 可选。只用于 `dropDown` / `comboBox`，整份替换 `w:listItem` |
| `fullDate` | 可选。只用于 `date`，写入 `w:date/@w:fullDate` |

用新文本替换该控件当前值：行内把 `sdtContent` 换成一条 `w:r`/`w:t`；块级只改 `sdtContent` 里第一个 `w:p` 的可见文本，并去掉其余 `w:p`。有 `w:showingPlcHdr` 则删掉。不改 `kind`，不改 `w:id`。带 `w:dataBinding` 报 `NotImplemented`。`kind` 为 `repeatingSection` 报 `NotImplemented`。块级 `sdtContent` 里有 `w:tbl` 报 `NotImplemented`。空 `text`、给了 `items` 却不是下拉/组合框、给了 `fullDate` 却不是日期，报 `InvalidField`。找不到报 `IdNotFound`。id 不是内容控件 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteContentControl

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteContentControl"` |
| `id` | 内容控件 id |
| `keep` | 可选。`content`（缺省，拆掉 `w:sdt` 外壳，把 `sdtContent` 的子节点留在原位）或 `none`（连内容一起删） |

`keep` 为 `none` 时：因此变空且无 `sectPr` 的正文段可删掉。表格单元格内的空段留下。删完后若 `w:body` 里没有 `w:p`，在 `sectPr` 之前补一个空段。某段带分节符且不能整段删掉时，报 `NotImplemented`。找不到报 `IdNotFound`。id 不是内容控件 id 报 `InvalidId`。不认识的 `keep` 报 `InvalidField`。xlsx / pptx 上发此 op 报 `InvalidField`。

对含本期收入的行内内容控件的段落发 `setParaText` 报 `NotImplemented`。块级控件里的内层段落仍可 `setParaText`。

示例见 `docs/examples/docx-apply-sdt.json`。

### insertNumbering

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertNumbering"` |
| `id` | 必须是 `docx:document` |
| `kind` | `"decimal"` 或 `"bullet"` |

写入一份抽象定义（9 级 `hybridMultilevel`）加一个 `w:num` 实例。没有 `/word/numbering.xml` 则新建该部件、主文档关系与内容类型 Override。`decimal` 的 `lvlText` 为 `%1.`、`%1.%2.` 这类累计小数；`bullet` 用 Symbol 字体，各级 `lvlText` 轮换实心点、`o` 与节号。apply 不返回新 id；随后 inspect 按 `w:num` 文档顺序，新实例在名单末尾。`kind` 缺或不认识报 `InvalidField`。id 不是 `docx:document` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。create 本期不写编号。

### setNumbering

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setNumbering"` |
| `id` | 抽象编号 id |
| `ilvl` | 级别，0 到 8，必填 |
| `numFmt` | 可选。`decimal`、`bullet`、`lowerLetter`、`upperLetter`、`lowerRoman` 或 `upperRoman` |
| `start` | 可选。起始值，从 1 起的整数 |
| `lvlText` | 可选。该级 `w:lvlText/@w:val` |

`numFmt`、`start`、`lvlText` 至少给一个。只改被点名抽象定义上已有的 `w:lvl`，不自动建级，也不改该实例上的 `w:lvlOverride`。不认识的 `numFmt`、`ilvl` 超出 0–8、或该抽象定义没有这一级，报 `InvalidField`。找不到抽象定义报 `IdNotFound`。id 不是抽象编号 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### setParaNumbering

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setParaNumbering"` |
| `id` | 段落 id（正文或页眉页脚） |
| `num` | `num:{n}`，或 `null` 表示去掉该段 `w:numPr` |
| `ilvl` | 可选。绑定级别，缺省 `0`，取值 0 到 8 |

`num` 为实例 id 时写入（或改写）`w:numPr`，插在 `pStyle` 之后。找不到该编号实例报 `IdNotFound`。`num` 为 `null` 时去掉 `numPr`，不要求 numbering 部件存在。缺 `num` 报 `InvalidField`。`ilvl` 超出 0–8 报 `InvalidField`。找不到段落报 `IdNotFound`。id 不是正文或页眉页脚段落 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/docx-apply-numbering.json`。

### setStyle

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setStyle"` |
| `id` | 样式 id，例如 `style:Normal` |
| `font` | 可选。写入 `w:rFonts` 的 `ascii` / `hAnsi` / `cs` |
| `eastAsiaFont` | 可选。写入 `w:rFonts` 的 `eastAsia` |
| `sz` | 可选。半磅整数，与 `w:sz/@w:val` 相同，2 到 3276 |
| `bold` | 可选。布尔。写入 `w:b` / `w:bCs`；`false` 显式 `val="0"` |
| `color` | 可选。六位 RGB 十六进制，不带 `#`。写入 `w:color/@w:val`（大写），并去掉 theme 属性 |
| `before` | 可选。段前距，单位 twip，非负整数 |
| `after` | 可选。段后距，单位 twip，非负整数 |

`font`、`eastAsiaFont`、`sz`、`bold`、`color`、`before`、`after` 至少给一个。只改被点名的段落样式（`w:type` 为 `paragraph` 或缺省）。字符 / 表格 / 编号样式报 `InvalidField`。空 `font` / `eastAsiaFont`、不是六位十六进制的 `color`、`sz` 越界报 `InvalidField`。不改 `outlineLvl`、`basedOn`、`name`。不新建样式。找不到样式报 `IdNotFound`。id 不是 `style:{styleId}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### setParaStyle

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setParaStyle"` |
| `id` | 段落 id（正文或页眉页脚） |
| `style` | `style:{styleId}`，或 `null` 表示去掉该段 `w:pStyle` |

`style` 为样式 id 时写入（或改写）`w:pStyle`，放在 `pPr` 子元素最前。目标必须已在 `styles.xml` 里，且是段落样式。字符 / 表格 / 编号样式报 `InvalidField`。找不到该样式报 `IdNotFound`。`style` 为 `null` 时去掉 `pStyle`，不要求 styles 部件存在。缺 `style` 报 `InvalidField`。找不到段落报 `IdNotFound`。id 不是正文或页眉页脚段落 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/docx-apply-style.json`。

### setParaFormat

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setParaFormat"` |
| `id` | 段落 id：正文、页眉页脚、框内、注内或批注内 |
| `align` | 可选。写入 `w:jc/@w:val`。允许 `left`、`center`、`right`、`both`、`distribute`、`start`、`end`。`null` 表示去掉 `w:jc` |
| `left` | 可选。左缩进，单位 twip，非负整数。写入 `w:ind/@w:left`，并去掉 `w:start`。`null` 表示去掉 `left` 与 `start` |
| `right` | 可选。右缩进，单位 twip。写入 `w:ind/@w:right`，并去掉 `w:end`。`null` 表示去掉 `right` 与 `end` |
| `firstLine` | 可选。首行缩进，单位 twip。写入 `w:ind/@w:firstLine`，并去掉 `hanging`。`null` 表示去掉 `firstLine` |
| `hanging` | 可选。悬挂缩进，单位 twip。写入 `w:ind/@w:hanging`，并去掉 `firstLine`。`null` 表示去掉 `hanging` |
| `before` | 可选。段前距，单位 twip。写入 `w:spacing/@w:before`。`null` 表示去掉该属性 |
| `after` | 可选。段后距，单位 twip。写入 `w:spacing/@w:after`。`null` 表示去掉该属性 |
| `line` | 可选。行距，单位 twip。写入 `w:spacing/@w:line`。`null` 表示去掉该属性 |
| `lineRule` | 可选。`auto`、`exact` 或 `atLeast`。写入 `w:spacing/@w:lineRule`。`null` 表示去掉该属性 |

`align`、`left`、`right`、`firstLine`、`hanging`、`before`、`after`、`line`、`lineRule` 至少给一个。`firstLine` 与 `hanging` 不能同时写成非 `null` 的数字。不认识的 `align` / `lineRule` 报 `InvalidField`。只改该段自己的 `w:pPr`，不改 `styles.xml`。属性删光后去掉空的 `w:ind` / `w:spacing`。找不到段落报 `IdNotFound`。id 不是段落 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### setRunFormat

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setRunFormat"` |
| `id` | run id，例如 `para:3F2A1B00/run:1` |
| `bold` | 可选。布尔。写入 `w:b` / `w:bCs`；`false` 显式 `val="0"` |
| `italic` | 可选。布尔。写入 `w:i` / `w:iCs`；`false` 显式 `val="0"` |
| `underline` | 可选。`ST_Underline` 常用值：`single`、`none`、`double`、`thick`、`dotted`、`dash`、`wave`、`words`、`dottedHeavy`、`dashedHeavy`、`dashLong`、`dashLongHeavy`、`dotDash`、`dashDotHeavy`、`dotDotDash`、`dashDotDotHeavy`、`wavyHeavy`、`wavyDouble`。`null` 表示去掉 `w:u` |
| `sz` | 可选。半磅整数，2 到 3276。写入 `w:sz` / `w:szCs` |
| `color` | 可选。六位 RGB 十六进制，不带 `#`。写入 `w:color/@w:val`（大写），并去掉 theme 属性 |
| `font` | 可选。写入 `w:rFonts` 的 `ascii` / `hAnsi` / `cs` |
| `eastAsiaFont` | 可选。写入 `w:rFonts` 的 `eastAsia` |

`bold`、`italic`、`underline`、`sz`、`color`、`font`、`eastAsiaFont` 至少给一个。只改被点名的那一个 `w:r` 的 `w:rPr`，不拆 run，不改 `styles.xml`。不认识的 `underline`、空 `font` / `eastAsiaFont`、不是六位十六进制的 `color`、`sz` 越界报 `InvalidField`。找不到 run 报 `IdNotFound`。id 不是 run id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/docx-apply-format.json`。

### setTextBoxLink

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTextBoxLink"` |
| `id` | 框 id |
| `next` | 后一框的 id；`null` 表示断开与后一框的链接 |

把 `id` 接到 `next` 前面，或断开。两框必须在同一部件（同一主文档，或同一页眉页脚部件）。跨部件链接报 `InvalidField`。断开时后一框变成独立故事（自己的 `wps:txbx`）。`next` 可以是没有 `w:txbxContent` 的溢出后继框（Word 常只留 `wps:linkedTxbx`）。xlsx / pptx 上发此 op 报 `InvalidField`。

`setParaText` 接受框内段落 id、注内段落 id 与批注内段落 id。对框 id、注容器 id 或批注容器 id 发 `setParaText` 报 `InvalidId`。

### insertFootnote / insertEndnote

| 字段 | 含义 |
| --- | --- |
| `op` | `"insertFootnote"` 或 `"insertEndnote"` |
| `id` | 宿主段落 id：正文、页眉页脚或框内段落 |
| `text` | 可选。新注第一段的可见文本，省略当作 `""` |

在该段末插入一个新 run，内含 `w:footnoteReference` 或 `w:endnoteReference`。不提供字符偏移。分配该注部件里尚未占用的 `w:id`（十进制，跳过已有用户注和分隔符，常见从 `2` 起）。没有注部件则新建 `/word/footnotes.xml` 或 `/word/endnotes.xml`、主文档关系、内容类型 Override，并写入 `separator`（id `0`）与 `continuationSeparator`（id `1`）；`settings.xml` 里补上指向这两条分隔符的 `footnotePr` / `endnotePr` 引用（§17.11）。新用户注至少一段，第一段含对应的 `w:footnoteRef` / `w:endnoteRef` 标记 run，再跟 `text`。不写自定义标记（`customMarkFollows`）。对图 id、容器 id、注 id 或 `section:{n}` 发此 op 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteFootnote / deleteEndnote

| 字段 | 含义 |
| --- | --- |
| `op` | `"deleteFootnote"` 或 `"deleteEndnote"` |
| `id` | 用户注 id，例如 `footnote:2` |

去掉主文档、页眉、页脚和文本框里所有匹配的引用 run，再去掉注部件里该条 `w:footnote` / `w:endnote`。不得删分隔符。已经不存在报 `IdNotFound`。id 不是用户注 id 报 `InvalidId`。注内图按图片规则：该媒体不再被任何关系引用则删除部件。删光用户注后仍保留注部件和两条分隔符，不拆掉整个 part。xlsx / pptx 上发此 op 报 `InvalidField`。

### setNoteNumbering

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setNoteNumbering"` |
| `id` | `docx:document` 改整篇：写 `settings.xml` 的 `footnotePr` / `endnotePr`，并写每一节 `sectPr` 里同名元素。Word 16.0 的 `Footnotes.NumberStyle` 读的是节属性；只写 settings 时打开后仍显示阿拉伯数字。`section:{n}` 只改该节 `sectPr` |
| `kind` | `footnote` 或 `endnote` |
| `numFmt` | 可选。`decimal`、`upperRoman`、`lowerRoman`、`upperLetter`、`lowerLetter`、`chicago`（`ST_NumberFormat` 的常用子集，§17.11.18） |
| `numStart` | 可选。起始值，正整数（§17.11.20） |
| `numRestart` | 可选。`continuous`、`eachSect`、`eachPage`（`ST_RestartNumber`，§17.11.19） |

`numFmt`、`numStart`、`numRestart` 至少给一个，否则报 `InvalidField`。只改被点名的属性，其余编号属性保留。不重算页面上已经画出来的序号（那是渲染，Word 打开时自己画）。不认识的枚举报 `InvalidField`。id 不是 `docx:document` 或 `section:{n}` 报 `InvalidId`。节不存在报 `IdNotFound`。xlsx / pptx 上发此 op 报 `InvalidField`。

写入 `sectPr` 时，`footnotePr` / `endnotePr` 必须落在 `EG_SectPrContents` 顺序里：页眉页脚引用之后、`type` / `pgSz` 等之前。写在 `pgSz` 后面时 Word 会当没看见。`CT_FtnDocProps` 里 `numFmt` 必须写在特殊 `footnote` / `endnote` 引用之前。

### insertComment

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertComment"` |
| `start` | docx：范围起始段落 id：正文、页眉页脚、框内或注内。pptx 上出现报 `InvalidField` |
| `end` | docx：范围结束段落 id，可与 `start` 相同。pptx 上出现报 `InvalidField` |
| `id` | pptx：页 id `slide:{n}`。docx 上出现报 `InvalidField` |
| `x` / `y` | pptx 可选。画布坐标，EMU，非负整数，必须成对。docx 上出现报 `InvalidField` |
| `text` | 可选。新批注第一段的可见文本，省略当作 `""` |
| `author` | 可选。省略当作 `"ooxp"` |
| `initials` | 可选。省略当作 `"oo"` |
| `date` | 可选。`xsd:dateTime` 字符串。docx 省略则不写 `w:date`；pptx 省略则写入当前 UTC（无尾 `Z`） |

`start` 与 `end` 必须在同一部件，且 `start` 在文档顺序上不得晚于 `end`，否则报 `InvalidField`。不提供字符偏移。在 `start` 段 `w:pPr` 之后插入 `w:commentRangeStart`；在 `end` 段已有内容之后插入 `w:commentRangeEnd`，再跟一个含 `w:commentReference` 的 run（`rStyle` 为 `CommentReference`）。跨段时中间段落整段落在范围内，不另写标记。

分配尚未占用的 `w:id`（十进制，常见从 `0` 起）。没有 `/word/comments.xml` 则新建该部件、主文档关系、内容类型 Override。同时按 Word 16.0 最小集补 `/word/commentsExtended.xml` 与 `/word/people.xml`（已有则改，没有则建）。新 `w:comment` 至少一段，带新的 `w14:paraId`，第一段含 `w:annotationRef`，段落样式引用 `CommentText`，再跟 `text`。不在 `styles.xml` 里新建这些样式。`commentEx/@paraId` 写成该批注最后一段的 `paraId`，`done` 为 `0`。`people` 里没有该 `author` 则追加一条。`commentsIds` / `commentsExtensible` 只在文件里已有时给这条根批注补一项；没有这两份部件则不新建。

对图 id、容器 id、注 id、批注 id 或 `section:{n}` 作 `start` / `end` 报 `InvalidId`。xlsx 上发此 op 报 `InvalidField`。pptx 不认 `start` / `end`；出现这两个字段报 `InvalidField`。pptx 的 `id` 必须是 `slide:{n}`，对备注容器 id、图 id、形状 id 或 `pptx:presentation` 报 `InvalidId`。可选 `x` / `y` 为 EMU 非负整数，必须成对出现或成对省略；只给一个时报 `InvalidField`。缺坐标则放到该页空白处，规则与 pptx `insertPicture` 相同。写入该页现代批注部件（MS-PPTX 2018/8 `p188:cmLst`，内容类型 `application/vnd.ms-powerpoint.comments+xml`），幻灯片 `.rels` 的 Type 为 `http://schemas.microsoft.com/office/2018/10/relationships/comments`（必须比完整 URI，不要把 2006 旧式 comments 关系当成现代批注）。没有则新建 comments 部件，并在幻灯片 `p:extLst` 写入 `p188:commentRel`（uri `{6950BFC3-D8DA-4A85-94F7-54DA5524770B}`）。没有 `/ppt/authors.xml` 则新建（内容类型 `application/vnd.ms-powerpoint.authors+xml`），presentation `.rels` 的 Type 为 `http://schemas.microsoft.com/office/2018/10/relationships/authors`。按 `name` 加 `initials` 复用已有 `p188:author`；否则新建，`providerId` 为 `None`，`userId` 取新 GUID 的十六进制前 16 位。`author` / `initials` 缺省 `"ooxp"` / `"oo"`。`date` 省略则写入当前 UTC `xsd:dateTime`（无尾 `Z`，与 PowerPoint 16 另存一致）。新 `p188:cm/@id` 为带花括号的 GUID。`pc:sldMk/@sldId` 取该页 `p:sldId/@id`（无前缀那一项）。不写 ECMA-376 §19.4 旧式 `commentAuthors`。同一页多条批注共用一个 comments 部件。`create` 本期不写批注。

### replyComment

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"replyComment"` |
| `id` | 父批注 id。docx 为 `comment:0`；pptx 为 `slide:{n}/comment:{guid}` |
| `text` | 可选。新回复第一段的可见文本，省略当作 `""` |
| `author` / `initials` / `date` | 与 `insertComment` 相同 |

新增一条 `w:comment` 和新的 `commentEx`，`paraIdParent` 为父批注当前最后一段的 `paraId`。在父批注已有的范围上再套一套相同范围的标记（`commentRangeStart` 插在父 start 之后，`commentRangeEnd` 与 `commentReference` 插在父 `commentReference` 所在 run 之后），与 Word 16 `Replies.Add` 一致；`rStyle` 抄父引用 run。找不到父批注的范围标记时仍写 `comment` 与 `commentEx`，不报错。inspect 的 `start` / `end` 因此与线程根相同。父批注最后一段没有 `w14:paraId` 时报 `InvalidField`。父 id 不是批注容器 id 报 `InvalidId`。找不到父批注报 `IdNotFound`。对 `kind` 为 `ink` 的批注发此 op 报 `NotImplemented`。xlsx 上发此 op 报 `InvalidField`。pptx 的 `id` 必须是父批注容器 id（`slide:{n}/comment:{guid}`），不能是回复 id；对回复 id 报 `InvalidId`。写入该 `p188:cm` 的 `p188:replyLst`。新回复 `id` 为 `{父 id}/reply:{guid}`。找不到父批注报 `IdNotFound`。对 `kind` 为 `ink` 的批注发此 op 报 `NotImplemented`。

### deleteComment

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteComment"` |
| `id` | 批注 id。docx 为 `comment:0`；pptx 为 `slide:{n}/comment:{guid}` 或回复 id |

去掉该批注在主文档、页眉、页脚、文本框和注里所有匹配的范围标记与 `commentReference`，再去掉 `comments.xml` 里该条。子回复级联删除（`paraIdParent` 链指向它的都删），清单里验回复不残留。已经不存在报 `IdNotFound`。id 不是批注容器 id 报 `InvalidId`。批注内图按图片规则收。删光批注后仍保留 `comments.xml` 空壳和已有的 extended / people 部件，不拆掉整个 part。xlsx 上发此 op 报 `InvalidField`。pptx 的 `id` 可以是 `slide:{n}/comment:{guid}` 或回复 id。删父 `p188:cm` 即级联丢掉其 `replyLst`。删回复只去掉对应 `p188:reply`。已经不存在报 `IdNotFound`。id 不是上述两种报 `InvalidId`。删光后仍保留空的 `p188:cmLst` 与 `authors.xml` 空壳，不拆 part。墨迹批注可以删。

### setCommentText

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setCommentText"` |
| `id` | 批注容器 id。docx 为 `comment:0`；pptx 为 `slide:{n}/comment:{guid}` 或回复 id |
| `text` | 新的可见纯文本，字符串，可空 |

只改该 `w:comment` 的可见文本，不改范围标记。留下第一段（含 `annotationRef`），按 `setParaText` 的规则改这段的可见 `w:t`，删掉后面多出来的 `w:p`。随后把 `commentEx/@paraId` 和指向旧最后一段的 `paraIdParent` 改成留下的那一段。id 不是批注容器 id 报 `InvalidId`。对批注内段落 id 请用 `setParaText`。对 `kind` 为 `ink` 的批注报 `NotImplemented`。xlsx 上发此 op 报 `InvalidField`。pptx 的 `id` 可以是批注容器 id 或回复 id。只改对应 `p188:txBody` 的可见文本，规则与 `setShapeText` 相同。对 `kind` 为 `ink` 的批注报 `NotImplemented`。

### setCommentMeta

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setCommentMeta"` |
| `id` | 批注容器 id |
| `author` | 可选 |
| `initials` | 可选 |
| `date` | 可选。`xsd:dateTime` 字符串 |

`author`、`initials`、`date` 至少给一个，否则报 `InvalidField`。只改被点名的属性，其余不动。改 `author` 时若 `people.xml` 里还没有这个作者则补一条，不删旧作者。id 不是批注容器 id 报 `InvalidId`。xlsx 上发此 op 报 `InvalidField`。pptx 的 `id` 可以是批注容器 id 或回复 id。改作者或缩写时按新的 `name` / `initials` 指向（必要时新建）一条 `p188:author`，只改该 `cm` 或 `reply` 的 `authorId`，不改共享的那条 author 记录，以免误伤同作者其它批注。对 `kind` 为 `ink` 的批注报 `NotImplemented`。

### setCommentDone

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setCommentDone"` |
| `id` | 批注容器 id |
| `done` | 布尔。`true` 写成 `commentEx/@done="1"`，`false` 写成 `"0"` |

只改被点名这一条的 `done`，不自动改回复。没有 `commentsExtended` 则按 Word 16 最小集新建，并为文件里每条批注补 `commentEx`（默认 `done="0"`），再改被点名的那条。父批注最后一段没有 `w14:paraId` 时报 `InvalidField`。xlsx 上发此 op 报 `InvalidField`。pptx 的 `id` 可以是批注容器 id 或回复 id。`done: true` 写 `status="resolved"`；`false` 去掉 `status`（与另存样例默认无 status、即 active 一致）。对 `kind` 为 `ink` 的批注报 `NotImplemented`。

### 图表 op 的共同限制

`type` 为 `other` 或 `chartex`，或 `flags` 含 `secondaryAxis`、`trendline`、`errorBars` 时，除 `deleteChart`、`setChartSize`、`setChartPos`、`setChartWrap`、`setChartAnchor` 外一律报 `NotImplemented`，不写文件。xlsx 没有 `setChartWrap` / `setChartPos`，对应例外是 `deleteChart`、`setChartSize`、`setChartAnchor`。pptx 没有 `setChartWrap` / `setChartAnchor`，对应例外是 `deleteChart`、`setChartSize`、`setChartPos`。`flags` 含 `pivot` 时：`setChartSeries`、`addChartSeries`、`deleteChartSeries`、`setChartType` 报 `NotImplemented`；`setChartTitle`、`setChartAxisTitle`、`setChartLegend`、`setChartDataLabels`、`deleteChart`、`setChartSize`、`setChartPos`、`setChartWrap`、`setChartAnchor` 可以。`type` 为 `combo` 不再整张锁死，系列留在原来的包裹里改。xlsx 上发 `setChartWrap` / `setChartPos` 报 `InvalidField`。docx 上发 `setChartAnchor` / `setChartPos` 报 `InvalidField`。pptx 上发 `setChartWrap` / `setChartAnchor` 报 `InvalidField`。`insertChart` 不能插入透视图表。

docx 改系列数据时必须同时改 chart 部件里的 cache（`c:numCache` / `c:strCache` / `c:numLit` / `c:strLit`）以及 embedding 工作簿里公式指向的单元格。没有 embedding 则只改字面量。不在库里求值公式，不改 `c:f` 里工作表名以外的结构，除非新数据的点数与公式区域长度不同：那时把公式区域扩到或收到新长度，并改 embedding 里对应格子。未点名的系列不动。

xlsx 没有 embedding。有可解析的连续 A1 `c:f` 时：把数组写入这些格子（按 `setCell` 语义），点数变了就按原起点扩或收公式区域，并回写 cache。命名区域、三维引用、不连续区域报 `NotImplemented`。没有 `c:f`（只有 lit）时只改 lit / cache，不新造单元格区域。写入单元格时走与 `setCell` 同一套重算标记。未点名的系列不动。

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

`name`、`categories`、`values`、`sizes` 至少给一个，否则报 `InvalidField`。只给的字段改，其余保留。`categories` 与 `values` 都给时长度可以不同。对非气泡系列给 `sizes` 报 `InvalidField`。找不到该系列报 `IdNotFound`。id 不是系列 id 报 `InvalidId`。

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

在 chart 部件末尾追加一条 `c:ser`，`c:idx` / `c:order` 取尚未占用的最小值。样式能从最后一条系列抄则抄，抄不了就用最小默认。docx 有 embedding 则在工作表右侧追加一列。xlsx 能从已有系列的 `c:f` 推出右侧相邻空列则写入；推不出报 `NotImplemented`。不能用本 op 把单类型图加成组合图。id 不是图表 id 报 `InvalidId`。

### deleteChartSeries

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteChartSeries"` |
| `id` | 系列 id |

删掉该 `c:ser`。docx 若能从公式定位则同步 embedding 里对应列。xlsx 不删单元格。删完后一条都不剩时报 `InvalidField`（要整张去掉请用 `deleteChart`）。找不到报 `IdNotFound`。id 不是系列 id 报 `InvalidId`。

### setChartType

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setChartType"` |
| `id` | 图表 id |
| `type` | `bar`、`col`、`line`、`pie`、`area`、`scatter`、`doughnut`、`radar` 或 `bubble` |

不认识的枚举、或值为 `combo`，报 `InvalidField`。只改系列的包裹元素（`c:barChart` 等）和 `c:barDir`。`bar` 与 `col` 只改 `barDir`。`bar` / `col` / `line` / `area` / `radar` 互换时留下 `c:ser`。`pie` ↔ `doughnut` 只换包裹，必须恰好一条系列，否则 `NotImplemented`。转成 `pie` 或 `doughnut` 时必须恰好一条系列，否则 `NotImplemented`。与 `scatter` 互转：分类当 `xVal`、数值当 `yVal`，或反过来；系列结构对不上则 `NotImplemented`。`scatter` → `bubble`：留下 `xVal` / `yVal`，补上与 `yVal` 等长的 `bubbleSize`（值均为 `1`）。`bubble` → `scatter`：丢掉 `bubbleSize`。组合图收到单一类型时，把所有包裹收成那一种；对不上（例如多系列转饼）则 `NotImplemented`。单一类型不能经本 op 变成 `combo`，请用 `insertChart`。不要写出 Word 要修复的包。id 不是图表 id 报 `InvalidId`。找不到报 `IdNotFound`。

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

### insertChart

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertChart"` |
| `id` | docx：宿主段落 id（正文、页眉页脚、框内、注内或批注内段落）。xlsx：工作表 id `sheet:{name}`。pptx：页 id `slide:{n}` |
| `type` | `bar`、`col`、`line`、`pie`、`area`、`scatter`、`doughnut`、`radar`、`bubble` 或 `combo` |
| `title` | 可选。图表标题 |
| `series` | 必须。非空数组。每项可含 `name`、`categories`、`values`、`sizes`、`type`；`values` 必填且非空 |
| `kind` | 可选。docx：`inline` 或 `anchor`，缺省 `anchor`。xlsx / pptx 上出现报 `InvalidField` |
| `wrap` | 可选。docx 浮动时的绕排，缺省 `square`。行内必须省略或为 `null`。xlsx / pptx 上出现报 `InvalidField` |
| `cx` / `cy` | 可选。EMU，正整数。docx 都省略则用 `5486400` × `3200400`。xlsx 规则与 `insertPicture` 相同。pptx 都省略则用同一组缺省。只给一个报 `InvalidField` |
| `x` / `y` | pptx 可选。画布坐标，EMU，非负整数。必须成对。缺则放到该页空白处，规则与 pptx `insertPicture` 相同。docx / xlsx 上出现报 `InvalidField` |
| `from` | xlsx 必填。图的左上角单元格，A1。docx / pptx 上出现报 `InvalidField` |
| `to` | xlsx 且 `anchor` 为 `twoCell` 时必填。docx / pptx 上出现报 `InvalidField` |
| `anchor` | xlsx 可选。`oneCell`（缺省）或 `twoCell`。不认 `absolute`。docx / pptx 上出现报 `InvalidField` |
| `fromOffX` / `fromOffY` / `toOffX` / `toOffY` | xlsx 可选。与 `insertPicture` 相同。docx / pptx 上出现报 `InvalidField` |
| `dataFrom` | xlsx 必填。写到该表上的数据表左上角，A1。第 1 行是系列名，第 1 列是分类，数值向右。docx / pptx 上出现报 `InvalidField` |

docx：在该段末插入 `w:drawing`，默认浮动。新建 `/word/charts/chartN.xml`（`N` 从 1 起尚未占用）、chart 的 `.rels`、一份最小 embedding xlsx（工作表名 `Sheet1`，分类从 `A2` 起，系列名在第 1 行，数值从 `B2` 起向右），以及宿主部件关系（主文档、页眉页脚、`comments.xml`、`footnotes.xml` 或 `endnotes.xml`）、`[Content_Types].xml` Override。chart 部件仍写在 `/word/charts/`。chart 的 Content Type 为 `application/vnd.openxmlformats-officedocument.drawingml.chart+xml`。关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart`。embedding 关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/package`。`wp:docPr/@id` 取该部件已有 `docPr` 的最大整数加 1，至少为 1。不强制写 chart colors / style 部件，也不强制套 `mc:AlternateContent`。对图 id、图表 id、系列 id、容器 id、框 id、注容器 id、批注容器 id 或 `section:{n}` 发此 op 报 `InvalidId`。找不到段落报 `IdNotFound`。

xlsx：找不到该表报 `IdNotFound`。`id` 不是 `sheet:{name}` 报 `InvalidId`。该表不是 `worksheet` 报 `InvalidField`。必须给 `from` 与 `dataFrom`，否则报 `MissingField`。`from` / `to` / `dataFrom` 不是合法 A1 报 `InvalidRange`。`anchor` 规则与 `insertPicture` 相同。按 `setCell` 语义把数据表写到 `dataFrom` 起的格子（已有内容会被覆盖）。新建 `/xl/charts/chartN.xml`、chart 的 `.rels`（不写 embedding），`c:f` 指向这些格子。没有 drawing 则按插图同一套规则新建 drawing，并在锚点下写 `xdr:graphicFrame`（须带空的 `xdr:clientData`）。`xdr:cNvPr/@id` 取该表已有全部 `cNvPr`（图与框）的最大整数加 1，至少为 1。对图 id、图表 id、系列 id 或单元格 id 发此 op 报 `InvalidId`。不要用本 op 新建图表工作表。

pptx：找不到该页报 `IdNotFound`。`id` 必须是 `slide:{n}`。对图 id、形状 id、图表 id 或 `pptx:presentation` 发此 op 报 `InvalidId`。写入该页 `p:spTree` 一个 `p:graphicFrame`，新建 `/ppt/charts/chartN.xml`、chart 的 `.rels`、一份最小 embedding（`/ppt/embeddings/Microsoft_Excel_WorksheetN.xlsx`，规则与 docx 相同）。`p:cNvPr/@id` 取该页已有全部 `cNvPr` 的最大整数加 1，至少为 2。`x` / `y` 成对规则与 `insertPicture` 相同。不写演示文稿里的工作表。

`type` 不认识或 `series` 为空报 `InvalidField`。`pie` 与 `doughnut` 只允许恰好一条系列，否则报 `InvalidField`。`bubble` 的 `sizes` 省略则写成与 `values` 等长的 `1`。`type` 为 `combo` 时，每条系列必须带 `type`（`bar`、`col`、`line`、`area` 或 `scatter`），且至少出现两种不同包裹，否则报 `InvalidField`；`combo` 的系列 `type` 不能是 `pie`、`doughnut`、`radar`、`bubble`。对非组合图的系列给 `type` 报 `InvalidField`。

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
      "series": [{ "name": "S1", "categories": ["a", "b"], "values": [1, 2] }]
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
| `type` / `title` / `series` | 与 `insertChart` 相同 |
| `cx` / `cy` | 可选。`absoluteAnchor` 的 EMU 尺寸。都省略则用与 `insertChart` 相同的缺省。只给一个报 `InvalidField` |

xlsx：`id` 不是 `xlsx:workbook` 报 `InvalidId`。`name` 非法报 `InvalidField`。与已有表重名（不分大小写）报 `InvalidField`。找不到 `dataSheet` 报 `IdNotFound`。`dataSheet` 不是 `worksheet`，或省略时工作簿里没有 `worksheet`，报 `InvalidField`。出现 `from` / `to` / `anchor` / 偏移 / `kind` / `wrap` 报 `InvalidField`。类型与系列限制与 `insertChart` 相同。按 `setCell` 语义把数据表写到点名的普通表上，不删原有其它格子。新建 `/xl/chartsheets/sheetN.xml`（CT `application/vnd.openxmlformats-officedocument.spreadsheetml.chartsheet+xml`）、一份 drawing、`/xl/charts/chartN.xml`，并在 `workbook.xml` 的 `sheets` 末尾加一项。关系 Type 以 `/chartsheet` 结尾。`chartsheet` 必须带 `<sheetViews>` 与 `<drawing r:id>`（CT_Chartsheet，§18.3.1.12）。那一张框写 `xdr:absoluteAnchor`，从 (0,0) 起，带空的 `xdr:clientData`。`xdr:cNvPr/@id` 至少为 2。不写 embedding。不在同一张图表工作表上再插第二张。`create` 的 `sheets[].kind` 仍拒绝。docx / pptx 上发此 op 报 `InvalidField`。

### deleteChartSheet

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteChartSheet"` |
| `id` | `sheet:{name}`，且该表 `kind` 为 `chartsheet` |

找不到该表报 `IdNotFound`。id 不是工作表 id 报 `InvalidId`。对 `worksheet` / `dialogsheet` 发此 op 报 `InvalidId`。删掉工作簿里最后一张表报 `InvalidField`。去掉 workbook 表项、chartsheet 部件及其 drawing / chart（无人引用则删），以及对应关系与内容类型。不删 `dataFrom` 那些单元格。docx / pptx 上发此 op 报 `InvalidField`。

### deleteChart

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteChart"` |
| `id` | 图表 id |

docx：删掉对应 `w:drawing`。若在 `mc:AlternateContent` 的 Choice 里，整段 `AlternateContent` 删掉。所在 `w:r` 若删完后只剩 `w:rPr` 或空，把该 run 也删掉。重新解析关系之后，若该 chart 部件、专属 colors / style、embedding 再无人引用，删掉这些部件、关系条目和对应内容类型。

xlsx：按图表 id 删掉包含该 `xdr:graphicFrame` 的整个锚点。不删数据单元格。drawing 部件删空后的清理与 `deletePicture` 相同。chart 部件无人引用则删。对图表工作表上那张（或仅剩一张）图表发此 op 报 `NotImplemented`，不写文件；要删整张表请用 `deleteChartSheet`。

找不到报 `IdNotFound`。id 不是图表 id 报 `InvalidId`。

### setChartSize

字段与 `setPictureSize` 相同，`id` 为图表 id。docx 改宿主 `wp:extent`（以及 `a:ext` 若有），规则与图相同。xlsx 只改 `oneCell` / `absolute` 的 `xdr:ext`（以及存在的 `a:xfrm/a:ext`）；对 `twoCell` 报 `NotImplemented`。pptx 写入 graphicFrame 的 `p:xfrm/a:ext`。

### setChartPos

字段与 `setPicturePos` 相同，`id` 为 pptx 图表 id。写入 graphicFrame 的 `p:xfrm/a:off`。docx / xlsx 上发此 op 报 `InvalidField`。

### setChartWrap

字段与 `setPictureWrap` 相同，`id` 为图表 id。改宿主 `wp:inline` / `wp:anchor` 与绕排，规则与图相同。xlsx / pptx 上发此 op 报 `InvalidField`。xlsx 改锚点请用 `setChartAnchor`。

### setChartAnchor

字段与 `setPictureAnchor` 相同，`id` 必须是 xlsx 图表 id。改包含该 `xdr:graphicFrame` 的锚点，规则与 `setPictureAnchor` 相同（留下 `xdr:graphicFrame` 与 `xdr:clientData`）。docx / pptx 上发此 op 报 `InvalidField`。

### SmartArt op 的共同限制

xlsx / pptx 上发任何 SmartArt op 报 `InvalidField`。改点只动 data 部件里对应 `dgm:pt` 的 `dgm:t`，以及必要的 `cxnLst`。不自己算版式坐标、连接线或形状几何。包里若有 diagram drawing 缓存（常见 `/word/diagrams/drawingN.xml`，关系 Type 含 `diagramDrawing`）：改字时同步改缓存里对应形状的可见 `a:t`；对不上 modelId 则删掉该缓存部件及其关系，让 Word 打开时重排。插入不写这份缓存。

库内模板写在 `crates/ooxp` 资源里，不从 Office 安装目录抄。两份 layout 短名与 uniqueId：

| 短名 | uniqueId |
| --- | --- |
| `list` | `urn:microsoft.com/office/officeart/2005/8/layout/default` |
| `org` | `urn:microsoft.com/office/officeart/2005/8/layout/orgChart1` |

家族按 uniqueId 最后一个 `/` 之后的末段判定（大小写不敏感）：`default`，或以 `hList` / `vList` 开头，属 `list`；`orgChart1`，或以 `orgChart` 开头，属 `org`。其余 uniqueId 不属于这两族。

### setSmartArtText

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setSmartArtText"` |
| `id` | 点 id，例如 `para:3F2A1B00/smartdiagram:4/pt:2` |
| `text` | 新的可见纯文本，字符串，可空。`\n` 写成 `a:br` |

只改该点 `dgm:t` 的可见文字。留下第一个含 `a:t` 的 run 收下全部新文本，规则对齐 `setShapeText` 的可见替换（不整段重建 `dgm:t` 的 `a:bodyPr`）。没有 `dgm:t` 则补一份最小文本体。找不到该点报 `IdNotFound`。id 不是点 id 报 `InvalidId`。对 `doc` / `pres` / `parTrans` / `sibTrans` 点发此 op 报 `InvalidId`（它们不在树上）。

### addSmartArtPoint

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"addSmartArtPoint"` |
| `id` | 图示 id（挂到文档根点下）或点 id（挂到该点下） |
| `text` | 可选。省略当作 `""` |
| `role` | 可选。`node` 或 `asst`，缺省 `node` |

在 data 部件追加一个可见 `dgm:pt`，并追加一条 `type` 为 `parOf`（或省略 type）的 `cxn`：`srcId` 为父点 `modelId`（图示 id 时父为 `type="doc"` 的点），`destId` 为新点。不手写 `pres` / `parTrans` / `sibTrans`。现有 `modelId` 都能解析成非负整数时，新点与新连接用尚未占用的最小正整数；否则新点写新的大写 GUID（带花括号），连接 `modelId` 同样不与已有 pt / cxn 冲突。`role` 为 `asst` 且当前 layout 属 `list` 家族时报 `InvalidField`。找不到父点或图示报 `IdNotFound`。id 不是图示 id 或点 id 报 `InvalidId`。缺少 data 部件报 `OpcPartMissing`。

### deleteSmartArtPoint

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteSmartArtPoint"` |
| `id` | 点 id |

删掉该点，并沿 `parOf` 边删掉所有子孙可见点。同时删掉 `srcId` / `destId` 指向这些 `modelId` 的连接，以及 `cxnId` 指向已删连接的 `parTrans` / `sibTrans` 点。不删 `type="doc"` 的点。删完后可见点一条都不剩时报 `InvalidField`（要整张去掉请用 `deleteSmartArt`）。找不到报 `IdNotFound`。id 不是点 id 报 `InvalidId`。

### insertSmartArt

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertSmartArt"` |
| `id` | docx：宿主段落 id，正文、页眉页脚、框内、注内或批注内段落。pptx：页 id `slide:{n}` |
| `layout` | `list` 或 `org` |
| `points` | 可选。可见点数组。每项可含 `text`、`parent`、`role`。`parent` 是本数组里从 0 起的下标，表示父点；省略或 `null` 表示挂到文档根点下 |
| `kind` | docx 可选。`inline` 或 `anchor`，缺省 `anchor`。pptx 上出现报 `InvalidField` |
| `wrap` | docx 可选。浮动时的绕排，缺省 `square`。行内必须省略或为 `null`。pptx 上出现报 `InvalidField` |
| `x` / `y` | pptx 可选。画布坐标，EMU，非负整数。必须成对。docx 上出现报 `InvalidField` |
| `cx` / `cy` | 可选。EMU，正整数。都省略则用 `5486400` × `3200400`。只给一个报 `InvalidField` |

docx：在该段末插入 `w:drawing`，默认浮动。新建 `/word/diagrams/` 下尚未占用的 data / layout / colors / quickStyle 四件套（序号从 1 起），并把四条关系写在**宿主** `.rels` 上（主文档、页眉页脚、`comments.xml`、`footnotes.xml` 或 `endnotes.xml`）。diagram 部件仍写在 `/word/diagrams/`。`wp:docPr/@id` 取该部件已有 `docPr` 的最大整数加 1，至少为 1。对图 id、图表 id、SmartArt id、点 id、容器 id、框 id、注容器 id、批注容器 id、`section:{n}` 或 `slide:{n}` 发此 op 报 `InvalidId`。找不到段落报 `IdNotFound`。

pptx：在该页 `p:spTree` 插入 `p:graphicFrame`。新建 `/ppt/diagrams/` 下同样四件套，四条关系写在该页 `.rels`。`p:cNvPr/@id` 取该页已有全部 `cNvPr` 的最大整数加 1，至少为 2。缺 `x` / `y` 则放到该页空白处。对图 id、图表 id、图示 id、点 id、形状 id 或 `pptx:presentation` 发此 op 报 `InvalidId`。找不到页报 `IdNotFound`。

`dgm:relIds` 的 `r:dm` / `r:lo` / `r:qs` / `r:cs` 指向这四条。不强制套 `mc:AlternateContent`，不写 Fallback 位图，不写 diagram drawing 缓存。

省略 `points` 时：`list` 写三个可见点，文本为 `文本 1`、`文本 2`、`文本 3`，都挂在文档根点下；`org` 写一个根点 `文本 1`，再写两个子点 `文本 2`、`文本 3`。给了 `points` 则按数组顺序写；`parent` 越界、指向自身或成环报 `InvalidField`。`layout` 不认识报 `InvalidField`。`list` 上出现 `role` 为 `asst`，或 `parent` 指向另一个可见点（深度大于 1）报 `InvalidField`。

Content Type：data 为 `application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml`，layout 为 `…diagramLayout+xml`，colors 为 `…diagramColors+xml`，quickStyle 为 `…diagramQuickStyle+xml`。关系 Type 分别为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramData`、`…/diagramLayout`、`…/diagramColors`、`…/diagramQuickStyle`。

### deleteSmartArt

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteSmartArt"` |
| `id` | 图示 id |

删掉对应宿主 drawing。docx 删 `w:drawing`；若在 `mc:AlternateContent` 的 Choice 里，整段 `AlternateContent` 删掉（Fallback 位图一起走）。所在 `w:r` 若删完后只剩 `w:rPr` 或空，把该 run 也删掉。pptx 删 `p:graphicFrame`；若在 Choice 里同样整段 `AlternateContent` 删掉。重新解析关系之后，若该图示的 data / layout / colors / quickStyle / drawing 缓存再无人引用，删掉这些部件、关系条目和对应内容类型。Fallback 用过的媒体部件若再无人引用也删。找不到报 `IdNotFound`。id 不是图示 id 报 `InvalidId`。

### setSmartArtSize

字段与 `setPictureSize` 相同，`id` 为图示 id。docx 改宿主 `wp:extent`（以及 `a:ext` 若有），规则与图相同。pptx 改 graphicFrame 的 `p:xfrm/a:ext`。

### setSmartArtPos

字段与 `setPicturePos` 相同，`id` 为 pptx 图示 id。写入 graphicFrame 的 `p:xfrm/a:off`。docx / xlsx 上发此 op 报 `InvalidField`。

### setSmartArtWrap

字段与 `setPictureWrap` 相同，`id` 为图示 id。改宿主 `wp:inline` / `wp:anchor` 与绕排，规则与图相同。xlsx / pptx 上发此 op 报 `InvalidField`。pptx 改画布坐标请用 `setSmartArtPos`。

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
| `layout` | `list` 或 `org` |

不认识的短名报 `InvalidField`。当前 uniqueId 已经是目标那一条：空操作，仍计入 `opsApplied`。当前与目标属同一家族：把 layout 部件换成库内对应模板，并改 `r:lo`；可见点与 `parOf` 连接不动。否则（跨家族，或当前 uniqueId 不在两族里）报 `NotImplemented`，不写文件。不要写出 Word 要修复的包。找不到图示报 `IdNotFound`。id 不是图示 id 报 `InvalidId`。

### 改动范围（docx）

允许变脏的部件：主文档 XML（通常是 `/word/document.xml`）及其 `.rels`，被 `setParaText`、`setParaFormat`、`setRunFormat`、图片 op、文本框 op、图表 op、SmartArt op、OLE op、表格结构 op、修订 op、超链接与书签 op、域 op、公式 op、内容控件 op、`setParaNumbering` 或 `setParaStyle` 点名的页眉、页脚部件及其 `.rels`，`setHeaderFooterLink` / `insertHeaderFooter` / `deleteSection` 改到的主文档关系、`[Content_Types].xml`、新建或删除的 header/footer 部件及其 `.rels`，脚注 / 尾注部件及其 `.rels`，批注部件 `/word/comments.xml`、`commentsExtended.xml`、`people.xml`，以及文件里已有的 `commentsIds.xml` / `commentsExtensible.xml`，图表部件 `/word/charts/…` 及其 `.rels`，embedding `/word/embeddings/…`，SmartArt 四件套 `/word/diagrams/…` 及可选 drawing 缓存，`setNoteNumbering` 在整篇上改到的 `settings.xml` 与主文档（各节 `sectPr`），`setEvenAndOddHeaders` / `setMirrorMargins` / `setGutterAtTop` / `setProtection` 改到的 `settings.xml`，`setProtection` 打开或关掉窗体保护时还可以脏主文档各节 `w:formProt`，编号 op 改到的 `/word/numbering.xml`、主文档 `.rels` 与 `[Content_Types].xml`，`setStyle` 改到的 `/word/styles.xml`，以及新媒体或被删的 `/word/media/…`。OLE op 还可以脏 `/word/embeddings/…` 与预览图媒体。图表 op、SmartArt op 与文本框 op 点名注内或批注内段落时，还可以脏对应 notes / comments 部件及其 `.rels`。`setHeaderFooterDistance`、`setPageMargin`、`setPageSize`、`setColumns`、`setVerticalAlign`、`setLineNumbering`、`setDocGrid`、`setRtlGutter`、`setTitlePage`、`insertSection`、`insertToc`、`deleteToc`、`insertMath`、`setMath` 与 `deleteMath` 只脏主文档（页眉页脚上的公式 op 另脏对应 header/footer 部件）。`insertContentControl`、`setContentControl` 与 `deleteContentControl` 只脏主文档（页眉页脚上的内容控件 op 另脏对应 header/footer 部件）。`deleteSection` 脏主文档，并可能删掉不再被引用的 header/footer 部件。正文图片 op、文本框 op、图表 op、SmartArt op、正文表格结构 op、正文修订 op、正文超链接 / 书签 op、正文域 op、正文公式 op、正文内容控件 op、正文 `setParaNumbering`、正文 `setParaStyle`、正文 `setParaFormat` 与正文 `setRunFormat` 脏主文档与主文档 `.rels`。一次 `ops` 可以混用 `setParaText`、`setHeaderFooterDistance`、`setPageMargin`、`setPageSize`、`setColumns`、`setVerticalAlign`、`setLineNumbering`、`setDocGrid`、`setRtlGutter`、`setMirrorMargins`、`setGutterAtTop`、`setHeaderFooterLink`、`insertHeaderFooter`、`setTitlePage`、`setEvenAndOddHeaders`、`insertSection`、`deleteSection`、`insertToc`、`deleteToc`、`insertMath`、`setMath`、`deleteMath`、`insertContentControl`、`setContentControl`、`deleteContentControl`、图片 op、文本框 op、脚注尾注 op、批注 op、图表 op、SmartArt op、表格结构 op、修订 op、超链接 / 书签 op、域 op 与编号 op、样式 op、直接格式 op、以及 `setProtection`、以及 `setProps` / `setCustomProp` / `deleteCustomProp`。文件属性 op 还可以脏 `/docProps/core.xml`、`/docProps/app.xml`、`/docProps/custom.xml`、根 `.rels` 与 `[Content_Types].xml`。只改 core 字段时不要新建 custom。未点名的 `styles.xml`、未点名的页眉页脚、主题一律原样写回。未点名的 `numbering.xml` 也原样写回。未点名的 `settings.xml` 也原样写回。`setNoteNumbering` 只点名某一节时只脏主文档，不改 `settings.xml`。`replyComment` 会脏出现父批注范围标记的宿主部件（常见是主文档）。只改批注文本、元数据或完成标记时，未出现范围标记的宿主部件（常见是主文档）原字节写回。只改图表 cache / 标题 / 图例、且宿主 drawing 未动时，未点名的正文段落字节按「drawing 未动」验收。只改 SmartArt 点文字或增删点、且宿主 drawing 未动时，同样按「drawing 未动」验收。

被改部件同样整份重序列化，保留程度与 xlsx 工作表相同：元素与属性顺序、未知命名空间保留；注释、处理指令、DOCTYPE 会丢。

### setShapeText

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "output": "C:\\data\\deck.patched.pptx",
  "ops": [
    { "op": "setShapeText", "id": "slide:1/shape:2", "text": "新标题" }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setShapeText"` |
| `id` | 形状 id（`slide:{n}/shape:{shapeId}`、`slide:{n}/shape:index:{k}`，或备注上的 `slide:{n}/notes/shape:…`） |
| `text` | 新的可见纯文本，字符串，可空。必填。`\\n` 写成 `a:br`，不把字面换行两个字符写进 `a:t` |

找不到该形状报 `IdNotFound`。第一期不插入、不删除形状。对 `slide:{n}` 或 `slide:{n}/notes`（没有 `/shape:`）发本 op 报 `InvalidId`。对备注页上占位符 `p:ph/@type` 为 `sldImg` 的形状发本 op 报 `InvalidField`，不写文件。

替换规则（手术刀，不整页重建 `p:sp`，更不整份重建演示文稿）：

1. 只改命中的那一个 `p:sp` 的 `p:txBody` 可见文字。`p:nvSpPr`、`p:spPr`（几何与位置）、`p:style`、不认识的子元素都留着。禁止删掉该形状再新建一个。
2. 不改同页的 `<p:timing>`、`<p:transition>`，也不改其他形状。重序列化该页时这两节必须作为未点名节点原样保留。
3. 在该 `p:txBody` 里找出含 `a:t` 的 run（`a:r`）。遇到 `mc:AlternateContent` 只进第一个 `mc:Choice`（没有就进 `mc:Fallback`）。
4. 若一个都没有：在现有 `a:bodyPr` / `a:lstStyle` 之后插入 `a:p`/`a:r`/`a:t`，文本放进去；已经有段落时挂在第一段末尾，不凭空多出一段。没有 `p:txBody` 则本 op 报 `IdNotFound`（第一期不给无文本框的形状补 `txBody`）。
5. 若有：第一个这样的 run 收下全部新文本。第一行写进它的第一个 `a:t`（需要保留首尾空格时加 `xml:space="preserve"`），该 run 里多余的 `a:t` 删掉。`text` 里每遇一个 `\n`，就在**段落**里紧接着插一个 `a:br` 加一个新的 `a:r`，新 run 沿用第一个 run 的 `a:rPr`。
6. 换行只能拆在段落这一级：`a:br` 与 `a:r` 平级、同为 `a:p` 的子元素（`EG_TextRun`，§A.4），而 `CT_RegularTextRun` 只允许一个可选 `a:rPr` 加**恰好一个** `a:t`。所以 `a:br` 不许塞进 `a:r` 里，一个 `a:r` 也不许带多个 `a:t`，两者都会生成不合 schema 的 run。
7. 该 `p:txBody` 里其余的可见内容一律清掉：别的 `a:t` 删掉，**所有 `a:br` 删掉**，因此变空、且里面再也没有别的 run 内容的 `a:r` 删掉。`a:br` 必须一起清，因为 inspect 把它读成 `\n`，留一个下来读回值就比写入值多一个换行，与本节最后一条冲突。收下新文本的那一段原地保留，它自己的 `a:pPr`、`a:endParaRPr` 不许删，也不许换成别段的。清完之后，段落里若只剩下 `a:pPr`、`a:endParaRPr`（没有 `a:r`、`a:br`、`a:fld` 以及其它子元素），把这段 `a:p` 删掉。`CT_TextBody` 的 `a:p` 至少留一段（§A.4，`minOccurs="1"`）。不许把整棵 `txBody` 的段落删光再重建，也不许把别段的 `a:pPr` 搬到写入段上。空段要删，是因为 §21.1.2.2.6 把 `a:p` 定为文本框里最高一层分隔，PowerPoint 会把空段画成空行；inspect 的 `text` 又不在段落之间插入 `\n`。含 `a:fld` 的段按第 8 条不清域，因此也不会被本条删掉。
8. `a:fld` 是应用程序定期更新的生成文本（§21.1.2.2.4），不是用户打的字。apply 不改、不删域里的节点。inspect 的 `text` 也不收域里的 `a:t`。写进去的用户文本因此能原样读回；屏幕上域的缓存字（页码、日期）仍可能跟在旁边。第一期不把域做成独立节点。
9. apply 写进去的文本，随后 inspect 读回来必须与写入值相同。

### setNotesText

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    { "op": "setNotesText", "id": "slide:1", "text": "演讲者备注" }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setNotesText"` |
| `id` | `slide:{n}` 或 `slide:{n}/notes` |
| `text` | 新的演讲者备注可见纯文本，字符串，可空。必填。换行规则与 `setShapeText` 相同 |

只改备注页上占位符 `p:ph/@type` 为 `body` 的那个文本框。文档顺序中第一个 `body` 占位符命中。找不到该页报 `IdNotFound`。`id` 不是上述两种报 `InvalidId`。没有 notesSlide 时新建最小 Transitional 备注页：若包内还没有 notesMaster，则补 `/ppt/notesMasters/notesMaster1.xml`（指向已有 theme）、presentation 关系、`p:notesMasterIdLst` 与内容类型；再写 `/ppt/notesSlides/notesSlideN.xml`，该页 `.rels` 指向 notesMaster 与对应幻灯片。新建的备注页含 `sldImg`、`body`、`sldNum` 三个占位符，结构对照 PowerPoint 16.0 另存样例，不写 `p14:creationId`。已有备注页却没有 `body` 占位符时，补一个 `body` 再写字。不改 `sldImg` 缩略图占位符。xlsx / docx 上发此 op 报 `InvalidField`。`create` 本期不写备注。

示例见 `docs/examples/pptx-apply-notes.json`。inspect 示例见 `docs/examples/pptx-inspect-notes.json`。批注示例见 `docs/examples/pptx-apply-comments.json` 与 `docs/examples/pptx-inspect-comments.json`。

### 改动范围（pptx）

允许变脏的部件：被改的那一张幻灯片 XML（通常是 `/ppt/slides/slideN.xml`，N 以关系目标为准）及其 `.rels`，被改的 notesSlide 及其 `.rels`，必要时 `presentation.xml` 及其 `.rels`、新建的 notesMaster 及其 `.rels`，`[Content_Types].xml`，以及新建或被删的 `/ppt/media/…`、`/ppt/charts/…`、embedding、`/ppt/diagrams/…` 及可选 drawing 缓存，以及现代批注部件 `/ppt/comments/modernComment_*.xml` 与 `/ppt/authors.xml`。`setShapeText` 只脏目标宿主 XML（幻灯片或 notesSlide）。`setNotesText` 可脏 notesSlide、该页 `.rels`、notesMaster、presentation。图片 op 还可脏该宿主关系与媒体。图表 op 还可脏 chart 部件、embedding。SmartArt op 还可脏四件套与缓存。批注 op 还可脏该页 comments 部件、幻灯片 XML（新建时的 `commentRel`）、presentation `.rels` 与 authors。其他页、幻灯片母版、版式、主题、`presProps` 一律原样写回。一次 `ops` 可以混用 `setShapeText`、`setNotesText`、批注 op、图片 op、图表 op、SmartArt op 与 `setProps` / `setCustomProp` / `deleteCustomProp`。文件属性 op 还可以脏 `/docProps/core.xml`、`/docProps/app.xml`、`/docProps/custom.xml`、根 `.rels` 与 `[Content_Types].xml`。只改 core 字段时不要新建 custom。未点名的 `Slides` 留下。只改 SmartArt 点文字或增删点、且宿主 graphicFrame 未动时，该页 XML 按「drawing 未动」验收。

被改部件同样整份重序列化，保留程度与 xlsx 工作表相同：元素与属性顺序、未知命名空间保留；注释、处理指令、DOCTYPE 会丢。

## insertOle

把本地文件作为嵌入对象写入目标段落。不在库里启动 OLE 服务器，不生成真实预览图（写入一张最小 PNG 占位）。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    { "op": "insertOle", "id": "para:3F2A1B00", "file": "C:\\\\data\\\\book.xlsx" }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertOle"` |
| `id` | 必须是正文或页眉页脚段落 id |
| `file` | 必须。本地文件路径 |

按扩展名决定 ProgID 与写入方式：`.xlsx` 为 `Excel.Sheet.12`，`.xlsm` 为 `Excel.SheetMacroEnabled.12`，`.xls` 为 `Excel.Sheet.8`，`.docx` 为 `Word.Document.12`，`.docm` 为 `Word.DocumentMacroEnabled.12`，`.pptx` 为 `PowerPoint.Show.12`，`.pptm` 为 `PowerPoint.ShowMacroEnabled.12`。这些扩展名把源文件原字节写入 `/word/embeddings/…`，Content Type 与该格式主文档一致（xlsx 为 `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`，docx / pptx 同理；`.xls` 为 `application/vnd.ms-excel`）。其它扩展名 ProgID 为 `Package`，把源文件打成 OLE 复合文档（Ole10Native）写入 `/word/embeddings/oleObjectN.bin`，Content Type 为 `application/vnd.openxmlformats-officedocument.oleObject`。找不到 `file` 报 `Io`。空路径报 `InvalidField`。xlsx / pptx 上发此 op 报 `InvalidField`。

宿主 XML 写 Transitional 的 `w:object`：`o:OLEObject` 的 `Type` 为 `Embed`，`DrawAspect` 为 `Content`，关系指向 embeddings；另写一条 image 关系指向占位 PNG。不改 VML 几何。`ObjectID` 取本文件尚未用过的正整数，写成 `_` 加该数字。Office 文件的关系 Type 用 `package`（与 Word 16.0 另存一致）；Package 用 `oleObject`。

## deleteOle

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteOle"` |
| `id` | 必须是 OLE id |

找不到报 `IdNotFound`。删掉整棵 `w:object`（含预览 markup），去掉 oleObject / package 关系与 embeddings 部件；预览图若无其它关系再指向则一并去掉。xlsx / pptx 上发此 op 报 `InvalidField`。

## exportMedia

按图 id 把 OPC 里的媒体部件原样写到本地文件。docx 也认 OLE id，写出嵌入部件原字节。不读进 inspect 树，不做转码、缩放或预览。

```json
{
  "schemaVersion": 1,
  "action": "exportMedia",
  "path": "C:\\data\\note.docx",
  "id": "para:3F2A1B00/pic:1",
  "output": "C:\\out\\logo.png"
}
```

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `path` | 是 | 源 docx、xlsx 或 pptx |
| `id` | 是 | 图 id，与 inspect 的 `pictures[].id` 相同；docx 也可以是 OLE id，与 `oleObjects[].id` 相同 |
| `output` | 否 | 本地目标路径。省略时用命令行 `--output`。两者都缺报 `MissingField`。不得默认写成源 `path`，以免把图片字节盖掉文档 |
| `format` | 否 | 可省略，从 `path` 扩展名推断 |

`id` 必须是图 id，或 docx 的 OLE id。对段落 id、工作表 id、单元格 id、页眉页脚容器 id、`section:{n}`、形状 id 或 `slide:{n}`（没有 `/pic:`）报 `InvalidId`。xlsx / pptx 上给 OLE id 同样报 `InvalidId`。找不到该图或该嵌入对象报 `IdNotFound`。图 id 优先走 `a:blip/@r:embed`；只有 `@r:link`、没有 embed 时按外部关系处理，报 `OpcPartMissing`，不下载 http。embed 指向外部关系同样报 `OpcPartMissing`。OLE id 走 oleObject 或 package 关系目标；关系缺失或指向外部同样报 `OpcPartMissing`。

写出的是该媒体部件或嵌入部件的原字节。`contentType` 取自 `[Content_Types].xml` 对该部件的 Default 或 Override；都没有则省略该字段。扩展名由调用方自己定，库不按类型改名。

xlsx 用工作表图 id 或页眉图 id，写出 `/xl/media/…`。pptx 用幻灯片图 id 或备注图 id，写出 `/ppt/media/…`。docx 文本框里的图用框内段落上的图 id，同一动作能导出。脚注、尾注里的图用注内段落上的图 id，同一动作能导出。批注里的图用批注内段落上的图 id，同一动作能导出。docx 的 OLE id 写出 `/word/embeddings/…`，不是预览图。嵌入图表的工作簿另走 `exportChartData`，不在本动作。

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

`id` 必须是图表 id，含正文、页眉页脚、框内、注内与批注内带前缀的 id。对系列 id、段落 id、形状 id、图 id 报 `InvalidId`。找不到该图报表 `IdNotFound`。没有 embedding 报 `OpcPartMissing`。关系指向外部资源同样报 `OpcPartMissing`，不下载 http。xlsx 本期报 `InvalidField`。

写出的是 embedding 部件的原字节。`contentType` 取自 `[Content_Types].xml`；没有则省略。

成功时 stdout 见 [errors.md](errors.md)。源包不写回。

## 两层 JSON

领域 JSON 就是本文，本仓库只认这一层。外部 MCP 可以把本文整个对象放进自己的信封（例如 RPC 的 `params`）。本仓库不实现、不解析那一层信封。不许把 JSON-RPC 的 `jsonrpc`、`method`、`id` 塞进领域对象，也不许让单元格或段落字段去迁就 RPC。
