# PR12 t127：图例位置 / 布局与有符号值轴实现、复审证据

2026-11-20，Asia/Shanghai。基线 `f3afb70acd44e94351a9dab7d27b6f90a5bf2f2a`，分支 `feat/office-preview-follow-up-4`。t127u 明确要求按官方修复后再审。本轮是**实际实现 + 测试 + 复审**，不是只补审计；尚未取得新通过，不改绿任何待审项或计划。

## 官方位置、默认与自定义

- 公开 OOXML `ST_LegendPos` 只有 `b / tr / l / r / t`（下 / 右上 / 左 / 右 / 上）；没有单独的“浮动居中”枚举。
- 浮动由 `manualLayout` 定义，Office 明确其精确位置覆盖 `legendPos`。自动右置 baseline 不是自定义；factor/edge 样例才显式自定义。保留两者的原点区别及合法边界裁切。
- **先验证后改默认**：五位置 × overlay absent / 空元素 / 0 / 1，原始代表源与 origin-zero 控制源各一套。每个位置官方 absent / `<overlay/>` / 1 的 PNG 均字节相同；0 不同，且公共 `Legend.IncludeInLayout=true`。不是仅拿 baseline 一张图推断全局。
- style 字体、factor/edge x/y=.0/.25、七张旧图例配置均纳入；默认 10pt 与 Arial18 粗斜两套实测，不修改源为 overlay=0 掩盖问题。

公开依据（本轮打开）：
- https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.charts.legendpositionvalues?view=openxml-3.0.1
- https://learn.microsoft.com/en-us/openspecs/office_standards/ms-oi29500/8314e929-129a-4991-a7f9-9f37010521e5
- https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.charts.manuallayout?view=openxml-3.0.1

## 通用实现（不是像素后处理）

1. 图例省略/空 overlay 走已验证的 Excel true，只有显式 false 预留绘图区；不改变冻结显式非叠加的默认右侧墨迹链路。
2. 图例墨迹排版与绘图区预留分离：墨迹用 typographic advance，不能把 GDI 防裁切 padding、key/text space 再计入流式宽度；行节距采用字号 + 8pt entry gutter，修复字号变化时位置和键尺寸/行距偏差。
3. 上下左右/右上按点数留边，右上非叠加同时预留顶带/右带及底部净空。手动 edge 可合法超边裁切，不能强制移回边界；factor 仍自动限定。
4. 值轴 autoZero 不在底部时，不再白留底部类目标签 gutter；有符号轴挂号宽度、短系列名预留带按字体度量，解除 mixed/explicit 的约 20px 累积刻度和横向范围错位。零轴直接使用对应主网格的栅格行，避免 133.5 独立 round 到另一行。
5. 按 step 尺度归一化累加产生的伪零：`-0.3..0.5 by 0.1` 不再出现 `-2.78e-17` 标签挤宽整幅绘图区；真实小数值仍可科学计数。
6. 不改 dpi 默认值、不加统一 +1、不平移/拉伸官方或库图。

## +1 高度结论（比 t126 更进一步的证据）

不是 oneCell 固有多一行，也不是简单“非整数尺寸”。保持 chart XML 和 432×252pt **完全相同**，只改绝对锚点：

| 绝对 Top(pt) | 官方 PNG |
|---|---|
| 0.0000000 | 636×371 |
| 1.0000000 | 636×372 |
| 4.0000000 | 636×372 |
| 14.0000000 | 636×372 |
| 14.2359056 | 636×372 |
| 14.2641735 | 636×371 |

由绝对像素边界/子像素相位触发官方导出外包络变化。前轮顶对齐裁切会伪造 1px 内容偏移；本轮完全保留 native PNG，不以裁切消差。独立图像 API 仍按物理尺寸输出，不能因为 Excel 此行为给所有画布加一行。

新控制样例真实修改锚点并重新公开导出：图例固定 432×252pt、origin=0；值轴保留原 extents 仅移 origin=0；几何图表保留物理尺寸与 chart XML、改 origin-zero oneCell，仅隔离渲染，**不替代原 twoCell 尺寸链路验收**。每对控制/native 的 chart XML 已逐字节核相同。原始代表源和官方 PNG 保留；均非找回 PR10 原包 SHA。

## 原始尺寸对照与未解残差

以下都是对应控制源上的原生同尺寸逐像素对照；differ 不是目测或通过替代品：

