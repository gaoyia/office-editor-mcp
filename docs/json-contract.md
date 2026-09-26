# 领域 JSON 契约

`schemaVersion` 固定为 `1`。库和命令行只认这一层。MCP 或其他传输信封不许改这里的字段名和含义。机器可读副本见 [json-schema/request.schema.json](json-schema/request.schema.json)，与本文冲突时以本文为准。

xlsx、docx、pptx 的 inspect、create、apply 已经按本文实现。pptx 的 PowerPoint / WPS 演示人工打开见 [checklist-pptx.md](checklist-pptx.md)。

## 信封

每份请求都是一个 JSON 对象，必须含：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `schemaVersion` | 整数 | 必须是 `1` |
| `action` | 字符串 | `inspect`、`create`、`apply` 之一 |
| `locale` | 字符串 | 可选。错误 `message` 的语言偏好，见 [errors.md](errors.md) |

`locale` 不属于动作参数，三种动作都认。取值宽松匹配：`zh`、`zh-CN`、`zh-Hans`、`zh-SG` 出中文；`en` 及其地区变体出英文；不认识的标签（包括空串）退回英文，不报 `InvalidField`。这是信封里唯一一处对未知取值不报错的字段：语言只影响呈现，不该挡下文档操作。优先级是请求 `locale` 高于命令行 `--lang`，再高于环境变量 `OOXP_LANG`，都没有时是英文。

另外按动作需要 `path`、`format`、`scope`、`workbook`、`document`、`presentation`、`ops` 等字段，见下文。禁止在请求里放一整棵 inspect 树来表示「把文档改成这样」。create 的 xlsx 用 `workbook`，docx 用 `document`，pptx 用 `presentation`，三者不能同时出现。

`format` 取值：`xlsx`、`docx`、`pptx`。可省略，省略时从 `path` 的扩展名推断。推断不出就报 `UnsupportedFormat`。

`path` 是本地文件系统路径。Windows 反斜杠和正斜杠都接受。

## 稳定 id

id 是给模型用的字符串，必须能从文件内容稳定算出来，不能依赖本次运行的内存地址或随机数。

解析规则按格式走。id 里的工作表名、段落 id、形状 id 都按文件里的原文，不做 Unicode 正规化。

### xlsx

| 节点 | id | 例子 |
| --- | --- | --- |
| 工作簿 | `xlsx:workbook` | `xlsx:workbook` |
| 工作表 | `sheet:{name}` | `sheet:Sheet1`、`sheet:销售 2026` |
| 单元格 | `cell:{name}!{A1}` | `cell:Sheet1!A1`、`cell:销售 2026!BC12` |

`{name}` 是 `workbook.xml` 里 `<sheet name="...">` 的值，不是 `sheetId`，也不是 `r:id`。查找工作表时大小写不敏感，输出 id 时用文件里的原始大小写。

`{A1}` 是大写列字母加行号，例如 `A1`、`AA100`。请求里若写 `a1`，库先规范成 `A1` 再生成 id。

解析单元格 id：从右侧第一个 `!` 切开。右侧必须是合法 A1，左侧是工作表名。这样工作表名里即使含 `!`，也不会切错。

图表工作表、宏表出现在 inspect 的表名单里，但没有单元格 id。它们的 id 仍是 `sheet:{name}`。对它们发 `setCell` 报 `InvalidField`。

### docx

文档根节点 id 固定为 `docx:document`。

段落 id 优先用 `w14:paraId`（十六进制，MS-DOCX §2.6.2.3，命名空间 `http://schemas.microsoft.com/office/word/2010/wordml`）。有该属性时：`para:{paraId}`，例如 `para:3F2A1B00`。输出用文件里的原始大小写。请求里的十六进制按大小写不敏感匹配。

没有 `w14:paraId` 时：`para:index:{n}`，`n` 从 1 起，按主文档部件里 `w:p` 的文档顺序（含表格单元格里的段落）。树上同时给 `"stable": false`。这类 id 在增删段落后会变，只够同一次 inspect 之后立刻 apply。

标题和正文都是段落，不另编一套 id。页眉、页脚、脚注、尾注、批注、文本框里的段落第一期不进这棵树，也不接受 apply。

「文本框里的段落不算段落」这句要有可执行的判定，否则实现容易一路钻进去。边界按下面两条划：

- 遍历主文档时遇到 `w:txbxContent` 就停住，不再往下走。文本框里的 `w:p` 不计入 `paraCount`，不出现在 `outline` 与 `paragraphs` 里，它们的 `w:t` 也不并进外层段落的 `text`。拿文本框里的 `paraId` 发 apply，报 `IdNotFound`。
- 遇到 `mc:AlternateContent` 只走第一个 `mc:Choice`；没有 `mc:Choice` 才走 `mc:Fallback`。两支都走会把同一份内容读出两遍，也会让 apply 只改到其中一支、另一支不同步。这条对 `paraCount`、`outline`、`paragraphs`、`text` 和 apply 一律适用。

