# xlsx create 带批注、超链接、形状、SmartArt

状态：✅ 库测试与 Excel 16.0 手测 C17–C21 已过  
日期：2026-10-09

从 `roadmap/xlsx.md` 待做序号 25。`create` 是唯一允许整包生成的动作。本期让新建工作簿能带线程化批注、工作表超链接，以及 worksheet drawing 上的文本框、无 `txBox` 形状、连接线与 SmartArt。复用已有 insert 写入，不另做生成器。

前置：`.plan/2026年-9月-27日-✅-xlsx-批注.md`、`.plan/2026年-9月-27日-✅-xlsx-超链接.md`、`.plan/2026年-9月-27日-✅-xlsx-形状与SmartArt.md`、`.plan/2026年-9月-29日-✅-xlsx-无txBox形状与连接线.md`。

契约真源：`docs/json-contract.md`、`docs/json-contract-xlsx.md`、`docs/json-schema/xlsx.schema.json`、`docs/errors.md`、`docs/architecture.md`、`docs/opc.md`、`docs/checklist-xlsx.md`。

## 目标

从领域 JSON 新建一份带批注、超链接、形状与 SmartArt 的 xlsx。字段与对应 insert 同一套（不含 `id`）。Excel 打开时自己画，不实现放映引擎或 SmartArt 版式算法。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档先行（本计划 + 契约、错误、架构、清单、示例） | ✅ |
| 2.1 | `sheets[].comments`：必填 `cell`，可选 `text` / `author` / `initials` / `date` | ✅ |
| 2.2 | `sheets[].hyperlinks`：必填 `ref`，`url` / `location` 至少给一个 | ✅ |
| 2.3 | `sheets[].textBoxes` / `shapes` / `connectors` / `smartArts`：必填 `from`，锚点与 insert 相同 | ✅ |
| 3 | 与已有 `pictures` / `charts` 共用一份 drawing；`cNvPr/@id` 按先图后表、再框、形状、连接线、图示递增 | ✅ |
| 4 | 库测试：create 后再 inspect，名单与字段对；缺字段拒写 | ✅ |
| 5 | Excel 16.0 打开 create 产物，无修复；COM 能看见批注、超链接、形状与 SmartArt | ✅ |

## 已锁定（写入契约后不要改）

- 这些对象只挂在 `workbook.sheets[]` 上。不挂在 `workbook` 根上，也不挂在 `cells[]` 上。
- 批注字段名 `comments`。每项：`cell` 必须（单格 A1）；`text` 可选，省略当作 `""`；`author` 可选，省略 `"ooxp"`；`initials` 可选，xlsx 接受但不写入 XML；`date` 可选，省略则写入当前 UTC。不写回复。出现 `id` / `replies` / `start` / `end` / `x` / `y` 报 `InvalidField`。复用 `insertComment`。
- 超链接字段名 `hyperlinks`。每项字段与 xlsx `insertHyperlink` 相同，但不含 `id`。出现 `text` / `anchor` 报 `InvalidField`。与同表已有链相交报 `InvalidField`。不写单元格值。
- 文本框 / 形状 / 连接线 / SmartArt 字段与对应 insert 相同，但不含 `id`。`from` 必须。`kind` / `wrap` / `x` / `y` 报 `InvalidField`。SmartArt 必须给 `layout`。
- 一张表上的图、图表、文本框、形状、连接线与 SmartArt 共用一份 drawing：先写 `pictures`，再写 `charts`，再按 `textBoxes`、`shapes`、`connectors`、`smartArts` 数组顺序写入。
- 省略或空数组：该表不写对应部件 / 元素。
- 不写组合、不写 OLE、不写切片器、不写迷你图。不写图表工作表。`sheets[].kind` 仍拒绝。
- 不编辑 VML 几何；批注仍按 Excel 16.0 另存补占位 `comments.xml` 与 Note `v:shape`。

## 不做

- create 写回复批注
- create 写组合、OLE、切片器、迷你图
- create 写图表工作表（序号 26）
- SmartArt 版式算法、公式求值

## 验收

手测编号 C17–C21，写入 `docs/checklist-xlsx.md`。脚本 `manual-check/office/cases/xlsx-create-drawing.ps1`。用 `docs/examples/xlsx-create-drawing.json` 生成。

- C17：inspect `comments: true` 时第一条 `ref` 为 `A1`，`text` 含约定正文。
- C18：inspect `hyperlinks: true` 时第一条 id 为 `sheet:Sheet1/link:1`，`url` 对上。
- C19：inspect `shapes: true` 时第一条 `from` 对上，`prst` 对上。
- C20：inspect `smartArt: true` 时第一条 `layout` 为 list 族 uniqueId，`from` 对上。
- C21：Excel 16.0 打开无修复；COM 线程化批注、超链接与 Shapes（含 SmartArt）都能看见。Excel 16.0：5 PASS / 0 FAIL。
