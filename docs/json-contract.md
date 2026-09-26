# 领域 JSON 契约

`schemaVersion` 固定为 `1`。库和命令行只认这一层。MCP 或其他传输信封不许改这里的字段名和含义。机器可读副本见 [json-schema/request.schema.json](json-schema/request.schema.json)，与本文冲突时以本文为准。

xlsx、docx、pptx 的 inspect、create、apply 已经按本文实现。docx 另有 `exportMedia` 与 `exportChartData`。xlsx 的 `exportMedia` 认工作表图 id 与页眉页脚图 id。pptx 的 `exportMedia` 认幻灯片图 id，写出 `/ppt/media/…`。xlsx 的 `scope.charts` 与图表 op 认工作表 / 图表工作表上的图；`scope.headerFooter` 展开工作表上的页眉页脚槽位。pptx 的 `scope.charts` 与图表 op 认幻灯片上的 `p:graphicFrame`；`exportChartData` 认幻灯片图表 id，写出 embedding。pptx 的 `scope.smartArt` 与 SmartArt op 认幻灯片上的 diagram `p:graphicFrame`。xlsx 的 `exportChartData` 仍拒绝。xlsx 的 `scope.smartArt` 与 SmartArt op 仍拒绝。人工打开只认 Microsoft Office，见三份 checklist。

## 信封

每份请求都是一个 JSON 对象，必须含：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `schemaVersion` | 整数 | 必须是 `1` |
| `action` | 字符串 | `inspect`、`create`、`apply`、`exportMedia`、`exportChartData` 之一 |
| `locale` | 字符串 | 可选。错误 `message` 的语言偏好，见 [errors.md](errors.md) |

`locale` 不属于动作参数，五种动作都认。取值宽松匹配：`zh`、`zh-CN`、`zh-Hans`、`zh-SG` 出中文；`en` 及其地区变体出英文；不认识的标签（包括空串）退回英文，不报 `InvalidField`。这是信封里唯一一处对未知取值不报错的字段：语言只影响呈现，不该挡下文档操作。优先级是请求 `locale` 高于命令行 `--lang`，再高于环境变量 `OOXP_LANG`，都没有时是英文。

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

标题和正文都是段落，不另编一套 id。脚注、尾注、批注都是独立故事：根上始终有计数；给了对应 `scope` 才列出名单，并可 apply。分隔符三条不进用户名单，禁止 apply。文本框是独立节点：框本身、框内段落与框内图按下面规则进树，并可 apply。页眉、页脚按下面规则进树，可按 id 改字；节上的页眉页脚距边界、以及是否链接到前一节，都可以读写。页眉、页脚、正文、注和批注里的 DrawingML 图片可查、换、插、删，并可改尺寸与绕排。apply 不做新建页眉。不写入 `titlePg` / `evenAndOddHeaders`。`create` 可在正文和默认页眉页脚上写 DrawingML 图，也可写文本框与 SmartArt；不写脚注、尾注、批注、图表。带图字段见下文 create。嵌入图表不是 `pic:pic`：根上始终有 `chartCount`；给了 `scope.charts` 才列出名单，并可 apply。SmartArt 不是 `pic:pic`：根上始终有 `smartArtCount`；给了 `scope.smartArt` 才列出名单，并可 apply。框内、注内、批注内的图表与 SmartArt 本期不进树、不接受 apply。

页眉、页脚 id 按部件编，不按节编。`w:sectPr` 里的 `w:headerReference` / `w:footerReference`（§17.10.2、§17.10.5）经主文档关系解到部件路径。同一部件只出一个容器节点；多个节引用同一部件时，`sections` 列出所有节号。后一节某类型引用省略时按规范继承前节（§17.10.2）；`even` 是否继承还看 `settings.xml` 的 `w:evenAndOddHeaders`（§17.10.1），`first` 是否继承还看该节的 `w:titlePg`（§17.10.6）。OOXML 没有「链接到前一节」元素；Word UI 的勾选就是这种省略。inspect 在每个节上用 `headerLink` / `footerLink` 标出该类型是显式引用、继承前节，还是对本节不生效。apply 用 `setHeaderFooterLink` 断开或重新链接。两节显式指向同一部件不算链接，两边都是 `false`。

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
| 脚注 | `footnote:{id}` | `footnote:2` |
| 尾注 | `endnote:{id}` | `endnote:2` |
| 注内段落 | `{注 id}/para:{paraId}` 或 `{注 id}/para:index:{k}` | `footnote:2/para:3F2A1B00` |
| 注内图 | `{注内段落 id}/pic:{docPrId}` | `footnote:2/para:3F2A1B00/pic:1` |
| 分隔符（只读） | `footnote:sep:{type}` / `endnote:sep:{type}` | `footnote:sep:separator` |
| 批注 | `comment:{id}` | `comment:0` |
| 批注内段落 | `{批注 id}/para:{paraId}` 或 `{批注 id}/para:index:{k}` | `comment:0/para:3F2A1B00` |
| 批注内图 | `{批注内段落 id}/pic:{docPrId}` | `comment:0/para:3F2A1B00/pic:1` |
| 正文图表 | `{段落 id}/chart:{docPrId}` 或 `{段落 id}/chart:index:{k}` | `para:3F2A1B00/chart:7` |
| 页眉/页脚图表 | `{页眉或页脚段落 id}/chart:{docPrId}` | `header:default:1/para:AA/chart:7` |
| 图表系列 | `{图表 id}/series:{idx}` | `para:3F2A1B00/chart:7/series:0` |
| 正文 SmartArt | `{段落 id}/smartdiagram:{docPrId}` 或 `{段落 id}/smartdiagram:index:{k}` | `para:3F2A1B00/smartdiagram:4` |
| 页眉/页脚 SmartArt | `{页眉或页脚段落 id}/smartdiagram:{docPrId}` | `header:default:1/para:AA/smartdiagram:4` |
| SmartArt 点 | `{图示 id}/pt:{modelId}` | `para:3F2A1B00/smartdiagram:4/pt:2` |

节号 `{n}` 从 1 起，按主文档里 `w:sectPr` 的文档顺序（与页眉目录相同）。距边界挂在节上，不挂在页眉部件节点上。页眉页脚图挂在容器上，不挂在节上。正文图挂在它所在的主文档段落上，不挂在文档根上。对 `section:{n}` 发 `setParaText` 或图片 op 报 `InvalidId`。

`{type}` 取引用上 `w:type` 的原文，只允许 `default`、`even`、`first`（`ST_HdrFtr`，§17.18.36）。缺失或不是这三项报 `XmlBroken`。`{n}` 在同一 `kind`（header 或 footer）与 `{type}` 下，按该部件首次出现的顺序从 1 起。页眉页脚段落没有 `w14:paraId` 时用 `para:index:{k}`，`k` 只在该部件内从 1 计。

正文段落 id 仍是 `para:{paraId}` / `para:index:{n}`，只表示主文档部件。对容器 id（没有 `/para:`）发 `setParaText` 报 `InvalidId`。拿正文 `para:` 去改页眉页脚、注或批注里同 hex 的段落报 `IdNotFound`。

脚注、尾注的 `{id}` 取 `w:footnote/@w:id` / `w:endnote/@w:id` 的原文（ECMA-376 §17.11.2）。输出用文件里的原始数字写法。请求里若双方都能解析成十进制整数，按数值匹配（`2` 与 `02` 视为同一条）；否则按原文精确比。同一文件里脚注 id 与尾注 id 可以都是 `2`，靠前缀区分。只把 `w:type` 为 `normal` 或未写 type 的当作用户注（§17.18.33 `ST_FtnEdn`）。`separator`、`continuationSeparator`、`continuationNotice` 是分隔符，id 写成 `footnote:sep:{type}` / `endnote:sep:{type}`，只出现在 `noteSeparators`，对它们发任何 apply 报 `InvalidId`。同一侧重复 `w:id` 时 inspect 都列出，apply 命中该部件文档顺序第一处。

批注的 `{id}` 取 `w:comment/@w:id` 的原文（ECMA-376 §17.13.4.2）。匹配规则与脚注 `w:id` 相同。同一文件里批注 id 与脚注 id 可以都是 `0`，靠前缀区分。重复 `w:id` 时 inspect 都列出，apply 命中 `comments.xml` 文档顺序第一处。范围标记 `w:commentRangeStart` / `w:commentRangeEnd` / `w:commentReference` 的 `w:id` 与 `w:comment/@w:id` 按数值对应（§17.13.4.4、§17.13.4.3、§17.13.4.5）。线程关系靠 `commentsExtended` 的 `paraIdParent` 挂到父批注（MS-DOCX §2.5.3.1）。Word 16 给回复也写一套范围标记，套在父批注范围上：`commentRangeStart` 紧跟父 start 之后，`commentRangeEnd` 与 `commentReference` 紧跟父 `commentReference` 所在 run 之后。inspect 读到回复自己的标记就用；没有标记时沿用线程根。`commentEx/@paraId` 是该批注**最后一段**的 `w14:paraId`，不是第一段，也不是宿主段落。

图表的 `{docPrId}` 取该 `w:drawing` 里 `wp:docPr/@id` 的原文，匹配规则与图相同。没有该属性时用 `chart:index:{k}`，`stable` 为 `false`。正文的 `{k}` 只在该段内从 1 计；页眉页脚的 `{k}` 只在该部件内从 1 计。系列 `{idx}` 从 0 起，按 chart 部件里 `c:ser` 的文档顺序，不跟 `c:idx/@val`。只认 `a:graphicData/@uri` 为 `http://schemas.openxmlformats.org/drawingml/2006/chart`（可带尾斜杠）的 drawing（ECMA-376 §21.2）。Office 2016 以后的 chartex URI（`http://schemas.microsoft.com/office/drawing/2014/chartex`）也列入，`type` 为 `chartex`。VML 图表不编。`w:drawing` 里同时有 `pic:pic` 和图表的，各走各的名单。正文图表只收主文档段落的子孙，遇 `w:txbxContent` 停住；表格单元格里的段落与正文段落同一套规则。页眉页脚图表挂在它所在的页眉或页脚段落上，名单出现在该容器的 `charts` 上。框内、注内、批注内的图表本期不进树、不接受 apply。

