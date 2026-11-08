import init, { Session } from "./pkg/ooxp_wasm.js";

const PLACEHOLDER = {
  openai: "https://maas-api.cn-huabei-1.xf-yun.com/v2",
  anthropic: "https://maas-api.cn-huabei-1.xf-yun.com/anthropic",
};

const MIME = {
  html: "text/html;charset=utf-8",
  css: "text/css;charset=utf-8",
  js: "text/javascript;charset=utf-8",
  json: "application/json",
  txt: "text/plain;charset=utf-8",
  svg: "image/svg+xml",
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  gif: "image/gif",
  xlsx: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
  xlsm: "application/vnd.ms-excel.sheet.macroEnabled.12",
  docx: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  pptx: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
};

const BINARY_EXT = new Set([
  "docx", "xlsx", "pptx", "xlsm", "png", "jpg", "jpeg", "gif", "webp", "zip", "wasm", "pdf",
]);

const prefixEl = document.querySelector("#prefix");
const keyEl = document.querySelector("#key");
const modelEl = document.querySelector("#model");
const modelList = document.querySelector("#model-list");
const modelStatus = document.querySelector("#model-status");
const urlsEl = document.querySelector("#urls");
const dirName = document.querySelector("#dir-name");
const filesEl = document.querySelector("#files");
const logEl = document.querySelector("#log");
const textEl = document.querySelector("#text");

let session = null;
let root = null;
let stop = false;
const history = [];

function saved(key) {
  return localStorage.getItem("ooxp-demo:" + key) || "";
}

function save(key, value) {
  localStorage.setItem("ooxp-demo:" + key, value);
}

function currentKind() {
  const picked = document.querySelector('input[name="kind"]:checked');
  return picked ? picked.value : "openai";
}

function normalizePrefix(raw) {
  let text = raw.trim().replace(/\/+$/, "");
  const cuts = ["/chat/completions", "/v1/messages", "/v1/models", "/models"];
  for (const cut of cuts) {
    if (text.endsWith(cut)) {
      text = text.slice(0, -cut.length).replace(/\/+$/, "");
      break;
    }
  }
  return text;
}

function endpoints() {
  const base = normalizePrefix(prefixEl.value);
  if (!base) return null;
  if (currentKind() === "anthropic") {
    return {
      chat: base + "/v1/messages",
      models: base + "/v1/models",
    };
  }
  return {
    chat: base + "/chat/completions",
    models: base + "/models",
  };
}

function renderUrls() {
  const ep = endpoints();
  prefixEl.placeholder = PLACEHOLDER[currentKind()];
  if (!ep) {
    urlsEl.textContent = "先填前缀。OpenAI 拼 /chat/completions 和 /models。Anthropic 拼 /v1/messages 和 /v1/models。";
    return;
  }
  urlsEl.textContent = "对话 " + ep.chat + "\n模型列表 " + ep.models;
}

function authHeaders() {
  const key = keyEl.value.trim();
  const headers = { Authorization: "Bearer " + key };
  if (currentKind() === "anthropic") {
    headers["x-api-key"] = key;
    headers["anthropic-version"] = "2023-06-01";
    headers["anthropic-dangerous-direct-browser-access"] = "true";
  }
  return headers;
}

function append(role, text) {
  const p = document.createElement("p");
  const strong = document.createElement("strong");
  strong.textContent = role;
  p.append(strong, document.createTextNode(" " + text));
  logEl.append(p);
  logEl.scrollTop = logEl.scrollHeight;
}

function partsOf(path) {
  const text = String(path ?? "").trim().replace(/\\/g, "/");
  if (!text || text.startsWith("/") || /^[A-Za-z]:/.test(text)) {
    throw new Error("路径必须相对授权目录");
  }
  const parts = text.split("/").filter(Boolean);
  if (parts.some((part) => part === "." || part === "..")) {
    throw new Error("路径不能包含 . 或 ..");
  }
  return parts;
}

function baseName(path) {
  const parts = String(path).split(/[/\\]/);
  return parts[parts.length - 1] || "download";
}

function extOf(path) {
  const name = baseName(path).toLowerCase();
  const dot = name.lastIndexOf(".");
  return dot < 0 ? "" : name.slice(dot + 1);
}

