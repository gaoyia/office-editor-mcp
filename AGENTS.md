# AGENTS.md

在本仓库动手改代码之前，请先读完这份文件，再读 `.agents/rules/engineering-principles.md`（工程原则）和 `.agents/rules/output-style.md`（输出规范）。

## 这个仓库要做什么

做一个 **Rust 库，外加一个命令行工具**，用来读写本地的 Office 文件。命令行可以执行 `ooxp mcp`，从标准输入读 JSON-RPC，再把领域 JSON 交给库。库不解析 JSON-RPC。领域 JSON 必须原样放进工具参数：不要为了信封去改字段名，也不要把动作拆扁成一套 MCP 专用工具参数。

只支持新版的 OOXML 格式，也就是 `.docx`、`.xlsx`、`.pptx`。老的二进制格式 `.doc`、`.xls`、`.ppt` 一概不管。另外有三件事明确不做：不写 Word 的排版引擎，不写 Excel 的公式计算引擎，不实现 PowerPoint 放映引擎，也不接受任意时间轴树。

终极目标是这三种格式都能全功能增、删、改、查，也就是计划文档里说的「全量」。但那是远期的事，第一期只做 `.plan/` 里列出的那几项。全量还剩什么、按什么顺序开，看 `roadmap/`，不要在 `.plan/` 里再写路线总表。设计的时候往全量的方向留余地，不要做出以后堵死路的决定；验收的时候只按第一期的清单验收。

## 已经定下来的交互方式

下面三个动作是整套设计的地基，实现的时候不要改：

1. `inspect`：返回一棵精简的 JSON 树，树上每个节点都带一个稳定 id，专门给模型看。要按调用方给的范围截断，默认不要把整份文件的内容全部返回。
2. `apply`：拿树上的 id 去打补丁。文件的唯一可信来源（下文称「真源」）始终是 ZIP 包里的那些 XML，绝对不能把 JSON 树当成完整文档、改完之后整包重新生成。
3. `create`：只有新建文件的时候，才允许完全由 JSON 生成内容。

docx 另有只读动作 `exportMedia`：按图 id 把媒体部件原字节写到本地，按 OLE id 把嵌入部件原字节写到本地。它不改上述三个动作的语义，也不把字节塞进 inspect 树。

这里有两层 JSON，不要混成一层：

- **领域 JSON**：读写 Office 文档用的约定，即 `inspect`、`create`、`apply`、`exportMedia` 的入参和出参。本仓库的库和命令行只认这一层。外部 MCP 也必须整份使用这一层，不许改名、拆扁或丢掉字段。
- **传输 JSON**：JSON-RPC 的 `jsonrpc` / `method` / `params`。`ooxp mcp` 只在命令行里解析这一层，解析完把领域 JSON 交给库。外部调用方也可以自己包信封。`params` 或工具参数里必须是完整的领域 JSON，不许为了迁就信封去改领域契约。

## 实现上的硬约束

- OPC 层（ZIP 容器、`[Content_Types].xml`、各级 `.rels`）自己写。改内容时只动目标 XML 节点，其余部件按原来的字节写回。
- **不要**把 `office_oxide` 写进 `Cargo.toml`。想参考它的做法，去它的公开仓库看就行；不要在本仓库放三方包目录，也不要加 git 子模块。
- 代码按 `.plan/` 推进当期切片；下一期开哪一项以 `roadmap/` 对应格式文件的待做表为准。xlsx、docx、pptx 第一期都已实现，并已通过 Microsoft Office 16.0。验收只认官方 Office，不再要求 WPS。
- 规格和契约以 `docs/` 为真源：**先改文档，等用户确认，再动代码**。写 `docs/` 时禁止超长单行：表格只放短字段，长说明改成无序列表。

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
│   ├── ooxp-cli/                      ← 命令行，二进制名 ooxp
│   └── ooxp-wasm/                     ← 浏览器绑定，只调用 run_bytes
├── .agents/
│   └── rules/
│       ├── engineering-principles.md
│       └── output-style.md
├── .plan/                             ← 当期功能切片（带日期与状态）
│   └── 2026年-9月-25日-✅-ooxp.md
├── roadmap/                           ← 三种格式的全量路线（稳定文件名，无日期无状态）
│   ├── docx.md
│   ├── xlsx.md
│   └── pptx.md
├── demo/
│   └── web/                           ← 临时演示页
├── docs/                              ← 规格真源
│   ├── architecture.md
│   ├── json-contract.md
│   ├── json-contract-xlsx.md
│   ├── json-contract-docx.md
│   ├── json-contract-pptx.md
│   ├── opc.md
│   ├── errors.md
│   ├── cli.md
│   ├── checklist-xlsx.md
│   ├── checklist-docx.md
│   ├── checklist-pptx.md
│   ├── wasm-demo.md
│   ├── examples/
│   └── json-schema/
│       ├── README.md
│       ├── build.py                  ← 把 src/ 拼成 request.schema.json
│       ├── request.schema.json       ← 拼出来的副本，不要手改
│       └── src/                      ← 按格式和主题分块，改这里
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

按计划表来：**1 文档 → 2 OPC → 3.1 和 3.2（xlsx）→ 6 命令行**，先把 Excel 这条路整个跑通，再回头做 **4（docx）** 和 **5（pptx）**。xlsx、docx、pptx 第一期都已实现。`ooxp mcp` 只是命令行上的 JSON-RPC 入口，领域 JSON 仍原样交给库。
