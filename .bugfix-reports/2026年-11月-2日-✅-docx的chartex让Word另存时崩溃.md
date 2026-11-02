# 本库写进 docx 的 chartex，Word 另存时进程崩溃

状态：✅ 已修复并有回归测试。修复切片见 `.plan/2026年-11月-2日-✅-docx-chartex包AlternateContent.md`
日期：2026-11-02
影响面：docx 里本库写出的所有 chartex（瀑布、漏斗、树状、旭日、直方、帕累托、箱线）。xlsx、pptx 不受影响。

## 怎么发现的

C21 第 1 步探测时，Word 16.0 另存本库写的漏斗图，`SaveAs2` 返回「远程过程调用失败」（`0x800706BE`），接着 `Quit` 返回「RPC 服务器不可用」。C21 只记了文件写出来了。

2026-11-02 审计第八轮时追查，见 `.plan/2026年-11月-2日-✅-chartex另存RPC错误追查.md`。

## 现象

- Word 打开本库写的 docx chartex，空闲 6 秒进程都在，7 种布局都一样。
- 一调用 `SaveAs2`，0.2 到 0.4 秒内进程崩溃。Application 日志事件 1000：`WINWORD.EXE`，出错模块 `ntdll.dll`，异常码 `0xc0000374`（堆损坏）。
- 另存的文件已经写到磁盘上。有一次 `SaveAs2` 先返回成功，0.3 秒后进程照样崩。
- 漏斗、瀑布各重复多次，每次都崩。

对照：

| 文件 | Word 另存 |
| --- | --- |
| 本库写的普通柱形图 | 正常 |
| Excel 原生漏斗粘进 Word | 正常，新进程重开再另存也正常 |
| 本库瀑布图经 Word 另存后的文件 | 正常 |

## 根因

本库把 chartex 写成裸的 `w:drawing`。Word 自己写的是：

```xml
<mc:AlternateContent>
  <mc:Choice Requires="cx1"><w:drawing>…</w:drawing></mc:Choice>
  <mc:Fallback><w:drawing>…PNG 海报…</w:drawing></mc:Fallback>
</mc:AlternateContent>
```

对本库的瀑布图每次只改一处，再让 Word 另存：

| 改动 | 结果 |
| --- | --- |
| 不改 | 崩 |
| 补主题部件 `theme1.xml` | 崩 |
| 内嵌工作簿路径改成 `../embeddings/` | 崩 |
| 包 `Requires="cx1"` 的 `AlternateContent`，Fallback 放占位文字 | 正常 |
| 包同样的外壳，不写 Fallback | 第一次正常，第二次崩 |
| 包同样的外壳，Fallback 为空 | 正常，再另存也正常 |

图表部件、样式、配色两边逐字相同，差的只是外壳。

没有 Fallback 时，Word 第一次另存会拆掉外壳，写回裸的 `w:drawing`。这份 Word 自己写出的裸文件再另存，同样崩。所以裸 chartex 不管谁写的，Word 另存都会崩；外壳里必须有 Fallback，Word 才会保留外壳。空的 `mc:Fallback` 就够。

## 顺带查清的另一处 RPC 错误

PowerPoint 关图表数据工作簿时的 `0x800706BE` 不是本库的问题。PowerPoint 自己用 `AddChart2` 插入的柱形图，关数据工作簿时托管它的 `EXCEL.EXE` 同样崩溃（`0xc0000005`），只是那次 COM 调用先返回了。这是 Office 自动化本身的行为。

## 修法

- docx 写 chartex 时外面包 `mc:AlternateContent`，Fallback 为空。前缀照 Word 粘贴 Excel 原生图后另存的结果：漏斗 `cx2`，其余 `cx1`。
- 外壳由 `xml::alternate_content` 构造，三种格式的 3D 外壳也改用它。
- 已经被本库写出、还没包外壳的旧文件，apply 不去补。契约写明先 `deleteChart` 再重插。

## 教训

手测只看「打开没有 Repair」，没有一条让 Office 另存。打开时 Word 很宽容，另存时才把问题暴露出来。

C21 的探测其实已经撞上了崩溃，但只记了「文件写出来了」。COM 返回 RPC 错误时，先查进程还在不在、系统日志有没有崩溃记录，再下结论。

一次另存不崩还不够。不写 Fallback 的写法第一次另存正常，第二次才崩。修法要拿 Office 的另存结果再另存一次。

## 回归测试

`crates/ooxp/tests/docx_charts.rs`：

- `inserted_chartex_is_wrapped_for_word`：七种布局的外壳、前缀和空 Fallback。
- `created_chartex_is_wrapped_for_word`：create 走同一条路径。
- `classic_chart_stays_bare`：普通图表不包外壳。
- `chart_ops_keep_the_chartex_shell`：外壳里的图改标题、图例、数据、增删系列、尺寸、绕排之后外壳还在，删图不留空外壳。

手测 `manual-check/office/cases/docx-chartex-save.ps1`（Cxs1–Cxs7）：七种布局 Word 16.0 另存后进程还在，日志没有崩溃，另存结果仍有外壳。
