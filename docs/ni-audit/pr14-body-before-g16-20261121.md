继续 t171/t172 前置优先。最新普通commit **`41fb10e3c5734f50c20b666ac1249a316f3fc7b3`** 已发布；HEAD=origin、clean/ahead0，PR14 OPEN，未合并/未自动接受。2026-11-21 22:00:45–22:00:49 Asia/Shanghai一次原SSH transport普通push成功，无retry loop/凭据/transport变更。

## 权威计划与完整历史

- [当前49组计划与边界](https://github.com/gaoyia/office-editor-mcp/blob/41fb10e3c5734f50c20b666ac1249a316f3fc7b3/.plan/2026%E5%B9%B4-11%E6%9C%88-20%E6%97%A5-%E2%9A%A0%EF%B8%8F-xlsx-%E5%9B%BE%E8%A1%A8%E5%87%BA%E5%9B%BE%E6%9C%AA%E5%AE%9E%E7%8E%B0%E6%9D%A1%E4%BB%B6%E8%BF%BD%E8%B8%AA.md)：**4 accepted /5 pending /26 partial /10 NI /1 safety /3 illegal**，不是全部NI实现。
- [g15 alphaMod真实实现/合同/证据](https://github.com/gaoyia/office-editor-mcp/blob/41fb10e3c5734f50c20b666ac1249a316f3fc7b3/docs/ni-audit/F07-alpha-modulation-foundation-g15.md)；g14 hueMod/g13 satMod及更早lumOff/IEC证据保持互补，不重写旧接受边界。
- 历史audit已版本化：[本轮前3638字符逐字原文](https://github.com/gaoyia/office-editor-mcp/blob/41fb10e3c5734f50c20b666ac1249a316f3fc7b3/docs/ni-audit/pr14-body-before-g15-20261121.md)，SHA `fad1af659e851040d33db2a2ff14ee5194b513722f84bda6c409c21f9a9fa204`。更早3513及62450字符完整归档保持原字节；仅更新简洁body，不改title/base/state。

## 最新 g15：F07 alphaMod与精确opacity乘积前置（部分待审）

- **实际共享功能**：sRGB/theme chart frame及Cartesian plot纯色支持独立alphaMod、alpha→alphaMod、alphaMod→alpha。SDK明确调制乘输入opacity、最多100%，absolute alpha指定新opacity。复用严格非负i32 percentage解析；factor可大于100000，新alpha操作数严格0..100000。末尾alpha覆盖但不忽略前项校验，其他未验证混用仍NI。
- **先补真实精度前置**：整数百分比不能精确保留乘积。u64保存精确分子/100000²、封顶1后仅在最终8bit coverage舍入；整百分比/opaque仍走原Translucent/Solid，只有新fractional用AlphaProduct。无浮点拟合/颜色表/样本路径或舍入补偿。
- **45受控native判别**：21变换+24预声明旧alpha参考，同title/288×180pt/轴/数据，两base/gray/theme/plot/两种顺序/0..200%。sourceSHA `41c50fed169a01020140b342a5ef70e89e12604b514a32a4abe0fb033ec6e7cc`。全部21变换有完整native PNG相同参考，3张错误fractional参考不同；50000×33333与50001×33333的native alpha分别42/43，排除统一提前round或floor百分比。不是native↔库全图通过。
- **实际残差**：19chart背景alpha与native全部一致；21个chart/plot颜色探针10精确，chart RGB读回最大3byte，plot合成最大1byte。保留输入RGB，不拟合native解预乘读回。native425×266、库424×265各原尺寸，无拉伸/配准；颜色与整图AA/layout残差分开，zero-pixel验收标准不降。
- **回归/完整测试**：21原NI输入新增渲染；301历史PNG（含g14全部32）+24本轮旧alpha参考=325既存PNG整字节不变，有意改旧PNG=0，4scatter仍NI。14旧helper（含alpha解析/byte转换）函数体hash不变。新增9Rust，mandatory2212passed/190targets/0failed/0ignored，Python15、actual CLI build及binary/request/source/PNG逐项回执通过。一项已支持语义的旧NI负例改为仍不支持的混用，初次失败log保留；未弱化接受守卫。
- **准确来源/生命周期**：OOXML_DERIVED_REEXPORTED_BY_EXCEL / EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE，不是未改官方原件、打印或CopyPicture Bitmap gold。一次自有readonly seed+copy经原MC preflight、explicit名字/title/canvas、FullName/PID/SHA/锁审计，finally Close(false)两本且当时Workbooks0；无repair/Save/Quit/kill/userbook关闭/第二Office会话，binding/Bitmap guard不改。21:55观察进程1，21:58观察0，仅OS观察，不能冒称主动cleanup或代替book关闭证明。

## 桌面审阅与真实未完成边界

入口仍 `NI-待审-INDEX-20261121.html`。新唯一generation `20261121-F07-alpha-modulation-g15/F07-alpha-modulation-foundation-待审.html`，61412bytes SHA `2da6b414d5ee431dd22e47f94efc26c1feacab3748686a738e0478d925e41a0a`。当前71artifacts=65NI证据/41IDs+6PR12，1518 local links验证；INDEX39169bytes SHA `004aeb69dc0f6a1a14139100d49716397b41f6581fe23f3633564d32cc4ef140`。

旧70artifacts/3282保护文件=3236归档+46保留桌面，全部原字节/URL/localStorage状态/g8a及g13勘误/manifest/rollback保持。本轮新增542文件（含manifest），现3778归档文件+46保留桌面=3824全范围保护文件，不混称archive。新页默认未审，无自动通过/接受。

下一独立可做色彩前置是hueOff（signed-angle合同和新受控证据），新Office前仍重新安全idle核对。sysClr/hslClr、其他任意transform组合、pie透明plot、legend auto-key、square terminal-gap、lim4plot、180°retrace/scatter及其他已记边界保留；继续原授权前置优先，不等待审阅、不扫描阈值，也不把基本语义当像素接受。
