原t171/t172前置优先继续。g23 `6267201530af9b5f4e2c825bf9ceebaf3d9b996e` 已发布，HEAD=origin，clean/ahead0；保留g22提交，无合并/自动接受，PR14保持OPEN。一次原SSH只读probe成功，随后一次普通push exit=0，无重试/凭据或transport变更。

F07实际新增严格无transform的hslClr基色输入：属性hue/sat/lum必需，忽略namespace声明但拒绝额外/重复semantic attrs、元素children/非空文本；严格i32整数，H0<=H<21600000（60000/degree）、S/L通常0..100000。不猜wrap/clamp或额外gamma，复用既有perceptual HSL。resolver新基色早返回，故其alpha/其他transform仍NI；原23helper/精确alpha/default/accepted、srgb/theme及其他颜色模型保持。上游solidFill仍仅一个color，Slide host新增专项断言保持旧输出，不误放其他host。

SDK矛盾明确保留：HslColor表的1/6000与其引用的ST_PositiveFixedAngle不同；蓝例H14400000/S100%/L50%声称000080，常规HSL及既有helper给0000FF。没有据例子加gamma/缩放/查表补偿，也未声称Excel已裁定；当前是正常域数学模型部分待审。原SDK全文/合同差异及可判别蓝色控制均保留。

源SHA 720666e478e81c40d3a8db56c271dd8f7b7d0bc38d963aefe5c775a4ea7ed925，40个原MC preflight控制=20HSL+20改代码前预声明colorsys RGB参考；旧binary20NI/20成功，新20输入均可渲染，数学参考整PNG同20/20、RGB最大差0byte。最初新增test手写808000与已冻结的独立参考7F8000不一致，保留首次7pass/1fail log及PNG诊断，仅修正此新增test至原预声明值；renderer/helper/source/旧test未改。修正后8新test通过，非篡改旧gold。

HSL native仍pending/0，hue native也仍pending；00:05:01 Asia/Shanghai初始Excel1/PID43732，独立实现/全套测试/571旧PNG后一次fresh OS gate 2026-11-22T00:29:20.8010263+08:00，Count=1、PIDs=[43732]，故本轮无Office会话。未循环等idle/复用COM/Save/Quit/kill/碰用户工作簿；SourceKind=OOXML_DERIVED_LIBRARY_ONLY_NOT_REEXPORTED，数学参考非native或print/Bitmap gold。未重复完成plot-alpha/satOff/sat/lum；原1byte/AA/geometry边界不补偿，不降低zero-pixel标准，不宣称全图通过。

重新mandatory2268passed/190targets/0failed/0ignored，Python15/CLI通过。551历史（含g22新支持20）+20当前旧RGB参考=571既存PNG整字节不变，有意变化0，4scatter仍NI，全部保护SHA/page_paint/t131/main/Pages/shared guards保持。

原INDEX接入唯一generations/20261122-F07-hsl-base-g23/F07-hsl-base-foundation-待审.html，默认未审。79artifacts=73NI证据/41IDs+6PR12；2213本地href及全部img引用有效。原78页/8104files原字节/URL/localStorage/state/g8a/g13勘误/all manifest/rollback保持，现fullscope 8786files。

详情docs/ni-audit/F07-hsl-base-foundation-g23.md及原NI计划。此前PR1896字符/2792bytes逐字版本化pr14-body-before-g23-20261122.md。49组仍4accepted/5pending/26partial/10NI/1safety/3illegal，未全实现。下一独立prstClr规范enum/RGB合同及严格leaf基础需先可溯源映射，不能猜色表；HSL混合transform/hue native与SDK蓝例缺口不隐瞒，不等待review或网络。