`insertChart` 的 `id` 必须是段落 id（正文或页眉页脚段落）。对图 id、图表 id、系列 id、SmartArt id、点 id、容器 id、框 id、注容器 id、批注容器 id 或 `section:{n}` 发 `insertChart` 报 `InvalidId`。`deleteChart` / `setChartTitle` / `setChartType` / `setChartAxisTitle` / `setChartLegend` / `setChartDataLabels` / `setChartSize` / `setChartWrap` / `addChartSeries` 的 `id` 必须是图表 id。`setChartSeries` / `deleteChartSeries` 的 `id` 必须是系列 id。对不上的 id 形态报 `InvalidId`。

SmartArt 的 `{docPrId}` 取该 `w:drawing` 里 `wp:docPr/@id` 的原文，匹配规则与图相同。没有该属性时用 `smartdiagram:index:{k}`，`stable` 为 `false`。正文的 `{k}` 只在该段内从 1 计；页眉页脚的 `{k}` 只在该部件内从 1 计。点的 `{modelId}` 取 data 部件里 `dgm:pt/@modelId` 的原文，输出用文件里的原始大小写。请求按精确字符串匹配（GUID 的花括号和连字符都按原文）。同一 data 部件里重复 `modelId` 时 inspect 都列出，apply 命中 `ptLst` 文档顺序第一处。只认 `a:graphicData/@uri` 为 `http://schemas.openxmlformats.org/drawingml/2006/diagram`（可带尾斜杠）的 drawing（ECMA-376 §21.4）。`dgm:relIds` 的 `r:dm` / `r:lo` / `r:qs` / `r:cs` 指向**宿主部件**关系里的 data / layout / quickStyle / colors（§21.4.2.22）。VML 图示不编。`w:drawing` 里同时有 `pic:pic`、图表和 SmartArt 的，各走各的名单。正文 SmartArt 只收主文档段落的子孙，遇 `w:txbxContent` 停住；表格单元格里的段落与正文段落同一套规则。页眉页脚 SmartArt 挂在它所在的页眉或页脚段落上，名单出现在该容器的 `smartArts` 上。框内、注内、批注内的 SmartArt 本期不进树、不接受 apply。

可见点：`dgm:pt` 的 `type` 缺省（规范默认 `node`，§21.4.7.51）或为 `node` / `asst` 的点进 `points`。`doc`、`pres`、`parTrans`、`sibTrans` 不进树。没有 `dgm:t` 时 `text` 为 `""`。层次看 `cxnLst` 里 `type` 缺省或为 `parOf` 的连接：`srcId` 是父、`destId` 是子。父点是 `doc` 或不在可见名单里时 `parent` 为 `null`；否则 `parent` 是父点的点 id。

`insertSmartArt` 的 `id` 必须是段落 id（正文或页眉页脚段落）。对图 id、图表 id、SmartArt id、点 id、容器 id、框 id、注容器 id、批注容器 id 或 `section:{n}` 发 `insertSmartArt` 报 `InvalidId`。`deleteSmartArt` / `setSmartArtSize` / `setSmartArtWrap` / `setSmartArtColors` / `setSmartArtStyle` / `setSmartArtLayout` 的 `id` 必须是图示 id。`setSmartArtText` / `deleteSmartArtPoint` 的 `id` 必须是点 id。`addSmartArtPoint` 的 `id` 是图示 id（挂到文档根点下）或点 id（挂到该点下）。对不上的 id 形态报 `InvalidId`。

图的 `{docPrId}` 取该 `w:drawing` 里 `wp:docPr/@id` 的原文（ECMA-376 §20.4）。输出用文件里的原始大小写与数字写法。请求里若双方都能解析成十进制整数，按数值匹配（`1` 与 `01` 视为同一处）；否则按原文精确比。同一归属范围内重复 `docPr/@id` 时，inspect 都列出，apply 命中该范围内文档顺序第一处。页眉页脚的范围是该容器部件；正文的范围是该段。没有 `docPr/@id` 时用 `pic:index:{k}`，树上 `"stable": false`。页眉页脚的 `{k}` 只在该部件内从 1 计；正文的 `{k}` 只在该段内从 1 计。两种 `{k}` 都计入该范围内所有被收入的图，不只是缺 id 的那些。只认 DrawingML 的 `pic:pic` 且带 `a:blip` 的 `w:drawing`。VML / `w:pict`、图表、SmartArt、文本框不进宿主段落的 `pictures`。框内图走框内段落上的 `pictures`，id 带框前缀。`mc:AlternateContent` 仍只进 Choice。正文图只收主文档段落的子孙，遇 `w:txbxContent` 停住，表格单元格里的段落与正文段落同一套规则。

`insertPicture` 的 `id` 必须是段落 id：正文用 `para:…` / `para:index:{n}`，页眉页脚用 `{容器}/para:…`，框内用 `{框 id}/para:…`，注内用 `{注 id}/para:…`，批注内用 `{批注 id}/para:…`。对图 id、容器 id、框 id、注容器 id、批注容器 id 或 `section:{n}` 发 `insertPicture` 报 `InvalidId`。`replacePicture` / `deletePicture` / `setPictureSize` / `setPictureWrap` 的 `id` 必须是图 id。对段落 id、容器 id、框 id、注容器 id、批注容器 id 或 `section:{n}` 发这四个 op 报 `InvalidId`。

文本框只认 DrawingML：`w:drawing` 里有 `w:txbxContent`，或 `wps:wsp` / `wps:txbx`（MS-DOCX WordprocessingShape）。常包在 `mc:AlternateContent` 里，只进 Choice。VML `v:textbox` / `w:pict` 不编、不改。组合 `wpg:wgp` 本身不进树，只展开里面符合条件的框。框的 `{docPrId}` 取该 `w:drawing` 里 `wp:docPr/@id`，规则与图相同。组合里的框取该 `wps:wsp` 的 `cNvPr/@id`；再没有则 `txbx:index:{k}`。`k` 只在该宿主段内从 1 计，`stable` 为 `false`。页眉页脚框挂在它所在的页眉或页脚段落上，不挂在节上。

链接框：`wps:txbx` 是链的第一框（§20.4.2.37），后续框用 `linkedTxbx`（§20.4.2.34），同一 `id` 为同一故事，`seq` 为顺序。树上 `link.prev` / `link.next` 是相邻框的图 id 风格框 id；未链接则为 `null`。不把后继框的字并进当前框。

「宿主段落遇到框就停住」仍成立，否则外层 `paraCount` 会把框内段算进去。边界按下面两条划：

- 遍历宿主段落（正文或页眉页脚）时遇到 `w:txbxContent` 就停住，不再往下走。框内 `w:p` 不计入宿主 `paraCount`，不出现在 `outline` 与宿主 `paragraphs` 里，它们的 `w:t` 也不并进外层段落的 `text`。拿框内 `paraId` 当正文 `para:` 发 apply，报 `IdNotFound`。框内段落必须用带框前缀的 id。
- 遇到 `mc:AlternateContent` 只走第一个 `mc:Choice`；没有 `mc:Choice` 才走 `mc:Fallback`。两支都走会把同一份内容读出两遍，也会让 apply 只改到其中一支、另一支不同步。这条对 `paraCount`、`outline`、`paragraphs`、`text`、页眉页脚、`pictureCount`、`pictures`、`textBoxCount`、`textBoxes`、`chartCount`、`charts`、`smartArtCount`、`smartArts` 和 apply 一律适用。

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

`{n}` 从 1 起，按 Presentation 部件（§13.3.6）里 `p:sldIdLst` / `p:sldId` 的出现顺序（§19.2.1.34），再跟 `r:id` 找到幻灯片部件。`n` 不是文件名里的数字，也不是 `p:sldId/@id`（那是 `ST_SlideId`，常见从 256 起）。隐藏页仍占序号，仍可 inspect 和 apply。

`{shapeId}` 取该形状 `p:nvSpPr/p:cNvPr/@id` 的原文（§19.3.1.12）。输出用文件里的原始数字字符串。请求按十进制数值匹配，不因前导零而失败。

没有 `cNvPr/@id` 时用 `slide:{n}/shape:index:{k}`，`k` 从 1 起，按该页列入树的形状顺序。树上同时给 `"stable": false`。这类 id 在增删形状后会变，只够同一次 inspect 之后立刻 apply。Office 亲手存的文件几乎都有 `cNvPr/@id`。

图 id 的 `{cNvPrId}` 取该 `p:pic` 的 `p:nvPicPr/p:cNvPr/@id` 原文。没有该属性时用 `slide:{n}/pic:index:{k}`，`stable` 为 `false`。`{k}` 只在该页全部被收入的 `p:pic` 内从 1 计，与形状的 `shape:index` 分家，互不影响。只认带 `a:blip` 的 `p:pic`。`p:sp`、连接线、`p:graphicFrame`（图表 / 表 / SmartArt）不进 `pictures`。组合本身不进树，里面符合条件的 `p:pic` 要展开。`mc:AlternateContent` 只进第一个 `Choice`，规则与形状相同。

图表 id 的 `{cNvPrId}` 取该 `p:graphicFrame` 的 `p:nvGraphicFramePr/p:cNvPr/@id` 原文。没有则用 `slide:{n}/chart:index:{k}`，`stable` 为 `false`。`{k}` 只在该页全部被收入的图表内从 1 计，与 `pic:index` / `shape:index` 分家。只认 `p:graphicFrame` 且 `a:graphicData/@uri` 为 chart URI（可带尾斜杠）。chartex URI 列入，`type` 为 `chartex`。`p:pic`、表格、SmartArt 不进 `charts`。组合里符合条件的框要展开。系列 `{idx}` 从 0 起，按 chart 部件里 `c:ser` 的文档顺序，不跟 `c:idx/@val`。数据在 embedding 工作簿与 cache 里；以 embedding 为准，不一致则该系列 `cacheStale: true`。

