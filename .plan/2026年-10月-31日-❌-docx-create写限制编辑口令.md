# docx create 写限制编辑口令

状态：❌ 未开始。
日期：2026-10-31

对应 `roadmap/docx.md` 待做序号 **69**。前置：保护（27）已过。

## 现状

`docs/json-contract.md` 的 `setProtection`：apply 给了 `password` 时，已经按 Word 16.0 写 `cryptAlgorithmSid="14"` 的 SHA-512 哈希（遗留预处理加 100000 次迭代）。

同一节最后写着：「create 的 `document.protection` 复用同一套规则。出现 `password` 仍报 `NotImplemented`。」`docs/architecture.md` 的 create 一节也写着同样的限制。

create 和 apply 的能力在这里不对齐。

## 目标

create 的 `document.protection.password` 按 apply 的同一套规则写入。

## 不做

- 不改哈希算法，不写 ISO 的 `algorithmName` / `hashValue`。

## 清单

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档：删掉 create 遇 `password` 报 `NotImplemented` 的说法 | ❌ |
| 2 | create 复用 apply 的写哈希路径 | ❌ |
| 3 | 测试与手测 | ❌ |

### 1. 文档

改 `docs/json-contract.md` 的 `setProtection` 末段、`docs/json-contract-docx.md` 的 create 一节、`docs/architecture.md` 的 create 一节，以及 `docs/json-schema/src` 里对应的 `limits`，然后重新生成 schema。

### 3. 测试与手测

- create 带 `password`：`settings.xml` 里的哈希属性组和 apply 写的一致（盐不同，算法、迭代次数、属性名相同）；用同一口令再 apply 解开保护能通过校验。
- 手测：Word 16.0 打开产物，限制编辑生效，输入口令能解开。

## 验收

- 上面的测试全过，`cargo test --workspace` 全过。
- `cargo fmt --all` 与 `cargo clippy --all-targets --all-features -- -D warnings` 通过。
- 路线图把 69 挪到已过。
