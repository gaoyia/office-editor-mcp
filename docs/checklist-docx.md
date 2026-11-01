# docx 人工检查清单

代码阶段的单元测试覆盖 JSON 解析和 XML 不变量。下面这些必须用 Microsoft Word 打开一次，不能只靠测试绿。验收只认官方 Office，不再要求 WPS。

检查前用 create 或 apply 在本机生成文件，不要用改过扩展名的 zip 目录冒充。

## 本机结果（2026-09-26）

Microsoft Word 16.0 已打开，用户确认通过。create、inspect、apply 三组都过了。O1 已放弃，见下表。
本机若装了 WPS，`Word.Application` 会被抢走，测试仍直接启动 `WINWORD.EXE` 再挂 COM。
2026-10-30 补跑 `id-pin.ps1`。同一批里先插段再改 `para:index:4`，Word 16.0 读到的是原来的第四段。
2026-10-31 补跑 `strict-docx.ps1`。Word 16.0 另存的 Strict 文档可以 inspect，页边距按 twip 读出，apply 被拒绝，源文件不变。没有 Repair。
2026-10-31 补跑 `docx-create-password.ps1`。create 写入的限制编辑口令，Word 16.0 认，错口令解不开，对口令能解开。没有 Repair。
2026-10-31 补跑 `docx-table-pin.ps1`。先插行再给原第 3 行上色，字仍在原来那一行。先删第一列再合并原来的第 2、3 列，两格合成一格。Word 16.0 没有 Repair。
2026-10-31 补跑 `docx-inline-pin.ps1`。先删第一个书签再改原来第三个，先删第一个公式再改原来第二个。Word 16.0 没有 Repair。
2026-10-31 补跑 `docx-doc-pin.ps1`。先插节再改原来第 2 节的页边距，改在最后一节上。Word 16.0 没有 Repair。
2026-11-01 补跑 `docx-noteref.ps1`。`insertCrossRef` 用 `target` 点名脚注和尾注，库自己套 `_Ref` 书签并写成 `NOTEREF`。Word 16.0 更新域后显示脚注编号 1、尾注编号 i，没有 Repair。
2026-11-01 补跑 `docx-model3d-float.ps1`。`insertModel3d` 写成四周型浮动后，Word 16.0 打开没有 Repair，形状绕排是四周型。
2026-11-01 补跑 `docx-sdt-bind.ps1`。Word 另存的绑定控件改字后，控件和自定义 XML 都是新值。重复节再克隆一项后有两项。没有 Repair。
2026-11-01 补跑 `docx-container-ole.ps1`。文本框段落里插入的嵌入对象，Word 16.0 打开没有 Repair，框内能看见该 OLE。
2026-11-01 补跑 `docx-container-toc.ps1`。文本框里的目录更新域后能看见标题「第一章」，没有 Repair。
2026-11-01 重跑整套手测。页眉书目改为插入成功。create 只给 `password` 为 `InvalidField`。`setThemeOverride` 在 xlsx 上对 `slide:1` 为 `InvalidId`。
2026-11-02 页眉题注和索引改回拒绝：Word 更新后分别是「错误!仅主文档。」和「不允许有索引」。页眉书目更新后含 `Ecma` 与 `2006`。正文文本框题注更新后是 `1`。
2026-11-01 探测墨迹批注。Word 16.0 保存时丢掉空的 `wpi:ink`，`Replies.Add` 和改 `Range.Text` 的另存同样丢掉。回复和改字仍拒绝，不进 `run.ps1`。
2026-11-02 补跑 `docx-model3d-notes.ps1`。`insertModel3d` 写入脚注和批注后，Word 16.0 打开没有 Repair，两处 Range 的 InlineShape 都是 1。D3n1 通过。
2026-11-01 补探测容器里的六类域块。页眉、文本框、脚注、批注里的交叉引用、索引项、引文、书目更新后有结果，没有 Repair。索引更新报不允许。题注只在文本框里出编号，页眉、脚注、批注是「错误!仅主文档。」
2026-10-30 补跑 `docx-tab-br.ps1`。段落 `a`、制表、`b`、换行、`c`，Word 16.0 能看见。
A11 补过 `word/settings.xml` 之后，2026-09-26 用新 `docx-create.docx` 重跑，Word 报 `CompatibilityMode=15`。

C、I、A 三组现在由 `manual-check/office/cases/docx-*.ps1` 自动判定，跑 `pwsh -File manual-check\office\run.ps1 -Case 'docx-*'` 即可复现，
逐条结果落在 `manual-check/office/out/office-results.csv`。
2026-09-27 补跑 `docx-charts-remain.ps1`，Cr1–Cr6 通过。同日重跑 `docx-charts-ext.ps1`，Cx8 股价三条转柱形。
2026-09-29 补跑 `docx-charts-rare.ps1`，Cq1–Cq5 通过。同日重跑 `docx-smartart.ps1`，Sa1–Sa9 通过。
同日重跑 `docx-comments.ps1`，Cmt1–Cmt7、Ik1–Ik4 通过。2026-09-28 补跑 `docx-format.ps1`，Pf1–Pf6 通过。同日补跑 `docx-ole.ps1`，Ol1–Ol5 通过。
同日补跑 `docx-protect.ps1`，Pr1–Pr6 通过。同日补跑 `docx-watermark.ps1`，Wm1–Wm5 通过。2026-10-09 补跑 `docx-theme.ps1`，Th1–Th5 通过。
同日补跑 `docx-caption-xref.ps1`，Cp1–Cp6 通过。同日补跑 `docx-index.ps1`，Ix1–Ix6 通过。同日补跑 `docx-biblio.ps1`，Bi1–Bi6 通过。
同日补跑 `docx-glossary.ps1`，Bb1–Bb6 通过。同日补跑 `docx-font-embed.ps1`，Fe1–Fe6 通过。同日补跑 `docx-page-bg.ps1`，Pb1–Pb6 通过。
同日补跑 `docx-protect-hash.ps1`，Ph1–Ph5 通过。同日补跑 `docx-revision-remain.ps1`，Rm1–Rm9 通过。同日补跑 `docx-groups.ps1`，Dg1–Dg8 通过。
2026-10-09 探测框内再套框，Word 16.0 只写出故事级兄弟框，Nt1 放弃。同日补跑 `docx-create-remain.ps1`，Cs1–Cs9 通过。
2026-10-09 补跑 `docx-field-types.ps1`，Ft1–Ft7 通过。
2026-10-09 补跑 `docx-svg.ps1`，Sv1–Sv6 通过。Word COM `Type=17`，72×36 磅。
2026-10-09 补跑 `docx-ink.ps1`，Ic1–Ic6 通过。Word COM 在 `Shapes` 上 `Type=20`（`msoCanvas`），144×72 磅。
同日补跑 `docx-emf.ps1`，Dem1–Dem6 通过。Word 行内 COM `Type=3`，无 Repair。
同日补跑 `docx-model3d.ps1`，D3d1–D3d6 通过。Word 行内 COM `Type=19`，无 Repair。
2026-10-10 补跑 `docx-charts-rare-remain.ps1`，Ccx1–Ccx5 通过。无 Repair。
2026-10-10 补跑 `docx-east-asia.ps1`，Ea1–Ea8 通过。无 Repair。COM `FarEastLineBreakControl` / `HangingPunctuation` / `AddSpaceBetweenFarEastAndAlpha` 对得上，`EmphasisMark=1`。
2026-10-10 补跑 `docx-drawing-hlink.ps1`，Dh1–Dh6 通过。无 Repair。COM 图上 `Hyperlink.Address` 对得上。`hyperlinkCount` 仍为 0。内部书签关系 `Target="#SrcBm"` 且无 `TargetMode`。
2026-10-10 补跑 `docx-drawing-shape.ps1`，Ds1–Ds8 通过。无 Repair。COM 矩形 `AutoShapeType=1`，椭圆 `9`，填色红 `RGB=255`，描边蓝 `RGB=16711680`。
2026-10-10 补跑 `docx-style-table.ps1`，Sy1–Sy6、Tf1–Tf6 通过。无 Repair。COM 字符样式 `Color` RGB=255、`Bold=-1`。表 `WrapAroundText=True`，格底纹 `RGB=255`，斜线 `Borders.Item(-7)` 的 `LineStyle=1`。
2026-10-10 补跑 `docx-theme-bg-remain.ps1`，Fm1–Fm7 通过。无 Repair。
2026-10-10 补跑 `docx-protect-rev-math.ps1`，Pq1–Pq8 通过。无 Repair。COM `OMath.Justification=2`，`BibliographyStyle=MLA`。窗体 `ProtectionType=2`。修改密码 `WriteReserved=True`。
2026-10-10 补跑 `docx-create-round4.ps1`，Cy1–Cy8 通过。无 Repair。COM `Shapes.Count=2`，第一格底纹 RGB=255，图上 `Hyperlink.Address` 为 example.com，`BibliographyStyle=MLA`。
2026-10-11 补跑 `docx-box-table.ps1`，Bx1–Bx5 通过。无 Repair。COM 框内 `TextFrame.TextRange.Tables.Count=1`。标题栏仍是 `docx-box-table-nest.docx`。
2026-10-11 补跑 `docx-box-fields.ps1`，Bf1–Bf5 通过。无 Repair。COM 框内 `TextFrame.TextRange.Hyperlinks.Count=1`。标题栏仍是 `docx-box-fields-link.docx`。
2026-10-11 补跑 `docx-para-run-remain.ps1`，Bd1–Bd3、Rh1–Rh3 通过。无 Repair。COM 段底纹 RGB=65535、上边框 `LineStyle=1`。`HighlightColorIndex=7`，`StrikeThrough=-1`。标题栏仍是对应文件名。
2026-10-11 补跑 `docx-create-round5.ps1`，Cz1–Cz7 通过。无 Repair。COM `Shapes` 有 `Type=6` 的组合。段底纹 RGB=65535。修改密码 `WriteReserved=True`。标题栏仍是 `docx-create-round5.docx`。
2026-10-11 补跑 `docx-table-width.ps1`，Tw1–Tw6 通过。无 Repair。COM `Rows.Alignment=1`，`PreferredWidth=180`，行高 20 磅。标题栏仍是 `docx-table-width-apply.docx` / `docx-table-width-create.docx`。
2026-10-12 补跑 `docx-cell-valign.ps1`，Va1–Va5 通过。无 Repair。COM `VerticalAlignment` 居中为 1、底对齐为 3。
同日补跑 `docx-textbox-anchor.ps1`，Bta1–Bta5 通过。无 Repair。ZIP 有 `anchor="ctr"`，`just` / `dist` 树上对得上。
2026-10-13 补跑 `docx-drawing-hlink-remain.ps1`，Dhr1–Dhr6 通过。无 Repair。框内图 COM `Hyperlink.Address` 对得上。`hyperlinkCount` 仍为 0。OLE id 仍报 `InvalidId`。
同日补跑 `docx-model3d-geom.ps1`，D3g1–D3g6 通过。无 Repair。COM `Width` / `Height` 288×144 磅。浮动 `square` 后再改回 `inline`。页眉 3D 可改尺寸。`replaceModel3d` 不改几何。
同日补跑 `docx-run-spacing.ps1`，Rcs1–Rcs6 通过。无 Repair。COM `Font.Spacing` 为 2 / -1 磅，`Font.Kerning` 为 14 磅。框内 run 可改间距。
同日补跑 `docx-cell-borders.ps1`，Tcb1–Tcb6 通过。无 Repair。COM 该格上边 `LineStyle=7`。`null` 去掉正交边后斜线仍在。
同日补跑 `docx-run-scale.ps1`，Rsc1–Rsc6 通过。无 Repair。COM `Font.Scaling` 为 200 / 50。框内 run 可改缩放。
同日补跑 `docx-theme-remain.ps1`，Thr1–Thr6 通过。无 Repair。空白另存 `extraSchemes=[]`，脚本列表在 `minorCs` 之后。改名 / 脚本 / 补充色板 / `themeFontLang` 树上对得上。
2026-10-14 补跑 `docx-theme-defaults.ps1`，Tdf1–Tdf5 通过。无 Repair。create 后 `objectDefaults` 为 `null`。`lnW=38100`。`lnRef=1` `fontRef=minor`。`null` 清掉。
同日补跑 `docx-cell-props.ps1`，Tcp1–Tcp6 通过。无 Repair。COM `WordWrap` 为 False，`Range.Orientation` 为 3，`FitText` 为 True。`headers=["A1"]`。xlsx 拒写 `setCellFormat`。
同日补跑 `docx-model3d-camera.ps1`，D3c1–D3c5 通过。无 Repair。缺省 `pos.z=66519324` `fov=2700000`，`rot` 三轴 `null`，`lightRig` `null`。改后 `pos.x=50400000` `fov=1892375` `rev=18000000`。`lightRig` 可清。页眉可改 `fov`。

- **C1** ✅
  - Word 16.0 能打开 `manual-check/office/out/docx-create.docx`，标题栏无 Repair
  - `CompatibilityMode=15`
- **C3–C4** ✅
  - 标题 1 / 「标题」，正文 `hello`
- **C5–C7** ✅
  - Word 16.0：`docx-create-pictures.ps1`，4 PASS / 0 FAIL
  - 第二段行内图 `para:00000002/pic:1`
  - 默认页眉浮动 `square`
  - 标题栏无 Repair，正文与页眉都能看见图
- **I1–I5** ✅
  - `docx:document`、`paraCount=2`、大纲 level 1、「标题」、`para:{hex}` 且 `stable`、无 scope 时没有 `paragraphs`，仍有 `pictureCount`
- **A1–A5** ✅
  - Word 打开 apply 产物：第二段为「改过的正文」且仍加粗，第一段仍是「第一章」/标题 1，第三段未改
- **HF1–HF5** ✅
  - Word 另存带页眉页脚样例：名单与展开、改页眉后打开无修复，页眉变「新页眉」、正文与页脚未改
- **D1–D4** ✅
  - Word 16.0：`section:1` 原距离 708/708
  - apply 后 HeaderDistance=72 磅、FooterDistance=36 磅，正文未改
- **L1–L4** ✅
  - Word 16.0：第二节 `headerLink.default` 为 true 且 `LinkToPrevious`
  - 断开后无修复且不再链接
  - 改第二节页眉后第一节仍是原文
  - 正文未改
- **P1–P7** ✅
  - Word 16.0：inspect 列出行内图与浮动图
  - 断链后第二节仍见同一张图
  - 换/插/删无修复
  - 尺寸 144 磅
  - 行内改浮动 `square`
- **B1–B7** ✅
  - Word 16.0：根上 `pictureCount=2`
  - 第二段行内+浮动
  - 换/插/删无修复
  - 尺寸 144 磅
  - 行内改浮动 `square`
  - 改字后图还在
- **E1–E4** ✅
  - 库测试：页眉图 / 正文图导出字节与 `/word/media/…` 一致，`contentType` 为 `image/png`
  - 换图后再导出得到新文件
  - 段落 id 报 `InvalidId`，缺图报 `IdNotFound`
  - 不强制开 Word
- **T1–T9** ✅
  - Word 16.0：`docx-textbox.ps1`，13 PASS / 0 FAIL
  - 行内+浮动+链接+组合
  - 框内改字、插删框、尺寸 144 磅、绕排、红填蓝边、框内插删段与插图、断开再接上
  - 标题栏无 Repair
- **Bta1–Bta5** ✅
  - Word 16.0：`docx-textbox-anchor.ps1`，5 PASS / 0 FAIL
  - 框上有 `textAnchor`；apply `ctr` 后 ZIP 有 `anchor="ctr"`；去掉后无该属性；`just` / `dist` 树上对得上；段落 id 与 xlsx 拒写
  - 标题栏无 Repair
- **Nt1** 🚫
  - Word 16.0 探测：框内再套框只写出故事级兄弟框，`inside-txbxContent` 无 `wps:wsp`
  - 对框内段落 / 框 id 发 `insertTextBox` 报 `InvalidId`
  - 不进 `run.ps1`
- **N1–N6** ✅
  - Word 16.0：`docx-notes.ps1`，7 PASS / 0 FAIL
  - 计数与名单、改注文、插删脚注、编号 `NumberStyle=1`（大写罗马）、注内插图
  - 标题栏无 Repair
- **Cmt1–Cmt7** ✅
  - Word 16.0：`docx-comments.ps1`，8 PASS / 0 FAIL
  - 计数与名单、改字、插删、回复线程（`Comments.Count=4` 且 `Replies=1`）、完成标记、批注内插图
  - 标题栏无 Repair
- **Ik1–Ik4** ✅
  - Word 16.0：`docx-comments.ps1` 墨迹组，4 PASS / 0 FAIL
  - 另存批注注入 `wpi:ink` 后 `kind=ink`
  - `setCommentMeta` / `setCommentDone` 无修复
  - 无可见 `w:t` 时 `setCommentText` 为 `NotImplemented` 且未写文件
  - `deleteComment` 无修复
- **Ch1–Ch8** ✅
  - Word 16.0：`docx-charts.ps1`，9 PASS / 0 FAIL
  - 计数与名单（COM 系列值对得上）、改系列为 11/22/33/44、插饼图、删除、改折线、关图例开标签、组合图当时 `NotImplemented` 且未写文件、`exportChartData` 与 embedding 字节一致
  - 标题栏无 Repair
  - 组合图改数与改类型见下方 Cx 组
- **Cx1–Cx8** ✅
  - Word 16.0：`docx-charts-ext.ps1`，8 PASS / 0 FAIL
  - 圆环 / 雷达 / 气泡各自有 `type`
  - 改数、插入、组合图改数与收成柱形、透视改标题、股价 `type=stock` 且三条系列转柱形无 Repair
- **Cr1–Cr6** ✅
  - Word 16.0：`docx-charts-remain.ps1`，6 PASS / 0 FAIL
  - 股价 `type=stock` 且改数为 10..50
  - 趋势线 / 误差线增删改
  - 次轴 COM `AxisGroup=2`
  - 标题栏无 Repair
- **Cq1–Cq5** ✅
  - Word 16.0：`docx-charts-rare.ps1`，5 PASS / 0 FAIL
  - 插入股价 / 曲面 / 复合条无 Repair
  - 股价三条转柱形、复合饼转饼
  - 增系列与 `type=chartex`、两条股价均拒写
