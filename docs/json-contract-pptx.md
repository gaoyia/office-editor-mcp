# pptx 领域 JSON

本文是 [json-contract.md](json-contract.md) 的 pptx 分册。这里写稳定 id、inspect 的 scope 与响应、create，以及只属于演示文稿的 apply op。

信封、跨格式 op（图、图表、SmartArt、批注、文本框、表格插删行列、`setProps`、`setShapeText` 等）以及 `exportMedia` / `exportChartData` 仍在主文件。

机器可读副本见 [json-schema/pptx.schema.json](json-schema/pptx.schema.json)，入口仍是 [json-schema/request.schema.json](json-schema/request.schema.json)。与 Markdown 冲突时以本文与主契约为准。

## 稳定 id

演示文稿根节点 id 固定为 `pptx:presentation`。

| 节点 | id | 例子 |
| --- | --- | --- |
| 页 | `slide:{n}` | `slide:1` |
| 幻灯片母版 | `master:{n}` | `master:1` |
| 版式 | `master:{n}/layout:{k}` | `master:1/layout:2` |
| 母版占位符 | `master:{n}/shape:{shapeId}` 或 `master:{n}/shape:index:{k}` | `master:1/shape:2` |
| 版式占位符 | `master:{n}/layout:{k}/shape:{shapeId}` 或 `…/shape:index:{i}` | `master:1/layout:1/shape:2` |
| 形状 | `slide:{n}/shape:{shapeId}` | `slide:1/shape:4` |
| 无 `cNvPr/@id` 的形状 | `slide:{n}/shape:index:{k}` | `slide:1/shape:index:2` |
| 图 | `slide:{n}/pic:{cNvPrId}` | `slide:1/pic:4` |
| 无 `cNvPr/@id` 的图 | `slide:{n}/pic:index:{k}` | `slide:1/pic:index:1` |
| 图表 | `slide:{n}/chart:{cNvPrId}` | `slide:1/chart:5` |
| 无 `cNvPr/@id` 的图表 | `slide:{n}/chart:index:{k}` | `slide:1/chart:index:1` |
| 系列 | `{图表 id}/series:{idx}` | `slide:1/chart:5/series:0` |
| 趋势线 | `{系列 id}/trendline:{k}` | `slide:1/chart:5/series:0/trendline:1` |
| 误差线 | `{系列 id}/errorBars:{k}` | `slide:1/chart:5/series:0/errorBars:1` |
| SmartArt | `slide:{n}/smartdiagram:{cNvPrId}` | `slide:1/smartdiagram:4` |
| 无 `cNvPr/@id` 的 SmartArt | `slide:{n}/smartdiagram:index:{k}` | `slide:1/smartdiagram:index:1` |
| SmartArt 点 | `{图示 id}/pt:{modelId}` | `slide:1/smartdiagram:4/pt:2` |
| 备注容器 | `slide:{n}/notes` | `slide:1/notes` |
| 备注形状 | `slide:{n}/notes/shape:{shapeId}` | `slide:1/notes/shape:3` |
| 无 `cNvPr/@id` 的备注形状 | `slide:{n}/notes/shape:index:{k}` | `slide:1/notes/shape:index:1` |
| 备注图 | `slide:{n}/notes/pic:{cNvPrId}` | `slide:1/notes/pic:5` |
| 无 `cNvPr/@id` 的备注图 | `slide:{n}/notes/pic:index:{k}` | `slide:1/notes/pic:index:1` |
| 表 | `slide:{n}/table:{cNvPrId}` | `slide:1/table:4` |
| 无 `cNvPr/@id` 的表 | `slide:{n}/table:index:{k}` | `slide:1/table:index:1` |
| 表行 | `{表 id}/row:{r}` | `slide:1/table:4/row:2` |
| 表单元格 | `{表 id}/r:{r}/c:{c}` | `slide:1/table:4/r:1/c:2` |
| 批注 | `slide:{n}/comment:{guid}` | `slide:1/comment:{A1B2C3D4-E5F6-4789-ABCD-EF1234567890}` |
| 批注回复 | `{批注 id}/reply:{guid}` | `slide:1/comment:{A1B2C3D4-E5F6-4789-ABCD-EF1234567890}/reply:{11111111-2222-4333-8444-555555555555}` |
| 超链接 | `slide:{n}/link:{k}` | `slide:1/link:1` |
| 讲义母版 | `handout` | `handout` |
| 自定义属性 | `prop:{name}` | `prop:Editor` |

`{n}` 从 1 起，按 Presentation 部件（§13.3.6）里 `p:sldIdLst` / `p:sldId` 的出现顺序（§19.2.1.34），再跟 `r:id` 找到幻灯片部件。`n` 不是文件名里的数字，也不是 `p:sldId/@id`（那是 `ST_SlideId`，常见从 256 起）。隐藏页仍占序号，仍可 inspect 和 apply。`insertSlide` / `deleteSlide` / `moveSlide` 之后名单重排，旧的 `slide:{n}` 以及挂在它下面的形状 / 图 / 图表 / 表 / 备注 / 批注 id 都按新顺序计。只够同一次 inspect 之后立刻 apply；同一 `apply` 里后面的 op 用新序号。

母版 `{n}` 从 1 起，按 Presentation 里 `p:sldMasterIdLst` / `p:sldMasterId` 的出现顺序（§19.2.1.35、§13.3.10），再跟 `r:id` 找到 slideMaster 部件。不是文件名里的数字，也不是 `sldMasterId/@id`。版式 `{k}` 从 1 起，按**该母版** `p:sldLayoutIdLst` / `p:sldLayoutId` 的出现顺序，再跟母版 `.rels` 找到 slideLayout 部件（§13.3.9）。换页的版式关系不改这些序号。讲义母版最多一份（`p:handoutMasterIdLst` / `p:handoutMasterId`，§19.2.1.24、§19.3.1.24）。容器 id 固定为 `handout`。本期不把讲义占位符编成 `handout/shape:…`。

`{shapeId}` 取该形状 `p:nvSpPr/p:cNvPr/@id` 的原文（§19.3.1.12）。输出用文件里的原始数字字符串。请求按十进制数值匹配，不因前导零而失败。

没有 `cNvPr/@id` 时用 `slide:{n}/shape:index:{k}`，`k` 从 1 起，按该页列入树的形状顺序。树上同时给 `"stable": false`。这类 id 在增删形状后会变，只够同一次 inspect 之后立刻 apply。Office 亲手存的文件几乎都有 `cNvPr/@id`。

图 id 的 `{cNvPrId}` 取该 `p:pic` 的 `p:nvPicPr/p:cNvPr/@id` 原文。没有该属性时用 `slide:{n}/pic:index:{k}`，`stable` 为 `false`。`{k}` 只在该页全部被收入的 `p:pic` 内从 1 计，与形状的 `shape:index` 分家，互不影响。只认带 `a:blip` 的 `p:pic`。`p:sp`、连接线、`p:graphicFrame`（图表 / 表 / SmartArt）不进 `pictures`。组合本身不进树，里面符合条件的 `p:pic` 要展开。`mc:AlternateContent` 只进第一个 `Choice`，规则与形状相同。

图表 id 的 `{cNvPrId}` 取该 `p:graphicFrame` 的 `p:nvGraphicFramePr/p:cNvPr/@id` 原文。没有则用 `slide:{n}/chart:index:{k}`，`stable` 为 `false`。`{k}` 只在该页全部被收入的图表内从 1 计，与 `pic:index` / `shape:index` 分家。只认 `p:graphicFrame` 且 `a:graphicData/@uri` 为 chart URI（可带尾斜杠）。chartex URI 列入，`type` 为 `chartex`。`p:pic`、表格、SmartArt 不进 `charts`。组合里符合条件的框要展开。系列 `{idx}` 从 0 起，按 chart 部件里 `c:ser` 的文档顺序，不跟 `c:idx/@val`。趋势线 id 为 `{系列 id}/trendline:{k}`，误差线 id 为 `{系列 id}/errorBars:{k}`，`{k}` 从 1 起。数据在 embedding 工作簿与 cache 里；以 embedding 为准，不一致则该系列 `cacheStale: true`。

