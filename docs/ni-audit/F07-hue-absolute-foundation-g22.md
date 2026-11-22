# F07 standalone absolute hue foundation — g22, partial pending review

## 实际缺失的共享功能与合同

新增frame_absolute_hue以及resolve_frame_solid独立["hue"]分支，适用于Excel chart frame/Cartesian plot既有颜色路径的sRGB/theme输入。a:hue以指定H替换原H、保持S/L，不是hueMod乘数或hueOff偏移。ST_PositiveFixedAngle是ST_Angle i32 restriction：0<=val<21600000，60000units/degree。拒绝21600000或负/overflow，不能猜360度wrap/clamp；无需像sat增加灰色饱和度那样编造未定义hue，灰/黑白S0时输出保持输入。

复用原parse_signed_percentage的单val、无元素children、严格i32整数词法，helper单独执行fixed-angle范围合同；parser名称不改变angle单位。额外attrs/nested/百分数字符串/小数/+号/空白/overflow保持NI；独立hue外的mixed/repeated/alpha及既有sat/satOff范围/灰色gate不扩展。22旧helper函数体、精确alpha/default/accepted等旧路径不修改，不加样本条件/查表/色值补偿。

合同：
https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.hue?view=openxml-3.0.1
https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.hue.val?view=openxml-3.0.1
https://schemas.liquid-technologies.com/LibraryDocs/OfficeOpenXML/2006/st_positivefixedangle.html
https://webapp.docx4java.org/OnlineDemo/ecma376/DrawingML/ST_PositiveFixedAngle.html

## 预声明控制 / 实际新增能力

唯一新sourceSHA `c0fabaf01a506317d7890f651fbe982e41836e10b40c8767232ac129fb685787`，40个MC preflight有效控制=20hue+20原有显式RGB数学参考。两有色base的六主/次色angle，theme、gray/black/white，异背景EAF7F7 plot，最大合法angle21599999与45degree非sector边界。Python colorsys在改renderer前预声明RGB，不能据native结果反改参考；旧binary20NI/20参考成功，新20NI输入均可渲染。

20对新hue/预声明RGB数学参考整PNG同20/20、探针RGB最大差0byte。示例156082设0/60/120/180/240/300degree分别821515/828215/158215/158282/151582/821582；D27E4A设45degree D2B04A。纯粹合同数学与库结果，不冒称Excel读回。

absolute hue native=0，仍pending。初始23:47:49 Asia/Shanghai观察Excel1/PID43732，未启动Office；实现/全套测试与531既有PNG重放后仅一次新鲜OS gate：2026-11-22T00:01:17.0990801+08:00，ExcelProcessCount=1、PIDs=[43732]，原guard不允许，故未启动新会话/复用COM/循环等idle。SourceKind=OOXML_DERIVED_LIBRARY_ONLY_NOT_REEXPORTED，ExportKind=null，全部数学参考明确不是native gold。

所有实际取得native如果存在仅为此新hue矩阵，绝不重复完成的plot-alpha/satOff/sat/lum。派生Excel Chart.Export不是print/CopyPicture Bitmap/未改官方原件gold；native若取得425×266/库424×265仍各原尺寸，无拉伸/配准、公共原点差只诊断，全图pixel acceptance=false。此前sat/lum/satOff的1byte与geometry/AA残差原样保留，不追逐或补偿。

## 测试与保护

8新增Rust实际通过：六primary/secondary和45degree；angle单位/绝对覆盖与mod/off区分；256gray及黑白identity；i32/词法/结构/schema边界；sRGB/theme一致；chart/异背景plot在96/106/144dpi与独立RGB整输出相同；RGB cube extrema即S/L保持；mixed/repeated/nested与其他旧gate仍NI。

mandatory原命令cargo test --offline --locked -p ooxp --lib --tests -- --test-threads=1，2260passed/190targets/0failed/0ignored；Python15、CLI build通过。511历史PNG+20当前原有RGB参考=531既存成功PNG字节不变，20新能力不算旧PNG改动，有意变化0，4scatter仍NI；binary SHA `217c63c3cdeebcbb0e01f8310b553200d6d92c12a4570d75868e223b93987c24`。22旧helper/精确ALPHA_PRODUCT_DENOM/accepted矩形函数体及opaque PNG/全部受保护实现SHA、page_paint/t131/main/Pages/shared Office guard均保持。

自有唯一Temp输出；未Save/Quit/kill/关闭用户工作簿，未改guard、未复用旧COM、未新开Office绕已有进程；若native执行，只由fresh idle0+原binding、显式FullName/name/title/尺寸/PID/sourceSHA/106dpi与finally Close(false)完整证明。

## 审阅状态与后续真实缺口

唯一generation20261121-F07-hue-absolute-g22/F07-hue-absolute-foundation-待审.html接入原INDEX，默认未审，无自动接受。旧77artifacts/7468files（7422archive+46desktop）字节/URL/localStorage/state、g8a/g13勘误/all manifest/rollback保持；新78artifacts=72NI证据/41IDs+6PR12，精确文件/链接数见manifest/final receipt。49组仍4accepted/5pending/26partial/10NI/1safety/3illegal。

原PR1857字符/2861bytes逐字版本化pr14-body-before-g22-20261121.md，SHA478380694028e4e662037539c9bae9461fd2f25010731b89f48755d43a78a482；正常正确作者commit/push，发布以最终receipt为准，不合并/改main/Pages。

下一独立缺口：resolve_frame_solid的base输入目前仍仅srgbClr/schemeClr，hslClr为_错误；可检查上游入口与SDK通常域合同，优先做无transform的HSL基础色严格解析/通用HSL转换，再由controlled证据决定扩展，而不是等待native或继续1byte/AA扫描。本轮不声称hslClr已实现，hue mixed gate不因数学相关性自动放开；其他legend/square terminal-gap/lim4plot/180retrace/scatter边界保持。