Word 2010 以后插入文本框、图表、SmartArt 都是 `mc:AlternateContent` 套 `w:txbxContent` 的写法，所以这两条不是边角情况。

同一主文档部件里若出现重复的 `w14:paraId`（违反 MS-DOCX，但现实文件可能有），inspect 照样列出每一段；apply 命中文档顺序中的第一段。

### pptx

演示文稿根节点 id 固定为 `pptx:presentation`。

| 节点 | id | 例子 |
| --- | --- | --- |
| 页 | `slide:{n}` | `slide:1` |
| 形状 | `slide:{n}/shape:{shapeId}` | `slide:1/shape:4` |
| 无 `cNvPr/@id` 的形状 | `slide:{n}/shape:index:{k}` | `slide:1/shape:index:2` |

`{n}` 从 1 起，按 Presentation 部件（§13.3.6）里 `p:sldIdLst` / `p:sldId` 的出现顺序（§19.2.1.34），再跟 `r:id` 找到幻灯片部件。`n` 不是文件名里的数字，也不是 `p:sldId/@id`（那是 `ST_SlideId`，常见从 256 起）。隐藏页仍占序号，仍可 inspect 和 apply。

`{shapeId}` 取该形状 `p:nvSpPr/p:cNvPr/@id` 的原文（§19.3.1.12）。输出用文件里的原始数字字符串。请求按十进制数值匹配，不因前导零而失败。

没有 `cNvPr/@id` 时用 `slide:{n}/shape:index:{k}`，`k` 从 1 起，按该页列入树的形状顺序。树上同时给 `"stable": false`。这类 id 在增删形状后会变，只够同一次 inspect 之后立刻 apply。Office 亲手存的文件几乎都有 `cNvPr/@id`。

同一页里若出现重复的 `cNvPr/@id`，inspect 照样列出每一处；apply 命中该页文档顺序中的第一处。

apply 不得删形状再重建，以免 `cNvPr/@id` 变化导致绑在形状上的动画失效。不改 `<p:timing>`。

备注页、讲义、幻灯片母版、版式第一期不进这棵树，也不接受 apply，对齐 docx 页眉页脚那条边界。

「文本形状」要有可执行的判定：

- 只收带 `p:txBody` 的 `p:sp`（§19.3.1.45、§19.3.1.51），含嵌在 `p:grpSp` 里的。标题占位符、正文占位符、自由文本框都算。
- 不收 `p:pic`、`p:cxnSp`、`p:graphicFrame`（表格、图表、SmartArt 等）、`p:contentPart`。组合本身不进树，只展开里面符合条件的 `p:sp`。
- 遇到 `mc:AlternateContent` 只走第一个 `mc:Choice`；没有 `mc:Choice` 才走 `mc:Fallback`。两支都走会把同一份内容读出两遍，也会让 apply 只改到其中一支。这条对 `shapes`、`text` 和 apply 一律适用。

## inspect

成功响应的对象字段按下面各节示例的语义顺序写出，不是按字母排序。命令行打印同一棵树，见 [cli.md](cli.md)。

### 请求

```json
{
  "schemaVersion": 1,
  "action": "inspect",
  "path": "C:\\data\\book.xlsx",
  "format": "xlsx",
  "scope": {
    "sheet": "Sheet1",
    "range": "A1:D20",
    "maxCells": 5000
  }
}
```

xlsx、docx、pptx 共用 `scope` 这个字段名，里面的键按格式分，混用报 `InvalidField`。

#### xlsx 的 scope

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `path` | 是 | 已有文件 |
| `scope` | 否 | 省略则只返回工作表名单，不含单元格 |
| `scope.sheet` | 要单元格时必填 | 工作表名 |
| `scope.range` | 要单元格时必填 | A1 区域，例如 `A1:D20`。单个单元格写成 `B3` 或 `B3:B3` |
| `scope.maxCells` | 否 | 返回单元格上限，默认 `5000`，允许范围 `1` 到 `20000` |

省略 `scope`、或只给空对象：返回工作簿节点和表名单，每张表不含 `cells`。这是默认截断。

只要想看单元格，就必须同时给 `sheet` 和 `range`。不默认扫第一张表的整张纸。

#### docx 的 scope

