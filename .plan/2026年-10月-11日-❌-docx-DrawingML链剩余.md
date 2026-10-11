# docx DrawingML 链剩余

状态：❌
日期：2026-10-11

对应 `roadmap/docx.md` 待做序号 **61**。前置：`.plan/2026年-10月-10日-✅-docx-图与形状超链接.md`。不要和 `w:hyperlink`、OLE、域合成一份。不打开浏览器。

47 只写正文与页眉页脚的图、文本框。框内图、注内 / 批注内的图或框、连接线、无框形状、SmartArt、图表、墨迹、3D 发超链接 op 仍报 `InvalidId`。OLE 继续不写 DrawingML 链。

## 目标

上述除 OLE 以外的 DrawingML 对象，inspect 带 `hyperlink` 字段，apply 能插、改、删 `a:hlinkClick`。对象形状与图上那条相同。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | 文档：扩 id 与 inspect | ❌ |
| 1 | 库：读写与测试 | ❌ |
| 2 | Word 16.0 手测 | ❌ |

## 已锁定

- 图上已有的 `hyperlink` 对象字段顺序不变：`kind`、`rId`、`url`、`anchor`、`tooltip`。
- `insertHyperlink` / `setHyperlink` / `deleteHyperlink` 的 id 增加：框内图、注内 / 批注内图或框、连接线、无框形状、SmartArt、图表、墨迹画布、3D。对这些 id 禁止 `text`。
- 链仍不进 `hyperlinks` 名单，不计入 `hyperlinkCount`。
- OLE id 仍报 `InvalidId`。xlsx / pptx 行为不变。
- create：对应数组项可带 `hyperlink`，规则与 `pictures[].hyperlink` 相同。
- 手测编号 **Dhr**。脚本 `manual-check/office/cases/docx-drawing-hlink-remain.ps1`。不要和 Dh、Dl 撞。

## 不做

- OLE 上的 DrawingML 链
- `hlinkHover`
- 打开浏览器

## 确认后要改的文档

- `docs/json-contract.md`、`docs/json-contract-docx.md`、`docs/json-schema/`
- `docs/architecture.md`、`docs/checklist-docx.md`、`docs/errors.md`
- `roadmap/docx.md` 待做 61
