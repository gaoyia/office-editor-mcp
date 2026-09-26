# docx create 带图

状态：✅ 已通过  
日期：2026-09-26

`create` 是唯一允许整包生成的动作。本期让新建文档能带 DrawingML 图。复用 `insertPicture` 的写入，不另做生成器。

前置：`.plan/2026年-9月-26日-✅-docx-正文图.md`。页眉插图已有。

## 目标

从领域 JSON 新建一份带图的 docx。正文必做。页眉、页脚带图一起做，避免以后再拆一次 create。

## 范围

| 序号 | 项 | 状态 |
| --- | --- | --- |
| 1 | 文档先行：`document.paragraphs` 与页眉页脚上的图字段、示例、清单 | ✅ |
| 2.1 | 正文段落声明 `pictures`：本地 `file`，可选 `cx`/`cy`/`kind`/`wrap`/位置 | ✅ |
| 2.2 | 写入 `/word/media/…`、主文档 `.rels`、内容类型、段末或按序 `w:drawing` | ✅ |
| 3.1 | `document.headers` / `document.footers`：本期允许 create 写默认页眉页脚（只 `default`） | ✅ |
| 3.2 | 页眉页脚段落同样能带 `pictures` | ✅ |
| 4 | 多张图、行内与浮动混排；`docPr/@id` 从 1 起在该部件内递增 | ✅ |
| 5 | 库测试：create 后再 inspect，图 id 与 `kind`/`wrap` 对 | ✅ |
| 6 | Word 16.0 打开 create 产物，无修复，图能看见 | ✅ |

建议提交顺序：`1 → 2.1 → 2.2 → 4 → 5 → 3.x → 6`。先正文，再页眉页脚。

## 契约要点

段落对象在现有 `text` / `style` 之外增加可选 `pictures` 数组。每项：

| 字段 | 含义 |
| --- | --- |
| `file` | 本地路径，与 apply 相同，按文件头认 PNG / JPEG / GIF / BMP |
| `cx` / `cy` | 可选，EMU；都省则按 96 dpi 换算 |
| `kind` | 默认 `inline`；`anchor` 时要 `wrap` |
| `wrap` | 与 `setPictureWrap` 同一套枚举 |
| `alignH` / `alignV` / `offsetX` / `offsetY` | 仅浮动，相对页边 |

不把 base64 写进 create JSON。多节、`first` / `even`、`titlePg` 仍不做。

文本框、图表、SmartArt 的 create 写在各自计划里，不要塞进本字段表。

## 不做

- 从已有文件当模板复制
- 嵌入 OLE、图标、SVG 转 DrawingML
- 生成 VML Fallback

## 验收

- 两段正文、第二段一张行内 PNG，Word 打开无修复；inspect 有 `para:…/pic:…`。
- 默认页眉一张浮动 `square`，Word 能看见。
- 缺 `file`、只给一个尺寸、xlsx 上带 `pictures` 键，报错类别对。
