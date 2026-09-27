# docx 人工检查清单

代码阶段的单元测试覆盖 JSON 解析和 XML 不变量。下面这些必须用 Microsoft Word 打开一次，不能只靠测试绿。验收只认官方 Office，不再要求 WPS。

检查前用 create 或 apply 在本机生成文件，不要用改过扩展名的 zip 目录冒充。

## 本机结果（2026-09-26）

Microsoft Word 16.0 已打开，用户确认通过。create、inspect、apply 三组都过了。O1 已放弃，见下表。本机若装了 WPS，`Word.Application` 会被抢走，测试仍直接启动 `WINWORD.EXE` 再挂 COM。A11 补过 `word/settings.xml` 之后，2026-09-26 用新 `docx-create.docx` 重跑，Word 报 `CompatibilityMode=15`。

C、I、A 三组现在由 `manual-check/office/cases/docx-*.ps1` 自动判定，跑 `pwsh -File manual-check\office\run.ps1 -Case 'docx-*'` 即可复现，逐条结果落在 `manual-check/office/out/office-results.csv`。

| 序号 | 状态 | 结果 |
| --- | --- | --- |
| C1 | ✅ | Word 16.0 能打开 `manual-check/office/out/docx-create.docx`，标题栏无 Repair；`CompatibilityMode=15` |
| C3–C4 | ✅ | 标题 1 / 「标题」，正文 `hello` |
| C5–C7 | ✅ | Word 16.0：`docx-create-pictures.ps1`，4 PASS / 0 FAIL。第二段行内图 `para:00000002/pic:1`；默认页眉浮动 `square`；标题栏无 Repair，正文与页眉都能看见图 |
| I1–I5 | ✅ | `docx:document`、`paraCount=2`、大纲 level 1、「标题」、`para:{hex}` 且 `stable`、无 scope 时没有 `paragraphs`，仍有 `pictureCount` |
| A1–A5 | ✅ | Word 打开 apply 产物：第二段为「改过的正文」且仍加粗，第一段仍是「第一章」/标题 1，第三段未改 |
| HF1–HF5 | ✅ | Word 另存带页眉页脚样例：名单与展开、改页眉后打开无修复，页眉变「新页眉」、正文与页脚未改 |
| D1–D4 | ✅ | Word 16.0：`section:1` 原距离 708/708；apply 后 HeaderDistance=72 磅、FooterDistance=36 磅，正文未改 |
| L1–L4 | ✅ | Word 16.0：第二节 `headerLink.default` 为 true 且 `LinkToPrevious`；断开后无修复且不再链接；改第二节页眉后第一节仍是原文；正文未改 |
| P1–P7 | ✅ | Word 16.0：inspect 列出行内图与浮动图；断链后第二节仍见同一张图；换/插/删无修复；尺寸 144 磅；行内改浮动 `square` |
| B1–B7 | ✅ | Word 16.0：根上 `pictureCount=2`；第二段行内+浮动；换/插/删无修复；尺寸 144 磅；行内改浮动 `square`；改字后图还在 |
| E1–E4 | ✅ | 库测试：页眉图 / 正文图导出字节与 `/word/media/…` 一致，`contentType` 为 `image/png`；换图后再导出得到新文件；段落 id 报 `InvalidId`，缺图报 `IdNotFound`。不强制开 Word |
| T1–T9 | ✅ | Word 16.0：`docx-textbox.ps1`，13 PASS / 0 FAIL。行内+浮动+链接+组合；框内改字、插删框、尺寸 144 磅、绕排、红填蓝边、框内插删段与插图、断开再接上。标题栏无 Repair |
| N1–N6 | ✅ | Word 16.0：`docx-notes.ps1`，7 PASS / 0 FAIL。计数与名单、改注文、插删脚注、编号 `NumberStyle=1`（大写罗马）、注内插图。标题栏无 Repair |
| Cmt1–Cmt7 | ✅ | Word 16.0：`docx-comments.ps1`，8 PASS / 0 FAIL。计数与名单、改字、插删、回复线程（`Comments.Count=4` 且 `Replies=1`）、完成标记、批注内插图。标题栏无 Repair |
| Ch1–Ch8 | ✅ | Word 16.0：`docx-charts.ps1`，9 PASS / 0 FAIL。计数与名单（COM 系列值对得上）、改系列为 11/22/33/44、插饼图、删除、改折线、关图例开标签、组合图当时 `NotImplemented` 且未写文件、`exportChartData` 与 embedding 字节一致。标题栏无 Repair。组合图改数与改类型见下方 Cx 组 |
| Cx1–Cx8 | ✅ | Word 16.0：`docx-charts-ext.ps1`，8 PASS / 0 FAIL。圆环 / 雷达 / 气泡各自有 `type`；改数、插入、组合图改数与收成柱形、透视改标题、股价仍 `NotImplemented`。标题栏无 Repair |
| Sa1–Sa8 | ✅ | Word 16.0：`docx-smartart.ps1`。列表型 SmartArt 改字、插删、增删点、跨家族 `NotImplemented` |
| Ip1–Ip5 | ✅ | Word 16.0：`docx-body-para.ps1`，5 PASS / 0 FAIL。插在第二段之后、插到文档末、删第二段、最后一段 `InvalidField`、默认页眉插一段。标题栏无 Repair |
| Nh1–Nh7 | ✅ | Word 16.0：`docx-new-header.ps1`，7 PASS / 0 FAIL。新建默认页眉、首页不同、奇偶页、第二节 even 独立、create even/first、类型未生效与已链接均 `InvalidField`。标题栏无 Repair |
| Sp1–Sp8 | ✅ | Word 16.0：`docx-section.ps1`，8 PASS / 0 FAIL。四边距与横向纸张、距边界仍独立、分栏、垂直对齐与行号、镜像边距。标题栏无 Repair |
| Tb1–Tb8 | ✅ | Word 16.0：`docx-table.ps1`，8 PASS / 0 FAIL。名单、改字、增行增列、删表、create 2×2、合并、最后一行 `InvalidField`。标题栏无 Repair |
| Rv1–Rv6 | ✅ | Word 16.0：`docx-revision.ps1`，6 PASS / 0 FAIL。名单、接受删除、拒绝插入、接受插入、拒绝删除、带修订段 `setParaText` 得 `NotImplemented`。标题栏无 Repair |
| Hb1–Hb7 | ✅ | Word 16.0：`docx-hyperlink.ps1`，7 PASS / 0 FAIL。名单、改 URL、插链、删链、插书签、删书签、带链段 `setParaText` 得 `NotImplemented`。标题栏无 Repair |
| Fd1–Fd7 | ✅ | Word 16.0：`docx-field.ps1`，7 PASS / 0 FAIL。名单、改 PAGE 为 DATE、插 DATE、删域留结果、删域留指令、页眉 PAGE、带域段 `setParaText` 得 `NotImplemented`。标题栏无 Repair |
| Nl1–Nl6 | ✅ | Word 16.0：`docx-numbering.ps1`，6 PASS / 0 FAIL。名单、段上 `num`/`ilvl`、插 decimal 定义、绑两段、改 bullet、解绑。标题栏无 Repair |
| St1–St6 | ✅ | Word 16.0：`docx-style.ps1`，6 PASS / 0 FAIL。名单、换 Heading2、大纲对得上、改 Normal 为 24 磅、去掉 Heading1 加粗、解绑。标题栏无 Repair |
| Sc1–Sc6 | ✅ | Word 16.0：`docx-insert-section.ps1`，6 PASS / 0 FAIL。插节后两节且第二节 `headerLink.default`；打开无修复；断链后两节页眉能独立改字；删节后只剩一节、无孤儿 `headerN.xml`；create 带 `sectionType` 为两节。标题栏无 Repair |
| Tc1–Tc5 | ✅ | Word 16.0：`docx-toc.ps1`，5 PASS / 0 FAIL。无 `scope.tocs` 时只有 `tocCount`；插入后 `toc:1` / `kind=toc` 且 `fields` 含 TOC；打开无修复；更新域后可见「第一章」；删后 `tocCount=0` 标题还在；`kind=tof` 树上为 `tof`。标题栏无 Repair |
| O1 | 🚫 | 领域层不做原样保存，与 xlsx 清单相同。零脏写回见 OPC 库测试。Word 另存 create 产物见 `docx-O1-word-resave`。此前记的 `SaveAs2` 挂起，拆分后重跑没再出现，一秒内返回 |

