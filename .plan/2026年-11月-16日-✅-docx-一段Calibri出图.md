# Word 一段 Calibri 出图

状态：✅ 已完成
日期：2026-11-16
分类：部分实现

对应 C25 第 4 段，`roadmap/docx.md` 序号 **80**，父计划 `.plan/2026年-11月-14日-⚠️-docx-页面出图.md`。没有字的空段页已差 0。本切片画出单节、缺省 Calibri、一段、一行、无表格图和页眉的有字页。换行、多段分页、无文字表、页眉已另案交付，见对应计划（其中表格、页眉等仍为 ⚠️，不是整页差 0）；不要把后来已交付的能力写成仍未实现。

## 目标

这样的一页与 Word 16.0 文档导出 PDF 再 96 dpi 栅格化的金样对比。非文字像素 `maxDelta=0`。字形灰度仍按第 0 段暂缓，但字母位置和黑框大小要对齐。不把灰阶差算成通过，也不用近似图冒充整页差 0。

## 步骤

| 序号 | 内容 | 状态 |
| --- | --- | --- |
| 0 | 契约写明本切片接受的正文条件 | ✅ 见 `docs/json-contract.md` `exportPageImage` |
| 1 | 画出这一档有字页 | ✅ Windows 上走已有公开 GDI+ `Face::draw`（`AntiAliasGridFit`）。非 Windows 仍 `NotImplemented` |
| 2 | 手测 PDF→PNG 金样；位置和黑框对齐，非文字像素差 0 | ⚠️ 2026-11-17 05:18–05:19 Asia/Shanghai：8/8 PDF PASS，金样均为 794×1123；px 全 FAIL（灰阶）；bbox 6/8 精确对齐 |

验收时：非文字像素必须 `differ=0`、`maxDelta=0`。字形灰度仍按 C25 第 0 段暂缓，位置和黑框大小要对齐；不把灰阶差靠放宽容差算通过。幻灯片已交付的多行文本是布局近似、不保证文字逐像素一致，见已合入 main 的 `.plan/2026年-11月-16日-✅-pptx-多行文本布局近似.md`（用户 t66u 验收），不要在本切片里重开文字灰度 MaxDelta=0，也不借用该许可把 Word 字页标成整页通过。

## 库侧已测（未替代官方像素门）

`cargo test -p ooxp --test docx_page_image` 6 项通过（2026-11-17 约 04:18 Asia/Shanghai）。默认「A」墨迹盒 (96, 101) 9×9，与 C25 第 0 段对 Word 金样的测量一致。独立样本：`Hello` 左缘在 96..99（侧承）；12/24 磅「A」盒随字号变大；左缩进 720 twip → (144, 101) 9×9；段前 240 twip → (96, 117) 9×9；上/左边距 2160 twip → (144, 149) 9×9。超宽仍可能 `NotImplemented`。第二段与拉丁换行已另案交付，不要把本段早期记录当成当前库能力。排版原点是左边距+缩进、上边距+段前，不是按样本写死的坐标。

未宣称整页 `MaxDelta=0`。未改金样、未放宽容差、未再穷举已证伪的 DrawString / GetGlyphOutline / WPF / 亚像素平移。

## 官方对照（2026-11-17 Asia/Shanghai）

新增用例 `docx-calibri-image`（8 个独立样本：11 磅 A / Hello / Abc，12 磅与 24 磅 A，左缩进 720，段前 240，上/左边距 2160）已写入并在 `run.ps1` 注册。代表回归 `docx-page-image`（空白 A4 与 52 空段第 2 页）一并注册。库 `exportPageImage` 对空白页已写出 `docx-page-image-lib.png`。

随后 Word 16.0 公开 PDF 路径未落盘：

- 04:32:46–04:40:54 `ExportAsFixedFormat(..., 17)`：已打开 `docx-page-image.docx`（Pid 30208，Visible，无 Repair 窗），持续占 CPU（约 7.7s 升至 149s），工作集约 242 MB，无 PDF 写到 out 或常见目录。未加 `-CloseOffice`。
- 04:41:16–04:43:01 同一份空白 A4 再试公开 `SaveAs2(..., 17)`（Pid 17480）：同样打开成功后持续占 CPU，约 90 秒仍无 PDF。

未调用私有接口，未把 Office COM 当作库 renderer。因金样未写出，本轮没有官方 `differ` / `maxDelta` / 墨迹盒数字，步骤 2 保持 ❌，计划保持 ⚠️。官方用例仍走该公开路径；`Export-WordPdfBounded` 用 Start-Job/Wait-Job 60s 有界等待，超时记 FAIL blocked，只停作业 PowerShell，不杀 WINWORD、不 CloseOffice、不把超时算 PASS/SKIP。

## 不做

- 不靠加大容差、不改金样、不在对照图上补像素
- 不把像素写回包，也不塞进 inspect
- 不把其它段、其它对象类算进这一份的通过
- 官方对照只认 Microsoft Office 16.0，不认 WPS