```json
{
  "schemaVersion": 1,
  "action": "inspect",
  "path": "C:\\data\\note.docx",
  "format": "docx",
  "scope": { "from": 1, "maxParas": 200 }
}
```

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `path` | 是 | 已有文件 |
| `scope` | 否 | 省略或空对象：只返回 `outline` 和 `paraCount`，没有 `paragraphs` |
| `scope.from` | 要段落正文时必填 | 从第几段开始，从 1 起，按主文档 `w:p` 顺序 |
| `scope.maxParas` | 要段落正文时必填 | 本次最多返回多少段，允许范围 `1` 到 `5000` |

只要想看段落正文，就必须同时给 `from` 和 `maxParas`。不默认把全文倒出来。`from` 超出 `paraCount` 时 `paragraphs` 为空数组，`truncated` 为 `false`，不报错。

#### pptx 的 scope

```json
{
  "schemaVersion": 1,
  "action": "inspect",
  "path": "C:\\data\\deck.pptx",
  "format": "pptx",
  "scope": { "slide": 1, "maxShapes": 200 }
}
```

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `path` | 是 | 已有文件 |
| `scope` | 否 | 省略或空对象：只返回 `slideCount` 和页名单，没有 `shapes` |
| `scope.slide` | 要形状时必填 | 页序号，从 1 起，按 `p:sldIdLst` 顺序 |
| `scope.maxShapes` | 否 | 该页最多返回多少个文本形状，默认 `2000`，允许范围 `1` 到 `5000` |

只要想看形状，就必须给 `slide`。不默认把每一页的文本框都倒出来。只给 `maxShapes`、不给 `slide` 报 `ScopeRequired`。`slide` 小于 `1` 报 `InvalidField`。`slide` 超出 `slideCount` 时各页都不带 `shapes`，不报错。

### 响应（xlsx）

```json
{
  "schemaVersion": 1,
  "format": "xlsx",
  "id": "xlsx:workbook",
  "sheets": [
    {
      "id": "sheet:Sheet1",
      "name": "Sheet1",
      "kind": "worksheet",
      "truncated": true,
      "range": "A1:D20",
      "cells": [
        {
          "id": "cell:Sheet1!A1",
          "ref": "A1",
          "type": "number",
          "value": 1,
          "formula": null
        },
        {
          "id": "cell:Sheet1!B1",
          "ref": "B1",
          "type": "text",
          "value": "hello",
          "formula": null
        },
        {
          "id": "cell:Sheet1!C1",
          "ref": "C1",
          "type": "number",
          "value": 6,
          "formula": "=SUM(A1:A10)"
        }
      ]
    },
    {
      "id": "sheet:Sheet2",
      "name": "Sheet2",
      "kind": "worksheet"
    }
  ]
}
```

规则：

- `sheets` 始终列出工作簿里全部表（含隐藏表），便于模型知道还有哪些表。未点名的表不带 `cells`、`range`、`truncated`。
- `kind`：`worksheet`、`chartsheet`、`dialogsheet`。第一期只对 `worksheet` 填单元格。
- `cells` 稀疏：范围内没有 `<c>` 的格子不出现。
- 范围内有内容的格子超过 `maxCells` 时，按行优先、从左到右截断，`truncated` 为 `true`。未截断则为 `false`。
- `type`：`number`、`text`、`boolean`、`error`、`date`、`empty`。对应 SpreadsheetML `ST_CellType`（§18.18.11）的映射见下表。
- `value` 是缓存显示值，不是库算出来的。公式格的 `value` 来自 `<v>`，可能过期。没有 `<v>` 时 `value` 为 `null`。
- `formula`：有公式则返回以 `=` 开头的字符串。XML 里 `<f>` 不含 `=`，由库补上。没有公式则为 `null`。
- 共享公式（`<f t="shared">`）：主单元格返回其公式文本；从属单元格若 XML 里没有文本，第一期 `formula` 为 `null`，并多一个 `"shared": true`。以后可以改成展开成该格的 A1 公式，不改 id 规则。
- 内联字符串（`t="inlineStr"`）和共享字符串（`t="s"`）在树上都是 `type: "text"`，`value` 为拼接后的可见纯文本，不含 run 级样式。`t="s"` 的 `<v>` 必须是共享字符串表里的合法非负整数索引。越界或无法解析报 `XmlBroken`，不要当成空串。

类型映射：

| XML `t` | 树上 `type` | `value` JSON 类型 |
| --- | --- | --- |
| 缺省或 `n` | `number` | 数字。解析失败则当 `text` 并把原文放进去 |
| `s` 或 `inlineStr` 或 `str` | `text` | 字符串 |
| `b` | `boolean` | `true` / `false`（XML 里 `1`/`0`） |
| `e` | `error` | 字符串，例如 `"#DIV/0!"` |
| `d` | `date` | 字符串，ISO 8601 原文 |
| 有 `r` 但无 `v`/`is`/`f` | `empty` | `null` |

