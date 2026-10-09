# xlsx / pptx 嵌入 OMML

状态：✅ 文档、代码、库测试与 Office 16.0 手测已过
日期：2026-10-09

对应 `roadmap/xlsx.md` 与 `roadmap/pptx.md` 跨格式 **C8**。前置：工作表图 / 文本框、幻灯片图。Word 公式已过（`.plan/2026年-9月-27日-✅-docx-公式.md`），**不要改 Word 写入路径**。不要和 EMF / 3D / 墨迹捆在一起。不算公式排版引擎。

契约真源：`docs/json-contract.md`、xlsx / pptx 分册、两份 json-schema、`docs/errors.md`、`docs/architecture.md`、`docs/opc.md`、xlsx / pptx 清单。示例：`docs/examples/xlsx-create-math-drawing.json`、`docs/examples/pptx-create-math-drawing.json`。

## 目标

Excel 工作表 drawing、PowerPoint 幻灯片（及备注）里的 OMML 公式框能进 inspect，能 `insertMath` / `setMath` / `deleteMath`，create 能按同一套 JSON 生成。Office 16.0 打开无修复。不进单元格公式，不进 `textBoxes` / `shapes`。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | Excel / PowerPoint 16.0 探测插入公式后的 XML | ✅ |
| 1 | 锁 id、与文本框的分界、写入 | ✅ |
| 2 | 文档 | ✅ |
| 3 | inspect / apply / create | ✅ |
| 4 | 库测试与 Office 16.0 手测 | ✅ |

## 探测结论（Office 16.0）

`CommandBars.ExecuteMso("EquationInsertNew")` 另存：

Excel：`xdr:oneCellAnchor` 里 `xdr:sp`（`txBox="1"`），`mc:Choice Requires="a14"`。`xdr:txBody/a:p` 里是 `a14:m` 包 `m:oMathPara/m:oMath`。空公式是 `m:t` 里的 `a:fld type="mathplaceholder"`。Fallback 是普通文本框，字为「"在此处键入公式。"」。

PowerPoint：`p:sp`（`txBox="1"`），同样 `a14:m` + `m:oMathPara`。Fallback 是带海报 PNG 的形状（Office 栅格化了占位符）。库写入不栅格化：Fallback 走 Excel 那种纯文本框。手测若 Repair 再补 PNG。

命名空间：

- `a14`：`http://schemas.microsoft.com/office/drawing/2010/main`
- `m`：`http://schemas.openxmlformats.org/officeDocument/2006/math`

这是 drawing 公式框，不是单元格 `f`。Word 的段落级 `m:oMath` 已过，本期不动。

## 已锁规则

- 只认 **xlsx / pptx**。docx 上这些 op 与 `scope.maths` 仍走已过的段落公式，不改写入、不改 `expr`
- 判定：`mc:AlternateContent` 的 Choice `Requires` 含 `a14`，且子孙有 `a14:m` 或 `m:oMath`。整段 AlternateContent **只进** `maths`。Choice 与 Fallback **都不进** `textBoxes` / `shapes`，也不计入 `textBoxCount` / `shapeCount`
- 不是单元格 `f`。对单元格 id、图 id、文本框 id 发公式 op 报 `InvalidId`
- 主契约里「xlsx / pptx 上发 `insertMath` / `setMath` / `deleteMath` 报 `InvalidField`」要改掉。xlsx / pptx 从「docx 专用 op」名单里拿掉这三条。docx 分册里那句「xlsx / pptx 上发此 op 报 `InvalidField`」改为指向本切片
- id：`sheet:{name}/math:{cNvPr}`；pptx `slide:{n}/math:{cNvPr}`；备注 `slide:{n}/notes/math:…`。没有 `cNvPr/@id` 时用 `math:index:{k}`，`stable` 为 `false`
- inspect：
  - xlsx：工作簿与每张 `worksheet` 始终 `mathCount`（只计工作表 drawing，不含页眉）。`chartsheet` / `dialogsheet` / `macrosheet` 本期不列、不插。计数字段插在 `textBoxCount` 之后
  - pptx：根与每页始终 `mathCount`（只计幻灯片，不含备注）。备注容器在 `scope.notes` 时另给 `mathCount`。计数字段插在 `pictureCount` 之后
  - `scope.maths: true` 才出 `maths`。xlsx / pptx 的 `scope` 要收这个键，不再报未知字段
  - 字段顺序：xlsx 为 `id`、`cNvPrId`、`name`、`anchor`，然后按锚点种类跟位置字段，再跟 `text`、`stable`。pptx 为 `id`、`cNvPrId`、`name`、`x`、`y`、`cx`、`cy`、`text`、`stable`
  - `text` 为该 `oMath` 内 `m:t` 拼接；忽略 `a:fld` 外壳。不把 OMML 原文、不把 `expr` 放进 xlsx / pptx 树
- apply：复用 `insertMath` / `setMath` / `deleteMath`，**不**新开 op 名
  - `insertMath`：`id` 为 `sheet:{name}` 或 `slide:{n}`（或 `slide:{n}/notes`）。必填非空 `text`。出现 `expr` / `kind` 报 `InvalidField`。xlsx 可选 `from`（缺省 A1、`oneCell`）；pptx 可选 `x`/`y`（缺省 0）。`cx`/`cy` 成对可选，缺则 1 英寸见方。写成 `mc:AlternateContent` + `Choice Requires="a14"` + `a14:m` + `m:oMathPara` + 一条 `m:r/m:t`。不要写 `mathplaceholder` 域
  - `setMath`：`id` 为公式框 id。只认非空 `text`。用一条 `m:r/m:t` 替换该 `oMath` 子节点。出现 `expr` 报 `InvalidField`。不改锚点与尺寸
  - `deleteMath`：删该 AlternateContent；海报无人引用则删
- Fallback：纯文本框，字为该 `text`。不栅格化，不写 `a:blipFill`
- create：`sheets[].maths[]` / `slides[].maths[]`，每项 `text`，可选位置。不要写进 `textBoxes[]` / `shapes[]`。不要 `paragraphs[].maths`（那是 Word）
- 组合里符合条件的公式框要展开，组合本身不进树
- 不改 Word

## 不做

- 公式排版、TeX、UnicodeMath 全量互转
- 把 Word 的 `expr` 树搬到 xlsx / pptx
- 改 Word 公式
- 工作表 / 幻灯片墨迹
- 往图表工作表、xlsx 页眉、pptx 批注里插公式
- 把 OMML 编成完整 JSON 树

## 建议提交顺序

`0 → 1 → 2 → 3 → 4`。先改 `docs/`。探测脚本不要提交。手测 `xlsx-omml.ps1` / `pptx-omml.ps1`，编号 **Xom** / **Pom**。不要和 Word 的 Mm 混号。挂机在 C7 之后做本份。