- **Ccx1–Ccx5** ✅
  - Word 16.0：`docx-charts-rare-remain.ps1`，5 PASS / 0 FAIL
  - OHLC / 俯视线框 / 自定义误差线 / chartex funnel 无 Repair
  - 缺 `layoutId` 与两条股价 `InvalidField`，`regionMap` `NotImplemented`
- **Sa1–Sa9** ✅
  - Word 16.0：`docx-smartart.ps1`，10 PASS / 0 FAIL
  - 列表型改字、插删、增删点、尺寸
  - 列表改成循环后文字还在、无修复
  - 库外 uniqueId 得 `NotImplemented` 且未写文件
- **Ip1–Ip5** ✅
  - Word 16.0：`docx-body-para.ps1`，5 PASS / 0 FAIL
  - 插在第二段之后、插到文档末、删第二段、最后一段 `InvalidField`、默认页眉插一段
  - 标题栏无 Repair
- **Nh1–Nh7** ✅
  - Word 16.0：`docx-new-header.ps1`，7 PASS / 0 FAIL
  - 新建默认页眉、首页不同、奇偶页、第二节 even 独立、create even/first、类型未生效与已链接均 `InvalidField`
  - 标题栏无 Repair
- **Sp1–Sp8** ✅
  - Word 16.0：`docx-section.ps1`，8 PASS / 0 FAIL
  - 四边距与横向纸张、距边界仍独立、分栏、垂直对齐与行号、镜像边距
  - 标题栏无 Repair
- **Tb1–Tb8** ✅
  - Word 16.0：`docx-table.ps1`，8 PASS / 0 FAIL
  - 名单、改字、增行增列、删表、create 2×2、合并、最后一行 `InvalidField`
  - 标题栏无 Repair
- **Tf1–Tf6** ✅
  - Word 16.0：`docx-style-table.ps1`，6 PASS / 0 FAIL
  - 名单字段、浮动 `WrapAroundText`、表边距 6 磅、格底纹 RGB=255、斜线 `LineStyle=1`、表格样式与 `bandedRows`
  - 标题栏无 Repair
- **Tw1–Tw6** ✅
  - Word 16.0：`docx-table-width.ps1`，6 PASS / 0 FAIL
  - 名单字段、居中 `Rows.Alignment=1`、`PreferredWidth=180`、行高 20 磅、create 对得上、段落 id 与 xlsx 拒写
  - 标题栏无 Repair
- **Va1–Va5** ✅
  - Word 16.0：`docx-cell-valign.ps1`，5 PASS / 0 FAIL
  - 格上有 `vAlign`；apply `center` 后 COM `VerticalAlignment=1`；create `bottom` 后 COM 为 3；`null` 去掉元素；段落 id 与 xlsx 拒写
  - 标题栏无 Repair
- **Tcb1–Tcb6** ✅
  - Word 16.0：`docx-cell-borders.ps1`，6 PASS / 0 FAIL
  - 格上有 `borders`；apply `double` 后 COM 上边 `LineStyle=7`；只改 `insideH` 后 `top` 仍在；`null` 去掉正交边、斜线留下；create `cellFormats[].borders` 对得上；缺 `borders`、表 id、xlsx 拒写
  - 标题栏无 Repair
- **Tcp1–Tcp6** ✅
  - Word 16.0：`docx-cell-props.ps1`，6 PASS / 0 FAIL
  - 格上有 `noWrap` / `textDirection` / `fitText` / `hideMark` / `headers`；apply 不换行与竖排 COM `WordWrap=False`、`Range.Orientation=3`；适应文字 COM `FitText=True`；表头引用；`null` 去掉；create 与 xlsx 拒写
  - 标题栏无 Repair
- **Bx1–Bx5** ✅
  - Word 16.0：`docx-box-table.ps1`，5 PASS / 0 FAIL
  - 框上 `tableCount`、根上不加；`insertRow` 与黄底；嵌套表 `table:2`；打开 `docx-box-table-nest.docx` 无 Repair；脚注 `insertTable` 得 `InvalidId`
  - COM `Tables.Count=1`
- **Bf1–Bf5** ✅
  - Word 16.0：`docx-box-fields.ps1`，5 PASS / 0 FAIL
  - 框上 `hyperlinkCount`、根上不加；书签与 DATE 域；公式拼音控件各 1；含链段 `setParaText` 得 `NotImplemented`；脚注 `insertField` 得 `InvalidId`
  - COM `Hyperlinks.Count=1`
- **Rv1–Rv6** ✅
  - Word 16.0：`docx-revision.ps1`，6 PASS / 0 FAIL
  - 名单、接受删除、拒绝插入、接受插入、拒绝删除、带修订段 `setParaText` 得 `NotImplemented`
  - 标题栏无 Repair
- **Rm1–Rm9** ✅
  - Word 16.0：`docx-revision-remain.ps1`，9 PASS / 0 FAIL
  - 表属性 Change 名单与接受 / 拒绝、段落标记插入接受 / 拒绝、表行删除接受 / 拒绝
  - 标题栏无 Repair
- **Dg1–Dg8** ✅
  - Word 16.0：`docx-groups.ps1`，8 PASS / 0 FAIL
  - 无名单时根上有 `connectorCount=0`
  - `insertConnector` 后 `prst=straightConnector1`、COM 连接线可见
  - 删后计数为 0
  - 两框 / 两浮动图 `groupShapes` 后 COM `Type=6`、树上仍是两个成员
  - `ungroup` 后独立
  - 页眉连接线容器计数为 1、正文仍为 0
  - 行内当成员 / `setShapeGeom` / 错 id / xlsx 用段落 id 分别得 `InvalidField` / `IdNotFound` / `InvalidId`
  - 标题栏无 Repair
- **Hb1–Hb7** ✅
  - Word 16.0：`docx-hyperlink.ps1`，7 PASS / 0 FAIL
  - 名单、改 URL、插链、删链、插书签、删书签、带链段 `setParaText` 得 `NotImplemented`
  - 标题栏无 Repair
- **Dh1–Dh6** ✅
  - Word 16.0：`docx-drawing-hlink.ps1`，6 PASS / 0 FAIL
  - 图上 `insertHyperlink` 后 inspect `hyperlink.url` 与 COM `Hyperlink.Address` 对得上，`hyperlinkCount` 为 0
  - `setHyperlink` 改 URL；`deleteHyperlink` 后 `hyperlink` 为 `null`、图还在
  - 文本框链 inspect 对得上
  - 内部书签 `kind=anchor`，关系 `Target="#SrcBm"` 且无 `TargetMode`
  - 图上带 `text` 得 `InvalidField`
  - 标题栏无 Repair
- **Dhr1–Dhr6** ✅
  - Word 16.0：`docx-drawing-hlink-remain.ps1`，6 PASS / 0 FAIL
  - 框内图、无框形状、连接线可写 DrawingML 链
  - create `shapes[].hyperlink` 后树上对得上
  - `deleteHyperlink` 后 `hyperlink` 为 `null`、对象还在
  - OLE id 仍报 `InvalidId`
  - 标题栏无 Repair
- **D3g1–D3g6** ✅
  - Word 16.0：`docx-model3d-geom.ps1`，6 PASS / 0 FAIL
  - `setModel3dSize` 后树上 `cx` / `cy` 对得上
  - `setModel3dWrap` 浮动 `square` 后再改回 `inline`
  - `replaceModel3d` 不改几何
  - 页眉 3D 可改尺寸
  - 空 `cx` / `cy`、对 3D id 发 `setPictureSize` 拒写
  - 标题栏无 Repair
- **D3c1–D3c5** ✅
  - Word 16.0：`docx-model3d-camera.ps1`，5 PASS / 0 FAIL
  - 缺省相机 / 旋转 / 光源对得上
  - `setModel3dCamera` 写位移、视野、三轴与 `lightRig.rev`
  - `lightRig` 为 `null` 后树上为 `null`
  - 页眉 3D 可改 `fov`
  - 空字段与负 `fov` 拒写
  - 标题栏无 Repair
- **Ds1–Ds8** ✅
  - Word 16.0：`docx-drawing-shape.ps1`，8 PASS / 0 FAIL
  - `insertShape` 矩形 `prst=rect`、COM `AutoShapeType=1`；椭圆 `prst=ellipse`、`AutoShapeType=9`
  - `setShapeSize` 后 `cx` / `cy` 对得上
  - `setShapeFill` 红、`setShapeLine` 蓝，COM RGB 对得上
  - `deleteShape` 后 `shapeCount` 为 0
  - 页眉插入后容器计数为 1、正文仍为 0
  - 连接线几何名 / 带 `text` / `setShapeGeom` 分别得 `InvalidField`
  - 标题栏无 Repair
- **Fd1–Fd7** ✅
  - Word 16.0：`docx-field.ps1`，7 PASS / 0 FAIL
  - 名单、改 PAGE 为 DATE、插 DATE、删域留结果、删域留指令、页眉 PAGE、带域段 `setParaText` 得 `NotImplemented`
  - 标题栏无 Repair
- **Ft1–Ft7** ✅
  - Word 16.0：`docx-field-types.ps1`，7 PASS / 0 FAIL
  - `code` 写 PAGE 无 `form`
  - MERGEFIELD 带 `arg`
  - FORMTEXT 有 `ffData` 且 `FormFields.Count≥1`
  - 复选 `checked=true`
  - 下拉 `items` 含 A/B
  - create `NUMPAGES`
  - 同时给 `instr`/`code`、缺 `arg`、`INCLUDETEXT` 为 `InvalidField`
  - 标题栏无 Repair
- **Nl1–Nl6** ✅
  - Word 16.0：`docx-numbering.ps1`，6 PASS / 0 FAIL
  - 名单、段上 `num`/`ilvl`、插 decimal 定义、绑两段、改 bullet、解绑
  - 标题栏无 Repair
- **St1–St6** ✅
  - Word 16.0：`docx-style.ps1`，6 PASS / 0 FAIL
  - 名单、换 Heading2、大纲对得上、改 Normal 为 24 磅、去掉 Heading1 加粗、解绑
  - 标题栏无 Repair
- **Sy1–Sy6** ✅
  - Word 16.0：`docx-style-table.ps1`，6 PASS / 0 FAIL
  - 插字符样式、COM 红加粗、run `style=ooxpChar`、删除自定义、拒删 Normal、xlsx / numbering 拒写
  - 标题栏无 Repair
- **Sc1–Sc6** ✅
  - Word 16.0：`docx-insert-section.ps1`，6 PASS / 0 FAIL
  - 插节后两节且第二节 `headerLink.default`
  - 打开无修复
  - 断链后两节页眉能独立改字
  - 删节后只剩一节、无孤儿 `headerN.xml`
  - create 带 `sectionType` 为两节
  - 标题栏无 Repair
- **Tc1–Tc5** ✅
  - Word 16.0：`docx-toc.ps1`，5 PASS / 0 FAIL
  - 无 `scope.tocs` 时只有 `tocCount`
  - 插入后 `toc:1` / `kind=toc` 且 `fields` 含 TOC
  - 打开无修复
  - 更新域后可见「第一章」
  - 删后 `tocCount=0` 标题还在
  - `kind=tof` 树上为 `tof`
  - 标题栏无 Repair
- **Mm1–Mm6** ✅
  - Word 16.0：`docx-math.ps1`，6 PASS / 0 FAIL
  - 无 `scope.maths` 时只有 `mathCount`
  - inline 插入后 `math:1` / `kind=inline`
  - 打开无修复且 `OMaths.Count=1`、`Type=1`
  - `setMath` 后仍是公式
  - display `Type=0`
  - 删后 `mathCount=0` 标题还在
  - 页眉 `mathCount=1` 且无修复
  - 标题栏无 Repair
- **Ms1–Ms8** ✅
  - Word 16.0：`docx-math-struct.ps1`，8 PASS / 0 FAIL
  - insert 分数后 `expr[0].type=frac`、display
  - 平方根 `degHide`
  - nary `∑`
  - 嵌套 `sSup`
  - `setMath` 给 `expr` 不摊平
  - 只给 `text` 摊成一条 `text`
  - 同时给 `text` 与 `expr` 为 `InvalidField`
  - create `expr` 为分数
  - 标题栏无 Repair
- **Cc1–Cc6** ✅
  - Word 16.0：`docx-sdt.ps1`，6 PASS / 0 FAIL
  - 无 `scope.contentControls` 时只有 `contentControlCount`
  - plainText 插入后 `sdt:1` / `kind=plainText`
  - 打开无修复且 `ContentControls.Count=1`、`Type=1`
  - `setContentControl` 后仍是纯文本控件
  - 下拉 `Type=4`
  - 删后 `contentControlCount=0` 标题与原文还在
  - 页眉 `contentControlCount=1` 且无修复
  - 标题栏无 Repair
- **Ct1–Ct8** ✅
  - Word 16.0：`docx-sdt-types.ps1`，8 PASS / 0 FAIL
  - 复选 `kind=checkbox` 且 `checked=true`、COM Type=8
  - 图片 `kind=picture` 且 Type=2
  - 组合 `kind=group` 且 Type=7
  - 公式 `kind=equation`、XML 含 `w:equation`、COM 可以没有该控件
  - 改复选、换图
  - 给 `group` 发 `setContentControl` 为 `NotImplemented`
  - `picture` 不给 `file` 为 `InvalidField`
  - 标题栏无 Repair
- **Cf1–Cf6** ✅
  - Word 16.0：`docx-create-full.ps1`，6 PASS / 0 FAIL
  - 两节、饼图、脚注、尾注、批注
  - 标题栏无 Repair
  - `commentsExtended` 的 CT 须为 `…wordprocessingml.commentsExtended+xml`
- **Cs1–Cs9** ✅
  - Word 16.0：`docx-create-remain.ps1`，9 PASS / 0 FAIL
  - 链 / 书签 / DATE 与 TOC 域 / `num:1` / 行内公式 / 纯文本控件 / `tocCount=1` / 批注回复 `parent=comment:0`
  - 打开无 Repair，COM 能看见链、书签、编号、公式、内容控件与回复
- **Cy1–Cy8** ✅
  - Word 16.0：`docx-create-round4.ps1`，8 PASS / 0 FAIL
  - 样式、表外观、形状、连接线、拼音、题注 / 索引 / 书目、嵌入字体、DrawingML 链
  - 只给 `protection.password`、不给限制编辑种类，为 `InvalidField`
  - 标题栏无 Repair
- **Cd1–Cd6** ✅
  - Word 16.0：`docx-container-drawing.ps1`，6 PASS / 0 FAIL
  - 批注内柱形图
  - 注内与批注内 `insertTextBox` 为 `InvalidField`
  - 框内图表根计数不含该图
  - 标题栏无 Repair
- **Dp1–Dp5** ✅
  - Word 16.0：`docx-docprops.ps1`，5 PASS / 0 FAIL
  - 根上 `props`、`creator=ooxp`
  - 只改标题后面板标题是新字、公司名还在
  - 自定义 `prop:部门`
  - 删后名单空
  - 错 id 为 `InvalidId`
  - 标题栏无 Repair
- **Pf1–Pf6** ✅
  - Word 16.0：`docx-format.ps1`，6 PASS / 0 FAIL
  - 无 `scope.runs` 时有 `runCount`、没有名单
  - 对齐为居中
  - 第一 run 加粗后第二 run 红色还在
  - 段前 12 磅、段后 6 磅
  - 标题栏无 Repair
- **Bd1–Bd3、Rh1–Rh3** ✅
  - Word 16.0：`docx-para-run-remain.ps1`，6 PASS / 0 FAIL
  - 上边框 `single`、黄底 `FFFF00`、左对齐制表位 720
  - COM 底纹 RGB=65535、上边框 `LineStyle=1`
  - `highlight=yellow` 时 `HighlightColorIndex=7`，`StrikeThrough=-1`
  - 上下标与双删除线树上对得上
  - xlsx / 不认识的 `highlight` 为 `InvalidField`
  - 标题栏无 Repair
- **Rcs1–Rcs6** ✅
  - Word 16.0：`docx-run-spacing.ps1`，6 PASS / 0 FAIL
  - `setRunFormat` 写 `spacing` / `kern` 后树上对得上
  - 负 `spacing` 可写
  - `null` 去掉对应元素
  - 框内 run 同样认
  - 字符串 `spacing`、负 `kern`、xlsx 上发此 op 拒写
  - 标题栏无 Repair
- **Rsc1–Rsc6** ✅
  - Word 16.0：`docx-run-scale.ps1`，6 PASS / 0 FAIL
  - `setRunFormat` 写 `scale: 200` 后 COM `Font.Scaling=200`；`50` 后为 50
  - `null` 去掉 `w:w`
  - 框内 run 同样认
  - create `runs[].scale` 对得上
  - `0` / `601`、xlsx 拒写
  - 标题栏无 Repair
- **Thr1–Thr6** ✅
  - Word 16.0：`docx-theme-remain.ps1`，6 PASS / 0 FAIL
  - 空白另存 `extraSchemes` 为数组，`fonts` 在 `minorCs` 之后有脚本列表
  - 改配色名 / theme 名后树上对得上
  - 改脚本字体后树上对得上
  - 补充色板写入后树上对得上，`null` 清掉列表
  - `themeFontLang` 写入后树上对得上
  - 无 theme / 错 id / xlsx 无 theme 同样 `IdNotFound`
- **Tdf1–Tdf5** ✅
  - Word 16.0：`docx-theme-defaults.ps1`，5 PASS / 0 FAIL
  - create 给了 `document.theme` 后 `objectDefaults` 为 `null`
  - `sp.lnW=38100` 后树上对得上
  - `sp.style.lnRef=1` `fontRef=minor` 后树上对得上
  - `objectDefaults: null` 后为 `null`
  - 段落 id / 缺字段 / 非法 `fontRef` / 无 theme 拒写
- **Ea1–Ea8** ✅
  - Word 16.0：`docx-east-asia.ps1`，8 PASS / 0 FAIL
  - 段禁则树上 `kinsoku` / `overflowPunct` / `autoSpaceDE` 对得上
  - COM `FarEastLineBreakControl=-1`、`HangingPunctuation=-1`、`AddSpaceBetweenFarEastAndAlpha=0`
  - 强调点 `em=dot`、`EmphasisMark=1`；两行合一 `combine=true`
  - 拼音 `rubyCount=1`，改 `rt` 后树上对得上
  - 整篇 `characterSpacingControl=compressPunctuation`
  - xlsx / 空 `base` 为 `InvalidField`
  - 标题栏无 Repair
