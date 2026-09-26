# docx 人工检查清单

代码阶段的单元测试覆盖 JSON 解析和 XML 不变量。下面这些必须用 Microsoft Word 打开一次，不能只靠测试绿。验收只认官方 Office，不再要求 WPS。

检查前用 create 或 apply 在本机生成文件，不要用改过扩展名的 zip 目录冒充。

## 本机结果（2026-09-26）

Microsoft Word 16.0 已打开，用户确认通过。create、inspect、apply 三组都过了。O1 已放弃，见下表。本机若装了 WPS，`Word.Application` 会被抢走，测试仍直接启动 `WINWORD.EXE` 再挂 COM。A11 补过 `word/settings.xml` 之后，2026-09-26 用新 `docx-create.docx` 重跑，Word 报 `CompatibilityMode=15`。C2（WPS 打开）已放弃。

C、I、A 三组现在由 `manual-check/office/cases/docx-*.ps1` 自动判定，跑 `pwsh -File manual-check\office\run.ps1 -Case 'docx-*'` 即可复现，逐条结果落在 `manual-check/office/out/office-results.csv`。

| 序号 | 状态 | 结果 |
| --- | --- | --- |
| C1 | ✅ | Word 16.0 能打开 `manual-check/office/out/docx-create.docx`，标题栏无 Repair；`CompatibilityMode=15` |
| C2 | 🚫 | 不再要求 WPS。此前 WPS 文字已打开过，不作为验收条件 |
| C3–C4 | ✅ | 标题 1 / 「标题」，正文 `hello` |
| I1–I5 | ✅ | `docx:document`、`paraCount=2`、大纲 level 1、「标题」、`para:{hex}` 且 `stable`、无 scope 时没有 `paragraphs` |
| A1–A5 | ✅ | Word 打开 apply 产物：第二段为「改过的正文」且仍加粗，第一段仍是「第一章」/标题 1，第三段未改 |
| HF1–HF5 | ✅ | Word 另存带页眉页脚样例：名单与展开、改页眉后打开无修复，页眉变「新页眉」、正文与页脚未改 |
| O1 | 🚫 | 领域层不做原样保存，与 xlsx 清单相同。零脏写回见 OPC 库测试。Word 另存 create 产物见 `docx-O1-word-resave`。此前记的 `SaveAs2` 挂起，拆分后重跑没再出现，一秒内返回 |

这一轮手测样例没有文本框。带 `mc:AlternateContent` 或 `w:txbxContent` 的边界已由 `crates/ooxp/tests/docx_fidelity.rs` 覆盖；带文本框样例的 Word 手测尚未另加。

## create

用 `docs/examples/docx-create.json` 生成 `note.docx`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| C1 | Word 打开 | 无修复提示、无「文件损坏」；`CompatibilityMode` 为 `15` |
| C2 | WPS 打开 | 🚫 第一期不跑。验收只认 Microsoft Word |
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
| I5 | 无 scope | 只发 path 时没有 `paragraphs` 字段，仍有 `outline`、`paraCount`、`headers`、`footers` |

## apply

准备一份 Word 亲手另存的样例（至少两段正文、第一段是标题、第二段带加粗或别的 run 样式）。对它执行 `setParaText`，改第二段文字，id 用 inspect 返回的值。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| A1 | 打开 | Microsoft Word 能打开，无修复提示 |
| A2 | 目标段 | 被点名的那段可见文字变成请求里的 `text` |
| A3 | 未改段 | 第一段标题文字和标题样式还在 |
| A4 | 样式 | 目标段若原先有加粗等 run 样式，改完后看起来还在（允许整段变成单一 run，但加粗还在） |
| A5 | 未改部件 | 未点名的页眉页脚与样式表看起来不变 |

## 页眉页脚

准备一份 Word 亲手另存的样例：至少有默认页眉与默认页脚，页眉里有一段无域、无超链接的可见文字。不要用库 `create` 产物（create 不写页眉页脚）。对它 `inspect` 时带 `scope.headerFooter: true`，再用返回的页眉段落 id 做 `setParaText`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| HF1 | 名单 | 无 scope 时也有 `headers`、`footers`；容器带 `id`、`type`、`sections`、`paraCount`，不带 `paragraphs` |
| HF2 | 展开 | `headerFooter: true` 时页眉节点有 `paragraphs`，段落 id 形如 `header:default:1/para:…` |
| HF3 | 打开 | apply 改页眉后 Word 能打开，无修复提示 |
| HF4 | 目标 | 页眉可见文字变成请求里的 `text`；正文未改 |
| HF5 | 未改部件 | 未点名的页脚与 `styles.xml` 看起来不变 |

## 原样保存（opc 验收）

与 xlsx 清单相同：O1 第一期放弃；零脏写回在 OPC 层验收；O2 按 apply 看未改区域。
