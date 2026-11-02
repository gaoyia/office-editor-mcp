# docx chartex 包 AlternateContent

状态：✅ 已完成。
日期：2026-11-02

对应 `roadmap/docx.md` 已过序号 **78**。前置：C17、C21 已过。缺陷记录见 `.bugfix-reports/2026年-11月-2日-✅-docx的chartex让Word另存时崩溃.md`。

## 现状

- 本库把 docx 里的 chartex 写成裸的 `w:drawing`。Word 16.0 打开不报错，一另存进程就崩溃（`ntdll.dll`，`0xc0000374`）。
- 写图的代码在 `docx/charts.rs` 的 `apply_insert`。3D 已经在写 `Requires="am3d"` 的外壳。
- 读取已经认 `cx1`、`cx2`、`cx7` 的 Choice（C21）。找图、删图的函数已经会进入 Choice，删图时整个外壳一起删。

## 目标

docx 里新写的 chartex 都包 `AlternateContent`，前缀和 Word 一致，Word 另存不崩，另存之后外壳还在。

## 不做

- 不渲染海报，不写 PNG Fallback。
- 不改普通图表（`c:chart`），它们另存正常。
- 不改 xlsx、pptx。Excel、PowerPoint 另存本库的 chartex 都正常。
- 本库以前写出、没包外壳的旧文件，apply 不去补，包括被点名改动的那张。契约写明先 `deleteChart` 再重插。

## 清单

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 探测：Word 用的前缀；不写 Fallback 的另存结果 | ✅ 漏斗 `cx2`，其余 `cx1`；Fallback 必须写 |
| 2 | 文档 | ✅ |
| 3 | 写入：`insertChart` 与 create 的 chartex 包外壳 | ✅ |
| 4 | 事后改：各图表 op 在外壳里照常工作，外壳不丢 | ✅ 现有找图逻辑已经认外壳，代码不用改 |
| 5 | 测试 | ✅ |
| 6 | 手测：七种布局 Word 另存不崩 | ✅ |

### 1. 探测

2026-11-02，Word 16.0。脚本 `target/probe-rpc-*.ps1`，不提交。

- 前缀：Excel 原生直方（`AddChart2` 类型 118）、帕累托（122）粘进 Word 另存，都是 `Requires="cx1"`。漏斗（123）是 `cx2`。Excel 自己的文件里直方和帕累托是 `cx7`，Word 不用。
- Fallback 必须写：
  - 只有 Choice、没有 Fallback 的外壳，Word 第一次另存不崩，但会拆掉外壳写回裸 `w:drawing`。这份 Word 写出的裸文件再另存，照样崩。
  - Fallback 为空的外壳，Word 第一次另存保留外壳；拿这份结果用新进程再另存，不崩，外壳还在。
- 所以写成空的 `mc:Fallback`，不写海报。

### 2. 文档

- `docs/json-contract.md` 的 chartex 一节：docx 外壳、前缀、空 Fallback、不包外壳会崩、旧文件先删再插。
- `docs/opc.md` 的 docx 图表一节补上外壳。
- `docs/architecture.md` 的「标记兼容性」：新写外壳用 `xml::alternate_content`。

### 3. 写入

- `xml::alternate_content(prefix, ns, choice_child, fallback)` 新写外壳，前缀在 Choice 上声明，`fallback` 为 `None` 时写空 Fallback。
- `docx/charts.rs` 的 `apply_insert` 设完绕排后，chartex 交给它包一层，前缀由 `chartex_choice` 按布局取。create 走同一条路径。
- 三种格式的 3D（`docx/model3d.rs`、`pptx/model3d.rs`、`xlsx/model3d.rs`）原先各自手写外壳，改成调用同一个函数，写出的字节不变。

### 4. 事后改

`find_chart_walk`、`find_chart_mut_walk` 已经进入 `alternate_child` 选出的支，`remove_chart_walk` 删除时整个外壳一起删，空 run 也去掉。全部 1509 个现有测试在改动后照常通过。

## 测试

`crates/ooxp/tests/docx_charts.rs`：

- `inserted_chartex_is_wrapped_for_word`：七种布局各插一次，文档里只有一个外壳，Choice 声明并要求对应前缀，里面是 chartex 的 `w:drawing`，后面是 `<mc:Fallback/>`。inspect 的 `layoutId` 正确。
- `created_chartex_is_wrapped_for_word`：create 写漏斗，外壳是 `cx2`。
- `classic_chart_stays_bare`：插普通柱形图，文档里没有 `AlternateContent`。
- `chart_ops_keep_the_chartex_shell`：在包了外壳的瀑布图上依次 `setChartTitle`、`setChartLegend`、`setChartSeries`、`addChartSeries`、`deleteChartSeries`、`setChartSize`、`setChartWrap`，每步之后外壳还在、inspect 认得出。最后 `deleteChart`，外壳和空 run 都不留。

撤掉 `charts.rs` 的改动后，前两条和最后一条失败。

## 手测

- 新脚本 `manual-check/office/cases/docx-chartex-save.ps1`（Cxs1–Cxs7）：七种布局 Word 16.0 打开没有 Repair，`SaveAs2` 后进程还在，日志没有崩溃，另存结果仍有外壳。
- `chartex-edit.ps1` 的 docx-Cx1 补上另存，同样断言。
- 判据写成 `manual-check/lib/office-com.ps1` 的 `Save-WordDocChecked`，两个脚本共用。
- 2026-11-02 跑这两个脚本：PASS 10，FAIL 0。漏斗和瀑布的另存结果再用新进程另存一次，也不崩，外壳还在。

## 验收

- `cargo test --workspace` 全过。
- `cargo fmt --all` 与 `cargo clippy --all-targets --all-features -- -D warnings` 通过。
- 缺陷记录改成 ✅。路线图把 78 挪到已过。
- 没有残留的 `WINWORD.EXE`、`EXCEL.EXE`、`POWERPNT.EXE`。
