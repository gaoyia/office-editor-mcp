# pptx 人工检查清单

代码阶段的单元测试覆盖 JSON 解析和 XML 不变量。下面这些必须用 Microsoft PowerPoint 打开一次，不能只靠测试绿。验收只认官方 Office，不再要求 WPS。

检查前用 create 或 apply 在本机生成文件，不要用改过扩展名的 zip 目录冒充。本机若 `PowerPoint.Application` 被 WPS 抢走，直接启动 `POWERPNT.EXE` 再挂 COM，不要走被抢的 ProgID。Office 16 的对象模型在 `mdiClass` 上。

C、I、A、Pp、Pc、Psa、Pn、Pcm、Pm、Ps、Pt、Tn、Tr、Am、Aw、Mm、Ho、Phl、Pmd、Psc 各组由 `manual-check/office/cases/pptx-*.ps1` 自动判定，跑 `pwsh -File manual-check\office\run.ps1 -Case 'pptx-*'` 即可复现。O1 已放弃，与 xlsx / docx 相同。

## 本机结果（2026-09-28）

Microsoft PowerPoint 16.0 已打开，C1、C3、C4、C5–C13、I1–I5、A1–A5、Pp1–Pp7、Pc1–Pc7、Psa1–Psa7、Pn1–Pn6、Pcm1–Pcm6、Pik1–Pik4、Pm1–Pm5、Ps1–Ps5、Pt1–Pt6、Tn1–Tn5、Am1–Am5、Mm1–Mm5、Ho1–Ho5、Phl1–Phl5、Pmd1–Pmd5、Psc1–Psc5 均 PASS。本机若装了 WPS，`PowerPoint.Application` 会被抢走，测试仍直接启动 `POWERPNT.EXE`，从 `mdiClass` 挂 COM。O1 已放弃。2026-09-27 补跑 `pptx-pictures.ps1`，Pp1–Pp7 通过。同日补跑 `pptx-create-pictures.ps1`，C5–C7 通过。同日补跑 `pptx-charts.ps1`，Pc1–Pc7 通过。同日补跑 `pptx-create-charts.ps1`，C8–C10 通过。同日补跑 `pptx-smartart.ps1`，Psa1–Psa7 通过。同日补跑 `pptx-notes.ps1`，Pn1–Pn4 通过。同日补跑 `pptx-comments.ps1`，Pcm1–Pcm5、Pik1–Pik4 通过。同日补跑 `pptx-docprops.ps1`，Pd1–Pd5 通过。同日补跑 `pptx-charts-remain.ps1`，Pr1–Pr3 通过。2026-09-28 补跑 `pptx-slides.ps1`，Pm1–Pm5 通过。同日补跑 `pptx-shapes.ps1`，Ps1–Ps5 通过。同日补跑 `pptx-tables.ps1`，Pt1–Pt6 通过。同日补跑 `pptx-transition.ps1`，Tn1–Tn5 通过。同日补跑 `pptx-timing.ps1`，Am1–Am5 通过。同日补跑 `pptx-masters.ps1`，Mm1–Mm5 通过。同日补跑 `pptx-handout.ps1`，Ho1–Ho5 通过。同日补跑 `pptx-hyperlinks.ps1`，Phl1–Phl5 通过。同日补跑 `pptx-create-notes-comments.ps1`，C11–C13 通过。同日补跑 `pptx-notes.ps1`，Pn5–Pn6 通过。同日补跑 `pptx-comments.ps1`，Pcm6 通过。同日补跑 `pptx-media.ps1`，Pmd1–Pmd5 通过。同日补跑 `pptx-sections.ps1`，Psc1–Psc5 通过。2026-09-29 补跑 `pptx-anim-write.ps1`，Aw1–Aw5 通过。同日补跑 `pptx-transition-remain.ps1`，Tr1–Tr5 通过。

