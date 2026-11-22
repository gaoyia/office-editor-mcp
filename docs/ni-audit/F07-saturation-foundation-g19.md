# F07 absolute saturation / contrasting plot-alpha evidence — g19, partial pending review

## 实际通用基础：sat是覆盖，不是mod或offset

SDK Saturation说明：用指定saturation替换输入S，hue/luminance不变；通常S在[0%,100%]。类型CT_Percentage/ST_Percentage，不是CT_FixedPercentage。
https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.drawing.saturation?view=openxml-3.0.1

新增frame_saturation复用既有严格signed i32整数千分之一百分比parser及rgb_hsl/hsl_rgb，仅独立sat、sRGB/theme、0..100000常规范围。非零灰色的hue选择、越界clip/extrapolation/inversion及alpha/其他mixed/repeated/nested保持NI。灰色S0保持原字节；有色转S0中和为原HSL L的灰色。没有猜越界clamp，未扩展Word/PPTX/foreground/legend gate。satOff的S0..1与灰色限制原样保留，不从新sat推断另一个变换的native行为。

例156082设S25%=39535E、S50%=265A71、S75%=136184；这与原色S乘50%或加25%不同。这些是预声明Python colorsys数学参考，不是native gold。旧20helper（含satOff/alphaOff/hueOff/精确alphaMod）函数体及ALPHA_PRODUCT_DENOM不变；默认painter/accepted语义保持。

absolute sat固定源SHA `e1c17c05b21de9035afb03ead31cb2dcaa6cd7c25e938acc7ded0c2d273bda60`，30个MC preflight通过控制=15sat+15旧显式RGB参考，覆盖两base的0/25/50/75/100%、theme、grayS0、纯红及不同EAF7F7背景plot。旧binary先验证15NI/15可渲染。新15输入可渲染，数学参考整PNG相同 `15`/15，RGB探针最大误差 `0` byte。其结果证明合同驱动库语义/集成，不是Excel量化或全图通过；absolute sat与g18 satOff native仍pending。

## 完成原固定plot-alpha pair独立证据审计，无重复Office

此前22:58:05 Asia/Shanghai的fresh safe-idle0后，仅一次原verifier会话导出原已固定2个plot-alpha控制。源SHA `570b86bc4f07bf0c4af2409ec2701889c345ae41e38e801d2cc3a374f6457709`，与g17预声明源逐字节相同，未重新生成/改参数。精确名字/Sheet1/title/288×180pt、源/readonly/PID/106dpi/Excel16、原MC preflight均按原guard断言。来源OOXML_DERIVED_REEXPORTED_BY_EXCEL / EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE，非未改官方原件、print或CopyPicture Bitmap gold。

这两张native PNG的完整SHA均 `6eeea1795a6760965fdb4855fa12bbaf05ca96b748c2cc8717fed59a621312e8`，整字节同；两库PNG完整SHA均 `cabfc5e10fe12ffa0dbe9779d62e42cf40af6c4ee4554e5d88ba1379522bcc95`，整字节同。不同EAF7F7 chart背景上，native plot主色(128,171,188,255)、库(127,171,188,255)，两者均与背景及输入蓝色不同。这独立辨识该输入alpha40%+offset10%与直接alpha50%的plot opacity，修复g17主pair同背景不可辨识缺口；不外推所有组合/所有offset，也不改旧页面历史表述。native/库425×266/424×265，原点共有区域69327px不同仅诊断，无拉伸/配准。RGB1byte及整图AA/layout残差诚实保留，不修改coverage舍入拟合此例，zero-pixel接受标准不变。

本次23:00后的恢复chunk只有文件审计/CLI/代码/测试，没有新Office、重复导出、PID轮询、复用COM或force cleanup。历史22:58生命周期证明已实读：自有readonly seed+copy在finally Close(false)，Workbooks0/FilesClean/源SHA不变/独占读/锁消失/no repair，Save/Quit/Kill=false；当时自有PID54676仍有窗口，不能冒称已主动清理或当前用户工作簿状态。satOff native没有趁该PID仍存在重开或复用。

## 实际验证与受保护输出

新增8Rust：绝对覆盖不同于乘/加、H/L量化约束、原色/中和/256gray、常规域和achromatic hue边界、严格signed percentage/i32/结构、sRGB/theme、chart/不同背景plot在96/106/144dpi整输出同数学RGB、混用与旧satOff边界NI。mandatory2244passed/190targets/0failed/0ignored，Python15/actual CLI build通过。

447历史PNG（g18全部28控制+此前419）、本轮15旧RGB参考，以及固定plot pair的2个旧binary输出，共464既存PNG字节不变，有意改旧PNG=0；4scatter仍NI。binary/request/source/PNG逐项记录、20helper、accepted rectangular函数体/PNG SHA、page_paint/t131/main/Pages、shared Office/native exporter/Bitmap guard均复核。新库binary重放plot pair也保持旧SHA，未调整alpha像素。

## 审阅、版本、未解决部分

唯一generation20261121-F07-saturation-absolute-g19/F07-saturation-foundation-待审.html通过原桌面INDEX。旧74review artifacts/5485保护文件（5439归档+46桌面）的原字节/URL/localStorage状态/g8a/g13勘误/manifest/rollback保持；新增默认未审。现75artifacts=69NI证据/41IDs+6PR12，精确文件/链接数见manifest/final receipt。历史主pair的同背景限制不涂改，而由本代独立证据补充。

此前PR body1379字符/2014bytes逐字备份于docs/ni-audit/pr14-body-before-g19-20261121.md，SHAfc93808a46de263d6a53bedba8ffc28e12cf1cbb952265a2e2de3f7c5fe35e9c。普通正确作者commit，原g18 local195261a5不得丢弃；远端发布/PR状态以最终probe/push receipt为准，不把未push代码称已发布。父代理维护简洁PR说明，旧audit完整保留。

49组仍4accepted/5pending/26partial/10NI/1safety/3illegal，不是全部实现。下一未来safe-idle0允许的批次才验证固定satOff/absolute sat native；本chunk不再开Office、不重复plot pair。若Office条件不满足，可独立评估绝对lum合同/解析数学基础。非零灰色、越界/mixed、sysClr/hslClr、legend auto-key、square terminal-gap、lim4plot、180retrace/scatter等仍在，不能把语义基础冒称像素完成，不做AA/阈值无界扫描。
