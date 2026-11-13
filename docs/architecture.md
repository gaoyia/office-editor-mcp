# 架构

状态：xlsx、docx、pptx 第一期已实现，并已通过 Microsoft Office 16.0 人工打开。验收只认官方 Office，不再要求 WPS。
日期：2026-10-30

## 产品形态

产品名是 `ooxp`，全称 Office Open XML Patch。一个 Rust 库，外加一个命令行工具。库只认领域 JSON。命令行把 stdin 或文件里的领域 JSON 交给库，再把结果写到 stdout 或磁盘。

命令行另有 `ooxp mcp`。它从标准输入读 JSON-RPC，把七个动作登记成工具。七个动作是 `inspect`、`create`、`apply`、`exportMedia`、`exportChartData`、`layoutCheck`、`help`。
参数有两种形状：直接写领域 JSON，或只写 `requestPath` 指向一份领域 JSON 文件。命令行读完文件后，交给库的仍是领域 JSON。库不解析这一层信封。

不许为了信封去改领域 JSON。`requestPath` 不是领域字段。也不许把动作拆扁成一套 MCP 专用字段。

对齐 Transitional OOXML，也就是 Office 默认存出来的那种包。
ECMA-376 示例里常见的 Strict 命名空间（`http://purl.oclc.org/ooxml/...`）只作对照，create 不按 Strict 生成。
根关系里 `officeDocument` 的类型，或主部件根元素的命名空间，以 `http://purl.oclc.org/ooxml/` 开头，就是 Strict 包。
inspect、exportMedia、exportChartData 只读，照常执行。
apply 在执行第一条 op 之前拒绝，报 `NotImplemented`，不写文件。
请先用 Office 另存为 Transitional。本程序不实现 Strict 写入。

## 三个动作

这三个动作是整套设计的地基，实现时不许改语义。

| 动作 | 真源 | 做什么 |
| --- | --- | --- |
| `inspect` | 已有 ZIP 包里的 XML | 返回带稳定 id 的精简 JSON 树，按调用方给的范围截断 |
| `apply` | 已有 ZIP 包里的 XML | 按 id 打补丁，只改目标节点，其余部件按原字节写回 |
| `create` | 领域 JSON | 从 JSON 生成一份新的完整包。这是唯一允许整包生成的动作 |

禁止把 inspect 树当成完整文档模型，改完之后整包重新生成。apply 的入参只能是操作列表，不能是整棵树。

本工具不支持预览，也不把页面画成图。库不调用 Office 或 WPS。机器上有其中之一时，调用方可以自己导出后再查看：

- Word、Excel，以及 WPS 的 `kwps`、`ket`，用 `ExportAsFixedFormat` 写成 PDF
- PowerPoint 和 WPS 的 `kwpp`，用幻灯片的 `Export` 写成 PNG

怎么查到这句话，见 [json-contract.md](json-contract.md) 的 `help`。

xlsx 另有只读动作 `exportMedia`：按图 id 把媒体部件原字节写到本地。
xlsx 的 `exportMedia` 认工作表图、页眉页脚图、OLE、3D 模型与工作表墨迹。图写出 `/xl/media/…`，OLE 写出 `/xl/embeddings/…`，3D 写出 GLB，墨迹写出 `/xl/ink/…`。
pptx 的 `exportMedia` 认幻灯片图、备注图、批注内图、母版 / 版式图、视频、音频、OLE 与 3D。图、视频和音频写出 `/ppt/media/…`，OLE 写出 `/ppt/embeddings/…`，3D 写出 GLB。
docx 的 `exportMedia` 认图、OLE、墨迹画布、3D 与嵌入字体。图写出 `/word/media/…`，OLE 写出 embeddings，墨迹写出 `/word/ink/…`，3D 写出 GLB，`font:{n}` 写出 `/word/fonts/…`。
docx 与 pptx 另有 `exportChartData`：按图表 id 把 embedding 工作簿原字节写到本地。它们不改上述三个动作的语义，也不把字节塞进 inspect 树。
xlsx 对 `exportChartData` 仍报 `InvalidField`（图表数据就在本工作簿单元格里）。

## 只读检查与试运行

`layoutCheck` 是第七个动作，只读几何。它按 `path` 打开包，读每页形状的包围盒，返回越界、重叠与文字溢出估算。
不写文件、不生成图片、不调用 Office、不按新画布重排。它不进 `opc` 的写回路径，也不改 `inspect` / `apply` / `create` 的语义。
本期只对 pptx 实现，xlsx / docx 报 `InvalidField`。

`apply` 的 `dryRun` 是可选布尔字段。设为 `true` 时，各条 op 仍按顺序在内存树上执行，但不调 `save_to_vec`，也不调 `write_atomic`。
成功时返回本批标脏的部件名，不写任何文件。任一 op 失败仍按现有路径提前返回，内存树丢弃，源包不变。
现有实现已经是全批原子，不新增事务或回滚类型。

## 模块

库和命令行是两个 crate。浏览器演示另有 `ooxp-wasm`，只包门口，不增加动作。xlsx、docx、pptx 已落地。信封解析在 `json`，按 `format` 分发在 `dispatch`，每种格式自己解析自己的 `scope` 与 `ops`。

```text
crates/ooxp/                 库
  error/                     错误身份、错误码、按语言渲染
    messages/                en.rs、zh_cn.rs；每种语言一个穷尽 match
  opc/                       ZIP、内容类型、关系、按路径读写部件
  xml.rs                     元素树
  json.rs                    领域 JSON 信封
  op_fields.rs               各格式 request 共用的字段读取
  dispatch.rs                按 format 分发
  help.rs                    help 动作
  host_env.rs                时间、进程号、随机数
  host_files.rs              用户文件字节
  props/                     core / app / custom
  password.rs                保护口令哈希
  theme/                     DrawingML theme 的解析、inspect 与补丁
  chart/                     c:chart 与 chartex
  smartart/                  图示数据与四件套
  ole.rs                     嵌入对象分类与包装
  model3d.rs                 GLB 与 am3d
  image.rs                   EMF / WMF / TIFF，以及 PNG / JPEG / GIF / BMP 尺寸
  ink.rs                     InkML 校验、常量和写媒体
  svg.rs                     SVG 扩展
  drawing_text.rs            DrawingML 文本格式
  drawing_math.rs            a14 公式框
  drawing_geom.rs            预设几何名单
  shape_runs.rs              形状可见文字的切分
  text_span.rs               在可见文字里定位
  xlsx/                      工作簿
  docx/                      文档
  pptx/                      演示文稿
crates/ooxp-cli/             命令行，只依赖 ooxp
crates/ooxp-wasm/            浏览器绑定，只调用 run_bytes
demo/web/                    临时演示页
```

每个名字都是 `crates/ooxp/src` 下的文件或目录。`json` 与 `dispatch` 是单文件，不是目录。

- `error` 管错误身份、错误码和中英文文案。各模块可以调用它。它不反向依赖 `json`、`opc` 或任何格式模块。
- `opc` 管 ZIP、`[Content_Types].xml` 和各级 `.rels`。格式模块、`props`、`ole` 通过它读写部件。
- `xml` 管元素树的解析、遍历、写回和堆上释放，以及真假属性的两种读法。改 XML 的模块调用它。它不依赖格式模块。
- `json` 管领域信封，并记下每个动作允许哪些字段。`dispatch`、`help` 和各格式的 `request/` 调用它。它不依赖格式模块。
- `op_fields` 读取各格式 request 里相同的字段：布尔、id、文件名、EMU、颜色和未知字段。格式的 `request/` 调用它。它只依赖 `error`，不依赖任何格式模块。
- `dispatch` 是库门口。命令行调用 `run` / `run_bytes`。它按信封字段表拒绝未知字段，再按格式分发。它调用 `json`、`help`、`host_env`、`host_files` 和各格式模块。
- `help` 从拼好的 schema 里查动作和操作，一次一层。只有 `dispatch` 的 help 动作调用它。
- `host_env` 提供时间、进程号和随机数，并生成 GUID 和批注的时间字符串。`dispatch` 在本机 `run` 里装入系统实现。需要这些值的模块调用它。
- `host_files` 按路径提供用户文件字节。`dispatch` 装入读取回调。插图、OLE、媒体等模块调用它。
- `props` 管 core / app / custom。三种格式调用它。它可以调用 `opc` 与 `json`，不依赖格式模块。
- `password` 算保护口令哈希，不破解密码。xlsx、docx、pptx 的保护模块调用它。
- `theme` 负责主题 op 的字段解析、inspect 与 XML 补丁。格式模块只交给它 theme 部件名、themeOverride 部件名和宿主 id 的写法。三种格式的主题模块调用它。它不把 `phClr` 或 `schemeClr` 展开成 RGB。
- `chart` 管 `c:chart` 与 chartex，图表 op 的字段体也在这里。它可以调用 `xlsx` 读写 embedding。xlsx、docx、pptx 调用它。pptx 不直接引用 `xlsx` 或 `docx`。
- `smartart` 管图示数据和四件套，不含宿主 graphicFrame。三种格式的 SmartArt 模块调用它。
- `ole` 做嵌入对象分类和 Package 包装，不执行 OLE 服务器。它可以调用 `opc`。三种格式调用它，彼此不引用。
- `model3d` 认 GLB，读写 `am3d` 的相机、旋转和光源，并写出 GLB 与后备图。三种格式的 3D 模块调用它。媒体目录由调用方传入。
- `image` 认 EMF、WMF、TIFF，以及 PNG、JPEG、GIF、BMP 的尺寸，并把调用方给的图读成 `ImageFile`。认图的格式模块调用它。SVG 由 `svg` 认，`image` 只放 PNG 占位，原件放进 `ImageFile.svg`。
- `ink` 放 InkML 常量、校验和写媒体。AlternateContent 外壳留在各格式。三种格式的墨迹模块调用它。
- `svg` 管 Office 2016 的 SVG 扩展。xlsx、docx、pptx 调用它。
- `drawing_text` 读写 `a:rPr`、`a:pPr`、`a:bodyPr`。xlsx、pptx 的形状和表，以及 `shape_runs` 调用它。
- `drawing_math` 判定 `a14:m`，拼接公式可见文字，并按 id 找到公式框。xlsx 和 pptx 的公式框调用它。docx 公式走自己的 OMML。
- `drawing_geom` 只放三种格式都认的预设几何名单。形状名单不含连接线。连接线名单含 `line`。三种格式的形状和连接线解析调用它。
- `shape_runs` 计划形状可见文字怎么切。它调用 `drawing_text` 和 `text_span`。pptx 的形状写入调用它。
- `text_span` 在一段可见文字里按原文定位。`shape_runs` 和 docx 段落调用它。
- `xlsx`、`docx`、`pptx` 做各自的 inspect、apply、create。它们调用 `opc`、`json`、`xml`、`props`，以及上面这些共享模块。
- 命令行只依赖库的门面。禁止格式模块绕过 `opc` 直接改 ZIP。`SetCellOp`、工作表 `scope`、`setParaText` 放在对应格式模块里。

