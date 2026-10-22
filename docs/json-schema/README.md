# JSON Schema

机器读的那一份是 `request.schema.json`。要改定义，改 `src/` 里的分块，不要手改拼出来的文件。

和 Markdown 契约冲突时，以 `docs/json-contract.md` 及三份分册为准。

## 怎么改

在仓库根目录：

```text
python docs/json-schema/build.py
```

脚本把 `src/` 拼回 `request.schema.json`。只想核对、不写文件：

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

每个操作的 `description` 只写它做什么。不能怎么填写在 `limits`，另成一条。`help` 两处都会拿来检索。

同一种格式里名字相同、填法不同的定义，定义名前面带格式，例如 `xlsxInsertMathOp`。请求里的 `op` 仍写 `insertMath`。

## 哪些文件入库

`request.schema.json` 是脚本写出来的，但要提交。测试靠它和 `src/` 对得上。

跑 Python 会留下 `__pycache__` 和 `.pyc`。这些不入库，删了再跑会再生。
