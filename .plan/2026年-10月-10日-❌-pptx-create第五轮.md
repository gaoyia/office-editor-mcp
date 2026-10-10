# pptx create 第五轮

状态：❌
日期：2026-10-10

对应 `roadmap/pptx.md` 待做序号 **39**。前置：第四轮 create 已过；图 / 表 / 图表 / SmartArt 链与备注页公式 / 3D 的 apply 已过。

`.plan/2026年-10月-10日-✅-pptx-create第四轮.md` 明确不写图 / 表 / 图表 / SmartArt 上的链。契约还写：create 本期不在 `slides[].notes` 上写公式与 3D，也不写批注内图。

## 目标

create 复用已有 apply，写出备注页公式与 3D、以及图 / 表 / 图表 / SmartArt 上的链。不要另写一套 XML。任意时间轴 JSON 仍拒绝。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | 文档：create 字段 | ❌ |
| 1 | 库：create 复用 apply | ❌ |
| 2 | 库测试与 PowerPoint 16.0 手测 | ❌ |

## 已锁定

- 只复用已有 apply。`slides[]` 上出现 `timing` 仍报未知字段。
- `slides[].notes` 对象可带 `maths` / `model3ds`，字段与页上同名数组相同（不要 `op` / `id`）。字符串形式的 `notes` 仍只表示备注文本。
- `pictures[].hyperlink` / `tables[].hyperlink` / `charts[].hyperlink` / `smartArts[].hyperlink` 字段与 DrawingML `insertHyperlink` 相同：`url` / `file` / `slide` 恰好一个，可选 `tooltip`，可选 `event=hover`。不要 `text`。
- 表单元格链走已有 `tables[].cells` 的链字段，不要另发明名字。若契约尚无单元格 create 链，本切片补上并复用 apply。
- 批注内插图仍不在 create 里写。
- 手测编号 **Ck**。脚本 `manual-check/office/cases/pptx-create-round5.ps1`。不要和 Cq、C、Phr、Pcip 撞。

## 不做

- 放映引擎、任意时间轴树
- 从已有文件当模板复制
- 批注内图
- 幻灯片墨迹（未探测，不算欠账）

## 确认后要改的文档

- `docs/json-contract-pptx.md`、`docs/json-schema/`、`docs/examples/`
- `docs/architecture.md`、`docs/checklist-pptx.md`
- `roadmap/pptx.md` 待做 39
