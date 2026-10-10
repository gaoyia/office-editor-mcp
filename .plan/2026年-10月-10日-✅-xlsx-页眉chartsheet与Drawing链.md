# xlsx 页眉、chartsheet drawing 与 DrawingML 链

状态：✅
日期：2026-10-10

对应 `roadmap/xlsx.md` 待做序号 **36**。前置：页眉页脚、形状、OLE、超链接已过。

## 目标

页眉 `drawingHF` 里已有的图 / 形状 / 连接线能进树。图表工作表上 SpreadsheetDrawing 与 Chart Drawing（`c:userShapes`）里的图 / 形状 / 文本框 / 连接线能进树，并能复用已有 insert / delete / 尺寸 / 锚点 op。工作表图、文本框、形状、SmartArt 上的 DrawingML 超链接能读写。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | 文档：宿主 id、DrawingML 链、chartsheet 相对锚点 | ✅ |
| 1 | 库：inspect / apply | ✅ |
| 2 | 库测试与 Excel 16.0 手测 | ✅ |

## Excel 16.0 探测锁定

- 工作表图 / 文本框 / 形状 / SmartArt 的 COM 超链接写成 `xdr:cNvPr/a:hlinkClick`，关系在 **drawing** `.rels`，Type 以 `/hyperlink` 结尾。外部链 `TargetMode=External`。内部位置 `Target="#Sheet1!A1"`，不写 `TargetMode`。`tooltip` 写在 `a:hlinkClick/@tooltip`。不写 run 级链，也不写 `hlinkHover`。
- `HYPERLINK()` 仍当公式字符串。单元格 `<hyperlinks>` 与 DrawingML 链分家，不要混进同一份名单。
- 图表工作表上 COM `Shapes.AddShape` / `AddPicture` / `AddConnector` 写到该表那张图的 Chart Drawing：`c:chartSpace/c:userShapes` → `chartUserShapes` 关系，部件根为 `c:userShapes`，锚点 `cdr:relSizeAnchor`（0–1 分数），内容类型 `drawingml.chartshapes+xml`。
- 往图表工作表 SpreadsheetDrawing 再塞 `xdr:absoluteAnchor` 时 Excel 另存会留下且无 Repair，但 `Chart.Shapes` 数不增加。inspect 两边都收。**apply 往 chartsheet 插入时写 Chart Drawing**，与 COM 另存一致。
- 页眉：Excel UI / 另存仍是 VML `legacyDrawingHF`。往 `drawingHF` 塞形状后 Excel 另存会丢掉 DrawingML 页眉、只留一张 VML 图。页眉 **apply 仍只认图**。`drawingHF` 里若已有形状 / 连接线 / 文本框则 inspect 列出，不提供 insertShape 到槽位。
- 图表工作表 OLE：COM `OLEObjects.Add` / `Shapes.AddOLEObject` 失败。链接 OLE `@link` 本机 COM 也建不出来。本期仍不进树、不提供 apply。
- 不编辑 VML / `legacyDrawingHF`。不按列宽或图表框反推分数坐标。不激活 OLE 服务器。

## 不做

- 编辑 VML
- 公式求值、打开浏览器
- 图表工作表上插第二张图、插 OLE / SmartArt / 3D / 切片器 / 公式框
- 页眉槽位上 insertShape / insertConnector / insertOle / insertChart
- create 第四轮（序号 38）

## 手测

- 编号 **Xdh**，脚本 `manual-check/office/cases/xlsx-drawing-hlink.ps1`。
- Excel 16.0 手测 Xdh1–Xdh5 已过。