## create

用 `docs/examples/docx-create.json` 生成 `note.docx`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| C1 | Word 打开 | 无修复提示、无「文件损坏」；`CompatibilityMode` 为 `15` |
| C3 | 第一段 | 样式为标题 1，文字为 `标题` |
| C4 | 第二段 | 正文，文字为 `hello` |
| C5 | 带图正文 | 用 `docs/examples/docx-create-pictures.json`（换成本机 PNG）生成；第二段有行内图，inspect id 为 `para:…/pic:1` |
| C6 | 带图页眉 | 同上，默认页眉有浮动 `square`，Word 能看见 |
| C7 | Word 打开带图产物 | 无修复；图可见 |

## inspect

对上面的文件发 `docs/examples/docx-inspect.json`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| I1 | 根节点 | `id` 为 `docx:document`，`paraCount` 为 `2` |
| I2 | 大纲 | `outline` 含第一段，`level` 为 `1`，`text` 为 `标题` |
| I3 | 段落 id | 两段都有 `para:{hex}`，`stable` 为 `true` |
| I4 | 第二段 | `text` 为 `hello` |
| I5 | 无 scope | 只发 path 时没有 `paragraphs` 字段，仍有 `outline`、`paraCount`、`pictureCount`、`tableCount`、`revisionCount`、`hyperlinkCount`、`bookmarkCount`、`fieldCount`、`tocCount`、`headers`、`footers`、`sections` |

