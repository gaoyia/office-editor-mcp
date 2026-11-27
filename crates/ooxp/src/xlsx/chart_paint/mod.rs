//! 簇状柱形图、簇状条形图、普通折线图、带标记的散点图、普通面积图和普通饼图只读栅格化。优先读图部件里已有的缓存。没有缓存时，读公式所指格子里已有的值。不写回包，也不算公式。
//! 像素差要改这里的布局、颜色和字形，不要靠放宽对照容差。
//! 折线本切片只画 `c:lineChart` 且 grouping 为 standard 或省略的图：实线、缺省/auto 按系列循环标记（diamond、square、triangle、x、star、circle、plus、dot、dash，9 个一圈）、显式符号、缺格子或省略的 cache 点按缺口断线、空 inlineStr 与空 cache/lit <c:v></c:v> 按 0 连线。省略 `c:marker` 时尺寸为 9 磅；写出的 `c:size` 优先，写出标记但省略尺寸时按 OOXML 缺省 5 磅。平滑线、预设虚线、复杂自定义虚线、组合图及其余类型仍是 NotImplemented，不用缺一块的图充数。已锁定的 lineChart grouping=stacked/percentStacked（线性显式 min/max autoZero、marker=none、crossBetween=between）按系列顺序把各类目的代数累加端点连成折线，见 F23；省略 min/max、可见标记、空点、percentStacked 异号或缺值仍是 NotImplemented。
//! 散点本切片只画 `c:scatterChart` 且 scatterStyle 为 marker 或 lineMarker（缺省按 lineMarker）的图：X/Y 都是数值轴，点按系列顺序用数值坐标定位；autoZero 在 0 落在轴范围内时把坐标轴画在原点并在轴旁标刻度，0 在轴外时轴仍在较近端；marker 与 lineMarker 都画实线（官方 Excel 把 scatterStyle=marker 且系列未隐藏线条的图报为 xlXYScatterLines=74），缺格子或省略的 cache 点断开，空 inlineStr 与空 cache/lit <c:v></c:v> 按 0 连线；系列 `a:ln`/`a:noFill` 仍可关掉连线只留标记；标记规则与折线相同。官方 Excel 16 公开 API 的散点 valAx 写 crossBetween=midCat（不是类目槽公式，数值仍按线性 min/max 定位）；xlXYScatterLinesNoMarkers 写 scatterStyle=lineMarker 且 marker symbol=none，不是 scatterStyle=line；xlXYScatter 写 lineMarker 加系列 a:ln/a:noFill。这些已锁定路径在 midCat 下同样出图。scatterStyle=none/line/smooth，以及 scatterStyle=smoothMarker 或系列 smooth=1 的平滑曲线，仍是 NotImplemented。
//! 条形本切片只画 `c:barChart` 且 `barDir` 为 `bar` 或省略、grouping 为 clustered 或省略的簇状条形图：类目轴在左；类目 maxMin 时数值轴在上、minMax（或缺省）时数值轴在下；类目 minMax（或缺省）时第一类在下，maxMin 时第一类在上；系列 0 在类目轴方向的槽起点（maxMin 时槽顶，minMax 时槽底）；正值向右、负值向左，缺值不画该条。已锁定的 barChart grouping=stacked/percentStacked（overlap=100，线性显式 min/max autoZero）见 F22；三维、可见数据标签、空值按零或连线、类目轴网格、非 l/b 轴位及其余类型仍是 NotImplemented。
//! 面积本切片只画 `c:areaChart` 且 grouping 为 standard 或省略的图：按与折线相同的类目槽公式定位，用不透明纯色填到数值轴交叉（autoZero 为零，零在轴外时取较近端）；正值在交叉之上、负值在下，相邻异号在交叉处剖开以免自交；系列按 order 先画后盖；缺格子或省略的 cache 点：未写 dispBlanksAs 时按官方画到零基线（连续 V 形谷），显式 dispBlanksAs=gap 时仍按缺口断开、不跨空点相连；空 inlineStr 与空 cache/lit <c:v></c:v> 按 0 连接填充。有写出的实线描边才画上沿线，不默认画标记。已锁定的 areaChart grouping=stacked/percentStacked（线性显式 min/max autoZero、valAx crossBetween=midCat）按系列顺序代数累加，并填充相邻累加线之间的带（后画的系列盖住先画的），见 F23；省略 min/max、空点、between 类目位、percentStacked 异号或缺值、三维、可见数据标签、显式 dispBlanksAs=zero 与显式 dispBlanksAs=span 对真实空点/省略 cache 点按与省略项相同的零基线 V 形填充（官方面积 span 不是 4-到-9 弦）；span 下的 #N/A 亦按该 V 形填充，其余错误点仍 NI；半透明填充、组合图、标记及其余类型仍是 NotImplemented，不用缺一块的图充数。
//! 已有官方 Chart.Export 金样的三维图（`c:bar3DChart` 且 barDir=col、grouping=clustered、shape=box 或省略 shape；`c:line3DChart` grouping=standard 或省略；`c:area3DChart` grouping=standard 或省略；单系列 `c:pie3DChart`；`c:surface3DChart` 且 wireframe=0 或省略）按接近官方的投影出图，见 F24。残差留在对照页，不自动判过，也不把零像素标准改写成已达到。barDir=bar、堆积/百分比堆积三维、锥/圆柱/棱台等非 box 柱体、曲面线框、三维可见数据标签、三维次坐标轴及其余三维变体仍是 NotImplemented。
//! 饼图本切片只画单系列 `c:pieChart`：扇区从 12 点方向按 firstSliceAng（缺省 0，顺时针，单位度）起算，只把有限正值按总和比例切开；空值跳过该扇区但仍出现在图例并占用 accent 槽，零值不占角度但仍可出现在图例；负值、多系列、三维、环形、复合饼、爆炸、可见数据标签、半透明或非纯色扇区、非右侧缺省图例及其余类型仍是 NotImplemented，不用缺一块的图充数。

#[allow(unused_imports)]
#[allow(unused_imports)]
use fontdue::{Font, FontSettings};
#[allow(unused_imports)]
use super::render_dpi::RenderDpi;
#[allow(unused_imports)]
use crate::error::{Error, ErrorKind};
#[allow(unused_imports)]
use crate::xml::Element;

mod axis;
mod canvas;
mod color;
mod gradient;
mod layout;
mod legend;
mod paint;
mod pattern;
mod read;
mod render;
mod stroke;
mod typography;
mod types;
mod view3d;

#[cfg(test)]
mod tests;

// Private re-exports keep the pre-split flat namespace for child modules and unit tests.
#[allow(unused_imports)]
use canvas::*;
#[allow(unused_imports)]
use color::*;
#[allow(unused_imports)]
use layout::*;
#[allow(unused_imports)]
use paint::*;
#[allow(unused_imports)]
use read::*;
#[allow(unused_imports)]
use render::*;
#[allow(unused_imports)]
use typography::*;
#[allow(unused_imports)]
use types::*;
use view3d::*;

pub(crate) use color::{theme_accents, SystemColorPalette};
pub(crate) use layout::bitmap_px;
#[allow(unused_imports)]
pub(crate) use render::{
    render_chart, render_chart_at, render_chart_with_media, render_chart_with_sys_palette,
    render_chart_with_theme, ChartHost,
};