| case | before differ | after differ | size |
|---|---:|---:|---|
| controlled-legend-baseline | 62378 | 2535 | 636×371 |
| controlled-legend-style-only | 85627 | 3093 | 636×371 |
| controlled-legend-manual-only | 62209 | 2504 | 636×371 |
| controlled-legend-overlay-only | 5182 | 2535 | 636×371 |
| controlled-legend-manual-edge | 62420 | 2543 | 636×371 |
| controlled-legend-combined | 6376 | 3144 | 636×371 |
| controlled-legend-bottom | 26991 | 2560 | 636×371 |
| controlled-axis-mixed | 6821 | 1687 | 314×221 |
| controlled-axis-explicit | 6476 | 1710 | 314×221 |
| signed-scale-0.1 | 9995 | 2223 | 314×221 |
| controlled-geom-chart-calibri11-explicit | 3472 | 1493 | 324×220 |
| controlled-geom-chart-arial11-explicit | 3582 | 1567 | 364×220 |
| controlled-position-l-0 | 5784 | 2786 | 636×371 |
| controlled-position-t-0 | 12922 | 6201 | 636×371 |
| controlled-position-b-0 | 12128 | 5315 | 636×371 |
| controlled-position-tr-0 | 20691 | 8102 | 636×371 |

- mixed 主网格：官方 [16, 39, 63, 86, 110, 133, 157, 180, 204]，修后 [16, 39, 63, 86, 110, 133, 157, 180, 204]；0轴与主网格同一行。
- explicit 主网格：官方 [16, 53, 91, 129, 166, 204]，修后 [16, 53, 91, 128, 166, 204]。端点与 0 轴已对齐，但中间仍有 1px 栅格相位残差。
- **未解/待复审**：显式非叠加 top/bottom/top-right 及 height-144 等个别中间网格/柱边有约 1px 偏差。已打开红图核对，并非把所有红线都称作 AA；未用样例特判拟合，也未自动改绿。原始 native 高度不同的几何卡只诊断，不给“通过”选项。
- `manualLayout layoutTarget=inner`、wMode/hMode=edge、复杂文本/边框填充等未验证能力仍 NI；不扩称支持所有 OOXML 手动布局。

## 测试、冻结与目测

- `cargo test -p ooxp --lib`：336 passed。
- `cargo test -p ooxp --test xlsx_chart_legend_image --test xlsx_charts --test xlsx_line_charts --test xlsx_bar_charts`：35 passed（8 + 19 + 4 + 4）。包含五位置 absent/empty/1 同图、0 必须不同的真实导出集成断言；新增 signed crossing 与 decimal-zero 单测。
- 原测试中声明固定留白的 helper 改写显式 overlay=0（不再把旧错误默认嵌进断言）；横条/柱方向对比使用真正相同的单个 +8 数据，原 ±8 条形方向断言保留，不放宽像素阈值。
- 冻结 17 项：簇状柱 4、外框/纯色底/plot 填充 3、普通柱值标签 1、折线空值/错误/标签 5、已过共享列 4（含 tiles）。当前库图与已验收库图 **17/17 字节一致，逐像素差 0**。tile 六块拼接与整图差 0。旧官方及库金样哈希验证无写回，不重复请用户验收这些项。
- 实际打开看过：`diagnostics/frozen-contact-1.png` 至 `5.png`（原尺寸拼图，含 frozen/current/exact difference）；`controlled-legend-{baseline,style-only,manual-only,bottom}` 的 triptych；mixed/explicit、signed-scale-0.1、left0/tr0 的 triptych，以及 baseline/manual-only/mixed/tr0 的精确红图。还有之前 native official/source XML 检查。不是只看 differ。

## 产物及安全性

- 证据根：`%TEMP%\pr12-t127-fix-20261120`。102 个源的 `certs/*.json` 当前均 `CLEAN_PUBLIC_OFFICE_EXPORT`，源 SHA before=after、导出前后独占读、无 owner lock；只公共 API、Close(false)、无 Save/Quit/kill。原有 Excel PID16872 未动；本轮新增实例也未 Quit/kill。首次新文件未设只读导致 ReadOnly 断言失败，未作金样；后以专用只读副本与显式 public Open ReadOnly=true 重导，证书全部通过。
- `official-manifest.json` / `controlled-manifest.json` / `origin-manifest.json` / `signed-manifest.json` / `geom-controlled-manifest.json`，对应 `*-export.ps1`；`comparison.json`、`regression.json`、`frozen-shas.json`、`validation.json`。
- Temp 驱动：`t127-prep.py` / `t127-controlled.py` / `t127-origin.py` / `t127-evaluate.py` / `t127-post.py` / `t127-page.py`。signed 初始探索 cache 与格值不一致已纠正并重导，只以当前同源证书为准，不把那次探索当证据。
- 桌面原 `pr12-图例样例.xlsx` 保留，新增 `pr12-图例位置矩阵-t127.xlsx`（说明 + 17 图）。未用 Excel Save。
- 同一桌面唯一 `pr12-待审阅.html` 更新为本轮实现 HEAD，官方/修前/修后三列原尺寸，不拉伸不裁切；原始 native 与诚实控制源分开说明；新 localStorage key，默认全未审，短 JSON schema `pr12-x-review-pass-v1`。旧页备份只在 Temp。

这些是实现与复审记录，不是新用户通过记录；图例、值轴、共享列计划均保持 ⚠️。t126 仅四项明确通过的记录保持不变。