## apply

准备一份 Word 亲手另存的样例（至少两段正文、第一段是标题、第二段带加粗或别的 run 样式）。对它执行 `setParaText`，改第二段文字，id 用 inspect 返回的值。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| A1 | 打开 | Microsoft Word 能打开，无修复提示 |
| A2 | 目标段 | 被点名的那段可见文字变成请求里的 `text` |
| A3 | 未改段 | 第一段标题文字和标题样式还在 |
| A4 | 样式 | 目标段若原先有加粗等 run 样式，改完后看起来还在（允许整段变成单一 run，但加粗还在） |
| A5 | 未改部件 | 未点名的页眉页脚与样式表看起来不变 |

## 页眉页脚

准备一份 Word 亲手另存的样例：至少有默认页眉与默认页脚，页眉里有一段无域、无超链接的可见文字。不要用库 `create` 产物（这两项要测 Word 另存的已有页眉，不要用 create 刚写的默认页眉）。对它 `inspect` 时带 `scope.headerFooter: true`，再用返回的页眉段落 id 做 `setParaText`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| HF1 | 名单 | 无 scope 时也有 `headers`、`footers`；容器带 `id`、`type`、`sections`、`paraCount`、`pictureCount`，不带 `paragraphs`、`pictures` |
| HF2 | 展开 | `headerFooter: true` 时页眉节点有 `paragraphs`，段落 id 形如 `header:default:1/para:…` |
| HF3 | 打开 | apply 改页眉后 Word 能打开，无修复提示 |
| HF4 | 目标 | 页眉可见文字变成请求里的 `text`；正文未改 |
| HF5 | 未改部件 | 未点名的页脚与 `styles.xml` 看起来不变 |

## 距边界

对 Word 亲手另存或 create 后再经 Word 打开过的文件 `inspect`，根上应有 `sections`。再用 `setHeaderFooterDistance` 改第一节。Word COM 的 `PageSetup.HeaderDistance` / `FooterDistance` 单位是磅，等于 twip ÷ 20。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| D1 | 名单 | 无 scope 时有 `sections`；第一节 `id` 为 `section:1`，带 `headerDistance`、`footerDistance` |
| D2 | 打开 | apply 后 Word 能打开，无修复提示 |
| D3 | 距离 | 页眉/页脚距边界变成请求值（允许 0.05 磅误差） |
| D4 | 正文 | 正文段落文字未改 |

## 链接前节

准备一份 Word 亲手另存的两节样例：第二节默认页眉勾选「链接到前一节」。不要用库 `create` 产物（本组仍要求 Word 另存的两节样例）。对它 `inspect` 后发 `setHeaderFooterLink` 断开第二节默认页眉，再改第二节页眉文字。Word COM 的 `Headers.Item(1).LinkToPrevious` 对应默认页眉。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| L1 | 名单 | 第二节 `headerLink.default` 为 `true`；COM 读第二节 `LinkToPrevious` 为 `true` |
| L2 | 打开 | 断开后 Word 能打开，无修复提示；COM 读第二节 `LinkToPrevious` 为 `false` |
| L3 | 目标 | 改第二节页眉后，第一节页眉仍是原文 |
| L4 | 正文 | 正文段落文字未改 |

## 页眉页脚图文混排

