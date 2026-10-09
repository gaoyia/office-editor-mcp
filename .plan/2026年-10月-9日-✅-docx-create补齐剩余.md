# docx create 补齐剩余

状态：✅ 文档、实现与 Word 16.0 手测 Cs1–Cs9 已过
日期：2026-10-09

对应 `roadmap/docx.md` 待做序号 **40**。前置：`.plan/2026年-9月-27日-✅-docx-create补齐.md`。不要和 41–45 合成一份。

23 当时明确不进 create 的：目录、修订、超链接、书签、域、编号、公式、内容控件、批注回复。本期把其中**已有 insert / reply 写入**的接进 create，复用同一套 XML，不另发明。

没有 `insertRevision`。create **不写修订痕迹**，也不写 `w:trackRevisions`。

契约真源：`docs/json-contract.md`、`docs/json-contract-docx.md`、`docs/json-schema/docx.schema.json`、`docs/errors.md`、`docs/architecture.md`、`docs/opc.md`、`docs/checklist-docx.md`。

## 目标

一份 `document` JSON 能写出超链接、书签、复杂域、目录块、编号并绑到段落、行内/独立公式、内容控件，以及批注回复。Word 16.0 打开无修复。inspect 与单独发对应 op 相同。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | 锁 create 字段 | ✅ |
| 1 | 文档：分册、schema、架构、opc、清单、示例 | ✅ |
| 2 | create 复用既有 apply | ✅ |
| 3 | 库测试与 Word 16.0 手测 | ✅ Cs1–Cs9 |

## 契约（已锁）

- `paragraphs[].hyperlinks[]`：字段与 `insertHyperlink` 相同，不含 `op` / `id`。必须有 `text`。`url` 与 `anchor` 恰好一个。出现 `location` / `file` / `slide` / `ref` / `display` / `on` 报未知字段。页眉页脚段落同样允许。
- `paragraphs[].bookmarks[]`：`name` 可选（省略则 `insertBookmark` 分配 `ooxp{n}`）。`end` 可选，为**同一故事** `paragraphs` 的 1 起下标；省略则起止都是本段。`end` 小于本段下标、大于该故事段数，或不是正整数，报 `InvalidField`。不要 `id`。页眉页脚同样允许。
- `paragraphs[].fields[]`：`instr` 必填，`result` 可选，规则与 `insertField` 相同。不要 `id`。页眉页脚同样允许。
- `paragraphs[].maths[]`：`text` 必填非空，`kind` 可选 `inline`（缺省）或 `display`，规则与 `insertMath` 相同。不要 `id`。页眉页脚同样允许。
- `paragraphs[].contentControls[]`：字段与 `insertContentControl` 相同，不含 `op` / `id`。`text` 必填非空。页眉页脚同样允许。
- `document.numberings[]`：每项只要 `kind`（`decimal` 或 `bullet`），按数组顺序发 `insertNumbering`。空数组或省略则不写 numbering 部件。
- `paragraphs[].num`：可选。从 1 起的整数，指向 `document.numberings` 下标（不是 `num:{n}` 字符串）。省略则该段不绑编号。没有 `numberings` 或越界报 `InvalidField`。`ilvl` 可选，缺省 `0`，0–8。只给 `ilvl` 不给 `num` 报 `InvalidField`。页眉页脚同样允许。
- `document.tocs[]`：只挂在 `document` 上。每项：`kind` / `instr` 与 `insertToc` 相同；`after` 可选，为正文 `paragraphs` 的 1 起下标（插在该段之后）。省略 `after` 则插到正文末尾（`docx:document`）。`after` 为 0 或超过段数报 `InvalidField`。页眉页脚上出现 `tocs` 报未知字段。
- `comments[].replies[]`：可选。每项字段与 `comments[]` 相同，但不允许再套 `replies`。按 `replyComment` 写入。范围仍是该段自身。页眉页脚同样允许。
- 同一段上的 follow-up 顺序：骨架文字 / 图 / 框 / SmartArt，再图表，再 OLE，再脚注，再尾注，再批注，再该条批注的回复，再书签，再超链接，再域，再公式，再内容控件，再绑编号。全部故事写完后再写 `tocs`。`numberings` 在任何段落绑定之前写入。
- 仍不写：修订痕迹、题注、交叉引用、索引、参考文献、构建基块、嵌入字体、theme、页面背景 / 边框、连接线、框内再套框、批注的回复再回复。

## 不做

- create 写修订痕迹或打开跟踪修订
- 题注 / 交叉引用 / 索引 / 参考文献 / 构建基块 / 嵌入字体 / theme / 页面背景 / 页面边框 / 连接线（仍走 apply）
- 公式结构剩余、域类型补齐、内容控件类型补齐（41–43）
- 排版引擎、域求值、公式排版

## 验收

手测组号 **Cs1–Cs9**。示例 `docs/examples/docx-create-remain.json`。`pwsh -File manual-check\office\run.ps1 -Case docx-create-remain`：9 PASS / 0 FAIL。Word 16.0 打开无修复。
