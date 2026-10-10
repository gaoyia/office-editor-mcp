# docx create 第五轮

状态：✅
日期：2026-10-10

对应 `roadmap/docx.md` 待做序号 **56**。前置：本轮 53–55 的 apply 已过。不要和 53–55 合成一份。

第四轮 create 不写修订、不写 `writePassword`、不写组合。框内表 / 框内域与直接格式当时也还没有 apply。

## 目标

create 复用已有 apply，写出框内表与框内链 / 书签 / 域 / 公式 / 控件 / 拼音、段边框底纹制表位、run 突出显示 / 删除线 / 上下标，以及组合与 `writePassword`。不要另写一套 XML。修订痕迹仍不写。限制编辑口令 `password` 仍报 `NotImplemented`。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | 文档：create 字段 | ✅ |
| 1 | 库：create 复用 apply | ✅ |
| 2 | 库测试与 Word 16.0 手测 | ✅ |

## 已锁定

- 只复用已有 apply。不写修订痕迹。不写批注的回复再回复。
- `writePassword`：create 的 `document.protection.writePassword` 走 `setProtection` 同一套（UTF-16LE SHA-512）。`password` 仍报 `NotImplemented`。
- `textBoxes[].tables` 字段与 `document.tables` 相同。`after` 相对该框骨架段落。省略 `after` 或等于框内现有段数时插到框末。`after: 0` 报 `InvalidField`（`MinValue` min=1）。
- 框内允许 `hyperlinks` / `bookmarks` / `fields` / `maths` / `contentControls` / `rubies`，作用在框内第一段，规则与正文段落相同。不要另开 `textBoxes[].paragraphs[]`。
- 段落增加 `borders` / `shd` / `tabs`，与 `setParaFormat` 相同（不要 `op` / `id`）。不要把 `align` / 缩进整表加进 create。
- `runs[]`：有则忽略同段 `text`。空数组报 `InvalidField`（`EmptyArray`）。每项必须有 `text`（可空）。其余字段覆盖全部 `setRunFormat` 可写字段。
- `groups[]` 挂在 `document` / `headers[]` / `footers[]`。只要 `ids`，至少 2 个 inspect 风格字符串。成员必须是浮动图 / 文本框 / 无框形状 / 连接线。行内图不能进组。组合本身不进 inspect 树。
- 手测编号 **Cz**。脚本 `manual-check/office/cases/docx-create-round5.ps1`。不要和 Cy、Cs、Cf、C、Cd、Bx、Bf、Bd、Rh 撞。

Word 16.0 手测 Cz1–Cz7 通过。无 Repair。COM `Shapes` 有 `Type=6` 的组合。段底纹 RGB=65535。修改密码 `WriteReserved=True`。

## 不做

- 排版引擎、域求值、公式排版
- 框内再套框、注内 / 批注内表与域
- create 写修订、写回复再回复
- 从已有文件当模板复制

## 已改文档

- `docs/json-contract-docx.md`、`docs/json-schema/`、`docs/examples/`
- `docs/architecture.md`、`docs/checklist-docx.md`
- `roadmap/docx.md` 待做 56 已挪到已过
