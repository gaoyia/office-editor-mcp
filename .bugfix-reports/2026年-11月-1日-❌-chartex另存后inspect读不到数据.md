# chartex 被 Excel 另存后，inspect 读出的分类和数值是空数组

状态：❌ 未修复。修复切片见 `.plan/2026年-11月-1日-❌-三家-chartex缓存缺失时按引用取值.md`
日期：2026-11-01
影响面：Excel 另存过的工作表 chartex（瀑布、漏斗、树状、旭日、直方、帕累托、箱线）。docx、pptx 内嵌的 chartex 待探测。

## 怎么发现的

2026-11-01 审计第七轮，对比 `.plan/2026年-10月-31日-✅-三家-chartex改数据与地图图表.md` 的探测记录和代码。

计划第 1 步记下了 Excel 16.0 改数据后另存的样子：

- `cx:f` 从 `Sheet1!$B$2:$B$4` 改成隐藏定义名 `_xlchart.v1.2`，定义名正文仍是原来的区域。
- 分类和系列名同样变成 `_xlchart.v1.0`、`_xlchart.v1.1`。
- 缓存用的 `cx:pt` 被去掉，只留 `cx:f`。

而 `crates/ooxp/src/chart/chartex.rs` 的 `series_json` 只读 `cx:lvl/cx:pt`。

## 现象

用临时脚本按上面的记录造样本：库写一张瀑布图，再把三处 `cx:f` 换成 `_xlchart.v1.N`，删掉 `cx:lvl`，在 `workbook.xml` 补三个 `hidden="1"` 的定义名。

- inspect：系列名 `W` 还在（来自 `cx:v`），`categories` 和 `values` 都是 `[]`。
- `setChartSeries` 写 `[7, 8, 9]`：成功。B11:B13 变成 7、8、9，说明写入已经能顺着定义名找到单元格。
- 再 inspect：仍是 `[]`。

模型看到的是一张没有数据的图，改了数据也读不回来。

## 根因

`chart::series_json` 带一个 `read_cells` 回调：普通图表先按 `c:f` 原文（含定义名）读单元格，读不到再退回缓存。chartex 走的是开头那条分支：

```rust
if space.child_by_local("chartData").is_some() {
    return Ok(chartex::series_json(chart_id, space));
}
```

回调没有传进去。`chartex::series_json` 只认缓存，缓存没了就是空数组。

## 修法（计划）

- 先在 `docs/json-contract.md` 的 chartex 一节写明：inspect 先按 `cx:f` 读单元格（含 `_xlchart.v1.N` 这类定义名），读不到再用 `cx:lvl` 缓存。和普通图表一致。
- `chartex::series_json` 接收同一个 `read_cells`，`strDim`、`numDim`、`txData` 的 `cx:f` 都走它。
- docx、pptx 内嵌工作簿的读法照普通图表接上。先探测 Word 与 PowerPoint 另存时会不会同样去掉缓存。

## 教训

和 2026-10-30 那三份是同一条：拿自己写出的文件判自己的卷。

这次更糟一点：计划的探测已经把 Excel 另存后的结构写进了正文，测试和手测却都只用本库写出的文件，没有拿探测结果造一份样本回读。审计时我的抽查也只用了本库写出的文件，所以第一轮没看出来。

探测记下了 Office 会怎么改写 XML，就要有一条测试用那份结构回读。

## 回归测试（计划）

- xlsx：手写一份照 Excel 另存结构的 chartex（`_xlchart.v1.N`、无 `cx:lvl`、隐藏定义名），inspect 读出单元格里的值，`refs` 时给出定义名原文。
- 同一份样本 `setChartSeries` 后再 inspect，读回新值。
- `cx:f` 是直接区域、`cx:lvl` 缺失时也读单元格。
- docx、pptx 按探测结论各一条。
