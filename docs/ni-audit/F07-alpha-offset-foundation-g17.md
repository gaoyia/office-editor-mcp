# F07 alphaOff signed-opacity-offset foundation — g17, partial pending review

## 合同与实际通用实现

SDK AlphaOffset明确：在输入alpha上加/减指定百分比点，例如50%+10%=60%、50%-10%=40%，结果限制0..100%；不是乘法。CT_FixedPercentage是signed fixed百分比，baseline支持严格整数千分之一百分比[-100000,100000]。来源：https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.alphaoffset?view=openxml-3.0.1

现有parse_luminance_offset已经提供相同signed-fixed叶子结构/范围解析前置，直接复用而非修改或复制。新frame_alpha_off_fill以i64计算initial+offset并clamp0..100000，复用原Solid/Translucent及原coverage。支持Excel sRGB/theme chart frame与Cartesian plot纯色的独立alphaOff、alpha/alphaOff、alphaOff/alpha；末尾绝对alpha覆盖，但前offset仍严格校验。新alpha操作数复用原frame_initial_alpha。默认输入100%opaque，RGB不变。18个旧helper完整函数体hash保持，包含alphaMod精确乘积和hueOff；没有改变原alpha路径、alphaMod精度/舍入、HSL或默认输出。其他混用/重复/嵌套、其他前景/legend颜色语义仍NI，不扩展Word/PPTX或pie透明plot。

## 先固定控制，再进入Office

主源SHA `98fae59b871e8ea5face775d8d009f90fd82aff2c440784e90d076aa5678ee25`。38同title/288×180pt/数据/轴控制=19变换+19预声明旧alpha参考：两彩色base、-100/-50/0/+50/+100%、输入40%后-50/-10/+10/+100%、两种覆盖顺序、theme/gray/plot。19个native变换全部与对应参考整PNG字节相同，18个chart alpha与合同精确相同。加法/夹取/绝对覆盖被分开判别，未用图像相关性猜offset。

主矩阵plot与chart同色，不能独立辨识局部opacity。已预声明不同背景的补充pair，但未运行Office、不冒称native证明。一般plot路径复用既有Translucent与精确DPI集成测试；此项native对比边界保留。

实际native数38、新增可渲染输入19。库对新native探针RGB 12个精确，chart最大1byte、plot最大0byte；alpha全部相同（opaque plot的alpha255本身不能证明局部透明度）。native425×266与库424×265各原尺寸，无拉伸/配准。颜色读回/合成与全图AA/layout残差分开，不改原RGBA实现拟合；不宣称整图通过或降低zero-pixel接受标准。

准确来源OOXML_DERIVED_REEXPORTED_BY_EXCEL / EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE。控制源为派生，不是未改官方原件；输出不是打印或CopyPicture Bitmap gold。

## 测试、旧输出与生命周期

381历史PNG（含g16全部35）及本轮既有alpha参考，共400旧PNG整字节不变，有意改旧PNG=0，4scatter仍NI。新增8Rust覆盖signed-fixed范围/词法结构、加法非乘法、双端clamp与安全运算、绝对覆盖但验证前项、initial alpha结构、theme/RGB保持、chart/plot96/106/144dpi、混用/重复/嵌套NI。mandatory2228passed/190targets/0failed/0ignored，Python15与actual CLI build通过；binary/request/source/PNG逐项回执完整。接受rectangular函数体/PNG SHA、page_paint/t131/main/Pages、18旧helper及shared binding/native exporter/Bitmap guard保持。

每个已运行Office会话仅自有readonly seed+copy，经原MC preflight、explicit名字/title/canvas、FullName/PID/源SHA/锁审计；finally Close(false)只关闭自有两本，验证Workbooks0/源不变/no repair，无Save/Quit/kill/userbook关闭。不同会话必须前次closure证明和OS idle0之后，未累积窗口；后续进程数仅OS观察，不冒称主动cleanup。

## 评审、发布与剩余前置

唯一generation20261121-F07-alpha-offset-g17/F07-alpha-offset-foundation-待审.html通过原INDEX入口。旧72review artifacts/4369保护文件=4323归档+46保留桌面，原字节/URL/localStorage状态/g8a/g13勘误/all manifest/rollback全部不变。新73artifacts=67NI证据/41IDs+6PR12，默认未审，无自动接受；精确增量数量/链接计数以generation manifest/final receipt为准。

前3089字符PR body已逐字存pr14-body-before-g17-20261121.md，SHA0bfa268ab8a02f02f82db4a18b8c271b7d82cac551ee84defb25a1cb5ec6b1ea；旧版本均保持。PR保持简洁，title/base/state不改，发布以普通commit/push最终回执为准。

49组仍4accepted/5pending/26partial/10NI/1safety/3illegal，不是全部实现。下一独立可做基础是satOff的合同/控制；如本轮对比plot未能安全导出，先完成该对比前置。sysClr/hslClr、其他混用/legend auto-key、square terminal-gap、lim4plot、180retrace/scatter等边界仍在，不扫描AA或阈值、不等待审阅才继续原授权。
