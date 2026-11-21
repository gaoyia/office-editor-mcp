# F07 standalone saturation-offset foundation — g18, partial pending review

## 合同驱动的实际通用基础

SDK SaturationOffset明确：输入颜色的HSL饱和度偏移，hue和luminance保持；20% saturation加10%得到30%，不是20%乘1.1或10%。a:satOff的类型为CT_Percentage，不是alphaOff的CT_FixedPercentage。
来源：https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.saturationoffset?view=openxml-3.0.1

实际新增frame_sat_offset：复用原rgb_hsl/hsl_rgb和parse_signed_percentage，按S + signed_offset / 100000计算。只支持独立satOff、sRGB/theme、结果仍在[0,1]的明确定义子集；offset=0逐字节identity（包括灰色）。严格整数千分之一百分比i32解析与原词法/叶子结构检查保持，不能误用alphaOff/lumOff的bounded parser。非零灰色的hue选择、结果超范围的clip/extrapolation/inversion没有native证明，保持NI；不从不同的satMod变换外推。satOff→alpha/alpha→satOff、其他混用、重复与嵌套也保持NI。未扩展foreground/legend/Word/PPTX、未动共享painter、无表格补偿或阈值调参。

独立math例：156082偏移-25%得28596F、+25%得026795；D27E4A偏移-25%得B68566、+25%得EE772E；纯红减25%得DF2020。这些是预声明Python colorsys与合同的数学参考，不是Excel native或打印gold。旧satMod、hueOff、alphaOff、alphaMod精确乘积、HSL和18旧helper再加g17新helper，共19个函数体保持；ALPHA_PRODUCT_DENOM常量也不变。

## 新鲜安全检查与真实证据缺口

2026-11-21 22:38:05 Asia/Shanghai，机器在线、HEAD04e0b4d3/clean；只读OS观察见Excel PID45756、MainWindowHandle10106916、进程1。本轮没有COM绑定、打开Office、复用该进程、Save/Quit/kill/关闭任何工作簿。没有idle轮询或force cleanup。

因此g17已经准备的不同背景2个plot-alpha native控制仍未运行，原来源/文件/声明保持；同色背景主plot不能独立证明局部opacity。g18也没有native导出：NativeCount=0，ExportKind=null，SourceKind=OOXML_DERIVED_LIBRARY_ONLY_NOT_REEXPORTED。不能把本轮library/mathematical reference冒称EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE或CopyPicture/print gold。上一g17会话的自有readonly seed+copy Close(false)及Workbooks0历史证明保持，不冒称本轮执行过close或当前用户工作簿状态。

新源SHA71fdf4010d3098fe9296c2519006e35df4b997933e8536c2e7e31f700aed94de。28个MC preflight通过的控制=14 standalone satOff+14预声明显式RGB参考，两彩色base的负/零/正offset、theme、纯红、gray identity以及3个EAF7F7不同背景plot。旧binary先证明14 satOff NI/14参考可渲染，再实现；新14输入可渲染，14组library整PNG与数学RGB参考相同。此项证明库语义/集成，不证明Excel颜色量化或整图接受；没有新的native-vs-library像素数字可报。

## 实际测试与保护

8新增Rust：百分比点非乘法、原色/中和边界、4096采样RGB cube与256gray identity、H/L量化保持、CT_Percentage严格词法/i32、sRGB/theme、chart与不同背景plot在96/106/144dpi整输出同显式RGB、灰色非零/越界和mixed/repeated/nested NI。mandatory2236passed/190targets/0failed/0ignored，Python15、actual CLI build通过。

419历史PNG（g17全部38控制+此前381）及本轮14旧RGB参考，共433既存PNG整字节不变，有意改旧PNG=0，4scatter仍NI。所有binary/request/source/PNG逐项回执、19helper函数体hash、accepted rectangular函数体/PNG SHA、page_paint/t131/main/Pages、shared Office/Bitmap guard保持；完整验证见post-implementation-validation.json。测试/PNG通过不提升任何接受边界。

## 审阅、历史与下一真实前置

唯一generation20261121-F07-saturation-offset-g18/F07-saturation-offset-foundation-待审.html通过原桌面INDEX入口。旧73review artifacts/4966保护文件（4920归档+46桌面）原字节/URL/localStorage状态、g8a/g13勘误和全部manifest/rollback保持。新增默认未审，74artifacts=68NI证据/41IDs+6PR12；精确文件/链接数以本代manifest与最终回执为准。

前3317字符PR body逐字保存docs/ni-audit/pr14-body-before-g18-20261121.md，SHAfb7315ef9f75afded4895d3d20a40f187156d9106839faa3cabdf9c8d7e4f23e。普通正确作者commit/push的最终回执才是发布依据，PR14不合并/不接受。

49组仍4accepted/5pending/26partial/10NI/1safety/3illegal，不是全部实现。下一具体前置：未来新的safe-idle0允许时，先完成已准备的g17 plot-alpha独立pair，再对本代satOff固定源做bounded native颜色/预声明参考验证；achromatic hue和extrapolation须独立native证据，不猜clamp/invert。若Office条件仍不满足，可独立评估F07绝对sat的合同/解析数学基础。其他mixed/legend auto-key、sysClr/hslClr、square terminal-gap、lim4plot、180retrace/scatter等残差保持；不等待review，不做AA/阈值无界扫描。
