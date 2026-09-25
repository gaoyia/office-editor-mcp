# xlsx 人工检查清单

代码阶段的单元测试覆盖 JSON 解析和 XML 不变量。下面这些必须用 Excel 和 WPS 各打开一次，不能只靠测试绿。

检查前用 create 或 apply 在本机生成文件，不要用改过扩展名的 zip 目录冒充。

## 本机结果（2026-09-26）

文件已从工作区删除，结论留在本清单。本机是 WPS 表格，没有 Microsoft Excel。用户确认：ooxp 打补丁后的工作簿，与 WPS 对同一源文件做同样三处编辑后的工作簿，肉眼一致。WPS COM 把值、公式、数字格式、加粗、字号、字体色、填充、列宽逐格读出，差异数为 0。

| 序号 | 结果 |
| --- | --- |
| C1 | 未测（无 Excel） |
| C2–C7 | 通过（WPS 打开 create 产物，格子与公式符合标准） |
| I1–I6 | 通过（有范围的 inspect 含 A1/A2/A3/B1、无 E1；无 scope 时没有 `cells`） |
| A1 | WPS 通过；Excel 未测 |
| A2–A7 | 通过（WPS；ooxp 产物与 WPS 同样编辑后的文件一致） |
| O1 | xlsx/WPS 能打开；Word、PowerPoint、Excel 未测 |
| O2 | 通过（上述两份肉眼与 COM 均一致；未改部件 CRC 与打补丁前相同） |

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

准备一份 Excel 亲手另存的样例（含两张表、若干样式、一张表里有公式）。对它执行 `docs/examples/xlsx-apply.json` 里的操作，必要时改 id 里的表名。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| A1 | 打开 | Excel 与 WPS 都能打开，无修复提示 |
| A2 | 目标格 | `Sheet1!A1` 变成 `42` |
| A3 | 公式格 | `Sheet1!B1` 公式栏为 `=SUM(A1:A10)`，打开后会重算 |
| A4 | 清空 | `Sheet1!C1` 值为空，若原先有数字格式，格式还在 |
| A5 | 未改表 | 第二张表的格子与另存前一致 |
| A6 | 未改样式 | 未点名的格子字体、填充、列宽看起来不变 |
| A7 | 重算 | 依赖 A1 的其他公式在打开后得到新结果，而不是旧缓存 |

## 原样保存（opc 验收，实现第 2 项时做）

打开样例再保存，一个部件都不改。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| O1 | Excel / Word / PowerPoint 都能打开另存后的文件 | 无修复提示 |
| O2 | 未改区域 | 与另存前对比，肉眼无差异 |
