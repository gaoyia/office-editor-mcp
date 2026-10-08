# xlsx 人工检查清单

代码阶段的单元测试覆盖 JSON 解析和 XML 不变量。下面这些必须用 Microsoft Excel 打开一次，不能只靠测试绿。验收只认官方 Office，不再要求 WPS。

检查前用 create 或 apply 在本机生成文件，不要用改过扩展名的 zip 目录冒充。

## 本机结果（2026-09-26）

Microsoft Excel 16.0 已打开，用户确认通过。本机若装了 WPS，`Excel.Application` 会被抢走，测试仍直接启动 `EXCEL.EXE` 再挂 COM。

C、I、A、O、Xp、Xc、Xh 这几组现在由 `manual-check/office/cases/xlsx-*.ps1` 自动判定，跑 `pwsh -File manual-check\office\run.ps1 -Case 'xlsx-*'` 即可复现，逐条结果落在 `manual-check/office/out/office-results.csv`。2026-09-27 补跑 `xlsx-pictures.ps1`，Xp1–Xp7 通过。同日补跑 `xlsx-charts.ps1`，Xc1–Xc7 通过。同日补跑 `xlsx-create-pictures.ps1`，C8–C10 通过。同日补跑 `xlsx-create-charts.ps1`，C11–C13 通过。同日补跑 `xlsx-headerfooter.ps1`，Xh1–Xh5 通过。同日补跑 `xlsx-docprops.ps1`，Xd1–Xd5 通过。同日补跑 `xlsx-charts-remain.ps1`，Xr1–Xr4 通过。同日补跑 `xlsx-sheets.ps1`，Xm1–Xm7 通过。同日补跑 `xlsx-cellstyle.ps1`，Xs1–Xs5 通过。同日补跑 `xlsx-layout.ps1`，Xl1–Xl5 通过。同日补跑 `xlsx-formula.ps1`，Xf1–Xf5 通过。2026-09-28 补跑 `xlsx-validation.ps1`，Xv1–Xv5 通过。同日补跑 `xlsx-conditional-format.ps1`，Xn1–Xn5 通过。同日补跑 `xlsx-pivot.ps1`，Xpt1–Xpt5 通过。同日补跑 `xlsx-defined-name.ps1`，Xnm1–Xnm5 通过。同日补跑 `xlsx-print.ps1`，Xps1–Xps5 通过。同日补跑 `xlsx-view.ps1`，Xvw1–Xvw5 通过。同日补跑 `xlsx-comments.ps1`，Xcm1–Xcm5 通过。同日补跑 `xlsx-shapes.ps1`，Xsa1–Xsa5 通过。同日补跑 `xlsx-hyperlinks.ps1`，Xhl1–Xhl5 通过。同日补跑 `xlsx-sparklines.ps1`，Xsk1–Xsk5 通过。同日补跑 `xlsx-protect.ps1`，Xpr1–Xpr5 通过。同日补跑 `xlsx-create-headerfooter.ps1`，C14–C16 通过。同日补跑 `xlsx-anchor.ps1`，Xa1–Xa5 通过。2026-09-29 补跑 `xlsx-charts-rare.ps1`，Xq1–Xq5 通过。同日补跑 `xlsx-ole.ps1`，Xol1–Xol5 通过。同日补跑 `xlsx-slicers.ps1`，Xsl1–Xsl5 通过。同日补跑 `xlsx-connections.ps1`，Xcn1–Xcn5 通过。2026-10-09 补跑 `xlsx-outline.ps1`，Xon1–Xon5 通过。同日补跑 `xlsx-custom-view.ps1`，Xsv1–Xsv5 通过。同日补跑 `xlsx-create-drawing.ps1`，C17–C21 通过。同日补跑 `xlsx-create-chartsheet.ps1`，C22–C24 通过。同日补跑 `xlsx-sort.ps1`，Xsr1–Xsr5 通过。同日补跑 `xlsx-external-link.ps1`，Xel1–Xel5 通过。2026-09-28 补跑 `xlsx-chartsheet.ps1`，图表工作表 Xs1–Xs4 通过。同日补跑 `xlsx-table.ps1`，Xt1–Xt5 通过。