日期序列值（`t` 缺省、单元格看起来像日期）第一期仍标 `number`，不根据样式猜日期。这是有意的，避免做半套日期引擎。

### 响应（docx）

```json
{
  "schemaVersion": 1,
  "format": "docx",
  "id": "docx:document",
  "paraCount": 4,
  "outline": [
    {
      "id": "para:3F2A1B00",
      "index": 1,
      "level": 1,
      "style": "Heading1",
      "text": "标题"
    }
  ],
  "from": 1,
  "truncated": false,
  "paragraphs": [
    {
      "id": "para:3F2A1B00",
      "index": 1,
      "style": "Heading1",
      "level": 1,
      "text": "标题",
      "stable": true
    },
    {
      "id": "para:index:2",
      "index": 2,
      "style": "Normal",
      "text": "hello",
      "stable": false
    }
  ]
}
```

规则：

- 只读主文档部件（根关系里 `/officeDocument` 那条，常见路径 `/word/document.xml`）。页眉页脚、脚注、批注不读。
- `paraCount` 是主文档里 `w:p` 的总数，含表格单元格里的段落，不含文本框里的段落，始终出现。
- `outline` 始终出现。收入满足下面任一条件的段落：`w:pStyle/@w:val` 为 `Heading1`–`Heading9`（大小写不敏感）或 `标题1`–`标题9`；或者 `w:outlineLvl/@w:val` 为 `0`–`8`。树上 `level` 从 1 到 9。`Heading1` / `标题1` / `outlineLvl=0` 都是 `level: 1`。不是标题的段落不进 `outline`。
- 省略 `scope` 时没有 `paragraphs`、`from`、`truncated`。
- 给了 `from` 和 `maxParas` 时，`paragraphs` 按文档顺序切片。范围内段落数超过 `maxParas` 则 `truncated` 为 `true`。
- `style` 取 `w:pStyle/@w:val` 的原文。没有 `pStyle` 则为 `null`（Word 此时按 Normal 显示，树上不猜）。
- `text` 是可见纯文本：该 `w:p` 内、不在 `w:del` 里的 `w:t` 按顺序拼接；`w:tab` 变成 `\t`，`w:br` 变成 `\n`。`w:instrText`（域指令）不进入。不含 run 级样式。嵌在 `w:txbxContent` 里的 `w:t` 不算，`mc:AlternateContent` 只取一支，判定见上文 docx 那节的边界两条。
- `stable`：有 `w14:paraId` 为 `true`，否则为 `false`。
- 第一期不返回修订痕迹和目录域。文本框里的段落既不作为独立节点出现，也不并进外层段落的 `text`。含超链接或域的段落仍出现在树上，`text` 只含可见结果。

### 响应（pptx）

```json
{
  "schemaVersion": 1,
  "format": "pptx",
  "id": "pptx:presentation",
  "slideCount": 2,
  "slides": [
    {
      "id": "slide:1",
      "index": 1,
      "name": null,
      "hidden": false,
      "truncated": false,
      "shapes": [
        {
          "id": "slide:1/shape:2",
          "shapeId": "2",
          "name": "文本框 1",
          "text": "标题",
          "stable": true
        },
        {
          "id": "slide:1/shape:3",
          "shapeId": "3",
          "name": "文本框 2",
          "text": "hello",
          "stable": true
        }
      ]
    },
    {
      "id": "slide:2",
      "index": 2,
      "name": null,
      "hidden": false
    }
  ]
}
```

规则：

- 只读 Presentation 部件和被点名的幻灯片部件。备注、讲义、母版、版式不读。
- `slideCount` 是 `p:sldIdLst` 里 `p:sldId` 的个数，始终出现。
- `slides` 始终列出全部页（含隐藏页）。未点名的页不带 `shapes`、`truncated`。
- `name` 取 `p:cSld/@name` 的原文。没有该属性则为 `null`。
- `hidden`：`p:sld/@show` 为 `0` 或 `false` 时为 `true`，其余为 `false`。
- 省略 `scope` 或只给空对象时，没有 `shapes`、`truncated`。
- 给了 `scope.slide` 时，只给那一页填 `shapes`。该页带 `p:txBody` 的 `p:sp` 超过 `maxShapes` 则 `truncated` 为 `true`，按该页文档顺序截断。未截断则为 `false`。
- `scope.slide` 大于 `slideCount` 时，各页都不带 `shapes`，不报错。
- `shapeId` 是 `cNvPr/@id` 的原文。走 `shape:index:{k}` 时为 `null`。
- 形状上的 `name` 取 `cNvPr/@name` 的原文。没有则为 `null`。
- `text` 是可见纯文本：该形状 `p:txBody` 内、位于 `a:r` 里的 `a:t` 按文档顺序拼接；`a:br` 变成 `\n`。不含 run 级样式。`a:fld` 是应用程序更新的生成文本（§21.1.2.2.4），域里的 `a:t` 是缓存，不进入。`mc:AlternateContent` 只取一支，判定见上文 pptx 那节。
- `stable`：有 `cNvPr/@id` 为 `true`，否则为 `false`。
- 第一期不返回动画、切换、备注、母版、版式。图片和图表不作为节点出现。

