# pptx 领域 JSON

本文是 [json-contract.md](json-contract.md) 的 pptx 分册。这里写稳定 id、inspect 的 scope 与响应、create，以及只属于演示文稿的 apply op。

信封、跨格式 op（图、图表、SmartArt、批注、文本框、表格插删行列、`setProps`、`setShapeText` 等）以及 `exportMedia` / `exportChartData` 仍在主文件。

机器可读副本见 [json-schema/pptx.schema.json](json-schema/pptx.schema.json)，
入口仍是 [json-schema/request.schema.json](json-schema/request.schema.json)。与 Markdown 冲突时以本文与主契约为准。

## 稳定 id

演示文稿根节点 id 固定为 `pptx:presentation`。

- **页**
  - id：`slide:{n}`
  - 例子：`slide:1`
- **幻灯片母版**
  - id：`master:{n}`
  - 例子：`master:1`
- **版式**
  - id：`master:{n}/layout:{k}`
  - 例子：`master:1/layout:2`
- **母版占位符 / 装饰形**
  - id：`master:{n}/shape:{shapeId}` 或 `master:{n}/shape:index:{k}`
  - 例子：`master:1/shape:2`
  - 有 `p:ph` 的是占位符，没有的是装饰形。共用 `cNvPr/@id`，不要抢号
- **版式占位符 / 装饰形**
  - id：`master:{n}/layout:{k}/shape:{shapeId}` 或 `…/shape:index:{i}`
  - 例子：`master:1/layout:1/shape:2`
- **形状**
  - id：`slide:{n}/shape:{shapeId}`
  - 例子：`slide:1/shape:4`
- **无 `cNvPr/@id` 的形状**
  - id：`slide:{n}/shape:index:{k}`
  - 例子：`slide:1/shape:index:2`
- **连接线**
  - id：`slide:{n}/cxn:{cNvPrId}`
  - 例子：`slide:1/cxn:6`
- **无 `cNvPr/@id` 的连接线**
  - id：`slide:{n}/cxn:index:{k}`
  - 例子：`slide:1/cxn:index:1`
- **备注连接线**
  - id：`slide:{n}/notes/cxn:{cNvPrId}`
  - 例子：`slide:1/notes/cxn:6`
- **图**
  - id：`slide:{n}/pic:{cNvPrId}`
  - 例子：`slide:1/pic:4`
- **无 `cNvPr/@id` 的图**
  - id：`slide:{n}/pic:index:{k}`
  - 例子：`slide:1/pic:index:1`
- **视频**
  - id：`slide:{n}/video:{cNvPrId}`
  - 例子：`slide:1/video:3`
- **无 `cNvPr/@id` 的视频**
  - id：`slide:{n}/video:index:{k}`
  - 例子：`slide:1/video:index:1`
- **音频**
  - id：`slide:{n}/audio:{cNvPrId}`
  - 例子：`slide:1/audio:4`
- **无 `cNvPr/@id` 的音频**
  - id：`slide:{n}/audio:index:{k}`
  - 例子：`slide:1/audio:index:1`
- **备注视频**
  - id：`slide:{n}/notes/video:{cNvPrId}`
  - 例子：`slide:1/notes/video:3`
- **备注音频**
  - id：`slide:{n}/notes/audio:{cNvPrId}`
  - 例子：`slide:1/notes/audio:4`
- **图表**
  - id：`slide:{n}/chart:{cNvPrId}`
  - 例子：`slide:1/chart:5`
- **无 `cNvPr/@id` 的图表**
  - id：`slide:{n}/chart:index:{k}`
  - 例子：`slide:1/chart:index:1`
- **系列**
  - id：`{图表 id}/series:{idx}`
  - 例子：`slide:1/chart:5/series:0`
- **趋势线**
  - id：`{系列 id}/trendline:{k}`
  - 例子：`slide:1/chart:5/series:0/trendline:1`
- **误差线**
  - id：`{系列 id}/errorBars:{k}`
  - 例子：`slide:1/chart:5/series:0/errorBars:1`
- **SmartArt**
  - id：`slide:{n}/smartdiagram:{cNvPrId}`
  - 例子：`slide:1/smartdiagram:4`
- **无 `cNvPr/@id` 的 SmartArt**
  - id：`slide:{n}/smartdiagram:index:{k}`
  - 例子：`slide:1/smartdiagram:index:1`
- **SmartArt 点**
  - id：`{图示 id}/pt:{modelId}`
  - 例子：`slide:1/smartdiagram:4/pt:2`
- **备注容器**
  - id：`slide:{n}/notes`
  - 例子：`slide:1/notes`
- **备注形状**
  - id：`slide:{n}/notes/shape:{shapeId}`
  - 例子：`slide:1/notes/shape:3`
- **无 `cNvPr/@id` 的备注形状**
  - id：`slide:{n}/notes/shape:index:{k}`
  - 例子：`slide:1/notes/shape:index:1`
- **备注图**
  - id：`slide:{n}/notes/pic:{cNvPrId}`
  - 例子：`slide:1/notes/pic:5`
- **无 `cNvPr/@id` 的备注图**
  - id：`slide:{n}/notes/pic:index:{k}`
  - 例子：`slide:1/notes/pic:index:1`
- **备注图表**
  - id：`slide:{n}/notes/chart:{cNvPrId}`
  - 例子：`slide:1/notes/chart:6`
- **无 `cNvPr/@id` 的备注图表**
  - id：`slide:{n}/notes/chart:index:{k}`
  - 例子：`slide:1/notes/chart:index:1`
- **备注 SmartArt**
  - id：`slide:{n}/notes/smartdiagram:{cNvPrId}`
  - 例子：`slide:1/notes/smartdiagram:7`
- **无 `cNvPr/@id` 的备注 SmartArt**
  - id：`slide:{n}/notes/smartdiagram:index:{k}`
  - 例子：`slide:1/notes/smartdiagram:index:1`
- **表**
  - id：`slide:{n}/table:{cNvPrId}`
  - 例子：`slide:1/table:4`
- **无 `cNvPr/@id` 的表**
  - id：`slide:{n}/table:index:{k}`
  - 例子：`slide:1/table:index:1`
- **备注表**
  - id：`slide:{n}/notes/table:{cNvPrId}`
  - 例子：`slide:1/notes/table:5`
- **无 `cNvPr/@id` 的备注表**
  - id：`slide:{n}/notes/table:index:{k}`
  - 例子：`slide:1/notes/table:index:1`
- **表行**
  - id：`{表 id}/row:{r}`
  - 例子：`slide:1/table:4/row:2`
- **表单元格**
  - id：`{表 id}/r:{r}/c:{c}`
  - 例子：`slide:1/table:4/r:1/c:2`
- **嵌入对象**
  - id：`slide:{n}/ole:{cNvPrId}`
  - 例子：`slide:1/ole:4`
- **无 `cNvPr/@id` 的嵌入对象**
  - id：`slide:{n}/ole:index:{k}`
  - 例子：`slide:1/ole:index:1`
- **备注嵌入对象**
  - id：`slide:{n}/notes/ole:{cNvPrId}`
  - 例子：`slide:1/notes/ole:4`
- **3D 模型**
  - id：`slide:{n}/model3d:{cNvPrId}`
  - 例子：`slide:1/model3d:4`
- **无 `cNvPr/@id` 的 3D 模型**
  - id：`slide:{n}/model3d:index:{k}`
  - 例子：`slide:1/model3d:index:1`
- **备注 3D 模型**
  - id：`slide:{n}/notes/model3d:{cNvPrId}`
  - 例子：`slide:1/notes/model3d:2`
- **公式框**
  - id：`slide:{n}/math:{cNvPrId}`
  - 例子：`slide:1/math:4`
- **无 `cNvPr/@id` 的公式框**
  - id：`slide:{n}/math:index:{k}`
  - 例子：`slide:1/math:index:1`
- **备注公式框**
  - id：`slide:{n}/notes/math:{cNvPrId}`
  - 例子：`slide:1/notes/math:2`
- **Zoom**
  - id：`slide:{n}/zoom:{cNvPrId}`
  - 例子：`slide:1/zoom:3`
- **无 `cNvPr/@id` 的 Zoom**
  - id：`slide:{n}/zoom:index:{k}`
  - 例子：`slide:1/zoom:index:1`
- **备注 Zoom**
  - id：`slide:{n}/notes/zoom:{cNvPrId}`
  - 例子：`slide:1/notes/zoom:3`
- **批注**
  - id：`slide:{n}/comment:{guid}`
  - 例子：`slide:1/comment:{A1B2C3D4-E5F6-4789-ABCD-EF1234567890}`
- **批注回复**
  - id：`{批注 id}/reply:{guid}`
  - 例子：`slide:1/comment:{A1B2C3D4-E5F6-4789-ABCD-EF1234567890}/reply:{11111111-2222-4333-8444-555555555555}`
- **批注内图**
  - id：`{批注 id}/pic:{k}`
  - 例子：`slide:1/comment:{A1B2C3D4-E5F6-4789-ABCD-EF1234567890}/pic:1`
- **超链接**
  - id：`slide:{n}/link:{k}`
  - 例子：`slide:1/link:1`
- **备注超链接**
  - id：`slide:{n}/notes/link:{k}`
  - 例子：`slide:1/notes/link:1`
- **字段**
  - id：`slide:{n}/fld:{guid}`
  - 例子：`slide:1/fld:{8900A98D-CC96-461D-8E1A-9E2F9D60FB49}`
- **无 `@id` 的字段**
  - id：`slide:{n}/fld:index:{k}`
  - 例子：`slide:1/fld:index:1`
- **备注字段**
  - id：`slide:{n}/notes/fld:{guid}`
  - 例子：`slide:1/notes/fld:{8900A98D-CC96-461D-8E1A-9E2F9D60FB49}`
- **母版 / 版式对象**
  - id：与幻灯片同一套段名，前缀换成 `master:{n}` 或 `master:{n}/layout:{k}`
  - 例子：`master:1/pic:4`、`master:1/layout:1/table:5`、`master:1/fld:{guid}`、`master:1/link:1`
  - 含图、图表、SmartArt、表、视频、音频、OLE、Zoom、连接线、公式、3D、超链接、域
  - 不含备注母版与讲义
- **讲义母版**
  - id：`handout`
  - 例子：`handout`
- **讲义占位符**
  - id：`handout/shape:{shapeId}`
  - 例子：`handout/shape:2`
- **备注母版**
  - id：`notesMaster`
  - 例子：`notesMaster`
- **备注母版占位符**
  - id：`notesMaster/shape:{shapeId}`
  - 例子：`notesMaster/shape:5`
- **幻灯片节**
  - id：`section:{n}`
  - 例子：`section:1`
- **自定义放映**
  - id：`show:{n}`
  - 例子：`show:1`
- **自定义属性**
  - id：`prop:{name}`
  - 例子：`prop:Editor`

`{n}` 从 1 起，按 Presentation 部件（§13.3.6）里 `p:sldIdLst` / `p:sldId` 的出现顺序（§19.2.1.34），再跟 `r:id` 找到幻灯片部件。
`n` 不是文件名里的数字，也不是 `p:sldId/@id`（那是 `ST_SlideId`，常见从 256 起）。隐藏页仍占序号，仍可 inspect 和 apply。
`insertSlide` / `deleteSlide` / `moveSlide` 之后名单重排，旧的 `slide:{n}` 以及挂在它下面的形状 / 图 / 图表 / 表 / 备注 / 批注 id 都按新顺序计。
只够同一次 inspect 之后立刻 apply；同一 `apply` 里后面的 op 用新序号。

母版 `{n}` 从 1 起，按 Presentation 里 `p:sldMasterIdLst` / `p:sldMasterId` 的出现顺序（§19.2.1.35、§13.3.10），再跟 `r:id` 找到 slideMaster 部件。
不是文件名里的数字，也不是 `sldMasterId/@id`。版式 `{k}` 从 1 起，按**该母版** `p:sldLayoutIdLst` / `p:sldLayoutId` 的出现顺序，
再跟母版 `.rels` 找到 slideLayout 部件（§13.3.9）。
换页的版式关系不改这些序号。讲义母版最多一份（`p:handoutMasterIdLst` / `p:handoutMasterId`，§19.2.1.24、§19.3.1.24）。容器 id 固定为 `handout`。
讲义占位符编成 `handout/shape:…`。备注母版最多一份（`p:notesMasterIdLst` / `p:notesMasterId`，§19.2.1.27、§19.3.1.27）。
容器 id 固定为 `notesMaster`。备注母版占位符编成 `notesMaster/shape:…`。

`{shapeId}` 取该形状 `p:nvSpPr/p:cNvPr/@id` 的原文（§19.3.1.12）。输出用文件里的原始数字字符串。请求按十进制数值匹配，不因前导零而失败。

没有 `cNvPr/@id` 时用 `slide:{n}/shape:index:{k}`，`k` 从 1 起，按该页列入树的形状顺序。树上同时给 `"stable": false`。
这类 id 在增删形状后会变，只够同一次 inspect 之后立刻 apply。Office 亲手存的文件几乎都有 `cNvPr/@id`。

连接线 id 的 `{cNvPrId}` 取该 `p:cxnSp` 的 `p:nvCxnSpPr/p:cNvPr/@id` 原文（§19.3.1.19）。
没有则用 `slide:{n}/cxn:index:{k}`，`stable` 为 `false`。`{k}` 只在该页全部被收入的 `p:cxnSp` 内从 1 计，与 `shape:index` / `pic:index` 分家。
只认 `p:cxnSp`，不认无 `txBody` 的 `p:sp`。组合里符合条件的连接线要展开，组合本身不进树。备注页、幻灯片母版、版式上的连接线进树，id 带对应前缀。批注、备注母版、讲义上的连接线不进树。
`insertConnector` 的 `id` 必须是 `slide:{n}`、`slide:{n}/notes`、`master:{n}` 或 `master:{n}/layout:{k}`。`deleteConnector` 的 `id` 必须是连接线 id。
`setShapePos` / `setShapeSize` / `setShapeLine` 也认连接线 id。
对连接线发 `setShapeText` / `setShapeFormat` / `setShapeFill` / `deleteShape` 报 `InvalidId`。

图 id 的 `{cNvPrId}` 取该 `p:pic` 的 `p:nvPicPr/p:cNvPr/@id` 原文。没有该属性时用 `slide:{n}/pic:index:{k}`，`stable` 为 `false`。
`{k}` 只在该页全部被收入的 `p:pic` 内从 1 计，与形状的 `shape:index` 分家，互不影响。
只认带 `a:blip` 的 `p:pic`，且 `p:nvPr` 下没有 `a:videoFile` / `a:audioFile` / `a:wavAudioFile`。
带这三种子元素的 `p:pic` 走视频或音频，不进 `pictures`。`p:sp`、连接线、`p:graphicFrame`（图表 / 表 / SmartArt / OLE / Zoom）不进 `pictures`。
组合本身不进树，里面符合条件的 `p:pic` 要展开。OLE 预览 `p:pic`（写在 `p:oleObj` 里，常在 `mc:Fallback`）不进 `pictures`，也不计入 `pictureCount`。
Zoom Fallback 预览 `p:pic` 同样不进 `pictures`。`mc:AlternateContent` 只进第一个 `Choice`，规则与形状相同。

视频 id 的 `{cNvPrId}` 取该 `p:pic` 的 `p:nvPicPr/p:cNvPr/@id` 原文。没有则用 `slide:{n}/video:index:{k}`，`stable` 为 `false`。
`{k}` 只在该页全部被收入的视频内从 1 计，与 `pic:index` / `audio:index` / `shape:index` 分家。
只认 `p:nvPr` 下带 `a:videoFile` 的 `p:pic`（ECMA-376 §20.1.3.6）。海报 `a:blip` 有无都不影响是否进 `videos`。组合里符合条件的 `p:pic` 要展开。备注页、幻灯片母版、版式上的视频进树，id 带对应前缀。批注、备注母版、讲义上的视频不进树。

音频 id 同样取 `cNvPr/@id`，没有则用 `slide:{n}/audio:index:{k}`。`{k}` 只在该页全部被收入的音频内从 1 计。
只认 `p:nvPr` 下带 `a:audioFile`（§20.1.3.5）或 `a:wavAudioFile`（§20.1.3.7）的 `p:pic`；同时带 `a:videoFile` 时算视频，不算音频。备注页、幻灯片母版、版式上的音频进树，id 带对应前缀。批注、备注母版、讲义上的音频不进树。

`insertVideo` / `insertAudio` 的 `id` 必须是 `slide:{n}`、`slide:{n}/notes`、`master:{n}` 或 `master:{n}/layout:{k}`。
`deleteMedia` / `setMedia` 的 `id` 必须是视频 id 或音频 id。
`setPicturePos` / `setPictureSize` 的 id 也可以是视频 id 或音频 id。
对图 id 发 `setMedia` / `insertVideo` / `insertAudio` / `deleteMedia` 报 `InvalidId`。`id` 为 `slide:{n}/notes` 且该页还没有 notesSlide 时，按 `setNotesText` 同一套新建最小备注页再插入。
对 `notesMaster` / `handout` 发 `insertVideo` / `insertAudio` 报 `InvalidId`。

节 `{n}` 从 1 起，按 Presentation 里 `p:extLst/p:ext[@uri="{521415D9-36F7-43E2-AB2F-B90AF26B5E84}"]/p14:sectionLst/p14:section`
 的出现顺序（MS-PPTX §2.3.1.25、§2.3.3.24）。
不是 `p14:section/@id` 那个 GUID，也不是 Word 的 `w:sectPr`。没有这份扩展则为 0 节。
`insertSlideSection` / `deleteSlideSection` 之后旧的 `section:{n}` 会变，只够同一次 inspect 之后立刻 apply。
`insertSlideSection` 的 `id` 必须是 `pptx:presentation`。
`setSlideSectionName` / `deleteSlideSection` 的 `id` 必须是 `section:{n}`。`setSlideSection` 的 `id` 必须是 `slide:{n}`。
对不上的 id 形态报 `InvalidId`。docx 的 `insertSection` / `deleteSection` 在 pptx 上仍报 `InvalidField`。

自定义放映 `{n}` 从 1 起，按 Presentation 里 `p:custShowLst/p:custShow` 的出现顺序（ECMA-376 §19.2.1.5–19.2.1.7）。
不是 `p:custShow/@id`（那是 `xsd:unsignedInt`，Office 另存从 0 起），也不是 `ST_SlideId`。
名单里的页是 `p:sld/@r:id`，对到 `p:sldIdLst` 里同一条 `r:id` 之后的 `slide:{n}`。没有 `custShowLst` 则为 0 条。
`insertCustomShow` / `deleteCustomShow` 之后旧的 `show:{n}` 会变，只够同一次 inspect 之后立刻 apply。
`insertCustomShow` 的 `id` 必须是 `pptx:presentation`。`setCustomShow` / `deleteCustomShow` 的 `id` 必须是 `show:{n}`。
对不上的 id 形态报 `InvalidId`。不实现放映引擎。`showPr` 用 `setShowPr` 改，不要跟自定义放映名单绑在一起。

图表 id 的 `{cNvPrId}` 取该 `p:graphicFrame` 的 `p:nvGraphicFramePr/p:cNvPr/@id` 原文。
没有则用 `slide:{n}/chart:index:{k}`，`stable` 为 `false`。`{k}` 只在该页全部被收入的图表内从 1 计，与 `pic:index` / `shape:index` 分家。
只认 `p:graphicFrame` 且 `a:graphicData/@uri` 为 chart URI（可带尾斜杠）。chartex URI 列入，`type` 为 `chartex`。
`p:pic`、表格、SmartArt 不进 `charts`。组合里符合条件的框要展开。系列 `{idx}` 从 0 起，按 chart 部件里 `c:ser` 的文档顺序，不跟 `c:idx/@val`。
趋势线 id 为 `{系列 id}/trendline:{k}`，误差线 id 为 `{系列 id}/errorBars:{k}`，`{k}` 从 1 起。
数据在 embedding 工作簿与 cache 里；以 embedding 为准，不一致则该系列 `cacheStale: true`。
备注页上同样判定，id 带 `slide:{n}/notes/` 前缀；`{k}` 只在该备注页被收入的图表内从 1 计，不和幻灯片图表混编。

SmartArt id 的 `{cNvPrId}` 取该 `p:graphicFrame` 的 `p:nvGraphicFramePr/p:cNvPr/@id` 原文。
没有则用 `slide:{n}/smartdiagram:index:{k}`，`stable` 为 `false`。
`{k}` 只在该页全部被收入的图示内从 1 计，与 `pic:index` / `chart:index` / `shape:index` 分家。
只认 `p:graphicFrame` 且 `a:graphicData/@uri` 为 `http://schemas.openxmlformats.org/drawingml/2006/diagram`（可带尾斜杠，ECMA-376 §21.4）。
`dgm:relIds` 的四条关系写在该宿主 `.rels` 上（幻灯片或 notesSlide）。`p:pic`、图表、表格不进 `smartArts`。组合里符合条件的框要展开。点 id 规则与 docx 相同。
备注页上同样判定，id 带 `slide:{n}/notes/` 前缀；`{k}` 只在该备注页被收入的图示内从 1 计。幻灯片母版 / 版式上同样判定，id 带 `master:{n}/` 或 `master:{n}/layout:{k}/` 前缀。批注、备注母版、讲义上的图示不进树。

表 id 的 `{cNvPrId}` 取该 `p:graphicFrame` 的 `p:nvGraphicFramePr/p:cNvPr/@id` 原文。
没有则用 `slide:{n}/table:index:{k}`，`stable` 为 `false`。
`{k}` 只在该页全部被收入的表内从 1 计，与 `pic:index` / `chart:index` / `smartdiagram:index` / `shape:index` 分家。
只认 `p:graphicFrame` 且 `a:graphicData/@uri` 为 `http://schemas.openxmlformats.org/drawingml/2006/table`（可带尾斜杠，ECMA-376 §21.1.3.13）。
图表 URI、diagram URI 不进 `tables`。`p:pic`、带 `txBody` 的 `p:sp` 不进 `tables`。组合里符合条件的框要展开，组合本身不进树。
`mc:AlternateContent` 只进第一个 Choice。行号 `{r}` 是该 `a:tbl` 内 `a:tr` 的文档顺序，从 1 起。
单元格 `{c}` 是该 `a:tc` 在网格上的起始列，从 1 起：无 `hMerge` 时该格占 `gridSpan` 列（缺省 1，§21.1.3.16）；随后的 `hMerge` 继续格仍列出，各占 1 列。
垂直合并的 `vMerge` 继续格仍有 `a:tc`，树上 `vMerge` 为 `true`。`colCount` 取 `a:tblGrid` 的 `a:gridCol` 个数；没有则取第一行各格占的列之和。
备注页上同样判定，id 带 `slide:{n}/notes/` 前缀；`{k}` 只在该备注页被收入的表内从 1 计，不和幻灯片表混编。幻灯片母版 / 版式上同样判定，id 带 `master:{n}/` 或 `master:{n}/layout:{k}/` 前缀。批注、备注母版、讲义上的表不进树。

`insertChart` 的 `id` 必须是 `slide:{n}`、`slide:{n}/notes`、`master:{n}` 或 `master:{n}/layout:{k}`。
`deleteChart` / `setChartTitle` / `setChartSeries` / `addChartSeries` / `deleteChartSeries` / `setChartType`
 / `setChartAxisTitle` / `setChartLegend` / `setChartDataLabels` / `setChartSize` / `setChartPos` 以及趋势线、误差线、次轴 op 的图表 id 可以带 `notes/` 或母版 / 版式前缀。
`insertSmartArt` 的 `id` 同样是 `slide:{n}`、`slide:{n}/notes`、`master:{n}` 或 `master:{n}/layout:{k}`。点 id、删整张与改点 op 认备注与母版 / 版式图示 id。

`insertTable` 的 `id` 必须是 `slide:{n}`、`slide:{n}/notes`、`master:{n}` 或 `master:{n}/layout:{k}`。
`deleteTable` / `setTablePos` / `setTableSize` / `setTableColWidths` / `insertCol` / `deleteCol` 的 `id` 必须是表 id（含备注表 id）。
`insertRow` 的 `id` 是表 id（追加一行）或行 id（插到该行之后）。`deleteRow` / `setTableTrHeight` 的 `id` 必须是行 id。`setTableCellText` / `setTableCellFill` / `setTableCellBorders` / `setTableCellInset` / `setTableCellPr` 的 `id` 必须是单元格 id。
对不上的 id 形态报 `InvalidId`。xlsx 上这些 op（除 Word 已认的结构名在 Word 上的语义外）见各节；pptx 上发 `mergeCells` / `splitCells` 仍报 `InvalidField`。

