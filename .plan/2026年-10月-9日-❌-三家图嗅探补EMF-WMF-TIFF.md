# 三家图嗅探补 EMF / WMF / TIFF

状态：❌ 探测已过，规则已锁，文档与代码未动
日期：2026-10-09

对应 `roadmap/docx.md`、`roadmap/xlsx.md`、`roadmap/pptx.md` 跨格式 **C6**。前置：正文图 / 工作表图 / 幻灯片图；C5 已过。不要和 3D / 墨迹 / OMML 捆在一起。不算渲染引擎。不认 SVGZ / WebP。不要改 docx 44 与 C5 的 `svgBlip` 写入路径。

契约真源：`docs/json-contract.md`、三份分册、`docs/errors.md`、`docs/architecture.md`、`docs/opc.md`、三份 checklist。

request schema 的 `file` 仍是路径字符串。inspect 的 `contentType` 是出参，不进请求 schema。

## 目标

三种格式的 `insertPicture` / `replacePicture` / create / `exportMedia` 按文件头认 EMF、WMF、TIFF，把原字节写入 media。Office 16.0 打开无修复。不把图栅格化成 PNG。不新开对象、不新开 op。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 0 | Word / Excel / PowerPoint 16.0 探测插入后的 XML、关系、Content Type | ✅ |
| 1 | 锁 sniff、扩展名、inspect `contentType` | ✅ |
| 2 | 文档：主契约、三份分册、错误、opc、清单 | ❌ |
| 3 | 三家 `classify_image` / 写入 / export | ❌ |
| 4 | 库测试与 Office 16.0 手测 | ❌ |

## 探测结论（Office 16.0）

`AddPicture`（Word 走 `InlineShapes`）插入本地文件后另存，**不转 PNG**。仍是一张 `a:blip`，没有 `svgBlip`。

| 文件头 | 部件 | Default | COM |
| --- | --- | --- | --- |
| EMF | `image1.emf` | `image/x-emf` | xlsx / pptx `Type=13`；Word 行内 `Type=3` |
| WMF | `image1.wmf` | `image/x-wmf` | 同上 |
| TIFF | `image1.tif`（不是 `.tiff`） | `image/tiff` | 同上 |

关系 Type 仍是 `…/image`。Excel 探测样例因 COM 传入 `72×36` 磅，`a:ext` 为 `914400×457200` EMU。Word 按文件像素：32×16 像素 → `304800×152400` EMU（96 dpi）。TIFF 的 `a:blip` 上 Office 会加 `a14:useLocalDpi`；库不必写。

## 已锁规则

- 文件头（扩展名只用来给新媒体取名）：
  - EMF：开头 `EMR_HEADER`（type=1），偏移 40 为 `" EMF"`（`0x464D4520`）
  - WMF：Aldus 头 `D7 CD C6 9A`；或非放置头 `mtType` 为 1/2、`mtHeaderSize=9`、`mtVersion` 为 `0x0100` / `0x0300`
  - TIFF：`II*\0` 或 `MM\0*`
- 不认 SVGZ、WebP。写出扩展名用 `.emf` / `.wmf` / `.tif`
- Content Type：`image/x-emf` / `image/x-wmf` / `image/tiff`
- 插入 / create / 换成这类图：把**原字节**写入该格式 media，一条 image 关系。`a:blip/@r:embed` 指向它。不要 PNG 占位，不要 `svgBlip`
- `cx` / `cy` 都省时：
  - EMF：优先用 `rclFrame`（单位 0.01 mm）换 EMU；读不到再用 `rclBounds` 当像素、96 dpi
  - 可放置 WMF：`(right-left)` / `(bottom-top)` 除以 Aldus `inch` 再乘 914400
  - TIFF：IFD `ImageWidth` / `ImageLength`，96 dpi；不读分辨率标签
  - 都读不到，或非放置 WMF 没有框：1 英寸见方
  - xlsx 仍受锚点规则约束；`twoCell` 仍要写 `a:ext`，显示尺寸仍由起止格决定
- `replacePicture` 换成 EMF / WMF / TIFF：按现有位图规则换 `a:blip` 媒体；若原来是 SVG，去掉 `svgBlip`，SVG 部件无人引用则删
- `replacePicture` 从这类图换成 SVG：走已过的 C5 / 44 规则，不要改 `svgBlip` 写入
- `deletePicture` / `exportMedia` 仍走现有图 op。`exportMedia` 写出原字节，`contentType` 取 `[Content_Types].xml`
- inspect：图仍走现有 id。EMF / WMF / TIFF 在 `stable` 之后带 `contentType`（与 SVG 同一位置）。PNG / JPEG / GIF / BMP 仍不加该字段。SVG 仍只在有 `svgBlip` 时带 `image/svg+xml`
- 页眉图、备注图、批注内图复用同一套，不另开 op
- 垃圾字节、WebP、SVGZ 仍报 `InvalidImageFile`（对外 `InvalidField`）
- 不改 docx 44、C5 的 SVG 写入（1×1 PNG 占位加 `svgBlip`）

## 不做

- 把 EMF / WMF / TIFF 栅格化成 PNG
- WebP / SVGZ
- 3D / 墨迹 / OMML
- 编辑 VML
- 改 docx 44 / C5 的 `svgBlip` 写入

## 建议提交顺序

`0 → 1 → 2 → 3 → 4`。先改 `docs/`。探测脚本不要提交、不要进 `run.ps1`。不要强杀 Office。手测脚本 `xlsx-emf.ps1` / `pptx-emf.ps1` / `docx-emf.ps1`，编号 **Xem** / **Pem** / **Dem**。COM：xlsx / pptx 认 `Type=13`，Word 行内认 `Type=3`。挂机从本份开始，下一份是 C7。