function mimeOf(path, contentType) {
  if (contentType) return contentType;
  return MIME[extOf(path)] || "application/octet-stream";
}

function download(name, bytes, type) {
  const blob = new Blob([bytes], { type: type || "application/octet-stream" });
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = baseName(name);
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(link.href), 1500);
}

async function dirHandle(parts, create) {
  let dir = root;
  for (const name of parts) {
    dir = await dir.getDirectoryHandle(name, { create });
  }
  return dir;
}

async function resolve(path) {
  const parts = path ? partsOf(path) : [];
  if (parts.length === 0) return { kind: "directory", handle: root };
  const name = parts[parts.length - 1];
  const parent = await dirHandle(parts.slice(0, -1), false);
  try {
    return { kind: "file", handle: await parent.getFileHandle(name) };
  } catch {
    try {
      return { kind: "directory", handle: await parent.getDirectoryHandle(name) };
    } catch {
      throw new Error("目录里没有「" + path + "」");
    }
  }
}

async function listDir(path) {
  const hit = await resolve(path || "");
  if (hit.kind !== "directory") throw new Error("「" + path + "」不是目录");
  const rows = [];
  for await (const [name, handle] of hit.handle.entries()) {
    rows.push({ name, kind: handle.kind });
  }
  rows.sort((a, b) => a.name.localeCompare(b.name, "zh"));
  return rows;
}

async function readBytes(path) {
  const hit = await resolve(path);
  if (hit.kind !== "file") throw new Error("「" + path + "」不是文件");
  const file = await hit.handle.getFile();
  return new Uint8Array(await file.arrayBuffer());
}

async function writeBytes(path, bytes) {
  const parts = partsOf(path);
  const name = parts.pop();
  if (!name) throw new Error("缺少文件名");
  const dir = await dirHandle(parts, true);
  const handle = await dir.getFileHandle(name, { create: true });
  const writable = await handle.createWritable();
  await writable.write(bytes);
  await writable.close();
}

function renderFiles(rows) {
  filesEl.replaceChildren();
  const shown = rows.slice(0, 200);
  for (const row of shown) {
    const li = document.createElement("li");
    const name = document.createElement("span");
    name.textContent = row.kind === "directory" ? row.name + "/" : row.name;
    li.append(name);
    if (row.kind === "file") {
      const button = document.createElement("button");
      button.type = "button";
      button.textContent = "下载";
      button.addEventListener("click", () => {
        downloadFile(row.name).catch((err) => append("目录", err.message));
      });
      li.append(button);
    }
    filesEl.append(li);
  }
}

async function refreshList() {
  if (!root) {
    dirName.textContent = "还没有授权目录。";
    filesEl.replaceChildren();
    return [];
  }
  dirName.textContent = "已授权：" + root.name;
  const rows = await listDir("");
  renderFiles(rows);
  return rows;
}

async function downloadFile(path) {
  const bytes = await readBytes(path);
  download(path, bytes, mimeOf(path));
}

function openDb() {
  return new Promise((resolvePromise, reject) => {
    const request = indexedDB.open("ooxp-demo", 1);
    request.onupgradeneeded = () => request.result.createObjectStore("kv");
    request.onsuccess = () => resolvePromise(request.result);
    request.onerror = () => reject(request.error);
  });
}

async function saveHandle(handle) {
  const db = await openDb();
  await new Promise((resolvePromise, reject) => {
    const tx = db.transaction("kv", "readwrite");
    tx.objectStore("kv").put(handle, "dir");
    tx.oncomplete = () => resolvePromise();
    tx.onerror = () => reject(tx.error);
  });
  db.close();
}

async function loadHandle() {
  const db = await openDb();
  const handle = await new Promise((resolvePromise, reject) => {
    const tx = db.transaction("kv", "readonly");
    const request = tx.objectStore("kv").get("dir");
    request.onsuccess = () => resolvePromise(request.result || null);
    request.onerror = () => reject(request.error);
  });
  db.close();
  return handle;
}

async function adopt(handle) {
  root = handle;
  await refreshList();
}

