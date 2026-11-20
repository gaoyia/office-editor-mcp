# ✅ Normal继承6路径核实（原PR已有问题）

- [x] 原始2族×3路径，4源SHA/缓存/actualcells校验，不按6新功能计数。
- [x] 非零styleXF和builtin0的规范链、Normal字体与cell实际字体分开，库96几何6/6验证。
- [x] 17组有限控制公开COM：源Arial11，Office NormalCalibri11；两自有SaveCopyAs未重开，ZIP证实Normal字体语义改变，不是name/index变化。
- [x] 当前6个CLI输出与既有after字节一致，原gold不动；不改renderer、不font0/7/8硬回退。
- [x] 新验收工具和4回归，加现19共23 Python；无Rust改动，原1868日志不冒称重跑。
- [x] Office同语义可比证据仍不足，DPI豁免保持38/49，不将此6强判仅DPI/库bug/全图PASS。（2026-11-20：列宽 MDW 已按 fonts[0] 收口并经 t164 审核通过；载入改写现象如实保留，不再阻塞本项列宽收口。图表高 220/221 为已接受残差。）

精确缺证：公开兼容规则说明此载入字体语义改写，或Office能保留非零Normal-font引用的独立同语义合法文件。现仅证明本探针族现象，停止扩探针，不研究AA/legend/5 Export边界，不恢复十项功能。

证据：桌面office-editor-PR10-Normal继承核实-20261117-151855；normal-inheritance-certificates.json/six-path-certificates.csv/finite-control-results.csv/index/README。官方规范解释：https://learn.microsoft.com/en-us/answers/questions/1110580/where-is-the-normal-style-font-defined-in-an-xlsx 。


## 2026-11-20 t106u 盘点（不改绿）

Office 同语义可比证据仍不足：公开规则只说明载入时可能改写 Normal 字体语义，没有可保留非零 Normal-font 引用的独立合法对照文件。本轮不扩探针、不改 renderer、不把此 6 项强判为仅 DPI 或库 bug。计划保持 ⚠️，不放入待审对照。


## 2026-11-20 t162–t164：列宽 MDW 按 fonts[0] 收口并审核通过

用户确认桌面 `pr12-列宽fonts0-待审阅.html` 审核通过（t164），并要求同步文档、提交，准备自行合并。

根因：Excel 16.0 列宽 MDW 取 stylesheet 的 `fonts[0]`，不是 Normal cellStyle。localized-normal / normal-font-only 的 `fonts[0]` 是 Calibri 11，Normal 才指向 Arial 11。库原先按 Normal=Arial 在请求 dpi=106 重测得 MDW 9，公式 `floor(((256*10.125+floor(128/MDW))/256)*MDW)` 得 91px/列，导出 364×220 / 365×23 / 92×23。应按 `fonts[0]=Calibri`，106 下 MDW 8，得 81px/列。

五组拆分探针 P1–P5（窗口 106，只读副本，Close(false)）锁定：`fonts[0]=Calibri` 时 81px/列、表 325×23；`fonts[0]=Arial` 时 91px/列。纯 Arial 工作簿官方图 364×221，与 MDW 9 一致。等线 11 在 96/106 都是 MDW 8，普通迷你宽度本来就对。「列宽永远按 96 MDW、不随请求 DPI 重测」事先疑点不成立。

实现：`GeometryContext::from_styles` 改用 `fonts[0]` 测 MDW，仍按请求 DPI 重测；`normal_font_index` 只留给单元格默认字体绘制。公式未改。

修后 106 对官方：图表 324×220 对 324×221（宽已齐，高差 1px）；工作表 325×23、迷你图 82×23 画布已齐。等线迷你 96/106 哈希与 t161 一致。96 下这两份 Normal 探针变为 284×200 / 285×21 / 72×21，是同一规则在 Calibri MDW 7（10.125→71）下的结果，本机不能改系统 DPI 做 96 金样。图表高度 220 对 221 与 Excel `Chart.Height` 点尺寸一致，属 CopyPicture 外缝，未硬插 +1（也不能宽高同时 +1，官方宽是 324 不是 325）。已接受该 1px 高度残差。

历史 Office 载入改写 Arial→Calibri 的现象如实保留，不再作为阻塞本项列宽收口的未勾项。单元格默认字体仍走 Normal。

测试当时：geometry 单测 9 过，`xlsx_geometry_image` 5 过（含新 split-Normal），`xlsx_export_dpi` 6 过；`cargo build -p ooxp-cli --release --bin ooxp` 成功。新 CLI SHA256：`a1ecc56bdb9e6ab884dfb3d3803f67efe4f681401db7a9bccf302edf25ca32c9`。源 SHA 未变。证据在工作区外 `%TEMP%\pr12-t162-width-height-20261120\`，不把桌面 HTML / TEMP 二进制或 `.plan/_evidence/pr12-t131-column-bounds.md` 提交进仓库。

文件名改为 ✅。
