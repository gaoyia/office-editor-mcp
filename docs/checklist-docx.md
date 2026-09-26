# docx 人工检查清单

代码阶段的单元测试覆盖 JSON 解析和 XML 不变量。下面这些必须用 Word 和 WPS 文字各打开一次，不能只靠测试绿。

检查前用 create 或 apply 在本机生成文件，不要用改过扩展名的 zip 目录冒充。

## 本机结果（2026-09-26）

WPS 文字与 Microsoft Word 16.0 均已打开，用户确认通过。create、inspect、apply 三组都过了，原样保存的 O1 验不了。本机 `Word.Application` 仍被 WPS 抢走，测试是直接启动 `WINWORD.EXE` 再挂 COM。create 产物在 Word 里以兼容模式打开（`CompatibilityMode=12`），无修复提示，原因见计划表的 A11。

C、I、A 三组现在由 `manual-check/office/cases/docx-*.ps1` 自动判定，跑 `pwsh -File manual-check\office\run.ps1 -Case 'docx-*'` 即可复现，逐条结果落在 `manual-check/office/out/office-results.csv`。

| 序号 | 状态 | 结果 |
| --- | --- | --- |
| C1 | ✅ | Word 16.0 能打开 `manual-check/office/out/docx-create.docx`，标题栏无 Repair；以兼容模式打开（`CompatibilityMode=12`） |
| C2 | ✅ | WPS 此前已过 |
| C3–C4 | ✅ | 标题 1 / 「标题」，正文 `hello` |
| I1–I5 | ✅ | `docx:document`、`paraCount=2`、大纲 level 1、「标题」、`para:{hex}` 且 `stable`、无 scope 时没有 `paragraphs` |
| A1–A5 | ✅ | Word 打开 apply 产物：第二段为「改过的正文」且仍加粗，第一段仍是「第一章」/标题 1，第三段未改 |
| O1 | ⚠️ | 验不了：O1 要的是「ooxp 打开再原样保存，未改部件 CRC 不变」，可 ooxp 第一期没有「什么都不改地保存」这个动作。退一步只验到 Word 能把 create 产物另存成功（`docx-O1-word-resave`）。此前记的 `SaveAs2` 挂起，拆分后重跑没再出现，一秒内返回 |

这一轮用的样例都没有文本框。带 `mc:AlternateContent` 或 `w:txbxContent` 的文件另有缺陷，见计划表的 A9，那类文件的 I1–I5 现在过不了。

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