库门口是 `run_bytes`。

- 调用方交出领域 JSON、已经读好的包字节，以及 JSON 里 `file`、`sound` 指向的文件字节。`help` 和 `create` 不需要包。
- 库返回树，或返回要落盘的字节。它不打开 Office 文件，也不按路径去读图片、音视频、OLE、墨迹、3D 模型或嵌入字体。
- 随机数、当前时间和进程号由调用方传入。本机的 `run` 每次仍问系统，所以拼出来的 id 和以前一样。
- 没有传入时，本机编译用系统的这三项。`wasm32` 上没有系统来源。
- 本机命令行和测试走 `run`。`run` 先把包和用户文件读进内存，再调用 `run_bytes`，最后把结果写回磁盘。`run` 不编进 `wasm32`。
- 领域 JSON 的字段不变。路径仍由本机这一层解释。

这些装入是线程局部的。

- 同一线程嵌套另一次调用时，内层压在外层上面，结束时只弹自己的一层。
- 栈空时与没有装入相同：没有读取回调就读不到用户文件，时间和随机数走系统。
- `wasm32` 上库不自己读磁盘。
- 浏览器演示的绑定在 `crates/ooxp-wasm`，页面在 `demo/web`。页面按格式列出创建和修改示例，把字节交给 `run_bytes`，并装入时间和随机数。结果只写入授权目录，并标出这次新增或改过的文件。浏览器没有目录选择接口时，页面提示更换浏览器。细节见 [wasm-demo.md](wasm-demo.md)。它不增加领域 JSON 的动作。

每种格式的 `scope` 与 `ops` 解析放在该格式的 `request/` 目录。
`parse_scope` 与 `parse_ops` 仍是入口；类型和解析函数按领域拆到子文件。
对外仍从 `request` 再导出，apply / inspect / create 继续写 `use super::request::…`，路径不变。

`apply.rs` 只打开包、按 op 分发、写回脏部件。
某个对象已经有模块时，这个 op 的函数体放在那个模块。
多个对象都要用的部件装载留在 `apply.rs`，不在每个模块各抄一份。docx 已按此拆开。
xlsx 的单元格、公式和 `sheetData` 读写在 `xlsx/cell.rs`。排序颜色编进样式表的那段在 `xlsx/view.rs`。
pptx 的每个 op 在对象模块里做完。`apply.rs` 只分发，并留下大家共用的宿主装载。
docx 的 create 字段解析在 `request/` 里按领域拆开，和 apply 的解析放在一起。`create.rs` 只拼骨架并复用已有 apply。骨架 XML 在 `create_xml.rs`。
xlsx 的 create 也按这个拆：字段解析在 `request/`，骨架 XML 在 `create_xml.rs`。
xlsx 切片器在 `slicer/` 目录。drawing 框、视图属性和缓存命名共用。表格切片器、透视切片器和时间线各一份。
pptx 的 create 也按这个拆：字段解析在 `request/`，骨架 XML 在 `create_xml.rs`。

### opc

把 `.docx`、`.xlsx`、`.pptx` 都当成 OPC 包。负责打开、列出部件、按路径读字节、标记脏部件、保存。
保存时没改过的部件保持原来的压缩方法和未压缩字节，不重排 ZIP 中央目录里未改条目的相对顺序，也不去美化无关 XML。细节见 [opc.md](opc.md)。

### 元素树

元素树不设层数上限。解析、遍历、写回和按子元素查找都用堆上的栈。

释放也走堆上的栈，不靠 Rust 默认的递归析构。`Element` 和 `Document` 的 `Drop` 都把子节点收到堆上的栈再拆。要拿走 `children`，用 `take_children`，不能直接移出字段。`Document` 的 `Drop` 和 `take_root` 用 `mem::replace` 换出根，没有 `unsafe`。

只在一次调用的内存里挂在元素上的属性，名字以 `ooxp:` 开头，写回时丢掉，不进文件。
一个族在 apply 开始、第一次装载部件时按文档顺序编号一次，属性名是 `ooxp:pin` 或 `ooxp:pin-族名`。之后按这个编号找元素。编号为空表示本批新建，不占旧编号。编号不把部件标脏。
docx 段落用 `ooxp:pin`。文本框、脚注、尾注、批注各自从 1 起。
docx 表用 `ooxp:pin-tbl`，行用 `ooxp:pin-tr`，格的起始列用 `ooxp:pin-col`。文本框里的表各自从 1 起。
docx 段内的书签、域、链接、公式、拼音、内容控件、编辑区、修订和 run 各用自己的 `ooxp:pin-族名`。
书签、域、链接、公式、拼音和内容控件在故事里从 1 起，文本框里再从 1 起。
编辑区和修订在正文里含文本框；脚注、尾注和批注里的修订各自从 1 起。
run 在各段里从 1 起。
图、文本框、连接线、形状、墨迹、图表和三维模型的位置退路在各段里从 1 起。页眉页脚里的图、SmartArt、OLE，以及页眉水印，在整份部件里从 1 起。正文里的 SmartArt 和 OLE 仍在各段里从 1 起。
docx 节用 `ooxp:pin-section`。目录、题注、交叉引用、索引、索引项、引文和书目域各用自己的 `ooxp:pin-族名`。页眉页脚容器在 apply 开始时记下部件名，之后按部件名找回原来的编号。书目源、构建基块和嵌入字体在各自部件里按元素顺序编号。
xlsx 工作表内的数据验证、条件格式、超链接、迷你图、保护区域和表部件，以及工作簿的定义名、自定义视图、单元格样式、外部引用和没有 id 的连接，各用自己的 `ooxp:pin-族名`。数据透视表在 apply 开始时记下关系 id。
drawing 没有 `cNvPr` id 时，图、图表、文本框、形状、连接线、公式、SmartArt、三维模型、墨迹、切片器和页眉页脚图各按自己的序号记下。图表系列、趋势线和误差线在图表部件里按开始时的顺序编号。
pptx 母版和版式按开始时的 `sldMasterId/@id`、`sldLayoutId/@id` 认回序号。节和自定义放映在演示文稿上编号。页上没有 `cNvPr` id 的对象、链接、域和表的行列，在第一次读到该页时编号。

注释和处理指令收成原样节点，写回时按解析到的字节拼回去。`text_content` 不收它们。

`DOCTYPE` 不保留。ECMA-376 Part 2 的 M1.18（§8.1.4）要求把它当成错误。解析时返回 `XmlBroken`。

写回时由本库转义，再以已转义的字节交给 quick-xml 0.37.5，避免它再转一遍：

- `(&[u8], &[u8])` 转成属性，以及 `BytesText::from_escaped`，都不再转义。
- `(&str, &str)` 和 `BytesText::new` 会按 `<`、`>`、`&`、`'`、`"` 再转一次。
- 属性值在这五个字符之外，把 TAB、LF、CR 写成 `&#9;`、`&#10;`、`&#13;`。
  XML 1.0 §3.3.3 会把属性里这三个原样字符规整成空格。
- 文本在这五个字符之外，把 CR 写成 `&#13;`。
  XML 1.0 §2.11 会把文本里的 CR 规整成 LF。TAB 和 LF 留在文本里。
- 字符串拼接用的 `escape_attr` 补上 TAB、LF、CR，`escape_text` 补上 CR。这两处仍不转义 `'`。

### 标记兼容性

读取 `mc:AlternateContent` 时，全库用 `xml::alternate_child` 决定进入哪一支。create 写出的 `AlternateContent` 形状不变。
新写的 3D 外壳和 docx chartex 外壳用 `xml::alternate_content`：前缀在 `Choice` 上声明，不给 Fallback 时写空的 `mc:Fallback`。

`Choice` 的 `Requires` 按空白拆成前缀。每个前缀都在下表里，才进入这支。有一个不在表里，就跳过这支。多个 `Choice` 按文档顺序取第一支全部在表里的。都没有，就进入 `Fallback`。没有 `Fallback` 就不进入这段。

| 前缀 | 用在 |
| --- | --- |
| `a14` | 公式，以及部分 OLE 锚点 |
| `am3d` | 3D 模型 |
| `p14` | 幻灯片墨迹，以及一部分切换 |
| `p15` | 较新的切换 |
| `pslz` | 幻灯片缩放 |
| `psez` | 节缩放 |
| `psuz` | 摘要缩放 |
| `sle15` | 切片器 |
| `tsle` | 时间线 |
| `v` | 幻灯片 OLE |
| `wpc` | 墨迹画布 |
| `wpg` | 组合 |
| `wps` | 文本框、形状、连接线 |
| `x14` | 工作表 OLE |
| `xdr14` | 工作表墨迹 |

`Requires` 缺了、拆完没有前缀、或前缀名不合法，返回 `XmlBroken`，不改走另一支。

命名空间声明常常在祖先上。这个函数只拿到 `AlternateContent` 这一元素，看不见祖先。本元素或当前 `Choice` 上若写了 `xmlns:前缀` 且值为空，返回 `XmlBroken`。没写这条声明时，不因此判损坏。

