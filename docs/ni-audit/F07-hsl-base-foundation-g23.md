# F07 strict untransformed HSL base-color foundation — g23, partial pending review

## 实际功能与上游依赖检查

read_frame_with只由ChartHost::Excel启用；read_frame_fill(solidFill)要求且仅要求一个color，并委托resolve_frame_solid。原resolver仅srgbClr/schemeClr，故hslClr确实缺失，无需放宽共享shape/host guard。新增parse_frame_integer与parse_frame_hsl_color，resolver新增hslClr早返回分支：只支持无transform的hue/sat/lum基色，按已有perceptual hsl_rgb通用转换，不改任何旧23helper或精确alpha/default/accepted路径。

严格3个非namespace声明属性hue/sat/lum必需、无额外/重复semantic attrs，无元素children或非空文本（允许格式空白）；严格i32整数词法。hue0<=h<21600000，60000units/degree；sat/lum在通常域0..100000，越界/负数/overflow不猜wrap/clamp。百分号/+号等有些是合法OOXML词法，但不在本次显式整数profile，并非将它们称为非法规范。带alpha/其他transform即使已有srgb/theme同名transform仍NI，避免误放混合语义。Slide host仍绕过frame，新增专项像素断言保持其既有结果；series/legend/其他host与sysClr/prstClr/scrgbClr gate未改。

## 合同证据与未解决的文档矛盾

CT_HslColor真实属性是hue/sat/lum，不采信某些第三方h/s/l及21600尺度表。引用ST_PositiveFixedAngle的schema角度单位60000、domain0..21600000；SDK HslColor属性文字写1/6000与该类型不一致。SDK正文蓝色例hue14400000/sat100%/lum50%声称000080，常规HSL及既有helper模型给0000FF，且SDK写perceptual gamma2.2 assumed。保留原opened SDK全文及此差异，不为贴例子插入gamma/缩放/补偿，不声称已由native裁定。本轮支持的是明确通常域perceptual HSL数学基线、部分待审，非已验证Excel颜色profile；控制中含该蓝色输入，未来native可区分，不能把数学一致冒称官方例已对齐。
https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.hslcolor?view=openxml-3.0.1
https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/3b541eef-c2ea-4134-9e73-932962cedf9f
https://schemas.liquid-technologies.com/LibraryDocs/OfficeOpenXML/2006/st_positivefixedangle.html

## 预声明控制与实际新增能力

新sourceSHA `720666e478e81c40d3a8db56c271dd8f7b7d0bc38d963aefe5c775a4ea7ed925`，40个原MC preflight控制=20HSL+20独立预声明Python colorsys RGB参考，含六主/次色、sat/lum通常域、gray/black/white、非sector45degree、最大合法角度及异背景plot。改实现前old binary20NI/20参考成功，新20输入均可渲染；数学参考整PNG同20/20，探针RGB最大差0byte。它们是数学参考，不是native gold。

最初新写integration test手写808000，而改renderer前已经固定的独立colorsys/source中plot-h3600000-s100000-l25000-rgbReference为7F8000。保留首次7pass/1fail原log与PNG数组诊断（库[127,128,0]、手写[128,128,0]，47634px差），仅将此新增test参考改成冻结的7F8000；renderer/helper/原控制源及任何旧test未改。修正后8新test通过，差异/修正回执均展示，非悄悄改native/旧PNG来过测。

HSL native=0/pending，hue native仍pending。初始00:05:01 Asia/Shanghai观察Excel1/PID43732；独立代码/全套测试/571既存PNG验证后仅一次fresh OS gate：2026-11-22T00:29:20.8010263+08:00、Count=1、PIDs=[43732]，故本轮未启动Office/绑定或复用COM/循环等idle。SourceKind=OOXML_DERIVED_LIBRARY_ONLY_NOT_REEXPORTED，ExportKind=null。

未重复完成plot-alpha/satOff/sat/lum；若执行native仅本轮新HSL源，hue pending不以重复或额外会话填补。Chart.Export是派生native不是print/CopyPicture Bitmap/未改官方gold；native若取得425×266/库424×265各原尺寸，不拉伸/配准，公共原点差只诊断，GlobalPixelAcceptance=false。既有1byte/AA/legend/stroke残差不补偿、不降低zero-pixel标准。

## 完整验证、保护与审阅

8新增Rust：主/次色及非sector、gray/黑白/角度边界、完整属性/namespace声明/无transform/文本规则、严格整数/正常域、theme独立与旧srgb保持、chart/异背景plot96/106/144dpi整RGB对照、Slide host隔离、重复fill及其他颜色模型/mixed旧gate。

mandatory原命令重新2268passed/190targets/0failed/0ignored，Python15/CLI通过。551历史（g22全部40+此前511，包含g22新支持20，未缩小为531）+当前20旧RGB参考=571既存PNG整字节不变，有意改旧PNG0，4scatter仍NI。binarySHA `0cdf8161af02c0762d5f580366a653e32c3864312eefc0843dc13726080461cb`；23旧helper/精确ALPHA_PRODUCT_DENOM/accepted矩形函数体及PNG、全部保护SHA/page_paint/t131/main/Pages/共享Office guard保持。

唯一generation20261122-F07-hsl-base-g23/F07-hsl-base-foundation-待审.html接入原INDEX，默认未审。原78artifacts/8104files（8058archive+46desktop）字节/URL/localStorage/state/g8a/g13勘误/all manifest/rollback不变；现79artifacts=73NI证据/41IDs+6PR12。49组仍4accepted/5pending/26partial/10NI/1safety/3illegal，未全实现。原PR1896字符/2792bytes逐字版本化pr14-body-before-g23-20261122.md，SHAb93da615d7165e327e2da04607c69eb589568b4ce1696408a7cd1830387b7c06。

正确作者普通commit，保留g22本地commit；只有batch末一次bounded原SSH只读probe成功且远端身份符合才普通push所有pending，否则保持local ahead并如实报告，不重试/改transport/凭据。发布以最终receipt为准，不合并/自动接受。

下一独立缺口prstClr基色：当前仍NI，源码未发现aliceBlue/dkBlue既有规范map。下一先核对标准enum到RGB的可溯源合同及normal leaf/parser，再实现可验证基础；不能编猜颜色表，不以等待Office/网络或1byte调参代替实际功能。HSL带transform/alpha仍NI，不能因srgb上已支持就自动放开；hue native与SDK HSL蓝例仍有诚实证据缺口。
