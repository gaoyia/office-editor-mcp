# 浏览器演示

日期：2026-11-08

这是临时页面，不是新的领域动作。领域 JSON 仍以各契约分册为准。绑定只调用 `run_bytes`。

页面在 `demo/web`。绑定在 `crates/ooxp-wasm`。推到 `main` 后，`.github/workflows/pages.yml` 编出 wasm 并发布到 GitHub Pages。

## 本地目录

页面可以读写用户授权的一个本地目录。

- 用户要点「选择目录」，并在浏览器里授读写权限。之后代理只能用相对这个目录的路径。
- 库不按路径读磁盘。页面读出字节，交给 `run_bytes`；写出的字节再由页面写回该目录。
- 碰不到没有授权的目录，也没有盘符路径。路径里不能有 `..`。
- 需要 Chrome 或 Edge。页面必须在 `https` 或 `localhost` 上打开。
- 授权句柄存在这个站点的 IndexedDB 里。刷新后如果权限还在，会接着用；权限掉了，要点「恢复授权」。

生成或改写得到的文件写入该目录，并立刻下载。页面不打开预览，也不渲染网页或 Office 内容。

要核对时，代理用 `read` 读文本，或用 `inspect` 看 Office 文件，再用文字说明结果。

## 两种接口

地址填前缀，不要带具体动作。页面自己拼对话地址和模型列表。密钥只留在当前标签页，只发给这个前缀。

| 类型 | 前缀示例 | 对话 | 模型列表 |
| --- | --- | --- | --- |
| OpenAI | `…/v2` | `{前缀}/chat/completions` | `{前缀}/models` |
| Anthropic | `…/anthropic` | `{前缀}/v1/messages` | `{前缀}/v1/models` |

模型列表是这两种协议的通用地址。OpenAI 用 `GET`，头是 `Authorization: Bearer`。Anthropic 同时带 `x-api-key`、`anthropic-version: 2023-06-01` 和 `Authorization: Bearer`，网关认哪一种都能过。列表里的 `id` 填进模型框。列表失败时仍可手写模型 id。

临时试验可以用下面两个前缀：

- `https://maas-api.cn-huabei-1.xf-yun.com/v2`
- `https://maas-api.cn-huabei-1.xf-yun.com/anthropic`

若粘贴的是完整对话地址，页面会去掉末尾的 `/chat/completions` 或 `/v1/messages`，再按前缀拼接。

## 代理怎么动文件

- `files`：`list` 列目录，`read` 读文本，`write` 写文本。网页文件用 `write`。
- `ooxp`：`request` 是完整领域 JSON。`path`、`output`、`file`、`sound` 都相对授权目录。
- 不确定字段时先 `help`。`create` 新建，`inspect` 查看，`apply` 修改。

## 时钟

`wasm32` 上库没有系统时间和随机数。绑定在创建会话时用页面的 `Date` 和 `crypto.getRandomValues` 装入。进程号固定为 1。

## 本地打开

在 `demo/web` 下先编出 `pkg/`，再起一个本机 http 服务。不要双击 html。

```text
cargo build -p ooxp-wasm --target wasm32-unknown-unknown --release
wasm-bindgen target/wasm32-unknown-unknown/release/ooxp_wasm.wasm --out-dir demo/web/pkg --target web
```

`wasm-bindgen` 命令行的版本必须和 `crates/ooxp-wasm` 里锁定的 `wasm-bindgen` 相同。当前是 `0.2.127`。
