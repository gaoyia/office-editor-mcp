继续 t171/t172 前置优先。最新普通commit **`04e0b4d30bd3e07783597f7e466eb04d3c89b5ea`** 已发布，HEAD=origin、clean/ahead0；PR14 OPEN，未合并/未自动接受。2026-11-21 22:35:31–22:35:41 Asia/Shanghai一次原SSH普通push成功，无网络retry loop/凭据/transport变更。

## 权威计划与历史

- [49组计划与边界](https://github.com/gaoyia/office-editor-mcp/blob/04e0b4d30bd3e07783597f7e466eb04d3c89b5ea/.plan/2026%E5%B9%B4-11%E6%9C%88-20%E6%97%A5-%E2%9A%A0%EF%B8%8F-xlsx-%E5%9B%BE%E8%A1%A8%E5%87%BA%E5%9B%BE%E6%9C%AA%E5%AE%9E%E7%8E%B0%E6%9D%A1%E4%BB%B6%E8%BF%BD%E8%B8%AA.md)：**4 accepted /5 pending /26 partial /10 NI /1 safety /3 illegal**，不是全部NI实现。
- [g17 alphaOff实现、合同、实际证据与未验证边界](https://github.com/gaoyia/office-editor-mcp/blob/04e0b4d30bd3e07783597f7e466eb04d3c89b5ea/docs/ni-audit/F07-alpha-offset-foundation-g17.md)。g16 hueOff/g15精确alphaMod等旧证据互补、不重写接受边界。
- [前3089字符PR body逐字归档](https://github.com/gaoyia/office-editor-mcp/blob/04e0b4d30bd3e07783597f7e466eb04d3c89b5ea/docs/ni-audit/pr14-body-before-g17-20261121.md)，SHA `0bfa268ab8a02f02f82db4a18b8c271b7d82cac551ee84defb25a1cb5ec6b1ea`；更早全部audit保持。仅body更新，title/base/state不变。

## 最新 g17：F07 signed alphaOff（部分待审）

- **真实基本语义**：Excel sRGB/theme chart frame与Cartesian plot纯色支持alphaOff、alpha→alphaOff、alphaOff→alpha。按百分比点相加（40%+10%=50%而非44%），结果clamp0..100%；默认输入100%，末尾绝对alpha覆盖但前offset仍严格校验。复用既有signed-fixed parser，整数千分之一百分比[-100000,100000]；新helper以i64安全相加后复用原Solid/Translucent。其他mixed/repeated/nested/legend前景语义仍NI，不扩展Word/PPTX/pie透明plot。
- **38预声明native控制**：19变换+19旧alpha参考，同title/288×180pt/数据/轴，覆盖两base、-100/-50/0/+50/+100%、显式输入40%±offset、两种顺序/theme/gray/plot。sourceSHA `98fae59b871e8ea5face775d8d009f90fd82aff2c440784e90d076aa5678ee25`。19对native整PNG字节同，18chart背景alpha精确。SDK明确加法和双端clamp，没有猜测补偿。
- **真实残差与对比边界**：库19探针RGB12精确，chart最大1byte，alpha全部同。主plot和chart同色，其RGB误差0不能独立证明局部opacity。已另行固定EAF7F7背景的2个plot对比控制，但22:30安全idle观察仍有Excel进程1，故未再开Office、未重试/绕guard。该补充源只作预声明、不是native gold；独立plot对比尚未取得。一般plot复用原Translucent，并有不同背景DPI集成测试，不能把它冒充native证明。native425×266/库424×265各原尺寸、无拉伸配准；全图AA/layout与颜色读回分开，不宣称像素通过或降低zero-pixel标准。
- **完整验证**：8新增Rust；mandatory2228passed/190targets/0failed/0ignored，Python15、actual CLI build通过。381历史PNG（含g16全部35）+19本轮旧alpha参考=400既存PNG整字节不变，有意改旧PNG=0，4scatter仍NI。18旧helper含alphaMod精确乘积/hueOff/alpha解析转换函数体hash不变；accepted test/PNG SHA、page_paint/t131/main/Pages、旧默认与mixed guard均保持。binary/request/source/PNG逐项回执可重验。
- **生命周期与来源**：一次自有readonly seed+copy，经原MC preflight、explicit名字/title/canvas、FullName/PID/SHA/锁审计，finally Close(false)两本、当时Workbooks0、文件不变/no repair；无Save/Quit/kill/userbook关闭，binding/Bitmap guard不改。22:34仍观察Excel PID45756/进程1，未再次COM绑定，不冒称当前工作簿状态或主动清理。OOXML_DERIVED_REEXPORTED_BY_EXCEL / EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE，非未改官方原件、打印或CopyPicture Bitmap gold。

## 桌面入口与剩余前置

仍从 `NI-待审-INDEX-20261121.html`（40900bytes SHA `5eeadc7063e7603428e83a7b1d1eb93dc2549dc80ea8e0414ac476f01f9fb5c4`）进入。新增唯一 `20261121-F07-alpha-offset-g17/F07-alpha-offset-foundation-待审.html`，50965bytes SHA `44c891d741ce6463c9d3d94aa5d068df8bd6d0f990898e57bb3e7d72860be93a`。73artifacts=67NI证据/41IDs+6PR12，1768 local links通过；旧72artifacts/4369保护文件（4323归档+46桌面）原字节/URL/localStorage状态/g8a/g13勘误/manifest/rollback保持。现4920归档+46桌面=4966全范围保护文件。新页默认未审。

下一先在新的safe-idle检查允许时完成已固定的plot对比pair（不重做主38矩阵），随后可做satOff合同/控制。sysClr/hslClr、其他mixed/legend auto-key、square terminal-gap、lim4plot、180°retrace/scatter等仍未解决；继续原授权基础优先，不等待审阅，不无边界扫AA或阈值。
