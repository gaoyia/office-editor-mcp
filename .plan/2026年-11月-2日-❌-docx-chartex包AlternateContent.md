# docx chartex 包 AlternateContent

状态：❌ 未开始。
日期：2026-11-02

对应 `roadmap/docx.md` 待做序号 **78**。前置：C17、C21 已过。缺陷记录见 `.bugfix-reports/2026年-11月-2日-❌-docx的chartex让Word另存时崩溃.md`。

## 现状

- 本库把 docx 里的 chartex 写成裸的 `w:drawing`。Word 16.0 打开不报错，一另存进程就崩溃（`ntdll.dll`，`0xc0000374`）。
- 包上 `mc:AlternateContent`、`Choice Requires="cx1"` 之后另存正常。不写 Fallback、Fallback 为空、Fallback 放占位文字，三种都正常。
- Office 自己写的 Choice 前缀：瀑布、树状、旭日、箱线是 `cx1`，漏斗是 `cx2`。直方、帕累托在 Excel 和 PowerPoint 里是 `cx7`，Word 还没看过。
- 写图的代码在 `docx/charts.rs` 的 `build_chart_drawing`。3D 已经在写 `Requires="am3d"` 的外壳（`docx/model3d.rs`）。
- 读取已经认 `cx1`、`cx2`、`cx7` 的 Choice（C21）。

## 目标

docx 里新写的 chartex 都包 `AlternateContent`，前缀和 Office 一致，Word 另存不崩。

## 不做

- 不渲染海报，不写 PNG Fallback。
- 不改普通图表（`c:chart`），它们另存正常。
- 不改 xlsx、pptx。Excel、PowerPoint 另存本库的 chartex 都正常。
- 已被本库写出、没包外壳的旧文件，apply 不去改没点名的图。被点名改动的那张图要不要顺手包上，在第 2 步定下来写进契约。

## 清单

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 探测：Word 另存直方、帕累托时用的前缀；确认不写 Fallback 的另存结果 | ❌ |
| 2 | 文档 | ❌ |
| 3 | 写入：`insertChart` 与 create 的 chartex 包外壳 | ❌ |
| 4 | 事后改：各图表 op 在外壳里照常工作，外壳不丢 | ❌ |
| 5 | 测试 | ❌ |
| 6 | 手测：七种布局 Word 另存不崩 | ❌ |

### 1. 探测

- 把本库写的直方图、帕累托图包上 `cx1` 外壳，让 Word 另存，看另存结果里的 `Requires` 是不是 `cx7`。
- 不写 Fallback 的瀑布图经 Word 另存后，看 Word 有没有自己补 Fallback。
- 结论写回本节。

### 2. 文档

- `docs/json-contract.md` 的 chartex 一节：docx 写成 `mc:AlternateContent`，各布局用哪个前缀，Fallback 怎么写。
- 删掉 C21 计划和契约里「写出仍不包 `AlternateContent`」对 docx 的说法，xlsx、pptx 保持。
- `docs/opc.md` 如有写 docx 图表的 XML 结构，同步。
- 写明旧文件的处理。

### 3. 写入

- `build_chart_drawing` 在 chartex 时把 `w:drawing` 放进 Choice。前缀的命名空间声明放在 Choice 上，或主文档根上；`mc` 命名空间要在作用域里。
- 前缀按布局取：`cx1` / `cx2` / `cx7`，以第 1 步为准。
- 构造外壳的代码和 3D 那份共用，不另写一份。
- create 写 chartex 走同一条路径。

### 4. 事后改

- docx 上能对图表发的 op（改数据、增删系列、标题、图例、删图、尺寸、位置、绕排、链接等）逐个确认：能在 Choice 里找到图，改完外壳和 Fallback 原样留下。
- `deleteChart` 删掉整个 `AlternateContent`，不留空外壳。
- 位置编号、`chart:index:{k}` 退路照常。

### 5. 测试

- 七种布局各 `insertChart` 一次：`w:drawing` 在 Choice 里，`Requires` 对得上布局，前缀有声明。断言写出的字节。
- create 写一张 chartex，同样断言。
- 包了外壳的图上，第 4 步的 op 各一条：改完再 inspect 照常，外壳还在。`deleteChart` 后没有空外壳。
- 普通柱形图仍不包外壳。

### 6. 手测

- 新脚本 `docx-chartex-save.ps1`：七种布局各写一份，Word 16.0 打开、`SaveAs2`、等 3 秒。断言进程还在，Application 日志没有 `WINWORD.EXE` 的事件 1000。
- `chartex-edit.ps1` 里 Word 那一段补上另存和同样的断言，不再只看没有 Repair。
- 结果记进 `docs/checklist-docx.md`。

## 验收

- 上面的测试全过，`cargo test --workspace` 全过。
- `cargo fmt --all` 与 `cargo clippy --all-targets --all-features -- -D warnings` 通过。
- 缺陷记录改成 ✅，写上回归测试名。
- 路线图把 78 挪到已过。
- 结束后自己关掉残留的 `WINWORD.EXE`、`EXCEL.EXE`、`POWERPNT.EXE`。
