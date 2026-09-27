# xlsx create 带图表

状态：✅ 库测试与 Excel 16.0 手测 C11–C13 已过  
日期：2026-09-27

从 `roadmap/xlsx.md` 已过序号 3。`create` 是唯一允许整包生成的动作。本期让新建工作簿能带 SpreadsheetDrawing 的 `xdr:graphicFrame` 和 `/xl/charts/chartN.xml`。复用 `insertChart` 的写入，不另做生成器。

前置：`.plan/2026年-9月-27日-✅-xlsx-工作表图表.md`、`.plan/2026年-9月-27日-✅-xlsx-create带图.md`。

契约真源：`docs/json-contract.md`、`docs/json-schema/request.schema.json`、`docs/errors.md`、`docs/architecture.md`、`docs/opc.md`、`docs/checklist-xlsx.md`。

## 目标

从领域 JSON 新建一份带工作表图表的 xlsx。图表挂在 `worksheet` 上，字段与 `insertChart` 同一套（不含 `id`）。数据写到本表单元格，不写 embedding。Excel 打开时自己重画。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档先行（本计划 + 契约、错误、架构、清单、示例） | ✅ |
| 2.1 | `workbook.sheets[].charts`：必填 `from`、`dataFrom`、`type`、`series`；可选锚点与尺寸 | ✅ |
| 2.2 | 写入 `/xl/charts/chartN.xml`、drawing 上的 `xdr:graphicFrame`、工作表单元格 | ✅ |
| 3 | 一张表可同时有 `pictures` 与 `charts`；共用一份 drawing；`cNvPr/@id` 按先图后表、数组顺序递增 | ✅ |
| 4 | 库测试：create 后再 inspect，图表 id、`type`、系列值与 `dataFrom` 一带单元格对 | ✅ |
| 5 | Excel 16.0 打开 create 产物，无修复，图能看见 | ✅ |

建议提交顺序：`1 → 2.1 → 2.2 → 3 → 4 → 5`。文档与代码已按此顺序落地。

## 已锁定（写入契约后不要改）

- 图表只挂在 `workbook.sheets[]` 上，字段名 `charts`。不挂在 `workbook` 根上，也不挂在单元格上。
- 每项字段与 xlsx `insertChart` 相同，但不含 `id`：`from`、`dataFrom`、`type`、`series` 必须；`anchor` 可选，缺省 `oneCell`；`twoCell` 必须再给 `to`；`title` / `cx` / `cy` / 偏移规则与 `insertChart` 相同。
- `kind` / `wrap` 是 Word 绕排，xlsx create 上出现报 `InvalidField`。
- `workbook` 根上、`cells[]` 上出现 `charts`，或 `sheets[]` 上出现 `smartArts`，报 `InvalidField`。
- `dataFrom` 按 `setCell` 语义覆盖已有格子，包括同一张表 `cells[]` 里已经声明的值。`c:f` 指向这些格子。不写 embedding。
- 省略 `charts` 或给空数组：该表不写图表，行为与现在无图表 create 相同。
- 一张表上的图与图表写入同一份 drawing。先按现有规则写 `pictures`，再按数组顺序写 `charts`。`xdr:cNvPr/@id` 取该表已有全部 `cNvPr` 的最大整数加 1，与 `insertChart` 相同。
- 锚点必须带空的 `xdr:clientData`。不插 `absolute`。不新建图表工作表。`sheets[].kind` 仍拒绝。
- 类型、组合图、系列条数限制与 `insertChart` 相同。不插透视图表。
- `exportChartData` 在 xlsx 上仍报 `InvalidField`。pptx create 上的 `charts` 仍拒绝。docx create 仍不写图表。

## 不做

- 从已有文件当模板复制
- create 写图表工作表
- 页眉页脚里的图表
- 改透视源、刷新透视缓存
- 图表渲染、公式求值

## 验收

手测编号 C11–C13，写入 `docs/checklist-xlsx.md`。脚本建议 `manual-check/office/cases/xlsx-create-charts.ps1`。用 `docs/examples/xlsx-create-charts.json` 生成。

- C11：inspect `charts: true` 时第一张 id 为 `sheet:Sheet1/chart:1`，`type` 对上，`from` 对上。
- C12：系列值与 `dataFrom` 一带单元格一致。
- C13：Excel 16.0 打开无修复，图能看见。
