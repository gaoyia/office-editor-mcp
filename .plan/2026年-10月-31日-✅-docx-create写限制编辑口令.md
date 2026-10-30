# docx create 写限制编辑口令

状态：✅ 已完成。
日期：2026-10-31

对应 `roadmap/docx.md` 待做序号 **69**。前置：保护（27）已过。

## 现状

`setProtection` 给了 `password` 时，已经按 Word 16.0 写 `cryptAlgorithmSid="14"` 的 SHA-512 哈希。create 的 `document.protection` 原先一见到 `password` 就报 `NotImplemented`。

## 目标

create 的 `document.protection.password` 按 apply 的同一套规则写入。

## 不做

- 不改哈希算法，不写 ISO 的 `algorithmName` / `hashValue`。

## 清单

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档：删掉 create 遇 `password` 报 `NotImplemented` 的说法 | ✅ |
| 2 | create 复用 apply 的写哈希路径 | ✅ |
| 3 | 测试与手测 | ✅ |

### 实现

`apply_create` 不再单独拒绝 `password`。它和 `writePassword` 一样交给 `apply`。只给 `password`、不给限制编辑字段，仍报 `InvalidField`，和 apply 相同。

### 验收

- create 与 apply 写出的 `documentProtection` 属性名相同，盐不同。同一口令再 apply `enforced: false` 能解开。
- 2026-10-31 手测 `docx-create-password.ps1`。Word 16.0 打开无 Repair，`ProtectionType` 为只读，错口令失败，对口令解开。
- `cargo test --workspace` 全过。
- `cargo fmt --all` 与 `cargo clippy --all-targets --all-features -- -D warnings` 通过。
- 路线图 69 已挪到已过。
