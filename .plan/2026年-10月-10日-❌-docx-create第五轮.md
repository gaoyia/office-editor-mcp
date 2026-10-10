# docx create 第五轮

状态：❌
日期：2026-10-10

对应 `roadmap/docx.md` 待做序号 **56**。前置：本轮 53–55 的 apply 已过。

第四轮 create 不写修订、不写 `writePassword`、不写组合。框内表 / 框内域与直接格式当时也还没有 apply。

## 目标

create 复用已有 apply，写出框内表与框内链 / 书签 / 域 / 公式 / 控件、段边框底纹制表位、run 突出显示 / 删除线 / 上下标，以及组合。不要另写一套 XML。修订痕迹仍不写。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | 文档：create 字段 | ❌ |
| 1 | 库：create 复用 apply | ❌ |
| 2 | 库测试与 Word 16.0 手测 | ❌ |

## 已锁定

- 只复用已有 apply。不写修订痕迹。不写批注的回复再回复。
- `writePassword`：若 51 的 apply 已能写修改密码哈希，create 的 `protection.writePassword` 走同一套；否则仍报 `NotImplemented`，不要另写哈希。
- `textBoxes[].tables` 规则与 `document.tables` 相同，`after` 相对该框段落。
- 框内段落允许 `hyperlinks` / `bookmarks` / `fields` / `maths` / `contentControls` / `rubies`，规则与正文段落相同。
- 段落增加 `borders` / `shd` / `tabs`，与 `setParaFormat` 相同（不要 `op` / `id`）。
- create 的段落仍是一段一份 `text`，不能直接写 run 级突出显示。需要 run 级时：要么只走自定义字符样式，要么本切片允许 `runs[]`（每项 `text` 加 `setRunFormat` 那几个新字段）。二者选一，写进契约后不要并存。推荐 `runs[]`：有则忽略同段 `text`。
- `groups[]` 复用 `groupShapes`。成员用该故事已写出的图 / 框 / 形状 / 连接线 id，或 create 数组 1 起下标。至少 2 个。不要 `op` / `id`。
- 手测编号 **Cz**。脚本 `manual-check/office/cases/docx-create-round5.ps1`。不要和 Cy、Cs、Cf、C、Cd、Bx、Bf、Bd、Rh 撞。

## 不做

- 排版引擎、域求值、公式排版
- 框内再套框、注内 / 批注内表与域
- create 写修订、写回复再回复
- 从已有文件当模板复制

## 确认后要改的文档

- `docs/json-contract-docx.md`、`docs/json-schema/`、`docs/examples/`
- `docs/architecture.md`、`docs/checklist-docx.md`
- `roadmap/docx.md` 待做 56
