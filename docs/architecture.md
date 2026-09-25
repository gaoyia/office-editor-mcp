# 架构

状态：xlsx 文档待确认。docx / pptx 只定边界，不定第一期实现细节。

## 产品形态

产品名是 `ooxp`，全称 Office Open XML Patch。一个 Rust 库，外加一个命令行工具。库只认领域 JSON。命令行把 stdin 或文件里的领域 JSON 交给库，再把结果写到 stdout 或磁盘。

MCP 是以后的传输层，第一期不做。将来无论用不用 JSON-RPC，都不许为了信封去改领域 JSON。

对齐 Transitional OOXML，也就是 Office 默认存出来的那种包。ECMA-376 示例里常见的 Strict 命名空间（`http://purl.oclc.org/ooxml/...`）只作对照，create 不按 Strict 生成。

## 三个动作

这三个动作是整套设计的地基，实现时不许改语义。

| 动作 | 真源 | 做什么 |
| --- | --- | --- |
| `inspect` | 已有 ZIP 包里的 XML | 返回带稳定 id 的精简 JSON 树，按调用方给的范围截断 |
| `apply` | 已有 ZIP 包里的 XML | 按 id 打补丁，只改目标节点，其余部件按原字节写回 |
| `create` | 领域 JSON | 从 JSON 生成一份新的完整包。这是唯一允许整包生成的动作 |

禁止把 inspect 树当成完整文档模型，改完之后整包重新生成。apply 的入参只能是操作列表，不能是整棵树。

## 模块

建议用 Cargo workspace，两个 crate。第一期只实现 `opc`、`json`、`xlsx`、`cli`。`docx` 和 `pptx` 目录可以先建空模块，但不写业务。

```text
crates/ooxp/                 库
  opc/                       ZIP、[Content_Types].xml、各级 .rels、按路径读写部件
  json/                      领域 JSON 的解析、校验、错误
  xlsx/                      工作簿 inspect / apply / create
  docx/                      第一期不实现
  pptx/                      第一期不实现
crates/ooxp-cli/             命令行，只依赖 ooxp。二进制名也是 ooxp
```

依赖方向只能是：`cli` → 库的门面；`xlsx` / `docx` / `pptx` → `opc` 与 `json`；`opc` 和 `json` 互不依赖格式模块。禁止格式模块直接操作 ZIP 字节而不走 `opc`。

### opc

把 `.docx`、`.xlsx`、`.pptx` 都当成 OPC 包。负责打开、列出部件、按路径读字节、标记脏部件、保存。保存时没改过的部件保持原来的压缩方法和未压缩字节，不重排 ZIP 中央目录里未改条目的相对顺序，也不去美化无关 XML。细节见 [opc.md](opc.md)。

### json

只处理领域信封：`schemaVersion`、`action`、各动作的必填字段。不认识的字段在第一期直接报错，避免默默丢数据。契约见 [json-contract.md](json-contract.md)。

### xlsx

读 `xl/workbook.xml` 拿工作表名单和关系。按范围读对应 `sheetN.xml`，需要时再读 `xl/sharedStrings.xml`。apply 只改目标工作表 XML，以及为了重算而改的 `workbook.xml` 里 `calcPr`，必要时删掉 `xl/calcChain.xml`。不改 `styles.xml`，不求值公式。

### docx / pptx

第一期不实现。模块职责预留如下，避免以后把格式逻辑塞进 `opc`。

- docx：主文档在 `word/document.xml`。inspect 返回大纲和段落。apply 按段落 id 改可见文本，不整篇重建。
- pptx：幻灯片在 `ppt/slides/slideN.xml`。inspect 按页列出形状文本。apply 按页加形状改文字，不删形状重建，以免动画绑定失效。不改 `<p:timing>`。

### cli

从文件或 stdin 读入一份领域 JSON，按 `action` 分发。inspect 把树打到 stdout。create 和 apply 写出目标文件。契约见 [cli.md](cli.md)。

## 数据流

### inspect

调用方给出路径和范围。库打开 OPC 包，定位格式入口部件（xlsx 是 Workbook 部件）。按范围读必要的 XML，生成带 id 的树。默认不读完全部单元格。包不写回。

### apply

调用方给出路径和 `ops`。库打开包，按每条 op 的 id 找到 XML 节点，只改这些节点。xlsx 在改了单元格之后，把工作簿标成打开时需要重算。保存时只重写脏部件。

### create

调用方给出格式、输出路径和内容对象。库在内存里生成一套最小可打开的 Transitional 部件，打成新 ZIP。不读取任何已有文件当模板，除非以后另开「从模板复制」的动作。

## 硬约束

- 自己实现 OPC。不要把 `office_oxide` 写进 `Cargo.toml`，不要加它的 git 子模块，不要在仓库里放它的源码目录。
- 查规范用 `standards/md/` 的章节索引。xlsx 单元格见 ECMA-376 §18.3.1.4，公式见 §18.3.1.96 与 §18.17，共享字符串见 §18.4，计算属性见 §18.2.2，包结构见 §9 与 §12.2。
- 不实现 Excel 计算引擎、Word 排版引擎、PowerPoint 完整动画时间轴。
- 不支持 `.doc`、`.xls`、`.ppt`。
- XML 编辑是手术刀：不认识的命名空间、属性、子元素，只要没被当前 op 点到，就原样保留。

## 实现时允许的依赖（文档约定，代码阶段再写入 Cargo.toml）

库可以依赖常见的 ZIP 与 XML crate，例如 `zip`、`quick-xml`、`serde`、`serde_json`。禁止依赖任何把整份 Office 文档建成自己那套对象模型、再整包写出的高层 Office crate。

## 阶段边界

xlsx 第一期做完的标志：create 能生成 Excel 和 WPS 都能打开的工作簿；inspect 能按范围返回值和公式字符串；apply 能 `setCell`；命令行能把这三件事跑通。docx 和 pptx 不在本阶段验收。