SmartArt id 的 `{cNvPrId}` 取该 `p:graphicFrame` 的 `p:nvGraphicFramePr/p:cNvPr/@id` 原文。没有则用 `slide:{n}/smartdiagram:index:{k}`，`stable` 为 `false`。`{k}` 只在该页全部被收入的图示内从 1 计，与 `pic:index` / `chart:index` / `shape:index` 分家。只认 `p:graphicFrame` 且 `a:graphicData/@uri` 为 `http://schemas.openxmlformats.org/drawingml/2006/diagram`（可带尾斜杠，ECMA-376 §21.4）。`dgm:relIds` 的四条关系写在该幻灯片 `.rels` 上。`p:pic`、图表、表格不进 `smartArts`。组合里符合条件的框要展开。点 id 规则与 docx 相同。备注、批注、母版、版式上的图示本期不进树。

同一页里若出现重复的 `cNvPr/@id`，inspect 照样列出每一处；apply 命中该页文档顺序中的第一处。

apply 不得删形状再重建，以免 `cNvPr/@id` 变化导致绑在形状上的动画失效。不改 `<p:timing>`。

备注页、讲义、幻灯片母版、版式第一期不进这棵树，也不接受 apply。xlsx / pptx 的批注另开，不走 docx 这套 `comment:` id。

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
      "scope": { "from": 1, "maxParas": 200, "headerFooter": true, "pictures": true, "textBoxes": true, "notes": true, "comments": true, "charts": true, "smartArt": true }
}
```

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `path` | 是 | 已有文件 |
| `scope` | 否 | 省略或空对象：返回 `outline`、`paraCount`、`pictureCount`、`textBoxCount`、`footnoteCount`、`endnoteCount`、`commentCount`、`chartCount`、`smartArtCount`、`headers`、`footers`、`sections`；正文没有 `paragraphs`；页眉页脚容器不带 `paragraphs`；没有 `footnotes` / `endnotes` / `noteSeparators` / `comments` / `charts` / `smartArts` |
| `scope.from` | 要正文段落时必填 | 从第几段开始，从 1 起，按主文档 `w:p` 顺序 |
| `scope.maxParas` | 要正文段落时必填 | 本次最多返回多少段，允许范围 `1` 到 `5000` |
| `scope.headerFooter` | 否 | 为 `true` 时给每个页眉、页脚节点填 `paragraphs`。省略或 `false` 则容器上没有 `paragraphs` |
| `scope.pictures` | 否 | 为 `true` 时给每个页眉、页脚节点填 `pictures`；若同时给了 `from` 与 `maxParas`，切片里每段也填 `pictures`；若同时 `notes: true`，每条用户注也填 `pictures`；若同时 `comments: true`，每条批注也填 `pictures`。省略或 `false` 则这些节点上都没有 `pictures`。不读媒体字节 |
| `scope.textBoxes` | 否 | 为 `true` 时给每个页眉、页脚节点填 `textBoxes`；若同时给了 `from` 与 `maxParas`，切片里每段也填 `textBoxes`。省略或 `false` 则这些节点上都没有 `textBoxes`。注内文本框本期不进树 |
| `scope.textBoxContent` | 否 | 为 `true` 且该节点已有 `textBoxes` 时，每个框再带全部 `paragraphs`；若同时 `pictures: true`，框内段再带 `pictures`。省略或 `false` 则框上只有计数，没有框内名单。不与 `from` / `maxParas` 抢正文切片 |
| `scope.notes` | 否 | 为 `true` 时列出全部用户脚注、尾注（`footnotes` / `endnotes`）以及分隔符（`noteSeparators`）。每条用户注带全部注内段落，不再用 `from` / `maxParas` 切。省略或 `false` 则只有两个计数，没有这三份名单 |
| `scope.comments` | 否 | 为 `true` 时列出全部批注（`comments`）。每条带全部批注内段落，不再用 `from` / `maxParas` 切。省略或 `false` 则只有 `commentCount`，没有名单 |
| `scope.charts` | 否 | 为 `true` 时列出正文图表（根上 `charts`）以及每个页眉、页脚容器上的 `charts`。不依赖 `from` / `maxParas`。省略或 `false` 则只有 `chartCount`，没有名单。不把 embedding 字节放进树 |
| `scope.smartArt` | 否 | 为 `true` 时列出正文 SmartArt（根上 `smartArts`）以及每个页眉、页脚容器上的 `smartArts`。不依赖 `from` / `maxParas`。省略或 `false` 则只有 `smartArtCount`，没有名单。不把图示画成图，不读 Fallback 位图 |

`from` / `maxParas` 只切正文，`headerFooter` 只管页眉页脚段落，`pictures` 只展开图名单，`textBoxes` 只展开框名单，`notes` 只展开脚注尾注，`comments` 只展开批注，`charts` 只展开图表名单，`smartArt` 只展开图示名单。八者独立，可只开一边。只要想看正文段落，就必须同时给 `from` 和 `maxParas`。不默认把全文倒出来。正文图与正文框名单只出现在已切出的段落上，不挂到文档根。正文图表名单与正文 SmartArt 名单挂在文档根上，不挂在段落上。`from` 超出 `paraCount` 时正文 `paragraphs` 为空数组，`truncated` 为 `false`，不报错。页眉页脚通常很短，给了对应键就返回该部件全部段落、图、框、图表或 SmartArt，都不再单独分页。框通常短，给了 `textBoxContent: true` 就返回该框全部段落，不再单独分页。用户注和批注通常不多，给了 `notes: true` 或 `comments: true` 就返回全部名单，每条倒出全部段落，不再单独分页。图表通常不多，给了 `charts: true` 就返回全部名单，每张带全部系列，不再单独分页。SmartArt 通常不多，给了 `smartArt: true` 就返回全部名单，每张带全部可见点，不再单独分页。xlsx 的 `scope` 里出现 `textBoxes`、`textBoxContent`、`notes`、`comments` 或 `smartArt` 报 `InvalidField`。pptx 的 `scope` 里出现 `headerFooter`、`textBoxes`、`textBoxContent`、`notes` 或 `comments` 报 `InvalidField`。pptx 认 `scope.pictures`、`scope.charts` 与 `scope.smartArt`。只给 `textBoxContent`、不给 `textBoxes` 报 `InvalidField`。

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
| `scope` | 否 | 省略或空对象：返回 `slideCount`、`pictureCount`、`chartCount`、`smartArtCount` 和页名单；各页有这三项计数，没有 `shapes`，也没有 `pictures` / `charts` / `smartArts` 名单 |
| `scope.slide` | 要形状时必填 | 页序号，从 1 起，按 `p:sldIdLst` 顺序 |
| `scope.maxShapes` | 否 | 该页最多返回多少个文本形状，默认 `2000`，允许范围 `1` 到 `5000` |
| `scope.pictures` | 否 | 为 `true` 时给每页填 `pictures`。省略或 `false` 则只有 `pictureCount`，没有名单。不读媒体字节。不依赖 `scope.slide` |
| `scope.charts` | 否 | 为 `true` 时给每页填 `charts`。省略或 `false` 则只有 `chartCount`，没有名单。不把 chart 部件或 embedding 字节放进树。不依赖 `scope.slide` / `pictures` |
| `scope.smartArt` | 否 | 为 `true` 时给每页填 `smartArts`。省略或 `false` 则只有 `smartArtCount`，没有名单。不把图示画成图，不读 Fallback 位图。不依赖 `scope.slide` / `pictures` / `charts` |

只要想看形状，就必须给 `slide`。不默认把每一页的文本框都倒出来。只给 `maxShapes`、不给 `slide` 报 `ScopeRequired`。只给 `pictures: true`、`charts: true` 或 `smartArt: true`、不给 `slide` 合法，各页都带对应名单、都不带 `shapes`。`slide` 小于 `1` 报 `InvalidField`。`slide` 超出 `slideCount` 时各页都不带 `shapes`，不报错；图名单、图表名单与 SmartArt 名单仍按各自键决定。

### 响应（xlsx）

```json
{
  "schemaVersion": 1,
  "format": "xlsx",
  "id": "xlsx:workbook",
  "pictureCount": 0,
  "chartCount": 0,
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

- 工作簿字段顺序：`id`、`pictureCount`、`chartCount`、`sheets`。`pictureCount` 始终出现，等于各 `worksheet` 的 `pictureCount` 之和，不含 `chartsheet` / `dialogsheet`。`chartCount` 始终出现，等于各 `worksheet` 与 `chartsheet` 的 `chartCount` 之和，不含 `dialogsheet`。
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
      "smartArtCount": 0
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
      "headerLink": { "default": false, "even": null, "first": null },
      "footerLink": { "default": false, "even": null, "first": null }
    }
  ],
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
- 文档根字段顺序：`id`、`paraCount`、`pictureCount`、`textBoxCount`、`footnoteCount`、`endnoteCount`、`commentCount`、`chartCount`、`smartArtCount`、`outline`、`headers`、`footers`、`sections`，有注名单时再跟 `footnotes`、`endnotes`、`noteSeparators`，有批注名单时再跟 `comments`，有图表名单时再跟 `charts`，有 SmartArt 名单时再跟 `smartArts`，有正文切片时再跟 `from`、`truncated`、`paragraphs`。
- `paraCount` 是主文档里 `w:p` 的总数，含表格单元格里的段落，不含文本框里的段落，始终出现。
- 文档根上的 `pictureCount` 始终出现，只计主文档里被收入的 DrawingML 图片个数（判定见上文 docx 图 id 那节），不含页眉页脚，也不含框内图、注内图、批注内图。根上不挂正文 `pictures`。
- 文档根上的 `textBoxCount` 始终出现，只计主文档里被收入的文本框个数，不含页眉页脚。根上不挂正文 `textBoxes`。
- 文档根上的 `footnoteCount` / `endnoteCount` 始终出现，只计用户注（`normal` 或未写 type），不含分隔符。没有注部件则为 `0`。根上不挂注名单，除非给了 `scope.notes`。
- 文档根上的 `commentCount` 始终出现，计 `comments.xml` 里全部 `w:comment`（含回复与墨迹）。没有批注部件则为 `0`。根上不挂批注名单，除非给了 `scope.comments`。
- 文档根上的 `chartCount` 始终出现，只计主文档里被收入的图表个数（判定见上文 docx 图表 id 那节），不含页眉页脚，也不含框内、注内、批注内的图表。没有图表则为 `0`。根上不挂正文 `charts`，除非给了 `scope.charts`。
- 文档根上的 `smartArtCount` 始终出现，只计主文档里被收入的 SmartArt 个数（判定见上文 docx SmartArt id 那节），不含页眉页脚，也不含框内、注内、批注内的图示。没有图示则为 `0`。根上不挂正文 `smartArts`，除非给了 `scope.smartArt`。
- `outline` 始终出现，只含主文档标题段。收入满足下面任一条件的段落：`w:pStyle/@w:val` 为 `Heading1`–`Heading9`（大小写不敏感）或 `标题1`–`标题9`；或者 `w:outlineLvl/@w:val` 为 `0`–`8`。树上 `level` 从 1 到 9。`Heading1` / `标题1` / `outlineLvl=0` 都是 `level: 1`。不是标题的段落不进 `outline`。页眉页脚段落不进 `outline`。
- `headers`、`footers` 始终出现。没有引用则为空数组。容器字段顺序：`id`、`type`、`sections`、`paraCount`、`pictureCount`、`textBoxCount`、`chartCount`、`smartArtCount`，有图名单时再跟 `pictures`，有框名单时再跟 `textBoxes`，有图表名单时再跟 `charts`，有 SmartArt 名单时再跟 `smartArts`，有段落时再跟 `paragraphs`。`sections` 是从 1 起的节号数组，按主文档里 `w:sectPr` 的文档顺序。容器上的 `paraCount` 是该部件内 `w:p` 的个数（边界与正文相同）。`pictureCount` 始终出现，是该部件内被收入的 DrawingML 图片个数（不含框内图）。`textBoxCount` 始终出现，是该部件内被收入的文本框个数。`chartCount` 始终出现，是该部件内被收入的图表个数（不含框内图表）。`smartArtCount` 始终出现，是该部件内被收入的 SmartArt 个数（不含框内图示）。
- 文档根上的 `sections` 始终出现。每个 `w:sectPr` 一项，字段顺序：`id`、`index`、`headerDistance`、`footerDistance`、`headerLink`、`footerLink`。`headerDistance` / `footerDistance` 取该节 `w:pgMar` 的 `w:header` / `w:footer`（§17.6.11），单位 twip，JSON 整数。缺 `w:pgMar` 或缺对应属性则为 `null`，不猜默认值。属性存在但不是非负整数报 `XmlBroken`。不返回 `top` / `bottom` / `left` / `right` / `gutter`。
- `headerLink` / `footerLink` 始终出现，键顺序为 `default`、`even`、`first`。每个键的取值：`true` 表示该类型对本节生效，且本节 `sectPr` 没有对应引用（继承前节；前节也没有部件、规范要求空白时仍是 `true`）；`false` 表示该类型生效，且本节有显式引用；`null` 表示该类型对本节不生效。`even` 是否生效看整篇 `w:evenAndOddHeaders`；`first` 是否生效看本节 `w:titlePg`。第一节的 `default` 只能是 `false`（没有前节可链）。第一节生效的 `even` / `first` 若无引用也是 `false`（空白，不是链接）。两节显式指向同一部件时两边都是 `false`。`headers` / `footers` 仍按部件去重；断开链接后会出现新的容器 id。
- 省略 `scope`、或没有 `from`/`maxParas` 时，没有正文 `paragraphs`、`from`、`truncated`。
- 给了 `from` 和 `maxParas` 时，正文 `paragraphs` 按文档顺序切片。范围内段落数超过 `maxParas` 则 `truncated` 为 `true`。切片里每段始终有 `pictureCount` 与 `textBoxCount`。段落字段顺序：`id`、`index`、`style`、`level`（标题才有）、`text`、`stable`、`pictureCount`、`textBoxCount`，有图名单时再跟 `pictures`，有框名单时再跟 `textBoxes`，该段有脚注或尾注引用时再跟 `footnoteRef` / `endnoteRef`（注 id 数组，按引用在该段内的文档顺序），该段落落在批注范围内时再跟 `commentIds`（批注 id 数组，按范围开始在文档中的顺序；同一段重叠多条都列出）。没有引用或范围则没有这些键。已展开的页眉页脚、框内、注内段落同样可以带 `commentIds`。
- `scope.headerFooter` 为 `true` 时，每个页眉、页脚节点带 `paragraphs`；否则容器上没有该字段。页眉页脚段落节点不带 `pictureCount` / `pictures` / `textBoxCount` / `textBoxes` / `chartCount` / `charts` / `smartArtCount` / `smartArts`，图、框、图表与 SmartArt 仍挂在容器上。
- `scope.pictures` 为 `true` 时，每个页眉、页脚节点带 `pictures`；若同时给了 `from` 与 `maxParas`，切片里每段也带 `pictures`。否则这些节点上都没有该字段。不把媒体字节放进树。每张图的字段顺序：`id`、`docPrId`、`name`、`kind`、`cx`、`cy`、`wrap`、`stable`。
- `scope.textBoxes` 为 `true` 时，每个页眉、页脚节点带 `textBoxes`；若同时给了 `from` 与 `maxParas`，切片里每段也带 `textBoxes`。否则这些节点上都没有该字段。只给 `textBoxes`、不给 `from` / `maxParas`：只展开页眉页脚容器上的框。每个框的字段顺序：`id`、`docPrId`、`name`、`kind`、`cx`、`cy`、`wrap`、`link`、`paraCount`、`pictureCount`、`stable`，有框内段名单时再跟 `paragraphs`，有框内图名单时再跟 `pictures`。
- `scope.notes` 为 `true` 时，根上带 `footnotes`、`endnotes`、`noteSeparators`。没有用户注时前两个为空数组；没有分隔符时 `noteSeparators` 为空数组。用户注按部件内文档顺序。每条用户注字段顺序：`id`、`noteId`、`type`、`host`、`paraCount`、`pictureCount`，有图名单时再跟 `pictures`，然后 `paragraphs`。`noteId` 是 `w:id` 原文。`type` 恒为 `normal`。`host` 是引用所在段落 id 的数组：在主文档、页眉、页脚和文本框里找 `w:footnoteReference` / `w:endnoteReference`，按文档顺序，id 规则与该段落本身相同。一条注被引用多次则 `host` 有多项；没有任何引用则为 `[]`，注仍列出。`paraCount` / `pictureCount` 只计该注元素内、遇 `w:txbxContent` 停住之后的段落与图，规则与正文相同。注内段落字段顺序与正文段落相同，id 带注前缀。注内图的字段顺序与宿主图相同。注内文本框本期不进树、不接受 apply。
- `noteSeparators` 每项字段顺序：`id`、`kind`、`type`、`noteId`。`kind` 为 `footnote` 或 `endnote`。`type` 为 `separator`、`continuationSeparator` 或 `continuationNotice`。对 `noteSeparators` 里的 id 发 apply 报 `InvalidId`。
- `scope.comments` 为 `true` 时，根上带 `comments`。没有批注时为空数组。按 `comments.xml` 里 `w:comment` 的文档顺序。每条字段顺序：`id`、`commentId`、`author`、`initials`、`date`、`done`、`parent`、`kind`、`start`、`end`、`paraCount`、`pictureCount`，有图名单时再跟 `pictures`，然后 `paragraphs`。`commentId` 是 `w:id` 原文。`author` / `initials` / `date` 取对应属性原文，缺则为 `null`。`done` 取 `commentEx/@done`：`1` / `true` / `on` 为 `true`，其余或没有 `commentEx` 为 `false`。`parent` 用 `paraIdParent` 对上父批注最后一段的 `w14:paraId`，再写成 `comment:{id}`；对不上或没有则为 `null`。根批注的 `parent` 为 `null`。`kind` 为 `normal`；批注内出现 `inkml:ink` 或等价墨迹标记则为 `ink`。`start` / `end` 是范围落在的段落 id（正文、页眉、页脚、框内或注内，规则与该段落本身相同）。从配对的 `commentRangeStart` 走到 `commentRangeEnd`，路径上经过的每个 `w:p` 都算在范围内；`start` 是第一个，`end` 是最后一个。回复有自己的范围标记时按自己的标记算；没有时沿用线程根的 `start` / `end`，并出现在那些段的 `commentIds` 里。找不到对应段（残缺标记）则该侧为 `null`，该批注仍列出，不报 `XmlBroken`。`paraCount` / `pictureCount` 只计该 `w:comment` 内、遇 `w:txbxContent` 停住之后的段落与图，规则与正文相同。批注内段落字段顺序与正文段落相同，id 带批注前缀。批注内图的字段顺序与宿主图相同。批注内文本框本期不进树、不接受 apply。
- `scope.charts` 为 `true` 时，根上带 `charts`（正文），每个页眉、页脚容器带 `charts`。没有图表时为空数组。正文按主文档段落文档顺序，同一段内按 drawing 顺序。每张字段顺序：`id`、`docPrId`、`name`、`kind`、`cx`、`cy`、`wrap`、`type`、`title`、`catAxisTitle`、`valAxisTitle`、`series`、`legend`、`dataLabels`、`flags`、`stable`。`kind` / `cx` / `cy` / `wrap` / `docPrId` / `name` / `stable` 规则与图相同。`type` 为 `bar`、`col`、`line`、`pie`、`area`、`scatter`、`doughnut`、`radar`、`bubble`、`combo`、`chartex` 或 `other`。`barChart` / `bar3DChart` 看 `c:barDir`：`bar` 为 `bar`，`col` 或缺省为 `col`。`lineChart` / `line3DChart` 为 `line`，`pieChart` / `pie3DChart` 为 `pie`，`areaChart` / `area3DChart` 为 `area`，`scatterChart` 为 `scatter`，`doughnutChart` 为 `doughnut`，`radarChart` 为 `radar`，`bubbleChart` 为 `bubble`。`plotArea` 里出现两种及以上系列包裹（例如 `barChart` 加 `lineChart`）则为 `combo`。其余（股价、曲面、复合饼图、chartex 以外的扩展）为 `other`。`title` 取 `c:title` 可见文本（`c:tx` 的 `c:rich` 里 `a:t` 拼接，或 `c:strRef` 的 cache）；没有标题、或 `c:autoTitleDeleted/@val` 为真则为 `null`。`catAxisTitle` / `valAxisTitle` 取主分类轴、主数值轴的 `c:title`，规则与图表标题相同；没有则为 `null`。散点图的横轴当分类轴。`series` 按 `c:ser` 顺序。每条字段顺序：`id`、`index`、`name`、`categories`、`values`，气泡图再跟 `sizes`，cache 与 embedding 不一致时再跟 `cacheStale: true`。`index` 是从 0 起的出现序号。`name` 取系列 `c:tx` 的 cache 或字面量，缺则为 `null`。`categories` / `values` 取 `c:cat` / `c:val`（散点图、气泡图为 `c:xVal` / `c:yVal`）的 cache 或字面量；没有 cache 再读 embedding 里公式指向的单元格。气泡图的 `sizes` 取 `c:bubbleSize`，规则与 `values` 相同；没有则为 `[]`。非气泡系列不出现 `sizes`。全是数字则 JSON 数字，否则字符串。没有数据则为 `[]`。cache 与 embedding 都有且对不上时以 embedding 为准，并标 `cacheStale`。`legend` 为 `{ "show": bool, "pos": ... }`。没有 `c:legend` 则 `show` 为 `false`、`pos` 为 `null`；有则 `show` 为 `true`，`pos` 取 `c:legendPos/@val`（`b` / `t` / `l` / `r` / `tr`），缺属性则为 `null`。`dataLabels` 为 `{ "show": bool }`：`plotArea` 或任一系列带 `c:dLbls`，且其中至少一个 `c:showVal` / `c:showCatName` / `c:showSerName` / `c:showPercent` 为 `1` / `true` / `on` 时为 `true`，否则 `false`。`flags` 是字符串数组，可能含 `secondaryAxis`（出现 `c:overlay` 为 1 的轴，或系列 `c:axId` 指向次轴）、`trendline`（`c:trendline`）、`errorBars`（`c:errBars`）、`pivot`（`c:pivotSource` 或等价透视引用）。没有这些则为 `[]`。不把图表画成图，不读图表的渲染缓存图。xlsx / pptx 的 `scope` 里出现 `charts` 报 `InvalidField`。
- `scope.smartArt` 为 `true` 时，根上带 `smartArts`（正文），每个页眉、页脚容器带 `smartArts`。没有图示时为空数组。正文按主文档段落文档顺序，同一段内按 drawing 顺序。每张字段顺序：`id`、`docPrId`、`name`、`kind`、`cx`、`cy`、`wrap`、`layout`、`colors`、`style`、`points`、`stable`。`kind` / `cx` / `cy` / `wrap` / `docPrId` / `name` / `stable` 规则与图相同。`layout` 取 layout 部件根元素 `dgm:layoutDef/@uniqueId` 的原文；没有该部件或没有该属性则为 `null`。`colors` 取 colors 部件 `dgm:colorsDef/@uniqueId`；`style` 取 quickStyle 部件 `dgm:styleDef/@uniqueId`；缺则为 `null`。`points` 按 data 部件 `ptLst` 文档顺序，只含可见点（判定见上文 SmartArt id 那节）。每条字段顺序：`id`、`modelId`、`role`、`text`、`parent`。`modelId` 是 `dgm:pt/@modelId` 原文。`role` 为 `node` 或 `asst`（`type` 缺省当作 `node`）。`text` 取该点 `dgm:t` 里 `a:t` 按文档顺序拼接；`a:br` 变成 `\n`；没有 `dgm:t` 则为 `""`。`parent` 规则见上文。缺少 `r:dm` 或 data 部件解析失败时该图示仍列出，`points` 为 `[]`，`layout` / `colors` / `style` 能读则读、不能则为 `null`，不报 `XmlBroken`。不读 Fallback 位图，不把图示画成图。xlsx 的 `scope` 里出现 `smartArt` 报 `InvalidField`。
- `link` 始终出现。未链接则为 `{ "prev": null, "next": null }`。链接时 `prev` / `next` 是相邻框的 id，没有那一端则为 `null`。
- 框内 `paragraphs` 的字段顺序与正文段落相同（含 `pictureCount`）。框内图的字段顺序与宿主图相同，id 带框前缀。
- `kind`：`inline`（`wp:inline`）或 `anchor`（`wp:anchor`）。
- `cx` / `cy`：`wp:extent` 的值，单位 EMU，JSON 整数。缺则为 `null`。属性存在但不是整数报 `XmlBroken`。
- `wrap`：行内为 `null`；浮动取子元素 `wrapSquare` / `wrapTight` / `wrapThrough` / `wrapTopAndBottom` / `wrapNone`，对应 `square` / `tight` / `through` / `topAndBottom` / `none`。没有这些子元素则为 `null`。
- `docPrId` 取 `wp:docPr/@id` 的原文；没有该属性则为 `null`。`name` 取 `wp:docPr/@name` 的原文；没有则为 `null`。
- `stable`：有 `wp:docPr/@id` 为 `true`，否则为 `false`。
- `style` 取 `w:pStyle/@w:val` 的原文。没有 `pStyle` 则为 `null`（Word 此时按 Normal 显示，树上不猜）。
- `text` 是可见纯文本：该 `w:p` 内、不在 `w:del` 里的 `w:t` 按顺序拼接；`w:tab` 变成 `\t`，`w:br` 变成 `\n`。`w:instrText`（域指令）不进入。不含 run 级样式。嵌在 `w:txbxContent` 里的 `w:t` 不算，`mc:AlternateContent` 只取一支，判定见上文 docx 那节的边界两条。
- `stable`：有 `w14:paraId` 为 `true`，否则为 `false`。
- 不返回修订痕迹和目录域。文本框里的段落不并进外层段落的 `text`；它们只出现在对应框的 `paragraphs` 里（需 `textBoxContent`）。含超链接或域的段落仍出现在树上，`text` 只含可见结果。页码域按同一规则：inspect 不收域指令与缓存，apply 验收只用无域、无超链接的页眉页脚段落。

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
  "slides": [
    {
      "id": "slide:1",
      "index": 1,
      "name": null,
      "hidden": false,
      "pictureCount": 0,
      "chartCount": 0,
      "smartArtCount": 0,
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
      "pictureCount": 0,
      "chartCount": 0,
      "smartArtCount": 0
    }
  ]
}
```

规则：

- 只读 Presentation 部件和被点名的幻灯片部件。备注、讲义、母版、版式不读。
- `slideCount` 是 `p:sldIdLst` 里 `p:sldId` 的个数，始终出现。
- `pictureCount` 在演示文稿根上始终出现，等于各页 `p:pic`（带 `a:blip`）之和。每页始终有自己的 `pictureCount`。
- `chartCount` 在演示文稿根上始终出现，等于各页被收入的图表之和。每页始终有自己的 `chartCount`。
- `smartArtCount` 在演示文稿根上始终出现，等于各页被收入的 SmartArt 之和。每页始终有自己的 `smartArtCount`。
- `slides` 始终列出全部页（含隐藏页）。未点名的页不带 `shapes`、`truncated`。
- `name` 取 `p:cSld/@name` 的原文。没有该属性则为 `null`。
- `hidden`：`p:sld/@show` 为 `0` 或 `false` 时为 `true`，其余为 `false`。
- 省略 `scope` 或只给空对象时，没有 `shapes`、`truncated`，也没有 `pictures` / `charts` / `smartArts` 名单。
- 给了 `scope.slide` 时，只给那一页填 `shapes`。该页带 `p:txBody` 的 `p:sp` 超过 `maxShapes` 则 `truncated` 为 `true`，按该页文档顺序截断。未截断则为 `false`。
- `scope.slide` 大于 `slideCount` 时，各页都不带 `shapes`，不报错。
- `scope.pictures` 为 `true` 时，每页带 `pictures`。没有图时为空数组。不依赖 `scope.slide`。每张字段顺序：`id`、`cNvPrId`、`name`、`x`、`y`、`cx`、`cy`、`stable`。`cNvPrId` 是 `p:cNvPr/@id` 原文，没有则为 `null`。`name` 取 `p:cNvPr/@name`，没有则为 `""`。`x` / `y` 取 `p:spPr/a:xfrm/a:off` 的 EMU；`cx` / `cy` 取 `a:ext`。缺对应属性则为 `null`。不读媒体字节。
- `scope.charts` 为 `true` 时，每页带 `charts`。没有图表时为空数组。不依赖 `scope.slide`。每张字段顺序：`id`、`cNvPrId`、`name`、`x`、`y`、`cx`、`cy`、`type`、`title`、`catAxisTitle`、`valAxisTitle`、`series`、`legend`、`dataLabels`、`flags`、`stable`。`cNvPrId` / `name` / `stable` 规则与图相同。`x` / `y` / `cx` / `cy` 取 graphicFrame 的 `p:xfrm`。`type` / `title` / 轴标题 / `series` / `legend` / `dataLabels` / `flags` 规则与 docx 图表相同；系列值优先读 embedding 单元格，读不到再退回 cache。
- `scope.smartArt` 为 `true` 时，每页带 `smartArts`。没有图示时为空数组。不依赖 `scope.slide`。每张字段顺序：`id`、`cNvPrId`、`name`、`x`、`y`、`cx`、`cy`、`layout`、`colors`、`style`、`points`、`stable`。`cNvPrId` / `name` / `stable` 规则与图相同。`x` / `y` / `cx` / `cy` 取 graphicFrame 的 `p:xfrm`。`layout` / `colors` / `style` / `points` 规则与 docx SmartArt 相同。pptx 没有 `kind` / `wrap`。
- 页对象字段顺序：`id`、`index`、`name`、`hidden`、`pictureCount`、`chartCount`、`smartArtCount`，然后按需 `pictures`，再按需 `charts`，再按需 `smartArts`，再按需 `truncated` 与 `shapes`。根对象字段顺序：`schemaVersion`、`format`、`id`、`slideCount`、`pictureCount`、`chartCount`、`smartArtCount`、`slides`。
- `shapeId` 是 `cNvPr/@id` 的原文。走 `shape:index:{k}` 时为 `null`。
- 形状上的 `name` 取 `cNvPr/@name` 的原文。没有则为 `null`。
- `text` 是可见纯文本：该形状 `p:txBody` 内、位于 `a:r` 里的 `a:t` 按文档顺序拼接；`a:br` 变成 `\n`。不含 run 级样式。`a:fld` 是应用程序更新的生成文本（§21.1.2.2.4），域里的 `a:t` 是缓存，不进入。`mc:AlternateContent` 只取一支，判定见上文 pptx 那节。
- `stable`：有 `cNvPr/@id` 为 `true`，否则为 `false`。
- 第一期不返回动画、切换、备注、母版、版式。

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

最小 `styles.xml` 必须生成，见 [opc.md](opc.md)。默认字体颜色写 `<color rgb="FF000000"/>`，不引用 theme，包里也不生成 theme 部件。给了 `sheets[].pictures` 时按 `insertPicture` 同一套规则写 drawing 与 media。给了 `sheets[].charts` 时按 `insertChart` 同一套规则写 `/xl/charts/chartN.xml`、`xdr:graphicFrame` 和 `dataFrom` 一带单元格，不写 embedding。一张表上的图与图表共用一份 drawing：先写 `pictures`，再写 `charts`。`xdr:cNvPr/@id` 取该表已有全部 `cNvPr` 的最大整数加 1。不要生成透视表、条件格式、图表工作表。`workbook` 根上或 `cells[]` 上出现 `pictures` / `charts`，或 `sheets[]` 上出现 `kind` / `wrap` / `smartArts`，报 `InvalidField`。`sheets[].kind` 仍拒绝。

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

| 字段 | 含义 |
| --- | --- |
| `path` | 输出路径。已存在则覆盖。省略时必须由调用方另给落盘路径，命令行用 `--output` |
| `document.paragraphs` | 段落列表。可空；空则生成一个空的 `w:p`，否则 Word 可能打不开 |
| `paragraphs[].text` | 可见文本，可空字符串。省略当作 `""` |
| `paragraphs[].style` | 段落样式 id。省略或 `Normal` 表示正文。第一期只允许 `Normal`、`Heading1`、`Heading2`、`Heading3`（大小写不敏感，写出时用这里的 canonical 形式） |
| `paragraphs[].textBoxes` | 可选。该段上的文本框列表。省略则不写框 |
| `paragraphs[].pictures` | 可选。该段上的图列表。省略则不写图 |
| `paragraphs[].smartArts` | 可选。该段上的 SmartArt 列表。省略则不写图示。字段与 `insertSmartArt` 相同，但不含 `id` |
| `document.headers` / `document.footers` | 可选。默认页眉或页脚。省略则不写对应部件 |
| `headers[].type` / `footers[].type` | 必须是 `"default"`，或省略当作 `default`。`even` / `first` 报 `InvalidField`。同一侧只能有一项 |
| `headers[].paragraphs` / `footers[].paragraphs` | 可选。空或省略则写一个空的 `w:p`。字段与正文段落相同，但不允许 `style`、不允许 `textBoxes`；允许 `pictures` 与 `smartArts` |

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

不把 base64 写进 create JSON。xlsx 的图只认 `workbook.sheets[].pictures`，图表只认 `workbook.sheets[].charts`。pptx 的图只认 `presentation.slides[].pictures`，图表只认 `presentation.slides[].charts`，SmartArt 只认 `presentation.slides[].smartArts`。`presentation` 根上或 `shapes[]` 上出现 `pictures` / `charts` / `smartArts` 报 `InvalidField`。xlsx 上出现 `smartArts` 同样报 `InvalidField`。docx create 仍不写图表。

create 写出 Transitional 最小包，见 [opc.md](opc.md)。每个 `w:p` 都带唯一的 `w14:paraId` 和 `w14:textId`（取值大于 0、小于 `0x80000000`，MS-DOCX §2.6.2.3 / §2.6.2.4），并在根元素上把 `w14` 列入 `mc:Ignorable`。这样随后 inspect 得到的 id 都是 `para:{paraId}`，`stable` 为 `true`。图的 `wp:docPr/@id` 在该部件内从 `1` 起递增，与 `insertPicture` 同一套写入。

标题段写出 `<w:pStyle w:val="Heading1"/>` 这类引用，样式定义放在 `/word/styles.xml`。同时写 `/word/settings.xml`，含 `<w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/>`（MS-DOCX §2.3.5）。缺这项时 Word 按兼容模式打开（默认 12）。`textBoxes` 写成浮动 `square` 的 `wps:wsp`，与 `insertTextBox` 同一套最小 Choice。`smartArts` 复用 `insertSmartArt` 的四件套生成。给了 `headers` / `footers` 时写 `/word/header1.xml` 或 `/word/footer1.xml`，并在第一节 `sectPr` 上挂 `default` 引用。不要生成多节、目录、修订、脚注、尾注、批注、图表。默认页眉页脚带框不在本期 create。不写入 `titlePg` / `evenAndOddHeaders`。

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

create 写出 Transitional 最小包，见 [opc.md](opc.md)。必须带 theme、slideMaster、slideLayout、presentation、各页 slide、presProps，不能只写 ECMA 示例里那个仅含 `notesSz` 的 presentation。幻灯片尺寸用 16:9（`sldSz` 为 `cx="12192000"` `cy="6858000"`）。不要生成动画、切换、备注、批注。给了 `slides[].pictures` 时按 `insertPicture` 同一套规则写 `/ppt/media/…`、该页 `.rels` 与 `p:pic`。给了 `slides[].charts` 时按 `insertChart` 同一套规则写 `/ppt/charts/…`、embedding 与图表 `p:graphicFrame`。给了 `slides[].smartArts` 时按 `insertSmartArt` 同一套规则写 `/ppt/diagrams/…` 与图示 `p:graphicFrame`。先写该页 `shapes`，再按数组顺序写 `pictures`，再写 `charts`，再写 `smartArts`。`p:cNvPr/@id` 取该页已有全部 `cNvPr` 的最大整数加 1，至少为 2。`presentation` 根上或 `shapes[]` 上出现 `pictures` / `charts` / `smartArts` 报 `InvalidField`。

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

不认识的 `op` 报 `UnsupportedOp`，不要忽略。xlsx 认 `setCell`、`setHeaderFooterText`、`replacePicture`、`insertPicture`、`deletePicture`、`setPictureSize`、`setPictureAnchor`、以及图表 op（`insertChart`、`insertChartSheet`、`deleteChartSheet`、`deleteChart`、`setChartTitle`、`setChartSeries`、`addChartSeries`、`deleteChartSeries`、`setChartType`、`setChartAxisTitle`、`setChartLegend`、`setChartDataLabels`、`setChartSize`、`setChartAnchor`；不含 `setChartWrap`）。docx 认 `setParaText`、`setHeaderFooterDistance`、`setHeaderFooterLink`、图片 op（含 `setPictureWrap`，不含 `setPictureAnchor`）、文本框 op、`insertPara` / `deletePara`、脚注尾注 op（`insertFootnote`、`insertEndnote`、`deleteFootnote`、`deleteEndnote`、`setNoteNumbering`）、批注 op（`insertComment`、`deleteComment`、`replyComment`、`setCommentText`、`setCommentMeta`、`setCommentDone`）、图表 op（`insertChart`、`deleteChart`、`setChartTitle`、`setChartSeries`、`addChartSeries`、`deleteChartSeries`、`setChartType`、`setChartAxisTitle`、`setChartLegend`、`setChartDataLabels`、`setChartSize`、`setChartWrap`；不含 `setChartAnchor`、`insertChartSheet`、`deleteChartSheet`）、以及 SmartArt op（`insertSmartArt`、`deleteSmartArt`、`setSmartArtText`、`addSmartArtPoint`、`deleteSmartArtPoint`、`setSmartArtSize`、`setSmartArtWrap`、`setSmartArtColors`、`setSmartArtStyle`、`setSmartArtLayout`）。pptx 认 `setShapeText`、`replacePicture`、`insertPicture`、`deletePicture`、`setPictureSize`、`setPicturePos`、以及图表 op（`insertChart`、`deleteChart`、`setChartTitle`、`setChartSeries`、`addChartSeries`、`deleteChartSeries`、`setChartType`、`setChartAxisTitle`、`setChartLegend`、`setChartDataLabels`、`setChartSize`、`setChartPos`；不含 `setChartWrap`、`setChartAnchor`、`insertChartSheet`、`deleteChartSheet`）、以及 SmartArt op（`insertSmartArt`、`deleteSmartArt`、`setSmartArtText`、`addSmartArtPoint`、`deleteSmartArtPoint`、`setSmartArtSize`、`setSmartArtPos`、`setSmartArtColors`、`setSmartArtStyle`、`setSmartArtLayout`；不含 `setSmartArtWrap`）。对不上格式的 op（例如对 pptx 发 `setCell` / `setParaText`，对 xlsx 发 `setPictureWrap`、`setChartWrap` 或 `setHeaderFooterDistance` / `setHeaderFooterLink`、文本框、注、批注、SmartArt 的 op，对 docx / pptx 发 `setHeaderFooterText`、`insertChartSheet` 或 `deleteChartSheet`，对 docx 发 `setPictureAnchor` / `setChartAnchor` / `setSmartArtPos`，对 pptx 发 `setPictureWrap` / `setPictureAnchor` / `setChartWrap` / `setChartAnchor` / `setSmartArtWrap`，或对 docx / xlsx 发 `setPicturePos` / `setChartPos` / `setSmartArtPos`）报 `InvalidField`。

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

`styles.xml`、其他工作表、主题一律原样写回，字节不变。只改单元格、不改图、图表或页眉时，绘图原样写回。图表 op 还可以脏 drawing、`/xl/charts/…` 及其 `.rels`、`[Content_Types].xml`，以及被写入数据的工作表 XML；写入单元格时走上面同一套重算标记。`insertChartSheet` / `deleteChartSheet` 还可以脏 `/xl/workbook.xml` 的 `sheets`、workbook 关系、`/xl/chartsheets/…`。页眉文本只脏目标表 XML。页眉图还可以脏 `drawingHF` 指向的 drawing、工作表 `.rels`、`/xl/media/…` 与 `[Content_Types].xml`。

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
5. 含超链接或域的段落仍按上面做。超链接可能不再包住整段新字，域可能被拆掉。验收只用无域、无超链接的段落（含页眉页脚）。
6. 不往文本框里写字。第 2 步找 run 的时候，遇到 `w:txbxContent` 停住，遇到 `mc:AlternateContent` 只进第一个 `mc:Choice`（没有就进 `mc:Fallback`），判定与 inspect 那两条边界一致。某一段除文本框之外没有可改的 `w:t` 时走第 3 步插新 run，插在 `w:pPr` 之后、该段其余内容之前。这样 apply 写进去的文本，随后 inspect 读回来必须与写入值相同。

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

只改目标节 `w:pgMar` 上被点名的属性。没有 `w:pgMar` 时插入一个，只写本次给出的字段，不补 `top` / `left` / `right` / `bottom` / `gutter`。已有的其它边距属性原样留下。

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

不写入 `titlePg` / `evenAndOddHeaders`。未点名的其它页眉页脚字节仍不变。

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
| `id` | 图 id：docx 为 `{容器}/pic:…` 或 `{段落 id}/pic:…`；xlsx 为 `sheet:{name}/pic:…`；pptx 为 `slide:{n}/pic:…` |
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
| `id` | docx：段落 id（正文用 `para:…` / `para:index:{n}`，页眉页脚用 `{容器}/para:…`）。xlsx：工作表 id `sheet:{name}`，或页眉槽位 id `sheet:{name}/hf:{kind}:{side}`。pptx：页 id `slide:{n}` |
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

pptx：找不到该页报 `IdNotFound`。`id` 必须是 `slide:{n}`。对图 id、形状 id 或 `pptx:presentation` 发此 op 报 `InvalidId`。`cx` / `cy` 规则与 docx 相同。`x` / `y` 必须成对出现或成对省略；只给一个时报 `InvalidField`。缺坐标则放到该页空白处，不要叠在 `(0,0)`：读取 `presentation.xml` 的 `p:sldSz`，扫描该页已有 `p:sp` / `p:pic` / `p:graphicFrame` / 嵌套 `p:grpSp` 的 `a:xfrm`，优先放到最右侧空隙，放不下再放到下方。`kind` / `wrap` / `from` / `to` / `anchor` 以及 xlsx 偏移字段出现报 `InvalidField`。写入该页 `p:spTree` 一个新的 `p:pic`（带 `a:blip`），关系指向 `/ppt/media/imageN.*`。`p:cNvPr/@id` 取该页已有全部 `cNvPr`（`sp` / `pic` / `graphicFrame` / 组合，含 `spTree` 自己的 `id="1"`）的最大整数加 1，至少为 2。

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
| `id` | pptx 图 id `slide:{n}/pic:…` |
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

`id` 必须是段落 id，对图 id、框 id、容器 id 或 `section:{n}` 报 `InvalidId`。找不到该段报 `IdNotFound`。`cx` / `cy` 都省略时用 2 英寸宽、1 英寸高（`1828800` / `914400`）。只给一个时报 `InvalidField`。插入浮动 `anchor` + `wrapSquare`，挂在该段末尾一个新的 `w:r` 里，写成 `mc:AlternateContent`（Choice 为 `wps:wsp` / `wps:txbx`，Fallback 为空白 `w:pict`，不编 VML 几何）。`wp:docPr/@id` 取该部件已有 `docPr` 的最大整数加 1。xlsx / pptx 上发此 op 报 `InvalidField`。

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

### insertPara / deletePara（仅框内与注内）

| 字段 | 含义 |
| --- | --- |
| `op` | `"insertPara"` 或 `"deletePara"` |
| `id` | `insertPara` 用框 id、注 id 或批注 id（插到该容器末）或框内 / 注内 / 批注内段落 id（插到该段之后）；`deletePara` 必须是框内、注内或批注内段落 id |
| `text` | 仅 `insertPara`。可选，新段可见文本 |

正文（框外、注外、批注外）插删段仍报 `InvalidId`。框内只剩一段时 `deletePara` 报 `InvalidField`（`w:txbxContent` 至少留一段）。注内只剩一段时同样报 `InvalidField`（一条用户注至少留一个 `w:p`）。批注内只剩一段时同样报 `InvalidField`（一条 `w:comment` 至少留一个 `w:p`）。新段带新的 `w14:paraId`。批注内插删段后必须把该批注 `commentEx/@paraId` 改成新的最后一段 `paraId`；若有回复的 `paraIdParent` 指向旧的最后一段，一并改成新值。xlsx / pptx 上发此 op 报 `InvalidField`。

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
| `start` | 范围起始段落 id：正文、页眉页脚、框内或注内 |
| `end` | 范围结束段落 id，可与 `start` 相同 |
| `text` | 可选。新批注第一段的可见文本，省略当作 `""` |
| `author` | 可选。省略当作 `"ooxp"` |
| `initials` | 可选。省略当作 `"oo"` |
| `date` | 可选。`xsd:dateTime` 字符串，例如 `2026-08-31T12:00:00Z`。省略则不写 `w:date` |

`start` 与 `end` 必须在同一部件，且 `start` 在文档顺序上不得晚于 `end`，否则报 `InvalidField`。不提供字符偏移。在 `start` 段 `w:pPr` 之后插入 `w:commentRangeStart`；在 `end` 段已有内容之后插入 `w:commentRangeEnd`，再跟一个含 `w:commentReference` 的 run（`rStyle` 为 `CommentReference`）。跨段时中间段落整段落在范围内，不另写标记。

分配尚未占用的 `w:id`（十进制，常见从 `0` 起）。没有 `/word/comments.xml` 则新建该部件、主文档关系、内容类型 Override。同时按 Word 16.0 最小集补 `/word/commentsExtended.xml` 与 `/word/people.xml`（已有则改，没有则建）。新 `w:comment` 至少一段，带新的 `w14:paraId`，第一段含 `w:annotationRef`，段落样式引用 `CommentText`，再跟 `text`。不在 `styles.xml` 里新建这些样式。`commentEx/@paraId` 写成该批注最后一段的 `paraId`，`done` 为 `0`。`people` 里没有该 `author` 则追加一条。`commentsIds` / `commentsExtensible` 只在文件里已有时给这条根批注补一项；没有这两份部件则不新建。

对图 id、容器 id、注 id、批注 id 或 `section:{n}` 作 `start` / `end` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### replyComment

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"replyComment"` |
| `id` | 父批注 id，例如 `comment:0` |
| `text` | 可选。新回复第一段的可见文本，省略当作 `""` |
| `author` / `initials` / `date` | 与 `insertComment` 相同 |

