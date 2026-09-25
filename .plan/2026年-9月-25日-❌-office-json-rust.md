# Office JSON Rust 工具（中低难度）

状态：❌ 未开始  
日期：2026-09-25

## 目标

用 Rust 做一套 **库 + 命令行**：入参为 JSON，对 `.docx` / `.xlsx` / `.pptx` 做 **创建** 与 **局部修改**。不复刻 Word 排版引擎，不实现 Excel 计算引擎，不做 PPT 完整动画时间轴。

「全量」指：在本阶段范围内，创建与修改都走同一套 JSON 契约，并做到 Office 能打开、常见样式在局部改字时尽量保留。不是覆盖 Office 全部功能。

## 范围

### 做（中低）

| 项 | 状态 |
| --- | --- |
| OPC（ZIP + Content_Types + rels）打开 / 保存，未改部件原样写回 | ❌ |
| xlsx：读单元格值与公式字符串；写值 / 写公式；清过期缓存或标需重算 | ❌ |
| xlsx：从 JSON 创建简易工作簿（表、单元格、公式文本） | ❌ |
| docx：读大纲与段落文本（带稳定 id）；按段落改字，不整篇重建 | ❌ |
| docx：从 JSON 创建简易文档（段落、标题） | ❌ |
| pptx：按页列出形状文本；按页/形状改字 | ❌ |
| pptx：从 JSON 创建简易演示文稿（若干页、文本框） | ❌ |
| CLI：stdin / 文件读 JSON，写出文档或 inspect JSON | ❌ |
| 文档先行：`DESIGN.md` + JSON schema | ❌ |

### 暂缓 💤

- PPT 通用动画时间轴（可读列表可二期；完整写入不做）
- Word 修订痕迹、页眉页脚精细编辑、复杂表格结构编辑
- Excel 公式求值、数据透视、图表渲染
- `.doc` / `.xls` / `.ppt` 二进制
- 分页与 Word 像素级一致

## 架构要点

- 自研 OPC + 定点改 XML，不要拼三套互不兼容的高层 crate 当唯一模型。
- **不依赖** `office_oxide`。允许 git 子模块克隆到例如 `third_party/office_oxide`，只作对照实现；不链进 Cargo 依赖。抄代码须保留 MIT / Apache-2.0 许可声明。
- 修改策略：**手术刀**。只改目标节点，未知命名空间与未触及部件原样保留（Transitional OOXML，对齐 Office 默认文件，不是项目「旧版兼容」）。
- **两层 JSON，不要混成一种：**
  - 领域契约：`inspect` / `create` / `apply` 的文档操作 JSON（库与 CLI 的入参）。
  - 传输：MCP 使用 JSON-RPC 2.0（stdio）。RPC 的 `params` 里放领域 JSON，不要用 JSON-RPC 信封当单元格/段落模型。
- 禁止用「整份文档 JSON 再生成」作为修改路径。
- 规范查阅用 `standards/md/`，不要整本打开 ECMA-376 五千页 PDF。

## 阶段出口

1. 空白 xlsx/docx/pptx 可从 JSON 创建，Office 能打开。
2. 对样例文件 apply 改一格 / 一段 / 一页文字后，Office 能打开，未改区域观感基本不变。
3. 公式只作为字符串写入；不在库内求值。
4. 单元测试覆盖 JSON 契约与 XML 不变量；有手工打开清单。

## 新会话任务

先写 `DESIGN.md` 与 JSON 契约，用户确认后再写代码。
