# xlsx create 页眉页脚

状态：❌ 文档未写，实现与手测未开始  
日期：2026-09-27

从 `roadmap/xlsx.md` 拆出待做序号 16。apply 已能改六槽文本和 DrawingML 页眉图。create 不写页眉页脚。

## 目标

create 的 `sheets[]` 能带六个槽位原文和页眉图，写入规则与 `setHeaderFooterText` / 页眉 `insertPicture` 相同。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档：create 字段 | ❌ |
| 2 | 实现 | ❌ |
| 3 | 库测试与 Excel 16.0 手测 | ❌ |

## 已锁定

- 不写 VML `legacyDrawingHF`。
- `kind: chartsheet` 仍不在 create 里写，除非图表工作表计划已允许。

## 不做

- 解释 `&P` 为独立节点

## 前置

`.plan/2026年-9月-27日-✅-xlsx-页眉页脚.md`。

## 验收

JSON 生成带奇数页页眉文字的工作簿，Excel PageSetup 对得上，无修复。