新增一条 `w:comment` 和新的 `commentEx`，`paraIdParent` 为父批注当前最后一段的 `paraId`。在父批注已有的范围上再套一套相同范围的标记（`commentRangeStart` 插在父 start 之后，`commentRangeEnd` 与 `commentReference` 插在父 `commentReference` 所在 run 之后），与 Word 16 `Replies.Add` 一致；`rStyle` 抄父引用 run。找不到父批注的范围标记时仍写 `comment` 与 `commentEx`，不报错。inspect 的 `start` / `end` 因此与线程根相同。父批注最后一段没有 `w14:paraId` 时报 `InvalidField`。父 id 不是批注容器 id 报 `InvalidId`。找不到父批注报 `IdNotFound`。对 `kind` 为 `ink` 的批注发此 op 报 `NotImplemented`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteComment

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteComment"` |
| `id` | 批注 id，例如 `comment:0` |

去掉该批注在主文档、页眉、页脚、文本框和注里所有匹配的范围标记与 `commentReference`，再去掉 `comments.xml` 里该条。子回复级联删除（`paraIdParent` 链指向它的都删），清单里验回复不残留。已经不存在报 `IdNotFound`。id 不是批注容器 id 报 `InvalidId`。批注内图按图片规则收。删光批注后仍保留 `comments.xml` 空壳和已有的 extended / people 部件，不拆掉整个 part。xlsx / pptx 上发此 op 报 `InvalidField`。

