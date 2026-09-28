# xlsx create 页眉页脚

状态：✅  
日期：2026-09-27

从 `roadmap/xlsx.md` 拆出待做序号 16。apply 已能改六槽文本和 DrawingML 页眉图。

## 目标

create 的 `sheets[]` 能带六个槽位原文和页眉图，写入规则与 `setHeaderFooterText` / 页眉 `insertPicture` 相同。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档：create 字段 | ✅ |
| 2 | 实现 | ✅ |
| 3 | 库测试与 Excel 16.0 手测 | ✅ |

## 已锁定

- 不写 VML `legacyDrawingHF`。
- `kind: chartsheet` 仍不在 create 里写。
- create 用 `sheets[].headerFooters`（数组）。不要用 inspect 形状的单数 `headerFooter`。
- 同一 `kind`+`side` 出现两次报 `InvalidField`。
- 每一项至少要有非空 `text` 或一张图。
- 页眉图只要 `file`，可选 `cx` / `cy`；出现 `from` / `to` / `anchor` / 偏移报 `InvalidField`。
- 省略或空数组则不写 `headerFooter` / `drawingHF`。
- 先写文本槽，再按数组顺序往对应槽插图。

## 不做

- 解释 `&P` 为独立节点

## 前置

`.plan/2026年-9月-27日-✅-xlsx-页眉页脚.md`。

## 验收

JSON 生成带奇数页页眉文字的工作簿，Excel PageSetup 对得上，无修复。手测编号 C14–C16。Excel 16.0：3 PASS / 0 FAIL。