function looksBinary(bytes, path) {
  if (BINARY_EXT.has(extOf(path))) return true;
  const n = Math.min(bytes.length, 8000);
  for (let i = 0; i < n; i += 1) {
    if (bytes[i] === 0) return true;
  }
  return false;
}

function collectSide(value, out) {
  if (!value || typeof value !== "object") return;
  if (Array.isArray(value)) {
    value.forEach((item) => collectSide(item, out));
    return;
  }
  for (const [key, item] of Object.entries(value)) {
    if ((key === "file" || key === "sound") && typeof item === "string") out.add(item);
    else collectSide(item, out);
  }
}

function needsPackage(action) {
  return action === "inspect" || action === "apply" || action === "exportMedia" || action === "exportChartData";
}

function domainFrom(args) {
  if (args && Object.prototype.hasOwnProperty.call(args, "request")) {
    const request = args.request;
    if (typeof request === "string") return JSON.parse(request);
    if (request && typeof request === "object") return request;
    throw new Error("request 不是对象");
  }
  if (args && typeof args.action === "string") return args;
  throw new Error("缺少 request");
}

async function callFiles(args) {
  const op = args.op;
  const path = args.path || "";
  if (op === "list") {
    const rows = await listDir(path);
    const entries = rows.slice(0, 200).map((row) => (row.kind === "directory" ? row.name + "/" : row.name));
    append("目录", "已列出" + (path || "目录") + "，" + rows.length + " 项");
    return { ok: true, path: path || ".", entries, truncated: rows.length > 200 };
  }
  if (op === "read") {
    if (!path) throw new Error("缺少 path");
    const hit = await resolve(path);
    if (hit.kind === "directory") throw new Error("「" + path + "」是目录，请用 list");
    const bytes = await readBytes(path);
    append("目录", "已读取 " + path);
    if (looksBinary(bytes, path)) {
      return {
        ok: true,
        kind: "binary",
        path,
        bytes: bytes.length,
        hint: "这是二进制文件。docx、xlsx、pptx 请用 ooxp 的 inspect。",
      };
    }
    let text = new TextDecoder().decode(bytes);
    if (text.length > 20000) text = text.slice(0, 20000) + "…（已截断）";
    return { ok: true, kind: "text", path, text };
  }
  if (op === "write") {
    if (!path) throw new Error("缺少 path");
    if (typeof args.text !== "string") throw new Error("缺少 text");
    const bytes = new TextEncoder().encode(args.text);
    await writeBytes(path, bytes);
    download(path, bytes, mimeOf(path));
    await refreshList();
    append("目录", "已写入目录并下载 " + path);
    return { ok: true, path, bytes: bytes.length, downloaded: true, saved: true };
  }
  throw new Error("op 只能是 list、read 或 write");
}

async function callOoxp(args) {
  const request = domainFrom(args || {});
  const action = request.action || "";
  const side = new Set();
  collectSide(request, side);
  session.clear_files();
  for (const rel of side) {
    try {
      session.put_file(rel, await readBytes(rel));
    } catch (err) {
      return { ok: false, error: "读不到附属文件「" + rel + "」：" + err.message };
    }
  }
  let pkg;
  if (needsPackage(action)) {
    if (!request.path) return { ok: false, error: "缺少 path" };
    try {
      pkg = await readBytes(request.path);
    } catch {
      return { ok: false, error: "目录里没有「" + request.path + "」" };
    }
  }
  const out = session.run(JSON.stringify(request), pkg);
  if (!out.ok) return out;
  if (out.kind === "bytes") {
    const path = out.path || request.output || request.path || "output.bin";
    await writeBytes(path, out.bytes);
    download(path, out.bytes, mimeOf(path, out.contentType));
    await refreshList();
    append("ooxp", "已写入目录并下载 " + path);
    return {
      ok: true,
      kind: "bytes",
      path,
      bytes: out.bytes.length,
      downloaded: true,
      saved: true,
      opsApplied: out.opsApplied ?? null,
    };
  }
  append("ooxp", "已执行 " + (action || "请求"));
  return { ok: true, kind: "json", value: out.value };
}