返回 `Result` 的遍历用 `?` 把这个错误交上去。返回 `Option` 或 `()` 的遍历把 `Err` 当成没有可进的支，跳过这段。

文本框内容、组合、OLE 各自原有的跳过规则仍由各遍历决定。

### json

只处理领域信封：`schemaVersion`、`action`、各动作的必填字段。每个动作允许哪些字段记在 `envelope_fields`。`dispatch` 按这张表拒绝未知字段。`ooxp mcp` 按这张表列出工具参数名，字段说明仍写在命令行。不认识的字段直接报错，避免默默丢数据。
契约见 [json-contract.md](json-contract.md)，xlsx / docx / pptx 分册见同目录 `json-contract-xlsx.md` 等。

### xlsx

读 `xl/workbook.xml` 拿工作表名单和关系。`dialogsheet` 与 `macrosheet`（含 Type 以 `/xlIntlMacrosheet` 结尾的国际宏表）进 `sheets`，字段只有 `id`、`name`、`kind`、`hidden`、`hasHeaderFooter`。
给了 `scope.headerFooter` 时，这两类表再跟 `headerFooters`。
不填单元格、图、保护、打印、视图。apply 仍拒写这两类表（`setHeaderFooterText` 对 `dialogsheet` 仍可改文本）。create 不生成这两类。
`format` 认 `.xlsx` 与 `.xlsm`，inspect 树的 `format` 仍为 `xlsx`。不执行宏。
按范围读对应 `sheetN.xml`，需要时再读 `xl/sharedStrings.xml`。工作簿与每张 `worksheet` 始终带 `pictureCount`。每张 `worksheet` 始终带 `background`（没有则为 `null`）。
工作簿、每张 `worksheet` 与每张 `chartsheet` 始终带 `chartCount`。每张表始终带 `hasHeaderFooter`。
每张 `worksheet` 与 `chartsheet` 始终带 `print` 与 `view`。每张 `worksheet` 始终带 `autoFilter`。每张 `worksheet` 始终带 `sortState`。
每张 `worksheet` 始终带 `outline`。
每张 `worksheet` 始终带 `commentCount`（只计无 `parentId` 的顶层线程化批注）、`hyperlinkCount`（该表 `<hyperlinks>` 里 `<hyperlink>` 的条数）与 `sparklineCount`（该表 `x14:sparklineGroup` 的组数）。
同一张表还带 `oleObjectCount`、`model3dCount`、`inkCount`（一笔一个对象）、`slicerCount`（含时间线）与 `protectedRangeCount`。
工作簿与每张 `worksheet` 始终带 `textBoxCount`、`mathCount`、`shapeCount`、`connectorCount` 与 `smartArtCount`。
每张 `worksheet` 与 `chartsheet` 始终带 `sheetProtection`。
工作簿根始终带各表之和的 `commentCount`、`hyperlinkCount`、`sparklineCount`、`oleObjectCount`、`model3dCount`、`inkCount` 与 `slicerCount`。
根上还带 `connectionCount`、`externalLinkCount`、`customViewCount`、`workbookProtection`、`theme`、`themeOverride` 与 `props`（core / app / custom，规则见契约「文档属性」）。
`scope.comments` 为 true 时列出该表线程化批注名单，不读作者不以 `tc=` 开头的旧式 VML 批注。
`scope.hyperlinks` 为 true 时列出该表 `<hyperlinks>`，不把 `HYPERLINK()` 公式或图上的链算进来。
`scope.sparklines` 为 true 时列出该表 `x14:sparklineGroups`，数据范围当字符串，不把折线画进树。
`scope.oleObjects` 为 true 时列出该表 `<oleObjects>`，不把嵌入字节或预览图放进树。
`scope.model3ds` 为 true 时列出该表 drawing 里的 `am3d` AlternateContent，不把 GLB 或海报放进树。
`scope.inks` 为 true 时列出该表 drawing 里 `Requires="xdr14"` 的 `xdr:contentPart`，不把 InkML 或海报放进树。Fallback 海报不进 `pictures`。
`scope.maths` 为 true 时列出该表 drawing 里的 `a14:m` 公式框，不把 OMML 原文放进树，也不进 `textBoxes`。
`scope.slicers` 为 true 时列出该表 drawing 里的 slicer 与时间线 `graphicFrame`，已勾选进 `selected`，时间线区间进 `filter`，不把未选项名单摊开。
`scope.protectedRanges` 为 true 时列出该表 `<protectedRanges>`，不把哈希放进树。
`scope.connections` 为 true 时列出工作簿 `/xl/connections.xml`，不刷新。
`scope.externalLinks` 为 true 时列出 workbook `<externalReferences>`，不打开目标文件。
`scope.pictures` 为 true 时读该表 `<drawing>` 指向的 drawing 部件，列出 `xdr:pic`，不读媒体字节。
`scope.charts` 为 true 时列出 `xdr:graphicFrame` 里的图表，系列一次倒出；数值优先读 `c:f` 指向的单元格。
`scope.textBoxes` 为 true 时列出 `xdr:sp` 且 `txBox` 的文本框，可见字取 `xdr:txBody`。`scope.shapes` 为 true 时列出无 `txBox` 的 `xdr:sp`。
`scope.connectors` 为 true 时列出 `xdr:cxnSp`。
`scope.smartArt` 为 true 时列出 diagram URI 的 `xdr:graphicFrame`；四条关系从 drawing `.rels` 解到 `/xl/diagrams/`。
`scope.headerFooter` 为 true 时展开六个页眉页脚槽位的原文；同时 `pictures: true` 时再列出 `drawingHF` 里的图。
apply 可改目标单元格，也可换、插、删工作表图，并可改 `oneCell` / `absolute` 的尺寸与锚点。
也可插、删、改工作表图表：改系列时写本表单元格并回写 cache（连续 A1、能解析成连续 A1 的命名区域、以及长度对得上的不连续并集），插图时在 `dataFrom` 写下数据表。也可增删趋势线、误差线，并把系列挂到次轴。
也可新建或删除图表工作表：数据仍写在点名的普通表上，图表工作表本身没有 `sheetData`。也可插入、删除、改名、重排与隐藏普通工作表，脏 `workbook.xml` 的 `sheets` 与对应 worksheet 部件。
对 `dialogsheet` / `macrosheet` 这些 op 仍报 `NotImplemented`。
也可用 `setCellStyle` 写单元格数字格式、字体、填充、边框与对齐：复用或追加 `styles.xml` 里的 `xf`，只改被点名格子的 `s`。边框含对角线，对齐可写缩进、旋转与缩小字体。
也可用 `insertCellStyle` / `deleteCellStyle` / `setCellNamedStyle` 读写 `cellStyles`：insert 写入 `cellStyleXfs` 再挂名，套用时把格子的 `s` 指到引用该 `xfId` 的 `cellXfs`。不改主题色板。
也可用 `setThemeColors` / `setThemeFonts` / `setThemeFmt` / `setThemeExtraSchemes` / `setThemeObjectDefaults` / `setThemeOverride` 读写工作簿 theme / themeOverride 部件，不把格子上的 theme 指数算成 RGB。
也可插、删、改工作表单元格超链接：脏该表 XML 的 `<hyperlinks>` 与工作表 `.rels`（Type 以 `/hyperlink` 结尾、`TargetMode` 为 `External`），不标重算，不改单元格值。
也可插、删、改图 / 文本框 / 形状 / SmartArt 上的 DrawingML 链：脏 drawing 的 `a:hlinkClick` 与 drawing `.rels`。
也可按槽位改页眉页脚原文，并可对 `worksheet` 与 `chartsheet` 的页眉槽位插、换、删 DrawingML 图。
也可往图表工作表的 Chart Drawing（`c:userShapes`）插图、形状、文本框与连接线，锚点用 0–1 分数。
也可插、删、改工作表文本框（`xdr:sp` `txBox`）、无 `txBox` 的形状、连接线（`xdr:cxnSp`）与 SmartArt（diagram `graphicFrame`）：改字只动 `xdr:txBody` 或 data 部件可见点；文字格式走 `setShapeFormat`；几何走锚点。
也可插改删工作表 drawing 公式框（`a14:m`），并可改已有公式框的锚点与尺寸；`setMath` 仍只改字。不进 `textBoxes`。
插 SmartArt 再新建 `/xl/diagrams/` 四件套，关系写在 drawing `.rels`。`insertChartSheet` 再新建 `/xl/chartsheets/sheetN.xml`。
往页眉插图可能新建另一份 drawing 并写 `<drawingHF>`。改单元格值或公式之后，把工作簿标成打开时需要重算，必要时删掉 `xl/calcChain.xml`。
`setCellStyle` 可改 `styles.xml`，不求值公式。也可用 `setColWidth` / `setRowHeight` / `mergeRange` / `unmergeRange` 写 `cols`、`row/@ht` 与 `mergeCells`：只改被点名的列、行与合并区，不算最适列宽，不按列宽反推 `twoCell` 图尺寸。
也可改共享公式主格（保住整组或拆成独立公式）并读写数组公式 `t="array"`，并可新建数据表或改已有主格的 `TABLE` 输入格。单元格公式、数据表和本机外部引用里库能算的写入缓存。不改已有数据表区域的大小。
也可用 `insertDataValidation` / `setDataValidation` / `deleteDataValidation` 读写工作表 `dataValidations`：公式当字符串，不求值、不展开下拉列表；`extLst` 里的扩展校验原样留下。
也可用 `insertConditionalFormat` / `setConditionalFormat` / `deleteConditionalFormat` 读写工作表 `conditionalFormatting`。公式当字符串，颜色写入 `styles.xml` 的 `dxf`，并可写 `top10`、`aboveAverage` 与文本类规则。
不渲染、不求值。`extLst` 里的扩展条件格式原样留下。
不写 embedding，不对 xlsx 做 `exportChartData`。`setFormControl` 同时改 drawing 正文里的标题、VML 文本框，以及 ctrlProp 的链接、列表、勾选和选中项。不编辑 `legacyDrawingHF`。不把窗体当成只有 VML。
insert / delete 线程化批注时只追加或去掉对应 Note `v:shape` 与占位 `comments.xml`，不改已有框的几何。
也可插、删、回复、改字、改作者与完成标记线程化批注（MS-XLSX `threadedComment`），脏 threadedComments、persons、占位 comments、VML、工作表 `legacyDrawing`、关系与内容类型。
也可插、删、改工作表迷你图组：脏该表 `extLst` 里的 `x14:sparklineGroups`，可改颜色、标记与坐标轴，不新建部件，不标重算。
也可用 `setProtection` 开关工作表 `sheetProtection` 与工作簿 `workbookProtection`，脏该表 XML 或 `workbook.xml`。
无密码时不写哈希。给了 `password` 则写 Excel 16.0 的 SHA-512 哈希，不新建部件，不标重算。
也可用 `insertProtectedRange` / `setProtectedRange` / `deleteProtectedRange` 写工作表 `<protectedRanges>`：只改 `worksheet`，不标重算，不新建部件。
create 可写图、图表、图表工作表、页眉页脚、批注、超链接、文本框、公式框、形状、连接线、SmartArt 与 3D 模型，也可按已有 apply 写出列宽、行高、合并、数据验证、条件格式、表格、透视、定义名、打印、视图、筛选、排序、分组、隐藏行列、拆分、自定义视图、迷你图、OLE、切片器、连接、外部引用、保护、允许编辑区域、组合，以及单元格样式与共享 / 数组公式。
给了 `workbook.theme` 时写 theme 部件。
可新建数据表公式。细则见 create。
create 到 `.xlsm` 时 workbook Override 用 macroEnabled Content Type，不建空的 `vbaProject.bin`。

