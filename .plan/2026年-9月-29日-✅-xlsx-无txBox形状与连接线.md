# xlsx 无 txBox 的形状与连接线

状态：✅  
日期：2026-09-29

对应 `roadmap/xlsx.md` 待做序号 **18**。前置：形状与 SmartArt（序号 12 已过）。不要和艺术字画廊绑在一起。不要和待做 19（组合）合成一份。

不算公式求值。不算图表渲染。不编辑 VML。组合本身仍不进树（19）。页眉里的形状仍不进树。

契约真源：`docs/json-contract.md`、`docs/json-contract-xlsx.md`、`docs/json-schema/`、`docs/errors.md`、`docs/architecture.md`、`docs/opc.md`、`docs/checklist-xlsx.md`。

## 目标

inspect 能列出工作表上无 `txBox` 的 `xdr:sp` 与连接线 `xdr:cxnSp`。`insertShape` / `insertConnector` 能写出 Excel 16.0 能打开、无修复的对象。已有 `textBoxes`（`txBox=1`）契约不拆。三种格式里只有 xlsx 本期改可写面。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | Excel 16.0 另存：矩形 / 椭圆 / 直线 / 直线连接符 / 肘形连接符，锁 drawing XML | ✅ |
| 1 | 文档：新名单、id 前缀、insert / 删 / 尺寸锚点、与 `textBoxes` 分家 | ✅ |
| 2 | `crates/ooxp/src/xlsx/shape.rs`（或旁路）与 request / inspect / create 拒写 | ✅ |
| 3 | 库测试与 Excel 16.0 手测 | ✅ |

建议提交：`0 → 1 → 2 → 3`。

## 已锁定（规范）

ECMA-376 §20.5：`xdr:cNvSpPr/@txBox` 缺省表示**不是**文本框；无 `txBox` 的 `xdr:sp` 仍可带 `xdr:txBody`。连接线是 `xdr:cxnSp`（`xdr_CT_Connector`）：`nvCxnSpPr` / `cNvCxnSpPr` / `spPr`，可选 `style`。锚点仍是 `oneCell` / `twoCell` / `absolute`，与工作表图相同，不是 pptx 画布 `a:xfrm` 的页坐标。

已过 12：只收 `xdr:sp` 且 `txBox` 为 `1`/`true`。id 为 `sheet:{name}/sp:{cNvPrId}`。`insertTextBox` 写 `txBox=1`。`insertConnector` / `scope.connectors` 目前在 xlsx 上报 `InvalidField`。

## 已锁定（Excel 16.0，2026-09-29）

COM `AddShape(1)` 矩形、`AddShape(9)` 椭圆、`AddLine`、`AddConnector(1)` 直线连接符、`AddConnector(2)` 肘形。ZIP 以 `xl/drawings/drawing1.xml` 为准。

- **矩形**：`xdr:twoCellAnchor` + `xdr:sp`。`cNvPr/@name` 为「矩形 1」。**空** `xdr:cNvSpPr`（**没有** `txBox`）。`a:prstGeom/@prst="rect"`。有空 `xdr:txBody`（`a:p` 只有 `endParaRPr sz=1100`）。有 `xdr:style` 主题引用。`macro=""` `textlink=""`。
- **椭圆**：同上，`prst="ellipse"`，名「椭圆 2」。
- **`AddLine`**：是 **`xdr:cxnSp`**，不是 `xdr:sp`。空 `xdr:cNvCxnSpPr`（无 `stCxn` / `endCxn`）。`prst="line"`。名「直接连接符 3」。无 `txBody`。
- **直线箭头连接符**：`xdr:cxnSp`，`prst="straightConnector1"`，名「直接箭头连接符 4」。空 `cNvCxnSpPr`。
- **肘形**：`xdr:cxnSp`，`prst="bentConnector3"`，名「连接符: 肘形 5」。空 `cNvCxnSpPr`。
- 五者都是 `twoCellAnchor`。COM `AutoShapeType` 不要当包裹依据：`AddLine` 的 `AutoShapeType=-2`。

库契约（已按探测收口）：

- `textBoxes` 仍只收 `txBox=1` 的 `xdr:sp`，id 仍是 `sheet:{name}/sp:{cNvPrId}`。
- 新 `shapes`：`xdr:sp` 且不是文本框（无 `txBox` 或非 1/true）；id `sheet:{name}/shape:{cNvPrId}`。有 `txBody` 则 `text` 规则与文本框相同。
- 新 `connectors`：`xdr:cxnSp`；id `sheet:{name}/cxn:{cNvPrId}`。`prst` 取 `a:prstGeom/@prst`。
- 根与每张 `worksheet` 始终有 `shapeCount` / `connectorCount`。名单靠 `scope.shapes` / `scope.connectors`。
- `insertShape` 对 xlsx：`id` 为 `sheet:{name}`，必填 `from`（与 `insertTextBox` 相同锚点字段）。`prst` 缺省 `rect`。不写 `txBox`。写空 `txBody`。不写 `a16:creationId`。不写 `xdr:style`（对齐已过文本框：实心填充 + 线）。连接线几何名报 `InvalidField`。
- `insertConnector` 对 xlsx：`id` 为 `sheet:{name}`，必填 `from`。缺省 `prst=straightConnector1`。另允许 `line`、以及 pptx 那份连接符名单（未绑点）。空 `cNvCxnSpPr`。无 `txBody`。无填充，黑线。
- `setTextBoxSize` / `setTextBoxAnchor` 认 `shape:` 与 `cxn:`。`deleteShape` 认 `shape:`；`deleteConnector` 认 `cxn:`；`deleteTextBox` 仍只认 `/sp:`。
- `setShapeText` 对已有 `txBody` 的 `shape:` 可改字；对 `cxn:` 报 `InvalidId`。
- create 本期不写 `shapes` / `connectors`。
- 组合里符合条件的仍展开。页眉、chartsheet 不进。

## 不做（本期）

- 艺术字画廊 / 字填充 / 弯曲
- 组合 `xdr:grpSp` 进树或 `groupShapes`（待做 19）
- 绑连接点、自动绕障碍
- 页眉形状、chartsheet 上的形状
- VML
- 自定义几何 `a:custGeom` 写入（inspect 可标 `prst=null`）
- docx / pptx 契约改名

## 前置

形状与 SmartArt 已过。工作表图锚点已过。

## 验收

手测脚本 `manual-check/office/cases/xlsx-shapes-geom.ps1`。编号用 **Xsg**（不要和 Xsa、Xa、Xs、Xq 撞）。Excel 16.0 打开无修复。