### setCommentText

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setCommentText"` |
| `id` | 批注容器 id，例如 `comment:0` |
| `text` | 新的可见纯文本，字符串，可空 |

只改该 `w:comment` 的可见文本，不改范围标记。留下第一段（含 `annotationRef`），按 `setParaText` 的规则改这段的可见 `w:t`，删掉后面多出来的 `w:p`。随后把 `commentEx/@paraId` 和指向旧最后一段的 `paraIdParent` 改成留下的那一段。id 不是批注容器 id 报 `InvalidId`。对批注内段落 id 请用 `setParaText`。对 `kind` 为 `ink` 的批注报 `NotImplemented`。xlsx / pptx 上发此 op 报 `InvalidField`。

### setCommentMeta

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setCommentMeta"` |
| `id` | 批注容器 id |
| `author` | 可选 |
| `initials` | 可选 |
| `date` | 可选。`xsd:dateTime` 字符串 |

`author`、`initials`、`date` 至少给一个，否则报 `InvalidField`。只改被点名的属性，其余不动。改 `author` 时若 `people.xml` 里还没有这个作者则补一条，不删旧作者。id 不是批注容器 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### setCommentDone

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setCommentDone"` |
| `id` | 批注容器 id |
| `done` | 布尔。`true` 写成 `commentEx/@done="1"`，`false` 写成 `"0"` |

