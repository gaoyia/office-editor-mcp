# xlsx 页眉、chartsheet drawing 与 DrawingML 链

状态：❌
日期：2026-10-10

对应 `roadmap/xlsx.md` 待做序号 **36**。前置：页眉页脚、形状、OLE、超链接已过。

`docs/json-contract-xlsx.md` 现状：页眉页脚里的图表、形状、OLE 不进树。`chartsheet` 上的形状、连接线、OLE、3D、公式不进树。`HYPERLINK()` 公式、图上 `a:hlinkClick`、文本框 / SmartArt 上的链不进树。组合里的 OLE 不进树。

## 目标

页眉 drawing 与图表工作表上的形状 / 连接线 / OLE / 图能进树，能复用已有 op。工作表图、文本框、SmartArt 上的 DrawingML 超链接能读写。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | 文档：宿主 id、DrawingML 链 | ❌ |
| 1 | 库：inspect / apply | ❌ |
| 2 | 库测试与 Excel 16.0 手测 | ❌ |

## 已锁定

- 不编辑 VML / `legacyDrawingHF`。
- 不按列宽反推尺寸。不激活 OLE 服务器。
- 链接 OLE（`@link`）确认时写入；ActiveX `controls` 仍不进树。
- `HYPERLINK()` 仍当公式字符串，不要和 DrawingML 链混成一个节点。

## 不做

- 编辑 VML
- 公式求值、打开浏览器

## 确认后要改的文档

- `docs/json-contract-xlsx.md`、`docs/json-schema/`、`docs/checklist-xlsx.md`
- `docs/json-contract.md`、`docs/errors.md`
- `roadmap/xlsx.md` 待做 36
