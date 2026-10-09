# xlsx / pptx 嵌入 SVG

状态：✅ 文档、实现与 Excel / PowerPoint 16.0 手测 Xvg1–Xvg6、Pvg1–Pvg6 已过
日期：2026-10-09

对应 `roadmap/xlsx.md` 与 `roadmap/pptx.md` 跨格式 **C5**。前置：工作表图、幻灯片图、`.plan/2026年-10月-9日-✅-docx-嵌入SVG.md`。不要和 EMF / 3D / 墨迹捆在一起。不算渲染引擎，不把 SVG 原文塞进 inspect 树。

契约真源：`docs/json-contract.md`、`docs/json-contract-xlsx.md`、`docs/json-contract-pptx.md`、`docs/errors.md`、`docs/architecture.md`、`docs/opc.md`、`docs/checklist-xlsx.md`、`docs/checklist-pptx.md`。

request schema 的 `file` 仍是路径字符串。inspect 的 `contentType` 是出参，不进请求 schema。

## 目标

Excel / PowerPoint 工作表图、页眉图、幻灯片图、备注图、批注内图能按文件头写入 SVG，inspect 在图名单末尾带 `contentType: image/svg+xml`，`exportMedia` 写出 SVG 原字节。Office 16.0 打开无修复。docx 44 已过，不要改 Word 写入路径。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | Excel / PowerPoint 16.0 探测插入 SVG 后的 XML、关系、Content Type、Fallback | ✅ |
| 1 | 锁 inspect 字段与写入规则 | ✅ |
| 2 | 文档：主契约、两份分册、架构、opc、错误、清单 | ✅ |
| 3 | inspect / insertPicture / replacePicture / exportMedia / create | ✅ |
| 4 | 库测试与 Office 16.0 手测 | ✅ |

## 探测结论（Office 16.0）

`Shapes.AddPicture` 插入本地 SVG。COM `Type=28`（`msoGraphic`），不是普通图的 `13`。

Excel 另存与 Word 44 同一套：

- `/xl/media/image1.png` 加 `/xl/media/image2.svg`
- `Default Extension="png"` / `"svg"`
- `a:blip/@r:embed` 指向 PNG，`asvg:svgBlip` 指向 SVG
- URI `{96DAC541-7B7A-43D3-8B79-37D633B846F1}`
- 命名空间 `http://schemas.microsoft.com/office/drawing/2016/SVG/main`
- 样例 96×48 的 SVG，`a:ext` 为 `914400×457200` EMU（96 dpi），COM 72×36 磅

PowerPoint 另存可以没有 PNG：`a:blip` 不写 `r:embed`，只有 `svgBlip` 指向 `/ppt/media/image1.svg`。库写入仍走 Word / Excel 那条（1×1 PNG 占位加 `svgBlip`），手测确认 16.0 无修复。若 Repair 再改成 PPT 原样（无 PNG）。

## 已锁规则

- 文件头认 SVG：BOM / 空白之后能看到 `<svg` 标签。不认 SVGZ。
- 插入 / create：写 1×1 PNG 占位作 `a:blip` 后备，再写 SVG 原字节。两条 image 关系。xlsx 在 `/xl/media/`，pptx 在 `/ppt/media/`。
- `cx` / `cy` 都省时：根元素 `width` / `height`（无单位或 `px` 当 CSS 像素，96 dpi）；没有则 `viewBox`；再没有则 1 英寸见方。xlsx 仍受锚点规则约束。
- `replacePicture` 换成 SVG：保留已有 `a:blip` 后备图，只加或改 `svgBlip` 与 SVG 部件。
- `replacePicture` 换成位图：按原规则换 `a:blip` 媒体，去掉 `svgBlip`；SVG 部件无人引用则删。
- `deletePicture`：PNG 与 SVG 两条关系、两个媒体都按无人引用再删。
- `exportMedia`：有 `svgBlip` 时写出 SVG 原字节。没有则仍走 `a:blip`。PPT 原样文件没有 `a:blip/@r:embed` 时只走 `svgBlip`。
- inspect：图仍走现有 id。有 `svgBlip` 时名单末尾多一个 `contentType`，值为 `image/svg+xml`。位图图没有该字段。xlsx 插在 `stable` 之后。pptx 同样插在 `stable` 之后。
- 页眉图、备注图、批注内图复用同一套，不另开 op。
- 不改 docx 已过路径。

## 不做

- 把 SVG 渲染成真正的 Fallback 位图
- EMF / WMF / TIFF / 3D / 墨迹 / OMML
- 编辑 VML
- 改 docx 44

## 建议提交顺序

`0 → 1 → 2 → 3 → 4`。先改 `docs/`。探测脚本不要提交、不要进 `run.ps1`。不要强杀 Office。手测 COM 认 `Type=28`。