准备一份 Word 亲手另存的样例：默认页眉里同时有一张行内图和一张浮动图（`wrapSquare`）。不要用库 `create` 产物。另备一张本地 PNG 供换图、插图。手测脚本 `manual-check/office/cases/docx-picture.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| P1 | 名单 | 无 scope 时容器有 `pictureCount`、没有 `pictures`；`pictures: true` 时两张图都有 id，`kind` 分别为 `inline` 与 `anchor`，浮动 `wrap` 为 `square` |
| P2 | 断链 | 两节样例、第二节链着带图页眉；断开后 Word 无修复，第二节仍能看见同一张图 |
| P3 | 换图 | `replacePicture` 后 Word 无修复；位置与绕排不变；可见图已换成新文件 |
| P4 | 插图 | `insertPicture` 往页眉段落插行内图，Word 能看见且无修复 |
| P5 | 删图 | `deletePicture` 后该图消失，Word 无修复 |
| P6 | 尺寸 | `setPictureSize` 后 COM 或目视尺寸对上（允许 EMU 换算误差） |
| P7 | 绕排 | `setPictureWrap` 把行内改成浮动 `square` 后，Word 按四周型显示 |

## 正文图

准备一份 Word 亲手另存的样例：正文某段同时有一张行内图和一张浮动图（`wrapSquare`）。不要用库 `create` 产物。另备一张本地 PNG 供换图、插图。手测脚本 `manual-check/office/cases/docx-body-picture.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| B1 | 名单 | 无 scope 时根上有 `pictureCount`、没有正文 `pictures`；`from`+`maxParas`+`pictures` 时目标段两张图都有 id，形如 `para:…/pic:…`，`kind` 分别为 `inline` 与 `anchor`，浮动 `wrap` 为 `square` |
| B2 | 换图 | `replacePicture` 后 Word 无修复；位置与绕排不变；可见图已换成新文件 |
| B3 | 插图 | `insertPicture` 往正文段落插行内图，Word 能看见且无修复 |
| B4 | 删图 | `deletePicture` 后该图消失，Word 无修复 |
| B5 | 尺寸 | `setPictureSize` 后 COM 或目视尺寸对上（允许 EMU 换算误差） |
| B6 | 绕排 | `setPictureWrap` 把行内改成浮动 `square` 后，Word 按四周型显示 |
| B7 | 改字 | 对该段 `setParaText` 后文字已改、图还在，Word 无修复 |

## 导出媒体

对带图样例发 `exportMedia`。不必开 Word，比对导出文件与 ZIP 内 `/word/media/…` 的字节即可。手测脚本不强制。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| E1 | 页眉图 | 对页眉图 id 导出，字节与 `/word/media/…` 一致，`contentType` 与内容类型相符 |
| E2 | 换图后 | `replacePicture` 后再导出，得到的是新文件 |
| E3 | 正文图 | 对正文图 id 同样能导出，字节一致 |
| E4 | 错 id | 对段落 id 报 `InvalidId`；对不存在的图 id 报 `IdNotFound` |

## 文本框

准备一份 Word 亲手另存的样例：正文有行内框、浮动框，其中一对链接框；另有组合里的框。不要用库 `create` 产物当唯一样例。手测脚本 `manual-check/office/cases/docx-textbox.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| T1 | 名单 | 无 scope 时根上有 `textBoxCount`、没有正文 `textBoxes`；`from`+`maxParas`+`textBoxes` 时目标段有框 id，形如 `para:…/txbx:…` |
| T2 | 框内 | `textBoxContent` 时框内段落 id 带框前缀，`text` 对，不并进宿主 `text` |
| T3 | 链接 | 链接框的 `link.prev` / `link.next` 互指 |
| T4 | 改字 | 对框内段落 `setParaText` 后 Word 无修复，框还在 |
| T5 | 插删框 | `insertTextBox` / `deleteTextBox` 后 Word 无修复 |
| T6 | 尺寸绕排 | `setTextBoxSize` / `setTextBoxWrap` 后 COM 或目视对上 |
| T7 | 填色描边 | `setTextBoxFill` / `setTextBoxLine` 后可见变化，无修复 |
| T8 | 框内段与图 | 框内插删段、插图与正文图规则相同 |
| T9 | 链接 | `setTextBoxLink` 断开或接上后 Word 能继续编辑链条 |

## 脚注与尾注

准备一份 Word 亲手另存的样例：至少一条用户脚注、一条用户尾注。不要用库 `create` 产物当唯一样例（create 不写注）。手测脚本为 `manual-check/office/cases/docx-notes.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| N1 | 计数与名单 | 无 `notes` 时根上有 `footnoteCount` / `endnoteCount`、没有名单；`notes: true` 时用户注 id 为 `footnote:{id}` / `endnote:{id}`，`host` 对，分隔符只在 `noteSeparators` |
| N2 | 改注文 | 对注内段落 `setParaText` 后 Word 无修复，正文未改 |
| N3 | 插入 | `insertFootnote` 后 Word 能看见引用与注文，无修复 |
| N4 | 删除 | `deleteFootnote` 后引用与注文都消失，无修复 |
| N5 | 编号 | `setNoteNumbering` 改 `numFmt` 后 Word 打开时编号样式变了（允许打开时才重画） |
| N6 | 注内图 | 注内 `replacePicture` 或 `insertPicture` 后 Word 无修复 |

## 批注

