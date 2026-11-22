原t166–172前置优先继续。g26 `03c18dc1106fc4e6e45dcea51771550ca5f13191` 已发布，HEAD=origin，clean/ahead0；正确作者普通commit/hooks，PR14保持OPEN未合并，无接受/外部评论/PR正文写入。一次原SSH只读probe成功，随后一次普通push exit=0，无重试/凭据/transport变更。

先核sysClr：Ecma2016 §20.1.2.3.33定义lastClr为生成器最后计算色，§20.1.10.58规定val取查看系统当前色、不支持系统色时SHALL用lastClr。两者不是同一值。现有Excel保真profile无显式平台palette/回退策略输入，不能静默把缓存当当前色或查询ambient host。sysClr及矛盾缓存NI测试保持；需显式deterministic palette或opt-in fallback合同，不伪造支持、不因此停做。

实际新增独立red/green/blue正常域共享功能：规范§20.1.2.3.4/.10/.23替换指定分量、其他两分量不变；48新控制辨别linear-IEC强度与encoded-byte百分比。新frame_rgb_component严格leaf/unqualified val整数0..100000，复用既有严格integer parser及精确IEC encoder，不猜gamma/clamp或混用。29旧helper/190preset map/HSL/精确alpha/default/accepted/其他host保持；新增1后30helper。无旧断言/旧图修改。

源SHA 1d9bedf9cc7d4c650fb207f7d616cb93db91bd203bf6f914a47e80f5d357fef2，实现/native前固定48控制=24分量+24独立80digit Decimal RGB参考，另存encoded替代模型。old binary24NI/24参考成功；新24均渲染。24native/native参考整PNG同24/24；encoded替代仅6端点同、18非端点支持预声明linear-IEC，并非唯一公式/穷举native证明。native/库RGB精确24/24/maxerror0byte，库/独立参考整PNG同24/24。

fresh idle0 2026-11-22T01:20:28.4578343+08:00，仅此新矩阵一次48派生Chart.Export：owned readonly seed+显式copy/身份MC preflight不变，2本finally Close(false)、Workbooks0、SHA不变/no锁repair/SaveQuitKill=false。不是print/clipboard/未改官方gold。末尾2026-11-22T01:20:35.9145717+08:00观察进程1/空窗口PID68396仍在，仅历史观察非reuse/cleanup许可，无后续Office/idle循环/重复旧矩阵。Hue/HSL/preset native、SDK蓝例仍pending。

native425×266/库424×265，整PNG同0/24，原点差18074..18075px仅诊断，不拉伸/补偿/降低zero-pixel标准；旧1byte/AA/geometry/stroke边界不变。Mandatory2292passed/190targets/0failed/0ignored，8新Rust/Python15/CLI通过；全687历史+24当前旧参考=711既存PNG字节不变，变化0，4scatter仍NI。下轮完整成功集735=Cases48+Prior687；29旧helper和accepted/page_paint/t131/main/Pages/Office guard SHA保持。

唯一generations/20261122-F07-rgb-components-g26/F07-rgb-components-foundation-待审.html接入原INDEX，82artifacts=76NI证据/41IDs+6PR12，2554有效href及全部img。原81artifacts/10411files/URL/localStorage/state/g8a/g13/all manifests/rollback原字节保持，现fullscope11333files，默认未审。旧PR3057bytes逐字版本化pr14-body-before-g26-20261122.md；详情docs/ni-audit/F07-rgb-components-foundation-g26.md。

49组仍4accepted/5pending/26partial/10NI/1safety/3illegal。下一原scope独立RGB分量offset/modulation需先核线性域算术/正常域合同和控制，不猜byte math/clamp；sysClr仍有显式policy前置，其他mixed/new-base gating保持。不等review/网络，不把native颜色语义进展说成全图pass。