## create

只有 create 允许从 JSON 生成整包。

### xlsx

```json
{
  "schemaVersion": 1,
  "action": "create",
  "path": "C:\\out\\book.xlsx",
  "format": "xlsx",
  "workbook": {
    "sheets": [
      {
        "name": "Sheet1",
        "cells": [
          { "ref": "A1", "value": 1 },
          { "ref": "A2", "value": "hello" },
          { "ref": "A3", "value": true },
          { "ref": "B1", "formula": "=SUM(A1)" }
        ]
      }
    ]
  }
}
```

| 字段 | 含义 |
| --- | --- |
| `path` | 输出路径。已存在则覆盖。省略时必须由调用方另给一个落盘路径，命令行用 `--output` |
| `workbook.sheets` | 至少一张表 |
| `sheets[].name` | 工作表名。Excel 限制：最长 31 个字符，不含 `:*?/[]\\`，不能叫 `History`，同一工作簿内不重复 |
| `cells[].ref` | A1 地址 |
| `cells[].value` | 数字、字符串、布尔、或 `null`。`null` 表示不写这个格 |
| `cells[].formula` | 以 `=` 开头的公式文本。库不求值 |

`value` 和 `formula` 不能在同一格同时出现。要公式就只写 `formula`。公式格不写缓存 `<v>`，并在 `workbook.xml` 的 `calcPr` 上设 `fullCalcOnLoad="1"`，让 Excel 打开时重算。

文本写入共享字符串表（`t="s"`），这是 Office 新建文件的常见写法。每个文本值必须先进入该表再写索引；表里找不到对应项时 create 失败，不许退回索引 0。布尔写成 `t="b"`。数字不写 `t`（默认为 `n`）。

行与单元格按行号、列号升序写出。补上 `sheetData` 和合理的 `dimension`。

最小 `styles.xml` 必须生成，见 [opc.md](opc.md)。默认字体颜色写 `<color rgb="FF000000"/>`，不引用 theme，包里也不生成 theme 部件。不要生成图表、透视表、条件格式。

### docx

```json
{
  "schemaVersion": 1,
  "action": "create",
  "path": "C:\\out\\note.docx",
  "format": "docx",
  "document": {
    "paragraphs": [
      { "style": "Heading1", "text": "标题" },
      { "text": "hello" }
    ]
  }
}
```

| 字段 | 含义 |
| --- | --- |
| `path` | 输出路径。已存在则覆盖。省略时必须由调用方另给落盘路径，命令行用 `--output` |
| `document.paragraphs` | 段落列表。可空；空则生成一个空的 `w:p`，否则 Word 可能打不开 |
| `paragraphs[].text` | 可见文本，可空字符串。省略当作 `""` |
| `paragraphs[].style` | 段落样式 id。省略或 `Normal` 表示正文。第一期只允许 `Normal`、`Heading1`、`Heading2`、`Heading3`（大小写不敏感，写出时用这里的 canonical 形式） |

create 写出 Transitional 最小包，见 [opc.md](opc.md)。每个 `w:p` 都带唯一的 `w14:paraId` 和 `w14:textId`（取值大于 0、小于 `0x80000000`，MS-DOCX §2.6.2.3 / §2.6.2.4），并在根元素上把 `w14` 列入 `mc:Ignorable`。这样随后 inspect 得到的 id 都是 `para:{paraId}`，`stable` 为 `true`。

标题段写出 `<w:pStyle w:val="Heading1"/>` 这类引用，样式定义放在 `/word/styles.xml`。同时写 `/word/settings.xml`，含 `<w:compatSetting w:name="compatibilityMode" w:uri="http://schemas.microsoft.com/office/word" w:val="15"/>`（MS-DOCX §2.3.5）。缺这项时 Word 按兼容模式打开（默认 12）。不要生成页眉页脚、目录、修订。

### pptx

