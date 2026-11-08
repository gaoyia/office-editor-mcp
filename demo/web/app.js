import init, { Session } from "./pkg/ooxp_wasm.js";

const EXAMPLES = [
  {
    title: "工作簿",
    items: [
      {
        name: "创建",
        request: {
          schemaVersion: 1,
          action: "create",
          path: "book.xlsx",
          format: "xlsx",
          workbook: {
            sheets: [
              {
                name: "Sheet1",
                cells: [
                  { ref: "A1", value: 1 },
                  { ref: "A2", value: "hello" },
                  { ref: "A3", value: true },
                  { ref: "B1", formula: "=SUM(A1)" },
                ],
              },
            ],
          },
        },
      },
      {
        name: "修改",
        request: {
          schemaVersion: 1,
          action: "apply",
          path: "book.xlsx",
          output: "book.patched.xlsx",
          ops: [
            { op: "setCell", id: "cell:Sheet1!A1", value: 42 },
            { op: "setCell", id: "cell:Sheet1!A2", value: "已修改" },
          ],
        },
      },
    ],
  },
  {
    title: "文档",
    items: [
      {
        name: "创建",
        request: {
          schemaVersion: 1,
          action: "create",
          path: "note.docx",
          format: "docx",
          document: {
            paragraphs: [
              { style: "Heading1", text: "标题" },
              { text: "hello" },
            ],
          },
        },
      },
      {
        name: "修改",
        request: {
          schemaVersion: 1,
          action: "apply",
          path: "note.docx",
          output: "note.patched.docx",
          ops: [{ op: "setParaText", id: "para:00000001", text: "新标题" }],
        },
      },
    ],
  },
  {
    title: "演示文稿",
    items: [
      {
        name: "创建",
        request: {
          schemaVersion: 1,
          action: "create",
          path: "deck.pptx",
          format: "pptx",
          presentation: {
            slides: [
              { shapes: [{ text: "标题" }, { text: "hello" }] },
              { shapes: [{ text: "第二页" }] },
            ],
          },
        },
      },
      {
        name: "修改",
        request: {
          schemaVersion: 1,
          action: "apply",
          path: "deck.pptx",
          output: "deck.patched.pptx",
          ops: [{ op: "setShapeText", id: "slide:1/shape:2", text: "新标题" }],
        },
      },
    ],
  },
];

const MIME = {
  xlsx: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
  docx: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  pptx: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
};

const logEl = document.querySelector("#log");
const dirName = document.querySelector("#dir-name");
const made = new Map();
let session = null;
let root = null;

function append(text) {
  const p = document.createElement("p");
  p.textContent = text;
  logEl.append(p);
  logEl.scrollTop = logEl.scrollHeight;
}

function baseName(path) {
  const parts = String(path).split(/[/\\]/);
  return parts[parts.length - 1] || "download";
}

function mimeOf(path) {
  const name = baseName(path).toLowerCase();
  const dot = name.lastIndexOf(".");
  const ext = dot < 0 ? "" : name.slice(dot + 1);
  return MIME[ext] || "application/octet-stream";
}

function download(name, bytes) {
  const blob = new Blob([bytes], { type: mimeOf(name) });
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = baseName(name);
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(link.href), 1500);
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

async function dirHandle(parts, create) {
  let dir = root;
  for (const name of parts) {
    dir = await dir.getDirectoryHandle(name, { create });
  }
  return dir;
}

async function readBytes(path) {
  const parts = partsOf(path);
  const name = parts.pop();
  const dir = await dirHandle(parts, false);
  const handle = await dir.getFileHandle(name);
  const file = await handle.getFile();
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

function needsPackage(action) {
  return action === "inspect" || action === "apply" || action === "exportMedia" || action === "exportChartData";
}

async function loadPackage(path) {
  if (root) {
    try {
      return await readBytes(path);
    } catch {
      /* 目录里没有时，用这次页面创建的字节。 */
    }
  }
  const hit = made.get(path);
  if (hit) return hit;
  throw new Error("还没有「" + path + "」。请先运行创建，或把它放进授权目录。");
}

async function runRequest(raw) {
  if (!session) throw new Error("库还没有就绪");
  let request;
  try {
    request = JSON.parse(raw);
  } catch {
    throw new Error("请求不是 JSON");
  }
  const action = request.action || "";
  let pkg;
  if (needsPackage(action)) pkg = await loadPackage(request.path);
  const out = session.run(JSON.stringify(request), pkg || new Uint8Array(0));
  if (!out.ok) {
    const message = out.error && out.error.message ? out.error.message : "执行失败";
    throw new Error(message);
  }
  if (out.kind !== "bytes") {
    append("完成，没有文件可下载。");
    return;
  }
  const path = out.path || request.output || request.path || "output.bin";
  made.set(path, out.bytes);
  if (!request.output && request.path) made.set(request.path, out.bytes);
  if (root) {
    await writeBytes(path, out.bytes);
    append("已写入目录并下载 " + path);
  } else {
    append("已下载 " + path);
  }
  download(path, out.bytes);
}

function renderExamples() {
  const rootEl = document.querySelector("#examples");
  for (const group of EXAMPLES) {
    const section = document.createElement("section");
    const heading = document.createElement("h2");
    heading.textContent = group.title;
    section.append(heading);
    for (const item of group.items) {
      const title = document.createElement("h3");
      title.textContent = item.name;
      const box = document.createElement("textarea");
      box.spellcheck = false;
      box.value = JSON.stringify(item.request, null, 2);
      const button = document.createElement("button");
      button.type = "button";
      button.textContent = "运行";
      button.addEventListener("click", () => {
        runRequest(box.value).catch((err) => append(err.message || String(err)));
      });
      section.append(title, box, button);
    }
    rootEl.append(section);
  }
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

function showDir() {
  dirName.textContent = root
    ? "已授权：" + root.name + "。运行结果会写入这里，并下载。"
    : "还没有授权目录。不选也能运行，结果只下载。";
}

document.querySelector("#pick").addEventListener("click", async () => {
  if (!window.showDirectoryPicker) {
    append("这个浏览器不能授权本地目录，请用 Chrome 或 Edge。");
    return;
  }
  try {
    root = await window.showDirectoryPicker({ mode: "readwrite", id: "ooxp" });
    await saveHandle(root);
    document.querySelector("#restore").hidden = true;
    showDir();
  } catch (err) {
    if (err && err.name === "AbortError") return;
    append(err.message || String(err));
  }
});

async function boot() {
  renderExamples();
  try {
    await init();
    session = new Session();
    const ping = session.run(JSON.stringify({ action: "help", locale: "zh" }), new Uint8Array(0));
    if (!ping.ok) throw new Error(ping.error && ping.error.message ? ping.error.message : "库没有就绪");
    append("库已就绪。");
  } catch (err) {
    append("库没有加载：" + (err.message || String(err)));
    return;
  }
  try {
    const handle = await loadHandle();
    if (!handle) return;
    const perm = await handle.queryPermission({ mode: "readwrite" });
    if (perm === "granted") {
      root = handle;
      showDir();
      return;
    }
    const button = document.querySelector("#restore");
    button.hidden = false;
    button.addEventListener("click", async () => {
      const next = await handle.requestPermission({ mode: "readwrite" });
      if (next !== "granted") {
        append("目录权限没有授下来。");
        return;
      }
      button.hidden = true;
      root = handle;
      showDir();
    });
  } catch (err) {
    append(err.message || String(err));
  }
}

boot();
