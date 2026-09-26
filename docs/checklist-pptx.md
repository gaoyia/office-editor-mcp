# pptx 人工检查清单

代码阶段的单元测试覆盖 JSON 解析和 XML 不变量。下面这些必须用 Microsoft PowerPoint 打开一次，不能只靠测试绿。验收只认官方 Office，不再要求 WPS。

检查前用 create 或 apply 在本机生成文件，不要用改过扩展名的 zip 目录冒充。本机若 `PowerPoint.Application` 被 WPS 抢走，直接启动 `POWERPNT.EXE` 再挂 COM，不要走被抢的 ProgID。Office 16 的对象模型在 `mdiClass` 上。

C、I、A、Pp 各组由 `manual-check/office/cases/pptx-*.ps1` 自动判定，跑 `pwsh -File manual-check\office\run.ps1 -Case 'pptx-*'` 即可复现。O1 已放弃，与 xlsx / docx 相同。

## 本机结果（2026-09-27）

Microsoft PowerPoint 16.0 已打开，C1、C3、C4、C5–C7、I1–I5、A1–A5、Pp1–Pp7、Pc1–Pc7 均 PASS。本机若装了 WPS，`PowerPoint.Application` 会被抢走，测试仍直接启动 `POWERPNT.EXE`，从 `mdiClass` 挂 COM。O1 已放弃。2026-09-27 补跑 `pptx-pictures.ps1`，Pp1–Pp7 通过。同日补跑 `pptx-create-pictures.ps1`，C5–C7 通过。同日补跑 `pptx-charts.ps1`，Pc1–Pc7 通过。

| 序号 | 状态 | 结果 |
| --- | --- | --- |
| C1 | ✅ | PowerPoint 16.0 能打开 `manual-check/office/out/pptx-create.pptx`，标题栏无 Repair |
| C3–C4 | ✅ | 第一页 `标题` / `hello`，第二页 `第二页` |
| C5–C7 | ✅ | PowerPoint 16.0：inspect 第一张 `slide:1/pic:3`；同一页第二张 `slide:1/pic:4`；打开无 Repair，Type=13 的图为 2 |
| I1–I5 | ✅ | `pptx:presentation`、`slideCount=2`、无 scope 时没有 `shapes`、仍有 `pictureCount`、形状 id 为 `slide:1/shape:2` 且 `stable` |
| A1–A5 | ✅ | 目标框为「改过的正文」且仍加粗（`Bold=-1`），同页另一框与第二页未改 |
| Pp1–Pp7 | ✅ | PowerPoint 16.0：无 `pictures` 时 `pictureCount=1`；名单 id 为 `slide:1/pic:5`；换/插/删无修复；尺寸 144 磅；位置 72×108 磅；export 与 `/ppt/media/…` 字节一致 |
| Pc1–Pc7 | ✅ | PowerPoint 16.0：无 `charts` 时 `chartCount=1`；名单 id 为 `slide:1/chart:4`；改系列 / 插 / 删无修复；折线类型对上；尺寸 144×72 磅、位置 72×108 磅；export 与 embedding 字节一致 |
| O1 | 🚫 | 领域层不做原样保存，与 xlsx 清单相同。零脏写回见 OPC 库测试。PowerPoint 另存 create 产物见 `pptx-O1-ppt-resave` |

## create

用 `docs/examples/pptx-create.json` 生成 `deck.pptx`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| C1 | PowerPoint 打开 | 无修复提示、无「文件损坏」 |
| C3 | 第一页 | 两个文本框，可见文字分别为 `标题`、`hello` |
| C4 | 第二页 | 一个文本框，可见文字为 `第二页` |
| C5 | 带图 id | 用 `docs/examples/pptx-create-pictures.json`（换成本机 PNG）生成；inspect `pictures: true` 时第一张 id 为 `slide:1/pic:3` |
| C6 | 同一页第二张 | 同上，名单里有两张图 |
| C7 | PowerPoint 打开带图产物 | 无修复；图能看见 |

