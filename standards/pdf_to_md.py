# -*- coding: utf-8 -*-
"""Split OOXML specification PDFs into Markdown files by bookmarks.

These PDFs are born-digital (text layer), so we extract embedded text
instead of running OCR.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

import pymupdf

ROOT = Path(__file__).resolve().parent
OUT_DIR = ROOT / "md"
MAX_PAGES_PER_FILE = 80


def safe_name(text: str) -> str:
    for ch in ("\u2013", "\u2014", "\u2212"):
        text = text.replace(ch, "-")
    text = re.sub(r"[^\w\u4e00-\u9fff.\-]+", "-", text, flags=re.ASCII)
    text = re.sub(r"-{2,}", "-", text).strip("-._")
    return text[:80] or "untitled"


def chapter_ranges(toc: list, page_count: int, level: int = 1) -> list[tuple[int, int, str]]:
    entries = [(title.strip(), page) for lvl, title, page in toc if lvl == level and page >= 1]
    ranges: list[tuple[int, int, str]] = []
    for i, (title, start) in enumerate(entries):
        end = entries[i + 1][1] - 1 if i + 1 < len(entries) else page_count
        if end < start:
            end = start
        ranges.append((start, end, title))
    return ranges


def nested_ranges(toc: list, parent_start: int, parent_end: int, child_level: int) -> list[tuple[int, int, str]]:
    children = [
        (title.strip(), page)
        for lvl, title, page in toc
        if lvl == child_level and parent_start <= page <= parent_end
    ]
    if not children:
        return []
    if children[0][1] > parent_start:
        children = [("front-matter", parent_start)] + children
    ranges: list[tuple[int, int, str]] = []
    for i, (title, start) in enumerate(children):
        end = children[i + 1][1] - 1 if i + 1 < len(children) else parent_end
        if end < start:
            end = start
        ranges.append((start, end, title))
    return ranges


def page_to_markdown(page: pymupdf.Page, page_no: int) -> str:
    text = page.get_text("text") or ""
    text = text.replace("\x00", "")
    text = re.sub(r"[ \t]+\n", "\n", text)
    text = re.sub(r"\n{3,}", "\n\n", text).strip()
    if not text:
        return f"<!-- page {page_no} (empty) -->\n"
    return f"<!-- page {page_no} -->\n\n{text}\n"


def write_range(doc: pymupdf.Document, start: int, end: int, out_path: Path, heading: str) -> None:
    parts = [f"# {heading}\n", f"<!-- source-pages {start}-{end} -->\n"]
    for page_no in range(start, end + 1):
        parts.append(page_to_markdown(doc[page_no - 1], page_no))
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text("\n".join(parts), encoding="utf-8")
    print(f"  {out_path.name}  pages {start}-{end}  ({end - start + 1})")


def convert_pdf(pdf_path: Path, dest_dir: Path) -> None:
    print(f"convert {pdf_path.name}")
    doc = pymupdf.open(pdf_path)
    toc = doc.get_toc()
    dest_dir.mkdir(parents=True, exist_ok=True)

    if not toc:
        for start in range(1, doc.page_count + 1, MAX_PAGES_PER_FILE):
            end = min(start + MAX_PAGES_PER_FILE - 1, doc.page_count)
            name = f"{start:04d}-{end:04d}.md"
            write_range(doc, start, end, dest_dir / name, f"{pdf_path.stem} {start}-{end}")
        doc.close()
        return

    index_lines = [
        "# Index\n",
        f"Source: `{pdf_path.name}`, {doc.page_count} pages.\n",
    ]
    for start, end, title in chapter_ranges(toc, doc.page_count, level=1):
        page_span = end - start + 1
        if page_span > MAX_PAGES_PER_FILE:
            children = nested_ranges(toc, start, end, child_level=2)
            if not children:
                children = [
                    (
                        s,
                        min(s + MAX_PAGES_PER_FILE - 1, end),
                        f"{title} {s}-{min(s + MAX_PAGES_PER_FILE - 1, end)}",
                    )
                    for s in range(start, end + 1, MAX_PAGES_PER_FILE)
                ]
            for c_start, c_end, c_title in children:
                slug = safe_name(f"{c_start:04d}-{c_title}")
                write_range(doc, c_start, c_end, dest_dir / f"{slug}.md", c_title)
                index_lines.append(f"- [{c_title}]({slug}.md) (p.{c_start}-{c_end})")
        else:
            slug = safe_name(f"{start:04d}-{title}")
            write_range(doc, start, end, dest_dir / f"{slug}.md", title)
            index_lines.append(f"- [{title}]({slug}.md) (p.{start}-{end})")

    (dest_dir / "00-index.md").write_text("\n".join(index_lines) + "\n", encoding="utf-8")
    doc.close()


def main() -> int:
    files = [
        ROOT / "ECMA-376-Part1-5th-2016.pdf",
        ROOT / "MS-DOCX.pdf",
        ROOT / "MS-XLSX.pdf",
        ROOT / "MS-PPTX.pdf",
    ]
    for pdf in files:
        if not pdf.exists():
            print(f"skip missing {pdf}")
            continue
        convert_pdf(pdf, OUT_DIR / pdf.stem)
    print(f"done: {OUT_DIR}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
