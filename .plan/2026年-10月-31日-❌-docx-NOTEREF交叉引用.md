# docx NOTEREF 交叉引用

状态：❌ 未开始。
日期：2026-10-31

对应 `roadmap/docx.md` 待做序号 **71**。前置：交叉引用与题注（30）已过。

## 现状

- inspect 已经认得 `NOTEREF`：交叉引用名单里 `kind` 为 `note`（`docs/json-contract-docx.md` 第 368–369 行）。
- `insertCrossRef` 写着「本期不写 `NOTEREF`」，只写 `REF` 与 `PAGEREF`。

## 目标

`insertCrossRef` 可以写 `NOTEREF`：引用脚注或尾注的编号，指令参数（如 `\h`、`\f`、`\p`）按 Word 16.0 的写法。

## 清单

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 探测：Word 16.0「插入交叉引用 → 脚注编号」另存的书签与域指令 | ❌ |
| 2 | 文档：`insertCrossRef` 的 `kind: note` 与参数 | ❌ |
| 3 | 实现 | ❌ |
| 4 | 测试与手测 | ❌ |

### 1. 探测

记下 Word 给被引用的脚注或尾注引用标记加的隐藏书签（`_Ref…`）放在哪里、`NOTEREF` 的指令格式，以及「插入为超链接」「相对位置」对应的开关。

### 4. 测试与手测

- 引用脚注、引用尾注各一条测试：书签放对位置，域指令与探测结果一致，inspect 读回 `kind: note`。
- 手测：Word 16.0 打开产物，全选按 F9 更新域，显示的是对应注的编号，不报修复。

## 验收

- 上面的测试全过，`cargo test --workspace` 全过。
- `cargo fmt --all` 与 `cargo clippy --all-targets --all-features -- -D warnings` 通过。
- 路线图把 71 挪到已过。
