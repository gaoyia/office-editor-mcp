# xlsx create 第五轮

状态：❌
日期：2026-10-10

对应 `roadmap/xlsx.md` 待做序号 **40**。前置：本轮 39 的 apply 已过。

第四轮 create 不写回复批注、不新建数据表公式。命名单元格样式当时还没有 apply。

## 目标

create 复用已有 apply，写出命名单元格样式，并可写一层批注回复。不要另写一套 XML。不求值。不新建数据表公式。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | 文档：create 字段 | ❌ |
| 1 | 库：create 复用 apply | ❌ |
| 2 | 库测试与 Excel 16.0 手测 | ❌ |

## 已锁定

- 骨架仍由现有 create 写出。其余解析成 `XlsxOp` 再 `xlsx::apply`。
- 根上 `cellStyles[]` 每项字段与 `insertCellStyle` 相同（不要 `op` / `id`）。
- 单元格可带 `namedStyle`：字符串，对应该 `cellStyles[]` 的 `name`，复用 `setCellNamedStyle`。与 `numFmt` / `font` / `fill` / `border` / `align` 不要同时给冲突项；冲突报 `InvalidField`。
- 批注 `replies[]` 一层，字段与 `replyComment` 相同。不要再套 `replies`。
- 出现 `dataTable` / `dataTableRef` 仍报 `InvalidField`。
- 手测编号 **C30–C32**。脚本 `manual-check/office/cases/xlsx-create-round5.ps1`。不要和 C25–C29、Xns 撞。

## 不做

- 公式求值、透视聚合引擎
- 新建数据表公式
- 从已有文件当模板复制
- Power Query / queryTable

## 确认后要改的文档

- `docs/json-contract-xlsx.md`、`docs/json-schema/`、`docs/examples/`
- `docs/architecture.md`、`docs/checklist-xlsx.md`
- `roadmap/xlsx.md` 待做 40
