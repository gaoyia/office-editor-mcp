# xlsx 形状与 SmartArt

状态：✅ 文档、实现与 Excel 16.0 手测已过  
日期：2026-09-27

从 `roadmap/xlsx.md` 拆出待做序号 12。工作表原先只收 `xdr:pic` 与图表 `graphicFrame`。`xdr:sp`、连接线、SmartArt 不进树。xlsx 的 SmartArt op 原先拒绝。

## 目标

列出并增删改工作表上的 DrawingML 形状（文本框）与 SmartArt。几何用锚点，规则对齐工作表图。SmartArt 复用 `crates` 里已有 diagram 逻辑，不要抄一份。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档：形状 id、与 pic/chart 分家 | ✅ |
| 2 | 对着 Excel 16.0 另存探测真源 | ✅ |
| 3 | 形状：文本、尺寸、锚点 | ✅ |
| 4 | SmartArt：与 docx 同一套 op | ✅ |
| 5 | 库测试与 Excel 16.0 手测 | ✅ |

## 已锁定

- 组合展开子节点，组合本身不进树。
- 不编辑 VML 控件。
- 跨家族换 layout 走 C2。Word / PowerPoint 已过；表上换族等本期把图示送进树之后再验收。
- Excel 16.0 `Shapes.AddTextbox` 另存：工作表 `<drawing>` 指向 `/xl/drawings/drawingN.xml`。文本框是锚点直接子元素 `xdr:sp`，`xdr:cNvSpPr/@txBox="1"`，可见字在 `xdr:txBody` 的 `a:t`（本机样例拆成 `en-US` / `zh-CN` 两个 run）。`xdr:cNvPr/@id` 作稳定 id（样例为 `2`，`name` 为「文本框 1」）。几何是 `twoCellAnchor`，规则对齐工作表图，不要套 Word 框内段落。
- Excel 16.0 `Shapes.AddSmartArt` 另存：同一 drawing 里锚点直接子元素 `xdr:graphicFrame`，`a:graphicData/@uri` 为 `http://schemas.openxmlformats.org/drawingml/2006/diagram`，`dgm:relIds` 的 `r:dm` / `r:lo` / `r:qs` / `r:cs` 指向 drawing `.rels` 里的 `/xl/diagrams/` 四件套。另有 `diagramDrawing` 关系指向 `/xl/diagrams/drawingN.xml`。`cNvPr/@id` 作稳定 id（样例为 `3`，`name` 为「图示 2」）。图表 `graphicFrame` 仍只认 chart / chartex URI，不要把 diagram 算进 `charts`。
- 文本框不要做成 docx 那种框内 `w:p`。改字对齐 pptx `setShapeText`：只动 `xdr:txBody` 可见 `a:t`。
- SmartArt 复用 `crates` 里已有 diagram 逻辑，不要再抄一份。drawing 缓存按 docx / pptx 同一套：对不上 modelId 则删缓存。
- 连接线 `xdr:cxnSp`、无 `txBox` 的普通 `xdr:sp`、VML 控件不进树。

## 不做

- 控件 ActiveX
- 按列宽反推尺寸

## 前置

工作表图已过。

## 验收

另存带文本框与 SmartArt 的表能列出；改字后 Excel 无修复。2026-09-28 跑 `xlsx-shapes.ps1`，Xsa1–Xsa5 通过。
