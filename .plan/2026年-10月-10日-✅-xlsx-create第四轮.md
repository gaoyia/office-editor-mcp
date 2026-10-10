# xlsx create 第四轮

状态：✅
日期：2026-10-10

对应 `roadmap/xlsx.md` 待做序号 **38**。前置：本轮 32–37 的 apply 已过。

## 目标

create 能按已有 apply 写出列宽、行高、合并、数据验证、条件格式、表格、透视、定义名、打印、视图、筛选、排序、分组、隐藏行列、拆分、自定义视图、迷你图、OLE、切片器、连接、外部引用、保护、允许编辑区域、组合，以及单元格样式与共享 / 数组公式。不要另写一套 XML。公式格仍不写缓存 `<v>`，仍设 `fullCalcOnLoad`。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | 文档：create 字段表 | ✅ |
| 1 | 库：create 复用 apply | ✅ |
| 2 | 库测试与 Excel 16.0 手测 | ✅ |

## 已锁定

- 骨架仍由现有 create 写出（格子、图、图表、页眉、批注、超链接、drawing）。其余字段解析成同一套 `XlsxOp`，再调用 `xlsx::apply`。
- 不求值。不刷新透视。不刷新外部连接。不新建数据表公式。批注不写回复。
- 单元格 `shared` / `sharedRef` / `array` / `arrayRef` 与 `setCell` 相同；`numFmt` / `font` / `fill` / `border` / `align` 与 `setCellStyle` 相同。出现 `dataTable` / `dataTableRef` 报 `InvalidField`。
- 工作簿根：`definedNames` / `connections` / `externalLinks` / `customViews` / `workbookProtection`。不要 `op` / `id`。
- 工作表：`cols` / `rows` / `merges` / `dataValidations` / `conditionalFormats` / `tables` / `pivots` / `print` / `view` / `autoFilter` / `sortState` / `groupRows` / `groupCols` / `outline` / `sheetProtection` / `protectedRanges` / `sparklines` / `oleObjects` / `slicers`。
- `groups[].shapes` / `textBoxes` / `connectors` 是该表对应数组的 1 起下标，合计至少 2 个。不要 `op` / `id`。
- 表格 / 透视 / 切片器的 inspect id 按 create 数组顺序：`tables[0]` 为 `table:1`。`slicers[].source` 用完整 id。
- 给了保护口令则写 SHA-512。手测不要 COM 打开带哈希产物。
- 手测编号 **C25–C29**。脚本 `xlsx-create-round4.ps1`。

## 不做

- 公式求值、透视聚合引擎
- 从已有文件当模板复制
- create 写回复批注、数据表公式