function parseArgs(raw) {
  if (raw == null || raw === "") return {};
  if (typeof raw === "object") return raw;
  return JSON.parse(raw);
}

async function callTool(name, raw) {
  let args;
  try {
    args = parseArgs(raw);
  } catch {
    return { ok: false, error: "参数不是 JSON" };
  }
  try {
    const result = name === "files" ? await callFiles(args) : name === "ooxp" ? await callOoxp(args) : { ok: false, error: "没有工具 " + name };
    if (result && result.ok === false) {
      const message = typeof result.error === "string" ? result.error : result.error && result.error.message;
      if (message) append("工具", message);
    }
    return result;
  } catch (err) {
    const message = err.message || String(err);
    append("工具", message);
    return { ok: false, error: message };
  }
}

function toolText(value) {
  let text = JSON.stringify(value);
  if (text.length > 24000) text = text.slice(0, 24000) + "…（已截断）";
  return text;
}

function systemPrompt(rows) {
  const lines = rows.slice(0, 200).map((row) => (row.kind === "directory" ? row.name + "/" : row.name));
  const more = rows.length > 200 ? "\n（只列出前 200 项）" : "";
  return [
    "你是浏览器里的文件代理，面前只有用户授权的一个本地目录。",
    "你只能用相对这个目录的路径。不要写盘符，不要写绝对路径，不要包含 .. 。",
    "工具 files 用来列目录、读文本、写文本。生成 html、css、js 时用 write。写完后浏览器会下载，页面不预览。",
    "要核对网页内容时，用 read 把文本读回来检查，不要假设页面上有预览。",
    "工具 ooxp 用来处理 docx、xlsx、pptx。request 是完整领域 JSON。不确定字段时，action 用 help，locale 用 zh。",
    "create 是新建。inspect 和 apply 的 path 必须是目录里已有的文件。file 和 sound 也是这个目录里的相对路径。",
    "ooxp 写出文件后，浏览器会把文件保存进该目录并下载，页面不预览。需要核对时再 inspect，用文字说明结果。",
    "当前目录名：" + (root ? root.name : ""),
    "顶层条目：\n" + (lines.join("\n") || "（空）") + more,
  ].join("\n");
}

const filesSchema = {
  type: "object",
  properties: {
    op: { type: "string", enum: ["list", "read", "write"] },
    path: { type: "string", description: "相对授权目录的路径。list 可省略" },
    text: { type: "string", description: "write 时的全文" },
  },
  required: ["op"],
};

const ooxpSchema = {
  type: "object",
  properties: {
    request: {
      type: "object",
      description: "完整领域 JSON。path、output、file、sound 都相对授权目录",
    },
  },
  required: ["request"],
};

const openaiTools = [
  {
    type: "function",
    function: {
      name: "files",
      description: "在授权目录里列目录、读文本或写文本。写完会下载，没有预览。",
      parameters: filesSchema,
    },
  },
  {
    type: "function",
    function: {
      name: "ooxp",
      description: "对目录里的 docx、xlsx、pptx 执行领域 JSON。写出的文件会保存并下载，没有预览。",
      parameters: ooxpSchema,
    },
  },
];

const anthropicTools = [
  {
    name: "files",
    description: "在授权目录里列目录、读文本或写文本。写完会下载，没有预览。",
    input_schema: filesSchema,
  },
  {
    name: "ooxp",
    description: "对目录里的 docx、xlsx、pptx 执行领域 JSON。写出的文件会保存并下载，没有预览。",
    input_schema: ooxpSchema,
  },
];