| 序号 | 状态 | 结果 |
| --- | --- | --- |
| C1 | ✅ | PowerPoint 16.0 能打开 `manual-check/office/out/pptx-create.pptx`，标题栏无 Repair |
| C3–C4 | ✅ | 第一页 `标题` / `hello`，第二页 `第二页` |
| C5–C7 | ✅ | PowerPoint 16.0：inspect 第一张 `slide:1/pic:3`；同一页第二张 `slide:1/pic:4`；打开无 Repair，Type=13 的图为 2 |
| C8–C10 | ✅ | PowerPoint 16.0：inspect 第一张 `slide:1/chart:3`、`type=col`；系列 1,2,3 与 embedding B2/B4 一致；打开无 Repair，charts=1 |
| C11–C13 | ✅ | PowerPoint 16.0：inspect 第一页 `hasNotes=true`、`body` 为「演讲者备注」；根与第一页 `commentCount=1`、作者 Alice；打开无 Repair，备注窗格可见「演讲者备注」 |
| I1–I5 | ✅ | `pptx:presentation`、`slideCount=2`、无 scope 时没有 `shapes`、仍有 `pictureCount`、形状 id 为 `slide:1/shape:2` 且 `stable` |
| A1–A5 | ✅ | 目标框为「改过的正文」且仍加粗（`Bold=-1`），同页另一框与第二页未改 |
| Pp1–Pp7 | ✅ | PowerPoint 16.0：无 `pictures` 时 `pictureCount=1`；名单 id 为 `slide:1/pic:5`；换/插/删无修复；尺寸 144 磅；位置 72×108 磅；export 与 `/ppt/media/…` 字节一致 |
| Pc1–Pc7 | ✅ | PowerPoint 16.0：无 `charts` 时 `chartCount=1`；名单 id 为 `slide:1/chart:4`；改系列 / 插 / 删无修复；折线类型对上；尺寸 144×72 磅、位置 72×108 磅；export 与 embedding 字节一致 |
| Psa1–Psa7 | ✅ | PowerPoint 16.0：`pptx-smartart.ps1`，7 PASS / 0 FAIL。另存列表型 id 为 `slide:1/smartdiagram:4`；改字 / 增删点 / 删整张无修复；create `list` 为 `slide:1/smartdiagram:3`；列表改成组织图后文字还在、无修复；库外 uniqueId 得 `NotImplemented` 且未写文件 |
| Pn1–Pn6 | ✅ | PowerPoint 16.0：无 `notes` 时有 `hasNotes`；展开后 `notes.id` 为 `slide:1/notes`；`setNotesText` 后备注窗格是新字且无修复；改备注不改幻灯片字；往备注插图无修复；往备注插柱形图 id 为 `slide:1/notes/chart:5`，改系列后无修复且不并进幻灯片 `chartCount`；往备注插 `list` id 为 `slide:1/notes/smartdiagram:5`，无修复且不并进幻灯片 `smartArtCount` |
| Pcm1–Pcm6 | ✅ | PowerPoint 16.0：无 `comments` 时有 `commentCount`；展开后 id 为 `slide:…/comment:{guid}`；改字 / 插入 / 删除无修复；回复与完成标记能看见；注入 `a:blip` 后 `pictureCount=1`、`exportMedia` 字节一致，对批注 id 发 `insertPicture` 为 `InvalidId` |
| Pik1–Pik4 | ✅ | PowerPoint 16.0：`pptx-comments.ps1` 墨迹组，4 PASS / 0 FAIL。现代批注注入 `ink` 后 `kind=ink`；`setCommentMeta` / `setCommentDone` 无修复；无可见文本时 `setCommentText` 为 `NotImplemented` 且未写文件；`deleteComment` 无修复 |
| Pd1–Pd5 | ✅ | PowerPoint 16.0：`pptx-docprops.ps1`，5 PASS / 0 FAIL。根上 `props`、`creator=ooxp`；只改标题后面板标题是新字、公司名还在；自定义 `prop:部门`；删后名单空；形状 id 发 `setProps` 为 `InvalidId`。无 Repair |
| Pm1–Pm5 | ✅ | PowerPoint 16.0：`pptx-slides.ps1`，5 PASS / 0 FAIL。两页名单与 `hidden=false`；`insertSlide` 后第三页是原第二页、新页无文本框；`moveSlide` 到最前；`setSlideHidden` 后 COM Hidden 为真；删最后一页 / 缺 `hidden` 为 `InvalidField` 且未写文件。无 Repair |
| Ps1–Ps5 | ✅ | PowerPoint 16.0：`pptx-shapes.ps1`，5 PASS / 0 FAIL。`insertShape` 后 id 为 `slide:1/shape:4`；位置 72×108 磅、尺寸 144×72 磅、填充红；改字后旧框还在；删除后 COM 形状数为 2；另存样例插新框后旧框动画 Count 仍为 1；错 id / 缺成对 `cx` / 缺 `color` 为 `InvalidField` 且未写文件。无 Repair |
| Pt1–Pt6 | ✅ | PowerPoint 16.0：`pptx-tables.ps1`，7 PASS / 0 FAIL。另存样例 id 为 `slide:1/table:4`；改格 COM 可见「改过的正文」；增行后 COM Rows=3；`insertTable` 后 id 为 `slide:1/table:4` 且 `stable`、COM 格可见「甲」；`deleteTable` 后 COM 表数为 0；形状 id 得 `InvalidId`，缺 `rows` / 最后一行得 `InvalidField` 且未写文件。无 Repair |
| Tn1–Tn5 | ✅ | PowerPoint 16.0：`pptx-transition.ps1`，5 PASS / 0 FAIL。无切换页 `transition` 为 null；另存淡入 `type=fade`、`dur=500`；`setTransition` 后 COM EntryEffect=3849、Duration=0.5；只改字切换还在；`type: null` 后 COM 效果为 0；形状 id 得 `InvalidId`，`type=other` 得 `InvalidField` 且未写文件。无 Repair |
| Tr1–Tr5 | ✅ | PowerPoint 16.0：`pptx-transition-remain.ps1`，5 PASS / 0 FAIL。定时进片 `advTm` 对上 COM AdvanceOnTime；`wipe` `dir=r` 对上 EntryEffect=2819；`fallOver` 不是 `other`；只改字字段还在；错 `dir` 为 `InvalidField` 且未写文件。无 Repair |
| Am1–Am5 | ✅ | PowerPoint 16.0：`pptx-timing.ps1`，5 PASS / 0 FAIL。无 `timing` 时根与各页有 `animCount`、没有名单；另存进入淡入 `target=slide:1/shape:2`、`type=fade`、`trigger=click`；COM MainSequence.Count=1；只改字名单还在；`setTiming` 为 `UnsupportedOp`；xlsx / docx 的 `scope.timing` 为 `InvalidField`。无 Repair |
| Aw1–Aw5 | ✅ | PowerPoint 16.0：`pptx-anim-write.ps1`，5 PASS / 0 FAIL。create 后 `insertAnim` `type=fade`，名单一条且 COM Count=1；另存页再插一条旧的还在、新的在末尾；`deleteAnim` 后 Count 少 1，删光没有 `<p:timing>`；只改字名单还在；错 id / `type` 为 `InvalidId` 或 `InvalidField` 且未写文件。无 Repair |
| Mm1–Mm5 | ✅ | PowerPoint 16.0：`pptx-masters.ps1`，5 PASS / 0 FAIL。无 `masters` 时只有 `masterCount`；create 版式名「空白」；另存默认文稿列出「标题幻灯片」等；标题占位符改字无修复；插用户框后换成 `master:1/layout:7`（空白）用户框还在；占位符 `setShapePos` 几何有变；xlsx / docx 拒 `scope.masters` 与 `setSlideLayout`。无 Repair |
| Ho1–Ho5 | ✅ | PowerPoint 16.0：`pptx-handout.ps1`，5 PASS / 0 FAIL。无 `handout` 时只有 `hasHandout`；create 为 false、展开为 null；`setHandoutText` 写页脚 / 页眉后 COM 与 inspect 可见新字、无 Repair；xlsx / docx 拒 `scope.handout` 与 `setHandoutText`。 |
| Phl1–Phl5 | ✅ | PowerPoint 16.0：`pptx-hyperlinks.ps1`，5 PASS / 0 FAIL。无名单时根与各页有 `hyperlinkCount=0`；`insertHyperlink` 后 COM 单击地址对上且无 Repair；改地址后对得上；删除后框上字还在；对页 id 得 `InvalidId`，缺目标得 `InvalidField` 且未写文件。 |
| Pmd1–Pmd5 | ✅ | PowerPoint 16.0：`pptx-media.ps1`，8 PASS / 0 FAIL。无 `media` 时有 `videoCount` / `audioCount`、没有名单；另存视频 id 为 `slide:1/video:3`，不并进 `pictures`；`exportMedia` 与 `/ppt/media/media1.mp4` 字节一致；`insertVideo` 后 Type=16、id 为 `slide:1/video:4`、无 Repair；`insertAudio` 后 `audioCount` 加 1、无 Repair；`deleteMedia` 后对象消失；对 `slide:1/notes` 得 `InvalidId`，WAV 当视频得 `InvalidField` 且未写文件。 |
| Psc1–Psc5 | ✅ | PowerPoint 16.0：`pptx-sections.ps1`，5 PASS / 0 FAIL。无 `sections` 时有 `sectionCount=2`、没有名单；另存两节 id 为 `section:1`，`slides` 对得上页序；连续 `insertSlideSection` 两节再划第二页后 COM 第二节名为「内容」、FirstSlide=2、无 Repair；改名后左侧是「正文」；删节后页还在、节名没了；对页 id 得 `InvalidId`，`start` 与 `after` 同给得 `InvalidField` 且未写文件。 |
| Pr1–Pr3 | ✅ | PowerPoint 16.0：`pptx-charts-remain.ps1`，3 PASS / 0 FAIL。趋势线 COM Count=1；改删趋势线；次轴 `AxisGroup=2`。无 Repair |
| O1 | 🚫 | 领域层不做原样保存，与 xlsx 清单相同。零脏写回见 OPC 库测试。PowerPoint 另存 create 产物见 `pptx-O1-ppt-resave` |

