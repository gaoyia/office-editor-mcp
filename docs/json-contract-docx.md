# docx 领域 JSON

本文是 [json-contract.md](json-contract.md) 的 docx 分册。这里写稳定 id、inspect 的 scope 与响应、create、文档保护，以及只属于 Word 文档的 apply op。

信封、跨格式 op（图、图表、SmartArt、批注、超链接、文本框、表格插删行列、`setProps`、`setProtection` 等）以及 `exportMedia` / `exportChartData` 仍在主文件。

机器可读副本见 [json-schema/docx.schema.json](json-schema/docx.schema.json)，
入口仍是 [json-schema/request.schema.json](json-schema/request.schema.json)。与 Markdown 冲突时以本文与主契约为准。

## 稳定 id

文档根节点 id 固定为 `docx:document`。

段落 id 优先用 `w14:paraId`（十六进制，MS-DOCX §2.6.2.3，命名空间 `http://schemas.microsoft.com/office/word/2010/wordml`）。
有该属性时：`para:{paraId}`，例如 `para:3F2A1B00`。输出用文件里的原始大小写。请求里的十六进制按大小写不敏感匹配。

没有 `w14:paraId` 时：`para:index:{n}`，`n` 从 1 起，按主文档部件里 `w:p` 的文档顺序（含表格单元格里的段落）。树上同时给 `"stable": false`。
这类 id 在增删段落后会变，只够同一次 inspect 之后立刻 apply。

标题和正文都是段落，不另编一套 id。脚注、尾注、批注都是独立故事：根上始终有计数；给了对应 `scope` 才列出名单，并可 apply。分隔符三条不进用户名单，禁止 apply。
文本框是独立节点：框本身、框内段落与框内图按下面规则进树，并可 apply。页眉、页脚按下面规则进树，可按 id 改字；
节上的页眉页脚距边界、页面边距、纸张、分栏、垂直对齐、行号、文档网格、装订线侧、页面边框，以及是否链接到前一节，都可以读写。
整篇的镜像边距与装订线在上，写在 `settings.xml`。整篇页面背景色写在主文档 `w:background`，是否显示看 `settings.xml` 的 `w:displayBackgroundShape`。
页面填充可以是 VML 渐变 / 图片，也可以是 `w:drawing` 里的 DrawingML。
节上页面边框的 `val` 认 `ST_Border` 全套，含艺术型（如 `apples`）。
页眉、页脚、正文、注和批注里的 DrawingML 图片可查、换、插、删，并可改尺寸与绕排。正文与页眉页脚里的嵌入对象（OLE / 包）可查、插、删，并可按 id 导出嵌入部件；框内、注内、批注内的 OLE 本期不编。
正文与页眉页脚里的 `w:tbl` 可查、插、删，并可插删行与列、合并与拆分单元格。
也可改表格样式、`tblLook`、浮动定位、表级格边距，以及单元格底纹、格边距与斜线。
正文与页眉页脚里的 run 级插入、删除与段落/run 属性修订可按范围列出，并可接受或拒绝被点名的一条。
可用 `insertStyle` / `deleteStyle` 增删自定义样式，可用 `setRunStyle` 给 run 套字符样式。
apply 可用 `insertHeaderFooter` 为某节某类型新建页眉或页脚部件，可用 `setTitlePage` 写本节 `w:titlePg`，
可用 `setEvenAndOddHeaders` 写整篇 `w:evenAndOddHeaders`，可用 `insertSection` 在某段之后插入分节符，可用 `deleteSection` 删掉非末节并把内容并进下一节，
可用 `insertToc` 插入目录域、用 `deleteToc` 删掉目录块，
可用 `insertCaption` 插入题注、用 `insertCrossRef` 插入交叉引用、用 `deleteCaption` / `deleteCrossRef` 删掉，
可用 `insertIndex` 插入索引、用 `insertIndexEntry` 插入索引项、用 `deleteIndex` / `deleteIndexEntry` 删掉，
可用 `insertSource` 插入书目源、用 `setBibliographyStyle` 改 `SelectedStyle`、用 `insertCitation` 插入引文、用 `insertBibliography` 插入书目、
用 `deleteSource` / `deleteCitation` / `deleteBibliography` 删掉，
可用 `insertDocPart` 插入构建基块、用 `insertDocPartContent` 把基块复制进正文、用 `deleteDocPart` 删掉，
可用 `insertEmbeddedFont` / `deleteEmbeddedFont` / `setFontEmbed` 读写嵌入字体，
可用 `setPageBackground` 写整篇页面背景色或 DrawingML / 图片填充、用 `setPageBorders` 写节上页面边框（含艺术型），可用 `insertMath` / `setMath` / `deleteMath` 插改删 OMML 公式，
可用 `insertContentControl` / `setContentControl` / `deleteContentControl` 插改删内容控件，
可用 `setParaFormat` / `setRunFormat` 写禁则、强调点、压缩排版、段边框、段底纹、制表位，以及 run 的突出显示、删除线、上下标、字符间距、字距调整与字符缩放，可用 `insertRuby` / `setRuby` / `deleteRuby` 插改删拼音，
可用 `setEastAsianSettings` 写整篇压缩、自定义禁则字符与 `themeFontLang`。
docx 可用 `setProtection` 开关无密码的限制编辑与只读推荐。xlsx 可用同一 `op` 开关工作表保护与工作簿结构保护（可写 SHA-512 哈希），不要复用 docx 根上那个 `protection` 对象。
docx 可用 `insertWatermark` / `deleteWatermark` 读写页眉里的 DrawingML 水印。
正文与页眉页脚里的浮动图 / 文本框 / 连接线 / 无框形状可用 `groupShapes` / `ungroup` 组合或解开；
Word 写出的连接线可进 inspect，并可用 `insertConnector` / `deleteConnector` 增删。
无文本框的 `wps:wsp` 形状可进 inspect，并可用 `insertShape` / `deleteShape` / `setShapePos` / `setShapeSize` / `setShapeFill` / `setShapeLine` 增删改。
正文与页眉页脚里的墨迹画布（非批注）可进 inspect，并可用 `insertInkCanvas` / `replaceInkCanvas` / `deleteInkCanvas` / `setInkCanvasSize` / `setInkCanvasWrap` 增删换与改尺寸绕排，
也可按墨迹 id `exportMedia`。不要和批注里的 `kind: ink` 混。
正文与页眉页脚里的 3D 模型可进 inspect，并可用 `insertModel3d` / `replaceModel3d` / `deleteModel3d` / `setModel3dSize` / `setModel3dWrap` / `setModel3dCamera` 增删换、改尺寸绕排与改相机 / 旋转 / 光源，
也可按 3D id `exportMedia`。不是图，不进 `pictures`。注内、批注内本期不插。
组合本身不进树。docx 可用 `setThemeColors` / `setThemeFonts` / `setThemeFmt` 改主文档 theme 的配色、字体、方案名与 `fmtScheme` 常用槽，可用 `setThemeExtraSchemes` 整份替换补充色板，可用 `setThemeObjectDefaults` 改默认形状 / 线 / 文本框样式，可用 `setThemeOverride` 读写主文档 `themeOverride` 部件。可用 `setEastAsianSettings` 写 `themeFontLang`。
三种格式都可用 `setProps` / `setCustomProp` / `deleteCustomProp` 改文件属性。
`create` 可在正文和页眉页脚上写 DrawingML 图，也可写文本框、SmartArt、图表、墨迹画布、3D 模型、脚注、尾注、批注与回复、超链接、书签、域、公式、内容控件、拼音、无框形状与连接线；
根上可写编号定义、目录块、自定义样式、题注、交叉引用、索引、参考文献、构建基块、嵌入字体、东亚设置、theme 与页面背景 / 页面边框；
页眉页脚允许 `default` / `even` / `first`；不写修订痕迹。
字段见下文 create。嵌入图表不是 `pic:pic`：根上始终有 `chartCount`；给了 `scope.charts` 才列出名单，并可 apply。
SmartArt 不是 `pic:pic`：根上始终有 `smartArtCount`；给了 `scope.smartArt` 才列出名单，并可 apply。
正文与页眉页脚、文本框、脚注、尾注、批注里的图表与 SmartArt 走同一套 op，id 带所在段落前缀。注内与批注内的文本框同样进树，并可 apply。
框内再套框仍拒绝：Word 16.0 只写出故事级兄弟框，`w:txbxContent` 里没有嵌套 `wps:wsp`。对框 id 或框内段落发 `insertTextBox` 报 `InvalidId`。

页眉、页脚 id 按部件编，不按节编。`w:sectPr` 里的 `w:headerReference` / `w:footerReference`（§17.10.2、§17.10.5）经主文档关系解到部件路径。
同一部件只出一个容器节点；多个节引用同一部件时，`sections` 列出所有节号。
后一节某类型引用省略时按规范继承前节（§17.10.2）；`even` 是否继承还看 `settings.xml` 的 `w:evenAndOddHeaders`（§17.10.1），
`first` 是否继承还看该节的 `w:titlePg`（§17.10.6）。
OOXML 没有「链接到前一节」元素；Word UI 的勾选就是这种省略。inspect 在每个节上用 `headerLink` / `footerLink` 标出该类型是显式引用、继承前节，还是对本节不生效。
apply 用 `setHeaderFooterLink` 断开或重新链接；用 `insertHeaderFooter` 在空白槽位新建部件。两节显式指向同一部件不算链接，两边都是 `false`。

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
| 正文连接线 | `{段落 id}/cxn:{docPrId}` 或 `{段落 id}/cxn:index:{k}` | `para:3F2A1B00/cxn:8` |
| 页眉/页脚连接线 | `{页眉或页脚段落 id}/cxn:{docPrId}` | `header:default:1/para:AA/cxn:8` |
| 正文无框形状 | `{段落 id}/shape:{docPrId}` 或 `{段落 id}/shape:index:{k}` | `para:3F2A1B00/shape:8` |
| 页眉/页脚无框形状 | `{页眉或页脚段落 id}/shape:{docPrId}` | `header:default:1/para:AA/shape:8` |
| 正文墨迹画布 | `{段落 id}/ink:{cNvPrId}` 或 `{段落 id}/ink:index:{k}` | `para:3F2A1B00/ink:9` |
| 页眉/页脚墨迹画布 | `{页眉或页脚段落 id}/ink:{cNvPrId}` | `header:default:1/para:AA/ink:9` |
| 正文 3D 模型 | `{段落 id}/model3d:{docPrId}` 或 `{段落 id}/model3d:index:{k}` | `para:3F2A1B00/model3d:1` |
| 页眉/页脚 3D 模型 | `{页眉或页脚段落 id}/model3d:{docPrId}` | `header:default:1/para:AA/model3d:1` |
| 框内段落 | `{框 id}/para:{paraId}` 或 `{框 id}/para:index:{k}` | `para:3F2A1B00/txbx:8/para:00FF00AA` |
| 框内图 | `{框内段落 id}/pic:{docPrId}` | `para:3F2A1B00/txbx:8/para:00FF00AA/pic:1` |
| 框内图表 | `{框内段落 id}/chart:{docPrId}` | `para:3F2A1B00/txbx:8/para:00FF00AA/chart:1` |
| 框内 SmartArt | `{框内段落 id}/smartdiagram:{docPrId}` | `para:3F2A1B00/txbx:8/para:00FF00AA/smartdiagram:1` |
| 框内表 | `{框 id}/table:{n}` | `para:3F2A1B00/txbx:8/table:1` |
| 框内表行 | `{框 id}/table:{n}/row:{r}` | `para:3F2A1B00/txbx:8/table:1/row:1` |
| 框内单元格 | `{框 id}/table:{n}/r{r}c{c}` | `para:3F2A1B00/txbx:8/table:1/r1c1` |
| 框内超链接 | `{框 id}/link:{n}` | `para:3F2A1B00/txbx:8/link:1` |
| 框内书签 | `{框 id}/bookmark:{n}` | `para:3F2A1B00/txbx:8/bookmark:1` |
| 框内域 | `{框 id}/field:{n}` | `para:3F2A1B00/txbx:8/field:1` |
| 框内公式 | `{框 id}/math:{n}` | `para:3F2A1B00/txbx:8/math:1` |
| 框内拼音 | `{框 id}/ruby:{n}` | `para:3F2A1B00/txbx:8/ruby:1` |
| 框内内容控件 | `{框 id}/sdt:{n}` | `para:3F2A1B00/txbx:8/sdt:1` |
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
| 页眉水印 | `{页眉容器 id}/watermark:{docPrId}` 或 `{页眉容器 id}/watermark:index:{k}` | `header:default:1/watermark:1` |
| 正文图表 | `{段落 id}/chart:{docPrId}` 或 `{段落 id}/chart:index:{k}` | `para:3F2A1B00/chart:7` |
| 页眉/页脚图表 | `{页眉或页脚段落 id}/chart:{docPrId}` | `header:default:1/para:AA/chart:7` |
| 图表系列 | `{图表 id}/series:{idx}` | `para:3F2A1B00/chart:7/series:0` |
| 趋势线 | `{系列 id}/trendline:{k}` | `para:3F2A1B00/chart:7/series:0/trendline:1` |
| 误差线 | `{系列 id}/errorBars:{k}` | `para:3F2A1B00/chart:7/series:0/errorBars:1` |
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
| 正文例外编辑区 | `perm:{n}` | `perm:1` |
| 页眉/页脚例外编辑区 | `{容器 id}/perm:{n}` | `header:default:1/perm:1` |
| 正文域 | `field:{n}` | `field:1` |
| 页眉/页脚域 | `{容器 id}/field:{n}` | `header:default:1/field:1` |
| 正文目录 | `toc:{n}` | `toc:1` |
| 正文题注 | `caption:{n}` | `caption:1` |
| 正文交叉引用 | `xref:{n}` | `xref:1` |
| 正文公式 | `math:{n}` | `math:1` |
| 页眉/页脚公式 | `{容器 id}/math:{n}` | `header:default:1/math:1` |
| 正文拼音 | `ruby:{n}` | `ruby:1` |
| 页眉/页脚拼音 | `{容器 id}/ruby:{n}` | `header:default:1/ruby:1` |
| 正文内容控件 | `sdt:{n}` | `sdt:1` |
| 页眉/页脚内容控件 | `{容器 id}/sdt:{n}` | `header:default:1/sdt:1` |
| 编号实例 | `num:{numId}` | `num:1` |
| 抽象编号 | `abstractNum:{abstractNumId}` | `abstractNum:0` |
| 样式 | `style:{styleId}` | `style:Normal` |
| 构建基块 | `docPart:{n}` | `docPart:1` |
| 嵌入字体 | `font:{n}` | `font:1` |
| 段落内 run | `{段落 id}/run:{n}` | `para:3F2A1B00/run:1`、`header:default:1/para:AA/run:2` |
| 自定义属性 | `prop:{name}` | `prop:Editor` |

节号 `{n}` 从 1 起，按主文档里 `w:sectPr` 的文档顺序（与页眉目录相同）。
`insertSection` / `deleteSection` 之后旧的 `section:{n}` 会变，只够同一次 inspect 之后立刻 apply。距边界挂在节上，不挂在页眉部件节点上。页眉页脚图挂在容器上，不挂在节上。
正文图挂在它所在的主文档段落上，不挂在文档根上。对 `section:{n}` 发 `setParaText` 或图片 op 报 `InvalidId`。
`insertSection` 的 `id` 必须是主文档 `w:body` 的直接子段落 id，或 `docx:document`。`deleteSection` 的 `id` 必须是 `section:{n}`。
对页眉页脚、框内、注内、批注内、表格单元格内的段落 id，或容器 id、表 id、run id 发 `insertSection` 报 `InvalidId`。

`{type}` 取引用上 `w:type` 的原文，只允许 `default`、`even`、`first`（`ST_HdrFtr`，§17.18.36）。缺失或不是这三项报 `XmlBroken`。
`{n}` 在同一 `kind`（header 或 footer）与 `{type}` 下，按该部件首次出现的顺序从 1 起。
页眉页脚段落没有 `w14:paraId` 时用 `para:index:{k}`，`k` 只在该部件内从 1 计。

正文段落 id 仍是 `para:{paraId}` / `para:index:{n}`，只表示主文档部件。对容器 id（没有 `/para:`）发 `setParaText` 报 `InvalidId`。
拿正文 `para:` 去改页眉页脚、注或批注里同 hex 的段落报 `IdNotFound`。

脚注、尾注的 `{id}` 取 `w:footnote/@w:id` / `w:endnote/@w:id` 的原文（ECMA-376 §17.11.2）。输出用文件里的原始数字写法。
请求里若双方都能解析成十进制整数，按数值匹配（`2` 与 `02` 视为同一条）；否则按原文精确比。同一文件里脚注 id 与尾注 id 可以都是 `2`，靠前缀区分。
只把 `w:type` 为 `normal` 或未写 type 的当作用户注（§17.18.33 `ST_FtnEdn`）。
`separator`、`continuationSeparator`、`continuationNotice` 是分隔符，id 写成 `footnote:sep:{type}` / `endnote:sep:{type}`，
只出现在 `noteSeparators`，对它们发任何 apply 报 `InvalidId`。
同一侧重复 `w:id` 时 inspect 都列出，apply 命中该部件文档顺序第一处。

批注的 `{id}` 取 `w:comment/@w:id` 的原文（ECMA-376 §17.13.4.2）。匹配规则与脚注 `w:id` 相同。同一文件里批注 id 与脚注 id 可以都是 `0`，靠前缀区分。
重复 `w:id` 时 inspect 都列出，apply 命中 `comments.xml` 文档顺序第一处。
范围标记 `w:commentRangeStart` / `w:commentRangeEnd` / `w:commentReference` 的 `w:id` 与 `w:comment/@w:id` 按数值对应（§17.13.4.4、§17.13.4.3、§17.13.4.5）。
线程关系靠 `commentsExtended` 的 `paraIdParent` 挂到父批注（MS-DOCX §2.5.3.1）。
Word 16 给回复也写一套范围标记，套在父批注范围上：`commentRangeStart` 紧跟父 start 之后，
`commentRangeEnd` 与 `commentReference` 紧跟父 `commentReference` 所在 run 之后。
inspect 读到回复自己的标记就用；没有标记时沿用线程根。`commentEx/@paraId` 是该批注**最后一段**的 `w14:paraId`，不是第一段，也不是宿主段落。

图表的 `{docPrId}` 取该 `w:drawing` 里 `wp:docPr/@id` 的原文，匹配规则与图相同。没有该属性时用 `chart:index:{k}`，`stable` 为 `false`。
正文的 `{k}` 只在该段内从 1 计；页眉页脚的 `{k}` 只在该部件内从 1 计。系列 `{idx}` 从 0 起，按 chart 部件里 `c:ser` 的文档顺序，不跟 `c:idx/@val`。
趋势线 id 为 `{系列 id}/trendline:{k}`，误差线 id 为 `{系列 id}/errorBars:{k}`，`{k}` 从 1 起。
只认 `a:graphicData/@uri` 为 `http://schemas.openxmlformats.org/drawingml/2006/chart`（可带尾斜杠）的 drawing（ECMA-376 §21.2）。
Office 2016 以后的 chartex URI（`http://schemas.microsoft.com/office/drawing/2014/chartex`）也列入，`type` 为 `chartex`。VML 图表不编。
`w:drawing` 里同时有 `pic:pic` 和图表的，各走各的名单。正文图表只收主文档段落的子孙，遇 `w:txbxContent` 停住；表格单元格里的段落与正文段落同一套规则。
页眉页脚图表挂在它所在的页眉或页脚段落上，名单出现在该容器的 `charts` 上。框内图表挂在框节点上，id 带框内段落前缀。注内、批注内图表挂在该注或批注节点上，id 带注内或批注内段落前缀。
框内图表不算进注或批注自己的 `chartCount`。根上 `chartCount` 仍只计正文，不含页眉页脚，也不含框内、注内、批注内。

`insertChart` 的 `id` 必须是段落 id：正文、页眉页脚、框内、注内或批注内段落。
对图 id、图表 id、系列 id、SmartArt id、点 id、容器 id、框 id、注容器 id、批注容器 id 或 `section:{n}` 发 `insertChart` 报 `InvalidId`。
`deleteChart` / `setChartTitle` / `setChartType` / `setChartAxisTitle` / `setChartLegend` / `setChartDataLabels`
 / `setChartSize` / `setChartWrap` / `addChartSeries` 的 `id` 必须是图表 id。
`setChartSeries` / `deleteChartSeries` / `insertTrendline` / `insertErrorBars` / `setChartSeriesAxis` 的 `id` 必须是系列 id。
`setTrendline` / `deleteTrendline` 的 `id` 必须是趋势线 id。`setErrorBars` / `deleteErrorBars` 的 `id` 必须是误差线 id。对不上的 id 形态报 `InvalidId`。

SmartArt 的 `{docPrId}` 取该 `w:drawing` 里 `wp:docPr/@id` 的原文，匹配规则与图相同。没有该属性时用 `smartdiagram:index:{k}`，`stable` 为 `false`。
正文的 `{k}` 只在该段内从 1 计；页眉页脚的 `{k}` 只在该部件内从 1 计。点的 `{modelId}` 取 data 部件里 `dgm:pt/@modelId` 的原文，输出用文件里的原始大小写。
请求按精确字符串匹配（GUID 的花括号和连字符都按原文）。同一 data 部件里重复 `modelId` 时 inspect 都列出，apply 命中 `ptLst` 文档顺序第一处。
只认 `a:graphicData/@uri` 为 `http://schemas.openxmlformats.org/drawingml/2006/diagram`（可带尾斜杠）的 drawing（ECMA-376 §21.4）。
`dgm:relIds` 的 `r:dm` / `r:lo` / `r:qs` / `r:cs` 指向**宿主部件**关系里的 data / layout / quickStyle / colors（§21.4.2.22）。
VML 图示不编。`w:drawing` 里同时有 `pic:pic`、图表和 SmartArt 的，各走各的名单。
正文 SmartArt 只收主文档段落的子孙，遇 `w:txbxContent` 停住；表格单元格里的段落与正文段落同一套规则。页眉页脚 SmartArt 挂在它所在的页眉或页脚段落上，名单出现在该容器的 `smartArts` 上。
框内 SmartArt 挂在框节点上，id 带框内段落前缀。注内、批注内 SmartArt 挂在该注或批注节点上，id 带注内或批注内段落前缀。框内图示不算进注或批注自己的 `smartArtCount`。
根上 `smartArtCount` 仍只计正文，不含页眉页脚，也不含框内、注内、批注内。

可见点：`dgm:pt` 的 `type` 缺省（规范默认 `node`，§21.4.7.51）或为 `node` / `asst` 的点进 `points`。`doc`、`pres`、`parTrans`、`sibTrans` 不进树。
没有 `dgm:t` 时 `text` 为 `""`。层次看 `cxnLst` 里 `type` 缺省或为 `parOf` 的连接：`srcId` 是父、`destId` 是子。
父点是 `doc` 或不在可见名单里时 `parent` 为 `null`；否则 `parent` 是父点的点 id。

`insertSmartArt` 的 `id` 必须是段落 id：正文、页眉页脚、框内、注内或批注内段落。
对图 id、图表 id、SmartArt id、点 id、容器 id、框 id、注容器 id、批注容器 id、表 id、行 id、
单元格 id 或 `section:{n}` 发 `insertSmartArt` 报 `InvalidId`。
`deleteSmartArt` / `setSmartArtSize` / `setSmartArtWrap` / `setSmartArtColors` / `setSmartArtStyle` / `setSmartArtLayout` 的 `id` 必须是图示 id。
`setSmartArtText` / `deleteSmartArtPoint` 的 `id` 必须是点 id。`addSmartArtPoint` 的 `id` 是图示 id（挂到文档根点下）或点 id（挂到该点下）。对不上的 id 形态报 `InvalidId`。

OLE 的 `{token}` 优先取 `o:OLEObject/@ObjectID` 原文（常带前导 `_`），否则 `w:objectEmbed/@shapeId`，否则该 `w:object` 里 `wp:docPr/@id`。
输出用文件里的原始大小写。请求里若双方都能解析成十进制整数，按数值匹配；否则按原文精确比。同一归属范围内重复 token 时 inspect 都列出，apply 命中该范围内文档顺序第一处。正文的范围是该段；页眉页脚的范围是该部件。
都没有上述属性时用 `ole:index:{k}`，树上 `"stable": false`。
正文的 `{k}` 只在该段内从 1 计；页眉页脚的 `{k}` 只在该部件内从 1 计，计入该范围内所有被收入的 OLE，不只是缺 token 的那些。
只认 `w:object`（ECMA-376 §17.3.3.19）且带嵌入关系：子孙里有 `o:OLEObject`（Office 命名空间 `urn:schemas-microsoft-com:office:office`）且 `Type` 不是 `Link`，
或有 `w:objectEmbed`。
关系 Type 后缀为 `oleObject` 或 `package`（Transitional
`http://schemas.openxmlformats.org/officeDocument/2006/relationships/oleObject` 与 `…/package`）。
Word 16.0 把嵌入的 xlsx 等 Office 文件写成 `package`；ECMA 与部分文件写成 `oleObject`。两者都进树。
`objectLink`、`Type="Link"`、`w:control`、`w:movie` 不进树。
预览图（VML `v:imagedata` 或 `w:object` 里的 `w:drawing`）不进 `pictures`，也不进 `charts` / `smartArts`。
正文 OLE 只收主文档段落的子孙，遇 `w:txbxContent` 停住；表格单元格里的段落与正文段落同一套规则。页眉页脚 OLE 挂在它所在的页眉或页脚段落上，名单出现在该容器的 `oleObjects` 上。
框内、注内、批注内的 `w:object` 本期不编；对这些容器发 OLE op 报 `InvalidId`。根上 `oleObjectCount` 仍只计正文，不含页眉页脚。xlsx / pptx 本期不编 OLE。

`insertOle` 的 `id` 必须是段落 id：正文或页眉页脚。对 OLE id、图 id、容器 id、框 id、注容器 id、批注容器 id、
框内 / 注内 / 批注内段落 id 或 `section:{n}` 发 `insertOle` 报 `InvalidId`。
`deleteOle` 的 `id` 必须是 OLE id。对段落 id 发 `deleteOle` 报 `InvalidId`。

3D 模型的 `{docPrId}` 取该 `w:drawing` 的 `wp:docPr/@id`。没有则用 `model3d:index:{k}`，树上 `"stable": false`。
`k` 只在该宿主段内从 1 计。页眉页脚 3D 挂在它所在的页眉或页脚段落上，名单出现在该容器的 `model3ds` 上。
只认 `mc:AlternateContent` 且 `Choice Requires` 含 `am3d`，Choice 里是 `w:drawing` / `wp:inline` 包 `am3d:model3d`，不是 `pic:pic`。
只走 `Choice Requires="am3d"`。Fallback 海报不进 `pictures`。正文 3D 只收主文档段落的子孙，遇 `w:txbxContent` 停住。
组合里符合条件的要展开，组合本身不进树。注内、批注内本期不编；对这些容器发 `insertModel3d` 报 `InvalidField`。
`insertModel3d` 的 `id` 必须是正文或页眉页脚段落 id。`replaceModel3d` / `deleteModel3d` / `setModel3dSize` / `setModel3dWrap` / `setModel3dCamera` 的 `id` 必须是 3D 模型 id。
只认 GLB。不是 GLB 报 `InvalidImageFile`（对外 `InvalidField`）。`contentType` 恒为 `model/gltf.binary`。
`camera` 始终是对象，字段顺序：`pos`、`up`、`lookAt`、`fov`。
`pos` / `lookAt` 字段顺序 `x`、`y`、`z`，取 `am3d:camera` 下对应元素属性，能解析成整数则写整数，否则 `null`。
`up` 字段顺序 `dx`、`dy`、`dz`，规则相同。
`fov` 取 `am3d:perspective/@fov` 非负整数，缺或解析失败为 `null`。
`rot` 始终是对象，字段顺序：`ax`、`ay`、`az`。取 `am3d:rot` 对应属性；缺属性或没有该元素则为 `null`。
`lightRig` 取 `am3d:spPr/a:scene3d/a:lightRig`。没有则为 `null`。有则字段顺序：`rig`、`dir`、`lat`、`lon`、`rev`。
`rig` / `dir` 取属性原文，缺则为 `null`。`lat` / `lon` / `rev` 取子元 `a:rot` 对应属性整数，缺则为 `null`。
`insertModel3d` 写入只做 `inline`。已有 3D 可用 `setModel3dWrap` 改成浮动。xlsx / pptx 见各自分册。

水印只认页眉部件（`w:hdr`）里的 DrawingML `w:drawing`。
`wp:docPr/@name` 以 `ooxpWatermark`、`PowerPlusWaterMarkObject` 或 `WordPictureWatermark` 开头，
或者子孙 `a:ext/@uri` 为 `{C0A5BD15-81BF-4B96-A0C4-0C0A5BD15001}`，即视为水印。
页脚、正文、框内、注内、批注内不算。`w:pict` 不算。被判定为水印的 drawing 不进 `pictures` / `textBoxes` / `charts` / `smartArts`。
没有 `wp:docPr/@id` 时用 `watermark:index:{k}`，`{k}` 只在该页眉部件内从 1 计，树上 `"stable": false`。
Word 16.0 自己插入的水印通常是 VML；另存若把 DrawingML 改成 VML，inspect 可以不认，`deleteWatermark` 也删不掉。不要为了兼容去改 VML。

`insertWatermark` 的 `id` 必须是 `docx:document`。`deleteWatermark` 的 `id` 必须是水印 id 或 `docx:document`（删全部 DrawingML 水印）。
对段落 id、图 id、框 id 或节 id 发这两个 op 报 `InvalidId`。

正文与页眉页脚表的 `{n}` 从 1 起，按该故事里 `w:tbl` 的文档顺序（遇 `w:txbxContent` 停住；`mc:AlternateContent` 只进 Choice）。嵌套在单元格里的表也编号，不另起前缀。
OOXML 没有表级稳定 id，因此 `table:{n}` 在增删表之后会变，只够同一次 inspect 之后立刻 apply。行号 `{r}` 是该表内 `w:tr` 的文档顺序，从 1 起。
列号 `{c}` 是该 `w:tc` 在 `w:tblGrid` 上的起始列，从 1 起；`w:gridSpan` 缺省为 1（§17.4.17）。水平合并只出一个单元格节点，`gridSpan` 大于 1。
垂直合并的继续格仍有 `w:tc`，树上 `vMerge` 为 `continue`。没有 `tblGrid` 时，`colCount` 取第一行各格 `gridSpan` 之和。

文本框里的表另编号：`{n}` 是该框 `w:txbxContent` 里 `w:tbl` 的文档顺序（再遇嵌套 `w:txbxContent` 停住；`mc:AlternateContent` 只进 Choice），id 为 `{框 id}/table:{n}`。
嵌套在框内表格单元格里的表也编号，不另起前缀。单元格段落仍一次倒出，规则与正文表相同，段落 id 带框前缀。
注内、批注内的 `w:tbl` 仍不编、不改。框内再套框保持 🚫。

`insertTable` 的 `id` 可以是：

- 正文、页眉页脚或框内段落 id：插到该 `w:p` 之后，与该段同父
- `docx:document` 或页眉页脚容器 id：插到该故事末尾、`sectPr` 之前
- 框 id：插到该框 `w:txbxContent` 末尾
- 已有表 id（含框内表）：插到该 `w:tbl` 之后
- 单元格 id（含框内单元格）：作为嵌套表插到该格 `tcPr` 之后、已有段落之前；格末仍保留至少一个 `w:p`

对注内、批注内段落或注内 / 批注内框发 `insertTable` 报 `InvalidId`。
`deleteTable` 的 `id` 必须是表 id。`insertRow` 的 `id` 是表 id（追加一行）或行 id（插到该行之后）。`deleteRow` 的 `id` 必须是行 id。
`insertCol` / `deleteCol` / `mergeCells` 的 `id` 必须是表 id。`splitCells` 的 `id` 必须是单元格 id。
`setTableStyle` / `setTableFloat` / `setTableWidth` / `setTableAlign` / `setTableCellMar` / `setTableBorders` 的 `id` 必须是表 id。`setTableRowHeight` 的 `id` 必须是行 id。`setCellFormat` / `setCellBorders` 的 `id` 必须是单元格 id。
正文、页眉页脚与框内的表可以改结构与外观。对注内、批注内的表发这些 op 报 `InvalidId`。对不上的 id 形态报 `InvalidId`。
框内表带修订痕迹时结构 op 仍报 `NotImplemented`。

修订的 `{n}` 从 1 起，按该故事里被收入节点的文档顺序（遇 `w:txbxContent` 停住；`mc:AlternateContent` 只进 Choice）。
OOXML 的 `w:id` 在拆 run 时会重复，因此不用它当树上的唯一键；树上另给 `xmlId` 存原文。`rev:{n}` 在增删修订之后会变，只够同一次 inspect 之后立刻 apply。
同一 `w:id` 的多个节点是多条修订，`acceptRevision` / `rejectRevision` 按 id 点名；
run 级与 `*Change` 只动被点名的那一个 XML 节点，`rowIns` / `rowDel` 以及会删掉整段的段落标记修订按下面 apply 节处理。
本期收入：不在 `pPr` / `rPr` / `trPr` / `tblPr` / `tcPr` / `tblPrEx` / `sectPr` / `numPr` 里、
且直接子元素不是 `w:p` / `w:tbl` 的 `w:ins` / `w:del`（run 级，
`kind` 为 `ins` / `del`，ECMA-376 §17.13.5.18、§17.13.5.14）；
`w:pPr`（含其中的 `w:rPr`）里、且不在 `w:r` 内的空 `ins`/`del`（段落标记，`kind` 为 `paraMarkIns` / `paraMarkDel`）；
`w:trPr` 里的空 `ins`/`del`（表行，`kind` 为 `rowIns` / `rowDel`）；
以及 `w:rPrChange`（§17.13.5.31）、`w:pPrChange`（§17.13.5.27）、`w:tblPrChange`（§17.13.5.34）、`w:tblGridChange`（§17.13.5.33）、
`w:tblPrExChange`（§17.13.5.35）、`w:tcPrChange`（§17.13.5.36）、`w:trPrChange`（§17.13.5.37）、`w:sectPrChange`（§17.13.5.32）。
祖先 `w:tr` 的 `trPr` 已有 `ins`/`del` 时，该行格子里的段落标记 `ins`/`del` 不另列。
本期还收入不在属性容器里、直接子元素不是 `w:p` / `w:tbl` 的 `w:moveFrom` / `w:moveTo`（`kind` 为 `moveFrom` / `moveTo`）。
`moveFromRangeStart` / `moveFromRangeEnd` / `moveToRangeStart` / `moveToRangeEnd` 不另列，接受 / 拒绝对应包裹节点时删掉成对标记。
不进树：直接子元素是 `w:p` / `w:tbl` 的块级 `ins`/`del`、其它属性容器上的 `ins`/`del`。
框内、注内、批注内的修订本期不编；对这些容器发 `acceptRevision` / `rejectRevision` 报 `InvalidId`。`w:id` 缺失时仍列出，`xmlId` 为 `null`。`tblGridChange` 可以没有 `author` / `date`。

`acceptRevision` / `rejectRevision` 的 `id` 必须是修订 id。对段落 id、表 id、容器 id 或 `section:{n}` 发这两个 op 报 `InvalidId`。

超链接有两套，不要混：

- `w:hyperlink`：挂在故事上，不挂在段落节点上。`{n}` 从 1 起，按该故事里 `w:hyperlink` 的文档顺序（遇 `w:txbxContent` 停住；`mc:AlternateContent` 只进 Choice）。
  关系 `r:id` 不当树上的唯一键，只进 `rId`。`link:{n}` 在增删链之后会变，只够同一次 inspect 之后立刻 apply。
  本期只收 `w:hyperlink` 元素（ECMA-376 §17.16.22）。
  HYPERLINK 域不进本名单，仍走域切片。
  文本框里的 `w:hyperlink` 另编号，id 为 `{框 id}/link:{n}`。`{n}` 是该框 `w:txbxContent` 里链的文档顺序。
  根上 `hyperlinkCount` 仍只计正文。框节点另报 `hyperlinkCount`。`scope.hyperlinks` 为 true 时框上带 `hyperlinks`。
  注内 / 批注内的 `w:hyperlink` 仍不进树；对注内 / 批注内段落或注内 / 批注内框发 `w:hyperlink` 超链接 op 报 `InvalidId`。
  有 `r:id` 时为外部链（规范说此时忽略 `w:anchor`）；只有 `w:anchor` 时为内部链，指向书签名。两者都没有仍列出，`kind` 为 `anchor`，`anchor` 为 `null`。
  根上 `hyperlinkCount` 与页眉页脚容器上的 `hyperlinkCount` 只计这一套。
- DrawingML 链：写在图、文本框、连接线、无框形状、SmartArt、图表、墨迹画布、3D 上的 `a:hlinkClick`。
  宿主可以是正文、页眉页脚、框内图、注内 / 批注内的图或框。注内 / 批注内不列出的墨迹与 3D 仍不能当链宿主。
  图读 `wp:docPr/a:hlinkClick`；没有则读该 `pic` 的 `cNvPr/a:hlinkClick`（组合子图）。
  文本框、连接线、无框形状读 `wps:cNvPr/a:hlinkClick`；没有则读外层 `wp:docPr/a:hlinkClick`。
  SmartArt、图表、墨迹画布、3D 读该 drawing 的 `wp:docPr/a:hlinkClick`；没有则读该对象的 `cNvPr`。
  外部链关系写在该**故事部件** `.rels`（`document.xml.rels` / `headerN.xml.rels` / 注或批注部件 `.rels`），Type 以 `/hyperlink` 结尾，`TargetMode` 为 `External`，`Target` 为 URL 原文。
  内部书签：无 `r:id` 时 `kind` 为 `anchor`；有 `r:id` 且关系 Target 以 `#` 开头时 `kind` 为 `anchor`，`anchor` 为去掉开头 `#` 的书签名。
  apply 写内部书签时在宿主 `.rels` 写关系，`Target` 为 `#{name}`，不写 `TargetMode`。
  `tooltip` 取 `a:hlinkClick/@tooltip`。不写 `hlinkHover`。
  inspect 时出现在该对象的 `hyperlink` 字段（对象；没有链则为 `null`），不进 `hyperlinks` 名单，也不计入 `hyperlinkCount`。
  `hyperlink` 对象字段顺序：`kind`、`rId`、`url`、`anchor`、`tooltip`。
  `kind` 为 `url`（有 `r:id` 且 Target 不以 `#` 开头）或 `anchor`。
  对 OLE id 发超链接 op 报 `InvalidId`。
  create 给了 `pictures[]` / `textBoxes[]` / `shapes[]` / `connectors[]` / `smartArts[]` / `charts[]` / `inkCanvases[]` / `model3ds[]` 的 `hyperlink` 时按同一套规则写入。
  xlsx / pptx 行为不变。docx 上出现 `location` / `ref` / `on` / `event` 报未知字段或 `InvalidField`（内部用 `anchor`）。

书签的 `{n}` 从 1 起，按该故事里被收入的 `w:bookmarkStart` 的文档顺序（同一套边界）。只收起止都在同一故事里、且 `w:name` 不以 `_` 开头的配对（Word 隐藏书签含 `_GoBack` 不进树）。
配对按 `w:id`（§17.13.6.1、§17.13.6.2）。找不到配对的不进树。`w:id` 进 `xmlId`，缺则为 `null`。`w:name` 进 `name`。
文本框里的书签另编号，id 为 `{框 id}/bookmark:{n}`。起止必须同在该框。
根上 `bookmarkCount` 仍只计正文。框节点另报 `bookmarkCount`。`scope.bookmarks` 为 true 时框上带 `bookmarks`。
注内、批注内的书签仍不编；对这些容器发书签 op 报 `InvalidId`。

例外编辑区的 `{n}` 从 1 起，按该故事里被收入的 `w:permStart` 的文档顺序（同一套边界）。配对按 `w:id`。不成对的 Start 仍列出。
框内、注内、批注内的 `permStart` 本期不编；对这些容器发例外区 op 报 `InvalidId`。

域的 `{n}` 从 1 起，按该故事里被收入域起点的文档顺序（遇 `w:txbxContent` 停住；`mc:AlternateContent` 只进 Choice）。
起点是 `w:fldSimple`，或配对完整的 `w:fldChar/@w:fldCharType="begin"`（§17.16.18、§17.16.19）。
嵌套域各算一条：外层 begin 先于内层 begin，因此外层 `{n}` 更小。故事结束时尚未配对 end 的 begin 不进树。
文本框里的域另编号，id 为 `{框 id}/field:{n}`。根上 `fieldCount` 仍只计正文。框节点另报 `fieldCount`。
`scope.fields` 为 true 时框上带 `fields`。注内、批注内的域仍不编；对这些容器发域 op 报 `InvalidId`。
HYPERLINK 域进本名单，不进 `hyperlinks`。主文档里 `code` 为 `TOC` 或 `TOA` 的域另外编目录 id，它们仍出现在 `fields` 里。
主文档里顶层 `SEQ` 另外编题注 id，顶层 `REF` / `PAGEREF` / `NOTEREF` 另外编交叉引用 id，顶层 `INDEX` 另外编索引 id，顶层 `XE` 另外编索引项 id，它们仍出现在 `fields` 里。

目录的 `{n}` 从 1 起，按主文档里被收入的 TOC / TOA 域起点的文档顺序，与域名单同一套边界。`toc:{n}` 在插删目录之后会变，只够同一次 inspect 之后立刻 apply。
`kind` 为 `toc`（`TOC` 且指令里没有 `\c` 开关）、`tof`（`TOC` 且有 `\c`，图表目录）或 `toa`（`TOA`）。
页眉页脚、框内、注内、批注内的 TOC 本期不编 `toc:`；对这些容器发 `insertToc` / `deleteToc` 报 `InvalidId`。
嵌套在 TOC 结果里的 `PAGEREF` / `HYPERLINK` 域不算目录块。

题注的 `{n}` 从 1 起，按主文档里被收入的顶层 `SEQ` 域起点的文档顺序，与域名单同一套边界。嵌套在其它域（含 TOC 结果）里的 `SEQ` 不编 `caption:`。
`caption:{n}` 在插删题注之后会变，只够同一次 inspect 之后立刻 apply。
页眉页脚、框内、注内、批注内的 SEQ 本期不编 `caption:`；对这些容器发 `insertCaption` / `deleteCaption` 报 `InvalidId`。
Word 给题注写的 `_Ref*` 书签仍不进 `bookmarks`（隐藏书签规则不变），题注节点上的 `bookmark` 给出包住该 SEQ 的书签名。

交叉引用的 `{n}` 从 1 起，按主文档里被收入的顶层 `REF` / `PAGEREF` / `NOTEREF` 域起点的文档顺序。嵌套在其它域里的（含 TOC 结果里的 `PAGEREF`）不编 `xref:`。
`xref:{n}` 在插删交叉引用之后会变，只够同一次 inspect 之后立刻 apply。`kind` 为 `ref`（`REF`）、`page`（`PAGEREF`）或 `note`（`NOTEREF`）。
页眉页脚里的交叉引用本期不编 `xref:`，仍出现在该容器 `fields` 里；对页眉页脚段落发 `insertCrossRef` 可以写入，但树上的 `xref:` 只计正文。
框内、注内、批注内本期不编；对这些容器发 `insertCrossRef` / `deleteCrossRef` 报 `InvalidId`。

索引的 `{n}` 从 1 起，按主文档里被收入的顶层 `INDEX` 域起点的文档顺序，与域名单同一套边界。嵌套在其它域结果里的 `INDEX` 不编 `index:`。
`index:{n}` 在插删索引之后会变，只够同一次 inspect 之后立刻 apply。
页眉页脚、框内、注内、批注内的 INDEX 本期不编 `index:`；对这些容器发 `insertIndex` / `deleteIndex` 报 `InvalidId`。

索引项的 `{n}` 从 1 起，按主文档里被收入的顶层 `XE` 域起点的文档顺序。嵌套在其它域里的不编 `xe:`。`xe:{n}` 在插删索引项之后会变，只够同一次 inspect 之后立刻 apply。
页眉页脚里的 XE 本期不编 `xe:`，仍出现在该容器 `fields` 里；对页眉页脚段落发 `insertIndexEntry` 可以写入，但树上的 `xe:` 只计正文。
框内、注内、批注内本期不编；对这些容器发 `insertIndexEntry` / `deleteIndexEntry` 报 `InvalidId`。

书目源的 `{n}` 从 1 起，按 bibliography customXml 里 `b:Source` 的文档顺序。没有该部件则为 0。`source:{n}` 在插删源之后会变。
`insertSource` / `deleteSource` 的 id 规则见 apply。

引文的 `{n}` 从 1 起，按主文档里被收入的顶层 `CITATION` 域起点的文档顺序。嵌套在其它域结果里的不编 `cite:`。
页眉页脚里的 CITATION 本期不编 `cite:`，仍出现在该容器 `fields` 里；对页眉页脚段落发 `insertCitation` 可以写入。框内、注内、批注内本期不编。

书目块的 `{n}` 从 1 起，按主文档里被收入的顶层 `BIBLIOGRAPHY` 域起点的文档顺序。嵌套在其它域结果里的不编 `biblio:`。
页眉页脚、框内、注内、批注内本期不编 `biblio:`；对这些容器发 `insertBibliography` / `deleteBibliography` 报 `InvalidId`。

构建基块的 `{n}` 从 1 起，按 glossary 部件里 `w:docPart` 的文档顺序。没有 glossary 部件则为 0。`docPart:{n}` 在插删基块之后会变。
`insertDocPart` / `deleteDocPart` / `insertDocPartContent` 的 id 规则见 apply。
主文档里带 `w:docPartObj` 的 `w:sdt` 仍不进内容控件名单（待做 43）。不实现 Word 构建基块库 UI，不改用户 `Normal.dotm`。

嵌入字体的 `{n}` 从 1 起，按 `fontTable` 里所有 `w:embedRegular` / `w:embedBold` / `w:embedItalic` / `w:embedBoldItalic` 的文档顺序（每个嵌入面一条，不是每个 `w:font`）。
没有 fontTable 或没有任何 embed* 则为 0。`font:{n}` 在插删嵌入面之后会变。
`insertEmbeddedFont` / `deleteEmbeddedFont` / `setFontEmbed` 的 id 规则见 apply。不把 odttf 字节塞进 inspect 树。
不实现字体渲染、PANOSE 替换或 TTF 子集化引擎。create 给了 `document.embeddedFonts` 时按 `insertEmbeddedFont` 写入。不改用户本机字体库。

公式的 `{n}` 从 1 起，按该故事里顶层 `m:oMath` 的文档顺序（遇 `w:txbxContent` 停住；`mc:AlternateContent` 只进 Choice）。嵌在另一个 `oMath` 里的不算一条。
`oMathPara` 里的每个 `oMath` 各算一条。`math:{n}` 在插删公式之后会变，只够同一次 inspect 之后立刻 apply。
父节点是 `oMathPara` 时 `kind` 为 `display`，否则为 `inline`。正文用 `math:{n}`，页眉页脚用 `{容器}/math:{n}`。
文本框用 `{框 id}/math:{n}`。根上 `mathCount` 仍只计正文。框节点另报 `mathCount`。`scope.maths` 为 true 时框上带 `maths`。
注内、批注内的公式仍不编；对这些容器发公式 op 报 `InvalidId`。Equation Editor 3.0 的 OLE / `w:object` 不编。只认 OMML。名单带 `expr` 结构树，不另编嵌套对象 id。
不解析 UnicodeMath / TeX，也不实现排版引擎。

拼音的 `{n}` 从 1 起，按该故事里 `w:ruby` 的文档顺序（遇 `w:txbxContent` 停住；`mc:AlternateContent` 只进 Choice；`w:del` 里的不编）。
`ruby:{n}` 在插删拼音之后会变，只够同一次 inspect 之后立刻 apply。正文用 `ruby:{n}`，页眉页脚用 `{容器}/ruby:{n}`。
文本框用 `{框 id}/ruby:{n}`。根上 `rubyCount` 仍只计正文。框节点另报 `rubyCount`。`scope.rubies` 为 true 时框上带 `rubies`。
注内、批注内的拼音仍不编；对这些容器发拼音 op 报 `InvalidId`。不实现注音排版引擎。

内容控件的 `{n}` 从 1 起，按该故事里被收入的 `w:sdt` 的文档顺序（遇 `w:txbxContent` 停住；`mc:AlternateContent` 只进 Choice，ECMA-376 §17.5.2）。
嵌套控件各算一条，外层 `{n}` 更小。`sdt:{n}` 在插删控件之后会变，只够同一次 inspect 之后立刻 apply。正文用 `sdt:{n}`，页眉页脚用 `{容器}/sdt:{n}`。
文本框用 `{框 id}/sdt:{n}`。根上 `contentControlCount` 仍只计正文。框节点另报 `contentControlCount`。
`scope.contentControls` 为 true 时框上带 `contentControls`。注内、批注内的控件仍不编；对这些容器发内容控件 op 报 `InvalidId`。
本期收入：`plainText`（`w:text`）、`richText`（`w:richText`，或 `sdtPr` 未写类型元素，
规范缺省为富文本）、`dropDown`（`w:dropDownList`）、`comboBox`（`w:comboBox`）、`date`（`w:date`）、
`repeatingSection`（`repeatingSection` / `repeatingSectionItem`）、`checkbox`（`w14:checkbox`，
MS-DOCX `CT_SdtCheckbox`）、`picture`（`w:picture`）、`group`（`w:group`）、`equation`（`w:equation`）。
不收 `docPartObj` / `docPartList`（目录等构建基块库仍走 `toc:` / `docPart:`）、`bibliography`、`citation`。
祖先里有 `w:p` 时 `level` 为 `inline`，否则为 `block`。控件内段落仍走段落 id。`w:sdtPr/w:id/@w:val` 进 `xmlId`，不是树上的唯一键。
带 `w:dataBinding` 的仍列出，`bound` 为 `true`。不实现 Word 开发者模式 UI，也不写自定义 XML 部件。

编号定义在 `/word/numbering.xml`（关系 `/numbering`，ECMA-376 §17.9）。实例 id 用 `w:num/@w:numId`：`num:{numId}`。
抽象定义 id 用 `w:abstractNum/@w:abstractNumId`：`abstractNum:{abstractNumId}`。`numId` 为 0 表示无编号，不进树。
这与 `table:{n}` 不同：编号 XML 本身有稳定数字 id，增删定义后已有实例的 id 不变。不实现编号计算引擎，树上不出现「显示为 3.」这类结果。页眉页脚容器不挂编号计数（整篇共用一份定义）。
框内、注内、批注内的段若自带 `w:numPr` 仍能看见 `num` / `ilvl`，但编号 op 只接受正文或页眉页脚段落。

样式定义在 `/word/styles.xml`（关系 `/styles`，ECMA-376 §17.7）。实例 id 用 `w:style/@w:styleId`：`style:{styleId}`。`styleId` 为空的不进树。
这与 `table:{n}` 不同：样式 XML 本身有稳定字符串 id。不实现排版引擎，树上不出现算出来的字号或颜色。页眉页脚容器不挂样式计数（整篇共用一份定义）。
段落上的 `style` 仍只读自己的 `w:pStyle/@w:val`，不解析 `basedOn`，也不把样式定义摊到段上。
run 上的 `style` 只读自己的 `w:rStyle/@w:val`。框内、注内、批注内的段若自带 `pStyle` 仍能看见 `style`。
`setParaStyle` 只接受正文或页眉页脚段落。`setRunStyle` 只接受正文或页眉页脚 run。
`insertStyle` / `deleteStyle` / `setStyle` 走 `styles.xml`，与段落 id 无关。

直接格式写在该段自己的 `w:pPr` 或该 run 自己的 `w:rPr` 上，不改 `styles.xml`。run 的 `{n}` 从 1 起，按该段内被收入的 `w:r` 文档顺序。
遇 `w:txbxContent` 停住；`mc:AlternateContent` 只进 Choice；`w:del` 里的 run 不编。`w:ins`、`w:hyperlink` 里的 run 要编。
增删 run 之后 `{n}` 会变，只够同一次 inspect 之后立刻 apply。对 run id 发 `setParaText` / `setParaFormat` 报 `InvalidId`。对段落 id 发 `setRunFormat` 报 `InvalidId`。

`insertHyperlink` 的 `id` 必须是段落 id（正文、页眉页脚或框内，写 `w:hyperlink`），或图 / 文本框 / 连接线 / 无框形状 / SmartArt / 图表 / 墨迹画布 / 3D 的 id（写 `a:hlinkClick`，含框内图、注内 / 批注内的图或框）。
对 DrawingML 对象禁止 `text`。`url` 与 `anchor` 必须恰好一个。同一对象已有 `hlinkClick` 报 `InvalidField`（改地址用 `setHyperlink`）。对 OLE id 报 `InvalidId`。
`setHyperlink` / `deleteHyperlink` 的 `id` 必须是超链接 id（`link:{n}`，含页眉页脚前缀与框前缀），或带（或要改）DrawingML 链的上述对象 id。
对 DrawingML 链禁止 `text`。对象上没有 `hlinkClick` 时 `set` / `delete` 报 `IdNotFound`。删 DrawingML 链时去掉 `a:hlinkClick` 和无人引用的 hyperlink 关系，不删对象。对 OLE id 报 `InvalidId`。
`insertBookmark` 的 `id` 必须是起始段落 id（正文、页眉页脚或框内）；可选 `end` 为同一故事里的结束段落 id。`setBookmark` / `deleteBookmark` 的 `id` 必须是书签 id。
`insertField` 的 `id` 必须是段落 id（正文、页眉页脚或框内）。`setField` / `deleteField` 的 `id` 必须是域 id。
`insertNumbering` 的 `id` 必须是 `docx:document`。`setNumbering` 的 `id` 必须是抽象编号 id。
`setParaNumbering` 的 `id` 必须是段落 id（正文或页眉页脚）。`setStyle` / `deleteStyle` 的 `id` 必须是样式 id。
`insertStyle` 的 `id` 必须是 `docx:document`。`setParaStyle` 的 `id` 必须是段落 id（正文或页眉页脚）。
`setRunStyle` 的 `id` 必须是正文或页眉页脚 run id。
`setParaFormat` 的 `id` 必须是段落 id（正文、页眉页脚、框内、注内或批注内）。`setRunFormat` 的 `id` 必须是 run id。
`insertSection` 的 `id` 必须是主文档 `w:body` 的直接子段落 id 或 `docx:document`。`deleteSection` 的 `id` 必须是 `section:{n}`。
`insertToc` 的 `id` 必须是主文档 `w:body` 的直接子段落 id 或 `docx:document`。`deleteToc` 的 `id` 必须是 `toc:{n}`。
`insertCaption` 的 `id` 必须是主文档 `w:body` 的直接子段落 id 或 `docx:document`。`deleteCaption` 的 `id` 必须是 `caption:{n}`。
`insertCrossRef` 的 `id` 必须是段落 id（正文或页眉页脚）。`deleteCrossRef` 的 `id` 必须是 `xref:{n}`。
`insertIndex` 的 `id` 必须是主文档 `w:body` 的直接子段落 id 或 `docx:document`。`deleteIndex` 的 `id` 必须是 `index:{n}`。
`insertIndexEntry` 的 `id` 必须是段落 id（正文或页眉页脚）。`deleteIndexEntry` 的 `id` 必须是 `xe:{n}`。
`insertSource` 的 `id` 必须是 `docx:document`。`deleteSource` 的 `id` 必须是 `source:{n}`。
`insertCitation` 的 `id` 必须是段落 id（正文或页眉页脚）。`deleteCitation` 的 `id` 必须是 `cite:{n}`。
`insertBibliography` 的 `id` 必须是主文档 `w:body` 的直接子段落 id 或 `docx:document`。`deleteBibliography` 的 `id` 必须是 `biblio:{n}`。
`insertDocPart` 的 `id` 必须是 `docx:document`。`deleteDocPart` 的 `id` 必须是 `docPart:{n}`。
`insertDocPartContent` 的 `id` 必须是主文档 `w:body` 的直接子段落 id 或 `docx:document`。
`insertEmbeddedFont` / `setFontEmbed` / `setPageBackground` / `setThemeFmt` / `setThemeExtraSchemes` / `setThemeObjectDefaults` / `setThemeOverride` 的 `id` 必须是 `docx:document`。
`deleteEmbeddedFont` 的 `id` 必须是 `font:{n}`。`setPageBorders` 的 `id` 必须是 `section:{n}`。
`insertMath` 的 `id` 必须是段落 id（正文、页眉页脚或框内）或 `docx:document`。`setMath` / `deleteMath` 的 `id` 必须是公式 id。
`insertRuby` 的 `id` 必须是段落 id（正文、页眉页脚或框内）或 `docx:document`。`setRuby` / `deleteRuby` 的 `id` 必须是拼音 id。
`setEastAsianSettings` 的 `id` 必须是 `docx:document`。
`insertContentControl` 的 `id` 必须是段落 id（正文、页眉页脚或框内）或 `docx:document`。
`setContentControl` / `deleteContentControl` 的 `id` 必须是内容控件 id。对不上的 id 形态报 `InvalidId`。

图的 `{docPrId}` 取该 `w:drawing` 里 `wp:docPr/@id` 的原文（ECMA-376 §20.4）。输出用文件里的原始大小写与数字写法。
请求里若双方都能解析成十进制整数，按数值匹配（`1` 与 `01` 视为同一处）；否则按原文精确比。同一归属范围内重复 `docPr/@id` 时，inspect 都列出，apply 命中该范围内文档顺序第一处。
页眉页脚的范围是该容器部件；正文的范围是该段。没有 `docPr/@id` 时用 `pic:index:{k}`，树上 `"stable": false`。
页眉页脚的 `{k}` 只在该部件内从 1 计；正文的 `{k}` 只在该段内从 1 计。两种 `{k}` 都计入该范围内所有被收入的图，不只是缺 id 的那些。只认 DrawingML 的 `pic:pic` 且带 `a:blip`。
独立图取外层 `wp:docPr/@id`。组合 `wpg:wgp` 本身不进树，里面符合条件的子 `pic` 按该 `pic:cNvPr/@id` 展开；没有则 `pic:index:{k}`。
VML / `w:pict`、图表、SmartArt、文本框、连接线不进宿主段落的 `pictures`。框内图走框内段落上的 `pictures`，id 带框前缀。`mc:AlternateContent` 仍只进 Choice。
正文图只收主文档段落的子孙，遇 `w:txbxContent` 停住，表格单元格里的段落与正文段落同一套规则。

`insertPicture` 的 `id` 必须是段落 id：正文用 `para:…` / `para:index:{n}`，页眉页脚用 `{容器}/para:…`，框内用 `{框 id}/para:…`，
注内用 `{注 id}/para:…`，批注内用 `{批注 id}/para:…`。
对图 id、容器 id、框 id、注容器 id、批注容器 id 或 `section:{n}` 发 `insertPicture` 报 `InvalidId`。
`replacePicture` / `deletePicture` / `setPictureSize` / `setPictureWrap` 的 `id` 必须是图 id。
对段落 id、容器 id、框 id、注容器 id、批注容器 id 或 `section:{n}` 发这四个 op 报 `InvalidId`。

文本框只认 DrawingML：`w:drawing` 里有 `w:txbxContent`，或 `wps:wsp` / `wps:txbx`（MS-DOCX WordprocessingShape）。
常包在 `mc:AlternateContent` 里，只进 Choice。VML `v:textbox` / `w:pict` 不编、不改。组合 `wpg:wgp` 本身不进树，只展开里面符合条件的框。
框的 `{docPrId}` 取该 `w:drawing` 里 `wp:docPr/@id`，规则与图相同。组合里的框取该 `wps:wsp` 的 `cNvPr/@id`；再没有则 `txbx:index:{k}`。
`k` 只在该宿主段内从 1 计，`stable` 为 `false`。页眉页脚框挂在它所在的页眉或页脚段落上，不挂在节上。

链接框：`wps:txbx` 是链的第一框（§20.4.2.37），后续框用 `linkedTxbx`（§20.4.2.34），同一 `id` 为同一故事，`seq` 为顺序。
树上 `link.prev` / `link.next` 是相邻框的图 id 风格框 id；未链接则为 `null`。不把后继框的字并进当前框。

连接线只认 `wps:wsp`：没有 `txbx` / `txbxContent` / `linkedTxbx`，且带 `wps:cNvCnPr`，
或 `a:prstGeom/@prst` 为 `line`、`straightConnector1`、`bentConnector2`–`5`、`curvedConnector2`–`5`。
Word 16.0 把连接线写成 `wps:wsp`，不是独立 `cxnSp`。
连接线的 `{docPrId}` 取该 `w:drawing` 的 `wp:docPr/@id`；组合里取该 `wps:wsp` 的 `cNvPr/@id`；再没有则 `cxn:index:{k}`。
`k` 只在该宿主段内从 1 计，`stable` 为 `false`。页眉页脚连接线挂在它所在的页眉或页脚段落上。
注内、批注内、框内本期不编连接线；对这些容器发 `insertConnector` 报 `InvalidField` 或 `InvalidId`（见 apply）。
`insertConnector` 的 `id` 必须是正文或页眉页脚段落 id。`deleteConnector` 的 `id` 必须是连接线 id。

无框形状只认 `wps:wsp`：没有 `txbx` / `txbxContent` / `linkedTxbx`，且不是连接线（见上）。
不要把连接线收进 `shapes`。VML / `w:pict` 不编、不改。组合 `wpg:wgp` 本身不进树，里面符合条件的子 `wps:wsp` 展开。
形状的 `{docPrId}` 取该 `w:drawing` 的 `wp:docPr/@id`；组合里取该 `wps:wsp` 的 `cNvPr/@id`；再没有则 `shape:index:{k}`。
`k` 只在该宿主段内从 1 计，`stable` 为 `false`。页眉页脚形状挂在它所在的页眉或页脚段落上。
注内、批注内、框内本期不编形状；对这些容器发 `insertShape` 报 `InvalidField` 或 `InvalidId`（见 apply）。
`insertShape` 的 `id` 必须是正文或页眉页脚段落 id。`deleteShape` / `setShapePos` / `setShapeSize` / `setShapeFill` / `setShapeLine` 的 `id` 必须是形状 id。
`groupShapes` 的 `id` 必须是 `docx:document` 或页眉/页脚容器 id。`ungroup` 的 `id` 必须是该组合内一个图 / 文本框 / 连接线 / 无框形状 id。
`groupShapes` 不收墨迹画布成员；对墨迹 id 发 `groupShapes` 报 `InvalidId`。

墨迹画布只认 DrawingML 里本地名为 `contentPart` 的节点，且关系目标部件 Content Type 为 `application/inkml+xml`，
或该部件根元素本地名为 `ink`、命名空间为 `http://www.w3.org/2003/InkML`。
空 `wpc`（只有 `bg` / `whole`）不算。批注里的 `inkml:ink` / `wpi:ink` 仍只标 `kind: ink`，不进 `inkCanvases`。
`{cNvPrId}` 优先取该 `contentPart` 的 `cNvPr/@id`；没有则用外层 `wp:docPr/@id`；再没有则 `ink:index:{k}`，
树上 `"stable": false`。`k` 只在该宿主段内从 1 计。页眉页脚墨迹挂在它所在的页眉或页脚段落上，名单出现在该容器的 `inkCanvases` 上。
正文墨迹只收主文档段落的子孙，遇 `w:txbxContent` 停住。组合 `wpg:wgp` 本身不进树，里面符合条件的 contentPart 展开。
`mc:AlternateContent` 只进 Choice。注内、批注内、框内本期不编。
`insertInkCanvas` 的 `id` 必须是正文或页眉页脚段落 id；对注内或批注内段落报 `InvalidField`，对框内段落报 `InvalidId`。
`replaceInkCanvas` / `deleteInkCanvas` / `setInkCanvasSize` / `setInkCanvasWrap` 的 `id` 必须是墨迹画布 id。xlsx / pptx 上这些 op 与 `scope.inkCanvases` 报 `InvalidField`。
写入只走 `wpc` 画布加 `w14:contentPart` 加 InkML 部件；独立 `wpi` / `wordprocessingInk` 不作为写入路径。
inspect 若文件里已有这种 markup，仍认 `contentPart` 加 InkML。
不把 InkML 原文摊进树。

「宿主段落遇到框就停住」仍成立，否则外层 `paraCount` 会把框内段算进去。边界按下面两条划：

- 遍历宿主段落（正文或页眉页脚）时遇到 `w:txbxContent` 就停住，不再往下走。
  框内 `w:p` 不计入宿主 `paraCount`，不出现在 `outline` 与宿主 `paragraphs` 里，它们的 `w:t` 也不并进外层段落的 `text`。
  拿框内 `paraId` 当正文 `para:` 发 apply，报 `IdNotFound`。框内段落必须用带框前缀的 id。
- 遇到 `mc:AlternateContent` 只走第一个 `mc:Choice`；没有 `mc:Choice` 才走 `mc:Fallback`。两支都走会把同一份内容读出两遍，也会让 apply 只改到其中一支、另一支不同步。
  这条对 `paraCount`、`outline`、`paragraphs`、`text`、`runCount`、`runs`、页眉页脚、`pictureCount`、`pictures`、`textBoxCount`、
  `textBoxes`、`connectorCount`、`connectors`、`shapeCount`、`shapes`、`inkCanvasCount`、`inkCanvases`、`chartCount`、`charts`、`smartArtCount`、`smartArts`、`oleObjectCount`、
  `oleObjects`、`model3dCount`、`model3ds`、`watermarkCount`、`watermarks` 和 apply 一律适用。

Word 2010 以后插入文本框、图表、SmartArt 都是 `mc:AlternateContent` 套 `w:txbxContent` 的写法，所以这两条不是边角情况。

同一部件里若出现重复的 `w14:paraId`（违反 MS-DOCX，但现实文件可能有），inspect 照样列出每一段；apply 命中该部件文档顺序中的第一段。


## inspect

xlsx、docx、pptx 共用请求字段名 `scope`，里面的键按格式分，混用报 `InvalidField`。请求信封见 [json-contract.md](json-contract.md#请求)。

### scope

```json
{
  "schemaVersion": 1,
  "action": "inspect",
  "path": "C:\\data\\note.docx",
  "format": "docx",
  "scope": {
    "from": 1,
    "maxParas": 200,
    "headerFooter": true,
    "pictures": true,
    "textBoxes": true,
    "connectors": true,
    "shapes": true,
    "inkCanvases": true,
    "notes": true,
    "comments": true,
    "charts": true,
    "smartArt": true,
    "oleObjects": true,
    "watermarks": true,
    "tables": true,
    "revisions": true,
    "hyperlinks": true,
    "bookmarks": true,
    "fields": true,
    "tocs": true,
    "captions": true,
    "crossRefs": true,
    "indexes": true,
    "indexEntries": true,
    "sources": true,
    "citations": true,
    "bibliographies": true,
    "docParts": true,
    "maths": true,
    "contentControls": true,
    "numbering": true,
    "styles": true,
    "runs": true
  }
}
```

- **`path`**（是）
  - 已有文件
- **`scope`**（否）
  - 省略或空对象：返回下列计数与结构
    - `outline`
    - `paraCount`
    - `pictureCount`
    - `textBoxCount`
    - `connectorCount`
    - `shapeCount`
    - `inkCanvasCount`
    - `footnoteCount`
    - `endnoteCount`
    - `commentCount`
    - `chartCount`
    - `smartArtCount`
    - `oleObjectCount`
    - `watermarkCount`
    - `tableCount`
    - `revisionCount`
    - `hyperlinkCount`
    - `bookmarkCount`
    - `fieldCount`
    - `tocCount`
    - `captionCount`
    - `crossRefCount`
    - `indexCount`
    - `indexEntryCount`
    - `sourceCount`
    - `citationCount`
    - `bibliographyCount`
    - `docPartCount`
    - `mathCount`
    - `contentControlCount`
    - `numberingCount`
    - `styleCount`
    - `embeddedFontCount`
    - `fontEmbed`
    - `pageBackground`
    - `headers`
    - `footers`
    - `sections`
  - 正文没有 `paragraphs`
  - 页眉页脚容器不带 `paragraphs`
  - 没有下列名单
    - `footnotes`
    - `endnotes`
    - `noteSeparators`
    - `comments`
    - `charts`
    - `smartArts`
    - `oleObjects`
    - `watermarks`
    - `tables`
    - `revisions`
    - `hyperlinks`
    - `bookmarks`
    - `fields`
    - `tocs`
    - `captions`
    - `crossRefs`
    - `indexes`
    - `indexEntries`
    - `sources`
    - `citations`
    - `bibliographies`
    - `docParts`
    - `embeddedFonts`
    - `maths`
    - `contentControls`
    - `numberings`
    - `styles`
- **`scope.from`**（要正文段落时必填）
  - 从第几段开始，从 1 起，按主文档 `w:p` 顺序
- **`scope.maxParas`**（要正文段落时必填）
  - 本次最多返回多少段，允许范围 `1` 到 `5000`
- **`scope.headerFooter`**（否）
  - 为 `true` 时给每个页眉、页脚节点填 `paragraphs`
  - 省略或 `false` 则容器上没有 `paragraphs`
- **`scope.pictures`**（否）
  - 为 `true` 时给每个页眉、页脚节点填 `pictures`
  - 若同时给了 `from` 与 `maxParas`，切片里每段也填 `pictures`
  - 若同时 `notes: true`，每条用户注也填 `pictures`
  - 若同时 `comments: true`，每条批注也填 `pictures`
  - 省略或 `false` 则这些节点上都没有 `pictures`
  - 不读媒体字节
- **`scope.textBoxes`**（否）
  - 为 `true` 时给每个页眉、页脚节点填 `textBoxes`
  - 若同时给了 `from` 与 `maxParas`，切片里每段也填 `textBoxes`
  - 若同时 `notes: true`，每条用户注也填 `textBoxes`
  - 若同时 `comments: true`，每条批注也填 `textBoxes`
  - 省略或 `false` 则这些节点上都没有 `textBoxes`
- **`scope.connectors`**（否）
  - 为 `true` 时给每个页眉、页脚节点填 `connectors`
  - 若同时给了 `from` 与 `maxParas`，切片里每段也填 `connectors`
  - 若同时 `notes: true`，每条用户注也填 `connectors`
  - 若同时 `comments: true`，每条批注也填 `connectors`
  - 省略或 `false` 则这些节点上都没有 `connectors`
  - 不依赖 `headerFooter`
  - xlsx / pptx 也认此键，见各自分册
- **`scope.shapes`**（否）
  - 为 `true` 时给每个页眉、页脚节点填 `shapes`
  - 若同时给了 `from` 与 `maxParas`，切片里每段也填 `shapes`
  - 省略或 `false` 则这些节点上都没有 `shapes`
  - 不依赖 `headerFooter`
  - 根上不挂正文 `shapes`
  - 注节点与批注节点不填此名单
  - xlsx / pptx 也认此键，见各自分册
- **`scope.inkCanvases`**（否）
  - 为 `true` 时给每个页眉、页脚节点填 `inkCanvases`
  - 若同时给了 `from` 与 `maxParas`，切片里每段也填 `inkCanvases`
  - 省略或 `false` 则这些节点上都没有 `inkCanvases`
  - 不依赖 `headerFooter`
  - 根上不挂正文 `inkCanvases`
  - 注节点与批注节点不填此名单
  - xlsx / pptx 上出现此键报 `InvalidField`
- **`scope.textBoxContent`**（否）
  - 为 `true` 且该节点已有 `textBoxes` 时，每个框再带全部 `paragraphs`
  - 若同时 `pictures: true`，框内段再带 `pictures`
  - 省略或 `false` 则框上只有计数，没有框内名单
  - 不与 `from` / `maxParas` 抢正文切片
- **`scope.notes`**（否）
  - 为 `true` 时列出全部用户脚注、尾注（`footnotes` / `endnotes`）以及分隔符（`noteSeparators`）
  - 每条用户注带全部注内段落，不再用 `from` / `maxParas` 切
  - 省略或 `false` 则只有两个计数，没有这三份名单
- **`scope.comments`**（否）
  - 为 `true` 时列出全部批注（`comments`）
  - 每条带全部批注内段落，不再用 `from` / `maxParas` 切
  - 省略或 `false` 则只有 `commentCount`，没有名单
- **`scope.charts`**（否）
  - 为 `true` 时列出正文图表（根上 `charts`）以及每个页眉、页脚容器上的 `charts`
  - 若同时 `notes: true` / `comments: true`，每条用户注或批注也填 `charts`
  - 已列出的框在给了 `textBoxes` 时也填 `charts`
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `chartCount`，没有名单
  - 不把 embedding 字节放进树
- **`scope.smartArt`**（否）
  - 为 `true` 时列出正文 SmartArt（根上 `smartArts`）以及每个页眉、页脚容器上的 `smartArts`
  - 若同时 `notes: true` / `comments: true`，每条用户注或批注也填 `smartArts`
  - 已列出的框在给了 `textBoxes` 时也填 `smartArts`
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `smartArtCount`，没有名单
  - 不把图示画成图，不读 Fallback 位图
- **`scope.oleObjects`**（否）
  - 为 `true` 时列出正文嵌入对象（根上 `oleObjects`）以及每个页眉、页脚容器上的 `oleObjects`
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `oleObjectCount`，没有名单
  - 不把嵌入字节或预览图放进树
  - 框内、注内、批注内本期不列出
- **`scope.model3ds`**（否）
  - 为 `true` 时列出每个页眉、页脚容器上的 `model3ds`
  - 若同时给了 `from` 与 `maxParas`，切片里每段也填 `model3ds`
  - 省略或 `false` 则这些节点上都没有 `model3ds`
  - 不依赖 `headerFooter`
  - 根上不挂正文 `model3ds`
  - 注节点与批注节点不填此名单
- **`scope.watermarks`**（否）
  - 为 `true` 时列出全部页眉 DrawingML 水印（根上 `watermarks`）
  - 不依赖 `headerFooter` 或 `from` / `maxParas`
  - 省略或 `false` 则只有 `watermarkCount`，没有名单
  - xlsx / pptx 上出现此键报 `InvalidField`
- **`scope.tables`**（否）
  - 为 `true` 时列出正文表（根上 `tables`）以及每个页眉、页脚容器上的 `tables`
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `tableCount`，没有名单
- **`scope.revisions`**（否）
  - 为 `true` 时列出正文修订（根上 `revisions`）以及每个页眉、页脚容器上的 `revisions`
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `revisionCount`，没有名单
- **`scope.hyperlinks`**（否）
  - 为 `true` 时列出正文 `w:hyperlink`（根上 `hyperlinks`）以及每个页眉、页脚容器上的 `hyperlinks`
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `hyperlinkCount`，没有名单
  - DrawingML 链不进这份名单，而在图 / 文本框 / 连接线 / 无框形状 / SmartArt / 图表 / 墨迹画布 / 3D 节点的 `hyperlink` 字段
- **`scope.bookmarks`**（否）
  - 为 `true` 时列出正文书签（根上 `bookmarks`）以及每个页眉、页脚容器上的 `bookmarks`
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `bookmarkCount`，没有名单
- **`scope.editRanges`**（否）
  - 为 `true` 时列出正文例外编辑区（根上 `editRanges`）以及每个页眉、页脚容器上的 `editRanges`
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `editRangeCount`，没有名单
  - xlsx / pptx 上出现此键报 `InvalidField`
- **`scope.fields`**（否）
  - 为 `true` 时列出正文域（根上 `fields`）以及每个页眉、页脚容器上的 `fields`
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `fieldCount`，没有名单
- **`scope.tocs`**（否）
  - 为 `true` 时列出主文档目录块（根上 `tocs`）
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `tocCount`，没有名单
  - 页眉页脚里的 TOC 仍只出现在该容器 `fields` 里
- **`scope.captions`**（否）
  - 为 `true` 时列出主文档题注（根上 `captions`）
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `captionCount`，没有名单
  - 页眉页脚里的 SEQ 仍只出现在该容器 `fields` 里
- **`scope.crossRefs`**（否）
  - 为 `true` 时列出主文档交叉引用（根上 `crossRefs`）
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `crossRefCount`，没有名单
  - 页眉页脚里的 REF / PAGEREF / NOTEREF 仍只出现在该容器 `fields` 里
- **`scope.indexes`**（否）
  - 为 `true` 时列出主文档 INDEX 块（根上 `indexes`）
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `indexCount`，没有名单
  - 页眉页脚里的 INDEX 仍只出现在该容器 `fields` 里
- **`scope.indexEntries`**（否）
  - 为 `true` 时列出主文档 XE 索引项（根上 `indexEntries`）
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `indexEntryCount`，没有名单
  - 页眉页脚里的 XE 仍只出现在该容器 `fields` 里
- **`scope.sources`**（否）
  - 为 `true` 时列出书目源（根上 `sources`，来自 bibliography customXml）
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `sourceCount`，没有名单
- **`scope.citations`**（否）
  - 为 `true` 时列出主文档 CITATION（根上 `citations`）
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `citationCount`，没有名单
  - 页眉页脚里的 CITATION 仍只出现在该容器 `fields` 里
- **`scope.bibliographies`**（否）
  - 为 `true` 时列出主文档 BIBLIOGRAPHY 块（根上 `bibliographies`）
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `bibliographyCount`，没有名单
  - 页眉页脚里的 BIBLIOGRAPHY 仍只出现在该容器 `fields` 里
- **`scope.docParts`**（否）
  - 为 `true` 时列出 glossary 里的构建基块（根上 `docParts`）
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `docPartCount`，没有名单
- **`scope.embeddedFonts`**（否）
  - 为 `true` 时列出 fontTable 里的嵌入字体面（根上 `embeddedFonts`）
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `embeddedFontCount` 与 `fontEmbed`，没有名单
- **`scope.maths`**（否）
  - 为 `true` 时列出正文公式（根上 `maths`）以及每个页眉、页脚容器上的 `maths`
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `mathCount`，没有名单
- **`scope.contentControls`**（否）
  - 为 `true` 时列出正文内容控件（根上 `contentControls`）以及每个页眉、页脚容器上的 `contentControls`
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `contentControlCount`，没有名单
- **`scope.numbering`**（否）
  - 为 `true` 时列出编号实例（根上 `numberings`）
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `numberingCount`，没有名单
  - 页眉页脚容器不挂这份名单
- **`scope.styles`**（否）
  - 为 `true` 时列出样式定义（根上 `styles`）
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `styleCount`，没有名单
  - 页眉页脚容器不挂这份名单
- **`scope.runs`**（否）
  - 为 `true` 时，已列出的段落（正文切片、已展开的页眉页脚、框内、注内、批注内、表格单元格）带 `runs`
  - 省略或 `false` 则这些段只有 `runCount`，没有名单
- **`scope.rubies`**（否）
  - 为 `true` 时列出正文拼音（根上 `rubies`）以及每个页眉、页脚容器上的 `rubies`
  - 不依赖 `from` / `maxParas`
  - 省略或 `false` 则只有 `rubyCount`，没有名单

`from` / `maxParas` 只切正文，`headerFooter` 只管页眉页脚段落，`pictures` 只展开图名单，`textBoxes` 只展开框名单，`connectors` 只展开连接线名单，
`shapes` 只展开无框形状名单，`inkCanvases` 只展开墨迹画布名单，
`notes` 只展开脚注尾注，`comments` 只展开批注，`charts` 只展开图表名单，`smartArt` 只展开图示名单，`oleObjects` 只展开嵌入对象名单，`watermarks` 只展开水印名单，
`tables` 只展开表名单，`revisions` 只展开修订名单，`hyperlinks` 只展开超链接名单，`bookmarks` 只展开书签名单，`editRanges` 只展开例外编辑区名单，`fields` 只展开域名单，`tocs` 只展开目录名单，
`captions` 只展开题注名单，`crossRefs` 只展开交叉引用名单，`indexes` 只展开索引名单，`indexEntries` 只展开索引项名单，`sources` 只展开书目源名单，
`citations` 只展开引文名单，`bibliographies` 只展开书目块名单，`docParts` 只展开构建基块名单，`embeddedFonts` 只展开嵌入字体名单，`maths` 只展开公式名单，
`contentControls` 只展开内容控件名单，`numbering` 只展开编号实例名单，`styles` 只展开样式定义名单，`runs` 只展开已列出段落上的 run 名单，`rubies` 只展开拼音名单。
这些键彼此独立，可只开一边。只要想看正文段落，就必须同时给 `from` 和 `maxParas`。不默认把全文倒出来。正文图、正文框、正文连接线、正文无框形状与正文墨迹画布名单只出现在已切出的段落上，不挂到文档根。
正文图表名单、正文 SmartArt 名单、正文 OLE 名单、水印名单、正文表名单、正文修订名单、正文超链接名单、正文书签名单、正文域名单、目录名单、题注名单、交叉引用名单、索引名单、索引项名单、书目源名单、引文名单、书目块名单、
构建基块名单、嵌入字体名单、公式名单、拼音名单、内容控件名单、编号实例名单与样式定义名单挂在文档根上，不挂在段落上。
`from` 超出 `paraCount` 时正文 `paragraphs` 为空数组，`truncated` 为 `false`，不报错。
页眉页脚通常很短，给了对应键就返回该部件全部段落、图、框、图表、SmartArt、嵌入对象、表、修订、超链接、书签、域、公式、拼音或内容控件，都不再单独分页。
框通常短，给了 `textBoxContent: true` 就返回该框全部段落，不再单独分页。用户注和批注通常不多，给了 `notes: true` 或 `comments: true` 就返回全部名单，每条倒出全部段落，不再单独分页。
图表通常不多，给了 `charts: true` 就返回全部名单，每张带全部系列，不再单独分页。SmartArt 通常不多，给了 `smartArt: true` 就返回全部名单，每张带全部可见点，不再单独分页。
嵌入对象通常不多，给了 `oleObjects: true` 就返回全部名单。水印通常不多，给了 `watermarks: true` 就返回全部名单。表通常不多，给了 `tables: true` 就返回全部名单。
修订通常不多，给了 `revisions: true` 就返回全部名单。超链接、书签与域通常不多，给了对应键就返回全部名单。编号定义通常不多，给了 `numbering: true` 就返回全部实例。
样式定义通常不多，给了 `styles: true` 就返回全部样式。已列出的段落始终有 `runCount`；给了 `runs: true` 才带 `runs`。
xlsx 的 `scope` 里出现 `textBoxContent`、`notes`、`oleObjects`、`watermarks`、`tables`、`timing`、`revisions`、`hyperlinks`、
`bookmarks`、`editRanges`、`fields`、`tocs`、`captions`、`crossRefs`、`indexes`、`indexEntries`、`sources`、`citations`、`bibliographies`、
`docParts`、`maths`、`contentControls`、`numbering`、`styles`、`runs`、`rubies` 或 `inkCanvases` 报 `InvalidField`。
xlsx 认 `scope.pictures`、`scope.charts`、`scope.headerFooter`、`scope.comments`、`scope.textBoxes` 与 `scope.smartArt`。
pptx 的 `scope` 里出现 `headerFooter`、`textBoxes`、`textBoxContent`、`revisions`、`hyperlinks`、`bookmarks`、`editRanges`、`fields`、`tocs`、
`captions`、`crossRefs`、`indexes`、`indexEntries`、`sources`、`citations`、`bibliographies`、`docParts`、`maths`、
`contentControls`、`numbering`、`styles`、`oleObjects`、`watermarks`、`runs`、`rubies` 或 `inkCanvases` 报 `InvalidField`。
pptx 认 `scope.pictures`、`scope.charts`、`scope.smartArt`、`scope.tables`、`scope.notes`、`scope.comments` 与 `scope.timing`。
xlsx / pptx 的 `scope.tables` 不是同一件事：xlsx 仍拒绝；pptx 展开幻灯片 DrawingML 表；docx 展开 `w:tbl`。
只给 `textBoxContent`、不给 `textBoxes` 报 `InvalidField`。xlsx 的 `scope` 里出现 `textBoxContent` 仍报 `InvalidField`。


### 响应

```json
{
  "schemaVersion": 1,
  "format": "docx",
  "id": "docx:document",
  "paraCount": 4,
  "pictureCount": 0,
  "textBoxCount": 0,
  "connectorCount": 0,
  "shapeCount": 0,
  "inkCanvasCount": 0,
  "footnoteCount": 0,
  "endnoteCount": 0,
  "commentCount": 0,
  "chartCount": 0,
  "smartArtCount": 0,
  "oleObjectCount": 0,
  "model3dCount": 0,
  "watermarkCount": 0,
  "tableCount": 0,
  "revisionCount": 0,
  "hyperlinkCount": 0,
  "bookmarkCount": 0,
  "editRangeCount": 0,
  "fieldCount": 0,
  "tocCount": 0,
  "captionCount": 0,
  "crossRefCount": 0,
  "indexCount": 0,
  "indexEntryCount": 0,
  "sourceCount": 0,
  "citationCount": 0,
  "bibliographyCount": 0,
  "docPartCount": 0,
  "mathCount": 0,
  "rubyCount": 0,
  "contentControlCount": 0,
  "numberingCount": 0,
  "styleCount": 4,
  "embeddedFontCount": 0,
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
      "sections": [
        1
      ],
      "paraCount": 1,
      "pictureCount": 0,
      "textBoxCount": 0,
      "connectorCount": 0,
      "shapeCount": 0,
      "inkCanvasCount": 0,
      "chartCount": 0,
      "smartArtCount": 0,
      "oleObjectCount": 0,
      "model3dCount": 0,
      "watermarkCount": 0,
      "tableCount": 0,
      "revisionCount": 0,
      "hyperlinkCount": 0,
      "bookmarkCount": 0,
      "editRangeCount": 0,
      "fieldCount": 0,
      "mathCount": 0,
      "rubyCount": 0,
      "contentControlCount": 0
    }
  ],
  "footers": [
    {
      "id": "footer:default:1",
      "type": "default",
      "sections": [
        1
      ],
      "paraCount": 1,
      "pictureCount": 0,
      "textBoxCount": 0,
      "connectorCount": 0,
      "shapeCount": 0,
      "inkCanvasCount": 0,
      "chartCount": 0,
      "smartArtCount": 0,
      "oleObjectCount": 0,
      "model3dCount": 0,
      "watermarkCount": 0,
      "tableCount": 0,
      "revisionCount": 0,
      "hyperlinkCount": 0,
      "bookmarkCount": 0,
      "editRangeCount": 0,
      "fieldCount": 0,
      "mathCount": 0,
      "rubyCount": 0,
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
      "headerLink": {
        "default": false,
        "even": null,
        "first": null
      },
      "footerLink": {
        "default": false,
        "even": null,
        "first": null
      },
      "pageBorders": null,
      "formProt": null
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
  "protection": {
    "enforced": false,
    "edit": null,
    "formatting": false,
    "hasPassword": false,
    "readOnlyRecommended": false
  },
  "theme": null,
  "themeOverride": null,
  "fontEmbed": {
    "trueType": false,
    "systemFonts": false,
    "subset": false
  },
  "pageBackground": {
    "color": null,
    "display": false,
    "fill": null
  },
  "eastAsianSettings": {
    "characterSpacingControl": null,
    "strictFirstAndLastChars": null,
    "noLineBreaksBefore": null,
    "noLineBreaksAfter": null,
    "themeFontLang": null
  },
  "bibliographyStyle": null,
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
      "textBoxCount": 0,
      "connectorCount": 0,
      "shapeCount": 0,
      "inkCanvasCount": 0
    },
    {
      "id": "para:index:2",
      "index": 2,
      "style": "Normal",
      "text": "hello",
      "stable": false,
      "pictureCount": 0,
      "textBoxCount": 0,
      "connectorCount": 0,
      "shapeCount": 0,
      "inkCanvasCount": 0
    }
  ]
}
```

规则：

- 主文档部件（根关系里 `/officeDocument` 那条，常见路径 `/word/document.xml`）始终读。页眉、页脚部件按节引用与继承解析后读。
  脚注、尾注、批注部件按主文档关系去读（常见 `/word/footnotes.xml`、`/word/endnotes.xml`、`/word/comments.xml`）；没有对应关系则对应计数为 0。
  `commentsExtended`、`people`、`commentsIds`、`commentsExtensible` 有则读，没有不报错。
- 文档根字段顺序：`id`、`paraCount`、`pictureCount`、`textBoxCount`、`connectorCount`、`shapeCount`、`inkCanvasCount`、`footnoteCount`、`endnoteCount`、`commentCount`、
  `chartCount`、`smartArtCount`、`oleObjectCount`、`model3dCount`、`watermarkCount`、`tableCount`、`revisionCount`、`hyperlinkCount`、
  `bookmarkCount`、`editRangeCount`、`fieldCount`、`tocCount`、`captionCount`、`crossRefCount`、`indexCount`、`indexEntryCount`、`sourceCount`、
  `citationCount`、`bibliographyCount`、`docPartCount`、`mathCount`、`rubyCount`、`contentControlCount`、`numberingCount`、`styleCount`、
  `embeddedFontCount`、`outline`、`headers`、`footers`、`sections`、`mirrorMargins`、`gutterAtTop`、`props`、`protection`、`theme`、`themeOverride`、
  `fontEmbed`、`pageBackground`、`eastAsianSettings`、`bibliographyStyle`，
  有注名单时再跟 `footnotes`、`endnotes`、`noteSeparators`，有批注名单时再跟 `comments`，有图表名单时再跟 `charts`，有 SmartArt 名单时再跟 `smartArts`，
  有 OLE 名单时再跟 `oleObjects`，有水印名单时再跟 `watermarks`，有表名单时再跟 `tables`，有修订名单时再跟 `revisions`，有超链接名单时再跟 `hyperlinks`，
  有书签名单时再跟 `bookmarks`，有例外编辑区名单时再跟 `editRanges`，有域名单时再跟 `fields`，有目录名单时再跟 `tocs`，有题注名单时再跟 `captions`，有交叉引用名单时再跟 `crossRefs`，有索引名单时再跟 `indexes`，
  有索引项名单时再跟 `indexEntries`，有书目源名单时再跟 `sources`，有引文名单时再跟 `citations`，有书目块名单时再跟 `bibliographies`，有构建基块名单时再跟 `docParts`，
  有嵌入字体名单时再跟 `embeddedFonts`，有公式名单时再跟 `maths`，有内容控件名单时再跟 `contentControls`，有编号名单时再跟 `numberings`，有样式名单时再跟 `styles`，
  有正文切片时再跟 `from`、`truncated`、`paragraphs`。
  `props` 始终出现，规则见 [json-contract.md](json-contract.md#文档属性三格式共用)。`protection` 始终出现，规则见下文「文档保护」。`theme` 与 `themeOverride` 始终出现，规则见下文「主题」。
  xlsx / pptx 根上不要带 `protection`，也不要带 `watermarkCount` / `watermarks`，
  也不要带 `embeddedFontCount` / `fontEmbed` / `embeddedFonts`，也不要带 `pageBackground`。
  pptx 根上不要带 `themeOverride`。pptx 的 theme 挂在 `masters[].theme`。
  xlsx 根上也有 `theme` / `themeOverride`，见 [xlsx 响应](json-contract-xlsx.md#响应)。
  xlsx 根上带 `workbookProtection`。
- `paraCount` 是主文档里 `w:p` 的总数，含表格单元格里的段落，不含文本框里的段落，始终出现。
- 文档根上的 `pictureCount` 始终出现，只计主文档里被收入的 DrawingML 图片个数（判定见上文 docx 图 id 那节），不含页眉页脚，也不含框内图、注内图、批注内图。根上不挂正文 `pictures`。
- 文档根上的 `textBoxCount` 始终出现，只计主文档里被收入的文本框个数，不含页眉页脚，也不含注内、批注内的框。根上不挂正文 `textBoxes`。
- 文档根上的 `connectorCount` 始终出现，只计主文档里被收入的连接线个数（判定见上文连接线 id 那节），不含页眉页脚，也不含注内、批注内、框内。根上不挂正文 `connectors`。
- 文档根上的 `shapeCount` 始终出现，只计主文档里被收入的无框形状个数（判定见上文无框形状 id 那节），不含页眉页脚，也不含注内、批注内、框内。没有则为 `0`。
  根上不挂正文 `shapes`。注节点与批注节点不加 `shapeCount`。
- 文档根上的 `inkCanvasCount` 始终出现，只计主文档里被收入的墨迹画布个数（判定见上文墨迹画布 id 那节），不含页眉页脚，也不含注内、批注内、框内。没有则为 `0`。
  根上不挂正文 `inkCanvases`。注节点与批注节点不加 `inkCanvasCount`。
- 文档根上的 `footnoteCount` / `endnoteCount` 始终出现，只计用户注（`normal` 或未写 type），不含分隔符。没有注部件则为 `0`。根上不挂注名单，除非给了 `scope.notes`。
- 文档根上的 `commentCount` 始终出现，计 `comments.xml` 里全部 `w:comment`（含回复与墨迹）。没有批注部件则为 `0`。根上不挂批注名单，除非给了 `scope.comments`。
- 文档根上的 `chartCount` 始终出现，只计主文档里被收入的图表个数（判定见上文 docx 图表 id 那节），不含页眉页脚，也不含框内、注内、批注内的图表。没有图表则为 `0`。根上不挂正文 `charts`，除非给了 `scope.charts`。
- 文档根上的 `smartArtCount` 始终出现，只计主文档里被收入的 SmartArt 个数（判定见上文 docx SmartArt id 那节），不含页眉页脚，也不含框内、注内、批注内的图示。没有图示则为 `0`。
  根上不挂正文 `smartArts`，除非给了 `scope.smartArt`。
- 文档根上的 `oleObjectCount` 始终出现，只计主文档里被收入的嵌入对象个数（判定见上文 docx OLE id 那节），不含页眉页脚，也不含框内、注内、批注内。没有则为 `0`。
  根上不挂正文 `oleObjects`，除非给了 `scope.oleObjects`。每条字段顺序：`id`、`progId`、`name`、`kind`、`contentType`、`stable`。
  `progId` 取 `o:OLEObject/@ProgID` 或 `w:objectEmbed/@progId`，缺则为 `null`。
  `name` 取该 `w:object` 里 `wp:docPr/@name`，没有则为 `null`。`kind`：ProgID 为 `Package`（大小写不敏感）时为 `package`，否则为 `embed`。
  `contentType` 取 embeddings 部件在 `[Content_Types].xml` 里的 Default 或 Override，都没有则为 `null`。不把嵌入字节放进树。
- 文档根上的 `model3dCount` 始终出现，只计主文档里被收入的 3D 模型个数（判定见上文 3D 模型 id 那节），不含页眉页脚，也不含框内、注内、批注内。没有则为 `0`。
  根上不挂正文 `model3ds`。注节点与批注节点不加 `model3dCount`。
- 文档根上的 `watermarkCount` 始终出现，计所有页眉部件里被收入的 DrawingML 水印个数（判定见上文水印 id 那节），不含页脚。没有则为 `0`。
  根上不挂 `watermarks`，除非给了 `scope.watermarks`。每条字段顺序：`id`、`kind`、`name`、`text`、`stable`。
  `kind` 为 `text`（含 `wps:txbx` / `w:txbxContent`）或 `picture`（含 `pic:pic` 与 `a:blip`）。`name` 取 `wp:docPr/@name`，缺则为 `null`。
  `text`：文字水印为框内可见文本，图片水印为 `null`。
- 文档根上的 `tableCount` 始终出现，只计主文档里被收入的 `w:tbl` 个数（含嵌套表，判定见上文表 id 那节），不含页眉页脚，也不含框内、注内、批注内的表。没有表则为 `0`。根上不挂正文 `tables`，除非给了 `scope.tables`。框节点另报自己的 `tableCount`。
- 文档根上的 `revisionCount` 始终出现，只计主文档里被收入的修订节点个数（判定见上文修订 id 那节），不含页眉页脚，也不含框内、注内、批注内的修订。没有修订则为 `0`。
  根上不挂正文 `revisions`，除非给了 `scope.revisions`。
- 文档根上的 `hyperlinkCount` 始终出现，只计主文档里被收入的 `w:hyperlink` 个数（判定见上文超链接 id 那节），不含页眉页脚，也不含框内、注内、批注内的链，也不含 DrawingML `a:hlinkClick`。没有链则为 `0`。
  根上不挂正文 `hyperlinks`，除非给了 `scope.hyperlinks`。
- 文档根上的 `bookmarkCount` 始终出现，只计主文档里被收入的书签个数（判定见上文书签 id 那节），不含页眉页脚，也不含框内、注内、批注内的书签。没有书签则为 `0`。
  根上不挂正文 `bookmarks`，除非给了 `scope.bookmarks`。
- 文档根上的 `editRangeCount` 始终出现，只计主文档里成对或不成对的 `w:permStart` 个数，不含页眉页脚，也不含框内、注内、批注内。没有则为 `0`。
  根上不挂正文 `editRanges`，除非给了 `scope.editRanges`。xlsx / pptx 根上不要带 `editRangeCount`。
- 文档根上的 `fieldCount` 始终出现，只计主文档里被收入的域个数（判定见上文域 id 那节），不含页眉页脚，也不含框内、注内、批注内的域。没有域则为 `0`。根上不挂正文 `fields`，除非给了 `scope.fields`。
- 文档根上的 `tocCount` 始终出现，只计主文档里被收入的目录块个数（判定见上文目录 id 那节）。没有目录则为 `0`。根上不挂 `tocs`，除非给了 `scope.tocs`。页眉页脚容器不出现 `tocCount`。
- 文档根上的 `captionCount` 始终出现，只计主文档里被收入的题注个数（判定见上文题注 id 那节）。没有题注则为 `0`。根上不挂 `captions`，除非给了 `scope.captions`。页眉页脚容器不出现 `captionCount`。
- 文档根上的 `crossRefCount` 始终出现
- 只计主文档里被收入的交叉引用个数（判定见上文交叉引用 id 那节）。没有交叉引用则为 `0`。根上不挂 `crossRefs`
- 除非给了 `scope.crossRefs`。页眉页脚容器不出现 `crossRefCount`。
- 文档根上的 `indexCount` 始终出现，只计主文档里被收入的 INDEX 块个数（判定见上文索引 id 那节）。没有索引则为 `0`。根上不挂 `indexes`，除非给了 `scope.indexes`。页眉页脚容器不出现 `indexCount`。
- 文档根上的 `indexEntryCount` 始终出现，只计主文档里被收入的 XE 索引项个数（判定见上文索引项 id 那节）。没有索引项则为 `0`。
  根上不挂 `indexEntries`，除非给了 `scope.indexEntries`。页眉页脚容器不出现 `indexEntryCount`。
- 文档根上的 `sourceCount` 始终出现
- 只计 bibliography customXml 里 `b:Source` 的个数。没有该部件则为 `0`。根上不挂 `sources`
- 除非给了 `scope.sources`。页眉页脚容器不出现 `sourceCount`。
- 文档根上的 `citationCount` 始终出现
- 只计主文档里被收入的 CITATION 个数（判定见上文引文 id 那节）。没有引文则为 `0`。根上不挂 `citations`
- 除非给了 `scope.citations`。页眉页脚容器不出现 `citationCount`。
- 文档根上的 `bibliographyCount` 始终出现，只计主文档里被收入的 BIBLIOGRAPHY 块个数（判定见上文书目块 id 那节）。没有书目块则为 `0`。
  根上不挂 `bibliographies`，除非给了 `scope.bibliographies`。页眉页脚容器不出现 `bibliographyCount`。
- 文档根上的 `docPartCount` 始终出现，只计 glossary 部件里 `w:docPart` 的个数。没有该部件则为 `0`。根上不挂 `docParts`，除非给了 `scope.docParts`。页眉页脚容器不出现 `docPartCount`。
- 文档根上的 `embeddedFontCount` 始终出现，只计 fontTable 里 `w:embedRegular` / `w:embedBold` / `w:embedItalic` / `w:embedBoldItalic` 的个数。
  没有 fontTable 或没有任何 embed* 则为 `0`。根上不挂 `embeddedFonts`，除非给了 `scope.embeddedFonts`。页眉页脚容器不出现 `embeddedFontCount`。
- 文档根上的 `mathCount` 始终出现，只计主文档里被收入的顶层 `m:oMath` 个数（判定见上文公式 id 那节），不含页眉页脚，也不含框内、注内、批注内的公式。没有公式则为 `0`。根上不挂 `maths`，除非给了 `scope.maths`。
- 文档根上的 `rubyCount` 始终出现，只计主文档里被收入的 `w:ruby` 个数（判定见上文拼音 id 那节），不含页眉页脚，也不含框内、注内、批注内的拼音。没有拼音则为 `0`。根上不挂 `rubies`，除非给了 `scope.rubies`。
- 文档根上的 `contentControlCount` 始终出现，只计主文档里被收入的 `w:sdt` 个数（判定见上文内容控件 id 那节），不含页眉页脚，也不含框内、注内、批注内的控件。没有控件则为 `0`。
  根上不挂 `contentControls`，除非给了 `scope.contentControls`。
- 文档根上的 `numberingCount` 始终出现，计 `numbering.xml` 里 `w:num` 且 `numId` 不为 0 的个数。没有 numbering 部件则为 `0`。
  根上不挂 `numberings`，除非给了 `scope.numbering`。页眉页脚容器不出现 `numberingCount`。
- 文档根上的 `styleCount` 始终出现，计 `styles.xml` 里 `w:style` 且 `styleId` 非空的个数。没有 styles 部件则为 `0`。
  根上不挂 `styles`，除非给了 `scope.styles`。页眉页脚容器不出现 `styleCount`。
- `outline` 始终出现，只含主文档标题段。收入满足下面任一条件的段落：`w:pStyle/@w:val` 为 `Heading1`–`Heading9`（大小写不敏感）或 `标题1`–`标题9`；或者 `w:outlineLvl/@w:val` 为 `0`–`8`。
  树上 `level` 从 1 到 9。`Heading1` / `标题1` / `outlineLvl=0` 都是 `level: 1`。不是标题的段落不进 `outline`。页眉页脚段落不进 `outline`。
- `headers`、`footers` 始终出现。没有引用则为空数组。容器字段顺序：`id`、`type`、`sections`、`paraCount`、`pictureCount`、`textBoxCount`、
  `connectorCount`、`shapeCount`、`inkCanvasCount`、`chartCount`、`smartArtCount`、`oleObjectCount`、`model3dCount`、`watermarkCount`、`tableCount`、`revisionCount`、
  `hyperlinkCount`、`bookmarkCount`、`editRangeCount`、`fieldCount`、`mathCount`、`rubyCount`、`contentControlCount`，
  有图名单时再跟 `pictures`，有框名单时再跟 `textBoxes`，有连接线名单时再跟 `connectors`，有形状名单时再跟 `shapes`，有墨迹画布名单时再跟 `inkCanvases`，有图表名单时再跟 `charts`，有 SmartArt 名单时再跟 `smartArts`，
  有 OLE 名单时再跟 `oleObjects`，有 3D 模型名单时再跟 `model3ds`，有表名单时再跟 `tables`，有修订名单时再跟 `revisions`，有超链接名单时再跟 `hyperlinks`，有书签名单时再跟 `bookmarks`，
  有例外编辑区名单时再跟 `editRanges`，有域名单时再跟 `fields`，有公式名单时再跟 `maths`，有拼音名单时再跟 `rubies`，有内容控件名单时再跟 `contentControls`，有段落时再跟 `paragraphs`。
  `sections` 是从 1 起的节号数组，按主文档里 `w:sectPr` 的文档顺序。容器上的 `paraCount` 是该部件内 `w:p` 的个数（边界与正文相同）。
  `pictureCount` 始终出现，是该部件内被收入的 DrawingML 图片个数（不含框内图）。`textBoxCount` 始终出现，是该部件内被收入的文本框个数。
  `connectorCount` 始终出现，是该部件内被收入的连接线个数。`shapeCount` 始终出现，是该部件内被收入的无框形状个数。`inkCanvasCount` 始终出现，是该部件内被收入的墨迹画布个数。
  `chartCount` 始终出现，是该部件内被收入的图表个数（不含框内图表）。
  `smartArtCount` 始终出现，是该部件内被收入的 SmartArt 个数（不含框内图示）。`oleObjectCount` 始终出现，是该部件内被收入的嵌入对象个数（不含框内）。
  `model3dCount` 始终出现，是该部件内被收入的 3D 模型个数。
  `watermarkCount` 始终出现，是该部件内被收入的 DrawingML 水印个数（页脚恒为 `0`）。`tableCount` 始终出现，是该部件内被收入的 `w:tbl` 个数（含嵌套表，不含框内表）。
  `revisionCount` 始终出现，是该部件内被收入的修订节点个数（不含框内修订）。`hyperlinkCount` 始终出现，是该部件内被收入的 `w:hyperlink` 个数（不含框内链）。
  `bookmarkCount` 始终出现，是该部件内被收入的书签个数（不含框内书签）。`editRangeCount` 始终出现，是该部件内被收入的 `w:permStart` 个数。`fieldCount` 始终出现，是该部件内被收入的域个数（不含框内域）。
  `mathCount` 始终出现，是该部件内被收入的顶层 `m:oMath` 个数（不含框内公式）。`rubyCount` 始终出现，是该部件内被收入的 `w:ruby` 个数（不含框内拼音）。`contentControlCount` 始终出现，是该部件内被收入的内容控件个数（不含框内控件）。
- 文档根上的 `sections` 始终出现。每个 `w:sectPr` 一项，字段顺序：`id`、`index`、`headerDistance`、`footerDistance`、`top`、`bottom`、`left`、
  `right`、`gutter`、`pageWidth`、`pageHeight`、`orient`、`columns`、`vAlign`、`lineNumbering`、`docGrid`、`rtlGutter`、
  `headerLink`、`footerLink`、`pageBorders`、`formProt`。
- `headerDistance` / `footerDistance` 取该节 `w:pgMar` 的 `w:header` / `w:footer`（§17.6.11），单位 twip，JSON 非负整数。
  `left` / `right` / `gutter` 同属 `w:pgMar`，同样非负。`top` / `bottom` 是 `ST_SignedTwipsMeasure`，可为负整数。
  缺 `w:pgMar` 或缺对应属性则为 `null`，不猜默认值。属性存在但不是整数，或本该非负却为负，报 `XmlBroken`。
- `pageWidth` / `pageHeight` 取该节 `w:pgSz` 的 `w` / `h`（§17.6.13），单位 twip，非负整数。
  `orient` 取 `w:orient`，值为 `portrait` 或 `landscape`。缺 `w:pgSz` 或缺对应属性则为 `null`；规范说省略 `orient` 视为纵向，树上仍给 `null`，不猜。
  属性存在但不是非负整数，或 `orient` 不是这两个词，报 `XmlBroken`。不读写 `w:code`。改 `orient` 时不自动对调宽高。
- `columns` 取 `w:cols`（§17.6.4）。没有该元素则为 `null`。有则对象字段顺序：`num`、`space`、`equalWidth`、`sep`、`cols`。
  `num` / `space` 缺则为 `null`（`space` 单位 twip，非负）。
  `equalWidth` / `sep` 取 OnOff：`1` / `true` / `on` 为 `true`，`0` / `false` / `off` 为 `false`，缺属性为 `null`。
  `cols` 是子元素 `w:col` 的数组，按文档顺序；没有子元素则为 `[]`。每项字段顺序：`width`、`space`，对应 `w:w` / `w:space`，缺则为 `null`。不等宽时列数以子元素为准，树上仍原样给出 `num`。
- `vAlign` 取 `w:vAlign/@w:val`（§17.6.23），值为 `top` / `center` / `both` / `bottom`。没有元素则为 `null`。有元素却没有 `val` 或不认识，报 `XmlBroken`。
- `lineNumbering` 取 `w:lnNumType`（§17.6.8）。没有该元素则为 `null`。有则对象字段顺序：`countBy`、`start`、`distance`、`restart`。
  前三项缺则为 `null`（`distance` 单位 twip，非负；`countBy` / `start` 为整数）。
  `restart` 为 `newPage` / `newSection` / `continuous`，缺则为 `null`。不认识的 `restart` 或不是整数的数字属性报 `XmlBroken`。
- `docGrid` 取 `w:docGrid`（§17.6.5）。没有该元素则为 `null`。有则对象字段顺序：`type`、`linePitch`、`charSpace`。
  `type` 为 `default` / `lines` / `linesAndChars` / `snapToChars`，缺则为 `null`。`linePitch` / `charSpace` 为整数，缺则为 `null`。
  不认识的 `type` 或不是整数报 `XmlBroken`。
- `rtlGutter` 取本节 `w:rtlGutter`（§17.6.16）：没有元素为 `null`；有且 OnOff 为开为 `true`，为关为 `false`。
- 文档根上的 `mirrorMargins` / `gutterAtTop` 始终出现，
  取 `settings.xml` 的 `w:mirrorMargins` / `w:gutterAtTop`（§17.15.1.64、§17.15.1.50）。
  没有 settings 部件、或没有对应元素，则为 `null`。有则按 OnOff 写成布尔。这两项是整篇的，不出现在节节点上。
- `headerLink` / `footerLink` 始终出现，键顺序为 `default`、`even`、`first`。
  每个键的取值：`true` 表示该类型对本节生效，且本节 `sectPr` 没有对应引用（继承前节；前节也没有部件、规范要求空白时仍是 `true`）；`false` 表示该类型生效，且本节有显式引用；`null` 表示该类型对本节不生效。
  `even` 是否生效看整篇 `w:evenAndOddHeaders`；`first` 是否生效看本节 `w:titlePg`。第一节的 `default` 只能是 `false`（没有前节可链）。
  第一节生效的 `even` / `first` 若无引用也是 `false`（空白，不是链接）。两节显式指向同一部件时两边都是 `false`。`headers` / `footers` 仍按部件去重；断开链接后会出现新的容器 id。
- 省略 `scope`、或没有 `from`/`maxParas` 时，没有正文 `paragraphs`、`from`、`truncated`。
- 给了 `from` 和 `maxParas` 时，正文 `paragraphs` 按文档顺序切片。范围内段落数超过 `maxParas` 则 `truncated` 为 `true`。
  切片里每段始终有 `pictureCount`、`textBoxCount`、`connectorCount`、`shapeCount`、`inkCanvasCount`、`model3dCount` 与 `runCount`。
  段落字段顺序：`id`、`index`、`style`、`level`（标题才有）、该段自己的 `w:pPr/w:numPr` 且 `numId` 非 0 时再跟 `num`、`ilvl`、然后始终跟 `align`、`indent`、
  `spacing`、`eastAsia`、`borders`、`shd`、`tabs`、`text`、`stable`、`pictureCount`、`textBoxCount`、`connectorCount`、`shapeCount`、`inkCanvasCount`、`model3dCount`、`runCount`，
  有图名单时再跟 `pictures`，有框名单时再跟 `textBoxes`，有连接线名单时再跟 `connectors`，有形状名单时再跟 `shapes`，有墨迹画布名单时再跟 `inkCanvases`，有 3D 模型名单时再跟 `model3ds`，该段有脚注或尾注引用时再跟 `footnoteRef` / `endnoteRef`（注 id 数组，
  按引用在该段内的文档顺序），该段落落在批注范围内时再跟 `commentIds`（批注 id 数组，按范围开始在文档中的顺序；同一段重叠多条都列出），给了 `scope.runs` 时再跟 `runs`。
  没有引用或范围则没有这些键。已展开的页眉页脚、框内、注内段落同样可以带 `commentIds`。不解析 `styles.xml` 继承的编号：样式上有 `numPr`、段上没有的，树上没有 `num` / `ilvl`。
  缺 `ilvl` 当作 `0`。`num` 是 `num:{numId}`。
- `scope.headerFooter` 为 `true` 时，每个页眉、页脚节点带 `paragraphs`；否则容器上没有该字段。
  页眉页脚段落节点不带 `pictureCount` / `pictures` / `textBoxCount` / `textBoxes` / `connectorCount` / `connectors`
   / `shapeCount` / `shapes`
   / `inkCanvasCount` / `inkCanvases`
   / `model3dCount` / `model3ds`
   / `chartCount` / `charts` / `smartArtCount` / `smartArts` / `oleObjectCount` / `oleObjects`，图、框、连接线、无框形状、墨迹画布、3D 模型、图表、SmartArt 与嵌入对象仍挂在容器上。
  页眉页脚段落仍有 `align`、`indent`、`spacing`、`eastAsia`、`borders`、`shd`、`tabs`、`runCount`；给了 `scope.runs` 时再带 `runs`。
- `scope.pictures` 为 `true` 时，每个页眉、页脚节点带 `pictures`；若同时给了 `from` 与 `maxParas`，切片里每段也带 `pictures`。否则这些节点上都没有该字段。不把媒体字节放进树。
  每张图的字段顺序：`id`、`docPrId`、`name`、`kind`、`cx`、`cy`、`wrap`、`stable`。
  有 `asvg:svgBlip` 时再跟 `contentType`，值为 `image/svg+xml`。
  没有 `svgBlip` 时，按 image 关系 Target 扩展名：
  `.emf` 为 `image/x-emf`，`.wmf` 为 `image/x-wmf`，`.tif` / `.tiff` 为 `image/tiff`。
  PNG / JPEG / GIF / BMP 没有该字段。不读媒体字节，不把原图放进树。
  最后始终跟 `hyperlink`（`null` 或对象，字段顺序 `kind`、`rId`、`url`、`anchor`、`tooltip`），见上文 DrawingML 链。
- `scope.textBoxes` 为 `true` 时，每个页眉、页脚节点带 `textBoxes`；若同时给了 `from` 与 `maxParas`，切片里每段也带 `textBoxes`。否则这些节点上都没有该字段。
  只给 `textBoxes`、不给 `from` / `maxParas`：只展开页眉页脚容器上的框。
  每个框的字段顺序：`id`、`docPrId`、`name`、`kind`、`cx`、`cy`、`wrap`、`textAnchor`、`link`、
  `paraCount`、`pictureCount`、`chartCount`、`smartArtCount`、`tableCount`、`hyperlinkCount`、`bookmarkCount`、`fieldCount`、`mathCount`、`rubyCount`、`contentControlCount`、`stable`、`hyperlink`，
  有框内段名单时再跟 `paragraphs`，有框内图名单时再跟 `pictures`，给了 `scope.charts` 时再跟 `charts`，给了 `scope.smartArt` 时再跟 `smartArts`，给了 `scope.tables` 时再跟 `tables`，
  给了 `scope.hyperlinks` 时再跟 `hyperlinks`，给了 `scope.bookmarks` 时再跟 `bookmarks`，给了 `scope.fields` 时再跟 `fields`，
  给了 `scope.maths` 时再跟 `maths`，给了 `scope.rubies` 时再跟 `rubies`，给了 `scope.contentControls` 时再跟 `contentControls`。
  `hyperlink` 为 `null` 或对象（字段顺序 `kind`、`rId`、`url`、`anchor`、`tooltip`），见上文 DrawingML 链。
  `textAnchor` 取 `wps:bodyPr/@anchor`：`t` / `ctr` / `b` / `just` / `dist`。缺属性为 `null`。不认识报 `XmlBroken`。
  `chartCount` / `smartArtCount` 始终出现，只计该框 `w:txbxContent` 内、再遇嵌套 `w:txbxContent` 停住之后的图表与 SmartArt。框内图表与 SmartArt 不挂到文档根，也不挂到宿主段落。
  `tableCount` 始终出现，只计该框 `w:txbxContent` 内、再遇嵌套 `w:txbxContent` 停住之后的 `w:tbl`（含嵌套表）。
  `hyperlinkCount` / `bookmarkCount` / `fieldCount` / `mathCount` / `rubyCount` / `contentControlCount` 始终出现，只计该框内容，规则与正文同一套边界。
  注内 / 批注内的框这几项计数为 `0`，也不带对应名单。框内这些对象不挂到文档根，也不挂到宿主段落。
- `scope.connectors` 为 `true` 时，每个页眉、页脚节点带 `connectors`；若同时给了 `from` 与 `maxParas`，切片里每段也带 `connectors`。否则这些节点上都没有该字段。
  只给 `connectors`、不给 `from` / `maxParas`：只展开页眉页脚容器上的连接线。
  每条字段顺序：`id`、`docPrId`、`name`、`kind`、`cx`、`cy`、`wrap`、`prst`、`stable`、`hyperlink`。
  `kind` / `cx` / `cy` / `wrap` / `docPrId` / `name` / `stable` 规则与图相同。`prst` 取 `a:prstGeom/@prst`，没有则为 `null`。不报 `text`。组合里的连接线要展开。
  `hyperlink` 为 `null` 或对象，见上文 DrawingML 链。
- `scope.shapes` 为 `true` 时，每个页眉、页脚节点带 `shapes`；若同时给了 `from` 与 `maxParas`，切片里每段也带 `shapes`。否则这些节点上都没有该字段。
  只给 `shapes`、不给 `from` / `maxParas`：只展开页眉页脚容器上的无框形状。
  每条字段顺序：`id`、`docPrId`、`name`、`kind`、`cx`、`cy`、`wrap`、`prst`、`stable`、`hyperlink`。
  `kind` / `cx` / `cy` / `wrap` / `docPrId` / `name` / `stable` 规则与图相同。`prst` 取 `a:prstGeom/@prst`，没有则为 `null`。不报 `text`。不报 fill / line。
  组合里符合条件的子形状要展开。注内、批注内、框内不列出。
  `hyperlink` 为 `null` 或对象，见上文 DrawingML 链。
- `scope.inkCanvases` 为 `true` 时，每个页眉、页脚节点带 `inkCanvases`；若同时给了 `from` 与 `maxParas`，切片里每段也带 `inkCanvases`。否则这些节点上都没有该字段。
  只给 `inkCanvases`、不给 `from` / `maxParas`：只展开页眉页脚容器上的墨迹画布。
  每条字段顺序：`id`、`name`、`kind`、`cx`、`cy`、`wrap`、`contentType`、`stable`、`hyperlink`。
  `kind` / `cx` / `cy` / `wrap` / `name` / `stable` 规则与图相同。`contentType` 恒为 `application/inkml+xml`。不把 InkML 原文放进树。
  组合里符合条件的 contentPart 要展开。注内、批注内、框内不列出。
  `hyperlink` 为 `null` 或对象，见上文 DrawingML 链。
- `scope.model3ds` 为 `true` 时，每个页眉、页脚节点带 `model3ds`；若同时给了 `from` 与 `maxParas`，切片里每段也带 `model3ds`。否则这些节点上都没有该字段。
  只给 `model3ds`、不给 `from` / `maxParas`：只展开页眉页脚容器上的 3D 模型。
  每条字段顺序：`id`、`name`、`kind`、`cx`、`cy`、`wrap`、`contentType`、`stable`、`camera`、`rot`、`lightRig`、`hyperlink`。
  `kind` / `cx` / `cy` / `wrap` / `name` / `stable` 规则与图相同。`contentType` 恒为 `model/gltf.binary`。不把 GLB 放进树。
  `camera` / `rot` / `lightRig` 规则与上文 3D 模型 id 那节相同。不把 `am3d:ambientLight` / `ptLight` 放进树。
  组合里符合条件的 graphicFrame / `wp:inline` 要展开。注内、批注内、框内不列出。
  `hyperlink` 为 `null` 或对象，见上文 DrawingML 链。
- `scope.notes` 为 `true` 时，根上带 `footnotes`、`endnotes`、`noteSeparators`。没有用户注时前两个为空数组；没有分隔符时 `noteSeparators` 为空数组。
  用户注按部件内文档顺序。每条用户注字段顺序：`id`、`noteId`、`type`、`host`、`paraCount`、`pictureCount`、`textBoxCount`、`connectorCount`、`chartCount`、`smartArtCount`，
  有图名单时再跟 `pictures`，有框名单时再跟 `textBoxes`，有连接线名单时再跟 `connectors`，有图表名单时再跟 `charts`，有 SmartArt 名单时再跟 `smartArts`，然后 `paragraphs`。
  `noteId` 是 `w:id` 原文。`type` 恒为 `normal`。
  `host` 是引用所在段落 id 的数组：在主文档、页眉、页脚和文本框里找 `w:footnoteReference` / `w:endnoteReference`，按文档顺序，id 规则与该段落本身相同。
  一条注被引用多次则 `host` 有多项；没有任何引用则为 `[]`，注仍列出。
  `paraCount` / `pictureCount` / `textBoxCount` / `connectorCount` / `chartCount` / `smartArtCount` 只计该注元素内、
  遇 `w:txbxContent` 停住之后的段落、图、框、连接线、图表与 SmartArt，规则与正文相同。
  框内的图、图表与 SmartArt 不算进该注自己的这几项计数。注内段落字段顺序与正文段落相同，id 带注前缀。注内图、框、连接线、图表与 SmartArt 的字段顺序与宿主相同，id 带注前缀。
- `noteSeparators` 每项字段顺序：`id`、`kind`、`type`、`noteId`。`kind` 为 `footnote` 或 `endnote`。
  `type` 为 `separator`、`continuationSeparator` 或 `continuationNotice`。对 `noteSeparators` 里的 id 发 apply 报 `InvalidId`。
- `scope.comments` 为 `true` 时，根上带 `comments`。没有批注时为空数组。按 `comments.xml` 里 `w:comment` 的文档顺序。
  每条字段顺序：`id`、`commentId`、`author`、`initials`、`date`、`done`、`parent`、`kind`、`start`、`end`、`paraCount`、`pictureCount`、
  `textBoxCount`、`connectorCount`、`chartCount`、`smartArtCount`，
  有图名单时再跟 `pictures`，有框名单时再跟 `textBoxes`，有连接线名单时再跟 `connectors`，有图表名单时再跟 `charts`，有 SmartArt 名单时再跟 `smartArts`，然后 `paragraphs`。
  `commentId` 是 `w:id` 原文。`author` / `initials` / `date` 取对应属性原文，缺则为 `null`。
  `done` 取 `commentEx/@done`：`1` / `true` / `on` 为 `true`，其余或没有 `commentEx` 为 `false`。
  `parent` 用 `paraIdParent` 对上父批注最后一段的 `w14:paraId`，再写成 `comment:{id}`；对不上或没有则为 `null`。根批注的 `parent` 为 `null`。
  `kind` 为 `normal`；批注内出现 `inkml:ink` 或等价墨迹标记则为 `ink`。`start` / `end` 是范围落在的段落 id（正文、页眉、页脚、框内或注内，规则与该段落本身相同）。
  从配对的 `commentRangeStart` 走到 `commentRangeEnd`，路径上经过的每个 `w:p` 都算在范围内；`start` 是第一个，`end` 是最后一个。
  回复有自己的范围标记时按自己的标记算；没有时沿用线程根的 `start` / `end`，并出现在那些段的 `commentIds` 里。找不到对应段（残缺标记）则该侧为 `null`，该批注仍列出，不报 `XmlBroken`。
  `paraCount` / `pictureCount` / `textBoxCount` / `connectorCount` / `chartCount` / `smartArtCount` 只计该 `w:comment` 内、
  遇 `w:txbxContent` 停住之后的段落、图、框、连接线、图表与 SmartArt，规则与正文相同。
  框内的图、图表与 SmartArt 不算进该批注自己的这几项计数。批注内段落字段顺序与正文段落相同，id 带批注前缀。批注内图、框、连接线、图表与 SmartArt 的字段顺序与宿主相同，id 带批注前缀。
- `scope.charts` 为 `true` 时，根上带 `charts`（正文），每个页眉、页脚容器带 `charts`；
  若同时给了 `scope.notes` / `scope.comments`，每条用户注或批注也带 `charts`（只含该注或批注段落上的，不含框内）。
  没有图表时为空数组。正文按主文档段落文档顺序，同一段内按 drawing 顺序。
  每张字段顺序：`id`、`docPrId`、`name`、`kind`、`cx`、`cy`、`wrap`、`type`，`type` 为 `ofPie` 时再跟 `ofPieType`，
  `type` 为 `stock` 时再跟 `stockKind`，`type` 为 `surface` 时再跟 `surfaceKind`，
  `type` 为 `chartex` 时再跟 `layoutId`，然后 `title`、`catAxisTitle`、`valAxisTitle`、`series`、`legend`、`dataLabels`、`flags`、`stable`、`hyperlink`。
  `kind` / `cx` / `cy` / `wrap` / `docPrId` / `name` / `stable` 规则与图相同。
  `hyperlink` 为 `null` 或对象，见上文 DrawingML 链。
  `type` 为 `bar`、`col`、`line`、`pie`、`area`、`scatter`、`doughnut`、`radar`、`bubble`、`combo`、`stock`、`surface`、`ofPie`、`chartex` 或 `other`。
  `barChart` / `bar3DChart` 看 `c:barDir`：`bar` 为 `bar`，`col` 或缺省为 `col`。
  `lineChart` / `line3DChart` 为 `line`，`pieChart` / `pie3DChart` 为 `pie`，`areaChart` / `area3DChart` 为 `area`，
  `scatterChart` 为 `scatter`，`doughnutChart` 为 `doughnut`，`radarChart` 为 `radar`，`bubbleChart` 为 `bubble`。
  `stockChart` 为 `stock`，`surfaceChart` / `surface3DChart` 为 `surface`，`ofPieChart` 为 `ofPie`。
  `type` 为 `ofPie` 时 `ofPieType` 取 `c:ofPieType/@val`（`pie` / `bar`），缺或对不上则当作 `pie`。
  `type` 为 `stock` 时 `stockKind` 为 `ohlc`（四条 `c:ser` 或有 `c:upDownBars`）或 `hlc`。
  带成交量的柱＋股价组合仍是 `combo`，不要报 `stockKind`。
  `type` 为 `surface` 时 `stockKind` 不出现；`surfaceKind` 为 `wireframe`（`surface3DChart` 且 `wireframe=1`）、
  `contourWire`（`surfaceChart` 且 `wireframe=1`）、`contour`（`surfaceChart`）或 `surface3d`。
  `type` 为 `chartex` 时 `layoutId` 取部件里第一条 `cx:series/@layoutId` 原文；若任一条为 `paretoLine` 则报 `pareto`。
  没有该属性则没有这个键。
  `plotArea` 里出现两种及以上系列包裹（例如 `barChart` 加 `lineChart`）则为 `combo`。其余（chartex 以外的扩展）为 `other`。
  `title` 取 `c:title` 可见文本（`c:tx` 的 `c:rich` 里 `a:t` 拼接，或 `c:strRef` 的 cache）；
  没有标题、或 `c:autoTitleDeleted/@val` 为真则为 `null`。
  `catAxisTitle` / `valAxisTitle` 取主分类轴、主数值轴的 `c:title`，规则与图表标题相同；没有则为 `null`。散点图的横轴当分类轴。`series` 按 `c:ser` 顺序。
  每条字段顺序：`id`、`index`、`name`、`categories`、`values`，气泡图再跟 `sizes`，再跟 `axis`，有 `c:trendline` 时再跟 `trendlines`，
  有 `c:errBars` 时再跟 `errorBars`，cache 与 embedding 不一致时再跟 `cacheStale: true`。
  `index` 是从 0 起的出现序号。`axis` 为 `primary` 或 `secondary`：该系列所在包裹的 `c:axId` 指向带 `c:overlay` 为真的轴，
  或数值轴 id 不是主数值轴时为 `secondary`，否则 `primary`。
  饼、圆环、复合饼没有轴时为 `primary`。`trendlines` 按该 `c:ser` 里 `c:trendline` 顺序。
  每条字段顺序：`id`、`index`、`type`，有 `c:order` 再跟 `order`，有 `c:period` 再跟 `period`，再跟 `displayRSquared`、`displayEquation`。
  `id` 为 `{系列 id}/trendline:{k}`，`index` 与 `{k}` 从 1 起。
  `type` 取 `c:trendlineType/@val`，只认 `linear`、`exp`、`log`、`poly`、`power`、`movingAvg`；缺或对不上则该条仍列出，`type` 为原文。
  `displayRSquared` / `displayEquation` 看 `c:dispRSqr` / `c:dispEq`，缺省 `false`。`errorBars` 按该系列 `c:errBars` 顺序。
  每条字段顺序：`id`、`index`、`dir`、`barType`、`valType`，有 `c:val` 且能解析成数字再跟 `value`，
  `valType` 为 `cust` 时再跟 `plus`、`minus`（取 `c:plus` / `c:minus` 的 `numLit` 各点，全是数字则 JSON 数字）。
  `id` 为 `{系列 id}/errorBars:{k}`，`index` 与 `{k}` 从 1 起。`dir` 取 `c:errDir/@val`（`x` / `y`），缺省 `y`。
  `barType` 取 `c:errBarType/@val`（`both` / `plus` / `minus`），缺省 `both`。
  `valType` 取 `c:errValType/@val`（`stdErr` / `stdDev` / `fixedVal` / `percentage` / `cust` / `lCaps`），缺省 `stdErr`。
  `name` 取系列 `c:tx` 的 cache 或字面量，缺则为 `null`。
  `categories` / `values` 取 `c:cat` / `c:val`（散点图、气泡图为 `c:xVal` / `c:yVal`）的 cache 或字面量；没有 cache 再读 embedding 里公式指向的单元格。
  气泡图的 `sizes` 取 `c:bubbleSize`，规则与 `values` 相同；没有则为 `[]`。非气泡系列不出现 `sizes`。全是数字则 JSON 数字，否则字符串。没有数据则为 `[]`。
  cache 与 embedding 都有且对不上时以 embedding 为准，并标 `cacheStale`。`legend` 为 `{ "show": bool, "pos": ... }`。
  没有 `c:legend` 则 `show` 为 `false`、`pos` 为 `null`；
  有则 `show` 为 `true`，`pos` 取 `c:legendPos/@val`（`b` / `t` / `l` / `r` / `tr`），缺属性则为 `null`。
  `dataLabels` 为 `{ "show": bool }`：`plotArea` 或任一系列带 `c:dLbls`，
  且其中至少一个 `c:showVal` / `c:showCatName` / `c:showSerName` / `c:showPercent` 为 `1` / `true` / `on` 时为 `true`，否则 `false`。
  `flags` 是字符串数组，可能含 `secondaryAxis`（出现 `c:overlay` 为 1 的轴，
  或系列 `c:axId` 指向次轴）、`trendline`（`c:trendline`）、`errorBars`（`c:errBars`）、`pivot`（`c:pivotSource` 或等价透视引用）。
  没有这些则为 `[]`。不把图表画成图，不读图表的渲染缓存图。xlsx / pptx 的 `scope` 里出现 `charts` 报 `InvalidField`。
- `scope.smartArt` 为 `true` 时，根上带 `smartArts`（正文），每个页眉、页脚容器带 `smartArts`；
  若同时给了 `scope.notes` / `scope.comments`，每条用户注或批注也带 `smartArts`（只含该注或批注段落上的，不含框内）。
  没有图示时为空数组。正文按主文档段落文档顺序，同一段内按 drawing 顺序。
  每张字段顺序：`id`、`docPrId`、`name`、`kind`、`cx`、`cy`、`wrap`、`layout`、`colors`、`style`、`points`、`stable`、`hyperlink`。
  `kind` / `cx` / `cy` / `wrap` / `docPrId` / `name` / `stable` 规则与图相同。
  `hyperlink` 为 `null` 或对象，见上文 DrawingML 链。
  `layout` 取 layout 部件根元素 `dgm:layoutDef/@uniqueId` 的原文；没有该部件或没有该属性则为 `null`。
  `colors` 取 colors 部件 `dgm:colorsDef/@uniqueId`；`style` 取 quickStyle 部件 `dgm:styleDef/@uniqueId`；缺则为 `null`。
  `points` 按 data 部件 `ptLst` 文档顺序，只含可见点（判定见上文 SmartArt id 那节）。每条字段顺序：`id`、`modelId`、`role`、`text`、`parent`。
  `modelId` 是 `dgm:pt/@modelId` 原文。`role` 为 `node` 或 `asst`（`type` 缺省当作 `node`）。
  `text` 取该点 `dgm:t` 里 `a:t` 按文档顺序拼接；`a:br` 变成 `\n`；没有 `dgm:t` 则为 `""`。`parent` 规则见上文。
  缺少 `r:dm` 或 data 部件解析失败时该图示仍列出，`points` 为 `[]`，`layout` / `colors` / `style` 能读则读、不能则为 `null`，不报 `XmlBroken`。不读 Fallback 位图，不把图示画成图。
- `scope.tables` 为 `true` 时，根上带 `tables`（正文），每个页眉、页脚容器带 `tables`。没有表时为空数组。按该故事里 `w:tbl` 的文档顺序，嵌套表也列出。
  每张字段顺序：`id`、`index`、`rowCount`、`colCount`、`nested`、`style`、`look`、`pos`、`align`、`indent`、`width`、`layout`、`colWidths`、`cellMar`、`borders`、`rows`。
  `index` 与 id 里的 `{n}` 相同。`rowCount` 是 `w:tr` 个数。
  `colCount` 优先取 `tblGrid` 的 `gridCol` 个数；没有 `tblGrid` 则取第一行各格 `gridSpan` 之和（缺省 1）。
  `nested` 在该表的直接父元素是 `w:tc` 时为 `true`，否则为 `false`。
  `style` 取 `tblPr/tblStyle/@val` 原文，没有则为 `null`。
  `look` 取 `tblLook`。没有该元素则为 `null`。有则对象字段顺序：`firstRow`、`lastRow`、`firstColumn`、`lastColumn`、`bandedRows`、`bandedColumns`。
  六个开关优先读对应属性的 OnOff；缺属性时按 `val` 的十六进制位：`0x20` 首行、`0x40` 末行、`0x80` 首列、`0x100` 末列、`0x200` 为 `noHBand`、`0x400` 为 `noVBand`。
  `bandedRows` 是 `noHBand` 的反相，`bandedColumns` 是 `noVBand` 的反相。
  `pos` 取 `tblpPr`。没有该元素则为 `null`。有则对象字段顺序：`horzAnchor`、`vertAnchor`、`x`、`y`、`leftFromText`、`rightFromText`、`topFromText`、`bottomFromText`。
  锚点取属性原文。`x` / `y` 取 `tblpX` / `tblpY`，能解析成整数则写整数，否则为 `null`。其余边距同样。单位 twip。不把 `tblOverlap` 摊进树。
  `align` 取 `tblPr/jc/@val` 原文，没有则为 `null`。
  `indent` 取 `tblInd/@w`，能解析成整数则写整数，否则为 `null`。单位 twip。
  `width` 取 `tblW`。没有该元素则为 `null`。有则对象字段顺序：`type`、`w`。
  `type` 取 `@w:type` 原文，缺则为 `null`。`w` 取 `@w:w`，能解析成整数则写整数，否则为 `null`。
  `layout` 取 `tblLayout/@type` 原文，没有则为 `null`。
  `colWidths` 按 `tblGrid/gridCol` 文档顺序。每项取 `@w:w`，能解析成整数则写整数，否则为 `null`。没有 `tblGrid` 则为 `[]`。
  `cellMar` 取表级 `tblCellMar`。没有该元素则为 `null`。有则对象字段顺序：`top`、`left`、`bottom`、`right`。各边取 `w:w`，能解析成整数则写整数，否则为 `null`。
  `borders` 取表级 `tblBorders`。没有该元素则为 `null`。有则对象字段顺序：`top`、`left`、`bottom`、`right`、`insideH`、`insideV`。
  每边与页面边框同一套：`val`、`sz`、`space`、`color`。缺该边则为 `null`。
  `left` 先取 `w:left`，没有则取 `w:start`。`right` 先取 `w:right`，没有则取 `w:end`。
  不要把样式里的边框摊进表节点。`CT_TblBorders` 没有斜线；格斜线仍在格上 `tl2br` / `tr2bl`。
  `rows` 按 `w:tr` 顺序。每行字段顺序：`id`、`index`、`height`、`cells`。
  `height` 取 `trPr/trHeight`。没有则为 `null`。有则对象字段顺序：`h`、`rule`。
  `h` 取 `@w:val`，能解析成整数则写整数，否则为 `null`。`rule` 取 `@w:hRule` 原文，缺则为 `null`。单位 twip。
  `cells` 按该行 `w:tc` 顺序，不是按空网格补格。
  每格字段顺序：`id`、`row`、`col`、`gridSpan`、`vMerge`、`vAlign`、`noWrap`、`textDirection`、`fitText`、`hideMark`、`headers`、`fill`、`cellMar`、`borders`、`tl2br`、`tr2bl`、`paragraphs`。
  `row` / `col` 与 id 里的数字相同。
  `gridSpan` 为整数，缺元素则为 `1`。`vMerge` 为 `restart`、`continue` 或 `null`。
  `vAlign` 取 `tcPr/w:vAlign/@w:val`：`top` / `center` / `both` / `bottom`。没有该元素则为 `null`。不认识报 `XmlBroken`。
  `noWrap` 取 `tcPr/w:noWrap` 的 OnOff。没有该元素则为 `null`。有元素却没有 `val` 当作 `true`。
  `textDirection` 取 `tcPr/w:textDirection/@w:val`：`lrTb` / `tbRl` / `btLr` / `lrTbV` / `tbRlV` / `tbLrV`。没有该元素则为 `null`。不认识报 `XmlBroken`。
  `fitText` 取 `tcPr/w:tcFitText` 的 OnOff。没有该元素则为 `null`。有元素却没有 `val` 当作 `true`。
  `hideMark` 取 `tcPr/w:hideMark` 的 OnOff。没有该元素则为 `null`。有元素却没有 `val` 当作 `true`。
  `headers` 取 `tcPr/w:headers/w:header` 文本数组。没有 `w:headers` 则为 `[]`。
  `fill` 取 `tcPr/shd/@fill` 原文，没有 `shd` 或没有 `fill` 则为 `null`。
  格上 `cellMar` 取 `tcMar`，规则与表级相同。
  格上 `borders` 取 `tcBorders` 的正交四边与格内内外框。没有这些边则为 `null`。有则对象字段顺序：`top`、`left`、`bottom`、`right`、`insideH`、`insideV`。
  每边与页面边框同一套：`val`、`sz`、`space`、`color`。缺该边则为 `null`。
  `left` 先取 `w:left`，没有则取 `w:start`。`right` 先取 `w:right`，没有则取 `w:end`。
  斜线不进这个对象。
  `tl2br` / `tr2bl` 取 `tcBorders` 对应子元素。没有则为 `null`。有则对象字段顺序：`val`、`sz`、`color`。
  `val` / `color` 取属性原文，缺则为 `null`。`sz` 能解析成整数则写整数，否则为 `null`。
  `paragraphs` 是该格内、遇嵌套 `w:tbl` 停住之后的 `w:p`（嵌套表自己的段挂在那张嵌套表上）。
  段落字段顺序与正文段落相同，id 仍是 `para:…` / `para:index:…`（页眉页脚带容器前缀），`index` 是该故事全部 `w:p` 的序号。
  给了 `scope.pictures` 时，这些段落同样带 `pictureCount` / `pictures`。xlsx 的 `scope` 里出现 `tables` 报 `InvalidField`。
  pptx 的 `scope.tables` 展开幻灯片 DrawingML 表，见 [pptx 响应](json-contract-pptx.md#响应)，与本文 `w:tbl` 不是同一棵名单。
  `gridSpan` 或 `vMerge/@val` 不是正整数 / 不是 `restart` 或 `continue` 时报 `XmlBroken`。
  `vAlign/@val` 不是 `top` / `center` / `both` / `bottom` 时报 `XmlBroken`。
  `textDirection/@val` 不是上述六值时报 `XmlBroken`。
  `noWrap` / `tcFitText` / `hideMark` 的 OnOff 不认识时报 `XmlBroken`。
  某行各格占的网格列之和对不上 `colCount` 时仍列出，结构 op 报 `NotImplemented`。
- `scope.revisions` 为 `true` 时，根上带 `revisions`（正文），每个页眉、页脚容器带 `revisions`。没有修订时为空数组。按该故事里被收入节点的文档顺序。
  每条字段顺序：`id`、`index`、`kind`、`xmlId`、`author`、`date`、`host`、`text`。`index` 与 id 里的 `{n}` 相同。
  `kind` 为 `ins`、`del`、`moveFrom`、`moveTo`、`paraMarkIns`、`paraMarkDel`、`rowIns`、`rowDel`、`rPrChange`、`pPrChange`、`tblPrChange`、
  `tblGridChange`、`tblPrExChange`、`tcPrChange`、`trPrChange` 或 `sectPrChange`。
  `xmlId` 是 `w:id` 原文，缺则为 `null`。`author` / `date` 取对应属性原文，缺则为 `null`。
  `host` 是所在段落 id（正文 `para:…`，页眉页脚带容器前缀）；修订不在任何 `w:p` 里则为 `null`（表行与表 / 节属性修订通常为 `null`）。
  `text`：run 级 `ins` / `moveFrom` / `moveTo` 为该节点内可见 `w:t` 拼接；run 级 `del` 为 `w:delText`（以及其中若有的 `w:t`）拼接；段落标记、表行与属性修订为 `null`。
  xlsx / pptx 的 `scope` 里出现 `revisions` 报 `InvalidField`。
- `scope.hyperlinks` 为 `true` 时，根上带 `hyperlinks`（正文），每个页眉、页脚容器带 `hyperlinks`。没有链时为空数组。按该故事里被收入节点的文档顺序。
  每条字段顺序：`id`、`index`、`kind`、`rId`、`url`、`anchor`、`tooltip`、`host`、`text`。`kind` 为 `url`（有 `r:id`）或 `anchor`（没有 `r:id`）。
  `rId` 是关系 id 原文，缺则为 `null`。`url` 是该关系的 `Target` 原文，缺关系则为 `null`。`anchor` 是 `w:anchor` 原文，缺则为 `null`。
  有 `r:id` 时树上仍可带 `anchor`，但规范说跳转时忽略它。`tooltip` 取 `w:tooltip`，缺则为 `null`。`host` 是所在段落 id；不在任何 `w:p` 里则为 `null`。
  `text` 为该节点内可见 `w:t` 拼接。HYPERLINK 域不进本名单。DrawingML 链不进本名单。
  图 / 文本框节点另有 `hyperlink`：`null` 或对象，字段顺序 `kind`、`rId`、`url`、`anchor`、`tooltip`。
  pptx 的 `scope.hyperlinks` 见 [pptx 响应](json-contract-pptx.md#响应)。
  xlsx 的 `scope.hyperlinks` 见 [xlsx 响应](json-contract-xlsx.md#响应)。
- `scope.bookmarks` 为 `true` 时，根上带 `bookmarks`（正文），每个页眉、页脚容器带 `bookmarks`。没有书签时为空数组。按该故事里被收入的 `w:bookmarkStart` 的文档顺序。
  每条字段顺序：`id`、`index`、`name`、`xmlId`、`start`、`end`。`name` 是 `w:name` 原文。`xmlId` 是 `w:id` 原文，缺则为 `null`。
  `start` / `end` 是起止所在段落 id；标记不在任何 `w:p` 里则为 `null`。xlsx / pptx 的 `scope` 里出现 `bookmarks` 报 `InvalidField`。
- `scope.editRanges` 为 `true` 时，根上带 `editRanges`（正文），每个页眉、页脚容器带 `editRanges`。没有例外区时为空数组。按该故事里被收入的 `w:permStart` 的文档顺序。
  每条字段顺序：`id`、`index`、`xmlId`、`edGrp`、`ed`、`host`、`text`。`id` 是 `perm:{n}` 或 `{容器}/perm:{n}`。
  `xmlId` 是 `w:id` 原文，缺则为 `null`。`edGrp` 取 `w:edGrp`，缺则为 `null`。`ed` 取 `w:ed`，缺则为 `null`。
  `host` 是起点所在段落 id。`text` 为配对 `permEnd` 之前的可见 `w:t` 拼接；不成对则为 `""`。
  框内 / 注内 / 批注内不编。xlsx / pptx 的 `scope` 里出现 `editRanges` 报 `InvalidField`。
- `scope.fields` 为 `true` 时，根上带 `fields`（正文），每个页眉、页脚容器带 `fields`。没有域时为空数组。按该故事里被收入域起点的文档顺序。
  每条字段顺序：`id`、`index`、`kind`、`code`、`instr`、`host`、`end`、`text`，有 `w:ffData` 时再跟 `form`。
  `kind` 为 `simple`（`w:fldSimple`）或 `complex`（配对 `w:fldChar`）。
  `instr` 是本层指令原文：简单域取 `w:instr`；复杂域拼接 begin 与 separate 之间（没有 separate 则到 end）本层的 `w:instrText`，不含 `w:delInstrText`，也不含嵌套域的指令。
  `code` 是 `instr` 去掉首尾空白后第一个词，按 ASCII 大写；公式域以 `=` 开头时 `code` 为 `"="`；没有词则为 `""`。
  `host` 是起点所在段落 id；`end` 是终点所在段落 id（简单域与 `host` 相同）；不在任何 `w:p` 里则为 `null`。
  `text` 是缓存结果的可见文本：简单域为元素内可见 `w:t`；复杂域为 separate 与 end 之间的可见 `w:t`（没有 separate 则为 `""`）。
  `form` 仅当 begin 的 `fldChar` 含 `w:ffData`：字段顺序 `type`（`text` / `checkBox` / `dropDown`）、`name`、`enabled`；复选再跟 `checked`；下拉再跟 `items`。
  xlsx 的 `scope` 里出现 `fields` 报 `InvalidField`。
  pptx 认 `scope.fields`，见 [json-contract-pptx.md](json-contract-pptx.md#inspect)。
- `scope.tocs` 为 `true` 时，根上带 `tocs`。没有目录时为空数组。按主文档里被收入的 TOC / TOA 域起点的文档顺序。
  每条字段顺序：`id`、`index`、`kind`、`field`、`instr`、`host`、`end`、`text`。`id` 是 `toc:{n}`。`kind` 为 `toc`、`tof` 或 `toa`。
  `field` 是对应的正文域 id（`field:{m}`）。`instr` / `host` / `end` / `text` 与该域在 `fields` 里的同名字段相同。
  xlsx / pptx 的 `scope` 里出现 `tocs` 报 `InvalidField`。
- `scope.captions` 为 `true` 时，根上带 `captions`。没有题注时为空数组。按主文档里被收入的顶层 SEQ 域起点的文档顺序。
  每条字段顺序：`id`、`index`、`field`、`instr`、`identifier`、`bookmark`、`host`、`end`、`text`。`id` 是 `caption:{n}`。`field` 是对应的正文域 id。
  `identifier` 是 SEQ 指令里的序列名（第一个参数），缺则为 `""`。`bookmark` 是包住该 SEQ 的书签名：优先最内层以 `_Ref` 开头的配对书签，否则任意包住该域的书签；没有则为 `null`。
  `instr` / `host` / `end` / `text` 与该域在 `fields` 里的同名字段相同。xlsx / pptx 的 `scope` 里出现 `captions` 报 `InvalidField`。
- `scope.crossRefs` 为 `true` 时，根上带 `crossRefs`。没有交叉引用时为空数组。按主文档里被收入的顶层 REF / PAGEREF / NOTEREF 域起点的文档顺序。
  每条字段顺序：`id`、`index`、`kind`、`field`、`instr`、`bookmark`、`host`、`end`、`text`。`id` 是 `xref:{n}`。
  `kind` 为 `ref`、`page` 或 `note`。`field` 是对应的正文域 id。`bookmark` 是指令里的书签参数，缺则为 `""`。
  xlsx / pptx 的 `scope` 里出现 `crossRefs` 报 `InvalidField`。
- `scope.indexes` 为 `true` 时，根上带 `indexes`。没有 INDEX 时为空数组。按主文档里被收入的顶层 INDEX 域起点的文档顺序。
  每条字段顺序：`id`、`index`、`field`、`instr`、`host`、`end`、`text`。`id` 是 `index:{n}`。`field` 是对应的正文域 id。
  `instr` / `host` / `end` / `text` 与该域在 `fields` 里的同名字段相同。xlsx / pptx 的 `scope` 里出现 `indexes` 报 `InvalidField`。
- `scope.indexEntries` 为 `true` 时，根上带 `indexEntries`。没有 XE 时为空数组。按主文档里被收入的顶层 XE 域起点的文档顺序。
  每条字段顺序：`id`、`index`、`field`、`instr`、`entry`、`host`、`end`、`text`。`id` 是 `xe:{n}`。
  `entry` 是 XE 指令里的第一个参数（条目文本，可含冒号表示次级项），缺则为 `""`。xlsx / pptx 的 `scope` 里出现 `indexEntries` 报 `InvalidField`。
- `scope.sources` 为 `true` 时，根上带 `sources`。没有书目源时为空数组。按 bibliography customXml 里 `b:Source` 的文档顺序。
  每条字段顺序：`id`、`index`、`tag`、`sourceType`、`title`、`year`、`author`。`id` 是 `source:{n}`。
  `tag` / `sourceType` / `title` / `year` 取对应子元素文本，缺则为 `""`。
  `author` 取第一个 `b:Person` 的 `Last` 与 `First`，写成 `Last, First`（缺哪一侧就省略，两侧都缺则为 `""`）。
  xlsx / pptx 的 `scope` 里出现 `sources` 报 `InvalidField`。
- `scope.citations` 为 `true` 时，根上带 `citations`。没有 CITATION 时为空数组。按主文档里被收入的顶层 CITATION 域起点的文档顺序。
  每条字段顺序：`id`、`index`、`field`、`instr`、`tag`、`host`、`end`、`text`。`id` 是 `cite:{n}`。`tag` 是 CITATION 指令里的第一个参数，缺则为 `""`。
  xlsx / pptx 的 `scope` 里出现 `citations` 报 `InvalidField`。
- `scope.bibliographies` 为 `true` 时，根上带 `bibliographies`。没有 BIBLIOGRAPHY 时为空数组。按主文档里被收入的顶层 BIBLIOGRAPHY 域起点的文档顺序。
  每条字段顺序：`id`、`index`、`field`、`instr`、`host`、`end`、`text`。`id` 是 `biblio:{n}`。
  xlsx / pptx 的 `scope` 里出现 `bibliographies` 报 `InvalidField`。
- `scope.docParts` 为 `true` 时，根上带 `docParts`。没有构建基块时为空数组。按 glossary 里 `w:docPart` 的文档顺序。
  每条字段顺序：`id`、`index`、`name`、`gallery`、`category`、`type`、`behavior`、`guid`、`text`。`id` 是 `docPart:{n}`。
  `name` 取 `w:name/@w:val`，缺则为 `""`。`gallery` 取 `w:category/w:gallery/@w:val`，缺则为 `null`。
  `category` 取 `w:category/w:name/@w:val`，缺则为 `null`。`type` 取第一个 `w:types/w:type/@w:val`，缺则为 `null`。
  `behavior` 取第一个 `w:behaviors/w:behavior/@w:val`，缺则为 `null`。`guid` 取 `w:guid/@w:val`，缺则为 `null`。
  `text` 为 `docPartBody` 子孙 `w:t` 按文档顺序拼接。xlsx / pptx 的 `scope` 里出现 `docParts` 报 `InvalidField`。
- `scope.embeddedFonts` 为 `true` 时，根上带 `embeddedFonts`。没有嵌入面时为空数组。按 fontTable 里 embed* 的文档顺序。
  每条字段顺序：`id`、`index`、`name`、`style`、`subsetted`、`fontKey`。`id` 是 `font:{n}`。`name` 取所属 `w:font/@w:name`，缺则为 `""`。
  `style` 为 `regular` / `bold` / `italic` / `boldItalic`，
  对应 `embedRegular` / `embedBold` / `embedItalic` / `embedBoldItalic`。
  `subsetted` 为布尔：`w:subsetted` 为 `1` / `true` / `on` 时为 `true`，否则为 `false`。`fontKey` 取 `w:fontKey` 原文，缺则为 `null`。
  不把 odttf 字节放进树。xlsx / pptx 的 `scope` 里出现 `embeddedFonts` 报 `InvalidField`。
- 文档根上的 `fontEmbed` 始终出现，对象字段顺序：`trueType`、`systemFonts`、`subset`。
  三者都是布尔，分别对应 `settings.xml` 的 `w:embedTrueTypeFonts` / `w:embedSystemFonts` / `w:saveSubsetFonts`。
  没有对应元素为 `false`；有元素且省略 `val`、或 `val` 为 `1` / `true` / `on` 为 `true`。没有 settings 部件则三者都是 `false`。页眉页脚容器不出现 `fontEmbed`。
  xlsx / pptx 根上不要带 `fontEmbed`。
- 文档根上的 `pageBackground` 始终出现，对象字段顺序：`color`、`display`、`fill`。
  `color` 取主文档根上 `w:background/@w:color` 原文（ECMA-376 §17.2.1），没有 `w:background` 或缺该属性则为 `null`。
  不读 `themeColor`，不算页面上实际画出的像素。
  `display` 对应 `settings.xml` 的 `w:displayBackgroundShape`：有元素且 OnOff 为开为 `true`，否则为 `false`。没有 settings 则为 `false`。
  `fill` 没有 `v:background` 也没有 `w:drawing` 时为 `null`。有则对象只有 `kind`：
  - `w:drawing` 为 `"drawing"`
  - 否则 VML `v:fill/@type` 为 `gradient` 时为 `"gradient"`
  - VML 图片填（`frame` / `tile` / 带 `r:id`）为 `"picture"`
  - 其余 VML 为 `"gradient"`
  页眉页脚容器不出现 `pageBackground`。xlsx / pptx 根上不要带 `pageBackground`。create 给了 `document.pageBackground` 时按 `setPageBackground` 写入。
- 文档根上的 `eastAsianSettings` 始终出现，对象字段顺序：`characterSpacingControl`、`strictFirstAndLastChars`、`noLineBreaksBefore`、`noLineBreaksAfter`、`themeFontLang`。
  `characterSpacingControl` 取 `settings.xml` 的 `w:characterSpacingControl/@w:val`（§17.15.1.18），没有该元素或缺 `val` 则为 `null`。
  `strictFirstAndLastChars` 取 `w:strictFirstAndLastChars`（§17.15.1.82）：没有元素为 `null`；有且 OnOff 为开为 `true`，为关为 `false`。
  `noLineBreaksBefore` / `noLineBreaksAfter` 取 `w:noLineBreaksBefore` / `w:noLineBreaksAfter`（§17.15.1.59、§17.15.1.58）。
  没有该元素为 `null`。有则对象字段顺序：`lang`、`chars`，对应 `w:lang` / `w:val`，缺则为 `null`。
  `themeFontLang` 取 `w:themeFontLang`（§17.15.1.86）。没有该元素为 `null`。有则对象字段顺序：`val`、`eastAsia`、`bidi`，取对应属性原文，缺则为 `null`。
  没有 settings 部件则五项都是 `null`。页眉页脚容器不出现 `eastAsianSettings`。xlsx / pptx 根上不要带此对象。create 给了 `document.eastAsianSettings` 时按 `setEastAsianSettings` 写入。
- 文档根上的 `bibliographyStyle` 始终出现。没有 bibliography customXml 为 `null`。有则对象字段顺序：`selectedStyle`、`styleName`、`version`。
  三者取 `b:Sources` 的 `SelectedStyle` / `StyleName` / `Version` 原文，缺为 `null`。不跑 XSL。
  页眉页脚容器不出现此对象。xlsx / pptx 根上不要带 `bibliographyStyle`。create 给了 `document.bibliographyStyle` 时按 `setBibliographyStyle` 写入。
- 节上的 `pageBorders` 始终出现。没有 `w:pgBorders` 为 `null`。有则对象字段顺序：`offsetFrom`、`display`、`zOrder`、`top`、`left`、`bottom`、`right`。
  前三项取对应属性原文，缺则为 `null`。四边取 `w:top` / `w:left` / `w:bottom` / `w:right`，没有该子元素为 `null`。
  每边对象字段顺序：`val`、`sz`、`space`、`color`。`val` 取 `@w:val`，缺则为 `""`。
  `sz` 为八分之一磅（`ST_EighthPointMeasure`），能解析成非负整数则写整数，否则为 `null`。`space` 为单位磅的非负整数，缺或解析失败为 `null`。
  `color` 取 `@w:color` 原文，缺则为 `null`。`val` 认 ECMA-376 `ST_Border` 全套，含艺术型（Word 16.0 把苹果边框写成 `w:val="apples"`，没有独立 `w:art`）。
  不把 `bordersDoNotSurroundHeader` / `bordersDoNotSurroundFooter` / `alignBordersAndEdges` 单开到树上。
  xlsx / pptx 节上不要带 `pageBorders`。create 给了 `document.pageBorders` 时写入末节 `sectPr`。
- 节上的 `formProt` 始终出现。没有 `w:formProt` 为 `null`；OnOff 为开为 `true`，为关为 `false`。
  Word 16.0 窗体保护下，未写 `formProt` 的节仍受保护；例外节写出 `w:formProt w:val="0"`。
  xlsx / pptx 节上不要带 `formProt`。
- `scope.maths` 为 `true` 时，根上带 `maths`（正文），每个页眉、页脚容器带 `maths`。没有公式时为空数组。按该故事里顶层 `m:oMath` 的文档顺序。
  每条字段顺序：`id`、`index`、`kind`、`jc`、`host`、`text`、`expr`。`id` 是 `math:{n}` 或 `{容器}/math:{n}`。`kind` 为 `inline` 或 `display`。
  `jc` 取父 `m:oMathParaPr/m:jc/@m:val`，值为 `center` / `centerGroup` / `left` / `right`。没有该元素或行内为 `null`。不认识报 `XmlBroken`。
  `host` 是所在段落 id；不在任何 `w:p` 里则为 `null`。`text` 为该 `oMath` 子孙里所有 `t`（即 `m:t`）按文档顺序拼接，不做 UnicodeMath 全量互转。
  `expr` 是 `oMath` 直接子节点的数组，节点 `type` 见下文「公式 expr」。
  xlsx / pptx 的 drawing 公式框见 [xlsx 分册](json-contract-xlsx.md#inspect) 与 [pptx 分册](json-contract-pptx.md#inspect)。
- `scope.rubies` 为 `true` 时，根上带 `rubies`（正文），每个页眉、页脚容器带 `rubies`。没有拼音时为空数组。按该故事里 `w:ruby` 的文档顺序。
  每条字段顺序：`id`、`index`、`host`、`base`、`rt`、`align`、`hps`、`hpsRaise`、`hpsBaseText`、`lid`。
  `id` 是 `ruby:{n}` 或 `{容器}/ruby:{n}`。`host` 是所在段落 id；不在任何 `w:p` 里则为 `null`。
  `base` 为 `w:rubyBase` 子孙可见 `w:t` 拼接。`rt` 为 `w:rt` 子孙可见 `w:t` 拼接。
  `align` 取 `w:rubyPr/w:rubyAlign/@w:val`，缺则为 `null`。
  `hps` / `hpsRaise` / `hpsBaseText` 取对应子元素 `@w:val`，能解析成整数则写整数，否则为 `null`。
  `lid` 取 `w:lid/@w:val`，缺则为 `null`。xlsx / pptx 的 `scope` 里出现 `rubies` 报 `InvalidField`。
- `scope.contentControls` 为 `true` 时，根上带 `contentControls`（正文），每个页眉、页脚容器带 `contentControls`。没有控件时为空数组。
  按该故事里被收入的 `w:sdt` 的文档顺序。每条字段顺序：`id`、`index`、`kind`、`level`、`host`、`tag`、`alias`、`xmlId`、`bound`、`parent`、`text`；
  `kind` 为 `dropDown` 或 `comboBox` 时再跟 `items`；`kind` 为 `date` 时再跟 `fullDate`；`kind` 为 `checkbox` 时再跟 `checked`。
  `id` 是 `sdt:{n}` 或 `{容器}/sdt:{n}`。`kind` 为 `plainText`、`richText`、`dropDown`、`comboBox`、`date`、`repeatingSection`、
  `checkbox`、`picture`、`group` 或 `equation`。
  `level` 为 `inline` 或 `block`。`host` 是所在段落 id（行内为祖先 `w:p`；块级为 `sdtContent` 里第一个 `w:p`）；对不上则为 `null`。
  `tag` / `alias` 取 `w:tag` / `w:alias` 的 `w:val`，缺则为 `null`。`xmlId` 是 `w:sdtPr/w:id/@w:val` 原文，缺则为 `null`。
  `bound` 在有 `w:dataBinding` 时为 `true`，否则为 `false`。`parent` 是最近一层被收入的祖先控件 id，没有则为 `null`。
  `text` 为 `sdtContent` 子孙里所有 `w:t` 按文档顺序拼接。
  `items` 是 `w:listItem` 数组，每项字段顺序：`display`、`value`，取 `w:displayText` / `w:value`，缺则为 `null`。
  `fullDate` 取 `w:date/@w:fullDate` 原文，缺则为 `null`。
  `checked` 看 `w14:checkbox/w14:checked`：没有该子元素为 `false`；有且 OnOff 为开为 `true`，否则为 `false`。
  图片控件的 `text` 通常为空（内容是 DrawingML，没有 `w:t`）；图仍走该段 `pictures` / `exportMedia`，不把字节放进控件节点。
  xlsx / pptx 的 `scope` 里出现 `contentControls` 报 `InvalidField`。
- `scope.numbering` 为 `true` 时，根上带 `numberings`。没有实例时为空数组。按 `numbering.xml` 里 `w:num`（`numId` 非 0）的文档顺序。
  每条字段顺序：`id`、`numId`、`abstractId`、`abstractNumId`、`multiLevelType`、`levels`。`id` 是 `num:{numId}`。
  `abstractId` 是 `abstractNum:{abstractNumId}`。`multiLevelType` 取该实例指向的抽象定义上 `w:multiLevelType/@w:val`，缺则为 `null`。
  `levels` 按 `ilvl` 升序。每一级先取抽象定义上的 `w:lvl`，该实例上同 `ilvl` 的 `w:lvlOverride/w:lvl` 盖住对应级。
  每级字段顺序：`ilvl`、`numFmt`、`lvlText`、`start`、`lvlJc`。`numFmt` / `lvlText` / `lvlJc` 取对应子元素 `@w:val` 原文，缺则为 `null`。
  `start` 能解析成整数则写整数，否则为 `null`。inspect 列出 XML 原文，不把 `numFmt` 收成白名单。
  xlsx / pptx 的 `scope` 里出现 `numbering` 报 `InvalidField`。
- `scope.styles` 为 `true` 时，根上带 `styles`。没有样式时为空数组。按 `styles.xml` 里 `w:style`（`styleId` 非空）的文档顺序。
  每条字段顺序：`id`、`styleId`、`type`、`name`、`basedOn`、`default`。`id` 是 `style:{styleId}`。`styleId` 是 `w:styleId` 原文。
  `type` 取 `w:type` 原文，缺则为 `null`（规范缺省为段落样式，树上不猜）。`name` 取 `w:name/@w:val`，缺则为 `null`。
  `basedOn` 取 `w:basedOn/@w:val`，缺则为 `null`。`default` 看 `w:default`：`1` / `true` / `on` 为 `true`，其余或没有为 `false`。
  `w:latentStyles` 与 `w:docDefaults` 不进名单。xlsx / pptx 的 `scope` 里出现 `styles` 报 `InvalidField`。
- `align` 取该段自己的 `w:pPr/w:jc/@w:val`（§17.3.1.13），没有 `jc` 则为 `null`。不把 `styles.xml` 摊进来。
- `indent` 取该段自己的 `w:pPr/w:ind`（§17.3.1.12）。没有该元素则为 `null`。有则对象字段顺序：`left`、`right`、`firstLine`、`hanging`。
  `left` 取 `w:left`，没有则取 `w:start`。`right` 取 `w:right`，没有则取 `w:end`。能解析成非负整数则写整数，否则为 `null`。缺属性为 `null`。单位 twip。
- `spacing` 取该段自己的 `w:pPr/w:spacing`（§17.3.1.33）。没有该元素则为 `null`。有则对象字段顺序：`before`、`after`、`line`、`lineRule`。
  前三项能解析成非负整数则写整数，否则为 `null`。`lineRule` 取 `w:lineRule` 原文，缺则为 `null`。单位 twip。不计算自动行距。
- `eastAsia` 取该段自己的 `w:pPr` 上东亚排版开关（§17.3.1.16、§17.3.1.23、§17.3.1.45、§17.3.1.2、§17.3.1.3）。
  五个子元素都没有则为 `null`。有任意一个则对象字段顺序：`kinsoku`、`overflowPunct`、`topLinePunct`、`autoSpaceDE`、`autoSpaceDN`。
  各键：没有该子元素为 `null`；有且 OnOff 为关为 `false`，否则为 `true`。不把 `styles.xml` 摊进来。不计算禁则断行。
- `borders` 取该段自己的 `w:pPr/w:pBdr`（§17.3.1.24）。没有该元素则为 `null`。有则对象字段顺序：`top`、`left`、`bottom`、`right`。
  每边取对应子元素，没有则为 `null`。有则字段顺序与页面边框相同：`val`、`sz`、`space`、`color`。
  `val` 取 `@w:val`，缺则为 `""`。`sz` 为八分之一磅，能解析成非负整数则写整数，否则为 `null`。
  `space` 为单位磅的非负整数，缺或解析失败为 `null`。`color` 取 `@w:color` 原文，缺则为 `null`。
  不把 `between` / `bar` 放进树。不把 `styles.xml` 摊进来。
- `shd` 取该段自己的 `w:pPr/w:shd`（§17.3.1.31）。没有该元素则为 `null`。有则对象字段顺序只有 `fill`。
  `fill` 取 `@w:fill` 原文，缺则为 `null`。不解析 theme。
- `tabs` 取该段自己的 `w:pPr/w:tabs`（§17.3.1.38）。没有该元素、或没有 `w:tab` 子元素，则为 `null`。
  有则数组按 `w:tab` 文档顺序。每项字段顺序：`val`、`pos`。
  `val` 取 `@w:val` 原文，缺则为 `null`。`pos` 能解析成非负整数则写整数，否则为 `null`。单位 twip。不报 `leader`。
- `runCount` 始终出现在已列出的段落上（正文切片、页眉页脚段落、框内、注内、批注内、表格单元格）。计该段内被收入的 `w:r` 个数，规则与 run id 相同。
- `scope.runs` 为 `true` 时，已列出的段落带 `runs`。每条字段顺序：`id`、`index`、`style`、`text`、`bold`、`italic`、`underline`、`sz`、`color`、`font`、`eastAsiaFont`、`em`、`eastAsianLayout`、`highlight`、`strike`、`dstrike`、`vertAlign`、`spacing`、`kern`、`scale`。
  `id` 是 `{段落 id}/run:{n}`。`index` 与 `{n}` 相同。
  `style` 取该 run 自己的 `w:rStyle/@w:val`，没有 `rStyle` 则为 `null`。不把 `styles.xml` 摊进来。
  `text` 是该 run 内可见纯文本，规则与段上 `text` 相同，范围限于这个 `w:r`。
  `bold` / `italic` 只看该 run 自己的 `w:b` / `w:i`：没有元素为 `null`；有且 OnOff 为关为 `false`，否则为 `true`。
  `underline` 取 `w:u/@w:val`，没有 `u` 为 `null`，有元素却没有 `val` 则当作 `single`。`sz` 取 `w:sz/@w:val` 能解析成整数则写整数，否则为 `null`。
  `color` 取 `w:color/@w:val` 原文，缺则为 `null`；不解析 theme。`font` 取 `w:rFonts` 的 `ascii`，没有则取 `hAnsi`，再没有为 `null`。
  `eastAsiaFont` 取 `eastAsia`，缺则为 `null`。
  `em` 取 `w:em/@w:val`（§17.3.2.12），没有 `em` 为 `null`。
  `eastAsianLayout` 取 `w:eastAsianLayout`（§17.3.2.10）。没有该元素为 `null`。有则对象字段顺序：`id`、`combine`、`combineBrackets`、`vert`、`vertCompress`。
  `id` 能解析成整数则写整数，否则为 `null`。`combine` / `vert` / `vertCompress` 取对应属性 OnOff：`1` / `true` / `on` 为 `true`，`0` / `false` / `off` 为 `false`，缺属性为 `null`。
  `combineBrackets` 取 `w:combineBrackets` 原文，缺则为 `null`。xlsx / pptx 的 `scope` 里出现 `runs` 报 `InvalidField`。
  `highlight` 取 `w:highlight/@w:val`（§17.3.2.15），没有 `highlight` 为 `null`。
  `strike` / `dstrike` 取该 run 自己的 `w:strike` / `w:dstrike`（§17.3.2.37、§17.3.2.9）：没有元素为 `null`；有且 OnOff 为关为 `false`，否则为 `true`。
  `vertAlign` 取 `w:vertAlign/@w:val`（§17.3.2.41），没有 `vertAlign` 为 `null`。
  `spacing` 取该 run 自己的 `w:rPr/w:spacing/@w:val`（§17.3.2.35），单位 twip，能解析成整数则写整数（可负），否则为 `null`。没有该元素为 `null`。
  这是整数，不是段上那个 `spacing` 对象。
  `kern` 取 `w:kern/@w:val`（§17.3.2.19），半磅，能解析成非负整数则写整数，否则为 `null`。没有该元素为 `null`。
  `scale` 取 `w:rPr/w:w/@w:val`（§17.3.2.43，`ST_TextScale` §17.18.95），百分比整数。
  属性可带或不带 `%` 后缀；能解析成整数则写整数，否则为 `null`。没有该元素为 `null`。
  不要叫 `w`。不把样式里的缩放摊进来。
  不把 `styles.xml` 摊进来。不把文档级 `characterSpacingControl` 摊到 run 上。
- `link` 始终出现。未链接则为 `{ "prev": null, "next": null }`。链接时 `prev` / `next` 是相邻框的 id，没有那一端则为 `null`。
- 框内 `paragraphs` 的字段顺序与正文段落相同（含 `pictureCount`）。框内图的字段顺序与宿主图相同，id 带框前缀。
- `kind`：`inline`（`wp:inline`）或 `anchor`（`wp:anchor`）。
- `cx` / `cy`：`wp:extent` 的值，单位 EMU，JSON 整数。缺则为 `null`。属性存在但不是整数报 `XmlBroken`。
- `wrap`：行内为 `null`；浮动取子元素 `wrapSquare` / `wrapTight` / `wrapThrough` / `wrapTopAndBottom` / `wrapNone`，
  对应 `square` / `tight` / `through` / `topAndBottom` / `none`。没有这些子元素则为 `null`。
- `docPrId` 取 `wp:docPr/@id` 的原文；没有该属性则为 `null`。`name` 取 `wp:docPr/@name` 的原文；没有则为 `null`。
- `stable`：有 `wp:docPr/@id` 为 `true`，否则为 `false`。
- `contentType`：该 `a:blip` 里有 `asvg:svgBlip` 时为 `image/svg+xml`。
  没有 `svgBlip` 时，按 image 关系 Target 扩展名：
  `.emf` 为 `image/x-emf`，`.wmf` 为 `image/x-wmf`，`.tif` / `.tiff` 为 `image/tiff`。
  命名空间为 `http://schemas.microsoft.com/office/drawing/2016/SVG/main`。
  PNG / JPEG / GIF / BMP 省略该字段。不读媒体字节。
- `style` 取 `w:pStyle/@w:val` 的原文。没有 `pStyle` 则为 `null`（Word 此时按默认段落样式显示，树上不猜）。不把 `styles.xml` 里的字体、字号摊到段上。
- `text` 是可见纯文本：该 `w:p` 内、不在 `w:del` 里的 `w:t` 按顺序拼接；`w:tab` 变成 `\t`，`w:br` 变成 `\n`。`w:instrText`（域指令）不进入。不含 run 级样式。
  嵌在 `w:txbxContent` 里的 `w:t` 不算，`mc:AlternateContent` 只取一支，判定见上文 docx 那节的边界两条。
- `stable`：有 `w14:paraId` 为 `true`，否则为 `false`。
- 无 `scope.revisions` 时不返回修订名单，仍返回 `revisionCount`。无 `scope.hyperlinks` 时不返回超链接名单，仍返回 `hyperlinkCount`。
  无 `scope.bookmarks` 时不返回书签名单，仍返回 `bookmarkCount`。无 `scope.fields` 时不返回域名单，仍返回 `fieldCount`。
  无 `scope.maths` 时不返回公式名单，仍返回 `mathCount`。无 `scope.rubies` 时不返回拼音名单，仍返回 `rubyCount`。
  无 `scope.tocs` 时不返回目录名单，仍返回 `tocCount`。无 `scope.captions` 时不返回题注名单，仍返回 `captionCount`。
  无 `scope.crossRefs` 时不返回交叉引用名单，仍返回 `crossRefCount`。无 `scope.indexes` 时不返回索引名单，仍返回 `indexCount`。
  无 `scope.indexEntries` 时不返回索引项名单，仍返回 `indexEntryCount`。无 `scope.sources` 时不返回书目源名单，仍返回 `sourceCount`。
  无 `scope.citations` 时不返回引文名单，仍返回 `citationCount`。无 `scope.bibliographies` 时不返回书目块名单，仍返回 `bibliographyCount`。
  无 `scope.docParts` 时不返回构建基块名单，仍返回 `docPartCount`。
  无 `scope.embeddedFonts` 时不返回嵌入字体名单，仍返回 `embeddedFontCount` 与 `fontEmbed`。无 `scope.maths` 时不返回公式名单，仍返回 `mathCount`。
  无 `scope.contentControls` 时不返回内容控件名单，仍返回 `contentControlCount`。无 `scope.numbering` 时不返回编号名单，仍返回 `numberingCount`。
  无 `scope.styles` 时不返回样式名单，仍返回 `styleCount`。无 `scope.runs` 时不返回 run 名单，已列出的段落仍返回 `runCount`。
  TOC / TOA 域同时出现在 `fields` 与 `tocs` 里。顶层 SEQ 同时出现在 `fields` 与 `captions` 里。
  顶层 REF / PAGEREF / NOTEREF 同时出现在 `fields` 与 `crossRefs` 里。顶层 INDEX 同时出现在 `fields` 与 `indexes` 里。
  顶层 XE 同时出现在 `fields` 与 `indexEntries` 里。顶层 CITATION 同时出现在 `fields` 与 `citations` 里。
  顶层 BIBLIOGRAPHY 同时出现在 `fields` 与 `bibliographies` 里。
  文本框里的段落不并进外层段落的 `text`；它们只出现在对应框的 `paragraphs` 里（需 `textBoxContent`）。
  含超链接或域或公式或拼音或行内内容控件的段落仍出现在树上，`text` 只含可见结果（不含 `w:instrText`）。apply 的 `setParaText` 验收只用无域、无超链接、无公式、无拼音、无行内内容控件的页眉页脚段落。


### 文档保护（仅 docx）

docx 根上始终带 `protection`，不要另给 scope 键。来源是 `/word/settings.xml` 的 `w:documentProtection`（ECMA-376 §17.15.1.29）与 `w:writeProtection`（§17.15.1.93）。
没有 settings 部件时按全关处理，不报错。xlsx / pptx 不要带此对象。xlsx 用 `sheetProtection` / `workbookProtection`。
pptx 根上用 `modifyProtection`，不要带 `protection`。

`protection` 字段顺序：`enforced`、`edit`、`formatting`、`hasPassword`、`readOnlyRecommended`。

- **`enforced`**
  - 来源：`documentProtection/@enforcement`
  - 说明：`ST_OnOff` 为 on / 1 / true 时为 `true`，缺元素或未强制为 `false`
- **`edit`**
  - 来源：`documentProtection/@edit`
  - 说明：`readOnly`、`comments`、`trackedChanges` 或 `forms`
  - 缺元素、省略或 `none` 时为 `null`
  - 不认识的取值报 `XmlBroken`
- **`formatting`**
  - 来源：`documentProtection/@formatting`
  - 说明：限制样式与直接格式
  - 缺或未开为 `false`
- **`hasPassword`**
  - 来源：两元素上的哈希属性
  - 说明：`documentProtection` 或 `writeProtection` 带非空 `hash` / `hashValue` / `salt` / `saltValue` / `reservationPassword` 时为 `true`
  - 树上不出现哈希、盐或明文
- **`readOnlyRecommended`**
  - 来源：`writeProtection/@recommended`
  - 说明：为 on 且该元素没有密码哈希时为 `true`
  - 有写保护密码时规范要求忽略 recommended，树上为 `false`

不把 `algorithmName`、`cryptProviderType`、`cryptAlgorithmSid`、`spinCount`、`hash`、`salt` 放进树。
列出 `w:permStart` / `w:permEnd` 见上文 `editRangeCount` / `editRanges`。不报告 IRM。
`setProtection` 带 `password` 时按 [json-contract.md](json-contract.md#setprotection) 写 Word 16.0 限制编辑的 `crypt*` SHA-512。
带 `writePassword` 时写修改密码哈希（UTF-16LE，不要遗留预处理）。不把口令或哈希写进树。

### 主题（仅 docx）

docx 根上始终带 `theme`，不要另给 scope 键。来源是主文档 `.rels` 里 Type 以 `/theme` 结尾的部件（Word 16.0 另存常见 `/word/theme/theme1.xml`，
ECMA-376 §14.2.7 / DrawingML `a:theme`）。
没有该关系或部件不在包内时为 `null`，不报错。pptx 根上不要带此对象。pptx 的 theme 见 [pptx 响应](json-contract-pptx.md#响应) 的 `masters[].theme`。xlsx 见 [xlsx 响应](json-contract-xlsx.md#主题xlsx)。create 给了 `document.theme` 时写 theme 部件；省略则仍不写。

有部件时字段顺序：`name`、`colors`、`fonts`、`fmt`、`extraSchemes`、`objectDefaults`。配色与字体规则与 pptx 母版 `theme` 相同，不把段落 / run 上的 `themeColor` 展开成 RGB。pptx 母版 `theme` 同一套字段，见 [pptx 分册](json-contract-pptx.md#响应)。

`fmt` 取 `a:fmtScheme`。没有该元素时四项都是空数组，`name` 为 `null`。有则字段顺序：`name`、`fills`、`lines`、`effects`、`bgFills`。
`name` 取 `@name`，缺则为 `null`。四个数组按 `fillStyleLst` / `lnStyleLst` / `effectStyleLst` / `bgFillStyleLst` 的直接子元素顺序。
`fills[]` / `bgFills[]` 对象只有 `kind`：`solidFill` 为 `solid`，`gradFill` 为 `grad`，`blipFill` 为 `blip`，`noFill` 为 `none`，`pattFill` 为 `patt`，其余为其本地名。
`lines[]` 对象只有 `w`：能解析成非负整数则写整数（EMU），否则为 `null`。
`effects[]` 对象只有 `kind`：`effectLst` 为空或没有子元素为 `none`，否则取第一个效果的本地名（如 `outerShdw`）。
不展开 `phClr`、渐变停点或效果参数。

`extraSchemes` 取 `a:extraClrSchemeLst` 下按文档顺序的 `a:extraClrScheme`。没有该列表则为 `[]`。
每项字段顺序：`name`、`colors`。`name` 取该项 `a:clrScheme/@name`，没有或空则为 `null`。
`colors` 规则与主 `clrScheme` 相同。不把 `clrMap` 展开进树。

`objectDefaults` 取 `a:objectDefaults`。没有该元素则为 `null`。有则字段顺序：`sp`、`ln`、`tx`。
每一项取对应 `a:spDef` / `a:lnDef` / `a:txDef`。没有该子元为 `null`。有则对象字段顺序：`style`、`lnW`。
没有 `a:style` 则 `style` 为 `null`。有则对象字段顺序：`lnRef`、`fillRef`、`effectRef`、`fontRef`。
`lnRef` / `fillRef` / `effectRef` 取 `@idx` 非负整数；缺或解析失败为 `null`。
`fontRef` 取 `@idx`：`major` / `minor` / `none`；缺或不在白名单为 `null`。
`lnW` 取 `spPr/a:ln/@w` 非负整数（EMU）；缺为 `null`。
不展开填充、效果或 `schemeClr`。

根上始终有 `themeOverride`，放在 `theme` 之后。来源是主文档 `.rels` 里 Type 以 `/themeOverride` 结尾的部件（ECMA-376 §14.2.8 / DrawingML `a:themeOverride`）。
没有该关系或部件不在包内时为 `null`。有则字段顺序：`name`、`colors`、`fonts`、`fmt`、`extraSchemes`。
配色 / 字体 / `fmt` 取根下的 `clrScheme` / `fontScheme` / `fmtScheme`，不经 `themeElements`。
`themeOverride` 不要 `objectDefaults`。
`themeOverride` 通常没有 `extraClrSchemeLst`，此时 `extraSchemes` 为 `[]`。
Word 16.0 另存通常不写该部件；自行写入后 Word 仍能打开。pptx 根上不要带此对象。xlsx 见 [xlsx 分册](json-contract-xlsx.md#主题xlsx)。

`colors` 字段顺序：`name`、`dk1`、`lt1`、`dk2`、`lt2`、`accent1`、`accent2`、`accent3`、`accent4`、`accent5`、`accent6`、`hlink`、`folHlink`。
`colors.name` 取 `a:clrScheme/@name`，没有则为 `null`。每个槽位：有 `a:srgbClr/@val` 取其原文；否则有 `a:sysClr/@lastClr` 取该原文；再没有则为 `null`。

`fonts` 字段顺序：`name`、`majorLatin`、`minorLatin`、`majorEa`、`minorEa`、`majorCs`、`minorCs`、`majorScripts`、`minorScripts`。
`fonts.name` 取 `a:fontScheme/@name`，没有则为 `null`。
`latin` / `ea` / `cs` 取 `a:majorFont` / `a:minorFont` 下对应元素的 `typeface` 原文，没有该元素则为 `null`。
`majorScripts` / `minorScripts` 取同一组里 `a:font` 的文档顺序。没有则为 `[]`。
每项字段顺序：`script`、`typeface`，取对应属性原文；缺则为 `null`。
pptx 母版 `theme.fonts` 同一套字段。pptx 页级 `themeOverride` 挂在幻灯片 / 备注页上，不在根上。

## create

```json
{
  "schemaVersion": 1,
  "action": "create",
  "path": "C:\\out\\note.docx",
  "format": "docx",
  "document": {
    "paragraphs": [
      {
        "style": "Heading1",
        "text": "标题"
      },
      {
        "text": "hello",
        "pictures": [
          {
            "file": "C:\\\\img\\\\a.png"
          }
        ]
      }
    ],
    "headers": [
      {
        "type": "default",
        "paragraphs": [
          {
            "text": "页眉",
            "pictures": [
              {
                "file": "C:\\\\img\\\\a.png",
                "kind": "anchor",
                "wrap": "square"
              }
            ]
          }
        ]
      }
    ]
  }
}
```

带图表、注、批注与两节的示例见 `docs/examples/docx-create-full.json`。
带超链接、书签、域、目录、编号、公式、内容控件与批注回复的示例见 `docs/examples/docx-create-remain.json`。
带 SVG 图的示例见 `docs/examples/docx-create-svg.json`。
带墨迹画布的示例见 `docs/examples/docx-create-ink.json`。
改墨迹画布尺寸与绕排的示例见 `docs/examples/docx-apply-ink-geom.json`。
带 3D 模型的示例见 `docs/examples/docx-create-model3d.json`。
带 theme 与页面背景的示例见 `docs/examples/docx-create-theme-bg.json`。
带样式、表外观、形状、连接线、题注、索引、参考文献、构建基块、嵌入字体、拼音与 DrawingML 链的示例见 `docs/examples/docx-create-round4.json`。
给框内图、无框形状写 DrawingML 链的示例见 `docs/examples/docx-apply-drawing-hlink-remain.json`。
带框内表、框内链、段边框底纹制表位、`runs[]`、组合与 `writePassword` 的示例见 `docs/examples/docx-create-round5.json`。

- **`path`**
  - 输出路径
  - 已存在则覆盖
  - 省略时必须由调用方另给落盘路径，命令行用 `--output`
- **`document.paragraphs`**
  - 段落列表
  - 可空
  - 空则生成一个空的 `w:p`，否则 Word 可能打不开
- **`document.theme`**
  - 可选对象
  - 字段认 `name`、`colors`、`fonts`、`fmt`、`extraSchemes`
  - 出现则写 `/word/theme/theme1.xml`
  - `colors` / `fonts` / `fmt.name` / `extraSchemes` 与对应 apply 相同
  - 省略的配色 / 字体槽与 `fmtScheme` 矩阵用 Office 默认
  - `fmt` 只认 `name`，不要在 JSON 里给 `fills` / `lines` / `effects` / `bgFills`
- **`document.pageBackground`**
  - 可选对象
  - 字段与 `setPageBackground` 相同（`color` 必填，`fill` 可选）
- **`document.pageBorders`**
  - 可选对象
  - 字段与 `setPageBorders` 相同（不要 `id` / `enabled`），写入末节 `sectPr`
- **`paragraphs[].text`**
  - 可见文本，可空字符串
  - 省略当作 `""`
  - 同段给了 `runs` 则忽略此字段
- **`paragraphs[].style`**
  - 段落样式 id
  - 省略或 `Normal` 表示正文
  - 第一期只允许 `Normal`、`Heading1`、`Heading2`、`Heading3`（大小写不敏感，写出时用这里的 canonical 形式）
- **`paragraphs[].sectionType`**
  - 可选
  - 该段是非末节的末段，取值与 `insertSection` 的 `type` 相同
  - 在该段 `pPr` 写出 `w:sectPr`（含 `w:type` 与默认 `pgSz` / `pgMar`）
  - 最后一段禁止带此字段，否则报 `InvalidField`
  - 省略则该段不是分节末段
  - 页眉、页脚段落不允许此字段
- **`paragraphs[].textBoxes`**
  - 可选
  - 该段上的文本框列表
  - 省略则不写框
- **`paragraphs[].pictures`**
  - 可选
  - 该段上的图列表
  - 省略则不写图
- **`paragraphs[].smartArts`**
  - 可选
  - 该段上的 SmartArt 列表
  - 省略则不写图示
  - 字段与 `insertSmartArt` 相同，但不含 `id`
  - 可选 `hyperlink`，规则与 `pictures[].hyperlink` 相同
- **`paragraphs[].charts`**
  - 可选
  - 该段上的图表列表
  - 省略则不写图表
  - 字段与 `insertChart` 相同，但不含 `op` / `id`，也不含 `from` / `to` / `anchor` / `dataFrom` / `x` / `y`
  - 出现这些 xlsx / pptx 字段报 `InvalidField`
  - 缺省与 `insertChart` 相同：`kind` 为 `anchor`，`wrap` 为 `square`
  - 可选 `hyperlink`，规则与 `pictures[].hyperlink` 相同
  - 页眉页脚段落同样允许
- **`paragraphs[].oleObjects`**
  - 可选
  - 该段上的嵌入对象列表
  - 省略则不写
  - 每项只要 `file`，规则与 `insertOle` 相同
  - 页眉页脚段落同样允许
- **`paragraphs[].inkCanvases`**
  - 可选
  - 该段上的墨迹画布列表
  - 省略则不写
  - 每项必须有 `file`，可选成对的 `cx` / `cy`，规则与 `insertInkCanvas` 相同
  - 可选 `hyperlink`，规则与 `pictures[].hyperlink` 相同
  - 页眉页脚段落同样允许
- **`paragraphs[].model3ds`**
  - 可选
  - 该段上的 3D 模型列表
  - 省略则不写
  - 每项必须有 `file`，可选成对的 `cx` / `cy`，规则与 `insertModel3d` 相同
  - 可选 `hyperlink`，规则与 `pictures[].hyperlink` 相同
  - 页眉页脚段落同样允许
- **`paragraphs[].footnote`**
  - 可选
  - 字符串
  - 在该段末插入一条脚注，规则与 `insertFootnote` 的 `text` 相同，可空
  - 省略则不写
  - 页眉页脚段落同样允许
- **`paragraphs[].endnote`**
  - 可选
  - 字符串
  - 在该段末插入一条尾注，规则与 `insertEndnote` 相同
  - 可与 `footnote` 同时出现
  - 页眉页脚段落同样允许
- **`paragraphs[].comments`**
  - 可选
  - 该段上的批注列表
  - 每条范围的 start 与 end 都是该段自身，不跨段
  - 页眉页脚段落同样允许
- **`paragraphs[].hyperlinks`**
  - 可选
  - 该段上的超链接列表
  - 省略则不写
  - 字段与 `insertHyperlink` 相同，不含 `op` / `id`
  - 必须有 `text`
  - `url` 与 `anchor` 恰好一个
  - 出现 报未知字段
    - `location`
    - `file`
    - `slide`
    - `ref`
    - `display`
    - `on`
  - 页眉页脚段落同样允许
- **`paragraphs[].bookmarks`**
  - 可选
  - 该段上的书签列表
  - 省略则不写
  - `name` 可选
  - 省略则按 `insertBookmark` 分配尚未占用的 `ooxp{n}`
  - `end` 可选，为同一故事 `paragraphs` 的 1 起下标
  - 省略则起止都是本段
  - `end` 小于本段下标、大于该故事段数、为 0 或不是正整数，报 `InvalidField`
  - 不要 `id`
  - 页眉页脚段落同样允许
- **`paragraphs[].fields`**
  - 可选
  - 该段上的域列表
  - 省略则不写
  - `instr` 与 `code` 恰好一个，规则与 `insertField` 相同
  - 不要 `id`
  - 页眉页脚段落同样允许
- **`paragraphs[].maths`**
  - 可选
  - 该段上的公式列表
  - 省略则不写
  - `text` 与 `expr` 恰好一个，规则与 `insertMath` 相同
  - `kind` 可选 `inline`（缺省）或 `display`
  - 不要 `id`
  - 页眉页脚段落同样允许
- **`paragraphs[].contentControls`**
  - 可选
  - 该段上的内容控件列表
  - 省略则不写
  - 字段与 `insertContentControl` 相同，不含 `op` / `id`
  - `checkbox` / `picture` 不要 `text`
  - 其余 kind `text` 必填非空
  - 页眉页脚段落同样允许
- **`paragraphs[].shapes`**
  - 可选
  - 该段上的无框形状列表
  - 字段与 `insertShape` 相同，不含 `op` / `id`
  - 可选 `hyperlink`，规则与 `pictures[].hyperlink` 相同
  - 页眉页脚段落同样允许
- **`paragraphs[].connectors`**
  - 可选
  - 该段上的连接线列表
  - 字段与 `insertConnector` 相同，不含 `op` / `id`
  - 可选 `hyperlink`，规则与 `pictures[].hyperlink` 相同
  - 页眉页脚段落同样允许
- **`paragraphs[].rubies`**
  - 可选
  - 该段上的拼音列表
  - 字段与 `insertRuby` 相同，不含 `op` / `id`
  - 页眉页脚段落同样允许
- **`paragraphs[].citations`**
  - 可选
  - 该段上的引文列表
  - 每项只要 `tag`，规则与 `insertCitation` 相同
  - 页眉页脚段落同样允许
- **`paragraphs[].indexEntries`**
  - 可选
  - 该段上的索引项列表
  - 每项只要 `entry`，规则与 `insertIndexEntry` 相同
  - 页眉页脚段落同样允许
- **`paragraphs[].crossRefs`**
  - 可选
  - 该段上的交叉引用列表
  - 字段与 `insertCrossRef` 相同，不含 `op` / `id`
  - 页眉页脚段落同样允许
- **`paragraphs[].editRange`**
  - 可选
  - `true` 时按 `insertEditRange` 省略 `edGrp` / `ed` 写入
  - 对象时字段与 `insertEditRange` 相同，不含 `op` / `id`
  - `false` 报 `InvalidField`
  - 页眉页脚段落同样允许
- **`paragraphs[].num`**
  - 可选
  - 从 1 起的整数，指向 `document.numberings` 的下标（不是 `num:{n}`）
  - 省略则该段不绑编号
  - 没有 `numberings`、越界或为 0 报 `InvalidField`
  - 页眉页脚段落同样允许
- **`paragraphs[].ilvl`**
  - 可选
  - 绑定级别，缺省 `0`，取值 0 到 8
  - 只给 `ilvl` 不给 `num` 报 `InvalidField`
- **`paragraphs[].borders` / `paragraphs[].shd` / `paragraphs[].tabs`**
  - 可选
  - 与 `setParaFormat` 相同（不要 `op` / `id`）
  - 不要 `align`、缩进、段距、行距、东亚开关
  - 页眉页脚段落同样允许
- **`paragraphs[].runs`**
  - 可选
  - 该段的 run 列表
  - 有则忽略同段 `text`
  - 空数组报 `InvalidField`
  - 每项必须有 `text`（可空字符串）
  - 其余字段与 `setRunFormat` 相同（不要 `op` / `id`）
  - 某项除 `text` 外没有可写字段则只写骨架文字，不发 `setRunFormat`
  - 页眉页脚段落同样允许
- **`document.groups`**
  - 可选
  - 正文组合列表
  - 每项只要 `ids`，至少两个 inspect 风格字符串
  - 成员必须是该故事已写出的浮动图、文本框、无框形状或连接线
  - 段 id 为 `para:{8 位大写十六进制}`，从 `00000001` 起
  - 框为 `{段 id}/txbx:{docPr}`
  - 无框形状为 `{段 id}/shape:{docPr}`
  - 连接线为 `{段 id}/cxn:{docPr}`
  - 浮动图为 `{段 id}/pic:{docPr}`
  - 行内图不能进组
  - 不要 `op` / `id`
  - 组合本身不进 inspect 树
  - 复用 `groupShapes`，`id` 为 `docx:document`
- **`headers[].groups` / `footers[].groups`**
  - 可选
  - 规则与 `document.groups` 相同
  - 成员 id 带页眉或页脚容器前缀，例如 `header:default:1/para:00000001/txbx:1`
  - `groupShapes` 的 `id` 为该部件容器 id
- **`document.numberings`**
  - 可选
  - 编号定义列表
  - 省略或空数组则不写 numbering 部件
  - 每项只要 `kind`：`decimal` 或 `bullet`，按数组顺序发 `insertNumbering`
- **`document.tocs`**
  - 可选
  - 目录块列表
  - 省略或空数组则不写
  - 只挂在 `document` 上
  - 每项 `kind` / `instr` 与 `insertToc` 相同
  - `after` 可选，为正文 `paragraphs` 的 1 起下标（插在该段之后）
  - 省略 `after` 则插到正文末尾
  - `after` 为 0 或超过段数报 `InvalidField`
  - 页眉、页脚上出现 `tocs` 报未知字段
- **`document.captions`**
  - 可选
  - 题注列表
  - 只挂在 `document` 上
  - 每项 `label` / `title` / `bookmark` 与 `insertCaption` 相同（不要 `id`）
  - `after` 规则与 `tocs` 相同
- **`document.indexes`**
  - 可选
  - 索引块列表
  - 只挂在 `document` 上
  - 每项 `instr` 与 `insertIndex` 相同
  - `after` 规则与 `tocs` 相同
- **`document.bibliographies`**
  - 可选
  - 书目块列表
  - 只挂在 `document` 上
  - 每项 `instr` 与 `insertBibliography` 相同
  - `after` 规则与 `tocs` 相同
- **`document.sources`**
  - 可选
  - 书目源列表
  - 每项字段与 `insertSource` 相同（不要 `op` / `id`）
- **`document.bibliographyStyle`**
  - 可选字符串
  - 取值与 `setBibliographyStyle` 的 `styleName` 相同
- **`document.docParts`**
  - 可选
  - 构建基块列表
  - 每项字段与 `insertDocPart` 相同（不要 `op` / `id`）
- **`document.styles`**
  - 可选
  - 自定义样式列表
  - 每项字段与 `insertStyle` 相同（不要 `op` / `id`）
- **`document.embeddedFonts`**
  - 可选
  - 嵌入字体列表
  - 每项字段与 `insertEmbeddedFont` 相同（不要 `op` / `id`）
- **`document.fontEmbed`**
  - 可选对象
  - 字段与 `setFontEmbed` 相同（不要 `id`）
- **`document.eastAsianSettings`**
  - 可选对象
  - 字段与 `setEastAsianSettings` 相同（不要 `id`）
- **`document.headers` / `document.footers`**
  - 可选
  - 页眉或页脚
  - 省略则不写对应部件
- **`document.props`**
  - 可选
  - 文件属性，字段与 [json-contract.md](json-contract.md#setprops) 的 `setProps` / `setCustomProp` 相同
- **`document.protection`**
  - 可选
  - 文档保护，字段与 [json-contract.md](json-contract.md#setprotection) 的 `setProtection` 相同（不要 `id`，不要 `hasPassword`）
  - 出现 `password` 仍报 `NotImplemented`
  - `writePassword` 按 `setProtection` 同一套规则写修改密码哈希
- **`document.watermarks`**
  - 可选
  - 水印列表
  - 省略或空数组则不写
  - 每项恰好给 `text` 或 `file` 之一，规则与 `insertWatermark` 相同（不要 `id`）
  - 在 follow-up 与 `protection` 之后写入
- **`document.tables`**
  - 可选
  - 正文表列表
  - 省略或空数组则不写 `w:tbl`
- **`headers[].type` / `footers[].type`**
  - `default`、`even` 或 `first`
  - 省略当作 `default`
  - 同一侧同一 `type` 只能出现一次，否则报 `InvalidField`
  - 给了 `even` 则写入整篇 `w:evenAndOddHeaders`
  - 给了 `first` 则第一节写入 `w:titlePg`
- **`headers[].paragraphs` / `footers[].paragraphs`**
  - 可选
  - 空或省略则写一个空的 `w:p`
  - 字段与正文段落相同，但不允许 `style`、不允许 `textBoxes`、不允许 `sectionType`
  - 允许
    - `pictures`
    - `smartArts`
    - `charts`
    - `oleObjects`
    - `inkCanvases`
    - `model3ds`
    - `footnote`
    - `endnote`
    - `comments`
    - `hyperlinks`
    - `bookmarks`
    - `fields`
    - `maths`
    - `contentControls`
    - `shapes`
    - `connectors`
    - `rubies`
    - `citations`
    - `indexEntries`
    - `crossRefs`
    - `editRange`
    - `num`
    - `ilvl`
    - `borders`
    - `shd`
    - `tabs`
    - `runs`
- **`headers[].tables` / `footers[].tables`**
  - 可选
  - 该部件里的表，规则与 `document.tables` 相同，`after` 相对该部件的 `paragraphs`

| `tables[].cells` | 必须。二维字符串数组。至少一行一列，至多 500 行、63 列（Word 列上限）。每一行的长度必须相同，否则报 `InvalidField`。每个字符串是该格第一段可见文本，可空 |
| `tables[].after` | 可选。非负整数。表插在第 `after` 个段落之后（段落按下标从 1 计）；`0` 表示插在第一段之前。省略则插在全部段落之后。超过段落个数报 `InvalidField`。多张表 `after` 相同则按数组顺序相邻 |
| `tables[].style` | 可选。`style:{styleId}`，规则与 `setTableStyle` 相同 |
| `tables[].look` | 可选。对象，规则与 `setTableStyle` 的 `look` 相同 |
| `tables[].pos` | 可选。对象，规则与 `setTableFloat` 相同 |
| `tables[].align` | 可选。`left` / `center` / `right`，规则与 `setTableAlign` 相同 |
| `tables[].indent` | 可选。非负 twip，规则与 `setTableAlign` 的 `indent` 相同 |
| `tables[].width` | 可选。对象，字段 `type` / `w`，规则与 `setTableWidth` 相同 |
| `tables[].cols` | 可选。正整数数组，规则与 `setTableWidth` 的 `cols` 相同 |
| `tables[].rowHeights` | 可选。与行数相同。每项字段与 `setTableRowHeight` 相同（不要 `id`） |
| `tables[].cellMar` | 可选。对象，字段 `top` / `left` / `bottom` / `right`，规则与 `setTableCellMar` 相同 |
| `tables[].borders` | 可选。对象或 `null`，规则与 `setTableBorders` 相同 |
| `tables[].cellFormats` | 可选。每项必须有 1 起的 `r` / `c`，其余字段与 `setCellFormat` 相同，并可带 `borders`（规则与 `setCellBorders` 相同，不要 `id`） |

| `textBoxes[].text` | 框内第一段可见文本，可空。省略当作 `""` |
| `textBoxes[].cx` | 可选。宽度，EMU。省略为 `1828800` |
| `textBoxes[].cy` | 可选。高度，EMU。省略为 `914400` |
| `textBoxes[].hyperlink` | 可选。`url` 与 `anchor` 恰好一个，可选 `tooltip`。不要 `text`。规则与 DrawingML `insertHyperlink` 相同 |
| `textBoxes[].tables` | 可选。框内表列表。字段与 `document.tables` 相同。`after` 相对该框骨架段落。省略 `after` 或 `after` 等于框内现有段数（骨架为 1）时插到框末。`after` 为 0 报 `InvalidField` |
| `textBoxes[].hyperlinks` | 可选。作用在框内第一段。规则与 `paragraphs[].hyperlinks` 相同 |
| `textBoxes[].bookmarks` | 可选。作用在框内第一段。规则与 `paragraphs[].bookmarks` 相同。`end` 相对该框段落 |
| `textBoxes[].fields` | 可选。作用在框内第一段。规则与 `paragraphs[].fields` 相同 |
| `textBoxes[].maths` | 可选。作用在框内第一段。规则与 `paragraphs[].maths` 相同 |
| `textBoxes[].contentControls` | 可选。作用在框内第一段。规则与 `paragraphs[].contentControls` 相同 |
| `textBoxes[].rubies` | 可选。作用在框内第一段。规则与 `paragraphs[].rubies` 相同 |

- **`pictures[].file`**
  - 本地路径，与 `insertPicture` 相同
  - 按文件头认 PNG、JPEG、GIF、BMP、SVG、EMF、WMF、TIFF
  - 缺此字段报 `MissingField`
- **`pictures[].cx` / `pictures[].cy`**
  - 可选，EMU，正整数
  - 都省则与 `insertPicture` 相同
  - 只给一个时报 `InvalidField`
- **`pictures[].kind`**
  - 可选。`inline` 或 `anchor`，省略为 `inline`
- **`pictures[].wrap`**
  - `kind` 为 `anchor` 时必填，枚举与 `setPictureWrap` 相同
  - `kind` 为 `inline` 时必须省略
- **`pictures[].alignH` / `alignV` / `offsetX` / `offsetY`**
  - 仅浮动，规则与 `setPictureWrap` 相同
- **`pictures[].hyperlink`**
  - 可选对象
  - `url` 与 `anchor` 恰好一个
  - 可选 `tooltip`
  - 不要 `text`
  - 规则与 DrawingML `insertHyperlink` 相同

| `smartArts[].layout` | 必须。`list`、`org` 或 `cycle`，与 `insertSmartArt` 相同 |
| `smartArts[].points` | 可选。与 `insertSmartArt` 相同；省略则用该版式的缺省占位点 |
| `smartArts[].kind` / `wrap` / `cx` / `cy` | 可选。与 `insertSmartArt` 相同 |
| `smartArts[].hyperlink` | 可选。规则与 `pictures[].hyperlink` 相同 |

| `charts[].type` | 必须。与 `insertChart` 相同 |
| `charts[].series` | 必须。非空。字段与 `insertChart` 相同 |
| `charts[].title` | 可选 |
| `charts[].kind` / `wrap` / `cx` / `cy` | 可选。与 docx `insertChart` 相同 |
| `charts[].hyperlink` | 可选。规则与 `pictures[].hyperlink` 相同 |

| `comments[].text` | 可选。新批注第一段可见文本，省略当作 `""` |
| `comments[].author` | 可选。省略当作 `"ooxp"` |
| `comments[].initials` | 可选。省略当作 `"oo"` |
| `comments[].date` | 可选。`xsd:dateTime` 字符串。省略则不写 `w:date` |
| `comments[].replies` | 可选。该批注的回复列表。每项字段与 `comments[]` 相同，但不允许再套 `replies`。按 `replyComment` 写入。省略或空数组则不写回复 |

| `numberings[].kind` | 必须。`decimal` 或 `bullet`，与 `insertNumbering` 相同 |

| `tocs[].kind` | 可选。`toc` / `tof` / `toa`。省略且没有 `instr` 时当作 `toc` |
| `tocs[].instr` | 可选。与 `insertToc` 相同 |
| `tocs[].after` | 可选。正文段落 1 起下标。省略则插到正文末尾 |

| `hyperlinks[].text` | 必须。显示文本，可空 |
| `hyperlinks[].url` / `anchor` | 恰好一个。规则与 `insertHyperlink` 相同 |
| `hyperlinks[].tooltip` | 可选 |

| `bookmarks[].name` | 可选。书签名。规则与 `insertBookmark` 相同 |
| `bookmarks[].end` | 可选。同一故事段落的 1 起下标 |

| `fields[].instr` | 与 `code` 恰好一个。域指令 |
| `fields[].code` | 与 `instr` 恰好一个。规则与 `insertField` 相同 |
| `fields[].arg` / `name` / `items` / `checked` | 可选。规则与 `insertField` 相同 |
| `fields[].result` | 可选。缓存结果 |

| `maths[].text` | 与 `expr` 恰好一个。非空 |
| `maths[].expr` | 与 `text` 恰好一个。规则与 `insertMath` 相同 |
| `maths[].kind` | 可选。`inline` 或 `display` |

| `shapes[].prst` / `cx` / `cy` | 可选。与 `insertShape` 相同 |
| `shapes[].hyperlink` | 可选。规则与 `pictures[].hyperlink` 相同 |
| `connectors[].prst` / `cx` / `cy` | 可选。与 `insertConnector` 相同 |
| `connectors[].hyperlink` | 可选。规则与 `pictures[].hyperlink` 相同 |
| `rubies[].base` / `rt` | 必须非空。其余字段与 `insertRuby` 相同 |
| `citations[].tag` | 必须。与 `insertCitation` 相同 |
| `indexEntries[].entry` | 必须。与 `insertIndexEntry` 相同 |
| `crossRefs[].bookmark` | 必须。可选 `kind`，与 `insertCrossRef` 相同 |
| `editRange` | `true` 或对象。对象字段与 `insertEditRange` 相同 |

| `contentControls[].text` | `checkbox` / `picture` 不要此字段。其余 kind 必须非空 |
| `contentControls[].kind` | 可选。`plainText`（缺省）、`richText`、`dropDown`、`comboBox`、`date`、`checkbox`、`picture`、`group`、`equation` |
| `contentControls[].file` | 只用于 `picture`。本地图片路径，规则与 `insertPicture` 相同 |
| `contentControls[].checked` | 只用于 `checkbox`。布尔，缺省 `false` |
| `contentControls[].tag` / `alias` / `items` / `fullDate` | 可选。规则与 `insertContentControl` 相同 |

不把 base64 写进 create JSON。xlsx 的图只认 `workbook.sheets[].pictures`，图表只认 `workbook.sheets[].charts`。
pptx 的图只认 `presentation.slides[].pictures`，图表只认 `presentation.slides[].charts`，
SmartArt 只认 `presentation.slides[].smartArts`，表只认 `presentation.slides[].tables`。
`presentation` 根上或 `shapes[]` 上出现 `pictures` / `charts` / `smartArts` / `tables` 报 `InvalidField`。
xlsx 上出现 `smartArts` 或 `textBoxes` 同样报 `InvalidField`。

create 写出 Transitional 最小包，见 [opc.md](opc.md)。
每个 `w:p` 都带唯一的 `w14:paraId` 和 `w14:textId`（取值大于 0、小于 `0x80000000`，MS-DOCX §2.6.2.3 / §2.6.2.4），并在根元素上把 `w14` 列入 `mc:Ignorable`。
这样随后 inspect 得到的 id 都是 `para:{paraId}`，`stable` 为 `true`。图的 `wp:docPr/@id` 在该部件内从 `1` 起递增，与 `insertPicture` 同一套写入。

标题段写出 `<w:pStyle w:val="Heading1"/>` 这类引用，样式定义放在 `/word/styles.xml`。
同时写 `/word/settings.xml`，含 `<w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/>`
（MS-DOCX §2.3.5）。
缺这项时 Word 按兼容模式打开（默认 12）。`textBoxes` 写成浮动 `square` 的 `wps:wsp`，与 `insertTextBox` 同一套最小 Choice。
`smartArts` 复用 `insertSmartArt` 的四件套生成。给了 `headers` / `footers` 时按出现顺序写 `/word/headerN.xml` 或 `/word/footerN.xml`，
并在第一节 `sectPr` 上挂对应 `w:type` 的引用。
若第一节在某段上结束，引用写在该段的 `sectPr` 上，不要留在 `w:body`。给了 `even` 则 `settings.xml` 含 `<w:evenAndOddHeaders/>`。
给了 `first` 则第一节 `sectPr` 含 `<w:titlePg/>`。给了 `paragraphs[].sectionType` 时写出对应中间节；末节的 `sectPr` 仍在 `w:body` 末尾。
给了 `paragraphs[].charts` 时按 `insertChart` 同一套规则写 `/word/charts/…`、embedding 与宿主 `w:drawing`。
给了 `paragraphs[].oleObjects` 时按 `insertOle` 同一套规则写 embeddings、预览图与 `w:object`。
给了 `paragraphs[].inkCanvases` 时按 `insertInkCanvas` 同一套规则写 `/word/ink/…` 与宿主 `w:drawing`。
给了 `paragraphs[].model3ds` 时按 `insertModel3d` 同一套规则写 `/word/media/…` 的 GLB、海报 PNG 与宿主 `w:drawing`。
给了 `footnote` / `endnote` 时按对应 insert 同一套规则写注部件、分隔符与引用 run。
给了 `comments` 时按 `insertComment` 同一套规则写 `comments.xml`、`commentsExtended.xml`、`people.xml` 与范围标记。
给了 `comments[].replies` 时按 `replyComment` 写回复。
给了 `hyperlinks` / `bookmarks` / `fields` / `maths` / `contentControls` 时按对应 insert 写入。
给了 `shapes` / `connectors` / `rubies` / `citations` / `indexEntries` / `crossRefs` / `editRange` 时按对应 insert 写入。
给了 `pictures[]` / `textBoxes[]` / `shapes[]` / `connectors[]` / `smartArts[]` / `charts[]` / `inkCanvases[]` / `model3ds[]` 的 `hyperlink` 时按 DrawingML `insertHyperlink` 写入。
给了 `document.numberings` 时按 `insertNumbering` 写 `/word/numbering.xml`，再按 `paragraphs[].num` 发 `setParaNumbering`。
给了 `document.styles` 时按 `insertStyle` 追加自定义样式。
给了 `document.sources` 时按 `insertSource` 写入。给了 `document.bibliographyStyle` 时按 `setBibliographyStyle` 写入。
给了 `document.embeddedFonts` 时按 `insertEmbeddedFont` 写入。给了 `document.fontEmbed` 时按 `setFontEmbed` 写入。
给了 `document.eastAsianSettings` 时按 `setEastAsianSettings` 写入。
给了 `document.docParts` 时按 `insertDocPart` 写入。
给了 `document.tocs` 时按 `insertToc` 在正文插入目录块。
给了 `document.captions` / `indexes` / `bibliographies` 时按对应 insert 插入新段。
同一段上先写骨架里的文字或 `runs[]` / 图 / 框 / SmartArt，再图表，再 OLE，再墨迹画布，再 3D 模型，再脚注，再尾注，再批注，再该条批注的回复，再段 `borders` / `shd` / `tabs`，再 run 格式，再框内表，再框内书签 / 链 / 域 / 公式 / 控件 / 拼音，再正文书签，再超链接，再域，再公式，再内容控件，再无框形状，再连接线，再拼音，再索引项，再引文，再交叉引用，再例外编辑区，再绑编号。
全部故事写完后再写 `tocs` / `captions` / `indexes` / `bibliographies`，再写 `groups`。
`numberings` 与 `styles` 在任何段落绑定之前写入。`sources` 在引文之前写入。不写修订痕迹。
也不写批注的回复再回复。出现 `password` 仍报 `NotImplemented`。给了 `writePassword` 时按 `setProtection` 写入。
给了 `tables[].style` / `look` / `pos` / `align` / `indent` / `width` / `cols` / `rowHeights` / `cellMar` / `borders` / `cellFormats` 时按对应 set op 写入。框内表同样。
给了 `document.props` 时按 `setProps` / `setCustomProp` 同一套规则写 core / app / custom；
省略则仍写空的 `dc:title` 与 `dc:creator` 为 `ooxp`，不写 custom 部件。
给了 `document.protection` 时按 `setProtection` 同一套规则写 `settings.xml`，窗体保护还可写各节 `w:formProt`。
给了 `document.watermarks` 时按 `insertWatermark` 同一套规则写入页眉。
给了 `document.theme` 时写 `/word/theme/theme1.xml` 与主文档 theme 关系；省略则不写 theme。
给了 `document.pageBackground` 时按 `setPageBackground` 写入。给了 `document.pageBorders` 时按 `setPageBorders` 写入末节。
默认页眉页脚带框不在本期 create。
给了 `tables` 时写出带单线边框的 `w:tbl`，每列 `gridCol` 宽 1440 twip，每格一段，带新的 `w14:paraId`。给了 `width` / `cols` 时按 `setTableWidth` 覆盖。不算自动列宽。框内、表内、注内本期 create 不写图表或注。


## apply

各格式认哪些 `op` 以 [json-contract.md](json-contract.md#apply) 那一段名单为准。跨格式 op 的字段表也在主文件。

### insertWatermark

把文字或图片水印写入页眉 DrawingML。不写 VML。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "insertWatermark",
      "id": "docx:document",
      "text": "CONFIDENTIAL"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertWatermark"` |
| `id` | 必须是 `docx:document` |
| `text` | 与 `file` 恰好给一个。非空字符串，作为文字水印 |
| `file` | 与 `text` 恰好给一个。本地图片路径，规则与 `insertPicture` 相同 |

两个都给或两个都不给报 `InvalidField`。空 `text` 报 `InvalidField`。id 不是 `docx:document` 报 `InvalidId`。
xlsx / pptx 上发此 op 报 `InvalidField`。读不到 `file` 报 `Io`。

第一节没有 default 页眉时，按 `insertHeaderFooter` 同一套规则新建 default 页眉。不自动打开 `titlePg` / `evenAndOddHeaders`，不新建 even / first。
对文档里每一个已有页眉部件（default / even / first，不含页脚）：先删掉该部件内已有 DrawingML 水印，再写入一条新的。共享同一页眉部件的多节只写一次。

写出 `wp:anchor`，`behindDoc="1"`，`wp:wrapNone`，水平垂直相对 `page` 居中。
`wp:docPr/@name` 为 `ooxpWatermarkText` 或 `ooxpWatermarkPicture`，并写 `a:ext uri="{C0A5BD15-81BF-4B96-A0C4-0C0A5BD15001}"`。
不要 `mc:Fallback` 或 `w:pict`。文字水印用 `wps:wsp` 文本框，灰色字，`a:xfrm/@rot` 为 `18900000`（315 度）。
图片水印按 `insertPicture` 同一套规则把媒体写入 `/word/media/…`。

create 的 `document.watermarks` 复用同一套规则。不要 `id`。

### deleteWatermark

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteWatermark"` |
| `id` | 水印 id，或 `docx:document` |

`docx:document` 删掉所有页眉里的 DrawingML 水印；没有水印则空操作，仍计入 `opsApplied`。水印 id 找不到报 `IdNotFound`。
id 不是水印 id 也不是 `docx:document` 报 `InvalidId`。不删 VML。xlsx / pptx 上发此 op 报 `InvalidField`。图片水印删掉后，若媒体没有其它关系再指向则去掉该部件。

空单元格：目标格子在 XML 里还不存在时，`setCell` 按需插入 `<row>` 和 `<c>`，插在正确的行列顺序上。找不到工作表才报 `IdNotFound`。

省略 `r` 的行和单元格：SpreadsheetML 允许 `<row>` 与 `<c>` 不写 `r`，编号由出现顺序推出。apply 在改这张表之前，先把这类隐式编号按顺序算出来写成显式 `r`。
不这样做，后面插入新行会让它们的编号整体漂移，等于悄悄改了别的数据。这是语义等价的改写，Office 打开看不出区别。

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

清空：去掉 `<f>`、`<v>`、`<is>`，保留 `<c>` 上的 `r`、`s` 以及不认识的属性。这样常见数字格式和字体还能在。若清空后 `<c>` 只剩 `r`（没有 `s` 也没有其他属性），可以把整个 `<c>` 删掉。
目标格在 XML 里还不存在时，清空是空操作，仍计入 `opsApplied`；这不是错误，因为 `setCell` 对空格子本来就不报 `IdNotFound`。

改已有格子：保留 `s` 和未知属性。把文本从共享字符串改成新文本时，写成 `t="inlineStr"`，不要去改 `sharedStrings.xml` 里的旧项。改共享字符串会牵动所有引用同一索引的格子。

数字、布尔按类型改 `t` 和 `<v>`。写入公式时：`<f>` 存去掉前导 `=` 的文本，`t` 若原先是 `s`/`inlineStr`/`b` 则去掉，删除 `<v>` 和 `<is>`。

### setParaText

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.patched.docx",
  "ops": [
    {
      "op": "setParaText",
      "id": "para:3F2A1B00",
      "text": "新标题"
    }
  ]
}
```

- **`op`**
  - 必须是 `"setParaText"`
- **`id`**
  - 段落 id：正文用 `para:…` / `para:index:n`
  - 页眉页脚用 `header:…/para:…` 或 `footer:…/para:…`
  - 框内用 `{框 id}/para:…`
  - 注内用 `{注 id}/para:…`
- **`text`**
  - 新的可见纯文本，字符串，可空
  - 必填

找不到该段落报 `IdNotFound`。不插入、不删除段落。对页眉或页脚容器 id（没有 `/para:`）报 `InvalidId`。该段里的 `w:drawing` 必须留下，包括只剩绘图、已经被清掉 `w:t` 的 run。

替换规则（手术刀，不整篇重建目标部件）：

1. 只改命中的那一个 `w:p`。`w:pPr`、书签、绘图、`w:hyperlink` 外壳、不认识的子元素都留着。不删 `w:drawing`。
2. 在该段里找出不位于 `w:del` 中的、含 `w:t` 的 run。
3. 若一个都没有：在 `w:pPr` 之后插入一个新的 `w:r`/`w:t`，文本放进去。
4. 若有：把完整新文本写入其中第一个 `w:t`（需要保留首尾空格时加 `xml:space="preserve"`）；其余这类 `w:t` 删掉；因此变空、且里面再也没有别的 run 内容的 `w:r` 删掉。第一个 run 上已有的 `w:rPr` 保留。
5. 段内若落在本期收入的域范围内，走第 9 条，不按本条拆域。
6. 不往文本框里写字。第 2 步找 run 的时候，遇到 `w:txbxContent` 停住，遇到 `mc:AlternateContent` 只进第一个 `mc:Choice`（没有就进 `mc:Fallback`），判定与 inspect 那两条边界一致。
   某一段除文本框之外没有可改的 `w:t` 时走第 3 步插新 run，插在 `w:pPr` 之后、该段其余内容之前。这样 apply 写进去的文本，随后 inspect 读回来必须与写入值相同。
7. 段内存在本期收入的修订节点（run 级 `w:ins` / `w:del`、段落标记 `paraMarkIns` / `paraMarkDel`、`w:rPrChange`、`w:pPrChange`）时报 `NotImplemented`，不改 XML。
   先对该段上的修订发 `acceptRevision` / `rejectRevision`。表行与表属性修订不在 `w:p` 内，不挡 `setParaText`。
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
    {
      "op": "setHeaderFooterDistance",
      "id": "section:1",
      "headerDistance": 720,
      "footerDistance": 1440
    }
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
四边距与装订线请用 `setPageMargin`，不要塞进本 op。

### setPageMargin

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.margin.docx",
  "ops": [
    {
      "op": "setPageMargin",
      "id": "section:1",
      "top": 1440,
      "left": 1800,
      "right": 1800,
      "bottom": 1440
    }
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

五个边距至少给一个，否则报 `InvalidField`。不要出现 `headerDistance` / `footerDistance`。节不存在报 `IdNotFound`。
id 不是 `section:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

只改目标节 `w:pgMar` 上被点名的属性。没有 `w:pgMar` 时插入一个，只写本次给出的字段。已有的 `header` / `footer` 以及未点名的边距原样留下。

### setPageSize

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.size.docx",
  "ops": [
    {
      "op": "setPageSize",
      "id": "section:1",
      "pageWidth": 16838,
      "pageHeight": 11906,
      "orient": "landscape"
    }
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

三个字段至少给一个，否则报 `InvalidField`。不认识的 `orient` 报 `InvalidField`。只改被点名的属性。没有 `w:pgSz` 时插入一个，只写本次给出的字段，不补宽高，也不猜纵向。
改 `orient` 时不自动对调 `pageWidth` / `pageHeight`。不读写 `w:code`。

### setColumns

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.cols.docx",
  "ops": [
    {
      "op": "setColumns",
      "id": "section:1",
      "num": 2,
      "space": 720,
      "equalWidth": true
    }
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

`enabled: false` 与其它字段不能同时出现，否则报 `InvalidField`。其余字段至少给一个，否则报 `InvalidField`。`num` 小于 1 报 `InvalidField`。只改被点名的属性。
没有 `w:cols` 且不是删除时插入一个。给了 `cols` 却同时 `equalWidth` 为 `true` 报 `InvalidField`。

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
    {
      "op": "setHeaderFooterLink",
      "id": "section:2",
      "kind": "header",
      "type": "default",
      "linked": false
    }
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

第一节上 `linked: true` 报 `InvalidField`。该类型对本节不生效时报 `InvalidField`（不替用户打开首页不同或奇偶页）。节不存在报 `IdNotFound`。
id 不是 `section:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。已经是目标状态则空操作，仍计入 `opsApplied`，不脏任何部件。

`linked: false`：若前节有可继承的该类型部件，把该部件字节复制到新路径（`/word/headerN.xml` 或 `/word/footerN.xml`，`N` 取未占用序号）；
写入主文档关系、`[Content_Types].xml` 的 Override，以及本节一条引用。
若源部件有自己的 `.rels`，把该关系文件原样抄到新部件对应路径（`Target` 仍指向同一 `/word/media/…` 等，不复制媒体字节）。
若前节没有部件（空白），写一份最小 `w:hdr` / `w:ftr`（一个空 `w:p`），再挂引用。

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
    {
      "op": "insertHeaderFooter",
      "id": "section:1",
      "kind": "header",
      "type": "default"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertHeaderFooter"` |
| `id` | 节 id，必须是 `section:{n}` |
| `kind` | `header` 或 `footer` |
| `type` | `default`、`even` 或 `first` |

该类型对本节不生效时报 `InvalidField`（不替用户打开首页不同或奇偶页）。
本节该类型已经链接前节（inspect 的 `headerLink` / `footerLink` 为 `true`）时报 `InvalidField`；
要独立部件走 `setHeaderFooterLink` 且 `linked: false`。
节不存在报 `IdNotFound`。id 不是 `section:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

本节已有该类型显式引用：空操作，仍计入 `opsApplied`，不脏任何部件。

否则写一份最小 `w:hdr` / `w:ftr`（一个空 `w:p`），路径为未占用的 `/word/headerN.xml` 或 `/word/footerN.xml`，
写入主文档关系、`[Content_Types].xml` 的 Override，以及本节一条引用。不要从母版或前节抄内容。

### setTitlePage

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.title.docx",
  "ops": [
    {
      "op": "setTitlePage",
      "id": "section:1",
      "titlePage": true
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTitlePage"` |
| `id` | 节 id，必须是 `section:{n}` |
| `titlePage` | `true` 写入本节 `w:titlePg`；`false` 去掉该元素 |

不新建 first 页眉页脚。关掉时不删除已有 first 部件。已经是目标状态则空操作，仍计入 `opsApplied`，不脏任何部件。节不存在报 `IdNotFound`。
id 不是 `section:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### setEvenAndOddHeaders

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.even.docx",
  "ops": [
    {
      "op": "setEvenAndOddHeaders",
      "id": "docx:document",
      "evenAndOdd": true
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setEvenAndOddHeaders"` |
| `id` | 必须是 `docx:document` |
| `evenAndOdd` | `true` 在 `settings.xml` 写入 `w:evenAndOddHeaders`；`false` 去掉该元素 |

整篇一份，不按节。不新建 even 页眉页脚。关掉时不删除已有 even 部件。已经是目标状态则空操作，仍计入 `opsApplied`，不脏任何部件。没有 `settings.xml` 时报 `OpcPartMissing`。
id 不是 `docx:document` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

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

- **`op`**
  - 必须是 `"setPictureWrap"`
- **`id`**
  - 图 id
- **`kind`**
  - `inline` 或 `anchor`
- **`wrap`**
  - `kind` 为 `anchor` 时必填：`square` / `tight` / `through` / `topAndBottom` / `none`
  - `kind` 为 `inline` 时必须省略或为 `null`
- **`alignH`**
  - 可选
  - 相对页边水平对齐：`left` / `center` / `right`
  - 只在 `kind` 为 `anchor` 时有意义
- **`alignV`**
  - 可选
  - 相对页边垂直对齐：`top` / `center` / `bottom`
- **`offsetX`**
  - 可选
  - 相对页边水平偏移，EMU 整数（可负）
  - 与 `alignH` 不能同时出现
- **`offsetY`**
  - 可选
  - 相对页边垂直偏移，EMU 整数（可负）
  - 与 `alignV` 不能同时出现

行内改浮动或反过来。位置只支持相对页边（`wp:positionH/@relativeFrom` 与 `wp:positionV/@relativeFrom` 都写 `margin`），不做任意 Dist，也不写其它 `relativeFrom`。
改成浮动且没给对齐或偏移时，水平默认 `alignH: "left"`，垂直默认 `alignV: "top"`。已经是浮动、只改 `wrap` 时，未点名的位置属性原样留下。
`behindDoc`：`wrap` 为 `none` 时写成 `1`（衬于文字下方），其余为 `0`。xlsx / pptx 上发此 op 报 `InvalidField`。xlsx 改锚点请用 `setPictureAnchor`。
pptx 改画布坐标请用 `setPicturePos`。

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

### setTextBoxTextAnchor

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTextBoxTextAnchor"` |
| `id` | 框 id |
| `textAnchor` | `t` / `ctr` / `b` / `just` / `dist`，或 `null` 去掉属性 |

写入该框 `wps:bodyPr/@anchor`。这是框内文字相对框的上下锚点，不是框相对页边的 `setPictureWrap.alignV`，也不是格级 `w:vAlign`。
不要和 xlsx 的 `setTextBoxAnchor`（改绘图锚点）混。xlsx / pptx 上发此 op 报 `InvalidField`。xlsx / pptx 框内垂直仍用已有 `setShapeFormat.textAnchor`。

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

`deletePara` 的 `id` 必须是段落 id（正文、页眉页脚、表格单元格内、框内、注内或批注内）。
对 `docx:document`、页眉页脚容器、框 id、注容器 id 或批注容器 id 发 `deletePara` 报 `InvalidId`。

只插删 `w:p`，不插删 `w:tbl` / `w:tr` / `w:tc`。表格单元格内的段落跟本 op 走。插删表、行、列见 `insertTable` 等结构 op。

直接父节点里只剩一个 `w:p` 时 `deletePara` 报 `InvalidField`。正文、页眉、页脚、表格单元格、`w:txbxContent`、一条用户注、一条 `w:comment` 都适用。不允许删空文档或删空页眉页脚。

新段带新的 `w14:paraId`。批注内插删段后必须把该批注 `commentEx/@paraId` 改成新的最后一段 `paraId`；若有回复的 `paraIdParent` 指向旧的最后一段，一并改成新值。

删主文档里收集顺序的最后一段，且该段 `pPr` 带 `sectPr`（文档末节属性）时：把 `sectPr` 挪到删完后仍留下的最后一段上，再删。
该段不是收集顺序的最后一段、却带 `sectPr`（分节符）时，报 `NotImplemented`。

`deletePara` 时，若该段里的书签、批注范围或许可范围起止不成对（跨到段外），报 `NotImplemented`。
段内有 `w:ins`、`w:del`、`w:moveFrom` 或 `w:moveTo` 时同样报 `NotImplemented`。不要拆坏跨段标记。框内、注内、批注内的既有规则不变。

xlsx / pptx 上发此 op 报 `InvalidField`。

### insertSection

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.split.docx",
  "ops": [
    {
      "op": "insertSection",
      "id": "para:3F2A1B00",
      "type": "nextPage"
    }
  ]
}
```

- **`op`**
  - 必须是 `"insertSection"`
- **`id`**
  - 主文档 `w:body` 的直接子段落 id（插在该段之后），或 `docx:document`（在文档末尾插入新的空节）
- **`type`**
  - 可选
  - `nextPage`、`continuous`、`evenPage`、`oddPage`、`nextColumn`（`ST_SectionMark`）
  - 省略当作 `nextPage`
  - 写在新分节符所在的那个 `w:sectPr` 上

插在点名段之后：该段成为当前节的最后一段。该段还没有 `pPr/sectPr` 时，把分节符写在该段上。该段已经带 `sectPr` 时，在它后面插入一个空的 `w:p`（新 `w14:paraId`），把新分节符放在这个空段上。不要拆 run，不要拆段落中间。

节属性副本克隆该段当前所属节的 `w:sectPr`（含 `pgSz` / `pgMar` / `cols` / `titlePg` / `footnotePr` 等）。分节符之前的那一节保留原来的页眉、页脚引用。
分节符之后的那一节省略 `headerReference` / `footerReference`，inspect 的 `headerLink` / `footerLink` 按已有规则为 `true`（第一节除外）。不要把引用抄到新节上。

点名段是当前最后一节的最后一段时：该段写上带引用的 `sectPr`，并在 `w:body` 的 `sectPr` 之前再插一个空的 `w:p`，保证末节至少有一段。
`id` 为 `docx:document` 时：已有正文留在原末节，新节至少有一个空段。

该段之后、下一 `sectPr` 之前的块级内容（含 `w:tbl`）属于新节。最后一节的 `sectPr` 仍在 `w:body` 末尾，中间节在段末，不要写反。插完之后 `section:{n}` 按新的文档顺序重编。

表格单元格内、页眉页脚、框内、注内、批注内的段落，以及 `section:{n}`、容器 id、表 id、run id，一律报 `InvalidId`。找不到段落报 `IdNotFound`。
不认识的 `type` 报 `InvalidField`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteSection

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.merge.docx",
  "ops": [
    {
      "op": "deleteSection",
      "id": "section:1"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteSection"` |
| `id` | 节 id，必须是 `section:{n}`。`n` 不能是最后一节 |

删掉结束第 `n` 节的那个 `w:sectPr`（一定在某段 `pPr` 上，不会是 `w:body` 末尾那一份）。第 `n` 节的正文并进第 `n+1` 节，沿用第 `n+1` 节的节属性。这与 Word 删除分节符的方向相同。

只剩一节，或对最后一节发此 op，报 `InvalidField`。节不存在报 `IdNotFound`。id 不是 `section:{n}` 报 `InvalidId`。

删完后，若某个 header/footer 部件没有任何节的 `headerReference` / `footerReference` 再用，
按 `setHeaderFooterLink` 且 `linked: true` 的既有逻辑删部件、主文档关系、该部件 `.rels`、Override。
媒体仍被其它页眉引用则留下。xlsx / pptx 上发此 op 报 `InvalidField`。

`deletePara` 现有规则不变：非末段带 `sectPr` 仍报 `NotImplemented`。要拆掉分节符请用 `deleteSection`。

示例见 `docs/examples/docx-apply-insert-section.json`。create 多节见 `docs/examples/docx-create-sections.json`。

### mergeCells

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"mergeCells"` |
| `id` | 表 id |
| `startRow` / `startCol` / `endRow` / `endCol` | 闭区间，从 1 起。起大于止报 `InvalidField` |

范围内已有合并、或格子对不上网格时报 `NotImplemented`。幸存格是左上角：水平方向写 `gridSpan`，垂直方向左上为 `vMerge=restart`、其余行为 `continue`。
被吃掉的格子里的段落（遇嵌套表停住）按先行后列挪进幸存格，然后删掉那些 `w:tc`。`continue` 格留下一个空的 `w:p`。xlsx / pptx 上发此 op 报 `InvalidField`。

### splitCells

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"splitCells"` |
| `id` | 单元格 id |

`gridSpan` 为 1 且没有 `vMerge` 时报 `InvalidField`。对 `continue` 格发此 op 报 `InvalidField`，必须点名 `restart` 那一格。
水平拆分后补出的新格各有一段空文本；垂直拆分去掉该列向下连续的 `vMerge`。xlsx / pptx 上发此 op 报 `InvalidField`。

表、行或单元格上出现 `w:ins` / `w:del` / `w:moveFrom` / `w:moveTo`，
或 `tblPrChange` / `trPrChange` / `tcPrChange` / `tblGridChange` / `tblPrExChange` 时，上述结构 op 报 `NotImplemented`，不写文件。
某行占的网格列之和对不上 `colCount` 时，`insertCol` / `deleteCol` / `mergeCells` / `splitCells` / `insertRow` 同样报 `NotImplemented`。
先对点名的修订发 `acceptRevision` / `rejectRevision`。

### acceptRevision

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "acceptRevision",
      "id": "rev:1"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"acceptRevision"` |
| `id` | 修订 id |

不调 Word COM。run 级 `ins`：拆掉 `w:ins` 外壳，留下内部 `w:r`，并去掉这些 run 的属性容器上的 `ins` / `del`（Word 会另写在 `rPr` 里，不进树）。
run 级 `del`：删掉整个 `w:del`（含 `delText`）。run 级 `moveFrom`：删掉该节点（含内容），并删掉包住它的同 `w:id` 的 `moveFromRangeStart` / `moveFromRangeEnd`。
run 级 `moveTo`：拆掉 `w:moveTo` 外壳，留下内部 `w:r`，并删掉包住它的同 `w:id` 的 `moveToRangeStart` / `moveToRangeEnd`。
`rPrChange` / `pPrChange` / `tblPrChange` / `tblGridChange` / `tblPrExChange`
 / `tcPrChange` / `trPrChange` / `sectPrChange`：删掉 Change 元素，留下当前属性。
`paraMarkIns`：去掉该空 `ins`，留下该段。`paraMarkDel`：删掉该 `w:p`。`rowDel`：删掉该 `w:tr`（该行上其余修订节点一并消失）。
`rowIns`：去掉 `trPr` 上的 `ins`，并把该行内 run 级 `ins` 拆壳、伴随段落标记去掉。接受 `rowDel` 或拒绝 `rowIns` 时若该表只剩该行，报 `InvalidField`。
拒绝 `paraMarkIns` 或接受 `paraMarkDel` 时若该容器只剩该段，报 `InvalidField`；该段带 `w:sectPr` 或跨段范围标记时报 `NotImplemented`。
找不到报 `IdNotFound`。id 不是修订 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### rejectRevision

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"rejectRevision"` |
| `id` | 修订 id |

run 级 `ins`：删掉整个 `w:ins`。run 级 `del`：拆掉 `w:del` 外壳，把其中 `w:delText` 改回 `w:t`，并去掉内部属性容器上的 `ins` / `del`。
run 级 `moveFrom`：拆掉 `w:moveFrom` 外壳，留下内部 `w:r`，并删掉包住它的 RangeStart/End。
run 级 `moveTo`：删掉该节点（含内容），并删掉包住它的 RangeStart/End。
`rPrChange`：用 Change 里的旧 `w:rPr` 替换当前 run 属性。`pPrChange`：用 Change 里的旧 `w:pPr` 替换当前段落属性。
其余 `*Change`：用 Change 里的旧属性容器（`tblPr` / `tblGrid` / `tblPrEx` / `tcPr` / `trPr` / `sectPr`）替换当前容器。
`paraMarkIns`：删掉该 `w:p`。`paraMarkDel`：去掉该空 `del`。`rowIns`：删掉该 `w:tr`。
`rowDel`：去掉 `trPr` 上的 `del`，并把该行内 run 级 `del` 还原、伴随段落标记去掉。
删整段 / 整行时的 `InvalidField` / `NotImplemented` 与 `acceptRevision` 相同。找不到报 `IdNotFound`。id 不是修订 id 报 `InvalidId`。
xlsx / pptx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/docx-apply-revision.json`。

### insertEditRange

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertEditRange"` |
| `id` | 段落 id（正文或页眉页脚） |
| `edGrp` | 可选。`everyone` / `administrators` / `contributors` / `editors` / `owners` / `current` |
| `ed` | 可选。编辑者别名 |

`edGrp` 与 `ed` 不能同时给。两个都不给时写 `w:edGrp="everyone"`。不认识的 `edGrp` 或空 `ed` 报 `InvalidField`。
在该段 `w:pPr` 之后插入 `w:permStart`，在段末插入配对 `w:permEnd`。`w:id` 取该部件已有 perm 标记里最大整数加 1，至少为 0。
框内、注内、批注内的段落，以及非段落 id，报 `InvalidId`。找不到段落报 `IdNotFound`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteEditRange

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteEditRange"` |
| `id` | 例外区 id |

删掉该对 `permStart` / `permEnd`，不删文字。找不到报 `IdNotFound`。id 不是 `perm:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/docx-apply-edit-range.json`。

### insertBookmark

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertBookmark"` |
| `id` | 起始段落 id（正文、页眉页脚或框内） |
| `name` | 可选。书签名。省略则用尚未占用的 `ooxp{n}`（n 从 1 起） |
| `end` | 可选。结束段落 id。省略则与 `id` 相同 |

在起始段 `w:pPr` 之后插入 `w:bookmarkStart`，在结束段末尾插入配对 `w:bookmarkEnd`。`w:id` 取该部件已有书签 id 的最大整数加 1，至少为 0。
`name` 不能为空、不能含空白、不能以 `_` 开头，否则报 `InvalidField`。与已有书签名（含隐藏书签）冲突报 `InvalidField`。
`end` 必须与 `id` 在同一故事，且文档顺序不得早于起始段，否则报 `InvalidField`。找不到段落报 `IdNotFound`。id / `end` 不是段落 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### setBookmark

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setBookmark"` |
| `id` | 书签 id |
| `name` | 新书签名 |

改 `w:bookmarkStart/@w:name`。同时把本部件里 `w:hyperlink/@w:anchor` 等于旧名的改成新名。新名规则与 `insertBookmark` 相同。与其他书签重名报 `InvalidField`。
已经是该名则空操作，仍计入 `opsApplied`。找不到报 `IdNotFound`。id 不是书签 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteBookmark

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteBookmark"` |
| `id` | 书签 id |

去掉配对的 `w:bookmarkStart` 与 `w:bookmarkEnd`。不改指向该书签的 `w:hyperlink/@w:anchor`。找不到报 `IdNotFound`。id 不是书签 id 报 `InvalidId`。
xlsx / pptx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/docx-apply-hyperlink.json`、`docs/examples/docx-apply-drawing-hlink.json`。
框内链、书签、域、公式、拼音与控件见 `docs/examples/docx-apply-box-fields.json`。

### insertField

- **`op`**
  - 必须是 `"insertField"`
- **`id`**
  - 段落 id（正文、页眉页脚或框内）
- **`instr`**
  - 与 `code` 恰好一个
  - 去掉首尾空白后不能为空
- **`code`**
  - 与 `instr` 恰好一个
  - 大小写不敏感：
    - `PAGE`
    - `NUMPAGES`
    - `DATE`
    - `TIME`
    - `FILENAME`
    - `AUTHOR`
    - `LISTNUM`
    - `DOCPROPERTY`
    - `MERGEFIELD`
    - `STYLEREF`
    - `REF`
    - `FORMTEXT`
    - `FORMCHECKBOX`
    - `FORMDROPDOWN`
- **`arg`**
  - `MERGEFIELD` / `STYLEREF` / `REF` / `DOCPROPERTY` 必填非空
  - `LISTNUM` 可选
  - 其它 `code` 不要此字段
- **`name`**
  - 仅窗体三类
  - 可选
  - 缺省 `Text1` / `Check1` / `Drop1`
- **`checked`**
  - 仅 `FORMCHECKBOX`
  - 可选布尔
- **`items`**
  - 仅 `FORMDROPDOWN`
  - 可选字符串数组
- **`result`**
  - 可选
  - 写入的缓存结果
  - 省略则没有结果 run（仍写 `separate`）

在该段已有内容之后插入一组复杂域：`fldChar begin`（`w:dirty="true"`）、一条 `instrText`（`xml:space="preserve"`）、`fldChar separate`、
若给了非空 `result` 则再写一个带 `w:t` 的 run、`fldChar end`。
给了窗体 `code` 时在 begin 里写 `w:ffData`，不写书签。不在库里求值。
空 `instr` / 空 `code` / 同时给两者 / 两个都不给 / 不认识的 `code` / 缺 `arg`，报 `InvalidField`。
`INCLUDETEXT` / `INCLUDEPICTURE` / `ASK` / `FILLIN` 报 `InvalidField`。找不到段落报 `IdNotFound`。id 不是段落 id 报 `InvalidId`。
xlsx 上发此 op 报 `InvalidField`。pptx 用同一 op 名写 `a:fld`，字段见 [json-contract-pptx.md](json-contract-pptx.md#insertfield)。

### setField

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setField"` |
| `id` | 域 id |
| `instr` | 新的指令字符串，必填。去掉首尾空白后不能为空 |

只改被点名那一域的指令，不改缓存结果，也不改 `ffData`。简单域写 `w:instr`。复杂域删掉本层指令区里的旧 `instrText` 以及落在指令区里的嵌套域，在 begin 之后插入一条新的 `instrText`。
结果区（separate 与 end 之间）不动。不在库里求值。`instr` 缺或只含空白报 `InvalidField`。找不到报 `IdNotFound`。id 不是域 id 报 `InvalidId`。
xlsx 上发此 op 报 `InvalidField`。pptx 用同一 op 名改 `a:fld` 的 `type` / 缓存，
见 [json-contract-pptx.md](json-contract-pptx.md#setfield)。

### deleteField

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteField"` |
| `id` | 域 id |
| `keep` | 可选。`result`（缺省）或 `instr`。其他值报 `InvalidField` |

`keep` 为 `result`：拆掉该域外壳与指令，留下缓存结果。指令区里的嵌套域一并删掉；结果区里的嵌套域留下。简单域拆掉 `w:fldSimple`，留下子节点。
`keep` 为 `instr`：把本层指令变成普通可见文本（`instrText` 改成 `w:t`，简单域则写成一个带指令原文的 run），删掉 begin / separate / end 与结果区。指令区里的嵌套域留下。
找不到报 `IdNotFound`。id 不是域 id 报 `InvalidId`。xlsx 上发此 op 报 `InvalidField`。
pptx 用同一 op 名拆 `a:fld`，见 [json-contract-pptx.md](json-contract-pptx.md#deletefield)。

示例见 `docs/examples/docx-apply-field.json`。
框内域见 `docs/examples/docx-apply-box-fields.json`。

### insertToc

- **`op`**
  - 必须是 `"insertToc"`
- **`id`**
  - 主文档 `w:body` 的直接子段落 id（插在该段之后），或 `docx:document`（插到正文末尾、`sectPr` 之前）
- **`kind`**
  - 可选
  - `toc`（目录）、`tof`（图表目录）、`toa`（引文目录）
  - 省略且没有 `instr` 时当作 `toc`
- **`instr`**
  - 可选
  - 域指令原文
  - 给了则按此写入，第一个词必须是 `TOC` 或 `TOA`（大小写不敏感）
  - 省略时：`toc` 为 ` TOC \o "1-3" \h \z \u`，`tof` 为 ` TOC \c "Figure" \h \z \u`，`toa` 为 ` TOA \h`

在点名段之后插入一个新的 `w:p`（新 `w14:paraId`），里面写一组复杂域：`fldChar begin`（`w:dirty="true"`）、一条 `instrText`、`fldChar separate`、`fldChar end`。
不写结果 run，不在库里根据标题算页码。不包 `w:sdt`。表格单元格内、页眉页脚、框内、注内、批注内的段落，以及 `section:{n}`、`toc:{n}`、容器 id、表 id、run id，一律报 `InvalidId`。
找不到段落报 `IdNotFound`。不认识的 `kind`、空白 `instr`、或 `instr` 不是 TOC/TOA，报 `InvalidField`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteToc

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteToc"` |
| `id` | 目录 id，必须是 `toc:{n}` |

删掉该目录块。若该 TOC 域落在 `w:sdt` 里，且 `sdtPr` 的 `docPartGallery` 为 `Table of Contents` / `Table of Figures`
 / `List of Figures` / `Table of Authorities`，
并且该 `sdt` 是 `w:body` 的直接子节点，则删掉整个 `sdt`（含「目录」标题段）。
否则删掉该域从 begin 到 end 的全部 run（含结果区里的嵌套域），再删掉因此变空、且没有 `sectPr` 的正文段落。不要留下空的目录段。删完后若 `w:body` 里没有 `w:p`，在 `sectPr` 之前补一个空段。
某段带分节符且不能整段删掉时，报 `NotImplemented`。找不到报 `IdNotFound`。id 不是 `toc:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

`deleteField` 现有规则不变：对 TOC 域发 `deleteField` 仍只拆外壳。要整块去掉请用 `deleteToc`。

示例见 `docs/examples/docx-apply-insert-toc.json`。

### insertCaption

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertCaption"` |
| `id` | 主文档 `w:body` 的直接子段落 id（插在该段之后），或 `docx:document`（插到正文末尾、`sectPr` 之前） |
| `label` | 可选。SEQ 序列名，默认 `Figure`。必须是拉丁字母开头、其后最多 39 个拉丁字母 / 数字 / 下划线，总长不超过 40（ECMA-376 §17.16.5.56） |
| `title` | 可选。写在书签之后的说明文字。省略或空串则不写 |
| `bookmark` | 可选。包住标签与 SEQ 的书签名。省略则写成尚未占用的 `_Ref` 加九位数字。允许以下划线开头。空串、含空白、或与文件里已有书签重名报 `InvalidField` |

在点名段之后插入一个新的 `w:p`（新 `w14:paraId`）：`bookmarkStart`、标签 run（`{label} ` 后带一个空格）、
一组复杂 SEQ 域（`fldChar begin` 带 `w:dirty="true"`、指令 ` SEQ {label} \* ARABIC `、`separate`、`end`，
不写结果 run）、`bookmarkEnd`、若给了 `title` 再写说明 run（若 `title` 不是以空白开头则前面补一个空格）。
不写 Caption 段落样式。不在库里求序号。表格单元格内、页眉页脚、框内、注内、批注内的段落，以及 `section:{n}`、`caption:{n}`、`toc:{n}`、容器 id、表 id、run id，一律报 `InvalidId`。
找不到段落报 `IdNotFound`。不合法的 `label` 报 `InvalidField`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteCaption

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteCaption"` |
| `id` | 题注 id，必须是 `caption:{n}` |

若该 SEQ 落在 `w:body` 的直接子段落上，则删掉整段（题注就是这一段）。某段带分节符且不能整段删掉时，报 `NotImplemented`。
否则只拆掉该 SEQ 域（含缓存）以及包住它的 `_Ref*` 书签，不删格子或页眉段落。删完后若 `w:body` 里没有 `w:p`，在 `sectPr` 之前补一个空段。找不到报 `IdNotFound`。
id 不是 `caption:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

`deleteField` 现有规则不变：对 SEQ 发 `deleteField` 仍只拆外壳。要整段去掉请用 `deleteCaption`。

### insertCrossRef

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertCrossRef"` |
| `id` | 段落 id（正文或页眉页脚） |
| `bookmark` | 必填。目标书签名，可以以下划线开头（题注的 `_Ref*`）。必须在主文档里已有对应 `w:bookmarkStart` |
| `kind` | 可选。`ref`（缺省）写入 ` REF {bookmark} \h `；`page` 写入 ` PAGEREF {bookmark} \h `。其它值报 `InvalidField` |

在该段已有内容之后插入一组复杂域：`fldChar begin`（`w:dirty="true"`）、一条 `instrText`、`fldChar separate`、`fldChar end`。不写结果 run，不在库里解析书签内容。
本期不写 `NOTEREF`。`bookmark` 缺、空串或含空白报 `InvalidField`。主文档里没有该名报 `IdNotFound`。找不到段落报 `IdNotFound`。id 不是段落 id 报 `InvalidId`。
xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteCrossRef

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteCrossRef"` |
| `id` | 交叉引用 id，必须是 `xref:{n}` |

拆掉该 REF / PAGEREF / NOTEREF 域（含指令与缓存），不删所在段落。找不到报 `IdNotFound`。id 不是 `xref:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/docx-apply-caption-xref.json`。

### insertIndex

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertIndex"` |
| `id` | 主文档 `w:body` 的直接子段落 id（插在该段之后），或 `docx:document`（插到正文末尾、`sectPr` 之前） |
| `instr` | 可选。域指令原文。给了则按此写入，第一个词必须是 `INDEX`（大小写不敏感）。省略时为 ` INDEX \h "A" \c "1"` |

在点名段之后插入一个新的 `w:p`（新 `w14:paraId`），里面写一组复杂域：`fldChar begin`（`w:dirty="true"`）、一条 `instrText`、`fldChar separate`、`fldChar end`。
不写结果 run，不在库里根据 XE 算页码。不包 `w:sdt`，不写 Index 样式，不改 `sectPr` 分栏。
表格单元格内、页眉页脚、框内、注内、批注内的段落，以及 `section:{n}`、`index:{n}`、`toc:{n}`、容器 id、表 id、run id，一律报 `InvalidId`。找不到段落报 `IdNotFound`。
空白 `instr`、或 `instr` 不是 INDEX，报 `InvalidField`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteIndex

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteIndex"` |
| `id` | 索引 id，必须是 `index:{n}` |

删掉该 INDEX 域从 begin 到 end 的全部 run（含结果区里的嵌套域），再删掉因此变空、且没有 `sectPr` 的正文段落。不要留下空的索引段。
删完后若 `w:body` 里没有 `w:p`，在 `sectPr` 之前补一个空段。某段带分节符且不能整段删掉时，报 `NotImplemented`。找不到报 `IdNotFound`。
id 不是 `index:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

`deleteField` 现有规则不变：对 INDEX 域发 `deleteField` 仍只拆外壳。要整块去掉请用 `deleteIndex`。

### insertIndexEntry

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertIndexEntry"` |
| `id` | 段落 id（正文或页眉页脚） |
| `entry` | 必填。写入 ` XE "{entry}" `。允许冒号表示次级项（ECMA-376 §17.16.5.72）。空串、只含空白、或含英文双引号报 `InvalidField` |

在该段已有内容之后插入一组复杂域：`fldChar begin`、一条 `instrText`、`fldChar end`。
不写 `separate`（与 Word 16.0 `Indexes.MarkEntry` 一致），不写 `w:vanish`，不写结果 run。不在库里求页码。
框内、注内、批注内的段落，以及索引 id、容器 id、表 id、run id，一律报 `InvalidId`。找不到段落报 `IdNotFound`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteIndexEntry

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteIndexEntry"` |
| `id` | 索引项 id，必须是 `xe:{n}` |

拆掉该 XE 域（含指令），不删所在段落。找不到报 `IdNotFound`。id 不是 `xe:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/docx-apply-index.json`。

### insertSource

- **`op`**
  - 必须是 `"insertSource"`
- **`id`**
  - 必须是 `docx:document`
- **`tag`**
  - 必填
  - 写入 `b:Tag`
  - 空串、只含空白、含空白或英文双引号、或与已有源重名，报 `InvalidField`
- **`sourceType`**
  - 可选
  - 默认 `Book`
  - 必须是 ECMA-376 `ST_SourceType` 里本期认的值
    - `Book`
    - `BookSection`
    - `JournalArticle`
    - `ArticleInAPeriodical`
    - `ConferenceProceedings`
    - `Report`
    - `InternetSite`
    - `DocumentFromInternetSite`
    - `ElectronicSource`
    - `Art`
    - `SoundRecording`
    - `Performance`
    - `Film`
    - `Interview`
    - `Patent`
    - `Case`
    - `Misc`
- **`title`**
  - 必填
  - 非空
  - 写入 `b:Title`
- **`year`**
  - 可选
- **`last` / `first`**
  - 可选
  - 作者姓 / 名
  - 给了任一侧就写一个 `b:Person`
- **`city`**
  - 可选
- **`publisher`**
  - 可选

没有 bibliography customXml 时新建 `/customXml/itemN.xml`（空 `b:Sources`，`SelectedStyle` 为 `\APASixthEditionOfficeOnline.xsl`，
`StyleName` 为 `APA`，`Version` 为 `6`）、`itemPropsN.xml`（`schemaRef` 为 bibliography 命名空间）、`itemN.xml.rels`，以及主文档 `/customXml` 关系。
只给 itemProps 写 Content Type Override。追加一个 `b:Source`，并写 `b:RefOrder` 为现有源个数加 1。不在库里按样式表排版。
id 不是 `docx:document` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteSource

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteSource"` |
| `id` | 书目源 id，必须是 `source:{n}` |

删掉该 `b:Source`，按剩余源重写 `b:RefOrder`（从 1 起）。不删 CITATION / BIBLIOGRAPHY 域。空了也留下 `b:Sources` 部件。找不到报 `IdNotFound`。
id 不是 `source:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### setBibliographyStyle

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setBibliographyStyle"` |
| `id` | 必须是 `docx:document` |
| `styleName` | 必须。`APA` / `MLA` / `Chicago` / `IEEE` / `GB7714` |

按 Word 16.0 另存写 `b:Sources` 的三个属性：

- `APA`：`SelectedStyle=\APASixthEditionOfficeOnline.xsl`，`Version=6`
- `MLA`：`\MLASeventhEditionOfficeOnline.xsl`，`Version=7`
- `Chicago`：`\CHICAGO.XSL`，`Version=15`
- `IEEE`：`\IEEE2006OfficeOnline.xsl`，`Version=2006`
- `GB7714`：`\GB.XSL`，`Version=2005`

没有 bibliography customXml 时按 `insertSource` 那套新建空 `b:Sources` 再改属性。不认识的 `styleName` 报 `InvalidField`。不跑 XSL。
id 不是 `docx:document` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### insertCitation

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertCitation"` |
| `id` | 段落 id（正文或页眉页脚） |
| `tag` | 必填。必须已在书目源里 |

在该段已有内容之后插入一行内 `w:sdt`：`sdtPr` 含 `w:id` 与空的 `w:citation`，
`sdtContent` 里写一组复杂域（`fldChar begin` 带 `w:dirty="true"`、指令 ` CITATION {tag} `、`separate`、`end`）。
不写结果 run，不写 `dataBinding`。不在库里排版引文。主文档里没有该 `tag` 报 `IdNotFound`。空串、只含空白或含英文双引号的 `tag` 报 `InvalidField`。框内、注内、批注内本期不编。
xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteCitation

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteCitation"` |
| `id` | 引文 id，必须是 `cite:{n}` |

若该 CITATION 包在带 `w:citation` 的 `w:sdt` 里，删掉整个 sdt；否则只拆掉该域。不删所在段落，也不删书目源。找不到报 `IdNotFound`。
id 不是 `cite:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### insertBibliography

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertBibliography"` |
| `id` | 主文档 `w:body` 的直接子段落 id（插在该段之后），或 `docx:document`（插到正文末尾、`sectPr` 之前） |
| `instr` | 可选。给了则第一个词必须是 `BIBLIOGRAPHY`。省略时为 ` BIBLIOGRAPHY` |

在点名段之后插入一个新的 `w:p`，里面写一组复杂域：`fldChar begin`（`w:dirty="true"`）、一条 `instrText`、`fldChar separate`、`fldChar end`。
不写结果 run，不包 `w:sdt`，不写 Bibliography 样式。不在库里按源排版。表格单元格内、页眉页脚、框内、注内、批注内的段落报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteBibliography

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteBibliography"` |
| `id` | 书目块 id，必须是 `biblio:{n}` |

删掉该 BIBLIOGRAPHY 域从 begin 到 end 的全部 run，再删掉因此变空、且没有 `sectPr` 的正文段落。删完后若 `w:body` 里没有 `w:p`，补一个空段。
某段带分节符且不能整段删掉时，报 `NotImplemented`。找不到报 `IdNotFound`。id 不是 `biblio:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

`deleteField` 现有规则不变：对 CITATION / BIBLIOGRAPHY 发 `deleteField` 仍只拆外壳。

示例见 `docs/examples/docx-apply-biblio.json`。

### insertDocPart

- **`op`**
  - 必须是 `"insertDocPart"`
- **`id`**
  - 必须是 `docx:document`
- **`name`**
  - 必填
  - 写入 `w:name/@w:val`
  - 空串或只含空白报 `InvalidField`
  - 与已有条目的 `name` + `gallery` + `category` 相同报 `InvalidField`
- **`text`**
  - 必填
  - 非空
  - 写成 `docPartBody` 里一条 `w:p`
- **`gallery`**
  - 可选
  - 默认 `custQuickParts`
  - 必须是 ECMA-376 `ST_DocPartGallery`
    - `placeholder`
    - `any`
    - `default`
    - `docParts`
    - `coverPg`
    - `eq`
    - `ftrs`
    - `hdrs`
    - `pgNum`
    - `tbls`
    - `watermarks`
    - `autoTxt`
    - `txtBox`
    - `pgNumT`
    - `pgNumB`
    - `pgNumMargins`
    - `tblOfContents`
    - `bib`
    - `custQuickParts`
    - `custCoverPg`
    - `custEq`
    - `custFtrs`
    - `custHdrs`
    - `custPgNum`
    - `custTbls`
    - `custWatermarks`
    - `custAutoTxt`
    - `custTxtBox`
    - `custPgNumT`
    - `custPgNumB`
    - `custPgNumMargins`
    - `custTblOfContents`
    - `custBib`
    - `custom1`
    - `custom2`
    - `custom3`
    - `custom4`
    - `custom5`
- **`category`**
  - 可选
  - 默认 `General`
  - 写入 `w:category/w:name/@w:val`
- **`behavior`**
  - 可选
  - 默认 `content`
  - 只认 `content`、`p`、`pg`（`ST_DocPartBehavior`）

没有 glossary 时新建 `/word/glossary/document.xml`（空 `w:docParts`），
并写卫星 `styles.xml` / `settings.xml` / `webSettings.xml` / `fontTable.xml`、glossary `.rels`、
主文档 `glossaryDocument` 关系与 Content Type Override。
`styles.xml` / `settings.xml` 复制自主文档对应部件；没有则写最小 styles / settings。`webSettings.xml` 与 `fontTable.xml` 写最小空壳。
GUID 新生成，写成 `{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}`。不写 `w:types`，不写 `w:style`。不实现 Word 构建基块库 UI。
id 不是 `docx:document` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteDocPart

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteDocPart"` |
| `id` | 构建基块 id，必须是 `docPart:{n}` |

删掉该 `w:docPart`。删光最后一条则去掉 glossary 部件、卫星、主文档关系与 Override，不要留空 `w:docParts`。找不到报 `IdNotFound`。
id 不是 `docPart:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### insertDocPartContent

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertDocPartContent"` |
| `id` | 主文档 `w:body` 的直接子段落 id（插在该段之后），或 `docx:document`（插到正文末尾、`sectPr` 之前） |
| `docPart` | 必须是 `docPart:{n}` |

把该条目 `docPartBody` 的块级子节点复制进主文档。跳过 `sectPr` 与末尾空段。给新的 `w:p` 分配新 `w14:paraId`。
`behavior` 为 `pg` 时先插一条只含 `w:br/@w:type="page"` 的段。`content` 与 `p` 都按块级兄弟插入（不把末段 run 并进点名段）。
表格单元格内、页眉页脚、框内、注内、批注内的段落报 `InvalidId`。找不到基块报 `IdNotFound`。xlsx / pptx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/docx-apply-glossary.json`。

### insertEmbeddedFont

- **`op`**
  - 必须是 `"insertEmbeddedFont"`
- **`id`**
  - 必须是 `docx:document`
- **`name`**
  - 必填
  - 写入 `w:font/@w:name`
  - 空串或只含空白报 `InvalidField`
- **`file`**
  - 必填
  - 本地 TrueType / OpenType 字体路径（`.ttf` / `.otf`）
  - 集合字体（`ttcf`）报 `InvalidField`
  - 长度不足 32 字节、或魔数不是 `00 01 00 00` / `OTTO` / `true` 报 `InvalidField`
- **`style`**
  - 可选
  - 默认 `regular`
  - 只认 `regular`、`bold`、`italic`、`boldItalic`，对应 `w:embedRegular` / `w:embedBold` / `w:embedItalic` / `w:embedBoldItalic`
  - 不认识报 `InvalidField`

出现 `subsetted` 报 `UnknownField`。本期不写 `w:subsetted`，不在库里做 TTF 子集化。同一 `name` + `style` 已有嵌入面报 `InvalidField`。

没有 fontTable 时新建 `/word/fontTable.xml`（空 `w:fonts`，
带 `xmlns:w` 与 `xmlns:r`）、`/word/_rels/fontTable.xml.rels`、主文档 `fontTable` 关系、
Content Type Override 与 `Default Extension="odttf"`。
按 ECMA-376 §17.8.1 混淆后写入 `/word/fonts/fontN.odttf`：新生成 16 字节 GUID，
`w:fontKey` 写成 `{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}`（字节按顺序十六进制），把这 16 字节整体反转后与字体文件前 32 字节 XOR（0–15 一次、16–31 再一次）。
已有同名 `w:font` 时把对应 embed* 挂到该条目上；没有则追加只含 `w:name` 与该 embed* 的最小 `w:font`。
同时在 `settings.xml` 写入空元素 `<w:embedTrueTypeFonts/>`（若无）。不新建 footnotes / theme / webSettings。不改用户本机字体库。
id 不是 `docx:document` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/docx-apply-font-embed.json`。

### deleteEmbeddedFont

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteEmbeddedFont"` |
| `id` | 嵌入字体 id，必须是 `font:{n}` |

删掉该 embed*、对应 font 部件及 fontTable 关系。所属 `w:font` 条目留下（即使再也没有 embed*）。fontTable 本身不因删光嵌入面而拆掉。找不到报 `IdNotFound`。
id 不是 `font:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### setFontEmbed

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setFontEmbed"` |
| `id` | 必须是 `docx:document` |
| `trueType` | 可选。布尔。写入 `w:embedTrueTypeFonts`：`true` 写空元素，`false` 删掉该元素 |
| `systemFonts` | 可选。布尔。写入 `w:embedSystemFonts` |
| `subset` | 可选。布尔。写入 `w:saveSubsetFonts` |

三者至少给一个，否则报 `InvalidField`。未点名的标志留下。本 op 不自动嵌字体、不子集化、不改 fontTable。
没有 settings 且要写成 `true` 时新建 `/word/settings.xml` 与主文档 settings 关系。id 不是 `docx:document` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。


### setPageBackground

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "setPageBackground",
      "id": "docx:document",
      "color": "FF0000"
    }
  ]
}
```

- **`op`**
  - 必须是 `"setPageBackground"`
- **`id`**
  - 必须是 `docx:document`
- **`color`**
  - 必填
  - 六位 RGB 十六进制，或 `null`
  - 字符串写入主文档 `w:background/@w:color`（写出大写），并在 `settings.xml` 写入空元素 `<w:displayBackgroundShape/>`
  - `null` 删掉 `w:background` 与 `displayBackgroundShape`

缺 `color`、不是字符串也不是 `null`、或非空却不是六位十六进制，报 `InvalidField`。不写 `themeColor`。
可选 `fill`：省略则只改 `@w:color`，并删掉已有 `v:background` 与 `w:drawing`。
`fill` 必须是对象，字段只认 `kind`、`color`、`file`。`kind` 必填，只能是 `gradient` 或 `picture`。
`kind` 为 `gradient`：写入 `w:drawing` / `wp:anchor` / `wps:wsp` 矩形，`a:gradFill` 两停点为 `color`（第一停）与 `fill.color`（第二停，省略为 `FFFFFF`）。
`kind` 为 `picture`：必须给 `file`（本地路径），写入 VML `v:background` / `v:fill`（`type="frame"`）与主文档 image 关系。
`color` 为 `null` 时不能带 `fill`，并删掉 `w:background`、填充与 `displayBackgroundShape`。
出现 `art` 或其它未知字段报 `UnknownField`。
不新建 footnotes / theme / webSettings / fontTable。没有 settings 且要写成显示时新建 `/word/settings.xml` 与主文档 settings 关系。
id 不是 `docx:document` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。create 给了 `document.pageBackground` 时按同一套规则写入。

示例见 `docs/examples/docx-apply-page-bg.json`、`docs/examples/docx-apply-page-bg-fill.json`。

### setPageBorders

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "setPageBorders",
      "id": "section:1",
      "offsetFrom": "page",
      "top": {
        "val": "single",
        "sz": 4,
        "space": 24,
        "color": "auto"
      },
      "left": {
        "val": "single",
        "sz": 4,
        "space": 24,
        "color": "auto"
      },
      "bottom": {
        "val": "single",
        "sz": 4,
        "space": 24,
        "color": "auto"
      },
      "right": {
        "val": "single",
        "sz": 4,
        "space": 24,
        "color": "auto"
      }
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setPageBorders"` |
| `id` | 节 id，必须是 `section:{n}` |
| `enabled` | 可选。只能是 `false`：删掉本节 `w:pgBorders` |
| `offsetFrom` | 可选。`page` 或 `text`，写入 `w:offsetFrom` |
| `display` | 可选。`allPages` / `firstPage` / `notFirstPage`，写入 `w:display` |
| `zOrder` | 可选。`front` 或 `back`，写入 `w:zOrder` |
| `top` / `left` / `bottom` / `right` | 可选。对象写入对应边；JSON `null` 删掉该边子元素；省略则不改该边 |

`enabled: false` 与其它字段不能同时出现。其余字段至少给一个，否则报 `InvalidField`。边对象必须有 `val`。
`val` 认 ECMA-376 `ST_Border` 全套（含 `apples` 等艺术型）。不认识的令牌报 `InvalidField`。JSON 不要 `art` 字段，出现则报 `UnknownField`。
`sz` 为非负整数（八分之一磅）。`space` 为非负整数（磅）。`color` 为 `auto` 或六位十六进制。
不认识的 `offsetFrom` / `display` / `zOrder`，或非法 `color`，报 `InvalidField`。
不写 `bordersDoNotSurroundHeader` / `bordersDoNotSurroundFooter` / `alignBordersAndEdges`。
没有 `w:pgBorders` 且不是删除时，按 `EG_SectPrContents` 插在 `pgMar` 之后。不新建 footnotes / theme / webSettings / fontTable。
节不存在报 `IdNotFound`。id 不是 `section:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。create 给了 `document.pageBorders` 时写入末节。

### setFormProt

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setFormProt"` |
| `id` | 节 id，必须是 `section:{n}` |
| `formProt` | 必须。`true` 写无 `val` 的 `w:formProt`；`false` 写 `val="0"`；`null` 删掉该元素 |

节不存在报 `IdNotFound`。id 不是 `section:{n}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。
`setProtection` 打开窗体仍给各节写 `formProt`；随后可用本 op 把某节改成 `val=0`。

示例见 `docs/examples/docx-apply-edit-range.json`。

### insertMath

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertMath"` |
| `id` | 段落 id（正文、页眉页脚或框内），或 `docx:document` |
| `text` | 与 `expr` 恰好一个。非空。写入一条 `m:r` / `m:t` |
| `expr` | 与 `text` 恰好一个。`oMath` 子节点数组，见「公式 expr」 |
| `kind` | 可选。`inline`（缺省）或 `display` |
| `jc` | 可选。仅显示公式。`center` / `centerGroup` / `left` / `right`。写入 `oMathParaPr/m:jc` |

写入前在该部件根上补 `xmlns:m`（`http://schemas.openxmlformats.org/officeDocument/2006/math`）。元素名写成 `m:oMath`，不要用宿主段落的 `w:` 前缀。

`kind` 为 `inline`：把 `m:oMath` 追加到该段。`id` 为 `docx:document` 时在正文末尾、`sectPr` 之前新插一个只含该 `oMath` 的 `w:p`。

`kind` 为 `display`：在点名段之后新插一个 `w:p`（新 `w14:paraId`），里面是 `m:oMathPara`/`m:oMath`。
`id` 为 `docx:document` 时插到正文末尾、`sectPr` 之前。

给了 `text` 时不解析成分数、根号。给了 `expr` 时按树写最小 OMML，不写 `m:rPr` / `ctrlPr`（`brk` 除外）。
行内带 `jc` 报 `InvalidField`。`kind` 为 `display` 且给了 `jc` 时写 `m:oMathParaPr`。
框内、注内、批注内的段落，以及公式 id、容器 id、表 id、run id、`section:{n}`，一律报 `InvalidId`。找不到段落报 `IdNotFound`。
空 `text`、空 `expr`、同时给两者、两个都不给、或不认识的 `kind` / `type`，报 `InvalidField`。出现 `tex` / `unicodemath` / `omml` 报未知字段。
xlsx / pptx 的 drawing 公式框见 [xlsx 分册](json-contract-xlsx.md#insertmath) 与 [pptx 分册](json-contract-pptx.md#insertmath)。create 给了 `paragraphs[].maths` 时按同一套规则写入。

### 公式 expr

`expr` 是数组，顺序即 `oMath` 子节点顺序。每个节点必须有 `type`。inspect 忽略 `ctrlPr` 以及 `m:*Pr` 里未点名的属性。`phant` / `borderBox` 读对应 `*Pr` 上本期列出的布尔。`type=text` 读 `m:rPr/m:brk`。apply 不写 `ctrlPr` 与字体。

| `type` | OMML | 槽 |
| --- | --- | --- |
| `text` | `m:r`/`m:t` | `text` 非空；可选 `brk`：`true` 写 `m:rPr/m:brk` |
| `frac` | `m:f` | `num`、`den`；可选 `fracType`：`bar`（缺省，不写 `fPr`）/ `skw` / `lin` / `noBar` |
| `rad` | `m:rad` | `e`；平方根 `degHide: true` 且不要 `deg`；n 次根给 `deg` |
| `sSub` / `sSup` / `sSubSup` / `sPre` | 同名 | `e` 加 `sub` / `sup` |
| `nary` | `m:nary` | `e`；可选 `chr`、`sub`、`sup` |
| `delim` | `m:d` | `args`：至少一组节点数组，每组一个 `m:e` |
| `func` | `m:func` | `fName`、`e` |
| `mat` | `m:m` | `rows[]`，每项 `{ "cells": [ 节点数组, … ] }`，各行列数相同 |
| `acc` / `bar` | 同名 | `e`；`acc` 可选 `chr` |
| `limLow` / `limUpp` | 同名 | `e`、`lim` |
| `eqArr` | `m:eqArr` | `rows`：节点数组的数组 |
| `phant` | `m:phant` | `e`；可选布尔 `show` / `zeroWid` / `zeroAsc` / `zeroDesc` / `transp`，有则写 `phantPr` |
| `borderBox` | `m:borderBox` | `e`；可选 `hideTop` / `hideBot` / `hideLeft` / `hideRight` |
| `other` | 其余对象 | 仅 inspect：`name` 为本地名，`text` 为拼接。apply / create 报 `InvalidField` |

槽都是节点数组。必填槽空数组报 `InvalidField`。`box` / `groupChr` inspect 进 `other`，apply / create 报 `InvalidField`。
`type=text` 的 `brk`：inspect 有 `m:brk` 为 `true`，否则不要该键。apply 给 `false` 或不给则不写 `m:brk`。

### setMath

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setMath"` |
| `id` | 公式 id |
| `text` | 与 `expr` 恰好一个。非空。用一条 `m:r`/`m:t` 整棵替换子节点，结构化对象会摊平 |
| `expr` | 与 `text` 恰好一个。按树替换 `oMath` 子节点，不摊平 |
| `jc` | 可选。`center` / `centerGroup` / `left` / `right` 或 `null`（删掉 `m:jc`） |

不改 `kind`，不拆 `oMathPara` 外壳。空 `text` / 空 `expr` / 同时给 / 两个都不给且也没有 `jc` 报 `InvalidField`。
只给 `jc` 时不改 `expr`。行内带 `jc` 报 `InvalidField`。`jc` 为 `null` 且 `oMathParaPr` 因此变空则删掉该 `oMathParaPr`。
找不到报 `IdNotFound`。
id 不是公式 id 报 `InvalidId`。xlsx / pptx 的 drawing 公式框见各自分册。

### deleteMath

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteMath"` |
| `id` | 公式 id |

删掉该 `oMath`；若父 `oMathPara` 里已经没有 `oMath`，连 `oMathPara` 一起删。因此变空且无 `sectPr` 的正文段可删掉。表格单元格内的空段留下。
删完后若 `w:body` 里没有 `w:p`，在 `sectPr` 之前补一个空段。某段带分节符且不能整段删掉时，报 `NotImplemented`。找不到报 `IdNotFound`。id 不是公式 id 报 `InvalidId`。
xlsx / pptx 的 drawing 公式框见各自分册。

对含本期收入的 `oMath` 的段落发 `setParaText` 报 `NotImplemented`。

示例见 `docs/examples/docx-apply-math.json`。
框内公式与拼音见 `docs/examples/docx-apply-box-fields.json`。

### insertContentControl

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertContentControl"` |
| `id` | 段落 id（正文、页眉页脚或框内），或 `docx:document` |
| `text` | `checkbox` / `picture` 不要此字段。其余 kind 必填非空，写入控件当前值 |
| `kind` | 可选。`plainText`（缺省）、`richText`、`dropDown`、`comboBox`、`date`、`checkbox`、`picture`、`group`、`equation` |
| `file` | 只用于 `picture`。本地图片路径，规则与 `insertPicture` 相同 |
| `checked` | 只用于 `checkbox`。布尔。省略当作 `false` |
| `tag` | 可选。写入 `w:tag/@w:val` |
| `alias` | 可选。写入 `w:alias/@w:val` |
| `items` | 可选。字符串数组。只用于 `dropDown` / `comboBox`。省略时用 `text` 作为唯一列表项 |
| `fullDate` | 可选。写入 `w:date/@w:fullDate`。只用于 `date` |

在点名段末尾追加一个行内 `w:sdt`（`sdtPr` 含 `w:id`，值为本故事里尚未占用的十进制数字；有 `tag` / `alias` 则写入；类型元素按 `kind`）。
`checkbox` 写 `w14:checkbox`（`checked` / `checkedState` / `uncheckedState`，缺省符号 U+2612 / U+2610、字体 MS Gothic），
`sdtContent` 里一条带 `MS Gothic` 的 `w:r`/`w:t`（勾选 ☒、未勾 ☐）。
根元素补 `xmlns:w14`，并把 `w14` 列入 `mc:Ignorable`。
`picture` 写空的 `w:picture`，`sdtContent` 里一张行内 DrawingML（媒体与 `wp:docPr/@id` 与 `insertPicture` 同一套）。
不写 `w:showingPlcHdr`，不写 Word 自带占位图。`group` 写空的 `w:group`；`equation` 写空的 `w:equation`。
这两类以及旧 kind 的 `sdtContent` 仍是一条 `w:r`/`w:t`。元素名跟宿主段落同一 `w:` 前缀（`w14:checkbox` 除外）。
`id` 为 `docx:document` 时在正文末尾、`sectPr` 之前新插一个只含该控件的 `w:p`。不插入块级控件。不写 `w:dataBinding`。不写占位符 `showingPlcHdr`。不写 `w:lock`。

框内、注内、批注内的段落，以及控件 id、容器 id、表 id、run id、`section:{n}`，一律报 `InvalidId`。找不到段落报 `IdNotFound`。
空 `text`、不认识的 `kind`、给了 `items` 却不是 `dropDown`/`comboBox`、给了 `fullDate` 却不是 `date`、给了 `file` 却不是 `picture`、
给了 `checked` 却不是 `checkbox`、`picture` 没有 `file`、或 `items` 不是非空字符串数组，报 `InvalidField`。
xlsx / pptx 上发此 op 报 `InvalidField`。create 给了 `paragraphs[].contentControls` 时按同一套规则写入。

### setContentControl

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setContentControl"` |
| `id` | 内容控件 id |
| `text` | `checkbox` / `picture` / `group` 不要此字段。其余 kind 必填非空 |
| `file` | 只用于 `picture`。本地图片路径，规则与 `insertPicture` 相同 |
| `checked` | 只用于 `checkbox`。布尔 |
| `tag` | 可选。改 `w:tag/@w:val`；给空串则删掉 `w:tag` |
| `alias` | 可选。改 `w:alias/@w:val`；给空串则删掉 `w:alias` |
| `items` | 可选。只用于 `dropDown` / `comboBox`，整份替换 `w:listItem` |
| `fullDate` | 可选。只用于 `date`，写入 `w:date/@w:fullDate` |

`checkbox`：改 `w14:checked` 与内容符号字符，不把 `sdtContent` 换成任意文本。`picture`：用新的行内 DrawingML 换掉控件里的图，旧媒体关系若已无引用则删掉。
其余 kind 用新文本替换当前值：行内把 `sdtContent` 换成一条 `w:r`/`w:t`；块级只改 `sdtContent` 里第一个 `w:p` 的可见文本，并去掉其余 `w:p`。
有 `w:showingPlcHdr` 则删掉。不改 `kind`，不改 `w:id`。带 `w:dataBinding` 报 `NotImplemented`。
`kind` 为 `repeatingSection` 或 `group` 报 `NotImplemented`。块级 `sdtContent` 里有 `w:tbl` 报 `NotImplemented`。
空 `text`、给了 `items` 却不是下拉/组合框、给了 `fullDate` 却不是日期、给了 `file` 却不是图片控件、给了 `checked` 却不是复选控件、图片控件没有 `file`、复选控件没有 `checked`，报 `InvalidField`。
找不到报 `IdNotFound`。id 不是内容控件 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteContentControl

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteContentControl"` |
| `id` | 内容控件 id |
| `keep` | 可选。`content`（缺省，拆掉 `w:sdt` 外壳，把 `sdtContent` 的子节点留在原位）或 `none`（连内容一起删） |

`keep` 为 `none` 时：因此变空且无 `sectPr` 的正文段可删掉。表格单元格内的空段留下。删完后若 `w:body` 里没有 `w:p`，在 `sectPr` 之前补一个空段。
某段带分节符且不能整段删掉时，报 `NotImplemented`。找不到报 `IdNotFound`。id 不是内容控件 id 报 `InvalidId`。不认识的 `keep` 报 `InvalidField`。xlsx / pptx 上发此 op 报 `InvalidField`。

对含本期收入的行内内容控件的段落发 `setParaText` 报 `NotImplemented`。块级控件里的内层段落仍可 `setParaText`。

示例见 `docs/examples/docx-apply-sdt.json`。
框内控件见 `docs/examples/docx-apply-box-fields.json`。

### insertNumbering

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertNumbering"` |
| `id` | 必须是 `docx:document` |
| `kind` | `"decimal"` 或 `"bullet"` |

写入一份抽象定义（9 级 `hybridMultilevel`）加一个 `w:num` 实例。没有 `/word/numbering.xml` 则新建该部件、主文档关系与内容类型 Override。
`decimal` 的 `lvlText` 为 `%1.`、`%1.%2.` 这类累计小数；`bullet` 用 Symbol 字体，各级 `lvlText` 轮换实心点、`o` 与节号。
apply 不返回新 id；随后 inspect 按 `w:num` 文档顺序，新实例在名单末尾。`kind` 缺或不认识报 `InvalidField`。id 不是 `docx:document` 报 `InvalidId`。
xlsx / pptx 上发此 op 报 `InvalidField`。create 给了 `document.numberings` 时按同一套规则写入，再用 `paragraphs[].num` 绑到段落。

### setNumbering

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setNumbering"` |
| `id` | 抽象编号 id |
| `ilvl` | 级别，0 到 8，必填 |
| `numFmt` | 可选。`decimal`、`bullet`、`lowerLetter`、`upperLetter`、`lowerRoman` 或 `upperRoman` |
| `start` | 可选。起始值，从 1 起的整数 |
| `lvlText` | 可选。该级 `w:lvlText/@w:val` |

`numFmt`、`start`、`lvlText` 至少给一个。只改被点名抽象定义上已有的 `w:lvl`，不自动建级，也不改该实例上的 `w:lvlOverride`。
不认识的 `numFmt`、`ilvl` 超出 0–8、或该抽象定义没有这一级，报 `InvalidField`。找不到抽象定义报 `IdNotFound`。id 不是抽象编号 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### setParaNumbering

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setParaNumbering"` |
| `id` | 段落 id（正文或页眉页脚） |
| `num` | `num:{n}`，或 `null` 表示去掉该段 `w:numPr` |
| `ilvl` | 可选。绑定级别，缺省 `0`，取值 0 到 8 |

`num` 为实例 id 时写入（或改写）`w:numPr`，插在 `pStyle` 之后。找不到该编号实例报 `IdNotFound`。`num` 为 `null` 时去掉 `numPr`，不要求 numbering 部件存在。
缺 `num` 报 `InvalidField`。`ilvl` 超出 0–8 报 `InvalidField`。找不到段落报 `IdNotFound`。id 不是正文或页眉页脚段落 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/docx-apply-numbering.json`。

### insertStyle

在 `/word/styles.xml` 末尾追加一条自定义样式。没有 styles 部件时新建最小部件，并补主文档关系与 `[Content_Types].xml`。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertStyle"` |
| `id` | 必须是 `docx:document` |
| `styleId` | XML 令牌，写入 `w:styleId`。不是 `style:` 前缀 |
| `type` | `paragraph`、`character` 或 `table` |
| `name` | 可选。写入 `w:name/@w:val`。省略则与 `styleId` 相同 |
| `basedOn` | 可选。`style:{id}`，写入 `w:basedOn` |

`styleId` 必须匹配 `[A-Za-z_][A-Za-z0-9_-]*`，否则报 `InvalidField`。已存在报 `InvalidField`。
`type` 为 `numbering` 或不认识时报 `InvalidField`。空 `name` 报 `InvalidField`。
省略 `basedOn` 时绑到同类型 `w:default` 样式；没有默认样式、或默认就是自己，则不写 `basedOn`。给了 `basedOn` 时按原文写入，不先查目标是否存在。
写入 `w:customStyle="1"`。表格样式另写空的 `w:tblPr`。不写 `tblStylePr`。
id 不是 `docx:document` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。create 给了 `document.styles` 时按同一套规则写入。

### deleteStyle

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteStyle"` |
| `id` | 样式 id，例如 `style:ooxpChar` |

只删 `w:customStyle` 为开（`1` / `true` / `on`）且不是 `w:default` 的样式。内置或没有 `customStyle` 的报 `InvalidField`。
不扫描文档里的 `pStyle` / `rStyle` / `tblStyle` 引用。找不到样式报 `IdNotFound`。id 不是 `style:{styleId}` 报 `InvalidId`。
xlsx / pptx 上发此 op 报 `InvalidField`。

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

`font`、`eastAsiaFont`、`sz`、`bold`、`color`、`before`、`after` 至少给一个。
段落、字符、表格样式都可改（`w:type` 为 `paragraph` / `character` / `table`，或缺省当段落）。编号样式报 `InvalidField`。
`before` / `after` 只允许段落与表格样式；字符样式带这两项报 `InvalidField`。rPr 字段三种都可写。
空 `font` / `eastAsiaFont`、不是六位十六进制的 `color`、`sz` 越界报 `InvalidField`。
不改 `outlineLvl`、`basedOn`、`name`。不新建样式（新建用 `insertStyle`）。找不到样式报 `IdNotFound`。
id 不是 `style:{styleId}` 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### setParaStyle

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setParaStyle"` |
| `id` | 段落 id（正文或页眉页脚） |
| `style` | `style:{styleId}`，或 `null` 表示去掉该段 `w:pStyle` |

`style` 为样式 id 时写入（或改写）`w:pStyle`，放在 `pPr` 子元素最前。目标必须已在 `styles.xml` 里，且是段落样式。字符 / 表格 / 编号样式报 `InvalidField`。
找不到该样式报 `IdNotFound`。`style` 为 `null` 时去掉 `pStyle`，不要求 styles 部件存在。缺 `style` 报 `InvalidField`。找不到段落报 `IdNotFound`。
id 不是正文或页眉页脚段落 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/docx-apply-style.json`。

### setRunStyle

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setRunStyle"` |
| `id` | 正文或页眉页脚 run id |
| `style` | `style:{styleId}`，或 `null` 表示去掉该 run 的 `w:rStyle` |

`style` 为样式 id 时写入（或改写）`w:rStyle`，放在 `rPr` 子元素最前（`EG_RPrBase`）。目标必须已在 `styles.xml` 里，且是字符样式。段落 / 表格 / 编号样式报 `InvalidField`。
找不到该样式报 `IdNotFound`。`style` 为 `null` 时去掉 `rStyle`，不要求 styles 部件存在。缺 `style` 报 `InvalidField`。
找不到 run 报 `IdNotFound`。id 不是 run id 报 `InvalidId`。框内 / 注内 / 批注内 run 报 `InvalidId`。
xlsx / pptx 上发此 op 报 `InvalidField`。

### setTableStyle

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTableStyle"` |
| `id` | 表 id，正文、页眉页脚、框内或嵌套表 |
| `style` | 可选。`style:{styleId}`，或 `null` 表示去掉 `tblStyle` |
| `look` | 可选。对象写入 `tblLook`，或 `null` 表示去掉 `tblLook` |

`style` 与 `look` 至少给一个。`style` 为样式 id 时目标必须是表格样式。段落 / 字符 / 编号样式报 `InvalidField`。找不到该样式报 `IdNotFound`。
`look` 对象字段：`firstRow`、`lastRow`、`firstColumn`、`lastColumn`、`bandedRows`、`bandedColumns`，均为可选布尔。未点名的开关与已有 `tblLook` 合并。
没有已有 `tblLook` 时缺省约 Word 的 `04A0`：首行开、末行关、首列开、末列关、行带开、列带关。
写出六个属性与 `val`（四位大写十六进制）。位：`0x20` 首行、`0x40` 末行、`0x80` 首列、`0x100` 末列、`0x200` `noHBand`、`0x400` `noVBand`。
`bandedRows` 为 `true` 写 `noHBand="0"`。不写 `tblStylePr`。
找不到表报 `IdNotFound`。id 不是表 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。pptx 的 `setTablePos` 语义不变。
示例见 `docs/examples/docx-apply-table-look.json`。框内表见 `docs/examples/docx-apply-box-table.json`。

### setTableFloat

不要复用 pptx 的 `setTablePos`（单位是 EMU）。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTableFloat"` |
| `id` | 表 id |
| `pos` | 对象写入 `tblpPr`，或 `null` 去掉 `tblpPr` 与 `tblOverlap` |

缺 `pos` 报 `InvalidField`。对象字段：

- `horzAnchor` / `vertAnchor`：可选。只认 `margin` / `page` / `text`。省略则分别为 `margin` 与 `text`
- `x` / `y`：可选。单位 twip，写入 `tblpX` / `tblpY`。省略为 `0`
- `leftFromText` / `rightFromText`：可选。单位 twip。省略为 `180`
- `topFromText` / `bottomFromText`：可选。单位 twip。省略则不写该属性

不认识的锚点报 `InvalidField`。写入对象时同时写 `tblOverlap val="never"`。
找不到表报 `IdNotFound`。id 不是表 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。create 给了 `tables[].pos` 时按同一套规则写入。

### setTableWidth

不要复用 pptx 的 `setTableSize`（单位是 EMU）。不要复用 xlsx 的 `setColWidth`。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTableWidth"` |
| `id` | 表 id |
| `type` | 可选。`auto` / `dxa` / `pct`。写入 `tblW/@type` |
| `w` | 可选。非负整数。`dxa` 为单位 twip，`pct` 为五十分之一百分点（5000 为 100%） |
| `cols` | 可选。正整数数组。写入 `tblGrid/gridCol/@w`，并按所占列把各格 `tcW` 写成之和 |

`type`、`w`、`cols` 至少给一个。`cols` 长度必须等于 `colCount`，每项至少 1。
`type` 为 `auto`：写 `tblW w=0 type=auto` 与 `tblLayout=autofit`。此时不要给大于 0 的 `w`。
`type` 为 `pct`：必须给 `w`，取值 1–5000，写 `tblLayout=autofit`。
`type` 为 `dxa` 或省略且给了 `w` / `cols`：写 `tblLayout=fixed`。只给 `w` 时按现有列宽比例缩放到总和为 `w`；没有网格则均分。只给 `cols` 时 `tblW/@w` 写成各列之和。
不算最适列宽。找不到表报 `IdNotFound`。id 不是表 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。
create 给了 `tables[].width` / `tables[].cols` 时按同一套规则写入。

### setTableAlign

行内表的水平对齐与左缩进。浮动位置请用 `setTableFloat`。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTableAlign"` |
| `id` | 表 id |
| `align` | 可选。`left` / `center` / `right`，或 `null` 去掉 `jc` |
| `indent` | 可选。非负 twip，或 `null` 去掉 `tblInd` |

`align` 与 `indent` 至少给一个。不认识的 `align` 报 `InvalidField`。
找不到表报 `IdNotFound`。id 不是表 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。
create 给了 `tables[].align` / `tables[].indent` 时按同一套规则写入。

### setTableRowHeight

不要复用 xlsx 的 `setRowHeight`（单位是磅）。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTableRowHeight"` |
| `id` | 行 id |
| `h` | 必须。非负 twip，或 `null` 去掉 `trHeight` |
| `rule` | 可选。`auto` / `atLeast` / `exact`。`h` 为数字且省略时：`h` 为 0 则 `auto`，否则 `atLeast` |

缺 `h` 报 `InvalidField`。`h` 为 `null` 时不要给 `rule`。不认识的 `rule` 报 `InvalidField`。
找不到行报 `IdNotFound`。id 不是行 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。
create 给了 `tables[].rowHeights` 时按行序写入。
示例见 `docs/examples/docx-apply-table-width.json`。

### setTableCellMar

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTableCellMar"` |
| `id` | 表 id |
| `top` / `left` / `bottom` / `right` | 可选。twip 非负整数，或 `null` 去掉该边 |

至少给一边。写入表级 `tblCellMar`，`type="dxa"`。四边都删光后去掉空的 `tblCellMar`。
找不到表报 `IdNotFound`。id 不是表 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### setTableBorders

表级 `w:tblBorders` 的四边与内外框。不要和格斜线、页面边框、段边框混。不算自动套表样式。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTableBorders"` |
| `id` | 表 id |
| `borders` | 必须。对象写入 `tblBorders`，或 `null` 去掉整个 `tblBorders` |

对象字段顺序：`top` / `left` / `bottom` / `right` / `insideH` / `insideV`。
未点名的边留下。某边 JSON `null` 去掉该边。至少点名一边，否则报 `InvalidField`。
每边对象与 `setPageBorders` 的边相同：必须有 `val`；可选 `sz` / `space` / `color`。
`val` 认 ECMA-376 `ST_Border` 全套。不认识的令牌报 `InvalidField`。JSON 不要 `art` 字段。
`sz` 为非负整数（八分之一磅）。`space` 为非负整数（磅）。`color` 为 `auto` 或六位十六进制。
写入 `left` / `right`（与现有插表骨架一致），并去掉对应的 `start` / `end`。
六边都删光后去掉空的 `tblBorders`。
不要把样式里的边框摊进表节点。格斜线仍走 `setCellFormat`。
找不到表报 `IdNotFound`。id 不是表 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。
create 给了 `tables[].borders` 时按同一套规则写入。
示例见 `docs/examples/docx-apply-table-borders.json`。

### setCellBorders

格级 `w:tcBorders` 的正交四边与格内 `insideH` / `insideV`。不要和 `setCellFormat` 的 `tcMar`、斜线、表级 `setTableBorders` 混。不算自动套表样式。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setCellBorders"` |
| `id` | 单元格 id，例如 `table:1/r1c1` |
| `borders` | 必须。对象写入正交边与格内内外框，或 `null` 去掉这些边（斜线留下） |

对象字段顺序：`top` / `left` / `bottom` / `right` / `insideH` / `insideV`。
未点名的边留下。某边 JSON `null` 去掉该边。至少点名一边，否则报 `InvalidField`。
每边对象与 `setTableBorders` 相同。写入 `left` / `right`，并去掉对应的 `start` / `end`。
六边都删光后，若没有斜线则去掉空的 `tcBorders`。
不要把 `setCellFormat` 的 `top` 等字段名当边框。
找不到单元格报 `IdNotFound`。id 不是单元格 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。
create 给了 `tables[].cellFormats[].borders` 时按同一套规则写入。
示例见 `docs/examples/docx-apply-cell-borders.json`。

### setCellFormat

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setCellFormat"` |
| `id` | 单元格 id，例如 `table:1/r1c1` |
| `fill` | 可选。六位 RGB，或 `null` 去掉 `shd` |
| `top` / `left` / `bottom` / `right` | 可选。格级 `tcMar`，规则与 `setTableCellMar` 相同 |
| `tl2br` / `tr2bl` | 可选。斜线对象，或 `null` 去掉该斜线 |
| `vAlign` | 可选。`top` / `center` / `both` / `bottom`，或 `null` 去掉 `w:vAlign` |
| `noWrap` | 可选。布尔，或 `null` 去掉 `w:noWrap` |
| `textDirection` | 可选。`lrTb` / `tbRl` / `btLr` / `lrTbV` / `tbRlV` / `tbLrV`，或 `null` 去掉 `w:textDirection` |
| `fitText` | 可选。布尔，或 `null` 去掉 `w:tcFitText` |
| `hideMark` | 可选。布尔，或 `null` 去掉 `w:hideMark` |
| `headers` | 可选。非空字符串数组，整份替换 `w:headers`。`[]` 或 `null` 去掉该元素 |

`fill`、四边、两条斜线、`vAlign`、`noWrap`、`textDirection`、`fitText`、`hideMark`、`headers` 至少给一个。
`fill` 写入 `shd val="clear" color="auto"`，并去掉 theme 填充属性。不是六位十六进制报 `InvalidField`。
`vAlign` 写入 `tcPr/w:vAlign/@w:val`。不认识报 `InvalidField`。不要和节上 `setVerticalAlign` 混。
`noWrap` / `fitText` / `hideMark` 写入对应 OnOff：`true` 省略 `val`；`false` 写 `val="0"`。
`textDirection` 不认识报 `InvalidField`。
`headers` 每项不能为空串。框内表同样认。
斜线对象字段：`val`（省略为 `single`）、`sz`（可选，八分之一磅）、`color`（省略为 `auto`）。写出 `space="0"`。
`val` 与页面边框同一套：`nil` / `none` / `single` / `dashed` / `dotted` / `double` / `thick` / `wave` / `dashSmallGap` / `dotDash` / `dotDotDash`。
不认识的 `val` 或非法 `color` 报 `InvalidField`。两条斜线都删光且没有其它边时去掉空的 `tcBorders`。
找不到单元格报 `IdNotFound`。id 不是单元格 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。create 给了 `tables[].cellFormats` 时按同一套规则写入。
示例见 `docs/examples/docx-apply-cell-props.json`。

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
| `kinsoku` | 可选。写入 `w:kinsoku`。`true` 省略 `val` 或按开写出；`false` 写 `val="0"`；`null` 删掉该元素 |
| `overflowPunct` | 可选。写入 `w:overflowPunct`。开关规则与 `kinsoku` 相同 |
| `topLinePunct` | 可选。写入 `w:topLinePunct`。开关规则与 `kinsoku` 相同 |
| `autoSpaceDE` | 可选。写入 `w:autoSpaceDE`。开关规则与 `kinsoku` 相同 |
| `autoSpaceDN` | 可选。写入 `w:autoSpaceDN`。开关规则与 `kinsoku` 相同 |
| `borders` | 可选。对象写入 `w:pBdr`；`null` 删掉整个 `w:pBdr`（含未编的 `between` / `bar`） |
| `shd` | 可选。对象写入 `w:shd`；`null` 删掉该元素 |
| `tabs` | 可选。数组按顺序重写 `w:tabs`；`null` 或空数组删掉该元素 |

`align`、`left`、`right`、`firstLine`、`hanging`、`before`、`after`、`line`、`lineRule`、
`kinsoku`、`overflowPunct`、`topLinePunct`、`autoSpaceDE`、`autoSpaceDN`、`borders`、`shd`、`tabs` 至少给一个。
`firstLine` 与 `hanging` 不能同时写成非 `null` 的数字。不认识的 `align` / `lineRule` 报 `InvalidField`。只改该段自己的 `w:pPr`，不改 `styles.xml`。
属性删光后去掉空的 `w:ind` / `w:spacing`。找不到段落报 `IdNotFound`。id 不是段落 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

`borders` 对象字段顺序：`top`、`left`、`bottom`、`right`。至少给一边。每边为页面边框同一套对象，或 JSON `null` 删掉该边。省略的边不改。
四边都删光且没有 `between` / `bar` 时去掉空的 `w:pBdr`。边对象必须有 `val`。`val` 认 `ST_Border` 全套。不认识的令牌报 `InvalidField`。
`sz` 为非负整数（八分之一磅）。`space` 为非负整数（磅）。`color` 为 `auto` 或六位十六进制。

`shd` 对象只有 `fill`，必须是六位 RGB。写入 `w:shd val="clear" color="auto"`，并去掉 theme 填充属性。不是六位十六进制报 `InvalidField`。

`tabs` 数组每项字段顺序：`val`、`pos`。`val` 为 `left` / `center` / `right` / `decimal` / `bar`。`pos` 为非负整数，单位 twip。
不认识的 `val` 报 `InvalidField`。不写 `leader`。

### setRunFormat

- **`op`**
  - 必须是 `"setRunFormat"`
- **`id`**
  - run id，例如 `para:3F2A1B00/run:1`
- **`bold`**
  - 可选
  - 布尔
  - 写入 `w:b` / `w:bCs`
  - `false` 显式 `val="0"`
- **`italic`**
  - 可选
  - 布尔
  - 写入 `w:i` / `w:iCs`
  - `false` 显式 `val="0"`
- **`underline`**
  - 可选
  - `ST_Underline` 常用值
    - `single`
    - `none`
    - `double`
    - `thick`
    - `dotted`
    - `dash`
    - `wave`
    - `words`
    - `dottedHeavy`
    - `dashedHeavy`
    - `dashLong`
    - `dashLongHeavy`
    - `dotDash`
    - `dashDotHeavy`
    - `dotDotDash`
    - `dashDotDotHeavy`
    - `wavyHeavy`
    - `wavyDouble`
  - `null` 表示去掉 `w:u`
- **`sz`**
  - 可选
  - 半磅整数，2 到 3276
  - 写入 `w:sz` / `w:szCs`
- **`color`**
  - 可选
  - 六位 RGB 十六进制，不带 `#`
  - 写入 `w:color/@w:val`（大写），并去掉 theme 属性
- **`font`**
  - 可选
  - 写入 `w:rFonts` 的 `ascii` / `hAnsi` / `cs`
- **`eastAsiaFont`**
  - 可选
  - 写入 `w:rFonts` 的 `eastAsia`
- **`em`**
  - 可选
  - `none`、`dot`、`comma`、`circle` 或 `underDot`
  - 写入 `w:em/@w:val`
  - `null` 表示去掉 `w:em`
- **`eastAsianLayout`**
  - 可选
  - 对象或 `null`
  - `null` 表示去掉 `w:eastAsianLayout`
  - 对象字段：`id`（可选整数）、`combine`（可选布尔）、`combineBrackets`（可选 `none` / `round` / `square` / `angle` / `curly`）、`vert`（可选布尔）、`vertCompress`（可选布尔）
  - 整份替换该元素上的这些属性；没点名的属性不写出
  - `combine` 与 `vert` 至少给一个
  - `combineBrackets` 只在 `combine` 为 `true` 时有效，否则报 `InvalidField`
- **`highlight`**
  - 可选
  - `ST_HighlightColor`：`yellow`、`green`、`cyan`、`magenta`、`red`、`blue`、`black`、`white`、`darkBlue`、`darkCyan`、`darkGreen`、`darkMagenta`、`darkRed`、`darkYellow`、`darkGray`、`lightGray`、`none`
  - 写入 `w:highlight/@w:val`
  - `null` 表示去掉 `w:highlight`
- **`strike`**
  - 可选
  - 布尔
  - 写入 `w:strike`
  - `false` 显式 `val="0"`
  - `null` 表示去掉该元素
- **`dstrike`**
  - 可选
  - 布尔
  - 写入 `w:dstrike`
  - `false` 显式 `val="0"`
  - `null` 表示去掉该元素
- **`vertAlign`**
  - 可选
  - `superscript`、`subscript` 或 `baseline`
  - 写入 `w:vertAlign/@w:val`
  - `null` 表示去掉 `w:vertAlign`
- **`spacing`**
  - 可选
  - 整数，单位 twip，可负
  - 写入 `w:rPr/w:spacing/@w:val`（§17.3.2.35）
  - `null` 表示去掉该元素
  - 不是段上 `setParaFormat` 的 `spacing` 对象，也不写 `w:w`
- **`kern`**
  - 可选
  - 非负整数，单位半磅
  - 写入 `w:kern/@w:val`（§17.3.2.19）
  - `null` 表示去掉该元素
  - 不是文档级 `characterSpacingControl`
- **`scale`**
  - 可选
  - 整数，1 到 600，单位百分比
  - 写入 `w:rPr/w:w/@w:val`（§17.3.2.43）
  - `null` 表示去掉该元素
  - 不要叫 `w`，不要和 `kern`、run 上的 `spacing` 混

`bold`、`italic`、`underline`、`sz`、`color`、`font`、`eastAsiaFont`、`em`、`eastAsianLayout`、`highlight`、`strike`、`dstrike`、`vertAlign`、`spacing`、`kern`、`scale` 至少给一个。只改被点名的那一个 `w:r` 的 `w:rPr`，不拆 run，不改 `styles.xml`。
不认识的 `underline` / `em` / `combineBrackets` / `highlight` / `vertAlign`、空 `font` / `eastAsiaFont`、不是六位十六进制的 `color`、`sz` 越界、`spacing` 不是整数、`kern` 不是非负整数、`scale` 不在 1–600，报 `InvalidField`。找不到 run 报 `IdNotFound`。
id 不是 run id 报 `InvalidId`。xlsx / pptx 上发此 op 或这些字段报 `InvalidField`。
格内、框内、注内、批注内、页眉页脚段落的 run 同样认。

示例见 `docs/examples/docx-apply-format.json`。
段边框、底纹、制表位与 run 剩余见 `docs/examples/docx-apply-para-run-remain.json`。
字符间距与字距调整见 `docs/examples/docx-apply-run-spacing.json`。
字符缩放见 `docs/examples/docx-apply-run-scale.json`。

### insertRuby

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertRuby"` |
| `id` | 段落 id（正文、页眉页脚或框内），或 `docx:document` |
| `base` | 必填非空。基文，写入 `w:rubyBase` 一条 `w:r` / `w:t` |
| `rt` | 必填非空。拼音，写入 `w:rt` 一条 `w:r` / `w:t` |
| `align` | 可选。`center`、`distributeLetter`、`distributeSpace`、`left`、`right` 或 `rightVertical`。缺省 `distributeSpace` |
| `hps` | 可选。拼音字号，半磅整数，2 到 3276。缺省 `12` |
| `hpsRaise` | 可选。拼音抬升，半磅整数，2 到 3276。省略则与 `hps` 相同 |
| `hpsBaseText` | 可选。基文字号，半磅整数，2 到 3276。缺省 `24` |
| `lid` | 可选。语言标签，写入 `w:lid/@w:val`。缺省 `zh-CN` |

在点名段末尾追加一个 `w:r`，里面是 `w:ruby`（`rubyPr` + `rt` + `rubyBase`）。
`id` 为 `docx:document` 时在正文末尾、`sectPr` 之前新插一个只含该拼音的 `w:p`。
框内、注内、批注内的段落，以及拼音 id、容器 id、表 id、run id、`section:{n}`，一律报 `InvalidId`。找不到段落报 `IdNotFound`。
空 `base` / `rt`、空 `lid`、不认识的 `align`、或 `hps` / `hpsRaise` / `hpsBaseText` 越界，报 `InvalidField`。
xlsx / pptx 上发此 op 报 `InvalidField`。create 给了 `paragraphs[].rubies` 时按同一套规则写入。

### setRuby

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setRuby"` |
| `id` | 拼音 id |
| `base` | 可选。非空。替换 `rubyBase` 里的可见文本 |
| `rt` | 可选。非空。替换 `rt` 里的可见文本 |
| `align` | 可选。写入 `w:rubyAlign/@w:val` |
| `hps` | 可选。半磅整数，2 到 3276 |
| `hpsRaise` | 可选。半磅整数，2 到 3276 |
| `hpsBaseText` | 可选。半磅整数，2 到 3276 |
| `lid` | 可选。非空。写入 `w:lid/@w:val` |

`base`、`rt`、`align`、`hps`、`hpsRaise`、`hpsBaseText`、`lid` 至少给一个。只改被点名的那一个 `w:ruby`。
空 `base` / `rt` / `lid`、不认识的 `align`、或字号越界报 `InvalidField`。找不到报 `IdNotFound`。
id 不是拼音 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteRuby

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteRuby"` |
| `id` | 拼音 id |

删掉该 `w:ruby`。若宿主 `w:r` 里除 `w:rPr` 外只剩这一份拼音，连该 run 一起删。不因此删掉空段。
找不到报 `IdNotFound`。id 不是拼音 id 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

对含本期收入的 `w:ruby` 的段落发 `setParaText` 报 `NotImplemented`。

示例见 `docs/examples/docx-apply-east-asia.json`。

### setEastAsianSettings

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setEastAsianSettings"` |
| `id` | 必须是 `docx:document` |
| `characterSpacingControl` | 可选。`doNotCompress`、`compressPunctuation` 或 `compressPunctuationAndJapaneseKana`。`null` 删掉该元素 |
| `strictFirstAndLastChars` | 可选。布尔。`true` 写入空元素；`false` 写 `val="0"`；`null` 删掉该元素 |
| `noLineBreaksBefore` | 可选。对象 `{ "lang", "chars" }` 或 `null`。`null` 删掉该元素 |
| `noLineBreaksAfter` | 可选。对象 `{ "lang", "chars" }` 或 `null` |
| `themeFontLang` | 可选。对象 `{ "val", "eastAsia", "bidi" }` 或 `null`。`null` 删掉该元素 |

五个字段至少给一个。禁则对象必须同时有非空 `lang` 与非空 `chars`。
`themeFontLang` 对象只认 `val` / `eastAsia` / `bidi`，值为字符串或 `null`。空串报 `InvalidField`。写入时整份替换该元素，只写出非 `null` 的属性。
不认识的 `characterSpacingControl` 报 `InvalidField`。
id 不是 `docx:document` 报 `InvalidId`。没有 settings 且要写入时新建 `/word/settings.xml` 与主文档 settings 关系。
xlsx / pptx 上发此 op 报 `InvalidField`。create 给了 `document.eastAsianSettings` 时按同一套规则写入。

### setTextBoxLink

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTextBoxLink"` |
| `id` | 框 id |
| `next` | 后一框的 id；`null` 表示断开与后一框的链接 |

把 `id` 接到 `next` 前面，或断开。两框必须在同一部件（同一主文档，或同一页眉页脚部件）。跨部件链接报 `InvalidField`。断开时后一框变成独立故事（自己的 `wps:txbx`）。
`next` 可以是没有 `w:txbxContent` 的溢出后继框（Word 常只留 `wps:linkedTxbx`）。xlsx / pptx 上发此 op 报 `InvalidField`。

`setParaText` 接受框内段落 id、注内段落 id 与批注内段落 id。对框 id、注容器 id 或批注容器 id 发 `setParaText` 报 `InvalidId`。

### insertFootnote / insertEndnote

| 字段 | 含义 |
| --- | --- |
| `op` | `"insertFootnote"` 或 `"insertEndnote"` |
| `id` | 宿主段落 id：正文、页眉页脚或框内段落 |
| `text` | 可选。新注第一段的可见文本，省略当作 `""` |

在该段末插入一个新 run，内含 `w:footnoteReference` 或 `w:endnoteReference`。不提供字符偏移。分配该注部件里尚未占用的 `w:id`（十进制，跳过已有用户注和分隔符，常见从 `2` 起）。
没有注部件则新建 `/word/footnotes.xml` 或 `/word/endnotes.xml`、主文档关系、内容类型 Override，
并写入 `separator`（id `0`）与 `continuationSeparator`（id `1`）；
`settings.xml` 里补上指向这两条分隔符的 `footnotePr` / `endnotePr` 引用（§17.11）。
新用户注至少一段，第一段含对应的 `w:footnoteRef` / `w:endnoteRef` 标记 run，再跟 `text`。不写自定义标记（`customMarkFollows`）。
对图 id、容器 id、注 id 或 `section:{n}` 发此 op 报 `InvalidId`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteFootnote / deleteEndnote

| 字段 | 含义 |
| --- | --- |
| `op` | `"deleteFootnote"` 或 `"deleteEndnote"` |
| `id` | 用户注 id，例如 `footnote:2` |

去掉主文档、页眉、页脚和文本框里所有匹配的引用 run，再去掉注部件里该条 `w:footnote` / `w:endnote`。不得删分隔符。已经不存在报 `IdNotFound`。id 不是用户注 id 报 `InvalidId`。
注内图按图片规则：该媒体不再被任何关系引用则删除部件。删光用户注后仍保留注部件和两条分隔符，不拆掉整个 part。xlsx / pptx 上发此 op 报 `InvalidField`。

### setNoteNumbering

- **`op`**
  - 必须是 `"setNoteNumbering"`
- **`id`**
  - `docx:document` 改整篇：写 `settings.xml` 的 `footnotePr` / `endnotePr`，并写每一节 `sectPr` 里同名元素
  - Word 16.0 的 `Footnotes.NumberStyle` 读的是节属性
  - 只写 settings 时打开后仍显示阿拉伯数字
  - `section:{n}` 只改该节 `sectPr`
- **`kind`**
  - `footnote` 或 `endnote`
- **`numFmt`**
  - 可选
  - `decimal`、`upperRoman`、`lowerRoman`、`upperLetter`、`lowerLetter`、`chicago`（`ST_NumberFormat` 的常用子集，§17.11.18）
- **`numStart`**
  - 可选
  - 起始值，正整数（§17.11.20）
- **`numRestart`**
  - 可选
  - `continuous`、`eachSect`、`eachPage`（`ST_RestartNumber`，§17.11.19）

`numFmt`、`numStart`、`numRestart` 至少给一个，否则报 `InvalidField`。只改被点名的属性，其余编号属性保留。不重算页面上已经画出来的序号（那是渲染，Word 打开时自己画）。
不认识的枚举报 `InvalidField`。id 不是 `docx:document` 或 `section:{n}` 报 `InvalidId`。节不存在报 `IdNotFound`。xlsx / pptx 上发此 op 报 `InvalidField`。

写入 `sectPr` 时，`footnotePr` / `endnotePr` 必须落在 `EG_SectPrContents` 顺序里：页眉页脚引用之后、`type` / `pgSz` 等之前。
写在 `pgSz` 后面时 Word 会当没看见。`CT_FtnDocProps` 里 `numFmt` 必须写在特殊 `footnote` / `endnote` 引用之前。

### setChartWrap

字段与 `setPictureWrap` 相同，`id` 为图表 id。改宿主 `wp:inline` / `wp:anchor` 与绕排，规则与图相同。xlsx / pptx 上发此 op 报 `InvalidField`。xlsx 改锚点请用 `setChartAnchor`。

### setSmartArtWrap

字段与 `setPictureWrap` 相同，`id` 为图示 id。改宿主 `wp:inline` / `wp:anchor` 与绕排，规则与图相同。xlsx / pptx 上发此 op 报 `InvalidField`。pptx 改画布坐标请用 `setSmartArtPos`。


### insertConnector

往正文或页眉页脚段落插入一条未绑连接点的连接线。写成浮动 `wp:anchor` + `wrapSquare` 的 `wps:wsp`，不是 `cxnSp`。

- **`op`**
  - 必须是 `"insertConnector"`
- **`id`**
  - 宿主段落 id：正文或页眉页脚
- **`prst`**
  - 可选
  - 缺省 `straightConnector1`
  - 允许
    - `line`
    - `straightConnector1`
    - `bentConnector2`
    - `bentConnector3`
    - `bentConnector4`
    - `bentConnector5`
    - `curvedConnector2`
    - `curvedConnector3`
    - `curvedConnector4`
    - `curvedConnector5`
- **`cx` / `cy`**
  - 可选
  - EMU，正整数
  - 必须成对出现或成对省略
  - 都省略时默认 `1828800` × `762000`

`id` 必须是段落 id。对连接线 id、图 id、框 id、容器 id、框内段落 id 或 `section:{n}` 发此 op 报 `InvalidId`。找不到该段报 `IdNotFound`。
对注内或批注内段落发此 op 报 `InvalidField`（`GraphicInRestrictedStory`）：Word 不能把图形对象放到脚注、尾注或批注里。对框内段落发此 op 报 `InvalidId`。
出现 `x` / `y` / `from` / `to` / `anchor` 以及 xlsx 偏移字段报 `UnknownField`。只给 `cx` 或只给 `cy` 报 `InvalidField`。
`prst` 不在名单里报 `InvalidField`。写入 `mc:AlternateContent`（`Requires="wps"`），
Choice 为 `wps:wsp`：`wps:cNvCnPr`、无 `txbx`、无填充、线宽 `12700` 的黑色描边、空 `wps:bodyPr`。
Fallback 为空白 `w:pict`，不编 VML。`wp:docPr/@id` 取该部件已有 `docPr` 与 `cNvPr` 的最大整数加 1。`wp:docPr/@name` 写成「连接线 {该 id}」。
xlsx 见 [xlsx 分册](json-contract-xlsx.md#insertconnector)。pptx 见 [pptx 分册](json-contract-pptx.md#insertconnector)。

### deleteConnector

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteConnector"` |
| `id` | 连接线 id |

找不到该连接线报 `IdNotFound`。id 不是连接线 id 报 `InvalidId`。
删掉对应 `w:drawing`；若在 `mc:AlternateContent` 的 Choice 里，整段 `AlternateContent` 删掉。组合里只摘该子 `wps:wsp`；空组合再摘整段。
所在 `w:r` 若删完后只剩 `w:rPr` 或空，把该 run 也删掉。不改其它对象的 `docPr` / `cNvPr`。xlsx 见 [xlsx 分册](json-contract-xlsx.md#deleteconnector)。
pptx 见 [pptx 分册](json-contract-pptx.md#deleteconnector)。

### insertShape

往正文或页眉页脚段落插入一个无文本框的预设几何形状。写成浮动 `wp:anchor` + `wrapSquare` 的 `wps:wsp`，不是文本框，也不是连接线。

- **`op`**
  - 必须是 `"insertShape"`
- **`id`**
  - 宿主段落 id：正文或页眉页脚
- **`prst`**
  - 可选
  - 缺省 `rect`
  - 必须是 pptx `insertShape` 那份可写预设名
- **`cx` / `cy`**
  - 可选
  - EMU，正整数
  - 必须成对出现或成对省略
  - 都省略时默认 `1828800` × `914400`

`id` 必须是段落 id。对形状 id、连接线 id、图 id、框 id、容器 id、框内段落 id 或 `section:{n}` 发此 op 报 `InvalidId`。找不到该段报 `IdNotFound`。
对注内或批注内段落发此 op 报 `InvalidField`（`GraphicInRestrictedStory`）。对框内段落发此 op 报 `InvalidId`。
出现 `text` / `x` / `y` / `geom` / `from` / `to` / `anchor` 以及 xlsx 偏移字段报 `UnknownField`。只给 `cx` 或只给 `cy` 报 `InvalidField`。
`prst` 不是可写预设名（含连接线几何名）报 `InvalidField`（`InvalidShapePrst`）。请用 `insertConnector` 写连接线。
写入 `mc:AlternateContent`（`Requires="wps"`），Choice 为 `wps:wsp`：空 `wps:cNvSpPr`、无 `txbx`、无 `txBox`、填充 `FFFFFF`、线宽 `6350` 的黑色描边、空 `wps:bodyPr`。
不写 `wps:style`。Fallback 为空白 `w:pict`，不编 VML。`wp:docPr/@id` 取该部件已有 `docPr` 与 `cNvPr` 的最大整数加 1。`wp:docPr/@name` 写成「形状 {该 id}」。
xlsx 见 [xlsx 分册](json-contract-xlsx.md#insertshape)。pptx 见 [pptx 分册](json-contract-pptx.md#insertshape)。

### deleteShape

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteShape"` |
| `id` | 形状 id |

找不到该形状报 `IdNotFound`。id 不是形状 id 报 `InvalidId`。
删掉对应 `w:drawing`；若在 `mc:AlternateContent` 的 Choice 里，整段 `AlternateContent` 删掉。组合里只摘该子 `wps:wsp`；空组合再摘整段。
所在 `w:r` 若删完后只剩 `w:rPr` 或空，把该 run 也删掉。不改其它对象的 `docPr` / `cNvPr`。xlsx 见 [xlsx 分册](json-contract-xlsx.md#deleteshape)。
pptx 见 [pptx 分册](json-contract-pptx.md#deleteshape)。

### setShapeSize

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setShapeSize"` |
| `id` | 形状 id |
| `cx` / `cy` | 可选。EMU，正整数。至少一个 |

两个都不给报 `InvalidField`。找不到该形状报 `IdNotFound`。id 不是形状 id 报 `InvalidId`。
改外层 `wp:extent` 与 `a:xfrm/a:ext`。组合子对象改外层 drawing，本期不单独重算组合框。xlsx 见 [xlsx 分册](json-contract-xlsx.md)。pptx 见 [pptx 分册](json-contract-pptx.md#setshapesize)。

### setShapePos

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setShapePos"` |
| `id` | 形状 id |
| `x` / `y` | 可选。EMU 整数。至少一个 |

两个都不给报 `InvalidField`。找不到该形状报 `IdNotFound`。id 不是形状 id 报 `InvalidId`。
Word 没有幻灯片画布。写成 `wp:positionH` / `wp:positionV` 的 `posOffset`，`relativeFrom="margin"`。行内没有 `wp:anchor` 时报 `InvalidField`（`DocxShapeNeedsAnchor`）。
xlsx 上发此 op 报 `InvalidField`。pptx 见 [pptx 分册](json-contract-pptx.md#setshapepos)。

### setShapeFill

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setShapeFill"` |
| `id` | 形状 id |
| `color` | 必填。六位 RGB，或 `null` 表示无填充 |

缺 `color` 或不是六位十六进制报 `InvalidField`。找不到该形状报 `IdNotFound`。id 不是形状 id 报 `InvalidId`。
写入该 `wps:wsp` 的 `wps:spPr`。不解析、不删除 `wps:style`；本地 `spPr` 覆盖主题。xlsx 上发此 op 报 `InvalidField`。pptx 见 [pptx 分册](json-contract-pptx.md#setshapefill)。

### setShapeLine

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setShapeLine"` |
| `id` | 形状 id |
| `color` | 必填。六位 RGB，或 `null` 表示无线 |
| `width` | 可选。EMU 正整数。仅有边框时允许 |

缺 `color` 报 `InvalidField`。`color` 为 `null` 时仍给 `width` 报 `InvalidField`。找不到该形状报 `IdNotFound`。id 不是形状 id 报 `InvalidId`。
写入该 `wps:wsp` 的 `wps:spPr`。不解析、不删除 `wps:style`。xlsx 上发此 op 报 `InvalidField`。pptx 见 [pptx 分册](json-contract-pptx.md#setshapeline)。

docx 上发 `setShapeGeom` / `setShapeFormat` / `setShapeText` / `setConnector` 报 `InvalidField`。

### groupShapes

把正文或同一页眉/页脚部件里至少两个独立浮动对象包进一个新 `wpg:wgp`。组合本身不进 inspect 树。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"groupShapes"` |
| `id` | 必须是 `docx:document`（正文）或页眉/页脚容器 id |
| `ids` | 必填。至少两个成员 id。每个必须是浮动图、文本框、连接线或无框形状 id，且当前是独立 drawing（不在 `wpg:wgp` 里） |

成员少于 2 个或有重复报 `InvalidField`（`GroupNeedsTwo`）。
成员已在某个 `wpg:wgp` 里、是行内、跨部件、或不是图 / 框 / 连接线 / 无框形状，报 `InvalidField`（`GroupMemberNested`）。找不到某个成员报 `IdNotFound`。
`id` 不是 `docx:document` 也不是页眉/页脚容器 id 报 `InvalidId`。对图表 id、SmartArt id、OLE id、段落 id 或 `section:{n}` 发此 op 报 `InvalidId`。
跨段可以，结果挂到第一个成员所在段。新组合 `wp:docPr/@id` 取该部件已有 `docPr` 与 `cNvPr` 的最大整数加 1；`name` 写成「组合 {该 id}」。
子对象把原来的 `wp:docPr` 抄到各自 `cNvPr`。`mc:Choice` 的 `Requires` 为 `wpg`，`graphicData/@uri` 为 wordprocessingGroup。
组合 `xfrm` 的 `off` 为 0，`ext` / `chOff` / `chExt` 取包围盒；第一个成员 `a:off` 为 `(0,0)`，后续按宽度右移。不重算已有组合框。不把图表、SmartArt、OLE 当成员。
xlsx 见 [xlsx 分册](json-contract-xlsx.md#groupshapes)。pptx 见 [pptx 分册](json-contract-pptx.md#groupshapes)。

### ungroup

解开某个成员所在的那一层 `wpg:wgp`，子节点各自回到独立浮动 drawing。组合本身没有 inspect id。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"ungroup"` |
| `id` | 该组合内一个图 id、文本框 id、连接线 id 或无框形状 id |

找不到该对象报 `IdNotFound`。id 不是图 / 框 / 连接线 / 无框形状 id 报 `InvalidId`。该对象不在 `wpg:wgp` 里报 `InvalidField`（`NotGrouped`）。只解一层。
子节点把 `cNvPr/@id` 写回各自 `wp:docPr`。框、连接线与无框形状走 `Requires="wps"`，图走图片 Choice。xlsx 见 [xlsx 分册](json-contract-xlsx.md#ungroup)。
pptx 见 [pptx 分册](json-contract-pptx.md#ungroup)。

### 改动范围（docx）

允许变脏的部件：主文档 XML（通常是 `/word/document.xml`）及其 `.rels`，
被 `setParaText`、`setParaFormat`、`setRunFormat`、图片 op、文本框 op、连接线与组合 op、无框形状 op、墨迹画布 op、图表 op、SmartArt op、OLE op、表格结构 op、修订 op、超链接与书签 op、
域 op、公式 op、拼音 op、内容控件 op、`setParaNumbering`、`setParaStyle`、`setRunStyle` 或表格外观 op 点名的页眉、页脚部件及其 `.rels`，
`setHeaderFooterLink` / `insertHeaderFooter` / `deleteSection` 改到的主文档关系、`[Content_Types].xml`、新建或删除的 header/footer 部件及其 `.rels`，
脚注 / 尾注部件及其 `.rels`，批注部件 `/word/comments.xml`、`commentsExtended.xml`、`people.xml`，
以及文件里已有的 `commentsIds.xml` / `commentsExtensible.xml`，图表部件 `/word/charts/…` 及其 `.rels`，embedding `/word/embeddings/…`，
SmartArt 四件套 `/word/diagrams/…` 及可选 drawing 缓存，`setNoteNumbering` 在整篇上改到的 `settings.xml` 与主文档（各节 `sectPr`），
`setEvenAndOddHeaders` / `setMirrorMargins` / `setGutterAtTop` / `setProtection` / `setEastAsianSettings` 改到的 `settings.xml`，
`setProtection` 打开或关掉窗体保护时还可以脏主文档各节 `w:formProt`，
`insertWatermark` / `deleteWatermark` 改到的页眉部件及其 `.rels`、必要时新建的 `/word/headerN.xml`、主文档关系、
`[Content_Types].xml` 与 `/word/media/…`，
编号 op 改到的 `/word/numbering.xml`、主文档 `.rels` 与 `[Content_Types].xml`，
`insertStyle` / `deleteStyle` / `setStyle` 改到的 `/word/styles.xml`（缺则新建，并可脏主文档 `.rels` 与 `[Content_Types].xml`），
以及新媒体或被删的 `/word/media/…`。
OLE op 还可以脏 `/word/embeddings/…` 与预览图媒体。`insertInkCanvas` / `replaceInkCanvas` / `deleteInkCanvas` 还可以脏 `/word/ink/…`、宿主 `.rels` 与 `[Content_Types].xml`。
`setInkCanvasSize` / `setInkCanvasWrap` 只脏宿主 XML。
`setModel3dSize` / `setModel3dWrap` / `setModel3dCamera` 只脏宿主 XML。
图表 op、SmartArt op 与文本框 op 点名注内或批注内段落时，还可以脏对应 notes / comments 部件及其 `.rels`。
`setHeaderFooterDistance`、`setPageMargin`、`setPageSize`、`setColumns`、`setVerticalAlign`、`setLineNumbering`、`setDocGrid`、
`setRtlGutter`、`setPageBorders`、`setFormProt`、`setTitlePage`、`insertSection`、`insertToc`、`deleteToc`、`insertCaption`、`deleteCaption`、
`insertCrossRef`、`deleteCrossRef`、`insertIndex`、`deleteIndex`、`insertIndexEntry`、`deleteIndexEntry`、`insertSource`、
`deleteSource`、`insertCitation`、`deleteCitation`、`insertBibliography`、`deleteBibliography`、`insertDocPart`、
`deleteDocPart`、`insertDocPartContent`、`insertEmbeddedFont`、`deleteEmbeddedFont`、`setFontEmbed`、`insertMath`、
`setMath`、`deleteMath`、`insertRuby`、`setRuby` 与 `deleteRuby` 只脏主文档（页眉页脚上的公式 op、拼音 op、`insertCrossRef`、
`insertIndexEntry` 与 `insertCitation` 另脏对应 header/footer 部件）。
`insertSource` / `deleteSource` / `setBibliographyStyle` 还可以脏 `/customXml/itemN.xml`、`itemPropsN.xml`、`itemN.xml.rels`、主文档 `.rels` 与 `[Content_Types].xml`。
`insertDocPart` / `deleteDocPart` 还可以脏 `/word/glossary/document.xml`、
其卫星 `styles.xml` / `settings.xml` / `webSettings.xml` / `fontTable.xml`、glossary `.rels`、主文档 `.rels` 与 `[Content_Types].xml`。
`insertDocPartContent` 只脏主文档。`insertEmbeddedFont` / `deleteEmbeddedFont` 还可以脏 `/word/fontTable.xml`、fontTable `.rels`、
`/word/fonts/fontN.odttf`、主文档 `.rels`、`[Content_Types].xml` 与 `settings.xml`。
`setFontEmbed` 只脏 `settings.xml`（缺则新建）。`setPageBackground` 脏主文档与 `settings.xml`（缺则新建），图片填充还可脏 `/word/media/…` 与主文档 `.rels`。
`setThemeColors` / `setThemeFonts` / `setThemeFmt` / `setThemeExtraSchemes` / `setThemeObjectDefaults` 只脏已有 theme 部件。
`setThemeOverride` 脏 `/word/theme/themeOverride1.xml`、主文档 `.rels` 与 `[Content_Types].xml`；`enabled: false` 时删掉该部件。
`setEastAsianSettings` 只脏 `settings.xml`（缺则新建）。
`insertContentControl`、`setContentControl` 与 `deleteContentControl` 只脏主文档（页眉页脚上的内容控件 op 另脏对应 header/footer 部件）。
`deleteSection` 脏主文档，并可能删掉不再被引用的 header/footer 部件。
正文图片 op、文本框 op、图表 op、SmartArt op、正文表格结构 op、正文修订 op、正文超链接 / 书签 op、正文域 op、正文公式 op、正文拼音 op、正文内容控件 op、正文 `setParaNumbering`、
正文 `setParaStyle`、正文 `setRunStyle`、正文 `setParaFormat` 与正文 `setRunFormat` 脏主文档与主文档 `.rels`。
正文 `setTableStyle` / `setTableFloat` / `setTableWidth` / `setTableAlign` / `setTableRowHeight` / `setTableCellMar` / `setTableBorders` / `setCellFormat` / `setCellBorders` 脏主文档。
框内表跟框所在故事走：正文框脏主文档，页眉页脚框脏对应 header/footer 部件。
页眉页脚上的这些 op 另脏对应 header/footer 部件。
一次 `ops` 可以混用 `setParaText`、`setHeaderFooterDistance`、`setPageMargin`、`setPageSize`、`setColumns`、`setVerticalAlign`、
`setLineNumbering`、`setDocGrid`、`setRtlGutter`、`setPageBorders`、`setFormProt`、`setMirrorMargins`、`setGutterAtTop`、
`setHeaderFooterLink`、`insertHeaderFooter`、`setTitlePage`、`setEvenAndOddHeaders`、`insertSection`、`deleteSection`、
`insertToc`、`deleteToc`、`insertCaption`、`deleteCaption`、`insertCrossRef`、`deleteCrossRef`、`insertIndex`、`deleteIndex`、
`insertIndexEntry`、`deleteIndexEntry`、`insertSource`、`deleteSource`、`setBibliographyStyle`、`insertCitation`、`deleteCitation`、
`insertBibliography`、`deleteBibliography`、`insertDocPart`、`deleteDocPart`、`insertDocPartContent`、`insertEmbeddedFont`、
`deleteEmbeddedFont`、`setFontEmbed`、`setPageBackground`、`setThemeFmt`、`setThemeExtraSchemes`、`setThemeObjectDefaults`、`setThemeOverride`、`insertMath`、`setMath`、`deleteMath`、`insertRuby`、`setRuby`、`deleteRuby`、`setEastAsianSettings`、`insertContentControl`、
`setContentControl`、`deleteContentControl`、图片 op、文本框 op、
`insertConnector` / `deleteConnector` / `insertShape` / `deleteShape` / `setShapePos` / `setShapeSize` / `setShapeFill` / `setShapeLine` / `groupShapes` / `ungroup`、墨迹画布 op、脚注尾注 op、批注 op、图表 op、SmartArt op、表格结构 op、修订 op、
超链接 / 书签 op、域 op 与编号 op、样式 op、表格外观 op、直接格式 op、以及 `setProtection`、以及 `insertWatermark` / `deleteWatermark`、
以及 `setProps` / `setCustomProp` / `deleteCustomProp`。
文件属性 op 还可以脏 `/docProps/core.xml`、`/docProps/app.xml`、`/docProps/custom.xml`、根 `.rels` 与 `[Content_Types].xml`。
只改 core 字段时不要新建 custom。未点名的 `styles.xml`、未点名的页眉页脚、主题一律原样写回。未点名的 `numbering.xml` 也原样写回。未点名的 `settings.xml` 也原样写回。
`setNoteNumbering` 只点名某一节时只脏主文档，不改 `settings.xml`。`replyComment` 会脏出现父批注范围标记的宿主部件（常见是主文档）。
只改批注文本、元数据或完成标记时，未出现范围标记的宿主部件（常见是主文档）原字节写回。只改图表 cache / 标题 / 图例、且宿主 drawing 未动时，未点名的正文段落字节按「drawing 未动」验收。
只改 SmartArt 点文字或增删点、且宿主 drawing 未动时，同样按「drawing 未动」验收。

被改部件同样整份重序列化，保留程度与 xlsx 工作表相同：元素与属性顺序、未知命名空间保留；注释、处理指令、DOCTYPE 会丢。

### insertOle

把本地文件作为嵌入对象写入目标段落。不在库里启动 OLE 服务器，不生成真实预览图（写入一张最小 PNG 占位）。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "insertOle",
      "id": "para:3F2A1B00",
      "file": "C:\\\\data\\\\book.xlsx"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertOle"` |
| `id` | 必须是正文或页眉页脚段落 id |
| `file` | 必须。本地文件路径 |

按扩展名决定 ProgID 与写入方式：`.xlsx` 为 `Excel.Sheet.12`，`.xlsm` 为 `Excel.SheetMacroEnabled.12`，`.xls` 为 `Excel.Sheet.8`，
`.docx` 为 `Word.Document.12`，`.docm` 为 `Word.DocumentMacroEnabled.12`，`.pptx` 为 `PowerPoint.Show.12`，
`.pptm` 为 `PowerPoint.ShowMacroEnabled.12`。
这些扩展名把源文件原字节写入 `/word/embeddings/…`，Content Type 与该格式主文档一致（xlsx 为
`application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`，docx / pptx 同理；`.xls` 为 `application/vnd.ms-excel`）。
其它扩展名 ProgID 为 `Package`，把源文件打成 OLE 复合文档（Ole10Native）写入 `/word/embeddings/oleObjectN.bin`，
Content Type 为 `application/vnd.openxmlformats-officedocument.oleObject`。
找不到 `file` 报 `Io`。空路径报 `InvalidField`。xlsx 上发此 op 报 `InvalidField`。
pptx 上的语义见 [json-contract-pptx.md](json-contract-pptx.md#insertole)。

宿主 XML 写 Transitional 的 `w:object`：`o:OLEObject` 的 `Type` 为 `Embed`，`DrawAspect` 为 `Content`，关系指向 embeddings；另写一条 image 关系指向占位 PNG。
不改 VML 几何。`ObjectID` 取本文件尚未用过的正整数，写成 `_` 加该数字。Office 文件的关系 Type 用 `package`（与 Word 16.0 另存一致）；Package 用 `oleObject`。

### deleteOle

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteOle"` |
| `id` | 必须是 OLE id |

找不到报 `IdNotFound`。删掉整棵 `w:object`（含预览 markup），去掉 oleObject / package 关系与 embeddings 部件；预览图若无其它关系再指向则一并去掉。
xlsx 上发此 op 报 `InvalidField`。pptx 上的语义见 [json-contract-pptx.md](json-contract-pptx.md#deleteole)。

### insertInkCanvas

把本地 InkML 作为墨迹画布写入目标段落。不把手写识别成字。不写独立 `wpi` 画布。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "insertInkCanvas",
      "id": "para:3F2A1B00",
      "file": "C:\\\\data\\\\ink.xml"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertInkCanvas"` |
| `id` | 必须是正文或页眉页脚段落 id |
| `file` | 必须。本地 InkML 路径 |
| `cx` / `cy` | 成对可选。EMU。都省则 `1828800` × `914400` |

- 只给其中一个报 `InvalidField`
- 根本地名不是 `ink`，或命名空间不是 `http://www.w3.org/2003/InkML`，报 `InvalidField`
- 找不到 `file` 报 `Io`
- 空路径报 `InvalidField`
- 对注内或批注内段落报 `InvalidField`
- 对框内段落、图 id、墨迹 id、容器 id 或 `section:{n}` 报 `InvalidId`
- xlsx / pptx 上发此 op 报 `InvalidField`

写入 `mc:AlternateContent`（`Requires="wpc"`），Choice 里是 `wpc:wpc` 加 `w14:contentPart`，Fallback 为 `<w:pict/>`。
InkML 部件路径 `/word/ink/inkN.xml`，Content Type `application/inkml+xml`，关系 Type 为 `customXml`。
`wp:docPr/@name` 用「墨迹画布 {id}」，contentPart `cNvPr/@name` 用「墨迹 {id}」。
`mc:Ignorable` 不要写入未声明的 `w15`。不必强塞 `isComment`。

### replaceInkCanvas

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"replaceInkCanvas"` |
| `id` | 必须是墨迹画布 id |
| `file` | 必须。本地 InkML 路径 |

换 InkML 字节，保留 drawing 与关系 id。找不到报 `IdNotFound`。不是墨迹 id 报 `InvalidId`。坏 InkML 报 `InvalidField`。xlsx / pptx 上发此 op 报 `InvalidField`。

### deleteInkCanvas

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteInkCanvas"` |
| `id` | 必须是墨迹画布 id |

找不到报 `IdNotFound`。删掉该 `contentPart`；若宿主 `wpc` 只剩 `bg` / `whole` 则删整段 drawing / AlternateContent。
关系无人引用则删部件与 Override。xlsx / pptx 上发此 op 报 `InvalidField`。

### setInkCanvasSize

字段与 `setPictureSize` 的 docx 一侧相同。`id` 必须是墨迹画布 id。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setInkCanvasSize"` |
| `id` | 必须是墨迹画布 id |
| `cx` | 可选。宽度，EMU，正整数 |
| `cy` | 可选。高度，EMU，正整数 |

- `cx` 与 `cy` 至少给一个，否则报 `InvalidField`
- 只改被点名的那一侧，另一侧保持原值
- 写入宿主 `wp:extent`；若 `a:ext` 存在，一并改
- 不换 InkML，不改关系
- 找不到报 `IdNotFound`
- 不是墨迹 id 报 `InvalidId`
- 对图 id 发此 op 报 `InvalidId`
- 对墨迹 id 发 `setPictureSize` 仍报 `InvalidId`
- xlsx / pptx 上发此 op 报 `InvalidField`

### setInkCanvasWrap

字段与 `setPictureWrap` 相同，`id` 必须是墨迹画布 id。行内 / 浮动与绕排规则同图片。
位置只支持相对页边。`replaceInkCanvas` 仍不改几何。xlsx / pptx 上发此 op 报 `InvalidField`。

### setModel3dSize

字段与 `setPictureSize` 的 docx 一侧相同。`id` 必须是 3D 模型 id。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setModel3dSize"` |
| `id` | 必须是 3D 模型 id |
| `cx` | 可选。宽度，EMU，正整数 |
| `cy` | 可选。高度，EMU，正整数 |

- `cx` 与 `cy` 至少给一个，否则报 `InvalidField`
- 只改被点名的那一侧，另一侧保持原值
- 写入 Choice 宿主 `wp:extent`；若 `a:ext` 存在，一并改
- Fallback 海报 drawing 同步改尺寸
- 不换 GLB，不改关系
- 找不到报 `IdNotFound`
- 不是 3D id 报 `InvalidId`
- 对图 id 发此 op 报 `InvalidId`
- 对 3D id 发 `setPictureSize` 仍报 `InvalidId`
- xlsx / pptx 上的语义见各自分册

### setModel3dWrap

字段与 `setPictureWrap` 相同，`id` 必须是 3D 模型 id。行内 / 浮动与绕排规则同图片。
位置只支持相对页边。Fallback 海报 drawing 同步改绕排。`replaceModel3d` 仍不改几何。
xlsx / pptx 上发此 op 报 `InvalidField`。xlsx 改锚点请用 `setModel3dAnchor`。pptx 改画布坐标请用 `setModel3dPos`。

改 3D 尺寸与绕排的示例见 `docs/examples/docx-apply-model3d-geom.json`。

### setModel3dCamera

改该 3D 模型的相机、模型旋转与界面「光源」。不算 3D 渲染。不换 GLB。不改画布尺寸。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "setModel3dCamera",
      "id": "para:3F2A1B00/model3d:1",
      "camera": { "pos": { "x": 50400000 }, "fov": 1892375 },
      "rot": { "ax": 1060962, "ay": -532862, "az": -169045 },
      "lightRig": { "rig": "freezing", "dir": "t", "rev": 18000000 }
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setModel3dCamera"` |
| `id` | 必须是 3D 模型 id |
| `camera` | 可选。对象 |
| `rot` | 可选。对象 |
| `lightRig` | 可选。对象或 `null` |

`camera` / `rot` / `lightRig` 至少给一个。未点名留下。
`camera` 可给 `pos` / `up` / `lookAt` / `fov`。子对象只写点名的轴。`fov` 为非负整数。
`rot` 可给 `ax` / `ay` / `az`，值为整数。没有 `am3d:rot` 时先补空元素。
`lightRig` 为 `null` 时去掉 `a:scene3d`。对象可给 `rig` / `dir` / `lat` / `lon` / `rev`。
写任一项时若还没有 `a:scene3d`，补 `a:camera prst="orthographicFront"` 再写 `a:lightRig`。
新建 lightRig 时：未给的 `rig` 为 `freezing`，未给的 `dir` 为 `t`，未给的 `a:rot` 轴为 `0`。
create / insert 不增加这些 JSON 字段。缺省 XML 仍抄 C7。inspect 得到缺省 `camera`（`pos.z` 为 `66519324`，`fov` 为 `2700000`）、`rot` 三轴 `null`、`lightRig` `null`。
id 不是 3D 模型 id 报 `InvalidId`。找不到报 `IdNotFound`。一个可写字段都不给、或 `fov` 为负，报 `InvalidField`。
xlsx / pptx 同一套字段，见各自分册。
示例见 `docs/examples/docx-apply-model3d-camera.json`。

### insertModel3d

把本地 GLB 作为 3D 模型写入目标段落。不进 `pictures`。不栅格化模型。海报用 1×1 PNG 占位。本期只写 `wp:inline`。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "insertModel3d",
      "id": "para:3F2A1B00",
      "file": "C:\\\\data\\\\model.glb"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertModel3d"` |
| `id` | 必须是正文或页眉页脚段落 id |
| `file` | 必须。本地 GLB 路径 |
| `cx` / `cy` | 成对可选。EMU。都省则 `914400` × `914400` |

- 只给其中一个报 `InvalidField`
- 不是 GLB 报 `InvalidField`
- 找不到 `file` 报 `Io`
- 空路径报 `InvalidField`
- 对注内或批注内段落报 `InvalidField`
- 对框内段落、图 id、3D id、容器 id 或 `section:{n}` 报 `InvalidId`
- xlsx / pptx 上的语义见各自分册

写入 `mc:AlternateContent`（`Requires="am3d"`），Choice 里是 `w:drawing` / `wp:inline` 包 `am3d:model3d`，Fallback 为海报 `pic`。
GLB 部件路径 `/word/media/model3dN.glb`，Content Type `model/gltf.binary`，关系 Type 为
`http://schemas.microsoft.com/office/2017/06/relationships/model3d`。
相机、`am3d:trans` 与 `am3d` 点光源抄 Office 16.0 默认值。inspect 给出 `camera` / `rot`；缺省 `lightRig` 为 `null`。不把 `ambientLight` / `ptLight` 放进树。

### replaceModel3d

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"replaceModel3d"` |
| `id` | 必须是 3D 模型 id |
| `file` | 必须。本地 GLB 路径 |

换 GLB 字节，保留 `wp:inline` 与关系 id。海报仍写成 1×1 PNG。不改位置尺寸。找不到报 `IdNotFound`。
不是 3D id 报 `InvalidId`。不是 GLB 报 `InvalidField`。xlsx / pptx 见各自分册。

### deleteModel3d

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteModel3d"` |
| `id` | 必须是 3D 模型 id |

找不到报 `IdNotFound`。删掉该 AlternateContent。GLB 与海报无人引用则删。xlsx / pptx 见各自分册。

### setThemeColors

改主文档 theme 部件的 `a:clrScheme` 槽位，以及配色方案名、theme 根名。不改未点名的 `fmtScheme` 槽。不把段落 / run 上的 `themeColor` 展开成直接色。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "setThemeColors",
      "id": "docx:document",
      "name": "Office",
      "themeName": "Office Theme",
      "accent1": "FF0000"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setThemeColors"` |
| `id` | 必须是 `docx:document` |
| `name` | 可选。`a:clrScheme/@name`。非空字符串；`null` 去掉该属性 |
| `themeName` | 可选。`a:theme/@name`。非空字符串；`null` 去掉该属性 |
| `dk1` / `lt1` / `dk2` / `lt2` / `accent1`–`accent6` / `hlink` / `folHlink` | 可选。六位 RGB 十六进制 |

docx：`id` 不是 `docx:document` 报 `InvalidId`。没有 theme 关系或部件不在包内报 `IdNotFound`。
`name`、`themeName` 与十二个槽位一个都不给，或某槽位不是六位十六进制，或名字是空串，报 `InvalidField`。
写入时把被点名槽位的子元素换成 `a:srgbClr`（原先若是 `sysClr` 也换成 `srgbClr`）。未点名槽位留下。
未点名的 `fmtScheme` 槽留下。不改正文 XML。xlsx 见 [xlsx 分册](json-contract-xlsx.md#setthemecolors)。
pptx 见 [pptx 分册](json-contract-pptx.md#setthemecolors)，`id` 为 `master:{n}`。
示例见 `docs/examples/docx-apply-theme-remain.json`。

### setThemeFonts

改主文档 theme 部件的 `a:fontScheme`。未点名的 `fmtScheme` 槽留下。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "setThemeFonts",
      "id": "docx:document",
      "name": "Office",
      "majorLatin": "Arial",
      "majorScripts": [
        { "script": "Jpan", "typeface": "Yu Mincho" }
      ]
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setThemeFonts"` |
| `id` | 必须是 `docx:document` |
| `name` | 可选。`a:fontScheme/@name`。非空字符串；`null` 去掉该属性 |
| `majorLatin` / `minorLatin` / `majorEa` / `minorEa` / `majorCs` / `minorCs` | 可选。`typeface` 原文 |
| `majorScripts` / `minorScripts` | 可选。数组，整份替换对应组里的 `a:font`。`[]` 或 `null` 清掉全部 `a:font`。不改 `latin` / `ea` / `cs` |

docx：`id` 不是 `docx:document` 报 `InvalidId`。找不到 theme 部件报 `IdNotFound`。
`name`、六个字体槽与两份脚本列表一个都不给报 `InvalidField`。
`majorLatin` / `minorLatin` 空串报 `InvalidField`。`ea` / `cs` 允许空串。
脚本每一项必须是对象，字段只认 `script`、`typeface`。`script` 必须是非空字符串。`typeface` 必须是字符串，允许空串。
没有 `fontScheme` / `majorFont` / `minorFont` 或对应 `latin` / `ea` / `cs` 则按需补上。只改被点名槽的 `typeface`，其它属性（如 `panose`）留下。
不改正文 XML。xlsx 见 [xlsx 分册](json-contract-xlsx.md#setthemefonts)。
pptx 见 [pptx 分册](json-contract-pptx.md#setthemefonts)，`id` 为 `master:{n}`。

### setThemeFmt

改主文档 theme 部件 `a:fmtScheme` 的常用槽。不算主题排版引擎，不把 `phClr` 展开成 RGB。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "setThemeFmt",
      "id": "docx:document",
      "line": { "index": 0, "w": 38100 },
      "fill": { "index": 0, "kind": "solid" }
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setThemeFmt"` |
| `id` | 必须是 `docx:document` |
| `line` | 可选。对象须有 `index`（0、1 或 2）与 `w`（非负整数，EMU） |
| `fill` | 可选。对象须有 `index`（0、1 或 2）与 `kind`（`solid` 或 `grad`） |
| `bgFill` | 可选。同 `fill`，改 `bgFillStyleLst` |
| `name` | 可选。`a:fmtScheme/@name`。非空字符串；`null` 去掉该属性 |

`line` / `fill` / `bgFill` / `name` 至少给一个。写入时只替换被点名槽：线改 `@w`；填充写成带 `schemeClr phClr` 的 `solidFill` 或两停点 `gradFill`。未点名槽留下。
`index` 超出该列表长度、缺少 `fmtScheme` 或对应列表，报 `InvalidField`。没有 theme 部件报 `IdNotFound`，不新建。
id 不是 `docx:document` 报 `InvalidId`。xlsx 见 [xlsx 分册](json-contract-xlsx.md#setthemefmt)。
pptx 见 [pptx 分册](json-contract-pptx.md#setthemefmt)，`id` 为 `master:{n}`。
示例见 `docs/examples/docx-apply-theme-fmt.json`。

### setThemeExtraSchemes

整份替换主文档 theme 部件的 `a:extraClrSchemeLst`。不算主题排版引擎。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "setThemeExtraSchemes",
      "id": "docx:document",
      "extraSchemes": [
        {
          "name": "Extra",
          "colors": { "accent1": "0F6FC6" }
        }
      ]
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setThemeExtraSchemes"` |
| `id` | 必须是 `docx:document` |
| `extraSchemes` | 必须。数组或 `null` |

`[]` 或 `null` 去掉 `extraClrSchemeLst`。否则按数组整份重写该列表。
每一项必须是对象，字段只认 `name`、`colors`。`name` 为非空字符串或 `null`（`null` 或省略则不写 `@name`）。
`colors` 规则与主 `clrScheme` 相同。项上 `name` 与 `colors.name` 都对应该项 `a:clrScheme/@name`；只给一个即可，两个都给则必须相同，否则报 `InvalidField`。
未点名的配色槽用 Office 默认十二槽补齐，再把点名槽写成 `a:srgbClr`。每项写出身份映射的 `a:clrMap`（`bg1=lt1` 那一套），不把映射暴露进 JSON。
docx：`id` 不是 `docx:document` 报 `InvalidId`。没有 theme 部件报 `IdNotFound`，不新建。
缺 `extraSchemes`、不是数组 / `null`、项不是对象、或槽位不是六位十六进制，报 `InvalidField`。
xlsx 见 [xlsx 分册](json-contract-xlsx.md#setthemeextraschemes)。
pptx 见 [pptx 分册](json-contract-pptx.md#setthemeextraschemes)，`id` 为 `master:{n}`。create 给了 `document.theme.extraSchemes` 时按同一套规则写入。
示例见 `docs/examples/docx-apply-theme-remain.json`。

### setThemeObjectDefaults

改主文档 theme 部件的 `a:objectDefaults`。不算主题排版引擎，不把 `phClr` 展开成 RGB。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "setThemeObjectDefaults",
      "id": "docx:document",
      "objectDefaults": {
        "sp": {
          "style": { "lnRef": 1, "fontRef": "minor" },
          "lnW": 38100
        }
      }
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setThemeObjectDefaults"` |
| `id` | 必须是 `docx:document` |
| `objectDefaults` | 必须。对象或 `null` |

`null` 去掉整个 `a:objectDefaults`。对象可给 `sp` / `ln` / `tx`，至少给一个。
子值为对象或 `null`（去掉对应 `spDef` / `lnDef` / `txDef`）。未点名的定义留下。
子对象可给 `style` 与 `lnW`。`style` 为对象或 `null`（去掉 `a:style`）。
`lnW` 为非负整数（EMU）或 `null`（去掉 `spPr/a:ln/@w`）。
`style` 对象字段：`lnRef` / `fillRef` / `effectRef` 为非负整数；`fontRef` 为 `major` / `minor` / `none`。未点名留下。
写某个 idx 时若还没有 `a:style`，补齐四个 ref：缺的矩阵 idx 为 `0`，`fontRef` 为 `minor`，颜色子元为 `schemeClr val="phClr"`。
新建 Def 时补空的 `spPr` / `bodyPr` / `lstStyle`。
docx：`id` 不是 `docx:document` 报 `InvalidId`。没有 theme 部件报 `IdNotFound`，不新建。
缺 `objectDefaults`、不是对象 / `null`、对象一个 Def 都不给、非法 `fontRef`、或 `lnW` / 矩阵 idx 非法，报 `InvalidField`。
xlsx 见 [xlsx 分册](json-contract-xlsx.md#setthemeobjectdefaults)。
pptx 见 [pptx 分册](json-contract-pptx.md#setthemeobjectdefaults)，`id` 为 `master:{n}`。
示例见 `docs/examples/docx-apply-theme-defaults.json`。

### setThemeOverride

读写主文档 `themeOverride` 部件的配色槽。根元素是 `a:themeOverride`，配色写在根下 `clrScheme`。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "ops": [
    {
      "op": "setThemeOverride",
      "id": "docx:document",
      "accent1": "FF0000"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setThemeOverride"` |
| `id` | 必须是 `docx:document` |
| `enabled` | 可选。只能是 `false`：删掉 themeOverride 部件与主文档关系 |
| `dk1` / `lt1` / `dk2` / `lt2` / `accent1`–`accent6` / `hlink` / `folHlink` | 可选。六位 RGB 十六进制 |

`enabled: false` 与配色槽不能同时出现。否则至少给一个槽位。没有部件时按 Office 默认十二槽新建 `/word/theme/themeOverride1.xml`，再把点名槽换成 `srgbClr`。有主文档 theme 时先抄它的 `clrScheme` 再打补丁。
pptx 见 [pptx 分册](json-contract-pptx.md#setthemeoverride)，`id` 为 `slide:{n}` 或 `slide:{n}/notes`。
xlsx 见 [xlsx 分册](json-contract-xlsx.md#setthemeoverride)。
示例见 `docs/examples/docx-apply-theme-override.json`。

