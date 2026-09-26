# pptx create 带图表

状态：❌ 计划已写，确认后再改 `docs/`  
日期：2026-09-27

从 `.plan/2026年-9月-27日-❌-pptx-编辑路线.md` 拆出序号 3。复用 `insertChart` 的写入，不另做生成器。数据写 embedding，不写演示文稿里的工作表。

前置：`.plan/2026年-9月-27日-❌-pptx-幻灯片图表.md`、`.plan/2026年-9月-27日-✅-pptx-create带图.md`。

契约真源：`docs/json-contract.md`、`docs/json-schema/request.schema.json`、`docs/errors.md`、`docs/architecture.md`、`docs/opc.md`、`docs/checklist-pptx.md`。

## 目标

从领域 JSON 新建一份带幻灯片图表的 pptx。图表挂在 `slides[]` 上，字段与 pptx `insertChart` 同一套（不含 `id`）。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档先行 | ❌ |
| 2.1 | `slides[].charts`：必填 `type`、`series`；可选标题与画布坐标尺寸 | ❌ |
| 2.2 | 写入 `/ppt/charts/…`、embedding、`p:graphicFrame` | ❌ |
| 3 | 一页可同时有 `pictures` 与 `charts`；`cNvPr/@id` 先形状、再图、再图表 | ❌ |
| 4 | 库测试 | ❌ |
| 5 | PowerPoint 16.0 打开无修复，图能看见 | ❌ |

建议提交顺序：`1 → 2.1 → 2.2 → 3 → 4 → 5`。先改 `docs/`，确认后再动代码。

## 已锁定（写入契约后不要改）

- 图表只挂在 `presentation.slides[]` 上，字段名 `charts`。不挂在根上，也不挂在 `shapes[]` 上。
- 每项字段与 pptx `insertChart` 相同，但不含 `id`。`from` / `dataFrom` / `anchor` / `kind` / `wrap` 报 `InvalidField`。
- `dataFrom` 不出现；数据只进 embedding。
- 省略 `charts` 或空数组：该页不写图表。
- 不插透视图表。类型限制与 `insertChart` 相同。
- `exportChartData` 对 create 产物也认（有 embedding）。
- xlsx / docx 的 create 仍按各自契约；docx create 仍不写图表。

## 不做

- create 写备注、批注、动画、母版
- 从模板复制

## 验收

手测编号 C8–C10，写入 `docs/checklist-pptx.md`。脚本建议 `manual-check/office/cases/pptx-create-charts.ps1`。

- C8：inspect `charts: true` 时第一张 id 为 `slide:1/chart:…`，`type` 对上。
- C9：系列值与 `exportChartData` 写出的表一致。
- C10：PowerPoint 16.0 打开无修复，图能看见。
