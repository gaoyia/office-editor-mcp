# ooxp

`ooxp` 是 Office Open XML Patch。`oox` 是 Office Open XML，也就是 OOXML。`p` 是 Patch：按稳定 id 给包里的 XML 打补丁，其余部件按原字节写回。只有新建文件才整包生成。

用领域 JSON 读写本地的 `.docx`、`.xlsx`、`.pptx`。仓库里是一个 Rust 库，外加命令行 `ooxp`。

对齐 Office 默认存出来的 Transitional OOXML。公式当字符串保存。只支持这三种新格式，路径也可以是 `.xlsm`（inspect 树的 `format` 仍是 `xlsx`，不执行宏）。验收只认 Microsoft Office 16.0。

命令行可以执行 `ooxp mcp`，从标准输入读 JSON-RPC。工具参数仍是领域 JSON。库不解析信封。外部调用方也可以自己包传输信封，`params` 里必须是完整的领域 JSON。

## 许可与捐赠

本仓库以 Business Source License 1.0 发布，全文在 `LICENSE`。作者目前有经济压力，选用这份许可是迫不得已。接受捐赠或出售商用授权。

**强烈谴责A股市场环境**：作者因买入股【华铁股份】现【R通达1】不完全统计**家庭总计亏损高达200万**。目前正在起诉，进入二审阶段，执行更是不知要到什么时候了，这已经是第三、四年了，其余股票也亏损严重，现金流状况至今难以缓解。

生产使用和商用必须向作者购买商业授权。购买请写信到 gaoyia@qq.com。

| 微信赞赏码 | 支付宝领红包 | 支付宝 |
| --- | --- | --- |
| ![微信赞赏码](https://cdn.jsdelivr.net/gh/gaoyia/gaoyia@v0.0.2/donate.jpg) | ![支付宝领红包](https://cdn.jsdelivr.net/gh/gaoyia/gaoyia@v0.0.2/zfbhb.jpg) | ![支付宝](https://cdn.jsdelivr.net/gh/gaoyia/gaoyia@v0.0.2/zfb.jpg) |

## 动作

| 动作 | 作用 |
| --- | --- |
| `inspect` | 按范围返回带稳定 id 的精简 JSON 树 |
| `apply` | 按 id 打补丁。真源是包里的 XML，只改目标节点，其余部件按原字节写回 |
| `create` | 从 JSON 生成一份新文件。只有这一步允许整包生成 |
| `exportMedia` | 按 id 把图片、音视频、OLE、墨迹、3D 模型或嵌入字体的原字节写到本地 |
| `exportChartData` | 按图表 id 把 docx / pptx 的 embedding 工作簿原字节写到本地 |
| `help` | 查动作、操作和字段怎么填。不读文件，也不改文件 |

`inspect` 默认不把整份文件倒出来，也不把媒体字节塞进树。`apply` 的入参是操作列表，不能是整棵 inspect 树。

## 构建

需要 Rust 2021 edition。

```text
cargo build -p ooxp-cli --release
cargo test
```

二进制名是 `ooxp`。库 crate 也叫 `ooxp`。

## 命令行

```text
ooxp [--input <文件|->] [--output <文件>] [--lang <标签>] [--pretty]
```

除 `mcp` 外没有子命令。不带 `mcp` 时，动作由 JSON 的 `action` 决定。`--input` 省略或写成 `-` 时从 stdin 读。成功的 JSON 写到 stdout，失败的 JSON 写到 stderr。

给编辑器当 MCP 用：

```json
{
  "mcpServers": {
    "ooxp": {
      "command": "ooxp",
      "args": ["mcp"]
    }
  }
}

{
  "mcpServers": {
    "ooxp": {
      "command": "ooxp",
      "args": ["mcp", "--lang", "en"]
    }
  }
}
```

工具是 `inspect`、`create`、`apply`、`exportMedia`、`exportChartData`、`help`。操作怎么填，调用 `help`。细节见 `docs/cli.md`。

参数有两种形状。

| 形状 | 写什么 |
| --- | --- |
| 直接写 | 领域 JSON，不必再写 `action` |
| 本地文件 | 只写 `requestPath` |

文件里是同一份领域 JSON。`requestPath` 不是领域字段。

语言是 `zh`（中文）或 `en`（英文）。它选择错误文案、`help` 的说明，以及 `ooxp mcp` 的 `initialize` 说明和工具介绍。写在越窄的地方，越先采用。

| 位置 | 范围 |
| --- | --- |
| 请求里的 `locale` | 这一次请求 |
| `--lang` | 这个进程 |
| `OOXP_LANG` | 环境变量 |

都没写时，错误文案是英文，`help` 是中文，`initialize` 的说明是中文。

新建一份工作簿：

```text
cargo run -p ooxp-cli -- --input docs/examples/xlsx-create.json
```

`docs/examples/xlsx-create.json` 会在当前目录写出 `book.xlsx`。查某个操作怎么填：

```json
{
  "schemaVersion": 1,
  "action": "help",
  "format": "xlsx",
  "op": "setCell",
  "locale": "zh"
}
```

```text
cargo run -p ooxp-cli -- --pretty
```

把上面的 JSON 从 stdin 送进去。

PowerShell 里如果中文显示乱码，先把输出解码设成 UTF-8：

```powershell
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
```

## 在 Rust 里调用

本机按路径读写用 `ooxp::run`。调用方已经拿着包字节时用 `ooxp::run_bytes`，它不打开 Office 文件，也不按路径读图片或音视频。`run` 不编进 `wasm32`。

```rust
let json = r#"{
  "schemaVersion": 1,
  "action": "inspect",
  "path": "book.xlsx",
  "scope": { "sheet": "Sheet1", "range": "A1:D20" }
}"#;
let out = ooxp::run(json, None)?;
```

`format` 可以省略，省略时从 `path` 的扩展名推断。`schemaVersion` 可以省略，省略视为 `1`。写了就必须是 `1`。

## 文档

行为以 `docs/` 为准。

| 文件 | 内容 |
| --- | --- |
| `docs/architecture.md` | 模块划分与数据流 |
| `docs/json-contract.md` | 领域 JSON 总册 |
| `docs/json-contract-xlsx.md` | xlsx 的 id、inspect、create、apply |
| `docs/json-contract-docx.md` | docx 的 id、inspect、create、apply |
| `docs/json-contract-pptx.md` | pptx 的 id、inspect、create、apply |
| `docs/cli.md` | 命令行入参、出参、退出码 |
| `docs/errors.md` | 错误码 |
| `docs/opc.md` | ZIP、Content Types、关系部件 |
| `docs/examples/` | 可直接交给命令行的样例 |
| `roadmap/` | docx、xlsx、pptx 还剩什么 |

机器可读的请求 Schema 在 `docs/json-schema/request.schema.json`，由 `docs/json-schema/src/` 拼出来。改分块后运行 `python docs/json-schema/build.py`。与 Markdown 冲突时以 `docs/` 里的契约为准。

OOXML 条款索引在 `standards/md/`。先打开对应的 `00-index.md`，再读那一章。

## 改这个仓库

先改 `docs/`，再改代码。新功能对上 `roadmap/` 里对应格式的待做序号，当期切片写在 `.plan/`。提交前跑：

```text
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

给代理看的工作流程在 `AGENTS.md` 和 `.cursor/skills/ooxp/SKILL.md`。