准备一份 Word 亲手另存的样例：至少两条根批注、一条回复。不要用库 `create` 产物当唯一样例（create 不写批注）。手测脚本为 `manual-check/office/cases/docx-comments.ps1`。编号用 Cmt，避免和 create 的 C 组重号。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Cmt1 | 计数与名单 | 无 `comments` 时根上有 `commentCount`、没有名单；`comments: true` 时 id 为 `comment:{id}`，`start` / `end` / `parent` / `done` 对 |
| Cmt2 | 改字 | `setCommentText` 后 Word 无修复，正文未改 |
| Cmt3 | 插入 | `insertComment` 后审阅窗格能看见新批注，无修复 |
| Cmt4 | 删除 | `deleteComment` 后范围标记与回复都消失，无修复 |
| Cmt5 | 回复 | `replyComment` 后 `parent` 指向父批注，Word 能看见线程 |
| Cmt6 | 完成 | `setCommentDone` 后 COM `Comment.Done` 为 true，或 UI 显示已解决 |
| Cmt7 | 批注内图 | 批注内 `insertPicture` 后 Word 无修复 |

## 嵌入图表

准备一份 Word 亲手另存的样例：至少一张柱形图（带标题与两个系列）。不要用库 `create` 产物当唯一样例（create 不写图表）。手测脚本为 `manual-check/office/cases/docx-charts.ps1`。编号用 Ch，避免和 create 的 C 组、批注的 Cmt 组重号。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Ch1 | 计数与名单 | ✅ 无 `charts` 时根上有 `chartCount`、没有名单；`charts: true` 时 id 为 `{段落}/chart:{docPrId}`，`type` / `title` / 系列值与数据表一致 |
| Ch2 | 改系列 | ✅ `setChartSeries` 后 Word 无修复，数据表是新值 |
| Ch3 | 插入 | ✅ `insertChart` 一张饼图后 Word 能看见，无修复 |
| Ch4 | 删除 | ✅ `deleteChart` 后图与数据都消失，无修复 |
| Ch5 | 改类型 | ✅ `setChartType` 为 `line` 后 Word 按折线显示（允许样式重置） |
| Ch6 | 图例与标签 | ✅ `setChartLegend` / `setChartDataLabels` 后 Word 无修复，开关能对上 |
| Ch7 | 组合图 | ✅ 当时对组合图发 `setChartType` 得到 `NotImplemented`，文件未写坏。现行契约已放开，改看 Cx5–Cx6 |
| Ch8 | 导出数据 | ✅ `exportChartData` 写出的 xlsx 与 embedding 部件字节一致。不强制开 Word |

## 扩展图表

准备一份 Word 亲手另存的样例：至少各一张圆环、雷达、气泡、柱线组合，以及一张带 `flags.pivot` 的图（可用 Excel 透视图表粘进 Word 再另存）。不要用库 `create` 产物当唯一样例。手测脚本为 `manual-check/office/cases/docx-charts-ext.ps1`。编号用 Cx。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Cx1 | 类型 | ✅ 圆环 / 雷达 / 气泡的 `type` 分别为 `doughnut` / `radar` / `bubble`，不再是 `other`；气泡系列有 `sizes`，值与数据表一致 |
| Cx2 | 改圆环或雷达 | ✅ `setChartSeries` 后 Word 无修复，数据表是新值 |
| Cx3 | 改气泡 | ✅ `setChartSeries` 改 `values` 与 `sizes` 后 Word 无修复 |
| Cx4 | 插入 | ✅ `insertChart` 一张圆环、一张雷达后 Word 能看见，无修复 |
| Cx5 | 组合图改数 | ✅ 组合图 `setChartSeries` 后两条系列的值都对，无修复 |
| Cx6 | 组合图改类型 | ✅ 组合图 `setChartType` 为 `col` 后变成柱形，无修复 |
| Cx7 | 透视图表 | ✅ `setChartTitle` 能改；`setChartSeries` 得到 `NotImplemented`，文件未写坏 |
| Cx8 | 仍受限 | ✅ 对股价或曲面图（`type=other`）发 `setChartType` 得到 `NotImplemented`，文件未写坏 |

## SmartArt

准备一份 Word 亲手另存的样例：至少一张列表型 SmartArt（可见点不少于 3 个）。不要用库 `create` 产物当唯一样例。手测脚本为 `manual-check/office/cases/docx-smartart.ps1`。编号用 Sa。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Sa1 | 计数与名单 | 无 `smartArt` 时根上有 `smartArtCount`、没有名单；`smartArt: true` 时 id 为 `{段落}/smartdiagram:{docPrId}`，点文本与 Word 里能看见的字一致 |
| Sa2 | 改字 | `setSmartArtText` 后 Word 无修复，目标点的字变了 |
| Sa3 | 插入 | `insertSmartArt` 一张 `list` 后 Word 能看见，无修复 |
| Sa4 | 删除 | `deleteSmartArt` 后图示与 Fallback 位图都消失，无修复 |
| Sa5 | 增点 | `addSmartArtPoint` 后 Word 打开重排，可见条数多 1 |
| Sa6 | 删点 | `deleteSmartArtPoint` 后 Word 打开重排，目标点及其子点消失 |
| Sa7 | 版式 | 对不在 `list` / `org` 家族的图示发 `setSmartArtLayout` 得到 `NotImplemented`，文件未写坏 |
| Sa8 | 尺寸 | `setSmartArtSize` 后宿主尺寸能对上，无修复 |

