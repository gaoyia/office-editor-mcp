# xlsx create 带图

状态：✅ 已通过  
日期：2026-09-27

从 `.plan/2026年-9月-27日-❌-xlsx-图与图表路线.md` 拆出序号 2。`create` 是唯一允许整包生成的动作。本期让新建工作簿能带 SpreadsheetDrawing 的 `xdr:pic`。复用 `insertPicture` 的写入，不另做生成器。

前置：`.plan/2026年-9月-27日-✅-xlsx-工作表图.md`。图表不是前置；`create` 仍不写图表。

契约真源：`docs/json-contract.md`、`docs/json-schema/request.schema.json`、`docs/errors.md`、`docs/architecture.md`、`docs/opc.md`、`docs/checklist-xlsx.md`。

## 目标

从领域 JSON 新建一份带工作表图的 xlsx。图挂在 `worksheet` 上，字段与 `insertPicture` 同一套（不含 `id`）。Excel 打开时自己画。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档先行（本计划 + 契约、错误、架构、清单、示例） | ✅ |
| 2.1 | `workbook.sheets[].pictures`：本地 `file`，必填 `from`，可选锚点与尺寸 | ✅ |
| 2.2 | 写入 `/xl/drawings/drawingN.xml`、`/xl/media/…`、工作表 `<drawing>`、内容类型 | ✅ |
| 3 | 一张表多张图；`oneCell` 与 `twoCell` 可混；`cNvPr/@id` 从 1 起按该表数组顺序递增 | ✅ |
| 4 | 库测试：create 后再 inspect，图 id 与 `anchor` / `from` 对 | ✅ |
| 5 | Excel 16.0 打开 create 产物，无修复，图能看见 | ✅ |

## 已锁定（写入契约后不要改）

- 图只挂在 `workbook.sheets[]` 上，字段名 `pictures`。不挂在 `workbook` 根上，也不挂在单元格上。
- 每项字段与 xlsx `insertPicture` 相同，但不含 `id`：`file` 必须；`from` 必须（A1）；`anchor` 可选，缺省 `oneCell`；`twoCell` 必须再给 `to`；`cx` / `cy` / `fromOffX` / `fromOffY` / `toOffX` / `toOffY` 规则与 `insertPicture` 相同。
- `kind` / `wrap` / `alignH` / `alignV` / `offsetX` / `offsetY` 是 Word 绕排，xlsx create 上出现报 `InvalidField`。
- `workbook` 根上、`cells[]` 上出现 `pictures`，或 `sheets[]` 上出现 `charts` / `smartArts`，报 `InvalidField`。
- 不把 base64 写进 create JSON。认 PNG / JPEG / GIF / BMP，按文件头判断。缺 `file` 或 `from` 报 `MissingField`。只给一个尺寸报 `InvalidField`。
- 省略 `pictures` 或给空数组：该表不写 drawing，行为与现在无图 create 相同。
- 一张表上的图写入同一份 drawing 部件。`xdr:cNvPr/@id` 从 `1` 起按数组顺序递增。多张表各写各的 `drawingN.xml`。
- 锚点必须带空的 `xdr:clientData`。关系 Target、`xmlns:r` 与 `insertPicture` 同一套。不插 `absolute`。
- 不写图表、SmartArt、形状、OLE、页眉页脚图。pptx create 上的 `pictures` 仍拒绝。

## 不做

- 从已有文件当模板复制
- create 带图表
- 嵌入 SVG 转 DrawingML、生成 VML
- 按列宽行高去算 `twoCell` 显示尺寸

## 验收

手测编号 C8–C10，写入 `docs/checklist-xlsx.md`。脚本建议 `manual-check/office/cases/xlsx-create-pictures.ps1`。用 `docs/examples/xlsx-create-pictures.json`（换成本机 PNG）生成。

- C8：inspect `pictures: true` 时第一张 id 为 `sheet:Sheet1/pic:1`，`anchor` 为 `oneCell`，`from` 对上。
- C9：同一张表第二张为 `twoCell`，inspect 两张都有。
- C10：Excel 16.0 打开无修复，图能看见。
