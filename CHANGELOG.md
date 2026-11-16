# 变更记录

从 0.1.3 起，每次发布在这里写这一版改了什么。更早的版本不补。

## 0.1.7

- 新增只读动作 `exportChartImage`。按图表 id 把 xlsx 簇状柱形图画成 PNG。不改包，像素不进 inspect。
- 新增只读动作 `exportSparklineImage`。按迷你图组 id 把一条折线画成 PNG。不改包，像素不进 inspect。
- 新增只读动作 `exportSheetImage`。按工作表 id 把已用区域画成 PNG。格子按 Excel 复制为图片的 100% 尺寸，不乘工作表缩放。不改包，像素不进 inspect。
- 新增只读动作 `exportSlideImage`。按幻灯片 id 把页底、PNG、矩形、圆角矩形、椭圆、两色渐变、形状文字、簇状柱形图和纯色单元格的表写成图。画不了的对象让整页失败。不改包，像素不进 inspect。
- 新增只读动作 `exportPageImage`。按页 id 把没有字的 Word 页写成 PNG。有字、表格和图还不画。不改包，像素不进 inspect。
- `exportSheetImage` 与 Excel 16.0 差 0 的范围包括：关掉网格、隐藏行列、没有文字时的从右到左翻转、图案、虚线双线和点划线、主题色亮度、没有描边的纯色矩形和不缩放的 PNG、整像素裁剪、纯色组合的缩放旋转和翻转、两层组合、公式条件格式的纯色和已对齐边框、非负柱形迷你图。斜线边框、单元格渐变、图标集、负数数据条和要缩放的裁剪仍不画。
- `exportSlideImage` 与 PowerPoint 16.0 差 0 的范围补上母版形状、整像素裁剪、没有颜色变换的主题色矩形和页底，以及纯色组合的旋转、翻转和两层嵌套。描边外角、线条、连接符、渐变页底和图片页底仍不画。
- `exportChartImage` 在图内没有数值缓存时改读格子。簇状柱形图的字形灰度仍未与 Excel 差 0。
- `create` 和会改公式的 `apply` 在写完后求值，能算的写入缓存 `<v>`。算不了的不写新缓存，也不删已有的 `<v>`。可以新建数据表公式。
- 求值覆盖逻辑、信息、数学与三角、文本、日期、统计、查找与引用、数据库、财务和工程里有 Excel 16.0 样本的函数。没有连接的多维数据集函数写入 `#N/A`。
- 依赖都能排进顺序时写出 `/xl/calcChain.xml`。有环、数据表、解析不了，或用了 `INDIRECT` / `OFFSET`，就删掉这条链。
- 图表系列可以写三维引用，或能落到格子上的定义名。求出格子后重写数值缓存。
- 能确定的透视版式会把合计写进 `sheetData`。多个列字段再同时加多个行字段或多个值时，仍留给 Excel。`GETPIVOTDATA` 按已经写入的透视格子回答。
- inspect 的条件格式在能判断时带 `hits`。数据验证在能判断时带 `results`。这两项不写回文件。
- 迷你图在范围是一张表上的连续区域时，inspect 带 `values`，取已经写入的格子缓存。
- xlsx 顶层批注可以改 `visible`、`width`、`height`。按格子的行、列找到 Note。
- 窗体控件可以 `setFormControl`、`insertFormControl`、`deleteFormControl`。认按钮、复选框和下拉。
- 只有 `legacyDrawingHF` 的旧式页眉图可以单独查看，并用 `replaceLegacyPicture`、`deleteLegacyPicture` 换图或删除。
- 工作表背景用 `setSheetBackground`、`deleteSheetBackground`。媒体走 picture 关系，不进 drawing。
- `setCell` 的 limits 写明：不要把共享公式的从属格当成主格来写。
- 说明分成中文 `README.zh-CN.md` 和英文 `README.md`，并换上三份演示示例。

## 0.1.6