### docx

主文档由根关系里 `/officeDocument` 指向，常见路径 `/word/document.xml`。
inspect 读该部件里的 `w:p`，返回大纲和按范围截断的段落；并始终列出页眉、页脚容器、各节页面与页眉页脚属性，以及每节是否链接到前一节。
文档根始终带正文 `pictureCount`、`textBoxCount`、`connectorCount`、`shapeCount`、`inkCanvasCount`、`footnoteCount`、`endnoteCount`、`commentCount` 与 `chartCount`。
根上还带 `smartArtCount`、`oleObjectCount`、`model3dCount`、`tableCount`、`revisionCount`、`hyperlinkCount`、`bookmarkCount`、`fieldCount` 与 `tocCount`。
根上还带 `captionCount`、`crossRefCount`、`indexCount`、`indexEntryCount`、`sourceCount`、`citationCount` 与 `bibliographyCount`。
根上还带 `docPartCount`、`mathCount`、`contentControlCount`、`numberingCount`、`styleCount` 与 `embeddedFontCount`。
根上还带整篇 `mirrorMargins` / `gutterAtTop`、`props`、`protection`、`theme`、`themeOverride`、`fontEmbed` 与 `pageBackground`。
页眉页脚容器始终带 `pictureCount`、`textBoxCount`、`connectorCount`、`shapeCount`、`inkCanvasCount`、`chartCount` 与 `smartArtCount`。
页眉页脚容器还带 `oleObjectCount`、`model3dCount`、`tableCount`、`revisionCount`、`hyperlinkCount`、`bookmarkCount`、`fieldCount`、`mathCount` 与 `contentControlCount`。
`scope.pictures` / `scope.textBoxes` / `scope.connectors` / `scope.shapes` / `scope.inkCanvases` / `scope.model3ds` 为 true 时列出页眉页脚图、框、连接线、无框形状、墨迹画布或 3D 模型；同时给了正文切片则段落上也列出。
组合本身不进树，组合里的图 / 框 / 连接线 / 无框形状 / 墨迹画布展开。`scope.textBoxContent` 再展开框内段落。`scope.tables` 为 true 时框节点带框内表名单。对应 `scope` 为 true 时框节点还带框内 `w:hyperlink`、书签、域、公式、拼音与内容控件名单。
`scope.notes` 为 true 时列出用户脚注、尾注与分隔符，注内段落一次倒出。
`scope.comments` 为 true 时列出批注；批注内段落一次倒出。`scope.charts` 为 true 时列出正文、页眉页脚、文本框、脚注、尾注与批注里的嵌入图表；系列一次倒出。
`scope.smartArt` 为 true 时列出正文、页眉页脚、文本框、脚注、尾注与批注里的 SmartArt；可见点一次倒出。`scope.oleObjects` 为 true 时列出正文、页眉页脚、文本框、脚注、尾注与批注里的嵌入对象。
`scope.tables` 为 true 时列出正文与页眉页脚里的表；单元格段落一次倒出。`scope.revisions` 为 true 时列出正文（含文本框）、页眉页脚、脚注、尾注与批注里的 run 级插入、删除与属性修订。
`scope.hyperlinks` 为 true 时列出正文、页眉页脚与文本框里的 `w:hyperlink`。`scope.bookmarks` 为 true 时列出正文、页眉页脚与文本框里的配对书签。
`scope.fields` 为 true 时列出正文、页眉页脚与文本框里的 `w:fldChar` / `w:fldSimple`。`scope.tocs` 为 true 时列出正文、页眉页脚、文本框、脚注、尾注与批注里的目录块。
`scope.captions` 为 true 时列出同样范围的题注。`scope.crossRefs` 为 true 时列出同样范围的交叉引用。`scope.indexes` 为 true 时列出同样范围的 INDEX 块。
`scope.indexEntries` 为 true 时列出同样范围的 XE 索引项。`scope.sources` 为 true 时列出书目源。`scope.citations` 为 true 时列出同样范围的 CITATION。
`scope.bibliographies` 为 true 时列出主文档 BIBLIOGRAPHY 块。`scope.docParts` 为 true 时列出 glossary 构建基块。
`scope.embeddedFonts` 为 true 时列出 fontTable 里的嵌入字体面。`scope.maths` 为 true 时列出正文、页眉页脚与文本框里的顶层 `m:oMath`，名单带 `expr`。
`scope.contentControls` 为 true 时列出正文、页眉页脚与文本框里被收入的 `w:sdt`。`scope.numbering` 为 true 时列出 `numbering.xml` 里的编号实例。
`scope.styles` 为 true 时列出 `styles.xml` 里的样式定义。`scope.runs` 为 true 时已列出的段落带 run 名单。
apply 可改被点名段落的可见文本（主文档、页眉页脚、框内、注内或批注内），可在主文档、页眉页脚与表格单元格插删 `w:p`，可插删 `w:tbl` 以及行与列（含文本框里的表，不含注内 / 批注内）。
apply 可按 `gridSpan` / `vMerge` 合并与拆分，可接受或拒绝被点名的修订，可插、删、改超链接地址与显示文本（含文本框里的 `w:hyperlink`）。
apply 可给图、文本框、连接线、无框形状、SmartArt、图表、墨迹画布与 3D 写 DrawingML `a:hlinkClick`（含框内图、注内 / 批注内的图、框或 3D；OLE 除外；不写 `a:hlinkHover`）。
apply 可插、删、改书签名（含框内），可插、改指令、删域（不求值，含框内），可写入编号定义并把段落套上或去掉编号。
apply 可增删自定义样式，可给段落换 `pStyle`，可给 run 套字符样式。
apply 可改被点名段落、字符或表格样式的字体、字号、加粗、颜色与段前段后。字符样式不能写段距。
apply 可改表的 `tblStyle`、`tblLook`、浮动 `tblpPr` 与表级 `tblCellMar`。
单元格还可改底纹、格边距、斜线、格级 `vAlign`、不换行、文字方向、适应文字、隐藏标记与表头引用。
段上可改直接对齐、缩进与间距。被点名 run 可改加粗、斜体、下划线、字号、颜色与字体。
apply 可改节上 `w:pgMar` / `w:pgSz` / `w:cols` / `w:vAlign` / `w:lnNumType` / `w:docGrid` / `w:rtlGutter` / `w:pgBorders`。
apply 可改整篇 `w:mirrorMargins` / `w:gutterAtTop`，以及整篇 `w:background` 与 `w:displayBackgroundShape`。
apply 可改节上页眉/页脚距离，也可断开或重新链接某节某类型的页眉页脚。断链时抄走 header/footer 自己的 `.rels`，媒体仍共用。也可为空白槽位新建页眉页脚部件。
apply 可写本节 `w:titlePg` 与整篇 `w:evenAndOddHeaders`，也可在正文段后插入分节符或删除非末节。
apply 可插入或删除目录块，也可插入题注与交叉引用。不在库里求 SEQ / REF 的值。
apply 可插入书目源、引文与书目块，不在库里按样式表排版。也可插删构建基块并把基块正文复制进主文档，也可插改删 OMML 公式与内容控件。
页眉、页脚、正文、注和批注里的 DrawingML 图片可换、插、删，并可改尺寸与绕排。
文本框可插、删（不含注内、批注内与框内再套框；Word 16.0 不会把框写进另一个 `w:txbxContent`），可改尺寸、绕排、填色、描边、横竖排、框内垂直锚点与链接。连接线可插、删（不含注内、批注内与框内）。
无框形状可插、删、改位置、尺寸、填充与线（不含注内、批注内与框内）。
墨迹画布可插、换、删，并可改尺寸与绕排（不含注内、批注内与框内），并可按 id 导出 InkML。
浮动图 / 框 / 连接线 / 无框形状可组合或解开。脚注、尾注可插、删，可改编号格式（整篇同时写 `settings.xml` 与各节 `sectPr`，Word 读节属性）。批注可插、删、回复、改字、改作者与完成标记。
嵌入图表可插、删、改标题、改系列数据、改类型与图例，并可改宿主 drawing 的尺寸与绕排；宿主可以是正文、页眉页脚、框内、注内或批注内段落。圆环、雷达、气泡与组合图走同一套 op；透视图表只改标题、图例、标签、删除与尺寸绕排，不改数。
股价（含 OHLC）、曲面（含线框与俯视）、复合饼可插、改类型。带成交量的股价按组合图写入。
chartex 可 insert / create（须带 `layoutId`）。已写入的可以改标题、图例、系列数据，并增删同布局系列；帕累托和箱线条数固定，`regionMap` 不插入。次轴、趋势线、误差线（含 `cust`）可写。
SmartArt 可插、删、改各点文字、增删点、改配色与快速样式，并可改宿主 drawing 的尺寸与绕排。宿主范围与图表相同。
三种格式都可改文件属性。docx 还可开关无密码的限制编辑与只读推荐。`exportMedia` 按图 id 写出 `/word/media/…` 原字节，按 OLE id 写出 embeddings 原字节，按墨迹画布 id 写出 `/word/ink/…` 原字节，按 3D 模型 id 写出 GLB 原字节。
`exportChartData` 按图表 id 写出 embedding xlsx 原字节。不整篇从 JSON 重建。
create 生成最小 Transitional 包，含 `styles.xml`（Normal 与 Heading1–3）和 `settings.xml`（`compatibilityMode` 为 15）。
给了 `paragraphs[].sectionType` 时可写多节，给了 `paragraphs[].charts` 时复用 `insertChart`，给了 `footnote` / `endnote` / `comments` 时复用对应 insert。
给了 `comments[].replies` 时复用 `replyComment`。
给了 `hyperlinks` / `bookmarks` / `fields` / `maths` / `contentControls` 时复用对应 insert。
给了 `document.numberings` 时复用 `insertNumbering`，再按 `paragraphs[].num` 发 `setParaNumbering`。
给了 `document.tocs` 时复用 `insertToc`。不写修订痕迹。段落上可带文本框、DrawingML 图、SmartArt 与图表。
给了 `document.tables` 时可在正文写表；页眉页脚可带 `tables`。
给了 `document.headers` / `footers` 时可写 `default` / `even` / `first`（给了 even 则写 `evenAndOddHeaders`，给了 first 则写 `titlePg`）。
不改注区怎么画，不改批注气泡怎么排，不画图表，不算 SmartArt 版式。表宽、列宽、行高、行内对齐与表级边框可按点名写入，不算自动列宽。

