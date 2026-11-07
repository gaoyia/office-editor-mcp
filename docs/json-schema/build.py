"""把 src/ 里的分块拼成 request.schema.json。

改分块，再运行：

    python docs/json-schema/build.py

不要手改拼出来的 request.schema.json。
脚本写两份同样的内容：本目录一份给文档，crate 里一份给 `include_str!`。
发布到 crates.io 时包里看不到 `docs/`，所以 crate 里必须自带这一份。
`--check` 只比较，不写文件。任一份和分块不一致时退出码为 1。
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
SRC = ROOT / "src"
OUTS = (
    ROOT / "request.schema.json",
    ROOT.parents[1] / "crates" / "ooxp" / "schema" / "request.schema.json",
)
FORMATS = ("xlsx", "docx", "pptx")
CATALOGS = (
    ("anyApplyOp", "anyOf", None),
    ("xlsxApplyOp", "oneOf", "xlsx"),
    ("docxApplyOp", "oneOf", "docx"),
    ("pptxApplyOp", "oneOf", "pptx"),
)


def dump(data: object) -> str:
    return json.dumps(data, ensure_ascii=False, indent=2) + "\n"


def load_parts() -> tuple[dict, dict, dict]:
    envelope_path = SRC / "envelope.json"
    if not envelope_path.is_file():
        raise SystemExit(f"缺少 {envelope_path}")
    envelope = json.loads(envelope_path.read_text(encoding="utf-8"))
    if "$defs" in envelope:
        raise SystemExit("envelope.json 不要放 $defs，定义写在旁边的分块里")

    defs: dict = {}
    formats: dict[str, list[str]] = {}
    paths = sorted(
        path for path in SRC.rglob("*.json") if path.resolve() != envelope_path.resolve()
    )
    if not paths:
        raise SystemExit(f"{SRC} 里没有分块")
    for path in paths:
        data = json.loads(path.read_text(encoding="utf-8"))
        block = data.get("$defs")
        if not isinstance(block, dict) or not block:
            raise SystemExit(f"{path} 缺少 $defs")
        for key, body in block.items():
            if key in defs or key in {name for name, _, _ in CATALOGS}:
                raise SystemExit(f"重复定义 {key}（{path}）")
            defs[key] = body
        declared = data.get("formats", {})
        if not isinstance(declared, dict):
            raise SystemExit(f"{path} 的 formats 必须是对象")
        for key, which in declared.items():
            if key not in block:
                raise SystemExit(f"{path} 的 formats 提到了不存在的 {key}")
            if not isinstance(which, list) or not which:
                raise SystemExit(f"{path} 里 {key} 的格式名单是空的")
            unknown = [item for item in which if item not in FORMATS]
            if unknown:
                raise SystemExit(f"{path} 里 {key} 有不认识的格式 {unknown}")
            formats[key] = [item for item in FORMATS if item in which]

    for key, body in defs.items():
        const = op_const(body)
        if const is None:
            continue
        if key not in formats:
            raise SystemExit(f"{key} 是操作，分块里要写 formats")
    for key in formats:
        if op_const(defs[key]) is None:
            raise SystemExit(f"{key} 写了 formats，但不是操作")
    return envelope, defs, formats


def op_const(body: object) -> str | None:
    if not isinstance(body, dict):
        return None
    properties = body.get("properties")
    if not isinstance(properties, dict):
        return None
    op = properties.get("op")
    if not isinstance(op, dict):
        return None
    const = op.get("const")
    if isinstance(const, str):
        return const
    return None


def catalog_names(defs: dict, formats: dict, fmt: str | None) -> list[str]:
    names = [
        name
        for name, which in formats.items()
        if fmt is None or fmt in which
    ]
    names.sort(key=lambda name: (op_const(defs[name]) or "", name))
    return names


def build() -> dict:
    envelope, defs, formats = load_parts()
    ordered = dict(defs)
    for name, combiner, fmt in CATALOGS:
        ordered[name] = {
            combiner: [
                {"$ref": f"#/$defs/{item}"}
                for item in catalog_names(defs, formats, fmt)
            ]
        }
    document = dict(envelope)
    document["$defs"] = ordered
    check_refs(document, set(ordered))
    return document


def check_refs(node: object, names: set[str]) -> None:
    if isinstance(node, dict):
        ref = node.get("$ref")
        if isinstance(ref, str):
            prefix = "#/$defs/"
            name = ref[len(prefix) :].split("/", 1)[0] if ref.startswith(prefix) else ""
            if name not in names:
                raise SystemExit(f"引用悬空：{ref}")
        for value in node.values():
            check_refs(value, names)
    elif isinstance(node, list):
        for value in node:
            check_refs(value, names)


def main() -> int:
    check_only = "--check" in sys.argv
    text = dump(build())
    if check_only:
        for out in OUTS:
            current = out.read_text(encoding="utf-8") if out.is_file() else ""
            if current != text:
                print(
                    f"{out} 和 src/ 不一致。运行 python docs/json-schema/build.py",
                    file=sys.stderr,
                )
                return 1
        return 0
    for out in OUTS:
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(text, encoding="utf-8", newline="\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
