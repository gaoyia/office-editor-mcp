# 三家 chartex 认 cx2 与 cx7

状态：✅ 已完成。
日期：2026-11-02

对应三份路线图的跨格式已过 **C21**。前置：C20 已过。缺陷记录见 `.bugfix-reports/2026年-11月-1日-✅-Excel另存后漏斗直方帕累托不进图表.md`。

## 现状

Excel 16.0 另存把 chartex 图形框放进 `mc:AlternateContent`。C20 只认 `cx1`，漏斗、直方、帕累托落到 Fallback。

## 目标

三种格式都认 `cx1`、`cx2`、`cx7` 的 Choice，这些图回到 `charts`。

## 不做

- 不改本库写出的 chartex，写出仍不包 `AlternateContent`。
- 地图图表（`cx4`）仍不进前缀表。
- 不改按前缀认 Choice 的做法本身。

## 清单

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 探测：Word、PowerPoint 16.0 另存漏斗、直方、帕累托时的 `Requires` | ✅ |
| 2 | 文档 | ✅ |
| 3 | 前缀表加 `cx2`、`cx7` | ✅ |
| 4 | 测试 | ✅ |
| 5 | 手测 | ✅ |

### 1. 探测

- Word 16.0 打开本库漏斗图。`InlineShapes` 是 0（图是浮动的）。`SaveAs2` 返回 RPC `0x800706BE`，磁盘上的文件已经包了 `mc:AlternateContent`，`Requires="cx2"`。文档根上声明了 `cx1` 到 `cx8`。Fallback 是另一个 `w:drawing`。
- PowerPoint 16.0 另存直方和帕累托，两张都是 `Requires="cx7"`，命名空间 `http://schemas.microsoft.com/office/drawing/2016/5/13/chartex`。Fallback 是 `p:pic`。改单元格后 `Workbook.Close` 也返回 RPC，`SaveAs` 仍然写出了文件。
- 2026-11-02 追查这两处 RPC（`.plan/2026年-11月-2日-✅-chartex另存RPC错误追查.md`）：Word 那处是进程崩溃，原因是本库写的 docx chartex 没有 `AlternateContent` 外壳，另挂 docx 78 修。PowerPoint 那处是图表数据宿主 Excel 崩溃，PowerPoint 原生图也会，不是本库的问题。

### 2. 文档

- `docs/json-contract.md`：库进入 `cx1`、`cx2`、`cx7`。漏斗对应 `cx2`，直方和帕累托对应 `cx7`。`cx4` 不进。
- `docs/architecture.md` 没有列这些前缀，没有改。

### 3. 实现

- `xml::prefix_understood` 加 `cx2`、`cx7`。三种格式的 `alternate_child` 共用这张表。
- 进入 Choice 之后，Fallback 不再被当成图或形状。Excel 的海报是 `xdr:sp`，PowerPoint 的是 `p:pic`，Word 的是另一个 `w:drawing`。其它前缀的图片、形状计数不变。`cx4` 仍走 Fallback。

### 4. 测试

- `chartex_save_prefixes_enter_choice`：`cx1`、`cx2`、`cx7` 进 Choice，`cx4` 进 Fallback。
- `chartex_alternate_content_prefixes_stay_charts`：一张表三个锚点，图形框分别包在 `cx1`、`cx2`、`cx7` 里，Fallback 是形状。inspect 有三张图，`shapeCount` 是 0。对漏斗 `setChartSeries` 后外壳和海报还在，数值是 `[7, 8]`。
- `word_saved_funnel_choice_is_the_chart`：`w:drawing` 包进 `cx2`，图还在，`pictureCount` 是 0，外壳留下。
- `powerpoint_saved_histogram_choice_is_the_chart`：`p:graphicFrame` 包进 `cx7`，Fallback 是 `p:pic`，`pictureCount` 是 0，外壳留下。

### 5. 手测

- `chartex-cache.ps1` 的 Cca3：六种布局另存后 inspect 有 6 张，漏斗、直方、帕累托的第一个值是 42。
- Cca4：这三张 `setChartSeries` 后 Excel 打开没有 Repair，B2、B31、B41 是 7。

## 验收

- `cargo test --workspace` 全过。
- `cargo fmt --all` 与 `cargo clippy --all-targets --all-features -- -D warnings` 通过。
- 缺陷记录改成 ✅。三份路线图把 C21 挪到已过。
