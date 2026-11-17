"""Pure OOXML checks for the geometry oracle corpus (not renderer behavior)."""
import copy
import re
import zipfile
import xml.etree.ElementTree as ET
from decimal import Decimal

NS = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
CNS = "http://schemas.openxmlformats.org/drawingml/2006/chart"
N = {"m": NS, "c": CNS}


def coord(ref):
    match = re.fullmatch(r"([A-Z]+)([1-9][0-9]*)", ref.replace("$", ""))
    assert match, ref
    col = 0
    for ch in match[1]:
        col = col * 26 + ord(ch) - 64
    return col, int(match[2])


def column(n):
    s = ""
    while n:
        n, digit = divmod(n - 1, 26)
        s = chr(65 + digit) + s
    return s


def merge_chart_seed(sheet, new_data):
    """Preserve chart formula data; never confuse the selected range with used range."""
    old = sheet.find("m:sheetData", N)
    if old is not None:
        for row in old:
            if int(row.get("r")) >= 20:
                assert not any(r.get("r") == row.get("r") for r in new_data)
                new_data.append(copy.deepcopy(row))
    new_data[:] = sorted(new_data, key=lambda row: int(row.get("r")))


def used_dimension(data):
    cells = [coord(c.get("r")) for c in data.findall("m:row/m:c", N)]
    assert cells
    lo = min(c[0] for c in cells), min(c[1] for c in cells)
    hi = max(c[0] for c in cells), max(c[1] for c in cells)
    return f"{column(lo[0])}{lo[1]}:{column(hi[0])}{hi[1]}"


def rewrite_styles(data, face, pt, variant=False):
    st = ET.fromstring(data)
    fonts = st.find("m:fonts", N)
    styles = st.find("m:cellStyles", N)
    normal = next(x for x in styles if x.get("builtinId") == "0" or x.get("name") == "Normal")
    normal.set("builtinId", "0")
    xfs = st.find("m:cellStyleXfs", N)
    font = fonts[0]
    if variant in (True, "font-only", "cell-only"):
        font = copy.deepcopy(font)
        fonts.append(font)
        fonts.set("count", str(len(fonts)))
        font.find("m:name", N).set("val", "Arial")
        font.find("m:sz", N).set("val", "11")
    if variant is True or variant == "xf-only":
        xf = copy.deepcopy(xfs[int(normal.get("xfId"))])
        if variant is True:
            xf.set("fontId", str(len(fonts) - 1))
        xfs.append(xf)
        xfs.set("count", str(len(xfs)))
        normal.set("xfId", str(len(xfs) - 1))
    if variant is True or variant == "name-only":
        normal.set("name", "常规重命名")
    if variant == "font-only":
        xfs[int(normal.get("xfId"))].set("fontId", str(len(fonts) - 1))
    if variant == "cell-only":
        cell = st.find("m:cellXfs", N)[0]
        cell.set("fontId", str(len(fonts) - 1))
        cell.set("applyFont", "1")
    if variant and variant != "cell-only":
        normal.set("customBuiltin", "1")
    if not variant:
        font.find("m:name", N).set("val", face)
        font.find("m:sz", N).set("val", str(pt))
    return ET.tostring(st, encoding="utf-8", xml_declaration=True)


def validate_fixture(path):
    """Fail before export if actual cells, formula references, caches, or XF chains disagree."""
    with zipfile.ZipFile(path) as z:
        sheet = ET.fromstring(z.read("xl/worksheets/sheet1.xml"))
        cells = {}
        for c in sheet.findall("m:sheetData/m:row/m:c", N):
            if c.get("t") == "inlineStr":
                value = "".join(t.text or "" for t in c.findall(".//m:t", N))
            elif c.get("t") == "s":
                strings = ET.fromstring(z.read("xl/sharedStrings.xml"))
                item = list(strings)[int(c.findtext("m:v", namespaces=N))]
                value = "".join(t.text or "" for t in item.findall(".//m:t", N))
            else:
                value = c.findtext("m:v", default="", namespaces=N)
            cells[c.get("r")] = value
        assert sheet.find("m:dimension", N).get("ref") == used_dimension(sheet.find("m:sheetData", N)), "incorrect used dimension"
        st = ET.fromstring(z.read("xl/styles.xml"))
        fonts, xfs, cxfs = (st.find("m:" + tag, N) for tag in ("fonts", "cellStyleXfs", "cellXfs"))
        for x in xfs:
            assert 0 <= int(x.get("fontId", "0")) < len(fonts)
        for x in cxfs:
            assert 0 <= int(x.get("xfId", "0")) < len(xfs)
            assert 0 <= int(x.get("fontId", "0")) < len(fonts)
        normals = [x for x in st.find("m:cellStyles", N) if x.get("builtinId") == "0"]
        assert len(normals) == 1
        normal = normals[0]
        ni = int(normal.get("xfId"))
        assert 0 <= ni < len(xfs)
        nf = int(xfs[ni].get("fontId", "0"))
        assert 0 <= ni < len(xfs) and 0 <= nf < len(fonts)
        refs = []
        for name in z.namelist():
            if not re.fullmatch(r"xl/charts/chart[0-9]+[.]xml", name):
                continue
            chart = ET.fromstring(z.read(name))
            for ref in list(chart.findall(".//c:strRef", N)) + list(chart.findall(".//c:numRef", N)):
                formula = ref.findtext("c:f", namespaces=N)
                sn, addresses = formula.rsplit("!", 1)
                assert sn.strip("'") == "Sheet1", formula
                bounds = addresses.split(":")
                a, b = coord(bounds[0]), coord(bounds[-1])
                actual = []
                for row in range(a[1], b[1] + 1):
                    for col in range(a[0], b[0] + 1):
                        address = f"{column(col)}{row}"
                        assert address in cells, f"missing chart cell {address}: {formula}"
                        actual.append(cells[address])
                numeric = ref.tag.endswith("}numRef")
                cache = ref.find("c:numCache" if numeric else "c:strCache", N)
                assert cache is not None, formula
                points = sorted(cache.findall("c:pt", N), key=lambda x: int(x.get("idx")))
                assert [int(x.get("idx")) for x in points] == list(range(len(actual))), formula
                assert int(cache.find("c:ptCount", N).get("val")) == len(actual), formula
                cached = [x.findtext("c:v", namespaces=N) for x in points]
                assert ([Decimal(v) for v in actual] == [Decimal(v) for v in cached] if numeric else actual == cached), f"cache disagrees: {formula}"
                refs.append({"formula": formula, "actual": actual, "cache": cached})
        cf = int(cxfs[0].get("fontId", "0"))
        def font_info(index):
            f = fonts[index]
            return {"name": f.find("m:name", N).get("val"), "pt": f.find("m:sz", N).get("val")}
        return {"file": str(path), "dimension": sheet.find("m:dimension", N).get("ref"), "chartReferences": refs, "normalName": normal.get("name"), "normalXf": ni, "normalFont": font_info(nf), "cellXf0Font": font_info(cf), "customBuiltin": normal.get("customBuiltin", "0")}
