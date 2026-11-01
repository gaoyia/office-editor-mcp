# chartex 另存时的 RPC 错误追查

状态：✅ 已完成。Word 那处是本库缺陷，PowerPoint 那处是 Office 自身行为。
日期：2026-11-02

2026-11-02 审计第八轮拆出。不进路线图。只出探测结论，不改库代码。

## 现状

C21 第 1 步的探测日志 `target/probe-c21-save.txt`：

- Word 16.0 打开本库写的漏斗图，`SaveAs2` 返回「远程过程调用失败」（`0x800706BE`），紧接着 `Quit` 返回「RPC 服务器不可用」（`0x800706BA`）。Word 进程已经不在，另存的文件写出来了。
- PowerPoint 16.0 改直方图数据后，图表数据工作簿 `Close` 返回 `0x800706BE`，之后 `SaveAs` 照常成功。
- C21 只记了文件写出来了，没有追查。

## 清单

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | Word 对照 | ✅ 本库 chartex 另存必崩，普通图表和 Office 原生 chartex 不崩 |
| 2 | PowerPoint 对照 | ✅ PowerPoint 原生柱形图同样让数据宿主 Excel 崩 |
| 3 | Windows 应用程序日志 | ✅ |
| 4 | 结论与后续 | ✅ 记缺陷，挂 docx 78 |

### 1. Word 对照

每个情况新起一个 Word 进程。脚本 `target/probe-rpc*.ps1`，日志 `target/probe-rpc*.txt`。

- 本库漏斗：`SaveAs2` 返回 `0x800706BE`，进程不在。
- 本库普通柱形图：另存正常。
- Word 的 `InlineShapes.AddChart2(-1, 123)` 拒绝漏斗类型，返回「调用失败」。改用 Excel 原生：`Shapes.AddChart2(-1, 123)` 成功，复制粘贴进 Word 空文档，另存正常；新进程重开再另存也正常。
- 本库七种布局只打开不另存，空闲 6 秒进程都在。
- 同一份漏斗重复三次，加漏斗、瀑布各一份：Word 空闲 6 秒都在，一调 `SaveAs2`，0.2 到 0.4 秒内崩。第一次 `SaveAs2` 先返回成功，0.3 秒后照样崩。普通柱形图另存正常。
- 本库瀑布图经 Word 另存（崩溃前已写出）的文件，再让 Word 另存：正常。

对本库瀑布图每次只改一处：

| 改动 | 另存 |
| --- | --- |
| 不改 | 崩 |
| 补 `theme1.xml` | 崩 |
| 内嵌工作簿路径改成 `../embeddings/` | 崩 |
| 包 `Requires="cx1"` 的 `AlternateContent`，Fallback 放占位文字 | 正常 |
| 同上，不写 Fallback | 正常 |
| 同上，Fallback 为空 | 正常 |

### 2. PowerPoint 对照

- 本库直方图：改一格后关数据工作簿，返回 `0x800706BE`；PowerPoint 还在，`SaveAs` 正常。
- PowerPoint 的 `Shapes.AddChart2(-1, 51)` 柱形图：同样步骤，`Close` 没有报错，`SaveAs` 正常。

### 3. 崩溃记录

Application 日志事件 1000：

- Word 每次崩溃：`WINWORD.EXE`，出错模块 `ntdll.dll`，异常码 `0xc0000374`（堆损坏）。时间都在 `SaveAs2` 开始后 0.2 到 0.4 秒。
- PowerPoint 两个情况各有一条 `EXCEL.EXE` 崩溃，异常码 `0xc0000005`。时间分别对上本库直方图和原生柱形图关数据工作簿的时刻。崩的是托管图表数据的 Excel，PowerPoint 自己没崩。
- 两条都是图表数据宿主的问题，原生图也会，不是本库文件引起的。

### 4. 结论

- Word：本库写进 docx 的 chartex 没有 `AlternateContent` 外壳，另存时 Word 崩溃。缺陷记录 `.bugfix-reports/2026年-11月-2日-❌-docx的chartex让Word另存时崩溃.md`，修复切片 `.plan/2026年-11月-2日-❌-docx-chartex包AlternateContent.md`（docx 78）。
- PowerPoint：Office 自身行为，不改。手测以后关数据工作簿报 RPC 错误时，先看 PowerPoint 进程是否还在。
- 之前的 chartex 手测只开不存，所以一直没暴露。

## 验收

- 四项都有实际返回值和日志记录，写在上面。
- 不改库代码。
- 残留的两个 `EXCEL.EXE`（图表数据宿主）已结束。没有残留的 `WINWORD.EXE`、`POWERPNT.EXE`。
