# xlsx 人工检查清单

代码阶段的单元测试覆盖 JSON 解析和 XML 不变量。下面这些必须用 Excel 和 WPS 各打开一次，不能只靠测试绿。

检查前用 create 或 apply 在本机生成文件，不要用改过扩展名的 zip 目录冒充。

## 本机结果（2026-09-26）

WPS 表格与 Microsoft Excel 16.0 均已打开，用户确认通过。本机 `Excel.Application` 仍被 WPS 抢走，测试是直接启动 `EXCEL.EXE` 再挂 COM。

C、I、A、O 这几组现在由 `manual-check/office/cases/xlsx-*.ps1` 自动判定，跑 `pwsh -File manual-check\office\run.ps1 -Case 'xlsx-*'` 即可复现，逐条结果落在 `manual-check/office/out/office-results.csv`。

| 序号 | 状态 | 结果 |
| --- | --- | --- |
| C1 | ✅ | Excel 16.0 打开 `manual-check/office/out/xlsx-create.xlsx`，标题栏无 Repair |
| C2–C7 | ✅ | WPS 此前已过；Excel：Sheet1、A1=1、A2=hello、A3=TRUE、B1=`=SUM(A1)` 且值为 1 |
| I1–I6 | ✅ | 有范围的 inspect 含 A1/A2/A3/B1、无 E1；无 scope 时没有 `cells` |
| A1 | ✅ | Excel 打开 `out/xlsx-applied.xlsx`，标题栏无 Repair；WPS 此前已过 |
| A2–A7 | ✅ | Excel：A1=42，B1=`=SUM(A1:A10)` 值为 47，C1 空且数字格式仍是 `0.00`，Sheet2 未动，E1 加粗/红字/黄底/列宽 20 还在，D1 重算为 84 |
| O1 | 🚫 | 领域层不做原样保存：三个动作里没有「只保存」，`apply` 的 `ops` 必须非空。零脏写回见 OPC 库测试。Excel 另存 create 产物见 `xlsx-O1-excel-resave` |
| O2 | ✅ | 未改表与未点名格子的样式还在；未改部件 CRC 与打补丁前相同 |

## create

用 `docs/examples/xlsx-create.json` 生成 `book.xlsx`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| C1 | Excel 打开 | 无修复提示、无「文件损坏」 |
| C2 | WPS 打开 | 同上 |
| C3 | 工作表名 | 存在 `Sheet1` |
| C4 | A1 | 数字 `1` |
| C5 | A2 | 文本 `hello` |
| C6 | A3 | 布尔，显示为 `TRUE` / `真` |
| C7 | B1 | 公式栏为 `=SUM(A1)`，打开后值变成 `1`（允许先出现空白再重算） |

## inspect

对上面的文件发 `docs/examples/xlsx-inspect.json`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| I1 | 表名单 | 含 `Sheet1`，id 为 `sheet:Sheet1` |
| I2 | 范围外 | `E1` 不出现在 `cells` |
| I3 | A1 | `type` 为 `number`，`value` 为 `1` |
| I4 | A2 | `type` 为 `text`，`value` 为 `hello` |
| I5 | B1 | `formula` 为 `=SUM(A1)` |
| I6 | 无 scope | 只发 path 时没有 `cells` 字段 |

## apply

准备一份 Excel 亲手另存的样例（含两张表、若干样式、一张表里有公式）。仓库里那份是 `crates/ooxp/tests/fixtures/excel-source.xlsx`，由 Excel 16.0 `SaveAs` 写出，重做跑 `manual-check/office/export-excel-fixture.ps1`。对它执行 `docs/examples/xlsx-apply.json` 里的操作，必要时改 id 里的表名。库测试 `xlsx_excel_source.rs` 覆盖下面 A2–A7 的 XML 与 O2；A1 仍要 Office 打开。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| A1 | 打开 | Excel 与 WPS 都能打开，无修复提示 |
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
