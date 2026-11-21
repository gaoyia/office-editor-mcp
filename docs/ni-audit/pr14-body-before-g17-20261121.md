继续 t171/t172 前置优先。最新普通commit **`51e9a6a9eff1918c13e8a51ca69484d57a7daaf8`** 已发布；HEAD=origin、clean/ahead0，PR14 OPEN，未合并/未自动接受。2026-11-21 22:19:39–22:19:44 Asia/Shanghai一次原SSH transport普通push成功，无retry loop/凭据/transport变更。

## 权威计划与完整历史

- [当前49组计划与边界](https://github.com/gaoyia/office-editor-mcp/blob/51e9a6a9eff1918c13e8a51ca69484d57a7daaf8/.plan/2026%E5%B9%B4-11%E6%9C%88-20%E6%97%A5-%E2%9A%A0%EF%B8%8F-xlsx-%E5%9B%BE%E8%A1%A8%E5%87%BA%E5%9B%BE%E6%9C%AA%E5%AE%9E%E7%8E%B0%E6%9D%A1%E4%BB%B6%E8%BF%BD%E8%B8%AA.md)：**4 accepted /5 pending /26 partial /10 NI /1 safety /3 illegal**，不是全部NI实现。
- [g16 signed hueOff实现/合同/证据](https://github.com/gaoyia/office-editor-mcp/blob/51e9a6a9eff1918c13e8a51ca69484d57a7daaf8/docs/ni-audit/F07-hue-offset-foundation-g16.md)；g15 alphaMod、g14 hueMod、g13 satMod等证据保持互补。
- 历史audit已版本化：[本轮前3409字符逐字原文](https://github.com/gaoyia/office-editor-mcp/blob/51e9a6a9eff1918c13e8a51ca69484d57a7daaf8/docs/ni-audit/pr14-body-before-g16-20261121.md)，SHA `8aaaf9d244bd3f0b2f15cfb0ea283f8e7d82403d99836f25600d74d3183312df`；更早g15/g14/g13归档原字节不变。仅更新body，title/base/state不变。

## 最新 g16：F07 signed hueOff共享前置（部分待审）

- **实际功能**：Excel frame（sRGB/theme chart frame及Cartesian plot纯色）支持独立hueOff与可选末尾alpha。Val为signed Int32 ST_Angle，60000单位=1度；复用原严格signed解析和原RGB↔HSL helper，饱和度/亮度不变。重复/混用/alphaMod组合仍NI；Word/PPTX host不扩展。
- **SDK与native差异如实记录**：SDK HueOffset.Val注释写越0..360夹角。进Office前预声明clamp与wrap两模型；35个native控制（14变换+14clamp参考+7额外wrap参考，sourceSHA `7e1ebfa6ab58bbaece6fe691dc16779146b27f49eeb84f7870d482e80ea612db`）中14变换均与参考整PNG字节相同，7个真正越界控制全部拒绝clamp、匹配色环周期。实现采用这一已验证的Excel16 native profile，不宣称SDK文字也说wrap，也不宣称其他host已验证。
- **实际残差**：14个native颜色探针库RGB 8个精确、最大1byte（chart/plot均1），alpha全部相同；原HSL舍入未为拟合改动。native425×266/库424×265各原尺寸，无拉伸/配准，不是全图像素通过，zero-pixel验收标准不降。
- **回归/测试**：14原NI输入新增渲染；346历史PNG（含g15全部45）+21本轮旧RGB参考=367既存PNG整字节不变，有意改旧PNG=0，4scatter仍NI。17旧helper（含alphaMod精确乘积/转换/初始alpha校验）函数体hash不变。新增8Rust；mandatory2220passed/190targets/0failed/0ignored，Python15、actual CLI build及binary/request/source/PNG回执通过。旧hueMod NI负例中一项裸hueOff已被支持，改为仍不支持的hueOff+hueMod；初始测试按Python数学舍入期望RGB导致的失败log保留，生产helper未为其修改。
- **来源/生命周期**：OOXML_DERIVED_REEXPORTED_BY_EXCEL / EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE，不是未改官方原件、打印或CopyPicture Bitmap gold。一次自有readonly seed+copy，经MC preflight、explicit名字/title/canvas、FullName/PID/SHA/锁审计，finally Close(false)两本且Workbooks0、源不变；无repair/Save/Quit/kill/userbook关闭/第二Office会话，binding/Bitmap guard不改。22:18观察Excel进程0，仅OS观察，不是主动cleanup。

## 桌面审阅与未完成边界

入口仍 `NI-待审-INDEX-20261121.html`（40045bytes SHA `406d1f76b4b14fe42968c1c14c3b24f510faddbbbd124609b3f6aafc19cc9701`）。新唯一generation `20261121-F07-hue-offset-g16/F07-hue-offset-foundation-待审.html`，45010bytes SHA `98e00effccbb2d92cbb457aeccebb359b34683fbb89d559e88f91b602991d5cb`。现72artifacts=66NI证据/41IDs+6PR12，1639 local links通过；旧3824保护文件（3778归档+46桌面）原字节/URL/localStorage状态/g8a/g13勘误/manifest/rollback保持，现4323归档+46=4369全范围保护文件。新页默认未审。

下一独立可做色彩前置是alphaOff（signed fixed percentage合同+新受控证据）。sysClr/hslClr、其他任意组合、pie透明plot、legend auto-key、square terminal-gap、lim4plot、180°retrace/scatter等边界保留；继续前置优先，不等待审阅、不扫阈值，也不把基本语义当像素接受。
