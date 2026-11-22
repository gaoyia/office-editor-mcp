# F07 absolute sat/lum fixed native prerequisite — g21, partial pending review

## 本轮范围与实际完成

遵照t172u前置优先及本轮明确优先fixed sat+lum native，首次完成g19/g20已实现通常域的62个native证据；不是重复已完成plot-alpha或satOff，也不因native仍有残差而阻塞新的独立基础。本轮renderer代码新增0、NI→实现新增0；没有把证据补齐冒称新feature或全图通过。此前独立sat覆盖S/保持H,L，lum覆盖L/保持H,S的实际实现保持；下一独立实际缺口是absolute hue。无需等待当前review。

## 预声明合并源 / 不改变控制以凑结果

原sat源 `e1c17c05b21de9035afb03ead31cb2dcaa6cd7c25e938acc7ded0c2d273bda60` 的30控制与原lum源 `1f1c8aaad920b75f2db7008d4dfdd60071e64def3a1dee00d07e01e9d7ba1fef` 的32控制，所有共享工作簿/data/theme/style/非mutable ZIP部分逐字节相同，drawing root namespace集合相同。每张原chart XML及其rels内容保持字节不变，单独记录原源SHA/原chart part/hash和合并part/hash；仅重编号drawing对象ID/relationship/part名并整合ContentTypes，显式sheet/name/title/288×180pt不变，无生成新的色值或改控制参数。原两源保持不变，原MC namespace preflight通过。唯一新合并源SHA `fdc0ae59364209479563d717201d8e38a053466ba968fb85808e43a21c278924`。

23:35:34 Asia/Shanghai fresh OS idle0，原native verifier自身再次拒绝既有Excel/全局COM；只启动一次owned readonly seed+copy会话。62导出=15sat变换/15独立预声明RGB参考+16lum变换/16独立预声明RGB参考；无plot-alpha pair、无satOff重跑。所有native证书的SourceSha是新合并源，不能标成原sat/lum SHA；原控制来源由ChartXml hash保持证实。

## 首次native结果 / 诚实的量化与几何边界

absolute sat：15对native变换/RGB参考，整PNG同13对；native/既有库RGB精确13/15，最大差1byte。不同case：[{"Id": "chart-156082-s0-sat", "Native": [75, 75, 75, 255], "Library": [76, 76, 76, 255], "RgbError": 1, "NativeVsReferenceRgbError": 1}, {"Id": "chart-D27E4A-s50000-sat", "Native": [198, 129, 86, 255], "Library": [199, 129, 86, 255], "RgbError": 1, "NativeVsReferenceRgbError": 1}]。

absolute lum：16对native变换/RGB参考，整PNG同14对；native/既有库RGB精确14/16，最大差1byte。不同case：[{"Id": "chart-000000-l50000-lum", "Native": [127, 127, 127, 255], "Library": [128, 128, 128, 255], "RgbError": 1, "NativeVsReferenceRgbError": 1}, {"Id": "chart-FFFFFF-l50000-lum", "Native": [127, 127, 127, 255], "Library": [128, 128, 128, 255], "RgbError": 1, "NativeVsReferenceRgbError": 1}]。

特别是black/white设L50% native127而库128；原gray808080设L50%控制不出现该差。它不能唯一证明一个统一舍入规则，禁止用样本条件、1byte补偿或猜测阈值修正既有模型。保留实际逐case证据，而不是把数学RGB参考等同Excel gold。sat/satOff灰色选择、越界/mixed gate不扩展。

OOXML_DERIVED_REEXPORTED_BY_EXCEL / EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE，派生native Chart.Export，不是未改官方原件、print或CopyPicture Bitmap gold。native425×266/库424×265各自原尺寸；公共原点逐case差保留仅诊断，无拉伸/配准，不能因native变换与native RGB参考同而宣称native与库全图同。颜色1byte与AA/布局独立，未指定唯一成因，不改zero-pixel接受标准。

## 生命周期与完整回归

Excel16、106dpi、显式FullName/name/title/尺寸/PID/源SHA，guard与共享Office代码完全未改。23:35:43.7097085 Asia/Shanghai finally Close(false)验证2本自有readonly seed+copy关闭，RemainingWorkbookCount0、源SHA不变、独占读/no lock/no repair、FilesClean，Save/Quit/Kill=false；当时自有PID43732/窗口仍在，ObservedExcelProcessCount1只是历史观察，未复用/强制退出/触碰用户工作簿。本轮此后没有第二次Office会话或PID idle循环。

原mandatory命令重新执行2252passed/190targets/0failed/0ignored，Python15及CLI build通过。当前g20完整32输出+此前479=511既存成功PNG全部CLI重放字节相同，有意变化0，4scatter仍NI；binarySHA `4a5815f93e70ca90c8296857bd851e37fd5c26fa5768c45b530f88a6d3342451`。22旧transform helper/精确alphaMod常量/全部Rust源及受保护实现文件、accepted矩形函数体/PNG、page_paint/t131/main/Pages/共享guard均保持。本轮新Rust测试0，不能把基线2252冒称新增加。

原76review artifacts/6734files（6688archive+46desktop）、URL/localStorage/state、g8a/g13勘误/all manifest/rollback不变；新增唯一g21 generation和原INDEX链接，默认未审，不改原g19/g20页的历史native pending文字，当前状态由互补g21条目说明。49组仍4accepted/5pending/26partial/10NI/1safety/3illegal。

## 下一真正可做的基础，而不是量化扫描

绝对hue合同已查：SDK Hue指定H而保持S/L，ST_PositiveFixedAngle是ST_Angle i32限制0<=val<21600000、单位60000/degree（不是hueOff的signed unrestricted angle，不是hueMod百分比）。可独立实现正常角度域的共享H替换及严格结构/range测试，保留未验证mixed，未来native另需fresh safe-idle；本轮没有声称已实现或native验证hue。
https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.hue?view=openxml-3.0.1
https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.hue.val?view=openxml-3.0.1
https://schemas.liquid-technologies.com/LibraryDocs/OfficeOpenXML/2006/st_positivefixedangle.html
https://webapp.docx4java.org/OnlineDemo/ecma376/DrawingML/ST_PositiveFixedAngle.html

其他square terminal-gap/lim4plot/180retrace/scatter/legend key等既有边界保持；不作AA/阈值hunt、不自动接受、不合并。

此前PR正文1852字符/2707bytes原样版本存pr14-body-before-g21-20261121.md，SHA2fc03ca256e1f58d098df10f99064664b5553ad25e52e5f83c21590a1b6ba08c。正常正确作者commit及单次bounded原SSH probe/普通push，发布状态以最终receipt为准。