```json
{
  "schemaVersion": 1,
  "action": "create",
  "path": "C:\\out\\deck.pptx",
  "format": "pptx",
  "presentation": {
    "slides": [
      {
        "shapes": [
          { "text": "标题" },
          { "text": "hello" }
        ]
      },
      {
        "shapes": [
          { "text": "第二页" }
        ]
      }
    ]
  }
}
```

| 字段 | 含义 |
| --- | --- |
| `path` | 输出路径。已存在则覆盖。省略时必须由调用方另给落盘路径，命令行用 `--output` |
| `presentation.slides` | 页列表。可空；空则生成一页，页上有一个空文本框，否则 PowerPoint 可能打不开 |
| `slides[].shapes` | 该页文本框列表。可空；空则生成一个空文本框 |
| `shapes[].text` | 可见文本，可空字符串。省略当作 `""`。`\n` 写成 `a:br` |

`workbook`、`document`、`presentation` 三者必须恰好出现一个。xlsx 路径必须用 `workbook`，docx 路径必须用 `document`，pptx 路径必须用 `presentation`，对不上报 `InvalidField`。

create 写出 Transitional 最小包，见 [opc.md](opc.md)。必须带 theme、slideMaster、slideLayout、presentation、各页 slide、presProps，不能只写 ECMA 示例里那个仅含 `notesSz` 的 presentation。幻灯片尺寸用 16:9（`sldSz` 为 `cx="12192000"` `cy="6858000"`）。不要生成动画、切换、备注。

每个文本框写成带 `p:txBody` 的 `p:sp`，`p:cNvPr/@id` 从 `2` 起在该页内递增（`1` 留给 `spTree`）。随后 inspect 得到的 id 都是 `slide:{n}/shape:{id}`，`stable` 为 `true`。形状纵向错开，避免叠在同一位置。颜色用 RGB，不依赖主题色语义。

## apply

apply 的入参是操作列表，不是树。下面这种写法非法，必须报 `InvalidField`：

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "book.xlsx",
  "sheets": [],
  "cells": []
}
```

合法请求：

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\book.xlsx",
  "output": "C:\\data\\book.patched.xlsx",
  "ops": [
    { "op": "setCell", "id": "cell:Sheet1!A1", "value": 42 },
    { "op": "setCell", "id": "cell:Sheet1!B1", "formula": "=SUM(A1:A10)" },
    { "op": "setCell", "id": "cell:Sheet1!C1", "value": null }
  ]
}
```

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `path` | 是 | 源文件 |
| `output` | 否 | 省略则覆盖 `path` |
| `ops` | 是 | 非空数组。按数组顺序执行，中途失败则整次 apply 不写文件 |

create 与 apply 落盘时先写入目标同目录的临时文件，再改名为目标路径。这样写到一半失败时，原文件还在。

### setCell

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setCell"` |
| `id` | 单元格 id |
| `value` | 与 create 相同。`null` 表示清空该格的值和公式 |
| `formula` | 以 `=` 开头。不能与 `value` 同时出现 |

不认识的 `op` 报 `UnsupportedOp`，不要忽略。第一期 xlsx 只认 `setCell`，docx 只认 `setParaText`，pptx 只认 `setShapeText`。对不上格式的 op（例如对 pptx 发 `setCell` / `setParaText`，或对 xlsx / docx 发 `setShapeText`）报 `InvalidField`。

空单元格：目标格子在 XML 里还不存在时，`setCell` 按需插入 `<row>` 和 `<c>`，插在正确的行列顺序上。找不到工作表才报 `IdNotFound`。

省略 `r` 的行和单元格：SpreadsheetML 允许 `<row>` 与 `<c>` 不写 `r`，编号由出现顺序推出。apply 在改这张表之前，先把这类隐式编号按顺序算出来写成显式 `r`。不这样做，后面插入新行会让它们的编号整体漂移，等于悄悄改了别的数据。这是语义等价的改写，Office 打开看不出区别。

共享公式：主单元格的 `<f>` 带 `ref` 和 `si`，从属单元格只带 `si`。对**主单元格**发 `setCell`（写值、写公式、清空都算）会让从属单元格的 `si` 失去定义，Office 打开时报需要修复，因此第一期直接报 `NotImplemented`，不写文件。想改主单元格，先把同组的从属单元格逐个改成独立公式。对**从属单元格**发 `setCell` 是允许的，主单元格不受影响。库不做公式引用平移，这属于计算引擎的活。

清空：去掉 `<f>`、`<v>`、`<is>`，保留 `<c>` 上的 `r`、`s` 以及不认识的属性。这样常见数字格式和字体还能在。若清空后 `<c>` 只剩 `r`（没有 `s` 也没有其他属性），可以把整个 `<c>` 删掉。目标格在 XML 里还不存在时，清空是空操作，仍计入 `opsApplied`；这不是错误，因为 `setCell` 对空格子本来就不报 `IdNotFound`。

