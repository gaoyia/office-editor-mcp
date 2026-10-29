# 命令行

日期：2026-10-29

二进制名：`ooxp`。不带 `mcp` 时只做三件事：读入一份领域 JSON，交给库，把结果写到 stdout 或磁盘。`ooxp mcp` 从标准输入读 JSON-RPC，把同一份领域 JSON 交给库。

## 调用

```text
ooxp [--input <文件|->] [--output <文件>] [--lang <标签>] [--pretty]
ooxp mcp [--lang <标签>]
```

- **`--input`**
  - 领域 JSON 的路径
  - 值为 `-` 或省略该参数时，从 stdin 读
- **`--output`**
  - 仅当请求里没有 `path`（create）或没有 `output`（apply / exportMedia / exportChartData）时作为落盘路径
  - inspect 忽略此参数
  - `exportMedia` 与 `exportChartData` 不得把源 `path` 当默认输出
- **`--lang`**
  - 错误 `message` 的语言
  - 请求里写了 `locale` 时以请求为准
- **`--pretty`**
  - stdout 的 JSON 带缩进
  - 默认紧凑一行

请求没有 `locale`、命令行也没给 `--lang` 时，读环境变量 `OOXP_LANG`。再没有就用英文。标签匹配规则见 [errors.md](errors.md)。`--help` 本身不翻译。

除 `mcp` 外不提供子命令。`mcp` 不能带 `--input`、`--output` 或 `--pretty`。它可以带 `--lang`，作为没写 `locale` 时的默认语言。不带 `mcp` 时，动作完全由 JSON 的 `action` 决定。

stdin 与 `--input` 同时有数据时，以 `--input` 为准；`--input` 指向普通文件时不要再读 stdin。

## 标准流

| 动作 | stdout | stderr | 退出码 |
| --- | --- | --- | --- |
| inspect 成功 | 领域树 JSON，UTF-8，末尾换行 | 空 | 0 |
| help 成功 | 帮助 JSON，UTF-8，末尾换行 | 空 | 0 |
| create / apply / exportMedia / exportChartData 成功 | `{ "ok": true, "path": "...", ... }` | 空 | 0 |
| 失败 | 空 | `{ "ok": false, "error": { ... } }` | 见 [errors.md](errors.md) |

JSON 打印用稳定的字段顺序，不要求 pretty-print。调试时可以加 `--pretty`，让 stdout 带缩进。默认紧凑一行，方便管道。

## 编码与路径

输入输出 JSON 均为 UTF-8。路径按操作系统原样传给库，命令行自己不做 glob。

Windows 上不必理会控制台代码页。下面三种情形写出去的东西都是对的：

| 情形 | 实际写出的东西 |
| --- | --- |
| 输出接管道或文件 | UTF-8 字节 |
| 输出接控制台，代码页不是 65001 | Rust 标准库先把 UTF-8 转成 UTF-16 再走 `WriteConsoleW`，显示与代码页无关 |
| 输出接控制台，代码页是 65001 | UTF-8 字节，控制台按 UTF-8 解释 |

所以本程序不调 `SetConsoleOutputCP`：那会改掉调用方控制台的全局状态，进程退了还得有人复原。

真看到乱码，是读的一方拿非 UTF-8 的方式解码了 UTF-8 字节。PowerShell 里捕获输出之前先把解码方式定下来：

```powershell
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
```

用 `ProcessStartInfo` 时改设 `StandardOutputEncoding` 与 `StandardErrorEncoding`，`manual-check/office/` 下的脚本就是这么写的。

## mcp

`ooxp mcp` 启动后从标准输入读消息，把结果写回标准输出。标准输入是终端时，向 stderr 写一句正在等待，这句跟着 `--lang`。没带 `--lang` 时这句是中文。然后继续等。`--lang` 后面要有语言标签，例如 `ooxp mcp --lang zh-CN`。其它参数向 stderr 写「mcp 只接受 --lang」，退出码 1，此时还没有协议消息。

编辑器配置示例：

```json
{
  "mcpServers": {
    "ooxp": {
      "command": "ooxp",
      "args": ["mcp", "--lang", "zh-CN"]
    }
  }
}
```

`command` 也可以写成这个 exe 的完整路径。

协议是 JSON-RPC 2.0。一行一条 JSON，也接受 `Content-Length` 头。没有 `id` 的通知不回复。入站一帧最大 256 MiB，一行一条和 `Content-Length` 共用这个上限。超过则拒绝这条消息，不按声明的长度分配内存。`requestPath` 指向的文件不在这 256 MiB 里，文件有多大就读多大。认这些方法：

- `initialize`：回报协议版本、服务器名 `ooxp`，以及一段说明。这段说明用启动时的 `--lang`。没带 `--lang` 时是中文。`zh` 是中文，其余标签按 [errors.md](errors.md) 归到英文
  - 名称含义、六个工具、调用顺序
  - 不知道如何使用时先调用 `help`，查完再调用对应工具
  - `create` 是 action，不是 op
  - `schemaVersion` 与 `format` 可以省略
  - `path` 是 Office 文件的本机路径。另存或导出时，目标路径写在 `output`
  - `help` 按层查：不带参数、`format`、`q`、`format` 加 `op`、`name`
  - 返回里的 `kind` 和 `next`，以及被拒绝时读 `code`
- `ping`
- `tools/list`
- `tools/call`
- `logging/setLevel`：接受并返回空对象

工具有六个，名字就是动作名。`arguments` 有两种形状。

| 形状 | 参数 |
| --- | --- |
| 直接写 | 领域 JSON 去掉 `action` 之后的字段 |
| 本地文件 | 只有 `requestPath`，指向一份领域 JSON 文件 |

文件内容和直接写的参数一样。可以不写 `action`，由工具名补上。写了 `action` 时必须和工具名相同。`requestPath` 与其它字段同时出现则拒绝，不把两边拼在一起。文件打不开、不是 JSON 对象时，工具结果带 `isError`。`requestPath` 不是领域字段，库看不到它。

没写 `schemaVersion` 视为 `1`。写了又不是 `1`，工具结果带 `isError`，正文仍是命令行那种错误 JSON。

`path` 写在 `arguments` 里，表示要打开的 Office 文件。`apply` 另存、`exportMedia` 和 `exportChartData` 落盘时，目标路径写在 `output`。这个模式没有 `--output`。`--lang` 是这个进程的默认语言。它决定 `initialize` 的说明，以及 `tools/list` 里的工具介绍。参数里的 `locale` 优先于它，再看 `OOXP_LANG`，都没有则错误文案用英文。`help` 在请求和 `--lang` 都没给语言时，说明仍是中文。给了 `--lang` 又没写 `locale` 时，`help` 的说明跟着 `--lang`。

成功时，`result.content[0].text` 是命令行会写到 stdout 的那份 JSON。失败时同一位置放 stderr 那种 `{"ok":false,"error":...}`，并带 `isError: true`。库看到的仍是领域 JSON，看不到 `jsonrpc`、`method`、`id`。

操作名单不放进 `tools/list`。要查某个 `op` 怎么填，调用 `help`。