## create

用 `docs/examples/pptx-create.json` 生成 `deck.pptx`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| C1 | PowerPoint 打开 | 无修复提示、无「文件损坏」 |
| C3 | 第一页 | 两个文本框，可见文字分别为 `标题`、`hello` |
| C4 | 第二页 | 一个文本框，可见文字为 `第二页` |
| C5 | 带图 id | 用 `docs/examples/pptx-create-pictures.json`（换成本机 PNG）生成；inspect `pictures: true` 时第一张 id 为 `slide:1/pic:3` |
| C6 | 同一页第二张 | 同上，名单里有两张图 |
| C7 | PowerPoint 打开带图产物 | 无修复；图能看见 |
| C8 | 带图表 id | 用 `docs/examples/pptx-create-charts.json` 生成；inspect `charts: true` 时第一张 id 为 `slide:1/chart:3`，`type` 为 `col` |
| C9 | 系列与 embedding | 系列值与 `exportChartData` 写出的表一致 |
| C10 | PowerPoint 打开带图表产物 | 无修复；图能看见 |
| C11 | 带备注 | 用 `docs/examples/pptx-create-notes-comments.json` 生成；inspect 无 `notes` 时第一页 `hasNotes` 为 `true`，第二页为 `false`；`scope.notes` 时第一页 `body` 为 `演讲者备注` |
| C12 | 带批注 | 同上；无名单时根与第一页 `commentCount` 为 `1`，第二页为 `0`；`scope.comments` 时第一条 `text` 为 `一条批注`、`author` 为 `Alice` |
| C13 | PowerPoint 打开带备注与批注产物 | 无修复；备注窗格可见 `演讲者备注` |

