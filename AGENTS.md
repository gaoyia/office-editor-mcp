# AGENTS.md

在本仓库动手改代码之前，请先读完这份文件，再读 `.agents/rules/engineering-principles.md`（工程原则）和 `.agents/rules/output-style.md`（输出规范）。

## 这个仓库要做什么

做一个 **Rust 库，外加一个命令行工具**，用来读写本地的 Office 文件；等库和命令行稳定下来，再在外面包一层 **MCP 服务**，交给 AI 调用。

只支持新版的 OOXML 格式，也就是 `.docx`、`.xlsx`、`.pptx`。老的二进制格式 `.doc`、`.xls`、`.ppt` 一概不管。另外有三件事明确不做：不写 Word 的排版引擎，不写 Excel 的公式计算引擎，不做 PowerPoint 完整的动画时间轴。

终极目标是这三种格式都能全功能增、删、改、查，也就是计划文档里说的「全量」。但那是远期的事，第一期只做 `.plan/` 里列出的那几项。设计的时候往全量的方向留余地，不要做出以后堵死路的决定；验收的时候只按第一期的清单验收。

## 已经定下来的交互方式

下面三个动作是整套设计的地基，实现的时候不要改：

1. `inspect`：返回一棵精简的 JSON 树，树上每个节点都带一个稳定 id，专门给模型看。要按调用方给的范围截断，默认不要把整份文件的内容全部返回。
2. `apply`：拿树上的 id 去打补丁。文件的唯一可信来源（下文称「真源」）始终是 ZIP 包里的那些 XML，绝对不能把 JSON 树当成完整文档、改完之后整包重新生成。
3. `create`：只有新建文件的时候，才允许完全由 JSON 生成内容。

这里有两层 JSON，不要混成一层：

- **领域 JSON**：读写 Office 文档用的约定，即 `inspect`、`create`、`apply` 的入参和出参。库和命令行只认这一层。
- **传输 JSON**：MCP 那一层的协议信封。目前倾向于用 JSON-RPC 2.0（走 stdio），把领域 JSON 放进 RPC 的 `params` 字段里。这只是当前的倾向，不是硬性规定，真正开工做 MCP 时可以重新评估；但不管最后选哪种传输方式，都不允许为了迁就它去改动领域 JSON。

## 实现上的硬约束

- OPC 层（ZIP 容器、`[Content_Types].xml`、各级 `.rels`）自己写。改内容时只动目标 XML 节点，其余部件按原来的字节写回。
- **不要**把 `office_oxide` 写进 `Cargo.toml`。想参考它的做法，去它的公开仓库看就行；不要在本仓库放三方包目录，也不要加 git 子模块。
- 代码按 `.plan/` 推进。xlsx、docx、pptx 第一期都已实现。pptx 已通过 PowerPoint 16.0；WPS 演示未跑。
- 规格和契约以 `docs/` 为真源：**先改文档，等用户确认，再动代码**。

## 怎么查 OOXML 规范

查规范请用 `standards/md/` 下面按章节切好的 Markdown，把它当成线索和条款索引来用：先打开对应的 `standards/md/*/00-index.md`，找到条款号，再去读那一章的文件。

`ECMA-376-Part1-5th-2016.pdf` 有五千多页，不要整本打开。也不要对任何一份规范 PDF 跑 OCR，这些 PDF 本身就带电子文字层，直接抽文字即可。

## 目录结构

仓库根目录现在大致是这样：

```text
.
├── AGENTS.md                          ← 本文件
├── Cargo.toml                         ← workspace
├── crates/
│   ├── ooxp/                          ← 库：opc、json、xlsx、docx
│   └── ooxp-cli/                      ← 命令行，二进制名 ooxp
├── .agents/
│   └── rules/
│       ├── engineering-principles.md
│       └── output-style.md
├── .plan/
│   └── 2026年-9月-25日-⚠️-ooxp.md
├── docs/                              ← 规格真源
│   ├── architecture.md
│   ├── json-contract.md
│   ├── opc.md
│   ├── errors.md
│   ├── cli.md
│   ├── checklist-xlsx.md
│   ├── checklist-docx.md
│   ├── checklist-pptx.md
│   ├── examples/
│   └── json-schema/
├── .bugfix-reports/                   ← 严重缺陷记录（计划中的目录，现在还不存在）
└── standards/                         ← OOXML 规范原件与抽取结果
    ├── ECMA-376-Part1-5th-2016.pdf
    ├── MS-DOCX.pdf
    ├── MS-XLSX.pdf
    ├── MS-PPTX.pdf
    ├── pdf_to_md.py
    └── md/
        ├── ECMA-376-Part1-5th-2016/
        ├── MS-DOCX/
        ├── MS-XLSX/
        └── MS-PPTX/
```

## 建议的开工顺序

按计划表来：**1 文档 → 2 OPC → 3.1 和 3.2（xlsx）→ 6 命令行**，先把 Excel 这一条路整个跑通，再回头做 **4（docx）** 和 **5（pptx）**。xlsx、docx、pptx 第一期都已实现。MCP 用的是同一套领域 JSON，第一期不要把它和 OPC 的实现混在一起做。