SmartArt id 的 `{cNvPrId}` 取该 `p:graphicFrame` 的 `p:nvGraphicFramePr/p:cNvPr/@id` 原文。没有则用 `slide:{n}/smartdiagram:index:{k}`，`stable` 为 `false`。`{k}` 只在该页全部被收入的图示内从 1 计，与 `pic:index` / `chart:index` / `shape:index` 分家。只认 `p:graphicFrame` 且 `a:graphicData/@uri` 为 `http://schemas.openxmlformats.org/drawingml/2006/diagram`（可带尾斜杠，ECMA-376 §21.4）。`dgm:relIds` 的四条关系写在该幻灯片 `.rels` 上。`p:pic`、图表、表格不进 `smartArts`。组合里符合条件的框要展开。点 id 规则与 docx 相同。备注、批注、母版、版式上的图示本期不进树。

表 id 的 `{cNvPrId}` 取该 `p:graphicFrame` 的 `p:nvGraphicFramePr/p:cNvPr/@id` 原文。没有则用 `slide:{n}/table:index:{k}`，`stable` 为 `false`。`{k}` 只在该页全部被收入的表内从 1 计，与 `pic:index` / `chart:index` / `smartdiagram:index` / `shape:index` 分家。只认 `p:graphicFrame` 且 `a:graphicData/@uri` 为 `http://schemas.openxmlformats.org/drawingml/2006/table`（可带尾斜杠，ECMA-376 §21.1.3.13）。图表 URI、diagram URI 不进 `tables`。`p:pic`、带 `txBody` 的 `p:sp` 不进 `tables`。组合里符合条件的框要展开，组合本身不进树。`mc:AlternateContent` 只进第一个 Choice。行号 `{r}` 是该 `a:tbl` 内 `a:tr` 的文档顺序，从 1 起。单元格 `{c}` 是该 `a:tc` 在网格上的起始列，从 1 起：无 `hMerge` 时该格占 `gridSpan` 列（缺省 1，§21.1.3.16）；随后的 `hMerge` 继续格仍列出，各占 1 列。垂直合并的 `vMerge` 继续格仍有 `a:tc`，树上 `vMerge` 为 `true`。`colCount` 取 `a:tblGrid` 的 `a:gridCol` 个数；没有则取第一行各格占的列之和。备注、批注、母版、版式上的表本期不进树。

`insertTable` 的 `id` 必须是 `slide:{n}`。`deleteTable` / `setTablePos` / `setTableSize` / `insertCol` / `deleteCol` 的 `id` 必须是表 id。`insertRow` 的 `id` 是表 id（追加一行）或行 id（插到该行之后）。`deleteRow` 的 `id` 必须是行 id。`setTableCellText` 的 `id` 必须是单元格 id。对不上的 id 形态报 `InvalidId`。xlsx 上这些 op（除 Word 已认的结构名在 Word 上的语义外）见各节；pptx 上发 `mergeCells` / `splitCells` 仍报 `InvalidField`。

同一页里若出现重复的 `cNvPr/@id`，inspect 照样列出每一处；apply 命中该页文档顺序中的第一处。

批注 `{guid}` 取该页现代批注部件里 `p188:cm/@id` 的原文（MS-PPTX 2018/8），含花括号。回复 `{guid}` 取同一条 `p188:cm` 里 `p188:reply/@id` 的原文。按文件原文精确匹配，不因大小写或有无花括号而放宽。同一页多条批注共用一个 comments 部件。ECMA-376 §19.4 旧式 `/ppt/comments/commentN.xml` 与 `commentAuthors` 不进树、不接受 apply；只有旧式时 `commentCount` 为 `0`，`scope.comments` 为空数组。insert 只写现代部件。

超链接 `{k}` 从 1 起，按该页被收入的文本形状上 `p:cNvPr/a:hlinkClick` 的文档顺序（与 `shapes` 同一套判定：带 `p:txBody` 的 `p:sp`，含组合里的；`mc:AlternateContent` 只进第一个 Choice）。不认 `a:hlinkHover`，不认 run 级 `a:rPr/a:hlinkClick`，不认图、表、图表、SmartArt、备注或母版上的链。增删之后 `{k}` 会变，只够同一次 inspect 之后立刻 apply。`insertHyperlink` 的 `id` 必须是幻灯片文本形状 id。`setHyperlink` / `deleteHyperlink` 的 `id` 必须是超链接 id。对不上的 id 形态报 `InvalidId`。

apply 不得把**未点名**的形状删掉再重建，以免 `cNvPr/@id` 变化导致绑在形状上的动画失效。`deleteShape` 只摘被点名的那一个 `p:sp`，并清掉 `<p:timing>` 里对应的 `p:tgtEl`（`p:spTgt/@spid` 对该形状 `cNvPr/@id`）；未点名形状的 timing 节点留下，不要整节删掉 `<p:timing>`。插、删、重排页只改 `p:sldIdLst` 与被点名页的部件，未点名页 XML 原字节写回。

讲义母版始终给出 `hasHandout`；`scope.handout: true` 时展开根上 `handout` 对象（页眉、页脚、日期、页码槽位）。幻灯片母版与版式见下文 `scope.masters`。备注页始终给出 `hasNotes`；`scope.notes: true` 时展开该页 `notesSlide` 上的文本形状。xlsx 批注走 `cell:{name}!{A1}/comment:{guid}`，不走 docx 的 `comment:{w:id}`，也不走 pptx 的 `slide:{n}/comment:{guid}`。pptx 批注走 `slide:{n}/comment:{guid}`，不走 `comment:{w:id}`。

「文本形状」要有可执行的判定：

- 只收带 `p:txBody` 的 `p:sp`（§19.3.1.45、§19.3.1.51），含嵌在 `p:grpSp` 里的。标题占位符、正文占位符、自由文本框都算。
- 不收 `p:pic`、`p:cxnSp`、`p:graphicFrame`（表格、图表、SmartArt 等）、`p:contentPart`。组合本身不进树，只展开里面符合条件的 `p:sp`。
- 遇到 `mc:AlternateContent` 只走第一个 `mc:Choice`；没有 `mc:Choice` 才走 `mc:Fallback`。两支都走会把同一份内容读出两遍，也会让 apply 只改到其中一支。这条对 `shapes`、`text` 和 apply 一律适用。


## inspect