| 序号 | 状态 | 结果 |
| --- | --- | --- |
| C1 | ✅ | Excel 16.0 打开 `manual-check/office/out/xlsx-create.xlsx`，标题栏无 Repair |
| C3–C7 | ✅ | Excel：Sheet1、A1=1、A2=hello、A3=TRUE、B1=`=SUM(A1)` 且值为 1 |
| I1–I6 | ✅ | 有范围的 inspect 含 A1/A2/A3/B1、无 E1；无 scope 时没有 `cells` |
| A1 | ✅ | Excel 打开 `out/xlsx-applied.xlsx`，标题栏无 Repair |
| A2–A7 | ✅ | Excel：A1=42，B1=`=SUM(A1:A10)` 值为 47，C1 空且数字格式仍是 `0.00`，Sheet2 未动，E1 加粗/红字/黄底/列宽 20 还在，D1 重算为 84 |
| O1 | 🚫 | 领域层不做原样保存：三个动作里没有「只保存」，`apply` 的 `ops` 必须非空。零脏写回见 OPC 库测试。Excel 另存 create 产物见 `xlsx-O1-excel-resave` |
| O2 | ✅ | 未改表与未点名格子的样式还在；未改部件 CRC 与打补丁前相同 |
| Xp1–Xp7 | ✅ | Excel 16.0：无 `pictures` 时 `pictureCount=3`；名单含 `oneCell` 与 `twoCell`；换/插/删无修复；`oneCell` 尺寸 144 磅；锚点改到 F12；export 与 `/xl/media/…` 字节一致 |
| Xc1–Xc7 | ✅ | Excel 16.0：无 `charts` 时 `chartCount=1`、没有名单；名单 id 为 `sheet:Sheet1/chart:2`，系列 10/20/30 与单元格一致；改系列为 100/200/300 无修复；`insertChart` 插 `oneCell` 且 `A10` 一带有数据表；删图后数据格还在；改折线 COM 认 line；`oneCell` 改到 F20、尺寸 144×72 磅；`exportChartData` 得 `InvalidField` |
| C8–C10 | ✅ | Excel 16.0：inspect 第一张 `sheet:Sheet1/pic:1`、`oneCell`、`from=B2`；第二张 `twoCell`、`D2:F8`；打开无 Repair，Shapes=2 |
| C11–C13 | ✅ | Excel 16.0：inspect 第一张 `sheet:Sheet1/chart:1`、`type=col`、`from=E2`；系列 1/2/3 与 `A10` 一带单元格一致；打开无 Repair，Shapes=1 且 HasChart |
| C14–C16 | ✅ | Excel 16.0：`xlsx-create-headerfooter.ps1`，3 PASS / 0 FAIL。inspect 奇数页页眉含 `C14页眉`；PageSetup 中区页眉页脚对得上；页眉图 id 为 `sheet:Sheet1/hf:odd:header/pic:2`。无 Repair |
| Xh1–Xh5 | ✅ | Excel 16.0：无 `headerFooter` 时有 `hasHeaderFooter`、没有名单；展开六槽，奇数页页眉原文含 `Xh页眉`；`setHeaderFooterText` 后 PageSetup 为新字且无 Repair；只开 `pictures` 时名单不含 `hf:`；往 `hf:odd:header` 插图 id 为 `…/pic:2`，换图后无 Repair |
| Xd1–Xd5 | ✅ | Excel 16.0：`xlsx-docprops.ps1`，5 PASS / 0 FAIL。根上 `props`、`creator=ooxp`；只改标题后面板标题是新字、公司名还在；自定义 `prop:部门`；删后名单空；单元格 id 发 `setProps` 为 `InvalidId`。无 Repair |
| Xr1–Xr4 | ✅ | Excel 16.0：`xlsx-charts-remain.ps1`，4 PASS / 0 FAIL。趋势线 COM Count=1；次轴 `AxisGroup=2`；命名区域改数 90/80/70 且 `c:f` 仍为 `ChartVals`；并集改数成功、三维引用 `NotImplemented`。无 Repair |
| Xq1–Xq5 | ✅ | Excel 16.0：`xlsx-charts-rare.ps1`，5 PASS / 0 FAIL。插入股价 / 曲面 / 复合条无 Repair；股价三条转柱形、复合饼转饼；增系列与 `type=chartex`、两条股价均拒写。 |
| Xm1–Xm7 | ✅ | Excel 16.0：`xlsx-sheets.ps1`，7 PASS / 0 FAIL。无 scope 时每张表有 `hidden`；插入 Sheet3、改名附录、挪到最前、隐藏 Sheet2、删除 Sheet2 均无 Repair；隐藏或删除最后一张可见表得 `InvalidField` 且未写文件 |
| Xs1–Xs5 | ✅ | Excel 16.0：`xlsx-cellstyle.ps1`，5 PASS / 0 FAIL。默认 `numFmt=General`；A1 写成 `0.00` 与黄底后值还在、无 Repair；B1 仍是通用格式且无黄底；缺字段得 `InvalidField` 且未写文件 |
| Xl1–Xl5 | ✅ | Excel 16.0：`xlsx-layout.ps1`，5 PASS / 0 FAIL。列宽 20、行高 30、合并 A1:B2 对得上；取消合并无修复；缺字段得 `InvalidField` 且未写文件 |
| Xf1–Xf5 | ✅ | Excel 16.0：`xlsx-formula.ps1`，5 PASS / 0 FAIL。共享主格改公式保住整组；拆组后各格独立；数组公式 Excel 认；非主格 `NotImplemented` |
| Xv1–Xv5 | ✅ | Excel 16.0：`xlsx-validation.ps1`，5 PASS / 0 FAIL。A1 序列下拉能看见 1、2、3；多区域 `A1`/`C1`；删除后无列表校验、无 Repair；缺字段得 `InvalidField` 且未写文件 |
| Xn1–Xn5 | ✅ | Excel 16.0：`xlsx-conditional-format.ps1`，5 PASS / 0 FAIL。A1:A10 大于 0 填绿；两色色阶；删除后无条件格式、无 Repair；缺字段得 `InvalidField` 且未写文件 |
| Xpt1–Xpt5 | ✅ | Excel 16.0：`xlsx-pivot.ps1`，5 PASS / 0 FAIL。建成 SalesPivot 无 Repair；改源到 A1:C5 后仍认表；删除后源数据还在；缺字段得 `InvalidField` 且未写文件 |
| Xnm1–Xnm5 | ✅ | Excel 16.0：`xlsx-defined-name.ps1`，5 PASS / 0 FAIL。建成 Sales 无 Repair；COM Names 能看见；改公式到 A1:A20 后仍认该名；删除后计数为 0；缺字段得 `InvalidField` 且未写文件 |
| Xps1–Xps5 | ✅ | Excel 16.0：`xlsx-print.ps1`，5 PASS / 0 FAIL。默认 `print` 左边距 0.7、竖向、无打印区域；左边距 1 英寸与横向后 COM 对得上且页眉还在；打印区域与网格线 COM 认；缺字段得 `InvalidField` 且未写文件。无 Repair |
| Xvw1–Xvw5 | ✅ | Excel 16.0：`xlsx-view.ps1`，5 PASS / 0 FAIL。默认 `view` 缩放 100、无冻结；冻结首行与缩放 80 后 COM 对得上；工作表筛选范围为 `$A$1:$A$10`；解冻后 `FreezePanes` 为假；缺字段得 `InvalidField` 且未写文件。无 Repair |
| Xcm1–Xcm5 | ✅ | Excel 16.0：`xlsx-comments.ps1`，5 PASS / 0 FAIL。无名单时根与工作表有 `commentCount=0`；`insertComment` 后 COM `CommentsThreaded` 能看见正文且无 Repair；`replyComment` 后 COM `Replies` 与 inspect `replies` 对得上；删除后计数为 0；缺字段得 `InvalidField` 且未写文件 |
| Xsa1–Xsa5 | ✅ | Excel 16.0：`xlsx-shapes.ps1`，5 PASS / 0 FAIL。无名单时根与工作表有 `textBoxCount=0`、`smartArtCount=0`；`insertTextBox` 后 COM `Shapes` 能看见文本框且无 Repair；`setShapeText` 后可见字对得上；`insertSmartArt` `layout=list` 后 COM 能看见 SmartArt，inspect `layout` 对得上；删除后计数为 0；缺 `from` 得 `InvalidField` 且未写文件 |
| Xhl1–Xhl5 | ✅ | Excel 16.0：`xlsx-hyperlinks.ps1`，5 PASS / 0 FAIL。无名单时根与工作表有 `hyperlinkCount=0`；`insertHyperlink` 后 COM `Hyperlinks` 能看见地址且无 Repair；改地址后对得上；删除后只留文字；缺字段得 `InvalidField` 且未写文件 |
| Xsk1–Xsk5 | ✅ | Excel 16.0：`xlsx-sparklines.ps1`，5 PASS / 0 FAIL。无名单时根与工作表有 `sparklineCount=0`；`insertSparkline` 后 COM `SparklineGroups` 能看见折线且无 Repair；改数据范围后对得上；删除后计数为 0；缺字段得 `InvalidField` 且未写文件 |
| Xpr1–Xpr5 | ✅ | Excel 16.0：`xlsx-protect.ps1`，5 PASS / 0 FAIL。无保护时根有 `workbookProtection`、工作表有 `sheetProtection` 且 `enforced=false`；打开工作表保护后 COM `ProtectContents` 为真且无 Repair；解开后能改格；工作簿锁结构后 COM `ProtectStructure` 为真；缺字段得 `InvalidField`，带哈希样例 `hasPassword=true` 且 apply 得 `NotImplemented` |
| Xa1–Xa5 | ✅ | Excel 16.0：`xlsx-anchor.ps1`，5 PASS / 0 FAIL。twoCell 改 `to=G10` 无 Repair；oneCell 转到 absolute 后 `posX=914400`；再改 `posX=2000000` 无 Repair；转到 absolute 不给 pos、twoCell 给 `cx` 均得 `InvalidField` 且未写文件 |
| Xsg1–Xsg5 | ✅ | Excel 16.0：`xlsx-shapes-geom.ps1`，5 PASS / 0 FAIL。无名单时根与工作表有 `shapeCount=0`、`connectorCount=0`；`insertShape` 后 COM 能看见矩形且无 Repair；`insertConnector` 后名单 id 为 `sheet:…/cxn:…`；改锚点与删后计数为 0；缺 `from` 得 `InvalidField` |
| Xg1–Xg5 | ✅ | Excel 16.0：`xlsx-groups.ps1`，5 PASS / 0 FAIL。`groupShapes` 后 COM Type=6、子 id 仍在；`ungroup` 后组合解开；形状与连接线可编成一组；已组合再编、只给一个成员、`id=slide:1`、对未组合对象 `ungroup` 均拒写。无 Repair |
| Xol1–Xol5 | ✅ | Excel 16.0：`xlsx-ole.ps1`，5 PASS / 0 FAIL。create 默认 `oleObjectCount=0`；`insertOle` 后 COM Type=7、id 为 `sheet:Sheet1/ole:1025`、`progId=Worksheet`、dummy 不进 `shapes`、无 Repair；`exportMedia` 与 embeddings 字节一致；删除后计数为 0；缺 `from` / `slide:1` / 形状 id 均拒写 |
| Xsl1–Xsl5 | ✅ | Excel 16.0：`xlsx-slicers.ps1`，5 PASS / 0 FAIL。create 默认 `slicerCount=0`；表格 `insertSlicer` 后 COM `SlicerCaches.Count=1`、id 为 `sheet:Sheet1/slicer:1`、`sourceKind=table`、无 Repair；透视源 `sourceKind=pivot`；删除后计数为 0；缺 `from` / `slide:1` / 不存在的 `field` 均拒写 |
| Xcn1–Xcn5 | ✅ | Excel 16.0：`xlsx-connections.ps1`，5 PASS / 0 FAIL。create 默认 `connectionCount=0`；网页 `insertConnection` 后 COM 认 `WebDemo`、inspect `kind=web`、无 Repair；文本连接 `kind=text`、`sourceFile` 对得上；删除后计数为 0；缺 `type` / `id=slide:1` / `type=odbc` 均拒写 |
| Xel1–Xel5 | ✅ | Excel 16.0：`xlsx-external-link.ps1`，5 PASS / 0 FAIL。create 默认 `externalLinkCount=0`；`insertExternalLink` 并写 `=[1]Sheet1!A1` 后 COM `LinkSources` 含源文件、inspect `kind=book`、无 Repair；占用时删得 `NotImplemented`；清公式后再删计数为 0；缺 `path` / `id=slide:1` 拒写 |
| Xon1–Xon5 | ✅ | Excel 16.0：`xlsx-outline.ps1`，5 PASS / 0 FAIL。默认 `outline.summaryBelow=true`、名单空；分组行 2:4 与列 B:C 后 COM `OutlineLevel` 对得上、无 Repair；折叠后细节行隐藏；取消分组后级回落；缺字段 / `id=slide:1` 拒写 |
| Xsv1–Xsv5 | ✅ | Excel 16.0：`xlsx-custom-view.ps1`，5 PASS / 0 FAIL。默认 `splitRows`/`splitCols` 为 0、`customViewCount` 为 0；拆分后 COM `SplitRow=4` `SplitColumn=2`、无 Repair；`insertCustomView` 后 COM Count=1；删除后计数 0；冻结+拆分 / 空名 / `id=slide:1` 拒写 |
| C17–C21 | ✅ | Excel 16.0：`xlsx-create-drawing.ps1`，5 PASS / 0 FAIL。inspect 批注 `ref=A1` 正文含 `C17批注`；超链接 id 为 `sheet:Sheet1/link:1`；形状 `from=B4` `prst=ellipse`；SmartArt `from=D6`；打开无 Repair，COM 批注 / 超链接 / Shapes=4 / SmartArt=1 |
| C22–C24 | ✅ | Excel 16.0：`xlsx-create-chartsheet.ps1`，3 PASS / 0 FAIL。inspect `kind=chartsheet`、id 为 `sheet:Chart1/chart:2`；系列 1/2/3 与 `A10` 一带单元格一致；打开无 Repair，COM 图表工作表系列值对得上 |
| Xsr1–Xsr5 | ✅ | Excel 16.0：`xlsx-sort.ps1`，5 PASS / 0 FAIL。默认 `sortState` 为 `null`；`setSortState` 后 inspect `ref=A1:A3`、`descending=true`、A1 仍为 3；COM `Sort.SortFields` 降序键 `$A$1:$A$3`、无 Repair；`ref: null` 后为 `null`；缺 `ref` / 列越界得 `InvalidField` |
| 图表工作表 Xs1–Xs4 | ✅ | Excel 16.0：`xlsx-chartsheet.ps1`，4 PASS / 0 FAIL。`insertChartSheet` 后 `kind=chartsheet`、`chartCount=1`，无 Repair；数据 1/2/3 与 COM 一致；删除后表消失、数据格还在；对 `worksheet` 发 `deleteChartSheet` 得 `InvalidId` 且未写文件 |
| Xt1–Xt5 | ✅ | Excel 16.0：`xlsx-table.ps1`，5 PASS / 0 FAIL。create 默认 `tableCount=0`；建成 Sales 后 COM 认表；改范围到 `A1:C4` 无 Repair；删除后数据还在、不再有表；缺 `ref` 得 `InvalidField` 且未写文件 |