只改被点名这一条的 `done`，不自动改回复。没有 `commentsExtended` 则按 Word 16 最小集新建，并为文件里每条批注补 `commentEx`（默认 `done="0"`），再改被点名的那条。父批注最后一段没有 `w14:paraId` 时报 `InvalidField`。xlsx / pptx 上发此 op 报 `InvalidField`。

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
| `id` | docx：宿主段落 id（正文或页眉页脚段落）。xlsx：工作表 id `sheet:{name}`。pptx：页 id `slide:{n}` |
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

docx：在该段末插入 `w:drawing`，默认浮动。新建 `/word/charts/chartN.xml`（`N` 从 1 起尚未占用）、chart 的 `.rels`、一份最小 embedding xlsx（工作表名 `Sheet1`，分类从 `A2` 起，系列名在第 1 行，数值从 `B2` 起向右），以及主文档或页眉页脚关系、`[Content_Types].xml` Override。chart 的 Content Type 为 `application/vnd.openxmlformats-officedocument.drawingml.chart+xml`。关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart`。embedding 关系 Type 为 `http://schemas.openxmlformats.org/officeDocument/2006/relationships/package`。`wp:docPr/@id` 取该部件已有 `docPr` 的最大整数加 1，至少为 1。不强制写 chart colors / style 部件，也不强制套 `mc:AlternateContent`。对图 id、图表 id、系列 id、容器 id、框 id、注 id、批注 id 或 `section:{n}` 发此 op 报 `InvalidId`。找不到段落报 `IdNotFound`。

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
| `id` | docx：宿主段落 id，正文或页眉页脚段落。pptx：页 id `slide:{n}` |
| `layout` | `list` 或 `org` |
| `points` | 可选。可见点数组。每项可含 `text`、`parent`、`role`。`parent` 是本数组里从 0 起的下标，表示父点；省略或 `null` 表示挂到文档根点下 |
| `kind` | docx 可选。`inline` 或 `anchor`，缺省 `anchor`。pptx 上出现报 `InvalidField` |
| `wrap` | docx 可选。浮动时的绕排，缺省 `square`。行内必须省略或为 `null`。pptx 上出现报 `InvalidField` |
| `x` / `y` | pptx 可选。画布坐标，EMU，非负整数。必须成对。docx 上出现报 `InvalidField` |
| `cx` / `cy` | 可选。EMU，正整数。都省略则用 `5486400` × `3200400`。只给一个报 `InvalidField` |