改已有格子：保留 `s` 和未知属性。把文本从共享字符串改成新文本时，写成 `t="inlineStr"`，不要去改 `sharedStrings.xml` 里的旧项。改共享字符串会牵动所有引用同一索引的格子。

数字、布尔按类型改 `t` 和 `<v>`。写入公式时：`<f>` 存去掉前导 `=` 的文本，`t` 若原先是 `s`/`inlineStr`/`b` 则去掉，删除 `<v>` 和 `<is>`。

### 重算标记

任意一条 `setCell` 成功之后，必须让打开文件的 Excel 重算。第一期用这两步，两步都做：

1. 在 `/xl/workbook.xml` 的 `<calcPr>` 上设置 `fullCalcOnLoad="1"`。没有 `<calcPr>` 就建一个。其余 `calcPr` 属性保留。依据 ECMA-376 §18.2.2。
2. 若存在 `/xl/calcChain.xml`，删除该部件、内容类型 Override、以及 workbook 关系里指向它的那条。过期计算链会让部分 Excel 版本沿用旧顺序。

不在库里求值，不写新的缓存 `<v>`。

### 改动范围

只允许变脏的部件：

- 目标工作表 XML
- `/xl/workbook.xml`（仅 `calcPr`，以及给隐式编号补 `r` 这类语义等价的改写）
- 可能被删除的 `/xl/calcChain.xml` 以及因此修改的关系与内容类型

`styles.xml`、其他工作表、主题、绘图一律原样写回，字节不变。

被改的部件是整份重新序列化的，不是按字节做定点替换。因此保证到这个程度：XML 声明、元素与属性的顺序、未知命名空间的元素与属性、文本内容都保留；注释、处理指令和 DOCTYPE 会丢，CDATA 会变成等价的转义文本。Office 的工作表部件不带这些东西，所以第一期接受这个代价。

