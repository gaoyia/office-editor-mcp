# pptx create 带图

状态：✅ 文档、实现、库测试与 PowerPoint 16.0 手测 C5–C7 已过  
日期：2026-09-27

从 `.plan/2026年-9月-27日-❌-pptx-编辑路线.md` 拆出序号 1。`create` 是唯一允许整包生成的动作。本期让新建演示文稿能带 `p:pic`。复用 `insertPicture` 的写入，不另做生成器。

前置：`.plan/2026年-9月-27日-✅-pptx-幻灯片图.md`。

契约真源：`docs/json-contract.md`、`docs/json-schema/request.schema.json`、`docs/errors.md`、`docs/architecture.md`、`docs/opc.md`、`docs/checklist-pptx.md`。

## 目标

从领域 JSON 新建一份带图的 pptx。图挂在 `presentation.slides[]` 上，字段与 `insertPicture` 同一套（不含 `id`）。PowerPoint 打开时自己画。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档先行 | ✅ |
| 2.1 | `slides[].pictures`：本地 `file`，可选 `x` / `y` / `cx` / `cy` | ✅ |
| 2.2 | 写入 `/ppt/media/…`、该页 `p:pic`、关系与内容类型 | ✅ |
| 3 | 一页多张图；`cNvPr/@id` 与该页文本框同一套计数（文本框从 2 起，图接着编） | ✅ |
| 4 | 库测试：create 后再 inspect，图 id 与尺寸对 | ✅ |
| 5 | PowerPoint 16.0 打开无修复，图能看见 | ✅ |

建议提交顺序：`1 → 2.1 → 2.2 → 3 → 4 → 5`。先改 `docs/`，确认后再动代码。

## 已锁定（写入契约后不要改）

- 图只挂在 `presentation.slides[]` 上，字段名 `pictures`。不挂在 `presentation` 根上，也不挂在 `shapes[]` 上。
- 每项：`file` 必须；`x` / `y` / `cx` / `cy` 规则与 pptx `insertPicture` 相同。
- `kind` / `wrap` / `from` / `to` / `anchor` 出现报 `InvalidField`。
- `presentation` 根上、`shapes[]` 上出现 `pictures`，或 `slides[]` 上出现 `charts` / `smartArts`，在对应计划未开时仍报 `InvalidField`。
- 不把 base64 写进 create JSON。认 PNG / JPEG / GIF / BMP。缺 `file` 报 `MissingField`。只给一个尺寸报 `InvalidField`。
- 省略 `pictures` 或给空数组：该页不写 `p:pic`。
- 先写该页 `shapes`，再按数组顺序写 `pictures`。`cNvPr/@id` 取该页已有最大整数加 1。
- 不写图表、SmartArt、备注、批注、动画。xlsx / docx 的 create 契约不动。

## 不做

- 从已有文件当模板复制
- create 带图表
- 母版 / 版式上的图
- 生成 VML

## 验收

手测编号 C5–C7，写入 `docs/checklist-pptx.md`。脚本建议 `manual-check/office/cases/pptx-create-pictures.ps1`。用 `docs/examples/pptx-create-pictures.json`（换成本机 PNG）生成。

- C5：inspect `pictures: true` 时第一张 id 为 `slide:1/pic:…`，能对上。
- C6：同一页第二张也在名单里。
- C7：PowerPoint 16.0 打开无修复，图能看见。
