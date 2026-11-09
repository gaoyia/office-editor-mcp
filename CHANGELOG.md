# 变更记录

从 0.1.3 起，每次发布在这里写这一版改了什么。更早的版本不补。

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