## create

用 `docs/examples/xlsx-create.json` 生成 `book.xlsx`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| C1 | Excel 打开 | 无修复提示、无「文件损坏」 |
| C3 | 工作表名 | 存在 `Sheet1` |
| C4 | A1 | 数字 `1` |
| C5 | A2 | 文本 `hello` |
| C6 | A3 | 布尔，显示为 `TRUE` / `真` |
| C7 | B1 | 公式栏为 `=SUM(A1)`，打开后值变成 `1`（允许先出现空白再重算） |
| C8 | 带图 oneCell | 用 `docs/examples/xlsx-create-pictures.json`（换成本机 PNG）生成；inspect `pictures: true` 时第一张 id 为 `sheet:Sheet1/pic:1`，`anchor` 为 `oneCell`，`from` 为 `B2` |
| C9 | 带图 twoCell | 同上，第二张 `anchor` 为 `twoCell`，`from` / `to` 对上 |
| C10 | Excel 打开带图产物 | 无修复；两张图都能看见 |
| C11 | 带图表 | 用 `docs/examples/xlsx-create-charts.json` 生成；inspect `charts: true` 时第一张 id 为 `sheet:Sheet1/chart:1`，`type` 为 `col`，`from` 为 `E2` |
| C12 | 图表数据 | 同上，系列值与 `dataFrom` 一带单元格一致 |
| C13 | Excel 打开带图表产物 | 无修复；图能看见 |
| C14 | 带页眉文字 | 用 `docs/examples/xlsx-create-headerfooter.json`（图换成本机 PNG）生成；`headerFooter: true` 时奇数页页眉 `text` 含 `C14页眉`，`hasHeaderFooter` 为 `true` |
| C15 | Excel 打开带页眉文字 | 无修复；PageSetup 中区页眉对得上 `C14页眉` |
| C16 | 带页眉图 | 同上；`headerFooter` 与 `pictures` 同时开时奇数页页眉图 id 为 `sheet:Sheet1/hf:odd:header/pic:2`；Excel 无修复 |
| C17 | 带批注 | 用 `docs/examples/xlsx-create-drawing.json` 生成；`comments: true` 时第一条 `ref` 为 `A1`，`text` 含 `C17批注` |
| C18 | 带超链接 | 同上；`hyperlinks: true` 时第一条 id 为 `sheet:Sheet1/link:1`，`url` 含 `example.com` |
| C19 | 带形状 | 同上；`shapes: true` 时第一条 `from` 为 `B4`，`prst` 为 `ellipse` |
| C20 | 带 SmartArt | 同上；`smartArt: true` 时第一条 `from` 为 `D6`，`layout` 为 list 族 uniqueId |
| C21 | Excel 打开带批注形状产物 | 无修复；COM 线程化批注、超链接与 Shapes（含 SmartArt）都能看见 |
| C22 | 带图表工作表 | 用 `docs/examples/xlsx-create-chartsheet.json` 生成；`charts: true` 时存在 `name=Chart1` 且 `kind=chartsheet`，`chartCount` 为 1，图表 id 为 `sheet:Chart1/chart:2` |
| C23 | 图表工作表数据 | 同上；Sheet1 `dataFrom` 一带单元格与系列值 1/2/3 一致 |
| C24 | Excel 打开带图表工作表产物 | 无修复；COM 图表工作表能看见，系列值对得上 |

