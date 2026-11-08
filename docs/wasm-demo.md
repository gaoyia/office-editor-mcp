# 浏览器演示

日期：2026-11-08

这是临时页面，不是新的领域动作。领域 JSON 仍以各契约分册为准。绑定只调用 `run_bytes`。

页面在 `demo/web`。绑定在 `crates/ooxp-wasm`。推到 `main` 后，`.github/workflows/pages.yml` 编出 wasm 并发布到 GitHub Pages。

页面标题是 Office Open XML Patch。页头的描述和关键词包含 `office-edit`、`xlsx-edit`、`docx-edit`、`pptx-edit`，以及 `office-edit-mcp`、`xlsx-edit-mcp`、`docx-edit-mcp`、`pptx-edit-mcp`。标题下面写明这是 ooxp 的 WebAssembly 示例。页面可以在中文和 English 之间切换，示例请求里的可见文字跟着切换。介绍页说明支持的格式。演示文稿那一卡顺带写了一句：支持写入动画。页面也说明文件如何写入目录、MCP 接入，以及按最新 GitHub Release 拼出的下载地址。许可证仍是 Business Source License 1.0。捐赠段落与 [ffrm](https://github.com/gaoyia/ffrm) 相同。每种格式两份请求：一份创建，一份就地修改。请求可以改。当前标签会换成对应的按钮颜色。

示例写这些文件：

- `ooxp-example-业绩.xlsx`：深色标题、数据条、色阶、图标集、迷你图、组合图、环形图、循环图示，以及一整页图表工作表。修改格是 `cell:业绩!C4` 和 `cell:业绩!A1`。英文界面写成 `ooxp-example-results.xlsx`
- `ooxp-example-发布说明.docx`：页眉页脚、水印、页边、分色导语、字体宽窄、公式、图片、拼音、控件、行内循环图示和环形图，以及一张三列色卡。图片路径是 `mark.png`，字节由页面事先放进会话。修改段是 `para:00000001`。英文界面写成 `ooxp-example-notes.docx`
- `ooxp-example-发布会.pptx`：六页。每页有一张背景图，标题所在的位置是透明的。角落三处装饰在进入该页时自动播放。内容是表格、图示或图表。每页一种切换。修改形状是 `slide:1/shape:2`。英文界面写成 `ooxp-example-briefing.pptx`

修改请求不写 `output`，结果覆盖 `path`。目录里还没有该文件时，修改会失败。

## 本地目录

必须先授权一个目录。页面不下载。运行结果只按 `path` 或 `output` 写进这个目录，旁边列出目录里的文件，并标出这次新增或改过的名字。

- 只能读写这一个目录，路径相对该目录，不能带盘符，也不能含 `..`
- 文件正被 Word、Excel 或 PowerPoint 打开时，浏览器会报缓存状态已变。页面改写成「文件被占用」，请先关掉那个程序再写入
- 库不按路径读磁盘。页面读出字节交给 `run_bytes`，再把结果写回去
- 浏览器要有目录选择接口。没有时，页面提示换新版 Chrome 或 Edge，并用 `https` 或 `localhost` 打开
- 授权句柄存在这个站点的 IndexedDB 里。刷新后权限还在就接着用，掉了要点「恢复授权」
- 目录已经打开时，「取消授权」会忘掉这次保存的句柄。浏览器自己记住的站点权限还在，下次选同一目录可能不再询问

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
