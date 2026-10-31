# docx NOTEREF 交叉引用

状态：✅ 已完成。
日期：2026-10-31

对应 `roadmap/docx.md` 已过序号 **71**。前置：交叉引用与题注（30）已过。

## 现状

- inspect 已经认得 `NOTEREF`：交叉引用名单里 `kind` 为 `note`。
- `insertCrossRef` 的 `kind: note` 写入 ` NOTEREF {bookmark} \h `。本 op 不新建 `_Ref` 书签。

## 目标

`insertCrossRef` 可以写 `NOTEREF`：引用脚注或尾注的编号。和 `ref` / `page` 一样只写 `\h`。

## 清单

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 探测：Word 16.0「插入交叉引用 → 脚注编号」另存的书签与域指令 | ✅ |
| 2 | 文档：`insertCrossRef` 的 `kind: note` 与参数 | ✅ |
| 3 | 实现 | ✅ |
| 4 | 测试与手测 | ✅ |

### 1. 探测

Word 16.0 把隐藏书签 `_Ref` 加数字套在正文的 `w:footnoteReference` 或 `w:endnoteReference` 上：`bookmarkStart` 紧挨引用 run 之前，`bookmarkEnd` 紧挨之后。脚注部件和尾注部件里没有这本书签。

同一条引用上，Word 写出的指令是：

- 只要超链接：` NOTEREF _Ref… \h `
- 超链接加相对位置：` NOTEREF _Ref… \p \h `
- 两个都不选：` NOTEREF _Ref… `
- 只要相对位置：` NOTEREF _Ref… \p `

`wdFootnoteNumber` 没有写出 `\f`。本库不另造开关，只写和 `ref` / `page` 相同的 `\h`。

### 4. 测试与手测

- 引用脚注、引用尾注各一条：书签套在引用标记上，指令为 ` NOTEREF {bookmark} \h `，inspect 读回 `kind: note`。
- 手测 `docx-noteref.ps1`：Word 16.0 更新域后显示脚注编号 `1`、尾注编号 `i`，没有 Repair。

## 验收

- 上面的测试全过，`cargo test --workspace` 全过。
- `cargo fmt --all` 与 `cargo clippy --all-targets --all-features -- -D warnings` 通过。
- 路线图把 71 挪到已过。
