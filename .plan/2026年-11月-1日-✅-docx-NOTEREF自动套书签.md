# docx NOTEREF 自动套书签

状态：✅ 已完成。
日期：2026-11-01

对应 `roadmap/docx.md` 已过序号 **75**。前置：NOTEREF 交叉引用（71）已过。

## 现状

- `insertCrossRef` 的 `kind: note` 写 ` NOTEREF {bookmark} \h `，`bookmark` 必填，而且必须已经套在 `w:footnoteReference` 或 `w:endnoteReference` 上。
- 库里没有 op 能放出这本书签：`insertBookmark` 只按整段放，书签名也不能以 `_` 开头。
- 71 的手测 `docx-noteref.ps1` 是用 Python 直接改 XML 套上的书签。
- 所以 `kind: note` 只能用在 Word 早已放好 `_Ref` 书签的文档上。新建文档里插脚注再引用它，单靠库做不到。

Word 16.0 的做法（71 的探测）：「插入交叉引用 → 脚注编号」时，Word 把隐藏书签 `_Ref` 加数字套在正文的引用 run 上，`bookmarkStart` 紧挨引用 run 之前，`bookmarkEnd` 紧挨之后。

## 目标

`insertCrossRef` 的 `kind: note` 可以直接点名一条脚注或尾注，书签由库照 Word 的做法套好。

## 不做

- 不改 `insertBookmark` 的段落粒度和命名规则。
- 不让 `insertFootnote` 顺手套书签。
- 不写结果 run，不在库里算注号。
- create 的 `crossRefs` 仍只认 `bookmark`，不认 `target`。

## 清单

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档：`insertCrossRef` 的 `target` 与报错 | ✅ |
| 2 | 实现：找引用 run、复用或新建 `_Ref` 书签 | ✅ |
| 3 | 测试 | ✅ |
| 4 | 手测：改 `docx-noteref.ps1`，不再用 Python 改 XML | ✅ |

### 1. 文档

- `docs/json-contract-docx.md` 的 `insertCrossRef`：加 `target`。`bookmark` 与 `target` 二选一。给 `target` 时库套书签。
- 同一批 apply 里刚插入的注还没有可回读的 id。先 inspect 拿到 `footnote:{id}` 或 `endnote:{id}`，再另一次 apply。
- `docs/json-schema/src/docx/refs.json` 同步，`python docs/json-schema/build.py` 重生 schema。`bookmark` 不再是必填。
- `docs/errors.md` 补上二选一、注 id 和页眉页脚引用标记的报错。
- `docs/checklist-docx.md` 的 NOTEREF 一行改成库自己套书签。
- 覆盖表里 `insertCrossRef` 仍是段落 id，没有改。

### 2. 实现

- `InsertCrossRefOp` 的 `bookmark` 改为可选，另加 `target`（注种类和 `w:id`）。
- 两个都给、都不给，或 `target` 配了不是 `note` 的 `kind`，报 `InvalidField`（`CrossRefBookmarkOrTarget`）。
- `target` 不是 `footnote:{id}` / `endnote:{id}` 报 `InvalidId`。找不到该注报 `IdNotFound`。
- 在主文档部件里找直接包住引用标记的那个 run（文本框里取内层 run，不套外层 drawing）。
- 该 run 的前一个兄弟是 `_Ref*` 的 `bookmarkStart`、后一个兄弟是同 `w:id` 的 `bookmarkEnd` 时，复用这个名字。
- 否则在 run 前后插入一对书签。名字用 `insertCaption` 的 `_Ref` 加九位数字。`w:id` 用已有书签最大 id 加 1。
- 不调用 `insertBookmark`，也不用按整段放置的 `insert_bookmark_start` / `insert_bookmark_end`。
- 引用标记只在页眉或页脚里时报 `InvalidField`（`NoteMarkOutsideMain`）。正文文本框里的可以套。
- 再写 ` NOTEREF {书签名} \h `。

### 3. 测试

- `noteref_wraps_footnote_and_endnote_marks`：脚注和尾注各套一本 `_Ref`，书签紧挨引用 run，指令是 ` NOTEREF {书签} \h `。同一条注再引用一次，复用第一本书签。
- `noteref_reuses_a_tight_bookmark_and_ignores_a_loose_one`：手写紧邻的 `_RefFn1` 会复用；包住整段的 `_RefLoose` 不复用，另外套一本新的。样本按 71 记下的紧邻结构手写。
- `noteref_wraps_a_mark_inside_a_text_box`：书签落在 `txbxContent` 里的引用 run 两侧。
- `noteref_rejects_a_header_mark_and_a_bad_target`：页眉里的引用标记报 `InvalidField`，源文件字节不变。缺注、不是注 id、两个字段同给、都不给、`kind: ref` 带 `target`，分别报 `IdNotFound` 或 `InvalidField`。

### 4. 手测

- `docx-noteref.ps1` 只用库：create，`insertFootnote` / `insertEndnote`，inspect 拿注 id，再 `insertCrossRef` 带 `target`。
- Word 16.0 更新域后显示脚注编号 `1`、尾注编号 `i`。书签是 `_Ref000000001`、`_Ref000000002`。标题没有 Repair。

## 验收

- `cargo test --workspace` 全过。
- `cargo fmt --all` 与 `cargo clippy --all-targets --all-features -- -D warnings` 通过。
- 路线图把 75 挪到已过。
