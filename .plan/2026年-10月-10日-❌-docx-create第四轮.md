# docx create 第四轮

状态：❌
日期：2026-10-10

对应 `roadmap/docx.md` 待做序号 **52**。前置：本轮 46–51 的 apply 已过后再写代码。

`docs/json-contract-docx.md` 现状：create 不写修订痕迹、题注、交叉引用、索引、参考文献、构建基块、嵌入字体、theme、页面背景、页面边框、连接线。也不写批注的回复再回复。

## 目标

create 能按已有 apply 写出 theme、页面背景与边框、嵌入字体、连接线、题注 / 交叉引用 / 索引 / 参考文献 / 构建基块。修订痕迹仍可不写，除非 51 已提供稳定写入规则。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | 文档：create 字段表 | ❌ |
| 1 | 库：create 复用 apply | ❌ |
| 2 | 库测试与 Word 16.0 手测 | ❌ |

## 已锁定

- 只复用已有 apply。对应 apply 还没过的字段不要先写进 create。
- 不写 VML。不打开跟踪修订除非契约明确允许。

## 不做

- 排版引擎、域求值、公式排版
- 框内再套框

## 确认后要改的文档

- `docs/json-contract-docx.md`、`docs/json-schema/`、`docs/examples/`
- `docs/architecture.md`、`docs/checklist-docx.md`
- `roadmap/docx.md` 待做 52
