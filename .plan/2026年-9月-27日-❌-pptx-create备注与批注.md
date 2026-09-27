# pptx create 备注与批注

状态：❌ 文档未写，实现与手测未开始  
日期：2026-09-27

从 `roadmap/pptx.md` 拆出待做序号 8。apply 已能写备注与现代批注。create 明确不写这两项。

## 目标

create 的 `slides[]` 能带备注文本与批注数组，写入规则分别与 `setNotesText`、`insertComment` 相同。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档：create 字段 | ❌ |
| 2 | 实现 | ❌ |
| 3 | 库测试与 PowerPoint 16.0 手测 | ❌ |

## 已锁定

- 不写旧式 §19.4 批注。
- 不写动画、切换，除非切换计划已过且 JSON 声明了。

## 不做

- create 写母版内容

## 前置

`.plan/2026年-9月-27日-✅-pptx-备注.md`、`.plan/2026年-9月-27日-✅-pptx-批注.md`。

## 验收

JSON 生成两页、第一页有备注和一条批注，PowerPoint 无修复，inspect 对得上。