- **Ol1–Ol5** ✅
  - Word 16.0：`docx-ole.ps1`，6 PASS / 0 FAIL
  - 无名单时 `oleObjectCount=1`
  - `progId` 为 `Excel.Sheet.12`
  - `exportMedia` 与 embeddings 字节一致
  - 删除后计数为 0
  - 插入后树上能看见
  - 错 id 为 `InvalidId` 且未写文件
  - 标题栏无 Repair
- **Pr1–Pr6** ✅
  - Word 16.0：`docx-protect.ps1`，6 PASS / 0 FAIL
  - 只读保护 `enforced=true`、`edit=readOnly`
  - 关掉后 `ProtectionType=-1`
  - 窗体保护为 `2`
  - 只读推荐为 true
  - 带哈希 `hasPassword=true` 且改 `enforced` 得 `NotImplemented`
  - 错 id / 空字段拒写
  - 标题栏无 Repair
- **Ph1–Ph5** ✅
  - Word 16.0：`docx-protect-hash.ps1`，5 PASS / 0 FAIL
  - 带 `password` 后 `hasPassword=true`、树无哈希或明文
  - COM `ProtectionType=3`、`Unprotect("secret")` 成功、错误口令失败、无 Repair
  - 对得上的口令 `enforced: false` 后无 Repair
  - 空口令 / 错口令为 `InvalidField`，不给口令改哈希为 `NotImplemented`
- **Wm1–Wm5** ✅
  - Word 16.0：`docx-watermark.ps1`，5 PASS / 0 FAIL
  - 文字水印 id 为 `header:default:1/watermark:1`、`kind=text`
  - 删除后计数为 0
  - 图片水印 `kind=picture`
  - 普通页眉图不进 `watermarkCount`
  - 错 id / 同时给 `text` 与 `file` 拒写
  - 标题栏无 Repair
- **Th1–Th5** ✅
  - Word 16.0：`docx-theme.ps1`，5 PASS / 0 FAIL
  - 空白另存根上有 `theme`，`accent1` 为六位十六进制
  - 改色为 `FF0000`
  - 改字为 Arial
  - 无 theme 部件得 `IdNotFound`
  - 错 id / 空槽位拒写
  - 标题栏无 Repair
- **Cp1–Cp6** ✅
  - Word 16.0：`docx-caption-xref.ps1`，6 PASS / 0 FAIL
  - 无 scope 时只有计数
  - 插入后 `caption:1` / `identifier=Figure`
  - Word 更新域后交叉引用可见 Figure
  - 删交叉引用与删题注无修复
  - 页眉 `insertCaption` 为 `InvalidId`，xlsx 上为 `InvalidField`
  - 标题栏无 Repair
- **Ix1–Ix6** ✅
  - Word 16.0：`docx-index.ps1`，6 PASS / 0 FAIL
  - 无 scope 时只有计数
  - 插入后 `xe:1` / `index:1`，条目为 fox
  - Word 更新域后可见 fox
  - 删索引与删索引项无修复
  - 页眉 `insertIndex` 为 `InvalidId`，xlsx 上为 `InvalidField`
  - 标题栏无 Repair
- **Bi1–Bi6** ✅
  - Word 16.0：`docx-biblio.ps1`，6 PASS / 0 FAIL
  - 无 scope 时只有计数
  - 插入后 `source:1` / `cite:1` / `biblio:1`
  - Word 更新域后可见 `(Ecma, 2006)`
  - 删书目、引文与源无修复
  - 页眉 `insertBibliography` 为 `InvalidId`，xlsx 上为 `InvalidField`
  - 标题栏无 Repair
- **Bb1–Bb6** ✅
  - Word 16.0：`docx-glossary.ps1`，6 PASS / 0 FAIL
  - 无 scope 时只有 `docPartCount`
  - 插入后 `docPart:1`
  - Word 无 Repair
  - `insertDocPartContent` 后正文可见 `glossary clause`
  - 删除后计数 0
  - 页眉 `insertDocPartContent` 为 `InvalidId`，xlsx 上为 `InvalidField`
  - 标题栏无 Repair
- **Fe1–Fe6** ✅
  - Word 16.0：`docx-font-embed.ps1`，6 PASS / 0 FAIL
  - 无 scope 时只有 `embeddedFontCount` 与 `fontEmbed`
  - 插入后 `font:1`
  - Word 无 Repair
  - `exportMedia` 与 zip 字节一致
  - 删除后计数 0 且 zip 无 `word/fonts/font1.odttf`
  - xlsx 上 `insertEmbeddedFont` 为 `InvalidField`
  - 标题栏无 Repair
- **Pb1–Pb6** ✅
  - Word 16.0：`docx-page-bg.ps1`，6 PASS / 0 FAIL
  - 无背景时 `color=null`
  - set 后 `FF0000` 且 `display=true`
  - Word 无 Repair
  - 删背景后 `color` 又为 null
  - `setPageBorders` 后节上有四边
  - xlsx 上 `setPageBackground` 为 `InvalidField`
  - 标题栏无 Repair
- **O1** 🚫
  - 领域层不做原样保存，与 xlsx 清单相同
  - 零脏写回见 OPC 库测试
  - Word 另存 create 产物见 `docx-O1-word-resave`
  - 此前记的 `SaveAs2` 挂起，拆分后重跑没再出现，一秒内返回

## create

用 `docs/examples/docx-create.json` 生成 `note.docx`。

- **C1**
  - 检查：Word 打开
  - 通过标准：无修复提示、无「文件损坏」
  - `CompatibilityMode` 为 `15`
- **C3**
  - 检查：第一段
  - 通过标准：样式为标题 1，文字为 `标题`
- **C4**
  - 检查：第二段
  - 通过标准：正文，文字为 `hello`
- **C5**
  - 检查：带图正文
  - 通过标准：用 `docs/examples/docx-create-pictures.json`（换成本机 PNG）生成
  - 第二段有行内图，inspect id 为 `para:…/pic:1`
- **C6**
  - 检查：带图页眉
  - 通过标准：同上，默认页眉有浮动 `square`，Word 能看见
- **C7**
  - 检查：Word 打开带图产物
  - 通过标准：无修复
  - 图可见
- **C8**
  - 检查：补齐
  - 通过标准：用 `docs/examples/docx-create-full.json` 生成：两节、饼图、一条脚注、一条尾注、一条批注
  - inspect 对得上
  - Word 无修复
  - 见下方 Cf 组
- **C9**
  - 检查：剩余
  - 通过标准：用 `docs/examples/docx-create-remain.json` 生成：超链接、书签、域、目录、编号、公式、内容控件、批注回复
  - inspect 对得上
  - Word 无修复
  - 见下方 Cs 组

## inspect

对上面的文件发 `docs/examples/docx-inspect.json`。

- **I1**
  - 检查：根节点
  - 通过标准：`id` 为 `docx:document`，`paraCount` 为 `2`
- **I2**
  - 检查：大纲
  - 通过标准：`outline` 含第一段，`level` 为 `1`，`text` 为 `标题`
- **I3**
  - 检查：段落 id
  - 通过标准：两段都有 `para:{hex}`，`stable` 为 `true`
- **I4**
  - 检查：第二段
  - 通过标准：`text` 为 `hello`
- **I5**
  - 检查：无 scope
  - 通过标准：只发 path 时没有
    - `paragraphs`
    - `outline`
    - `paraCount`
    - `pictureCount`
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
    - `embeddedFontCount`
    - `fontEmbed`
    - `pageBackground`
    - `headers`
    - `footers`
    - `sections`
    - `props`
    - `protection`
    - `watermarkCount`

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

准备一份 Word 亲手另存的样例：至少有默认页眉与默认页脚，页眉里有一段无域、无超链接的可见文字。不要用库 `create` 产物（这两项要测 Word 另存的已有页眉，不要用 create 刚写的默认页眉）。
对它 `inspect` 时带 `scope.headerFooter: true`，再用返回的页眉段落 id 做 `setParaText`。

- **HF1**
  - 检查：名单
  - 通过标准：无 scope 时也有 `headers`、`footers`
  - 容器带 `id`、`type`、`sections`、`paraCount`、`pictureCount`，不带 `paragraphs`、`pictures`
- **HF2**
  - 检查：展开
  - 通过标准：`headerFooter: true` 时页眉节点有 `paragraphs`，段落 id 形如 `header:default:1/para:…`
- **HF3**
  - 检查：打开
  - 通过标准：apply 改页眉后 Word 能打开，无修复提示
- **HF4**
  - 检查：目标
  - 通过标准：页眉可见文字变成请求里的 `text`
  - 正文未改
- **HF5**
  - 检查：未改部件
  - 通过标准：未点名的页脚与 `styles.xml` 看起来不变

## 距边界

对 Word 亲手另存或 create 后再经 Word 打开过的文件 `inspect`，根上应有 `sections`。再用 `setHeaderFooterDistance` 改第一节。
Word COM 的 `PageSetup.HeaderDistance` / `FooterDistance` 单位是磅，等于 twip ÷ 20。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| D1 | 名单 | 无 scope 时有 `sections`；第一节 `id` 为 `section:1`，带 `headerDistance`、`footerDistance` |
| D2 | 打开 | apply 后 Word 能打开，无修复提示 |
| D3 | 距离 | 页眉/页脚距边界变成请求值（允许 0.05 磅误差） |
| D4 | 正文 | 正文段落文字未改 |

## 链接前节

准备一份 Word 亲手另存的两节样例：第二节默认页眉勾选「链接到前一节」。不要用库 `create` 产物（本组仍要求 Word 另存的两节样例）。
对它 `inspect` 后发 `setHeaderFooterLink` 断开第二节默认页眉，再改第二节页眉文字。Word COM 的 `Headers.Item(1).LinkToPrevious` 对应默认页眉。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| L1 | 名单 | 第二节 `headerLink.default` 为 `true`；COM 读第二节 `LinkToPrevious` 为 `true` |
| L2 | 打开 | 断开后 Word 能打开，无修复提示；COM 读第二节 `LinkToPrevious` 为 `false` |
| L3 | 目标 | 改第二节页眉后，第一节页眉仍是原文 |
| L4 | 正文 | 正文段落文字未改 |

## 页眉页脚图文混排

准备一份 Word 亲手另存的样例：默认页眉里同时有一张行内图和一张浮动图（`wrapSquare`）。不要用库 `create` 产物。另备一张本地 PNG 供换图、插图。
手测脚本 `manual-check/office/cases/docx-picture.ps1`。

- **P1**
  - 检查：名单
  - 通过标准：无 scope 时容器有 `pictureCount`、没有 `pictures`
  - `pictures: true` 时两张图都有 id，`kind` 分别为 `inline` 与 `anchor`，浮动 `wrap` 为 `square`
- **P2**
  - 检查：断链
  - 通过标准：两节样例、第二节链着带图页眉
  - 断开后 Word 无修复，第二节仍能看见同一张图
- **P3**
  - 检查：换图
  - 通过标准：`replacePicture` 后 Word 无修复
  - 位置与绕排不变
  - 可见图已换成新文件
- **P4**
  - 检查：插图
  - 通过标准：`insertPicture` 往页眉段落插行内图，Word 能看见且无修复
- **P5**
  - 检查：删图
  - 通过标准：`deletePicture` 后该图消失，Word 无修复
- **P6**
  - 检查：尺寸
  - 通过标准：`setPictureSize` 后 COM 或目视尺寸对上（允许 EMU 换算误差）
- **P7**
  - 检查：绕排
  - 通过标准：`setPictureWrap` 把行内改成浮动 `square` 后，Word 按四周型显示

## 正文图

准备一份 Word 亲手另存的样例：正文某段同时有一张行内图和一张浮动图（`wrapSquare`）。不要用库 `create` 产物。另备一张本地 PNG 供换图、插图。
手测脚本 `manual-check/office/cases/docx-body-picture.ps1`。

- **B1**
  - 检查：名单
  - 通过标准：无 scope 时根上有 `pictureCount`、没有正文 `pictures`
  - `from`+`maxParas`+`pictures` 时目标段两张图都有 id，形如 `para:…/pic:…`，`kind` 分别为 `inline` 与 `anchor`，浮动 `wrap` 为 `square`
- **B2**
  - 检查：换图
  - 通过标准：`replacePicture` 后 Word 无修复
  - 位置与绕排不变
  - 可见图已换成新文件
- **B3**
  - 检查：插图
  - 通过标准：`insertPicture` 往正文段落插行内图，Word 能看见且无修复
- **B4**
  - 检查：删图
  - 通过标准：`deletePicture` 后该图消失，Word 无修复
- **B5**
  - 检查：尺寸
  - 通过标准：`setPictureSize` 后 COM 或目视尺寸对上（允许 EMU 换算误差）
- **B6**
  - 检查：绕排
  - 通过标准：`setPictureWrap` 把行内改成浮动 `square` 后，Word 按四周型显示
- **B7**
  - 检查：改字
  - 通过标准：对该段 `setParaText` 后文字已改、图还在，Word 无修复

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

- **T1**
  - 检查：名单
  - 通过标准：无 scope 时根上有 `textBoxCount`、没有正文 `textBoxes`
  - `from`+`maxParas`+`textBoxes` 时目标段有框 id，形如 `para:…/txbx:…`
- **T2**
  - 检查：框内
  - 通过标准：`textBoxContent` 时框内段落 id 带框前缀，`text` 对，不并进宿主 `text`
- **T3**
  - 检查：链接
  - 通过标准：链接框的 `link.prev` / `link.next` 互指
- **T4**
  - 检查：改字
  - 通过标准：对框内段落 `setParaText` 后 Word 无修复，框还在
- **T5**
  - 检查：插删框
  - 通过标准：`insertTextBox` / `deleteTextBox` 后 Word 无修复
- **T6**
  - 检查：尺寸绕排
  - 通过标准：`setTextBoxSize` / `setTextBoxWrap` 后 COM 或目视对上
- **T7**
  - 检查：填色描边
  - 通过标准：`setTextBoxFill` / `setTextBoxLine` 后可见变化，无修复
- **T8**
  - 检查：框内段与图
  - 通过标准：框内插删段、插图与正文图规则相同
- **T9**
  - 检查：链接
  - 通过标准：`setTextBoxLink` 断开或接上后 Word 能继续编辑链条

## 文本框内垂直锚点

框内文字相对框的 `wps:bodyPr/@anchor`。不要和 `setTextBoxDirection`、格级 `vAlign`、xlsx `setTextBoxAnchor` 混。不算排版引擎。
手测脚本 `manual-check/office/cases/docx-textbox-anchor.ps1`。编号 Bta，不要和 Tb、T、Bx 撞。现有 T1–T9 继续过。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Bta1 | 名单字段 | 给了 `scope.textBoxes` 后框上有 `textAnchor`，缺属性为 `null` |
| Bta2 | apply | `setTextBoxTextAnchor` 写 `ctr` 后树上为 `ctr`。ZIP 有 `anchor="ctr"`。Word 无修复 |
| Bta3 | 去掉 | `textAnchor: null` 后树上为 `null`，ZIP 无 `anchor=` |
| Bta4 | just / dist | 写 `just` 与 `dist` 后树上对得上。Word 能打开无 Repair |
| Bta5 | 拒写 | 对段落 id 发此 op 得 `InvalidId`。xlsx 上发此 op 得 `InvalidField` |

## 脚注与尾注

准备一份 Word 亲手另存的样例：至少一条用户脚注、一条用户尾注。不要用库 `create` 产物当唯一样例；apply 手测仍用 Word 另存。
手测脚本为 `manual-check/office/cases/docx-notes.ps1`。

- **N1**
  - 检查：计数与名单
  - 通过标准：无 `notes` 时根上有 `footnoteCount` / `endnoteCount`、没有名单
  - `notes: true` 时用户注 id 为 `footnote:{id}` / `endnote:{id}`，`host` 对，分隔符只在 `noteSeparators`
- **N2**
  - 检查：改注文
  - 通过标准：对注内段落 `setParaText` 后 Word 无修复，正文未改
- **N3**
  - 检查：插入
  - 通过标准：`insertFootnote` 后 Word 能看见引用与注文，无修复
- **N4**
  - 检查：删除
  - 通过标准：`deleteFootnote` 后引用与注文都消失，无修复
- **N5**
  - 检查：编号
  - 通过标准：`setNoteNumbering` 改 `numFmt` 后 Word 打开时编号样式变了（允许打开时才重画）
- **N6**
  - 检查：注内图
  - 通过标准：注内 `replacePicture` 或 `insertPicture` 后 Word 无修复

## 批注

准备一份 Word 亲手另存的样例：至少两条根批注、一条回复。不要用库 `create` 产物当唯一样例；apply 手测仍用 Word 另存。
手测脚本为 `manual-check/office/cases/docx-comments.ps1`。编号用 Cmt，避免和 create 的 C 组重号。

- **Cmt1**
  - 检查：计数与名单
  - 通过标准：无 `comments` 时根上有 `commentCount`、没有名单
  - `comments: true` 时 id 为 `comment:{id}`，`start` / `end` / `parent` / `done` 对
- **Cmt2**
  - 检查：改字
  - 通过标准：`setCommentText` 后 Word 无修复，正文未改
- **Cmt3**
  - 检查：插入
  - 通过标准：`insertComment` 后审阅窗格能看见新批注，无修复
- **Cmt4**
  - 检查：删除
  - 通过标准：`deleteComment` 后范围标记与回复都消失，无修复
- **Cmt5**
  - 检查：回复
  - 通过标准：`replyComment` 后 `parent` 指向父批注，Word 能看见线程
- **Cmt6**
  - 检查：完成
  - 通过标准：`setCommentDone` 后 COM `Comment.Done` 为 true，或 UI 显示已解决
- **Cmt7**
  - 检查：批注内图
  - 通过标准：批注内 `insertPicture` 后 Word 无修复

## 批注墨迹

Word 亲手另存或在另存批注样例的 `comments.xml` 里带 `inkml:ink`（或本地名为 `ink`）。不把墨迹当位图列出。
手测并进 `manual-check/office/cases/docx-comments.ps1`。编号用 Ik。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Ik1 | 名单 | ✅ `comments: true` 时墨迹批注 `kind` 为 `ink`，没有墨迹位图字段 |
| Ik2 | 元数据 | ✅ `setCommentMeta` 后作者或缩写变了，Word 无修复 |
| Ik3 | 完成 | ✅ `setCommentDone` 后 `done` 为 true，无修复 |
| Ik4 | 改字仍拒 | ✅ 没有可见 `w:t` 时 `setCommentText` 得 `NotImplemented`，文件未写坏；`deleteComment` 后无修复 |

