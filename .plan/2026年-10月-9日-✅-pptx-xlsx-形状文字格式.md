# pptx / xlsx 形状文字格式

状态：✅ 文档、代码、库测试与 Office 16.0 手测已过
日期：2026-10-09

对应 `roadmap/pptx.md` 与 `roadmap/xlsx.md` 跨格式 **C9**。前置：形状增删、工作表文本框 / 形状。不要改 Word `setRunFormat`。不算排版引擎，不做字填充 / 艺术字，不导出幻灯片 PNG。

契约真源：`docs/json-contract.md`、pptx / xlsx 分册、两份 json-schema、`docs/architecture.md`、pptx / xlsx 清单。示例：`docs/examples/pptx-apply-shape-format.json`、`docs/examples/xlsx-apply-shape-format.json`。

## 目标

PowerPoint 形状（含备注、母版占位符、表单元格）和 Excel 文本框 / 带 `txBody` 的形状能改字号、字色，以及同类缺口：加粗、斜体、下划线、中英文字体、水平对齐、框内垂直锚点。inspect 能读回。create / insert 能带同一套字段。Office 16.0 打开无修复。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | 对照 ECMA-376 DrawingML `a:rPr` / `a:pPr` / `a:bodyPr` | ✅ |
| 1 | 文档与 schema | ✅ |
| 2 | inspect / apply / create | ✅ |
| 3 | 库测试与 Office 16.0 手测 | ✅ |

## 已锁规则

- 新 op **`setShapeFormat`**。不要扩 `setShapeText`（`text` 必填）。不要先做 run 级 inspect 树
- 字段至少给一个，否则 `InvalidField`。JSON 名与 OOXML：
  - `sz`：`a:rPr/@sz`，百分之一磅，整数 100–400000（§20.1.10.68）。`null` 去掉显式字号
  - `color`：`a:solidFill/a:srgbClr`，六位 RGB 或 `null` 去掉显式填色。写入大写。不写 theme 色
  - `bold` / `italic`：`@b` / `@i`。`false` 写 `"0"`
  - `underline`：`@u`，只认 `sng` / `none` / `dbl`；`null` 去掉属性
  - `font`：`a:latin/@typeface`。`eastAsiaFont`：`a:ea/@typeface`。空串报错
  - `align`：`a:pPr/@algn`，`l` / `ctr` / `r` / `just`（§20.1.10.59）；`null` 去掉属性
  - `textAnchor`：`a:bodyPr/@anchor`，`t` / `ctr` / `b`（§20.1.10.60）。**不能叫 `anchor`**（xlsx 锚点已占用）
- 目标：pptx 形状 id（与 `setShapeText` 相同，含备注 / 母版占位符；`sldImg` 拒）、pptx 表单元格 id、xlsx 文本框 id 或带 `txBody` 的形状 id。连接线报 `InvalidId`
- 改该 `txBody` 里每个文本 `a:r` 的 `a:rPr`，同步 `a:endParaRPr`。不改 `a:fld`。不重写 `a:lstStyle`。保留未点名子元
- inspect 在 `text` 之后、`stable` 之前始终带这九个键。各 run 不一致或没有显式值则为 `null`，不猜主题
- create / `insertShape` / xlsx `insertTextBox` / `insertShape` 认同一套可选键。pptx create 缺省仍是 18 磅黑字 Calibri / 宋体
- 共享代码在 `crates/ooxp/src/drawing_text.rs`。pptx 不要 `use xlsx::`
- 不改 Word

## 不做

- 排版引擎、渐变字、字轮廓、艺术字
- run 级 inspect / 同一框多种字号（不一致则 inspect 为 `null`）
- 幻灯片 PNG 导出
- 改 `setRunFormat`

## 手测

`manual-check/office/cases/pptx-shape-format.ps1`、`xlsx-shape-format.ps1`。编号 **Tf** / **Xtf**。不要和 Ps / Sg / Xsa 混号。
2026-10-09 Office 16.0：Tf1–Tf5、Xtf1–Xtf5 均 PASS，无 Repair。