### pptx

入口由根关系里 `/officeDocument` 指向，常见路径 `/ppt/presentation.xml`。幻灯片顺序跟 `p:sldIdLst`，不跟文件名。
inspect 默认只返回页名单，以及各页 `pictureCount`、`mathCount`、`videoCount`、`audioCount`、`chartCount`、`smartArtCount` 与 `commentCount`。
各页还带 `hyperlinkCount`、`fieldCount`、`tableCount`、`oleObjectCount`、`model3dCount`、`inkCount`、`zoomCount`、`connectorCount`、`animCount`、`layoutId`、`hasNotes` 与 `transition`。
根上还带 `masterCount`、`hasHandout`、`sectionCount`、`slideSize`、`customShowCount`、`showPr`、`modifyProtection` 与 `props`。
要图名单必须给 `scope.pictures`，要公式框名单必须给 `scope.maths`，要视频 / 音频名单必须给 `scope.media`，要图表名单必须给 `scope.charts`。
要 SmartArt 名单必须给 `scope.smartArt`，要表名单必须给 `scope.tables`，要嵌入对象名单必须给 `scope.oleObjects`。
要 3D 模型名单必须给 `scope.model3ds`，要墨迹名单必须给 `scope.inks`，要 Zoom 名单必须给 `scope.zooms`，要连接线名单必须给 `scope.connectors`。
要备注形状必须给 `scope.notes`，要批注名单必须给 `scope.comments`，要超链接名单必须给 `scope.hyperlinks`，要域名单必须给 `scope.fields`。
要动画摘要必须给 `scope.timing`，要母版与版式名单必须给 `scope.masters`，要备注母版占位符必须给 `scope.notesMaster`。
要讲义母版槽位必须给 `scope.handout`，要幻灯片节名单必须给 `scope.sections`，要自定义放映名单必须给 `scope.customShows`。
apply 可改被点名形状的 `p:txBody` 可见文字（含备注形状、母版 / 版式占位符与装饰形、备注母版与讲义占位符，缩略图占位符除外），可插入文本框与非矩形预设几何、写入自定义路径（`a:custGeom`）、插入连接线（可写 `stCxn` / `endCxn`）、组合与解开、删除被点名形状、改画布坐标与尺寸、填充描边与 DrawingML 文字格式。
母版 / 版式装饰形可用 `deleteShape`，占位符请用 `deletePlaceholder`。
删除时清掉 timing 里对应 `tgtEl`，不重建未点名形状，不整节无故丢掉仍有内容的 `<p:timing>`。
可用 `setNotesText` 按页改演讲者备注。也可换、插、删幻灯片或备注页上的 `p:pic`，并改尺寸与画布坐标。
也可插、删、改幻灯片或备注页上的嵌入图表。改系列时 cache 与 embedding 双写，也可增删趋势线、误差线，并把系列挂到次轴。
也可插、删、改幻灯片或备注页上的 SmartArt。改点时写 data 部件，对不上 drawing 缓存则删缓存。
也可插、删、改幻灯片与备注页上的 DrawingML 表，包括单元格字、填充、六边、格边距、行列、画布坐标、尺寸与格网列宽行高。删表时清对应 timing `tgtEl`。
也可插、删、回复、改字、改作者与完成标记现代批注（MS-PPTX 2018/8 `p188:cmLst`，不写 ECMA-376 §19.4 旧式部件）。
也可插入空白页、删除、按 `sldIdLst` 重排与隐藏（`p:sld/@show`）。也可用 `setTransition` 写或清除幻灯片、母版或版式上的 `<p:transition>`。
也可用 `insertAnim` / `setAnim` / `deleteAnim` / `moveAnim` 按模板增删改预设效果。不接受整棵时间轴树。第一条 `with` / `after` 进片自动播。
也可用 `setSlideLayout` 只改该页 `.rels` 的版式关系，不删用户形状，不抄占位符进页。
也可用 `insertLayout` / `deleteLayout` / `insertPlaceholder` / `deletePlaceholder` / `insertMaster` / `deleteMaster` 插删空白版式、带 `p:ph` 的占位符与整份母版。不算母版排版引擎。`insertPlaceholder` 还可写备注母版与讲义。
内容、图表、表格、图片、媒体占位符，以及备注母版上的幻灯片图像，也用 `insertPlaceholder` 写成 `p:sp`。
也可用 `insertShape` / `deleteShape` 插删母版 / 版式上没有 `p:ph` 的装饰形，不抄进各页。
也可用 `setSlideSize` 只改 `p:sldSz`，不重排形状。也可用 `setNotesSize` 只改 `p:notesSz`，不重排备注形状。
也可用 `setThemeColors` / `setThemeFonts` / `setThemeFmt` / `setThemeExtraSchemes` / `setThemeObjectDefaults` 改 slideMaster、notesMaster 或 handout 的 theme，不把 `schemeClr` 展开成 RGB。
也可用 `setClrMap` / `setClrMapOvr` / `setThemeOverride` 读写 `p:clrMap`、页 / 版式 / 备注页 `p:clrMapOvr` 与页级 themeOverride 部件。
也可用 `setHandoutText` 改讲义母版页眉、页脚、日期与页码占位符。没有 handoutMaster 则新建最小部件。
讲义与备注母版占位符也可绑同一套预设动画。
也可插删改幻灯片、备注页与幻灯片母版 / 版式上的 `a:hlinkClick` / `a:hlinkHover` / `a:hlinkMouseOver`（URL、文件或跳转到某页，含图、表、图表、SmartArt 与表单元格）。
也可插删改幻灯片、备注页与幻灯片母版 / 版式文本形状里的 `a:fld`（页码与日期域，不求值）。
也可往幻灯片、备注页与幻灯片母版 / 版式插、删内嵌视频与音频（`p:pic` 上的 `a:videoFile` / `a:audioFile` 与 `p14:media`，不写 timing）。
也可用 `setMedia` 改裁剪、书签与字幕轨，不写 fade。也可用 `setPicturePos` / `setPictureSize` 改已有视频 / 音频的画布坐标与尺寸。
也可用 `insertCustomShow` / `setCustomShow` / `deleteCustomShow` 读写 `p:custShowLst`，不算放映引擎。
也可插、删幻灯片、备注页与幻灯片母版 / 版式上的嵌入对象（`p:oleObj`，不激活 OLE 服务器），也可插、删、换幻灯片、备注页与幻灯片母版 / 版式上的 3D 模型（`am3d:model3d`，不栅格化）。
已有 3D 可改尺寸、画布坐标与相机 / 旋转 / 光源。
也可插、换、删幻灯片、备注页与幻灯片母版 / 版式上的墨迹（一笔一个 `p:contentPart`，不进 `pictures`），并可改已有墨迹的画布坐标与尺寸。
也可插改删幻灯片、备注页与幻灯片母版 / 版式上的 drawing 公式框（`a14:m`，不实现排版引擎），并可改已有公式框的画布坐标与尺寸。`setMath` 仍只改字。
也可插、删、改幻灯片、备注页与幻灯片母版 / 版式上的 Zoom（`sldZm` / `sectionZm` / `summaryZm`，不实现放映跳转），并可改已有 Zoom 的画布坐标与尺寸。
也可改文件属性。
插删重排不重写未点名页。`exportMedia` 按幻灯片、备注页、批注、母版 / 版式上的图 id、视频 id、音频 id、OLE id、3D 模型 id 或墨迹 id 写出 `/ppt/media/…`、`/ppt/embeddings/…` 或 `/ppt/ink/…`。
`exportChartData` 按图表 id 写出 embedding。create 生成最小 Transitional 包，必含 theme、slideMaster、slideLayout、presentation、各页 slide、presProps。
给了 `slides[].pictures` 时复用 `insertPicture`，写入 `/ppt/media/…` 与该页 `p:pic`。
给了 `slides[].maths` 时复用 `insertMath`，写入 `a14:m` AlternateContent。
给了 `slides[].model3ds` 时复用 `insertModel3d`，写入 GLB、海报 PNG 与 `am3d` AlternateContent。
给了 `slides[].inks` 时复用 `insertInk`，写入 InkML、海报 PNG 与 `p14` AlternateContent。
给了 `slides[].charts` 时复用 `insertChart`，写入 `/ppt/charts/…`、embedding 与 `p:graphicFrame`。
给了 `slides[].smartArts` 时复用 `insertSmartArt`，写入 `/ppt/diagrams/…` 与图示 `p:graphicFrame`。
给了 `slides[].tables` 时复用 `insertTable`，写入表 `p:graphicFrame`。给了 `slides[].notes` 时复用 `setNotesText`。
给了 `slides[].comments` 时复用 `insertComment`。给了 `slides[].anims` 时复用 `insertAnim`（白名单预设，可写 `delay` / `dur` / `repeat` / 声音 / 按段；第一条 `with` / `after` 进片自动播）。
母版与版式进树（`scope.masters`），含占位符与装饰形。`setSlideLayout` 只换关系，写入用 `insertLayout` / `insertPlaceholder` / `insertMaster` / `insertShape` 等。
动画效果摘要进树（`scope.timing`），写入用 `insertAnim` / `setAnim` / `deleteAnim` / `moveAnim`。切换进树，用 `setTransition` 改。幻灯片尺寸始终进树，写入用 `setSlideSize`。备注页尺寸始终进树，写入用 `setNotesSize`。
主题配色、字体、`fmtScheme` 常用槽、补充色板与 `objectDefaults` 挂在 `scope.masters`、`scope.notesMaster` 与 `scope.handout` 的 `theme` 上。
写入用 `setThemeColors` / `setThemeFonts` / `setThemeFmt` / `setThemeExtraSchemes` / `setThemeObjectDefaults`。
母版 `clrMap`、页 / 版式 `clrMapOvr` 与页级 `themeOverride` 进树，写入用 `setClrMap` / `setClrMapOvr` / `setThemeOverride`。`mc:AlternateContent` 取哪一支，见上文「标记兼容性」。