- 不在这一份里做换行、多段分页、表格、图或页眉
- 不把幻灯片文字的布局近似结论直接套到 Word 整页通过标准上
- 不在金样导出挂起时无限改字体 API 参数
## 官方对照续（2026-11-17 04:59 Asia/Shanghai）

`Export-WordPdfBounded`（60s）下空白 A4 的 `ExportAsFixedFormat(..., 17)` **写出了 PDF**（约 2 秒内 `exists=True`；作业内 Word Pid 9872，未 `-CloseOffice`，未杀进程）。`pdf-to-png.ps1 -Dpi 96` 日志写 `w=794x1123`，但落盘金样实际为 **873×1235**（`HorizontalResolution≈105.6`），库图 794×1123。`pixel-diff` 原文 `size-differ left=873x1235 right=794x1123`（页 1 与 52 空段页 2 相同）。本机 `LOGPIXELSX=106`。PDF `/MediaBox[ 0 0 595.32 841.92]`，WinRT `PdfPage.Size` 为 793.76×1122.56；公开 `PdfPageRenderOptions.DestinationWidth/Height` 设为 794×1123 仍得到 873×1235。不改系统缩放、不把金样缩小后冒充差 0。步骤 2 仍 ❌；有界超时机制保留（挂起与尺寸阻塞都记 FAIL，不 PASS/SKIP）。

## 2026-11-17 05:12 后续

空页 `docx-page-image` 在修好 `pdf-to-png` 后已官方差 0（见父计划）。同轮 `docx-calibri-image` 在导出金样前因用例脚本写只读 `$PID` 崩溃（FAIL CRASHED），步骤 2 仍 ❌。下一刀先修该脚本变量名，再跑有字页 PDF→PNG。
## 官方对照（2026-11-17 05:18:34–05:19:44 Asia/Shanghai）

修好用例只读 `$PID`→`$paraId` 后，`docx-calibri-image` 8 独立样本全跑（有界 Export-WordPdfBounded，未杀 WINWORD）：

| 样本 | pdf | px 原文 | bbox |
| --- | --- | --- | --- |
| a11 | PASS | differ=30 maxDelta=28 | PASS lib=gold 96,101 9×9 |
| hello11 | PASS | differ=155 maxDelta=255 | FAIL lib=97,100 31×10 gold=30×10 宽 |
| abc11 | PASS | differ=107 maxDelta=221 | PASS 96,100 23×10 |
| sz12 | PASS | differ=32 maxDelta=28 | PASS 96,101 9×10 |
| sz24 | PASS | differ=106 maxDelta=137 | FAIL lib=96,106 18×20 gold=18×21 高 |
| indent | PASS | differ=32 maxDelta=62 | PASS 144,101 9×9 |
| before | PASS | differ=30 maxDelta=28 | PASS 96,117 9×9 |
| margin | PASS | differ=32 maxDelta=62 | PASS 144,149 9×9 |

残差分布（a11 代表）：`residualBBox` 与墨迹盒重合 9×9；**全部为等通道灰阶差**（nonGrayPair=0）；delta 直方图以个位数～28 为主。hello/sz24 另有 1 像素黑框差。按 C25 第 0 段字形灰度暂缓，**不加大 MaxDelta、不样本写死、不借用 PPT 文字近似**。步骤 2 保持未整页差 0。缺新的公开通用抗锯齿证据前不开放参数搜索。

## hello / sz24 几何（勿统称纯灰阶）

2026-11-17 05:18–05:19（Asia/Shanghai）官方八样本：除等通道灰阶残差外，**hello11** 墨迹盒宽 lib 31×10 vs gold 30×10（右侧 +1 px）；**sz24** 高 lib 18×20 vs gold 18×21（底边 −1 px）。这是布局/墨迹盒几何差，不是「8 FAIL 全是纯灰阶」。缺公开通用布局修复证据前不重开字体 AA 枚举；不改 MaxDelta/金样/样本硬编码。

## 2026-11-20 文档核对（不改绿）

引言里原先把换行、多段、表格、页眉写成仍 `NotImplemented`，与后来已交付的切片不符，已更正。本计划状态保持 ⚠️：hello11 后续字母约偏右 1px、sz24 底边多一行浅灰抗锯齿，未改绘制，也未把灰阶残差写成通过。

## 2026-11-20 用户初审通过

用户初审（schema `pr12-x-review-pass-v1`，revision 初审，对照 HEAD `3bc905df`）：一段 Calibri 出图通过。复查 hello11 / sz24 / a11：布局已齐，剩余为字形抗锯齿（hello11 墨迹宽库 31 对官方 30、sz24 底边多一行浅灰）。未发现新的可唯一锁定逻辑错误，未改绘制。按用户把剩余抗锯齿当作可接受残差，改绿。
