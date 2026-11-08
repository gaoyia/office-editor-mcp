# 浏览器演示

日期：2026-11-08

这是临时页面，不是新的领域动作。领域 JSON 仍以各契约分册为准。绑定只调用 `run_bytes`。

页面在 `demo/web`。绑定在 `crates/ooxp-wasm`。推到 `main` 后，`.github/workflows/pages.yml` 编出 wasm 并发布到 GitHub Pages。

页面上是六份示例请求：xlsx、docx、pptx 各一份创建、一份修改。请求可以改。运行后文件直接下载，页面不预览。

修改请求用的是创建结果里的 id。

- xlsx 单元格是 `cell:Sheet1!A1`
- docx 第一段是 `para:00000001`
- pptx 第一页第一个文本框是 `slide:1/shape:2`

没跑过创建时，修改会失败，除非授权目录里已经有 `path` 指向的文件。

## 本地目录

选择目录是可选的。授了读写权限之后，运行结果除了下载，还会按 `path` 或 `output` 写进这个目录。修改时如果目录里已有 `path`，就读那一份，不再用刚才创建的字节。

- 只能读写这一个目录，路径相对该目录，不能带盘符，也不能含 `..`
- 库不按路径读磁盘。页面读出字节交给 `run_bytes`，再把结果写回去
- 需要 Chrome 或 Edge，页面要在 `https` 或 `localhost` 上打开
- 授权句柄存在这个站点的 IndexedDB 里。刷新后权限还在就接着用，掉了要点「恢复授权」

## 时钟

`wasm32` 上库没有系统时间和随机数。绑定在创建会话时用页面的 `Date` 和 `crypto.getRandomValues` 装入。进程号固定为 1。

压缩包上的部件时间不走这个时钟。`zip` 的 `time` 特性会在取默认时间时调用系统时钟，`wasm32` 上没有，所以浏览器里打出来的包用 1980-01-01。本机命令行仍用系统时间。

## 本地打开

在 `demo/web` 下先编出 `pkg/`，再起一个本机 http 服务。不要双击 html。

```text
cargo build -p ooxp-wasm --target wasm32-unknown-unknown --release
wasm-bindgen target/wasm32-unknown-unknown/release/ooxp_wasm.wasm --out-dir demo/web/pkg --target web
```

`wasm-bindgen` 命令行的版本必须和 `crates/ooxp-wasm` 里锁定的 `wasm-bindgen` 相同。当前是 `0.2.127`。