## inspect

对上面的文件发 `docs/examples/xlsx-inspect.json`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| I1 | 表名单 | 含 `Sheet1`，id 为 `sheet:Sheet1` |
| I2 | 范围外 | `E1` 不出现在 `cells` |
| I3 | A1 | `type` 为 `number`，`value` 为 `1` |
| I4 | A2 | `type` 为 `text`，`value` 为 `hello` |
| I5 | B1 | `formula` 为 `=SUM(A1)` |
| I6 | 无 scope | 只发 path 时没有 `cells` 字段，仍有 `pictureCount`、`props` |

## apply

准备一份 Excel 亲手另存的样例（含两张表、若干样式、一张表里有公式）。仓库里那份是 `crates/ooxp/tests/fixtures/excel-source.xlsx`，由 Excel 16.0 `SaveAs` 写出，重做跑 `manual-check/office/export-excel-fixture.ps1`。对它执行 `docs/examples/xlsx-apply.json` 里的操作，必要时改 id 里的表名。库测试 `xlsx_excel_source.rs` 覆盖下面 A2–A7 的 XML 与 O2；A1 仍要 Office 打开。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| A1 | 打开 | Microsoft Excel 能打开，无修复提示 |
| A2 | 目标格 | `Sheet1!A1` 变成 `42` |
| A3 | 公式格 | `Sheet1!B1` 公式栏为 `=SUM(A1:A10)`，打开后会重算 |
| A4 | 清空 | `Sheet1!C1` 值为空，若原先有数字格式，格式还在 |
| A5 | 未改表 | 第二张表的格子与另存前一致 |
| A6 | 未改样式 | 未点名的格子字体、填充、列宽看起来不变 |
| A7 | 重算 | 依赖 A1 的其他公式在打开后得到新结果，而不是旧缓存 |

## 原样保存（opc 验收）

O1（用领域 JSON 打开再原样写出）第一期放弃。三个动作没有「只保存」，`apply` 不许空 `ops`。零脏写回在 OPC 层验收，见 `docs/opc.md`。

O2 仍按 apply 验收：没点名的表和样式看起来不变。

Office 另存 create 产物是打开冒烟，见 `manual-check/office/cases/opc-resave.ps1`，不顶替 O1。

## 工作表图

准备一份 Excel 亲手另存的样例：某张 `worksheet` 上至少各有一张 `oneCell` 与一张 `twoCell` 的 DrawingML 图。不要用库 `create` 产物当带图样例。另备一张本地 PNG 供换图、插图。手测脚本建议 `manual-check/office/cases/xlsx-pictures.ps1`。编号用 Xp，避免和 create 的 C 组、inspect 的 I 组重号。Excel 16.0 的 `Shapes.AddPicture` 只写 `twoCellAnchor`（`Placement` 只改 `editAs`），真 `oneCellAnchor` 由 ooxp 插入后再让 Excel `Save()`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xp1 | 计数与名单 | 无 `pictures` 时工作簿与目标表有 `pictureCount`、没有 `pictures`；`pictures: true` 时 id 为 `sheet:…/pic:…`，`anchor` / `from` 与工作表一致 |
| Xp2 | 换图 | `replacePicture` 后 Excel 无修复；锚点不变 |
| Xp3 | 插图 | `insertPicture` 往 `worksheet` 插 `oneCell`，Excel 能看见且无修复 |
| Xp4 | 删图 | `deletePicture` 后该图消失，无修复 |
| Xp5 | 尺寸 | 对 `oneCell` `setPictureSize` 后 COM 或目视尺寸对上（允许 EMU 换算误差） |
| Xp6 | 锚点 | `setPictureAnchor` 把 `from` 改到另一格后，图跟着走，无修复 |
| Xp7 | 导出 | `exportMedia` 写出的文件与 `/xl/media/…` 字节一致。不强制开 Excel |

## 工作表图表

准备一份 Excel 亲手另存的样例：某张 `worksheet` 上至少有一张 DrawingML 柱形图，系列公式指向本表单元格。不要用库 `create` 产物当带图样例。手测脚本建议 `manual-check/office/cases/xlsx-charts.ps1`。编号用 Xc。Excel 16.0 插入的图通常是 `twoCellAnchor`；真 `oneCell` 由 `insertChart` 或 `setChartAnchor` 写入后再让 Excel 打开。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xc1 | 计数与名单 | 无 `charts` 时工作簿与目标表有 `chartCount`、没有 `charts`；`charts: true` 时 id 为 `sheet:…/chart:…`，`type` / 系列值与工作表一致 |
| Xc2 | 改系列 | `setChartSeries` 后 Excel 无修复，单元格与图上的值都是新值 |
| Xc3 | 插图 | `insertChart` 往 `worksheet` 插图，Excel 能看见且无修复；`dataFrom` 一带有数据表 |
| Xc4 | 删图 | `deleteChart` 后该图消失，数据单元格还在，无修复 |
| Xc5 | 改类型 | `setChartType` 改为折线后按折线显示（允许样式重置），无修复 |
| Xc6 | 尺寸或锚点 | 对 `oneCell` `setChartSize` 或 `setChartAnchor` 后位置或尺寸对上，无修复 |
| Xc7 | 导出 | 对 xlsx 发 `exportChartData` 得到 `InvalidField`。不强制开 Excel |

## 图表剩余

手测脚本 `manual-check/office/cases/xlsx-charts-remain.ps1`。编号用 Xr。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xr1 | 趋势线 | ✅ `insertTrendline` 后 Excel 无修复，COM 能看见趋势线 |
| Xr2 | 次轴 | ✅ `setChartSeriesAxis` 后 `axis` 为 `secondary`，无修复 |
| Xr3 | 命名区域 | ✅ 系列 `c:f` 为定义名时 `setChartSeries` 写入对应格子，无修复 |
| Xr4 | 不连续 | ✅ 并集引用长度对得上时改数成功；三维引用仍 `NotImplemented` |

## 图表稀有类型

手测脚本 `manual-check/office/cases/xlsx-charts-rare.ps1`。编号用 Xq。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xq1 | 插入股价 | ✅ `insertChart` `type=stock` 三条系列后 Excel 无修复，`type` 为 `stock` |
| Xq2 | 插入曲面 | ✅ `insertChart` `type=surface` 三条系列后 Excel 无修复，`type` 为 `surface` |
| Xq3 | 插入复合饼 | ✅ `insertChart` `type=ofPie` 一条系列后 Excel 无修复；`ofPieType=bar` 为复合条 |
| Xq4 | 改类型 | ✅ 股价三条转 `col` 成功；复合饼转 `pie` 成功；无修复 |
| Xq5 | 仍拒绝 | ✅ 对股价 `addChartSeries` 得 `NotImplemented`；源无 chartex 时 `insertChart type=chartex` 得 `InvalidField`；两条系列的股价 `insertChart` 得 `InvalidField`；均未写坏文件 |