## inspect

对上面的文件发 `docs/examples/pptx-inspect-slides.json`（无 scope）和 `docs/examples/pptx-inspect.json`（`scope.slide` 为 `1`）。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| I1 | 根节点 | `id` 为 `pptx:presentation`，`slideCount` 为 `2` |
| I2 | 页名单 | 两页 id 为 `slide:1`、`slide:2`，`index` 为 `1`、`2` |
| I3 | 无 scope | 只发 path 时各页都没有 `shapes` 字段，仍有 `slideCount`、`pictureCount` 和 `slides` |
| I4 | 形状 id | 第一页两个形状 id 为 `slide:1/shape:{n}`，`stable` 为 `true` |
| I5 | 第一页文字 | 两个形状的 `text` 分别为 `标题`、`hello` |

## apply

准备一份 PowerPoint 亲手另存的样例（至少两页，第一页至少两个文本框，其中一个带加粗或别的 run 样式）。对它执行 `setShapeText`，改第一页某一个文本框的文字，id 用 inspect 返回的值。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| A1 | 打开 | Microsoft PowerPoint 能打开，无修复提示 |
| A2 | 目标形状 | 被点名的那个文本框可见文字变成请求里的 `text`。源文件若有多段，改成不含换行的 `text` 之后，框里不应再多出空行 |
| A3 | 同页未改形状 | 第一页其余文本框文字和位置还在 |
| A4 | 样式 | 目标形状若原先有加粗等 run 样式，改完后看起来还在（允许整框变成单一 run，但加粗还在） |
| A5 | 其余页 | 第二页文字和版式看起来不变；有动画的样例里动画仍绑在原形状上 |

## 幻灯片图

源文件必须是 PowerPoint 亲手另存，至少一张 `p:pic`。另备一张本地 PNG。不要用库 `create` 产物当带图样例。脚本：`manual-check/office/cases/pptx-pictures.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pp1 | inspect | 无 `pictures` 时根与各页有 `pictureCount`、没有名单；`pictures: true` 时 id 为 `slide:…/pic:…` |
| Pp2 | 换图 | `replacePicture` 后无修复；位置尺寸不变 |
| Pp3 | 插图 | `insertPicture` 往一页插图，能看见且无修复 |
| Pp4 | 删图 | `deletePicture` 后该图消失，无修复 |
| Pp5 | 尺寸 | `setPictureSize` 后 COM 或目视尺寸对上（允许 EMU 误差） |
| Pp6 | 位置 | `setPicturePos` 后图跟着走，无修复 |
| Pp7 | 导出 | `exportMedia` 与 `/ppt/media/…` 字节一致。不强制开 PowerPoint |

## 幻灯片图表

源文件必须是 PowerPoint 亲手另存，至少一张柱形图。不要用库 `create` 产物当唯一样例。脚本：`manual-check/office/cases/pptx-charts.ps1`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| Pc1 | inspect | 无 `charts` 时根与各页有 `chartCount`、没有名单；`charts: true` 时 id 为 `slide:…/chart:…`，系列值与 embedding 一致 |
| Pc2 | 改系列 | `setChartSeries` 后无修复；数据表是新值 |
| Pc3 | 插图 | `insertChart` 后能看见且无修复 |
| Pc4 | 删图 | `deleteChart` 后该图消失，无修复 |
| Pc5 | 改类型 | 改折线后按折线显示（允许样式重置），无修复 |
| Pc6 | 尺寸或位置 | `setChartSize` 或 `setChartPos` 后位置或尺寸对上，无修复 |
| Pc7 | 导出 | `exportChartData` 与 embedding 字节一致。不强制开 PowerPoint |

## 原样保存（opc 验收）

与 xlsx 清单相同：O1 第一期放弃；零脏写回在 OPC 层验收；O2 按 apply 看未改页。