docx：在该段末插入 `w:drawing`，默认浮动。新建 `/word/diagrams/` 下尚未占用的 data / layout / colors / quickStyle 四件套（序号从 1 起），并把四条关系写在**宿主** `.rels` 上（主文档或页眉页脚）。`wp:docPr/@id` 取该部件已有 `docPr` 的最大整数加 1，至少为 1。对图 id、图表 id、SmartArt id、点 id、容器 id、框 id、注 id、批注 id、`section:{n}` 或 `slide:{n}` 发此 op 报 `InvalidId`。找不到段落报 `IdNotFound`。

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

允许变脏的部件：主文档 XML（通常是 `/word/document.xml`）及其 `.rels`，被 `setParaText`、图片 op、文本框 op、图表 op 或 SmartArt op 点名的页眉、页脚部件及其 `.rels`，`setHeaderFooterLink` 改到的主文档关系、`[Content_Types].xml`、新建或删除的 header/footer 部件及其 `.rels`，脚注 / 尾注部件及其 `.rels`，批注部件 `/word/comments.xml`、`commentsExtended.xml`、`people.xml`，以及文件里已有的 `commentsIds.xml` / `commentsExtensible.xml`，图表部件 `/word/charts/…` 及其 `.rels`，embedding `/word/embeddings/…`，SmartArt 四件套 `/word/diagrams/…` 及可选 drawing 缓存，`setNoteNumbering` 在整篇上改到的 `settings.xml` 与主文档（各节 `sectPr`），以及新媒体或被删的 `/word/media/…`。`setHeaderFooterDistance` 只脏主文档。正文图片 op、文本框 op、图表 op 与 SmartArt op 脏主文档与主文档 `.rels`。一次 `ops` 可以混用 `setParaText`、`setHeaderFooterDistance`、`setHeaderFooterLink`、图片 op、文本框 op、脚注尾注 op、批注 op、图表 op 与 SmartArt op。`styles.xml`、未点名的页眉页脚、主题、编号定义（`numbering.xml`）一律原样写回。未点名的 `settings.xml` 也原样写回。`setNoteNumbering` 只点名某一节时只脏主文档，不改 `settings.xml`。`replyComment` 会脏出现父批注范围标记的宿主部件（常见是主文档）。只改批注文本、元数据或完成标记时，未出现范围标记的宿主部件（常见是主文档）原字节写回。只改图表 cache / 标题 / 图例、且宿主 drawing 未动时，未点名的正文段落字节按「drawing 未动」验收。只改 SmartArt 点文字或增删点、且宿主 drawing 未动时，同样按「drawing 未动」验收。

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
| `id` | 形状 id（`slide:{n}/shape:{shapeId}` 或 `slide:{n}/shape:index:{k}`） |
| `text` | 新的可见纯文本，字符串，可空。必填。`\\n` 写成 `a:br`，不把字面换行两个字符写进 `a:t` |