## 页眉页脚

准备一份 Excel 亲手另存的样例：某张 `worksheet` 上至少有奇数页页眉文字。不要用库 `create` 产物当唯一样例。Excel 16.0 页眉图通常是 VML `legacyDrawingHF`，不进树；DrawingML 页眉图由 `insertPicture` 往槽位写入后再让 Excel 打开。有 VML 页眉图时不测换 VML。手测脚本建议 `manual-check/office/cases/xlsx-headerfooter.ps1`。编号用 Xh。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xh1 | 摘要与原文 | 无 `headerFooter` 时表上有 `hasHeaderFooter`、没有 `headerFooters`；`headerFooter: true` 时六个槽位都在，奇数页页眉 `text` 与 Excel 另存原文一致 |
| Xh2 | 改字 | `setHeaderFooterText` 改奇数页页眉后 Excel 无修复，打印预览或 PageSetup 里是新字 |
| Xh3 | 工作表图不混 | 源文件若另有工作表图，`pictures: true` 且不开 `headerFooter` 时名单不含 `hf:` |
| Xh4 | 插页眉图 | 对 `sheet:…/hf:odd:header` 发 `insertPicture` 后 Excel 无修复（允许页眉图在打印预览里才看见） |
| Xh5 | 换页眉图 | 若 Xh4 写出了 DrawingML 页眉图，则 `replacePicture` 后无修复；源文件只有 VML 页眉图时本条记跳过，不测换 VML |

## 图表工作表

准备一份 Excel 亲手另存、至少有一张 `worksheet` 的工作簿；手测里再用 `insertChartSheet` 加一张。源文件不要用库 `create` 产物当唯一样例。脚本建议 `manual-check/office/cases/xlsx-chartsheet.ps1`。编号用 Xs。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xs1 | 插入 | `insertChartSheet` 后表名单多一张 `kind: chartsheet`，`chartCount` 为 1，Excel 无修复 |
| Xs2 | 数据 | 点名的普通表 `dataFrom` 一带有数据表，图上的值对得上 |
| Xs3 | 删除 | `deleteChartSheet` 后该表消失，数据单元格还在，无修复 |
| Xs4 | 拒普通表 | 对 `worksheet` 发 `deleteChartSheet` 得到 `InvalidId`。不强制开 Excel |

## 文档属性

与 docx 同一套字段。脚本 `manual-check/office/cases/xlsx-docprops.ps1`。编号用 Xd（不要跟页眉图的 Xp 组混）。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xd1 | inspect | 无 scope 时根上有 `props`；create 产物 `creator` 为 `ooxp` |
| Xd2 | 改标题 | `setProps` 只改 `title` 后 Excel 文件属性标题是新字；未点名的公司名还在；无 Repair |
| Xd3 | 自定义 | `setCustomProp` 后树上 id 为 `prop:…`；Excel 自定义属性能看见 |
| Xd4 | 删除 | `deleteCustomProp` 后名单没有该项 |
| Xd5 | 拒错 id | 对单元格 id 发 `setProps` 得 `InvalidId`。不强制开 Excel |

## 工作表管理

准备一份 Excel 亲手另存、至少两张可见 `worksheet` 的工作簿。不要用库 `create` 产物当唯一样例。脚本 `manual-check/office/cases/xlsx-sheets.ps1`。编号用 Xm。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xm1 | 隐藏字段 | 无 scope 时每张表有 `hidden`；可见表为 `false` |
| Xm2 | 插入 | `insertSheet` 后名单多一张空 `worksheet`，Excel 无修复 |
| Xm3 | 改名 | `renameSheet` 后旧 id 失效、新 id 能 `setCell`，Excel 无修复 |
| Xm4 | 重排 | `moveSheet` 且 `after` 省略后该表排第一，Excel 无修复 |
| Xm5 | 隐藏 | `setSheetHidden` 后 COM 对应表 Hidden，无修复 |
| Xm6 | 删除 | `deleteSheet` 后该表消失，无修复 |
| Xm7 | 最后一张可见表 | 隐藏或删除最后一张可见表得 `InvalidField`，未写文件。不强制开 Excel |

## 单元格样式

准备一份 Excel 亲手另存或库 create 的工作簿，至少 A1 有数字、B1 有独立样式或未点名格子。脚本 `manual-check/office/cases/xlsx-cellstyle.ps1`。编号用 Xs。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xs1 | inspect | 范围内格子有 `numFmt`；create 默认格为 `General` |
| Xs2 | 数字格式与填充 | 给 A1 `setCellStyle` 写 `0.00` 与黄底后，Excel 无修复，`NumberFormat` 为 `0.00`，填充为黄，值还在 |
| Xs3 | 未点名 | B1 未出现在 ops 里，打开后样式与打补丁前观感一致 |
| Xs4 | 与 setCell 同批 | 同一 `ops` 先 `setCell` 再 `setCellStyle`（或反过来），值与格式都在，无修复 |
| Xs5 | 缺字段 | 只给 `op` 与 `id` 得 `InvalidField`，未写文件。不强制开 Excel |

## 列宽行高与合并

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-layout.ps1`。编号用 Xl。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xl1 | inspect | 有单元格范围时表上有 `cols` / `rows` / `merges`；create 默认均为空数组 |
| Xl2 | 列宽 | `setColWidth` 把 A 列写成 20 后，Excel 无修复，COM `ColumnWidth` 约为 20 |
| Xl3 | 行高 | `setRowHeight` 把第 1 行写成 30 磅后，COM `RowHeight` 约为 30，无修复 |
| Xl4 | 合并与取消 | `mergeRange` `A1:B2` 后左上角值还在、其余格空、COM 认合并；再 `unmergeRange` 后不再合并，无修复 |
| Xl5 | 缺字段 | 只给 `op` 与 `id` 的 `setColWidth` 得 `InvalidField`，未写文件。不强制开 Excel |

## 公式结构

用库 create 产物再 apply 写出共享组与数组。脚本 `manual-check/office/cases/xlsx-formula.ps1`。编号用 Xf。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xf1 | inspect | 共享主格有 `formula`、`shared`、`sharedRef`；从属格 `formula` 为展开后的 A1，且 `shared` 为 `true` |
| Xf2 | 改主格公式 | 保住共享组，Excel 无修复，从属格打开后重算正确 |
| Xf3 | 拆组 | `shared: false` 后各格都是独立公式，Excel 无修复 |
| Xf4 | 数组公式 | `arrayRef` 写入后 Excel 认数组、无修复 |
| Xf5 | 数组从属格 | 对非主格 `setCell` 得 `NotImplemented`，未写文件。不强制开 Excel |

## 共享公式改主格

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-formula-master.ps1`。编号用 Xsm。不要占用 Xf。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xsm1 | 向上迁主格 | 把 `B2:B3` 迁到 `B1:B3` 后，inspect 上 `B1` 有 `sharedRef`，`B2` / `B3` 是从属格且公式为展开后的 A1 |
| Xsm2 | Excel 重算 | 同一份文件 Excel 无修复，COM `B1`/`B2`/`B3` 公式与打开后的值正确 |
| Xsm3 | 向下收主格 | 把 `B1:B3` 收到 `B2:B3` 后，`B2` 是主格，`B1` 是独立公式；Excel 无修复 |
| Xsm4 | 两组相交 | `sharedRef` 同时盖住两组已有共享组得 `NotImplemented`，未写文件。不强制开 Excel |
| Xsm5 | 从属格缺范围 | 对从属格只给 `shared: true` 得 `InvalidField`，未写文件。不强制开 Excel |

