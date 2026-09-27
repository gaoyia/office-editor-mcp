# docx create 补齐

状态：✅ 文档、实现与 Word 16.0 手测已过  
日期：2026-09-27

从 `roadmap/docx.md` 拆出待做序号 0。create 已能写段落、标题、图、文本框、SmartArt、表、`default` / `even` / `first` 页眉页脚，以及 `paragraphs[].sectionType` 多节。还不写图表、脚注、尾注、批注。

## 目标

create 的 JSON 能声明图表、脚注、尾注、批注；多节继续用已有 `sectionType`。写入规则与对应 apply 相同。不要为 create 另发明一套 XML。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档：段落上 `charts` / `footnote` / `endnote` / `comments` | ✅ |
| 2 | 图表（骨架写完后走 `insertChart` 同一套写入） | ✅ |
| 3 | 注与批注（走 `insertFootnote` / `insertEndnote` / `insertComment`） | ✅ |
| 4 | 多节（已有 `sectionType`，本切片只并进同一份 JSON 验收） | ✅ |
| 5 | 库测试与 Word 16.0 手测 | ✅ |

## 已锁定

- apply 仍是改已有包。create 不读模板文件。先写现有骨架（段落、节、页眉、图、框、SmartArt、表），再按文档顺序对该骨架发与 apply 相同的插入。
- `paragraphs[].charts`：数组。字段与 `insertChart` 相同，但不含 `op` / `id`，也不含 xlsx / pptx 专用字段（`from` / `to` / `anchor` / `dataFrom` / `x` / `y`）。缺省与 `insertChart` 相同：`kind` 为 `anchor`，`wrap` 为 `square`。页眉页脚段落同样允许。框内、表内、注内本期 create 不写图表。
- `paragraphs[].footnote` / `paragraphs[].endnote`：可选字符串，当作该段末一条 `insertFootnote` / `insertEndnote` 的 `text`（可空）。省略则不写。同一段可以同时有脚注和尾注。页眉页脚段落同样允许。
- `paragraphs[].comments`：可选数组。每项字段：`text`（可空，省略当作 `""`）、`author`（省略 `"ooxp"`）、`initials`（省略 `"oo"`）、`date`（省略则不写 `w:date`）。每条范围的 start 与 end 都是**该段自身**，不跨段。不写回复。页眉页脚段落同样允许。
- 同一段上的写入顺序：骨架里的文字 / 图 / 框 / SmartArt，再图表，再脚注，再尾注，再批注。
- `sectionType` 契约不变。最后一段仍禁止带 `sectionType`。
- 仍不写：目录、修订、超链接、书签、域、编号、公式、内容控件、批注回复。
- 新建 `commentsExtended.xml` 的 Content Type 必须是 `application/vnd.openxmlformats-officedocument.wordprocessingml.commentsExtended+xml`。Word 16.0 把 `application/vnd.ms-word.commentsExtended+xml` 当成损坏。

## 不做

- create 写修订痕迹
- create 写 VML
- create 写跨段批注或回复
- create 往框内 / 表内 / 注内写图表或注

## 前置

嵌入图表、脚注尾注、批注、插删节已过。

## 验收

一份 JSON 生成带饼图、一条脚注、一条尾注、一条批注、两节的文档，Word 无修复，inspect 对得上。Word 16.0：`docx-create-full.ps1`，Cf1–Cf6。