找不到该形状报 `IdNotFound`。第一期不插入、不删除形状。对 `slide:{n}`（没有 `/shape:`）发本 op 报 `InvalidId`。

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

### 改动范围（pptx）

允许变脏的部件：被改的那一张幻灯片 XML（通常是 `/ppt/slides/slideN.xml`，N 以关系目标为准）及其 `.rels`，`[Content_Types].xml`，以及新建或被删的 `/ppt/media/…`、`/ppt/charts/…`、embedding、`/ppt/diagrams/…` 及可选 drawing 缓存。`setShapeText` 只脏该页 XML。图片 op 还可脏该页关系与媒体。图表 op 还可脏 chart 部件、embedding。SmartArt op 还可脏四件套与缓存。`presentation.xml`、其他页、母版、版式、主题、备注、`presProps` 一律原样写回。一次 `ops` 可以混用 `setShapeText`、图片 op、图表 op 与 SmartArt op。只改 SmartArt 点文字或增删点、且宿主 graphicFrame 未动时，该页 XML 按「drawing 未动」验收。

被改部件同样整份重序列化，保留程度与 xlsx 工作表相同：元素与属性顺序、未知命名空间保留；注释、处理指令、DOCTYPE 会丢。

## exportMedia

按图 id 把 OPC 里的媒体部件原样写到本地文件。不读进 inspect 树，不做转码、缩放或预览。

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
| `id` | 是 | 图 id，与 inspect 的 `pictures[].id` 相同 |
| `output` | 否 | 本地目标路径。省略时用命令行 `--output`。两者都缺报 `MissingField`。不得默认写成源 `path`，以免把图片字节盖掉文档 |
| `format` | 否 | 可省略，从 `path` 扩展名推断 |

`id` 必须是图 id。对段落 id、工作表 id、单元格 id、页眉页脚容器 id、`section:{n}`、形状 id 或 `slide:{n}`（没有 `/pic:`）报 `InvalidId`。找不到该图报 `IdNotFound`。优先走 `a:blip/@r:embed`；只有 `@r:link`、没有 embed 时按外部关系处理，报 `OpcPartMissing`，不下载 http。embed 指向外部关系同样报 `OpcPartMissing`。

写出的是该媒体部件的原字节。`contentType` 取自 `[Content_Types].xml` 对该部件的 Default 或 Override；都没有则省略该字段。扩展名由调用方自己定，库不按类型改名。

xlsx 用工作表图 id 或页眉图 id，写出 `/xl/media/…`。pptx 用幻灯片图 id，写出 `/ppt/media/…`。docx 文本框里的图用框内段落上的图 id，同一动作能导出。脚注、尾注里的图用注内段落上的图 id，同一动作能导出。批注里的图用批注内段落上的图 id，同一动作能导出。嵌入图表的工作簿另走 `exportChartData`，不在本动作。

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

`id` 必须是图表 id。对系列 id、段落 id、形状 id、图 id 报 `InvalidId`。找不到该图报表 `IdNotFound`。没有 embedding 报 `OpcPartMissing`。关系指向外部资源同样报 `OpcPartMissing`，不下载 http。xlsx 本期报 `InvalidField`。

写出的是 embedding 部件的原字节。`contentType` 取自 `[Content_Types].xml`；没有则省略。

成功时 stdout 见 [errors.md](errors.md)。源包不写回。

## 两层 JSON

领域 JSON 就是本文。传输层（将来的 MCP）可以把本文整个对象放进 RPC 的 `params`。不许把 JSON-RPC 的 `jsonrpc`、`method`、`id` 塞进领域对象，也不许让单元格字段去迁就 RPC。
