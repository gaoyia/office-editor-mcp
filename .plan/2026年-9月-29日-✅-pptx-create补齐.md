# pptx create 补齐

状态：✅ 已通过 PowerPoint 16.0 手测  
日期：2026-09-29

对应 `roadmap/pptx.md` 待做序号 **19**。前置：动画写入、幻灯片节、超链接、媒体、切换均已过。不要和 18 合成一份。本期**不写 OLE**，不写自定义放映，不写讲义，不写多份母版。

不算放映引擎。create 只复用已有 apply：`setTransition`、`insertAnim`、`insertVideo` / `insertAudio`、`insertHyperlink`、`insertSlideSection`。不要另写一套 XML。

契约真源：`docs/json-contract.md`、`docs/json-contract-pptx.md`、`docs/json-schema/`、`docs/errors.md`、`docs/architecture.md`、`docs/opc.md`、`docs/checklist-pptx.md`。

## 目标

create 能在同一份 `presentation` JSON 里写出幻灯片节、形状级超链接、内嵌视频 / 音频、页切换和预设进入淡入。PowerPoint 16.0 打开无修复。inspect 与随后 apply 的语义与单独发那些 op 相同。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | 锁 create 字段：节 / 链 / 媒体 / 切换 / 动画 | ✅ |
| 1 | 文档：`slides[]` 与 `presentation.sections`、schema、checklist | ✅ |
| 2 | create 复用既有 apply | ✅ |
| 3 | 库测试与 PowerPoint 16.0 手测 | ✅ |

建议提交：`0 → 1 → 2 → 3`。第 0 步不必另开 Office 探测：XML 已由对应 apply 切片锁过。

## 契约（已锁）

- `slides[].transition`：对象，字段与 `setTransition` 相同（`type`、`dur`、`advClick`、`advTm`、`dir`、`invX`、`invY`），不要 `op` / `id`。省略则该页没有 `<p:transition>`。`type` 为 `null` 报 `InvalidField`（create 没有「清除」）。
- `slides[].videos[]` / `slides[].audios[]`：只要 `file`，可选 `x` / `y` / `cx` / `cy`，规则与 `insertVideo` / `insertAudio` 相同。不要 `op` / `id`。
- `slides[].shapes[].url` / `file` / `slide` / `tooltip`：与 pptx `insertHyperlink` 相同，三者必须恰好一个（`tooltip` 可选）。写在该形状 `cNvPr` 上。`text` / `anchor` / `location` / `ref` / `display` 出现报未知字段。create 上 `slide` 是 `slides[]` 的 1 起整数，不是 `slide:{n}`。
- `slides[].anims[]`：`type` 必须为 `fade`；`trigger` 可选，规则与 `insertAnim` 相同。绑定用 1 起的序号，且恰好一个：`shape` / `picture` / `video` / `audio` / `table` / `chart` / `smartArt`，对应该页 create 数组（`shapes` 缺省补的空框算第 1 个）。序号对不上报 `InvalidField`。不要 `op` / `id`。`slides[]` 上出现 `timing` 仍报未知字段。
- `presentation.sections[]`：`{ "name", "start" }`。`start` 是 `slides[]` 的 1 起下标，不是 `slide:{n}`。`name` 缺省报 `MissingField`，允许空串。`start` 必须严格递增，第一项可为 1。省略 `sections` 则不写 `p14:sectionLst`。第一项 `start==1` 时复用 `insertSlideSection` 的省略 start（包住全部页）。
- 页内顺序：先 `shapes`，再 `pictures`，再 `videos`，再 `audios`，再 `charts`，再 `smartArts`，再 `tables`，再 `notes`，再 `comments`，再形状超链接，再 `transition`，再 `anims`。全部页写完后再写 `sections`。
- `presentation` 根上出现 `hyperlinks` / `videos` / `audios` / `anims` / `transition` 仍报未知字段。`slides[]` 上出现 `oleObjects` / `hyperlinks` / `customShows` 仍报未知字段（链写在 `shapes[]` 上，不另开页级名单）。
- create 仍不写 OLE、自定义放映、讲义、多份母版。

## 不做（本期）

- OLE、自定义放映、讲义、多份母版
- run 级超链接、备注上的链
- 任意时间轴 JSON（只复用 `insertAnim` 的 fade）
- xlsx / docx 的 create 超链接（xlsx 契约已写「本期不写」）

## 验收

PowerPoint 16.0：`pptx-create-remain.ps1`，5 PASS / 0 FAIL。编号 **Cr1–Cr5**。
