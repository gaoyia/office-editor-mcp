# JSON Schema

机器读的那一份是 `request.schema.json`。要改定义，改 `src/` 里的分块，不要手改拼出来的文件。

和 Markdown 契约冲突时，以 `docs/json-contract.md` 及三份分册为准。

## 怎么改

在仓库根目录：

```text
python docs/json-schema/build.py
```

脚本把 `src/` 拼回两份 `request.schema.json`：本目录一份，`crates/ooxp/schema/` 一份。库用 `include_str!` 读 crate 里那份，这样发到 crates.io 时不依赖 `docs/`。两份内容相同，都不要手改。只想核对、不写文件：

```text
python docs/json-schema/build.py --check
```

不一致时退出码是 1。`cargo test -p ooxp --test schema_ops` 会跑这一步。

## 分块放哪

| 路径 | 内容 |
| --- | --- |
| `src/envelope.json` | 信封：有哪些动作、各动作要哪些字段 |
| `src/shared/` | 三种格式共用的定义 |
| `src/xlsx/` | 只属于工作簿的定义 |
| `src/docx/` | 只属于文档的定义 |
| `src/pptx/` | 只属于演示文稿的定义 |

一个文件里是一块事情，不是一个操作一个文件。操作要在该文件的 `formats` 里写上给哪种格式用。`xlsx`、`docx`、`pptx` 三种里可以写一个或多个。总名单由脚本按 `formats` 扫出来，不用手写。

`inspectScope` 的每个 property 上另有 `formats` 数组，写这个 scope 键被哪几种格式接受。`help` 用这份名单按 `format` 过滤字段，不在代码里另抄一份键表。
它和分块顶层的 `formats` 不是一回事：顶层那份按定义名给 apply 操作标格式，由脚本汇总进三份操作目录；property 上这份只给 `help` 看，脚本原样带进生成物。
改完分块要重跑 `build.py`。`request.schema.json` 里的 `inspectScope` 仍是三格式键的并集，跨格式键由各格式的解析器拒绝。

`formats` 的取值只有 `xlsx`、`docx`、`pptx`，与 `build.py` 的 `FORMATS` 一致。`format` 为 `xlsm` 时先按 `parse_format` 收成 `xlsx` 再筛，不能拿原字符串去比。
`help.rs` 的 `format_key` 已经做了这一步，现有运行结果不会出现空字段表；实施欠账是 `def_detail` 还没收到这个归一后的值。

每个操作的 `description` 只写它做什么，英文写在 `descriptionEn`。不能怎么填写在 `limits`，英文写在 `limitsEn`。`help` 检索看中英两边，返回时按 `locale` 只留一种。不写 `locale` 用中文。

同一种格式里名字相同、填法不同的定义，定义名前面带格式，例如 `xlsxInsertMathOp`。请求里的 `op` 仍写 `insertMath`。

## 哪些文件入库

两份 `request.schema.json` 都是脚本写出来的，但要提交。测试靠它们和 `src/` 对得上。

跑 Python 会留下 `__pycache__` 和 `.pyc`。这些不入库，删了再跑会再生。
