# docx NOTEREF 自动套书签

状态：❌ 未开始。
日期：2026-11-01

对应 `roadmap/docx.md` 待做序号 **75**。前置：NOTEREF 交叉引用（71）已过。

## 现状

- `insertCrossRef` 的 `kind: note` 写 ` NOTEREF {bookmark} \h `，`bookmark` 必填，而且必须已经套在 `w:footnoteReference` 或 `w:endnoteReference` 上。
- 库里没有 op 能放出这本书签：`insertBookmark` 只按整段放，书签名也不能以 `_` 开头。
- 71 的手测 `docx-noteref.ps1` 是用 Python 直接改 XML 套上的书签。
- 所以 `kind: note` 只能用在 Word 早已放好 `_Ref` 书签的文档上。新建文档里插脚注再引用它，单靠库做不到。

Word 16.0 的做法（71 的探测）：「插入交叉引用 → 脚注编号」时，Word 把隐藏书签 `_Ref` 加数字套在正文的引用 run 上，`bookmarkStart` 紧挨引用 run 之前，`bookmarkEnd` 紧挨之后。

## 目标

`insertCrossRef` 的 `kind: note` 可以直接点名一条脚注或尾注，书签由库照 Word 的做法套好。

## 设计

- 新增可选字段 `target`：`footnote:{id}` 或 `endnote:{id}`，只在 `kind: note` 时可用。
- `bookmark` 与 `target` 二选一。两个都给、都不给，或 `kind` 不是 `note` 却给了 `target`，报 `InvalidField`。
- 给 `target` 时：
  - 在主文档部件里找引用该注的 `w:footnoteReference` / `w:endnoteReference` 所在 run。
  - 这个 run 已经被一本 `_Ref*` 配对书签紧紧套住时，复用它。
  - 否则在 run 前后写一对书签，名字是尚未占用的 `_Ref` 加九位数字，和 `insertCaption` 的写法相同。书签 `w:id` 的取法与 `insertBookmark` 相同。
  - 再写 ` NOTEREF {书签名} \h `。
- 找不到该注报 `IdNotFound`。`target` 不是注 id 报 `InvalidId`。引用标记不在主文档部件里（页眉、页脚）报 `InvalidField`，因为 `REF` 系书签必须在主文档里。
- 主文档文本框里的引用标记算主文档部件，可以套。
- 只写 `\h`，不加开关。

## 不做

- 不改 `insertBookmark` 的段落粒度和命名规则。
- 不让 `insertFootnote` 顺手套书签，避免两处各写一套。
- 不写结果 run，不在库里算注号。

## 清单

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档：`insertCrossRef` 的 `target` 与报错 | ❌ |
| 2 | 实现：找引用 run、复用或新建 `_Ref` 书签 | ❌ |
| 3 | 测试 | ❌ |
| 4 | 手测：改 `docx-noteref.ps1`，不再用 Python 改 XML | ❌ |

### 1. 文档

- `docs/json-contract-docx.md` 的 `insertCrossRef`：加 `target` 一行，改掉「本 op 不新建这本书签，调用方要事先放好」。
- `docs/json-schema/src` 同步，重新生成 schema。
- `docs/checklist-docx.md` 的 NOTEREF 一节更新用例。

### 2. 实现

- 书签命名和 `w:id` 分配复用 `insertCaption`、`insertBookmark` 已有的函数，不另写一份。
- 新注的 `w:id` 由库分配，调用方在同一批里拿不到。契约写明：引用新插的注，先 inspect 拿到 `footnote:{id}`，再 apply `insertCrossRef`。

### 3. 测试

- 新建文档：`insertFootnote`，inspect 拿到注 id，再用 `target` 引用它。书签紧挨引用 run 前后，名字以 `_Ref` 开头，指令是 ` NOTEREF {书签} \h `。
- 尾注同样一条。
- 同一条注引用两次：第二次复用第一次的书签，不再新建。
- Word 另存的样本里已有 `_Ref` 书签：复用它。
- 引用标记在页眉里：报 `InvalidField`，不写文件。
- `bookmark` 与 `target` 同给、都不给、`kind: ref` 带 `target`：各报 `InvalidField`。
- 断言写出的字节，不只用本库读回。

### 4. 手测

- `docx-noteref.ps1` 改成只用库：create，`insertFootnote` / `insertEndnote`，inspect 拿注 id，再 `insertCrossRef` 带 `target`。
- Word 16.0 更新域后显示脚注编号 `1`、尾注编号 `i`，没有 Repair。

## 验收

- 上面的测试全过，`cargo test --workspace` 全过。
- `cargo fmt --all` 与 `cargo clippy --all-targets --all-features -- -D warnings` 通过。
- 覆盖表里 `insertCrossRef` 一行如有变化同步。
- 路线图把 75 挪到已过。