## 数据验证

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-validation.ps1`。编号用 Xv。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xv1 | inspect | 有单元格范围时表上有 `dataValidationCount` 与 `dataValidations`；create 默认为 `0` 与空数组 |
| Xv2 | 序列下拉 | `insertDataValidation` 给 A1 写 `type=list`、`formula1=1,2,3` 后，Excel 无修复，该格下拉能看见 1、2、3 |
| Xv3 | 多区域 | `sqref` 为 `A1 C1` 时 inspect 的 `sqref` 为 `["A1","C1"]`，两条都能看见下拉 |
| Xv4 | 删除 | `deleteDataValidation` 后该格不再有校验，Excel 无修复 |
| Xv5 | 缺字段 | 只给 `op` 与 `id` 的 `insertDataValidation` 得 `InvalidField`，未写文件。不强制开 Excel |

## 条件格式

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-conditional-format.ps1`。编号用 Xn。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xn1 | inspect | 有单元格范围时表上有 `conditionalFormatCount` 与 `conditionalFormats`；create 默认为 `0` 与空数组 |
| Xn2 | 单元格值填绿 | `insertConditionalFormat` 给 `A1:A10` 写 `type=cellIs`、`greaterThan`、`formulas=["0"]`、绿底后，Excel 无修复；A1 为 1 时能看见绿色条件格式 |
| Xn3 | 色阶 | 写入两色 `colorScale` 后 Excel 认色阶、无修复 |
| Xn4 | 删除 | `deleteConditionalFormat` 后该范围不再有条件格式，Excel 无修复 |
| Xn5 | 缺字段 | 只给 `op` 与 `id` 的 `insertConditionalFormat` 得 `InvalidField`，未写文件。不强制开 Excel |

## 工作表表格

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-table.ps1`。编号用 Xt。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xt1 | inspect | 有单元格范围时表上有 `tableCount` 与 `tables`；create 默认为 `0` 与空数组 |
| Xt2 | 建成表 | `insertSheetTable` 给 `A1:C3` 写列名后，Excel 无修复；COM `ListObjects` 能看见表，列名对 |
| Xt3 | 改范围 | `setSheetTable` 把 `ref` 改到 `A1:C4` 后 Excel 仍认该表、无修复 |
| Xt4 | 删除 | `deleteSheetTable` 后数据还在、不再有表，Excel 无修复 |
| Xt5 | 缺字段 | 只给 `op` 与 `id` 的 `insertSheetTable` 得 `InvalidField`，未写文件。不强制开 Excel |

## 数据透视

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-pivot.ps1`。编号用 Xpt（与工作表图的 Xp 分开）。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xpt1 | inspect | 有单元格范围时表上有 `pivotCount` 与 `pivots`；create 默认为 `0` 与空数组 |
| Xpt2 | 建成表 | `insertPivotTable` 源 `A1:C4`、location `E3`、一行字段加一个值字段后，Excel 无修复；COM `PivotTables.Count>=1` |
| Xpt3 | 改源 | `setPivotTable` 把源改到含新数据行的范围后，Excel 仍认该表、无修复 |
| Xpt4 | 删除 | `deletePivotTable` 后 `pivotCount=0`、COM Count=0、源数据还在、无修复 |
| Xpt5 | 缺字段 | 只给 `op` 与 `id` 的 `insertPivotTable` 得 `InvalidField`，未写文件。不强制开 Excel |

## 命名区域

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-defined-name.ps1`。编号用 Xnm。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xnm1 | inspect | 根上有 `definedNameCount` 与 `definedNames`；create 默认为 `0` 与空数组 |
| Xnm2 | 建成名 | `insertDefinedName` 名 `Sales`、公式 `Sheet1!$A$1:$A$10` 后，Excel 无修复；COM `Names` 能看见 `Sales` |
| Xnm3 | 改公式 | `setDefinedName` 把公式改到 `Sheet1!$A$1:$A$20` 后 Excel 仍认该名、无修复 |
| Xnm4 | 删除 | `deleteDefinedName` 后 `definedNameCount=0`、COM 找不到 `Sales`、无修复 |
| Xnm5 | 缺字段 | 只给 `op` 与 `id` 的 `insertDefinedName` 得 `InvalidField`，未写文件。不强制开 Excel |

## 打印设置

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-print.ps1`。编号用 Xps。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xps1 | inspect | 每张 `worksheet` 始终有 `print`；create 默认为左边距 0.7、`orientation` 为 `portrait`、`printArea` 为 `null` |
| Xps2 | 边距与纸向 | 先写奇数页页眉，再 `setPrintSetup` 左边距 1 英寸、`landscape` 后，Excel 无修复；COM `LeftMargin` 与 `InchesToPoints(1)` 一致，`Orientation` 为横向；页眉字还在 |
| Xps3 | 打印区域 | `printArea` 写成 `A1:A10` 后 COM `PrintArea` 能对上，无修复 |
| Xps4 | 网格线 | `gridLines: true` 后 COM `PrintGridlines` 为真，无修复 |
| Xps5 | 缺字段 | 只给 `op` 与 `id` 的 `setPrintSetup` 得 `InvalidField`，未写文件。不强制开 Excel |

## 视图与筛选

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-view.ps1`。编号用 Xvw。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xvw1 | inspect | 每张 `worksheet` 始终有 `view` 与 `autoFilter`；create 默认为 `zoom` 100、冻结 0、`activeCell` 为 `A1`、`autoFilter` 为 `null` |
| Xvw2 | 冻结与缩放 | `setSheetView` 冻结首行、`zoom` 80 后，Excel 无修复；COM `FreezePanes` 为真、`SplitRow` 为 1、`Zoom` 为 80 |
| Xvw3 | 工作表筛选 | `setAutoFilter` 把 `ref` 写成 `A1:A10` 后 COM `AutoFilter.Range` 能对上，无修复 |
| Xvw4 | 解冻 | `frozenRows` 与 `frozenCols` 都为 0 后 COM `FreezePanes` 为假，无修复 |
| Xvw5 | 缺字段 | 只给 `op` 与 `id` 的 `setSheetView` 得 `InvalidField`，未写文件。不强制开 Excel |

## 排序

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-sort.ps1`。编号用 **Xsr**。不要和 Xvw 撞。不重排格子，不算排序引擎。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xsr1 | inspect | 每张 `worksheet` 始终有 `sortState`；create 默认为 `null` |
| Xsr2 | 写入 | `setSortState` `ref=A1:A3`、`columns[0].descending=true` 后 inspect 对上；Excel 无修复；A1 仍为 3（未重排） |
| Xsr3 | COM | COM `Sort.SortFields` 能看见对应列，降序；无修复 |
| Xsr4 | 删除 | `ref: null` 后 `sortState` 为 `null`，无修复 |
| Xsr5 | 拒写 | 缺 `ref`、列范围越出 `ref` 得 `InvalidField`，未写文件。不强制开 Excel |

