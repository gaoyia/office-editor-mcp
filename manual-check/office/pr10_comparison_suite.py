"""Bounded PR10 archive provenance and test-fixture preparation (never a renderer).

The archived 0914 index, not later experiment matrices, defines the denominator.
Raw Office and library images are immutable; size mismatch is not a pixel PASS.
"""
import argparse
import collections
import csv
import hashlib
import json
from pathlib import Path, PurePosixPath
import posixpath
import re
import shutil
import subprocess
import xml.etree.ElementTree as ET
import zipfile

DRAWING = "http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing"
MAIN = "http://schemas.openxmlformats.org/drawingml/2006/main"
REL = "http://schemas.openxmlformats.org/package/2006/relationships"
REQUIRED_STYLE_REFS = ("lnRef", "fillRef", "effectRef", "fontRef")


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def drawing_style_errors(data):
    root = ET.fromstring(data)
    errors = []
    for style in root.iter(f"{{{DRAWING}}}style"):
        children = [node.tag for node in style]
        expected = [f"{{{MAIN}}}{name}" for name in REQUIRED_STYLE_REFS]
        if children != expected:
            errors.append({"code": "invalid-shape-style", "children": children,
                           "required": list(REQUIRED_STYLE_REFS)})
    return errors


def repair_optional_empty_style(data):
    """Omit an optional empty xdr:style; never synthesize required theme refs.

    CT_ShapeStyle requires lnRef, fillRef, effectRef, fontRef (ECMA-376).
    This is a test-generator defect, not malformed-style renderer fallback.
    Retain all other bytes, including every shape coordinate and explicit fill.
    """
    empty = [s for s in ET.fromstring(data).iter(f"{{{DRAWING}}}style")
             if len(s) == 0 and not s.attrib and not (s.text or "").strip()]
    changed, count = re.subn(rb"<xdr:style\s*/>", b"", data)
    if count != len(empty):
        raise ValueError("Unrecognized style spelling; do not rewrite namespace prefixes")
    return changed, count


def check_package(path):
    errors = []
    with zipfile.ZipFile(path) as z:
        names = set(z.namelist())
        if len(names) != len(z.namelist()):
            errors.append({"code": "duplicate-zip-members"})
        if z.testzip():
            errors.append({"code": "zip-crc"})
        for name in sorted(names):
            if not name.endswith((".xml", ".rels")):
                continue
            try:
                root = ET.fromstring(z.read(name))
            except ET.ParseError as ex:
                errors.append({"code": "xml-syntax", "part": name, "message": str(ex)})
                continue
            if name.startswith("xl/drawings/") and name.endswith(".xml"):
                errors.extend(dict(e, part=name) for e in drawing_style_errors(z.read(name)))
            if name.endswith(".rels"):
                base = str(PurePosixPath(name).parent.parent)
                for rel in root.findall(f"{{{REL}}}Relationship"):
                    if rel.get("TargetMode") == "External":
                        continue
                    target = rel.get("Target", "").split("#", 1)[0]
                    resolved = (target.lstrip("/") if target.startswith("/") else
                                posixpath.normpath(posixpath.join(base, target)))
                    if resolved not in names:
                        errors.append({"code": "missing-relationship-target", "part": name,
                                       "target": target, "resolved": resolved})
    return errors


def prepare_source(original, destination):
    """Create a dedicated copy; forbid overwrites or writing the original."""
    original, destination = Path(original), Path(destination)
    if destination.exists() or original.resolve() == destination.resolve():
        raise ValueError("Preserve existing source and destination")
    destination.parent.mkdir(parents=True, exist_ok=True)
    changes = []
    if original.suffix.lower() == ".xlsx":
        with zipfile.ZipFile(original) as src, zipfile.ZipFile(destination, "w") as dst:
            for entry in src.infolist():
                data = src.read(entry.filename)
                if entry.filename.startswith("xl/drawings/") and entry.filename.endswith(".xml"):
                    data, n = repair_optional_empty_style(data)
                    if n:
                        changes.append({"part": entry.filename, "removedOptionalEmptyShapeStyle": n})
                dst.writestr(entry, data)
    else:
        shutil.copyfile(original, destination)
    with zipfile.ZipFile(original) as a, zipfile.ZipFile(destination) as b:
        assert a.namelist() == b.namelist()
        actual_changed = [n for n in a.namelist() if a.read(n) != b.read(n)]
    assert set(actual_changed) == {c["part"] for c in changes}
    return {"original": str(original), "source": str(destination),
            "originalSha256": sha256(original), "sourceSha256": sha256(destination),
            "changes": changes, "changedParts": actual_changed,
            "staticErrors": check_package(destination),
            "officeVerified": False, "note": "Static checks are not full OOXML schema validation"}


