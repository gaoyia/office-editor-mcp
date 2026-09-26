# pptx 人工检查清单

代码阶段的单元测试覆盖 JSON 解析和 XML 不变量。下面这些必须用 PowerPoint 和 WPS 演示各打开一次，不能只靠测试绿。

检查前用 create 或 apply 在本机生成文件，不要用改过扩展名的 zip 目录冒充。本机若 `PowerPoint.Application` 被 WPS 抢走，直接启动 `POWERPNT.EXE` 再挂 COM，不要走被抢的 ProgID。

## create

用 `docs/examples/pptx-create.json` 生成 `deck.pptx`。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| C1 | PowerPoint 打开 | 无修复提示、无「文件损坏」 |
| C2 | WPS 打开 | 同上 |
| C3 | 第一页 | 两个文本框，可见文字分别为 `标题`、`hello` |
| C4 | 第二页 | 一个文本框，可见文字为 `第二页` |

## inspect

对上面的文件发 `docs/examples/pptx-inspect-slides.json`（无 scope）和 `docs/examples/pptx-inspect.json`（`scope.slide` 为 `1`）。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| I1 | 根节点 | `id` 为 `pptx:presentation`，`slideCount` 为 `2` |
| I2 | 页名单 | 两页 id 为 `slide:1`、`slide:2`，`index` 为 `1`、`2` |
| I3 | 无 scope | 只发 path 时各页都没有 `shapes` 字段，仍有 `slideCount` 和 `slides` |
| I4 | 形状 id | 第一页两个形状 id 为 `slide:1/shape:{n}`，`stable` 为 `true` |
| I5 | 第一页文字 | 两个形状的 `text` 分别为 `标题`、`hello` |

## apply

准备一份 PowerPoint 或 WPS 亲手另存的样例（至少两页，第一页至少两个文本框，其中一个带加粗或别的 run 样式）。对它执行 `setShapeText`，改第一页某一个文本框的文字，id 用 inspect 返回的值。

| 序号 | 检查 | 通过标准 |
| --- | --- | --- |
| A1 | 打开 | PowerPoint 与 WPS 都能打开，无修复提示 |
| A2 | 目标形状 | 被点名的那个文本框可见文字变成请求里的 `text` |
| A3 | 同页未改形状 | 第一页其余文本框文字和位置还在 |
| A4 | 样式 | 目标形状若原先有加粗等 run 样式，改完后看起来还在（允许整框变成单一 run，但加粗还在） |
| A5 | 其余页 | 第二页文字和版式看起来不变；有动画的样例里动画仍绑在原形状上 |

## 原样保存（opc 验收）

xlsx 那份清单的 O1 / O2 对 PowerPoint 同样适用：打开样例再保存、一个未改部件都不改，PowerPoint 能打开，未改区域肉眼无差异。ooxp 第一期没有「什么都不改地保存」这个动作时，O1 记 SKIP，与 xlsx / docx 相同。