## 正文插删段落

准备一份 Word 亲手另存的样例：至少三段正文，第二段带加粗。页眉另用带默认页眉的 Word 另存样例。不要用库 `create` 产物当唯一样例。手测脚本 `manual-check/office/cases/docx-body-para.ps1`。编号用 Ip。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Ip1 | 插在第二段之后 | `insertPara` 后 Word 无修复；新段可见文字是请求里的 `text`；第二段仍加粗；第一段、原第三段未改 |
| Ip2 | 插到文档末 | 对 `docx:document` 发 `insertPara` 后，inspect 最后一段是新段；Word 无修复 |
| Ip3 | 删除非末段 | `deletePara` 删掉第二段后 Word 无修复；第一段标题还在 |
| Ip4 | 最后一段 | 故事里只剩一段时再 `deletePara` 得 `InvalidField`，文件未写坏 |
| Ip5 | 页眉 | 对默认页眉容器或页眉段落 `insertPara` 后，页眉可见多一段；Word 无修复；正文未改 |

## 新建页眉与奇偶首页

无页眉的源可用库 `create`（不要页眉）或 Word 空白文档另存。两节样例用 `Ensure-DocxWordLinkSource`。手测脚本 `manual-check/office/cases/docx-new-header.ps1`。编号用 Nh。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Nh1 | 新建默认页眉 | 对无页眉文档 `insertHeaderFooter` 后 Word 无修复；随后改字，页眉可见新字，正文未改 |
| Nh2 | 首页不同 | `setTitlePage` 为 true 再插入 first 页眉并改字；Word `DifferentFirstPageHeaderFooter` 为 true；首页页眉是新字 |
| Nh3 | 奇偶页 | `setEvenAndOddHeaders` 为 true 再插入 even 页眉并改字；Word `OddAndEvenPagesHeaderFooter` 为 true |
| Nh4 | 第二节 even 独立 | 两节文档打开奇偶页后，对第二节 even `setHeaderFooterLink` 断链并改字；第一节 even 仍是原文；Word 无修复 |
| Nh5 | create even/first | create 同时给 default / even / first；Word 无修复；奇偶与首页标志为真 |
| Nh6 | 类型未生效 | 未开奇偶页时对 even 发 `insertHeaderFooter` 得 `InvalidField`，文件未写坏 |
| Nh7 | 已链接 | 对第二节 default（仍链接前节）发 `insertHeaderFooter` 得 `InvalidField`，文件未写坏 |

## 节属性

对 Word 亲手另存或 create 后再经 Word 打开过的文件 `inspect`，节上应有边距与纸张字段。再用 `setPageMargin` / `setPageSize` 改第一节，并确认距边界仍可独立改。Word COM 的 `PageSetup` 边距与纸张单位是磅，等于 twip ÷ 20。手测脚本 `manual-check/office/cases/docx-section.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Sp1 | 名单 | 无 scope 时第一节有 `top` / `pageWidth`，根上有 `mirrorMargins`；缺属性则为 `null` |
| Sp2 | 打开 | 改四边距与横向纸张后 Word 能打开，无修复提示 |
| Sp3 | 边距 | COM `TopMargin` 等变成请求值（允许 0.05 磅误差）；未点名的 `gutter` 仍是原 twip |
| Sp4 | 纸张 | COM `Orientation` 为横向；页宽/页高对得上（允许 0.05 磅误差） |
| Sp5 | 距边界 | 随后只改 `setHeaderFooterDistance`，COM 页眉距变了，四边距仍是 Sp3 的值 |
| Sp6 | 分栏 | `setColumns` 等宽两栏后 COM `TextColumns.Count` 为 2；Word 无修复 |
| Sp7 | 垂直对齐与行号 | `setVerticalAlign` 为 `center`、`setLineNumbering` 的 `countBy` 为 1 后，COM 对得上；Word 无修复 |
| Sp8 | 镜像边距 | `setMirrorMargins` 为 true 后 COM `MirrorMargins` 为 true；Word 无修复 |

## 表格

准备一份 Word 亲手另存的样例：正文有一张 2×2 简单表，格子里是无域、无超链接的可见文字。`inspect` 带 `scope.tables: true`。手测脚本 `manual-check/office/cases/docx-table.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Tb1 | 名单 | 无 scope 时仍有 `tableCount`；给了 `scope.tables` 后有 `table:1`，两行两列，单元格带段落 id |
| Tb2 | 改字 | 对格子里的段落 `setParaText` 后 Word 无修复，该格可见新字，其余格与正文未改 |
| Tb3 | 增行 | `insertRow` 后 COM `Tables(1).Rows.Count` 加 1；Word 无修复 |
| Tb4 | 增列 | `insertCol` 的 `after` 为 1 后 COM `Columns.Count` 加 1；Word 无修复 |
| Tb5 | 删表 | `deleteTable` 后 `tableCount` 为 0；Word 无修复 |
| Tb6 | create | create 带 2×2 表，Word 能打开，无修复；COM 表数为 1 |
| Tb7 | 合并 | `mergeCells` 合并第一行两格后 Word 无修复；COM 第一行单元格数变为 1 |
| Tb8 | 最后一行 | 对仅剩一行的表 `deleteRow` 得 `InvalidField`，文件未写坏 |