def base_name(value):
    return str(value).replace("\\", "/").rsplit("/", 1)[-1]


def read_request(path):
    raw = Path(path).read_bytes()
    return json.loads(raw.decode("utf-16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig"))


def request_signature(request):
    q = {k: v for k, v in request.items() if k != "output"}
    if isinstance(q.get("path"), str):
        q["path"] = q["path"].replace("\\", "/")
    return json.dumps(q, sort_keys=True)


def size_status(a, b):
    equal = tuple(a) == tuple(b)
    return {"nativeSameSize": equal, "pixelDiffer": None,
            "pixelStatus": "NOT_COMPUTED" if equal else "NOT_COMPUTED_SIZE_MISMATCH",
            "accepted": False}


def inventory(archive, repo):
    archive, repo = Path(archive), Path(repo)
    pairs = sorted(archive.rglob("03-并排-原生尺寸.png"))
    needed_sizes = {p.with_name("02-库-lib.png").stat().st_size for p in pairs}
    out = repo / "manual-check/office/out"
    aliases = collections.defaultdict(list)
    for f in out.glob("*.png"):
        if f.stat().st_size in needed_sizes:
            aliases[sha256(f)].append(f.name)
    requests = []
    for f in list((out / "requests").glob("*.json")) + list(out.glob("*.json")):
        try:
            q = read_request(f)
        except (ValueError, UnicodeError):
            continue
        if isinstance(q, dict) and isinstance(q.get("action"), str) and q["action"].startswith("export") and q.get("output"):
            requests.append((f, q))
    rows = []
    for pair in pairs:
        directory = pair.parent
        relative = directory.relative_to(archive)
        gold, lib = directory / "01-官方-gold.png", directory / "02-库-lib.png"
        lib_sha = sha256(lib)
        matches = [(f, q) for f, q in requests if base_name(q["output"]) in aliases[lib_sha]]
        exact = [(f, q) for f, q in matches if base_name(q["output"]) == directory.name + "-lib.png"]
        if exact:
            matches = exact
        unique = {}
        for f, q in matches:
            key = request_signature(q)
            unique.setdefault(key, (f, q))
        selected = next(iter(unique.values())) if len(unique) == 1 else None
        row = {"entry": relative.as_posix(), "case": directory.name,
               "group": relative.parts[0], "scope": "main" if int(relative.parts[0][:2]) <= 22 else "historical-blocker-reference-only",
               "oldGold": str(gold), "oldLibrary": str(lib),
               "oldGoldSha256": sha256(gold), "oldLibrarySha256": lib_sha,
               "libraryAliases": aliases[lib_sha], "requestCandidates": [str(f) for f, q in unique.values()],
               "mappingStatus": "request-mapped" if selected else "unresolved-provenance",
               "acceptance": "NOT_REVIEWED", "officeStatus": "NOT_RUN"}
        if selected:
            f, q = selected
            row.update(requestFile=str(f), request=q)
            source = out / base_name(q.get("path", ""))
            if source.is_file():
                row.update(originalSource=str(source), originalSourceSha256=sha256(source))
            else:
                row["mappingStatus"] = "source-missing"
        # These exact existing fixture filenames were inspected for this repair.
        # This is a NEW explicit request, not fabricated historical provenance.
        reviewed_defaults = {"slant-hbottom", "slant-vright", "xlsx-cf-dashed-box",
                             "xlsx-cf-dashed-hbleft", "xlsx-cf-dashed-vright", "xlsx-rtl-text-probe"}
        if not selected and directory.name in reviewed_defaults:
            source = out / (directory.name + ".xlsx")
            if source.is_file():
                row.update(mappingStatus="reconstructed-explicit-default-request",
                           originalSource=str(source), originalSourceSha256=sha256(source),
                           request={"schemaVersion": 1, "action": "exportSheetImage", "id": "sheet:Sheet1",
                                    "path": str(source), "output": str(out / (directory.name + "-lib.png"))},
                           mappingNote="Existing same-named source inspected; new explicit whole-sheet test. Historical request not recovered.")
        rows.append(row)
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument("--prepare", action="store_true")
    parser.add_argument("--render-current", action="store_true")
    args = parser.parse_args()
    args.destination.mkdir(parents=True, exist_ok=True)
    rows = inventory(args.archive, args.repo)
    previous_file = args.destination / "suite-manifest.json"
    previous = json.loads(previous_file.read_text(encoding="utf-8")) if previous_file.exists() else {}
    prior_sources = {r["originalSourceSha256"]: r["prepared"] for r in previous.get("entries", []) if r.get("prepared")}
    sources = {}
    for row in rows:
        source = row.get("originalSource")
        if not source or row["scope"] != "main" or not args.prepare:
            continue
        key = row["originalSourceSha256"]
        if key not in sources:
            original = Path(source)
            dest = args.destination / "sources" / (original.stem + "-" + key[:8] + original.suffix)
            if dest.exists():
                prior = prior_sources.get(key)
                if not prior or prior["sourceSha256"] != sha256(dest) or sha256(original) != key:
                    raise ValueError("Existing prepared source has no matching certificate")
                sources[key] = prior
            else:
                sources[key] = prepare_source(original, dest)
        row["prepared"] = sources[key]
        if args.render_current and not sources[key]["staticErrors"]:
            folder = args.destination / "entries" / row["group"] / row["case"]
            folder.mkdir(parents=True, exist_ok=True)
            request = dict(row["request"], path=sources[key]["source"], output=str(folder / "current-library.png"))
            request_file = folder / "current-request.json"
            request_file.write_text(json.dumps(request, ensure_ascii=False, indent=2), encoding="utf-8")
            result = subprocess.run([str(args.repo / "target/debug/ooxp.exe"), "--input", str(request_file)], capture_output=True, text=True, encoding="utf-8", timeout=60)
            (folder / "current-cli.log").write_text(result.stdout + result.stderr, encoding="utf-8")
            row["currentCli"] = {"returnCode": result.returncode, "request": str(request_file), "output": request["output"]}
            if Path(request["output"]).exists():
                row["currentCli"]["sha256"] = sha256(request["output"])
    summary = {"archive": str(args.archive), "entries": len(rows),
               "mainEntries": sum(r["scope"] == "main" for r in rows),
               "historicalBlockerEntries": sum(r["scope"] != "main" for r in rows),
               "imageGroups": len({r["group"] for r in rows}),
               "featureRows": len(list(csv.DictReader((args.archive / "audit.csv").open(encoding="utf-8-sig")))),
               "mainFeatureRows": 22,
               "reconstructedRequestEntries": sum(r["mappingStatus"] == "reconstructed-explicit-default-request" for r in rows),
               "mappedEntries": sum(r["mappingStatus"] == "request-mapped" for r in rows),
               "preparedSources": len(sources),
               "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=args.repo, text=True).strip(),
               "exeSha256": sha256(args.repo / "target/debug/ooxp.exe"),
               "scopeNote": "22 main feature groups; groups 23/24 are archived historical references, not new renderer work"}
    (args.destination / "suite-manifest.json").write_text(json.dumps({"summary": summary, "entries": rows}, ensure_ascii=False, indent=2), encoding="utf-8")
    with (args.destination / "inventory.csv").open("w", encoding="utf-8-sig", newline="") as f:
        fields = ["group", "case", "scope", "mappingStatus", "originalSource", "originalSourceSha256", "officeStatus", "acceptance"]
        w = csv.DictWriter(f, fieldnames=fields, extrasaction="ignore");w.writeheader();w.writerows(rows)
    print(json.dumps(summary, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
