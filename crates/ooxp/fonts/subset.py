#!/usr/bin/env python3
"""Regenerate crates/ooxp/fonts/JiangChengHeiTi-400W-subset.ttf from JiangChengHeiTi 400W.

This only subsets glyphs. It does not rename the font or rewrite the name table.
Only the upright Regular 400W face is used; italic weights in the 2.1 pack are not embedded.

Default source (override with --src):
  D:/BaiduNetdiskDownload/江城黑体/JiangChengHeiTi 400W.ttf
  Version 2.1  2021/10/28
  size: 8,050,972 bytes
  SHA-256: c96b71d3931925e6bd3dd4f4f08adba5958a1d138638ea43e0e1fec95d2f29fd
"""

from __future__ import annotations

import argparse
import sys
import tempfile
from pathlib import Path

from fontTools.subset import main as subset_main

DEFAULT_SRC = Path(r"D:\BaiduNetdiskDownload\江城黑体\JiangChengHeiTi 400W.ttf")


def unicodes() -> str:
    cps = set()
    cps.update(range(0x20, 0x180))
    cps.update(range(0x2000, 0x2070))
    cps.update(range(0x20A0, 0x20D0))
    cps.update(range(0x2100, 0x2150))
    cps.update(range(0x2190, 0x21D0))
    cps.update(range(0x2500, 0x2580))
    cps.update(range(0x3000, 0x3100))
    cps.update(range(0xFF00, 0xFFEF))
    for b1 in range(0xA1, 0xF8):
        for b2 in range(0xA1, 0xFF):
            try:
                cps.update(ord(ch) for ch in bytes([b1, b2]).decode("gb2312"))
            except UnicodeDecodeError:
                pass
    return ",".join(f"U+{cp:04X}" for cp in sorted(cps))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--src", type=Path, default=DEFAULT_SRC)
    out = Path(__file__).with_name("JiangChengHeiTi-400W-subset.ttf")
    args = parser.parse_args()
    if not args.src.is_file():
        print(f"missing source font: {args.src}", file=sys.stderr)
        return 1
    with tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False, encoding="utf-8") as fh:
        fh.write(unicodes())
        uni = Path(fh.name)
    try:
        sys.argv = [
            "pyftsubset",
            str(args.src),
            f"--unicodes-file={uni}",
            f"--output-file={out}",
            "--no-hinting",
            "--desubroutinize",
            "--notdef-outline",
            "--recommended-glyphs",
            "--name-IDs=*",
            "--name-legacy",
            "--name-languages=*",
            "--layout-features=",
            "--recalc-bounds",
            "--recalc-average-width",
        ]
        subset_main()
    finally:
        uni.unlink(missing_ok=True)
    print(f"wrote {out} ({out.stat().st_size} bytes); name table left unchanged")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