### setParaText（docx 第一期唯一的 op）

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\note.docx",
  "output": "C:\\data\\note.patched.docx",
  "ops": [
    { "op": "setParaText", "id": "para:3F2A1B00", "text": "新标题" }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setParaText"` |
| `id` | 段落 id（`para:…` 或 `para:index:n`） |
| `text` | 新的可见纯文本，字符串，可空。必填 |

找不到该段落报 `IdNotFound`。第一期不插入、不删除段落。

替换规则（手术刀，不整篇重建 `document.xml`）：

1. 只改命中的那一个 `w:p`。`w:pPr`、书签、绘图、`w:hyperlink` 外壳、不认识的子元素都留着。
2. 在该段里找出不位于 `w:del` 中的、含 `w:t` 的 run。
3. 若一个都没有：在 `w:pPr` 之后插入一个新的 `w:r`/`w:t`，文本放进去。
4. 若有：把完整新文本写入其中第一个 `w:t`（需要保留首尾空格时加 `xml:space="preserve"`）；其余这类 `w:t` 删掉；因此变空、且里面再也没有别的 run 内容的 `w:r` 删掉。第一个 run 上已有的 `w:rPr` 保留。
5. 含超链接或域的段落仍按上面做。超链接可能不再包住整段新字，域可能被拆掉。验收只用无域、无超链接的段落。页眉页脚里的段落第一期改不到，id 若不在主文档里就 `IdNotFound`。
6. 不往文本框里写字。第 2 步找 run 的时候，遇到 `w:txbxContent` 停住，遇到 `mc:AlternateContent` 只进第一个 `mc:Choice`（没有就进 `mc:Fallback`），判定与 inspect 那两条边界一致。某一段除文本框之外没有可改的 `w:t` 时走第 3 步插新 run，插在 `w:pPr` 之后、该段其余内容之前。这样 apply 写进去的文本，随后 inspect 读回来必须与写入值相同。

### 改动范围（docx）

只允许变脏的部件：主文档 XML（通常是 `/word/document.xml`）。`styles.xml`、`settings.xml`、页眉页脚、主题、编号一律原样写回。

被改部件同样整份重序列化，保留程度与 xlsx 工作表相同：元素与属性顺序、未知命名空间保留；注释、处理指令、DOCTYPE 会丢。

### setShapeText（pptx 第一期唯一的 op）

```json
{
  "schemaVersion": 1,
  "action": "apply",
  "path": "C:\\data\\deck.pptx",
  "output": "C:\\data\\deck.patched.pptx",
  "ops": [
    { "op": "setShapeText", "id": "slide:1/shape:2", "text": "新标题" }
  ]
}
```

| 字段 | 含义 |
| --- | --- |
| `op` | 必须是 `"setShapeText"` |
| `id` | 形状 id（`slide:{n}/shape:{shapeId}` 或 `slide:{n}/shape:index:{k}`） |
| `text` | 新的可见纯文本，字符串，可空。必填。`\\n` 写成 `a:br`，不把字面换行两个字符写进 `a:t` |

找不到该形状报 `IdNotFound`。第一期不插入、不删除形状。对 `slide:{n}`（没有 `/shape:`）发本 op 报 `InvalidId`。

替换规则（手术刀，不整页重建 `p:sp`，更不整份重建演示文稿）：

1. 只改命中的那一个 `p:sp` 的 `p:txBody` 可见文字。`p:nvSpPr`、`p:spPr`（几何与位置）、`p:style`、不认识的子元素都留着。禁止删掉该形状再新建一个。
2. 不改同页的 `<p:timing>`、`<p:transition>`，也不改其他形状。重序列化该页时这两节必须作为未点名节点原样保留。
3. 在该 `p:txBody` 里找出含 `a:t` 的 run（`a:r`）。遇到 `mc:AlternateContent` 只进第一个 `mc:Choice`（没有就进 `mc:Fallback`）。
4. 若一个都没有：在现有 `a:bodyPr` / `a:lstStyle` 之后插入 `a:p`/`a:r`/`a:t`，文本放进去；已经有段落时挂在第一段末尾，不凭空多出一段。没有 `p:txBody` 则本 op 报 `IdNotFound`（第一期不给无文本框的形状补 `txBody`）。
5. 若有：第一个这样的 run 收下全部新文本。第一行写进它的第一个 `a:t`（需要保留首尾空格时加 `xml:space="preserve"`），该 run 里多余的 `a:t` 删掉。`text` 里每遇一个 `\n`，就在**段落**里紧接着插一个 `a:br` 加一个新的 `a:r`，新 run 沿用第一个 run 的 `a:rPr`。
6. 换行只能拆在段落这一级：`a:br` 与 `a:r` 平级、同为 `a:p` 的子元素（`EG_TextRun`，§A.4），而 `CT_RegularTextRun` 只允许一个可选 `a:rPr` 加**恰好一个** `a:t`。所以 `a:br` 不许塞进 `a:r` 里，一个 `a:r` 也不许带多个 `a:t`，两者都会生成不合 schema 的 run。
7. 该 `p:txBody` 里其余的可见内容一律清掉：别的 `a:t` 删掉，**所有 `a:br` 删掉**，因此变空、且里面再也没有别的 run 内容的 `a:r` 删掉。`a:br` 必须一起清，因为 inspect 把它读成 `\n`，留一个下来读回值就比写入值多一个换行，与本节最后一条冲突。收下新文本的那一段原地保留，它自己的 `a:pPr`、`a:endParaRPr` 不许删，也不许换成别段的。清完之后，段落里若只剩下 `a:pPr`、`a:endParaRPr`（没有 `a:r`、`a:br`、`a:fld` 以及其它子元素），把这段 `a:p` 删掉。`CT_TextBody` 的 `a:p` 至少留一段（§A.4，`minOccurs="1"`）。不许把整棵 `txBody` 的段落删光再重建，也不许把别段的 `a:pPr` 搬到写入段上。空段要删，是因为 §21.1.2.2.6 把 `a:p` 定为文本框里最高一层分隔，PowerPoint 会把空段画成空行；inspect 的 `text` 又不在段落之间插入 `\n`。含 `a:fld` 的段按第 8 条不清域，因此也不会被本条删掉。
8. `a:fld` 是应用程序定期更新的生成文本（§21.1.2.2.4），不是用户打的字。apply 不改、不删域里的节点。inspect 的 `text` 也不收域里的 `a:t`。写进去的用户文本因此能原样读回；屏幕上域的缓存字（页码、日期）仍可能跟在旁边。第一期不把域做成独立节点。
9. apply 写进去的文本，随后 inspect 读回来必须与写入值相同。

### 改动范围（pptx）

只允许变脏的部件：被改的那一张幻灯片 XML（通常是 `/ppt/slides/slideN.xml`，N 以关系目标为准）。`presentation.xml`、其他页、母版、版式、主题、备注、`presProps` 一律原样写回。

被改部件同样整份重序列化，保留程度与 xlsx 工作表相同：元素与属性顺序、未知命名空间保留；注释、处理指令、DOCTYPE 会丢。

## 两层 JSON

领域 JSON 就是本文。传输层（将来的 MCP）可以把本文整个对象放进 RPC 的 `params`。不许把 JSON-RPC 的 `jsonrpc`、`method`、`id` 塞进领域对象，也不许让单元格字段去迁就 RPC。
