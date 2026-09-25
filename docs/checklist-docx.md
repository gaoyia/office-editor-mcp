# docx 人工检查清单

代码阶段的单元测试覆盖 JSON 解析和 XML 不变量。下面这些必须用 Word 和 WPS 文字各打开一次，不能只靠测试绿。

检查前用 create 或 apply 在本机生成文件，不要用改过扩展名的 zip 目录冒充。

## create

用 `docs/examples/docx-create.json` 生成 `note.docx`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| C1 | Word 打开 | 无修复提示、无「文件损坏」 |
| C2 | WPS 打开 | 同上 |
| C3 | 第一段 | 样式为标题 1，文字为 `标题` |
| C4 | 第二段 | 正文，文字为 `hello` |

## inspect

对上面的文件发 `docs/examples/docx-inspect.json`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| I1 | 根节点 | `id` 为 `docx:document`，`paraCount` 为 `2` |
| I2 | 大纲 | `outline` 含第一段，`level` 为 `1`，`text` 为 `标题` |
| I3 | 段落 id | 两段都有 `para:{hex}`，`stable` 为 `true` |
| I4 | 第二段 | `text` 为 `hello` |
| I5 | 无 scope | 只发 path 时没有 `paragraphs` 字段，仍有 `outline` 和 `paraCount` |

## apply

准备一份 Word 或 WPS 亲手另存的样例（至少两段正文、第一段是标题、第二段带加粗或别的 run 样式）。对它执行 `setParaText`，改第二段文字，id 用 inspect 返回的值。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| A1 | 打开 | Word 与 WPS 都能打开，无修复提示 |
| A2 | 目标段 | 被点名的那段可见文字变成请求里的 `text` |
| A3 | 未改段 | 第一段标题文字和标题样式还在 |
| A4 | 样式 | 目标段若原先有加粗等 run 样式，改完后看起来还在（允许整段变成单一 run，但加粗还在） |
| A5 | 未改部件 | 页眉页脚、样式表看起来不变 |

## 原样保存（opc 验收）

xlsx 那份清单的 O1 / O2 对 Word 同样适用：打开样例再保存、一个未改部件都不改，Word 能打开，未改区域肉眼无差异。