### cli

从文件或 stdin 读入一份领域 JSON，按 `action` 分发。inspect 把树打到 stdout。create 和 apply 写出目标 Office 文件。`exportMedia` 写出媒体文件。
`exportChartData` 写出图表 embedding。契约见 [cli.md](cli.md)。

## 数据流

### inspect

调用方给出路径和范围。库打开 OPC 包，定位格式入口部件（xlsx 是 Workbook，docx 是 Main Document，pptx 是 Presentation）。按范围读必要的 XML，生成带 id 的树。
xlsx 默认不读完全部单元格，也不展开图、图表、文本框、公式框、形状、连接线、SmartArt、页眉原文、数据验证、条件格式、表格、透视、批注、超链接、迷你图、嵌入对象、切片器、连接、外部引用或自定义视图的名单。
这些名单不展开时，仍返回对应的计数字段，以及 `hasHeaderFooter`、`print`、`view`、`sheetProtection`，和根上的 `props`、`workbookProtection`、`theme`、`themeOverride`、`definedNameCount`、`definedNames`。
xlsx 的 `worksheet` 还返回 `autoFilter` 与 `sortState`。
docx 默认不返回正文 `paragraphs`，也不展开页眉页脚段落，以及图、文本框、连接线、注、批注、图表、SmartArt、OLE、水印、表、修订、超链接、书签、域、目录、题注、交叉引用、构建基块、公式、内容控件、编号、样式或 run 的名单。
这些名单不展开时，仍返回根上 `props`、`protection`、`theme`、`themeOverride`、`fontEmbed`、`pageBackground` 与 `watermarkCount`。
pptx 默认不返回 `shapes`，也不展开图、公式框、图表、SmartArt、表、嵌入对象、3D、墨迹、Zoom、连接线、备注、批注、超链接、域、动画、母版、备注母版、讲义、节或自定义放映的名单。
这些名单不展开时，仍返回根与各页的计数字段、各页 `layoutId` / `hasNotes` / `transition`，以及根上 `props`。
包不写回。

### apply

