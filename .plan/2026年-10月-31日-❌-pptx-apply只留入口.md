# pptx apply 只留入口

状态：❌ 未开始。
日期：2026-10-31

不在三种格式的待做表里，不占待做序号。这是 `.plan/2026年-10月-30日-✅-大文件按领域拆开.md` 文末「后续」表里的 `pptx/apply.rs`。领域 JSON 不变，写出的 XML 不变。

## 为什么要做

`docs/architecture.md` 规定 `apply.rs` 只做入口、分发和共用装载，op 函数体放在对象模块。

`crates/ooxp/src/pptx/apply.rs` 现在 2381 行，却只有 12 个函数，说明大段实现直接写在 `match` 的分支里：分支里先找宿主部件、解析 XML、改、再写回。

## 目标

- 每个 `match` 分支只调用一个对象模块里的函数，分支里不再有成段的逻辑。
- 多个对象共用的宿主装载（`ensure_host_part`、`open_shape_host`、`write_shape_host`、`notes_or_slide_part` 一类）留在 `apply.rs`，或收进一个共用的宿主模块，不在每个对象模块里各抄一份。

## 不做

- 不改 op 的语义、id 或错误码。
- 不改找页的机制（`SlideMeta.start` 与 `slide_at_start`）。

## 清单

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 列出分支里的成段实现与去向，写回本节 | ❌ |
| 2 | 搬到对应对象模块 | ❌ |
| 3 | 共用宿主装载收口 | ❌ |
| 4 | 测试 | ❌ |

### 4. 验收

- `cargo test --workspace` 全过，不改任何期望值。
- `cargo fmt --all` 与 `cargo clippy --all-targets --all-features -- -D warnings` 通过。
- `pptx/apply.rs` 的 `match` 分支都只是一次调用加必要的错误包装。
- 写出的 XML 与改动前逐字节相同，不要求 Office。
