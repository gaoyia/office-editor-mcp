# xlsx 人工检查清单

代码阶段的单元测试覆盖 JSON 解析和 XML 不变量。下面这些必须用 Microsoft Excel 打开一次，不能只靠测试绿。验收只认官方 Office，不再要求 WPS。

检查前用 create 或 apply 在本机生成文件，不要用改过扩展名的 zip 目录冒充。

## 本机结果（2026-09-26）

Microsoft Excel 16.0 已打开，用户确认通过。本机若装了 WPS，`Excel.Application` 会被抢走，测试仍直接启动 `EXCEL.EXE` 再挂 COM。

C、I、A、O、Xp、Xc 这几组现在由 `manual-check/office/cases/xlsx-*.ps1` 自动判定，跑 `pwsh -File manual-check\office\run.ps1 -Case 'xlsx-*'` 即可复现，逐条结果落在 `manual-check/office/out/office-results.csv`。2026-09-27 补跑 `xlsx-pictures.ps1`，Xp1–Xp7 通过。同日补跑 `xlsx-charts.ps1`，Xc1–Xc7 通过。同日补跑 `xlsx-create-pictures.ps1`，C8–C10 通过。同日补跑 `xlsx-create-charts.ps1`，C11–C13 通过。

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
| I6 | 无 scope | 只发 path 时没有 `cells` 字段，仍有 `pictureCount` |

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