xlsx、docx、pptx 共用请求字段名 `scope`，里面的键按格式分，混用报 `InvalidField`。请求信封见 [json-contract.md](json-contract.md#请求)。

### scope

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
| `scope` | 否 | 省略或空对象：返回 `slideCount`、`pictureCount`、`chartCount`、`smartArtCount`、`commentCount`、`hyperlinkCount`、`tableCount`、`animCount`、`masterCount`、`hasHandout` 和页名单；各页有这些计数、`layoutId`、`hasNotes` 和 `transition`，没有 `shapes`，也没有 `pictures` / `charts` / `smartArts` / `tables` / `timing` 名单，也没有 `notes` 对象，也没有 `comments` 名单，也没有 `hyperlinks` 名单，也没有根上 `masters`，也没有根上 `handout` |
| `scope.slide` | 要形状时必填 | 页序号，从 1 起，按 `p:sldIdLst` 顺序 |
| `scope.maxShapes` | 否 | 该页最多返回多少个文本形状，默认 `2000`，允许范围 `1` 到 `5000`。不管备注形状 |
| `scope.pictures` | 否 | 为 `true` 时给每页填 `pictures`。若同时 `notes: true` 且该页有备注对象，备注上也填 `pictures`。省略或 `false` 则只有计数，没有名单。不读媒体字节。不依赖 `scope.slide` |
| `scope.charts` | 否 | 为 `true` 时给每页填 `charts`。省略或 `false` 则只有 `chartCount`，没有名单。不把 chart 部件或 embedding 字节放进树。不依赖 `scope.slide` / `pictures`。备注里的图表本期不进树 |
| `scope.smartArt` | 否 | 为 `true` 时给每页填 `smartArts`。省略或 `false` 则只有 `smartArtCount`，没有名单。不把图示画成图，不读 Fallback 位图。不依赖 `scope.slide` / `pictures` / `charts`。备注里的 SmartArt 本期不进树 |
| `scope.notes` | 否 | 为 `true` 时给每页填 `notes`。有备注页则为对象，没有则为 `null`。省略或 `false` 则只有 `hasNotes`，不倒出备注形状。不依赖 `scope.slide` |
| `scope.comments` | 否 | 为 `true` 时给每页填 `comments`。没有现代批注时为空数组。省略或 `false` 则只有 `commentCount`，没有名单。不依赖 `scope.slide`。不读 ECMA-376 §19.4 旧式 `commentN.xml` |
| `scope.hyperlinks` | 否 | 为 `true` 时给每页填 `hyperlinks`。没有链时为空数组。省略或 `false` 则只有 `hyperlinkCount`，没有名单。不依赖 `scope.slide`。不认 run 级链、图、表、图表、SmartArt、备注或母版上的链。xlsx / docx 的 `scope.hyperlinks` 见各自响应 |
| `scope.tables` | 否 | 为 `true` 时给每页填 `tables`。省略或 `false` 则只有 `tableCount`，没有名单。不依赖 `scope.slide`。备注里的表本期不进树 |
| `scope.timing` | 否 | 为 `true` 时给每页填 `timing`（动画效果摘要）。省略或 `false` 则只有 `animCount`，没有名单。不依赖 `scope.slide`。不返回完整时间轴树，也不写 `<p:timing>` |
| `scope.masters` | 否 | 为 `true` 时根上填 `masters`（母版、版式与占位符）。省略或 `false` 则只有 `masterCount`，没有名单。不依赖 `scope.slide`。不算母版排版引擎。xlsx / docx 的 `scope` 里出现 `masters` 报 `InvalidField` |
| `scope.handout` | 否 | 为 `true` 时根上填 `handout`。有讲义母版则为对象，没有则为 `null`。省略或 `false` 则只有 `hasHandout`，不倒出槽位文字。不依赖 `scope.slide`。不算讲义打印布局。xlsx / docx 的 `scope` 里出现 `handout` 报 `InvalidField` |

只要想看形状，就必须给 `slide`。不默认把每一页的文本框都倒出来。只给 `maxShapes`、不给 `slide` 报 `ScopeRequired`。只给 `pictures: true`、`charts: true`、`smartArt: true`、`tables: true`、`notes: true`、`comments: true`、`hyperlinks: true`、`timing: true`、`masters: true` 或 `handout: true`、不给 `slide` 合法，各页都带对应名单、都不带幻灯片 `shapes`。根上 `masters` 只看 `scope.masters`。根上 `handout` 只看 `scope.handout`。`slide` 小于 `1` 报 `InvalidField`。`slide` 超出 `slideCount` 时各页都不带 `shapes`，不报错；图名单、图表名单、SmartArt 名单、表名单、备注对象、批注名单、超链接名单与动画摘要仍按各自键决定。


### 响应

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
  "hyperlinkCount": 0,
  "tableCount": 0,
  "animCount": 0,
  "masterCount": 1,
  "hasHandout": false,
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
      "chartCount": 0,
      "smartArtCount": 0,
      "commentCount": 0,
      "hyperlinkCount": 0,
      "tableCount": 0,
      "animCount": 0,
      "truncated": false,
      "shapes": [
        {
          "id": "slide:1/shape:2",
          "shapeId": "2",
          "name": "文本框 1",
          "x": 838200,
          "y": 685800,
          "cx": 10515600,
          "cy": 800100,
          "text": "标题",
          "stable": true
        },
        {
          "id": "slide:1/shape:3",
          "shapeId": "3",
          "name": "文本框 2",
          "x": 838200,
          "y": 1600200,
          "cx": 10515600,
          "cy": 800100,
          "text": "hello",
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
      "chartCount": 0,
      "smartArtCount": 0,
      "commentCount": 0,
      "hyperlinkCount": 0,
      "tableCount": 0,
      "animCount": 0
    }
  ]
}
```

规则：

- 读 Presentation 部件、各页幻灯片及其 `.rels`（用 `.rels` 判断 `hasNotes`，用现代 comments 关系判断批注）。`scope.notes` 为 `true` 且该页有 notesSlide 关系时再读 `/ppt/notesSlides/…`。`scope.comments` 为 `true` 且该页有现代 comments 关系时再读该 comments 部件与 `/ppt/authors.xml`。始终读 `p:sldMasterIdLst` 以填 `masterCount` 与各页 `layoutId`。始终看 presentation `.rels` 是否有 Type 以 `/handoutMaster` 结尾的关系，以填 `hasHandout`。`scope.masters` 为 `true` 时再读各 slideMaster / slideLayout 部件。`scope.handout` 为 `true` 且有讲义母版关系时再读该 handoutMaster 部件。备注里的图表、SmartArt 与表本期不进树。批注内图本期不进树。
- `slideCount` 是 `p:sldIdLst` 里 `p:sldId` 的个数，始终出现。
- `pictureCount` 在演示文稿根上始终出现，等于各页幻灯片 `p:pic`（带 `a:blip`）之和，不含备注图。每页始终有自己的 `pictureCount`，同样只计该页幻灯片。
- `chartCount` 在演示文稿根上始终出现，等于各页被收入的图表之和。每页始终有自己的 `chartCount`。
- `smartArtCount` 在演示文稿根上始终出现，等于各页被收入的 SmartArt 之和。每页始终有自己的 `smartArtCount`。
- `commentCount` 在演示文稿根上始终出现，等于各页 `p188:cm` 之和，不含 `p188:reply`。每页始终有自己的 `commentCount`，只计该页。旧式 §19.4 批注不计。
- `hyperlinkCount` 在演示文稿根上始终出现，等于各页被收入的形状级 `a:hlinkClick` 之和。每页始终有自己的 `hyperlinkCount`，只计该页幻灯片。没有链则为 `0`。根上不挂 `hyperlinks`，除非给了 `scope.hyperlinks`。不计 run 级链，也不计备注或母版上的链。
- `tableCount` 在演示文稿根上始终出现，等于各页被收入的 DrawingML 表之和。每页始终有自己的 `tableCount`。没有表则为 `0`。根上不挂 `tables`，除非给了 `scope.tables`。
- `hasNotes` 在每页始终出现：该页 `.rels` 有 Type 以 `/notesSlide` 结尾的关系则为 `true`，否则为 `false`。
- `slides` 始终列出全部页（含隐藏页）。未点名的页不带 `shapes`、`truncated`。
- `name` 取 `p:cSld/@name` 的原文。没有该属性则为 `null`。
- `layoutId` 在每页始终出现：该页 `.rels` 里 Type 以 `/slideLayout` 结尾的关系，对到某母版 `p:sldLayoutIdLst` 里的那一份，则为 `master:{n}/layout:{k}`。找不到该关系、或对不上任何已编母版版式，则为 `null`。不因换页关系改母版 / 版式序号。
- `hidden`：`p:sld/@show` 为 `0` 或 `false` 时为 `true`，其余为 `false`。
- `transition` 在每页始终出现。没有 `<p:transition>`（含包在 `mc:AlternateContent` 里的）则为 `null`。有则为对象，字段顺序：`type`、`dur`。`type` 取效果子元素的本地名（`sndAc` / `extLst` 不算）。ECMA-376 的 `blinds` / `checker` / `circle` / `dissolve` / `comb` / `cover` / `cut` / `diamond` / `fade` / `newsflash` / `plus` / `pull` / `push` / `random` / `randomBar` / `split` / `strips` / `wedge` / `wheel` / `wipe` / `zoom`，以及 MS-PPTX 的 `vortex` / `switch` / `flip` / `ripple` / `honeycomb` / `prism` / `doors` / `window` / `ferris` / `gallery` / `conveyor` / `pan` / `glitter` / `warp` / `flythrough` / `flash` / `shred` / `reveal` / `wheelReverse` / `morph`，按原名返回。不在这份名单里的效果（含 `prstTrans`）为 `other`。`mc:AlternateContent` 只走第一个 Choice。`dur` 优先取 `p14:dur`（毫秒，无单位整数）；没有则按 `spd` 折算：`slow` 为 `1000`，`med` 为 `750`，`fast` 或缺 `spd` 为 `500`。没有效果子元素但仍有 `<p:transition>` 时 `type` 为 `none`。
- 省略 `scope` 或只给空对象时，没有 `shapes`、`truncated`，也没有 `pictures` / `charts` / `smartArts` / `tables` / `timing` 名单，也没有 `notes`，也没有 `comments`，也没有 `hyperlinks`，也没有根上 `masters`，也没有根上 `handout`。
- 给了 `scope.slide` 时，只给那一页填 `shapes`。该页带 `p:txBody` 的 `p:sp` 超过 `maxShapes` 则 `truncated` 为 `true`，按该页文档顺序截断。未截断则为 `false`。备注形状不算进 `maxShapes`。
- `scope.slide` 大于 `slideCount` 时，各页都不带 `shapes`，不报错。
- `scope.pictures` 为 `true` 时，每页带 `pictures`。没有图时为空数组。不依赖 `scope.slide`。每张字段顺序：`id`、`cNvPrId`、`name`、`x`、`y`、`cx`、`cy`、`stable`。`cNvPrId` 是 `p:cNvPr/@id` 原文，没有则为 `null`。`name` 取 `p:cNvPr/@name`，没有则为 `""`。`x` / `y` 取 `p:spPr/a:xfrm/a:off` 的 EMU；`cx` / `cy` 取 `a:ext`。缺对应属性则为 `null`。不读媒体字节。幻灯片图 id 为 `slide:{n}/pic:…`。
- `scope.charts` 为 `true` 时，每页带 `charts`。没有图表时为空数组。不依赖 `scope.slide`。每张字段顺序：`id`、`cNvPrId`、`name`、`x`、`y`、`cx`、`cy`、`type`、`title`、`catAxisTitle`、`valAxisTitle`、`series`、`legend`、`dataLabels`、`flags`、`stable`。`cNvPrId` / `name` / `stable` 规则与图相同。`x` / `y` / `cx` / `cy` 取 graphicFrame 的 `p:xfrm`。`type` / `title` / 轴标题 / `series` / `legend` / `dataLabels` / `flags` 规则与 docx 图表相同；系列值优先读 embedding 单元格，读不到再退回 cache。
- `scope.smartArt` 为 `true` 时，每页带 `smartArts`。没有图示时为空数组。不依赖 `scope.slide`。每张字段顺序：`id`、`cNvPrId`、`name`、`x`、`y`、`cx`、`cy`、`layout`、`colors`、`style`、`points`、`stable`。`cNvPrId` / `name` / `stable` 规则与图相同。`x` / `y` / `cx` / `cy` 取 graphicFrame 的 `p:xfrm`。`layout` / `colors` / `style` / `points` 规则与 docx SmartArt 相同。pptx 没有 `kind` / `wrap`。
- `scope.tables` 为 `true` 时，每页带 `tables`。没有表时为空数组。不依赖 `scope.slide`。每张字段顺序：`id`、`cNvPrId`、`name`、`x`、`y`、`cx`、`cy`、`rowCount`、`colCount`、`rows`、`stable`。`cNvPrId` / `name` / `stable` 规则与图相同。`x` / `y` / `cx` / `cy` 取 graphicFrame 的 `p:xfrm`。`rowCount` 是 `a:tr` 个数。`colCount` 见上文表 id 那节。`rows` 按 `a:tr` 顺序。每行字段顺序：`id`、`index`、`cells`。`cells` 按该行 `a:tc` 文档顺序，不按空网格补格。每格字段顺序：`id`、`row`、`col`、`gridSpan`、`rowSpan`、`hMerge`、`vMerge`、`text`。`row` / `col` 与 id 里的数字相同。`gridSpan` / `rowSpan` 为整数，缺属性则为 `1`。`hMerge` / `vMerge` 为布尔（属性为 `1` / `true` / `on` 则为 `true`）。`text` 取该格 `a:txBody` 可见纯文本，规则与形状 `text` 相同。没有 `txBody` 则为 `""`。不把单元格编成段落 id。
- `animCount` 在演示文稿根上始终出现，等于各页效果摘要条数之和。每页始终有自己的 `animCount`。没有效果则为 `0`。根上不挂 `timing` 数组。名单只出现在各页上。本期不截断，一页全部列出。
- `masterCount` 在演示文稿根上始终出现，等于 `p:sldMasterIdLst` 里 `p:sldMasterId` 的个数。没有该表则为 `0`。根上不挂 `masters`，除非给了 `scope.masters`。不算讲义母版。本期不截断。
- `hasHandout` 在演示文稿根上始终出现：presentation `.rels` 有 Type 以 `/handoutMaster` 结尾的关系则为 `true`，否则为 `false`。根上不挂 `handout`，除非给了 `scope.handout`。
- `scope.handout` 为 `true` 时，根上带 `handout`。没有讲义母版关系则为 `null`。有则为对象，字段顺序：`id`、`header`、`footer`、`date`、`sldNum`。`id` 为 `handout`。四个槽位分别取 handoutMaster 上第一个 `p:ph/@type` 为 `hdr` / `ftr` / `dt` / `sldNum` 的 `p:sp` 的可见字。没有该类型占位符则为 `null`。有占位符但没有 `txBody` 则为 `""`。槽位文字把 `a:fld` 里的 `a:t` 也算进去（与幻灯片形状 `text` 跳过域缓存不同），以便看见日期与页码缓存。xlsx / docx 的 `scope` 里出现 `handout` 报 `InvalidField`。不算讲义打印布局，不列出 `sldImg` 缩略图占位符。
- `scope.timing` 为 `true` 时，每页带 `timing`。没有效果时为空数组。不依赖 `scope.slide`。只读 `<p:timing>`，不写。按文档顺序收集 `p:cTn` 上 `nodeType` 为 `clickEffect` / `withEffect` / `afterEffect` 的节点（ECMA-376 `ST_TLTimeNodeType`，§19.7.47）；`tmRoot` / `mainSeq` / `clickPar` / `withGroup` / `afterGroup` / `interactiveSeq` 不算一条。每条字段顺序：`target`、`type`、`trigger`。`trigger`：`clickEffect` 为 `click`，`withEffect` 为 `with`，`afterEffect` 为 `after`。`target` 取该效果子树里第一个 `p:spTgt/@spid`（不含嵌套的另一条效果 `cTn`），对到该页对象的 inspect id：`p:sp` 为 `slide:{n}/shape:{id}`，`p:pic` 为 `slide:{n}/pic:{id}`，表 graphicFrame 为 `slide:{n}/table:{id}`，图表为 `slide:{n}/chart:{id}`，SmartArt 为 `slide:{n}/smartdiagram:{id}`。对不上任何 `cNvPr/@id`，或对上的不是上述对象，则为 `null`。`type` 优先取该效果子树里第一个 `p:animEffect/@filter`，去掉括号及之后的参数（`blinds(horizontal)` 为 `blinds`）；没有则按第一个行为子元素：`animMotion` 为 `motion`，`animRot` 为 `spin`，`animScale` 为 `scale`，`animClr` 为 `color`，`anim` 为 `anim`，`set` 为 `set`；再没有则取该 `cTn` 的 `presetClass`（`entr` / `exit` / `emph` / `path` / `verb` / `mediacall`）；仍没有则为 `other`。不返回时长、路径点、文本范围或完整时间轴树。xlsx / docx 的 `scope` 里出现 `timing` 报 `InvalidField`。apply 不提供写 `<p:timing>` 的 op；对不认识的写动画 op（例如 `setTiming`）报 `UnsupportedOp`。
- `scope.masters` 为 `true` 时，根上带 `masters`。没有母版时为空数组。不依赖 `scope.slide`。不算母版排版引擎，也不把母版装饰形（没有 `p:ph` 的 `p:sp`）收进树。每份母版字段顺序：`id`、`index`、`name`、`placeholders`、`layouts`。`id` 为 `master:{n}`。`name` 取该 slideMaster 的 `p:cSld/@name`，没有则为 `null`。`placeholders` 只收带 `p:nvPr/p:ph` 的 `p:sp`（含组合里的），按文档顺序。每份版式字段顺序：`id`、`index`、`name`、`kind`、`placeholders`。`id` 为 `master:{n}/layout:{k}`。`name` 取该 slideLayout 的 `p:cSld/@name`，没有则为 `null`。`kind` 取 `p:sldLayout/@type` 原文，没有则为 `null`。create 写出的那份空白版式 `kind` 为 `blank`、名为「空白」。占位符字段顺序：`id`、`shapeId`、`name`、`type`、`idx`、`x`、`y`、`cx`、`cy`、`text`、`stable`。`id` 为 `master:{n}/shape:{cNvPrId}` 或 `master:{n}/layout:{k}/shape:{cNvPrId}`；没有 `cNvPr/@id` 则为 `…/shape:index:{i}`，`stable` 为 `false`，`{i}` 只在该母版或该版式被收入的占位符内从 1 计。`type` 取 `p:ph/@type`，缺则为 `null`。`idx` 取 `p:ph/@idx` 原文，缺则为 `null`。几何与 `text` 规则与幻灯片形状相同；没有 `txBody` 则 `text` 为 `""`。xlsx / docx 的 `scope` 里出现 `masters` 报 `InvalidField`。
- `scope.notes` 为 `true` 时，每页带 `notes`。没有 notesSlide 关系则为 `null`。有则为对象，字段顺序：`id`、`pictureCount`，有图名单时再跟 `pictures`，然后 `shapes`。`id` 为 `slide:{n}/notes`。`pictureCount` 只计该备注页上被收入的 `p:pic`，不并进幻灯片 `pictures`。备注形状判定与幻灯片形状相同（带 `p:txBody` 的 `p:sp`），id 为 `slide:{n}/notes/shape:…`。字段顺序与幻灯片形状相同：`id`、`shapeId`、`name`、`x`、`y`、`cx`、`cy`、`text`、`stable`。备注形状一次倒出，不用 `maxShapes`。
- `scope.comments` 为 `true` 时，每页带 `comments`。没有现代批注时为空数组。不依赖 `scope.slide`。按该页 `p188:cmLst` 里 `p188:cm` 的文档顺序。每条字段顺序：`id`、`commentId`、`author`、`initials`、`date`、`done`、`kind`、`x`、`y`、`text`、`replies`。`commentId` 是 `p188:cm/@id` 原文。`author` / `initials` 按 `authorId` 去 `/ppt/authors.xml` 的 `p188:author` 取 `name` / `initials`，对不上则为 `null`。`date` 取 `created` 原文，缺则为 `null`。`done`：`status` 为 `resolved` 或 `closed` 为 `true`；缺 `status`（即 active）为 `false`。`kind` 为 `normal`；该 `cm` 内出现墨迹标记（本地名为 `ink`）则为 `ink`。`x` / `y` 取 `p188:pos` 的 EMU，缺则为 `null`。`text` 取 `p188:txBody` 可见纯文本，规则与形状 `text` 相同。回复嵌在 `replies` 里，不单独占 `commentCount`。回复字段顺序：`id`、`commentId`、`author`、`initials`、`date`、`done`、`kind`、`text`（无 `x` / `y`）。回复 `id` 为 `{父批注 id}/reply:{guid}`。
- `scope.hyperlinks` 为 `true` 时，每页带 `hyperlinks`。没有链时为空数组。不依赖 `scope.slide`。按该页被收入形状上 `p:cNvPr/a:hlinkClick` 的文档顺序。每条字段顺序：`id`、`index`、`kind`、`host`、`rId`、`url`、`file`、`slide`、`tooltip`。`index` 与 id 里的 `{k}` 相同。`host` 是所在形状 id。`rId` 是 `r:id` 原文，缺则为 `null`。`kind` 为 `url`（External hyperlink 关系且没有文件跳转 `action`）、`file`（`action` 为 `ppaction://hlinkfile`）或 `slide`（slide 关系，或 `action` 为 `ppaction://hlinksldjump`）。`url` / `file` / `slide` 按 kind 填对应目标，其余为 `null`。`url` 与 `file` 取该关系 `Target` 原文，缺关系则为 `null`。`slide` 对到 `slide:{n}`，对不上则为 `null`。`tooltip` 取 `tooltip` 属性，缺则为 `null`。不把形状可见字抄进这条。xlsx / docx 的 `scope.hyperlinks` 见各自分册。
- 页对象字段顺序：`id`、`index`、`name`、`layoutId`、`hidden`、`transition`、`hasNotes`、`pictureCount`、`chartCount`、`smartArtCount`、`commentCount`、`hyperlinkCount`、`tableCount`、`animCount`，然后按需 `pictures`，再按需 `charts`，再按需 `smartArts`，再按需 `tables`，再按需 `notes`，再按需 `comments`，再按需 `hyperlinks`，再按需 `timing`，再按需 `truncated` 与 `shapes`。根对象字段顺序：`schemaVersion`、`format`、`id`、`slideCount`、`pictureCount`、`chartCount`、`smartArtCount`、`commentCount`、`hyperlinkCount`、`tableCount`、`animCount`、`masterCount`、`hasHandout`、`props`，有 `scope.masters` 时再跟 `masters`，有 `scope.handout` 时再跟 `handout`，然后 `slides`。`props` 始终出现，规则见 [json-contract.md](json-contract.md#文档属性三格式共用)。
- `shapeId` 是 `cNvPr/@id` 的原文。走 `shape:index:{k}` 时为 `null`。
- 形状上的 `name` 取 `cNvPr/@name` 的原文。没有则为 `null`。
- `x` / `y` 取 `p:spPr/a:xfrm/a:off` 的 EMU；`cx` / `cy` 取 `a:ext`。缺对应属性则为 `null`。备注形状同样。
- `text` 是可见纯文本：该形状 `p:txBody` 内、位于 `a:r` 里的 `a:t` 按文档顺序拼接；`a:br` 变成 `\n`。不含 run 级样式。`a:fld` 是应用程序更新的生成文本（§21.1.2.2.4），域里的 `a:t` 是缓存，不进入。`mc:AlternateContent` 只取一支，判定见上文 pptx 那节。
- `stable`：有 `cNvPr/@id` 为 `true`，否则为 `false`。
- 第一期不返回完整动画时间轴树。效果摘要见上文 `scope.timing`。母版与版式见上文 `scope.masters`。讲义母版槽位见上文 `scope.handout`。切换见上文 `transition`。批注内图本期不进树。对批注 id 发 `insertPicture` 报 `InvalidId`。


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
| `presentation.props` | 可选。文件属性，字段与 [json-contract.md](json-contract.md#setprops) 的 `setProps` / `setCustomProp` 相同 |
| `slides[].shapes` | 该页文本框列表。可空；空则生成一个空文本框 |
| `slides[].pictures` | 可选。该页上的图列表。省略或空数组则不写 `p:pic` |
| `slides[].charts` | 可选。该页上的图表列表。省略或空数组则不写图表 `p:graphicFrame` |
| `slides[].smartArts` | 可选。该页上的 SmartArt 列表。省略或空数组则不写图示 `p:graphicFrame` |
| `slides[].tables` | 可选。该页上的 DrawingML 表列表。省略或空数组则不写表 `p:graphicFrame` |
| `slides[].notes` | 可选。该页演讲者备注可见纯文本，字符串，可空。省略则不写 notesSlide。出现（含空串）则按 `setNotesText` 写该页 `body` 占位符 |
| `slides[].comments` | 可选。该页现代批注列表。省略或空数组则不写批注 |
| `shapes[].text` | 可见文本，可空字符串。省略当作 `""`。`\n` 写成 `a:br` |
| `pictures[].file` | 本地路径，与 `insertPicture` 相同，按文件头认 PNG / JPEG / GIF / BMP。缺此字段报 `MissingField` |
| `pictures[].x` / `pictures[].y` | 可选，EMU，非负整数。必须成对出现或成对省略；只给一个时报 `InvalidField`。缺则放到该页空白处，规则与 pptx `insertPicture` 相同 |
| `pictures[].cx` / `pictures[].cy` | 可选，EMU，正整数。都省则按图片像素以 96 dpi 换算。只给一个时报 `InvalidField` |
| `charts[].type` | 必须。与 pptx `insertChart` 相同 |
| `charts[].series` | 必须。非空。字段与 `insertChart` 相同 |
| `charts[].title` | 可选 |
| `charts[].x` / `charts[].y` / `charts[].cx` / `charts[].cy` | 可选。规则与 pptx `insertChart` 相同 |
| `smartArts[].layout` | 必须。`list`、`org` 或 `cycle`，与 pptx `insertSmartArt` 相同 |
| `smartArts[].points` | 可选。与 `insertSmartArt` 相同；省略则用该版式的缺省占位点 |
| `smartArts[].x` / `smartArts[].y` / `smartArts[].cx` / `smartArts[].cy` | 可选。规则与 pptx `insertSmartArt` 相同 |
| `tables[].rows` / `tables[].cols` | 必须。正整数，规则与 pptx `insertTable` 相同 |
| `tables[].cells` | 可选。二维字符串，行数与列数必须与 `rows` / `cols` 一致 |
| `tables[].x` / `tables[].y` / `tables[].cx` / `tables[].cy` | 可选。规则与 pptx `insertTable` 相同 |
| `comments[].text` | 可选。省略当作 `""`。规则与 pptx `insertComment` 相同 |
| `comments[].author` | 可选。省略当作 `"ooxp"` |
| `comments[].initials` | 可选。省略当作 `"oo"` |
| `comments[].date` | 可选。`xsd:dateTime`。省略则写入当前 UTC（无尾 `Z`） |
| `comments[].x` / `comments[].y` | 可选，EMU，非负整数。必须成对出现或成对省略；只给一个时报 `InvalidField`。缺则放到该页空白处，规则与 pptx `insertComment` 相同 |

带图示例见 `docs/examples/pptx-create-pictures.json`。带图表示例见 `docs/examples/pptx-create-charts.json`。带 SmartArt 示例见 `docs/examples/pptx-create-smartart.json`。带表示例见 `docs/examples/pptx-create-tables.json`。带备注与批注示例见 `docs/examples/pptx-create-notes-comments.json`。`kind` / `wrap` / `from` / `to` / `anchor` / `dataFrom` 以及 xlsx 偏移字段出现报 `InvalidField`。`comments[]` 上出现 `op` / `id` / `start` / `end` / `replies` / `ink` 报未知字段。create 不写回复、不写墨迹、不写 ECMA-376 §19.4 旧式批注。

`workbook`、`document`、`presentation` 三者必须恰好出现一个。xlsx 路径必须用 `workbook`，docx 路径必须用 `document`，pptx 路径必须用 `presentation`，对不上报 `InvalidField`。

create 写出 Transitional 最小包，见 [opc.md](opc.md)。必须带 theme、slideMaster、slideLayout、presentation、各页 slide、presProps，不能只写 ECMA 示例里那个仅含 `notesSz` 的 presentation。幻灯片尺寸用 16:9（`sldSz` 为 `cx="12192000"` `cy="6858000"`）。不要生成动画、切换、讲义。切换请用 `setTransition`，不要在 create 的 `slides[]` 上写 `transition` 或 `timing`。给了 `slides[].pictures` 时按 `insertPicture` 同一套规则写 `/ppt/media/…`、该页 `.rels` 与 `p:pic`。给了 `slides[].charts` 时按 `insertChart` 同一套规则写 `/ppt/charts/…`、embedding 与图表 `p:graphicFrame`。给了 `slides[].smartArts` 时按 `insertSmartArt` 同一套规则写 `/ppt/diagrams/…` 与图示 `p:graphicFrame`。给了 `slides[].tables` 时按 `insertTable` 同一套规则写表 `p:graphicFrame`（`a:tbl`）。给了 `slides[].notes` 时按 `setNotesText` 同一套规则写该页演讲者备注：没有 notesMaster 则新建，再写 notesSlide。给了 `slides[].comments` 时按 `insertComment` 同一套规则写该页现代批注（`p188:cmLst`、`/ppt/authors.xml`）。先写该页 `shapes`，再按数组顺序写 `pictures`，再写 `charts`，再写 `smartArts`，再写 `tables`，再写 `notes`，再按数组顺序写 `comments`。`p:cNvPr/@id` 取该页已有全部 `cNvPr` 的最大整数加 1，至少为 2。`presentation` 根上或 `shapes[]` 上出现 `pictures` / `charts` / `smartArts` / `tables` / `notes` / `comments` / `masters` / `handout` / `hyperlinks` 报 `InvalidField`。`slides[]` 上出现 `hyperlinks` 报未知字段。create 本期不写超链接。create 仍只写一份 `type="blank"`、名为「空白」的版式，不按 JSON 生成多份母版。给了 `presentation.props` 时按 `setProps` / `setCustomProp` 同一套规则写 core / app / custom；省略则仍写空的 `dc:title` 与 `dc:creator` 为 `ooxp`，不写 custom 部件。

每个文本框写成带 `p:txBody` 的 `p:sp`，`p:cNvPr/@id` 从 `2` 起在该页内递增（`1` 留给 `spTree`）。随后 inspect 得到的 id 都是 `slide:{n}/shape:{id}`，`stable` 为 `true`。形状纵向错开，避免叠在同一位置。颜色用 RGB，不依赖主题色语义。


## apply

各格式认哪些 `op` 以 [json-contract.md](json-contract.md#apply) 那一段名单为准。跨格式 op 的字段表也在主文件。

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

### insertSlide

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    { "op": "insertSlide", "id": "pptx:presentation", "after": "slide:1" }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertSlide"` |
| `id` | 必须是 `pptx:presentation` |
| `after` | 可选。已有页的 `slide:{n}`。省略则追加到 `sldIdLst` 末尾；给出则插到该页之后 |

pptx：`id` 不是 `pptx:presentation` 报 `InvalidId`。`after` 找不到报 `IdNotFound`；不是页 id 报 `InvalidId`。新建空白幻灯片部件（`p:spTree` 只有组合框，不抄版式占位符，不写 `<p:timing>`），关系指向一份已有 `slideLayout`：优先抄 `after` 那一页的版式关系，省略 `after` 则抄最后一页；该页没有版式关系则用第一份幻灯片母版的第一份版式。找不到任何版式报 `InvalidField`。`p:sldId/@id` 取已有最大 `ST_SlideId` 加 1，至少为 256。部件路径用尚未占用的 `/ppt/slides/slideN.xml`，不改已有页的文件名。若 presentation 带 `p14:sectionLst`，把新页的 `ST_SlideId` 写进 `after` 那一页所在节（省略 `after` 则跟最后一页那一节），使各节合起来与 `p:sldIdLst` 同序同成员。不改自定义放映名单。docx / xlsx 上发此 op 报 `InvalidField`。

### deleteSlide

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteSlide"` |
| `id` | 要删除的 `slide:{n}` |

找不到该页报 `IdNotFound`。id 不是页 id 报 `InvalidId`。删掉演示文稿里最后一页报 `InvalidField`，不写文件。去掉 `sldIdLst` 表项、presentation 关系、内容类型，以及该页部件及其 `.rels`。该页备注 `notesSlide`、现代批注部件、图表 / embedding / diagram / 媒体若再无人引用则删。不删幻灯片版式、母版、主题、notesMaster。自定义放映与 `p14:sectionLst` 里对应该页 `ST_SlideId` 的条目一并去掉。未点名页原字节写回。docx / xlsx 上发此 op 报 `InvalidField`。

### moveSlide

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"moveSlide"` |
| `id` | 要移动的 `slide:{n}` |
| `after` | 可选。目标前一页的 `slide:{n}`。省略或 `null` 表示移到名单最前 |

找不到该页或 `after` 报 `IdNotFound`。id / `after` 不是页 id 报 `InvalidId`。`after` 与 `id` 是同一页报 `InvalidField`。已经在目标位置则空操作。只改 `presentation.xml` 里 `sldIdLst` 子元素顺序，以及若有 `p14:sectionLst` 则按同一套顺序把该 `ST_SlideId` 挪到 `after` 那一页所在节（移到最前则进第一节）。不改部件路径、`p:sldId/@id` 或关系 id。docx / xlsx 上发此 op 报 `InvalidField`。

### setSlideHidden

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setSlideHidden"` |
| `id` | `slide:{n}` |
| `hidden` | `true` 隐藏，`false` 显示 |

找不到该页报 `IdNotFound`。id 不是页 id 报 `InvalidId`。缺 `hidden` 报 `InvalidField`。`true` 在该页 `p:sld` 上写 `show="0"`。`false` 去掉 `show`。已经是目标可见性则空操作。允许把全部页藏起来。不改 `sldIdLst`。docx / xlsx 上发此 op 报 `InvalidField`。

### setTransition

写或清除该页 `<p:transition>`。不写 `<p:timing>`。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTransition"` |
| `id` | `slide:{n}` |
| `type` | 效果名，或 `null` 表示去掉切换。必填 |
| `dur` | 可选。时长，毫秒，整数，范围 `1`–`60000` |

找不到该页报 `IdNotFound`。id 不是页 id 报 `InvalidId`。缺 `type` 报 `InvalidField`。`type` 为 `other`、空串、或不在 inspect 那份可写名单里（`none` 可写，表示空的 `<p:transition>` 且没有效果子元素）报 `InvalidField`。`dur` 不是正整数或不在范围内报 `InvalidField`。`type` 为 `null` 时忽略 `dur`，删掉该页的 `<p:transition>`；若它包在只含切换的 `mc:AlternateContent` 里，整段 AlternateContent 删掉。已经没有切换则空操作。

`type` 不是 `null` 时整段换成新的 `<p:transition>`（含原先的 AlternateContent）。ECMA 名单里的效果写成 `p:` 子元素，直接放在 `p:sld` 下，不包 AlternateContent。MS-PPTX 名单里的效果写成 `p14:` / `p15:`（`morph` 用 `p15`）子元素，外包 `mc:AlternateContent`：Choice 要求对应前缀，Fallback 写 `p:fade`。不写 `sndAc`。`dur` 省略则按 `500`。同时写 `p14:dur` 与 `spd`：`dur<=500` 为 `fast`，`dur<=750` 为 `med`，其余为 `slow`。不改 `advClick` / `advTm` 以外的未点名属性——新元素不带这两项，即单击进片、不自动进片。不改 `<p:timing>`。docx / xlsx 上发此 op 报 `InvalidField`。

### setSlideLayout

只改该页 `.rels` 里指向 `slideLayout` 的关系。不改幻灯片 XML，不删用户形状，不把版式占位符抄进页，不写 `<p:timing>`。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setSlideLayout"` |
| `id` | 必须是 `slide:{n}` |
| `layout` | 必须是 `master:{n}/layout:{k}` |

找不到该页报 `IdNotFound`。`id` 不是页 id 报 `InvalidId`。`layout` 形态不对报 `InvalidId`。找不到该版式报 `IdNotFound`。缺 `layout` 报 `InvalidField`。已经是目标版式则空操作，仍计入 `opsApplied`。已有 `slideLayout` 关系则只改 `Target`；没有则补一条。不改母版 XML、不改版式 XML、不改 `presentation.xml`。docx / xlsx 上发此 op 报 `InvalidField`。

### setChartPos

字段与 `setPicturePos` 相同，`id` 为 pptx 图表 id。写入 graphicFrame 的 `p:xfrm/a:off`。docx / xlsx 上发此 op 报 `InvalidField`。

### setSmartArtPos

字段与 `setPicturePos` 相同，`id` 为 pptx 图示 id。写入 graphicFrame 的 `p:xfrm/a:off`。docx / xlsx 上发此 op 报 `InvalidField`。

### insertShape

往幻灯片或备注页插入一个带 `p:txBody` 的矩形文本框。不抄版式占位符，不写 `<p:timing>`。

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    { "op": "insertShape", "id": "slide:1", "text": "新框", "x": 914400, "y": 914400 }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"insertShape"` |
| `id` | 必须是 `slide:{n}` 或 `slide:{n}/notes`（与 `insertPicture` 相同） |
| `text` | 可选。可见纯文本，省略则空。`\\n` 写成 `a:br` |
| `x` / `y` | 可选。画布坐标，EMU，非负整数。必须成对出现或成对省略 |
| `cx` / `cy` | 可选。尺寸，EMU，正整数。必须成对出现或成对省略。都省略时默认 `1828800` × `914400`（2 英寸 × 1 英寸） |

pptx：`id` 不是页 id 或备注容器 id 报 `InvalidId`（对形状 id、图 id、母版 id 或 `pptx:presentation` 发本 op 也是这条）。找不到该页报 `IdNotFound`。只给 `x` 或只给 `y`、只给 `cx` 或只给 `cy` 报 `InvalidField`。缺坐标时在画布上找空位（幻灯片读 `p:sldSz`，备注读 `p:notesSz`），不要叠在 `(0,0)`。`id` 为 `slide:{n}/notes` 且该页还没有 notesSlide 时，按 `setNotesText` 同一套新建最小备注页再插入。

写入该宿主 `p:spTree` 一个新 `p:sp`（插在 `extLst` 之前）：`p:cNvSpPr/@txBox` 为 `1`，`a:prstGeom/@prst` 为 `rect`，默认 `a:noFill` 与 `a:ln/a:noFill`，始终带 `p:txBody`。新 `cNvPr/@id` 取该页（含 `spTree` 的 `id="1"`）已有最大加 1，至少为 2。`cNvPr/@name` 写成「文本框 {该 id}」。不改未点名形状，不改已有 `<p:timing>`。docx / xlsx 上发此 op 报 `InvalidField`。

### deleteShape

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"deleteShape"` |
| `id` | 形状 id（`slide:{n}/shape:…` 或备注上的 `slide:{n}/notes/shape:…`） |

找不到该形状报 `IdNotFound`。id 不是形状 id 报 `InvalidId`。对母版或版式占位符 id 发本 op 报 `InvalidId`（本期不在母版上插删形状）。对备注页上占位符 `p:ph/@type` 为 `sldImg` 的形状发本 op 报 `InvalidField`，不写文件。删掉对应 `p:sp`。若在 `mc:AlternateContent` 的 Choice 里，整段 `AlternateContent` 删掉。组合本身不进树；删组合内子 `sp` 后若组合空了，把空 `p:grpSp` 一并摘掉。不改其它形状的 `cNvPr/@id`。按被删形状的 `cNvPr/@id` 清掉本页 `<p:timing>` 里对应的 `p:tgtEl`（其子为 `p:spTgt/@spid`）；祖先因此变空则一并摘掉，未点名形状的 timing 留下。不要整节无故删掉仍有内容的 `<p:timing>`。docx / xlsx 上发此 op 报 `InvalidField`。

### setShapePos

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setShapePos"` |
| `id` | 形状 id（幻灯片 / 备注上的 `slide:…/shape:…`，或母版 / 版式占位符 `master:…/shape:…`） |
| `x` / `y` | 至少给一个。EMU，非负整数 |

写入该 `p:sp` 的 `p:spPr/a:xfrm/a:off`。没有 `xfrm` 则补。找不到报 `IdNotFound`。id 不是形状 id 报 `InvalidId`。`x` 与 `y` 一个都不给报 `InvalidField`。对 `sldImg` 占位符报 `InvalidField`。母版 / 版式占位符只改对应 slideMaster / slideLayout 部件，不改各页 XML。docx / xlsx 上发此 op 报 `InvalidField`。

### setShapeSize

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setShapeSize"` |
| `id` | 形状 id（幻灯片 / 备注上的 `slide:…/shape:…`，或母版 / 版式占位符 `master:…/shape:…`） |
| `cx` / `cy` | 至少给一个。EMU，正整数 |

写入该 `p:sp` 的 `p:spPr/a:xfrm/a:ext`。没有 `xfrm` 则补。找不到报 `IdNotFound`。id 不是形状 id 报 `InvalidId`。`cx` 与 `cy` 一个都不给报 `InvalidField`。对 `sldImg` 占位符报 `InvalidField`。母版 / 版式占位符只改对应 slideMaster / slideLayout 部件。docx / xlsx 上发此 op 报 `InvalidField`。

### setShapeFill

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setShapeFill"` |
| `id` | 形状 id |
| `color` | 必填。六位 RGB 十六进制，或 `null` 表示无填充。不写 theme 色 |

写入 `p:spPr` 的 `a:solidFill/a:srgbClr`；`null` 则写成 `a:noFill`，并去掉原有填充。缺 `color` 或非法十六进制报 `InvalidField`。找不到报 `IdNotFound`。id 不是形状 id 报 `InvalidId`。对母版或版式占位符 id 发本 op 报 `InvalidId`。对 `sldImg` 占位符报 `InvalidField`。docx / xlsx 上发此 op 报 `InvalidField`。pptx 上发 `setTextBoxFill` 仍报 `InvalidField`。

### setShapeLine

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setShapeLine"` |
| `id` | 形状 id |
| `color` | 必填。六位 RGB 十六进制，或 `null` 表示无边框 |
| `width` | 可选。线宽，EMU，正整数。无边框时必须省略 |

写入 `p:spPr/a:ln`。`color` 为 `null` 时写成 `a:ln/a:noFill`；此时出现 `width` 报 `InvalidField`。缺 `color` 或非法十六进制报 `InvalidField`。找不到报 `IdNotFound`。id 不是形状 id 报 `InvalidId`。对母版或版式占位符 id 发本 op 报 `InvalidId`。对 `sldImg` 占位符报 `InvalidField`。docx / xlsx 上发此 op 报 `InvalidField`。pptx 上发 `setTextBoxLine` 仍报 `InvalidField`。

本期不改组合几何，不插连接线，不给无 `txBody` 的旧形状补框。

### setTableCellText

改幻灯片 DrawingML 表某一格的可见文字。手术刀规则与 `setShapeText` 相同，目标是该 `a:tc` 的 `a:txBody`，不整格重建。

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTableCellText"` |
| `id` | 单元格 id，`slide:{n}/table:…/r:{r}/c:{c}` |
| `text` | 新的可见纯文本，字符串，可空。必填。`\\n` 写成 `a:br` |

找不到该格报 `IdNotFound`。id 不是单元格 id 报 `InvalidId`。缺 `text` 报 `InvalidField`。没有 `a:txBody` 时报 `IdNotFound`，不补框。docx / xlsx 上发此 op 报 `InvalidField`。xlsx 上改工作表格子请用 `setCell`。不要对 pptx 表发 `setShapeText`。

### setTablePos

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTablePos"` |
| `id` | 表 id |
| `x` / `y` | 至少给一个。EMU，非负整数 |

写入该 `p:graphicFrame` 的 `p:xfrm/a:off`。没有 `xfrm` 则补。只改画布坐标，不重分配 `gridCol/@w` 与 `tr/@h`。找不到报表 `IdNotFound`。id 不是表 id 报 `InvalidId`。`x` 与 `y` 一个都不给报 `InvalidField`。docx / xlsx 上发此 op 报 `InvalidField`。

### setTableSize

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setTableSize"` |
| `id` | 表 id |
| `cx` / `cy` | 至少给一个。EMU，正整数 |

写入该 `p:graphicFrame` 的 `p:xfrm/a:ext`。没有 `xfrm` 则补。只改框尺寸，不算自动列宽。找不到报表 `IdNotFound`。id 不是表 id 报 `InvalidId`。`cx` 与 `cy` 一个都不给报 `InvalidField`。docx / xlsx 上发此 op 报 `InvalidField`。

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

只改备注页上占位符 `p:ph/@type` 为 `body` 的那个文本框。文档顺序中第一个 `body` 占位符命中。找不到该页报 `IdNotFound`。`id` 不是上述两种报 `InvalidId`。没有 notesSlide 时新建最小 Transitional 备注页：若包内还没有 notesMaster，则补 `/ppt/notesMasters/notesMaster1.xml`、presentation 关系、`p:notesMasterIdLst` 与内容类型；theme 部件按已有 theme 复制一份新的 `/ppt/theme/themeN.xml`，不和 slideMaster 共用同一部件（PowerPoint 16.0 拒开共用 theme 的备注母版，规则与 `setHandoutText` 的 handoutMaster 相同）；再写 `/ppt/notesSlides/notesSlideN.xml`，该页 `.rels` 指向 notesMaster 与对应幻灯片。新建的备注页含 `sldImg`、`body`、`sldNum` 三个占位符，结构对照 PowerPoint 16.0 另存样例，不写 `p14:creationId`。已有备注页却没有 `body` 占位符时，补一个 `body` 再写字。不改 `sldImg` 缩略图占位符。xlsx / docx 上发此 op 报 `InvalidField`。`create` 给了 `slides[].notes` 时按同一套规则写演讲者备注。

示例见 `docs/examples/pptx-apply-notes.json`。inspect 示例见 `docs/examples/pptx-inspect-notes.json`。批注示例见 `docs/examples/pptx-apply-comments.json` 与 `docs/examples/pptx-inspect-comments.json`。

### setHandoutText

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "ops": [
    { "op": "setHandoutText", "id": "handout", "footer": "讲义页脚" }
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

四个槽位至少给一个，否则报 `InvalidField`。未点名的槽位留下。`id` 不是 `handout` 报 `InvalidId`。不要和 `setNotesText` 混用：本 op 只改 `/ppt/handoutMasters/…`，不改 notesSlide。对 `handout` 发 `setShapeText` / `insertShape` / `deleteShape` / `setShapeFill` / `setShapeLine` 报 `InvalidId`。

只改文档顺序中该类型的第一个占位符。没有 handoutMaster 时新建最小 Transitional 讲义母版：写 `/ppt/handoutMasters/handoutMaster1.xml`，presentation 关系、`p:handoutMasterIdLst`（插在 `notesMasterIdLst` 之后，没有则插在 `sldMasterIdLst` 之后）与内容类型。theme 关系指向**新复制**的 theme 部件（内容与包内已有 theme 相同），不和 slideMaster 共用同一部件。新建部件含 `hdr`、`dt`、`ftr`、`sldNum` 四个占位符，结构对照 notesMaster 的页眉页脚框，不写 `sldImg`，不写 `p14:creationId`。已有讲义母版却没有被点名类型的占位符时，补一个该类型再写字。写入规则与 `setShapeText` 相同（整框写成新 run）；日期或页码原先若是 `a:fld`，改完变成普通文字，不再自动更新。该 handoutMaster 若已有 `p:hf`，把被点名槽位对应属性写成 `1`。不写 `<p:timing>`。xlsx / docx 上发此 op 报 `InvalidField`。`create` 本期不写讲义。

示例见 `docs/examples/pptx-apply-handout.json`。inspect 示例见 `docs/examples/pptx-inspect-handout.json`。

### 改动范围（pptx）

允许变脏的部件：被改的那一张幻灯片 XML（通常是 `/ppt/slides/slideN.xml`，N 以关系目标为准）及其 `.rels`，被改的 notesSlide 及其 `.rels`，必要时 `presentation.xml` 及其 `.rels`、新建的 notesMaster 及其 `.rels`，`[Content_Types].xml`，以及新建或被删的 `/ppt/media/…`、`/ppt/charts/…`、embedding、`/ppt/diagrams/…` 及可选 drawing 缓存，以及现代批注部件 `/ppt/comments/modernComment_*.xml` 与 `/ppt/authors.xml`。`setShapeText` 以及 `setShapePos` / `setShapeSize` / `setShapeFill` / `setShapeLine` / `deleteShape` 只脏目标宿主 XML（幻灯片、notesSlide、被点名的 slideMaster 或 slideLayout）。`insertShape` 往幻灯片插只脏该页 XML；往备注插时可脏 notesSlide、该页 `.rels`、notesMaster、presentation，与 `setNotesText` 相同。`insertTable` / `deleteTable` / `insertRow` / `deleteRow` / `insertCol` / `deleteCol` / `setTableCellText` / `setTablePos` / `setTableSize` 只脏该页幻灯片 XML，不新建部件。`setNotesText` 可脏 notesSlide、该页 `.rels`、notesMaster、presentation，以及新建 notesMaster 时复制的 theme 部件。图片 op 还可脏该宿主关系与媒体。图表 op 还可脏 chart 部件、embedding。SmartArt op 还可脏四件套与缓存。批注 op 还可脏该页 comments 部件、幻灯片 XML（新建时的 `commentRel`）、presentation `.rels` 与 authors。`insertSlide` / `deleteSlide` / `moveSlide` 还可脏 `presentation.xml` 的 `sldIdLst`（及若有则 `p14:sectionLst`）、presentation 关系、新页或被删页及其 `.rels`、内容类型，以及被删页无人引用的备注 / 现代批注 / 图表 / embedding / diagram / 媒体。`setSlideHidden` 只脏该页 `p:sld/@show`。`setTransition` 只脏该页幻灯片 XML 的 `<p:transition>`（或包住它的 `mc:AlternateContent`），不脏 timing。`setSlideLayout` 只脏该页 `.rels` 的 `slideLayout` 目标，不脏幻灯片 XML、母版 XML 或版式 XML。`setHandoutText` 可脏 handoutMaster 及其 `.rels`、必要时 `presentation.xml` 及其 `.rels` 与 `[Content_Types].xml`，以及新建 handoutMaster 时复制的 theme 部件。超链接 op 脏该页幻灯片 XML 与该页 `.rels`（新建或删除 hyperlink / slide 关系）。其他页、未点名的幻灯片母版、未点名的版式、主题、`presProps` 一律原样写回。一次 `ops` 可以混用 `setShapeText`、形状管理 op、表格 op、`setNotesText`、`setHandoutText`、批注 op、超链接 op、图片 op、图表 op、SmartArt op、幻灯片管理 op（含 `setTransition`、`setSlideLayout`）与 `setProps` / `setCustomProp` / `deleteCustomProp`。文件属性 op 还可以脏 `/docProps/core.xml`、`/docProps/app.xml`、`/docProps/custom.xml`、根 `.rels` 与 `[Content_Types].xml`。只改 core 字段时不要新建 custom。未点名的 `Slides` 留下。只改 SmartArt 点文字或增删点、且宿主 graphicFrame 未动时，该页 XML 按「drawing 未动」验收。

被改部件同样整份重序列化，保留程度与 xlsx 工作表相同：元素与属性顺序、未知命名空间保留；注释、处理指令、DOCTYPE 会丢。

