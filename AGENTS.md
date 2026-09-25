# AGENTS.md

给在本仓库工作的编码助手：先读本文和 `.agents/rules/engineering-principles.md`，再改代码。

## 背景

本仓库要做一个 **Rust 库 + 命令行**，最终再包一层 **MCP**（给 AI 操作本地 Office 文件）。

只支持现代 OOXML：`.docx`、`.xlsx`、`.pptx`。不支持 `.doc` / `.xls` / `.ppt`。不实现 Word 排版引擎、Excel 公式计算引擎、PPT 完整动画时间轴。

交互已经定死：

1. `inspect` 返回带稳定 id 的精简 JSON 树（给模型看，按范围截断，不要默认吐全书）。
2. `apply` 按树上的 id 打补丁；真源始终是 ZIP 里的 XML，不是把 JSON 当完整文档再生成。
3. `create` 才允许从 JSON 生成新文件。

领域 JSON（`inspect` / `create` / `apply`）与 MCP 用的 JSON-RPC 2.0 是两层：RPC 的 `params` 里放领域 JSON。CLI 只认领域 JSON。（由模型实现的时候取舍是否使用JSON-RPC 2.0，只是一个可能的取舍方案）

实现约束：自研 OPC + 定点改 XML；**不要**把 `office_oxide` 加进 Cargo 依赖。需要对照时自行查阅其仓库即可，不要在本仓库建三方包目录或子模块。

当前实现尚未开工。任务拆分与状态见 `.plan/`。规格与契约以 `docs` 为真源：先更新文档、用户确认后再写代码。

规范检索用 `standards/md/` 分章 Markdown，当作提示与条款索引。不要整本打开 `ECMA-376-Part1-5th-2016.pdf`（五千页），也不要对规范 PDF 跑 OCR。

## 目录结构

仓库根目录目前大致如下（Rust crate 布局尚未创建）。

```text
.
├── AGENTS.md                          ← 本文件
├── .agents/
│   └── rules/
│       └── engineering-principles.md  ← 质量、文档先行、计划/实施/验证、状态标记
├── .plan/
│   └── 2026年-9月-25日-❌-office-json-rust.md
│                                      ← 范围、序号 1–6 子任务、暂缓项、架构要点
├── .bugfix-reports/                   ← 计划中的严重缺陷记录目录（可尚不存在）
└── standards/                         ← OOXML 规范与抽取结果
    ├── ECMA-376-Part1-5th-2016.pdf
    ├── MS-DOCX.pdf
    ├── MS-XLSX.pdf
    ├── MS-PPTX.pdf
    ├── pdf_to_md.py                   ← 按书签把 PDF 抽成 md（电子文字层，非 OCR）
    └── md/
        ├── ECMA-376-Part1-5th-2016/   ← 总规范分章；xlsx 公式约 18.17；PPT 动画约 19.5
        ├── MS-DOCX/
        ├── MS-XLSX/
        └── MS-PPTX/
```

计划中、尚未落盘的路径：

```text
docs/                                  ← 架构与领域契约（schemaVersion: 1），真源见工程原则
src/ 或 crates/                        ← 自研库 + CLI（计划项 2 起）
```

## 建议开工顺序

见计划表：**1 文档 → 2 OPC → 3.1 / 3.2 xlsx → 6 CLI**（先把 Excel 路线跑通），再 **4 docx、5 pptx**。MCP 用同一套领域 JSON，不要在第一期和 OPC 实现缠在一起。

查规范时优先打开对应 `standards/md/*/00-index.md`，再读分章文件。
