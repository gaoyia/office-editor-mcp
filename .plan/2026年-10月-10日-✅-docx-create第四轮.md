# docx create 第四轮

状态：✅
日期：2026-10-10

对应 `roadmap/docx.md` 待做序号 **52**。前置：本轮 46–51 的 apply 已过。

`docs/json-contract-docx.md`：create 已能按 apply 写出嵌入字体、连接线、无框形状、题注 / 交叉引用 / 索引 / 参考文献 / 构建基块、自定义样式、表格外观、拼音、东亚设置、例外编辑区，以及图 / 文本框上的 DrawingML 链。修订痕迹仍不写。`writePassword` 仍报 `NotImplemented`。

## 目标

create 能按已有 apply 写出嵌入字体、连接线、无框形状、题注 / 交叉引用 / 索引 / 参考文献 / 构建基块、自定义样式、表格外观、拼音、东亚设置、例外编辑区，以及图 / 文本框上的 DrawingML 链。不要另写一套 XML。修订痕迹仍不写。`writePassword` 仍报 `NotImplemented`。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | 文档：create 字段表 | ✅ |
| 1 | 库：create 复用 apply | ✅ |
| 2 | 库测试与 Word 16.0 手测 | ✅ |

## 已锁定

- 只复用已有 apply。不写修订痕迹。不写批注的回复再回复。不写 `writePassword`。
- 根上新增：`styles`、`sources`、`bibliographyStyle`、`bibliographies`、`captions`、`indexes`、`docParts`、`embeddedFonts`、`fontEmbed`、`eastAsianSettings`。不要 `op` / `id`。
- 段落新增：`shapes`、`connectors`、`rubies`、`citations`、`indexEntries`、`crossRefs`、`editRange`。页眉页脚段落同样允许，但 `captions` / `indexes` / `bibliographies` 只挂在 `document` 上。
- `pictures[].hyperlink` / `textBoxes[].hyperlink` 字段与 DrawingML `insertHyperlink` 相同：`url` 与 `anchor` 恰好一个，可选 `tooltip`，不要 `text`。
- `tables[].style` / `look` / `pos` / `cellMar` / `cellFormats` 复用 `setTableStyle` / `setTableFloat` / `setTableCellMar` / `setCellFormat`。`cells` 仍是二维字符串。
- `bibliographyStyle` 是字符串，取值与 `setBibliographyStyle.styleName` 相同。
- `editRange` 为 `true` 时写 `edGrp=everyone`；为对象时字段与 `insertEditRange` 相同（不要 `id`）。
- 手测编号 **Cy**。脚本 `manual-check/office/cases/docx-create-round4.ps1`。不要和 Cs、Cf、C、Cd、Pq 撞。

## 不做

- 排版引擎、域求值、公式排版
- 框内再套框
- create 写修订、写 `writePassword`、写组合
- 从已有文件当模板复制

## 已改文档

- `docs/json-contract-docx.md`、`docs/json-schema/`、`docs/examples/`
- `docs/architecture.md`、`docs/checklist-docx.md`
- `roadmap/docx.md` 待做 52 已挪到已过
