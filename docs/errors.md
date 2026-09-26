# 错误

库用一个错误枚举表达失败。命令行把同一结构打成 JSON，写到 stderr，并以非零退出码结束。成功时 stdout 只放约定的成功载荷，不混进错误对象。

## 输出形状

```json
{
  "ok": false,
  "error": {
    "code": "IdNotFound",
    "message": "Sheet not found: Sheet9",
    "path": "ops[0].id"
  }
}
```

`code` 是稳定字符串，给程序认。取值与退出码见下表，不随语言变。`message` 是给人看的短句，按语言渲染，默认英文。`path` 是可选的 JSON 指针风格路径，指向入参里出错的字段；定位不到就省略。

`Io` 错误还可以带可选字段 `osMessage`，放操作系统给的原文。它的语言由系统决定，我们不承诺是哪一门，也不要用它做字符串匹配。只有系统确实给了原文时才出现。

```json
{
  "ok": false,
  "error": {
    "code": "Io",
    "message": "Cannot access \"C:\\nope\\missing.xlsx\" (os error 3)",
    "osMessage": "系统找不到指定的路径。"
  }
}
```

## 语言

`message` 的语言按下面的优先级决定。渲染发生在最外层，库的 `run()` 仍返回语言中立的错误。

| 优先级 | 来源 | 形式 |
| --- | --- | --- |
| 1 | 请求 | 可选字段 `locale` |
| 2 | 命令行 | `--lang` |
| 3 | 环境变量 | `OOXP_LANG` |
| 4 | 默认 | `en` |

第一期只做 `en` 与 `zh-CN`。语言标签宽松匹配：先按连字符切开取首段，再看后面的地区或字形段。`zh`、`zh-CN`、`zh-Hans`、`zh-SG` 归到中文；`en`、`en-US`、`en-GB` 归到英文；`ja`、空串、乱码以及其它不认识的标签都退回英文，**不报错**。语言是呈现偏好，为了一个不认识的标签把整个文档操作拒掉，代价不成比例。

`Error` 的 `Display` 恒用英文。这是 Rust 的 `Display` 不带参数导致的：`unwrap()` 的 panic 和上层 `anyhow` 打印都是英文。要中文请走 `to_json(lang)`。

## 错误码

| code | 何时出现 | CLI 退出码 |
| --- | --- | --- |
| `InvalidJson` | 入参不是 JSON，或缺 `schemaVersion` / `action` | 2 |
| `UnsupportedSchema` | `schemaVersion` 不是 `1` | 2 |
| `UnsupportedAction` | `action` 不是 `inspect` / `create` / `apply` / `exportMedia` | 2 |
| `UnsupportedFormat` | 扩展名或 `format` 不是 `xlsx` / `docx` / `pptx` | 2 |
| `UnsupportedOp` | apply 遇到第一期不认识的 `op` | 2 |
| `InvalidField` | 字段类型不对、互斥字段同时出现、sheet 名非法、对不上格式的 op 或 scope 键 | 2 |
| `InvalidRange` | A1 区域解析失败，或起止颠倒 | 2 |
| `InvalidId` | id 字符串不符合该格式的生成规则 | 2 |
| `IdNotFound` | id 语法对，但文件里没有这个工作表、段落、节、页眉页脚、图、形状或用户注 | 3 |
| `ScopeRequired` | inspect 要单元格却没给 `sheet`+`range`，要段落正文却没给 `from`+`maxParas`，或要形状却没给 `slide` | 2 |
| `TruncatedLimit` | `maxCells`、`maxParas`、`maxShapes` 超出允许范围，例如设成 `0` | 2 |
| `OpcOpen` | 不是 ZIP，或缺少 `[Content_Types].xml` / 根关系 | 3 |
| `OpcPartMissing` | 入口部件不存在，目标工作表或幻灯片部件不存在，关系 id 找不到，或者关系指向外部资源 | 3 |
| `XmlBroken` | 目标部件不是能解析的 XML；xlsx 里 `t="s"` 的索引越界或不是整数；create 写文本时共享字符串表里没有对应项 | 3 |
| `Io` | 读盘、写盘、创建目录失败 | 4 |
| `NotImplemented` | 第一期明确不做的动作。目前：对共享公式的主单元格发 `setCell` | 2 |