调用方给出路径和 `ops`。库打开包，按每条 op 的 id 找到 XML 节点，只改这些节点。xlsx 在改了单元格之后，把工作簿标成打开时需要重算。
xlsx 也可改被点名的工作表图（换、插、删、改尺寸、改锚点），也可插删改工作表图表，脏 drawing、chart 部件、工作表 XML、关系与媒体。
xlsx 也可新建或删除图表工作表，脏 workbook 的 `sheets`、chartsheet 部件及其 drawing / chart。
xlsx 也可插入、删除、改名、重排与隐藏普通工作表，脏 workbook 的 `sheets`、worksheet 部件与关系。
对 `dialogsheet` / `macrosheet` 这些 op 仍报 `NotImplemented`。xlsx 也可用 `setCellStyle` 改被点名格子的样式，脏该表 XML 与 `styles.xml`。
xlsx 也可设列宽、行高、合并与取消合并，脏该表 `cols` / `sheetData` / `mergeCells`，不标重算。xlsx 也可改共享公式主格、把主格迁到新区域左上角，或写成数组公式，仍标重算。能算的单元格公式写入缓存，算不了的不写。
xlsx 也可增删改数据验证，脏该表 `dataValidations`，不标重算。xlsx 也可增删改条件格式，脏该表 `conditionalFormatting` 与必要时 `styles.xml` 的 `dxfs`，不标重算。
xlsx 也可增删改工作表表格，脏 `/xl/tables/…`、工作表 `tableParts`、关系与内容类型，必要时脏 `sheetData`，不标重算。
xlsx 也可增删改透视表，脏 `/xl/pivotTables/…`、`/xl/pivotCache/…`、workbook 的 `pivotCaches`、关系与内容类型，不标重算。只有行字段和求和时，求值会把合计写进 `sheetData`。
xlsx 也可增删改定义名，脏 workbook 的 `definedNames`，不新建部件，不标重算。
xlsx 也可用 `setPrintSetup` 改工作表页边距、纸张、方向、缩放、网格线、打印区域与手工分页符，脏该表 XML，打印区域还脏 workbook 的 `definedNames`，不标重算。
xlsx 也可用 `setSheetView` 改默认视图的缩放、冻结与拆分，脏该表 `sheetViews`，不标重算。
xlsx 也可列出、插入、删除自定义视图，并按快照把窗格写回当前视图，脏 workbook 的 `customWorkbookViews` 与各表 `customSheetViews` / 当前 `sheetViews`，不标重算，不恢复隐藏行 / 列。
xlsx 也可用 `groupRows` / `ungroupRows` / `groupCols` / `ungroupCols` / `setOutline` 改行 / 列大纲与折叠，不标重算。
也可用 `setRowHidden` / `setColHidden` 写没有大纲级的隐藏行 / 列，脏该表 `sheetPr/outlinePr`、`sheetFormatPr`、`cols` 与 `sheetData` 行属性，不标重算。
xlsx 也可用 `setAutoFilter` 改工作表级筛选范围，脏该表 `autoFilter`，不改表格部件，不隐藏行，不标重算。
xlsx 也可用 `setSortState` 改工作表级排序条件（含自定义序列、按颜色 / 图标、`sortMethod`），脏该表 `sortState`，给了 `color` 时还可脏 styles 的 `dxfs`，不重排 `sheetData`，不标重算。
xlsx 也可插删回复并改线程化批注，脏 `/xl/threadedComments/`、`/xl/persons/person.xml`、占位 `/xl/commentsN.xml`、VML、工作表 `legacyDrawing`、关系与内容类型，不标重算。
xlsx 也可插删改工作表文本框、形状、连接线与 SmartArt，可用 `setShapeFormat` 改 DrawingML 文字格式，脏 worksheet drawing、drawing `.rels`、`/xl/diagrams/` 与内容类型，不标重算。
xlsx 也可插删改工作表单元格超链接，脏该表 XML 与工作表 `.rels`，不标重算。xlsx 也可插删改工作表迷你图组，脏该表 `extLst`，不新建部件，不标重算。
xlsx 也可列出、导出、插入并删除工作表嵌入对象，脏 `oleObjects`、worksheet drawing、VML `legacyDrawing`、工作表 `.rels`、embeddings 与预览媒体，不把字节塞进 inspect 树，不标重算。
xlsx 也可插、删、换工作表 3D 模型，并改已有 3D 的尺寸、锚点与相机 / 旋转 / 光源，脏 worksheet drawing、drawing `.rels`、GLB 与海报 PNG，不把 GLB 塞进 inspect 树，不标重算。
xlsx 也可插改删工作表 drawing 公式框，并可改已有公式框的锚点与尺寸，脏 worksheet drawing，写 `a14:m` AlternateContent，不标重算。`setMath` 仍只改字。
xlsx 也可列出、插入、删除工作表切片器与时间线，并改已有切片器（含时间线）的尺寸、锚点、视图属性与勾选。
这些 op 脏 `/xl/slicers/`、`/xl/slicerCaches/`、`/xl/timelines/`、`/xl/timelineCaches/`、worksheet drawing，以及工作表和 workbook 上的切片器引用、定义名、`.rels` 与内容类型。
表格切片器的勾选还脏该表 `autoFilter`，不隐藏行。
透视源的插入还脏 `/xl/pivotCache/pivotCacheDefinitionN.xml` 的 `extLst`（`x14:pivotCacheDefinition/@pivotCacheId`）。
时间线还会把该透视表 `updatedVersion` 升到至少 8，不在库里执行筛选，不重写透视缓存合计，不标重算。
xlsx 也可列出、插入并删除工作簿外部数据连接，脏 `/xl/connections.xml`、workbook `.rels` 与内容类型，不向数据源发请求，不把查询结果写进 `sheetData`，不标重算。
xlsx 也可列出、插入、删除并改路径对其他工作簿的外部引用，脏 workbook `<externalReferences>`、`/xl/externalLinks/…` 及其 `.rels`、关系与内容类型；改路径只动该链 `.rels` 的 `Target`，不写 `xxl21:alternateUrls`。求值时打开本机目标文件，把读到的格子写进公式缓存和 `sheetDataSet`。
xlsx 也可用 `setProtection` 改工作表 `sheetProtection` 或 workbook `workbookProtection`，可写 SHA-512 哈希，不标重算。
xlsx 也可插、改、删工作表允许编辑区域，脏该表 `<protectedRanges>`，可写同一套 SHA-512，不标重算。
xlsx 也可用 `setThemeColors` / `setThemeFonts` / `setThemeFmt` 改工作簿 theme 部件的配色、字体、方案名与 `fmtScheme` 常用槽，只脏该 theme 部件。
也可用 `setThemeExtraSchemes` 整份替换补充色板。也可用 `setThemeObjectDefaults` 改默认形状 / 线 / 文本框样式。也可用 `setThemeOverride` 新建或改 `/xl/theme/themeOverride1.xml`。不把格子上的 theme 指数算成 RGB。
xlsx 也可改工作表上的页眉页脚原文，并可对 `worksheet` 与 `chartsheet` 页眉槽位插换删 DrawingML 图，脏 `headerFooter` / `drawingHF` 与对应 drawing。
xlsx 也可改文件属性，脏 `docProps`。
docx 可改被点名的段落，可在主文档、页眉页脚与表格单元格插删段落，可插删表以及行与列并合并拆分单元格，可接受或拒绝 run 级、段落标记、表行与表属性修订。
可插删改超链接与书签，可插改删域指令（不求值；可按 code 写常用域与窗体 ffData），可插入或删除目录块（不在库里算页码），可插入题注与交叉引用（不在库里求 SEQ / REF）。
可插删构建基块并把基块正文复制进主文档，可嵌入或删除 TrueType 字体并改 `embedTrueTypeFonts` / `embedSystemFonts` / `saveSubsetFonts`。
可改页面背景色与节上页面边框，可插改删 OMML 公式（不实现排版引擎），可插改删拼音（不实现注音排版），可改段上禁则与 run 强调 / 压缩排版，可改整篇压缩与自定义禁则字符。
可插改删内容控件（含复选、图片、组合外壳与公式外壳；不实现开发者模式 UI），可写入编号定义并把段落套上或去掉编号，可给段落换样式并改样式定义上的一批常见属性。
可改段与 run 上的直接格式（含段边框、底纹、制表位，以及 run 的突出显示、删除线与上下标），可改节上页边距、纸张、分栏、垂直对齐、行号、文档网格与装订线侧。
可改整篇镜像边距与装订线在上，可改节上页眉页脚距边界，也可改节上页眉页脚是否链接前节，也可为空白槽位新建页眉页脚，也可写 `titlePg` 与 `evenAndOddHeaders`。
也可插入或删除节，也可改页眉、页脚、正文、注或批注里被点名的 DrawingML 图与文本框，也可插删连接线与无框形状并把浮动图 / 框 / 连接线 / 无框形状组合或解开。
也可插删脚注尾注并改编号属性，也可插删回复批注并改完成标记，也可插删改嵌入图表（含框内、注内、批注内；含趋势线、误差线与次轴），也可插删改 SmartArt（宿主范围相同）。
docx 也可往正文、页眉页脚、文本框、脚注、尾注与批注段落插入嵌入对象，脏宿主 XML、`.rels`、embeddings 与占位预览图。
docx 也可插删换 3D 模型，并可改已有 3D 的尺寸、绕排与相机 / 旋转 / 光源，脏宿主 XML、`.rels`、GLB 与海报 PNG。
脚注、尾注和批注段落也可以插入、替换、删除、改尺寸、改相机并按 id 导出，只写行内，关系挂在该部件上；改成浮动报 `InvalidField`。
docx 也可用 `setProtection` 改 `settings.xml` 的保护标记（限制编辑哈希走遗留预处理，修改密码走 UTF-16LE SHA-512）。
窗体保护还可脏主文档各节 `w:formProt`，也可用 `setFormProt` 把某节改成例外，也可用 `insertEditRange` / `deleteEditRange` 读写正文、页眉页脚与文本框段落上的 `w:permStart` / `w:permEnd`。
docx 也可插删页眉 DrawingML 水印，无 default 页眉时会新建，并可脏页眉部件与媒体。
docx 也可用 `setThemeColors` / `setThemeFonts` / `setThemeFmt` 改主文档 theme 部件的配色、字体、方案名与 `fmtScheme` 常用槽，只脏该 theme 部件。
也可用 `setThemeExtraSchemes` 整份替换补充色板。也可用 `setThemeObjectDefaults` 改默认形状 / 线 / 文本框样式。也可用 `setThemeOverride` 新建或改主文档 themeOverride 部件。
pptx 可改被点名形状的可见文字（含备注形状与母版 / 版式占位符），可插入文本框与非矩形预设几何、写入自定义路径、插入连接线（幻灯片、备注页与母版 / 版式，可写 `stCxn` / `endCxn`）、组合与解开、删除被点名形状并清对应 timing `tgtEl`、改画布坐标与尺寸、填充描边与 DrawingML 文字格式。
可用 `setNotesText` 按页改或新建演讲者备注，也可改被点名的幻灯片图、备注图与母版 / 版式图（换、插、删、改尺寸、改画布坐标），也可插删改幻灯片、备注页与母版 / 版式图表。
也可插删改幻灯片、备注页与母版 / 版式 SmartArt，也可插删改幻灯片、备注页与母版 / 版式 DrawingML 表（含格网列宽与行高、格填充（实心/渐变/图案/图片）与六边、格边距、格 `vert` / `anchorCtr` / `horzOverflow` / `cell3D` / `headers`、表样式 GUID 与 `tblLook`）。
也可插删回复并改现代批注，也可对批注或回复 id 插图并换、删批注内图（不改尺寸位置），也可按批注内图 id `exportMedia`，也可插入、删除、重排与隐藏幻灯片。
也可用 `setTransition` 写或清除该页、母版或版式上的 `<p:transition>`（含自动进片、方向、切换声音、`split` 的 `orient`、`wheel` 的 `spokes` 与 `p15:prstTrans`）。
也可用 `insertAnim` / `setAnim` / `deleteAnim` / `moveAnim` 按模板增删改该宿主 `<p:timing>` 里的预设效果（不接受整棵时间轴树）。
也可用 `setSlideLayout` 只改该页 `.rels` 的版式关系，也可用 `insertLayout` / `deleteLayout` / `insertPlaceholder` / `deletePlaceholder` / `insertMaster` / `deleteMaster` 插删版式、占位符与母版。
也可用 `setSlideSize` 只改 `p:sldSz`，也可用 `setNotesSize` 只改 `p:notesSz`。
也可用 `setThemeColors` / `setThemeFonts` / `setThemeFmt` / `setThemeExtraSchemes` / `setThemeObjectDefaults` 改 slideMaster、notesMaster 或 handout 的 theme。
也可用 `setClrMap` / `setClrMapOvr` / `setThemeOverride` 改配色映射与页级覆盖，也可用 `setHandoutText` 改或新建讲义母版。
也可用 `insertField` / `setField` / `deleteField` 插删改幻灯片、备注页与母版 / 版式文本形状里的 `a:fld`（不求值）。
也可用 `insertSlideSection` / `setSlideSectionName` / `deleteSlideSection` / `setSlideSection` 读写 `p14:sectionLst`（删节时页还在；把页划进某节时可重排 `sldIdLst`）。
也可用 `insertCustomShow` / `setCustomShow` / `deleteCustomShow` 读写 `p:custShowLst`。删页时按 `r:id` 清引用。
也可插、删幻灯片、备注页与母版 / 版式上的嵌入对象（`p:oleObj`，不激活 OLE 服务器）。
也可插、删、换幻灯片、备注页与母版 / 版式上的 3D 模型（`am3d:model3d`，不栅格化），并可改已有 3D 的尺寸、画布坐标与相机 / 旋转 / 光源。
也可插改删幻灯片、备注页与母版 / 版式上的 drawing 公式框（`a14:m`，不实现排版引擎），并可改已有公式框的画布坐标与尺寸，也可插、删幻灯片、备注页与母版 / 版式上的 Zoom（`sldZm` / `sectionZm` / `summaryZm`，不实现放映跳转）。
也可用 `setShowPr` 改 `presProps` 的 `p:showPr`，也可用 `setProtection` 写出或删掉 `p:modifyVerifier`（给了 `password` 时写 PowerPoint 16.0 的 SHA-512 哈希）。
脏 `presentation.xml` 的 `sldIdLst`、presentation 关系、新页或被删页及其备注 / 现代批注、内容类型，以及若有则同步 `p14:sectionLst` 与 `p:custShowLst`。
未点名页原字节写回。插删改图、图表、SmartArt、备注与批注时仍脏该页 XML、该页 `.rels`、notesSlide 及其 `.rels`、必要时 notesMaster 与 presentation、`/ppt/charts/`、embedding、`/ppt/diagrams/`、`/ppt/comments/` 与 `/ppt/authors.xml`。
表格 op 只脏该宿主 XML（幻灯片、notesSlide、slideMaster 或 slideLayout）。`setTableCellFill` 写 `blip` 时还可脏该宿主 `.rels`、媒体与内容类型。
形状管理 op 只脏目标宿主 XML（含被点名的 slideMaster / slideLayout）。`insertShape` 往备注插时与 `setNotesText` 相同，还可脏 notesMaster 与 presentation。
超链接 op 脏该宿主 XML（幻灯片、notesSlide、slideMaster 或 slideLayout）与该宿主 `.rels`。域 op（`insertField` / `setField` / `deleteField`）只脏该宿主 XML。`setSlideLayout` 只脏该页 `.rels`。
`insertLayout` / `deleteLayout` / `insertMaster` / `deleteMaster` 还可脏母版 / 版式 / theme 部件、presentation 与内容类型。
`insertPlaceholder` / `deletePlaceholder` 只脏目标母版或版式 XML。
`setSlideSize` 只脏 `presentation.xml` 的 `p:sldSz`。`setNotesSize` 只脏 `presentation.xml` 的 `p:notesSz`。
`setThemeColors` / `setThemeFonts` / `setThemeFmt` / `setThemeExtraSchemes` / `setThemeObjectDefaults` 只脏该母版指向的 theme 部件。
`setClrMap` 只脏被点名母版 XML。`setClrMapOvr` 只脏被点名页 / 版式 / 备注页 XML。
`setThemeOverride` 脏该宿主 `.rels` 与 `/ppt/theme/themeOverrideN.xml`。
三种格式都可用 `setProps` / `setCustomProp` / `deleteCustomProp` 改 core / app / custom，脏对应 `docProps` 部件与必要时的根关系、内容类型。保存时只重写脏部件。

