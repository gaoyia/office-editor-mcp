# xlsx create 第四轮

状态：❌
日期：2026-10-10

对应 `roadmap/xlsx.md` 待做序号 **38**。前置：本轮 32–37 的 apply 已过后再写代码。

`docs/json-contract-xlsx.md` 现状：create 不写列宽、行高、合并、数据验证、条件格式、工作表表格、透视表、定义名、打印、视图、排序、分组、拆分窗格、自定义视图、迷你图、OLE、切片器、连接、保护、组合、共享 / 数组公式、外部引用。批注不写回复。

## 目标

create 能按已有 apply 写出这些字段。不要另写一套 XML。公式格仍不写缓存 `<v>`，仍设 `fullCalcOnLoad`。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | 文档：create 字段表 | ❌ |
| 1 | 库：create 复用 apply | ❌ |
| 2 | 库测试与 Excel 16.0 手测 | ❌ |

## 已锁定

- 对应 apply 还没过的字段不要先写进 create。
- 不求值。不刷新透视。不刷新外部连接。

## 不做

- 公式求值、透视聚合引擎
- 从已有文件当模板复制

## 确认后要改的文档

- `docs/json-contract-xlsx.md`、`docs/json-schema/`、`docs/examples/`
- `docs/architecture.md`、`docs/checklist-xlsx.md`
- `roadmap/xlsx.md` 待做 38
