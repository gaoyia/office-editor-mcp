# F07 absolute luminance / fixed saturation native evidence — g20, partial pending review

## 实际通用基础：lum覆盖L，保持H/S

SDK Luminance指定输入颜色的luminance、hue/saturation保持，通常值0..100%；a:lum是CT_Percentage/ST_Percentage，不是固定有界alpha类型。
https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.luminance?view=openxml-3.0.1

新增frame_luminance复用rgb_hsl/hsl_rgb和严格signed i32整数千分之一百分比parser，支持独立lum、Excel sRGB/theme chart frame/Cartesian plot、0..100000通常域。不是原L乘factor或加offset，不猜越界clamp/extrapolation/inversion；其他mixed/alpha/repeated/nested仍NI。灰色/黑白S0使输出与未定义hue无关，可安全支持L替换，无需编造hue；sat/satOff非零灰色边界原样保留。

例156082设L25%=12516E、L50%=23A2DC、L75%=91D1ED；L0黑/L100白，灰色设L50%=808080。均为SDK合同与Python colorsys预声明数学参考，非native gold。21旧helper（原20+g19frame_saturation）、精确alphaMod常量/精度、旧lumMod/lumOff/default/accepted保持，无样本路径、查表补偿或AA/阈值扫描。

新lum源SHA `1f1c8aaad920b75f2db7008d4dfdd60071e64def3a1dee00d07e01e9d7ba1fef`，32个MC preflight有效控制=16lum+16旧RGB参考：两base的0/25/50/75/100%、theme、gray/black/white及不同EAF7F7背景plot。旧binary先证明16NI/16可渲染，新16输入可渲染。数学参考整PNG同16/16，RGB探针最大差0byte。lum native=0，SourceKind=OOXML_DERIVED_LIBRARY_ONLY_NOT_REEXPORTED/ExportKind=null，不冒称Excel颜色量化或整图接受。

## 首次固定satOff native，以及条件允许的fixed sat

23:16:17 Asia/Shanghai新鲜OS-only safe-idle0、HEAD6755df27/clean后，固定satOff源71fdf4010d3098fe9296c2519006e35df4b997933e8536c2e7e31f700aed94de逐字节复制到唯一owned Temp输出，原MC preflight通过；一次原verifier导出28native。未改控制/原源/共享binding或missing-stale Bitmap guard，未重复完成的plot-alpha pair。

14对native satOff/预声明native RGB参考整PNG同13对；native/旧库探针RGB精确13/14，最大差1byte；native变换与native显式RGB参考自身最大差1byte。不相同case：[{"Id": "chart-D27E4A-o-50000-satOff", "Native": [153, 139, 131, 255], "Library": [154, 139, 131, 255], "RgbError": 1}]。这补充通常in-gamut子集的实际native证据，不证明非零灰色、越界或mixed，故不扩限制、不调舍入或做1byte补偿。

固定absolute sat native未运行：后续一次fresh safe-idle不允许，未循环检查/绕guard/复用旧PID。

所有取得native均OOXML_DERIVED_REEXPORTED_BY_EXCEL/EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE，是派生固定控制经Excel Chart.Export，不是未改官方原件、print或CopyPicture Bitmap gold。native425×266/库424×265各原尺寸，逐case raw原点共有区差仅诊断，无拉伸/配准，不宣称全图通过或降低zero-pixel标准。颜色1byte与布局/AA残差分开，不指定未经判别的唯一成因。

每个运行会话需新的safe-idle0/原guard，明确sheet/name/title/288×180pt/readonly FullName/PID/源SHA/106dpi/Excel16；自有seed+copy finally Close(false)，Workbooks0/FilesClean/源不变/独占读/无锁/no repair，Save/Quit/Kill=false。satOff当时自有PID64788仍有窗口，未复用COM或force cleanup。固定sat如有执行，只有完成代码/测试等实质工作后的单次新鲜OS gate确认0才允许，非busy-idle循环。各生命周期/cert/gate保留时间与身份，不冒称当前工作簿状态。

## 实际测试与历史保护

8新增Rust：L覆盖不同于mod/off，256gray/黑白无hue依赖、RGB cube端点、严格词法/i32/范围/结构、sRGB/theme、chart/不同背景plot的96/106/144dpi数学RGB整输出、与旧lumMod0+lumOff合同等价、mixed及satOff原gate NI。mandatory2252passed/190targets/0failed/0ignored，Python15/actual CLI build通过。

479历史PNG（g19全部30+此前447+已完成plot2库输出）+本轮16旧RGB参考=495既存PNG字节不变，有意改旧PNG0，4scatter仍NI。binary/request/source/PNG回执、21helper/精确alpha/accepted函数体与PNG SHA、page_paint/t131/main/Pages/shared Office/native exporter/Bitmap guard全部复核；新增NI→可渲染不等于已接受。

唯一generation20261121-F07-luminance-absolute-g20/F07-luminance-foundation-待审.html通过原INDEX。旧75artifacts/6052文件（6006归档+46桌面）的字节/URL/localStorage状态/g8a/g13勘误/manifest/rollback保持，新页默认未审。现76artifacts=70NI证据/41IDs+6PR12，精确文件/链接总数见manifest/final receipt。

此前PR1655字符/2412bytes逐字存docs/ni-audit/pr14-body-before-g20-20261121.md，SHA29484d7e93e86073ed759810d5dc504513c2e4a69ead235875e87fef77b6e25e。普通正确作者commit/push，发布以最终bounded原SSH probe/ordinary push为准，不重写历史、不合并/自动接受。

49组仍4accepted/5pending/26partial/10NI/1safety/3illegal。下一lum native及尚未取得的fixed sat只能未来新鲜safe-idle允许才做，不重复完成plot/satOff矩阵。其他mixed/灰色选择/越界/sysClr/hslClr/legend auto-key/square terminal-gap/lim4plot/180retrace/scatter残差保持；可独立评估绝对hue合同/通常角度域基础，不扫AA/阈值，不等待review。