## 嵌入图表

准备一份 Word 亲手另存的样例：至少一张柱形图（带标题与两个系列）。不要用库 `create` 产物当唯一样例；apply 手测仍用 Word 另存。
手测脚本为 `manual-check/office/cases/docx-charts.ps1`。编号用 Ch，避免和 create 的 C 组、批注的 Cmt 组重号。

- **Ch1**
  - 检查：计数与名单
  - 通过标准：✅ 无 `charts` 时根上有 `chartCount`、没有名单
  - `charts: true` 时 id 为 `{段落}/chart:{docPrId}`，`type` / `title` / 系列值与数据表一致
- **Ch2**
  - 检查：改系列
  - 通过标准：✅ `setChartSeries` 后 Word 无修复，数据表是新值
- **Ch3**
  - 检查：插入
  - 通过标准：✅ `insertChart` 一张饼图后 Word 能看见，无修复
- **Ch4**
  - 检查：删除
  - 通过标准：✅ `deleteChart` 后图与数据都消失，无修复
- **Ch5**
  - 检查：改类型
  - 通过标准：✅ `setChartType` 为 `line` 后 Word 按折线显示（允许样式重置）
- **Ch6**
  - 检查：图例与标签
  - 通过标准：✅ `setChartLegend` / `setChartDataLabels` 后 Word 无修复，开关能对上
- **Ch7**
  - 检查：组合图
  - 通过标准：✅ 当时对组合图发 `setChartType` 得到 `NotImplemented`，文件未写坏
  - 现行契约已放开，改看 Cx5–Cx6
- **Ch8**
  - 检查：导出数据
  - 通过标准：✅ `exportChartData` 写出的 xlsx 与 embedding 部件字节一致
  - 不强制开 Word

## 扩展图表

准备一份 Word 亲手另存的样例：至少各一张圆环、雷达、气泡、柱线组合，以及一张带 `flags.pivot` 的图（可用 Excel 透视图表粘进 Word 再另存）。不要用库 `create` 产物当唯一样例。
手测脚本为 `manual-check/office/cases/docx-charts-ext.ps1`。编号用 Cx。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Cx1 | 类型 | ✅ 圆环 / 雷达 / 气泡的 `type` 分别为 `doughnut` / `radar` / `bubble`，不再是 `other`；气泡系列有 `sizes`，值与数据表一致 |
| Cx2 | 改圆环或雷达 | ✅ `setChartSeries` 后 Word 无修复，数据表是新值 |
| Cx3 | 改气泡 | ✅ `setChartSeries` 改 `values` 与 `sizes` 后 Word 无修复 |
| Cx4 | 插入 | ✅ `insertChart` 一张圆环、一张雷达后 Word 能看见，无修复 |
| Cx5 | 组合图改数 | ✅ 组合图 `setChartSeries` 后两条系列的值都对，无修复 |
| Cx6 | 组合图改类型 | ✅ 组合图 `setChartType` 为 `col` 后变成柱形，无修复 |
| Cx7 | 透视图表 | ✅ `setChartTitle` 能改；`setChartSeries` 得到 `NotImplemented`，文件未写坏 |
| Cx8 | 股价改类型 | ✅ 股价 `type` 为 `stock`；三条系列时 `setChartType` 为 `col` 成功，Word 无修复 |

## 图表稀有类型

`insertChart` 写出股价、曲面、复合饼；`setChartType` 在系列条数对得上时转入或转出。chartex 可以改标题、图例和系列；换类型、轴标题、数据标签、趋势线和误差线仍报 `NotImplemented`。
手测脚本 `manual-check/office/cases/docx-charts-rare.ps1`。编号用 Cq。xlsx / pptx 各有对应脚本。

- **Cq1**
  - 检查：插入股价
  - 通过标准：✅ `insertChart` `type=stock` 三条系列后 Word 无修复，`type` 为 `stock`
- **Cq2**
  - 检查：插入曲面
  - 通过标准：✅ `insertChart` `type=surface` 三条系列后 Word 无修复，`type` 为 `surface`
- **Cq3**
  - 检查：插入复合饼
  - 通过标准：✅ `insertChart` `type=ofPie` 一条系列后 Word 无修复
  - `ofPieType=bar` 为复合条
- **Cq4**
  - 检查：改类型
  - 通过标准：✅ 股价三条转 `col` 成功
  - 复合饼转 `pie` 成功
  - 无修复
- **Cq5**
  - 检查：仍拒绝
  - 通过标准：✅ 对股价 `addChartSeries` 得 `NotImplemented`
  - 源里已有 chartex 时 `setChartType` 得 `NotImplemented`
  - 源无 chartex 时 `insertChart type=chartex` 得 `InvalidField`
  - 两条系列的股价 `insertChart` 得 `InvalidField`
  - 均未写坏文件

## 图表稀有剩余

OHLC、线框 / 俯视曲面、自定义误差线、chartex 写入。脚本 `manual-check/office/cases/docx-charts-rare-remain.ps1`。编号用 Ccx。不要和 Cq、Cx、Cr 混。Word 不能靠 `AddChart` 插 chartex，手测打开库写出的文件。

- **Ccx1**
  - 检查：OHLC
  - 通过标准：✅ `insertChart` `type=stock` 四条系列后 Word 无修复，`type` 为 `stock`，`stockKind` 为 `ohlc`
- **Ccx2**
  - 检查：俯视线框
  - 通过标准：✅ `insertChart` `type=surface` `surfaceKind=contourWire` 三条系列后无修复，`surfaceKind` 为 `contourWire`
- **Ccx3**
  - 检查：自定义误差线
  - 通过标准：✅ 柱形图 `insertErrorBars` `valType=cust` 后无修复，系列上 `plus` / `minus` 对得上
- **Ccx4**
  - 检查：chartex funnel
  - 通过标准：✅ `insertChart` `type=chartex` `layoutId=funnel` 后无修复，`type` 为 `chartex`，`layoutId` 为 `funnel`
- **Ccx5**
  - 检查：仍拒绝
  - 通过标准：✅ `type=chartex` 不带 `layoutId` 得 `InvalidField`
  - `layoutId=regionMap` 得 `NotImplemented`
  - 两条系列的股价仍 `InvalidField`
  - 均未写坏文件

## 图表剩余

股价 / 曲面 / 复合饼的有限度写，以及次轴、趋势线、误差线。手测脚本 `manual-check/office/cases/docx-charts-remain.ps1`。编号用 Cr。xlsx / pptx 各有对应脚本。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Cr1 | 类型 | ✅ 股价 `type` 为 `stock`；给了 `scope.charts` |
| Cr2 | 股价改数 | ✅ `setChartSeries` 后 Word 无修复，数据是新值 |
| Cr3 | 趋势线 | ✅ `insertTrendline` 后树上有 `trendlines` 且 flags 含 `trendline`；Word 无修复；COM 能看见趋势线 |
| Cr4 | 改删趋势线 | ✅ `setTrendline` 改成 `exp` 后无修复；`deleteTrendline` 后名单空 |
| Cr5 | 误差线 | ✅ `insertErrorBars` 后有 `errorBars`；删后空；无修复 |
| Cr6 | 次轴 | ✅ `setChartSeriesAxis` 为 `secondary` 后 `axis` 为 `secondary`，flags 含 `secondaryAxis`；Word 无修复 |

## SmartArt

准备一份 Word 亲手另存的样例：至少一张列表型 SmartArt（可见点不少于 3 个）。不要用库 `create` 产物当唯一样例。
手测脚本为 `manual-check/office/cases/docx-smartart.ps1`。编号用 Sa。

- **Sa1**
  - 检查：计数与名单
  - 通过标准：无 `smartArt` 时根上有 `smartArtCount`、没有名单
  - `smartArt: true` 时 id 为 `{段落}/smartdiagram:{docPrId}`，点文本与 Word 里能看见的字一致
- **Sa2**
  - 检查：改字
  - 通过标准：`setSmartArtText` 后 Word 无修复，目标点的字变了
- **Sa3**
  - 检查：插入
  - 通过标准：`insertSmartArt` 一张 `list` 后 Word 能看见，无修复
- **Sa4**
  - 检查：删除
  - 通过标准：`deleteSmartArt` 后图示与 Fallback 位图都消失，无修复
- **Sa5**
  - 检查：增点
  - 通过标准：`addSmartArtPoint` 后 Word 打开重排，可见条数多 1
- **Sa6**
  - 检查：删点
  - 通过标准：`deleteSmartArtPoint` 后 Word 打开重排，目标点及其子点消失
- **Sa7**
  - 检查：跨家族
  - 通过标准：列表改成循环后可见点文字还在，Word 无修复
- **Sa8**
  - 检查：尺寸
  - 通过标准：`setSmartArtSize` 后宿主尺寸能对上，无修复
- **Sa9**
  - 检查：库外版式
  - 通过标准：当前 uniqueId 不在库内三族时 `setSmartArtLayout` 得 `NotImplemented`，文件未写坏

## 正文插删段落

准备一份 Word 亲手另存的样例：至少三段正文，第二段带加粗。页眉另用带默认页眉的 Word 另存样例。不要用库 `create` 产物当唯一样例。
手测脚本 `manual-check/office/cases/docx-body-para.ps1`。编号用 Ip。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Ip1 | 插在第二段之后 | `insertPara` 后 Word 无修复；新段可见文字是请求里的 `text`；第二段仍加粗；第一段、原第三段未改 |
| Ip2 | 插到文档末 | 对 `docx:document` 发 `insertPara` 后，inspect 最后一段是新段；Word 无修复 |
| Ip3 | 删除非末段 | `deletePara` 删掉第二段后 Word 无修复；第一段标题还在 |
| Ip4 | 最后一段 | 故事里只剩一段时再 `deletePara` 得 `InvalidField`，文件未写坏 |
| Ip5 | 页眉 | 对默认页眉容器或页眉段落 `insertPara` 后，页眉可见多一段；Word 无修复；正文未改 |

## 新建页眉与奇偶首页

无页眉的源可用库 `create`（不要页眉）或 Word 空白文档另存。两节样例用 `Ensure-DocxWordLinkSource`。
手测脚本 `manual-check/office/cases/docx-new-header.ps1`。编号用 Nh。

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

对 Word 亲手另存或 create 后再经 Word 打开过的文件 `inspect`，节上应有边距与纸张字段。再用 `setPageMargin` / `setPageSize` 改第一节，并确认距边界仍可独立改。
Word COM 的 `PageSetup` 边距与纸张单位是磅，等于 twip ÷ 20。手测脚本 `manual-check/office/cases/docx-section.ps1`。

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

准备一份 Word 亲手另存的样例：正文有一张 2×2 简单表，格子里是无域、无超链接的可见文字。`inspect` 带 `scope.tables: true`。
手测脚本 `manual-check/office/cases/docx-table.ps1`。

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

## 表格外观

表级样式、`tblLook`、浮动、默认格边距；单元格底纹、格边距与斜线。格级 `vAlign` 见 Va。不算自动列宽。create 本期不写这些字段。
手测脚本 `manual-check/office/cases/docx-style-table.ps1`。编号 Tf，不要和 Tb、T 撞。现有 Tb1–Tb8 继续过。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Tf1 | 名单字段 | 给了 `scope.tables` 后表上有 `style` / `look` / `pos` / `cellMar`，格上有 `fill` / `cellMar` / `tl2br` / `tr2bl` |
| Tf2 | 浮动 | `setTableFloat` 后树上 `pos.x` 对得上。Word 无修复。COM `Rows.WrapAroundText` 为 true |
| Tf3 | 表边距 | `setTableCellMar` 写 120/160 twip 后树上对得上。Word 无修复。COM 上边距约为 6 磅 |
| Tf4 | 底纹 | `setCellFormat` 写 `fill: FF0000` 后树上为 `FF0000`。Word 无修复。COM 该格 `Shading.BackgroundPatternColor` 的 RGB 为 255 |
| Tf5 | 斜线 | `setCellFormat` 写 `tl2br` 后树上 `val` 为 `single`。Word 无修复。COM `Borders.Item(-7)` 的 `LineStyle` 非 0 |
| Tf6 | 表样式 | `insertStyle` 表格样式再 `setTableStyle` 后树上 `style` 与 `look.bandedRows` 对得上，无修复 |

## 表格宽高与对齐

行内表 `jc` / `tblInd`、首选宽度、网格列宽、行高。不算自动列宽。不要和 pptx 的 `setTablePos` / `setTableSize`、xlsx 的 `setColWidth` / `setRowHeight` 混。
手测脚本 `manual-check/office/cases/docx-table-width.ps1`。编号 Tw，不要和 Tb、Tf、T、Bx 撞。现有 Tb1–Tb8、Tf1–Tf6 继续过。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Tw1 | 名单字段 | 给了 `scope.tables` 后表上有 `align` / `indent` / `width` / `layout` / `colWidths`，行上有 `height` |
| Tw2 | 对齐 | `setTableAlign` 写 `center` 后树上 `align` 为 `center`。Word 无修复。COM `Rows.Alignment` 为 1 |
| Tw3 | 宽度 | `setTableWidth` 写 `dxa` 与两列之和后树上 `width.w` 与 `colWidths` 对得上。Word 无修复。COM `PreferredWidth` 约为对应磅值 |
| Tw4 | 行高 | `setTableRowHeight` 写 `h=400` 后树上 `height.h` 为 400。Word 无修复。COM 该行高约为 20 磅 |
| Tw5 | create | create 带 `align` / `width` / `cols` / `rowHeights` 后树上对得上，无修复 |
| Tw6 | 拒写 | 对段落 id 发 `setTableWidth` 得 `InvalidId`。xlsx 上发此 op 得 `InvalidField` |

## 表级边框

表级 `w:tblBorders` 的四边与内外框。不要和格斜线、页面边框、段边框混。不算自动套表样式。
手测脚本 `manual-check/office/cases/docx-table-borders.ps1`。编号 Tbdr，不要和 Tb、Tf、Tw、Pb 撞。现有 Tf1–Tf6、Tw1–Tw6 继续过。

- **Tbdr1**
  - 检查：名单字段
  - 通过标准：给了 `scope.tables` 后表上有 `borders`。create 默认表的 `borders.top.val` 为 `single`
- **Tbdr2**
  - 检查：改外边
  - 通过标准：`setTableBorders` 写 `top.val` 为 `double` 后树上对得上。Word 无修复。COM 上边 `LineStyle` 非 0
- **Tbdr3**
  - 检查：只改一边
  - 通过标准：只点名 `insideH` 后 `top.val` 仍为 `single`，`insideH.val` 为 `dashed`。Word 无修复
- **Tbdr4**
  - 检查：去掉
  - 通过标准：`borders` 为 `null` 后树上 `borders` 为 `null`，ZIP 里没有 `tblBorders`。Word 无修复
- **Tbdr5**
  - 检查：create
  - 通过标准：create 带 `tables[].borders.top` 后树上对得上，无修复
- **Tbdr6**
  - 检查：拒写
  - 通过标准：缺 `borders` 报 `InvalidField`。xlsx 上发此 op 报 `InvalidField`。不强制开 Word

## 单元格垂直对齐

格级 `tcPr/w:vAlign`。不要和节上 `setVerticalAlign`、框内 `setTextBoxTextAnchor` 混。不算排版引擎。
手测脚本 `manual-check/office/cases/docx-cell-valign.ps1`。编号 Va，不要和 Tf、Tw、T、Bx 撞。现有 Tf1–Tf6、Tw1–Tw6 继续过。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Va1 | 名单字段 | 给了 `scope.tables` 后格上有 `vAlign`，缺元素为 `null` |
| Va2 | apply | `setCellFormat` 写 `vAlign: center` 后树上为 `center`。Word 无修复。COM 该格 `VerticalAlignment` 为 1 |
| Va3 | create | create `cellFormats` 带 `vAlign: bottom` 后树上为 `bottom`。Word 无修复 |
| Va4 | 去掉 | `vAlign: null` 后树上为 `null`，XML 无 `w:vAlign` |
| Va5 | 拒写 | 对段落 id 发 `setCellFormat` 得 `InvalidId`。xlsx 上发此 op 得 `InvalidField` |

## 格正交边框

格级 `w:tcBorders` 的四边与格内内外框。不要和表级 `setTableBorders`、`setCellFormat` 的 `tcMar`、斜线混。不算自动套表样式。
手测脚本 `manual-check/office/cases/docx-cell-borders.ps1`。编号 Tcb，不要和 Tc、Tb、Tf、Tbdr、Va 撞。

| 序号 | 检查 | 通过标准 | 状态 |
| --- | --- | --- | --- |
| Tcb1 | 名单字段 | 给了 `scope.tables` 后格上有 `borders`。create 默认表的格 `borders` 为 `null` | ✅ |
| Tcb2 | 改外边 | `setCellBorders` 写 `top.val` 为 `double` 后树上对得上。Word 无修复。COM 该格上边 `LineStyle` 非 0 | ✅ |
| Tcb3 | 只改一边 | 只点名 `insideH` 后 `top.val` 仍为 `double`，`insideH.val` 为 `dashed`。Word 无修复 | ✅ |
| Tcb4 | 去掉 | `borders` 为 `null` 后树上格 `borders` 为 `null`，斜线 `tl2br` 还在。Word 无修复 | ✅ |
| Tcb5 | create | create `cellFormats` 带 `borders.top` 后树上对得上，无修复 | ✅ |
| Tcb6 | 拒写 | 缺 `borders` 报 `InvalidField`。对表 id 发此 op 报 `InvalidId`。xlsx 上发此 op 报 `InvalidField`。不强制开 Word | ✅ |

## 格属性剩余

格级 `tcPr` 上 `noWrap`、`textDirection`、`tcFitText`、`hideMark`、`headers`。不要和 59 的 `vAlign`、64 的边框混。不算排版引擎。
手测脚本 `manual-check/office/cases/docx-cell-props.ps1`。编号 Tcp，不要和 Va、Tcb、Tf 撞。COM 竖排读 `Range.Orientation`，不是 `Cell.Orientation`。