## inspect

对上面的文件发 `docs/examples/pptx-inspect-slides.json`（无 scope）和 `docs/examples/pptx-inspect.json`（`scope.slide` 为 `1`）。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| I1 | 根节点 | `id` 为 `pptx:presentation`，`slideCount` 为 `2` |
| I2 | 页名单 | 两页 id 为 `slide:1`、`slide:2`，`index` 为 `1`、`2` |
| I3 | 无 scope | 只发 path 时各页都没有 `shapes` 字段，仍有 `slideCount`、`pictureCount` 和 `slides` |
| I4 | 形状 id | 第一页两个形状 id 为 `slide:1/shape:{n}`，`stable` 为 `true` |
| I5 | 第一页文字 | 两个形状的 `text` 分别为 `标题`、`hello` |

## apply

准备一份 PowerPoint 亲手另存的样例（至少两页，第一页至少两个文本框，其中一个带加粗或别的 run 样式）。对它执行 `setShapeText`，改第一页某一个文本框的文字，id 用 inspect 返回的值。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| A1 | 打开 | Microsoft PowerPoint 能打开，无修复提示 |
| A2 | 目标形状 | 被点名的那个文本框可见文字变成请求里的 `text`。源文件若有多段，改成不含换行的 `text` 之后，框里不应再多出空行 |
| A3 | 同页未改形状 | 第一页其余文本框文字和位置还在 |
| A4 | 样式 | 目标形状若原先有加粗等 run 样式，改完后看起来还在（允许整框变成单一 run，但加粗还在） |
| A5 | 其余页 | 第二页文字和版式看起来不变；有动画的样例里动画仍绑在原形状上 |

## 幻灯片图

源文件必须是 PowerPoint 亲手另存，至少一张 `p:pic`。另备一张本地 PNG。不要用库 `create` 产物当带图样例。脚本：`manual-check/office/cases/pptx-pictures.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pp1 | inspect | 无 `pictures` 时根与各页有 `pictureCount`、没有名单；`pictures: true` 时 id 为 `slide:…/pic:…` |
| Pp2 | 换图 | `replacePicture` 后无修复；位置尺寸不变 |
| Pp3 | 插图 | `insertPicture` 往一页插图，能看见且无修复 |
| Pp4 | 删图 | `deletePicture` 后该图消失，无修复 |
| Pp5 | 尺寸 | `setPictureSize` 后 COM 或目视尺寸对上（允许 EMU 误差） |
| Pp6 | 位置 | `setPicturePos` 后图跟着走，无修复 |
| Pp7 | 导出 | `exportMedia` 与 `/ppt/media/…` 字节一致。不强制开 PowerPoint |

## 幻灯片图表

源文件必须是 PowerPoint 亲手另存，至少一张柱形图。不要用库 `create` 产物当唯一样例。脚本：`manual-check/office/cases/pptx-charts.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pc1 | inspect | 无 `charts` 时根与各页有 `chartCount`、没有名单；`charts: true` 时 id 为 `slide:…/chart:…`，系列值与 embedding 一致 |
| Pc2 | 改系列 | `setChartSeries` 后无修复；数据表是新值 |
| Pc3 | 插图 | `insertChart` 后能看见且无修复 |
| Pc4 | 删图 | `deleteChart` 后该图消失，无修复 |
| Pc5 | 改类型 | 改折线后按折线显示（允许样式重置），无修复 |
| Pc6 | 尺寸或位置 | `setChartSize` 或 `setChartPos` 后位置或尺寸对上，无修复 |
| Pc7 | 导出 | `exportChartData` 与 embedding 字节一致。不强制开 PowerPoint |

