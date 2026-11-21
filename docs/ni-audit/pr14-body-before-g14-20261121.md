继续 t171/t172 前置优先。最新普通 commit **`60c6ff50b5fe350af6525aede188b5e4ccb7e838`** 已发布，HEAD=origin、clean/ahead0；PR14 OPEN，未合并、未自动接受。2026-11-21 21:07:11–21:07:16 Asia/Shanghai 基础commit `8d4b6781`一次普通 push 成功；清单计数勘误commit `60c6ff50` 于21:11:58–21:12:04 Asia/Shanghai另一次普通push发布（新metadata提交，不是网络重试）。renderer不变，无需为仅docs勘误重跑Rust suite；未变更凭据/transport，无重试循环。

## 权威计划与完整历史

- [当前49组计划/逐项边界](https://github.com/gaoyia/office-editor-mcp/blob/60c6ff50b5fe350af6525aede188b5e4ccb7e838/.plan/2026%E5%B9%B4-11%E6%9C%88-20%E6%97%A5-%E2%9A%A0%EF%B8%8F-xlsx-%E5%9B%BE%E8%A1%A8%E5%87%BA%E5%9B%BE%E6%9C%AA%E5%AE%9E%E7%8E%B0%E6%9D%A1%E4%BB%B6%E8%BF%BD%E8%B8%AA.md)：**4 accepted / 5 pending / 26 partial / 10 NI / 1 safety / 3 illegal**，不是全部NI解决。
- [g13实际实现、合同、验证范围](https://github.com/gaoyia/office-editor-mcp/blob/60c6ff50b5fe350af6525aede188b5e4ccb7e838/docs/ni-audit/F07-saturation-modulation-foundation-g13.md)。
- [本次压缩前PR14完整62450字audit（原文逐字保存）](https://github.com/gaoyia/office-editor-mcp/blob/60c6ff50b5fe350af6525aede188b5e4ccb7e838/docs/ni-audit/pr14-body-before-g13-20261121.md)，UTF8 SHA256 `cb8c0f1cee992b0690fb21305491a9535b0f194f8bdfc926cd57d9f77eb193c6`。旧批次进展/审计/纠正全部版本化保留，不靠追加挤满PR body。仅更新body，title/base/state不变。

## 最新 g13：F07独立饱和度调制共享基础（部分待审）

- **实际renderer功能**：sRGB/theme chart/plot solid fill 的独立signed `satMod`与可选末尾alpha。固定HSL hue/light，RGB通道绕(min+max)/2缩放，转换末端限幅；不是先夹saturation。严格signed integer-thousandth parser复用到lumOff，但旧±100000范围与原tint/shade/lum/默认路径不变。没有颜色表、样本路径、offset或几何补偿。
- **有界依赖评估**：g12已证明受控族legend存在依赖，仍没有唯一auto-key数值公式。没有凭相关性猜width/plot规则，转做上述独立实际缺失基础，不是又一轮audit-only。
- **新真实native32**：16调制+16数学RGB假设参考，同标题/288×180pt/数据，2彩色base、-50/0/50/100/200%、gray、theme、plot、alpha。源SHA `a0e75448ee77b1fb9a8349073644b25b6d74ce0954c993ef83c759b849251808`。早夹sat假设9/16整PNG字节相同、7对不同，已被反证；新一般代数颜色探针11 exact/5最大1byte（含alpha读回）残差，无rounding fudge。
- **范围准确**：来源是OOXML_DERIVED_REEXPORTED_BY_EXCEL / EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE，不是未修改官方原件、打印或CopyPicture Bitmap gold。native425×266、库424×265各自原尺寸，无拉伸/配准，未声明整图像素通过。16原NI新可渲染；16旧RGB参考+237历史控制=253旧PNG整字节不变，有意改旧PNG=0；4scatter仍NI。
- **测试/保护**：新增7Rust，完整mandatory2195tests/190targets/0failed/0ignored，Python15、actual CLI build通过。原accepted test/PNG SHA、page_paint/t131/main/Pages、旧默认输出和验收边界均保持。首次test DOM方法编译错误已修并重跑；首轮replay误把预期改动plan要求旧hash的harness断言已修正，成功原PNG保留重验，原日志透明保留，没有修改Office guards。
- **生命周期**：一次自有readonly seed+copy，explicit names/title/canvas、MC预检、FullName/PID、源SHA/锁审计，finally Close(false)两本且Workbooks0。无repair/Save/Quit/kill/userbook关闭/第二Office会话/shared binding或Bitmap guard改动。21:03 Asia/Shanghai仍观测空Excel PID30832；只是观察，没有声称主动清理或复用COM refs。

## 桌面审阅与剩余边界

入口保持 `NI-待审-INDEX-20261121.html`。新增唯一generation `20261121-F07-saturation-modulation-g13/F07-saturation-modulation-foundation-待审.html`（38506bytes，SHA `6ebee8724319e3dc7fd5402c4b85d9fdfd687248a066bf05fa283386ac29e213`）。当前69artifacts=63NI证据/41ID+6PR12，1252 local links已验证。清单计数明确勘误：2413是旧保护文件总数（2367归档内+46按原方案保留桌面），并非全部archive文件。68旧页及所有旧SHA/URL/localStorage状态/g8a/manifest/rollback保持；原g13页和manifest字节不改，INDEX显著链接metadata补充JSON。当前archive2790文件、全范围保护2836文件（含该补充5个metadata文件），没有丢失或意外文件。INDEX37279bytes SHA `060db22c9248f87cf4f3394dfb478e1979268165f72245f901039637d5fe9bed`。新页默认未审，无通过按钮或自动接受。

F07 percent-sign/fractional spelling、任意变换混用、sysClr/hueMod/alphaMod仍未扩展；颜色量化/alpha及整图layout/AA残差真实保留。legend auto-key、square terminal-gap、lim4plot、180°retrace/scatter及复杂pen边界不靠猜测“解决”。下一独立可做前置可评估standalone hue modulation或共享visible frame stroke；需要新Office证据前先安全idle重查，不沿用旧COM refs。其余各项状态及早期已实现基础以计划和完整audit为准；不等待审阅也不把待审当接受。