## 批注

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-comments.ps1`。编号用 Xcm。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xcm1 | inspect | 每张 `worksheet` 始终有 `commentCount`；create 默认为 `0`；给了 `scope.comments` 时名单为空数组，不依赖单元格切片 |
| Xcm2 | 插入 | `insertComment` 写到 `A1` 后 Excel 无修复；COM `CommentThreaded` 能看见正文 |
| Xcm3 | 回复 | `replyComment` 后 COM `Replies` 能看见；inspect 嵌在 `replies` |
| Xcm4 | 删除 | `deleteComment` 后 `commentCount=0`、COM 没有线程批注、无修复 |
| Xcm5 | 缺字段 | 只给 `op` 与 `id` 的 `setCommentMeta` 得 `InvalidField`，未写文件。不强制开 Excel |

## 形状与 SmartArt

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-shapes.ps1`。编号用 Xsa。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xsa1 | inspect | 每张 `worksheet` 始终有 `textBoxCount` 与 `smartArtCount`；create 默认为 `0`；给了 `scope.textBoxes` / `scope.smartArt` 时名单为空数组，不依赖单元格切片 |
| Xsa2 | 文本框 | `insertTextBox` 写到 `B2` 后 Excel 无修复；COM `Shapes` 能看见文本框；`setShapeText` 后可见字对得上 |
| Xsa3 | SmartArt | `insertSmartArt` `layout=list` 后 Excel 无修复；COM 能看见 SmartArt；inspect `layout` 对得上 |
| Xsa4 | 删除 | `deleteTextBox` / `deleteSmartArt` 后对应计数为 0、无修复 |
| Xsa5 | 缺字段 | 只给 `op` 与 `id` 的 `insertTextBox`（缺 `from`）得 `InvalidField`，未写文件。不强制开 Excel |

## 无 txBox 的形状与连接线

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-shapes-geom.ps1`。编号用 Xsg。不要和 Xsa 撞。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xsg1 | inspect | 每张 `worksheet` 始终有 `shapeCount` 与 `connectorCount`；create 默认为 `0`；给了 `scope.shapes` / `scope.connectors` 时名单为空数组，不依赖单元格切片 |
| Xsg2 | 矩形 | `insertShape` 写到 `B2` 后 Excel 无修复；inspect `prst` 为 `rect`，id 为 `sheet:…/shape:…`；COM `Shapes` 能看见；无 `txBox` |
| Xsg3 | 连接线 | `insertConnector` 后 inspect `connectors` 名单 id 为 `sheet:…/cxn:…`，`prst` 为 `straightConnector1`；COM 能看见连接线；无修复 |
| Xsg4 | 改锚点与删 | `setTextBoxSize` / `setTextBoxAnchor` 认形状 id；`setShapeText` 后可见字对得上；`deleteShape` / `deleteConnector` 后对应计数为 0、无修复 |
| Xsg5 | 拒写 | 只给 `op` 与 `id` 的 `insertShape`（缺 `from`）得 `InvalidField`；对 `/sp:` 发 `deleteShape`、对 `/cxn:` 发 `setShapeText` 得 `InvalidId`；均未写文件。不强制开 Excel |

## 组合

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-groups.ps1`。编号用 **Xg**。不要和 Xsg、Xsa 撞。不算按列宽反推已有 `twoCell` 图的显示尺寸。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xg1 | 组合 | `insertShape` 两个矩形后 `groupShapes`；Excel 无修复；COM Type=6；inspect 子形状 id 仍在 |
| Xg2 | 解开 | `ungroup` 后组合解开，子 id 还在；无 Repair |
| Xg3 | 形状与连接线 | 形状与连接线编成一组后 COM 认组合；inspect 两边名单都在；无 Repair |
| Xg4 | 再编拒绝 | 已组合的成员再 `groupShapes` 得 `InvalidField`，未写文件。不强制开 Excel |
| Xg5 | 拒写 | 只给一个成员、`id` 为 `slide:1`、对未组合对象发 `ungroup` 得 `InvalidField` 或 `InvalidId`；均未写文件。不强制开 Excel |

## OLE

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-ole.ps1`。编号用 **Xol**。不要和 Xg、Xsg、Xsa、Xa、Xs、Xq、O 撞。不激活 OLE 服务器。不把嵌入字节或预览图塞进 inspect 树。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xol1 | inspect | 每张 `worksheet` 始终有 `oleObjectCount`；create 默认为 `0`；给了 `scope.oleObjects` 时名单为空数组，不依赖单元格切片 |
| Xol2 | 插入 | `insertOle` 嵌入一份 xlsx 后 Excel 无修复；COM Type=7；inspect id 为 `sheet:Sheet1/ole:1025`，`progId` 为 `Worksheet`，dummy 不进 `shapes` |
| Xol3 | 导出 | `exportMedia` 字节与 `/xl/embeddings/…` 部件一致 |
| Xol4 | 删除 | `deleteOle` 后 `oleObjectCount=0`、无 Repair |
| Xol5 | 拒写 | 缺 `from`、`id` 为 `slide:1`、对形状 id 发 `insertOle` 得 `InvalidField` 或 `InvalidId`；均未写文件。不强制开 Excel |

## 切片器

用库 create 产物再 apply 出表格，再插切片器。脚本 `manual-check/office/cases/xlsx-slicers.ps1`。编号用 **Xsl**。不要和 Xs、Xsa、Xsk、Xol 撞。不执行筛选，不刷新透视。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xsl1 | inspect | 每张 `worksheet` 始终有 `slicerCount`；create 默认为 `0`；给了 `scope.slicers` 时名单为空数组，不依赖单元格切片 |
| Xsl2 | 插入 | 对表格 `insertSlicer` 后 Excel 无修复；COM 能看见切片器；inspect id 为 `sheet:Sheet1/slicer:{cNvPrId}`，`sourceKind=table` |
| Xsl3 | 透视源 | 对透视表 `insertSlicer` 后无修复；inspect `sourceKind=pivot` |
| Xsl4 | 删除 | `deleteSlicer` 后 `slicerCount=0`、无 Repair |
| Xsl5 | 拒写 | 缺 `from`、`id` 为 `slide:1`、对不存在的 `field` 得 `InvalidField` 或 `InvalidId`；均未写文件。不强制开 Excel |

## 外部数据连接

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-connections.ps1`。编号用 **Xcn**。不要和 Xc、Xcm 撞。不刷新连接。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xcn1 | inspect | 工作簿始终有 `connectionCount`；create 默认为 `0`；给了 `scope.connections` 时名单为空数组，不依赖单元格切片 |
| Xcn2 | 网页 | `insertConnection` `type=web` 后 Excel 无修复；COM `Connections` 能看见；inspect `kind=web`、`url` 对得上 |
| Xcn3 | 文本 | `insertConnection` `type=text` 后无修复；inspect `kind=text`、`sourceFile` 对得上 |
| Xcn4 | 删除 | `deleteConnection` 后 `connectionCount=0`、无 Repair |
| Xcn5 | 拒写 | 缺 `type`、`id` 为 `slide:1`、`type` 不是 `web`/`text` 得 `InvalidField` 或 `InvalidId`；均未写文件。不强制开 Excel |

