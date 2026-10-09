# 规格真源

本目录是库和命令行的规格真源。行为以这里为准，不以代码或口头约定为准。要改行为，先改这里的文档，等确认后再改代码。

当前 **xlsx**、**docx**、**pptx** 的契约都已实现。人工打开只认 Microsoft Office（Excel / Word / PowerPoint 16.0），见三份 checklist。不再要求 WPS。

| 文件 | 内容 |
| --- | --- |
| [architecture.md](architecture.md) | 模块划分、数据流、硬约束 |
| [errors.md](errors.md) | 错误码与 CLI 输出形态 |
| [opc.md](opc.md) | ZIP / Content Types / 关系部件的读写契约 |
| [json-contract.md](json-contract.md) | 领域 JSON 总册：信封、跨格式 op、exportMedia / exportChartData |
| [json-contract-xlsx.md](json-contract-xlsx.md) | xlsx 的 id、inspect、create 与专用 apply |
| [json-contract-docx.md](json-contract-docx.md) | docx 的 id、inspect、create 与专用 apply |
| [json-contract-pptx.md](json-contract-pptx.md) | pptx 的 id、inspect、create 与专用 apply |
| [cli.md](cli.md) | 命令行入参、出参、退出码 |
| [checklist-xlsx.md](checklist-xlsx.md) | 人工用 Microsoft Excel 打开工作簿的检查清单 |
| [checklist-docx.md](checklist-docx.md) | 人工用 Microsoft Word 打开文档的检查清单 |
| [checklist-pptx.md](checklist-pptx.md) | 人工用 Microsoft PowerPoint 打开文稿的检查清单 |
| [examples/](examples/) | 可直接拿去跑的请求和响应样例 |
| [json-schema/request.schema.json](json-schema/request.schema.json) | 请求 JSON Schema 入口；xlsx / docx / pptx 片段在同目录 |

规范条款以 `standards/md/` 为索引，本目录只引用条款号，不复制五千页原文。

写本目录时，表格只放短字段。超过一行的说明改成无序列表，不要写成超长单行，也不要靠硬折行凑短。
