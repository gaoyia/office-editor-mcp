"""Fixed-96 column geometry regression corpus. Never rescale an oracle bitmap."""
import argparse, csv, hashlib, json, subprocess, zipfile, xml.etree.ElementTree as ET
from pathlib import Path
from geometry_fixture import merge_chart_seed, used_dimension, rewrite_styles, validate_fixture
from datetime import datetime, timezone, timedelta
P = argparse.ArgumentParser()
P.add_argument('phase', choices=['before', 'after'])
P.add_argument('out', type=Path)
a = P.parse_args()
ROOT = Path(__file__).resolve().parents[2]
EXE = ROOT / 'target/debug/ooxp.exe'
OUT = a.out.resolve()
OUT.mkdir(parents=True, exist_ok=True)
if a.phase == 'before' and (OUT / 'before.csv').exists():
    raise SystemExit('Refusing to overwrite baseline evidence; use a new directory')
NS = 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'
N = {'m': NS}
ET.register_namespace('', NS)

def run(req):
    req = {'schemaVersion': 1, **req}
    f = OUT / 'request.json'
    f.write_text(json.dumps(req), encoding='utf-8')
    r = subprocess.run([str(EXE), '--input', str(f)], cwd=ROOT, capture_output=True, text=True, encoding='utf-8', timeout=30)
    if r.returncode:
        raise RuntimeError(r.stdout + r.stderr)
    return r.stdout
CASES = [('calibri11-explicit', 'Calibri', 11, 7, 'explicit', False, False, False), ('calibri11-default', 'Calibri', 11, 7, 'default', False, False, False), ('calibri11-absent', 'Calibri', 11, 7, 'absent', False, False, False), ('arial11-explicit', 'Arial', 11, 8, 'explicit', False, False, False), ('arial11-default', 'Arial', 11, 8, 'default', False, False, False), ('arial11-absent', 'Arial', 11, 8, 'absent', False, False, False), ('consolas11-explicit', 'Consolas', 11, 8, 'explicit', False, False, False), ('dengxian11-explicit', 'DengXian', 11, 8, 'explicit', False, False, False), ('dengxian11-absent', 'DengXian', 11, 8, 'absent', False, False, False), ('calibri14-explicit', 'Calibri', 14, 10, 'explicit', False, False, False), ('localized-normal', 'Arial', 11, 8, 'explicit', False, False, True), ('hidden-column', 'Calibri', 11, 7, 'explicit', True, False, False), ('non-a1', 'Arial', 11, 8, 'explicit', False, True, False)]
# Retain the old combined case explicitly; these four isolate semantic variables.
CASES += [('localized-normal-name-only', 'Calibri', 11, 7, 'explicit', False, False, 'name-only'), ('normal-xf-index-only', 'Calibri', 11, 7, 'explicit', False, False, 'xf-only'), ('normal-font-only', 'Arial', 11, 8, 'explicit', False, False, 'font-only'), ('cell-font-only', 'Calibri', 11, 7, 'explicit', False, False, 'cell-only')]
if a.phase == 'before':
    src = OUT / 'seed.xlsx'
    run({'action': 'create', 'path': str(src), 'workbook': {'sheets': [{'name': 'Sheet1', 'cells': [{'ref': 'B1', 'value': 1}, {'ref': 'C1', 'value': 4}, {'ref': 'D1', 'value': 2}]}]}})
    run({'action': 'apply', 'path': str(src), 'output': str(OUT / 'spark-seed.xlsx'), 'ops': [{'op': 'insertSparkline', 'id': 'sheet:Sheet1', 'ref': 'A1', 'formula': 'B1:D1'}]})
    run({'action': 'apply', 'path': str(src), 'output': str(OUT / 'chart-seed.xlsx'), 'ops': [{'op': 'insertChart', 'id': 'sheet:Sheet1', 'type': 'col', 'anchor': 'twoCell', 'from': 'A3', 'to': 'E13', 'dataFrom': 'A20', 'series': [{'name': 'S', 'categories': ['A', 'B', 'C'], 'values': [1, 4, 2]}]}]})
    for name, face, pt, mdw, mode, hidden, non_a1, localized in CASES:
        for kind in ['sheet', 'chart']:
            seed = OUT / ('spark-seed.xlsx' if kind == 'sheet' else 'chart-seed.xlsx')
            book = OUT / f'{name}-{kind}.xlsx'
            with zipfile.ZipFile(seed) as z, zipfile.ZipFile(book, 'w', zipfile.ZIP_DEFLATED) as dst:
                for entry in z.infolist():
                    data = z.read(entry.filename)
                    if entry.filename == 'xl/styles.xml':
                        data = rewrite_styles(data, face, pt, localized)
                    elif entry.filename == 'xl/worksheets/sheet1.xml':
                        st = ET.fromstring(data)
                        seed_sheet = ET.fromstring(data)
                        for tag in ['dimension', 'sheetViews', 'sheetFormatPr', 'cols', 'sheetData']:
                            for e in st.findall(f'm:{tag}', N):
                                st.remove(e)
                        row = '2' if non_a1 else '1'
                        host = 'B2' if non_a1 else 'A1'
                        refs = ['C2', 'D2', 'E2'] if non_a1 else ['B1', 'C1', 'D1']
                        ref = 'B2:E2' if non_a1 else 'A1:D1'
                        elems = [ET.Element('{' + NS + '}dimension', {'ref': ref})]
                        sv = ET.Element('{' + NS + '}sheetViews')
                        ET.SubElement(sv, '{' + NS + '}sheetView', {'showGridLines': '0', 'workbookViewId': '0', 'zoomScale': '100'})
                        elems.append(sv)
                        pr = ET.Element('{' + NS + '}sheetFormatPr', {'defaultRowHeight': '15', 'customHeight': '1'})
                        if mode == 'default':
                            pr.set('defaultColWidth', '10')
                        elems.append(pr)
                        if mode == 'explicit' or hidden:
                            cols = ET.Element('{' + NS + '}cols')
                            for i in range(1, 11):
                                at = {'min': str(i), 'max': str(i), 'width': '10.125', 'customWidth': '1'}
                                if hidden and i == 3:
                                    at['hidden'] = '1'
                                ET.SubElement(cols, '{' + NS + '}col', at)
                            elems.append(cols)
                        sd = ET.Element('{' + NS + '}sheetData')
                        r = ET.SubElement(sd, '{' + NS + '}row', {'r': row, 'ht': '15', 'customHeight': '1'})
                        ET.SubElement(r, '{' + NS + '}c', {'r': host})
                        for cr, v in zip(refs, [1, 4, 2]):
                            c = ET.SubElement(r, '{' + NS + '}c', {'r': cr})
                            ET.SubElement(c, '{' + NS + '}v').text = str(v)
                        if kind == 'chart':
                            merge_chart_seed(seed_sheet, sd)
                        elems[0].set('ref', used_dimension(sd))
                        elems.append(sd)
                        for i, e in enumerate(elems):
                            st.insert(i, e)
                        for e in st.iter():
                            if e.tag.endswith('}sqref'):
                                e.text = host
                            if e.tag.endswith('}f') and kind == 'sheet':
                                e.text = 'Sheet1!C2:E2' if non_a1 else 'Sheet1!B1:D1'
                        data = ET.tostring(st, encoding='utf-8', xml_declaration=True)
                    elif entry.filename.startswith('xl/drawings/drawing') and entry.filename.endswith('.xml') and non_a1:
                        st = ET.fromstring(data)
                        for mark in st.iter():
                            if mark.tag.endswith('}col') or mark.tag.endswith('}row'):
                                mark.text = str(int(mark.text) + 1)
                        data = ET.tostring(st, encoding='utf-8', xml_declaration=True)
                    dst.writestr(entry, data)