| 序号 | 检查 | 通过标准 | 状态 |
| --- | --- | --- | --- |
| Tcp1 | 名单字段 | 给了 `scope.tables` 后格上有 `noWrap`、`textDirection`、`fitText`、`hideMark`、`headers`。create 默认表缺元素为 `null` / `[]` | ✅ |
| Tcp2 | 不换行与方向 | `setCellFormat` 写 `noWrap: true` 与 `textDirection: tbRl` 后树上对得上。Word 无修复。COM 该格 `WordWrap` 为 False，`Orientation` 非 0 | ✅ |
| Tcp3 | 适应文字与隐藏标记 | 写 `fitText: true` 与 `hideMark: true` 后树上为 true。Word 无修复。COM `FitText` 为 True。XML 有 `w:hideMark` | ✅ |
| Tcp4 | 表头引用 | 写 `headers: ["A1"]` 后树上为该数组。Word 无修复 | ✅ |
| Tcp5 | 去掉 | 对应字段为 `null`、`headers` 为 `[]` 后树上为 `null` / `[]`，XML 无这些元素。Word 无修复 | ✅ |
| Tcp6 | create 与拒写 | create `cellFormats` 带 `noWrap` 后树上为 true。xlsx 上发 `setCellFormat` 报 `InvalidField`。不强制开 Word | ✅ |

## 框内表

文本框 `w:txbxContent` 里的 `w:tbl`。不算自动列宽。注内 / 批注内的表不测。create 本期不写，见待做 56。
手测脚本 `manual-check/office/cases/docx-box-table.ps1`。编号 Bx，不要和 Bb、Tf、T、Tb 撞。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Bx1 | 名单 | 框上始终有 `tableCount`。`insertTable` 点框 id 后，`scope.tables` 下列出 `{框 id}/table:1`，根上 `tableCount` 不加 |
| Bx2 | 结构与外观 | `insertRow` 后 `rowCount` 加 1。`setCellFormat` 写黄底后格上 `fill` 对得上 |
| Bx3 | 嵌套 | 对框内单元格 `insertTable` 后该格里出现嵌套表，id 为 `{框 id}/table:2` |
| Bx4 | 打开 | Word 16.0 打开无 Repair。COM 该形状 `TextFrame.TextRange.Tables.Count` 至少为 1 |
| Bx5 | 拒写 | 对脚注段落发 `insertTable` 得 `InvalidId`，未写文件 |

## 框内域、书签与公式控件

文本框 `w:txbxContent` 里的 `w:hyperlink`、书签、域、公式、拼音与内容控件。不求值域，不算公式排版。注内 / 批注内不测。create 本期不写，见待做 56。
手测脚本 `manual-check/office/cases/docx-box-fields.ps1`。编号 Bf，不要和 Bb、Bx、Bi、Fd 撞。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Bf1 | 链 | 对框内段落 `insertHyperlink` 后框上 `hyperlinkCount` 为 1，`scope.hyperlinks` 下列出 `{框 id}/link:1`，根上 `hyperlinkCount` 不加。Word 无 Repair。COM 该形状 `TextFrame.TextRange.Hyperlinks.Count` 至少为 1 |
| Bf2 | 书签与域 | `insertBookmark` 与 `insertField` 后框上对应计数为 1，id 带框前缀。Word 无 Repair |
| Bf3 | 公式拼音控件 | `insertMath`、`insertRuby`、`insertContentControl` 后框上对应计数各为 1。Word 无 Repair |
| Bf4 | 拒改字 | 对含链的框内段落发 `setParaText` 得 `NotImplemented` |
| Bf5 | 拒写 | 对脚注段落发 `insertField` 得 `InvalidId`，未写文件 |

## 修订痕迹

准备一份带修订的样例：Word 打开跟踪修订后，一段里删几个字、另一段里插入几个字，另存。`inspect` 带 `scope.revisions: true`。
手测脚本 `manual-check/office/cases/docx-revision.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Rv1 | 名单 | 无 scope 时仍有 `revisionCount` 且没有 `revisions`；给了 `scope.revisions` 后能列出 `ins` 与 `del`，带 `host` 段落 id |
| Rv2 | 接受删除 | `acceptRevision` 点名删除后，Word 无修复，被删的字从可见文本里消失 |
| Rv3 | 拒绝插入 | `rejectRevision` 点名插入后，Word 无修复，插入字消失，原文还在 |
| Rv4 | 接受插入 | `acceptRevision` 点名插入后，Word 无修复，插入字留下且不再标修订 |
| Rv5 | 拒绝删除 | `rejectRevision` 点名删除后，Word 无修复，被删的字重新可见 |
| Rv6 | 改字拒绝 | 对仍带本期修订的段发 `setParaText` 得 `NotImplemented`，文件未写坏 |

## 修订剩余类型

Word 跟踪修订下改表单元格底纹、插入空段、删除表行。`inspect` 带 `scope.revisions: true`。不调 COM 的 Accept/Reject。
手测脚本 `manual-check/office/cases/docx-revision-remain.ps1`。编号用 Rm。不要和 Rv 撞。不做 move。

- **Rm1**
  - 检查：表属性名单
  - 通过标准：改第一格底纹后，无 scope 时仍有 `revisionCount` 且没有 `revisions`
  - 给了 `scope.revisions` 后能列出 `tcPrChange` 或 `tblPrChange`，`host` 为 `null`
- **Rm2**
  - 检查：接受表属性
  - 通过标准：接受全部表 / 格 `*Change` 与 `rPrChange` 后 Word 无修复，该格仍有底纹
- **Rm3**
  - 检查：拒绝表属性
  - 通过标准：拒绝全部表 / 格 `*Change` 与 `rPrChange` 后 Word 无修复，该格没有底纹
- **Rm4**
  - 检查：段落标记名单
  - 通过标准：插入空段后能列出 `paraMarkIns`，`host` 为该段 id，`text` 为 `null`
- **Rm5**
  - 检查：接受段落标记
  - 通过标准：接受 `paraMarkIns` 后 Word 无修复，空段还在且不再标修订
- **Rm6**
  - 检查：拒绝段落标记
  - 通过标准：拒绝 `paraMarkIns` 后 Word 无修复，该空段消失，其余段还在
- **Rm7**
  - 检查：表行名单
  - 通过标准：删第二行后能列出 `rowDel`，`host` 为 `null`
- **Rm8**
  - 检查：接受表行
  - 通过标准：接受 `rowDel` 后 Word 无修复，COM 表行数为 1
- **Rm9**
  - 检查：拒绝表行
  - 通过标准：拒绝 `rowDel` 后 Word 无修复，COM 表行数为 2，第二行可见 A2

## 超链接与书签

准备一份带超链接与书签的样例：Word 给第一段加外部 URL，给第二段加用户书签，另存。`inspect` 带 `scope.hyperlinks: true` 与 `scope.bookmarks: true`。
手测脚本 `manual-check/office/cases/docx-hyperlink.ps1`。

- **Hb1**
  - 检查：名单
  - 通过标准：无 scope 时仍有 `hyperlinkCount` / `bookmarkCount` 且没有名单
  - 给了对应 scope 后能列出 `link:` 与 `bookmark:`，链带 `url`，书签带 `name`
- **Hb2**
  - 检查：改 URL
  - 通过标准：`setHyperlink` 改外部地址后 Word 无修复，COM `Hyperlinks(1).Address` 是新地址
- **Hb3**
  - 检查：插链
  - 通过标准：对无链段 `insertHyperlink` 后 Word 无修复，能看见新链
- **Hb4**
  - 检查：删链
  - 通过标准：`deleteHyperlink` 后字还在、链消失，无修复
- **Hb5**
  - 检查：插书签
  - 通过标准：`insertBookmark` 后 COM 能按名找到，无修复
- **Hb6**
  - 检查：删书签
  - 通过标准：`deleteBookmark` 后该书签消失，无修复
- **Hb7**
  - 检查：改字拒绝
  - 通过标准：对仍带本期超链接的段发 `setParaText` 得 `NotImplemented`，文件未写坏

## 图与形状超链接

正文与页眉页脚里图、文本框上的 `a:hlinkClick`。不进 `hyperlinks` 名单，不计入 `hyperlinkCount`。用库 create 产物再 apply。脚本 `manual-check/office/cases/docx-drawing-hlink.ps1`。编号用 **Dh**。不要和 Hb、B、T 撞。

- **Dh1**
  - 检查：图上链
  - 通过标准：`insertPicture` 后再 `insertHyperlink` 点图 id，inspect `hyperlink.url` 对得上
  - Word 无修复
  - COM 该形状 `Hyperlink.Address` 对得上
  - `hyperlinkCount` 仍为 0
- **Dh2**
  - 检查：改 URL
  - 通过标准：`setHyperlink` 改 `url` 后 COM 地址更新，无修复
- **Dh3**
  - 检查：删链
  - 通过标准：`deleteHyperlink` 后 `hyperlink` 为 `null`、图还在、无修复
- **Dh4**
  - 检查：文本框链
  - 通过标准：`insertTextBox` 后再 `insertHyperlink` 点框 id，inspect `hyperlink.url` 对得上，无修复
- **Dh5**
  - 检查：内部书签
  - 通过标准：图上 `insertHyperlink` 给 `anchor` 后 inspect `kind` 为 `anchor`，关系 Target 以 `#` 开头且无 `TargetMode`，无修复
- **Dh6**
  - 检查：拒写
  - 通过标准：对图发带 `text` 的 `insertHyperlink` 得 `InvalidField`，未写文件
  - 不强制开 Word

## DrawingML 链剩余

框内图、注内 / 批注内图或框、连接线、无框形状、SmartArt、图表、墨迹画布、3D 上的 `a:hlinkClick`。不进 `hyperlinks` 名单，不计入 `hyperlinkCount`。OLE 仍不写。脚本 `manual-check/office/cases/docx-drawing-hlink-remain.ps1`。编号用 **Dhr**。不要和 Dh、Dl 撞。现有 Dh1–Dh6 继续过。

- **Dhr1**
  - 检查：框内图
  - 通过标准：框内图 `insertHyperlink` 后 inspect `hyperlink.url` 对得上。Word 无修复。COM 该图 `Hyperlink.Address` 对得上。`hyperlinkCount` 仍为 0
- **Dhr2**
  - 检查：无框形状
  - 通过标准：`insertShape` 后再 `insertHyperlink` 点形状 id，inspect `hyperlink.url` 对得上，无修复
- **Dhr3**
  - 检查：连接线
  - 通过标准：`insertConnector` 后再 `insertHyperlink` 点连接线 id，inspect `hyperlink.url` 对得上，无修复
- **Dhr4**
  - 检查：create
  - 通过标准：create `shapes[].hyperlink` 后树上对得上，无修复
- **Dhr5**
  - 检查：删链
  - 通过标准：对形状 `deleteHyperlink` 后 `hyperlink` 为 `null`、形状还在、无修复
- **Dhr6**
  - 检查：拒写
  - 通过标准：对 OLE id 发 `insertHyperlink` 得 `InvalidId`，未写文件
  - 不强制开 Word

## 域

准备一份带 PAGE 或 DATE 域的样例：可用 `insertField` 写入，也可用 Word 另存。`inspect` 带 `scope.fields: true`。
手测脚本 `manual-check/office/cases/docx-field.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Fd1 | 名单 | 无 scope 时仍有 `fieldCount` 且没有 `fields`；给了 `scope.fields` 后能列出 `field:`，带 `instr` 与 `code` |
| Fd2 | 改指令 | `setField` 改 PAGE 为 DATE 后 Word 无修复 |
| Fd3 | 插域 | 对无域段 `insertField` 后 Word 无修复，能看见域 |
| Fd4 | 删域留结果 | `deleteField`（`keep` 为 `result` 或缺省）后缓存字还在、域消失，无修复 |
| Fd5 | 删域留指令 | `deleteField` 且 `keep` 为 `instr` 后指令字还在、域消失，无修复 |
| Fd6 | 页眉 PAGE | 默认页眉里的 PAGE 能列出，id 带 `header:` 前缀 |
| Fd7 | 改字拒绝 | 对仍带本期域的段发 `setParaText` 得 `NotImplemented`，文件未写坏 |

## 域类型

`insertField` 可给 `code` 写常用域与窗体 `ffData`。不在库里求值。手测脚本 `manual-check/office/cases/docx-field-types.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Ft1 | PAGE 预设 | `code` 为 `PAGE` 后树上 `code` 为 `PAGE`，无 `form`；Word 无修复且 `Fields.Count≥1` |
| Ft2 | MERGEFIELD | `code` 为 `MERGEFIELD` 且 `arg` 为 `Name` 后 `instr` 含 `Name`；Word 无修复 |
| Ft3 | 窗体文本 | `FORMTEXT` 后 `form.type` 为 `text`，`name` 为 `Text1`；Word `FormFields.Count≥1`，无修复 |
| Ft4 | 复选 | `FORMCHECKBOX` 且 `checked: true` 后 `form.type` 为 `checkBox`，`checked` 为 true；Word 无修复 |
| Ft5 | 下拉 | `FORMDROPDOWN` 且 `items` 为 `A`/`B` 后 `form.items` 含这两项；Word 无修复 |
| Ft6 | create | create 段上 `code: NUMPAGES` 后树上 `code` 为 `NUMPAGES`；Word 无修复 |
| Ft7 | 拒错 | 同时给 `instr` 与 `code`、或 `REF` 不给 `arg`、或 `INCLUDETEXT`，得 `InvalidField`，不写文件 |

## 编号列表

准备一份无编号的 create 产物，再用 `insertNumbering` / `setParaNumbering` 写入。`inspect` 带 `scope.numbering: true`。
手测脚本 `manual-check/office/cases/docx-numbering.ps1`。脚注编号与节行号不在本组。

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

## 样式增删与字符样式

增删自定义样式、改字符样式定义、给正文 run 套 `rStyle`。不扫描引用。create 本期不写 `styles[]`。
手测脚本 `manual-check/office/cases/docx-style-table.ps1`。编号 Sy，不要和 St 撞。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Sy1 | 插字符样式 | `insertStyle` 后 `scope.styles` 能列出 `style:ooxpChar`，`type` 为 `character` |
| Sy2 | 改定义 | `setStyle` 写 `color` / `bold` 后，套到 run 上 Word 该 run 为红且加粗，无修复 |
| Sy3 | 套 run | `setRunStyle` 后树上该 run 的 `style` 为 `ooxpChar` |
| Sy4 | 删除 | `deleteStyle` 后名单不再有 `ooxpChar`，Word 无修复 |
| Sy5 | 拒删内置 | 对 `style:Normal` 发 `deleteStyle` 报 `InvalidField` |
| Sy6 | 拒错 | 对 xlsx 发 `insertStyle` 报 `InvalidField`。`type` 为 `numbering` 报 `InvalidField`。不强制开 Word |

## 段落与 run 直接格式

`inspect` 段上始终有 `align` / `indent` / `spacing` / `runCount`。`scope.runs: true` 时列出 `runs`。
手测脚本 `manual-check/office/cases/docx-format.ps1`。不改 `styles.xml`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pf1 | 名单 | 无 `scope.runs` 时已切出的段有 `runCount` 且没有 `runs`；给了 `scope.runs` 后能列出 `{段落 id}/run:{n}` |
| Pf2 | 段对齐 | `setParaFormat` 把一段 `align` 改成 `center` 后树上为 `center` |
| Pf3 | 打开对齐 | Word 打开后该段居中，无修复 |
| Pf4 | run 加粗 | `setRunFormat` 把第一段第一个 run 设为加粗后 Word 该 run 加粗，无修复 |
| Pf5 | 未点名 | 同一段未点名的 run 颜色还在 |
| Pf6 | 间距 | `setParaFormat` 写 `before` / `after` 后 Word 段前段后距对得上（允许 0.05 磅误差），无修复 |

## 段落边框底纹与 run 剩余

段上 `pBdr` / `shd` / `tabs`，以及 run 上 `highlight` / `strike` / `dstrike` / `vertAlign`。只写该节点直接格式，不摊 `styles.xml`。
手测脚本 `manual-check/office/cases/docx-para-run-remain.ps1`。编号 Bd / Rh，不要和 Pb、Pf、Ea、Th 撞。create 本期不写，见待做 56。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Bd1 | 段边框底纹制表位 | `setParaFormat` 写上边框、黄底与左对齐制表位后，树上 `borders.top.val`、`shd.fill`、`tabs[0]` 对得上 |
| Bd2 | 打开 | Word 16.0 无 Repair。COM 该段底纹 RGB 对得上，上边框 `LineStyle` 非 0 |
| Bd3 | 删除 | 把 `borders` / `shd` / `tabs` 写成 `null` 后树上三项都是 `null`，无 Repair |
| Rh1 | 突出显示与删除线 | `setRunFormat` 写 `highlight: yellow` 与 `strike: true` 后树上对得上。Word 无 Repair。COM `HighlightColorIndex` 非 0，`StrikeThrough` 为真 |
| Rh2 | 上下标与双删除线 | `setRunFormat` 写 `vertAlign: superscript` 与 `dstrike: true` 后树上对得上，无 Repair |
| Rh3 | 拒错 | 对 xlsx 发带 `highlight` 的 `setRunFormat` 报 `InvalidField`。不认识的 `highlight` 报 `InvalidField`。不强制开 Word |

## 字符间距

run 上 `w:spacing`（字符间距）与 `w:kern`（字距调整）。只写该 run 直接格式，不摊 `styles.xml`，不写 `w:w`。不算排版引擎。
手测脚本 `manual-check/office/cases/docx-run-spacing.ps1`。编号 Rcs，不要和 Bd、Rh、Ea 撞。

| 序号 | 检查 | 通过标准 | 状态 |
| --- | --- | --- | --- |
| Rcs1 | 改间距 | `setRunFormat` 写 `spacing: 40` 后树上为 `40`。Word 无 Repair。COM `Font.Spacing` 约为 2 磅（允许 0.05 磅误差） | ✅ |
| Rcs2 | 改 kern | 写 `kern: 28` 后树上为 `28`。无 Repair。COM `Font.Kerning` 约为 14 磅（允许 0.05 磅误差） | ✅ |
| Rcs3 | 负间距 | 写 `spacing: -20` 后树上为 `-20`。无 Repair。COM `Font.Spacing` 约为 -1 磅（允许 0.05 磅误差） | ✅ |
| Rcs4 | 去掉 | 把 `spacing` / `kern` 写成 `null` 后树上两项都是 `null`，无 Repair | ✅ |
| Rcs5 | 框内 | 框内段落 run 写 `spacing` 后框内树上对得上，无 Repair | ✅ |
| Rcs6 | 拒错 | `spacing` 给字符串、`kern` 给负数报 `InvalidField`。对 xlsx 发带 `spacing` 的 `setRunFormat` 报 `InvalidField`。未写文件。不强制开 Word | ✅ |

