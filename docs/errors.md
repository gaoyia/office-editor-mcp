# 错误

库用一个错误枚举表达失败。命令行把同一结构打成 JSON，写到 stderr，并以非零退出码结束。成功时 stdout 只放约定的成功载荷，不混进错误对象。

## 输出形状

```json
{
  "ok": false,
  "error": {
    "code": "IdNotFound",
    "message": "找不到单元格 id：cell:Sheet1!Z9",
    "path": "ops[0].id"
  }
}
```

`code` 是稳定字符串，给程序认。`message` 是给人看的中文或英文短句。`path` 是可选的 JSON 指针风格路径，指向入参里出错的字段；定位不到就省略。

## 错误码

| code | 何时出现 | CLI 退出码 |
| --- | --- | --- |
| `InvalidJson` | 入参不是 JSON，或缺 `schemaVersion` / `action` | 2 |
| `UnsupportedSchema` | `schemaVersion` 不是 `1` | 2 |
| `UnsupportedAction` | `action` 不是 `inspect` / `create` / `apply` | 2 |
| `UnsupportedFormat` | 扩展名或 `format` 不是 `xlsx` / `docx` / `pptx`；第一期对 docx/pptx 也用这个码 | 2 |
| `UnsupportedOp` | apply 遇到第一期不认识的 `op` | 2 |
| `InvalidField` | 字段类型不对、互斥字段同时出现、sheet 名非法 | 2 |
| `InvalidRange` | A1 区域解析失败，或起止颠倒 | 2 |
| `InvalidId` | id 字符串不符合该格式的生成规则 | 2 |
| `IdNotFound` | id 语法对，但文件里没有这个工作表或目标不存在且 op 不允许新建 | 3 |
| `ScopeRequired` | inspect 要单元格却没给 `sheet`+`range` | 2 |
| `TruncatedLimit` | 仅当调用方把 `maxCells` 设成 `0` 这类无意义值 | 2 |
| `OpcOpen` | 不是 ZIP，或缺少 `[Content_Types].xml` / 根关系 | 3 |
| `OpcPartMissing` | 入口部件或目标工作表部件不存在 | 3 |
| `XmlBroken` | 目标部件不是能解析的 XML | 3 |
| `Io` | 读盘、写盘、创建目录失败 | 4 |
| `NotImplemented` | 第一期明确不做的格式或动作（例如对 docx 发 apply） | 2 |

找不到单元格时，`setCell` 的语义是新建该单元格，因此空单元格不是 `IdNotFound`。找不到工作表才是 `IdNotFound`。

## 成功形态

inspect 成功：stdout 是领域树，见 [json-contract.md](json-contract.md)。不要再包一层 `{ "ok": true, "tree": ... }`，以免和树本身的字段打架。

create / apply 成功：stdout 为一行 JSON：

```json
{
  "ok": true,
  "path": "C:\\out\\book.xlsx",
  "opsApplied": 3
}
```

`opsApplied` 仅 apply 出现，等于实际执行的 op 条数。create 省略该字段。
