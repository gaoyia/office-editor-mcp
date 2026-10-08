# xlsx create 图表工作表

状态：✅ 库测试与 Excel 16.0 手测 C22–C24 已过  
日期：2026-10-09

从 `roadmap/xlsx.md` 待做序号 26。`create` 是唯一允许整包生成的动作。本期让新建工作簿能带 `kind: chartsheet` 的图表工作表，复用 `insertChartSheet`，不另做生成器。不实现图表渲染。

前置：`.plan/2026年-9月-27日-✅-xlsx-图表工作表.md`、`.plan/2026年-9月-27日-✅-xlsx-create带图表.md`。

契约真源：`docs/json-contract.md`、`docs/json-contract-xlsx.md`、`docs/json-schema/xlsx.schema.json`、`docs/errors.md`、`docs/architecture.md`、`docs/opc.md`、`docs/checklist-xlsx.md`。

## 目标

从领域 JSON 新建一份带图表工作表的 xlsx。字段与 `insertChartSheet` 同一套（不含 `id`）。数据写在点名的普通工作表上。Excel 打开时自己画，不实现放映引擎或图表渲染。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档先行（本计划 + 契约、错误、架构、清单、示例） | ✅ |
| 2 | `sheets[].kind` 认 `worksheet` / `chartsheet`；图表工作表复用 `insertChartSheet` | ✅ |
| 3 | 表顺序与 JSON 数组一致；`app.xml` HeadingPairs 区分 Worksheets / Charts | ✅ |
| 4 | 库测试：create 后再 inspect，`kind` / `chartCount` / 数据格对；缺字段与无普通表拒写 | ✅ |
| 5 | Excel 16.0 打开 create 产物，无修复；COM 能看见图表工作表与系列值 | ✅ |

建议提交顺序：`1 → 2 → 3 → 4 → 5`。先改 `docs/`，再动代码。

## 已锁定（写入契约后不要改）

- `sheets[].kind` 省略或 `"worksheet"` 为普通工作表。`"chartsheet"` 为图表工作表。`dialogsheet` / `macrosheet` 以及其它取值报 `InvalidField`。
- 图表工作表必填 `name`、`kind`、`type`、`series`、`dataFrom`。可选 `dataSheet`、`ofPieType`、`title`、`cx` / `cy`。字段语义与 `insertChartSheet` 相同。
- 图表工作表上出现 `cells` / `pictures` / `charts` / `headerFooters` / `comments` / `hyperlinks` / `textBoxes` / `shapes` / `connectors` / `smartArts` / `from` / `to` / `anchor` / 偏移 / `wrap` / `id` 报 `InvalidField`。
- 至少一张 `worksheet`。全部是图表工作表报 `InvalidField`（`NeedAtLeastOneSheet`）。
- `dataSheet` 必须是本请求里已有的 `worksheet` 名。找不到报 `IdNotFound`。指向另一张图表工作表报 `InvalidField`。省略则取 `sheets[]` 里第一张 `worksheet`。
- 数据按 `setCell` 语义写到点名的普通表 `dataFrom` 一带，覆盖已有格子（含该表 `cells[]` 与 `charts[]` 已写的值）。写入时机在该表 `charts[]` 之后。
- 复用 `insertChartSheet`：`/xl/chartsheets/sheetN.xml`、drawing 上那一张 `xdr:absoluteAnchor` 框、`cNvPr/@id` 至少为 2。不写 embedding。一张图表工作表只有这一张图。
- `workbook.xml` 的 `<sheet>` 顺序与 `sheets[]` 一致，不要把图表工作表一律排到末尾。
- 不写图表工作表页眉。不在同一张图表工作表上再插第二张。

## 不做

- create 写 `dialogsheet` / `macrosheet`
- 一张图表工作表上放多张图
- 图表渲染、公式求值
- create 写打印、视图、保护到图表工作表

## 验收

手测编号 C22–C24，写入 `docs/checklist-xlsx.md`。脚本 `manual-check/office/cases/xlsx-create-chartsheet.ps1`。用 `docs/examples/xlsx-create-chartsheet.json` 生成。

- C22：inspect `charts: true` 时存在 `name=Chart1` 且 `kind=chartsheet`，`chartCount` 为 1，图表 id 为 `sheet:Chart1/chart:2`。
- C23：点名的普通表 `dataFrom` 一带单元格与系列值 1/2/3 一致。
- C24：Excel 16.0 打开无修复；COM 图表工作表能看见，系列值对得上。Excel 16.0：3 PASS / 0 FAIL。
