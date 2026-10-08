# xlsx dialogsheet / macrosheet 只读进树

状态：✅ 已通过 Excel 16.0
日期：2026-10-09

从 `roadmap/xlsx.md` 待做序号 31。前置：`.plan/2026年-9月-27日-✅-xlsx-工作表管理.md`。不要和 VBA、`vbaProject.bin`、控件编辑、宏表公式求值绑在一起。不执行宏。

契约真源：`docs/json-contract-xlsx.md`、`docs/json-contract.md`、`docs/errors.md`、`docs/architecture.md`、`docs/opc.md`、`docs/checklist-xlsx.md`。

## 目标

inspect 把工作簿里的 `dialogsheet` 与 `macrosheet` 列进 `sheets`，`kind` 对得上，字段按已锁定的精简形状。apply 仍不插、不删、不改名、不隐藏、不写单元格。Excel 16.0 打开对话框表无修复。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | Office 16.0：能否新建或另存 dialog / 宏表，锁 XML | ✅ |
| 1 | 文档先行（本计划 + 契约、错误、架构、清单） | ✅ |
| 2 | inspect 认关系 Type 与部件；apply / create 仍拒写 | ✅ |
| 3 | 库测试 | ✅ |
| 4 | Excel 16.0 手测 | ✅ |

## 第 0 步（Excel 16.0）

探测脚本 `manual-check/office/cases/_probe-xlsx-31-dialog-macro.ps1`（不要提交、不要进 `run.ps1`）。COM `Sheets.Add`：`xlDialogSheet=-4116`、`xlExcel4MacroSheet=3`、`xlExcel4IntlMacroSheet=4` 均成功。

另存 `.xlsx`：对话框表留下；宏表被写成普通 `worksheet`。另存 `.xlsm`：三类都在。本仓库 `format` 只认 `xlsx`，不打开 `.xlsm`。宏表手测与库测试把规范最小部件注入 `.xlsx` 包。

xlsx 对话框（Excel 另存）：

- 路径：`xl/dialogsheets/sheet1.xml`
- 关系 Type：`http://schemas.openxmlformats.org/officeDocument/2006/relationships/dialogsheet`
- CT：`application/vnd.openxmlformats-officedocument.spreadsheetml.dialogsheet+xml`
- 根：`<dialogsheet>`（spreadsheetml/2006/main）
- 常见子元素：`sheetViews`、`sheetFormatPr`、`sheetProtection sheet="1"`、`pageMargins`、`legacyDrawing`（VML，本期不进树、不改）

xlsm 宏表：

- 路径：`xl/macrosheets/sheet1.xml`
- Type：`http://schemas.microsoft.com/office/2006/relationships/xlMacrosheet`
- CT：`application/vnd.ms-excel.macrosheet+xml`
- 根：`<xm:macrosheet>`，默认 xmlns 仍是 spreadsheetml/2006/main，`xmlns:xm="http://schemas.microsoft.com/office/excel/2006/main"`

国际宏表：

- 路径：`xl/macrosheets/intlsheet1.xml`
- Type：`http://schemas.microsoft.com/office/2006/relationships/xlIntlMacrosheet`
- CT：`application/vnd.ms-excel.intlmacrosheet+xml`
- 根同样是 `<xm:macrosheet>`

## 已锁定（写入契约后不要改）

- 关系 Type 以 `/dialogsheet` 结尾 → `kind` 为 `dialogsheet`。Type 以 `/macrosheet`、`/xlMacrosheet` 或 `/xlIntlMacrosheet` 结尾 → `kind` 为 `macrosheet`。国际宏表不另开 `kind`。不要看部件路径或根元素名。
- 树上字段顺序仍是 `id`、`name`、`kind`、`hidden`、`hasHeaderFooter`，有 `scope.headerFooter` 再跟 `headerFooters`。不填单元格、图、图表、保护、打印、视图。
- 点名这两类表做单元格切片仍报 `InvalidField`（`InspectWorksheetOnly`）。
- create 的 `sheets[].kind` 不是 `worksheet` / `chartsheet` 报 `InvalidField`。
- apply：`deleteSheet` / `renameSheet` / `moveSheet` / `setSheetHidden` 以及本期已拒的其它写 op 仍报 `NotImplemented`。不新开编辑 op。
- `setHeaderFooterText` 对 `dialogsheet` 仍可写文本（已有契约）；对 `macrosheet` 本期同样只读页眉原文，不新开写。若实现已拒写则保持 `NotImplemented`。
- 手测编号 **Xdm**，脚本 `manual-check/office/cases/xlsx-dialog-macro.ps1`。不要和 Xm、Xd 撞。
- 手测：COM 加对话框表再另存 `.xlsx`（金样）。宏表用 ZIP 注入最小 `<xm:macrosheet>` + Type / CT。注入宏表后不强制 Excel 打开。

## 不做

- 宏表公式求值、执行 VBA
- 对话框控件增删改
- create 生成这两类表
- 把 `vbaProject.bin` 当对象编辑
- 打开 `.xlsm`

## 验收

手测编号 Xdm1–Xdm5。Excel 16.0：`xlsx-dialog-macro.ps1`，5 PASS / 0 FAIL。