rows = []
validations = {}
for name, face, pt, mdw, mode, hidden, non_a1, localized in CASES:
    for action, kind, obj in [('exportSheetImage', 'sheet', 'sheet:Sheet1'), ('exportSparklineImage', 'sheet', 'sheet:Sheet1/spark:1'), ('exportChartImage', 'chart', 'sheet:Sheet1/chart:1')]:
        tag = {'exportSheetImage': 'sheet', 'exportSparklineImage': 'spark', 'exportChartImage': 'chart'}[action]
        book = OUT / f'{name}-{kind}.xlsx'
        validations[book.name] = validate_fixture(book)
        output = OUT / f'{name}-{tag}-{a.phase}.png'
        if output.exists():
            output.unlink()
        try:
            run({'action': action, 'path': str(book), 'id': obj, 'output': str(output)})
            b = output.read_bytes()
            w = int.from_bytes(b[16:20], 'big')
            h = int.from_bytes(b[20:24], 'big')
            error = ''
        except Exception as e:
            w = h = 0
            error = str(e)
        rows.append(dict(case=name, action=tag, normalFont=face, normalPt=pt, mdw96=mdw, widthMode=mode, hidden=hidden, range=(('B2:E2' if non_a1 else 'A1:D1') if tag == 'sheet' else ('B2' if non_a1 else 'A1') if tag == 'spark' else ('anchor:B4:F14' if non_a1 else 'anchor:A3:E13')), width=w, height=h, error=error, workbookSha256=hashlib.sha256(book.read_bytes()).hexdigest()))
(OUT / f'{a.phase}-fixture-validation.json').write_text(json.dumps(validations, ensure_ascii=False, indent=2), encoding='utf-8')
with (OUT / f'{a.phase}.csv').open('w', encoding='utf-8-sig', newline='') as f:
    w = csv.DictWriter(f, rows[0])
    w.writeheader()
    w.writerows(rows)
print(OUT)
print(f"{a.phase}: {len(rows)} exports, {sum((bool(r['error']) for r in rows))} errors")

provenance = {'timeAsiaShanghai': datetime.now(timezone(timedelta(hours=8))).isoformat(), 'sourceHead': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(), 'workingTreeDirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT)), 'cli': str(EXE), 'cliSha256': hashlib.sha256(EXE.read_bytes()).hexdigest()}
(OUT / f'{a.phase}-provenance.json').write_text(json.dumps(provenance, indent=2), encoding='utf-8')
