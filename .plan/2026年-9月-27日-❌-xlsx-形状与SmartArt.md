# xlsx 形状与 SmartArt

状态：❌ 文档未写，实现与手测未开始  
日期：2026-09-27

从 `roadmap/xlsx.md` 拆出待做序号 12。工作表只收 `xdr:pic` 与图表 `graphicFrame`。`xdr:sp`、连接线、SmartArt 不进树。xlsx 的 SmartArt op 目前拒绝。

## 目标

列出并增删改工作表上的 DrawingML 形状（文本框）与 SmartArt。几何用锚点，规则对齐工作表图。SmartArt 复用 `crates` 里已有 diagram 逻辑，不要抄一份。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档：形状 id、与 pic/chart 分家 | ❌ |
| 2 | 形状：文本、尺寸、锚点 | ❌ |
| 3 | SmartArt：与 docx 同一套 op | ❌ |
| 4 | 库测试与 Excel 16.0 手测 | ❌ |

## 已锁定

- 组合展开子节点，组合本身不进树。
- 不编辑 VML 控件。
- 跨家族换 layout 走 C2。Word / PowerPoint 已过；表上换族等本期把图示送进树之后再验收。

## 不做

- 控件 ActiveX
- 按列宽反推尺寸

## 前置

工作表图已过。

## 验收

另存带文本框与 SmartArt 的表能列出；改字后 Excel 无修复。
