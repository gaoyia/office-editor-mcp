# create 解析与写出分开：docx

状态：❌ 未开始。
日期：2026-10-31

不在三种格式的待做表里，不占待做序号。这是 `.plan/2026年-10月-30日-✅-大文件按领域拆开.md` 文末「后续」表里的 `docx/create.rs`。领域 JSON 不变，写出的包不变。

三种格式各一份计划：docx、xlsx、pptx。顺序不限，互不依赖；第一份做的那一种顺带改 `docs/architecture.md`。

## 为什么要做

`docx/create.rs` 把「把领域 JSON 解析成结构体」和「把结构体写成 XML 部件」堆在同一个入口文件里：

| 文件 | 行数 | `parse_*` 函数 | 写出函数 |
| --- | --- | --- | --- |
| `docx/create.rs` | 3998 | 34 | 约 9 个 `*_xml`，另有 `create_doc.rs`、`create_para.rs` |

请求解析按架构规定应该在 `request/` 目录下按领域拆开，apply 已经这样做了，create 还没有。

## 目标

- create 的字段解析搬进 `docx/request/`，按领域拆成子文件，和 apply 的解析放在一起。能共用 `op_fields` 的字段读取就共用。
- `create.rs` 只留入口：拼最小骨架，再按顺序复用已有 apply op。
- 写出骨架 XML 的函数按对象归位；没有对象模块的，留在 create 自己的写出文件里。
- create 旁边的辅助文件按领域重新划分，名字说明它装的是什么。

## 不做

- 不改 create 的字段、默认值、错误码或写出的 XML。
- 不把 create 改成「全部走 apply」：骨架仍由 create 直接写。

## 清单

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档：`docs/architecture.md` 写明 create 的解析位置（三份里第一份做） | ❌ |
| 2 | 列出解析函数与写出函数的去向，写回本节 | ❌ |
| 3 | 搬移 | ❌ |
| 4 | 测试 | ❌ |

### 4. 验收

- docx 的 create 测试与 fidelity 测试全过，不改期望值；`cargo test --workspace` 全过。
- 用 `docs/examples/` 里覆盖字段最多的 docx create 示例生成一份文件，改动前后逐字节相同。
- `cargo fmt --all` 与 `cargo clippy --all-targets --all-features -- -D warnings` 通过。
- `docx/create.rs` 不再装着成段的字段解析。不要求 Office。