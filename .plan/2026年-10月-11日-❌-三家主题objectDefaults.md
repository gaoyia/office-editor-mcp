# 三家主题 objectDefaults

状态：❌
日期：2026-10-11

对应 `roadmap/docx.md`、`roadmap/xlsx.md`、`roadmap/pptx.md` 跨格式 **C16**。前置：docx 66、xlsx 43、pptx 46。不要和配色槽、`fmtScheme` 摘要合成一份。不算主题排版引擎。

`a:theme` 根下 `objectDefaults` 是默认形状 / 线 / 文本框样式，不是 `clrScheme`。66 / 43 / 46 不收它。三家 theme 部件同一套 DrawingML，一份计划改 `theme.rs`。

## 目标

theme 上的 `objectDefaults` 能 inspect、能 apply。没有该元素则为 `null`。不把 `phClr` 展开成 RGB。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | 文档：`objectDefaults` 与 `setThemeObjectDefaults` | ❌ |
| 1 | 库：读写与测试 | ❌ |
| 2 | Office 16.0 手测 | ❌ |

## 已锁定

- inspect `theme` 在 `extraSchemes` 之后加 `objectDefaults`。没有 `a:objectDefaults` 则为 `null`。有则字段顺序：`sp`、`ln`、`tx`。
- 每一项取对应 `a:spDef` / `a:lnDef` / `a:txDef`。没有该子元为 `null`。有则对象字段顺序：`style`、`lnW`。`style` 取 `a:style` 下 `lnRef` / `fillRef` / `effectRef` / `fontRef` 的 `@idx` 整数；缺为 `null`。`lnW` 取 `spPr/a:ln/@w`，缺为 `null`。不展开填充渐变。
- 新 op `setThemeObjectDefaults`。id 必须是 `docx:document`、`xlsx:workbook` 或 pptx 的 `master:{n}` / `notesMaster` / `handout`。值为对象或 `null`（去掉整个 `objectDefaults`）。对象可给 `sp` / `ln` / `tx`，至少给一个。子对象可给 `style` 四索引与 `lnW`。未点名的定义留下。
- 没有 theme 部件报 `IdNotFound`，不新建。
- 手测编号 **Tdf** / **Xdf** / **Pdf**。脚本 `docx-theme-defaults.ps1` / `xlsx-theme-defaults.ps1` / `pptx-theme-defaults.ps1`。不要和 Th、Xth、Tfm 撞。

## 不做

- 把 `schemeClr` / `phClr` 算成最终 RGB
- 展开 `fmt` 渐变停点（不够用时补进 docx 50）
- 改各页已有形状去套默认样式（那是主题排版）

## 确认后要改的文档

- `docs/json-contract.md`、三份分册、`docs/json-schema/`
- 三份 checklist、`docs/errors.md`
- 三份 `roadmap/` 的 C16
