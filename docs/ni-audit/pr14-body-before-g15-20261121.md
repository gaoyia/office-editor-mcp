继续 t171/t172 前置优先。最新普通commit **`74a5cac23225c754376b11647d1905dee1aedcbb`** 已发布，HEAD=origin、clean/ahead0；PR14 OPEN，未合并/未自动接受。2026-11-21 21:31:16–21:31:22 Asia/Shanghai一次普通SSH push成功，无retry loop/凭据或transport变更。

## 权威计划与完整历史

- [当前49组计划及逐项边界](https://github.com/gaoyia/office-editor-mcp/blob/74a5cac23225c754376b11647d1905dee1aedcbb/.plan/2026%E5%B9%B4-11%E6%9C%88-20%E6%97%A5-%E2%9A%A0%EF%B8%8F-xlsx-%E5%9B%BE%E8%A1%A8%E5%87%BA%E5%9B%BE%E6%9C%AA%E5%AE%9E%E7%8E%B0%E6%9D%A1%E4%BB%B6%E8%BF%BD%E8%B8%AA.md)：**4 accepted /5 pending /26 partial /10 NI /1 safety /3 illegal**，不是全部NI实现。
- [g14 hueMod实际实现/合同/证据](https://github.com/gaoyia/office-editor-mcp/blob/74a5cac23225c754376b11647d1905dee1aedcbb/docs/ni-audit/F07-hue-modulation-foundation-g14.md)；[g13 satMod及清单计数勘误](https://github.com/gaoyia/office-editor-mcp/blob/74a5cac23225c754376b11647d1905dee1aedcbb/docs/ni-audit/F07-saturation-modulation-foundation-g13.md)仍互补。
- 历史audit不靠无限追加PR body：[本轮前3513字符原文](https://github.com/gaoyia/office-editor-mcp/blob/74a5cac23225c754376b11647d1905dee1aedcbb/docs/ni-audit/pr14-body-before-g14-20261121.md)，SHA `0318a8e2b656eba9dd94641f5f841f818d708a528cb719b3417368da470ef801`；[更早62450字符完整原文](https://github.com/gaoyia/office-editor-mcp/blob/74a5cac23225c754376b11647d1905dee1aedcbb/docs/ni-audit/pr14-body-before-g13-20261121.md)，SHA `cb8c0f1cee992b0690fb21305491a9535b0f194f8bdfc926cd57d9f77eb193c6`。两者逐字版本化保留；仅body更新，title/base/state不变。

## 最新 g14：F07独立hueMod共享基础（部分待审）

- **真实renderer功能**：chart/plot纯色sRGB/theme的独立hueMod，以及可选末尾alpha。先核SDK“angular hue按百分比调制”与XSD PositivePercentage=xsd:int且>=0；复用严格signed解析后限制非负，接受0..i32MAX整数千分之一百分比，不误用alpha的100000上限。复用既有RGB↔HSL，角度乘比例后按整圈取模，不把越界全部夹为红色；无样本表/offset/舍入补偿。
- **32受控native、先判别后实现**：14变换+14预声明wrap数学RGB参考+4额外clamp参考，同title/288×180pt/轴/数据；2彩色base、0/50/100/200/300%、gray、theme、plot、alpha。sourceSHA `acf5e3d4c3486d8b0e4f21845f300b36263ebf26f2550988e57a241b84480c9c`。四个真正越整圈控制全部支持周期wrap、拒绝clamp；12/14 wrap参考native整PNG字节同、2对量化/alpha不同，不是native↔库整图通过。
- **实际残差**：新库对14个native目标颜色探针12精确，最大RGB残差2byte，alpha全部相同；不修改原HSL helper来拟合半色角/alpha读回差异。native425×266、库424×265各自原尺寸，无拉伸/配准；颜色语义与整图layout/AA残差分开，不降低既有验收标准。
- **实际收益/回归**：14原NI输入变体新增可渲染。269历史PNG（包含原253及g13新增16 satMod）+18本轮既存RGB参考=287旧PNG整字节不变，有意改旧PNG=0；4scatter仍NI。旧lumOff/satMod/tint/shade/alpha/严格解析/HSL共10个helper函数体hash不变。
- **完整验证**：新增8Rust；mandatory2203passed/190targets/0failed/0ignored，Python15和actual CLI build通过。每次replay记录实际binary/request/input source/output SHA。accepted test/PNG、page_paint/t131/main/Pages、旧默认路径及边界保持。
- **来源/生命周期准确**：OOXML_DERIVED_REEXPORTED_BY_EXCEL / EXCEL_CHARTOBJECT_CHART_EXPORT_NATIVE，不是未修改官方原件、打印或CopyPicture Bitmap gold。一次自有readonly seed+copy，经MC preflight、explicit name/title/canvas、FullName/PID/SHA/锁审计，finally Close(false)两本且Workbooks0；无repair/Save/Quit/kill/userbook关闭/第二Office会话或guard/binding改动。21:28 Asia/Shanghai观察Excel进程0，只是观察，不冒称主动cleanup，不复用旧COM引用。

## 桌面审阅与未完成边界

入口保持 `NI-待审-INDEX-20261121.html`；新增唯一generation `20261121-F07-hue-modulation-g14/F07-hue-modulation-foundation-待审.html`，41239bytes SHA `77d0f3148bea9902dca23d060c724eae59bfd19bd32460b981715cf5cbc91954`。当前70artifacts=64NI证据/41IDs+6PR12，1364 local links通过；INDEX38187bytes SHA `e260c840189e22d73d7cf5d13137be63692c061b2d69aa91733f6348b1599425`。

旧69artifacts/2836保护文件=2790归档内+46保留桌面，原URL/localStorage状态/g8a/g13计数勘误/所有manifest与rollback逐字节保留。本轮增加446文件（含manifest），当前3236归档文件、3282全范围保护文件；不是把全部保护文件叫archive文件。新页默认未审，无自动通过/接受。

下一独立前置可做alphaMod，复用本轮positive percentage解析；仍需合同与新受控证据，Office前重新安全idle核对。sysClr/hueOff/hslClr base/%及小数词法/任意变换混用、legend auto-key、square terminal-gap、lim4plot、180°retrace/scatter等边界保留。其余实现与历史残差以计划和audit为准，继续前置优先，不等待审阅也不把部分支持当接受。