## 字符缩放

run 上 `w:w`（字符缩放百分比）。只写该 run 直接格式，不摊 `styles.xml`，不写 `spacing` / `kern`。不算排版引擎。
手测脚本 `manual-check/office/cases/docx-run-scale.ps1`。编号 Rsc，不要和 Rcs、Rh、Ea 撞。

| 序号 | 检查 | 通过标准 | 状态 |
| --- | --- | --- | --- |
| Rsc1 | 改缩放 | `setRunFormat` 写 `scale: 200` 后树上为 `200`。Word 无 Repair。COM `Font.Scaling` 为 200 | ✅ |
| Rsc2 | 压缩 | 写 `scale: 50` 后树上为 `50`。无 Repair。COM `Font.Scaling` 为 50 | ✅ |
| Rsc3 | 去掉 | 把 `scale` 写成 `null` 后树上为 `null`，XML 无 `w:w`。无 Repair | ✅ |
| Rsc4 | 框内 | 框内段落 run 写 `scale` 后框内树上对得上，无 Repair | ✅ |
| Rsc5 | create | create `runs[].scale` 为 200 后树上对得上，无 Repair | ✅ |
| Rsc6 | 拒错 | `scale` 为 `0` 或 `601` 报 `InvalidField`。对 xlsx 发带 `scale` 的 `setRunFormat` 报 `InvalidField`。未写文件。不强制开 Word | ✅ |

## 东亚排版

段上禁则、run 强调与压缩、拼音、整篇压缩与自定义禁则字符。不算排版引擎。
手测脚本 `manual-check/office/cases/docx-east-asia.ps1`。create 本期不写。框内 / 注内 / 批注内拼音不测。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Ea1 | 段禁则 | `setParaFormat` 写 `kinsoku` / `overflowPunct` / `autoSpaceDE` 后树上 `eastAsia` 对得上 |
| Ea2 | 打开禁则 | Word 打开无修复。COM `FarEastLineBreakControl` / `HangingPunctuation` / `AddSpaceBetweenFarEastAndAlpha` 对得上 |
| Ea3 | 强调点 | `setRunFormat` 写 `em: dot` 后树上 `em` 为 `dot`。Word 无修复。COM `EmphasisMark` 非 0 |
| Ea4 | 两行合一 | `setRunFormat` 写 `eastAsianLayout.combine` 后树上 `combine` 为 `true`，无修复 |
| Ea5 | 插拼音 | `insertRuby` 后 `rubyCount` 为 1，`scope.rubies` 能列出 `base` / `rt`。Word 无修复 |
| Ea6 | 改拼音 | `setRuby` 改 `rt` 后树上对得上，无修复 |
| Ea7 | 整篇 | `setEastAsianSettings` 写 `characterSpacingControl` 后根上对得上。Word 无修复 |
| Ea8 | 拒错 | 对 xlsx 发 `insertRuby` 报 `InvalidField`。空 `base` 报 `InvalidField`。不强制开 Word |

## 插删节

`insertSection` 在某段之后写入分节符；`deleteSection` 删掉非末节。create 可用 `paragraphs[].sectionType` 写多节。
手测脚本 `manual-check/office/cases/docx-insert-section.ps1`。新建页眉的具体 XML 不在本组，断开链接仍走 `setHeaderFooterLink`。

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

- **Tc1**
  - 检查：名单
  - 通过标准：无 `scope.tocs` 时有 `tocCount` 且没有 `tocs`
  - `insertToc` 之后 `tocCount` 为 1，给了 `scope.tocs` 能列出 `toc:1`，`kind` 为 `toc`，且 `fields` 里有对应 `TOC` 域
- **Tc2**
  - 检查：打开
  - 通过标准：Word 打开后无修复
- **Tc3**
  - 检查：更新
  - 通过标准：Word 更新域或 `TablesOfContents.Update` 之后，目录可见文字含标题「第一章」
- **Tc4**
  - 检查：删除
  - 通过标准：`deleteToc` 之后 `tocCount` 为 0，标题段还在，Word 无修复
- **Tc5**
  - 检查：图表目录
  - 通过标准：`kind` 为 `tof` 的插入产物树上 `kind` 为 `tof`，Word 无修复

## 题注与交叉引用

`insertCaption` 写入 SEQ 与 `_Ref` 书签；`insertCrossRef` 写入 REF / PAGEREF。不在库里求值。
手测脚本 `manual-check/office/cases/docx-caption-xref.ps1`。

- **Cp1**
  - 检查：名单
  - 通过标准：无 `scope.captions` / `scope.crossRefs` 时有 `captionCount` / `crossRefCount` 且没有名单
  - `insertCaption` 之后 `captionCount` 为 1，给了 `scope.captions` 能列出 `caption:1`，`identifier` 为 `Figure`，且 `fields` 里有对应 `SEQ` 域
  - `bookmarkCount` 仍为 0（`_Ref` 隐藏书签）
- **Cp2**
  - 检查：打开
  - 通过标准：Word 打开后无修复，更新 `SEQ` 后结果是 `1`
- **Cp3**
  - 检查：交叉引用
  - 通过标准：用题注上的 `bookmark` 发 `insertCrossRef` 后 `crossRefCount` 为 1，`kind` 为 `ref`
  - Word 更新域后可见 `Figure`
- **Cp4**
  - 检查：删除交叉引用
  - 通过标准：`deleteCrossRef` 之后 `crossRefCount` 为 0，题注段还在，Word 无修复
- **Cp5**
  - 检查：删除题注
  - 通过标准：`deleteCaption` 之后 `captionCount` 为 0，标题段还在，Word 无修复
- **Cp6**
  - 检查：拒错
  - 通过标准：对页眉段落发 `insertCaption` 得 `InvalidId`，未写文件
  - 正文文本框里的题注，Word 更新 `SEQ` 后结果是 `1`，没有 Repair
  - xlsx 上发 `insertCaption` 得 `InvalidField`，未写文件

## 索引

`insertIndex` 写入 INDEX 域；`insertIndexEntry` 写入 XE。不在库里求值。手测脚本 `manual-check/office/cases/docx-index.ps1`。

- **Ix1**
  - 检查：名单
  - 通过标准：无 `scope.indexes` / `scope.indexEntries` 时有 `indexCount` / `indexEntryCount` 且没有名单
  - `insertIndexEntry` 与 `insertIndex` 之后两项计数均为 1，给了对应 scope 能列出 `xe:1` / `index:1`，且 `fields` 里有 `XE` 与 `INDEX`
- **Ix2**
  - 检查：打开
  - 通过标准：Word 打开后无修复
- **Ix3**
  - 检查：更新
  - 通过标准：Word 更新域后可见条目 `fox`
- **Ix4**
  - 检查：删除索引
  - 通过标准：`deleteIndex` 之后 `indexCount` 为 0，标题段与索引项还在，Word 无修复
- **Ix5**
  - 检查：删除索引项
  - 通过标准：`deleteIndexEntry` 之后 `indexEntryCount` 为 0，标题段还在，Word 无修复
- **Ix6**
  - 检查：拒错
  - 通过标准：对页眉段落发 `insertIndex` 得 `InvalidId`，未写文件
  - xlsx 上发 `insertIndex` 得 `InvalidField`，未写文件

## 参考文献

`insertSource` 写入 bibliography customXml；`insertCitation` 写入 CITATION 域（包在 `w:citation` 的 sdt 里）；`insertBibliography` 写入 BIBLIOGRAPHY 域。
不在库里按样式表排版。手测脚本 `manual-check/office/cases/docx-biblio.ps1`。

- **Bi1**
  - 检查：名单
  - 通过标准：无 `scope.sources` / `scope.citations` / `scope.bibliographies` 时有 `sourceCount` / `citationCount` / `bibliographyCount` 且没有名单
  - 插入源、引文与书目之后三项计数均为 1，给了对应 scope 能列出 `source:1` / `cite:1` / `biblio:1`，且 `fields` 里有 `CITATION` 与 `BIBLIOGRAPHY`
  - `contentControlCount` 仍为 0
- **Bi2**
  - 检查：打开
  - 通过标准：Word 打开后无修复
- **Bi3**
  - 检查：更新
  - 通过标准：Word 更新域后引文可见 `Ecma` 与 `2006`，书目块非空
- **Bi4**
  - 检查：删除书目
  - 通过标准：`deleteBibliography` 之后 `bibliographyCount` 为 0，源与引文还在，Word 无修复
- **Bi5**
  - 检查：删除引文与源
  - 通过标准：`deleteCitation` 之后 `citationCount` 为 0
  - `deleteSource` 之后 `sourceCount` 为 0
  - 标题段还在，Word 无修复
- **Bi6**
  - 检查：拒错
  - 通过标准：对已有正文书目的文档，页眉再插一块，id 带 `header:` 前缀，根上 `bibliographyCount` 仍是正文那一块
  - Word 更新该域后结果含 `Ecma` 与 `2006`，没有 Repair
  - xlsx 上发 `insertBibliography` 得 `InvalidField`，未写文件

## 构建基块

`insertDocPart` 写入 glossary `w:docPart`；`insertDocPartContent` 把 `docPartBody` 复制进主文档；`deleteDocPart` 删条目。不实现构建基块库 UI。
手测脚本 `manual-check/office/cases/docx-glossary.ps1`。

- **Bb1**
  - 项：名单
  - 期望：无 `scope.docParts` 时有 `docPartCount` 且没有名单
  - 插入之后计数为 1，给了 scope 能列出 `docPart:1`，`name` 为 `Clause1`，`gallery` 为 `custQuickParts`
- **Bb2**
  - 项：打开
  - 期望：Word 打开后无修复
- **Bb3**
  - 项：插入正文
  - 期望：`insertDocPartContent` 之后正文可见 `glossary clause`，Word 无修复
- **Bb4**
  - 项：删除
  - 期望：`deleteDocPart` 之后 `docPartCount` 为 0，Word 无修复
- **Bb5**
  - 项：空包
  - 期望：删光后包内没有 `/word/glossary/document.xml`
- **Bb6**
  - 项：拒错
  - 期望：对页眉段落发 `insertDocPartContent` 得 `InvalidId`
  - xlsx 上发 `insertDocPart` 得 `InvalidField`
  - 未写文件


## 字体嵌入

`insertEmbeddedFont` 把本地 TTF/OTF 按 ECMA-376 §17.8.1 混淆后写入 `fontTable`；`deleteEmbeddedFont` 删嵌入面与 odttf；`setFontEmbed` 写 settings 三个标志。
不实现子集化引擎，不改本机字体库。手测脚本 `manual-check/office/cases/docx-font-embed.ps1`。
`file` 用本机 `C:\Windows\Fonts\calibri.ttf`，不要把 ttf 提交进仓库。

- **Fe1**
  - 项：名单
  - 期望：无 `scope.embeddedFonts` 时有 `embeddedFontCount` 与 `fontEmbed` 且没有名单
  - 插入之后计数为 1，给了 scope 能列出 `font:1`，`name` 为 `Calibri`，`style` 为 `regular`，`subsetted` 为 `false`，`fontEmbed.trueType` 为 `true`
- **Fe2**
  - 项：打开
  - 期望：Word 打开插入产物后无修复
- **Fe3**
  - 项：导出
  - 期望：对该 `font:1` 发 `exportMedia`，字节与 ZIP 内 `word/fonts/font1.odttf` 一致
- **Fe4**
  - 项：删除
  - 期望：`deleteEmbeddedFont` 之后 `embeddedFontCount` 为 0，包内没有 `word/fonts/font1.odttf`，Word 无修复
- **Fe5**
  - 项：标志
  - 期望：`setFontEmbed` 把 `subset` 写成 `true` 之后树上 `fontEmbed.subset` 为 `true`，Word 无修复
- **Fe6**
  - 项：拒错
  - 期望：xlsx 上发 `insertEmbeddedFont` 得 `InvalidField`
  - 未写文件


## 页面背景与边框

`setPageBackground` 写整篇 `w:background` 与 `displayBackgroundShape`，并可写 DrawingML 渐变或 VML 图片填充；`setPageBorders` 写节上 `w:pgBorders`，`val` 含艺术型。
不实现排版引擎。手测脚本 `manual-check/office/cases/docx-page-bg.ps1`。艺术型与 DrawingML 背景见 Fm 组。

| 编号 | 项 | 期望 |
| --- | --- | --- |
| Pb1 | 名单 | 无背景时根上 `pageBackground.color` 为 `null`、`display` 为 `false`；节上 `pageBorders` 为 `null` |
| Pb2 | 背景 | `setPageBackground` 把 `color` 写成 `FF0000` 之后树上为 `FF0000` 且 `display` 为 `true`，Word 无修复 |
| Pb3 | 清除背景 | `color` 为 `null` 之后树上 `color` 又为 `null`、`display` 为 `false`，Word 无修复 |
| Pb4 | 边框 | `setPageBorders` 四边 `single` 之后节上 `pageBorders.top.val` 为 `single`，Word 无修复 |
| Pb5 | 清除边框 | `enabled: false` 之后节上 `pageBorders` 为 `null`，Word 无修复 |
| Pb6 | 拒错 | xlsx 上发 `setPageBackground` 得 `InvalidField`。未写文件 |


## 公式

`insertMath` / `setMath` / `deleteMath` 读写顶层 OMML。不在库里排版。手测脚本 `manual-check/office/cases/docx-math.ps1`。

- **Mm1**
  - 检查：名单
  - 通过标准：无 `scope.maths` 时有 `mathCount` 且没有 `maths`
  - `insertMath` 之后 `mathCount` 为 1，给了 `scope.maths` 能列出 `math:1`，`kind` 为 `inline`，`text` 为写入的原文
- **Mm2**
  - 检查：打开
  - 通过标准：Word 打开后无修复，能看见公式区
- **Mm3**
  - 检查：改字
  - 通过标准：`setMath` 之后树上 `text` 变成新值，Word 仍把它当公式，不是普通 run
- **Mm4**
  - 检查：独立公式
  - 通过标准：`kind` 为 `display` 的插入产物树上 `kind` 为 `display`，Word 无修复
- **Mm5**
  - 检查：删除
  - 通过标准：`deleteMath` 之后 `mathCount` 为 0，标题段还在，Word 无修复
- **Mm6**
  - 检查：页眉
  - 通过标准：默认页眉插入一条 inline 公式后，容器上 `mathCount` 为 1，Word 无修复

## 公式结构

`insertMath` / `setMath` / create `maths[]` 可按 `expr` 写结构化 OMML。不解析 UnicodeMath / TeX，不在库里排版。
手测脚本 `manual-check/office/cases/docx-math-struct.ps1`。

- **Ms1**
  - 检查：分数
  - 通过标准：`insertMath` 给 `expr` 分数后，给了 `scope.maths` 时 `expr[0].type` 为 `frac`，`text` 为 `ab`，`kind` 为 `display`
  - Word 无修复且 `OMaths.Count≥1`
- **Ms2**
  - 检查：根号
  - 通过标准：平方根 `degHide: true` 后 `expr[0].type` 为 `rad`，`degHide` 为 true
  - Word 无修复
- **Ms3**
  - 检查：n 元
  - 通过标准：`chr` 为 `∑` 的 nary 插入后树上 `type` 为 `nary`
  - Word 无修复
- **Ms4**
  - 检查：嵌套
  - 通过标准：`(a/b)^2` 写成 `sSup` 套 `delim` 套 `frac` 后 `expr[0].type` 为 `sSup`
  - Word 无修复
- **Ms5**
  - 检查：改结构
  - 通过标准：线性公式上 `setMath` 给 `expr` 后仍是分数，不摊平
  - Word 无修复
- **Ms6**
  - 检查：摊平
  - 通过标准：结构化公式上 `setMath` 只给 `text` 后 `expr` 为一条 `text`
  - Word 仍当公式
- **Ms7**
  - 检查：拒错
  - 通过标准：同时给 `text` 与 `expr` 得 `InvalidField`，不写输出文件
- **Ms8**
  - 检查：create
  - 通过标准：create 段上 `maths[].expr` 为分数后树上 `type` 为 `frac`
  - Word 无修复

## 内容控件

`insertContentControl` / `setContentControl` / `deleteContentControl` 读写 `w:sdt`。不实现开发者模式 UI。
手测脚本 `manual-check/office/cases/docx-sdt.ps1`。

- **Cc1**
  - 检查：名单
  - 通过标准：无 `scope.contentControls` 时有 `contentControlCount` 且没有 `contentControls`
  - `insertContentControl` 之后 `contentControlCount` 为 1，给了 `scope.contentControls` 能列出 `sdt:1`，`kind` 为 `plainText`，`text` 为写入的原文
- **Cc2**
  - 检查：打开
  - 通过标准：Word 打开后无修复，能看见内容控件
- **Cc3**
  - 检查：改字
  - 通过标准：`setContentControl` 之后树上 `text` 变成新值，Word 仍把它当内容控件
- **Cc4**
  - 检查：下拉
  - 通过标准：`kind` 为 `dropDown` 的插入产物树上 `kind` 为 `dropDown`，Word 无修复
- **Cc5**
  - 检查：删除
  - 通过标准：`deleteContentControl` 且 `keep` 为 `content` 之后 `contentControlCount` 为 0，原文字还在，标题段还在，Word 无修复
- **Cc6**
  - 检查：页眉
  - 通过标准：默认页眉插入一条纯文本控件后，容器上 `contentControlCount` 为 1，Word 无修复

手测脚本 `manual-check/office/cases/docx-sdt-types.ps1`。

- **Ct1**
  - 检查：复选
  - 通过标准：`insertContentControl` `kind` 为 `checkbox`、`checked` 为 true 之后树上 `kind` 为 `checkbox`、`checked` 为 true
  - Word 无修复且 `Type=8`
- **Ct2**
  - 检查：图片
  - 通过标准：`kind` 为 `picture` 且给了 `file` 之后树上 `kind` 为 `picture`
  - Word 无修复且 `Type=2`
- **Ct3**
  - 检查：组合
  - 通过标准：`kind` 为 `group` 之后树上 `kind` 为 `group`
  - Word 无修复且 `Type=7`
- **Ct4**
  - 检查：公式
  - 通过标准：`kind` 为 `equation` 之后树上 `kind` 为 `equation`，ZIP 含 `w:equation`
  - Word 无修复
  - COM `ContentControls` 可以没有这一条