async function postJson(url, headers, body) {
  let response;
  try {
    response = await fetch(url, {
      method: "POST",
      headers: { ...headers, "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
  } catch {
    throw new Error("没有连上接口。浏览器可能被跨域拦住了。");
  }
  const text = await response.text();
  let data = null;
  try {
    data = JSON.parse(text);
  } catch {
    data = null;
  }
  if (!response.ok) {
    const message = (data && (data.error?.message || data.message)) || text.slice(0, 400);
    throw new Error("接口返回 " + response.status + "：" + message);
  }
  return data;
}

function finish(answer) {
  history.push({ role: "assistant", content: answer });
  append("代理", answer || "（没有文字回复）");
}

async function openaiLoop(system, prior, userText) {
  const ep = endpoints();
  const messages = [{ role: "system", content: system }, ...prior, { role: "user", content: userText }];
  for (let round = 0; round < 8; round += 1) {
    if (stop) {
      finish("已停止。");
      return;
    }
    const data = await postJson(ep.chat, authHeaders(), {
      model: modelEl.value.trim(),
      messages,
      tools: openaiTools,
    });
    const message = data.choices && data.choices[0] && data.choices[0].message;
    if (!message) throw new Error("接口没有返回 message");
    const calls = message.tool_calls || [];
    if (calls.length === 0) {
      finish(message.content || "");
      return;
    }
    if (message.content) append("代理", message.content);
    messages.push({ role: "assistant", content: message.content || "", tool_calls: calls });
    for (const call of calls) {
      if (stop) {
        finish("已停止。");
        return;
      }
      const result = await callTool(call.function && call.function.name, call.function && call.function.arguments);
      messages.push({ role: "tool", tool_call_id: call.id, content: toolText(result) });
    }
  }
  finish("工具来回已到 8 轮，先停下来。");
}

async function anthropicLoop(system, prior, userText) {
  const ep = endpoints();
  const messages = [...prior, { role: "user", content: userText }];
  for (let round = 0; round < 8; round += 1) {
    if (stop) {
      finish("已停止。");
      return;
    }
    const data = await postJson(ep.chat, authHeaders(), {
      model: modelEl.value.trim(),
      max_tokens: 4096,
      system,
      messages,
      tools: anthropicTools,
    });
    const blocks = data.content || [];
    const calls = blocks.filter((block) => block.type === "tool_use");
    const text = blocks.filter((block) => block.type === "text").map((block) => block.text).join("\n").trim();
    if (calls.length === 0) {
      finish(text);
      return;
    }
    if (text) append("代理", text);
    messages.push({ role: "assistant", content: blocks });
    const results = [];
    for (const call of calls) {
      if (stop) {
        finish("已停止。");
        return;
      }
      const result = await callTool(call.name, call.input);
      results.push({ type: "tool_result", tool_use_id: call.id, content: toolText(result) });
    }
    messages.push({ role: "user", content: results });
  }
  finish("工具来回已到 8 轮，先停下来。");
}

function modelIds(data) {
  const rows = Array.isArray(data) ? data : Array.isArray(data?.data) ? data.data : Array.isArray(data?.models) ? data.models : [];
  const ids = [];
  for (const row of rows) {
    if (typeof row === "string") ids.push(row);
    else if (row && typeof row.id === "string") ids.push(row.id);
    else if (row && typeof row.name === "string") ids.push(row.name);
  }
  return [...new Set(ids)];
}

async function loadModels() {
  const ep = endpoints();
  if (!ep) throw new Error("先填前缀");
  if (!keyEl.value.trim()) throw new Error("先填密钥");
  let response;
  try {
    response = await fetch(ep.models, { headers: authHeaders() });
  } catch {
    throw new Error("没有连上模型列表。浏览器可能被跨域拦住了。");
  }
  const text = await response.text();
  let data = null;
  try {
    data = JSON.parse(text);
  } catch {
    data = null;
  }
  if (!response.ok) {
    const message = (data && (data.error?.message || data.message)) || text.slice(0, 400);
    throw new Error("模型列表返回 " + response.status + "：" + message);
  }
  const ids = modelIds(data);
  modelList.replaceChildren();
  for (const id of ids) {
    const option = document.createElement("option");
    option.value = id;
    modelList.append(option);
  }
  if (!modelEl.value.trim() && ids.length === 1) modelEl.value = ids[0];
  save("model", modelEl.value.trim());
  if (ids.length === 0) throw new Error("列表里没有模型 id");
  modelStatus.textContent = "已得到 " + ids.length + " 个模型";
}

function restoreSettings() {
  const kind = saved("kind") || "openai";
  const radio = document.querySelector('input[name="kind"][value="' + kind + '"]');
  if (radio) radio.checked = true;
  prefixEl.value = saved("prefix");
  modelEl.value = saved("model");
  keyEl.value = sessionStorage.getItem("ooxp-demo:key") || "";
  renderUrls();
}

function watchSettings() {
  for (const radio of document.querySelectorAll('input[name="kind"]')) {
    radio.addEventListener("change", () => {
      save("kind", currentKind());
      renderUrls();
    });
  }
  prefixEl.addEventListener("input", () => {
    save("prefix", prefixEl.value);
    renderUrls();
  });
  modelEl.addEventListener("change", () => save("model", modelEl.value.trim()));
  modelEl.addEventListener("input", () => save("model", modelEl.value.trim()));
  keyEl.addEventListener("input", () => sessionStorage.setItem("ooxp-demo:key", keyEl.value));
}

document.querySelector("#load-models").addEventListener("click", () => {
  modelStatus.textContent = "正在获取";
  loadModels().catch((err) => {
    modelStatus.textContent = "";
    append("接口", err.message);
  });
});

document.querySelector("#pick").addEventListener("click", async () => {
  if (!window.showDirectoryPicker) {
    append("目录", "这个浏览器不能授权本地目录，请用 Chrome 或 Edge。");
    return;
  }
  try {
    const handle = await window.showDirectoryPicker({ mode: "readwrite", id: "ooxp" });
    await saveHandle(handle);
    document.querySelector("#restore").hidden = true;
    await adopt(handle);
  } catch (err) {
    if (err && err.name === "AbortError") return;
    append("目录", err.message || String(err));
  }
});

document.querySelector("#dl-btn").addEventListener("click", () => {
  const path = document.querySelector("#dl-path").value.trim();
  if (!root) {
    append("目录", "请先选择本地目录。");
    return;
  }
  if (!path) return;
  downloadFile(path).catch((err) => append("目录", err.message));
});

document.querySelector("#stop").addEventListener("click", () => {
  stop = true;
});

document.querySelector("#composer").addEventListener("submit", async (event) => {
  event.preventDefault();
  const userText = textEl.value.trim();
  if (!userText || !session) return;
  if (!root) {
    append("目录", "请先选择本地目录。");
    return;
  }
  if (!endpoints()) {
    append("接口", "先填地址前缀。");
    return;
  }
  if (!keyEl.value.trim() || !modelEl.value.trim()) {
    append("接口", "先填密钥和模型。");
    return;
  }
  stop = false;
  textEl.value = "";
  append("你", userText);
  history.push({ role: "user", content: userText });
  try {
    const perm = await root.queryPermission({ mode: "readwrite" });
    if (perm !== "granted") {
      const next = await root.requestPermission({ mode: "readwrite" });
      if (next !== "granted") throw new Error("目录权限没有授下来");
    }
    const rows = await refreshList();
    const prior = history.slice(0, -1).map((item) => ({ role: item.role, content: item.content }));
    const system = systemPrompt(rows);
    if (currentKind() === "anthropic") await anthropicLoop(system, prior, userText);
    else await openaiLoop(system, prior, userText);
  } catch (err) {
    const message = err.message || String(err);
    history.push({ role: "assistant", content: message });
    append("接口", message);
  }
});

async function boot() {
  restoreSettings();
  watchSettings();
  try {
    await init();
    session = new Session();
    const ping = session.run(JSON.stringify({ action: "help", locale: "zh" }));
    if (!ping.ok) throw new Error(ping.error && ping.error.message ? ping.error.message : "库没有就绪");
    append("库", "已就绪。");
  } catch (err) {
    append("库", "没有加载：" + (err.message || String(err)));
    return;
  }
  try {
    const handle = await loadHandle();
    if (!handle) return;
    const perm = await handle.queryPermission({ mode: "readwrite" });
    if (perm === "granted") {
      await adopt(handle);
      return;
    }
    const button = document.querySelector("#restore");
    button.hidden = false;
    button.addEventListener("click", async () => {
      const next = await handle.requestPermission({ mode: "readwrite" });
      if (next !== "granted") {
        append("目录", "目录权限没有授下来。");
        return;
      }
      button.hidden = true;
      await adopt(handle);
    });
  } catch (err) {
    append("目录", err.message || String(err));
  }
}

boot();