- `help` 里 xlsx 单元格的 `font`、`fill`、`border`、`align` 指向 `cellFont`、`cellFill`、`cellBorder`、`cellAlign`，不再指向整条 `setCellStyleOp`。
- `shapePaint` 按 `solid`、`grad`、`blip` 分成三支。`grad` 不收 `color`，`blip` 不收 `alpha`。
- `help` 写明：docx 图表省略 `kind` 时为 `anchor`，绕排为 `square`。表的 `style` 是 `style:` 加上 `styleId`。`after` 从 1 起，`0` 表示插在第一段之前。
- `createRequest` 的 `required` 包含 `path`。
- `ooxp --help` 写明 `mcp`、stdout 与 stderr。新增 `ooxp --version`。

## 0.1.5

- `help` 不带参数时，`ask` 是可原样提交的完整请求，每条都带 `action`。
- `help` 打开定义或操作时，单独的 `$ref` 只展开一层，里面的 `$ref` 仍只留定义名。完整请求 schema 在 `docs/json-schema/request.schema.json`。
- create 的帮助写明：目标路径已存在则覆盖。改已有内容仍用 `apply`。
- `setParaText` 与 `setShapeText` 的 limits 写明：只留第一个有字的 run 上已有的字符格式，其余 run 清掉。
- pptx create 的 limits 写明：`slides` 为空则生成一页空文本框，某一页 `shapes` 为空则生成一个空文本框。create 不写画布尺寸，建好后用 `setSlideSize`。
- pptx 形状 id 写明：该宿主 `p:spTree` 的 `cNvPr/@id` 是 `1`，不是形状。新建形状至少从 `2` 起。
- xlsx create 单元格的 `font`、`fill`、`border`、`align` 与 `setCellStyle` 使用同一份字段。填充色的字段名是 `fg`。
- 形状的 `align`、`textAnchor`、`underline`、`prst` 在 schema 里是封闭词表。连接线的几何名不在形状 `prst` 里。
- inspect 的 limits 写明：不带 `scope` 只返回名单。xlsx 公式格的 `value` 是包内缓存，没有缓存则为 `null`，库不算。
- `cellColor.rgb` 写明：6 位或 8 位，写入时改为大写，6 位前面补 `FF`。
- docx 段落的 `num` 写明：从 1 起，指向 `document.numberings` 的下标。
- 对图表工作表带 `sheet` 和 `range` 读单元格时，错误指出改用 `scope.charts`，不要带 `range`。
- pptx 建表说明写明：格子对象没有字号、颜色、加粗。改文字用 `setTableCellText`，改字色用 `setShapeFormat`。
- README 写明预编译包在 GitHub Releases，Windows 用 `Get-FileHash` 对照 `SHASUMS256.txt`。没有 `win32-arm64`。`--lang` 省略时错误文案是英文。

## 0.1.4

- 已从 crates.io 撤回，由 0.1.5 代替。变更内容见上一节。

## 0.1.3

- `help` 的 `name` 为 `inspectScope` 且带了 `format` 时，只返回该格式认的键。不带 `format` 时仍返回 `chooseFormat`，不给字段表。
- `apply` 失败时，错误可以带 `objectId` 和 `objectName`。`objectId` 是该条操作的 `id`。`objectName` 只在该条操作顶层有字符串 `name` 时出现。`path` 仍指向入参字段。
- `apply` 可以写 `dryRun: true`。各条操作仍在内存里执行，成功时返回 `opsApplied` 和标脏的部件名 `parts`，不写盘。
- 新增只读动作 `layoutCheck`。它检查 pptx 每页形状的包围盒、越界、重叠和文字溢出估算，不写文件，也不生成图片。
- pptx 的 `setPageBackground` 可以写本页背景，纯色、两停点渐变或图片。`color` 为 `null` 时删掉背景。
- pptx 的 `setShapeFill` 可以写透明度、渐变和图片填充。
- pptx 的 `insertShape` 和 `insertPicture` 可以用 `index`、`before` 或 `after` 指定图层。create 的图片可以用 `index`。
- pptx 形状的坐标可以为负。连接线和图片的坐标仍从 0 起。
- pptx 的 create 可以在 `shapes` 里直接写预设几何、坐标、尺寸、填充和线条。这些都省略时，缺省文本框与上一版字节相同。
- 命令行压缩包的根上直接是可执行文件和 `LICENSE`，不再套同名目录。Release 另外附上同一份 `LICENSE`。`SHASUMS256.txt` 包含这份许可证和各个压缩包。
