# 规格真源

本目录是库和命令行的规格真源。行为以这里为准，不以代码或口头约定为准。要改行为，先改这里的文档，等确认后再改代码。

当前 **xlsx** 的契约已经实现：库在 `crates/ooxp`，命令行是 `ooxp`。docx 和 pptx 只定模块边界和稳定 id，细节等 xlsx 人工验收后再补。

| 文件 | 内容 |
| --- | --- |
| [architecture.md](architecture.md) | 模块划分、数据流、硬约束 |
| [errors.md](errors.md) | 错误码与 CLI 输出形态 |
| [opc.md](opc.md) | ZIP / Content Types / 关系部件的读写契约 |
| [json-contract.md](json-contract.md) | 领域 JSON：信封、id、inspect / create / apply |
| [cli.md](cli.md) | 命令行入参、出参、退出码 |
| [checklist-xlsx.md](checklist-xlsx.md) | 人工用 Excel / WPS 打开文件的检查清单 |
| [examples/](examples/) | 可直接拿去跑的请求和响应样例 |
| [json-schema/request.schema.json](json-schema/request.schema.json) | 请求的 JSON Schema，与 json-contract.md 同步 |

规范条款以 `standards/md/` 为索引，本目录只引用条款号，不复制五千页原文。