- **Ct5**
  - 检查：改复选
  - 通过标准：`setContentControl` 给 `checked` 为 false 之后树上 `checked` 为 false
  - Word 仍是复选控件
- **Ct6**
  - 检查：换图
  - 通过标准：图片控件上 `setContentControl` 给另一份 `file` 之后 Word 无修复，仍是 `Type=2`
- **Ct7**
  - 检查：拒改组
  - 通过标准：对 `group` 发 `setContentControl` 得 `NotImplemented`，不写输出文件
- **Ct8**
  - 检查：拒无图
  - 通过标准：`kind` 为 `picture` 却没有 `file` 得 `InvalidField`，不写输出文件

## create 补齐

`paragraphs[].charts` / `footnote` / `endnote` / `comments` 与已有 `sectionType` 写进同一份 create JSON。
手测脚本 `manual-check/office/cases/docx-create-full.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Cf1 | 节 | `sections` 长度为 2；第一段带 `sectionType` 对应的分节 |
| Cf2 | 图表 | `chartCount` 为 1，给了 `scope.charts` 时 `type` 为 `pie` |
| Cf3 | 脚注 | `footnoteCount` 为 1，注内可见文字为写入的原文 |
| Cf4 | 尾注 | `endnoteCount` 为 1，注内可见文字为写入的原文 |
| Cf5 | 批注 | `commentCount` 为 1，批注内可见文字为写入的原文 |
| Cf6 | 打开 | Word 打开后无修复；能看见饼图、脚注、尾注与批注 |

## create 补齐剩余

`paragraphs[].hyperlinks` / `bookmarks` / `fields` / `maths` / `contentControls`、`comments[].replies`、
`document.numberings` / `tocs` 写进同一份 create JSON。
不写修订痕迹。手测脚本 `manual-check/office/cases/docx-create-remain.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Cs1 | 超链接 | `hyperlinkCount` 为 1；给了 `scope.hyperlinks` 时 `url` 为 `https://example.com`，显示字为 `点这里` |
| Cs2 | 书签 | `bookmarkCount` 为 1；给了 `scope.bookmarks` 时 `name` 为 `ch1` |
| Cs3 | 域 | `fieldCount` 至少为 2（含目录域）；给了 `scope.fields` 时有一条 `instr` 含 `DATE` |
| Cs4 | 编号 | `numberingCount` 为 1；第二段绑了 `num:1` |
| Cs5 | 公式 | `mathCount` 为 1；给了 `scope.maths` 时 `text` 为 `x+1`，`kind` 为 `inline` |
| Cs6 | 内容控件 | `contentControlCount` 为 1；给了 `scope.contentControls` 时 `text` 为 `控件值`，`kind` 为 `plainText` |
| Cs7 | 目录 | `tocCount` 为 1 |
| Cs8 | 回复 | `commentCount` 为 2；给了 `scope.comments` 时一条 `parent` 为 `comment:0`，可见字为 `回复一` |
| Cs9 | 打开 | Word 打开后无修复；能看见链、书签、编号、公式、内容控件与批注回复 |

## create 第四轮

create 复用 46–51 已有 apply：自定义样式、表外观、无框形状、连接线、拼音、题注 / 交叉引用 / 索引 / 参考文献 / 构建基块、嵌入字体、东亚设置、例外编辑区、图与框上的 DrawingML 链。不写修订痕迹。只给 `protection.password`、不给限制编辑种类，报 `InvalidField`。配上 `edit` 的口令由 `docx-create-password.ps1` 覆盖。不算排版引擎。
手测脚本 `manual-check/office/cases/docx-create-round4.ps1`。编号用 Cy（不要和 Cd、Cs、Cf、C、Pq 撞）。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Cy1 | 样式与表 | `styleCount` 至少 5，名单含 `styleId=ooxpTbl` 且 `type=table`；`tableCount` 为 1，`style` 为 `ooxpTbl`，`look.firstRow` 为 true，第一格 `fill` 为 `FF0000` |
| Cy2 | 形状与连接线 | 第二段 `shapeCount` / `connectorCount` 为 1，`prst` 为 `ellipse` / `straightConnector1` |
| Cy3 | 拼音与引用 | `rubyCount` 为 1，`base` / `rt` 为汉字与拼音；`indexEntryCount`、`citationCount`、`crossRefCount`、`editRangeCount` 均为 1，`edGrp` 为 everyone |
| Cy4 | 题注书目基块 | `captionCount` / `indexCount` / `bibliographyCount` / `sourceCount` / `docPartCount` 均为 1，`bibliographyStyle.styleName` 为 MLA |
| Cy5 | 字体与东亚 | `embeddedFontCount` 为 1，名为 Calibri；`fontEmbed.trueType` 为 true；`eastAsianSettings.characterSpacingControl` 为 `compressPunctuation` |
| Cy6 | DrawingML 链 | `pictureCount` 为 1，图上 `hyperlink.url` 为 `https://example.com`；根上 `hyperlinkCount` 仍为 0 |
| Cy7 | password | 只给 `protection.password`、不给限制编辑种类，报 `InvalidField` |
| Cy8 | 打开 | Word 打开后无修复；COM `Shapes.Count` 至少 2，第一格底纹 RGB=255，图上 Hyperlink.Address 为 example.com，`BibliographyStyle` 为 MLA |

## create 第五轮

create 复用 53–55 已有 apply：框内表、框内链 / 书签 / 域 / 公式 / 控件 / 拼音、段边框底纹制表位、run 突出显示 / 删除线 / 上下标、组合、`writePassword`。不写修订痕迹。限制编辑口令要配 `edit`，只给 `password` 报 `InvalidField`。不算排版引擎。
手测脚本 `manual-check/office/cases/docx-create-round5.ps1`。编号用 Cz（不要和 Cy、Cs、Cf、C、Cd、Bx、Bf、Bd、Rh 撞）。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Cz1 | 框内表 | 框上 `tableCount` 为 1，根上 `tableCount` 为 0 |
| Cz2 | 框内链 | 框上 `hyperlinkCount` 为 1，根上 `hyperlinkCount` 为 0 |
| Cz3 | 段与 run | 段 `borders.top.val` 为 `single`，`shd.fill` 为 `FFFF00`，`tabs[0].pos` 为 720；run `highlight` 为 `yellow`，`strike` 为 true |
| Cz4 | 组合 | Word 无 Repair；COM `Shapes` 有 `Type=6` 的组合，或两个成员仍在 |
| Cz5 | writePassword | `hasPassword` 为 true，树里没有明文 |
| Cz6 | 打开 | Word 打开无 Repair；COM 能看见框内表或链、段底纹或突出显示。标题含文件名 |
| Cz7 | 拒写 | 空 `runs` 或 xlsx 上这些字段报 `InvalidField`。不强制开 Word |

## 容器内 drawing

文本框、脚注、尾注、批注里的图表与 SmartArt 进树，op 与正文相同。注内与批注内的文本框若已在文件里则进树；`insertTextBox` 不往这些故事里写，与 Word 相同。
手测脚本 `manual-check/office/cases/docx-container-drawing.ps1`。

- **Cd1**
  - 检查：批注内柱形图
  - 通过标准：批注段落上 `insertChart` 后，给了 `scope.comments` 与 `scope.charts` 时批注上 `chartCount` 为 1，图表 id 以 `comment:` 开头
  - 根上 `chartCount` 仍等于正文图表数
- **Cd2**
  - 检查：批注内打开
  - 通过标准：Word 打开后无修复，审阅窗格能看见该批注
- **Cd3**
  - 检查：注内文本框
  - 通过标准：对脚注段落发 `insertTextBox` 报 `InvalidField`，不写输出文件
- **Cd4**
  - 检查：批注内文本框
  - 通过标准：对批注段落发 `insertTextBox` 报 `InvalidField`，不写输出文件
- **Cd5**
  - 检查：框内图表
  - 通过标准：正文框内段落上 `insertChart` 后，框上 `chartCount` 为 1，根上 `chartCount` 不含该图
- **Cd6**
  - 检查：容器 id
  - 通过标准：对批注容器 id 发 `insertChart`、对框 id 发 `insertTextBox` 均 `InvalidId`

## 文档属性

三种格式共用字段与 op。脚本 `manual-check/office/cases/docx-docprops.ps1`、`xlsx-docprops.ps1`、`pptx-docprops.ps1`。
编号用 Dp / Xd / Pd（不要跟页眉图的 Xp、幻灯片图的 Pp 混）。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Dp1 | inspect | 无 scope 时根上有 `props`；create 产物 `creator` 为 `ooxp`，`title` 为 `""` |
| Dp2 | 改标题 | `setProps` 只改 `title` 后 Word 文件属性标题是新字；未点名的公司名还在；标题栏无 Repair |
| Dp3 | 自定义 | `setCustomProp` 后树上 id 为 `prop:…`；Word 自定义属性能看见 |
| Dp4 | 删除 | `deleteCustomProp` 后名单没有该项 |
| Dp5 | 拒错 id | 对段落 id 发 `setProps` 得 `InvalidId`；一个字段都不给得 `InvalidField`。不强制开 Word |

## 嵌入对象

正文与页眉页脚里的 OLE / 包嵌入。样例由 Word `InlineShapes.AddOLEObject` 另存嵌入 xlsx。手测脚本 `manual-check/office/cases/docx-ole.ps1`。

- **Ol1**
  - 检查：名单
  - 通过标准：无 scope 时根上有 `oleObjectCount`、没有 `oleObjects`
  - `scope.oleObjects` 时正文有一条，`progId` 含 `Excel`，id 形如 `para:…/ole:…`
- **Ol2**
  - 检查：导出
  - 通过标准：对该 OLE id 发 `exportMedia`，字节与 ZIP 内 embeddings 部件一致
- **Ol3**
  - 检查：删除
  - 通过标准：`deleteOle` 后 `oleObjectCount` 为 0，Word 无修复
- **Ol4**
  - 检查：插入
  - 通过标准：对段落发 `insertOle` 写入本地 xlsx 后 Word 无修复，树上能看见
- **Ol5**
  - 检查：拒错 id
  - 通过标准：对段落 id 发 `deleteOle`、对 OLE id 发 `insertOle` 得 `InvalidId`
  - 不强制开 Word
- **Ol6**
  - 检查：事后改尺寸
  - 通过标准：✅ `setOleSize` 改 `cx` 后树上 `cx` / `cy` 对得上，`v:shape` 的 `style` 宽高跟着变
  - Word 16.0 打开无修复
  - 不启动 OLE 服务器

## 文档保护

只认 `settings.xml` 的 `w:documentProtection` / `w:writeProtection`。不破解密码。
手测脚本 `manual-check/office/cases/docx-protect.ps1`。带密码哈希见下面 Ph 组。

- **Pr1**
  - 检查：inspect
  - 通过标准：Word 无密码只读保护另存后，无 scope 时根上有 `protection`
  - `enforced` 为 true，`edit` 为 `readOnly`，`hasPassword` 为 false
- **Pr2**
  - 检查：关掉
  - 通过标准：`setProtection` `enforced: false` 后 Word `ProtectionType` 为 `-1`，标题栏无 Repair
- **Pr3**
  - 检查：打开窗体
  - 通过标准：对未保护文件 `enforced: true`、`edit: forms` 后 Word `ProtectionType` 为 `2`，标题栏无 Repair
- **Pr4**
  - 检查：只读推荐
  - 通过标准：Word 勾选只读推荐另存后，`readOnlyRecommended` 为 true
- **Pr5**
  - 检查：有密码
  - 通过标准：Word 带密码保护另存后，`hasPassword` 为 true，树 JSON 不含该明文
  - 对其发改 `enforced` 得 `NotImplemented`，不写文件
- **Pr6**
  - 检查：拒错 id
  - 通过标准：对段落 id 发 `setProtection` 得 `InvalidId`
  - 一个字段都不给得 `InvalidField`
  - 不强制开 Word

## 保护密码哈希

`setProtection` 的 `password` 写 Word 16.0 限制编辑 SHA-512（先做遗留口令预处理）。inspect 仍只报 `hasPassword`。不破解密码。
手测脚本 `manual-check/office/cases/docx-protect-hash.ps1`。编号用 Ph。不要和 Pr、Pb 撞。Pr5「无口令改带哈希样例」仍为 `NotImplemented`。

- **Ph1**
  - 项：inspect
  - 期望：`setProtection` 带 `password` 后 `hasPassword=true`，树里没有哈希或明文
  - settings 有 `cryptAlgorithmSid="14"`
- **Ph2**
  - 项：打开
  - 期望：Word 打开写入产物后 `ProtectionType` 为 `3`
  - 错误口令 `Unprotect` 失败，`Unprotect("secret")` 成功
  - 标题栏无 Repair
- **Ph3**
  - 项：解开
  - 期望：给对得上的 `password` 且 `enforced: false` 后 `hasPassword=false`
- **Ph4**
  - 项：解开后打开
  - 期望：Word 打开解开产物后 `ProtectionType` 为 `-1`，无 Repair
- **Ph5**
  - 项：拒写
  - 期望：空 `password`、错口令解开、不给 `password` 改带 sid=14 的保护，分别得 `InvalidField` 或 `NotImplemented`
  - 均未写文件
  - 不强制为拒写开 Word

## 保护修订与公式属性

按节例外编辑区、节窗体保护、writeProtection 修改密码、修订 move、OMML 对齐 / 幻影 / 边框盒、参考文献 SelectedStyle。
不算排版引擎，不破解密码。
手测脚本 `manual-check/office/cases/docx-protect-rev-math.ps1`。编号用 Pq。不要和 Pr、Ph、Rm、Mm、Ms、Bi 撞。

| 编号 | 项 | 期望 |
| --- | --- | --- |
| Pq1 | 例外区 | `insertEditRange` 后正文与页眉各 1 条，`edGrp` 为 everyone / editors。无 Repair |
| Pq2 | formProt | 窗体保护后第 1 节 `formProt=false`。Word `ProtectionType=2` 且 `ProtectedForForms=false`。无 Repair |
| Pq3 | writePassword | `hasPassword=true`，树里没有明文，settings 有 sid=14 的 `writeProtection`。用 `WritePasswordDocument` 打开无 Repair，`ReadOnly=false` |
| Pq4 | 拒写 | 空 `writePassword`、错口令、不给口令改哈希，分别得 `InvalidField` / `InvalidField` / `NotImplemented`。未写文件 |
| Pq5 | move | inspect 收入 `moveFrom` / `moveTo`，接受后 Word `Revisions.Count=0`。无 Repair |
| Pq6 | OMML | 显示公式 `jc=center`，expr 含 phant / borderBox / brk。Word `OMath.Justification=2`。无 Repair |
| Pq7 | 参考文献样式 | `styleName=MLA`、`version=7`。Word `BibliographyStyle` 为 MLA 或空。无 Repair |
| Pq8 | 跨格式拒写 | xlsx `insertEditRange`、pptx `setBibliographyStyle` 报 `InvalidField`。未写文件。不强制开 Word |

- **Pq1–Pq8** ✅
  - Word 16.0：`docx-protect-rev-math.ps1`，8 PASS / 0 FAIL

## 水印

页眉里的 DrawingML 水印。不认、不改 Word 自带的 VML 水印。手测脚本 `manual-check/office/cases/docx-watermark.ps1`。

- **Wm1**
  - 检查：文字
  - 通过标准：对无页眉文件 `insertWatermark` 且 `text` 为 `CONFIDENTIAL` 后，无 scope 时 `watermarkCount` 为 1
  - `scope.watermarks` 时 `kind` 为 `text`，id 形如 `header:default:1/watermark:…`
  - Word 页眉形状可见该字，标题栏无 Repair
- **Wm2**
  - 检查：关掉
  - 通过标准：`deleteWatermark` 且 id 为 `docx:document` 后 `watermarkCount` 为 0，Word 无修复
- **Wm3**
  - 检查：图片
  - 通过标准：插入 PNG 水印后 Word 无修复，树上 `kind` 为 `picture`
- **Wm4**
  - 检查：普通图
  - 通过标准：往页眉插一张普通图后 `watermarkCount` 仍为 0，该图进 `pictures`
- **Wm5**
  - 检查：拒错
  - 通过标准：对段落 id 发 `insertWatermark` 得 `InvalidId`
  - `text` 与 `file` 都给得 `InvalidField`
  - 不强制开 Word

## 主题

主文档 theme 部件的配色与字体方案。不算主题排版引擎。手测脚本 `manual-check/office/cases/docx-theme.ps1`。编号用 Th。不要和表格 / 文本框的 T 组撞。`fmtScheme` 与 `themeOverride` 见 Fm 组。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Th1 | inspect | Word 空白另存后，无 scope 时根上有 `theme` 对象；`colors.accent1` 为六位十六进制；字段顺序在 `protection` 之后。标题栏无 Repair |
| Th2 | 改色 | `setThemeColors` `accent1` 为 `FF0000` 后树上为 `FF0000`，Word 无修复 |
| Th3 | 改字 | `setThemeFonts` `majorLatin` 为 `Arial` 后树上为 `Arial`，Word 无修复 |
| Th4 | 无 theme | 对 ooxp create（无 theme 部件）发 `setThemeColors` 得 `IdNotFound`，不写文件。不强制开 Word |
| Th5 | 拒错 id | 对段落 id 发 `setThemeColors` 得 `InvalidId`；一个槽位都不给得 `InvalidField`。不强制开 Word |

## 主题与页面背景剩余

`setThemeFmt` 改 `fmtScheme` 常用槽；`setThemeOverride` 读写主文档 themeOverride；`setPageBorders` 的 `val` 含艺术型；`setPageBackground` 可写 DrawingML 渐变或 VML 图片。
create 可写 `document.theme` / `pageBackground` / `pageBorders`。不算主题排版引擎。
手测脚本 `manual-check/office/cases/docx-theme-bg-remain.ps1`。编号用 Fm。不要和 Fe、Ft、Tf、Th、Pb 撞。

