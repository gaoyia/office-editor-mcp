# 命令行

二进制名：`ooxp`。它只做三件事：读入一份领域 JSON，交给库，把结果写到 stdout 或磁盘。

## 调用

```text
ooxp [--input <文件|->] [--output <文件>] [--lang <标签>] [--pretty]
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

第一期不提供子命令。动作完全由 JSON 的 `action` 决定。

stdin 与 `--input` 同时有数据时，以 `--input` 为准；`--input` 指向普通文件时不要再读 stdin。

## 标准流

| 动作 | stdout | stderr | 退出码 |
| --- | --- | --- | --- |
| inspect 成功 | 领域树 JSON，UTF-8，末尾换行 | 空 | 0 |
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

## 与 MCP 的关系

本仓库不提供 MCP 进程，也不解析 JSON-RPC。外部 MCP 应直接调用库，把领域 JSON 原样作为参数传入；也可以把同一份 JSON 交给本 CLI。不要为 MCP 另造一套字段。