### create

调用方给出格式、输出路径和内容对象。库在内存里生成一套最小可打开的 Transitional 部件，打成新 ZIP。不读取任何已有文件当模板，除非以后另开「从模板复制」的动作。
xlsx 的 `sheets[].pictures` 复用 `insertPicture`，`sheets[].charts` 复用 `insertChart`（数据写本表单元格，不写 embedding）。
`sheets[].kind: chartsheet` 复用 `insertChartSheet`（数据写点名的普通表，不写 embedding），`sheets[].headerFooters` 复用 `setHeaderFooterText` 与页眉 `insertPicture`（不写 VML）。
`sheets[].comments` 复用 `insertComment`，`comments[].replies` 复用 `replyComment`（只一层）。
`sheets[].hyperlinks` 复用 `insertHyperlink`，`sheets[].textBoxes` / `maths` / `shapes` / `connectors` / `smartArts` / `model3ds` 复用对应 insert。
`workbook.cellStyles` 复用 `insertCellStyle`，`cells[].namedStyle` 复用 `setCellNamedStyle`。
给了 `workbook.theme` 时写 `/xl/theme/theme1.xml`；省略则仍不写 theme 部件。
其余第四轮字段（列宽、表格、保护、迷你图、OLE、切片器等）复用对应 apply，见 [json-contract-xlsx.md](json-contract-xlsx.md#create)。
docx 段落与页眉页脚上的 `pictures` 同样复用插图，`charts` 复用 `insertChart`（数据写 embedding），`oleObjects` 复用 `insertOle`。
`inkCanvases` 复用 `insertInkCanvas`，`footnote` / `endnote` / `comments` 复用对应 insert。
`comments[].replies` 复用 `replyComment`，`hyperlinks` / `bookmarks` / `fields` / `maths` / `contentControls` 复用对应 insert。
`document.numberings` 复用 `insertNumbering`，`paragraphs[].num` 复用 `setParaNumbering`。`document.tocs` 复用 `insertToc`。
docx create 第四轮同样复用已有 apply：`styles` / `sources` / `bibliographyStyle` / `bibliographies` / `captions` / `indexes` / `docParts` / `embeddedFonts` / `fontEmbed` / `eastAsianSettings`。
段上的 `shapes`、`connectors`、`rubies`、`citations`、`indexEntries`、`crossRefs`、`editRange`，以及图与框的 DrawingML 链和表外观，同样复用已有 apply。
docx create 第五轮复用已有 apply：框内表与框内链 / 书签 / 域 / 公式 / 控件 / 拼音、段 `borders` / `shd` / `tabs`、`runs[]` 的 `setRunFormat`、`groups[]` 的 `groupShapes`，以及 `protection.writePassword`。docx create 不写修订痕迹。
限制编辑口令 `password` 与修改口令 `writePassword` 都按 `setProtection` 写入哈希。
pptx 的 `slides[].pictures` 复用 `insertPicture`，`slides[].maths` 复用 `insertMath`，`slides[].model3ds` 复用 `insertModel3d`。
`slides[].inks` 复用 `insertInk`，`slides[].videos` / `slides[].audios` 复用 `insertVideo` / `insertAudio`。
`slides[].charts` 复用 `insertChart`（数据写 embedding），`slides[].smartArts` 复用 `insertSmartArt`。
`slides[].tables` 复用 `insertTable`，`slides[].oleObjects` 复用 `insertOle`。
已有嵌入对象的预览框用 `setOleSize` 改尺寸，三家同一动作，不启动 OLE 服务器。
`slides[].connectors` 复用 `insertConnector`，`slides[].zooms` 复用 `insertZoom`。
`slides[].notes` 复用 `setNotesText`。对象形态的 `anims` 复用 `insertAnim` 写到备注页。`maths` / `model3ds` / `inks` 复用对应 insert 写到备注页。
`slides[].comments` 复用 `insertComment`，形状上的 `url` / `file` / `slide` 复用 `insertHyperlink`（可 `on=run`）。
`pictures[].hyperlink` / `tables[].hyperlink` / `charts[].hyperlink` / `smartArts[].hyperlink` 以及表单元格链字段同样复用 `insertHyperlink`。
`shapes[].geom` 复用 `setShapeGeom`，`shapes[].fields` 复用 `insertField`，`shapes[].textFill` / `textOutline` 复用 `setShapeFormat`。
`slides[].transition` 复用 `setTransition`，`slides[].anims` 复用 `insertAnim`（白名单预设，可写 `delay` / `dur` / `repeat` / 声音 / 按段，第一条 `with` / `after` 进片自动播，不接受整棵时间轴树）。
`presentation.sections` 复用 `insertSlideSection`，`presentation.customShows` 复用 `insertCustomShow`。
`presentation.handout` 复用 `setHandoutText`，`presentation.showPr` 复用 `setShowPr`。
`presentation.modifyProtection` 复用 `setProtection`。
create 本期不写批注内图。
三种格式的 `props` 复用 `setProps` / `setCustomProp` 同一套字段写 core / app / custom。
docx 的 `document.protection` 复用 `setProtection`。docx 的 `document.watermarks` 复用 `insertWatermark`。
docx 的 `document.theme` 写入 theme 部件；`document.pageBackground` / `document.pageBorders` 复用对应 apply。省略 `theme` 则不写该部件。

### exportMedia

调用方给出源 docx、xlsx 或 pptx、图 id、docx / xlsx / pptx OLE id、三种格式 3D 模型 id、docx 墨迹画布 id、pptx 视频 / 音频 id 或 docx 嵌入字体 id 和本地目标路径。
库打开包，图 id 按 `a:blip` 的关系写出媒体部件；带 `asvg:svgBlip` 时改走 SVG 关系。
OLE id 按 oleObject 或 package 关系写出 embeddings 部件。
3D 模型 id 按 model3d 关系写出 GLB 部件。
docx 墨迹画布 id 按 customXml 关系写出 InkML 部件。
docx `font:{n}` 按 fontTable 关系写出 odttf 原字节。包不写回。

### exportChartData

调用方给出源 docx 或 pptx、图表 id 和本地目标路径。库打开包，按图表关系找到 embedding 工作簿，把该部件原字节写出。包不写回。没有 embedding 则失败。xlsx 本期直接拒绝。

## 硬约束

- 自己实现 OPC。不要把 `office_oxide` 写进 `Cargo.toml`，不要加它的 git 子模块，不要在仓库里放它的源码目录。
- 查规范用 `standards/md/` 的章节索引。
- xlsx 单元格见 ECMA-376 §18.3.1.4，公式见 §18.3.1.96 与 §18.17，共享字符串见 §18.4，计算属性见 §18.2.2。
- 工作表图与 `graphicFrame` 见 §20.5，图表部件见 §21.2，透视表见 §18.10。线程化批注见 MS-XLSX §2.1.17 / §2.1.18 / §2.3.7 / §2.6.203–2.6.207，占位批注见 ECMA-376 §18.7。
- docx 主文档见 §11.3.10，段落见 §17.3.1.22，样式见 §17.7，页眉页脚见 §17.10，批注见 §17.13，编号见 §17.9，域见 §17.16，图表见 §21.2。
- SmartArt / diagram 见 §21.4，`w14:paraId` 见 MS-DOCX §2.6.2.3，`commentEx` 见 MS-DOCX §2.5.3.1。
- pptx Presentation 部件见 §13.3.6，幻灯片见 §13.3.8 与 §19.3，页名单见 §19.2.1.34，自定义放映见 §19.2.1.5–19.2.1.7。
- 形状与文本框见 §19.3.1.45 / §19.3.1.51，图片 `p:pic` 见 §19.3.1.37，`cNvPr` 见 §19.3.1.12，`p:graphicFrame` 见 §19.3.1.21。
- 嵌入对象 `p:oleObj` 见 §19.3.2.4，Zoom 见 MS-PPTX §2.2.15 / §2.9–2.11，备注页见 §13.3.5 与 §19.3.2，讲义母版见 §19.3.1.24。
- 现代批注见 MS-PPTX 2018/8（`p188:cmLst` / `p188:authorLst`）。包结构见 §9 与 §12.2。
- 不实现 Excel 计算引擎、Word 排版引擎、PowerPoint 放映引擎，也不实现图表渲染或 SmartArt 版式算法。不接受把整棵 `<p:timing>` 当 JSON 写入。
- 不支持 `.doc`、`.xls`、`.ppt`。
- XML 编辑是手术刀：不认识的命名空间、属性、子元素，只要没被当前 op 点到，就原样保留。

## 实现时允许的依赖（文档约定，代码阶段再写入 Cargo.toml）

库可以依赖常见的 ZIP 与 XML crate，例如 `zip`、`quick-xml`、`serde`、`serde_json`。xlsx / docx 写保护哈希可用 `sha2`、`getrandom`、`base64`。
禁止依赖任何把整份 Office 文档建成自己那套对象模型、再整包写出的高层 Office crate。

## 阶段边界

xlsx 与 docx 第一期已做完，人工打开见 [checklist-xlsx.md](checklist-xlsx.md)、[checklist-docx.md](checklist-docx.md)。
pptx 第一期实现已落地，人工打开见 [checklist-pptx.md](checklist-pptx.md)。