## 修订痕迹

准备一份带修订的样例：Word 打开跟踪修订后，一段里删几个字、另一段里插入几个字，另存。`inspect` 带 `scope.revisions: true`。手测脚本 `manual-check/office/cases/docx-revision.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Rv1 | 名单 | 无 scope 时仍有 `revisionCount` 且没有 `revisions`；给了 `scope.revisions` 后能列出 `ins` 与 `del`，带 `host` 段落 id |
| Rv2 | 接受删除 | `acceptRevision` 点名删除后，Word 无修复，被删的字从可见文本里消失 |
| Rv3 | 拒绝插入 | `rejectRevision` 点名插入后，Word 无修复，插入字消失，原文还在 |
| Rv4 | 接受插入 | `acceptRevision` 点名插入后，Word 无修复，插入字留下且不再标修订 |
| Rv5 | 拒绝删除 | `rejectRevision` 点名删除后，Word 无修复，被删的字重新可见 |
| Rv6 | 改字拒绝 | 对仍带本期修订的段发 `setParaText` 得 `NotImplemented`，文件未写坏 |

## 超链接与书签

准备一份带超链接与书签的样例：Word 给第一段加外部 URL，给第二段加用户书签，另存。`inspect` 带 `scope.hyperlinks: true` 与 `scope.bookmarks: true`。手测脚本 `manual-check/office/cases/docx-hyperlink.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Hb1 | 名单 | 无 scope 时仍有 `hyperlinkCount` / `bookmarkCount` 且没有名单；给了对应 scope 后能列出 `link:` 与 `bookmark:`，链带 `url`，书签带 `name` |
| Hb2 | 改 URL | `setHyperlink` 改外部地址后 Word 无修复，COM `Hyperlinks(1).Address` 是新地址 |
| Hb3 | 插链 | 对无链段 `insertHyperlink` 后 Word 无修复，能看见新链 |
| Hb4 | 删链 | `deleteHyperlink` 后字还在、链消失，无修复 |
| Hb5 | 插书签 | `insertBookmark` 后 COM 能按名找到，无修复 |
| Hb6 | 删书签 | `deleteBookmark` 后该书签消失，无修复 |
| Hb7 | 改字拒绝 | 对仍带本期超链接的段发 `setParaText` 得 `NotImplemented`，文件未写坏 |

## 域