嵌入对象 id 的 `{cNvPrId}` 取该 `p:graphicFrame` 的 `p:nvGraphicFramePr/p:cNvPr/@id` 原文（ECMA-376 §19.3.2.4：以 graphicFrame 为准，忽略预览 `p:pic` 的 id）。
没有则用 `slide:{n}/ole:index:{k}`，`stable` 为 `false`。
`{k}` 只在该页全部被收入的嵌入对象内从 1 计，与 `pic:index` / `table:index` / `chart:index` / `shape:index` 分家。
只认 `p:graphicFrame` 且 `a:graphicData/@uri` 为 Transitional `http://schemas.openxmlformats.org/presentationml/2006/ole` 或
ISO `http://purl.oclc.org/ooxml/presentationml/ole`（可带尾斜杠），
Choice 里有 `p:oleObj`，带 `p:embed`、没有 `p:link`，且 `r:id` 能对上非 External、Type 后缀为 `oleObject` 或 `package` 的关系。
`p:link`、ActiveX `p:control` 不进树。组合里符合条件的框要展开，组合本身不进树。备注页、幻灯片母版、版式上的 OLE 进树，id 带对应前缀。批注、备注母版、讲义上的 OLE 不进树。`insertOle` 的 `id` 必须是 `slide:{n}`、`slide:{n}/notes`、`master:{n}` 或 `master:{n}/layout:{k}`。
`deleteOle` 的 `id` 必须是 OLE id。对不上的 id 形态报 `InvalidId`。xlsx 见 [xlsx 分册](json-contract-xlsx.md#insertole)。
docx 上仍是段落 id / Word OLE id，见 [json-contract-docx.md](json-contract-docx.md#insertole)。不提供改尺寸、位置、图标或 ProgID 的 op。不写 `<p:timing>`。

3D 模型 id 的 `{cNvPrId}` 取 Choice 里 `p:graphicFrame` 的 `p:nvGraphicFramePr/p:cNvPr/@id` 原文。
没有则用 `slide:{n}/model3d:index:{k}`，`stable` 为 `false`。
`{k}` 只在该页全部被收入的 3D 模型内从 1 计，与 `pic:index` / `ole:index` / `shape:index` 分家。
只认 `mc:AlternateContent` 且 `Choice Requires` 含 `am3d`，Choice 里 `a:graphicData/@uri` 为
`http://schemas.microsoft.com/office/drawing/2017/model3d`（可带尾斜杠）。
只走 `Choice Requires="am3d"`。Fallback 海报 `p:pic` 不进 `pictures`。组合里符合条件的框要展开，组合本身不进树。
备注页上同样判定，id 带 `slide:{n}/notes/` 前缀。幻灯片母版 / 版式上同样判定，id 带 `master:{n}/` 或 `master:{n}/layout:{k}/` 前缀。批注、备注母版、讲义上的 3D 不进树。
`insertModel3d` 的 `id` 必须是 `slide:{n}`、`slide:{n}/notes`、`master:{n}` 或 `master:{n}/layout:{k}`。`replaceModel3d` / `deleteModel3d` / `setModel3dSize` / `setModel3dPos` 的 `id` 必须是 3D 模型 id。
只认 GLB。不是 GLB 报 `InvalidImageFile`（对外 `InvalidField`）。`contentType` 恒为 `model/gltf.binary`。
批注内仍不插。幻灯片、备注页与母版 / 版式上已有 3D 可改尺寸与画布坐标。
xlsx 见 [xlsx 分册](json-contract-xlsx.md#insertmodel3d)。docx 见 [json-contract-docx.md](json-contract-docx.md#insertmodel3d)。

公式框 id 的 `{cNvPrId}` 取 Choice 里 `p:sp` 的 `p:nvSpPr/p:cNvPr/@id` 原文。
没有则用 `slide:{n}/math:index:{k}`，`stable` 为 `false`。
`{k}` 只在该页全部被收入的公式框内从 1 计，与 `shape:index` / `pic:index` 分家。
只认 `mc:AlternateContent` 且 Choice `Requires` 含 `a14`，子孙有 `a14:m` 或 `m:oMath`。整段只进 `maths`。
Choice 与 Fallback 都不进 `shapes`，也不计入 `shapeCount`。组合里符合条件的公式框要展开，组合本身不进树。
备注页上同样判定，id 带 `slide:{n}/notes/` 前缀。幻灯片母版 / 版式上同样判定，id 带 `master:{n}/` 或 `master:{n}/layout:{k}/` 前缀。批注、备注母版、讲义不进树。
`insertMath` 的 `id` 必须是 `slide:{n}`、`slide:{n}/notes`、`master:{n}` 或 `master:{n}/layout:{k}`。`setMath` / `deleteMath` / `setMathSize` / `setMathPos` 的 `id` 必须是公式框 id。
对形状 id、图 id、`pptx:presentation` 发公式 op 报 `InvalidId`。
对形状 id 发 `setMathSize` / `setMathPos`，或对公式框 id 发 `setShapeSize` / `setShapePos`，仍报 `InvalidId`。出现 `expr` / `kind` 报 `InvalidField`。
不把 OMML 原文、不把 `expr` 放进树。create 可用 `slides[].maths`。
`slides[].notes` 为对象时可带 `maths`，字段与页上 `maths[]` 相同。字符串形式的 `notes` 仍只表示备注文本。
xlsx 见 [xlsx 分册](json-contract-xlsx.md#insertmath)。docx 见 [json-contract-docx.md](json-contract-docx.md#insertmath)。

Zoom id 的 `{cNvPrId}` 取 Choice 里 `p:graphicFrame` 的 `p:nvGraphicFramePr/p:cNvPr/@id` 原文。
没有则取 Fallback `p:pic` 的 `cNvPr/@id`。再没有则用 `slide:{n}/zoom:index:{k}`，`stable` 为 `false`。
`{k}` 只在该页全部被收入的 Zoom 内从 1 计，与 `pic:index` / `ole:index` / `table:index` / `chart:index` / `shape:index` 分家。
只认 `spTree` / `grpSp` 里的 `mc:AlternateContent`，
且第一个 Choice 含 `sldZm` / `sectionZm` / `summaryZm`（可包在 `p:graphicFrame` 里）。
`a:graphicData/@uri` 为 `http://schemas.microsoft.com/office/powerpoint/2016/slidezoom`、
`http://schemas.microsoft.com/office/powerpoint/2016/sectionzoom` 或
`http://schemas.microsoft.com/office/powerpoint/2016/summaryzoom`（可带尾斜杠，MS-PPTX §2.2.15 / §2.9–2.11）。
无 AlternateContent、仅 `p:graphicFrame` 且 uri 对上上述三者，也收。整段 AlternateContent 算 1 个 Zoom；摘要缩放多个 `summaryZmObj` 仍是一条。
组合里符合条件的要展开，组合本身不进树。Fallback 预览 `p:pic` 不进 `pictures`，也不计入 `pictureCount`。备注页、幻灯片母版、版式上的 Zoom 进树，id 带对应前缀。批注、备注母版、讲义上的 Zoom 不进树。
`insertZoom` 的 `id` 必须是 `slide:{n}`、`slide:{n}/notes`、`master:{n}` 或 `master:{n}/layout:{k}`。`deleteZoom` / `setZoom` 的 `id` 必须是 Zoom id。对不上的 id 形态报 `InvalidId`。
xlsx / docx 上发这三条报 `InvalidField`。`insertZoom` 可写 `type=summary`。`setZoom` 改已有 Zoom 的目标、预览或画布几何。不写 `<p:timing>`。不算放映引擎。

同一页里若出现重复的 `cNvPr/@id`，inspect 照样列出每一处；apply 命中该页文档顺序中的第一处。

批注 `{guid}` 取该页现代批注部件里 `p188:cm/@id` 的原文（MS-PPTX 2018/8），含花括号。回复 `{guid}` 取同一条 `p188:cm` 里 `p188:reply/@id` 的原文。
按文件原文精确匹配，不因大小写或有无花括号而放宽。同一页多条批注共用一个 comments 部件。
ECMA-376 §19.4 旧式 `/ppt/comments/commentN.xml` 与 `commentAuthors` 不进树、不接受 apply；
只有旧式时 `commentCount` 为 `0`，`scope.comments` 为空数组。insert 只写现代部件。

现代批注没有 `p:pic`。批注内图只收该 `p188:cm`（不含 `replyLst`）下带 `r:embed` 或 `asvg:svgBlip` 的 `a:blip`。`{k}` 只在该条批注内从 1 计。
回复内同样判定，id 为 `{回复 id}/pic:{k}`。不并进幻灯片 `pictureCount`。`insertPicture` 的 `id` 为批注或回复 id，写入 `a:blip`（见主契约）。
对 `kind` 为 `ink` 的批注或回复报 `NotImplemented`。`replacePicture` / `deletePicture` / `exportMedia` 认批注内图 id。
`setPictureSize` / `setPicturePos` 仍报 `InvalidId`（没有画布 `xfrm`）。

超链接 `{k}` 从 1 起，按该页 `spTree` 文档顺序（含组合；`mc:AlternateContent` 只进第一个 Choice）：
每个对象先 `cNvPr` 的 `a:hlinkClick`（若有），再 `a:hlinkHover`（若有）；
文本形状再按 `a:r` 顺序收 `a:rPr/a:hlinkClick` 与 `a:hlinkMouseOver`；
表框之后再按单元格行列收格内 run 链。
不认 `a:fld` / `a:br` / `endParaRPr` 上的链。不把 `ppaction://media` 收进树。不收 Zoom。
备注页、幻灯片母版、版式上同样走这套对象。批注、备注母版、讲义上的链不进树。
增删之后 `{k}` 会变，只够同一次 inspect 之后立刻 apply。
`insertHyperlink` 的 `id` 必须是文本形状、图、表、图表、SmartArt 或表单元格 id。
`setHyperlink` / `deleteHyperlink` 的 `id` 必须是超链接 id。对不上的 id 形态报 `InvalidId`。

字段 `{guid}` 取该 `a:fld/@id` 原文（ECMA-376 §21.1.2.2.4，`ST_Guid`，含花括号）。没有 `@id` 则为 `slide:{n}/fld:index:{k}`。
`{k}` 只在该宿主被收入的域内从 1 计。只收文本形状（带 `p:txBody` 的 `p:sp`，含组合；`mc:AlternateContent` 只进第一个 Choice）里的 `a:fld`。
备注页、幻灯片母版、版式上的域进树，id 带对应前缀。不进表、讲义、备注母版、批注。
增删之后无 `@id` 的序号会变，只够同一次 inspect 之后立刻 apply。`insertField` 的 `id` 必须是幻灯片、备注、母版或版式文本形状 id。
`setField` / `deleteField` 的 `id` 必须是 pptx 域 id。对不上的 id 形态报 `InvalidId`。xlsx 上这三条仍报 `InvalidField`。
docx 仍写 `w:fldChar` / `instr`，见 [json-contract-docx.md](json-contract-docx.md#insertfield)。不算放映引擎，不在库里按页码或系统时钟刷新 `a:t` 缓存。

apply 不得把**未点名**的形状删掉再重建，以免 `cNvPr/@id` 变化导致绑在形状上的动画失效。
`deleteShape` 只摘被点名的那一个 `p:sp`，并清掉 `<p:timing>` 里对应的 `p:tgtEl`（`p:spTgt/@spid` 对该形状 `cNvPr/@id`）；未点名形状的 timing 节点留下，不要整节删掉 `<p:timing>`。
插、删、重排页只改 `p:sldIdLst` 与被点名页的部件，未点名页 XML 原字节写回。

讲义母版始终给出 `hasHandout`；`scope.handout: true` 时展开根上 `handout` 对象（theme、页眉、页脚、日期、页码槽位与占位符名单）。
`scope.notesMaster: true` 时展开根上 `notesMaster`（theme 与占位符名单）。
幻灯片节始终给出 `sectionCount`；`scope.sections: true` 时展开根上 `sections`。
自定义放映始终给出 `customShowCount`；`scope.customShows: true` 时展开根上 `customShows`。
嵌入对象始终给出 `oleObjectCount`；`scope.oleObjects: true` 时展开各页 `oleObjects`。
Zoom 始终给出 `zoomCount`；`scope.zooms: true` 时展开各页 `zooms`。根上始终给出 `showPr` 与 `modifyProtection`，不要另给 scope 键。
根上始终给出 `slideSize` 与 `notesSize`。
幻灯片母版与版式见下文 `scope.masters`。备注页始终给出 `hasNotes`；`scope.notes: true` 时展开该页 `notesSlide` 上的文本形状。
xlsx 批注走 `cell:{name}!{A1}/comment:{guid}`，不走 docx 的 `comment:{w:id}`，也不走 pptx 的 `slide:{n}/comment:{guid}`。
pptx 批注走 `slide:{n}/comment:{guid}`，不走 `comment:{w:id}`。

「文本形状」要有可执行的判定：

- 只收带 `p:txBody` 的 `p:sp`（§19.3.1.45、§19.3.1.51），含嵌在 `p:grpSp` 里的。标题占位符、正文占位符、自由文本框、椭圆 / 箭头一类非矩形预设几何都算。
- 不收 `p:pic`、`p:cxnSp`、`p:graphicFrame`（表格、图表、SmartArt 等）、`p:contentPart`。组合本身不进树，只展开里面符合条件的 `p:sp`。
  连接线另走 `connectors`，id 前缀是 `cxn:`，不要编进 `shapes`。
- 遇到 `mc:AlternateContent` 只走第一个 `mc:Choice`；没有 `mc:Choice` 才走 `mc:Fallback`。两支都走会把同一份内容读出两遍，也会让 apply 只改到其中一支。这条对 `shapes`、`text` 和 apply 一律适用。
  若 Choice `Requires` 含 `a14` 且子孙有 `a14:m` 或 `m:oMath`，整段不进 `shapes`，只进 `maths`。


## inspect

xlsx、docx、pptx 共用请求字段名 `scope`，里面的键按格式分，混用报 `InvalidField`。请求信封见 [json-contract.md](json-contract.md#请求)。

### scope

```json
{
  "schemaVersion": 1,
  "action": "inspect",
  "path": "C:\\data\\deck.pptx",
  "format": "pptx",
  "scope": {
    "slide": 1,
    "maxShapes": 200
  }
}
```

- **`path`**（是）
  - 已有文件
- **`scope`**（否）
  - 省略或空对象：返回下列计数、结构和页名单
    - `slideCount`
    - `pictureCount`
    - `mathCount`
    - `videoCount`
    - `audioCount`
    - `chartCount`
    - `smartArtCount`
    - `commentCount`
    - `hyperlinkCount`
    - `fieldCount`
    - `tableCount`
    - `oleObjectCount`
    - `model3dCount`
    - `zoomCount`
    - `connectorCount`
    - `animCount`
    - `masterCount`
    - `hasHandout`
    - `sectionCount`
    - `slideSize`
    - `notesSize`
    - `customShowCount`
    - `showPr`
    - `modifyProtection`
  - 各页有这些计数、`layoutId`、`hasNotes` 和 `transition`
  - 没有 `shapes`
  - 也没有 `pictures` / `maths` / `videos` / `audios` / `charts` / `smartArts` / `tables` / `oleObjects` / `model3ds` / `zooms` / `connectors` / `timing` 名单
  - 也没有 `notes` 对象
  - 也没有 `comments` 名单
  - 也没有 `hyperlinks` 名单
  - 也没有 `fields` 名单
  - 也没有根上 `masters`
  - 也没有根上 `notesMaster`
  - 也没有根上 `handout`
  - 也没有根上 `sections`
  - 也没有根上 `customShows`
- **`scope.slide`**（要形状时必填）
  - 页序号，从 1 起，按 `p:sldIdLst` 顺序
- **`scope.maxShapes`**（否）
  - 该页最多返回多少个文本形状，默认 `2000`，允许范围 `1` 到 `5000`
  - 不管备注形状
- **`scope.pictures`**（否）
  - 为 `true` 时给每页填 `pictures`
  - 若同时 `notes: true` 且该页有备注对象，备注上也填 `pictures`
  - 若同时 `comments: true`，每条批注上也填 `pictures`
  - 省略或 `false` 则只有计数，没有名单
  - 不读媒体字节
  - 不依赖 `scope.slide`
- **`scope.maths`**（否）
  - 为 `true` 时给每页填 `maths`
  - 若同时 `notes: true` 且该页有备注对象，备注上也填 `maths`
  - 省略或 `false` 则只有 `mathCount`，没有名单
  - 不依赖 `scope.slide`
  - 不把 OMML 原文、不把 `expr` 放进树
  - 备注里的公式框不进幻灯片 `maths`，只在 `scope.notes` 时出现在 `notes` 上
  - 不是单元格 `f`
  - docx 的 `scope.maths` 见 [docx 分册](json-contract-docx.md#inspect)
- **`scope.media`**（否）
  - 为 `true` 时给每页填 `videos` 与 `audios`
  - 若同时 `notes: true` 且该页有备注对象，备注上也填 `videos` 与 `audios`
  - 若同时 `masters: true`，母版 / 版式上也填
  - 省略或 `false` 则只有 `videoCount` / `audioCount`，没有名单
  - 不读媒体字节，不解码播放
  - 不依赖 `scope.slide` / `pictures`
  - xlsx / docx 的 `scope` 里出现 `media` 报 `InvalidField`
  - 备注里的媒体不进幻灯片 `videos` / `audios`，只在 `scope.notes` 时出现在 `notes` 上
- **`scope.charts`**（否）
  - 为 `true` 时给每页填 `charts`
  - 若同时 `notes: true` 且该页有备注对象，备注上也填 `charts`
  - 省略或 `false` 则只有 `chartCount`，没有名单
  - 不把 chart 部件或 embedding 字节放进树
  - 不依赖 `scope.slide` / `pictures`
- **`scope.smartArt`**（否）
  - 为 `true` 时给每页填 `smartArts`
  - 若同时 `notes: true` 且该页有备注对象，备注上也填 `smartArts`
  - 省略或 `false` 则只有 `smartArtCount`，没有名单
  - 不把图示画成图，不读 Fallback 位图
  - 不依赖 `scope.slide` / `pictures` / `charts`
- **`scope.notes`**（否）
  - 为 `true` 时给每页填 `notes`
  - 有备注页则为对象，没有则为 `null`
  - 省略或 `false` 则只有 `hasNotes`，不倒出备注形状
  - 不依赖 `scope.slide`
- **`scope.comments`**（否）
  - 为 `true` 时给每页填 `comments`
  - 没有现代批注时为空数组
  - 省略或 `false` 则只有 `commentCount`，没有名单
  - 不依赖 `scope.slide`
  - 不读 ECMA-376 §19.4 旧式 `commentN.xml`
  - 若同时 `pictures: true`，每条批注上填 `pictures`
- **`scope.hyperlinks`**（否）
  - 为 `true` 时给每页填 `hyperlinks`
  - 没有链时为空数组
  - 省略或 `false` 则只有 `hyperlinkCount`，没有名单
  - 不依赖 `scope.slide`
  - 同时收形状级、run 级、悬停，以及图 / 表 / 图表 / SmartArt / 单元格上的链
  - 若同时 `notes: true` 且该页有备注对象，备注上也填 `hyperlinks`
  - 若同时 `masters: true`，母版 / 版式上也填
  - xlsx / docx 的 `scope.hyperlinks` 见各自响应
- **`scope.fields`**（否）
  - 为 `true` 时给每页填 `fields`
  - 没有域时为空数组
  - 省略或 `false` 则只有 `fieldCount`，没有名单
  - 不依赖 `scope.slide`
  - 若同时 `notes: true` 且该页有备注对象，备注上也填 `fields`
  - 若同时 `masters: true`，母版 / 版式上也填
  - xlsx 的 `scope` 里出现 `fields` 报 `InvalidField`
  - docx 见 [json-contract-docx.md](json-contract-docx.md#inspect)
- **`scope.tables`**（否）
  - 为 `true` 时给每页填 `tables`
  - 省略或 `false` 则只有 `tableCount`，没有名单
  - 不依赖 `scope.slide`
  - 备注里的表不进幻灯片 `tables`，只在 `scope.notes` 时出现在 `notes` 上
- **`scope.oleObjects`**（否）
  - 为 `true` 时给每页填 `oleObjects`
  - 省略或 `false` 则只有 `oleObjectCount`，没有名单
  - 不依赖 `scope.slide`
  - 不把嵌入字节或预览图放进树
  - 若同时 `notes: true` 且该页有备注对象，备注上也填 `oleObjects`
  - 若同时 `masters: true`，母版 / 版式上也填
  - 备注里的 OLE 不进幻灯片 `oleObjects`
  - xlsx 认 `scope.oleObjects`，见 [xlsx 分册](json-contract-xlsx.md#scope)
  - docx 见 [json-contract-docx.md](json-contract-docx.md)
- **`scope.model3ds`**（否）
  - 为 `true` 时给每页填 `model3ds`
  - 省略或 `false` 则只有 `model3dCount`，没有名单
  - 不依赖 `scope.slide`
  - 不把 GLB 或海报放进树
  - 备注里的 3D 不进幻灯片 `model3ds`，只在 `scope.notes` 时出现在 `notes` 上
  - xlsx / docx 见各自分册
- **`scope.zooms`**（否）
  - 为 `true` 时给每页填 `zooms`
  - 省略或 `false` 则只有 `zoomCount`，没有名单
  - 不依赖 `scope.slide`
  - 不把预览图放进树
  - 若同时 `notes: true` 且该页有备注对象，备注上也填 `zooms`
  - 若同时 `masters: true`，母版 / 版式上也填
  - 备注里的 Zoom 不进幻灯片 `zooms`
  - xlsx / docx 的 `scope` 里出现 `zooms` 报 `InvalidField`
- **`scope.connectors`**（否）
  - 为 `true` 时给每页填 `connectors`
  - 省略或 `false` 则只有 `connectorCount`，没有名单
  - 不依赖 `scope.slide`
  - 若同时 `notes: true` 且该页有备注对象，备注上也填 `connectors`
  - 若同时 `masters: true`，母版 / 版式上也填
  - 备注里的连接线不进幻灯片 `connectors`
  - docx 也认 `scope.connectors`，见 [docx 分册](json-contract-docx.md#scope)
  - xlsx 认 `scope.connectors`，见 [xlsx 分册](json-contract-xlsx.md#scope)
- **`scope.timing`**（否）
  - 为 `true` 时给每页填 `timing`（动画效果摘要）
  - 省略或 `false` 则只有 `animCount`，没有名单
  - 不依赖 `scope.slide`
  - 不返回完整时间轴树
  - 写入见 `insertAnim` / `setAnim` / `deleteAnim` / `moveAnim`
  - `scope.notes` 同开时备注对象带 `timing`
  - `scope.masters` 同开时母版 / 版式带 `timing`
  - `scope.notesMaster` 同开时备注母版带 `timing`
  - `scope.handout` 同开时讲义母版带 `timing`
- **`scope.masters`**（否）
  - 为 `true` 时根上填 `masters`（母版、版式与占位符）
  - 省略或 `false` 则只有 `masterCount`，没有名单
  - 不依赖 `scope.slide`
  - 不算母版排版引擎
  - xlsx / docx 的 `scope` 里出现 `masters` 报 `InvalidField`
- **`scope.notesMaster`**（否）
  - 为 `true` 时根上填 `notesMaster`
  - 有备注母版关系则为对象，没有则为 `null`
  - 省略或 `false` 则不挂该字段
  - 不依赖 `scope.slide`
  - 不算备注排版引擎
  - xlsx / docx 的 `scope` 里出现 `notesMaster` 报 `InvalidField`
- **`scope.handout`**（否）
  - 为 `true` 时根上填 `handout`
  - 有讲义母版则为对象，没有则为 `null`
  - 省略或 `false` 则只有 `hasHandout`，不倒出槽位文字
  - 不依赖 `scope.slide`
  - 不算讲义打印布局
  - xlsx / docx 的 `scope` 里出现 `handout` 报 `InvalidField`
- **`scope.sections`**（否）
  - 为 `true` 时根上填 `sections`
  - 省略或 `false` 则只有 `sectionCount`，没有名单
  - 不依赖 `scope.slide`
  - xlsx / docx 的 `scope` 里出现 `sections` 报 `InvalidField`
  - docx 的节始终在树的 `sections` 上，不走这个键
- **`scope.customShows`**（否）
  - 为 `true` 时根上填 `customShows`
  - 省略或 `false` 则只有 `customShowCount`，没有名单
  - 不依赖 `scope.slide`
  - xlsx / docx 的 `scope` 里出现 `customShows` 报 `InvalidField`

只要想看形状，就必须给 `slide`。不默认把每一页的文本框都倒出来。只给 `maxShapes`、不给 `slide` 报 `ScopeRequired`。
只给 `pictures: true`、`charts: true`、`smartArt: true`、`tables: true`、`oleObjects: true`、`zooms: true`、`connectors: true`、
`notes: true`、`comments: true`、`hyperlinks: true`、`fields: true`、`timing: true`、`masters: true`、`notesMaster: true`、`handout: true`、
`sections: true` 或 `customShows: true`、不给 `slide` 合法，各页都带对应名单、都不带幻灯片 `shapes`。
根上 `masters` 只看 `scope.masters`。根上 `notesMaster` 只看 `scope.notesMaster`。根上 `handout` 只看 `scope.handout`。根上 `sections` 只看 `scope.sections`。
根上 `customShows` 只看 `scope.customShows`。`slide` 小于 `1` 报 `InvalidField`。
`slide` 超出 `slideCount` 时各页都不带 `shapes`，不报错；
图名单、图表名单、SmartArt 名单、表名单、嵌入对象名单、Zoom 名单、连接线名单、备注对象、批注名单、超链接名单、域名单与动画摘要仍按各自键决定。


### 响应

```json
{
  "schemaVersion": 1,
  "format": "pptx",
  "id": "pptx:presentation",
  "slideCount": 2,
  "pictureCount": 0,
  "mathCount": 0,
  "videoCount": 0,
  "audioCount": 0,
  "chartCount": 0,
  "smartArtCount": 0,
  "commentCount": 0,
  "hyperlinkCount": 0,
  "fieldCount": 0,
  "tableCount": 0,
  "oleObjectCount": 0,
  "model3dCount": 0,
  "zoomCount": 0,
  "connectorCount": 0,
  "animCount": 0,
  "masterCount": 1,
  "hasHandout": false,
  "sectionCount": 0,
  "slideSize": {
    "cx": 12192000,
    "cy": 6858000,
    "type": null
  },
  "notesSize": {
    "cx": 6858000,
    "cy": 9144000
  },
  "customShowCount": 0,
  "showPr": {
    "type": "present",
    "loop": false,
    "showNarration": true,
    "showAnimation": true,
    "useTimings": true
  },
  "modifyProtection": {
    "hasPassword": false
  },
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
      "layoutId": "master:1/layout:1",
      "hidden": false,
      "transition": null,
      "hasNotes": false,
      "pictureCount": 0,
      "mathCount": 0,
      "videoCount": 0,
      "audioCount": 0,
      "chartCount": 0,
      "smartArtCount": 0,
      "commentCount": 0,
      "hyperlinkCount": 0,
      "fieldCount": 0,
      "tableCount": 0,
      "oleObjectCount": 0,
      "model3dCount": 0,
      "zoomCount": 0,
      "connectorCount": 0,
      "animCount": 0,
      "truncated": false,
      "shapes": [
        {
          "id": "slide:1/shape:2",
          "shapeId": "2",
          "name": "文本框 1",
          "prst": "rect",
          "x": 838200,
          "y": 685800,
          "cx": 10515600,
          "cy": 800100,
          "text": "标题",
          "sz": 1800,
          "color": "000000",
          "bold": null,
          "italic": null,
          "underline": null,
          "font": "Calibri",
          "eastAsiaFont": "宋体",
          "align": null,
          "textAnchor": null,
          "stable": true
        },
        {
          "id": "slide:1/shape:3",
          "shapeId": "3",
          "name": "文本框 2",
          "prst": "rect",
          "x": 838200,
          "y": 1600200,
          "cx": 10515600,
          "cy": 800100,
          "text": "hello",
          "sz": 1800,
          "color": "000000",
          "bold": null,
          "italic": null,
          "underline": null,
          "font": "Calibri",
          "eastAsiaFont": "宋体",
          "align": null,
          "textAnchor": null,
          "stable": true
        }
      ]
    },
    {
      "id": "slide:2",
      "index": 2,
      "name": null,
      "layoutId": "master:1/layout:1",
      "hidden": false,
      "transition": null,
      "hasNotes": false,
      "pictureCount": 0,
      "mathCount": 0,
      "videoCount": 0,
      "audioCount": 0,
      "chartCount": 0,
      "smartArtCount": 0,
      "commentCount": 0,
      "hyperlinkCount": 0,
      "fieldCount": 0,
      "tableCount": 0,
      "oleObjectCount": 0,
      "model3dCount": 0,
      "zoomCount": 0,
      "connectorCount": 0,
      "animCount": 0
    }
  ]
}
```

规则：

- 读 Presentation 部件、各页幻灯片及其 `.rels`（用 `.rels` 判断 `hasNotes`，用现代 comments 关系判断批注）。
  `scope.notes` 为 `true` 且该页有 notesSlide 关系时再读 `/ppt/notesSlides/…`。
  `scope.comments` 为 `true` 且该页有现代 comments 关系时再读该 comments 部件与 `/ppt/authors.xml`。
  始终读 `p:sldMasterIdLst` 以填 `masterCount` 与各页 `layoutId`。
  始终看 presentation `.rels` 是否有 Type 以 `/handoutMaster` 结尾的关系，以填 `hasHandout`。
  始终看 Presentation 的 `p14:sectionLst` 以填 `sectionCount`。始终读 `p:sldSz` 以填 `slideSize`。始终读 `p:notesSz` 以填 `notesSize`。
  始终看 `p:custShowLst` 以填 `customShowCount`。始终读 `/ppt/presProps.xml` 的 `p:showPr` 以填 `showPr`。
  始终看 Presentation 的 `p:modifyVerifier` 以填 `modifyProtection`。
  `scope.masters` 为 `true` 时再读各 slideMaster / slideLayout 部件，以及各母版的 theme 部件。
  `scope.notesMaster` 为 `true` 且有备注母版关系时再读该 notesMaster 部件。
  `scope.handout` 为 `true` 且有讲义母版关系时再读该 handoutMaster 部件。`scope.sections` 为 `true` 时再展开根上 `sections`。
  `scope.customShows` 为 `true` 时再展开根上 `customShows`。备注里的表不进幻灯片 `tables`，只在 `scope.notes` 时出现在 `notes` 上。
  母版 / 版式上的图、图表、SmartArt、表、媒体、OLE、Zoom、连接线、公式、3D、超链接、域只在 `scope.masters` 与该对象自己的 scope 同开时出现在该母版或版式对象上，不并进根计数。
- `slideCount` 是 `p:sldIdLst` 里 `p:sldId` 的个数，始终出现。
- `pictureCount` 在演示文稿根上始终出现，等于各页幻灯片被收入的图之和，不含备注图，也不含视频 / 音频 `p:pic`。每页始终有自己的 `pictureCount`，同样只计该页幻灯片。
- `mathCount` 在演示文稿根上始终出现，等于各页幻灯片被收入的公式框之和，不含备注。每页始终有自己的 `mathCount`。没有则为 `0`。
  根上不挂 `maths`，除非给了 `scope.maths`。不把 OMML 原文或 `expr` 放进树。
- `videoCount` 在演示文稿根上始终出现，等于各页幻灯片被收入的视频之和。每页始终有自己的 `videoCount`。没有则为 `0`。根上不挂 `videos`，除非给了 `scope.media`。
- `audioCount` 在演示文稿根上始终出现，等于各页幻灯片被收入的音频之和。每页始终有自己的 `audioCount`。没有则为 `0`。根上不挂 `audios`，除非给了 `scope.media`。
- `chartCount` 在演示文稿根上始终出现，等于各页幻灯片被收入的图表之和，不含备注图表。每页始终有自己的 `chartCount`，同样只计该页幻灯片。
- `smartArtCount` 在演示文稿根上始终出现，等于各页幻灯片被收入的 SmartArt 之和，不含备注图示。每页始终有自己的 `smartArtCount`，同样只计该页幻灯片。
- `commentCount` 在演示文稿根上始终出现，等于各页 `p188:cm` 之和，不含 `p188:reply`。每页始终有自己的 `commentCount`，只计该页。旧式 §19.4 批注不计。
- `hyperlinkCount` 在演示文稿根上始终出现，等于各页被收入的超链接之和（含悬停，含图 / 表 / 图表 / SmartArt / 单元格）。
  每页始终有自己的 `hyperlinkCount`，只计该页幻灯片。没有链则为 `0`。
  不计备注或母版上的链。备注链只在 `scope.notes` 与 `scope.hyperlinks` 同开时出现在 `notes` 上。母版 / 版式链只在 `scope.masters` 与 `scope.hyperlinks` 同开时出现。
- `fieldCount` 在演示文稿根上始终出现，等于各页被收入的 `a:fld` 之和。每页始终有自己的 `fieldCount`，只计该页幻灯片文本形状里的域。没有域则为 `0`。
  根上不挂 `fields`，除非给了 `scope.fields`。不计备注、母版、版式、表或讲义上的域。
  备注域只在 `scope.notes` 与 `scope.fields` 同开时出现在 `notes` 上。母版 / 版式域只在 `scope.masters` 与 `scope.fields` 同开时出现。
- `tableCount` 在演示文稿根上始终出现，等于各页被收入的 DrawingML 表之和。每页始终有自己的 `tableCount`。没有表则为 `0`。根上不挂 `tables`，除非给了 `scope.tables`。
- `oleObjectCount` 在演示文稿根上始终出现，等于各页被收入的嵌入对象之和。每页始终有自己的 `oleObjectCount`。没有则为 `0`。
  根上不挂 `oleObjects`，除非给了 `scope.oleObjects`。不把嵌入字节或预览图放进树。
- `model3dCount` 在演示文稿根上始终出现，等于各页被收入的 3D 模型之和，不含备注。每页始终有自己的 `model3dCount`。没有则为 `0`。
  根上不挂 `model3ds`，除非给了 `scope.model3ds`。不把 GLB 或海报放进树。
- `zoomCount` 在演示文稿根上始终出现，等于各页被收入的 Zoom 之和。每页始终有自己的 `zoomCount`。没有则为 `0`。根上不挂 `zooms`，除非给了 `scope.zooms`。不把预览图放进树。
- `connectorCount` 在演示文稿根上始终出现，等于各页被收入的 `p:cxnSp` 之和。每页始终有自己的 `connectorCount`。没有则为 `0`。
  根上不挂 `connectors`，除非给了 `scope.connectors`。
- `hasNotes` 在每页始终出现：该页 `.rels` 有 Type 以 `/notesSlide` 结尾的关系则为 `true`，否则为 `false`。
- `slides` 始终列出全部页（含隐藏页）。未点名的页不带 `shapes`、`truncated`。
- `name` 取 `p:cSld/@name` 的原文。没有该属性则为 `null`。
- `layoutId` 在每页始终出现：该页 `.rels` 里 Type 以 `/slideLayout` 结尾的关系，对到某母版 `p:sldLayoutIdLst` 里的那一份，则为 `master:{n}/layout:{k}`。
  找不到该关系、或对不上任何已编母版版式，则为 `null`。不因换页关系改母版 / 版式序号。
- `hidden`：`p:sld/@show` 为 `0` 或 `false` 时为 `true`，其余为 `false`。
- `transition` 在每页始终出现。没有 `<p:transition>`（含包在 `mc:AlternateContent` 里的）则为 `null`。
  有则为对象，字段顺序：`type`、`dur`、`advClick`、`advTm`、`sound`，
  有 `dir` 再跟 `dir`，有 `orient` 再跟 `orient`，有 `spokes` 再跟 `spokes`，`invX` / `invY` 为真再跟。
  `type` 取效果子元素的本地名（`sndAc` / `extLst` 不算）。
  效果本地名为 `prstTrans` 时，`type` 改取 `prst` 属性：值为 `fallOver` / `drape` / `curtains` / `wind` / `prestige` / `fracture`
   / `crush` / `peelOff` / `pageCurlSingle` / `pageCurlDouble` / `airplane` / `origami` 则按该名返回，其余 `prst` 仍为 `other`。
  ECMA-376 的 `blinds` / `checker` / `circle` / `dissolve` / `comb` / `cover` / `cut` / `diamond` / `fade` / `newsflash`
   / `plus` / `pull` / `push` / `random` / `randomBar` / `split` / `strips` / `wedge` / `wheel` / `wipe` / `zoom`，
  以及 MS-PPTX 的 `vortex` / `switch` / `flip` / `ripple` / `honeycomb` / `prism` / `doors` / `window` / `ferris`
   / `gallery` / `conveyor` / `pan` / `glitter` / `warp` / `flythrough` / `flash` / `shred` / `reveal` / `wheelReverse` / `morph`，按原名返回。
  不在这份名单、也不是上述 prst 名的效果为 `other`。`mc:AlternateContent` 只走第一个 Choice。
  `dur` 优先取 `p14:dur`（毫秒，无单位整数）；没有则按 `spd` 折算：`slow` 为 `1000`，`med` 为 `750`，`fast` 或缺 `spd` 为 `500`。
  没有效果子元素但仍有 `<p:transition>` 时 `type` 为 `none`。`advClick`：属性为 `0` / `false` 则为 `false`，缺属性或其余值为 `true`。
  `advTm`：有属性且能解析成正整数则取其毫秒，否则为 `null`。
  `sound` 取 `sndAc/stSnd/snd/@name`；没有则为 `null`。不把字节塞进树。只有 `endSnd`、没有 `stSnd` 时仍为 `null`。
  `dir` 取效果子元素的 `dir` 原文；没有该属性则不出现该字段。
  `orient` 取效果子元素的 `orient` 原文；没有该属性则不出现该字段。
  `spokes` 取效果子元素的 `spokes`；能解析成非负整数才出现，否则不出现该字段。
  `invX` / `invY` 只在对应属性为 `1` / `true` 时出现且值为 `true`。
- 省略 `scope` 或只给空对象时，没有 `shapes`、`truncated`，
  也没有 `pictures` / `maths` / `videos` / `audios` / `charts` / `smartArts` / `tables` / `oleObjects` / `model3ds` / `zooms` / `connectors` / `timing` 名单，
  也没有 `notes`，也没有 `comments`，也没有 `hyperlinks`，也没有根上 `masters`，也没有根上 `notesMaster`，也没有根上 `handout`，也没有根上 `sections`。
- 给了 `scope.slide` 时
- 只给那一页填 `shapes`。该页带 `p:txBody` 的 `p:sp` 超过 `maxShapes` 则 `truncated` 为 `true`
- 按该页文档顺序截断。未截断则为 `false`。备注形状不算进 `maxShapes`。
- `scope.slide` 大于 `slideCount` 时，各页都不带 `shapes`，不报错。
- `scope.pictures` 为 `true` 时，每页带 `pictures`。没有图时为空数组。不依赖 `scope.slide`。
  每张字段顺序：`id`、`cNvPrId`、`name`、`x`、`y`、`cx`、`cy`、`stable`。
  有 `asvg:svgBlip` 时再跟 `contentType`，值为 `image/svg+xml`。
  没有 `svgBlip` 时，按 image 关系 Target 扩展名：
  `.emf` 为 `image/x-emf`，`.wmf` 为 `image/x-wmf`，`.tif` / `.tiff` 为 `image/tiff`。
  PNG / JPEG / GIF / BMP 没有该字段。不读媒体字节，不把原图放进树。
  `cNvPrId` 是 `p:cNvPr/@id` 原文，没有则为 `null`。
  `name` 取 `p:cNvPr/@name`，没有则为 `""`。`x` / `y` 取 `p:spPr/a:xfrm/a:off` 的 EMU；`cx` / `cy` 取 `a:ext`。缺对应属性则为 `null`。不读媒体字节。
  幻灯片图 id 为 `slide:{n}/pic:…`。
- `scope.maths` 为 `true` 时，每页带 `maths`。没有公式框时为空数组。不依赖 `scope.slide`。
  每条字段顺序：`id`、`cNvPrId`、`name`、`x`、`y`、`cx`、`cy`、`text`、`stable`。`id` 为 `slide:{n}/math:…`。
  `cNvPrId` 是 Choice 里 `p:cNvPr/@id` 原文，没有则为 `null`。`name` 取 `cNvPr/@name`，没有则为 `""`。
  `x` / `y` / `cx` / `cy` 取 Choice 里该 `p:sp` 的 `a:xfrm`。缺对应属性则为 `null`。
  `text` 为该 `oMath` 内 `m:t` 拼接；忽略 `a:fld` 外壳。不把 OMML 原文、不把 `expr` 放进树。
  Fallback 文本框不进 `shapes`。xlsx / docx 见各自分册。
- `scope.media` 为 `true` 时，每页带 `videos` 与 `audios`（先 `videos` 后 `audios`）。没有则为空数组。不依赖 `scope.slide`。
  每条字段顺序：`id`、`cNvPrId`、`name`、`x`、`y`、`cx`、`cy`、`embedded`、`fileName`、`contentType`、`stable`、
  `st`、`end`、`bookmarks`、`tracks`。
  有字幕轨时再跟 `trackDisplayLoc`。
  `cNvPrId` / `name` / `x` / `y` / `cx` / `cy` / `stable` 规则与图相同。
  `embedded`：媒体关系 `TargetMode` 不是 `External`、且目标部件在包内为 `true`，否则为 `false`。
  优先取 `p14:media/@r:embed`（MS-PPTX §2.3.1.18）；
  没有则取 `a:videoFile` / `a:audioFile` / `a:wavAudioFile` 的 `r:link` 或 `r:embed`。
  `fileName` 取该关系 `Target` 的文件名（最后一段），外链取路径最后一段；对不上关系则为 `null`。
  `contentType` 取 `[Content_Types].xml` 对该部件的 Default 或 Override；外链或没有则为 `null`。不把字节放进树。
  `st` / `end` 取 `p14:media/p14:trim` 的 `st` / `end`（MS-PPTX `CT_MediaTrim`）。
  这是从片头 / 片尾裁掉的毫秒，不是播放窗口的起止时刻。没有 `trim` 或没有该属性则为 `0`。
  值为无单位数字；整数在 JSON 里写成整数，带小数则保留。
  `bookmarks` 取 `p14:bmkLst/p14:bmk` 文档顺序。没有则为 `[]`。
  每条字段顺序：`name`、`time`。`name` 取 `@name`，没有则为 `""`。
  `time` 取 `@time`，相对媒体文件开头的毫秒，规则与 `st` 相同；缺属性则为 `0`。
  `tracks` 取 `p14:media` 的 `extLst` 里 uri `{3AFAAA56-56D3-431D-BCD4-E75A35582382}` 的 `tracksInfo/trackLst/track`
  （MS-PPTX §2.2.4.1、§2.13；命名空间 `http://schemas.microsoft.com/office/powerpoint/2017/3/main`）。
  没有则为 `[]`。有字幕轨时再跟 `trackDisplayLoc`，取 `tracksInfo/@displayLoc`；缺则为 `"media"`。
  每条字幕字段顺序：`id`、`label`、`lang`、`fileName`、`embedded`。
  `id` 取 `@id` 原文，缺则为 `null`。`label` 取 `@label`，缺则为 `""`。
  `lang` 取 `@lang`，缺则为 `null`。
  `fileName` / `embedded` 取该 `r:embed` 或 `r:link`（`link` 优先）对上的关系，规则与媒体 `fileName` / `embedded` 相同。
  不把 VTT 字节放进树。不把 `p14:fade` 放进树。
  xlsx / docx 的 `scope` 里出现 `media` 报 `InvalidField`。
- `scope.charts` 为 `true` 时，每页带 `charts`。没有图表时为空数组。不依赖 `scope.slide`。
  每张字段顺序：`id`、`cNvPrId`、`name`、`x`、`y`、`cx`、`cy`、`type`，`type` 为 `ofPie` 时再跟 `ofPieType`，
  `type` 为 `stock` 时再跟 `stockKind`，`type` 为 `surface` 时再跟 `surfaceKind`，
  `type` 为 `chartex` 时再跟 `layoutId`，然后 `title`、`catAxisTitle`、`valAxisTitle`、`series`、`legend`、`dataLabels`、`flags`、`stable`。
  `cNvPrId` / `name` / `stable` 规则与图相同。`x` / `y` / `cx` / `cy` 取 graphicFrame 的 `p:xfrm`。
  `type` / `ofPieType` / `stockKind` / `surfaceKind` / `layoutId` / `title` / 轴标题 / `series` / `legend` / `dataLabels` / `flags` 规则与 docx 图表相同；
  系列值优先读 embedding 单元格，读不到再退回 cache。
- `scope.smartArt` 为 `true` 时，每页带 `smartArts`。没有图示时为空数组。不依赖 `scope.slide`。
  每张字段顺序：`id`、`cNvPrId`、`name`、`x`、`y`、`cx`、`cy`、`layout`、`colors`、`style`、`points`、`stable`。
  `cNvPrId` / `name` / `stable` 规则与图相同。`x` / `y` / `cx` / `cy` 取 graphicFrame 的 `p:xfrm`。
  `layout` / `colors` / `style` / `points` 规则与 docx SmartArt 相同。pptx 没有 `kind` / `wrap`。
- `scope.tables` 为 `true` 时，每页带 `tables`。没有表时为空数组。不依赖 `scope.slide`。
  每张字段顺序：`id`、`cNvPrId`、`name`、`x`、`y`、`cx`、`cy`、`rowCount`、`colCount`、`colWidths`、`rows`、`stable`。
  `cNvPrId` / `name` / `stable` 规则与图相同。`x` / `y` / `cx` / `cy` 取 graphicFrame 的 `p:xfrm`。`rowCount` 是 `a:tr` 个数。
  `colCount` 见上文表 id 那节。`colWidths` 取 `a:tblGrid` 各 `a:gridCol/@w`（EMU 整数）。没有网格则为 `[]`。缺 `@w` 或解析失败当作 `0`。
  `rows` 按 `a:tr` 顺序。每行字段顺序：`id`、`index`、`height`、`cells`。`height` 取 `a:tr/@h`，缺则为 `null`。没有 `rule`。
  `cells` 按该行 `a:tc` 文档顺序，不按空网格补格。
  每格字段顺序：`id`、`row`、`col`、`gridSpan`、`rowSpan`、`hMerge`、`vMerge`、`text`，
  然后 `sz`、`color`、`textFill`、`textOutline`、`bold`、`italic`、`underline`、`font`、`eastAsiaFont`、`align`、`textAnchor`、`spcBef`、`spcAft`、`lnSpc`、`runs`、`fill`、`borders`、`cellMar`、`vert`、`anchorCtr`、`horzOverflow`、`cell3D`、`headers`。
  `row` / `col` 与 id 里的数字相同。
  `gridSpan` / `rowSpan` 为整数，缺属性则为 `1`。`hMerge` / `vMerge` 为布尔（属性为 `1` / `true` / `on` 则为 `true`）。
  `text` 取该格 `a:txBody` 可见纯文本，规则与形状 `text` 相同。没有 `txBody` 则为 `""`。不把单元格编成段落 id。
  `fill` 取 `a:tcPr` 的 `EG_FillProperties`，为对象或 `null`。`a:noFill` 或没有填充子元则为 `null`。
  对象始终先有 `type`：`solid` / `grad` / `patt` / `blip` / `other`。
  `solid`：再跟 `color`（`a:srgbClr/@val`，大写六位）。
  `grad`：再跟 `ang`、`stops`。可读条件与 [json-contract.md](json-contract.md#setshapeformat) 的字渐变相同（仅线性 `a:lin`、停点只有无修饰 `srgbClr`，至少两项）。
  `patt`：再跟 `prst`、`fg`、`bg`。`prst` 取 `@prst` 原文。`fg` / `bg` 取 `fgClr` / `bgClr` 的 `srgbClr`（大写六位）。
  `blip`：再跟 `rId`、`contentType`。`rId` 取 `a:blip/@r:embed`。`contentType` 取该关系目标在 `[Content_Types].xml` 的 Default 或 Override，都没有则为 `null`。不把图片字节放进树。
  只有 `schemeClr`、读不了的渐变停点、缺 `r:embed` 的 `blipFill`、或缺 `prst` / `fg` / `bg` 的图案，则为 `{ "type": "other" }`，不报错。
  `borders` 对象字段顺序：`left`、`right`、`top`、`bottom`、`tl2br`、`bl2tr`。对应 `lnL` / `lnR` / `lnT` / `lnB` / `lnTlToBr` / `lnBlToTr`。缺该边或该边是 `a:noFill` 则为 `null`。
  每边字段顺序：`color`、`width`、`dash`。`color` 取该线 `a:solidFill/a:srgbClr`（大写六位，没有则为 `null`）。`width` 取 `@w`（EMU 整数，缺或解析失败则为 `null`）。`dash` 取 `a:prstDash/@val`，缺则为 `null`。
  `cellMar`：四个 `marT` / `marL` / `marB` / `marR` 都缺则为 `null`。有则对象字段顺序：`top`、`left`、`bottom`、`right`，各取对应属性（EMU 整数，缺该属性则为 `null`）。
  `vert` 取 `a:tcPr/@vert`（`ST_TextVerticalType` 原文，§20.1.10.83），缺则为 `null`。
  `anchorCtr` 取 `@anchorCtr` 布尔（`1` / `true` / `on` 为 `true`），缺则为 `null`。
  `horzOverflow` 取 `@horzOverflow`（`overflow` / `clip`），缺或不是这两值则为 `null`。
  `cell3D`：没有 `a:cell3D` 则为 `null`。有则对象字段顺序：`prstMaterial`、`bevel`。
  `prstMaterial` 取 `@prstMaterial`（`ST_PresetMaterialType` 原文），缺则为 `null`。
  `bevel` 取 `a:bevel`：对象字段顺序 `prst`、`w`、`h`。`prst` 取 `@prst` 原文，缺则为 `null`。`w` / `h` 为 EMU 整数，缺则为 `null`。没有 `a:bevel` 则为 `null`。
  不把 `lightRig` 放进树。
  `headers` 取 `a:headers` 下各 `a:header` 的文本，按文档顺序。没有该子元则为 `[]`。
  不把 `tcPr/@anchor` 放进树；垂直锚点仍是 C9 的 `textAnchor`（`txBody/bodyPr/@anchor`）。
- `scope.oleObjects` 为 `true` 时，每页带 `oleObjects`。没有嵌入对象时为空数组。不依赖 `scope.slide`。
  每条字段顺序：`id`、`progId`、`name`、`kind`、`contentType`、`showAsIcon`、`x`、`y`、`cx`、`cy`、`stable`。`id` 为 `slide:{n}/ole:…`。
  `progId` 取 `p:oleObj/@progId`，缺则为 `null`。`name` 取 `p:oleObj/@name`，缺则为 `""`。
  `kind`：关系 Type 后缀为 `/package` 时为 `package`，否则为 `embed`。
  `contentType` 取 embeddings 部件在 `[Content_Types].xml` 里的 Default 或 Override，都没有则为 `null`。
  `showAsIcon`：`p:oleObj/@showAsIcon` 为 `1` / `true` / `True` 则为 `true`，否则为 `false`。
  xlsx 认 `scope.oleObjects`，见 [xlsx 分册](json-contract-xlsx.md#scope)。
- `scope.model3ds` 为 `true` 时，每页带 `model3ds`。没有 3D 模型时为空数组。不依赖 `scope.slide`。
  每条字段顺序：`id`、`cNvPrId`、`name`、`x`、`y`、`cx`、`cy`、`contentType`、`stable`。`id` 为 `slide:{n}/model3d:…`。
  `cNvPrId` / `name` / `x` / `y` / `cx` / `cy` / `stable` 规则与图相同。`contentType` 恒为 `model/gltf.binary`。
  几何取 graphicFrame 的 `p:xfrm`。不把 GLB 或海报放进树。xlsx / docx 见各自分册。
- `scope.zooms` 为 `true` 时，每页带 `zooms`。没有 Zoom 时为空数组。不依赖 `scope.slide`。
  每条字段顺序：`id`、`type`，然后 slide / section 有 `target`，summary 有 `targets`，然后 `x`、`y`、`cx`、`cy`、`stable`。
  `id` 为 `slide:{n}/zoom:…`。`type` 为 `slide`（`sldZm`）、`section`（`sectionZm`）或 `summary`（`summaryZm`）。
  `target`：幻灯片缩放取 `sldZmObj/@sldId` 对到 `p:sldIdLst` 的 `slide:{n}`；节缩放取 `sectionZmObj/@sectionId` 对到 `section:{n}`；对不上则为 `null`。
  `targets` 是摘要缩放各 `summaryZmObj/@sectionId` 对到的 `section:{n}` 数组，对不上的 GUID 跳过。
  `x` / `y` / `cx` / `cy` 取 graphicFrame 的 `p:xfrm`；没有 graphicFrame 则取 `spPr/a:xfrm` 或 Fallback `p:pic` 的 `spPr/a:xfrm`。
  不把预览图放进树。xlsx / docx 的 `scope` 里出现 `zooms` 报 `InvalidField`。
- `scope.connectors` 为 `true` 时，每页带 `connectors`。没有连接线时为空数组。不依赖 `scope.slide`。
  每条字段顺序：`id`、`cNvPrId`、`name`、`prst`，有 `a:stCxn` 时再跟 `start`，有 `a:endCxn` 时再跟 `end`，
  然后 `x`、`y`、`cx`、`cy`、`stable`。`cNvPrId` / `name` / `stable` 规则与图相同。
  `prst` 取 `a:prstGeom/@prst`，没有则为 `null`。`x` / `y` / `cx` / `cy` 取 `p:spPr/a:xfrm`。组合里的 `p:cxnSp` 要展开。
  `start` / `end` 字段顺序：`id`、`site`。`id` 是同宿主上 `cNvPr/@id` 对得上的形状或连接线 inspect id。
  对不上时仍写成 `{host}/shape:{该 cNvPr id}`。`site` 为 `@idx` 加 1。没有对应元素则不要出现该键。
  docx 也认 `scope.connectors`，见 [docx 分册](json-contract-docx.md#scope)。
  xlsx 认 `scope.connectors`，见 [xlsx 分册](json-contract-xlsx.md#scope)。
  docx / xlsx 连接线不带 `start` / `end`。
- `animCount` 在演示文稿根上始终出现，等于各页效果摘要条数之和。每页始终有自己的 `animCount`。没有效果则为 `0`。根上不挂 `timing` 数组。名单只出现在各页上。本期不截断，一页全部列出。
- `masterCount` 在演示文稿根上始终出现，等于 `p:sldMasterIdLst` 里 `p:sldMasterId` 的个数。没有该表则为 `0`。根上不挂 `masters`，除非给了 `scope.masters`。不算讲义母版。本期不截断。
- `hasHandout` 在演示文稿根上始终出现：presentation `.rels` 有 Type 以 `/handoutMaster` 结尾的关系则为 `true`，否则为 `false`。根上不挂 `handout`，除非给了 `scope.handout`。
- `sectionCount` 在演示文稿根上始终出现，等于上述 `p14:sectionLst` 里 `p14:section` 的个数。没有该扩展则为 `0`。根上不挂 `sections`，除非给了 `scope.sections`。
  页顺序仍以 `p:sldIdLst` 为准；节只是分组元数据。
- `slideSize` 在演示文稿根上始终出现。有 `p:sldSz` 则为对象，字段顺序：`cx`、`cy`、`type`。`cx` / `cy` 取属性的 EMU 整数；解析失败则为 `null`。
  `type` 取 `@type` 原文，没有该属性则为 `null`。没有 `p:sldSz` 则 `slideSize` 为 `null`。xlsx / docx 根上没有此字段。
- `notesSize` 在演示文稿根上始终出现，紧跟 `slideSize`。有 `p:notesSz` 则为对象，字段顺序：`cx`、`cy`。`cx` / `cy` 取属性的 EMU 整数；解析失败则为 `null`。
  没有 `p:notesSz` 则 `notesSize` 为 `null`。不带 `type`。xlsx / docx 根上没有此字段。
- `customShowCount` 在演示文稿根上始终出现，等于 `p:custShowLst` 里 `p:custShow` 的个数。没有该表则为 `0`。
  根上不挂 `customShows`，除非给了 `scope.customShows`。不算放映引擎。xlsx / docx 根上没有此字段。
- `showPr` 在演示文稿根上始终出现。来源是 `/ppt/presProps.xml` 的 `p:showPr`（ECMA-376 §19.2.1.30）。
  没有 presProps 部件或没有该元素时，按 PowerPoint 16.0 对 create 产物的缺省：`type` 为 `present`，`loop` 为 `false`，`showNarration` 为 `true`，
  `showAnimation` 为 `true`，`useTimings` 为 `true`。
  有该元素时：`type` 取 `EG_ShowType` 子元素，`present` / 缺子元素为 `present`，`browse` 为 `browse`，`kiosk` 为 `kiosk`；
  `loop` / `showNarration` / `showAnimation` / `useTimings` 按对应属性，缺省分别为 `false` / `false` / `true` / `true`（schema 缺省）。
  不把 `sldAll` / `sldRg` / `custShow`、`penClr`、`extLst`、`kiosk/@restart` 或 `browse/@showScrollbar` 放进树。xlsx / docx 根上没有此字段。
- `modifyProtection` 在演示文稿根上始终出现。来源是 Presentation 的 `p:modifyVerifier`（§19.2.1.19）。
  字段只有 `hasPassword`：该元素带非空 `algorithmName` / `hashValue` / `saltValue` / `spinCount` / `spinValue` / `hash` / `salt` / `password` /
  `hashData` / `saltData` / `cryptProviderType` / `cryptAlgorithmClass` / `cryptAlgorithmType` / `cryptAlgorithmSid` / `cryptSpinCount` 时为 `true`，
  否则为 `false`。
  树上不出现哈希。xlsx / docx 根上不要带此对象。docx 仍用 `protection`，xlsx 用 `sheetProtection` / `workbookProtection`。
  `setProtection` 带 `password` 时按 [json-contract.md](json-contract.md#setprotection) 写 PowerPoint 16.0 的 `crypt*` SHA-512，不把口令或哈希写进树。
- `scope.handout` 为 `true` 时，根上带 `handout`。没有讲义母版关系则为 `null`。有则为对象，字段顺序：`id`、`theme`、`clrMap`、`header`、`footer`、`date`、`sldNum`、`animCount`，
  `scope.timing` 同开时再跟 `timing`，然后 `placeholders`。
  `id` 为 `handout`。
  `theme` 取该 handoutMaster `.rels` 里 Type 以 `/theme` 结尾的部件，规则与 `masters[].theme` 相同。没有该关系或部件不在包内则为 `null`。不和 slideMaster 共用同一份 `masters[].theme`。
  四个槽位分别取 handoutMaster 上第一个 `p:ph/@type` 为 `hdr` / `ftr` / `dt` / `sldNum` 的 `p:sp` 的可见字。
  没有该类型占位符则为 `null`。有占位符但没有 `txBody` 则为 `""`。槽位文字把 `a:fld` 里的 `a:t` 也算进去（与幻灯片形状 `text` 跳过域缓存不同），以便看见日期与页码缓存。
  `placeholders` 收全部带 `p:nvPr/p:ph` 的 `p:sp`（含 `sldImg`，含组合里的），按文档顺序。
  字段顺序与幻灯片母版占位符相同。`id` 为 `handout/shape:{cNvPrId}`；没有 `cNvPr/@id` 则为 `handout/shape:index:{i}`，`{i}` 只在该宿主占位符内从 1 计。
  `animCount` 只计该 handoutMaster 的 `<p:timing>`。讲义 `timing` 的 id 为 `handout/anim:{k}`，`target` 对应该讲义占位符 id。
  `clrMap` 规则与 `masters[].clrMap` 相同，读该 handoutMaster 的 `p:clrMap`。
  xlsx / docx 的 `scope` 里出现 `handout` 报 `InvalidField`。不算讲义打印布局。
- `scope.notesMaster` 为 `true` 时，根上带 `notesMaster`。没有备注母版关系则为 `null`。有则为对象，字段顺序：`id`、`theme`、`clrMap`、`animCount`，
  `scope.timing` 同开时再跟 `timing`，然后 `placeholders`。
  `id` 为 `notesMaster`。
  `theme` 取该 notesMaster `.rels` 里 Type 以 `/theme` 结尾的部件，规则与 `masters[].theme` 相同。没有该关系或部件不在包内则为 `null`。不和 slideMaster 共用同一份 `masters[].theme`。
  `placeholders` 收全部带 `p:ph` 的 `p:sp`（含 `sldImg`），按文档顺序。
  `id` 为 `notesMaster/shape:{cNvPrId}`；没有则为 `notesMaster/shape:index:{i}`。
  `animCount` 只计该 notesMaster 的 `<p:timing>`。备注母版 `timing` 的 id 为 `notesMaster/anim:{k}`。
  `clrMap` 规则与 `masters[].clrMap` 相同，读该 notesMaster 的 `p:clrMap`。
  不收没有 `p:ph` 的装饰形。xlsx / docx 的 `scope` 里出现 `notesMaster` 报 `InvalidField`。
  根上 `animCount` 仍只计幻灯片 XML，不含备注母版与讲义。
- `scope.sections` 为 `true` 时，根上带 `sections`。没有节时为空数组。不依赖 `scope.slide`。按 `p14:section` 文档顺序。
  每条字段顺序：`id`、`guid`、`name`、`slides`。`id` 为 `section:{n}`。`guid` 取 `p14:section/@id` 原文（含花括号），缺则为 `null`。
  `name` 取 `@name` 原文，缺则为 `""`。`slides` 是该节 `p14:sldIdLst/p14:sldId/@id` 对到 `p:sldIdLst` 之后的 `slide:{n}` 数组；对不上任何页的 `ST_SlideId` 跳过，不编进树。
  不认识的扩展（同 `p:ext` 上别的 uri、节上的 `extLst`）原样留，不进树。xlsx / docx 的 `scope` 里出现 `sections` 报 `InvalidField`。不写节的 UI 颜色。
- `scope.customShows` 为 `true` 时，根上带 `customShows`。没有自定义放映时为空数组。不依赖 `scope.slide`。按 `p:custShow` 文档顺序。
  每条字段顺序：`id`、`name`、`slides`。`id` 为 `show:{n}`。`name` 取 `@name` 原文，缺则为 `""`。
  `slides` 是该条 `p:sldLst/p:sld/@r:id` 对到 `p:sldIdLst` 之后的 `slide:{n}` 数组；对不上任何页的 `r:id` 跳过，不编进树。同一页可以出现多次。
  空 `sldLst` 则为 `[]`。`custShow` 上的 `extLst` 原样留，不进树。xlsx / docx 的 `scope` 里出现 `customShows` 报 `InvalidField`。不实现放映引擎。根上 `showPr` 始终出现，不跟本名单走。
- `scope.timing` 为 `true` 时，每页带 `timing`。没有效果时为空数组。不依赖 `scope.slide`。inspect 只读 `<p:timing>`。
  按文档顺序收集 `p:cTn` 上 `nodeType` 为 `clickEffect` / `withEffect` / `afterEffect` 的节点（ECMA-376 `ST_TLTimeNodeType`，§19.7.47）；
  `tmRoot` / `mainSeq` / `clickPar` / `withGroup` / `afterGroup` / `interactiveSeq` 不算一条。
  每条字段顺序：`id`、`target`、`type`、`trigger`、`delay`、`dur`、`repeat`、`sound`、`para`、`clickOn`。
  `id` 为 `slide:{n}/anim:{k}`，`k` 按文档顺序从 1 计；删中间一条后面要重排。
  `trigger`：祖先是 `interactiveSeq` 则为 `clickOn`；否则 `clickEffect` 为 `click`，`withEffect` 为 `with`，`afterEffect` 为 `after`。
  `delay` 取该效果 `cTn` 自身 `stCondLst` 第一条不带 `evt` 的 `cond/@delay`：
  无单位整数为该毫秒；`indefinite` 为 `null`；缺则为 `0`。
  `dur` 取该效果子树第一个 `p:animEffect` 的 `cBhvr/cTn/@dur`；
  没有则取第一个非 `set` 的行为（`anim` / `animRot` / `animScale` / `animMotion` / `animClr`）的 `cBhvr/cTn/@dur`；
  再没有则取第一个 `p:set` 的 `cBhvr/cTn/@dur`。
  无单位整数为该毫秒；`indefinite` 为 `null`；缺则为 `500`。
  `repeat` 取该效果 `cTn/@repeatCount`：缺为 `1`；`indefinite` 为 `null`；无单位整数按千分之一换成次数（`3000` 为 `3`）。
  `sound` 取该效果 `sndAc/stSnd/snd/@name`；没有则为 `null`。不把字节塞进树。
  `para` 取该效果子树第一个 `p:pRg/@st` 加 1；没有 `txEl` 则为 `null`。
  `clickOn`：`trigger` 为 `clickOn` 时，取该 `interactiveSeq` 上 `evt="onClick"` 的 `spTgt/@spid`，对到该宿主对象 id；对不上则为 `null`。其余 trigger 为 `null`。
  `target` 取该效果子树里第一个 `p:spTgt/@spid`（不含嵌套的另一条效果 `cTn`），对到该页对象的 inspect id：带 `p:txBody` 的 `p:sp` 为 `slide:{n}/shape:{id}`，
  `p:cxnSp` 为 `slide:{n}/cxn:{id}`，带 `a:videoFile` 的 `p:pic` 为 `slide:{n}/video:{id}`，
  带 `a:audioFile` / `a:wavAudioFile` 的 `p:pic` 为 `slide:{n}/audio:{id}`，其余 `p:pic` 为 `slide:{n}/pic:{id}`，
  表 graphicFrame 为 `slide:{n}/table:{id}`，图表为 `slide:{n}/chart:{id}`，SmartArt 为 `slide:{n}/smartdiagram:{id}`，OLE 为 `slide:{n}/ole:{id}`。
  对不上任何 `cNvPr/@id`，或对上的不是上述对象，则为 `null`。`type` 若该效果 `presetClass` 与 `presetID` 落在写入白名单，则用对应 JSON 名（见 `insertAnim` 的 `type`）。
  否则优先取该效果子树里第一个 `p:animEffect/@filter`，去掉括号及之后的参数（`blinds(horizontal)` 为 `blinds`）；
  没有则按第一个行为子元素：`animMotion` 为 `motion`，`animRot` 为 `spin`，`animScale` 为 `scale`，`animClr` 为 `color`，`anim` 为 `anim`，`set` 为 `set`；
  再没有则取该 `cTn` 的 `presetClass`（`entr` / `exit` / `emph` / `path` / `verb` / `mediacall`）；仍没有则为 `other`。
  不返回路径点或完整时间轴树。xlsx / docx 的 `scope` 里出现 `timing` 报 `InvalidField`。
  apply 用 `insertAnim` / `setAnim` / `deleteAnim` / `moveAnim` 按模板增删改预设效果。
  不接受把整棵 `<p:timing>` 当 JSON 写入；`setTiming` 报 `UnsupportedOp`。
  该宿主第一条效果 `trigger` 为 `with` 或 `after` 时，外层包装仍写 `delay="indefinite"`，并加 `evt="onBegin"` 指向 `mainSeq` 的 `cTn/@id`（进片自动播）。
  第一条 `click` 不加这条。`clickOn` 不走这条，写在 `interactiveSeq` 里。
- `scope.masters` 为 `true` 时，根上带 `masters`。没有母版时为空数组。不依赖 `scope.slide`。不算母版排版引擎。不把母版装饰形抄进各页。
  每份母版字段顺序：`id`、`index`、`name`、`theme`、`clrMap`、`transition`、`animCount`、`placeholders`、`shapes`、`layouts`。
  `transition` 规则与幻灯片相同，读该 slideMaster 的 `<p:transition>`。`scope.timing` 同开时在 `animCount` 之后、`placeholders` 之前带 `timing`。
  `id` 为 `master:{n}`。
  `animCount` 只计该 slideMaster 的 `<p:timing>`，不含其版式。
  母版不新增永远出现的对象 Count。名单只在 `scope.masters` 与该对象自己的 scope 同开时出现，插在 `shapes` 之后、`layouts` 之前。
  顺序与幻灯片相同：按需 `pictures`、`maths`、`videos`、`audios`、`charts`、`smartArts`、`tables`、`oleObjects`、`model3ds`、`zooms`、`connectors`、`hyperlinks`、`fields`。
  母版 `timing` 的 id 为 `master:{n}/anim:{k}`，`target` 对应该母版形状 id（占位符或装饰形）。
  占位符与装饰形都进名单。
  `name` 取该 slideMaster 的 `p:cSld/@name`，没有则为 `null`。`theme` 取该母版 `.rels` 里 Type 以 `/theme` 结尾的部件。没有该关系或部件不在包内则为 `null`。
  有则为对象，字段顺序：`name`、`colors`、`fonts`、`fmt`、`extraSchemes`。`theme.name` 取 `a:theme/@name`，没有则为 `null`。
  `colors` 字段顺序：`name`、`dk1`、`lt1`、`dk2`、`lt2`、`accent1`、`accent2`、`accent3`、`accent4`、`accent5`、`accent6`、`hlink`、`folHlink`。
  `colors.name` 取 `a:clrScheme/@name`，没有则为 `null`。每个槽位：有 `a:srgbClr/@val` 取其原文；否则有 `a:sysClr/@lastClr` 取该原文；再没有则为 `null`。
  `fonts` 字段顺序：`name`、`majorLatin`、`minorLatin`、`majorEa`、`minorEa`、`majorCs`、`minorCs`、`majorScripts`、`minorScripts`。
  `fonts.name` 取 `a:fontScheme/@name`，没有则为 `null`。
  其余取 `a:majorFont` / `a:minorFont` 下 `a:latin` / `a:ea` / `a:cs` 的 `typeface` 原文，没有该元素则为 `null`。
  `majorScripts` / `minorScripts`、`fmt`、`extraSchemes` 规则与 [docx 分册](json-contract-docx.md#响应) 相同。
  不算主题排版引擎，不把 `schemeClr` / `phClr` 展开成 RGB。
  备注母版与讲义母版的独立 theme 副本不进这份 `masters[].theme`。
  `clrMap` 取该 slideMaster 的 `p:clrMap`。对象十二键与属性同名：`bg1`、`tx1`、`bg2`、`tx2`、`accent1`–`accent6`、`hlink`、`folHlink`。
  值为 theme 槽名原文。缺该元素则为 `null`。缺某个属性则为 `null`。不把映射结果算成 RGB。
  `placeholders` 只收带 `p:nvPr/p:ph` 的 `p:sp`（含组合里的），按文档顺序。
  `shapes` 只收没有 `p:ph` 的 `p:sp`（含组合里的，含无 `txBody`），按文档顺序。不收 `p:pic`、`p:cxnSp`、`p:graphicFrame`。
  每份版式字段顺序：`id`、`index`、`name`、`kind`、`clrMapOvr`、`transition`、`animCount`、`placeholders`、`shapes`。
  `clrMapOvr` 规则见下文幻灯片。读该 slideLayout 的 `p:clrMapOvr`。
  `transition` 规则与幻灯片相同，读该 slideLayout 的 `<p:transition>`。
  `scope.timing` 同开时在 `animCount` 之后、`placeholders` 之前带 `timing`。
  版式对象名单规则与母版相同，插在 `shapes` 之后。
  `id` 为 `master:{n}/layout:{k}`。
  版式 `timing` 的 id 为 `master:{n}/layout:{k}/anim:{m}`，`target` 对应该版式形状 id（占位符或装饰形）。
  `animCount` 只计该 slideLayout 的 `<p:timing>`。`name` 取该 slideLayout 的 `p:cSld/@name`，没有则为 `null`。
  `kind` 取 `p:sldLayout/@type` 原文，没有则为 `null`。create 写出的那份空白版式 `kind` 为 `blank`、名为「空白」。
  占位符字段顺序：`id`、`shapeId`、`name`、`type`、`idx`、`x`、`y`、`cx`、`cy`、`text`，
  然后 `sz`、`color`、`textFill`、`textOutline`、`bold`、`italic`、`underline`、`font`、`eastAsiaFont`、`align`、`textAnchor`、`spcBef`、`spcAft`、`lnSpc`、`runs`、`stable`。
  `id` 为 `master:{n}/shape:{cNvPrId}` 或 `master:{n}/layout:{k}/shape:{cNvPrId}`；
  没有 `cNvPr/@id` 则为 `…/shape:index:{i}`，`stable` 为 `false`，`{i}` 只在该母版或该版式被收入的占位符内从 1 计。
  `type` 取 `p:ph/@type`，缺则为 `null`。`idx` 取 `p:ph/@idx` 原文，缺则为 `null`。几何与 `text` 规则与幻灯片形状相同；没有 `txBody` 则 `text` 为 `""`。
  装饰形字段顺序与幻灯片形状相同：`id`、`shapeId`、`name`、`prst`，有 `a:custGeom` 时再跟 `geom`，
  然后 `x`、`y`、`cx`、`cy`、`text`，
  然后 `sz`、`color`、`textFill`、`textOutline`、`bold`、`italic`、`underline`、`font`、`eastAsiaFont`、`align`、`textAnchor`、`spcBef`、`spcAft`、`lnSpc`、`runs`、`stable`。
  装饰形 `id` 同一套 `…/shape:{cNvPrId}`。没有 `cNvPr/@id` 则为 `…/shape:index:{i}`，`{i}` 只在该宿主被收入的装饰形内从 1 计，不和占位符的 `index` 混编。
  没有 `txBody` 则 `text` 为 `""`。不把这些 `p:sp` 抄进各页。
  xlsx / docx 的 `scope` 里出现 `masters` 报 `InvalidField`。
- `scope.notes` 为 `true` 时，每页带 `notes`。没有 notesSlide 关系则为 `null`。
  有则为对象，字段顺序：`id`、`clrMapOvr`、`themeOverride`、`pictureCount`、`mathCount`、`videoCount`、`audioCount`、`chartCount`、`smartArtCount`、`hyperlinkCount`、`fieldCount`、`tableCount`、`oleObjectCount`、`model3dCount`、`zoomCount`、`connectorCount`、`animCount`，
  有图名单时再跟 `pictures`，有公式框名单时再跟 `maths`，有媒体名单时再跟 `videos` 与 `audios`，有图表名单时再跟 `charts`，
  有 SmartArt 名单时再跟 `smartArts`，有表名单时再跟 `tables`，有 OLE 名单时再跟 `oleObjects`，有 3D 模型名单时再跟 `model3ds`，
  有 Zoom 名单时再跟 `zooms`，有连接线名单时再跟 `connectors`，有超链接名单时再跟 `hyperlinks`，有域名单时再跟 `fields`，
  `scope.timing` 同开时再跟 `timing`，然后 `shapes`。
  `id` 为 `slide:{n}/notes`。
  `clrMapOvr` / `themeOverride` 规则与幻灯片相同，读该 notesSlide。
  `pictureCount` 只计该备注页上被收入的 `p:pic`，不并进幻灯片 `pictures`。
  备注图字段顺序与幻灯片图相同，含末尾 `contentType`。
  备注上各 Count 只计该备注页，不并进幻灯片计数。根上各 Count 仍只计该页幻灯片 XML，不含备注与母版。
  备注 `timing` 的 id 为 `slide:{n}/notes/anim:{k}`，`target` 对该备注页对象。
  根上 `animCount` 与各页 `animCount` 仍只计该页幻灯片 XML，不含备注与母版。
  备注形状判定与幻灯片形状相同（带 `p:txBody` 的 `p:sp`），id 为 `slide:{n}/notes/shape:…`。
  字段顺序与幻灯片形状相同：`id`、`shapeId`、`name`、`prst`，有 `a:custGeom` 时再跟 `geom`，
  然后 `x`、`y`、`cx`、`cy`、`text`，
  然后 `sz`、`color`、`textFill`、`textOutline`、`bold`、`italic`、`underline`、`font`、`eastAsiaFont`、`align`、`textAnchor`、`spcBef`、`spcAft`、`lnSpc`、`runs`、`stable`。
  备注形状一次倒出，不用 `maxShapes`。
- `scope.comments` 为 `true` 时，每页带 `comments`。没有现代批注时为空数组。不依赖 `scope.slide`。按该页 `p188:cmLst` 里 `p188:cm` 的文档顺序。
  每条字段顺序：`id`、`commentId`、`author`、`initials`、`date`、`done`、`kind`、`x`、`y`、`pictureCount`，有图名单时再跟 `pictures`，然后 `text`、`replies`。
  `commentId` 是 `p188:cm/@id` 原文。`author` / `initials` 按 `authorId` 去 `/ppt/authors.xml` 的 `p188:author` 取 `name` / `initials`，对不上则为 `null`。
  `date` 取 `created` 原文，缺则为 `null`。`done`：`status` 为 `resolved` 或 `closed` 为 `true`；缺 `status`（即 active）为 `false`。
  `kind` 为 `normal`；该 `cm` 内出现墨迹标记（本地名为 `ink`）则为 `ink`。`x` / `y` 取 `p188:pos` 的 EMU，缺则为 `null`。
  `pictureCount` 只计该条 `cm` 下、不含 `replyLst` 的 `a:blip`（带 `r:embed` 或 `asvg:svgBlip`）。`text` 取 `p188:txBody` 可见纯文本，规则与形状 `text` 相同。
  回复嵌在 `replies` 里，不单独占 `commentCount`。回复字段顺序：`id`、`commentId`、`author`、`initials`、`date`、`done`、`kind`、`pictureCount`，
  有图名单时再跟 `pictures`，然后 `text`（无 `x` / `y`）。回复 `id` 为 `{父批注 id}/reply:{guid}`。
  批注 / 回复内每张图字段顺序：`id`，再跟 `contentType`（规则与幻灯片图相同）。
- `scope.hyperlinks` 为 `true` 时，每页带 `hyperlinks`。没有链时为空数组。不依赖 `scope.slide`。
  顺序见上文超链接 `{k}`。每条字段顺序：`id`、`index`、`kind`、`on`、`event`、`host`、`rId`、`url`、`file`、`slide`、`tooltip`、`text`。
  `index` 与 id 里的 `{k}` 相同。
  `on` 为 `shape` / `run` / `pic` / `table` / `chart` / `smartArt` / `cell`。
  `event` 为 `click`（`hlinkClick`）或 `hover`（`cNvPr` 上 `hlinkHover`，run 上 `hlinkMouseOver`）。
  `host` 是所在对象 id（单元格链用单元格 id）。`rId` 是 `r:id` 原文，缺则为 `null`。
  `kind` 为 `url`（External hyperlink 关系且没有文件跳转 `action`）、`file`（`action` 为 `ppaction://hlinkfile`）或 `slide`（slide 关系，
  或 `action` 为 `ppaction://hlinksldjump`）。
  `url` / `file` / `slide` 按 kind 填对应目标，其余为 `null`。`url` 与 `file` 取该关系 `Target` 原文，缺关系则为 `null`。
  `slide` 对到 `slide:{n}`，对不上则为 `null`。`tooltip` 取 `tooltip` 属性，缺则为 `null`。
  `text`：run / cell 取该 `a:r` 的 `a:t`；框级为 `null`。
  不把形状整框可见字抄进框级这条。xlsx / docx 的 `scope.hyperlinks` 见各自分册。
- `scope.fields` 为 `true` 时，每页带 `fields`。没有域时为空数组。不依赖 `scope.slide`。按该页被收入文本形状里 `a:fld` 的文档顺序。
  每条字段顺序：`id`、`type`、`host`、`text`。`type` 取 `@type` 原文，缺则为 `""`。inspect 不白名单；Office 自造类型原样回。`host` 是所在形状 id。
  `text` 是该域 `a:t` 缓存，不求值。xlsx 的 `scope` 里出现 `fields` 报 `InvalidField`。
  docx 见 [json-contract-docx.md](json-contract-docx.md#inspect)。
- 页对象字段顺序：`id`、`index`、`name`、`layoutId`、`hidden`、`transition`、`clrMapOvr`、`themeOverride`、`hasNotes`、`pictureCount`、`mathCount`、`videoCount`、`audioCount`、
  `chartCount`、`smartArtCount`、`commentCount`、`hyperlinkCount`、`fieldCount`、`tableCount`、`oleObjectCount`、`model3dCount`、`zoomCount`、
  `connectorCount`、`animCount`，
  然后按需 `pictures`，再按需 `maths`，再按需 `videos`，再按需 `audios`，再按需 `charts`，再按需 `smartArts`，再按需 `tables`，再按需 `oleObjects`，再按需 `model3ds`，再按需 `zooms`，
  再按需 `connectors`，再按需 `notes`，再按需 `comments`，再按需 `hyperlinks`，再按需 `fields`，再按需 `timing`，再按需 `truncated` 与 `shapes`。
  根对象字段顺序：`schemaVersion`、`format`、`id`、`slideCount`、`pictureCount`、`mathCount`、`videoCount`、`audioCount`、`chartCount`、
  `smartArtCount`、`commentCount`、`hyperlinkCount`、`fieldCount`、`tableCount`、`oleObjectCount`、`model3dCount`、`zoomCount`、`connectorCount`、
  `animCount`、`masterCount`、`hasHandout`、`sectionCount`、`slideSize`、`notesSize`、`customShowCount`、`showPr`、`modifyProtection`、`props`，
  有 `scope.masters` 时再跟 `masters`，有 `scope.notesMaster` 时再跟 `notesMaster`，有 `scope.handout` 时再跟 `handout`，有 `scope.sections` 时再跟 `sections`，
  有 `scope.customShows` 时再跟 `customShows`，然后 `slides`。
  `props` 始终出现，规则见 [json-contract.md](json-contract.md#文档属性三格式共用)。
  `clrMapOvr` 取该页（或版式、备注页）的 `p:clrMapOvr`。
  有 `a:masterClrMapping` 则为字符串 `"master"`。
  有 `a:overrideClrMapping` 则为对象，十二键与 `clrMap` 相同。
  缺 `p:clrMapOvr`、或该元素下两种子元都没有，则为 `null`。
  `themeOverride` 取该页或该备注页 `.rels` 里 Type 以 `/themeOverride` 结尾的部件。
  没有该关系或部件不在包内则为 `null`。有则字段与 [docx themeOverride](json-contract-docx.md#响应) 相同。
  版式不带 `themeOverride`。根上不要带 `themeOverride`。不把映射或覆盖色算成最终 RGB。
- `shapeId` 是 `cNvPr/@id` 的原文。走 `shape:index:{k}` 时为 `null`。
  每条字段顺序：`id`、`shapeId`、`name`、`prst`，有 `a:custGeom` 时再跟 `geom`，然后 `x`、`y`、`cx`、`cy`、`text`，
  然后 `sz`、`color`、`textFill`、`textOutline`、`bold`、`italic`、`underline`、`font`、`eastAsiaFont`、`align`、`textAnchor`、`spcBef`、`spcAft`、`lnSpc`、`runs`、`stable`。
- 形状上的 `name` 取 `cNvPr/@name` 的原文。没有则为 `null`。
- `prst` 取该 `p:sp` 的 `p:spPr/a:prstGeom/@prst`。没有 `prstGeom`（例如 `custGeom`）则为 `null`。
- `geom` 只在有 `a:custGeom` 时出现，不要给只有 `prstGeom` 的形状补 `null`。
  字段顺序：`paths`。`paths` 按 `a:pathLst` 里 `a:path` 的文档顺序。
  每条路径字段顺序：`w`、`h`、`ops`。`w` / `h` 取 `a:path/@w` / `@h` 的整数，缺则为 `null`。
  `ops` 按该 `a:path` 子元素顺序。
  - `moveTo` / `lnTo`：`type` 为该名，再跟 `x`、`y`（取唯一 `a:pt`）
  - `close`：只有 `type`
  - `cubicBezTo` / `quadBezTo`：`type` 为该名，再跟 `pts`（各 `a:pt` 的 `x` / `y` 数组）
  - `arcTo`：`type` 为 `arcTo`，再跟 `wR`、`hR`、`stAng`、`swAng`（属性原文，能解析为整数则报整数）
  - 其它子元素：`type` 为本地名；有 `a:pt` 则再跟 `pts`
  `x` / `y` / `pts[].x` / `pts[].y` 解析失败则为 `null`。create 给了 `shapes[].geom` 时按下文 create 写入。
- `x` / `y` 取 `p:spPr/a:xfrm/a:off` 的 EMU；`cx` / `cy` 取 `a:ext`。缺对应属性则为 `null`。备注形状同样。
- `text` 是可见纯文本：该形状 `p:txBody` 内、位于 `a:r` 里的 `a:t` 按文档顺序拼接；`a:br` 变成 `\n`。不含 run 级样式。
  `a:fld` 是应用程序更新的生成文本（§21.1.2.2.4），域里的 `a:t` 是缓存，不进入。`mc:AlternateContent` 只取一支，判定见上文 pptx 那节。
- `sz` / `color` / `textFill` / `textOutline` / `bold` / `italic` / `underline` / `font` / `eastAsiaFont` / `align` / `textAnchor` / `spcBef` / `spcAft` / `lnSpc` / `runs`
  始终出现在 `text` 之后、`stable` 之前。
  各文本 run 显式值一致则报该值，不一致或没有显式值则为 `null`，不猜主题。
  `sz` 是百分之一磅。`color` 只报 `a:solidFill/a:srgbClr` 的六位大写 RGB。渐变或 `noFill` 时 `color` 为 `null`。
  `textFill` / `textOutline` / `runs` 规则见 [json-contract.md](json-contract.md#setshapeformat)。
  `align` 取 `a:pPr/@algn`。`textAnchor` 取 `a:bodyPr/@anchor`。
  `spcBef` / `spcAft` / `lnSpc` 取各段 `a:pPr` 对应子元，不一致则为 `null`。`spc` / `kern` 只出现在 `runs[]`。
- `stable`：有 `cNvPr/@id` 为 `true`，否则为 `false`。
- 不返回完整动画时间轴树。效果摘要见上文 `scope.timing`。写入见 `insertAnim` / `setAnim` / `deleteAnim` / `moveAnim`。母版与版式见上文 `scope.masters`。备注母版见上文 `scope.notesMaster`。讲义母版槽位与占位符见上文 `scope.handout`。
  切换见上文 `transition`。对批注或回复 id 发 `insertPicture` 写 `a:blip`。对批注内图 id 发 `setPictureSize` / `setPicturePos` 报 `InvalidId`。


## create

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
          {
            "text": "标题"
          },
          {
            "text": "hello"
          }
        ]
      },
      {
        "shapes": [
          {
            "text": "第二页"
          }
        ]
      }
    ]
  }
}
```

- **`path`**
  - 输出路径
  - 已存在则覆盖
  - 省略时必须由调用方另给落盘路径，命令行用 `--output`
- **`presentation.slides`**
  - 页列表
  - 可空
  - 空则生成一页，页上有一个空文本框，否则 PowerPoint 可能打不开
- **`presentation.props`**
  - 可选
  - 文件属性，字段与 [json-contract.md](json-contract.md#setprops) 的 `setProps` / `setCustomProp` 相同
- **`presentation.sections`**
  - 可选
  - 幻灯片节列表
  - 省略或空数组则不写 `p14:sectionLst`
- **`presentation.customShows`**
  - 可选
  - 自定义放映列表
  - 省略或空数组则不写 `p:custShowLst`
- **`presentation.notesSize`**
  - 可选
  - 备注页画布 `p:notesSz`
  - 省略则仍写缺省 `6858000` × `9144000`
- **`presentation.handout`**
  - 可选
  - 讲义母版页眉页脚文字
  - 省略则不写讲义母版
- **`presentation.showPr`**
  - 可选
  - 放映设置
  - 省略则 `presProps` 仍只有 `clrMru`，inspect 根上仍是 PowerPoint 缺省
- **`presentation.modifyProtection`**
  - 可选
  - 修改保护
  - 省略则不写 `p:modifyVerifier`
- **`slides[].shapes`**
  - 该页文本框列表
  - 可空
  - 空则生成一个空文本框
- **`slides[].pictures`**
  - 可选
  - 该页上的图列表
  - 省略或空数组则不写 `p:pic`
- **`slides[].maths`**
  - 可选
  - 该页上的 drawing 公式框
  - 省略或空数组则不写
  - 每项必须有 `text`，可选 `x` / `y` / `cx` / `cy`，规则与 `insertMath` 相同
  - 不要写进 `shapes[]`
- **`slides[].videos`**
  - 可选
  - 该页上的内嵌视频
  - 省略或空数组则不写视频 `p:pic`
- **`slides[].audios`**
  - 可选
  - 该页上的内嵌音频
  - 省略或空数组则不写音频 `p:pic`
- **`slides[].charts`**
  - 可选
  - 该页上的图表列表
  - 省略或空数组则不写图表 `p:graphicFrame`
- **`slides[].smartArts`**
  - 可选
  - 该页上的 SmartArt 列表
  - 省略或空数组则不写图示 `p:graphicFrame`
- **`slides[].tables`**
  - 可选
  - 该页上的 DrawingML 表列表
  - 省略或空数组则不写表 `p:graphicFrame`
- **`slides[].model3ds`**
  - 可选
  - 该页上的 3D 模型列表
  - 省略或空数组则不写
  - 每项必须有 `file`，可选 `x` / `y` / `cx` / `cy`，规则与 `insertModel3d` 相同
- **`slides[].oleObjects`**
  - 可选
  - 该页嵌入对象
  - 省略或空数组则不写 `p:oleObj`
- **`slides[].connectors`**
  - 可选
  - 该页连接线
  - 省略或空数组则不写 `p:cxnSp`
- **`slides[].zooms`**
  - 可选
  - 该页 Zoom
  - 省略或空数组则不写 Zoom
- **`slides[].notes`**
  - 可选
  - 该页演讲者备注
  - 可以是字符串，可空
  - 也可以是对象：`text` 加可选 `anims`、`maths`、`model3ds`
  - 省略则不写 notesSlide
  - 字符串或对象出现（含空串）则按 `setNotesText` 写该页 `body` 占位符
  - 字符串形式仍只表示备注文本，不能带公式或 3D
- **`slides[].comments`**
  - 可选
  - 该页现代批注列表
  - 省略或空数组则不写批注
- **`slides[].transition`**
  - 可选
  - 该页切换对象
  - 省略则该页没有 `<p:transition>`
- **`slides[].anims`**
  - 可选
  - 该页预设进入效果
  - 省略或空数组则不写 `<p:timing>`
- **`shapes[].text`**
  - 可见文本，可空字符串
  - 省略当作 `""`
  - `\n` 写成 `a:br`
- **`shapes[].url` / `shapes[].file` / `shapes[].slide`**
  - 可选
  - 超链接，规则与 pptx `insertHyperlink` 相同，三者必须恰好一个
  - `slide` 是 `slides[]` 的 1 起下标，不是 `slide:{n}` 字符串
- **`shapes[].on`**
  - 可选。`shape`（缺省）或 `run`
- **`shapes[].event`**
  - 可选。`click`（缺省）或 `hover`
- **`shapes[].geom`**
  - 可选
  - 自定义几何，规则与 `insertShape` 的 `geom` 相同
  - 不要给矩形补 `geom`
- **`shapes[].fields`**
  - 可选
  - 写入该文本框的 `a:fld` 列表
  - 省略或空数组则不写域
  - 不要在 `slides[]` 上开 `fields` 名单
- **`shapes[].tooltip`**
  - 可选
  - 形状级超链接提示
  - 没有 `url` / `file` / `slide` 却给了此项报 `InvalidField`
- **`shapes[].sz` / `color` / `textFill` / `textOutline` / `bold` / `italic` / `underline` / `font` / `eastAsiaFont` / `align` / `textAnchor` / `spcBef` / `spcAft` / `lnSpc` / `spc` / `kern`**
  - 可选
  - 规则与 `setShapeFormat` 相同
  - 省略则缺省 18 磅黑字 Calibri / 宋体
- **`pictures[].file`**
  - 本地路径，与 `insertPicture` 相同，按文件头认 PNG / JPEG / GIF / BMP / SVG / EMF / WMF / TIFF
  - 缺此字段报 `MissingField`
- **`pictures[].x` / `pictures[].y`**
  - 可选，EMU，非负整数
  - 必须成对出现或成对省略
  - 只给一个时报 `InvalidField`
  - 缺则放到该页空白处，规则与 pptx `insertPicture` 相同
- **`pictures[].cx` / `pictures[].cy`**
  - 可选，EMU，正整数
  - 都省则与 `insertPicture` 相同
  - 只给一个时报 `InvalidField`
- **`pictures[].hyperlink`**
  - 可选对象
  - 字段与 `insertHyperlink` 相同：`url` / `file` / `slide` 恰好一个
  - 可选 `tooltip`
  - 可选 `event`（`click` 或缺省，或 `hover`）
  - 不要 `text`、不要 `on`、不要 `op` / `id`
  - `slide` 是 `slides[]` 的 1 起下标，不是 `slide:{n}`
- **`charts[].type`**
  - 必须
  - 与 pptx `insertChart` 相同
- **`charts[].series`**
  - 必须
  - 非空
  - 字段与 `insertChart` 相同
- **`charts[].title`**
  - 可选
- **`charts[].x` / `charts[].y` / `charts[].cx` / `charts[].cy`**
  - 可选
  - 规则与 pptx `insertChart` 相同
- **`charts[].hyperlink`**
  - 可选对象
  - 字段与 `pictures[].hyperlink` 相同
- **`smartArts[].layout`**
  - 必须
  - `list`、`org` 或 `cycle`，与 pptx `insertSmartArt` 相同
- **`smartArts[].points`**
  - 可选
  - 与 `insertSmartArt` 相同
  - 省略则用该版式的缺省占位点
- **`smartArts[].x` / `smartArts[].y` / `smartArts[].cx` / `smartArts[].cy`**
  - 可选
  - 规则与 pptx `insertSmartArt` 相同
- **`smartArts[].hyperlink`**
  - 可选对象
  - 字段与 `pictures[].hyperlink` 相同
- **`tables[].rows` / `tables[].cols`**
  - 必须
  - 正整数，规则与 pptx `insertTable` 相同
  - `cols` 是列数，不是列宽数组
- **`tables[].colWidths`**
  - 可选
  - EMU 正整数数组
  - 长度必须等于 `tables[].cols`
  - 省略则按框 `cx` 均分写入 `a:gridCol/@w`
- **`tables[].rowHeights`**
  - 可选
  - 数组，长度必须等于 `tables[].rows`
  - 每项为 EMU 正整数
  - 省略则按框 `cy` 均分写入 `@h`
- **`tables[].cells`**
  - 可选
  - 二维数组，行数与列数必须与 `rows` / `cols` 一致
  - 每格可以是字符串，也可以是对象
  - 对象可有 `text`（省略当作 `""`）
  - 对象可有可选 `fill`：规则与 `setTableCellFill` 的 `fill` 相同（六位字符串、对象或 `null`）
  - `null` 写成 `a:noFill`。省略则不写填充子元
  - `type` 为 `blip` 时必须有 `file`（本地图片路径，规则与 `insertPicture` 相同）
  - 对象可有可选 `borders`：规则与 `setTableCellBorders` 相同
  - 对象可有可选 `cellMar`：规则与 `setTableCellInset` 相同（至少一边；省略则不写 `mar*`）
  - 对象可有可选 `vert` / `anchorCtr` / `horzOverflow` / `cell3D` / `headers`：规则与 `setTableCellPr` 相同
  - 对象上的 `url` / `file` / `slide` 有则写格内链，规则与 `insertHyperlink` 相同，三者必须恰好一个
  - 可选 `tooltip`、可选 `event`
  - 都没有 `url` / `file` / `slide` 则只写文本
  - 不要 `on`、不要嵌套 `hyperlink`
- **`tables[].hyperlink`**
  - 可选对象
  - 写在表框上，字段与 `pictures[].hyperlink` 相同
  - 与格内链可以同时有
- **`tables[].x` / `tables[].y` / `tables[].cx` / `tables[].cy`**
  - 可选
  - 规则与 pptx `insertTable` 相同
- **`comments[].text`**
  - 可选
  - 省略当作 `""`
  - 规则与 pptx `insertComment` 相同
- **`comments[].author`**
  - 可选
  - 省略当作 `"ooxp"`
- **`comments[].initials`**
  - 可选
  - 省略当作 `"oo"`
- **`comments[].date`**
  - 可选
  - `xsd:dateTime`
  - 省略则写入当前 UTC（无尾 `Z`）
- **`comments[].x` / `comments[].y`**
  - 可选，EMU，非负整数
  - 必须成对出现或成对省略
  - 只给一个时报 `InvalidField`
  - 缺则放到该页空白处，规则与 pptx `insertComment` 相同
- **`videos[].file`**
  - 本地路径，与 `insertVideo` 相同，按文件头认 MP4
  - 缺此字段报 `MissingField`
- **`videos[].x` / `videos[].y` / `videos[].cx` / `videos[].cy`**
  - 可选
  - 规则与 pptx `insertVideo` 相同
- **`videos[].st` / `videos[].end` / `videos[].bookmarks` / `videos[].tracks` / `videos[].trackDisplayLoc`**
  - 可选
  - 规则与 pptx `insertVideo` 相同
- **`audios[].file`**
  - 本地路径，与 `insertAudio` 相同，按文件头认 WAV
  - 缺此字段报 `MissingField`
- **`audios[].x` / `audios[].y` / `audios[].cx` / `audios[].cy`**
  - 可选
  - 规则与 pptx `insertAudio` 相同
- **`audios[].st` / `audios[].end` / `audios[].bookmarks` / `audios[].tracks` / `audios[].trackDisplayLoc`**
  - 可选
  - 规则与 pptx `insertAudio` 相同
- **`transition.type`**
  - 必须
  - 效果名，规则与 `setTransition` 相同
  - `null`、空串或 `other` 报 `InvalidField`（create 没有「清除」）
- **`transition.dur` / `advClick` / `advTm` / `dir` / `invX` / `invY` / `sound` / `orient` / `spokes`**
  - 可选
  - 规则与 `setTransition` 相同
  - 不要 `op` / `id`
- **`anims[].type`**
  - 必须
  - 规则与 `insertAnim` 的 `type` 相同
- **`anims[].trigger`**
  - 可选
  - `click` / `with` / `after` / `clickOn`，省略为 `click`
- **`anims[].delay`**
  - 可选
  - 规则与 `insertAnim` 的 `delay` 相同
  - 省略为 `0`
- **`anims[].dur`**
  - 可选
  - 规则与 `insertAnim` 的 `dur` 相同
  - 省略为 `500`
- **`anims[].repeat`**
  - 可选
  - 规则与 `insertAnim` 的 `repeat` 相同
  - 省略为 `1`
- **`anims[].sound`**
  - 可选
  - 规则与 `insertAnim` 的 `sound` 相同
- **`anims[].para`**
  - 可选
  - 规则与 `insertAnim` 的 `para` 相同
- **`anims[].clickOn`**
  - `trigger` 为 `clickOn` 时必须
  - 规则与 `insertAnim` 的 `clickOn` 相同
  - 值为该页 create 数组的序号对象：与 `shape` / `picture` 等同一套键，且恰好一个
  - 不要写成 inspect id
- **`anims[].shape` / `picture` / `video` / `audio` / `table` / `chart` / `smartArt`**
  - 绑定目标，1 起的序号，且恰好一个
  - 对应该页 create 数组
  - `shapes` 缺省补的空框算第 1 个
  - 序号对不上报 `InvalidField`
  - 不要 `op` / `id`
- **`sections[].name`**
  - 必须
  - 显示名，字符串，允许空串
  - 缺省报 `MissingField`
- **`sections[].start`**
  - 必须
  - `slides[]` 的 1 起下标，不是 `slide:{n}`
  - 必须严格递增
  - 超出页数报 `InvalidField`
- **`shapes[].fields[].type`**
  - 必须
  - 规则与 `insertField` 的 `type` 相同
- **`shapes[].fields[].text`**
  - 可选
  - 规则与 `insertField` 的 `text` 相同
  - 省略当作 `""`
- **`oleObjects[].file`**
  - 必须
  - 本地路径，规则与 `insertOle` 相同
- **`connectors[].prst`**
  - 可选
  - 规则与 `insertConnector` 的 `prst` 相同
  - 省略为 `straightConnector1`
- **`connectors[].x` / `y` / `cx` / `cy`**
  - 可选
  - 规则与 `insertConnector` 相同
- **`connectors[].start` / `connectors[].end`**
  - 可选
  - 对象，必须有 `shape`，可选 `site`
  - `shape` 是该页 `shapes[]` 的 1 起下标，不是 inspect id
  - `site` 为正整数，省略为 `1`，写入 `idx = site - 1`
  - 不要写成 `{ "id": "slide:…/shape:…" }`
- **`zooms[].type`**
  - 必须
  - `slide` / `section` / `summary`，规则与 `insertZoom` 相同
- **`zooms[].target`**
  - `type` 为 `slide` 时必须，值为 `slides[]` 的 1 起下标，不能是本页
  - `type` 为 `section` 时必须，值为 `sections[]` 的 1 起下标
  - `type` 为 `summary` 时不要写 `target`
  - 不要写成 `slide:{n}` / `section:{n}`
- **`zooms[].targets`**
  - `type` 为 `summary` 时必须
  - 非空数组，每项是 `sections[]` 的 1 起下标
  - 其它 `type` 上出现报 `InvalidField`
- **`zooms[].x` / `y` / `cx` / `cy`**
  - 可选
  - 规则与 `insertZoom` 相同
- **`notes.text`**
  - `notes` 为对象时可选
  - 省略当作 `""`
- **`notes.anims`**
  - `notes` 为对象时可选
  - 备注页预设进入效果，规则与 `slides[].anims` 相同
  - `shape` 等序号对应该页备注页对象，不是幻灯片 `shapes[]`
  - 省略或空数组则备注页不写 `<p:timing>`
- **`notes.maths`**
  - `notes` 为对象时可选
  - 备注页 drawing 公式框
  - 每项字段与 `slides[].maths` 相同
  - 不要 `op` / `id`
  - 省略或空数组则不写
- **`notes.model3ds`**
  - `notes` 为对象时可选
  - 备注页 3D 模型
  - 每项字段与 `slides[].model3ds` 相同
  - 不要 `op` / `id`
  - 省略或空数组则不写
- **`customShows[].name`**
  - 必须
  - 规则与 `insertCustomShow` 的 `name` 相同
- **`customShows[].slides`**
  - 可选
  - `slides[]` 的 1 起下标数组，不是 `slide:{n}`
  - 省略或空数组则该放映没有页
- **`handout.header` / `footer` / `date` / `sldNum`**
  - 至少一个必须
  - 规则与 `setHandoutText` 相同
- **`showPr.type`**
  - 可选
  - `present` / `browse` / `kiosk`，规则与 `setShowPr` 相同
- **`showPr.loop` / `showNarration` / `showAnimation` / `useTimings`**
  - 可选
  - 规则与 `setShowPr` 相同
  - `showPr` 对象至少要有上述字段之一
- **`modifyProtection.enforced`**
  - 必须
  - 规则与 `setProtection` 相同
- **`modifyProtection.password`**
  - 可选
  - 给了则写 PowerPoint 16.0 的 SHA-512 `crypt*`
  - 空串报 `InvalidField`
  - 带哈希产物不要用 COM 打开

带图示例见 `docs/examples/pptx-create-pictures.json`。带 SVG 图见 `docs/examples/pptx-create-svg.json`。带 3D 模型见 `docs/examples/pptx-create-model3d.json`。带公式框见 `docs/examples/pptx-create-math-drawing.json`。带图表示例见 `docs/examples/pptx-create-charts.json`。
带 SmartArt 示例见 `docs/examples/pptx-create-smartart.json`。带表示例见 `docs/examples/pptx-create-tables.json`。
带备注与批注示例见 `docs/examples/pptx-create-notes-comments.json`。节、链、媒体、切换与动画合在一起的示例见 `docs/examples/pptx-create-remain.json`。
OLE、Zoom、域、连接线、自定义放映、讲义、备注动画与放映 / 修改保护合在一起的示例见 `docs/examples/pptx-create-round4.json`。
备注页公式 / 3D 与图 / 表 / 图表 / SmartArt 链合在一起的示例见 `docs/examples/pptx-create-round5.json`。
`kind` / `wrap` / `from` / `to` / `anchor` / `dataFrom` 以及 xlsx 偏移字段出现报 `InvalidField`。
`comments[]` 上出现 `op` / `id` / `start` / `end` / `replies` / `ink` 报未知字段。create 不写回复、不写墨迹、不写 ECMA-376 §19.4 旧式批注。
`shapes[]` 上出现 `text` / `url` / `file` / `slide` / `tooltip` / `on` / `event` / `sz` / `color` / `textFill` / `textOutline` / `bold` / `italic` / `underline` / `font` / `eastAsiaFont` / `align` / `textAnchor` / `spcBef` / `spcAft` / `lnSpc` / `spc` / `kern` / `geom` / `fields` 以外的键报未知字段。
`text` / `anchor` / `location` / `ref` / `display` 出现在形状超链接上（除可见 `text` 外）报未知字段。
`pictures[].hyperlink` / `tables[].hyperlink` / `charts[].hyperlink` / `smartArts[].hyperlink` 上出现 `text` / `on` / `op` / `id` 报未知字段。
`tables[].cells` 对象上出现 `on` / `hyperlink` 报未知字段。
`url`、`file`、`slide` 不是恰好一个报 `InvalidField`。空串报 `InvalidField`。`slide` 不是正整数报 `InvalidField`。跳到本页报 `InvalidField`。
目标页超出 `slides[]` 报 `InvalidField`。

`workbook`、`document`、`presentation` 三者必须恰好出现一个。
xlsx 路径必须用 `workbook`，docx 路径必须用 `document`，pptx 路径必须用 `presentation`，对不上报 `InvalidField`。

create 写出 Transitional 最小包，见 [opc.md](opc.md)。
必须带 theme、slideMaster、slideLayout、presentation、各页 slide、presProps，不能只写 ECMA 示例里那个仅含 `notesSz` 的 presentation。
幻灯片尺寸用 16:9（`sldSz` 为 `cx="12192000"` `cy="6858000"`）。省略 `handout` 则不写讲义母版。不要在 `slides[]` 上写整棵 `timing` 树。
给了 `slides[].pictures` 时按 `insertPicture` 同一套规则写 `/ppt/media/…`、该页 `.rels` 与 `p:pic`。
给了 `slides[].maths` 时按 `insertMath` 同一套规则写 `a14:m` AlternateContent。不写进 `shapes[]`。
给了 `slides[].videos` 时按 `insertVideo` 同一套规则写媒体与视频 `p:pic`。给了 `slides[].audios` 时按 `insertAudio` 同一套规则写媒体与音频 `p:pic`。
给了 `slides[].charts` 时按 `insertChart` 同一套规则写 `/ppt/charts/…`、embedding 与图表 `p:graphicFrame`。
给了 `slides[].smartArts` 时按 `insertSmartArt` 同一套规则写 `/ppt/diagrams/…` 与图示 `p:graphicFrame`。
给了 `slides[].tables` 时按 `insertTable` 同一套规则写表 `p:graphicFrame`（`a:tbl`）。
给了 `slides[].model3ds` 时按 `insertModel3d` 同一套规则写 GLB、海报 PNG 与 `am3d` AlternateContent。
给了 `shapes[].geom` 时按 `setShapeGeom` 同一套规则把该文本框改成 `a:custGeom`。不要另写一套 XML。
给了 `shapes[].fields` 时按 `insertField` 同一套规则往该文本框写 `a:fld`。
给了 `slides[].oleObjects` 时按 `insertOle` 同一套规则写 embedding 与 `p:oleObj`。
给了 `slides[].connectors` 时按 `insertConnector` 同一套规则写 `p:cxnSp`。
给了 `slides[].notes` 时按 `setNotesText` 同一套规则写该页演讲者备注：没有 notesMaster 则新建，再写 notesSlide。
`notes` 为对象且给了 `anims` 时，按 `insertAnim` 同一套规则写到该页备注页，不算放映引擎。
`notes` 为对象且给了 `maths` 时，按 `insertMath` 同一套规则写到该页备注页。
`notes` 为对象且给了 `model3ds` 时，按 `insertModel3d` 同一套规则写到该页备注页。
给了 `slides[].comments` 时按 `insertComment` 同一套规则写该页现代批注（`p188:cmLst`、`/ppt/authors.xml`）。
给了形状上的 `url` / `file` / `slide` 时按 `insertHyperlink` 同一套规则写链（可 `on=run`，可 `event=hover`）。
给了 `pictures[].hyperlink` / `tables[].hyperlink` / `charts[].hyperlink` / `smartArts[].hyperlink` 时按 `insertHyperlink` 同一套规则写到对应对象上。
给了表单元格上的 `url` / `file` / `slide` 时按 `insertHyperlink` 同一套规则写格内链。
create 本期不写批注内图。
给了 `slides[].transition` 时按 `setTransition` 同一套规则写 `<p:transition>`。给了 `slides[].anims` 时按 `insertAnim` 同一套规则写预设效果（第一条 `with` / `after` 进片自动播）。
给了 `presentation.sections` 时按 `insertSlideSection` 同一套规则写 `p14:sectionLst`：先写第一项（`start` 为 1 则包住全部页；大于 1 则前面的页进空名节），再按数组顺序用 `start` 拆后续节。
给了 `slides[].zooms` 时按 `insertZoom` 同一套规则写 Zoom。节缩放 / 摘要缩放在 `sections` 写完之后再写。
给了 `presentation.customShows` 时按 `insertCustomShow` 同一套规则写 `p:custShowLst`。
给了 `presentation.notesSize` 时按 `setNotesSize` 同一套规则写 `p:notesSz`（仍写在缺省 `sldSz` 之后）。
给了 `presentation.handout` 时按 `setHandoutText` 同一套规则写讲义母版。
给了 `presentation.showPr` 时按 `setShowPr` 同一套规则写 `p:showPr`。
给了 `presentation.modifyProtection` 时按 `setProtection` 同一套规则写 `p:modifyVerifier`。
先写该页 `shapes`（含 `geom` 与 `fields`），再按数组顺序写 `pictures`，再写 `maths`，再写 `model3ds`，再写 `videos`，再写 `audios`，再写 `charts`，再写 `smartArts`，再写 `tables`，
再写 `oleObjects`，再写 `connectors`，再写 `notes`，再写 `notes.maths`，再写 `notes.model3ds`，
再按数组顺序写 `comments`，再写形状超链接，再写图 / 表框 / 表单元格 / 图表 / SmartArt 超链接，再写 `transition`，再按数组顺序写 `anims`，再写 `notes.anims`。
全部页写完后再写 `sections`，再写 `zooms`，再写 `customShows`，再写 `handout`，再写 `showPr`，再写 `modifyProtection`。
`p:cNvPr/@id` 取该页已有全部 `cNvPr` 的最大整数加 1，至少为 2。
`presentation` 根上出现 `pictures` / `charts` / `smartArts` / `tables` / `notes` / `comments` / `videos` / `audios`
 / `anims` / `transition` / `masters` / `notesMaster` / `hyperlinks` / `oleObjects` / `zooms` 报 `InvalidField`。
`shapes[]` 上出现 `pictures` / `charts` / `smartArts` / `tables` / `notes` / `comments` / `videos` / `audios` / `anims`
 / `transition` / `masters` / `notesMaster` / `handout` / `hyperlinks` / `sections` / `customShows` / `oleObjects` / `zooms` / `showPr` 报 `InvalidField`。
`slides[]` 上出现 `hyperlinks` 报未知字段。
链写在 `shapes[]` 上，以及 `pictures[]` / `tables[]` / `charts[]` / `smartArts[]` 的 `hyperlink` 与 `tables[].cells` 的链字段上，不另开页级名单。
`slides[]` 上出现 `fields` 报未知字段（域写在 `shapes[].fields`）。
`slides[]` 上出现 `timing` 报未知字段。不算放映引擎。
create 仍只写一份 `type="blank"`、名为「空白」的版式，不按 JSON 生成多份母版。
给了 `presentation.props` 时按 `setProps` / `setCustomProp` 同一套规则写 core / app / custom；
省略则仍写空的 `dc:title` 与 `dc:creator` 为 `ooxp`，不写 custom 部件。

每个文本框写成带 `p:txBody` 的 `p:sp`，`p:cNvPr/@id` 从 `2` 起在该页内递增（`1` 留给 `spTree`）。
随后 inspect 得到的 id 都是 `slide:{n}/shape:{id}`，`stable` 为 `true`。形状纵向错开，避免叠在同一位置。颜色用 RGB，不依赖主题色语义。


## apply

各格式认哪些 `op` 以 [json-contract.md](json-contract.md#apply) 那一段名单为准。跨格式 op 的字段表也在主文件。

### setPicturePos

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "setPicturePos",
      "id": "slide:1/pic:4",
      "x": 914400,
      "y": 914400
    },
    {
      "op": "setPictureSize",
      "id": "slide:1/video:3",
      "cx": 1828800,
      "cy": 914400
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setPicturePos"` |
| `id` | pptx 图 id、视频 id 或音频 id（均可带 `notes/`、`master:{n}/` 或 `master:{n}/layout:{k}/` 前缀）。批注内图 id 报 `InvalidId` |
| `x` | 可选。画布横坐标，EMU，非负整数 |
| `y` | 可选。画布纵坐标，EMU，非负整数 |

`x` 与 `y` 至少给一个，否则报 `InvalidField`。只改被点名的那一侧。写入 `p:spPr/a:xfrm/a:off`。找不到该对象报 `IdNotFound`。
`id` 不是合法图 id、视频 id 或音频 id 报 `InvalidId`。批注内图没有画布坐标。`setMedia` 仍不改几何。docx / xlsx 上发此 op 报 `InvalidField`。

### insertSlide

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "insertSlide",
      "id": "pptx:presentation",
      "after": "slide:1"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertSlide"` |
| `id` | 必须是 `pptx:presentation` |
| `after` | 可选。已有页的 `slide:{n}`。省略则追加到 `sldIdLst` 末尾；给出则插到该页之后 |

pptx：`id` 不是 `pptx:presentation` 报 `InvalidId`。`after` 找不到报 `IdNotFound`；不是页 id 报 `InvalidId`。
新建空白幻灯片部件（`p:spTree` 只有组合框，不抄版式占位符，不写 `<p:timing>`），关系指向一份已有 `slideLayout`：优先抄 `after` 那一页的版式关系，省略 `after` 则抄最后一页；该页没有版式关系则用第一份幻灯片母版的第一份版式。
找不到任何版式报 `InvalidField`。`p:sldId/@id` 取已有最大 `ST_SlideId` 加 1，至少为 256。部件路径用尚未占用的 `/ppt/slides/slideN.xml`，不改已有页的文件名。
若 presentation 带 `p14:sectionLst`，把新页的 `ST_SlideId` 写进 `after` 那一页所在节（省略 `after` 则跟最后一页那一节），使各节合起来与 `p:sldIdLst` 同序同成员。
不改自定义放映名单。docx / xlsx 上发此 op 报 `InvalidField`。

### deleteSlide

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteSlide"` |
| `id` | 要删除的 `slide:{n}` |

找不到该页报 `IdNotFound`。id 不是页 id 报 `InvalidId`。删掉演示文稿里最后一页报 `InvalidField`，不写文件。
去掉 `sldIdLst` 表项、presentation 关系、内容类型，以及该页部件及其 `.rels`。该页备注 `notesSlide`、现代批注部件、图表 / embedding / diagram / 媒体若再无人引用则删。
不删幻灯片版式、母版、主题、notesMaster。自定义放映里 `p:sld/@r:id` 等于被删 `sldId/@r:id` 的条目一并去掉；`p14:sectionLst` 里对应该页 `ST_SlideId` 的条目一并去掉。
未点名页原字节写回。docx / xlsx 上发此 op 报 `InvalidField`。

### moveSlide

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"moveSlide"` |
| `id` | 要移动的 `slide:{n}` |
| `after` | 可选。目标前一页的 `slide:{n}`。省略或 `null` 表示移到名单最前 |

找不到该页或 `after` 报 `IdNotFound`。id / `after` 不是页 id 报 `InvalidId`。`after` 与 `id` 是同一页报 `InvalidField`。已经在目标位置则空操作。
只改 `presentation.xml` 里 `sldIdLst` 子元素顺序，以及若有 `p14:sectionLst` 则按同一套顺序把该 `ST_SlideId` 挪到 `after` 那一页所在节（移到最前则进第一节）。
不改部件路径、`p:sldId/@id` 或关系 id。docx / xlsx 上发此 op 报 `InvalidField`。

### setSlideHidden

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setSlideHidden"` |
| `id` | `slide:{n}` |
| `hidden` | `true` 隐藏，`false` 显示 |

找不到该页报 `IdNotFound`。id 不是页 id 报 `InvalidId`。缺 `hidden` 报 `InvalidField`。`true` 在该页 `p:sld` 上写 `show="0"`。
`false` 去掉 `show`。已经是目标可见性则空操作。允许把全部页藏起来。不改 `sldIdLst`。docx / xlsx 上发此 op 报 `InvalidField`。

### setTransition

写或清除该宿主 `<p:transition>`。不写 `<p:timing>`。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTransition"` |
| `id` | `slide:{n}`、`master:{n}` 或 `master:{n}/layout:{k}` |
| `type` | 效果名，或 `null` 表示去掉切换。必填 |
| `dur` | 可选。时长，毫秒，整数，范围 `1`–`60000` |
| `advClick` | 可选。`false` 表示不单击进片。省略为 `true` |
| `advTm` | 可选。自动进片延迟，毫秒，整数，范围 `1`–`86400000`。`null` 或不给表示不定时 |
| `dir` | 可选。效果子元素的方向。省略则不写该属性 |
| `invX` / `invY` | 可选。只对 prst 名有意义。`true` 时写 `invX="1"` / `invY="1"` |
| `sound` | 可选。本地 WAV 路径，按文件头认，规则与 `insertAnim` 的 `sound` 相同 |
| `orient` | 可选。只对 `split`。`horz` 或 `vert`。省略则不写该属性 |
| `spokes` | 可选。只对 `wheel` / `wheelReverse`。整数 `1`–`8`。省略则不写该属性 |

找不到该页或该母版 / 版式报 `IdNotFound`。id 不是上述宿主报 `InvalidId`。
对 `slide:{n}/notes`、`notesMaster`、`handout` 报 `InvalidId`。缺 `type` 报 `InvalidField`。
`type` 为 `other`、空串、或不在 inspect 那份可写名单里（`none` 可写，表示空的 `<p:transition>` 且没有效果子元素；prst 白名单见上文 inspect）报 `InvalidField`。
`dur` 不是正整数或不在范围内报 `InvalidField`。`advTm` 不是正整数或不在范围内报 `InvalidField`。
该效果没有 `dir` 却给了 `dir`、`dir` 不在该效果允许值、或非 prst 却给了 `invX` / `invY`，报 `InvalidField`。
非 `split` 却给了 `orient`、或 `orient` 不是 `horz` / `vert`，报 `InvalidField`。
非 `wheel` / `wheelReverse` 却给了 `spokes`、或 `spokes` 不是 `1`–`8` 的整数，报 `InvalidField`。
`sound` 空串或 `null` 报错，规则与 `insertAnim` 相同。不是 WAV 报 `InvalidMediaFile`。
`type` 为 `null` 时忽略 `dur` / `advClick` / `advTm` / `dir` / `invX` / `invY` / `sound` / `orient` / `spokes`，
删掉该宿主的 `<p:transition>`；
若它包在只含切换的 `mc:AlternateContent` 里，整段 AlternateContent 删掉。已经没有切换则空操作。
清掉该切换不再引用的音频关系；仍被同部件 `<p:timing>` 引用的不删。

`type` 不是 `null` 时整段换成新的 `<p:transition>`（含原先的 AlternateContent）。
ECMA 名单里的效果写成 `p:` 子元素，直接放在该宿主根下，不包 AlternateContent。
母版插在 `sldLayoutIdLst` 之后、`timing` / `hf` / `txStyles` 之前。版式与幻灯片插在 `cSld` / `clrMapOvr` 之后、`timing` 之前。
prst 白名单与 `morph` 写成 `p15:`，其余 MS-PPTX 名单写成 `p14:`，外包 `mc:AlternateContent`：Choice 要求对应前缀，Fallback 写 `p:fade`。
prst 的效果子元素是 `p15:prstTrans`，`prst` 为该 `type`。`dur` 省略则按 `500`。
同时写 `p14:dur` 与 `spd`：`dur<=500` 为 `fast`，`dur<=750` 为 `med`，其余为 `slow`。
`advClick` 为 `true` 或不给则不写该属性；`false` 写 `advClick="0"`。给了 `advTm` 则写在 `<p:transition>` 上。
给了 `dir` / `orient` / `spokes` 则写在效果子元素上。
`invX` / `invY` 为真只写在 `p15:prstTrans` 上。
给了 `sound` 才写 `p:sndAc/p:stSnd/p:snd`（`r:embed` 与 `name`），Choice 与 Fallback 各写一份同一 `rId`。不写 `loop`。不写 `endSnd`。
不改 `<p:timing>`。docx / xlsx 上发此 op 报 `InvalidField`。

### insertAnim

给该页对象追加一条预设效果。只写与 PowerPoint 16.0 另存同构的最小 `<p:timing>` 树，不接受调用方提交的时间轴 JSON。

- **`op`**
  - 必须是 `"insertAnim"`
- **`id`**
  - 可绑对象：
    - `slide:{n}/shape:…`
    - `…/cxn:…`
    - `…/pic:…`
    - `…/video:…`
    - `…/audio:…`
    - `…/table:…`
    - `…/chart:…`
    - `…/smartdiagram:…`
    - 备注页上同样判定，id 带 `slide:{n}/notes/` 前缀
    - 母版 / 版式形状（占位符与装饰形）：`master:{n}/shape:…` 或 `master:{n}/layout:{k}/shape:…`
    - 备注母版占位符：`notesMaster/shape:…`
    - 讲义占位符：`handout/shape:…`
- **`type`**
  - 必须
  - 白名单：`appear`、`fade`、`flyIn`、`wipe`、`split`、`zoom`、`spin`、`growShrink`、`fadeOut`、`flyOut`、`wipeOut`、`pathRight`
  - 进入：`appear` / `fade` / `flyIn` / `wipe` / `split` / `zoom`
  - 强调：`spin` / `growShrink`
  - 退出：`fadeOut` / `flyOut` / `wipeOut`
  - 路径：`pathRight`（固定向右曲线，不接受路径点 JSON）
- **`trigger`**
  - 可选
  - `click` / `with` / `after` / `clickOn`，省略为 `click`
- **`delay`**
  - 可选
  - 毫秒，整数，范围 `0`–`60000`
  - 省略为 `0`
  - 写入该效果 `cTn` 的 `stCondLst/cond/@delay`
  - 相对 `trigger`：`with` 是相对上一条开始再等；`after` 是相对上一条结束再等；`click` 是单击后再等；`clickOn` 是单击 `clickOn` 那个对象后再等
- **`dur`**
  - 可选
  - 毫秒，整数，范围 `1`–`60000`
  - 省略为 `500`
  - 写入该模板里带时长的行为（`animEffect` / `anim` / `animRot` / `animScale` / `animMotion` 等）
- **`repeat`**
  - 可选
  - 次数，整数，范围 `1`–`99`；或 `null` 表示无限
  - 省略为 `1`（不写 `repeatCount`）
  - 大于 1 时写入效果 `cTn/@repeatCount`，值为次数乘以 1000（`3` 写 `3000`）
  - `null` 写 `repeatCount="indefinite"`
- **`sound`**
  - 可选
  - 本地 WAV 路径，按文件头认，规则与 `insertAudio` 相同
  - 省略则不写效果声音
  - 空串或 `null` 报 `InvalidField`
- **`para`**
  - 可选
  - 1 起的段落序号，写进该效果所有 `spTgt` 的 `txEl/pRg`
  - 省略则绑整个对象，不写 `txEl`
  - 目标没有 `txBody`、没有段落、或序号超出，报 `InvalidField`
- **`clickOn`**
  - `trigger` 为 `clickOn` 时必须，否则不许出现
  - 同一宿主上的可绑对象 id（与 `insertAnim` 的 `id` 同一套判定）
  - 找不到报 `IdNotFound`；不是可绑对象报 `InvalidId`

找不到该页报 `IdNotFound`。找不到该对象报 `IdNotFound`。id 不是上述对象报 `InvalidId`。
缺 `type`、`type` 不在白名单、或 `trigger` 不是四个之一，报 `InvalidField`。
`delay` 不是整数或不在 `0`–`60000`，报 `InvalidField`。
`dur` 不是整数或不在 `1`–`60000`，报 `InvalidField`。
`repeat` 不是整数、不是 `null`、或不在 `1`–`99`，报 `InvalidField`。
`trigger` 为 `clickOn` 却缺 `clickOn` 报 `MissingField`。`trigger` 不是 `clickOn` 却给了 `clickOn` 报 `InvalidField`。
没有 `<p:timing>` 时写出带 `bldLst` / `bldP` 的最小树：`click` / `with` / `after` 走 `mainSeq`；`clickOn` 走 `interactiveSeq`（`evt="onClick"` 指向 `clickOn` 的 `spid`）。
该宿主第一条 `trigger` 为 `with` / `after` 时，外层包装仍写 `delay="indefinite"`，并加 `evt="onBegin"` 指向 `mainSeq` 的 `cTn/@id`。
第一条 `click` 不加这条。`clickOn` 不加这条。
已有树则接到末尾：`click` 再挂一个并列包装组；`with` / `after` 接到现有最后一组里，空树时仍建包装组但 `nodeType` 为 `withEffect` / `afterEffect`；`clickOn` 再挂一个 `interactiveSeq`。
新 `cTn/@id` 取该宿主 timing 已有最大整数加 1。该对象尚无 `bldP` 则补一条对应 `spid`、`grpId="0"`。给了 `para` 才写 `txEl`。给了 `sound` 才写 `sndAc`，并可脏该宿主 `.rels`、`/ppt/media/…` 与 `[Content_Types].xml`。
不改 `<p:transition>`。不改未点名效果。
备注没有 notesSlide 时报 `IdNotFound`。母版 / 版式上占位符与装饰形都能绑。
备注母版与讲义占位符也能绑。ECMA `CT_NotesMaster` / `CT_HandoutMaster` 没有 `timing`；Office 16.0 另存不剥、无 Repair 则写。
`<p:timing>` 插在 `clrMap` 之后、`hf` / `notesStyle` 之前。不把这两处动画抄进各页。
docx / xlsx 上发此 op 报 `InvalidField`。

### setAnim

改该宿主名单上已有一条的 `type` / `trigger` / `delay` / `dur` / `repeat` / `sound` / `para` / `clickOn`。不换绑定对象。不接受整棵时间轴 JSON。

- **`op`**
  - 必须是 `"setAnim"`
- **`id`**
  - `slide:{n}/anim:{k}`
  - 或 `slide:{n}/notes/anim:{k}`
  - 或 `master:{n}/anim:{k}`
  - 或 `master:{n}/layout:{k}/anim:{m}`
- **`type`**
  - 可选
  - 规则与 `insertAnim` 的 `type` 相同
- **`trigger`**
  - 可选
  - `click` / `with` / `after` / `clickOn`
- **`delay`**
  - 可选
  - 规则与 `insertAnim` 的 `delay` 相同
- **`dur`**
  - 可选
  - 规则与 `insertAnim` 的 `dur` 相同
- **`repeat`**
  - 可选
  - 规则与 `insertAnim` 的 `repeat` 相同
  - `1` 表示去掉 `repeatCount`
- **`sound`**
  - 可选
  - 本地 WAV 路径，规则与 `insertAnim` 相同
  - `null` 表示摘掉该条 `sndAc`；无人再引用则删对应媒体部件
- **`para`**
  - 可选
  - 规则与 `insertAnim` 的 `para` 相同
  - `null` 表示去掉 `txEl`，改回绑整个对象
- **`clickOn`**
  - 把 `trigger` 改成 `clickOn` 时必须
  - 规则与 `insertAnim` 的 `clickOn` 相同
  - `trigger` 仍是 `clickOn` 时也可单独改单击对象

`type`、`trigger`、`delay`、`dur`、`repeat`、`sound`、`para`、`clickOn` 至少一个。都不给报 `InvalidField`。省略的字段不改。
找不到该条报 `IdNotFound`。不是动画 id 报 `InvalidId`。
`type` 不在白名单、`trigger` 不是四个之一、或 `delay` / `dur` / `repeat` 越界，报 `InvalidField`。
给了 `dur` 但该效果没有可写时长的行为报 `InvalidField`。
改 `type` 时按新模板重写该效果子树，保留该效果 `cTn/@id` 与未点名的 `delay` / `trigger` / `repeat` / `sndAc` / `txEl`。
改 `trigger` 时：第一条改为 `with` / `after` 则给外层包装补 `onBegin`；改为 `click` 则去掉这条。
非第一条改为 `click` 时抽成新的单击包装组；改为 `with` / `after` 则并进上一组。
改为 `clickOn` 时把该条挪进 `interactiveSeq`；从 `clickOn` 改为另外三个则挪回 `mainSeq`。
不改未点名效果。不改 `<p:transition>`。docx / xlsx 上发此 op 报 `InvalidField`。

### deleteAnim

摘掉该宿主名单上的一条效果。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteAnim"` |
| `id` | 与 inspect 名单相同的动画 id，从 1 计 |

找不到该宿主报 `IdNotFound`。序号不存在报 `IdNotFound`。不是动画 id 报 `InvalidId`。
摘掉对应效果 `cTn`，空壳按 `deleteShape` 清 `tgtEl` 同一套修剪；该 `spid` 没有剩下的效果则摘对应 `bldP`。整节空了则删 `<p:timing>`。删中间一条后面的序号要重排。
该条带 `sndAc` 且对应媒体无人再引用则删该部件与关系。
不改 `<p:transition>`。docx / xlsx 上发此 op 报 `InvalidField`。

### moveAnim

把该宿主名单上的一条效果挪到新的文档顺序。不改 `type`，不换绑定对象，不改 delay / dur。不接受整棵时间轴 JSON。

- **`op`**
  - 必须是 `"moveAnim"`
- **`id`**
  - 与 inspect 名单相同的动画 id
- **`index`**
  - 必须
  - 1 起的新位置，范围是该宿主当前条数

找不到该条报 `IdNotFound`。不是动画 id 报 `InvalidId`。
缺 `index`、不是整数、小于 1、或大于当前条数，报 `InvalidField`。
已经在目标位置则空操作，仍计入 `opsApplied`。
只重排该宿主 `<p:timing>` 里效果的文档顺序，使 inspect 名单第 `index` 条变成这条。
`click` / `with` / `after` 仍在 `mainSeq`；`clickOn` 仍在 `interactiveSeq`。跨这两种 trigger 挪动时按 `setAnim` 改 trigger 同一套搬家。
不改 `<p:transition>`。docx / xlsx 上发此 op 报 `InvalidField`。

### setSlideLayout

只改该页 `.rels` 里指向 `slideLayout` 的关系。不改幻灯片 XML，不删用户形状，不把版式占位符抄进页，不写 `<p:timing>`。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setSlideLayout"` |
| `id` | 必须是 `slide:{n}` |
| `layout` | 必须是 `master:{n}/layout:{k}` |

找不到该页报 `IdNotFound`。`id` 不是页 id 报 `InvalidId`。`layout` 形态不对报 `InvalidId`。找不到该版式报 `IdNotFound`。
缺 `layout` 报 `InvalidField`。已经是目标版式则空操作，仍计入 `opsApplied`。已有 `slideLayout` 关系则只改 `Target`；没有则补一条。
不改母版 XML、不改版式 XML、不改 `presentation.xml`。docx / xlsx 上发此 op 报 `InvalidField`。

### insertLayout

在已有母版上追加一份空白版式。不抄占位符，不算母版排版引擎。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "insertLayout",
      "id": "master:1",
      "name": "空白 2"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertLayout"` |
| `id` | 必须是 `master:{n}` |
| `name` | 可选。写入 `p:cSld/@name`。省略则为「空白」。空串报 `InvalidField` |

pptx：`id` 不是母版 id 报 `InvalidId`（对 `master:{n}/layout:{k}`、占位符 id、页 id 或 `pptx:presentation` 发本 op 也是这条）。
找不到该母版报 `IdNotFound`。写出新 slideLayout 部件：`type="blank"` `preserve="1"`，`p:clrMapOvr/a:masterClrMapping`，空 `p:spTree`。
版式 `.rels` 只指向该母版。母版 `.rels` 补一条 `slideLayout`；
母版 `p:sldLayoutIdLst` 追加一条，`id` 取包内已有 `sldMasterId` / `sldLayoutId` 的 `@id` 最大整数加 1（没有则从 `2147483648`）。
`[Content_Types].xml` 补 Override。不写 `a16:creationId`。不改各页 XML。docx / xlsx 上发此 op 报 `InvalidField`。

### deleteLayout

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteLayout"` |
| `id` | 必须是 `master:{n}/layout:{k}` |

找不到该版式报 `IdNotFound`。`id` 不是版式 id 报 `InvalidId`。任何页的 `layoutId` 指向它则报 `InvalidField`，未写文件。该母版只剩这一份版式报 `InvalidField`。
摘掉母版 `p:sldLayoutIdLst` 对应项、母版 `.rels` 对应关系、该 slideLayout 部件及其 `.rels`、内容类型 Override。不改各页 XML，不改未点名版式。docx / xlsx 上发此 op 报 `InvalidField`。

### insertPlaceholder

往母版或版式插入一个带 `p:ph` 与 `p:txBody` 的占位符。装饰形请用 `insertShape`。不写 theme `p:style`，不写 `a16:creationId`。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "insertPlaceholder",
      "id": "master:1/layout:2",
      "type": "body"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertPlaceholder"` |
| `id` | 必须是 `master:{n}`、`master:{n}/layout:{k}`、`notesMaster` 或 `handout` |
| `type` | 必填。`p:ph/@type`。必须是可写占位符类型 |
| `text` | 可选。可见纯文本，省略则空。`\\n` 写成 `a:br` |
| `x` / `y` | 可选。画布坐标，EMU，非负整数。必须成对出现或成对省略 |
| `cx` / `cy` | 可选。尺寸，EMU，正整数。必须成对出现或成对省略。都省略时默认 `5080000` × `2540000` |

pptx：`id` 不是母版 id、版式 id、`notesMaster` 或 `handout` 报 `InvalidId`（对占位符 id、页 id 发本 op 也是这条）。找不到该宿主报 `IdNotFound`。
缺 `type`、或 `type` 不在白名单，报 `InvalidField`。只给 `x` 或只给 `y`、只给 `cx` 或只给 `cy` 报 `InvalidField`。
缺坐标时在画布上找空位。幻灯片母版 / 版式 / 讲义画布取 presentation 的 `p:sldSz`，没有则 `12192000` × `6858000`。
备注母版画布取 `p:notesSz`，没有则 `6858000` × `9144000`。
`id` 为 `notesMaster` 且包内还没有 notesMaster 时，按 `setNotesText` 同一套规则新建最小部件再插。
`id` 为 `handout` 且包内还没有 handoutMaster 时，按 `setHandoutText` 同一套规则新建最小部件再插。

写入该宿主 `p:spTree` 一个新 `p:sp`（插在 `extLst` 之前）。`p:cNvSpPr` 带 `a:spLocks noGrp="1"`，不写 `txBox`。`a:prstGeom/@prst` 为 `rect`。
默认 `a:noFill` 与 `a:ln/a:noFill`。始终带 `p:txBody`。新 `cNvPr/@id` 取该宿主（含 `spTree` 的 `id="1"`）已有最大加 1，至少为 2。
`cNvPr/@name` 按类型写成「标题占位符 {该 id}」「居中标题占位符 {该 id}」「副标题占位符 {该 id}」「文本占位符 {该 id}」「日期占位符 {该 id}」「页脚占位符 {该 id}」「幻灯片编号占位符 {该 id}」「页眉占位符 {该 id}」。
`title` / `ctrTitle` 不写 `p:ph/@idx`。其它类型：该宿主已有整数 `@idx` 则取最大加 1；没有则写 `10`（与 Office 16.0 另存正文占位符一致）。
`body` 另写 `sz="quarter"`。不改各页 XML。docx / xlsx 上发此 op 报 `InvalidField`。

可写 `type`（ECMA-376 `ST_PlaceholderType` 子集，§19.7.10）：`title`、`body`、`ctrTitle`、`subTitle`、`dt`、`ftr`、`sldNum`、`hdr`。
不在名单里报 `InvalidField`。`obj` / `chart` / `tbl` / `pic` / `media` / `sldImg` 本期不写。

### deletePlaceholder

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deletePlaceholder"` |
| `id` | 已有占位符 id（`master:{n}/shape:…`、`master:{n}/layout:{k}/shape:…`、`notesMaster/shape:…` 或 `handout/shape:…`） |

找不到该占位符报 `IdNotFound`。id 不是占位符 id 报 `InvalidId`。对幻灯片形状 id 发本 op 报 `InvalidId`。只删该 `p:sp`。
若在 `mc:AlternateContent` 的 Choice 里，整段 `AlternateContent` 删掉。组合里的占位符同样摘掉；组合空了则把空 `p:grpSp` 一并摘掉。
不改各页 XML，不改未点名占位符的 `cNvPr/@id`。不把备注母版 / 讲义的删除抄进 notesSlide 或各页。
docx / xlsx 上发此 op 报 `InvalidField`。`deleteShape` 对母版 / 版式 / 备注母版 / 讲义占位符仍报 `InvalidId`。

### insertMaster

抄现有一份母版及其全部版式与 theme 部件，新开 master / layout / theme。不生成新的占位符几何，不算母版排版引擎。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "insertMaster",
      "id": "pptx:presentation"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertMaster"` |
| `id` | 必须是 `pptx:presentation` |
| `from` | 可选。要抄的母版 id，必须是 `master:{n}`。省略则抄 `master:1` |

pptx：`id` 不是 `pptx:presentation` 报 `InvalidId`。`from` 形态不对报 `InvalidId`。找不到源母版报 `IdNotFound`。包里没有母版可抄报 `IdNotFound`。
新 slideMaster 的 `p:sldLayoutIdLst` 按抄来的版式顺序重写，`id` 与 `insertLayout` 同一套递增。
presentation `.rels` 补一条 `slideMaster`；`p:sldMasterIdLst` 追加一条。新母版 `.rels` 指向自己的版式与自己的 theme 副本；每份新版式 `.rels` 指向新母版。
源母版上除 slideLayout / theme 以外的关系按原目标留下。`[Content_Types].xml` 补 Override。不改各页 XML，不改源母版 / 源版式。docx / xlsx 上发此 op 报 `InvalidField`。

### deleteMaster

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteMaster"` |
| `id` | 必须是 `master:{n}` |

找不到该母版报 `IdNotFound`。`id` 不是母版 id 报 `InvalidId`（对 `master:{n}/layout:{k}` 或占位符 id 也是这条）。最后一份母版报 `InvalidField`。
任何页仍使用该母版下的版式报 `InvalidField`，未写文件。摘掉 presentation 的 `p:sldMasterId` 与对应关系，
以及该母版、其全部版式、仅被它引用的 theme 部件及其 `.rels` 与内容类型 Override。
不改各页 XML。docx / xlsx 上发此 op 报 `InvalidField`。不删 notesMaster。

### setSlideSize

只改 `presentation.xml` 的 `p:sldSz`。不按新画布重排已有形状，不算排版引擎。不改 `p:notesSz`。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "setSlideSize",
      "id": "pptx:presentation",
      "cx": 9144000,
      "cy": 6858000
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setSlideSize"` |
| `id` | 必须是 `pptx:presentation` |
| `cx` / `cy` | 必填。画布宽 / 高，EMU 整数，范围 `914400`–`51206400`（ECMA-376 `ST_SlideSizeCoordinate`，§19.7.17） |
| `type` | 可选。`p:sldSz/@type`。必须是 `ST_SlideSizeType`（§19.7.18） |

pptx：`id` 不是 `pptx:presentation` 报 `InvalidId`。缺 `cx` 或 `cy`、只给一个、越界、或 `type` 不在白名单，报 `InvalidField`。
已有 `p:sldSz` 则改属性；没有则新建，插在 `sldIdLst` 之后、`notesSz` 之前。省略 `type` 不改该属性（新建时也不写 `type`）。不改各页 XML、不改母版 / 版式 / theme。
docx / xlsx 上发此 op 报 `InvalidField`。

可写 `type`：`screen4x3`、`letter`、`A4`、`35mm`、`overhead`、`banner`、`custom`、`ledger`、`A3`、`B4ISO`、`B5ISO`、`B4JIS`、`B5JIS`、
`hagakiCard`、`screen16x9`、`screen16x10`。

### setNotesSize

只改 `presentation.xml` 的 `p:notesSz`。不按新画布重排已有备注形状，不算排版引擎。不改 `p:sldSz`。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "setNotesSize",
      "id": "pptx:presentation",
      "cx": 9144000,
      "cy": 12192000
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setNotesSize"` |
| `id` | 必须是 `pptx:presentation` |
| `cx` / `cy` | 必填。备注页画布宽 / 高，EMU 整数，范围与 `setSlideSize` 相同 |

pptx：`id` 不是 `pptx:presentation` 报 `InvalidId`。缺 `cx` 或 `cy`、只给一个、或越界，报 `InvalidField`。
已有 `p:notesSz` 则改属性；没有则新建，插在 `sldSz` 之后。没有 `sldSz` 则插在 `sldIdLst` 之后。
不写 `type`。不改各页 XML、不改 notesSlide、不改母版 / 版式 / theme。
docx / xlsx 上发此 op 报 `InvalidField`。
示例见 `docs/examples/pptx-apply-notes-size.json`。

### setThemeColors

改该母版 theme 部件的 `a:clrScheme` 槽位，以及配色方案名、theme 根名。不改未点名的 `fmtScheme` 槽。不把未点名形状上的 `schemeClr` 展开成直接色。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "setThemeColors",
      "id": "master:1",
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
| `id` | 必须是 `master:{n}`、`notesMaster` 或 `handout` |
| `name` | 可选。`a:clrScheme/@name`。非空字符串；`null` 去掉该属性 |
| `themeName` | 可选。`a:theme/@name`。非空字符串；`null` 去掉该属性 |
| `dk1` / `lt1` / `dk2` / `lt2` / `accent1`–`accent6` / `hlink` / `folHlink` | 可选。六位 RGB 十六进制 |

pptx：`id` 不是 `master:{n}`、`notesMaster` 或 `handout` 报 `InvalidId`。对页 id 报 `InvalidId`。
找不到该 slideMaster / notesMaster / handoutMaster 报 `IdNotFound`。该宿主没有 theme 关系或部件不在包内报 `IdNotFound`。不新建母版，不新建 theme。
`name`、`themeName` 与十二个槽位一个都不给，或某槽位不是六位十六进制，或名字是空串，报 `InvalidField`。
写入时把被点名槽位的子元素换成 `a:srgbClr`（原先若是 `sysClr` 也换成 `srgbClr`）。未点名槽位留下。未点名的 `fmtScheme` 槽留下。不改各页 XML。
xlsx 上这些 pptx 宿主 id 报 `InvalidId`。docx 仍只认 `docx:document`。
xlsx 见 [xlsx 分册](json-contract-xlsx.md#setthemecolors)。docx 见 [docx 分册](json-contract-docx.md#setthemecolors)。
示例见 `docs/examples/pptx-apply-theme-remain.json`。备注 / 讲义宿主见 `docs/examples/pptx-apply-theme-notes-handout.json`。

### setThemeFonts

改该母版 theme 部件的 `a:fontScheme`。未点名的 `fmtScheme` 槽留下。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "setThemeFonts",
      "id": "master:1",
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
| `id` | 必须是 `master:{n}`、`notesMaster` 或 `handout` |
| `name` | 可选。`a:fontScheme/@name`。非空字符串；`null` 去掉该属性 |
| `majorLatin` / `minorLatin` / `majorEa` / `minorEa` / `majorCs` / `minorCs` | 可选。`typeface` 原文 |
| `majorScripts` / `minorScripts` | 可选。数组，整份替换对应组里的 `a:font`。`[]` 或 `null` 清掉全部 `a:font`。不改 `latin` / `ea` / `cs` |

pptx：`id` 不是 `master:{n}`、`notesMaster` 或 `handout` 报 `InvalidId`。对页 id 报 `InvalidId`。
找不到该宿主或 theme 部件报 `IdNotFound`。不新建。xlsx 上这些 pptx 宿主 id 报 `InvalidId`。docx 仍只认 `docx:document`。
`name`、六个字体槽与两份脚本列表一个都不给报 `InvalidField`。
`majorLatin` / `minorLatin` 空串报 `InvalidField`。`ea` / `cs` 允许空串。
脚本每一项必须是对象，字段只认 `script`、`typeface`。`script` 必须是非空字符串。`typeface` 必须是字符串，允许空串。
没有 `fontScheme` / `majorFont` / `minorFont` 或对应 `latin` / `ea` / `cs` 则按需补上。只改被点名槽的 `typeface`，其它属性（如 `panose`）留下。不改各页 XML。
xlsx 见 [xlsx 分册](json-contract-xlsx.md#setthemefonts)。docx 见 [docx 分册](json-contract-docx.md#setthemefonts)。

### setThemeFmt

改该母版 theme 部件 `a:fmtScheme` 的常用槽。不算主题排版引擎，不把 `phClr` 展开成 RGB。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "setThemeFmt",
      "id": "master:1",
      "line": { "index": 0, "w": 38100 },
      "fill": { "index": 0, "kind": "solid" }
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setThemeFmt"` |
| `id` | 必须是 `master:{n}`、`notesMaster` 或 `handout` |
| `line` | 可选。对象须有 `index`（0、1 或 2）与 `w`（非负整数，EMU） |
| `fill` | 可选。对象须有 `index`（0、1 或 2）与 `kind`（`solid` 或 `grad`） |
| `bgFill` | 可选。同 `fill`，改 `bgFillStyleLst` |
| `name` | 可选。`a:fmtScheme/@name`。非空字符串；`null` 去掉该属性 |

`line` / `fill` / `bgFill` / `name` 至少给一个。写入时只替换被点名槽：线改 `@w`；填充写成带 `schemeClr phClr` 的 `solidFill` 或两停点 `gradFill`。未点名槽留下。
`index` 超出该列表长度、缺少 `fmtScheme` 或对应列表，报 `InvalidField`。没有 theme 部件报 `IdNotFound`，不新建。
id 不是 `master:{n}`、`notesMaster` 或 `handout` 报 `InvalidId`。对页 id 报 `InvalidId`。
找不到该宿主报 `IdNotFound`。xlsx 上这些 pptx 宿主 id 报 `InvalidId`。docx 仍只认 `docx:document`。
xlsx 见 [xlsx 分册](json-contract-xlsx.md#setthemefmt)。docx 见 [docx 分册](json-contract-docx.md#setthemefmt)。
示例见 `docs/examples/pptx-apply-theme-remain.json`。

### setThemeExtraSchemes

整份替换该母版 theme 部件的 `a:extraClrSchemeLst`。不算主题排版引擎。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "setThemeExtraSchemes",
      "id": "master:1",
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
| `id` | 必须是 `master:{n}`、`notesMaster` 或 `handout` |
| `extraSchemes` | 必须。数组或 `null` |

`[]` 或 `null` 去掉 `extraClrSchemeLst`。否则按数组整份重写该列表。
每一项必须是对象，字段只认 `name`、`colors`。`name` 为非空字符串或 `null`（`null` 或省略则不写 `@name`）。
`colors` 规则与主 `clrScheme` 相同。项上 `name` 与 `colors.name` 都对应该项 `a:clrScheme/@name`；只给一个即可，两个都给则必须相同，否则报 `InvalidField`。
未点名的配色槽用 Office 默认十二槽补齐，再把点名槽写成 `a:srgbClr`。每项写出身份映射的 `a:clrMap`（`bg1=lt1` 那一套），不把映射暴露进 JSON。
pptx：`id` 不是 `master:{n}`、`notesMaster` 或 `handout` 报 `InvalidId`。对页 id 报 `InvalidId`。
没有该宿主或 theme 部件报 `IdNotFound`，不新建。xlsx 上这些 pptx 宿主 id 报 `InvalidId`。docx 仍只认 `docx:document`。
缺 `extraSchemes`、不是数组 / `null`、项不是对象、或槽位不是六位十六进制，报 `InvalidField`。
xlsx 见 [xlsx 分册](json-contract-xlsx.md#setthemeextraschemes)。docx 见 [docx 分册](json-contract-docx.md#setthemeextraschemes)。

### setClrMap

改 slideMaster、notesMaster 或 handoutMaster 上的 `p:clrMap`。不把映射算成 RGB。不改 theme 部件。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "setClrMap",
      "id": "master:1",
      "bg1": "dk1"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setClrMap"` |
| `id` | 必须是 `master:{n}`、`notesMaster` 或 `handout` |
| `bg1` / `tx1` / `bg2` / `tx2` / `accent1`–`accent6` / `hlink` / `folHlink` | 可选。theme 槽名 |

十二槽至少给一个。值为已有 theme 槽名：`dk1`、`lt1`、`dk2`、`lt2`、`accent1`–`accent6`、`hlink`、`folHlink`。
不认识的令牌报 `InvalidField`。未点名属性留下。
没有 `p:clrMap` 时按 Office 默认身份映射新建（`bg1=lt1` 那一套），再打补丁。
id 不是这三种报 `InvalidId`。找不到该母版 / 备注母版 / 讲义母版报 `IdNotFound`。不新建 notesMaster 或 handoutMaster。
docx / xlsx 上发此 op 报 `InvalidField`。
示例见 `docs/examples/pptx-apply-theme-clrmap.json`。

### setClrMapOvr

改幻灯片、版式或备注页上的 `p:clrMapOvr`。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "setClrMapOvr",
      "id": "slide:1",
      "clrMapOvr": "master"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setClrMapOvr"` |
| `id` | 必须是 `slide:{n}`、`slide:{n}/notes` 或 `master:{n}/layout:{k}` |
| `clrMapOvr` | 必须。`"master"` 或十二槽对象 |

`"master"` 写成 `a:masterClrMapping`，去掉 `a:overrideClrMapping`。
对象键与 `setClrMap` 相同，至少一槽。写成 `a:overrideClrMapping`；未点名槽用当前覆盖或默认身份映射补齐。
值为 theme 槽名，规则与 `setClrMap` 相同。
没有 `p:clrMapOvr` 时插在 `p:cSld` 之后。
id 不是这三种报 `InvalidId`。找不到该页 / 版式报 `IdNotFound`。该页没有 notesSlide 时报 `IdNotFound`，不新建备注页。
docx / xlsx 上发此 op 报 `InvalidField`。

### setThemeOverride

改该页或该备注页的 `themeOverride` 部件。根元素是 `a:themeOverride`，配色写在根下 `clrScheme`。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "setThemeOverride",
      "id": "slide:1",
      "accent1": "FF0000"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setThemeOverride"` |
| `id` | 必须是 `slide:{n}` 或 `slide:{n}/notes` |
| `enabled` | 可选。只能是 `false`：删掉该宿主的 themeOverride 关系；无人引用则删部件 |
| `dk1` / `lt1` / `dk2` / `lt2` / `accent1`–`accent6` / `hlink` / `folHlink` | 可选。六位 RGB 十六进制 |

`enabled: false` 与配色槽不能同时出现。否则至少给一个槽位。
没有部件时新建 `/ppt/theme/themeOverrideN.xml`，挂该宿主关系。有该页（或备注母版）能追到的 theme 时先抄它的 `clrScheme` 再打补丁。
id 不是页 id 或备注页 id 报 `InvalidId`。对 `master:{n}`、`notesMaster`、`handout` 报 `InvalidId`。
找不到该页报 `IdNotFound`。该页没有 notesSlide 时报 `IdNotFound`，不新建备注页。
docx 见 [docx 分册](json-contract-docx.md#setthemeoverride)。xlsx 见 [xlsx 分册](json-contract-xlsx.md#setthemeoverride)。

### setChartPos

字段与 `setPicturePos` 相同，`id` 为 pptx 图表 id。写入 graphicFrame 的 `p:xfrm/a:off`。docx / xlsx 上发此 op 报 `InvalidField`。

### setSmartArtPos

字段与 `setPicturePos` 相同，`id` 为 pptx 图示 id。写入 graphicFrame 的 `p:xfrm/a:off`。docx / xlsx 上发此 op 报 `InvalidField`。

### insertShape

往幻灯片、备注页、母版或版式插入一个带 `p:txBody` 的形状。缺省 `prst` 为矩形文本框。不抄版式占位符，不写 `<p:timing>`。
往母版 / 版式插时写出没有 `p:ph` 的装饰形，不把该 `p:sp` 抄进各页。插占位符请用 `insertPlaceholder`。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "insertShape",
      "id": "slide:1",
      "text": "新框",
      "x": 914400,
      "y": 914400
    }
  ]
}
```

- **`op`**
  - 必须是 `"insertShape"`
- **`id`**
  - 必须是 `slide:{n}`、`slide:{n}/notes`、`master:{n}` 或 `master:{n}/layout:{k}`
- **`text`**
  - 可选
  - 可见纯文本，省略则空
  - `\\n` 写成 `a:br`
- **`x` / `y`**
  - 可选
  - 画布坐标，EMU，非负整数
  - 必须成对出现或成对省略
- **`cx` / `cy`**
  - 可选
  - 尺寸，EMU，正整数
  - 必须成对出现或成对省略
  - 都省略时默认 `1828800` × `914400`（2 英寸 × 1 英寸）
- **`prst`**
  - 可选
  - `a:prstGeom/@prst`
  - 省略为 `rect`
  - 必须是可写预设名
  - 连接线几何名（`straightConnector1` 等）报 `InvalidField`，请用 `insertConnector`
  - 与 `geom` 同时出现报 `InvalidField`
- **`geom`**
  - 可选
  - 自定义路径，结构与 inspect 的 `geom` 相同
  - `paths` 必须非空
  - 每条路径必须有正整数 `w` / `h` 与非空 `ops`
  - 可写 `type` 只有 `moveTo`、`lnTo`、`close`
  - `moveTo` / `lnTo` 必须给非负整数 `x` / `y`
  - `close` 不要带 `x` / `y`
  - 其它 `type` 报 `InvalidField`
- **`sz` / `color` / `textFill` / `textOutline` / `bold` / `italic` / `underline` / `font` / `eastAsiaFont` / `align` / `textAnchor` / `spcBef` / `spcAft` / `lnSpc` / `spc` / `kern`**
  - 可选
  - 规则与 `setShapeFormat` 相同
  - 省略则缺省 18 磅黑字 Calibri / 宋体

pptx：`id` 不是页 id、备注容器 id、母版 id 或版式 id 报 `InvalidId`（对形状 id、图 id 或 `pptx:presentation` 发本 op 也是这条）。
找不到该页 / 该母版 / 该版式报 `IdNotFound`。
只给 `x` 或只给 `y`、只给 `cx` 或只给 `cy` 报 `InvalidField`。缺坐标时在画布上找空位（幻灯片与母版 / 版式读 `p:sldSz`，备注读 `p:notesSz`），不要叠在 `(0,0)`。
`id` 为 `slide:{n}/notes` 且该页还没有 notesSlide 时，按 `setNotesText` 同一套新建最小备注页再插入。

写入该宿主 `p:spTree` 一个新 `p:sp`（插在 `extLst` 之前）。
`prst` 为 `rect`（或缺省）且没有 `geom` 时：`p:cNvSpPr/@txBox` 为 `1`，默认 `a:noFill` 与 `a:ln/a:noFill`，
`cNvPr/@name` 写成「文本框 {该 id}」，写 `a:prstGeom`。
其它可写 `prst`：不写 `txBox`，默认实心填充 `5B9BD5` 与线宽 `12700` 的描边 `2E75B6`，
`cNvPr/@name` 写成「形状 {该 id}」，写 `a:prstGeom`。
给了 `geom` 时：不写 `txBox`，不写 `prstGeom`，填充描边与非矩形预设相同，
`cNvPr/@name` 写成「形状 {该 id}」。
`a:custGeom` 必须含空 `avLst` / `gdLst` / `ahLst` / `cxnLst`，`a:rect` 的 `l` / `t` / `r` / `b` 都是 `0`，
再写 `a:pathLst`。始终带 `p:txBody`。
给了文字格式字段则按 `setShapeFormat` 写入该 `txBody`。省略则缺省 18 磅黑字 Calibri / 宋体。
新 `cNvPr/@id` 取该宿主（含 `spTree` 的 `id="1"`）已有最大加 1，至少为 2。不写 `p:style`，不写 `a16:creationId`。不改未点名形状，不改已有 `<p:timing>`。
往母版 / 版式插入时不写 `p:ph`。
docx 上发此 op 报 `InvalidField`。xlsx 见 [xlsx 分册](json-contract-xlsx.md#insertshape)：`id` 为 `sheet:{name}`，必须给 `from`，出现 `x` / `y` 报 `InvalidField`。
xlsx 的 `insertShape` 出现 `geom` 报 `InvalidField`。

可写 `prst`（ECMA-376 `ST_ShapeType` 子集，§20.1.10.56）：`rect`、`roundRect`、`round1Rect`、`round2SameRect`、`round2DiagRect`、
`snipRoundRect`、`snip1Rect`、`snip2SameRect`、`snip2DiagRect`、`ellipse`、`triangle`、`rtTriangle`、`parallelogram`、
`trapezoid`、`diamond`、`pentagon`、`hexagon`、`heptagon`、`octagon`、`decagon`、`dodecagon`、`star4`、`star5`、`star6`、`star8`、
`star16`、`star24`、`star32`、`plus`、`plaque`、`teardrop`、`homePlate`、`chevron`、`pie`、`donut`、`noSmoking`、`blockArc`、
`rightArrow`、`leftArrow`、`upArrow`、`downArrow`、`leftRightArrow`、`upDownArrow`、`quadArrow`、`stripedRightArrow`、
`notchedRightArrow`、`bentUpArrow`、`cube`、`can`、`lightningBolt`、`heart`、`sun`、`moon`、`smileyFace`、`foldedCorner`、`bevel`、
`frame`、`cloud`、`ribbon`、`wave`。不在名单里报 `InvalidField`。

### insertConnector

往幻灯片、备注页、母版或版式插入一条连接线（`p:cxnSp`）。不算自动绕障碍。
省略 `start` / `end` 时不写 `a:stCxn` / `a:endCxn`。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "insertConnector",
      "id": "slide:1",
      "prst": "straightConnector1",
      "x": 635000,
      "y": 4064000,
      "start": { "id": "slide:1/shape:2", "site": 1 },
      "end": { "id": "slide:1/shape:3", "site": 1 }
    }
  ]
}
```

- **`op`**
  - 必须是 `"insertConnector"`
- **`id`**
  - 必须是 `slide:{n}`、`slide:{n}/notes`、`master:{n}` 或 `master:{n}/layout:{k}`
- **`prst`**
  - 可选
  - 缺省 `straightConnector1`
  - 允许
    - `straightConnector1`
    - `bentConnector2`
    - `bentConnector3`
    - `bentConnector4`
    - `bentConnector5`
    - `curvedConnector2`
    - `curvedConnector3`
    - `curvedConnector4`
    - `curvedConnector5`
  - xlsx 另允许 `line`
- **`x` / `y`**
  - 可选
  - 画布坐标，EMU，非负整数
  - 必须成对出现或成对省略
- **`cx` / `cy`**
  - 可选
  - 尺寸，EMU，正整数
  - 必须成对出现或成对省略
  - 都省略时默认 `1828800` × `762000`
- **`start` / `end`**
  - 可选
  - 对象，字段顺序：`id`、`site`
  - `id` 必须是同宿主形状 id 或连接线 id
  - `site` 可选，正整数，省略为 `1`
  - 写入 `a:stCxn` / `a:endCxn`：`id` 为目标 `cNvPr/@id`，`idx` 为 `site - 1`

pptx：`id` 不是页 id、备注容器 id、母版 id 或版式 id 报 `InvalidId`（对形状 id、连接线 id 或 `pptx:presentation` 也是这条）。找不到该页 / 该母版 / 该版式报 `IdNotFound`。
只给 `x` 或只给 `y`、只给 `cx` 或只给 `cy` 报 `InvalidField`。`prst` 不在名单里报 `InvalidField`。缺坐标时在画布上找空位（幻灯片与母版 / 版式读 `p:sldSz`，备注读 `p:notesSz`）。
`id` 为 `slide:{n}/notes` 且该页还没有 notesSlide 时，按 `setNotesText` 同一套新建最小备注页再插入。
对 `notesMaster` / `handout` 发本 op 报 `InvalidId`。
`groupShapes` 的 `id` 仍必须是 `slide:{n}`，不认备注与母版。
`start` / `end` 不是对象、缺 `id`、`site` 小于 1 报 `InvalidField`。
目标不在同一宿主报 `InvalidField`。找不到目标报 `IdNotFound`。
id 不是形状 id 也不是连接线 id 报 `InvalidId`。
xlsx / docx 上出现 `start` / `end` 报 `InvalidField`。
写入 `p:cxnSp`：`p:nvCxnSpPr` / `p:cNvCxnSpPr`，`a:prstGeom` 为该 `prst`，默认线宽 `12700` 的黑色描边，无填充、无 `p:style`、无 `txBody`。
给了 `start` 则写 `a:stCxn`，给了 `end` 则写 `a:endCxn`。都省略则 `cNvCxnSpPr` 为空。
新 `cNvPr/@id` 规则与 `insertShape` 相同。`cNvPr/@name` 写成「连接线 {该 id}」。docx 见 [docx 分册](json-contract-docx.md#insertconnector)。
xlsx 见 [xlsx 分册](json-contract-xlsx.md#insertconnector)。

### deleteConnector

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteConnector"` |
| `id` | 连接线 id（含 `slide:{n}/notes/cxn:…`、`master:{n}/cxn:…`、`master:{n}/layout:{k}/cxn:…`） |

找不到该连接线报 `IdNotFound`。id 不是连接线 id 报 `InvalidId`。删掉对应 `p:cxnSp`。
若在 `mc:AlternateContent` 的 Choice 里，整段 `AlternateContent` 删掉。删组合内连接线后若组合空了，把空 `p:grpSp` 一并摘掉。不改其它对象的 `cNvPr/@id`。
按被删连接线的 `cNvPr/@id` 清掉本页 `<p:timing>` 里对应的 `p:tgtEl`。docx 见 [docx 分册](json-contract-docx.md#deleteconnector)。
xlsx 见 [xlsx 分册](json-contract-xlsx.md#deleteconnector)。

### groupShapes

把同一页 `spTree` 上至少两个直接子对象包进一个新 `p:grpSp`。组合本身不进 inspect 树。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"groupShapes"` |
| `id` | 必须是 `slide:{n}` |
| `ids` | 必填。至少两个成员 id。每个必须是该页的形状 id 或连接线 id，且当前是 `spTree` 的直接子节点 |

成员少于 2 个、有重复、不在同一页、已经在某个 `p:grpSp` 里、或不是 `p:sp`（带 `txBody`）/ `p:cxnSp`，报 `InvalidField`。找不到某个成员报 `IdNotFound`。
`id` 不是页 id 报 `InvalidId`。新组合 `cNvPr/@id` 取该页最大加 1；`name` 写成「组合 {该 id}」。
组合 `xfrm` 的 `off` / `ext` / `chOff` / `chExt` 取成员包围盒；成员自己的 `a:off` 不改（与 Office 16.0 另存一致）。不写 `a16:creationId`。不重算已有组合框。
docx 见 [docx 分册](json-contract-docx.md#groupshapes)。xlsx 见 [xlsx 分册](json-contract-xlsx.md#groupshapes)。

### ungroup

解开某个成员所在的 `p:grpSp`，子节点抬回父节点。组合本身没有 inspect id。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"ungroup"` |
| `id` | 该组合内一个形状 id 或连接线 id |

找不到该对象报 `IdNotFound`。id 不是形状 id 也不是连接线 id 报 `InvalidId`。该对象不在 `p:grpSp` 里报 `InvalidField`。子节点 `cNvPr/@id` 与各自 `xfrm` 不变。
docx 见 [docx 分册](json-contract-docx.md#ungroup)。xlsx 见 [xlsx 分册](json-contract-xlsx.md#ungroup)。

### deleteShape

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteShape"` |
| `id` | 形状 id（`slide:{n}/shape:…`、备注上的 `slide:{n}/notes/shape:…`，或母版 / 版式装饰形 `master:…/shape:…`） |

找不到该形状报 `IdNotFound`。id 不是形状 id 报 `InvalidId`。对母版、版式、备注母版或讲义占位符 id 发本 op 报 `InvalidId`。
插删占位符请用 `insertPlaceholder` / `deletePlaceholder`。对备注页、备注母版或讲义上占位符 `p:ph/@type` 为 `sldImg` 的形状发本 op 报 `InvalidField`，不写文件。
删掉对应 `p:sp`。若在 `mc:AlternateContent` 的 Choice 里，整段 `AlternateContent` 删掉。组合本身不进树；删组合内子 `sp` 后若组合空了，把空 `p:grpSp` 一并摘掉。
不改其它形状的 `cNvPr/@id`。按被删形状的 `cNvPr/@id` 清掉本宿主 `<p:timing>` 里对应的 `p:tgtEl`（其子为 `p:spTgt/@spid`）；祖先因此变空则一并摘掉，未点名形状的 timing 留下。
不要整节无故删掉仍有内容的 `<p:timing>`。母版 / 版式装饰形只改对应 slideMaster / slideLayout 部件，不改各页 XML。docx 上发此 op 报 `InvalidField`。xlsx 见 [xlsx 分册](json-contract-xlsx.md#deleteshape)。

### setShapePos

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setShapePos"` |
| `id` | 形状 id（幻灯片 / 备注上的 `slide:…/shape:…`，母版 / 版式占位符或装饰形 `master:…/shape:…`，备注母版 `notesMaster/shape:…`，讲义 `handout/shape:…`，或连接线 `slide:…/cxn:…`） |
| `x` / `y` | 至少给一个。EMU，非负整数 |

写入该 `p:sp` 或 `p:cxnSp` 的 `p:spPr/a:xfrm/a:off`。没有 `xfrm` 则补。找不到报 `IdNotFound`。id 不是形状 id 也不是连接线 id 报 `InvalidId`。
`x` 与 `y` 一个都不给报 `InvalidField`。对 `sldImg` 占位符报 `InvalidField`。母版 / 版式占位符与装饰形只改对应 slideMaster / slideLayout 部件，不改各页 XML。
备注母版只改 notesMaster 部件。讲义只改 handoutMaster 部件。
对已组合的子对象只改该子 `xfrm`，不重算父 `p:grpSp` 的包围盒。docx / xlsx 上发此 op 报 `InvalidField`。

### setShapeSize

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setShapeSize"` |
| `id` | 形状 id（幻灯片 / 备注上的 `slide:…/shape:…`，母版 / 版式占位符或装饰形 `master:…/shape:…`，备注母版 `notesMaster/shape:…`，讲义 `handout/shape:…`，或连接线 `slide:…/cxn:…`） |
| `cx` / `cy` | 至少给一个。EMU，正整数 |

写入该 `p:sp` 或 `p:cxnSp` 的 `p:spPr/a:xfrm/a:ext`。没有 `xfrm` 则补。找不到报 `IdNotFound`。id 不是形状 id 也不是连接线 id 报 `InvalidId`。
`cx` 与 `cy` 一个都不给报 `InvalidField`。对 `sldImg` 占位符报 `InvalidField`。母版 / 版式占位符与装饰形只改对应 slideMaster / slideLayout 部件。
备注母版只改 notesMaster 部件。讲义只改 handoutMaster 部件。
对已组合的子对象只改该子 `xfrm`，不重算父组合框。docx / xlsx 上发此 op 报 `InvalidField`。

### setShapeFill

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setShapeFill"` |
| `id` | 形状 id |
| `color` | 必填。六位 RGB 十六进制，或 `null` 表示无填充。不写 theme 色 |

写入 `p:spPr` 的 `a:solidFill/a:srgbClr`；`null` 则写成 `a:noFill`，并去掉原有填充。缺 `color` 或非法十六进制报 `InvalidField`。找不到报 `IdNotFound`。id 不是形状 id 报 `InvalidId`。对母版或版式占位符 id 发本 op 报 `InvalidId`。对 `sldImg` 占位符报 `InvalidField`。
母版 / 版式装饰形只改对应 slideMaster / slideLayout 部件，不改各页 XML。
docx / xlsx 上发此 op 报 `InvalidField`。pptx 上发 `setTextBoxFill` 仍报 `InvalidField`。

### setShapeLine

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setShapeLine"` |
| `id` | 形状 id 或连接线 id |
| `color` | 必填。六位 RGB 十六进制，或 `null` 表示无边框 |
| `width` | 可选。线宽，EMU，正整数。无边框时必须省略 |

写入 `p:spPr/a:ln`。`color` 为 `null` 时写成 `a:ln/a:noFill`；此时出现 `width` 报 `InvalidField`。缺 `color` 或非法十六进制报 `InvalidField`。
找不到报 `IdNotFound`。id 不是形状 id 也不是连接线 id 报 `InvalidId`。对母版或版式占位符 id 发本 op 报 `InvalidId`。对 `sldImg` 占位符报 `InvalidField`。
母版 / 版式装饰形只改对应 slideMaster / slideLayout 部件，不改各页 XML。
docx / xlsx 上发此 op 报 `InvalidField`。pptx 上发 `setTextBoxLine` 仍报 `InvalidField`。

不给无 `txBody` 的旧 `p:sp` 补框。不重算组合框几何。

### setShapeGeom

把带 `p:txBody` 的 `p:sp` 的 `a:prstGeom` 或已有 `a:custGeom` 整份换成自定义路径。不重算组合框。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "setShapeGeom",
      "id": "slide:1/shape:4",
      "paths": [
        {
          "w": 1905001,
          "h": 1270001,
          "ops": [
            { "type": "moveTo", "x": 0, "y": 0 },
            { "type": "lnTo", "x": 1905000, "y": 0 },
            { "type": "lnTo", "x": 1905000, "y": 1270000 },
            { "type": "lnTo", "x": 0, "y": 1270000 }
          ]
        }
      ]
    }
  ]
}
```

- **`op`**
  - 必须是 `"setShapeGeom"`
- **`id`**
  - 必须是形状 id（幻灯片 / 备注 `slide:…/shape:…`，或母版 / 版式装饰形 `master:…/shape:…`）
- **`paths`**
  - 必填
  - 非空数组
  - 规则与 `insertShape` 的 `geom.paths` 相同

找不到报 `IdNotFound`。id 不是形状 id 报 `InvalidId`（对连接线 id、图 id 也是这条）。
对母版或版式占位符、备注母版、讲义占位符发本 op 报 `InvalidId`。对 `sldImg` 占位符报 `InvalidField`。
缺 `paths`、空数组、缺 `w` / `h`、空 `ops`、不可写 `type` 报 `InvalidField`。
写入完整 `a:custGeom` 骨架（空 `avLst` / `gdLst` / `ahLst` / `cxnLst`，`rect` 全 0，再写 `pathLst`），去掉原 `prstGeom`。
不写 `p:style`，不写 `a16:creationId`。不改填充描边，不改 `txBody`。
母版 / 版式装饰形只改对应 slideMaster / slideLayout 部件。
xlsx / docx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/pptx-apply-custgeom.json`。

### setConnector

改一条已有连接线的起点 / 终点绑定。不算自动绕障碍。不改画布坐标。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "setConnector",
      "id": "slide:1/cxn:6",
      "start": { "id": "slide:1/shape:2", "site": 1 },
      "end": { "id": "slide:1/shape:5", "site": 1 }
    }
  ]
}
```

- **`op`**
  - 必须是 `"setConnector"`
- **`id`**
  - 必须是连接线 id
- **`start` / `end`**
  - 至少给一个
  - 规则与 `insertConnector` 相同
  - 未点名的一端留下

找不到该连接线报 `IdNotFound`。id 不是连接线 id 报 `InvalidId`。
`start` 与 `end` 一个都不给报 `InvalidField`。目标不在同一宿主报 `InvalidField`。
找不到目标报 `IdNotFound`。目标 id 不是形状 id 也不是连接线 id 报 `InvalidId`。
`site` 小于 1 报 `InvalidField`。
写入或替换 `p:cNvCxnSpPr` 下的 `a:stCxn` / `a:endCxn`。不删未点名的那一端。
xlsx / docx 上发此 op 报 `InvalidField`。

### setTableCellText

改幻灯片 DrawingML 表某一格的可见文字。手术刀规则与 `setShapeText` 相同，目标是该 `a:tc` 的 `a:txBody`，不整格重建。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTableCellText"` |
| `id` | 单元格 id，`slide:{n}/table:…/r:{r}/c:{c}` |
| `text` | 新的可见纯文本，字符串，可空。必填。`\\n` 写成 `a:br` |

找不到该格报 `IdNotFound`。id 不是单元格 id 报 `InvalidId`。缺 `text` 报 `InvalidField`。没有 `a:txBody` 时报 `IdNotFound`，不补框。
docx / xlsx 上发此 op 报 `InvalidField`。xlsx 上改工作表格子请用 `setCell`。不要对 pptx 表发 `setShapeText`。

### setTableCellFill

改某一格 `a:tcPr` 的填充（`EG_FillProperties`）。不要用 `setShapeFill`。不要用 docx 的 `setCellFormat`。
theme 色仍不展开成 RGB。备注表、母版表同样认。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTableCellFill"` |
| `id` | 单元格 id |
| `fill` | 填充。对象、六位 RGB 字符串，或 `null`（写成 `a:noFill`） |
| `color` | 44 的写法。六位 RGB 或 `null`。等价于 `fill` 为该字符串或 `null` |
| `file` | `type` 为 `blip` 时必须。本地图片路径 |

`fill` 与 `color` 必须恰好给一个，否则报 `InvalidField`。
`fill` 为字符串时当作 `{ "type": "solid", "color": 该串 }`。
`fill` 为对象时必须有 `type`：`solid` / `grad` / `patt` / `blip`。`other` 只出现在 inspect，写入报 `InvalidField`。
`solid` 必须有 `color`（六位 RGB）。`grad` 必须有 `stops`（至少两项，每项 `pos` 与 `color`），规则与字渐变相同；可选 `ang`（0–21600000），省略则写 `5400000`。
写入 `a:gradFill` 骨架与 C10 相同：`flip="none"` `rotWithShape="1"`，`a:lin scaled="1"`，空 `a:tileRect`。
`patt` 必须有 `prst`、`fg`、`bg`。`prst` 必须是 `ST_PresetPatternVal`（§20.1.10.51）原文。`fg` / `bg` 为六位 RGB。
写入 `a:pattFill/@prst` 与 `fgClr` / `bgClr` 的 `srgbClr`。
`blip` 必须有 `file`，不得给 `rId`。空 `file`、读不到、或不是该格式认的图片报与 `insertPicture` 相同的错。
写入 `a:blipFill/a:blip/@r:embed` 与 `a:stretch/a:fillRect`。已有 `r:embed` 则换该媒体部件字节；没有则新建 `/ppt/media/…` 与宿主 image 关系。
`file` 出现在非 `blip` 上报 `InvalidField`。
写入前去掉其它填充子元。换成非 `blip` 时不删无人引用的媒体部件。
找不到该格报 `IdNotFound`。id 不是单元格 id 报 `InvalidId`。xlsx / docx 上发此 op 报 `InvalidField`。
`exportMedia` 不认单元格 id。
示例见 `docs/examples/pptx-apply-table-cell-fill.json` 与 `docs/examples/pptx-apply-table-cell-fill-ex.json`。

### setTableCellBorders

改某一格 `a:tcPr` 的六边。未点名的边留下。不要用 `setShapeLine`。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTableCellBorders"` |
| `id` | 单元格 id |
| `left` / `right` / `top` / `bottom` / `tl2br` / `bl2tr` | 可选。边对象或 `null`（去掉该 `ln*`） |

边对象必须有 `color`（六位 RGB）与 `width`（EMU 正整数）。可选 `dash`（`ST_PresetLineDashVal` 原文，或 `null` 去掉 `prstDash`）。
六个边键至少一个，否则报 `InvalidField`。写入 `a:solidFill/a:srgbClr` 与 `@w`。不写箭头、`miter`。
找不到该格报 `IdNotFound`。id 不是单元格 id 报 `InvalidId`。xlsx / docx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/pptx-apply-table-cell-fill.json`。

### setTableCellInset

改某一格 `a:tcPr` 的 `marT` / `marL` / `marB` / `marR`。不要用 docx 的 `setTableCellMar` 或 `setCellFormat`。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTableCellInset"` |
| `id` | 单元格 id |
| `top` / `left` / `bottom` / `right` | 可选。EMU 非负整数，或 `null`（去掉该属性） |

四个边键至少一个，否则报 `InvalidField`。负数报 `InvalidField`。
找不到该格报 `IdNotFound`。id 不是单元格 id 报 `InvalidId`。xlsx / docx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/pptx-apply-table-cell-mar.json`。

### setTableCellPr

改某一格 `a:tcPr` 上 44 / 45 / 50 未覆盖的属性与子元：`vert`、`anchorCtr`、`horzOverflow`、`cell3D`、`headers`。
不要改 C9 的 `textAnchor`。不要写 `tcPr/@anchor`。不要用 `setShapeFormat`。备注表、母版表同样认。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTableCellPr"` |
| `id` | 单元格 id |
| `vert` | 可选。`ST_TextVerticalType` 原文，或 `null`（去掉 `@vert`） |
| `anchorCtr` | 可选。布尔，或 `null`（去掉 `@anchorCtr`） |
| `horzOverflow` | 可选。`overflow` / `clip`，或 `null`（去掉 `@horzOverflow`） |
| `cell3D` | 可选。对象或 `null`（去掉 `a:cell3D`） |
| `headers` | 可选。字符串数组，或 `null`（去掉 `a:headers`） |

`vert` / `anchorCtr` / `horzOverflow` / `cell3D` / `headers` 至少给一个，否则报 `InvalidField`。
`vert` 不是 `ST_TextVerticalType` 报 `InvalidField`。
`horzOverflow` 不是 `overflow` / `clip` 报 `InvalidField`。
`cell3D` 对象字段顺序：`prstMaterial`、`bevel`。都可省略。
`prstMaterial` 必须是 `ST_PresetMaterialType` 原文。
`bevel` 为对象时字段顺序：`prst`、`w`、`h`，都可省略；写入缺省与 xsd 相同（`prst=circle`，`w` / `h` 为 `76200`）。
`prst` 必须是 `ST_BevelPresetType` 原文。`w` / `h` 必须是 EMU 正整数。
写入 `a:cell3D` 时整份替换该子元（不保留已有 `lightRig`）。没有 `bevel` 时仍写一个带缺省的 `a:bevel`（xsd 必有）。
`headers` 数组整份替换 `a:headers`；空数组写成空 `a:headers`。
找不到该格报 `IdNotFound`。id 不是单元格 id 报 `InvalidId`。xlsx / docx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/pptx-apply-table-cell-pr.json`。

### setTablePos

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTablePos"` |
| `id` | 表 id |
| `x` / `y` | 至少给一个。EMU，非负整数 |

写入该 `p:graphicFrame` 的 `p:xfrm/a:off`。没有 `xfrm` 则补。只改画布坐标，不重分配 `gridCol/@w` 与 `tr/@h`。找不到报表 `IdNotFound`。
id 不是表 id 报 `InvalidId`。`x` 与 `y` 一个都不给报 `InvalidField`。docx / xlsx 上发此 op 报 `InvalidField`。

### setTableSize

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTableSize"` |
| `id` | 表 id |
| `cx` / `cy` | 至少给一个。EMU，正整数 |

写入该 `p:graphicFrame` 的 `p:xfrm/a:ext`。没有 `xfrm` 则补。只改框尺寸，不算自动列宽，不重分配 `a:gridCol/@w` 与 `a:tr/@h`。找不到报表 `IdNotFound`。id 不是表 id 报 `InvalidId`。
`cx` 与 `cy` 一个都不给报 `InvalidField`。docx / xlsx 上发此 op 报 `InvalidField`。

### setTableColWidths

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTableColWidths"` |
| `id` | 表 id |
| `cols` | 必须。EMU 正整数数组，长度必须等于该表 `colCount` |

只写 `a:tblGrid` 的 `a:gridCol/@w`。没有网格则补 `a:tblGrid`。不改 `p:xfrm`。不改各格 `tcW`。找不到报表 `IdNotFound`。id 不是表 id 报 `InvalidId`。
缺 `cols`、不是数组、长度对不上 `colCount`、或某项不是正整数报 `InvalidField`。docx / xlsx 上发此 op 报 `InvalidField`。
不要复用 Word 的 `setTableWidth`（单位是 twip）。不要复用 xlsx 的 `setColWidth`。

### setTableTrHeight

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTableTrHeight"` |
| `id` | 行 id |
| `h` | 必须。EMU 正整数 |

只改该行 `@h`。不改框 `p:xfrm`。找不到该行报 `IdNotFound`。id 不是行 id 报 `InvalidId`。缺 `h`、或不是正整数报 `InvalidField`。
`a:tr/@h` 必填，`h: null` 报 `InvalidField`。inspect 缺 `@h` 仍报 `null`。
docx / xlsx 上发此 op 报 `InvalidField`。不要复用 Word 的 `setTableRowHeight`（单位是 twip）。不要复用 xlsx 的 `setRowHeight`。

### setNotesText

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "setNotesText",
      "id": "slide:1",
      "text": "演讲者备注"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setNotesText"` |
| `id` | `slide:{n}` 或 `slide:{n}/notes` |
| `text` | 新的演讲者备注可见纯文本，字符串，可空。必填。换行规则与 `setShapeText` 相同 |

只改备注页上占位符 `p:ph/@type` 为 `body` 的那个文本框。文档顺序中第一个 `body` 占位符命中。找不到该页报 `IdNotFound`。`id` 不是上述两种报 `InvalidId`。
没有 notesSlide 时新建最小 Transitional 备注页：若包内还没有 notesMaster，
则补 `/ppt/notesMasters/notesMaster1.xml`、presentation 关系、`p:notesMasterIdLst` 与内容类型；
theme 部件按已有 theme 复制一份新的 `/ppt/theme/themeN.xml`，不和 slideMaster 共用同一部件（PowerPoint 16.0 拒开共用 theme 的备注母版，
规则与 `setHandoutText` 的 handoutMaster 相同）；
再写 `/ppt/notesSlides/notesSlideN.xml`，该页 `.rels` 指向 notesMaster 与对应幻灯片。
新建的备注页含 `sldImg`、`body`、`sldNum` 三个占位符，结构对照 PowerPoint 16.0 另存样例，不写 `p14:creationId`。
已有备注页却没有 `body` 占位符时，补一个 `body` 再写字。不改 `sldImg` 缩略图占位符。xlsx / docx 上发此 op 报 `InvalidField`。
`create` 给了 `slides[].notes` 时按同一套规则写演讲者备注。

示例见 `docs/examples/pptx-apply-notes.json`。inspect 示例见 `docs/examples/pptx-inspect-notes.json`。
批注示例见 `docs/examples/pptx-apply-comments.json` 与 `docs/examples/pptx-inspect-comments.json`。

### setHandoutText

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "setHandoutText",
      "id": "handout",
      "footer": "讲义页脚"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setHandoutText"` |
| `id` | 必须是 `handout` |
| `header` | 可选。页眉占位符（`p:ph/@type` 为 `hdr`）可见字 |
| `footer` | 可选。页脚占位符（`ftr`）可见字 |
| `date` | 可选。日期占位符（`dt`）可见字 |
| `sldNum` | 可选。页码占位符（`sldNum`）可见字 |

四个槽位至少给一个，否则报 `InvalidField`。未点名的槽位留下。`id` 不是 `handout` 报 `InvalidId`。
不要和 `setNotesText` 混用：本 op 只改 `/ppt/handoutMasters/…`，不改 notesSlide。
对 `handout` 容器发 `setShapeText` / `setShapeFormat` / `insertShape` / `deleteShape` / `setShapeFill` / `setShapeLine` / `setShapeGeom` 报 `InvalidId`。
对 `handout/shape:…` 发 `setShapeText` / `setShapeFormat` / `setShapePos` / `setShapeSize` 认该占位符。
`sldImg` 仍报 `InvalidField`。`insertShape` / `deleteShape` / `setShapeFill` / `setShapeLine` / `setShapeGeom` 对讲义占位符仍报 `InvalidId`。

只改文档顺序中该类型的第一个占位符。没有 handoutMaster 时新建最小 Transitional 讲义母版：写 `/ppt/handoutMasters/handoutMaster1.xml`，
presentation 关系、`p:handoutMasterIdLst`（插在 `notesMasterIdLst` 之后，没有则插在 `sldMasterIdLst` 之后）与内容类型。
theme 关系指向**新复制**的 theme 部件（内容与包内已有 theme 相同），不和 slideMaster 共用同一部件。
新建部件含 `hdr`、`dt`、`ftr`、`sldNum` 四个占位符，结构对照 notesMaster 的页眉页脚框，不写 `sldImg`，不写 `p14:creationId`。
已有讲义母版却没有被点名类型的占位符时，补一个该类型再写字。写入规则与 `setShapeText` 相同（整框写成新 run）；日期或页码原先若是 `a:fld`，改完变成普通文字，不再自动更新。
该 handoutMaster 若已有 `p:hf`，把被点名槽位对应属性写成 `1`。本 op 不写 `<p:timing>`。xlsx / docx 上发此 op 报 `InvalidField`。
`create` 给了 `presentation.handout` 时复用本 op。

示例见 `docs/examples/pptx-apply-handout.json`。inspect 示例见 `docs/examples/pptx-inspect-handout.json`。

### insertVideo

往幻灯片、备注页或幻灯片母版 / 版式插入一段内嵌视频。不解码、不抽帧。不写 `<p:timing>`。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "insertVideo",
      "id": "slide:1",
      "file": "C:\\clip\\intro.mp4"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertVideo"` |
| `id` | 必须是 `slide:{n}`、`slide:{n}/notes`、`master:{n}` 或 `master:{n}/layout:{k}` |
| `file` | 本地路径。按文件头认 MP4（`ftyp`），写出 `/ppt/media/mediaN.mp4`，内容类型 `video/mp4` |
| `x` / `y` | 可选。EMU，非负整数。必须成对出现或成对省略；只给一个时报 `InvalidField` |
| `cx` / `cy` | 可选。EMU，正整数。必须成对；只给一个时报 `InvalidField` |
| `st` / `end` | 可选。从片头 / 片尾裁掉的毫秒，非负数字。省略则不写 `<p14:trim>` |
| `bookmarks` | 可选。书签数组。省略则不写 `<p14:bmkLst>` |
| `tracks` | 可选。字幕轨数组。省略则不写 `tracksInfo` |
| `trackDisplayLoc` | 可选。只在给了 `tracks` 时出现。`media` 或 `slide`。省略为 `media` |

找不到该页 / 该母版 / 该版式报 `IdNotFound`。`id` 不是合法宿主报 `InvalidId`。缺 `file` 报 `MissingField`。文件不是 MP4 报 `InvalidField`。
`kind` / `wrap` / `from` / `to` / `anchor` 出现报 `InvalidField`。缺尺寸则用 `3048000` × `2286000`。
缺坐标则放到该宿主空白处，规则与 pptx `insertPicture` 相同。
`id` 为 `slide:{n}/notes` 且该页还没有 notesSlide 时，按 `setNotesText` 同一套新建最小备注页再插入。
对 `notesMaster` / `handout` 发本 op 报 `InvalidId`。
写入该页 `p:spTree` 一个新的 `p:pic`：`a:hlinkClick` 的 `action` 为 `ppaction://media`；
`p:nvPr` 写 `a:videoFile/@r:link` 与 `p14:media/@r:embed`（uri `{DAA4B4D4-6D71-4841-9C94-3DE7FCFB9230}`，
命名空间 `http://schemas.microsoft.com/office/powerpoint/2010/main`）。
该页 `.rels` 补三条关系：MS-PPTX media（Type `http://schemas.microsoft.com/office/2007/relationships/media`）、ISO video（Type 以 `/video` 结尾），
两者 `Target` 指向同一 `mediaN.mp4`；
再补一条 image 关系指向库内生成的海报 PNG（`/ppt/media/imageN.png`）。
`p:cNvPr/@id` 规则与 `insertPicture` 相同。
`st` / `end` 不是非负数字报 `InvalidField`。两者都省略或都为 `0` 则不写 `<p14:trim>`；否则写 `<p14:trim>`，缺的一侧按 `0`。
`bookmarks` 不是数组报 `InvalidField`。空数组不写 `bmkLst`。
每条必须是对象，字段只许 `name`、`time`。`name` 必须是非空字符串，同一列表内不得重复。
`time` 必须是非负数字，相对媒体文件开头。写入 `p14:bmkLst/p14:bmk`，顺序与数组相同。
`tracks` 不是数组报 `InvalidField`。空数组不写 `tracksInfo`。
每条必须是对象，字段只许 `id`、`label`、`lang`、`file`。
`label` 必须是非空字符串。`file` 必须是本地 WebVTT 路径，按文件头认 `WEBVTT`（可带 UTF-8 BOM），写出 `/ppt/media/mediaN.vtt`，内容类型 `text/vtt`。
该宿主 `.rels` 再补 Type `http://schemas.microsoft.com/office/2017/04/relationships/track`（MS-PPTX §2.1.3）。
`id` 省略则库生成带花括号的 GUID；给了必须是带花括号的 GUID。`lang` 可选非空字符串。
`trackDisplayLoc` 不是 `media` / `slide`，或没给 `tracks` 却给了它，报 `InvalidField`。
字幕写在 `p14:media/p14:extLst/p14:ext[@uri="{3AFAAA56-56D3-431D-BCD4-E75A35582382}"]/p174:tracksInfo`，
`displayLoc` 按上面的值，`track/@r:embed` 指向该 VTT。只写内嵌，不写 `r:link`。
不写 `<p14:fade>`。不写 `<p:timing>`。xlsx / docx 上发此 op 报 `InvalidField`。

### insertAudio

往幻灯片、备注页或幻灯片母版 / 版式插入一段内嵌音频。字段与 `insertVideo` 相同，`op` 为 `"insertAudio"`，`file` 按文件头认 WAV（`RIFF`…`WAVE`），写出 `/ppt/media/mediaN.wav`，内容类型 `audio/x-wav`。
`p:nvPr` 写 `a:audioFile/@r:link` 与 `p14:media/@r:embed`。ISO 关系 Type 以 `/audio` 结尾。缺尺寸则用 `609600` × `609600`。
其余规则与 `insertVideo` 相同。文件不是 WAV 报 `InvalidField`。xlsx / docx 上发此 op 报 `InvalidField`。

### setMedia

改已有内嵌视频 / 音频的裁剪、书签与字幕轨。不换媒体文件。不写 `<p:timing>`。不写 `<p14:fade>`。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setMedia"` |
| `id` | 视频 id 或音频 id |
| `st` / `end` | 可选。从片头 / 片尾裁掉的毫秒，非负数字 |
| `bookmarks` | 可选。整份替换书签数组。`[]` 或 `null` 去掉 `bmkLst` |
| `tracks` | 可选。整份替换字幕轨。`[]` 或 `null` 去掉 `tracksInfo` 与不再引用的 VTT |
| `trackDisplayLoc` | 可选。只在给了 `tracks` 且不是清掉时出现。`media` 或 `slide` |

找不到该对象报 `IdNotFound`。`id` 不是视频 id 也不是音频 id 报 `InvalidId`。
`st` / `end` / `bookmarks` / `tracks` 一个都不给报 `InvalidField`。
`st` / `end` 省略则留下现有 trim 的那一侧；两侧写完后都是 `0` 则去掉 `<p14:trim>`。
`bookmarks` / `tracks` 省略则留下现有名单。校验规则与 `insertVideo` 相同。
给了 `tracks` 且不是清掉时，每条必须带 `file`（不按已有 GUID 保留旧字节）。
`trackDisplayLoc` 省略则沿用原 `tracksInfo/@displayLoc`，再没有则为 `media`。
没有 `p14:media` 时补上：复用已有 media 关系；没有则按现有 `videoFile` / `audioFile` 目标补一条 media 关系。
原有 `<p14:fade>` 原样留。其它 `p14:ext` 原样留。xlsx / docx 上发此 op 报 `InvalidField`。

### deleteMedia

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteMedia"` |
| `id` | 视频 id 或音频 id |

找不到该对象报 `IdNotFound`。`id` 不是视频 id 也不是音频 id 报 `InvalidId`。摘掉对应 `p:pic`。
该 `p:pic` 上的 media / video / audio / 海报 image 关系若本页 XML 里无人引用则删除；对应 `/ppt/media/…` 若重新解析后无人引用则删掉。
清掉 `<p:timing>` 里对应该 `cNvPr/@id` 的 `p:tgtEl`，规则与 `deleteShape` 相同；未点名对象的 timing 留下，不要整节无故删掉 `<p:timing>`。不改未点名的 `p:pic`。
xlsx / docx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/pptx-apply-media.json`。inspect 示例见 `docs/examples/pptx-inspect-media.json`。

### insertSlideSection

在 `p:sldIdLst` 之上写入 `p14:sectionLst` 分组。不改页部件。不写节的 UI 颜色。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "insertSlideSection",
      "id": "pptx:presentation",
      "name": "内容",
      "start": "slide:2"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertSlideSection"` |
| `id` | 必须是 `pptx:presentation` |
| `name` | 新节显示名，字符串。缺省报 `MissingField`。允许空串 |
| `start` | 可选。`slide:{n}`。新节从该页起，把该页及原所在节里其后的页划进新节 |
| `after` | 可选。已有 `section:{n}`。新节插到该节之后，不带走任何页 |

`id` 不是 `pptx:presentation` 报 `InvalidId`。`start` 与 `after` 同时出现报 `InvalidField`。
`start` 不是页 id 报 `InvalidId`；找不到该页报 `IdNotFound`。`after` 不是节 id 报 `InvalidId`；找不到该节报 `IdNotFound`。
写入 `p:extLst/p:ext[@uri="{521415D9-36F7-43E2-AB2F-B90AF26B5E84}"]/p14:sectionLst`（MS-PPTX §2.3.1.25）。
`p14:section/@id` 写成带花括号的 GUID。`xmlns:p14` 写在 `sectionLst` 上，
命名空间 `http://schemas.microsoft.com/office/powerpoint/2010/main`。
已有同一 uri 的 `p:ext` 则复用，不另开一份。其它 `p:ext` 原样留。

没有 `sectionLst` 时：省略 `start` 与 `after`，把现有全部页放进这一节。
给了 `start` 且该页不是第一页，则 `start` 之前的页单独成节，`name` 为空串，新命名节从 `start` 起到末页；`start` 为第一页则与省略 `start` 相同。给了 `after` 时报 `IdNotFound`（还没有节）。

已有 `sectionLst` 时：省略 `start` 与 `after` 则在末尾追加空节。给了 `after` 则在该节之后插入空节。
给了 `start` 则拆开包含该页的那一节：该页之前留在原节，该页及其后（直到原节末尾）进新节，新节紧插在原节之后。该页已是该节第一页则原节变空。该页不在任何节里报 `InvalidField`。

新节成员与 `p:sldIdLst` 必须同序同成员。拆节不重排 `sldIdLst`。xlsx / docx 上发此 op 报 `InvalidField`。

### setSlideSectionName

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setSlideSectionName"` |
| `id` | `section:{n}` |
| `name` | 新显示名，字符串。缺省报 `MissingField`。允许空串 |

找不到该节报 `IdNotFound`。id 不是节 id 报 `InvalidId`。只改 `p14:section/@name`。不改 GUID、不改所含页。xlsx / docx 上发此 op 报 `InvalidField`。

### deleteSlideSection

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteSlideSection"` |
| `id` | `section:{n}` |

找不到该节报 `IdNotFound`。id 不是节 id 报 `InvalidId`。页留在 `sldIdLst`，不删幻灯片部件。该节里的 `ST_SlideId` 并进前一节（删的是第一节则并进后一节）。
只剩这一节时去掉整份 `p14:sectionLst`；若该 `p:ext` 里没有别的子元素则连 `p:ext` 一起去掉。空的 `p:extLst` 也去掉。xlsx / docx 上发此 op 报 `InvalidField`。

### setSlideSection

把一页划进某节。节必须覆盖 `sldIdLst` 的一个连续分段。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setSlideSection"` |
| `id` | `slide:{n}` |
| `section` | 目标 `section:{n}` |

找不到该页报 `IdNotFound`。`id` 不是页 id 报 `InvalidId`。`section` 缺省报 `MissingField`；不是节 id 报 `InvalidId`；找不到报 `IdNotFound`。
没有 `sectionLst` 报 `InvalidField`。从各节名单里摘掉该页 `ST_SlideId`，追加到目标节末尾。然后按节名单拼接顺序重排 `p:sldIdLst`（保留原来的 `p:sldId` 元素与 `r:id`）。
已经在目标节末尾则空操作。`sldIdLst` 里尚未出现在任何节的页，追加到最后一节，以保住「各节合起来与 `sldIdLst` 同序同成员」。xlsx / docx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/pptx-apply-sections.json`。inspect 示例见 `docs/examples/pptx-inspect-sections.json`。

### insertCustomShow

在 `p:custShowLst` 里追加或插入一条自定义放映。不改各页 XML。不实现放映引擎。不改 `presProps` 的 `showPr`（放映设置用 `setShowPr`）。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "insertCustomShow",
      "id": "pptx:presentation",
      "name": "Tour",
      "slides": [
        "slide:1",
        "slide:3"
      ]
    }
  ]
}
```

- **`op`**
  - 必须是 `"insertCustomShow"`
- **`id`**
  - 必须是 `pptx:presentation`
- **`name`**
  - 显示名
  - 缺省报 `MissingField`
  - 空串报 `InvalidField`
  - 超过 31 个字符报 `InvalidField`
  - 与已有 `p:custShow/@name` 相同报 `InvalidField`
- **`slides`**
  - 可选
  - `slide:{n}` 数组
  - 省略当作 `[]`
  - 允许空数组，允许同一页出现多次
- **`after`**
  - 可选
  - 已有 `show:{n}`
  - 新条插到该条之后
  - 省略则追加到末尾

`id` 不是 `pptx:presentation` 报 `InvalidId`。`after` 不是自定义放映 id 报 `InvalidId`；找不到报 `IdNotFound`。
`slides` 里不是页 id 报 `InvalidId`；找不到该页报 `IdNotFound`。
没有 `custShowLst` 则新建，插在 `notesSz`（以及若有则 `smartTags` / `embeddedFontLst`）之后、`photoAlbum` 之前。
`p:custShow/@id` 取已有最大整数加 1，没有则为 `0`。页条目写成 `<p:sld r:id="…"/>`，`r:id` 取该页 `p:sldId` 上的关系 id，不写 `ST_SlideId`。xlsx / docx 上发此 op 报 `InvalidField`。

### setCustomShow

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setCustomShow"` |
| `id` | `show:{n}` |
| `name` | 可选。新显示名，规则与 `insertCustomShow` 相同。改成与另一条已有名字相同报 `InvalidField`。改成自己当前名字则空操作 |
| `slides` | 可选。整份替换该条 `p:sldLst`，规则与 `insertCustomShow` 相同 |

`name` 与 `slides` 至少给一个，否则报 `InvalidField`。找不到该条报 `IdNotFound`。id 不是自定义放映 id 报 `InvalidId`。未点名的 `@id` 与 `extLst` 留下。
xlsx / docx 上发此 op 报 `InvalidField`。

### deleteCustomShow

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteCustomShow"` |
| `id` | `show:{n}` |

找不到该条报 `IdNotFound`。id 不是自定义放映 id 报 `InvalidId`。摘掉该 `p:custShow`。删光后去掉空的 `custShowLst`。不改各页 XML，不改 `sldIdLst`。
xlsx / docx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/pptx-apply-custom-shows.json`。inspect 示例见 `docs/examples/pptx-inspect-custom-shows.json`。

### insertOle

把本地文件作为嵌入对象写入目标宿主。不在库里启动 OLE 服务器，不生成真实预览图（写入一张最小 PNG 占位）。不写 `<p:timing>`。不进批注、备注母版、讲义。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "insertOle",
      "id": "slide:1",
      "file": "C:\\\\data\\\\book.xlsx"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertOle"` |
| `id` | 必须是 `slide:{n}`、`slide:{n}/notes`、`master:{n}` 或 `master:{n}/layout:{k}` |
| `file` | 必须。本地文件路径 |

`id` 不是合法宿主报 `InvalidId`。找不到该页 / 该母版 / 该版式报 `IdNotFound`。本期不要 `x` / `y` / `cx` / `cy`；出现报未知字段。
缺尺寸用 `3657600` × `2743200` EMU，坐标走该宿主空白处（与 `insertPicture` 同一套 `vacant_pos`）。
`id` 为 `slide:{n}/notes` 且该页还没有 notesSlide 时，按 `setNotesText` 同一套新建最小备注页再插入。
对 `notesMaster` / `handout` 发本 op 报 `InvalidId`。

按扩展名决定 ProgID 与写入方式：`.xlsx` 为 `Excel.Sheet.12`，`.xlsm` 为 `Excel.SheetMacroEnabled.12`，`.xls` 为 `Excel.Sheet.8`，
`.docx` 为 `Word.Document.12`，`.docm` 为 `Word.DocumentMacroEnabled.12`，`.pptx` 为 `PowerPoint.Show.12`，
`.pptm` 为 `PowerPoint.ShowMacroEnabled.12`。
这些扩展名把源文件原字节写入 `/ppt/embeddings/…`，关系 Type 为 `package`（与 PowerPoint 16.0 另存一致）。
其它扩展名 ProgID 为 `Package`，把源文件打成 OLE 复合文档（Ole10Native）写入 `/ppt/embeddings/oleObjectN.bin`，关系 Type 为 `oleObject`，并写 `showAsIcon="1"`。
找不到 `file` 报 `Io`。空路径报 `InvalidField`。xlsx 见 [xlsx 分册](json-contract-xlsx.md#insertole)。
docx 上仍用段落 id，见 [json-contract-docx.md](json-contract-docx.md#insertole)。

宿主是 `p:graphicFrame`，`a:graphicData/@uri` 为 Transitional `http://schemas.openxmlformats.org/presentationml/2006/ole`。
写 Office 16 的 `mc:AlternateContent`：`mc:Choice Requires="v"` 里只有 `p:oleObj`（没有 pic），
`mc:Fallback` 里是 `p:oleObj` 加 `p:pic` 预览。
预览 `p:cNvPr/@id` 为 `0`。`p:oleObj` 写 `name`、`r:id`、`imgW` / `imgH`（画布尺寸）、`progId` 与空的 `<p:embed/>`。
不写 `spid`、`a16:creationId`、`p14:modId`。`p:cNvPr/@id` 取该页已有全部 `cNvPr` 的最大整数加 1，至少为 2。

### deleteOle

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteOle"` |
| `id` | 必须是 OLE id |

找不到报 `IdNotFound`。id 不是 OLE id 报 `InvalidId`。摘整棵 graphicFrame（含 AlternateContent）。两支都扫 `r:id`（预览只在 Fallback）。
清掉无人引用的关系与 embeddings / 预览媒体。按 `deleteShape` 同一套规则清该 graphicFrame `cNvPr/@id` 的 timing `tgtEl`。
xlsx 见 [xlsx 分册](json-contract-xlsx.md#deleteole)。

示例见 `docs/examples/pptx-apply-ole.json`。inspect 示例见 `docs/examples/pptx-inspect-ole.json`。

### insertModel3d

把本地 GLB 作为 3D 模型写入目标幻灯片或备注页。不进 `pictures`。不栅格化模型。海报用 1×1 PNG 占位。不写 `<p:timing>`。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "insertModel3d",
      "id": "slide:1",
      "file": "C:\\\\data\\\\model.glb"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertModel3d"` |
| `id` | 必须是 `slide:{n}` 或 `slide:{n}/notes` |
| `file` | 必须。本地 GLB 路径 |
| `x` / `y` | 可选。画布 EMU。省略为 `0` |
| `cx` / `cy` | 成对可选。EMU。都省则 `914400` × `914400` |

`id` 不是页 id 或备注容器 id 报 `InvalidId`。找不到该页报 `IdNotFound`。
`cx` / `cy` 只给一个报 `InvalidField`。找不到 `file` 报 `Io`。空路径或不是 GLB 报 `InvalidField`。
批注内本期不插。create 可在 `slides[].notes` 对象上写 `model3ds`。

AlternateContent 放进 `p:spTree`。Choice 里是 `p:graphicFrame`。关系写在该页或 notesSlide `.rels`。
`[Content_Types].xml` 补 Default `glb` 为 `model/gltf.binary`。xlsx / docx 见各自分册。

### replaceModel3d

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"replaceModel3d"` |
| `id` | 必须是 3D 模型 id |
| `file` | 必须。本地 GLB 路径 |

换 GLB 字节，保留 graphicFrame 与关系 id。海报仍写成 1×1 PNG。不改位置尺寸。找不到报 `IdNotFound`。
id 不是 3D 模型 id 报 `InvalidId`。xlsx / docx 见各自分册。

### deleteModel3d

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteModel3d"` |
| `id` | 必须是 3D 模型 id |

找不到报 `IdNotFound`。id 不是 3D 模型 id 报 `InvalidId`。摘整棵 AlternateContent。
清掉无人引用的关系与 GLB / 海报。xlsx / docx 见各自分册。

### setModel3dSize

字段与 `setPictureSize` 的 pptx 一侧相同。`id` 必须是 3D 模型 id。
写入 graphicFrame 的 `p:xfrm/a:ext`。Fallback 海报 `p:pic` 同步改尺寸。
对图 id 发此 op、对 3D id 发 `setPictureSize` 仍报 `InvalidId`。xlsx / docx 见各自分册。

### setModel3dPos

字段与 `setPicturePos` 相同，`id` 必须是 pptx 3D 模型 id。
写入 graphicFrame 的 `p:xfrm/a:off`。Fallback 海报 `p:pic` 同步改位置。
docx / xlsx 上发此 op 报 `InvalidField`。

改 3D 尺寸与画布坐标的示例见 `docs/examples/pptx-apply-model3d-geom.json`。

### insertMath

把一条线性公式写入目标幻灯片或备注页。不进 `shapes`。不实现排版引擎。不写 `<p:timing>`。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "insertMath",
      "id": "slide:1",
      "text": "E=mc^2"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertMath"` |
| `id` | 必须是 `slide:{n}` 或 `slide:{n}/notes` |
| `text` | 必须。非空。写入一条 `m:r` / `m:t` |
| `x` / `y` | 可选。画布 EMU。省略为 `0` |
| `cx` / `cy` | 成对可选。EMU。都省则 `914400` × `914400` |

`id` 不是页 id 或备注容器 id 报 `InvalidId`。找不到该页报 `IdNotFound`。
`cx` / `cy` 只给一个报 `InvalidField`。空 `text` 报 `InvalidField`。出现 `expr` / `kind` 报 `InvalidField`。
出现 `tex` / `unicodemath` / `omml` 报未知字段。批注内本期不插。create 可在 `slides[].notes` 对象上写 `maths`。

AlternateContent 放进 `p:spTree`。Choice 里是 `p:sp`（`txBox="1"`），`p:txBody/a:p` 里是 `a14:m` 包 `m:oMathPara` / `m:oMath`。
不要写 `mathplaceholder` 域。Fallback 是同一套纯文本框，字为该 `text`。不栅格化，不写 `a:blipFill`。
xlsx / docx 见各自分册。

### setMath

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setMath"` |
| `id` | 必须是公式框 id |
| `text` | 必须。非空。用一条 `m:r` / `m:t` 替换该 `oMath` 子节点 |

不改位置尺寸。出现 `expr` / `kind` 报 `InvalidField`。空 `text` 报 `InvalidField`。
出现 `x` / `y` / `cx` / `cy` 报未知字段。
找不到报 `IdNotFound`。id 不是公式框 id 报 `InvalidId`。xlsx / docx 见各自分册。

### deleteMath

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteMath"` |
| `id` | 必须是公式框 id |

找不到报 `IdNotFound`。id 不是公式框 id 报 `InvalidId`。摘整棵 AlternateContent。
海报无人引用则删。xlsx / docx 见各自分册。

### setMathSize

字段与 `setShapeSize` 相同。`id` 必须是公式框 id。
写入 Choice 里该 `p:sp` 的 `p:spPr/a:xfrm/a:ext`。没有 `xfrm` 则补。不改 Fallback。
`cx` 与 `cy` 一个都不给报 `InvalidField`。对形状 id 发此 op、对公式框 id 发 `setShapeSize` 仍报 `InvalidId`。
docx 上发此 op 报 `InvalidField`。xlsx 见 [xlsx 分册](json-contract-xlsx.md#setmathsize)。

### setMathPos

字段与 `setShapePos` 相同。`id` 必须是公式框 id。
写入 Choice 里该 `p:sp` 的 `p:spPr/a:xfrm/a:off`。没有 `xfrm` 则补。不改 Fallback。
`x` 与 `y` 一个都不给报 `InvalidField`。docx / xlsx 上发此 op 报 `InvalidField`。
对公式框 id 发 `setShapePos` 仍报 `InvalidId`。

改公式框尺寸与画布坐标的示例见 `docs/examples/pptx-apply-math-geom.json`。

### insertField

往幻灯片文本形状最后一段、`a:endParaRPr` 之前插入一个 `a:fld`（ECMA-376 §21.1.2.2.4）。不写 `a:pPr`。不在库里求值，不刷新缓存。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "insertField",
      "id": "slide:1/shape:2",
      "type": "slidenum",
      "text": "1"
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertField"` |
| `id` | 必须是幻灯片、备注、母版或版式文本形状 id |
| `type` | 必须。白名单：`slidenum`、`datetime`、`datetime1`–`datetime13`。其它值报 `InvalidField` |
| `text` | 可选。写入 `a:t` 缓存。省略则空 `a:t`。不求值 |

`id` 不是合法文本形状 id 报 `InvalidId`。找不到形状报 `IdNotFound`。缺 `type` 报 `MissingField`。`instr` 出现报未知字段。
新 `@id` 写成带花括号的 GUID，避开本框已有 id。抄该段最后一个 `a:r` / `a:fld` 的 `a:rPr`（没有则抄 `a:endParaRPr`）并设 `smtClean="0"`。
xlsx 上发此 op 报 `InvalidField`。docx 仍要 `instr`，见 [json-contract-docx.md](json-contract-docx.md#insertfield)。

### setField

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setField"` |
| `id` | 必须是 pptx 域 id |
| `type` | 可选。白名单与 `insertField` 相同 |
| `text` | 可选。新的 `a:t` 缓存 |

`type` 与 `text` 至少给一个，否则报 `InvalidField`。改 `type` 不改 GUID。找不到报 `IdNotFound`。id 不是 pptx 域 id 报 `InvalidId`。
xlsx 上发此 op 报 `InvalidField`。docx 仍改 `instr`。

### deleteField

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteField"` |
| `id` | 必须是 pptx 域 id |
| `keep` | 可选。`result`（缺省）或 `instr`。其他值报 `InvalidField` |

`keep` 为 `result`：把 `a:fld` 换成 `a:r`，抄 `rPr` 与缓存 `a:t`。`keep` 为 `instr`：把 `a:t` 写成 `type` 字符串。找不到报 `IdNotFound`。
id 不是 pptx 域 id 报 `InvalidId`。xlsx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/pptx-apply-fields.json`。

### setShowPr

改 `/ppt/presProps.xml` 的 `p:showPr`（ECMA-376 §19.2.1.30）。不实现放映引擎，不改自定义放映名单，不改 `prnPr`。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "setShowPr",
      "id": "pptx:presentation",
      "type": "kiosk",
      "loop": true
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setShowPr"` |
| `id` | 必须是 `pptx:presentation` |
| `type` | 可选。`present`（演讲者）、`browse`（窗口）、`kiosk`（展台）。对应 `EG_ShowType` |
| `loop` | 可选。布尔。对应 `@loop` |
| `showNarration` | 可选。布尔。对应 `@showNarration` |
| `showAnimation` | 可选。布尔。对应 `@showAnimation` |
| `useTimings` | 可选。布尔。对应 `@useTimings` |

五个字段至少给一个，否则报 `InvalidField`。未点名的字段留下。`id` 不是 `pptx:presentation` 报 `InvalidId`。`type` 不是上述三值报 `InvalidField`。
xlsx / docx 上发此 op 报 `InvalidField`。没有 presProps 部件报 `OpcPartMissing`。

没有 `p:showPr` 时先按树上缺省再建，再打补丁。树上缺省与 PowerPoint 16.0 对 create 产物一致：`type` 为 `present`，`loop` 为 `false`，
`showNarration` 为 `true`，`showAnimation` 为 `true`，`useTimings` 为 `true`。

写出属性：`loop` / `showNarration` 为真写 `"1"`，为假省略；`showAnimation` / `useTimings` 为假写 `"0"`，为真省略。
这样只点 `loop: true` 会写出 `showNarration="1"`，避免把 COM 缺省旁白打成关。布尔按 `"1"` / `"true"` / `"TRUE"` / `"on"` 读成真。

类型子元素为 `p:present` / `p:browse` / `p:kiosk`。不写 `kiosk/@restart`、`browse/@showScrollbar`；已有则留下。新建时写 `<p:sldAll/>`。
已有 `sldRg` / `custShow` 则不改放映范围、不补 `sldAll`。不写 `penClr`、激光笔或 `p14:laserClr`；已有则原样留。
改成 `browse` 时，若还没有该扩展，在 `p:showPr` 的 `extLst` 写入 `p14:browseMode showStatus="0"`（uri
`{F99C55AA-B7CB-42B0-86F8-08522FDF87E8}`，MS-PPTX §2.3.1.2）。
没有该扩展时 PowerPoint 16.0 的 COM `ShowType` 为 `4`（Window2），有则是 `2`（窗口）。仅有这份 browseMode 的 `extLst` 不阻止删掉回到缺省的 `p:showPr`。

有效值回到 PowerPoint 缺省、且没有 `sldRg` / `custShow` / `penClr`、也没有除 browseMode 以外的 `extLst` 时，删掉整个 `p:showPr`。插在 `prnPr` 之后、`clrMru` 之前。

示例见 `docs/examples/pptx-apply-protect.json`。

### insertZoom

把幻灯片缩放、节缩放或摘要缩放写入目标宿主。不实现放映跳转，不写 `<p:timing>`，不生成真实预览图（写入一张最小 PNG 占位）。不进批注、备注母版、讲义。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    {
      "op": "insertZoom",
      "id": "slide:1",
      "type": "slide",
      "target": "slide:2"
    },
    {
      "op": "insertZoom",
      "id": "slide:1",
      "type": "summary",
      "targets": ["section:1", "section:2"]
    }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertZoom"` |
| `id` | 必须是 `slide:{n}`、`slide:{n}/notes`、`master:{n}` 或 `master:{n}/layout:{k}` |
| `type` | 必须是 `"slide"`、`"section"` 或 `"summary"`。其它字符串报 `InvalidField` |
| `target` | `type` 为 `slide` / `section` 时必须。`slide` 时是 `slide:{m}`，且不能是宿主自己；`section` 时是 `section:{n}`。`type` 为 `summary` 时出现报 `InvalidField` |
| `targets` | `type` 为 `summary` 时必须。节 id 数组，至少一条，按文档顺序写入 `summaryZmObj`。不许重复。`type` 为 `slide` / `section` 时出现报 `InvalidField` |
| `x` / `y` | 可选。画布坐标，EMU，非负整数。必须成对出现或成对省略；只给一个时报 `InvalidField`。缺则走该页空白处（与 `insertPicture` 同一套 `vacant_pos`） |
| `cx` / `cy` | 可选。尺寸，EMU，正整数。必须成对出现或成对省略。缺则用 `3657600` × `2743200` EMU |

`id` 不是页 id 报 `InvalidId`。找不到该页报 `IdNotFound`。`type` 缺省报 `MissingField`。
`type` 为 `slide` / `section` 时 `target` 缺省报 `MissingField`。`type` 为 `summary` 时 `targets` 缺省报 `MissingField`。
`target` 与 `targets` 同时出现报 `InvalidField`。`targets` 为空或含重复节报 `InvalidField`。
`type` 为 `slide` 时 `target` 不是页 id 报 `InvalidId`；找不到目标页报 `IdNotFound`；目标就是宿主自己报 `InvalidField`。
`type` 为 `section` 时 `target` 不是节 id 报 `InvalidId`；找不到该节、节没有 GUID、或节里没有页报 `IdNotFound`。
`type` 为 `summary` 时 `targets` 各项不是节 id 报 `InvalidId`；找不到该节、节没有 GUID、或节里没有页报 `IdNotFound`。
xlsx / docx 上发此 op 报 `InvalidField`。

宿主是 `p:graphicFrame`，包在 `mc:AlternateContent` 的 Choice 里。
幻灯片缩放 `a:graphicData/@uri` 为 `http://schemas.microsoft.com/office/powerpoint/2016/slidezoom`，`Requires` 为 `pslz`；
节缩放 uri 为 `http://schemas.microsoft.com/office/powerpoint/2016/sectionzoom`，`Requires` 为 `psez`；
摘要缩放 uri 为 `http://schemas.microsoft.com/office/powerpoint/2016/summaryzoom`，`Requires` 为 `psuz`。
Choice 里是 graphicFrame，内含 `sldZm` / `sectionZm` / `summaryZm`。`sldZmObj/@sldId` 取目标页 `p:sldId` 的无前缀 `id`（`ST_SlideId`）。
`sectionZmObj/@sectionId` 与 `summaryZmObj/@sectionId` 取该节 GUID。`zmPr` 写新 GUID，幻灯片缩放写 `returnToParent="1"`。预览关系写 `p166:blipFill/a:blip/@r:embed`。
摘要缩放按 `targets` 顺序写多个 `summaryZmObj`，再写空的 `psuz:gridLayout`。不写 `title` / `descr` / 偏移比例。
Fallback 是 `p:pic`，预览同一张 PNG，并带 `a:hlinkClick`（`action="ppaction://hlinksldjump"`）：
幻灯片缩放跳到目标页，节缩放跳到该节第一页，摘要缩放跳到 `targets` 第一节的第一页。
根 `p:sld` 补 `xmlns:mc`、对应 Zoom 前缀与 `xmlns:p166`（`http://schemas.microsoft.com/office/powerpoint/2016/6/main`），`mc:Ignorable` 含该前缀与 `p166`。
`p:cNvPr/@id` 取该页已有全部 `cNvPr` 的最大整数加 1，至少为 2。`p:cNvPr/@name` 为 `Zoom {n}`，`{n}` 为该页已有 Zoom 个数加 1。

### deleteZoom

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteZoom"` |
| `id` | 必须是 Zoom id |

找不到报 `IdNotFound`。id 不是 Zoom id 报 `InvalidId`。摘整段 AlternateContent（或无 AC 时的 graphicFrame）。两支都扫关系 id。
清掉无人引用的预览媒体与指向目标页的 slide 关系。按 `deleteShape` 同一套规则清该 graphicFrame `cNvPr/@id` 的 timing `tgtEl`。xlsx / docx 上发此 op 报 `InvalidField`。

### setZoom

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    { "op": "setZoom", "id": "slide:1/zoom:3", "target": "slide:3" },
    { "op": "setZoom", "id": "slide:1/zoom:4", "targets": ["section:1", "section:2"] },
    { "op": "setZoom", "id": "slide:1/zoom:3", "file": "C:\\img\\cover.png" },
    { "op": "setZoom", "id": "slide:1/zoom:3", "x": 914400, "y": 457200, "cx": 1828800, "cy": 914400 }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setZoom"` |
| `id` | 必须是 Zoom id |
| `target` | 可选。幻灯片缩放改为 `slide:{m}`，节缩放改为 `section:{n}`。摘要缩放上出现报 `InvalidField` |
| `targets` | 可选。摘要缩放改为这个节 id 数组。幻灯片缩放 / 节缩放上出现报 `InvalidField` |
| `file` | 可选。本地图片路径。换成自定义封面 |
| `x` / `y` | 可选。画布坐标，EMU，非负整数 |
| `cx` / `cy` | 可选。画布尺寸，EMU，正整数。必须成对 |

`target`、`targets`、`file`、几何至少给一类，否则报 `InvalidField`。只给几何也合法。`target` 与 `targets` 同时出现报 `InvalidField`。
`cx` 与 `cy` 只出现一侧报 `InvalidField`。
`targets` 为空或含重复节报 `InvalidField`。空 `file` 报 `InvalidField`。读不到 `file` 报 `Io`。不是该格式认的图片报 `InvalidField`。
SVG 只写入 PNG 海报，不写 `svgBlip`。
给了 `target` / `targets` 时改对应 `@sldId` / `@sectionId`，并改 Fallback `a:hlinkClick` 的 slide 关系。
给了 `file` 时换该 Zoom 内全部 `a:blip/@r:embed`（含 Fallback `p:pic`），各 `zmPr` 写 `imageType="cover"`。
给了几何时写入 Choice 里 graphicFrame 的 `p:xfrm`。没有 graphicFrame 则写 `spPr/a:xfrm` 或 Fallback `p:pic` 的 `xfrm`，与 inspect 取值同一条路径。
不改 `type`。不写 `<p:timing>`。找不到报 `IdNotFound`。id 不是 Zoom id 报 `InvalidId`。
xlsx / docx 上发此 op 报 `InvalidField`。

示例见 `docs/examples/pptx-apply-zoom.json`。inspect 示例见 `docs/examples/pptx-inspect-zoom.json`。

### 改动范围（pptx）

允许变脏的部件：被改的那一张幻灯片 XML（通常是 `/ppt/slides/slideN.xml`，N 以关系目标为准）及其 `.rels`，被改的 notesSlide 及其 `.rels`，
必要时 `presentation.xml` 及其 `.rels`、新建的 notesMaster 及其 `.rels`，`[Content_Types].xml`，
以及新建或被删的 `/ppt/media/…`、`/ppt/charts/…`、embedding、`/ppt/diagrams/…` 及可选 drawing 缓存，
以及现代批注部件 `/ppt/comments/modernComment_*.xml` 与 `/ppt/authors.xml`。
`setShapeText` / `setShapeFormat` 以及 `setShapePos` / `setShapeSize` / `setShapeFill` / `setShapeLine` / `setShapeGeom` / `deleteShape` 只脏目标宿主 XML（幻灯片、
notesSlide、被点名的 slideMaster 或 slideLayout、notesMaster 或 handoutMaster）。
`insertShape` 往幻灯片插只脏该页 XML；往备注插时可脏 notesSlide、该页 `.rels`、notesMaster、presentation，与 `setNotesText` 相同。
往母版 / 版式插只脏目标 slideMaster 或 slideLayout XML。
`insertConnector` / `deleteConnector` / `setConnector` 脏该宿主 XML（幻灯片、notesSlide、slideMaster 或 slideLayout）。`groupShapes` / `ungroup` 只脏该页幻灯片 XML。
`insertTable` / `deleteTable` / `insertRow` / `deleteRow` / `insertCol` / `deleteCol` / `setTableCellText`
 / `setTablePos` / `setTableSize` / `setTableColWidths` / `setTableTrHeight` / `setTableCellBorders` / `setTableCellInset` / `setTableCellPr` 脏该宿主 XML（幻灯片、notesSlide、slideMaster 或 slideLayout），不新建部件。
`setTableCellFill` 非 `blip` 同样只脏该宿主 XML；`type` 为 `blip` 时还可脏该宿主 `.rels`、`[Content_Types].xml` 与 `/ppt/media/…`；
往备注插且还没有 notesSlide 时可脏 notesSlide、该页 `.rels`、notesMaster、presentation，与 `setNotesText` 相同。
`setNotesText` 可脏 notesSlide、该页 `.rels`、notesMaster、presentation，以及新建 notesMaster 时复制的 theme 部件。图片 op 还可脏该宿主关系与媒体。
图表 op 还可脏 chart 部件、embedding。SmartArt op 还可脏四件套与缓存。
批注 op 还可脏该页 comments 部件、幻灯片 XML（新建时的 `commentRel`）、presentation `.rels` 与 authors。
`insertSlide` / `deleteSlide` / `moveSlide` 还可脏 `presentation.xml` 的 `sldIdLst`（及若有则 `p14:sectionLst` 与 `p:custShowLst`）、
presentation 关系、新页或被删页及其 `.rels`、内容类型，
以及被删页无人引用的备注 / 现代批注 / 图表 / embedding / diagram / 媒体。
`insertSlideSection` / `setSlideSectionName` / `deleteSlideSection`
 / `setSlideSection` 只脏 `presentation.xml` 的 `p:extLst` 里这份节扩展；`setSlideSection` 还可重排 `sldIdLst`。
`insertCustomShow` / `setCustomShow` / `deleteCustomShow` 只脏 `presentation.xml` 的 `p:custShowLst`，不脏各页 XML，不脏 `.rels`。
`insertOle` / `deleteOle` 脏该宿主 XML 及其 `.rels`、`[Content_Types].xml`，
以及新建或被删的 `/ppt/embeddings/…` 与预览 `/ppt/media/imageN.*`。
`deleteOle` 还可清该页 `<p:timing>` 里对应 `tgtEl`。
`setModel3dSize` / `setModel3dPos` 只脏该宿主 XML。
`setPicturePos` / `setPictureSize` 只脏该宿主 XML。视频 / 音频走同一套，不换媒体部件。
`setMathSize` / `setMathPos` 只脏该宿主 XML。不改字。
`insertZoom` / `setZoom` / `deleteZoom` 脏该宿主 XML 及其 `.rels`、`[Content_Types].xml`，以及新建或被删的预览 `/ppt/media/imageN.*`。
只改几何时 `setZoom` 只脏该宿主 XML。
`deleteZoom` 还可清该页 `<p:timing>` 里对应 `tgtEl`。`setShowPr` 只脏 `/ppt/presProps.xml` 的 `p:showPr`；没有该部件报 `OpcPartMissing`。
`setProtection` 只脏 `presentation.xml` 的 `p:modifyVerifier`。`setSlideHidden` 只脏该页 `p:sld/@show`。
`setTransition` 只脏被点名宿主 XML 的 `<p:transition>`（或包住它的 `mc:AlternateContent`），不脏 timing。
给了 `sound` 时还可脏该宿主 `.rels`、`/ppt/media/…` 与 `[Content_Types].xml`。
幻灯片宿主是该页幻灯片 XML；母版 / 版式是被点名的 slideMaster 或 slideLayout。
`insertAnim` / `setAnim` / `deleteAnim` / `moveAnim` 脏被点名宿主 XML 的 `<p:timing>`（含 `bldLst`），不脏 `<p:transition>`。
给了 `sound` 时还可脏该宿主 `.rels`、`/ppt/media/…` 与 `[Content_Types].xml`。
幻灯片宿主是该页幻灯片 XML；备注是该页 notesSlide；母版 / 版式是被点名的 slideMaster 或 slideLayout；备注母版是 notesMaster；讲义是 handoutMaster。
`setSlideLayout` 只脏该页 `.rels` 的 `slideLayout` 目标，不脏幻灯片 XML、母版 XML 或版式 XML。
`insertLayout` 脏新 slideLayout 及其 `.rels`、该母版 XML 与 `.rels`、`[Content_Types].xml`。
`deleteLayout` 脏该母版 XML 与 `.rels`、被删 slideLayout 及其 `.rels`、内容类型。
`insertPlaceholder` / `deletePlaceholder` 脏目标 slideMaster、slideLayout、notesMaster 或 handoutMaster XML。
往备注母版 / 讲义插且还没有该部件时，还可脏 `presentation.xml` 及其 `.rels`、新建的 notesMaster / handoutMaster 及其 `.rels`、内容类型与 theme 副本。
`insertMaster` 脏 `presentation.xml` 及其 `.rels`、新 slideMaster / slideLayout / theme 及其 `.rels`、内容类型。
`deleteMaster` 脏 `presentation.xml` 及其 `.rels`、被删母版及其版式与仅被它引用的 theme、内容类型。
`setSlideSize` 只脏 `presentation.xml` 的 `p:sldSz`。
`setNotesSize` 只脏 `presentation.xml` 的 `p:notesSz`。
`setThemeColors` / `setThemeFonts` / `setThemeFmt` / `setThemeExtraSchemes` 只脏该宿主指向的 theme 部件（slideMaster、notesMaster 或 handoutMaster），不新建 theme，不新建母版。
`setClrMap` 只脏被点名的 slideMaster、notesMaster 或 handoutMaster XML。
`setClrMapOvr` 只脏被点名的幻灯片、slideLayout 或 notesSlide XML。
`setThemeOverride` 脏该页或该备注页 `.rels`、`/ppt/theme/themeOverrideN.xml` 与必要时 `[Content_Types].xml`。
`enabled: false` 还可删掉无人引用的覆盖部件。
`setHandoutText` 可脏 handoutMaster 及其 `.rels`、必要时 `presentation.xml` 及其 `.rels` 与 `[Content_Types].xml`，以及新建 handoutMaster 时复制的 theme 部件。
超链接 op 脏该页幻灯片 XML 与该页 `.rels`（新建或删除 hyperlink / slide 关系）。域 op（`insertField` / `setField` / `deleteField`）只脏该页幻灯片 XML。
媒体 op 还可脏该页 `.rels`、`[Content_Types].xml`，以及新建或被删的 `/ppt/media/mediaN.*` 与海报 `/ppt/media/imageN.*`。
`setMedia` 给了 `tracks` 时还可新建或删除 `text/vtt` 部件。
其他页、未点名的幻灯片母版、未点名的版式、主题一律原样写回。未点名的 `presProps` 原样写回。
一次 `ops` 可以混用 `setShapeText`、`setShapeFormat`、形状管理 op、表格 op、`setNotesText`、`setHandoutText`、批注 op、超链接 op、域 op（`insertField`、`setField`、
`deleteField`）、图片 op、媒体 op（含 `setMedia`）、图表 op、SmartArt op、幻灯片管理 op（含 `setTransition`、`setSlideLayout`）、母版写入 op（`insertLayout`、
`deleteLayout`、`insertPlaceholder`、`deletePlaceholder`、`insertMaster`、`deleteMaster`）、主题与尺寸 op（`setSlideSize`、`setNotesSize`、
`setThemeColors`、`setThemeFonts`、`setThemeFmt`、`setThemeExtraSchemes`、`setClrMap`、`setClrMapOvr`、`setThemeOverride`）、动画 op（`insertAnim`、`setAnim`、`deleteAnim`、`moveAnim`）、幻灯片节 op、自定义放映 op（`insertCustomShow`、`setCustomShow`、
`deleteCustomShow`）、OLE op（`insertOle`、`deleteOle`）、Zoom op（`insertZoom`、`setZoom`、`deleteZoom`）、公式几何 op（`setMathSize`、`setMathPos`）、`setShowPr`、
`setProtection` 与 `setProps` / `setCustomProp` / `deleteCustomProp`。
文件属性 op 还可以脏 `/docProps/core.xml`、`/docProps/app.xml`、`/docProps/custom.xml`、根 `.rels` 与 `[Content_Types].xml`。
只改 core 字段时不要新建 custom。未点名的 `Slides` 留下。只改 SmartArt 点文字或增删点、且宿主 graphicFrame 未动时，该页 XML 按「drawing 未动」验收。

被改部件同样整份重序列化，保留程度与 xlsx 工作表相同：元素与属性顺序、未知命名空间保留；注释、处理指令、DOCTYPE 会丢。