找不到单元格时，`setCell` 的语义是新建该单元格，因此空单元格不是 `IdNotFound`。找不到工作表才是 `IdNotFound`。找不到段落时，`setParaText` 报 `IdNotFound`，不新建段落；正文 `para:` 只匹配主文档，页眉页脚要用带容器前缀的 id，注内要用带注前缀的 id。对页眉或页脚容器 id、框 id 或注容器 id（没有 `/para:`）发 `setParaText` 报 `InvalidId`。找不到节时，`setHeaderFooterDistance` 与 `setHeaderFooterLink` 报 `IdNotFound`；id 不是 `section:{n}` 报 `InvalidId`。对第一节发 `setHeaderFooterLink` 且 `linked: true`，或该类型对本节不生效，报 `InvalidField`。找不到图时，图片 op 报 `IdNotFound`，不新建图（`insertPicture` 除外）。`insertPicture` 的 `id` 必须是段落 id；对图 id、容器 id、注容器 id 或 `section:{n}` 发该 op 报 `InvalidId`。`replacePicture` / `deletePicture` / `setPictureSize` / `setPictureWrap` 的 `id` 必须是图 id；对段落 id、容器 id 或 `section:{n}` 发这四个 op 报 `InvalidId`。读不到图片 `file` 报 `Io`。create 的 `pictures[]` 缺 `file` 报 `MissingField`；只给 `cx` 或只给 `cy` 报 `InvalidField`。create 的页眉页脚 `type` 不是 `default`，或同一侧写了两项，报 `InvalidField`。xlsx / pptx 的 create 对象上出现 `pictures` 报 `InvalidField`。找不到形状时，`setShapeText` 报 `IdNotFound`，不新建形状。对幻灯片 id（`slide:{n}`，没有 `/shape:`）发 `setShapeText` 报 `InvalidId`。xlsx / pptx 的 `scope` 里出现 `pictures`、`textBoxes`、`textBoxContent` 或 `notes` 报 `InvalidField`。对 xlsx / pptx 发 `exportMedia`、文本框 op 或脚注尾注 op 报 `InvalidField`。找不到文本框时，框专用 op 报 `IdNotFound`。对宿主段落 id 发框专用 op（`insertTextBox` 除外）报 `InvalidId`。对框 id 发 `setParaText` 报 `InvalidId`。框内或注内只剩一段时 `deletePara` 报 `InvalidField`。只给 `textBoxContent`、不给 `textBoxes` 报 `InvalidField`。找不到用户注时，`deleteFootnote` / `deleteEndnote` 报 `IdNotFound`。对分隔符 id 发任何 apply 报 `InvalidId`。`setNoteNumbering` 的 id 不是 `docx:document` 或 `section:{n}` 报 `InvalidId`；三个编号字段一个都不给、或枚举不认识，报 `InvalidField`。`exportMedia` 找不到图报 `IdNotFound`；`id` 不是图 id 报 `InvalidId`；关系指向外部资源报 `OpcPartMissing`。

## 成功形态

inspect 成功：stdout 是领域树，见 [json-contract.md](json-contract.md)。不要再包一层 `{ "ok": true, "tree": ... }`，以免和树本身的字段打架。

create / apply / exportMedia 成功：stdout 为一行 JSON：

```json
{
  "ok": true,
  "path": "C:\\out\\book.xlsx",
  "opsApplied": 3
}
```

`opsApplied` 仅 apply 出现，等于实际执行的 op 条数。create 与 `exportMedia` 省略该字段。

`exportMedia` 成功时再带 `contentType`（取自包内 `[Content_Types].xml`）。没有对应 Default / Override 时省略该字段：

```json
{
  "ok": true,
  "path": "C:\\out\\logo.png",
  "contentType": "image/png"
}
```