| 编号 | 项 | 期望 |
| --- | --- | --- |
| Fm1 | fmt | Word 空白另存后 `theme.fmt.name` 为 `Office`，`fills` / `lines` 长度为 3；`themeOverride` 为 `null`。无 Repair |
| Fm2 | 改槽 | `setThemeFmt` 把 `line.index=0` 的 `w` 写成 `38100` 后树上对得上，Word 无修复 |
| Fm3 | 覆盖 | `setThemeOverride` `accent1=FF0000` 后树上 `themeOverride.colors.accent1` 为 `FF0000`，Word 无修复；`enabled: false` 后为 `null` |
| Fm4 | 艺术边框 | `setPageBorders` 四边 `val=apples` 后树上为 `apples`，Word 无修复 |
| Fm5 | DrawingML 背景 | `setPageBackground` `fill.kind=gradient` 后 `pageBackground.fill.kind` 为 `drawing`，Word 无修复 |
| Fm6 | create | 给了 `document.theme` 与 `pageBackground` 后树上有 theme 与背景色，Word 无修复。默认 create 仍无 theme |
| Fm7 | 拒错 | 对段落 id 发 `setThemeFmt` 得 `InvalidId`；xlsx 上对 `slide:1` 发 `setThemeOverride` 得 `InvalidId`。未写文件 |

## 主题剩余

theme 与配色 / 字体方案的 `name`、补充色板、脚本字体、以及 `themeFontLang`。不算主题排版引擎。
手测脚本 `manual-check/office/cases/docx-theme-remain.ps1`。编号用 Thr。不要和 Th、Fm、Ea 撞。

| 编号 | 项 | 期望 | 状态 |
| --- | --- | --- | --- |
| Thr1 | inspect | Word 空白另存后 `theme.extraSchemes` 为数组；`fonts.majorScripts` / `minorScripts` 为数组且在 `minorCs` 之后。无 Repair | ✅ |
| Thr2 | 改名 | `setThemeColors` 写 `name=Custom`、`themeName=Custom Theme` 后树上对得上。Word 无修复 | ✅ |
| Thr3 | 脚本 | `setThemeFonts` 写 `majorScripts` 一项 `Jpan` / `Yu Mincho` 后树上对得上。Word 无修复 | ✅ |
| Thr4 | 补充色板 | `setThemeExtraSchemes` 写入一份 `Extra` 后树上 `name` 与 `accent1` 对得上；再 `null` 后为 `[]`。Word 无修复 | ✅ |
| Thr5 | 主题语言 | `setEastAsianSettings` 写 `themeFontLang.val=ja-JP` 后树上对得上。Word 无修复 | ✅ |
| Thr6 | 拒错 | 无 theme 发名字 / 补充色板得 `IdNotFound`；xlsx 无 theme 发 `setThemeExtraSchemes` 同样得 `IdNotFound`。未写文件 | ✅ |

## 主题 objectDefaults

theme 上的 `a:objectDefaults`。不算主题排版引擎。不把 `phClr` 展开成 RGB。
手测脚本 `manual-check/office/cases/docx-theme-defaults.ps1`。编号用 Tdf。不要和 Th、Thr、Fm 撞。

| 编号 | 项 | 期望 | 状态 |
| --- | --- | --- | --- |
| Tdf1 | inspect | create 给了 `document.theme` 后 `objectDefaults` 为 `null`。无 Repair | ✅ |
| Tdf2 | 线宽 | `setThemeObjectDefaults` 写 `sp.lnW=38100` 后树上对得上。Word 无修复 | ✅ |
| Tdf3 | 样式 | 写 `sp.style.lnRef=1` `fontRef=minor` 后树上对得上。无 Repair | ✅ |
| Tdf4 | 清除 | `objectDefaults: null` 后为 `null`。无 Repair | ✅ |
| Tdf5 | 拒错 | 对段落 id 发得 `InvalidId`。缺字段、非法 `fontRef`、无 theme 得 `InvalidField` / `IdNotFound`。未写文件 | ✅ |

## 组合与连接线

正文与页眉页脚里的浮动图 / 文本框 / 连接线可组合、解开；Word 写成 `wps:wsp` 的连接线可插删。组合本身不进树。手测脚本 `manual-check/office/cases/docx-groups.ps1`。
编号用 Dg。不要和 T、Tb、Rv、Rm、Pb、Ph 撞。不实现放映引擎。无框形状见 Ds。

- **Dg1**
  - 项：名单
  - 期望：无 scope 时根上有 `connectorCount`、没有正文 `connectors`
- **Dg2**
  - 项：插连接线
  - 期望：`insertConnector` 后 Word 无 Repair，COM `Type=9` 或 `Connector`，树上 `prst=straightConnector1`
- **Dg3**
  - 项：删连接线
  - 期望：`deleteConnector` 后 `connectorCount` 为 0
- **Dg4**
  - 项：组合框
  - 期望：两个 `insertTextBox` 再 `groupShapes`，Word `Type=6`，树上仍是两个框
- **Dg5**
  - 项：解组
  - 期望：`ungroup` 后仍是独立框，无 Repair
- **Dg6**
  - 项：组合图
  - 期望：两张浮动图 group 后 `pictureCount=2`（须先 `setPictureWrap` 成浮动）
- **Dg7**
  - 项：页眉
  - 期望：页眉 `insertConnector` 后容器 `connectorCount=1`，根上正文计数仍为 0
  - 无 Repair
- **Dg8**
  - 项：拒写
  - 期望：错 id / 行内当成员 / 对 docx 发 `setShapeGeom` / 对 xlsx 用段落 id 发 `insertConnector` 分别得 `IdNotFound` 或 `InvalidField` / `InvalidId`
  - 均未写文件

## 无框形状几何

正文与页眉页脚里无文本框的 `wps:wsp`。不算排版引擎。不编辑 VML。注内 / 批注内 / 框内不插。用库 create 产物再 apply。
手测脚本 `manual-check/office/cases/docx-drawing-shape.ps1`。编号用 Ds。不要和 Dg、Dh、B、T 撞。

- **Ds1**
  - 项：插矩形
  - 期望：`insertShape` 后 Word 无 Repair，COM `AutoShapeType=1`，树上 `prst=rect`，id 带 `/shape:`
- **Ds2**
  - 项：插椭圆
  - 期望：`prst=ellipse` 后 COM `AutoShapeType=9`，树上 `prst=ellipse`
- **Ds3**
  - 项：改尺寸
  - 期望：`setShapeSize` 后 `cx` / `cy` 对得上，无 Repair
- **Ds4**
  - 项：改填充
  - 期望：`setShapeFill` `color=FF0000` 后 COM 填充为红，无 Repair
- **Ds5**
  - 项：改线
  - 期望：`setShapeLine` `color=0000FF` 后 COM 线条为蓝，无 Repair
- **Ds6**
  - 项：删除
  - 期望：`deleteShape` 后 `shapeCount` 为 0
- **Ds7**
  - 项：页眉
  - 期望：页眉 `insertShape` 后容器 `shapeCount=1`，根上正文计数仍为 0，无 Repair
- **Ds8**
  - 项：拒写
  - 期望：连接线几何名给 `insertShape`、带 `text`、对形状 id 发 `setShapeGeom` 分别得 `InvalidField`，未写文件
  - 不强制开 Word

## 嵌入 SVG

正文与页眉页脚的 DrawingML 图可带 Office 2016 `asvg:svgBlip`。不算排版引擎，不把 SVG 原文放进 inspect 树。xlsx / pptx 见 Xvg / Pvg（C5）。
手测脚本 `manual-check/office/cases/docx-svg.ps1`。编号用 Sv。

- **Sv1**
  - 检查：插入
  - 通过标准：`insertPicture` 把本地 SVG 写入正文段后，树上 `contentType` 为 `image/svg+xml`，Word 无修复
- **Sv2**
  - 检查：导出
  - 通过标准：对该图 `exportMedia` 得到 SVG 原字节，`contentType` 为 `image/svg+xml`
- **Sv3**
  - 检查：create
  - 通过标准：`create` 的 `pictures[].file` 指向 SVG 后 Word 无修复，树上同样有 `contentType`
- **Sv4**
  - 检查：换回位图
  - 通过标准：`replacePicture` 换成 PNG 后树上没有 `contentType`，导出是 PNG，Word 无修复
- **Sv5**
  - 检查：页眉
  - 通过标准：往页眉段落插 SVG，容器 `pictures` 有 `contentType`，无修复
- **Sv6**
  - 检查：拒错
  - 通过标准：空文件或不是 SVG / 位图的 `file` 报 `InvalidField`。不强制开 Word

## 嵌入 EMF / WMF / TIFF

正文与页眉页脚的 DrawingML 图按文件头认 EMF、WMF、TIFF，写入原字节。不要 PNG 占位，不要 `svgBlip`。不算排版引擎。xlsx / pptx 见 Xem / Pem（C6）。
手测脚本 `manual-check/office/cases/docx-emf.ps1`。编号用 Dem。
Word 行内 COM 认 `Type=3`（`wdInlineShapePicture`），不是 SVG 的 `17`。

- **Dem1**
  - 检查：插入
  - 通过标准：`insertPicture` 把本地 EMF 写入正文段后，树上 `contentType` 为 `image/x-emf`，Word 无修复
  - 行内 `Type=3`，包内有 `.emf`、没有 `svgBlip`
- **Dem2**
  - 检查：导出
  - 通过标准：对该图 `exportMedia` 得到 EMF 原字节，`contentType` 为 `image/x-emf`
- **Dem3**
  - 检查：create
  - 通过标准：`create` 的 `pictures[].file` 指向 WMF 后 Word 无修复，树上 `contentType` 为 `image/x-wmf`
- **Dem4**
  - 检查：换回位图
  - 通过标准：`replacePicture` 换成 PNG 后树上没有 `contentType`，导出是 PNG，Word 无修复
- **Dem5**
  - 检查：页眉
  - 通过标准：往页眉段落插 TIFF，容器 `pictures` 的 `contentType` 为 `image/tiff`，无修复
- **Dem6**
  - 检查：拒错
  - 通过标准：空文件或不是图片的 `file` 报 `InvalidField`。不强制开 Word

## 正文 3D 模型

正文与页眉页脚里的 `am3d` AlternateContent。只认 GLB。不进 `pictures`。Fallback 海报不进 `pictures`。不算 3D 渲染引擎。xlsx / pptx 见 X3d / P3d（C7）。
手测脚本 `manual-check/office/cases/docx-model3d.ps1`。编号用 D3d。
Word 行内 COM 认 `Type=19`（`wdInlineShape3DModel`）。脚注、尾注和批注见 D3n。

- **D3d1**
  - 检查：插入
  - 通过标准：`insertModel3d` 把本地 GLB 写入正文段后，树上 `contentType` 为 `model/gltf.binary`，不进 `pictures`，Word 无修复
  - 行内 `Type=19`，包内有 `.glb`、XML 含 `am3d:model3d`
- **D3d2**
  - 检查：导出
  - 通过标准：对该 3D id `exportMedia` 得到 GLB 原字节，`contentType` 为 `model/gltf.binary`
- **D3d3**
  - 检查：create
  - 通过标准：`create` 的 `model3ds[].file` 指向 GLB 后 Word 无修复，树上 `contentType` 为 `model/gltf.binary`
- **D3d4**
  - 检查：替换
  - 通过标准：`replaceModel3d` 换成另一份 GLB 后 id 不变，导出是新字节，Word 无修复
- **D3d5**
  - 检查：页眉
  - 通过标准：往页眉段落插 GLB，容器 `model3ds` 的 `contentType` 为 `model/gltf.binary`，根上 `model3dCount` 仍为 0，无修复
- **D3d6**
  - 检查：拒错
  - 通过标准：不是 GLB 的 `file` 报 `InvalidField`。不强制开 Word

## 注内与批注内 3D

脚注、尾注和批注段落上的行内 `am3d`。关系挂在该部件。不进 `pictures`。改成浮动报 `InvalidField`。
手测脚本 `manual-check/office/cases/docx-model3d-notes.ps1`。编号用 D3n。

- **D3n1**
  - 检查：插入
  - 通过标准：`insertModel3d` 写入脚注段和批注段后，根上 `model3dCount` 为 2，id 分别带 `footnote:` 与 `comment:`，`kind` 为 `inline`，`pictureCount` 为 0。Word 16.0 打开没有 Repair。脚注和批注 Range 的 `InlineShapes` 至少各有 1 个

## 3D 事后几何

已有 3D 模型改 `wp:extent` 与行内 / 浮动绕排。不换 GLB。不算 3D 渲染。对 3D id 发 `setPictureSize` / `setPictureWrap` 仍报 `InvalidId`。
手测脚本 `manual-check/office/cases/docx-model3d-geom.ps1`。编号用 D3g，不要和 D3d、Dh 撞。xlsx / pptx 见 X3g / P3g（C12）。

- **D3g1**
  - 检查：改尺寸
  - 通过标准：`setModel3dSize` 写 `cx` / `cy` 后树上对得上。Word 无修复。COM `Width` / `Height` 对得上（允许 2 磅误差）
- **D3g2**
  - 检查：改绕排
  - 通过标准：`setModel3dWrap` 写成浮动 `square` 后树上 `kind` 为 `anchor`、`wrap` 为 `square`。Word 无修复
- **D3g3**
  - 检查：改回行内
  - 通过标准：再发 `kind: inline` 后树上 `kind` 为 `inline`，`wrap` 为 `null`。Word 无修复
- **D3g4**
  - 检查：替换不改几何
  - 通过标准：改过尺寸后再 `replaceModel3d`，树上 `cx` / `cy` 仍是新值。Word 无修复
- **D3g5**
  - 检查：页眉
  - 通过标准：页眉 3D `setModel3dSize` 后容器树上对得上。Word 无修复
- **D3g6**
  - 检查：拒错
  - 通过标准：`cx` / `cy` 都不给报 `InvalidField`。对 3D id 发 `setPictureSize` 报 `InvalidId`。不强制开 Word

## 3D 相机与灯光

已有 3D 改 `am3d:camera`、`am3d:rot` 与 `spPr` 里的 `a:scene3d/a:lightRig`。不换 GLB。不算 3D 渲染。不把 `ambientLight` / `ptLight` 放进树。
手测脚本 `manual-check/office/cases/docx-model3d-camera.ps1`。编号用 D3c，不要和 D3d、D3g 撞。xlsx / pptx 见 X3c / P3k（pptx 格属性已占用 P3c）。

- **D3c1** ✅
  - 检查：缺省
  - 通过标准：`insertModel3d` 后树上 `camera.pos.z` 为 `66519324`，`fov` 为 `2700000`，`rot` 三轴 `null`，`lightRig` 为 `null`。Word 无修复
- **D3c2** ✅
  - 检查：改相机 / 旋转 / 光源
  - 通过标准：`setModel3dCamera` 写 `pos.x=50400000`、`fov=1892375`、旋转三轴与 `lightRig.rev=18000000` 后树上对得上。Word 无修复
- **D3c3** ✅
  - 检查：去掉光源
  - 通过标准：`lightRig` 为 `null` 后树上 `lightRig` 为 `null`。Word 无修复
- **D3c4** ✅
  - 检查：页眉
  - 通过标准：页眉 3D 可改 `fov`，容器树上对得上。Word 无修复
- **D3c5** ✅
  - 检查：拒错
  - 通过标准：三个字段都不给报 `InvalidField`。`fov` 为负报 `InvalidField`。不强制开 Word

## 墨迹画布

正文与页眉页脚里的 `wpc` 画布加 InkML 部件。不算排版引擎，不把 InkML 原文放进 inspect 树，不编批注墨迹。xlsx / pptx 本期不跟。
手测脚本 `manual-check/office/cases/docx-ink.ps1`。编号用 Ic，不要和批注墨迹 Ik 混号。

- **Ic1**
  - 检查：插入
  - 通过标准：`insertInkCanvas` 写入正文段后，树上 `contentType` 为 `application/inkml+xml`，Word 无修复。COM 在 `Shapes` 上 `Type=20`（`msoCanvas`），`InlineShapes.Count` 为 0
- **Ic2**
  - 检查：导出
  - 通过标准：对该墨迹 `exportMedia` 得到 InkML 原字节，`contentType` 为 `application/inkml+xml`
- **Ic3**
  - 检查：create
  - 通过标准：`create` 的 `inkCanvases[].file` 指向 InkML 后 Word 无修复，树上同样有 `contentType`
- **Ic4**
  - 检查：替换
  - 通过标准：`replaceInkCanvas` 换成另一份 InkML 后导出是新字节，Word 无修复
- **Ic5**
  - 检查：页眉
  - 通过标准：往页眉段落插墨迹画布，容器 `inkCanvases` 有 `contentType`，根上 `inkCanvasCount` 仍为 0，无修复
- **Ic6**
  - 检查：拒错
  - 通过标准：不是 InkML 的 `file` 报 `InvalidField`。不强制开 Word

## 墨迹画布尺寸与绕排

已有墨迹画布改 `wp:extent` 与行内 / 浮动绕排。不换 InkML。不写独立 `wpi` 画布。不算排版引擎。
手测脚本 `manual-check/office/cases/docx-ink-geom.ps1`。编号用 Is，不要和 Ic 撞。

- **Is1**
  - 检查：改尺寸
  - 通过标准：`setInkCanvasSize` 写 `cx` / `cy` 后树上对得上，ZIP 里 `wp:extent` 与 `a:ext` 同步。Word 无修复。COM `Width` / `Height` 对得上（允许 2 磅误差）
- **Is2**
  - 检查：改绕排
  - 通过标准：`setInkCanvasWrap` 写成浮动 `square` 后树上 `kind` 为 `anchor`、`wrap` 为 `square`。Word 无修复
- **Is3**
  - 检查：改回行内
  - 通过标准：再发 `kind: inline` 后树上 `kind` 为 `inline`，`wrap` 为 `null`。Word 无修复
- **Is4**
  - 检查：替换不改几何
  - 通过标准：改过尺寸后再 `replaceInkCanvas`，树上 `cx` / `cy` 仍是新值。Word 无修复
- **Is5**
  - 检查：拒错
  - 通过标准：`cx` / `cy` 都不给报 `InvalidField`。对 xlsx 发 `setInkCanvasSize` 报 `InvalidField`。不强制开 Word

## 图表走共享层

脚本 `manual-check/office/cases/docx-chart-shared.ps1`。

- **Cs1**
  - 检查：插入柱形图
  - 通过标准：✅ Word 16.0 无修复，能看见图表，编辑数据能打开 embedding
- **Cs2**
  - 检查：改系列
  - 通过标准：✅ `setChartSeries` 后 Word 无修复，图表仍在，embedding 能打开
- **Cs3**
  - 检查：趋势线
  - 通过标准：✅ `insertTrendline` 后 Word 无修复，图表仍在，embedding 能打开

## 原样保存（opc 验收）

与 xlsx 清单相同：O1 第一期放弃；零脏写回在 OPC 层验收；O2 按 apply 看未改区域。
