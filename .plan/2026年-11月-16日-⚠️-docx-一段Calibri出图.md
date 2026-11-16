# Word 一段 Calibri 出图

状态：⚠️ 库已画出单节单段一行的不透明拉丁 Calibri；官方 Word PDF 金样本轮未写出
日期：2026-11-16
分类：部分实现

对应 C25 第 4 段，`roadmap/docx.md` 序号 **80**，父计划 `.plan/2026年-11月-14日-⚠️-docx-页面出图.md`。没有字的空段页已差 0。本切片画出单节、缺省 Calibri、一段、一行、无表格图和页眉的有字页。换行、多段、表格、图、页眉仍 `NotImplemented`。

## 目标

这样的一页与 Word 16.0 文档导出 PDF 再 96 dpi 栅格化的金样对比。非文字像素 `maxDelta=0`。字形灰度仍按第 0 段暂缓，但字母位置和黑框大小要对齐。不把灰阶差算成通过，也不用近似图冒充整页差 0。

## 步骤

| 序号 | 内容 | 状态 |
| --- | --- | --- |
| 0 | 契约写明本切片接受的正文条件 | ✅ 见 `docs/json-contract.md` `exportPageImage` |
| 1 | 画出这一档有字页 | ✅ Windows 上走已有公开 GDI+ `Face::draw`（`AntiAliasGridFit`）。非 Windows 仍 `NotImplemented` |
| 2 | 手测 PDF→PNG 金样；位置和黑框对齐，非文字像素差 0 | ❌ 官方金样本轮未写出，见下方记录 |

验收时：非文字像素必须 `differ=0`、`maxDelta=0`。字形灰度仍按 C25 第 0 段暂缓，位置和黑框大小要对齐；不把灰阶差靠放宽容差算通过。幻灯片已交付的多行文本是布局近似、不保证文字逐像素一致，见已合入 main 的 `.plan/2026年-11月-16日-✅-pptx-多行文本布局近似.md`（用户 t66u 验收），不要在本切片里重开文字灰度 MaxDelta=0，也不借用该许可把 Word 字页标成整页通过。

## 库侧已测（未替代官方像素门）

`cargo test -p ooxp --test docx_page_image` 6 项通过（2026-11-17 约 04:18 Asia/Shanghai）。默认「A」墨迹盒 (96, 101) 9×9，与 C25 第 0 段对 Word 金样的测量一致。独立样本：`Hello` 左缘在 96..99（侧承）；12/24 磅「A」盒随字号变大；左缩进 720 twip → (144, 101) 9×9；段前 240 twip → (96, 117) 9×9；上/左边距 2160 twip → (144, 149) 9×9。超宽或第二段仍 `NotImplemented`。排版原点是左边距+缩进、上边距+段前，不是按样本写死的坐标。

未宣称整页 `MaxDelta=0`。未改金样、未放宽容差、未再穷举已证伪的 DrawString / GetGlyphOutline / WPF / 亚像素平移。

## 官方对照（2026-11-17 Asia/Shanghai）

新增用例 `docx-calibri-image`（8 个独立样本：11 磅 A / Hello / Abc，12 磅与 24 磅 A，左缩进 720，段前 240，上/左边距 2160）已写入并在 `run.ps1` 注册。代表回归 `docx-page-image`（空白 A4 与 52 空段第 2 页）一并注册。库 `exportPageImage` 对空白页已写出 `docx-page-image-lib.png`。

随后 Word 16.0 公开 PDF 路径未落盘：

- 04:32:46–04:40:54 `ExportAsFixedFormat(..., 17)`：已打开 `docx-page-image.docx`（Pid 30208，Visible，无 Repair 窗），持续占 CPU（约 7.7s 升至 149s），工作集约 242 MB，无 PDF 写到 out 或常见目录。未加 `-CloseOffice`。
- 04:41:16–04:43:01 同一份空白 A4 再试公开 `SaveAs2(..., 17)`（Pid 17480）：同样打开成功后持续占 CPU，约 90 秒仍无 PDF。

未调用私有接口，未把 Office COM 当作库 renderer。因金样未写出，本轮没有官方 `differ` / `maxDelta` / 墨迹盒数字，步骤 2 保持 ❌，计划保持 ⚠️。官方用例仍走该公开路径，不把超时杀进程写进默认验收（以免误伤个人 Word）。

## 不做

- 不靠加大容差、不改金样、不在对照图上补像素
- 不把像素写回包，也不塞进 inspect
- 不把其它段、其它对象类算进这一份的通过
- 官方对照只认 Microsoft Office 16.0，不认 WPS

- 不在这一份里做换行、多段分页、表格、图或页眉
- 不把幻灯片文字的布局近似结论直接套到 Word 整页通过标准上
- 不在金样导出挂起时无限改字体 API 参数