## 图表剩余

手测脚本 `manual-check/office/cases/pptx-charts-remain.ps1`。编号用 Pr。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pr1 | 趋势线 | ✅ `insertTrendline` 后 PowerPoint 无修复，COM 能看见趋势线 |
| Pr2 | 改删趋势线 | ✅ `setTrendline` / `deleteTrendline` 后无修复 |
| Pr3 | 次轴 | ✅ `setChartSeriesAxis` 后 `axis` 为 `secondary`，无修复 |

## SmartArt

源文件必须是 PowerPoint 亲手另存，至少一张列表型。不要用库 `create` 产物当唯一样例。脚本：`manual-check/office/cases/pptx-smartart.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Psa1 | inspect | 另存列表型；inspect 的点文本与屏幕上能看见的字一致 |
| Psa2 | 改字 | `setSmartArtText` 后无修复，目标点的字变了 |
| Psa3 | 增删点 | 增删点后条数对，PowerPoint 重排 |
| Psa4 | 删整张 | 删除整张后 Fallback 也不在（有则必须一起少） |
| Psa5 | create | 带一张 `list`，打开无修复 |
| Psa6 | 跨家族 | 列表改成组织图后可见点文字还在，无修复 |
| Psa7 | 库外版式 | 当前 uniqueId 不在库内三族时 `setSmartArtLayout` 得 `NotImplemented`，文件未写坏 |

## 备注

源文件必须是 PowerPoint 亲手另存，至少一页有演讲者备注文字。不要用库 `create` 产物当带备注的唯一样例。脚本：`manual-check/office/cases/pptx-notes.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pn1 | inspect | 无 `notes` 时各页有 `hasNotes`、没有 `notes` 对象；`notes: true` 时有备注页的 `notes.id` 为 `slide:…/notes`，能看见演讲者备注文字 |
| Pn2 | 改字 | `setNotesText` 后备注窗格是新字，无修复 |
| Pn3 | 互不影响 | 改备注后幻灯片上的 `setShapeText` 目标仍是原字；改幻灯片形状不改备注 |
| Pn4 | 备注插图 | 往 `slide:{n}/notes` 插图后无修复，图在备注页上 |
| Pn5 | 备注图表 | 往 `slide:{n}/notes` 插柱形图后 id 为 `slide:…/notes/chart:…`；改系列后 embedding 对得上；无修复。不并进幻灯片 `chartCount` |
| Pn6 | 备注 SmartArt | 往 `slide:{n}/notes` 插 `list` 后 id 为 `slide:…/notes/smartdiagram:…`；无修复。不并进幻灯片 `smartArtCount` |

## 批注