准备一份带 PAGE 或 DATE 域的样例：可用 `insertField` 写入，也可用 Word 另存。`inspect` 带 `scope.fields: true`。手测脚本 `manual-check/office/cases/docx-field.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Fd1 | 名单 | 无 scope 时仍有 `fieldCount` 且没有 `fields`；给了 `scope.fields` 后能列出 `field:`，带 `instr` 与 `code` |
| Fd2 | 改指令 | `setField` 改 PAGE 为 DATE 后 Word 无修复 |
| Fd3 | 插域 | 对无域段 `insertField` 后 Word 无修复，能看见域 |
| Fd4 | 删域留结果 | `deleteField`（`keep` 为 `result` 或缺省）后缓存字还在、域消失，无修复 |
| Fd5 | 删域留指令 | `deleteField` 且 `keep` 为 `instr` 后指令字还在、域消失，无修复 |
| Fd6 | 页眉 PAGE | 默认页眉里的 PAGE 能列出，id 带 `header:` 前缀 |
| Fd7 | 改字拒绝 | 对仍带本期域的段发 `setParaText` 得 `NotImplemented`，文件未写坏 |

## 编号列表

准备一份无编号的 create 产物，再用 `insertNumbering` / `setParaNumbering` 写入。`inspect` 带 `scope.numbering: true`。手测脚本 `manual-check/office/cases/docx-numbering.ps1`。脚注编号与节行号不在本组。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Nl1 | 名单 | 无 scope 时仍有 `numberingCount` 且没有 `numberings`；给了 `scope.numbering` 后能列出 `num:`，带 `levels` |
| Nl2 | 段上编号 | `setParaNumbering` 绑定后该段有 `num` 与 `ilvl` |
| Nl3 | 插定义 | `insertNumbering`（`decimal`）后 Word 无修复，`numberingCount` 增加 |
| Nl4 | 套用 | 两段绑同一实例后 Word 能看见编号，无修复 |
| Nl5 | 改格式 | `setNumbering` 把该级 `numFmt` 改成 `bullet` 后 Word 无修复 |
| Nl6 | 解绑 | `setParaNumbering` 且 `num` 为 `null` 后该段无编号，无修复 |

## 样式

`inspect` 带 `scope.styles: true`。手测脚本 `manual-check/office/cases/docx-style.ps1`。段落或 run 上的直接格式不在本组。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| St1 | 名单 | 无 scope 时仍有 `styleCount` 且没有 `styles`；给了 `scope.styles` 后能列出 `style:Normal` 与 Heading |
| St2 | 换样式 | `setParaStyle` 把正文段换成 `style:Heading2` 后该段 `style` 为 `Heading2`，`outline` 含 `level` 2 |
| St3 | 大纲 | Word 打开后该段样式为标题 2，无修复 |
| St4 | 改 Normal | `setStyle` 把 Normal 的 `sz` 改成 48 后，未点名段 Word 字号约为 24 磅，无修复 |
| St5 | 改标题 | `setStyle` 去掉 Heading1 加粗后，标题段 Word 不再加粗，无修复 |
| St6 | 去掉 | `setParaStyle` 且 `style` 为 `null` 后该段树上 `style` 为 `null`，无修复 |

## 段落与 run 直接格式

`inspect` 段上始终有 `align` / `indent` / `spacing` / `runCount`。`scope.runs: true` 时列出 `runs`。手测脚本 `manual-check/office/cases/docx-format.ps1`。不改 `styles.xml`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pf1 | 名单 | 无 `scope.runs` 时已切出的段有 `runCount` 且没有 `runs`；给了 `scope.runs` 后能列出 `{段落 id}/run:{n}` |
| Pf2 | 段对齐 | `setParaFormat` 把一段 `align` 改成 `center` 后树上为 `center` |
| Pf3 | 打开对齐 | Word 打开后该段居中，无修复 |
| Pf4 | run 加粗 | `setRunFormat` 把第一段第一个 run 设为加粗后 Word 该 run 加粗，无修复 |
| Pf5 | 未点名 | 同一段未点名的 run 颜色还在 |
| Pf6 | 间距 | `setParaFormat` 写 `before` / `after` 后 Word 段前段后距对得上（允许 0.05 磅误差），无修复 |

## 插删节

`insertSection` 在某段之后写入分节符；`deleteSection` 删掉非末节。create 可用 `paragraphs[].sectionType` 写多节。手测脚本 `manual-check/office/cases/docx-insert-section.ps1`。新建页眉的具体 XML 不在本组，断开链接仍走 `setHeaderFooterLink`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Sc1 | 插节 | `insertSection` 之后树上有两节；第二节 `headerLink.default` 为 `true` |
| Sc2 | 打开 | Word 打开后无修复；能看见分节 |
| Sc3 | 断开 | 对第二节发 `setHeaderFooterLink` 且 `linked: false` 后，两节页眉能独立改字，无修复 |
| Sc4 | 删节 | `deleteSection` 且 `id` 为 `section:1` 后只剩一节，Word 无修复 |
| Sc5 | 孤儿 | 删节后包里没有未被引用的 `/word/headerN.xml` |
| Sc6 | create | 带 `sectionType` 的 create 产物树上有两节，Word 无修复 |

## 目录

`insertToc` 写入 TOC / TOA 域；`deleteToc` 删掉目录块。不在库里算页码。手测脚本 `manual-check/office/cases/docx-toc.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Tc1 | 名单 | 无 `scope.tocs` 时有 `tocCount` 且没有 `tocs`；`insertToc` 之后 `tocCount` 为 1，给了 `scope.tocs` 能列出 `toc:1`，`kind` 为 `toc`，且 `fields` 里有对应 `TOC` 域 |
| Tc2 | 打开 | Word 打开后无修复 |
| Tc3 | 更新 | Word 更新域或 `TablesOfContents.Update` 之后，目录可见文字含标题「第一章」 |
| Tc4 | 删除 | `deleteToc` 之后 `tocCount` 为 0，标题段还在，Word 无修复 |
| Tc5 | 图表目录 | `kind` 为 `tof` 的插入产物树上 `kind` 为 `tof`，Word 无修复 |

## 原样保存（opc 验收）

与 xlsx 清单相同：O1 第一期放弃；零脏写回在 OPC 层验收；O2 按 apply 看未改区域。