## 外部引用

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-external-link.ps1`。编号用 **Xel**。不要和 Xl、Xcn 撞。不打开目标文件求值。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xel1 | inspect | 工作簿始终有 `externalLinkCount`；create 默认为 `0`；给了 `scope.externalLinks` 时名单为空数组，不依赖单元格切片 |
| Xel2 | 插入 | `insertExternalLink` 并写 `=[1]Sheet1!A1` 后 Excel 无修复；COM `LinkSources` 或 Formula 含源文件名；inspect `kind=book`、`path` 含源文件名。Excel 16.0 对未引用的 `externalReferences` 不填 `LinkSources` |
| Xel3 | 公式占用 | 写了 `=[1]Sheet1!A1` 后再 `deleteExternalLink` 得 `NotImplemented`，未写文件。COM Formula 含 `Sheet1` 与 `A1`（Excel 可能写成 `'…[file.xlsx]Sheet1'!A1`） |
| Xel4 | 删除 | 清掉公式后再删，`externalLinkCount=0`、无 Repair |
| Xel5 | 拒写 | 缺 `path`、`id` 为 `slide:1` 得 `InvalidField` 或 `InvalidId`；均未写文件。不强制开 Excel |

## 分组 / 分级显示

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-outline.ps1`。编号用 **Xon**。不要和 Xol、Xg 撞。不实现放映引擎。不算筛选。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xon1 | inspect | 每张 `worksheet` 始终有 `outline`；create 默认为 `summaryBelow` / `summaryRight` 为 `true`，`rows` / `cols` 为空数组。不依赖单元格切片 |
| Xon2 | 分组 | `groupRows` `2:4` 与 `groupCols` `B:C` 后 Excel 无修复；COM 行 2–4 与列 B–C 的 `OutlineLevel` 为 2；inspect `level` 为 1 |
| Xon3 | 折叠 | `setOutline` `collapseRows=2:4` `collapsed=true` 后无修复；COM 行 2–4 `Hidden` 为真 |
| Xon4 | 取消 | `ungroupRows` / `ungroupCols` 后 inspect 名单空，无 Repair |
| Xon5 | 拒写 | 只给 `op` 与 `id` 的 `setOutline`、`id` 为 `slide:1` 的 `groupRows` 得 `InvalidField` 或 `InvalidId`；均未写文件。不强制开 Excel |

## 拆分窗格与自定义视图

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-custom-view.ps1`。编号用 **Xsv**。不要和 Xvw 撞。不算放映引擎。不按自定义视图恢复隐藏行 / 列。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xsv1 | inspect | 每张 `worksheet` 的 `view` 始终有 `splitRows` / `splitCols`，create 默认为 0；工作簿始终有 `customViewCount`，create 默认为 0。给了 `scope.customViews` 时名单为空数组，不依赖单元格切片 |
| Xsv2 | 拆分 | `setSheetView` `splitRows=4` `splitCols=2` 后 Excel 无修复；COM `Split` 为真、`SplitRow` 为 4、`SplitColumn` 为 2、`FreezePanes` 为假 |
| Xsv3 | 插入 | `insertCustomView` `name=DemoView` 后无修复；COM `CustomViews.Count` 为 1；inspect 名单 `name` 对得上 |
| Xsv4 | 删除 | `deleteCustomView` 后 `customViewCount=0`、无 Repair |
| Xsv5 | 拒写 | 冻结与拆分同条 op、空 `name`、`id` 为 `slide:1` 的 `insertCustomView` 得 `InvalidField` 或 `InvalidId`；均未写文件。不强制开 Excel |

## 超链接

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-hyperlinks.ps1`。编号用 Xhl。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xhl1 | inspect | 每张 `worksheet` 始终有 `hyperlinkCount`；create 默认为 `0`；给了 `scope.hyperlinks` 时名单为空数组，不依赖单元格切片 |
| Xhl2 | 插入 | `insertHyperlink` 写到 `A1` 后 Excel 无修复；COM `Hyperlinks` 能看见 `Address`；inspect `url` 对得上 |
| Xhl3 | 改地址 | `setHyperlink` 改 `url` 后 COM `Address` 对得上、无修复 |
| Xhl4 | 删除 | `deleteHyperlink` 后 `hyperlinkCount=0`、单元格文字还在、无修复 |
| Xhl5 | 缺字段 | 只给 `op` 与 `id` 的 `insertHyperlink`（缺 `ref`）得 `InvalidField`，未写文件。不强制开 Excel |

## 迷你图

用库 create 产物再 apply。脚本 `manual-check/office/cases/xlsx-sparklines.ps1`。编号用 Xsk。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xsk1 | inspect | 每张 `worksheet` 始终有 `sparklineCount`；create 默认为 `0`；给了 `scope.sparklines` 时名单为空数组，不依赖单元格切片 |
| Xsk2 | 插入 | `insertSparkline` 写到 `A1`、数据 `B1:F1` 后 Excel 无修复；COM `SparklineGroups` 能看见；inspect `type=line`、`formula` 含 `B1:F1` |
| Xsk3 | 改范围 | `setSparkline` 改 `formula` 后 COM `SourceData` 对得上、无修复 |
| Xsk4 | 删除 | `deleteSparkline` 后 `sparklineCount=0`、无修复 |
| Xsk5 | 缺字段 | 只给 `op` 与 `id` 的 `insertSparkline`（缺 `ref`）得 `InvalidField`，未写文件。不强制开 Excel |

## 保护

用库 create 产物再 apply。带哈希样例由脚本在 ZIP 里写入 `sheetProtection` 属性，不靠 Excel 猜密码。脚本 `manual-check/office/cases/xlsx-protect.ps1`。编号用 Xpr。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xpr1 | inspect | 无 scope 时工作簿有 `workbookProtection`，每张 `worksheet` 有 `sheetProtection`；create 默认为 `enforced=false`、`hasPassword=false`、`lockStructure=false` |
| Xpr2 | 保护工作表 | `setProtection` `enforced: true` 后 Excel 无修复；COM `ProtectContents` 为真 |
| Xpr3 | 解开 | `enforced: false` 后 COM `ProtectContents` 为假，单元格能写入，无修复 |
| Xpr4 | 工作簿结构 | `lockStructure: true` 后 COM `ProtectStructure` 为真、无修复 |
| Xpr5 | 缺字段与哈希 | 只给 `op` 与 `id` 得 `InvalidField`，未写文件；带 `hashValue` 的样例 inspect `hasPassword=true`，再 apply 得 `NotImplemented`。不强制为缺字段开 Excel |

## 锚点剩余

用库 create 带图产物再 apply。不算列宽行高。脚本 `manual-check/office/cases/xlsx-anchor.ps1`。编号用 Xa。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Xa1 | twoCell 改 `to` | 对 `twoCell` 图发 `setPictureSize` 且给 `to` 后，inspect `to` 对得上；Excel 无 Repair |
| Xa2 | 转到 absolute | `oneCell` 图 `setPictureAnchor` `anchor=absolute` 且给齐 `posX` / `posY` 后，inspect `anchor=absolute`；Excel 无 Repair |
| Xa3 | 改绝对坐标 | 已是 `absolute` 时只改 `posX`，inspect 对得上；Excel 无 Repair |
| Xa4 | 互转缺字段 | 转到 `absolute` 却不给 `posX` / `posY` 得 `InvalidField`，未写文件。不强制开 Excel |
| Xa5 | twoCell 拒 `cx` | 对 `twoCell` 图发 `setPictureSize` 且给 `cx` 得 `InvalidField`，未写文件。不强制开 Excel |
