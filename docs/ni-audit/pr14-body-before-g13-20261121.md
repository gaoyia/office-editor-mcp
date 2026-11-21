继续t171/t172前置优先。HEAD=origin `d826dcb8d9fe06331bab33a9b0d09a9a4671e833` clean/ahead0；g12已发布可复用MC-preserving隔离前置。无legend26对native整PNG相同，有legend冻结26对不同；仅受控族legend依赖成立，不是通用key公式或native↔库像素接受。PR14 OPEN未合并。

## 最新 OOX-1 g12：MC-safe隔离前置实际实现 / legend存在受控验证（已发布、部分待审）

- **普通commit已发布** `d826dcb8d9fe06331bab33a9b0d09a9a4671e833`，parent=`e5d3278a785b2a181a661a9679fee914038ef02b`；正确author/committer 高奕GaoYi，normal hooks。2026-11-21 20:44:34–20:44:37 Asia/Shanghai唯一有限原SSH只读probe确认预期e5d，20:44:37–20:44:42一次普通push成功，HEAD=origin、clean/ahead0。无transport/凭据变更、重试循环或merge/自动接受。
- **真实前置代码，不是docs-only**：新增`manual-check/lib/controlled_chart_fixture.py`、`manual-check/derive-controlled-chart-without-legend.py`及8个单元测试。DOM保留所有namespace声明（包括仅MC属性tokens引用的prefix）、其他节点/部件与ZIP archive/entry metadata；严格源SHA、distinct new output/ZipFile x/no overwrite、源不变，继续调用原read-only preflight。只在.gitignore精确放行这3个手写source/test文件，不放行产物。g11失败草稿原样保留，本次用合法MC-preserving派生器修前置，不修guard绕过去。
- **实际单变量验证**：相对g11只移除52个直接legend节点，其他DOM节点/全部prefix绑定和非chart部件逐项相同，CLI replay与imported helper整包SHA完全相同。新源SHA `620f706f1358acfebedfa42930bad88b4d857baa2836afe9c59a395fce35008d`；冻结g11有legend源 `213adeab2514385afa0276b82120facd58201d5d279f0ccc1c11179b0dcab17b`。原预检通过后一次owned session获得52份新Excel16/DPI106 CLEAN native；52份冻结g11 PNG/证书重新审计，标题/288×180pt/名称/轴/data相同，old/new区分保留。
- **结论严格限定**：本轮无legend的26对preset/custom **native↔native整PNG全部字节相同**，冻结g11有legend26对全部不同。因此在这一受控族中差异依赖legend存在，已超出g11单纯关联；但尚未推导唯一内部key度量原因或通用自动布局数值公式。SDK Legend合同定义legend data/formatting及Layout/Overlay，不给此数值算法。不能把26对native相同冒称native↔library像素通过！native425×266、库424×265，各自原尺寸无拉伸/配准。准确来源`OOXML_DERIVED_REEXPORTED_BY_EXCEL / EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE`，非打印/CopyPicture Bitmap gold。
- **不编造新renderer规则**：g11已实现的preset功能保留并验证，没有因全图残差退回NI。本次实际补上可复用fixture前置；所有Rust renderer/geometry/limit/legacy整文件hash不变，未猜key宽、offset、period或加死元数据/无效painter。sq terminal-gap、lim4 plot真实分支、180°/scatter custom、未验cap/复杂pen/其他宿主边界仍在。
- **测试/回归**：精确mandatory `cargo test --offline --locked -p ooxp --lib --tests -- --test-threads=1`：190targets/**2188passed/0failed/0ignored**；新Python8+旧7=15最终通过，CLI构建通过。首次新增metadata/断言的2个Python失败已修，原日志保留，不称全程零失败。本轮52库baseline与历史185个唯一控制合**237 PNG字节不变**；4scatter仍真实NI，无伪造PNG。接受矩形体SHA `e27043cc735b6bf39742e3662cd6395acfcbc884f8fb13e6da7aed8d5271a978`、opaque PNG SHA `71309897dd3d569d5e26a91d126922283a471232e035333b9150d3de3380c06d`不变，接受范围/零像素准则不降。
- 生命周期：只开一次自有readonly seed/copy；FullName/进程身份、SHA/exclusive/ownerlock/repair审计，finally Close(false)两本、Workbooks0。无Save/Quit/kill/用户工作簿关闭、第二会话；shared exporter/Office binding/ownership/Bitmap freshness guard不改。**20:43:26 Asia/Shanghai最后OS只读观察仍有空Excel应用PID11040**，不是未关工作簿，也不能声称应用已清理；不再启动第二个会话/沿用旧COM refs。本批实现/验证已完成，无需用户操作；后续新Office会话必须重新核验空闲guard。
- **新待审页** `C:\Users\AMD\Desktop\NI-待审-历史归档-20261121-161243\generations\20261121-F29-F30-legend-isolation-mc-foundation-g12\F29-F30-legend-presence-isolation-MC-foundation-待审.html`，82595bytes SHA `0cff25e8a33319d033cb9d6a90d52c4252afca45bc9118578f1b5bc37a32cc04`。INDEX `C:\Users\AMD\Desktop\NI-待审-INDEX-20261121.html`，35817bytes SHA `3a0efc53aa506bcd9d2ed9173d7963445d1c4f53559f829abcccbd453d3e3435`；manifest SHA `3566af8952244f8eb2314e45c735fba1de8c9828ca99ac088db7cc1dec60972c`。554新增文件/1137本地链接核验，67旧artifacts/1858旧文件、两个localStorage原URL/状态/g8a/旧manifest/rollback完整；当前68artifacts=62NI证据/41ID+6PR12。全部默认未审、无通过按钮，g11及以前页面互补保留。
- 仅5文件（3新Python、精确.gitignore白名单、⚠️计划F29/F30）变更；其他47原始行与3个form-feed、main/Pages/page_paint.rs/t131/F45/接受/安全/非法范围保持。49组仍**4接受/5待审/26部分/10NI/1安全/3非法**，不称NI全解。下一缺失基础是可复用legend-key样式来源/自动布局合同或其他独立可做前置，不把此受控依赖泛化为唯一算法，也不转为无界AA/lim4/terminal阈值扫描。

## 历史互补 OOX-1 g11：presetDash合同映射基础（已发布，部分待审；2026-11-21 Asia/Shanghai）

- **已正常发布** `e5d3278a785b2a181a661a9679fee914038ef02b`，parent=`f208bf3a066917e80cb83f992a098ee24bba0e62`，正确author/committer 高奕GaoYi。20:29:17–20:29:22 Asia/Shanghai唯一有限原SSH只读probe确认预期f208，20:29:22–20:29:27一次普通push成功；HEAD=origin、clean/ahead0。无重试循环、transport/凭据变更、force/amend/rebase/hook绕过，PR14仍OPEN/unmerged，无自动接受。
- **实际代码**：普通line显式正w/显式cap、简单不透明pen，flat支持全部10个非solid `prstDash` token，rnd先支持`dash`/`dot`。以ST_PresetLineDashVal明确的每bit一pen宽二进制合同解码为百分比，进入原DashPattern/custom连续run/phase/cap/join/预算/legend路径；不是猜测周期表、独立preset painter或位置补偿。solid/省略dash、默认/省略width与cap和其他reader保持原路径；sq及其余rnd preset未借本轮扩大。
- **合同来源与实证分开**：公共ECMA文本镜像提供binary runs（不是Microsoft-hosted规范，准确注明镜像）；Microsoft SDK枚举文档确认token名称，原网页/来源/SHA保留。一次52项Excel16/DPI106 CLEAN native，同Join control标题、288×180pt、固定轴/data，26个preset/custom对。24直线对=10token×两宽度flat+dash/dot×两宽度rnd，其完整内部dash长度与相对首段相位一致；flat示例4pt dash内部24px/周期42px，8pt为48px/84px，非两个点猜阈值。另保留turn/gap两对，不把不同plot路径的全图差异称phase零误差。
- **不是全图像素通过**：26对native整PNG全都不等！观察到preset/custom的legend/key与plot宽度/第一点/末段裁切一起变化；去legend隔离草稿未进入Office，故这只是布局关联诊断，不是legend为唯一因果源的证明，例flat4pt dash首位置64与65但内部周期相同；库没有平移/拉伸图片或补坐标。基础周期语义先接通，legend/key-layout、位置与AA残差单列。26对库preset/custom整PNG相同证明复用，不能冒充与native相同。52份新证书和源保留，源SHA `213adeab2514385afa0276b82120facd58201d5d279f0ccc1c11179b0dcab17b`，准确`OOXML_DERIVED_REEXPORTED_BY_EXCEL / EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE`，不是CopyPicture Bitmap/打印gold/未修改官方源。
- **回归与真实新增**：26个原NI preset控制新增可输出；本轮原26 custom控制+历史133个去重控制（共159）PNG逐字节不变，历史133覆盖g10全部21项新增/纠正后控制与之前112项，无重复11项充数。4个scatter控制继续真实NI。g7已披露旧变化、两份derived非gold及g8a勘误保留，未改任何接受结果。
- **测试**：精确mandatory `cargo test --offline --locked -p ooxp --lib --tests -- --test-threads=1`，190targets/**2188passed/0failed/0ignored**；7新增Rust覆盖10token binary/尺度、非法值/width、连续run/cap/预算复用、严格gate、未验cap/重复dash/alpha拒绝、plot/legend/turn/gap端到端custom等价、旧solid/省略保护。targeted7、Python预检7与offline/locked CLI构建均通过。接受矩形体SHA `e27043cc735b6bf39742e3662cd6395acfcbc884f8fb13e6da7aed8d5271a978`、opaque PNG SHA `71309897dd3d569d5e26a91d126922283a471232e035333b9150d3de3380c06d`不变；F06既有零像素接受准则不降低。
- **生命周期真实记录**：只开一次自有readonly seed/copy会话，身份/FullName/进程追踪、finally Close(false)两本、Workbooks0，SHA/exclusive/ownerlock/repair全闭环；无Save/Quit/kill/userbook关闭。20:20:31 Asia/Shanghai OS观察仍有该Excel空应用，所以没有再开第二会话；20:28:31 Asia/Shanghai OS只读观察进程0，仅观察、不是主动应用清理。独立去legend草稿被MC namespace预检拒绝（c14未声明），从未进Office/未export/非gold，负例保留，未绕过guard。shared binding/ownership/native exporter/Bitmap freshness guard不改。
- **不掩盖边界**：旧全部geometry/MiterJoin/limit阈值、custom interval与legacy solid rasterizer逐字节保护，未展开lim4参数扫描。g9 lim4真实plot几何跨分支、square terminal-gap、精确180°/scatter custom、未验cap/compound/arrows/复杂alpha/effects/其他宿主仍在。下一具体缺失前置是preset/custom的legend-key布局合同/样式来源区分；去legend隔离fixture须先保留MC声明并通过预检，在安全空闲时再取新native，不能用猜测offset解本轮全图差异。
- **新待审入口** `C:\Users\AMD\Desktop\NI-待审-历史归档-20261121-161243\generations\20261121-F29-F30-preset-dash-contract-g11\F29-F30-preset-dash-contract-foundation-待审.html`，56765bytes SHA `74e0074cc4cbcb1a8e85aa40c3ff9c9edb303eb98dcf4201aaa8ccec6ee25f7d`。INDEX `C:\Users\AMD\Desktop\NI-待审-INDEX-20261121.html`，34324bytes SHA `0036d0dbc2520aabd17fe65f86feb320fd10b69925cb4b1c11125ae77468cb41`；manifest SHA `43dd1ab0621c2f5001692fde888e875190fe0334f0103761d668c6d3425636ce`。397新增文件、859本地链接验证；66旧artifacts/1460旧文件、两处localStorage URLs/状态、旧manifest/rollback不动。当前67artifacts=61NI证据/41ID+6PR12。26卡默认未审、无通过按钮，g10与更早页面全部互补保留。
- 仅chart_paint.rs/chart_stroke.rs与⚠️计划F29/F30前置说明变化，另外47原始行和3个form-feed保留；main/Pages/page_paint.rs/t131/F45/接受/安全/非法范围未改。49组仍**4接受/5待审/26部分/10NI/1安全/3非法**，并非所有NI完成，也没有任何自动接受/merge。无本批所需用户操作。

## 历史互补 OOX-1 g10：非custom显式join共享基础（已发布，部分待审；2026-11-21 Asia/Shanghai）

- **普通commit已发布** `f208bf3a066917e80cb83f992a098ee24bba0e62`，parent=`b0e9dde9146f9dbba4fe765e4b8c3636ec85313e`，正确作者/committer 高奕GaoYi。20:10:41–20:10:46 Asia/Shanghai唯一原SSH只读probe确认预期b0后，20:10:46–20:10:51一次普通push b0→f208成功；HEAD=origin、clean/ahead0。无网络重试循环、凭据/transport改动、force/amend/rebase/hook绕过，PR14保持OPEN/unmerged。
- **实际功能**：普通line显式正w/显式cap、简单不透明的非custom bevel/miter，不再拒绝/静默当round；严格选中后复用已有Join/BevelJoin/MiterJoin与同一coverage union。每个真实连续run两端加帽，gap由既有caller分run；不是伪造巨大dash，不消耗custom dash预算。支持prstDash=solid及验证过的省略dash。旧缺省join/显式round仍走原solid rasterizer，省略w/cap与其他reader保持原路径，不顺便实现其他宿主。
- **先官方判别再接通**：32项新Excel16/DPI106 CLEAN native，统一Join control标题、288×180pt、固定轴/数据，显式object name/title/geometry、MC namespace/关系预检。三几何×4/8pt×三join的18项flat非custom native，与冻结g8对应100pen长custom native整PNG逐字节相同，足以证明本完整run的共享几何可复用，而非声明任意custom pattern等于实线。另含rnd/sq显式join、3种cap省略join、2项省略dash与3项真实gap；native省略join=round、省略dash=solid均整PNG相同。18冻结证书再次逐项核验Excel16/DPI106/owned closure/source/PNG SHA，新增非default12库PNG也与既有custom逐字节相同，round不为关系而重光栅化。
- 新源SHA `c3aa077cd359e62a25b4f74fb7ad27df8f6203f848ede3edafc8cfbfb6f4fa16`；冻结custom对照源SHA `9700c4c4bc9347bfebe3af4c9dd38f9fd65826767a74c9c3c1fcd588cff1da53`。准确 `OOXML_DERIVED_REEXPORTED_BY_EXCEL / EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE`，非CopyPicture Bitmap/打印gold/未修改公共API源；32份新证书、18冻结PNG/证书/源随generation离线保留，无clipboard重试或guard绕过。
- **变化与不变分开**：32库控制全部可输出，其中19原NI获得基础；两个原先错误round的未接受控制有意纠正：`acute-w8-rnd-bevel`旧→新123px、`acute-w8-rnd-miter`192px。两张旧PNG均逐字节等于round控制，确证原忽略声明；old/native/new三列保留，未把旧图冒充gold或混成zero-change。另11本轮旧round/default与更早101份当前PNG合**112份逐字节不变**（含g7已披露变化后的partial和两份非官方derived）；4个scatter控制继续真实NI。控制数量不是49组接受数。
- **边界独立保留**：chart_stroke.rs、MiterJoin、miter阈值、Canvas共享join几何函数体、legacy solid rasterizer均逐字节核验不变，未调plot/字体/AA。g9 lim4源于plot几何跨分支的真实问题原样保留，未重开参数扫描、更未补偿阈值/坐标。本轮新非custom native隔离用lim8，其余lim沿已有数学合同，不冒称全域像素一致。square custom terminal-gap、精确180°/scatter custom、非solid preset/compound/箭头/复杂alpha/effects/其他宿主仍留界；旧round与axis/ticks/位置/粗pen key残差不假称解决，g8a数字勘误永久互补。
- **最终完整必跑** `cargo test --offline --locked -p ooxp --lib --tests -- --test-threads=1`：190targets/**2181passed/0failed/0ignored**，6新增Rust；覆盖显式gate/严格parser、旧省略/default/其他reader保护、真run共享/不占dash budget、旧round/单edge像素、rnd显式模式端到端、gap/endcaps/复杂pen拒绝。Python预检7通过，ooxp-cli offline/locked构建通过；接受矩形体SHA `e27043cc735b6bf39742e3662cd6395acfcbc884f8fb13e6da7aed8d5271a978`、opaque PNG SHA `71309897dd3d569d5e26a91d126922283a471232e035333b9150d3de3380c06d`不变，F06零像素准则不降低。
- 生命周期：只开一次自有readonly seed/copy，身份/FullName/进程追踪，finally Close(false)两本、Workbooks0、SHA/exclusive/ownerlock/repair闭环。无Save/Quit/kill/userbook关闭/叠会话；20:12:26 Asia/Shanghai末尾OS只读观察Excel进程0，仅观察不称主动应用清理。shared native exporter/ownership/binding/Bitmap与freshness guard未改。
- **当前待审入口** `C:\Users\AMD\Desktop\NI-待审-历史归档-20261121-161243\generations\20261121-F29-F30-solid-explicit-joins-g10\F29-F30-noncustom-explicit-joins-foundation-待审.html`，39209bytes SHA `3d4df791572610c7fae9531ebcff4d86ab5c39a81ec94dd49c2e55fe5b834601`；32卡、两项old/native/new三列，全部默认未审、无通过按钮。INDEX `C:\Users\AMD\Desktop\NI-待审-INDEX-20261121.html`，32953bytes SHA `933e3381fa208e9ab6ec9608f06f8e6d19b54f0f408d9fc80a2c45bd3b70d150`；manifest SHA `81b0f5237614dc7743c995018a396646f0a163efb0df64f0fbd3802168263032`，323新增文件与660本地链接核验。65旧artifacts/1136旧文件、两个localStorage URLs/状态/所有旧manifest/rollback保持；现66artifacts=60NI证据/41IDs+6PR12，不覆盖g8a或g9真实剩余。
- 仅改chart_paint.rs和⚠️NI计划F29/F30；另外47原始行与3个form-feed保持，main/Pages/page_paint.rs/t131/F45/已接受/安全/非法范围不动。49组仍 **4接受/5待审/26部分/10NI/1安全/3非法**，全新增部分待审，无接受/merge，不称NI全解。下一可做缺失前置是有多width native周期证据的presetDash映射等，不能猜表或把lim4精度变成无界追参。

## 历史互补 OOX-1 g9：custom miter求交/逐顶点limit基础（已发布，部分待审；2026-11-21 Asia/Shanghai）

- **代码已实际发布**：本轮普通commit `b0e9dde9146f9dbba4fe765e4b8c3636ec85313e`（parent/g8=`d8515e3cc8a2a8baf781816fe5973a1d84497424`），正确作者/committer 高奕GaoYi；19:49:17–19:49:21 Asia/Shanghai唯一一次原SSH只读probe读到预期cd51后，19:49:21–19:49:25仅一次正常push成功，cd51→b0e9，g8+g9两提交一并发布。HEAD=origin，clean/ahead0；无重试循环、凭据/transport变更、force/amend/rebase/hook绕过。本PR保持OPEN/unmerged，以下g8“本地未发布”只代表当时历史快照。
- **实际实现**：独立 `Join::Miter {limit}` 与 `MiterJoin`；普通line显式正w/显式cap/简单不透明custom dash使用两条外offset边交点。按MS-OI29500 20.1.8.43内角至外角距离/全线宽（等价tip至中心/半宽）逐顶点限长，超限复用bevel；未超限使用两切边+局部disk凸包的真实边界，避免假chord接缝。mask包含真实tip再裁canvas，近反转先比limit避免巨大世界空间mask；无经验角度表/COM偏移/补偿。lim非负32位整数百分比，默认800000来自Office defaults并有omitted实证，百分号词法不扩大。旧round/bevel/solid/default/短点/单edge/phase/budget保留，非custom/其他宿主不顺便放开。
- **证据**：一次16项新Excel16/DPI106 CLEAN native（gentle8、acute4/8、valley8×lim1/2/4/omitted），复用并逐SHA核验g8六项lim8 native；同Join control标题、288×180pt、固定轴/数据，显式name/title/geometry与MC关系预检。新源SHA `c9e686f07df2fb93d2cf2d38e28f9ad4a954f662c5f04ab720d2ff38b012d738`，冻结g8源SHA `9700c4c4bc9347bfebe3af4c9dd38f9fd65826767a74c9c3c1fcd588cff1da53`。准确标签 `OOXML_DERIVED_REEXPORTED_BY_EXCEL / EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE`，非CopyPicture Bitmap/打印gold/未修改公共API源；无更多Office会话/clipboard重试。
- Native lim1整PNG=bevel，gentle lim2=lim8，acute/valley lim2逐顶点混合，lim4/omitted在本矩阵整PNG=lim8。22个原NI miter控制现在可输出，但**仍部分待审**，不把控制数当49组接受数。旧g8成功13+更早66共79份当前PNG逐字节不变，保留g7已披露六项变化后的partial版本与2项非官方derived；4个scatter控制仍真实NI，无伪造库PNG。
- **必须单列的真实剩余：lim4分支并未与native一致。** acute4/8、valley8库的当前plot几何令尖顶超限回退bevel，native仍miter；不是仅AA。对acute4两臂远离join的原图blue-minus-red加权中心线独立拟合，native推定miter比3.941、库4.047，最大拟合残差约0.031/0.025px，跨越传入lim4；这些估计只作诊断，不输入renderer/不改变limit/不补坐标。各自425×266 vs424×265原尺寸，无stretch/配准；原始22对图和3张lim4卡均红框披露。初始post-render审计错误地要求库lim4与native同PNG关系，已保留该失败记录和105份真实完成输出，随后只做同exe/源码/全部输出SHA核验与audit-only继续，未重导Office/改PNG/假称相同，也没降低任何已接受标准。axis/ticks/位置/AA和g8a粗pen key剩余继续明确；不猜6px clamp。
- **最终完整测试** `cargo test --offline --locked -p ooxp --lib --tests -- --test-threads=1`：190targets/**2175passed/0failed/0ignored**，8新增Rust（交点/数学limit、镜像缩放反向、无内部假缝/尖角bbox、退化/近直线、严格parser/default/范围、真实line逐顶点、短点/端帽/budget/offcanvas）。第一轮full捕获意外放开旧flat非custom miter，已恢复旧gate、补flat/sq断言后重新完整通过；日志均保留。实际CLI包ooxp-cli offline/locked构建通过，Python预检7通过。接受矩形体SHA `e27043cc735b6bf39742e3662cd6395acfcbc884f8fb13e6da7aed8d5271a978` /opaque PNG SHA `71309897dd3d569d5e26a91d126922283a471232e035333b9150d3de3380c06d`不变，F06零像素准则不降低。
- 生命周期：只读自有seed/copy身份/FullName/进程与finally Close(false)两本闭环，Workbooks0、源SHA/exclusive/ownerlock/repair均通过；无Save/Quit/kill/userbook关闭/叠会话。19:50:36 Asia/Shanghai最终OS只读观察Excel进程0，仅观察不称主动应用清理。shared binding/native exporter/Bitmap与freshness guard未改。
- **当前待审入口** `C:\Users\AMD\Desktop\NI-待审-历史归档-20261121-161243\generations\20261121-F29-F30-miter-limit-g9\F29-F30-miter-intersection-limit-foundation-待审.html`，30876bytes SHA `2e7c9ac656317016ffb9b89ab7b9f29c9c9bb281cb2a577ec4fe9cc5591b8b53`；22对原尺寸卡，默认未审、无通过按钮，3张lim4特殊剩余明确红框。INDEX `C:\Users\AMD\Desktop\NI-待审-INDEX-20261121.html`，31528bytes SHA `8bfc20d152438dd796afb50131e36f960e725fa5fdf95dfa1abd3368e796ef38`；manifest `C:\Users\AMD\Desktop\NI-待审-历史归档-20261121-161243\generations\20261121-F29-F30-miter-limit-g9\manifest-generation-9.json` SHA `d6ba03a7ae8aac03df8a8cf9637bcbe545d4121ac4b4086098add579e663fc4f`，219新增文件核验；530本地链接通过。64旧artifact和916旧文件、两个localStorage原URL/状态/全部旧manifest/rollback保持；现65artifact=59NI证据/41IDs+6PR12，g8a测量勘误永久互补，未覆盖旧页。
- 只改chart_stroke.rs、chart_paint.rs、⚠️NI计划F29/F30两行；另47原始行与3个form-feed保留。main/Pages/page_paint.rs/t131/F45/已接受/安全/非法范围不改。49组仍 **4接受/5待审/26部分/10NI/1安全/3非法**。square terminal-gap、精确180°/scatter custom、preset/复合/箭头仍留界，无自动接受/merge，不称全NI解决。下一可做基础为非custom显式join的迁移/判别验证或有实证的preset周期；lim4的plot几何依赖应独立调查，不以阈值补偿伪装解决。

## 历史 OOX-1 g8：以下为19:27时本地未发布快照，现已随g9正常发布

> **发布状态必须区分：本地commit `d8515e3cc8a2a8baf781816fe5973a1d84497424`，clean/ahead1；远端/本PR代码仍 `cd51e861d107a44366ed3471f0cc2647c0e28f7d`，没有新bevel代码。** 19:24:10–19:24:15 Asia/Shanghai仅一次正常push，5.078秒exit128：`Connection closed by 28.0.0.62 port 443`。未重试、未切换transport、未读改凭据、未force/amend/rebase/绕hooks；这是连接关闭事实，不作认证/账户需重连诊断。本次API只更新PR状态文字，不以API绕路上传代码。普通commit已安全保留，网络不阻止后续本地前置实现。

- **实际新增基础**：显式正w/显式cap/简单不透明custom dash的bevel join。Join与Cap独立，经LineSeriesStyle/Series传到真实line renderer；用两条offset边的弦截局部join disk，外轮廓为bevel hull，内部与两条butt body充分重叠避免假AA缝，复用g7每piece coverage union/一次blend。无角度表、COM偏移或样本分支；不把join塞进dash period。单edge和正短round点没有内部join，明确复用已有端帽/一次圆点primitive，新增组合回归防止吞点。旧solid/default-round、phase/budget不改；miter/limit、preset、非custom-bevel/其他宿主不扩大。
- **一次判别性native会话23项**：19line（gentle/acute/valley ×4/8pt×round/bevel/miter(lim800000)共18，加omitted对照）+4 XY single/精确retrace；统一Join control标题、288×180pt、固定轴/单series、明确name/title/geometry/MC关系预检。新源SHA `9700c4c4bc9347bfebe3af4c9dd38f9fd65826767a74c9c3c1fcd588cff1da53`，所有父源保持；23份Excel16/DPI106 CLEAN，准确 `OOXML_DERIVED_REEXPORTED_BY_EXCEL` / `EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE`，非CopyPicture Bitmap/打印gold/未修改公共API源。native本轮以flat cap隔离bevel，已有其他cap合同复用，不冒称每种组合都有本轮gold。
- Native round/bevel/miter在三几何/两宽度明确可区分；gentle-w4 round与omitted整PNG字节相同。**精确180°回描不是全图等价**：single/retrace共享body和其余图像像素相同，但B转头4pt15px、8pt51px不同，含白→不透明蓝，是真退化join几何而非只AA。只据此验证body不反复混色，不虚构退化painter；scatter custom仍NI。square terminal-gap保持原边界，不再扫无合同阈值。
- 当前23库控制13成功（6原NI bevel获得基础，原7round/omitted整PNG不变），10控制仍NI=6miter+4scatter-custom；不是49组中的10NI被同一矩阵全部覆盖。另66份旧当前状态PNG全部逐字节不变，含g7六个有意变化后的partial版本和两个明确非官方derived；原/native/旧库/新库区别不回写抹除。
- **最终完整必跑suite** `cargo test --offline --locked -p ooxp --lib --tests -- --test-threads=1`：190targets/2167passed/0failed/0ignored，7新增Rust；CLI offline/locked成功，Python预检7通过。第一轮完整2166通过后发现单edge/圆点join分支组合缺口，修复并增加第7回归后重新完整跑2167，初轮日志保留；不是拿改前测试冒称改后通过。接受矩形体SHA `e27043cc735b6bf39742e3662cd6395acfcbc884f8fb13e6da7aed8d5271a978` /opaque PNG SHA `71309897dd3d569d5e26a91d126922283a471232e035333b9150d3de3380c06d`不变，F06零像素标准不降低。main/Pages/page_paint.rs/t131/F45/S01/非法范围不动，正确作者与正常hooks。
- 生命周期：只读自有seed/copy身份/FullName/进程拥有关系、finally Close(false)两本均闭环，Workbooks0、SHA/exclusive/ownerlock/repair验证；无Save/Quit/kill/userbook关闭、无第二批Office。19:26:16 Asia/Shanghai结束OS只读观察Excel进程0，仅观察不宣称主动应用清理；shared绑定/export/Bitmap与freshness guard未改。
- **当前待审统一入口（含统计勘误）** `C:\Users\AMD\Desktop\NI-待审-历史归档-20261121-161243\generations\20261121-F29-F30-bevel-key-profile-correction-g8a\F29-F30-bevel-truejoin-含统计勘误-待审.html`，1594bytes，SHA `d353008697efcfcc87cb8bb94892e215f0587fb821eca37405eea7f9f18b7c05`；固定勘误页头+原23卡iframe，默认未审，无通过按钮。原g8证据页 `C:\Users\AMD\Desktop\NI-待审-历史归档-20261121-161243\generations\20261121-F29-F30-bevel-truejoin-g8\F29-F30-bevel-truejoin-retrace-待审.html`，22191bytes SHA `6d11e2c58a3887e5c1e56e2309db8797990b633165e90caa14f24c364cdd1e0a`，字节保持，两个URL均永久保留。
- 统计勘误透明保留：g8初版把device pen宽度6/12误称“纯蓝行数”；按RGB(23,93,181)精确行统计，4pt native6/库5，8pt native6/库11。g8a新固定勘误入口、JSON与INDEX显著说明；未覆盖初版或改动代码/23 PNG/测试/状态。粗pen legend key确有独立剩余，不从两个宽度猜6px clamp；axis/ticks/位置/AA亦未冒称全图通过。
- INDEX `C:\Users\AMD\Desktop\NI-待审-INDEX-20261121.html`，30100bytes SHA `8cfccdd1105316bda16dbe70340f3088a139bd9f58eef247c7399b413ac2a8d7`；g8 manifest SHA `de603518fdb7a80cb7a916f1773d4b7dca71c0417a5377a71fbb119377593813`（184文件），g8a manifest SHA `a037d9178f2b3fc3e6371ac6caaacae6707111a592d75293b3694c8689667034`（5文件）；450本地链接核验。原62页+g8初版共63页全部保持，现64 artifacts=58NI证据/41ID+6PR12，旧localStorage URLs/状态/manifest/原rollback scope10均不改；g3–g7继续互补。
- 实际49行计划仍 **4接受/5待审/26部分/10NI/1安全/3非法**，只更新F29/F30，47行与3个form-feed保持。所有新工作部分待审，无接受/merge。下一本地可做的是miter偏移边求交/limit前置；square terminal、精确180°、粗pen key等明确留界，不把网络推送失败说成所有工作受阻，也不声称全NI解决。

## 已发布历史互补 OOX-1 g7：bounded square末端gap诊断 + 单dash union覆盖基础（部分待审，2026-11-21 Asia/Shanghai）

> 普通commit `cd51e861d107a44366ed3471f0cc2647c0e28f7d`（高奕GaoYi <17539193+gaoyia@users.noreply.github.com>；正常hooks），18:53:59 Asia/Shanghai唯一普通push exit0，4.281秒，`4b3198a9..cd51e861 HEAD -> feat/xlsx-chart-ni-20261120`。HEAD=origin、clean/ahead0；无force/amend/rebase/--no-verify、无网络重试/凭据/传输修改。只改chart_paint.rs与⚠️计划，main/Pages/page_paint.rs/t131/接受项不动。

- **实际代码基础**：同一个多edge custom dash先合并各segment的max AA coverage，再对底图每像素blend一次；消除turn/回描边缘反复混色，保留原round内部join、独立first/last caps、总弧长phase与piece budget。mask限定到该piece的canvas内bbox，edge仅扫描局部bbox。单edge、正短round点、solid/default旧rasterizer不改。仅每piece内部union；跨pieces极小gap/精确面积/native AA、miter/bevel/preset并未顺便解决，不能称全图通过。
- **一次有界native矩阵后停止猜阈值**：30项=2/4/6pt × d200/sp25/50/100/200及d300/sp100 × flat/sq，同标题Dash phase control、288×180pt、相同8类三series/真实gap数据。新源SHA `914e22edd48ce7f8ba8c809a7efe24fc0f64347bbfa893d50a4311f67319c7c7`，父源5dd8330b…与g6源不变；Office前MC namespace/关系预检，明确name/title/geometry。30份Excel16/DPI106 CLEAN，准确为 `OOXML_DERIVED_REEXPORTED_BY_EXCEL` / `EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE`，不是CopyPicture Bitmap/打印页gold/未修改公共API源。
- **square terminal选择本轮未伪装修复**：w4 d200/sp100 native H最后310..321在sq不变，否定无条件最后painted interval方帽；w4/sp50 H313..324→313..327，否定仅end==run.length；w2/sp50 G还新增326..327终段，不是仅延长旧末段319..323。没有唯一一般规则，保持原选择、记录语义边界，不称AA/不猜半宽阈值或preset表。按t171/t172转实补独立可定义的union覆盖前置，而非无尽terminal参数调优。
- **回归结果如实分类，不冒称全同**：64项重放中58份整PNG字节相同=30新矩阵+28旧项（包括g6、solid/default/省略w/F06）。6个旧“部分、未接受”custom true-turn因单次coverage有意改变：flat1=11、flat4=28、round4=8、square4=28、multi4=35、flat2=29像素。原6 PNG/页面保持原字节，g7并列原native/旧库/新库，未降低任何接受标准。复用6份已有5dd8330b源native证书，未增加Office会话。
- 另2个turn回归fixture为 `DERIVED_REGRESSION_FIXTURE_NOT_OFFICIAL_GOLD`，未经过Excel；各53像素变化，独立几何/一次混色合同验证，不冒充官方gold。v1的c14序列化前缀缺失由纯预检在Office前拒绝，未打开；v2修正namespace、改前2份PNG字节不变；v2源SHA `91ebf3b39fc15e062ff742ab5e1b093dbe56de6125cb299251ac115226b526e9`。不把30直线native控制冒称turn像素锁定。
- **完整测试** `cargo test --offline --locked -p ooxp --lib --tests -- --test-threads=1`：190targets /2160passed /0failed /0ignored，4新增Rust（回描单次覆盖、三cap折点max coverage、逆向/零edge/越界、turn phase/budget）；CLI offline/locked成功，Python预检7通过。矩形体SHA `e27043cc735b6bf39742e3662cd6395acfcbc884f8fb13e6da7aed8d5271a978` /tracked opaque PNG SHA `71309897dd3d569d5e26a91d126922283a471232e035333b9150d3de3380c06d`不变，F06零像素标准不放宽。
- **生命周期**：只读自有seed/copy身份/FullName/进程拥有关系保留；finally Close(false)两本均闭环、Workbooks0、SHA/exclusive/ownerlock/repair核验；无Save/Quit/kill/用户book关闭。关闭两本时应用仍在；18:54:32 Asia/Shanghai最终OS只读观察Excel进程0，仅观察不宣称主动应用清理。无第二批Office窗口堆积，绑定/缺Bitmap与freshness guard及shared export helper未改。
- **新增待审页** `C:\Users\AMD\Desktop\NI-待审-历史归档-20261121-161243\generations\20261121-F29-F30-terminal-gap-union-g7\F29-F30-terminal-gap-union-coverage-待审.html`，41180bytes，SHA `bbfa48426e069b973e66c657d1ec69c559a038ca1a6ffceaa200b7e7c64f8a95`；默认未审、无通过按钮，各自原尺寸无拉伸/配准。原61份review及其URL/localStorage/状态/旧manifest/原rollback scope10均字节保持。现62 artifacts=56NI证据/41ID+6PR12；g3–g6互补保留。
- 主INDEX `C:\Users\AMD\Desktop\NI-待审-INDEX-20261121.html`，27164bytes，SHA `13aff3274de813a02d7f0ea61c435ec7594b0bf4d218530752e9b8a8f7612dfd`；g7 manifest SHA `5cfbf7428f568bf9157d1d5b91819aff7e8386877f6d36ae7b6690ad81b1a7e4`；218新增文件SHA和362本地链接验证，独立guarded INDEX-only rollback，不移动/删除旧证据。
- 49组实际计划审计 **4接受/5待审/26部分/10NI/1安全/3非法** 不变；只改F29/F30行，另47行及3个form-feed保持。全部新工作待审，无自动接受/merge。下一合同性前置是独立controlled native turn/回描检验round-union及join几何，再考虑miter/bevel或有native周期支持的preset；不是再扫无合同的square阈值。当前无用户操作阻塞，并非全部NI解决。

## 历史互补 OOX-1 g6：正短round/dot与末段phase（部分待审，2026-11-21 Asia/Shanghai）

> 新普通commit `4b3198a99e0a46ec490463fb2d89f6907f2c55ec`（高奕GaoYi、指定noreply、正常hooks）于18:36:40 Asia/Shanghai唯一普通push exit0（4.484秒），`a56f1440..4b3198a9 HEAD -> feat/xlsx-chart-ni-20261120`。HEAD/tracking一致、clean，无force/amend/rebase/--no-verify、无网络循环或凭据/传输修改。先前6687/a56已发布且保留。

- 实现round正d≤100%：`effective_d=max(logical_d,device_pen)`，body=effective_d−pen，周期effective_d+space。零body为一次现有圆点primitive，不丢点、不双叠两帽、不沿错误较短周期前进；保留精确零点，避免fractional space累计后加减pen浮点抵消造成负body。正长dash原运算次序、所有旧solid/flat/省略宽度及两图例共用路线不改。zero d/sp、缺省cap/w、alpha/复杂色等原NI不放开。
- 一次现有安全native verifier，14项同标题Dash phase control/288×180pt/相同8类数据：H全程40、B仅前两点60、G仅后两点20、真实gap重启。新派生source SHA `eb912162879c6bee61cf93d9a0e784101f7c2af695d70b0bdfb67d04d42d16cf`，父源5dd8330b0237a20fcadbe339b6e8026092ee1cb07560d20107326e5c15d8ddcd不变；Office前MC namespace/关系纯预检成功。14份Excel16/DPI106 CLEAN证书准确标注 `OOXML_DERIVED_REEXPORTED_BY_EXCEL` / `EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE`；非CopyPicture Bitmap、打印gold或未修改公开API源。
- 4pt d25/50/100%在sp100%原生整PNG全同，12px点周期；1pt d50/100%也整PNG全同，薄点有颜色但无opaque纯色，不误写没画。d101%继续推进phase，不能吸回点周期。库同样两组整PNG全同；只是各自组内同义证明，**不是库/Excel全图像素相同**。新14库均可输出，5个原NI控制有点基础；原9个本矩阵成功项及另外12+8份旧PNG严格字节不变。
- 标准完整 `cargo test --offline --locked -p ooxp --lib --tests -- --test-threads=1`：190targets/2156passed/0failed/0ignored，8新增Rust；CLI ooxp-cli offline/locked构建、Python预检7通过。点/混合表/总弧长/真gap/末段nominal clip/两图例/shared budget及闭合polyline不混成点测试。矩形body SHA e27043cc735b6bf39742e3662cd6395acfcbc884f8fb13e6da7aed8d5271a978、tracked opaque PNG SHA71309897dd3d569d5e26a91d126922283a471232e035333b9150d3de3380c06d不变，F06零像素门槛不放宽。page_paint.rs/t131/接受F01-F03/F45/S01/非法项不改。
- **新暴露的真实语义剩余**：square d200/sp50末尾短gap，native H最后纯色313..327，flat313..324；当前`end==run.length`的true-end-only末帽选择不足。不是AA，不声称已修复，不从单个phase猜3px/半宽阈值。round d200/sp50、d300/sp100支持先算nominal body再clip而非拉满尾段。zero d/sp、精确pulse恰落run末端未由本矩阵扩大支持。下一步受控改变terminal gap/width/d/sp，再实现一般末帽选择。
- preset/truejoin依赖已复核：本矩阵只有custom，不能捏造prstDash周期表，非solid preset仍NI；当前custom保留round join并逐segment blend，真正union coverage/miter/bevel另需基础，未被圆点primitive顺带解决。不要求全图AA完美才做基本语义，也不把语义缺口伪装成AA。
- **生命周期真实边界**：自有seed+copy身份/FullName追踪finally Close(false)均核验，Workbooks0、exclusive/ownerlock/repair及源/副本SHA闭环，shared binding/ownership/guard原字节不改，无Save/Quit/kill/用户book操作。关闭两本时PID49852/应用窗口仍被观察到，不能把book关闭冒称应用窗口已清理；之后未再开Office，不堆下一批窗口。未持有旧COM refs或关闭别的book来清外观。
- g6永久页：`NI-待审-历史归档-20261121-161243\generations\20261121-F29-F30-short-round-phase-g6\F29-F30-short-round-dot-phase-待审.html`，18458bytes SHA b143cea23be65de1be61b175bbb02f4401a9c969264e1323c258d5ebbc40d15a；默认未审、无通过按钮，square语义剩余醒目标注；图片自身原尺寸、不拉伸/配准。
- INDEX25875bytes SHA1be789c322af8a6772b925a56b41e654cfd0199c0519f567740c8ab6a717562d；g6 manifest SHAfc86a16b297870f52149147147470b14424b9f27f7d636271ad1142ba902aaec。110新增文件SHA、225本地链接验证。原60页内容/位置/review state/localStorage URL/旧清单/原10页rollback scope保持；现61review artifacts=55NI证据/41ID+6PR12。g3/g4/g5对宽度、多stop/turn/实线互补，不被本直线phase矩阵替代，g6独立guarded INDEX-only rollback不删除页面。
- 已推送计划49组仍4接受/5实现待审/26部分/10NI/1安全/3非法；只是扩展F29部分子集，不是5个组解决或全NI完成。PR保持open/unmerged，新增全部待审。

### 已发布g4/g5基础（历史范围，完整证据保留）

6687显式pen nearest/最小1px（同源显式2.25pt=3px、省略w=4px），flat多stop/跨折点/gap/图例与共线简化；a56分离内部dash帽与run端帽，round逻辑长度含帽。g4页488359bytes SHA45ae1a368e59eb01f92c18e462ea8c680b870cf9185a635e6b16eaa301ede98c、g5页15925bytes SHA11ef1b59e12952c1806987d8c33ad886f77d5cb4ac7ebfd9055b627a7fc23eda均原样保留。它们对短round仍NI及true-end-only的描述是当时基础/历史边界，由g6逐步推进和收窄，不冒称全部帽phase已解决。以下旧阶段都是历史快照，当前以本节和49行主表为准。

## 历史阶段 t172u OOX-1续：F29/F30共享描边与显式端帽基础（部分待审，2026-11-21 Asia/Shanghai）

- 新提交 `1b46cb66624b80ab046ec0aa355d0d04cc1a9c6d`（正确作者高奕GaoYi）：共享Style/Cap/Run/Segment/CapSegment几何；折点间连续弧长，真实空值gap才另起run，重复零长边不截获端帽。支持普通实线line系列显式rnd/flat/sq端帽及实际线宽/端帽图例key；round旧路径及未显式cap行为保持不变。无预设虚线周期表、补偿偏移或桌面依赖。
- F30由仍NI转为部分待审；F29仍NI，已具备连续弧长前置。dash/customDash、compound、inset、箭头、非round miter/bevel，以及scatter/area/marker/bar-outline新端帽仍有限制；非round新路径拒绝未实现的alpha/颜色变换，而非悄悄拍平。
- 受控6项同标题/同数据/288×180pt，rnd/flat/sq×1/4pt，含水平/折线/真gap。修正源SHA `f800786941f516899f9a65a55584088243949b9333d6d0a18a66598eca8b60cf`，Excel16/DPI106 六CLEAN_PUBLIC_OFFICE_EXPORT；SourceKind=OOXML_DERIVED_REEXPORTED_BY_EXCEL，ExportKind=EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE，不冒充CopyPicture Bitmap/打印页gold。原始F29公共API源SHA `4beb05944d53aed6975dd5c0fb07e1d1be3da31893878510b03096e99ca1d9ff` 不变。native 424/425×265/266、library424×265，原尺寸展示，无拉伸。
- 初次派生源 `4025baabac85afc57ec5c2145e091463ebaaae0ff6e1097a7bac6be1ca4ff5a7` 因序列化令MC Choice/@Requires c14未声明而Open失败，0PNG/0CLEAN；owned seed已finally Close(false)，copy未成功打开，不把aggregate Closed=false改成成功。新增只读ZIP/XML命名空间预检和7项测试；独立修正版先内联预检再成功导出，不更换绑定/guard。成功运行owned seed+readonly copy均finally Close(false)、Workbooks0、FilesClean=true；无Save/Quit/kill/用户工作簿操作。17:11后Excel进程count1仅为观察，不推断空闲或主动清理。
- 最终全套 `cargo test --offline --locked -p ooxp --lib --tests -- --test-threads=1`：190 targets、2124 passed、0 failed/ignored；13项新Rust测试；Python预检7 passed；CLI build成功。8份旧整PNG字节回归不变（默认line/outline+F06六项）。新round样本本体不变，显式实际线宽图例为有意改进。保护的矩形测试体SHA `e27043cc735b6bf39742e3662cd6395acfcbc884f8fb13e6da7aed8d5271a978`、opaque PNG SHA `71309897dd3d569d5e26a91d126922283a471232e035333b9150d3de3380c06d` 均不变；未动page_paint.rs/t131/其他分支/已接受项。
- 未粉饰残差：受控1pt native仅1纯色蓝行，library2行；4pt均6行但位置约2px差。未用两点推断取整表/offset，保留旧宽度规则。下一高价值前置为多宽度、显式与省略w的受控pen矩阵，然后才是依赖线宽的dash。不是“全部NI已解决”。
- 新永久待审页：`C:\Users\AMD\Desktop\NI-待审-历史归档-20261121-161243\generations\20261121-F29-F30-shared-stroke-g3\F29-F30-shared-stroke-cap-foundation-待审.html`，241765 bytes，SHA `f09ae8813843d28e2cf164c5a03ce9fde0a48653c0f5fb37a6ae7b05000a7c56`；默认未审、无通过按钮。F30为新增主要入口，F29旧NI页仍主入口并互链前置。INDEX新增g3互补证据，不移页/改URL。
- 主INDEX 21694 bytes，SHA `ef4d12216472fb6418c84712da3ecd6bc99f0217b4bfd2ef5985ecc82202e72c`；g3 manifest SHA `05d83d6325d0078717c97900312fc63e838d581406998e1f0ae91cdb123b7833`。原57页字节/位置/状态、g1/g2及原rollback范围不变；现58 review artifacts，95本地链接验证，g3 73文件/g2 54文件核对。
- 网络收尾：此前F06两次失败保持为历史事实；本批先实施，不等网络。一度限时12秒、原传输只读ls-remote于7.734秒成功，remote仍28009a29；随后唯一一次普通push于17:12:40 Asia/Shanghai成功，将F06 `c740869bd40b277b733112e3331251392c692140` 与本次1b46一并发布。没有循环重试/换凭据/换传输。随后收到机器离线通知，停止新本机任务；最后可核对的本机状态是push后HEAD=origin且clean，离线后不能声称再次核验。更新PR正文不当作代码推送证明，实际push回执另存。
- 49组审计：已接受4 / 实现待审5 / 部分25 / 仍NI11 / 安全1 / 非法3。新增功能全部待审，未自动接受、未合并。

---
以下为此前阶段记录；F06“尚未推送/阻塞”仅对应当时16:42 Asia/Shanghai状态，本次成功push已解除该传输阻塞。

## 历史阶段 t172u OOX-1续：F06停止表 / 分段采样基础（部分待审，2026-11-21 Asia/Shanghai）

> **推送阻塞，未冒称远端代码已更新：** 本地提交 `c740869bd40b277b733112e3331251392c692140` 已完成且工作区干净；首次及一次相同普通push均在约152/151秒后报 `Connection closed by 28.0.0.62 port 443` / `fatal: Could not read from remote repository.`。已停止重试，未换传输/凭据/镜像或绕过任何guard。16:42 Asia/Shanghai复核PR仍OPEN/unmerged、远端HEAD仍 `28009a29e6a498bff4653484d27f4ae6b4097b7d`。下面实施/测试均为本地已完成结果，待普通push网络恢复；本次只更新PR技术状态，不合并/验收，不把PR正文更新当作代码推送成功。机器本身仍可通过授权工具读取，未证实需要用户重连账号或其他动作。

- Commit `c740869bd40b277b733112e3331251392c692140`：新增共享有序StopTable、严格端点/位置检查、二分区间与局部归一化、Arc共享；矩形轴向颜色缓存。只接已有0/90°几何，没有新增投影、COM补偿/像素偏移、字体表或硬编码桌面依赖。
- 六项同标题/同数据/288×180pt受控派生源，Excel16/DPI106按显式对象名只读Chart.Export。SourceKind=OOXML_DERIVED_REEXPORTED_BY_EXCEL；ExportKind=EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE，六CLEAN_PUBLIC_OFFICE_EXPORT证书，不冒称CopyPicture Bitmap/打印页gold。源SHA `18e75667fc02f1a6507f893264661de3f4a73ea602a82bd79c939f96cf1d407e` 前后不变。
- 判别实测支持3/4停止点分段encoded-sRGB；机械复用旧两停点gamma/Office权重会最大差48–50。新多停点背景剖面maxΔ1，仍未达到零像素接受；旧两停点原gamma路径完全保留，四个旧源完整PNG逐字节相同/0diff。未降低旧验收标准。
- 全图仍有实质残差：native标题更大且两行、库单行，plot预留/自动刻度不同，以及265/266高度、字形/AA；新页逐图列公共区域差与尺寸，无伸缩/配准，不把剖面max1偷换成全图通过。缺端点/重复停止/透明或非sRGB/变换/path/斜角/pattern/blip及系列/扇区新几何仍NI。
- 自有seed+copy同会话finally Close(false)，Workbooks0、源SHA/独占/无锁/无repair，未Save/Quit/kill/动用户书；结束当时空应用仍被观察到，16:35:49 Asia/Shanghai后续只读观察Excel进程0（不是主动清理）。共享绑定/ownership/Clipboard guard未改。
- 必跑 `cargo test --offline --locked -p ooxp --lib --tests -- --test-threads=1`：190targets / 2111passed / 0failed / 0ignored（9新增）；CLI离线locked重建及10项实际PNG回放成功。保护的矩形正文SHA `e27043cc735b6bf39742e3662cd6395acfcbc884f8fb13e6da7aed8d5271a978`、tracked PNG SHA `71309897dd3d569d5e26a91d126922283a471232e035333b9150d3de3380c06d`不变；page_paint.rs/t131/main/Pages未动。
- 桌面统一入口仍 `NI-待审-INDEX-20261121.html`，本代20471bytes/SHA `bcf4df7d7d83db70163365d80a9ef74d021b8c0fb016cad3d265f3a9ba6410b4`。F06新页在 `NI-待审-历史归档-20261121-161243/generations/20261121-F06-stop-table-g2/F06-gradient-stops-foundation-待审.html`，630880bytes/SHA `44bc5a92c619ab574796e18f5ba7f05f68894ab775399edbf3fcd21444ae561c`，默认未审/无通过按钮；原两停点页保留为互补。
- 只添加独立g2，不再收纳/移动：原56页字节/位置及原manifest/10页rollback范围均不变，原两localStorage URL不动；现51NI证据/41ID另6PR12。g2 manifest SHA `9c2a8c93a548895e4667c78ecbe96e9b44a864f1863d756c5794d0bc3fc01f34`，保存INDEX前后快照和本代回退说明。55统一入口+13历史入口+10新页本地链接复核，54新增证据文件哈希复核。
- 下一可执行前置已实际定位：read_solid_ln没有stroke样式模型，stroke_run独立segment无累计弧长/相位，Canvas最近点夹紧内含圆端帽；应补共享样式/端帽/连续相位并做多线宽横纵折线/缺口受控证据，不塞prstDash周期查表。F29/F30/F32未因此解锁；既有标题/轴布局另先核实接受边界。
- 主表逐行核对49组：4已接受 / 5实现待审 / 24部分 / 12仍NI / 1安全 / 3非法，不自动验收、不合并。OOX-2原收纳已完成，下面保留历史记录，不以顺序走完宣称所有NI解决。

---

## 最新 t172u：基础实施 OOX-1 + 收纳 OOX-2 分阶段完成（2026-11-21，Asia/Shanghai）

### OOX-1 实施，仍待用户审阅，不自动接受
- Commit `6a7032e977de6c4967d1efae28626dca8b143c31` 已推送：实际复查NI计划/code，补F07独立有符号lumOff基础。复用HSL亮度相加/限幅、主题解析、straight alpha与chart/plot合成；支持±100000基线、可选末尾alpha，原lumMod后lumOff也使用同一有符号偏移。没有新布局画笔、COM补偿/颜色查表/硬编码桌面依赖。
- 新通用受控native取证器：明确worksheet/object name/title/width/height、源SHA/只读seed+copy、PID/StartTime/FullName finally Close(false)，纯manifest唯一ID/JSON数组/几何预检；共享binding/ownership与Clipboard guard不变。
- 六项同标题/两系列数据/288×180pt的派生OOXML，在Excel16/DPI106/Zoom100各按名只读Chart.Export，六CLEAN证书。SourceKind=OOXML_DERIVED_REEXPORTED_BY_EXCEL，ExportKind=EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE，不叫未修改公开API源/CopyPicture Bitmap/打印整页gold。源SHA `a6174a83e1250925355de10579b83c7c9cb1b7f04fda540ae489fc6a9763af91`，原始F07源未改。
- 真实颜色：+40000 → RGB(122,199,233)，-20000 → (7,31,42)，+100000限幅白；theme/plot同正偏移，alpha50000 → straight RGBA(122,199,233,128)。实际库PNG六项精确RGBA语义通过，不拿位置/样式/字体/AA及native265/266高度残差卡住基础，也不冒称全图通过。
- 自有seed/copy finally均Closed=true、Workbooks0、SHA不变/独占/无锁/无repair；未Save/Quit/kill/动用户书。16:16:54观察Excel进程0仅为观察，不冒称主动关闭空应用。两次离线准备/manifest错误都在任何Office启动前，修复并测试后仅一轮真实取证。
- 最终完整190目标 / 2102通过 / 0失败 / 0忽略；受控native12项 + 原native14项通过，CLI offline locked构建通过。首轮仅派生plot-alpha测试把白底合成错当straight alpha，已仅修测试期待，失败日志保留。接受矩形测试体/跟踪PNG SHA及page_paint.rs/t131/main/Pages/F45/S01/非法项不改。
- 新页 `Desktop/pr-ni-F07-luminance-offset-foundation-待审.html`，258104字节，SHA `6d356546fcfd751cb2efb6ce6680462893c4a5f8c5ba69f54b33a7c36cdafab4`；默认未审、无通过按钮，与旧F07 theme/tint/shade互补。

### OOX-2 收纳完成，不等待用户偏好，也不删除/覆写原文
- 文档commit `28009a29e6a498bff4653484d27f4ae6b4097b7d` 已推送；实施/完整复核并推送后才开始收纳，未与phase1并行移动。
- 桌面统一入口：`NI-待审-INDEX-20261121.html`，19666字节，SHA `b000050f2a1e9cb4ccdb3d7eb39cbf02ccc986d8b7b5fdf599e10f23c40bf009`。
- 唯一归档：`Desktop/NI-待审-历史归档-20261121-161243/`，有INDEX.html / manifest.json / ROLLBACK.ps1。manifest SHA `7c7b90728bf4bc72972cf2a8dbd1d1c1eddd30f74d782d530eceb01b4fd266e0`；原→新路径、原名和SHA逐页列出。
- 50份NI原页/41ID：10份旧或互补原页实际移动，40份当前NI页留桌面；另6份PR12原路径/字节不动。全部56份原页SHA按新映射验证，66索引本地链接存在且URI无反斜线编码错误。两个localStorage当前页留原URL；移动10页无local/sessionStorage/IndexedDB或相对文件/CSS依赖，浏览器私有数据不读/不重置、标签不开关。
- 当前/互补明确：F07新lumOff + 归档scheme基线；F41 font-family-cell-metrics + 归档controlled-typography-lifecycle（均显著链接，互不冒充全集）；F42 frame-foundation + 旧效果证据；F43当前共享F41字体基础 + 自己的旧复杂字排NI证据；F44 native-B5 + 四份Clipboard/绑定/按名A4诊断历史。其余36个单页ID不动。
- ROLLBACK只恢复10移动页、先SHA/碰撞检查、无删除/覆盖；自有Temp验证恢复/幂等/拒绝碰撞3项，未在真实桌面执行回退。

49唯一条件组：4已接受 / 5实现待审 / 24部分 / 12NI / 1安全 / 3非法，未自动改绿。HEAD=origin `28009a29e6a498bff4653484d27f4ae6b4097b7d`，16:16:54机端复核干净；PR保持OPEN未合并。后续仍可直接做F06停止表/分段采样、共用描边等基础，不以旧顺序遍历或本次收纳完成为停止实施的理由。


## 最新 F44 native / JIS B5基础部分待审（2026-11-21，Asia/Shanghai）

- Commit `3ad486c9a854f28f6057bce6bbcf323d47c79f30`：现代只读独立 `Chart.Export` harness、PNG校验/新路径保护/14项测试，以及JIS B5 paperSize13基础画布。不是Clipboard guard旁路；原fresh/noBitmap失败与共享绑定保持不变。旧非只读覆盖脚本未运行。
- 先核验历史accepted-native-export-audit及实际源/证书/PNG SHA；旧t84接受仅为provenance，本轮依照用户当前NI/基础授权实施。
- 新F44源SHA `78c921b30213401f7eae5470f1a74ba673808a7bdf9c0752faccb9f27827131b` 不变。Excel16/DPI106/Zoom100，按明确名称及实际PageSetup断言；7项各两次真实native PNG，14 PNG / 7 CLEAN证书，exportKind=`EXCEL_CHARTSHEET_CHART_EXPORT_NATIVE`，ClipboardPathUsed=false。不得称为CopyPicture Bitmap gold或打印整页gold。
- B5纵向36/54/72pt为640×954、587×901、534×848；横向954×640、901×587、848×534。纸边182/257mm原生内界跨六项恒定，扩展既有物理纸边/边距/转向通式；没有按六图特判、像素补偿或COM渲染依赖。库实际源六项尺寸相同、sheet/chart输出逐字节相同。A4控制native757×1120/库757×1119旧残差不改。
- 原生输出受PageSetup影响，但只有宿主ChartArea，不含整纸/边距白边；不证明打印/PDF布局。全图仍有显著图形/样式/位置/字形/AA残差；源系列数量1或2不等，逐卡比较同一源，不跨卡推断绘图区。未知纸张/未锁纸边仍NI。
- 我方第一次harness遗漏Write-Log依赖，启动seed后失败，0 Export/PNG/CLEAN；失败证据保留，严格身份核验后只Close(false)自己的seed，Workbooks0。修复并补测试；观察Excel进程0后才运行成功矩阵，finally关闭自己的seed+copy、Workbooks0、源/副本SHA不变、独占/无锁/无修复。未Save/Quit/kill/关用户书。15:27:26观察Excel进程0，仅为观察，不宣称主动清理空应用。
- Required完整测试：190目标 / 2098通过 / 0失败 / 0忽略；native工具14项通过，CLI offline locked构建通过。接受矩形测试体/跟踪PNG SHA保持，page_paint.rs/t131/main/Pages/F45/S01/非法项未改。
- 新评审页 `Desktop/pr-ni-F44-native-B5-paper-margins-待审.html`，643201字节，SHA `bfa1a6c8c7984b632ab0fb650f33be85af80c13b244d71c3e595b4ff5fc8cd4b`；默认未审、无通过按钮，54份旧页SHA不变。全部native/library PNG与证书SHA在 `%TEMP%/t174-f44-native-matrix-v2-20261121/logs/f44-native-library-manifest.json`，manifest SHA `17c7172e4cdf381c41c49beb35b379e3f6adcd03172c3c492e47b2d1f96e70f2`。
- 49唯一ID覆盖审计通过；本轮顺序评估已完成，不是全部功能实现或接受：4已接受 / 5实现待审 / 24部分 / 12仍NI / 1安全 / 3非法 = 49。F44改为部分待审，⚠️计划文件名和3个FF保持。PR保持OPEN，不合并、不自动验收。


> **F44 t172重要勘误：旧脚本Charts.Item(1)实际取到B5l72，不是A4；此前“A4失败”结论撤回。新仪器化证据仅证明此次B5l72调用后序列新鲜但没有Bitmap。已修复后续按名取图与身份断言，t173现已完成正确A4实机对照，同样fresh但无Bitmap；另已核实既有独立native Chart.Export通道，见最新checkpoint。**




















































## 实现待审

























- F04 noFill `f58c8b71`：Desktop/pr-ni-F04-noFill-待审.html

























- F05 sRGB alpha `62dbbb98`：Desktop/pr-ni-F05-alpha-待审.html

- F35 pie有限负值按 |v| 占扇区（至少有一个有限正值） `5c3b20e449f9531aa79920659c537978f72c39b7`：Desktop/pr-ni-F35-pie-finite-negatives-待审.html。官方 Excel 16 cache 保留符号，Chart.Export 与同组类目 |v| 对照一致，不是 skip-zero 或省略类目。官方源因 showLeaderLines=1 库仍 NI，本轮不重开。全负/全零/全空见 F36。

- F36 pie全负按 |v|、全零/全空保留图框与图例无扇区 `ec571777f532e47a8b457a040a434e3f7fcd8af4` + `7312e4dd960a6c744ce12531d427c1a1c32b9e89`：Desktop/pr-ni-F36-pie-all-zero-empty-neg-待审.html。官方全空为类目 cache 有 pt、val cache 有 ptCount 但无 val pt，绘图区与全零相同。官方源因 showLeaderLines=1 库仍 NI，本轮不重开。剩余 NI：仅有 val ptCount、无类目 pt 时不按不可信 ptCount 分配定义域。

- F37 pie系列或dPt explosion非0 `2561ee9c7bf20573a90ea786c933d71663ba29c6`：Desktop/pr-ni-F37-pie-explosion-待审.html。官方系列 explosion 把整图绕同一圆心缩小为 1/(1+e/100)，并沿每扇区角平分线外移 (e/100)*R'，外接半径仍为 R；单个 dPt explosion 不改其余扇区与 R，只把该扇区沿角平分线外移 (e/100)*R。官方源因 showLeaderLines=1 库仍 NI，本轮不重开。剩余 NI：系列与 dPt explosion 同时出现；dPt bubble3D 非 0。



















































## 部分实现待审（不代表整组完成）

























- F38 pie半透明/非纯色扇区/复杂point override（子集） `b5ff52514eb65ecd88e9ef0bfa5fb86d18ae450a`：Desktop/pr-ni-F38-pie-fill-alpha-nofill-待审.html。已锁定：系列或 dPt 不透明 sRGB solid（即使 varyColors=1，dPt 优先）；sRGB 直通 alpha 叠在已画底色上（与 F05 相同）；noFill 不画扇区/图例色块，官方 hiddenFill 只作记忆。官方源因 showLeaderLines=1 库仍 NI，本轮不重开。剩余 NI：系列或 dPt 的 gradFill（rotWithShape/scaled 楔形作用域不能复用 F06 矩形规则）、pattFill、blipFill。

- F06 两停点不透明sRGB线性0°/90° `082364a3`：Desktop/pr-ni-F06-linear-grad-待审.html。patt/blip/path/斜向/多停/变换渐变仍NI。

























- F07 schemeClr主题槽、IEC线性sRGB tint/shade、HSL lumMod(+lumOff) `85ca96d4`：Desktop/pr-ni-F07-scheme-transforms-待审.html。sysClr、sat/hue/alphaMod、单独lumOff及混用仍NI。

























- F08 直角图表ln/noFill与hiddenLine编辑记忆 `4048bd55`：Desktop/pr-ni-F08-no-line-待审.html。可见线/绘图区线/圆角无边线/effectLst仍NI。

























- F09 饼图显式不透明solidFill `eed8fcaa`：Desktop/pr-ni-F09-pie-plot-solid-待审.html。半透明/渐变仍NI；约1–3px边界残差明确待审。

























- F10 普通水平簇状条完整General整数（负数/0）默认样式/outEnd `cdd6424c`：Desktop/pr-ni-F10-bar-outEnd-待审.html。缺值/错误/小数/其他位置/样式/逐点可见覆盖及其他类型仍NI。

























- F11 已有普通柱/折线/条标签的系列级idx+delete=true逐点删除 `8f061aff`：Desktop/pr-ni-F11-point-delete-待审.html。delete=false、可见逐点文本/内容/位置/样式、未知扩展、wrapper覆盖及其他类型仍NI。

























- F12 普通簇状柱完整非零General整数默认样式/inEnd且标签盒可完整落入柱内 `38ad4ad013fbdf39aa6c3c3763d47b1d919822dd`：Desktop/pr-ni-F12-column-inEnd-待审.html。0 的外侧回退、短/窄柱自动适配、缺值/错误/小数/其他位置/样式/条形与折线inEnd/逐点可见覆盖仍NI；原 outEnd 接受子集保留。公开探针盒距3pt+文本内距3pt。

























- F13 普通面积显式 dispBlanksAs=zero（真实空点/省略 cache 点，与省略项相同的零基线 V 形） `88f1bc030f1bd7f48ef2f4419fe7973c742974b6`：Desktop/pr-ni-F13-area-zero-待审.html。Excel16 公开保存把面积缺省写成 val=zero。面积错误点（#N/A 与 #DIV/0! 在 zero 下仍 NI）、柱/条/散点/饼 zero、dispNaAsBlank、未解决稀疏域仍NI（面积 span 见 F14）。负柱 gap 与 zero 官方仅标题字形差78px、轴范围相同，本轮不解锁柱形 zero；既有柱形解析若接受该标志仍按缺值不画柱，不宣称实现。

























- F14 普通面积显式 dispBlanksAs=span（真实空点/省略 cache 点，以及 #N/A，与省略项/显式 zero 相同的零基线 V 形，不是 4-到-9 弦） `ea9512967af95b7f178dd2d6c96369a3c68877af`：Desktop/pr-ni-F14-area-span-待审.html。#DIV/0! 及其他非 #N/A 错误、柱/条/散点/饼 span、dispNaAsBlank、未解决稀疏域仍NI。柱形 span 官方中间无柱、与 gap 不能唯一区分，本轮不解锁；既有解析若接受该标志仍按缺值不画柱，不宣称实现。折线 span 已接受子集保留。

























- F15 普通簇状柱与标准折线显式 logBase（[2,1000]，显式 min/max 为 ≥1 的底数整数幂含 1=base^0，省略 major/minor，值落在 [min,max]） `bc2c574e2ea125175efff44452e172f69cb32639`：Desktop/pr-ni-F15-logBase-待审.html。自动省略 min/max、低于 min 的截断（F15-6 官方细条）、非幂次端点、自定义 major/minor、面积/条/散点/饼对数、零/负值、可见值标签+对数仍NI。官方 F15-2/F15-4 Chart.Export 424×266，库 424×265；重叠 differ 10822/11349（1px 高度、绘图区顶、灰/黑网格、字体），待审不宣称整图像素一致。

























- F16 普通簇状柱与标准折线 catAx crossesAt（线性显式 min/max，有限值夹紧到 [min,max]；官方 Excel 16 写在 catAx，valAx 仍 autoZero；柱从交叉而非 0 生长） `0cc55316a7d620c4d8b42177c8d2070ed7b41d95`：Desktop/pr-ni-F16-crossesAt-待审.html。valAx crossesAt、log+crossesAt、条/散点/饼 crossesAt、官方面积 midCat 仍NI；catAx crosses=min/max 见 F17。官方 Chart.Export 424/425×266，库 424×265；残差为已知 1px 高度、绘图区顶 y=46 对官方 62、灰/黑网格、轴/字体，待审不宣称整图像素一致。

























- F17 普通簇状柱与标准折线 catAx crosses=min/max（线性显式 min/max；官方 Excel 16 写在 catAx，valAx 仍 autoZero；柱从端点生长。crosses=min 保留底边标签槽，crosses=max 把标签槽放到顶边，与夹紧到 max 的 crossesAt 不同） `4e82565c39a58d691c92f2225d132b885ebb1575`：Desktop/pr-ni-F17-crosses-min-max-待审.html。valAx crosses=min/max、log+min/max、条/散点/饼、官方面积 midCat 仍NI。官方 Chart.Export 424/425×266，库 424×265；max 的绘图区顶官方 82 / 库 67（autoZero 为官方 62 / 库 46），待审不宣称整图像素一致。

























- F18 普通簇状柱与标准折线 valAx 非 General formatCode（线性显式 min/max；官方 Excel 16 写在 valAx/numFmt。锁定字面量 General、0、0.00、0%、0.00%、#,##0、0.00E+00，非 General 时 sourceLinked=0；刻度文本按官方 Chart.Export，较宽标签加宽左槽） `0b6c27cd18f2d78475ba5e697846fc0e6efcdde5`：Desktop/pr-ni-F18-formatCode-待审.html。其它 formatCode、sourceLinked=1 的非 General、对数轴+非 General、数据标签/缓存 numFmt 仍NI。官方 Chart.Export 424/425×266，库 424×265；残差为已知 1px 高度、绘图区顶库 ≈47 对官方 63、灰/黑网格、轴/字体及左槽约 0–4px，待审不宣称整图像素一致。

























- F19 普通簇状柱、标准折线与簇状条 valAx orientation=maxMin（线性 autoZero；官方 Excel 16 把 ReversePlotOrder 写在 valAx/scaling/orientation=maxMin，catAx 仍 minMax autoZero。竖向 min 在上、max 在下；条形 max 在左、min 在右。类目标签槽由 orientation 与交叉几何推导，不信任 axPos） `62fbd58f359fa91975ef1c41468ed6c14bbc7492`：Desktop/pr-ni-F19-orientation-maxMin-待审.html。对数+maxMin、自定义 crosses/crossesAt 或 catAx crosses=min/max 与 maxMin、散点 maxMin、可见值标签+maxMin 仍NI。官方 Chart.Export 424/425×266，库 424×265；残差为已知 1px 高度、绘图区顶库 ≈47 对官方 63、灰/黑网格、轴/字体，待审不宣称整图像素一致。

























- F20 普通簇状柱、标准折线与簇状条 valAx delete=true（线性 autoZero minMax；官方 Excel 16 的 HasAxis(xlValue, xlPrimary)=false 与 Axes(xlValue).Delete() 都写 valAx/delete=1，tickLblPos 仍 nextTo，majorGridlines 仍在，catAx 仍 delete=0。省略值轴刻度标签、刻度线和值轴线，仍画网格并用同一尺度；释放值标签槽；保留类目轴与类目标签。条形释放底边值标签槽并保留左侧类目轴） `7b7e0ff8b646473636cb17114a59343869401c6e`：Desktop/pr-ni-F20-valAx-delete-待审.html。tickLblPos=none（官方仍保留值轴线）、类目轴 delete、delete+log、delete+maxMin / 自定义 crosses、散点/饼/面积 valAx delete 仍NI。官方 Chart.Export 424/425×266，库 424×265；残差为已知 1px 高度、绘图区顶库 ≈47 对官方 63、灰/黑网格、轴/字体，待审不宣称整图像素一致。

























- F21 恰好两个同类型 wrapper（普通簇状柱或标准折线）各一个系列的次轴（线性显式 min/max autoZero minMax；官方 Excel 16 的 SeriesCollection(2).AxisGroup=xlSecondary 写两个同类型 wrapper。主 valAx axPos=l crosses=autoZero 保留网格；次 valAx axPos=r，crosses=max 仅表示右置，不是 F17；次 catAx delete=1 为隐藏配对轴，不是 F20 可见类目轴 delete。同类型柱重叠，先主后次，镜像右标签槽） `2827da15658cb0acbe9974d583443b0cc2a3732e`：Desktop/pr-ni-F21-secondary-valAx-待审.html。簇状条（次 valAx axPos=t）、面积 midCat、组合不同 wrapper（F26）、省略 min/max、log/maxMin/delete+次轴、值标签+次轴、每 wrapper 多于一个系列、次轴 majorGridlines、可见次 catAx、以及本锁定对之外的 valAx crosses=max 仍NI。官方 Chart.Export 424/425×266，库 424×265；残差为已知 1px 高度、绘图区顶库 ≈47 对官方 63、灰/黑网格、轴/字体，待审不宣称整图像素一致。

























- F22 barChart grouping=stacked/percentStacked（柱形与条形；官方 Excel 16 写 overlap=100、gapWidth=150、invertIfNegative=0；线性显式 min/max autoZero minMax。正值按系列从零向上/向右累加，负值从零反向独立累加；省略 cache 点不画该段并继续累加。percentStacked 仅全正、显式 0–1，以及官方 valAx/numFmt formatCode=0% sourceLinked=1） `e2238ca888cc63fb179844130163598b89848689`：Desktop/pr-ni-F22-stacked-percentStacked-待审.html。省略 min/max、overlap 非 100、invertIfNegative=1、可见值标签、对数/maxMin/delete/自定义 crosses、次轴、percentStacked 混合符号或缺值仍NI；line/area 堆积见 F23。官方 Chart.Export 424/425×266，库 424×265；残差为已知 1px 高度、绘图区顶库 ≈47 对官方 63、灰/黑网格、轴/字体，待审不宣称整图像素一致。
































































- F23 lineChart/areaChart grouping=stacked/percentStacked（线性显式 min/max autoZero minMax。官方 Excel 16 堆积折线写 marker symbol=none、smooth=0、crossBetween=between；堆积面积写 crossBetween=midCat。折线按系列顺序连代数累加端点，正负混合也是代数求和，不是 F22 柱形的正负从零反向独立堆积；面积填充相邻累加线之间的带，后画覆盖先画。percentStacked 仅全正、显式 0–1，以及官方 valAx/numFmt formatCode=0% sourceLinked=1） `8effa81e634bed02d8b2a3031a3af7a18feee97a`：Desktop/pr-ni-F23-line-area-stacked-待审.html。省略 min/max、可见标记、空点、between 类目位的堆积面积、percentStacked 混合符号或缺值、可见值标签、对数/maxMin/delete/自定义 crosses、次轴仍NI。官方 Chart.Export 424/425×266，库 424×265；残差为已知 1px 高度、绘图区顶库 ≈47 对官方 63、灰/黑网格、轴/字体，待审不宣称整图像素一致。

























- F27 官方散点 valAx midCat，以及已锁定的 marker/lineMarker（含官方 lineMarker+symbol=none 的折线无标记、lineMarker+a:ln/noFill 的仅标记）在 midCat 下出图 `bcd3a7f06080d51e599eb419599cacce845e11f8`：Desktop/pr-ni-F27-scatter-styles-待审.html。公开 ChartType 不写 scatterStyle=none/line/smooth；smoothMarker 与系列 smooth=1 的平滑曲线仍NI。官方 Chart.Export 424/425×266，库 424×265；残差为已知 1px 高度、绘图区顶、灰/黑网格、轴/字体，待审不宣称整图像素一致。官方源 SHA `c32b9bc3f2ad7ba4d3365cb3054fff3e33e33cba78df890d7fbe0c24cc8ba7a4`。docs `19dbe04496677fc43cd980a5197cfa17153a517d`。

















F23 全量 2083 passed /0 failed /0 ignored（190 suites，lib367）。公开Excel16窗口DPI106，只读副本Close(false)；源与官方 PNG SHA 逐份核对。无+1px/COM/4pt补丁，无拉伸；旧桌面网页SHA不变。所有新项默认未审，不自动改绿。

























## 仍NI（本轮已取证，未实现）























- F24 三维 bar3D/line3D/area3D/pie3D/surface3D：官方 Excel 16 默认 view3D 为笛卡尔 rotX=15 rotY=20 rAngAx=0（透视），pie3D 为 rotX=15 rotY=0 rAngAx=0；均有 floor/sideWall/backWall thickness=0；line3D/area3D/surface3D 有 serAx；pie3D Chart.Export 438×266。仅一组默认视角，不能把透视、深度、墙面、系列轴间距与光照锁成一般规则。无实现提交，未删 wrapper 白名单，未发明三维投影。证据页 Desktop/pr-ni-F24-3d-charts-仍NI.html（155705 字节）。官方源 SHA `6197dde6ac24d9b0b21da794cd05dbd2b691693c1895f0b19b4bd3fb73a7eae3`。docs `f4f921cd70ce9557fa37b9fe3abedc50166ef1b8`。























- F25 doughnut/ofPie/radar/bubble/stock/surface：官方 Excel 16 写出 doughnutChart holeSize=50 firstSliceAng=0；ofPieChart pie gapWidth=150 secondPieSize=75，省略 splitType；xlRadar 为 radarStyle=marker 但 marker symbol=none；bubbleChart bubbleScale=100，省略 sizeRepresents；stockChart HLC 有 hiLowLines，收盘 dot size=3，dateAx 自动刻度；xlSurfaceTopView 写 surfaceChart，view3D rotX=90 perspective=0，有 serAx 与 bandFmts。仅各组默认，不能把孔径、自动拆分、蛛网、气泡尺寸、股票日期轴或俯视曲面锁成一般规则。无实现提交，未删 wrapper 白名单，未发明投影。证据页 Desktop/pr-ni-F25-other-chart-types-仍NI.html（114959 字节）。官方源 SHA `17faa611c7c531e28cc3af5e6942d4dbf1e7a03ba0685b135ca01b78cd0ab6de`。docs `2aeb6aab4782436996d05ccb86c928a03058ca4e`。





















- F26 组合图（plotArea 多个 wrapper）：官方 Excel 16 柱+折线可共享同一对轴（between，显式 0–80）；两柱+折线时柱簇只按 barChart 系列数；柱+折线次轴形态同 F21 但 wrapper 不同类型；柱+面积共享 valAx 为 between（不是单图面积 midCat）；条+折线会自动把折线放到次轴且次轴省略 min/max。不能按单图规则叠画。无实现提交，未删 combo guard。证据页 Desktop/pr-ni-F26-combo-wrappers-仍NI.html（119588 字节）。官方源 SHA `23bfc2db084aa527747112f1fae74fc38f75773f93b9637889c08e1b14b779f9`。docs `d7653cb4cfafd40bdcc7bd59b7fb1ec643509e77`。



















- F28 折线/散点系列 smooth=true：官方 Excel 16 的 Series.Smooth=true 写系列 c:smooth val=1，lineChart 包装级 smooth 仍为 0。仅一组默认 4 点，不能锁定一般样条。无实现提交，未发明样条。证据页 Desktop/pr-ni-F28-line-scatter-smooth-仍NI.html（103403 字节）。官方源 SHA `0eb3ae935e517c6370a9c35b2a64c2f30ccd13097854c86b0d64b00e9633e54a`。docs `f7892d5d60713cc5e10d748453ff6565c068f0db`。















- F29 系列线非 solid prstDash/customDash：官方 Excel 16 的 Format.Line.DashStyle 写 prstDash=solid/dash/dot/dashDot/lgDash，省略 @w；方点与圆点都是 prstDash=dot，只靠 cap=sq / cap=rnd 区分；公开 API 未写出 customDash。仅默认线宽，不能锁定周期/相位。无实现提交，未发明虚线周期表。证据页 Desktop/pr-ni-F29-prstDash-customDash-仍NI.html（87250 字节）。官方源 SHA `4beb05944d53aed6975dd5c0fb07e1d1be3da31893878510b03096e99ca1d9ff`。docs `7b4e3a95b1c9856248ddcea753d688bb1456a92b`。













- F30 系列线非 rnd cap / 非 sng cmpd / 非 ctr algn 及箭头：官方 Excel 16 写出 algn=in、cmpd=dbl/thinThick/thickThin/tri，以及 headEnd/tailEnd（triangle、arrow、stealth、oval、diamond，w/len 为 sm/med/lg 或省略）。cmpd=sng 与默认实线对照库已能画。这批实线 API 未写 cap；非 rnd cap 见 F29 的 cap=sq/rnd。不能锁定内缩、复合条纹比或箭头几何。无实现提交，未发明像素表。证据页 Desktop/pr-ni-F30-line-cap-cmpd-algn-arrows-仍NI.html（156351 字节）。官方源 SHA `d7d3a3f385ad69536dcbd1d11b4166fd07304d443db6f5b1e70375680e670ab3`。docs `0a736b617264a317743445dba152135c3167095b`。











- F31 line/scatter dropLines/hiLowLines/upDownBars 与 area dropLines：官方 Excel 16 折线/面积写出不带 spPr 的空 <c:dropLines/>、<c:hiLowLines/> 以及 gapWidth=150 的空 upBars/downBars，官方图可见默认装饰。散点 COM 读到 HasDropLines/HasHiLoLines/HasUpDownBars=True，但保存包未写出这些元素，库按已锁定 marker 路径出图。不能锁定默认几何与样式。无实现提交，未发明几何或默认描边表。证据页 Desktop/pr-ni-F31-drop-hilow-updown-仍NI.html（278398 字节）。官方源 SHA `e5c142929cda1500cfb2133594b72fe18bc4f1afe777bc893e105ee5822ac9b5`。docs `79e4c9481d8bb44538a29b6f915281fe3cf3044d`。









- F32 line/scatter/area 系列 dPt、trendline、errBars：官方 Excel 16 折线/散点 dPt idx=3 为 marker diamond size=10（无填充，bubble3D=0）；面积 dPt idx=3 为 solidFill srgbClr=FF0000；三种图的趋势线均为 linear 且 dispRSqr=0、dispEq=0、无 spPr；误差线均为 y/both/fixedVal/10、noEndCap=0、无 spPr。不能锁定默认拟合、端帽样式或未写明的标记颜色/面积点区域。无实现提交，未发明几何。证据页 Desktop/pr-ni-F32-dpt-trendline-errbars-仍NI.html（233203 字节）。官方源 SHA `7fc9fb2cc1dc6b545bd8e8bbcea0c37a40a65b689e01628147d59ef297540b63`。docs `9db9cddc92d7fb02efae0c9e6cbff67e0cead378`。







- F33 area 可见 marker、透明/无填充/非纯色填充：官方 Excel 16 标准面积 valAx 均为 midCat，库先在该门拒绝（同 F31-9/F32-9），本轮未改 midCat。COM 设置面积 MarkerStyle=diamond size=10，但保存包未写 c:marker；noFill 带 a14:hiddenFill srgb=156082；半透明为 srgb=156082 alpha=50000；图案为 pattFill prst=pct10 fg=156082 bg=FFFFFF。不能锁定 alpha 合成、图案平铺或未写入的面积标记。无实现提交，未发明几何。证据页 Desktop/pr-ni-F33-area-marker-fill-仍NI.html（63112 字节）。官方源 SHA `ffe8109fc31b654765fd80796b5cb67e9850446f0dbf5d50a1ab2e45144d0ece`。docs `cf966ebf26665570aa480f40477176ba034ecf8b`。





- F34 pie 多个系列：官方 Excel 16 公开 API 会在同一个 pieChart 写出 2 或 3 个 c:ser。官方图与单系列对照不同（differ 约 1673–1686），同样 S1+S2 的两条路径在同尺寸上仍 differ 1683，不能锁定只画第一或最后一列，或一般多系列布局。官方单系列对照因 dLbls showLeaderLines=1 被拒，本轮不重开。无实现提交，未发明布局。证据页 Desktop/pr-ni-F34-pie-multi-series-仍NI.html（77969 字节）。官方源 SHA `67a54afd1ac8436187991483ba9bcb1308623100dfbd5c51c3fa7c0c3d069a76`。docs `6fd35c4ff8f37454d49c8220a2b57cefca62d335`。

- F39 pie非default_right_ink的图例配置：官方 Excel 16 五个自动位置（legendPos r/l/t/b/tr，overlay=0，无 manualLayout）已取证。左右扇区同为 186×186、左移带约 43px；上下直径缩小约 35px 且与右侧共用一条外边；右上只缩小约 9px 且仍共用底边。色块 8×8、间距约 27 与直角 Layout.position 定性相符，但扇区预留不能唯一复用已锁定的 pie_plot_bounds 或 Layout.reserve，故不发明图例布局画笔，整组仍 NI。官方源因 showLeaderLines=1 另为 NI，本轮不重开。无实现提交。证据页 Desktop/pr-ni-F39-pie-legend-pos-仍NI.html（138355 字节，SHA 06a93c188fe95a8ab2ec45f637d8384ec5d2e9b877b66d69e055f8601cdd18d5）。官方源 SHA `9b0f72afaf6f03d0ac4e576f634b65585f4d0c5de0ee6c94330279653f419df8`。docs `0bd4a3bd2e4f02c9adf3d502b1b6c0fd7296b7f0`。

- F40 legend manual layoutTarget=inner 或 w/hMode=edge：官方 Excel 16 公开 API 用 Legend.Left/Top/Width/Height 只写出 xMode/yMode=edge 与默认 factor 宽高，不写这两类属性。在官方源副本注入后由 Excel 只读导出：inner 不改色块位置但显著扩大绘图区（约 7.5 万像素差），不能当 outer 空操作；wMode=edge 的 A/B 水平位移与“w 为右缘并居中”相符，但 C 同时垂直位移；hMode=edge 在 C 与“h 为下缘”相符，但 A 在小于一行 pitch 的高度仍绘制、B 高度为负且 8×8 色块消失，退化行为不能唯一锁定。不发明图例布局画笔，整组仍 NI。无实现提交。证据页 Desktop/pr-ni-F40-legend-manual-inner-edge-仍NI.html（187157 字节，SHA 01a910f68132701fdebaee529bd1f19d112f1bec3475a1e67587c8b98571272b）。官方源 SHA `bb810a0a4fa6227c9caf0666d86807f3f93fe9ce0a5af950464f5e4c75fe20fa`。docs `4df702611474f1a20223391edbf569ae2a164616`。

- F41 legendEntry单独txPr（非delete）：官方 Excel 16 公开 API 会写出 legendEntry+txPr。仅改颜色时目标项文字变为不透明 sRGB FF0000，两项 8×8 色块与对照逐像素相同，同画布 entry1 着色还与对照字形同位完全对应，但未改项和对掉标题后的绘图区并不能唯一证明保持不变。仅改 18pt 时两项色块都放大并左移、绘图区收缩；仅加粗时该项色块变为 8×9，去标题后绘图区仍有网格线差。混合项与图例级 Arial 还写出 cs。不发明图例排印/布局画笔，整组仍 NI。无实现提交。证据页 Desktop/pr-ni-F41-legend-entry-txpr-仍NI.html（102826 字节，SHA c220de115ad57f2afb894a9b74e33a29ef8f4aa593edf5af210a0c8697eee440）。官方源 SHA `fbc0012a5ddab5e7bcfe96dbf3c5472d4336981c082caa646b9c04e64402160d`。docs `f01bbe8dadf2ed9bf614fee6789729277611a078`。



当前49组：已接受4、实现待审5、部分22、仍NI14、安全1、非法3。S01与I01–I03保留；F44官方只读导出受阻，保留源后续恢复取证；F45已接受不重开。

























计划 `.plan/2026年-11月-20日-⚠️-xlsx-图表出图未实现条件追踪.md`


















- F42 仍NI（官方 Excel16 DPI106）：已取证 solid/alpha/noFill/2pt线/阴影/渐变；颜色运算可复用，但图例填充框/线/效果边界未唯一锁定，未猜测画笔。默认/noFill已有支持，不算新实现。源 SHA `3c0b834bfedbbc1db872f30a5bf6e464de5eb251e19251e3b35ff409571e9a30`。证据页 Desktop/pr-ni-F42-legend-frame-仍NI.html（101904 字节，SHA b8a1ac7e913d46c2f2a0d553eae2957869934102875342e9933844c375e1a301），默认未审、无通过按钮。docs `877c9b985f7ec40c77b4a3ef2e5c1e8809f110b7`。无代码变更，未重跑套件，计数不变。


- F43 仍NI：官方Excel16 DPI106证实单/双下划线、删除线、下标改变墨迹；上标设置未写txPr，不算上标证据。Arial写latin/ea/cs，未证明复杂脚本等价；旋转/竖排公开API探针失败，失败源/日志保留且不作金样，多段未锁定。不发明排印画笔。源 SHA `29b6553287afb5a9a60d5911f04f3ac187de85c07a3ed5507ff9a0808286afeb`。Desktop/pr-ni-F43-legend-text-仍NI.html（96837字节，SHA d72d4bb2514271b23e6055df364f259b87ec5d993e97bb0a7bfe19b6110ec4a5），默认未审、无通过按钮。docs `e68363a853d4a503d158576203aaa1a6160f9326`。无代码变更，计数不变。


- F44 仍NI，正式官方导出取证阻塞：已用公开Excel16 DPI106创建paperSize=13 B5纵横向/36、54、72pt边距矩阵及A4对照源。源 SHA `78c921b30213401f7eae5470f1a74ba673808a7bdf9c0752faccb9f27827131b`。正式只读导出两次在Ensure-MsExcel/FromWindow报hr=0x80004005，未进入CopyPicture/Chart.Export，CLEAN证书0份，PNG0份。源和只读副本SHA不变；不将创建阶段COM度量冒称金样，不猜新inset。缺口页 Desktop/pr-ni-F44-chartsheet-paper-仍NI-取证阻塞.html（5046字节，SHA 996458cd7d9466c5f1429f0a908772065657454fe6e95059c7d832cd331cb1b1），默认未审、无通过按钮。docs `d86d5e97fa78d3073eed89fe947f8fd213eecf09`。无代码变更，未重跑套件。下一步仍F44恢复取证，F45已接受不重开，S01/I01–I03跳过。

- 计数审计：49个唯一ID，F01–F45无缺漏，4接受/5实现待审/20部分/16仍NI/1安全/3非法，总49，与计划首段一致。F42/F43仍NI取证记录完成；F44官方导出未完成，不声称顺序全量取证完成或全部NI已解决。


## t171u 基础优先（当前有效进展，替代此前F41/F43整组仍NI判断）

- 实现 `3d38085af287bfee45a5fcce527485797b0b1c09`：Cartesian逐项legendEntry txPr继承/覆盖颜色、字号、bold/italic、family；每项face/metrics参与flow/reserve，共有key/row pitch用继承图例与可见项最大metrics。latin/ea/cs同名允许，异名cs/旋转/多段/装饰/pie非default仍NI。F41/F43记部分实现待审，不声称整图零差。
- 同标题/同extent/同数据派生库回归fixture明确非官方金样；原F41官方源及7份CLEAN证据不变。新页 Desktop/pr-ni-F41-legend-typography-foundation-待审.html（560969字节，SHA 069b6344445687b80dee53917bead0f28bbba40a9728ea34591fd3ffeac5467d），默认未审，旧47页不变。
- 完整指定套件2089 passed/0 failed/0 ignored；接受的矩形测试体和PNG SHA不变。生命周期 helper 新增显式Temp路径/进程/FullName归属与finally Close(false)核验，负向安全测试通过；共享binding guard未改。
- F44旧绑定失败经用户清理后同入口成功恢复，但遇到新的chartsheet CopyPicture非Bitmap剪贴板失败；不是CLEAN。旧导出finally漏关seed已定位；仅将己方已核实的f44-seed Close(false)，Workbooks.Count=0，仍有己方空Excel窗口PID44044，未Quit/kill，不声称应用窗口已清理。按更正停止F44新会话，优先基础能力；新生命周期集成导出脚本保留未执行。
- 当前计数4接受/5实现待审/22部分/14NI/1安全/3非法=49；文档 `efe0c103fb442caecc71280c7122840133c72672`，历史字节保留修正 `89ce09f0ad117c5aba75cce00617ae51ed9bbdb8`。后续F42背景/边框基础。


## t171u 第二基础：F42图例底色/边框（当前有效范围）

- 实现 `1f6d3cb5a2d617467faf54cf93d4f773d097fb0b`，文档 `8c0f456882c9316a499d3ce8e7194e364cbc0d1a`。现有Layout.frame上绘制sRGB solid/alpha/noFill矩形底色、显式EMU宽度转pt的简单不透明sRGB实线；复用Canvas/DPI/alpha转换，不加COM常数或位置补偿。支持语义与位置/内边距/角部AA残差分开，部分待审，未宣称全局像素一致。
- 五位置真实像素测试；noFill与默认输出逐字节相同。完整指定离线锁定套件190 targets / 2091 passed / 0 failed / 0 ignored；定向12测试通过。接受矩形测试体与PNG SHA不变。
- 原F42官方源SHA `3c0b834bfedbbc1db872f30a5bf6e464de5eb251e19251e3b35ff409571e9a30`、7份Excel16 DPI106 CLEAN证书复核；同标题F42/数据/extent，仅重导库图，未新开Office。ctrl/solid/alpha/nofill/line成功；shadow/gradient仍NI。复杂线型、主题底色/渐变/可见效果仍NI。
- 新页 Desktop/pr-ni-F42-legend-frame-foundation-待审.html：261248字节，SHA `6b3849265bfc12719772d7bfaea85a44c0032b753270e1a717558dd23995c02c`；5支持子集默认未审，2仍NI无通过按钮，所有旧48页SHA不变（共49页）。
- 生命周期：第二基础未开Office；末次只读快照Excel进程0，不归因为我主动清理。新owned helper正向真实Office导出仍待验证。F44绑定已恢复但chartsheet CopyPicture剪贴板失败，无CLEAN；按更正继续暂停F44新样本。没有Quit/kill或关闭用户book，没有更改共享binding guard。
- 最新审计49唯一ID、F01–F45无缺漏：4已接受/5实现待审/23部分/13仍NI/1安全/3非法=49。F41/F42/F43只推进基础子集；F44正式证据未完成，不声称全量顺序取证已完成或全部NI已解决。F45/S01/I01–I03不动。后续先受控同标题F41/owned生命周期正向验证，再依赖项与F44。


## t171u 受控排印修正 + 真实success/exception生命周期（本轮已实做）

- 实现 `ce0c167ad344637dd19b2f8ceae82ccc4cb688c2`，文档 `d45351416cbb389af3dfa5bc3d6c41f1dcc6fc77`。同title/anchor/data/axes/chart-settings包（除legend外chart XML相同）SHA `4729dcb02a6dadc526654820751f9eb1bf233f801c1d9446418aed780e99c513`。包明确是旧公开API源派生；其真实Excel16 DPI106只读导出有7份CLEAN，不冒称Excel原创源。
- 官方7张Chart.Export全部425×266、CopyPicture全部385×241，库全部424×265；只作各自renderer内同画布差分，不拉伸/补1px冒充跨renderer同画布。
- 实证修复：官方entry0/1仅改红只动85/90个目标文字像素、key/plot不变；旧库错误改动2193/2195像素。纯entry颜色且继承默认metrics现在复用既有默认右图例geometry、按原series索引取文字色，新库只动73/82文字像素。pie strict gate未扩大。混合字体/字号竖排不再逐行居中，key与text共享按真实metrics测得的列。绝对key size/pitch、位置/AA、title/轴等残差独立待审，不把整图零差当基础支持的前置门。
- 真实生命周期（2026-11-21 14:02:52–14:03:00 Asia/Shanghai）：仅一次原Ensure-MsExcel、单一PID66912。success导出7图后finally Close(false)自己的seed+copy，2本Closed=true/剩余0；同一持有app内另两本自己的seed+copy真实导出后故意抛异常，finally仍2本Closed=true/剩余0。四文件SHA不变、独占读、无ownerlock/repair。异常证书明确NOT_GOLD；无Save/Quit/kill、无用户book关闭。
- 新可复用脚本 manual-check/verify-legend-foundation-lifecycle.ps1，EvidenceRoot参数化，Temp归属启动前预检（非Temp拒绝已实测），共享binding guard未改。结束时自己的空Excel窗口仍存在，未声称应用清理成功；14:17:35 Asia/Shanghai只读进程快照为0，此消失仅观察，不归因为我的清理，不需要用户为本轮关闭窗口。
- 依赖F42当前CLI复跑ctrl/solid/alpha/nofill/line五图与既有待审输出逐字节相同；shadow/gradient仍NI。未再打开Office，F44仍未新导出，不冒称有CLEAN。
- 新页 Desktop/pr-ni-F41-controlled-typography-lifecycle-待审.html：467514字节，SHA `a87248c18139ba147115cbc339f4a26ea1e53cc184f7460e00084fa479227548`；7卡默认未审，复杂NI无通过选项。旧49页全部SHA不变，现50页。
- 最终完整指定离线锁定套件190 targets / 2093 passed / 0 failed / 0 ignored；legend定向14 passed。接受矩形测试体及PNG指定SHA均不变；未改page_paint.rs、无t131证据。普通提交/推送，不合并、不自动接受。
- 审计49唯一ID无缺漏，计数保持4接受/5待审/23部分/13NI/1安全/3非法。F45/S01/I01–I03不动；剩余字体family/key/pitch受控验证与复杂NI仍在，F44正式证据未完，不声称全部NI解决。


## 2026-11-21 字体design-cell基础与F44单次参数实验（未审，未接受）

- `e97b886a67908453b9be084c3502faae17dbaf26`：显式legend根字体读取GDI+ design ascent+descent/Em计算cell，key=0.45cell、pitch=cell+6pt；不按字体名查常量、不加COM拟合补偿。未解析theme/implicit family保留旧默认模型，legacy/right、pie和F42不因fallback而回归。
- `3db7c69e4e1379d764314297c3806d4f7fc8be01`：同标题/画布/data七字体18pt矩阵及10/24/30pt矩阵，共14份Excel16/DPI106 CLEAN。源为派生controlled包，不冒称Excel原生生成；SHA `571ae1ac040eac5fde957836761b1f60d386241cb769c7aa80eb621a8547e94e`、`96d138da3bf5f0d65db46c7db7c6c88a51b5554f0f7b56ed7c285f0f98f02b07`。官方Chart.Export425×266/库424×265，各自原尺寸、不拉伸；key最大差4→1px、pitch8→1px，整体位置/AA和个别1px仍待审，不声称全图通过。
- 两个单实例各success7导出+exception实际导出后抛错，finally己方seed+copy均Close(false)、Workbooks0，SHA/锁/独占读通过。每轮前观察Excel进程0，未Save/Quit/kill、未碰用户book；异常证明图非gold。字体targeted15通过；规定full suite190 targets/2094 passed/0 failed/ignored；protected rectangular体/PNG SHA不变，F42五图与前版逐字节一致，shadow/gradient继续NI。
- 新页 `Desktop/pr-ni-F41-font-family-cell-metrics-待审.html`：1102569字节，SHA `ae844643a3235106258d8ae41c904d5f8fb44aa9b4d7c35d082648cac1a80f12`，14卡默认未审，旧50页不变。
- `90efcff4325962699e735cc2f2c502071d1f7d3f`：F44按[Microsoft文档](https://learn.microsoft.com/en-us/office/vba/api/excel.chart.copypicture)只做一次有区分意义的第三Size=xlScreen实验，不更换入口/放松guard。Excel16/DPI106绑定成功，首项（旧误标A4，实际索引1为B5l72，见t172勘误）被`Reject missing/stale Bitmap clipboard`拒绝，余六张未执行，PNG0/CLEAN0。失败分支未持久化格式/序列，不能断言本次仍EMF-only。己方seed+copy均Close(false)、Workbooks0、SHA/独占读/无锁repair通过；未Save/Quit/kill/用户book关闭；未原样重试。结束时己方空应用仍在，14:38 Asia/Shanghai再观察进程0，仅观察而非主动清理证明，无需用户关闭全部Office或改DPI。
- F44新页 `Desktop/pr-ni-F44-size-screen-仍NI-取证阻塞.html`：5929字节，SHA `2b5e1605a08f8d4cd0a2b8d1a70a1f6581152a03910cda78647f83f0cf14b898`，默认未审/无通过按钮/零gold，原51页SHA不变，现52页。原源SHA `78c921b30213401f7eae5470f1a74ba673808a7bdf9c0752faccb9f27827131b`不变。下一步先完善失败诊断持久化，再据可区分证据决定新实验，不盲重试。此项仅文档，未声称额外重跑测试。
- 计划49唯一ID，计数4接受/5待审/23部分/13NI/1安全/3非法；F45/S01/I01-I03、main/Pages不变。F41–43基础部分待审，复杂效果等仍NI；F44取证未完成，不把顺序覆盖误称全部支持/通过。


## F44 t172：失败记录落盘与图身份修复（仍NI，未审）

提交 `3f91ebd79811766a38b65c3cc4f7051f0788def3`，作者高奕GaoYi，普通push，未合并/接受。

- 修改五文件：`manual-check/lib/clipboard-export-diagnostics.ps1`、其`test-clipboard-export-diagnostics.ps1`、`manual-check/verify-chartsheet-clipboard-diagnostic.ps1`、原`verify-legend-foundation-lifecycle.ps1`、⚠️NI计划。失败前排队记录，catch/finally保留API inputs、阶段/时间、异常、序列及native/auto formats；额外格式读取在原尝试/guard结束后，诊断文件拒绝覆盖。原missing/stale Bitmap条件/throw、共享office-com及owned-excel-books均未改。无Rust渲染代码变化。
- 本轮唯一实际CopyPicture：14:45:47 Asia/Shanghai，Excel16/DPI106，STA，PID48228，参数Appearance1/Format2/Size1；调用正常返回。精确实际对象`B5l72`，paperSize13/landscape2/四边72pt，源包首位确为B5l72，A4p36位于第7。旧JSON文件名A4p36保留作原始证据，不事后伪装成正确标签。
- 序列5280→5287，GetImage=null；之后格式枚举前后均5287。Native/auto均为`Preferred DropEffect, InShellDragLoop, EnhancedMetafile, MetaFilePict`，无Bitmap。分类`missing_bitmap_fresh_sequence`，不是stale或本次binding/STA/COM调用异常；不能外推所有chartsheet/A4不支持Bitmap，也不能仅凭此排除全部外部并发因素。PNG0/CLEAN0，没有Chart.Export-only或EMF转换冒充gold。
- 已修复后续按明确名称`Charts.Item(name)`并断言返回Name；34项无Office检查含反序集合、错身份拒绝及引用释放。此次名称修复未再次实机导出验证，未开第二实例/重复B5实验。下一可区分实验是明确名称A4p36对照，启动前重新核验无残留实例。
- finally己方seed+copy均Close(false)，Workbooks0、FilesClean=true、SHA/独占读/无ownerlock/repair通过；无Save/Quit/kill、未碰用户book/DPI。结束仍有己方空窗，但14:52:36 Asia/Shanghai再观察PID48228已不存在；仅观察、非主动清理，目前不需要用户关窗，更不要求关闭全部Office。
- 源SHA `78c921b30213401f7eae5470f1a74ba673808a7bdf9c0752faccb9f27827131b`保持不变。证据位于Temp/t172-f44-clipboard-diagnostic-20261121；原始失败记录、独立cleanup、源顺序均保留。
- 新页`Desktop/pr-ni-F44-clipboard-diagnostic-仍NI.html`：13066字节，SHA `42cbfaf23c4a2b1d7966ea6f5343f66b899085a51417f250e8f856abb85f9ea7`，默认未审，无通过按钮。旧52页SHA全部未变，现53页。
- 本轮完整规定Rust suite：190 targets/2094通过/0失败或忽略；诊断脚本34 checks另计，ownership负向检查通过；受保护矩形测试体/PNG SHA未变。49唯一ID计数仍4接受/5待审/23部分/13NI/1安全/3非法；F45/S01/I01-I03、main/Pages、page_paint.rs、t131未动。F44官方矩阵仍未完成，不声称全部NI解决或自动接受。
- 已完整重读unblock-research，仅核对公开语义，不搜索/采用绕过强制guard的办法：[CopyPicture](https://learn.microsoft.com/en-us/office/vba/api/excel.chart.copypicture)、[Charts.Item名称参数](https://learn.microsoft.com/en-us/office/vba/api/excel.charts.item)、[GetImage返回null](https://learn.microsoft.com/en-us/dotnet/api/system.windows.forms.clipboard.getimage?view=windowsdesktop-9.0)。未执行文档的SetImage示例。


## F44 t173：ONE正确A4实测与已接受native出口核验（仍NI）

提交 `3eb64359ce561661286fe0f453b4f21cd8dc3c12`，普通push，正确作者，未合并或自动接受。

- 本轮唯一Office控制：2026-11-21 14:59:02 Asia/Shanghai，按名称Charts.Item("A4p36")，实际Name/ActiveSheet都是A4p36、Index7；paper9/portrait1/四边36pt在CopyPicture前逐项断言，contextVerified=true。Excel16/DPI106、STA、PID32952，参数(1,2,1)正常返回。序列5289→5296，GetImage=null，之后格式读取前后均5296，native/auto=[Preferred DropEffect,InShellDragLoop,EnhancedMetafile,MetaFilePict]，无Bitmap。精确`missing_bitmap_fresh_sequence`，与B5l72同类，至少非仅B5现象，不外推所有环境。未再试参数、未开第二Office、未把向量转PNG或放松guard。
- 新通用context assertion、A4 fixture前置验证与负向测试；44 checks无Office通过，完整规定Rust suite190 targets/2094通过/0失败或忽略。没有Rust渲染改动；shared binding/ownership与Bitmap/freshness guard不变，受保护矩形体/PNG SHA不变。执行前4脚本与SHA已冻结并事后核验，非逆向重建。
- finally己方seed+readonly copy均Closed=true、Workbooks0、FilesClean=true，源/副本SHA不变、独占读、无锁/repair，无Save/Quit/kill/用户book关闭/DPI或打印机变更。结束空应用仍在，15:04:11 Asia/Shanghai观察PID32952不存在、Excel0，仅观察而非主动清理；目前无需用户关窗。短暂工具失联恢复后取得完整落盘记录，未因此重跑。
- 关键新发现：已核实t84u接受的`.plan/2026年-11月-16日-✅-xlsx-图表工作表出图.md`及实际历史证书/PNG/源，native `Chart.Export`、`EXCEL_CHARTSHEET_CHART_EXPORT_NATIVE`、CLEAN_PUBLIC_OFFICE_EXPORT均真实存在；旧source SHA `7c88b60df4662af66666f60e36bd70b3b4cc611efaed5d6ee9e5a0fb71604704`，PNG1078×704 SHA `7629e72fe0af47bdb4a3bf41861a2efbda08ca08108c68a8d2c27094dc7cae48`逐一复核通过。历史证据不与当前F44源混同，不是本轮新gold。
- 安全实质下一步存在，不是“Excel没有官方出口”或用户DPI决策：按[Microsoft Chart.Export](https://learn.microsoft.com/en-us/office/vba/api/excel.chart.export)和已接受的独立native协议，用现代共享binding、显式只读seed/copy ownership、finally Close(false)、唯一新输出、PNG签名/尺寸/SHA、源/锁/repair核验采F44自身矩阵；激活/100%缩放后读取ChartArea。不能把失败Clipboard证书改CLEAN，也不能当作失败guard内的fallback。旧证书readOnly=false、旧image-corpus helper会删除已有输出，故不可直接原样跑旧脚本。此轮ONE A4已用，未启动第二Office；仍缺的是F44本源新的native PNG/证书，不是历史gold或向量栅格化替代。
- 本轮F44源SHA仍 `78c921b30213401f7eae5470f1a74ba673808a7bdf9c0752faccb9f27827131b`；新PNG0/CLEAN0。证据根Temp/t173-f44-named-a4-20261121，包括原始失败记录、cleanup、执行前脚本、历史native审计与测试日志（full-tests.txt为UTF-16LE）。
- 新页`Desktop/pr-ni-F44-named-A4-native-export-audit-仍NI.html`：14664字节，SHA `e3896a9c62b29752b98d99141323357951e4f2c430563466e1095b5e7c42faae`，默认未审、无通过按钮，原53页SHA全部未变，现54页。
- 49唯一ID计数仍4接受/5待审/23部分/13NI/1安全/3非法；F45/S01/I01-I03、既有已接受chartsheet计划/金样、main/Pages、page_paint.rs、t131未改。F44本源native矩阵未完成，不声称整表已实现/已接受。