源文件必须是 PowerPoint 亲手另存，第一页至少两条线程化批注，其中一条带回复。不要用库 `create` 产物当带批注的唯一样例。脚本：`manual-check/office/cases/pptx-comments.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pcm1 | inspect | 无 `comments` 时根与各页有 `commentCount`、没有名单；`comments: true` 时 id 为 `slide:…/comment:{guid}`，作者与文字与审阅窗格对得上，回复挂在 `replies` |
| Pcm2 | 改字 | `setCommentText` 后无修复，目标批注是新字 |
| Pcm3 | 插入 | `insertComment` 后审阅窗格能看见新批注，无修复 |
| Pcm4 | 删除 | `deleteComment` 后该条消失，无修复 |
| Pcm5 | 回复与完成 | `replyComment` 后线程能看见回复；`setCommentDone` 后完成标记对上，无修复 |
| Pcm6 | 批注内图 | 注入 `a:blip` 后 `pictureCount` 为 1、id 为 `slide:…/comment:{guid}/pic:1`；`exportMedia` 字节与媒体部件一致。对批注 id 发 `insertPicture` 得 `InvalidId` |

## 批注墨迹

现代批注里带本地名为 `ink` 的标记。不把墨迹当位图列出。编号用 Pik。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pik1 | 名单 | ✅ `kind` 为 `ink` |
| Pik2 | 元数据 | ✅ `setCommentMeta` 后作者变了，无修复 |
| Pik3 | 完成 | ✅ `setCommentDone` 后 `done` 为 true，无修复 |
| Pik4 | 改字仍拒 | ✅ 没有可见文本时 `setCommentText` 得 `NotImplemented`，文件未写坏 |

## 文档属性

与 docx 同一套字段。脚本 `manual-check/office/cases/pptx-docprops.ps1`。编号用 Pd（不要跟图片的 Pp 组混）。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pd1 | inspect | 无 scope 时根上有 `props`；create 产物 `creator` 为 `ooxp` |
| Pd2 | 改标题 | `setProps` 只改 `title` 后 PowerPoint 文件属性标题是新字；未点名的公司名还在；无 Repair |
| Pd3 | 自定义 | `setCustomProp` 后树上 id 为 `prop:…`；PowerPoint 自定义属性能看见 |
| Pd4 | 删除 | `deleteCustomProp` 后名单没有该项 |
| Pd5 | 拒错 id | 对形状 id 发 `setProps` 得 `InvalidId`。不强制开 PowerPoint |

## 幻灯片管理

用库 create 产物再 apply。顺序跟 `sldIdLst`。脚本 `manual-check/office/cases/pptx-slides.ps1`。编号用 Pm。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pm1 | inspect | create 两页；`slideCount=2`，两页 `hidden=false`，id 为 `slide:1`、`slide:2` |
| Pm2 | 插入 | `insertSlide` `after=slide:1` 后 `slideCount=3`；新页是 `slide:2` 且无文本框；原第二页变成 `slide:3` 文字还在；PowerPoint 无 Repair |
| Pm3 | 重排 | 把 `slide:3` 移到最前；inspect 第一页是原先那页文字；无 Repair |
| Pm4 | 隐藏 | `setSlideHidden` `hidden: true` 后 inspect `hidden=true`；COM `SlideShowTransition.Hidden` 为真；无 Repair |
| Pm5 | 删除 | 只剩一页时 `deleteSlide` 得 `InvalidField` 且未写文件；三页里删一页后计数为 2、无 Repair。缺 `hidden` 得 `InvalidField` |

## 形状增删与几何

插文本框、改位置尺寸填色、删除。不删旧形状再重建。脚本 `manual-check/office/cases/pptx-shapes.ps1`。编号用 Ps（不要跟图的 Pp、幻灯片管理的 Pm 混）。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Ps1 | 插入 | ✅ 对 create 产物 `insertShape` 后新框可见；inspect id 为 `slide:…/shape:…` 且 `stable`；PowerPoint 无 Repair |
| Ps2 | 几何与填色 | ✅ `setShapePos` / `setShapeSize` 后位置尺寸对上；`setShapeFill` / `setShapeLine` 后填充与边框能看见；无 Repair |
| Ps3 | 改字 | ✅ 对新框 `setShapeText` 后文字是请求值；旧框文字还在 |
| Ps4 | 删除 | ✅ `deleteShape` 后该框消失；同页未删形状还在、`cNvPr/@id` 不变；无 Repair |
| Ps5 | 动画仍绑 | ✅ 对 PowerPoint 另存样例只插新框、不删带动画的旧框；旧框动画还在；无 Repair。对形状 id 发 `insertShape`、缺成对 `cx` 或缺 `color` 得 `InvalidField` 且未写文件 |

## 幻灯片表格

DrawingML 表（`a:tbl` 包在 `p:graphicFrame` 里）。不要把表编进 `shapes` / `charts` / `pictures`。源文件：PowerPoint 亲手另存，第一页至少一张表。不要用库 `create` 产物当带表的唯一样例。脚本：`manual-check/office/cases/pptx-tables.ps1`。编号用 Pt（不要跟形状的 Ps、图的 Pp 混）。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pt1 | inspect | ✅ 无 `tables` 时根与各页有 `tableCount`、没有名单；`tables: true` 时 id 为 `slide:…/table:…`，能看见单元格 `text` |
| Pt2 | 改格 | ✅ `setTableCellText` 后目标格可见字是请求值，无 Repair |
| Pt3 | 增行 | ✅ `insertRow` 后行数加 1，无 Repair |
| Pt4 | 插入 | ✅ 对 create 产物 `insertTable` 后新表可见；inspect id 为 `slide:…/table:…` 且 `stable`；无 Repair |
| Pt5 | 删除 | ✅ `deleteTable` 后该表消失；同页未删形状还在；无 Repair |
| Pt6 | 拒错 | ✅ 对形状 id 发 `insertTable` 得 `InvalidId`；缺 `rows`、只剩一行时 `deleteRow` 得 `InvalidField`；均未写文件 |

## 幻灯片切换

只动 `<p:transition>`，不写 `<p:timing>`。源文件：PowerPoint 亲手另存一页带淡入切换。脚本：`manual-check/office/cases/pptx-transition.ps1`。编号用 Tn。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Tn1 | inspect | ✅ 无切换的页 `transition` 为 `null`；另存淡入页 `type` 为 `fade`，有 `dur` |
| Tn2 | 设切换 | ✅ 对 create 产物 `setTransition` `type=fade` 后放映能看见淡入；inspect `type=fade`；无 Repair |
| Tn3 | 改字不丢 | ✅ 对另存带切换样例只 `setShapeText`，切换还在；无 Repair |
| Tn4 | 清除 | ✅ `type: null` 后 `transition` 为 `null`；无 Repair |
| Tn5 | 拒错 | ✅ 对形状 id 发 `setTransition` 得 `InvalidId`；`type` 为 `other` 得 `InvalidField`；均未写文件 |

## 切换剩余

自动进片、效果方向与 `p15:prstTrans`。仍只动 `<p:transition>`。脚本：`manual-check/office/cases/pptx-transition-remain.ps1`。编号用 Tr。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Tr1 | 自动进片 | ✅ create 产物 `setTransition` `type=fade` `advTm=2000` 后 inspect `advTm` 为 2000、`advClick` 为 true；COM `AdvanceOnTime` 为真、`AdvanceTime` 为 2；无 Repair |
| Tr2 | 方向 | ✅ `type=wipe` `dir=r` 后 inspect `dir` 为 `r`；COM EntryEffect 为 2819；无 Repair |
| Tr3 | prstTrans | ✅ `type=fallOver` 后 inspect `type` 为 `fallOver`（不是 `other`）；COM EntryEffect 为 3934；无 Repair |
| Tr4 | 改字不丢 | ✅ 先写 `wipe` `dir=r` 且 `advTm=2000`，再只 `setShapeText`，`dir` 与 `advTm` 还在；无 Repair |
| Tr5 | 拒错 | ✅ 对 `fade` 发 `dir` 得 `InvalidField`；对 `fade` 发 `invX` 得 `InvalidField`；均未写文件 |

## 动画清单

只读 `<p:timing>` 清单。写入见 `.plan/2026年-9月-29日-✅-pptx-动画写入.md`。旧「禁止 apply 写 timing」作废；`setTiming` 仍报 `UnsupportedOp`。源文件：PowerPoint 亲手另存一页带进入淡入动画。脚本：`manual-check/office/cases/pptx-timing.ps1`。编号用 Am。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Am1 | inspect | ✅ 无 `timing` 时根与各页有 `animCount`、没有名单；create 产物为 0；另存进入动画 `timing: true` 时 `target` 对上第一框 shape id |
| Am2 | COM | ✅ `TimeLine.MainSequence.Count` 与 `animCount` 一致；`trigger` 为 `click`；无 Repair |
| Am3 | 改字不丢 | ✅ 对另存带动画样例只 `setShapeText`，名单还在；无 Repair |
| Am4 | 无入口 | ✅ `setTiming` 得 `UnsupportedOp`，未写文件 |
| Am5 | 拒错 | ✅ xlsx / docx 的 `scope.timing` 得 `InvalidField` |

## 动画写入

按模板给对象加、删预设进入淡入。不实现放映引擎，也不接受整棵时间轴 JSON。脚本：`manual-check/office/cases/pptx-anim-write.ps1`。编号用 Aw。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Aw1 | 插入 | ✅ create 产物 `insertAnim` `type=fade` 后 inspect 名单一条，`id` 为 `slide:1/anim:1`，`target` 对上第一框，COM `MainSequence.Count` 为 1，无 Repair |
| Aw2 | 再插 | ✅ 另存已有动画的页再插一条，旧的还在，新的在末尾，COM Count 加 1，无 Repair |
| Aw3 | 删除 | ✅ `deleteAnim` 后 Count 少 1；删光则没有 `<p:timing>`，无 Repair |
| Aw4 | 改字不丢 | ✅ 只 `setShapeText`，名单还在，无 Repair |
| Aw5 | 拒错 | ✅ 错对象 id 得 `InvalidId`；`type` 不在白名单得 `InvalidField`；xlsx / docx 发 `insertAnim` 得 `InvalidField`；均未写文件。`setTiming` 仍为 `UnsupportedOp` |

## 母版与版式

inspect 列出 slideMaster / slideLayout 与占位符。apply 改占位符文字与几何，并用 `setSlideLayout` 只换页的版式关系。不算母版排版引擎。脚本：`manual-check/office/cases/pptx-masters.ps1`。编号用 Mm。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Mm1 | inspect | ✅ 无 `masters` 时根有 `masterCount`、没有名单；create 产物 `masterCount=1`、版式名「空白」；另存默认文稿列出「标题幻灯片」「标题和内容」「空白」等 |
| Mm2 | 改占位符字 | ✅ 对 `master:1/layout:2/shape:2` `setShapeText` 后文字为「改过的正文」；无 Repair |
| Mm3 | 换版式 | ✅ 插「用户框」后换成 `master:1/layout:7`（空白），用户框 id 为 `slide:1/shape:4`；无 Repair |
| Mm4 | 改几何 | ✅ 占位符 `setShapePos` 后 `x` 为 200000；无 Repair |
| Mm5 | 拒错 | ✅ xlsx / docx 的 `scope.masters` 与 `setSlideLayout` 得 `InvalidField`，未写文件 |

## 讲义

inspect 列出讲义母版页眉、页脚、日期、页码槽位。apply 用 `setHandoutText` 改字，没有 handoutMaster 则新建最小部件。不算讲义打印布局。脚本：`manual-check/office/cases/pptx-handout.ps1`。编号用 Ho。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Ho1 | inspect | 无 `handout` 时根有 `hasHandout`、没有 `handout` 对象；create 产物 `hasHandout=false`；`scope.handout: true` 时 `handout` 为 `null` |
| Ho2 | 改页脚 | `setHandoutText` 写 `footer` 后 inspect 能看见新字；PowerPoint 无 Repair |
| Ho3 | 改页眉 | 同一次或另一次 `setHandoutText` 写 `header` 后能看见新字；无 Repair |
| Ho4 | 新建 | 对 create 产物（原先没有讲义母版）发 `setHandoutText` 后 `hasHandout=true`；无 Repair |
| Ho5 | 拒错 | xlsx / docx 的 `scope.handout` 与 `setHandoutText` 得 `InvalidField`，未写文件 |

## 超链接

形状级 `a:hlinkClick`。inspect 计 `hyperlinkCount`，`scope.hyperlinks` 列出链。apply 能改地址、插、删。内部跳转用 `slide:{n}`。不写放映宏动作。脚本：`manual-check/office/cases/pptx-hyperlinks.ps1`。编号用 Phl。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Phl1 | inspect | 无 `hyperlinks` 时根与各页有 `hyperlinkCount`、没有名单；create 产物为 0；`scope.hyperlinks: true` 时名单为空数组 |
| Phl2 | URL | `insertHyperlink` 给第一框 `url` 后 PowerPoint 无 Repair；该形状单击超链接地址对上；inspect `kind=url` |
| Phl3 | 改地址 | `setHyperlink` 改 URL 后 COM 地址是新值；无 Repair |
| Phl4 | 删链留字 | `deleteHyperlink` 后链没了，框上字还在；无 Repair |
| Phl5 | 拒错 | 对页 id 发 `insertHyperlink` 得 `InvalidId`；缺 `url`/`file`/`slide` 得 `InvalidField`，未写文件 |

## 媒体

幻灯片上的视频 / 音频是带 `a:videoFile` / `a:audioFile` 的 `p:pic`，与普通图分家。inspect 计 `videoCount` / `audioCount`，`scope.media` 列名单。apply 能插本地 MP4 / WAV、删除。`exportMedia` 写出媒体部件，不是海报。不写 timing。源文件必须是 PowerPoint 亲手另存，第一页至少一段视频。脚本：`manual-check/office/cases/pptx-media.ps1`。编号用 Pmd。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pmd1 | inspect | 无 `media` 时根与各页有 `videoCount` / `audioCount`、没有名单；`scope.media: true` 时视频 id 为 `slide:1/video:…`，不并进 `pictureCount` |
| Pmd2 | 导出 | `exportMedia` 与 `/ppt/media/mediaN.*` 字节一致。不强制开 PowerPoint |
| Pmd3 | 插视频 | 对 create 产物 `insertVideo` 后 PowerPoint 无 Repair；COM Type=16；inspect id 为 `slide:1/video:…` |
| Pmd4 | 插音频 | `insertAudio` 一段 WAV 后无 Repair；`audioCount` 加 1 |
| Pmd5 | 删除 | `deleteMedia` 后该对象消失，无 Repair；对页 id 发 `insertVideo` 得 `InvalidId`，WAV 当视频得 `InvalidField`，未写文件 |

## 幻灯片节

PowerPoint 的节是 `p14:sectionLst` 分组元数据，页顺序仍以 `sldIdLst` 为准。inspect 计 `sectionCount`，`scope.sections` 列名单。apply 能新建节、改名、删节（页还在）、把页划进某节。不写节的 UI 颜色。脚本：`manual-check/office/cases/pptx-sections.ps1`。编号用 Psc。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Psc1 | inspect | 无 `sections` 时根上有 `sectionCount`、没有名单；另存两节以上时 `scope.sections: true` 的 id 为 `section:1`，`slides` 对得上页序 |
| Psc2 | 建节划页 | 两页 create 产物连续 `insertSlideSection` 两节，再把第二页划进第二节后 PowerPoint 无 Repair；COM 第二节名为「内容」、从第 2 页起 |
| Psc3 | 改名 | `setSlideSectionName` 后左侧节名是新字；无 Repair |
| Psc4 | 删节留页 | `deleteSlideSection` 后页还在、对应节名没了；无 Repair |
| Psc5 | 拒错 | 对页 id 发 `insertSlideSection` 得 `InvalidId`；`start` 与 `after` 同给得 `InvalidField`，未写文件 |

## 原样保存（opc 验收）

与 xlsx 清单相同：O1 第一期放弃；零脏写回在 OPC 层验收；O2 按 apply 看未改页。
