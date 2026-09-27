# xlsx 人工检查清单

代码阶段的单元测试覆盖 JSON 解析和 XML 不变量。下面这些必须用 Microsoft Excel 打开一次，不能只靠测试绿。验收只认官方 Office，不再要求 WPS。

检查前用 create 或 apply 在本机生成文件，不要用改过扩展名的 zip 目录冒充。

## 本机结果（2026-09-26）

Microsoft Excel 16.0 已打开，用户确认通过。本机若装了 WPS，`Excel.Application` 会被抢走，测试仍直接启动 `EXCEL.EXE` 再挂 COM。

C、I、A、O、Xp、Xc、Xh 这几组现在由 `manual-check/office/cases/xlsx-*.ps1` 自动判定，跑 `pwsh -File manual-check\office\run.ps1 -Case 'xlsx-*'` 即可复现，逐条结果落在 `manual-check/office/out/office-results.csv`。2026-09-27 补跑 `xlsx-pictures.ps1`，Xp1–Xp7 通过。同日补跑 `xlsx-charts.ps1`，Xc1–Xc7 通过。同日补跑 `xlsx-create-pictures.ps1`，C8–C10 通过。同日补跑 `xlsx-create-charts.ps1`，C11–C13 通过。同日补跑 `xlsx-headerfooter.ps1`，Xh1–Xh5 通过。同日补跑 `xlsx-docprops.ps1`，Xd1–Xd5 通过。同日补跑 `xlsx-charts-remain.ps1`，Xr1–Xr4 通过。同日补跑 `xlsx-sheets.ps1`，Xm1–Xm7 通过。同日补跑 `xlsx-cellstyle.ps1`，Xs1–Xs5 通过。同日补跑 `xlsx-layout.ps1`，Xl1–Xl5 通过。同日补跑 `xlsx-formula.ps1`，Xf1–Xf5 通过。2026-09-28 补跑 `xlsx-validation.ps1`，Xv1–Xv5 通过。同日补跑 `xlsx-conditional-format.ps1`，Xn1–Xn5 通过。同日补跑 `xlsx-pivot.ps1`，Xpt1–Xpt5 通过。同日补跑 `xlsx-defined-name.ps1`，Xnm1–Xnm5 通过。同日补跑 `xlsx-print.ps1`，Xps1–Xps5 通过。

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
| Xh1–Xh5 | ✅ | Excel 16.0：无 `headerFooter` 时有 `hasHeaderFooter`、没有名单；展开六槽，奇数页页眉原文含 `Xh页眉`；`setHeaderFooterText` 后 PageSetup 为新字且无 Repair；只开 `pictures` 时名单不含 `hf:`；往 `hf:odd:header` 插图 id 为 `…/pic:2`，换图后无 Repair |
| Xd1–Xd5 | ✅ | Excel 16.0：`xlsx-docprops.ps1`，5 PASS / 0 FAIL。根上 `props`、`creator=ooxp`；只改标题后面板标题是新字、公司名还在；自定义 `prop:部门`；删后名单空；单元格 id 发 `setProps` 为 `InvalidId`。无 Repair |
| Xr1–Xr4 | ✅ | Excel 16.0：`xlsx-charts-remain.ps1`，4 PASS / 0 FAIL。趋势线 COM Count=1；次轴 `AxisGroup=2`；命名区域改数 90/80/70 且 `c:f` 仍为 `ChartVals`；并集改数成功、三维引用 `NotImplemented`。无 Repair |
| Xm1–Xm7 | ✅ | Excel 16.0：`xlsx-sheets.ps1`，7 PASS / 0 FAIL。无 scope 时每张表有 `hidden`；插入 Sheet3、改名附录、挪到最前、隐藏 Sheet2、删除 Sheet2 均无 Repair；隐藏或删除最后一张可见表得 `InvalidField` 且未写文件 |
| Xs1–Xs5 | ✅ | Excel 16.0：`xlsx-cellstyle.ps1`，5 PASS / 0 FAIL。默认 `numFmt=General`；A1 写成 `0.00` 与黄底后值还在、无 Repair；B1 仍是通用格式且无黄底；缺字段得 `InvalidField` 且未写文件 |
| Xl1–Xl5 | ✅ | Excel 16.0：`xlsx-layout.ps1`，5 PASS / 0 FAIL。列宽 20、行高 30、合并 A1:B2 对得上；取消合并无修复；缺字段得 `InvalidField` 且未写文件 |
| Xf1–Xf5 | ✅ | Excel 16.0：`xlsx-formula.ps1`，5 PASS / 0 FAIL。共享主格改公式保住整组；拆组后各格独立；数组公式 Excel 认；非主格 `NotImplemented` |
| Xv1–Xv5 | ✅ | Excel 16.0：`xlsx-validation.ps1`，5 PASS / 0 FAIL。A1 序列下拉能看见 1、2、3；多区域 `A1`/`C1`；删除后无列表校验、无 Repair；缺字段得 `InvalidField` 且未写文件 |
| Xn1–Xn5 | ✅ | Excel 16.0：`xlsx-conditional-format.ps1`，5 PASS / 0 FAIL。A1:A10 大于 0 填绿；两色色阶；删除后无条件格式、无 Repair；缺字段得 `InvalidField` 且未写文件 |
| Xpt1–Xpt5 | ✅ | Excel 16.0：`xlsx-pivot.ps1`，5 PASS / 0 FAIL。建成 SalesPivot 无 Repair；改源到 A1:C5 后仍认表；删除后源数据还在；缺字段得 `InvalidField` 且未写文件 |
| Xnm1–Xnm5 | ✅ | Excel 16.0：`xlsx-defined-name.ps1`，5 PASS / 0 FAIL。建成 Sales 无 Repair；COM Names 能看见；改公式到 A1:A20 后仍认该名；删除后计数为 0；缺字段得 `InvalidField` 且未写文件 |
| Xps1–Xps5 | ✅ | Excel 16.0：`xlsx-print.ps1`，5 PASS / 0 FAIL。默认 `print` 左边距 0.7、竖向、无打印区域；左边距 1 英寸与横向后 COM 对得上且页眉还在；打印区域与网格线 COM 认；缺字段得 `InvalidField` 且未写文件。无 Repair |


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
