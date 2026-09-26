# 规格真源

本目录是库和命令行的规格真源。行为以这里为准，不以代码或口头约定为准。要改行为，先改这里的文档，等确认后再改代码。

当前 **xlsx**、**docx**、**pptx** 的契约都已实现。xlsx / docx 已通过 Excel、Word、WPS 人工打开。pptx 已通过 PowerPoint 16.0，见 [checklist-pptx.md](checklist-pptx.md)。WPS 演示未跑。

| 文件 | 内容 |
| --- | --- |
| [architecture.md](architecture.md) | 模块划分、数据流、硬约束 |
| [errors.md](errors.md) | 错误码与 CLI 输出形态 |
| [opc.md](opc.md) | ZIP / Content Types / 关系部件的读写契约 |
| [json-contract.md](json-contract.md) | 领域 JSON：信封、id、inspect / create / apply |
| [cli.md](cli.md) | 命令行入参、出参、退出码 |
| [checklist-xlsx.md](checklist-xlsx.md) | 人工用 Excel / WPS 打开工作簿的检查清单 |
| [checklist-docx.md](checklist-docx.md) | 人工用 Word / WPS 打开文档的检查清单 |
| [checklist-pptx.md](checklist-pptx.md) | 人工用 PowerPoint / WPS 演示打开文稿的检查清单 |
| [examples/](examples/) | 可直接拿去跑的请求和响应样例 |
| [json-schema/request.schema.json](json-schema/request.schema.json) | 请求的 JSON Schema，与 json-contract.md 同步 |

规范条款以 `standards/md/` 为索引，本目录只引用条款号，不复制五千页原文。
