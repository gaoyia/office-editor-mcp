# 三家 chartex 认 cx2 与 cx7

状态：❌ 未开始。
日期：2026-11-02

对应三份路线图的跨格式待做 **C21**。前置：C20 已过。缺陷记录见 `.bugfix-reports/2026年-11月-1日-❌-Excel另存后漏斗直方帕累托不进图表.md`。

## 现状

- Excel 16.0 另存时把 chartex 图形框放进 `mc:AlternateContent`。瀑布、树状、旭日、箱线是 `Requires="cx1"`，漏斗是 `cx2`，直方和帕累托是 `cx7`。
- 三种格式的 `AlternateContent` 都走 `xml::alternate_child`，认不认某个 Choice 看同一张前缀表 `prefix_understood`。C20 只往表里加了 `cx1`。
- 结果：Excel 另存过的漏斗、直方、帕累托落到 Fallback，inspect 当成普通形状，`charts` 里少了它们，`setChartSeries` 拿不到 id。
- C20 的回归测试手写了缓存缺失和定义名，但图形框没有包 `AlternateContent`。`Requires="cx1"` 这条分支也没有单元测试，只有 Excel 手测 `chartex-cache.ps1`。

## 目标

三种格式都认 `cx1`、`cx2`、`cx7` 的 Choice，Excel 另存过的 7 种 chartex 都回到 `charts`。

## 不做

- 不改本库写出的 chartex，写出仍不包 `AlternateContent`。Excel 16.0 打开已经没有 Repair。
- 地图图表（`cx4`）仍 🚫，不加进前缀表。
- 不改按前缀认 Choice 的做法本身。

## 清单

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 探测：Word、PowerPoint 16.0 另存漏斗、直方、帕累托时的 `Requires` | ❌ |
| 2 | 文档 | ❌ |
| 3 | 前缀表加 `cx2`、`cx7` | ❌ |
| 4 | 测试 | ❌ |
| 5 | 手测 | ❌ |

### 1. 探测

- 本库写一份 Word 漏斗、一份 PowerPoint 直方和帕累托，在 Office 里改一格数据另存。
- 记下图形框外面有没有 `AlternateContent`、`Requires` 是哪个前缀，以及 Fallback 里放的是什么。
- 结论写回本节。

### 2. 文档

- `docs/json-contract.md` 的 chartex 一节：删掉「库只进入 `cx1`」，写明认 `cx1`、`cx2`、`cx7`，各对应哪些布局。
- `docs/architecture.md` 里如有列出认得的 `Requires` 前缀，同步。

### 3. 实现

- `xml::prefix_understood` 加 `cx2`、`cx7`。
- 进入 Choice 后，原来落在 Fallback 的图片或形状不再进树。查清三种格式里会不会因此少计或多计 `pictures`、`shapes`，写回本节。

### 4. 测试

- 照 Excel 另存的结构手写 xlsx：一张表上三个锚点，图形框分别包在 `Requires="cx1"`、`cx2`、`cx7` 的 `AlternateContent` 里。各自声明 `xmlns:cx1` / `xmlns:cx2` / `xmlns:cx7`，Fallback 放一个形状。
  - inspect 的 `charts` 有三张，`layoutId` 正确，Fallback 形状不进 `shapes`。
  - 对 `cx2` 那张 `setChartSeries`，再 inspect 读回新值。写出时 `AlternateContent` 外壳和 Fallback 原样留下。
- docx、pptx 按第 1 步的结构各一条。Office 不包 `AlternateContent` 的，用手写外壳确认认得出即可。
- 断言写出的字节，不只用本库读回。

### 5. 手测

- 补探测用过的六种布局：Excel 另存后本库 inspect 有 6 张图；对漏斗、直方、帕累托各 `setChartSeries`，Excel 打开没有 Repair，图上是新值。
- 写进 `chartex-cache.ps1`，结果记进 `docs/checklist-xlsx.md`。

## 验收

- 上面的测试全过，`cargo test --workspace` 全过。
- `cargo fmt --all` 与 `cargo clippy --all-targets --all-features -- -D warnings` 通过。
- 缺陷记录改成 ✅，写上回归测试名。
- 三份路线图把 C21 挪到已过。
