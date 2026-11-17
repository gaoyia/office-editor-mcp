"""Summarize identical-input before/after and retain strict, unscaled Office comparisons."""
import argparse, csv, html, json, math
from pathlib import Path
from PIL import Image, ImageDraw
p = argparse.ArgumentParser()
p.add_argument('out', type=Path)
a = p.parse_args()
d = a.out
read = lambda name: list(csv.DictReader((d / name).open(encoding='utf-8-sig')))
before = {(r['case'], r['action']): r for r in read('before.csv')}
after = read('after.csv')
gold = {(r['case'], r['action']): r for r in read('oracle.csv')}
rows = []
cards = []
for r in after:
    k = (r['case'], r['action'])
    b = before[k]
    o = gold.get(k, {})
    assert b['workbookSha256'] == r['workbookSha256'], k
    width = int(r['width'])
    height = int(r['height'])
    mdw = int(r['mdw96'])
    col = math.floor((256 * 10.125 + math.floor(128 / mdw)) / 256 * mdw) if r['widthMode'] == 'explicit' else 10 * mdw if r['widthMode'] == 'default' else math.floor(8.43 * mdw + 5)
    expected = col * (3 if r['hidden'] == 'True' else 4) + (1 if r['action'] == 'sheet' else 0) if r['action'] != 'spark' else col
    expected_h = {'sheet': 21, 'spark': 20, 'chart': 200}[r['action']]
    opath = d / f'{k[0]}-{k[1]}-gold.png'
    if r['action'] == 'sheet':
        opath = d / f'{k[0]}-sheet-gold-raw.png'
    apath = d / f'{k[0]}-{k[1]}-after.png'
    bpath = d / f'{k[0]}-{k[1]}-before.png'
    status = ('NotImplemented' if '"NotImplemented"' in r['error'] else 'exportError') if r['error'] else 'sizeDiffer' if (width, height) != (int(o.get('goldWidth', 0)), int(o.get('goldHeight', 0))) else 'pixelCheck'
    differ = delta = ''
    if status == 'pixelCheck' and opath.exists():
        im1 = Image.open(opath).convert('RGBA')
        im2 = Image.open(apath).convert('RGBA')
        pairs = list(zip(im1.getdata(), im2.getdata()))
        differ = sum((x != y for x, y in pairs))
        delta = max((max((abs(u - v) for u, v in zip(x, y))) for x, y in pairs))
        status = 'PASS' if not differ else 'pixelDiffer'
    row = {**r, 'beforeWidth': b['width'], 'beforeHeight': b['height'], 'sourceWorkbook': f"{k[0]}-{'chart' if k[1] == 'chart' else 'sheet'}.xlsx", 'ooxmlColumnSource': ('xl/worksheets/sheet1.xml:cols/col@width=10.125' if r['widthMode'] == 'explicit' else 'xl/worksheets/sheet1.xml:sheetFormatPr@defaultColWidth=10' if r['widthMode'] == 'default' else 'No stored width; existing default 8.43*MDW+5'), 'ooxmlNormalSource': 'See before-fixture-validation.json: builtinId=0 -> styleXF -> font; cellXf0 is independent', 'oracleMeasure': ('ChartObject.Width/Height points' if k[1] == 'chart' else 'Range.Width/Height points'), 'expected96Width': expected, 'expected96Height': expected_h, 'fixed96DimensionsPass': not r['error'] and (width, height) == (expected, expected_h), 'widthChanged': r['width'] != b['width'], 'goldWidth': o.get('goldWidth', ''), 'goldHeight': o.get('goldHeight', ''), 'rangeWidthPt': o.get('rangeWidthPt', ''), 'rangeHeightPt': o.get('rangeHeightPt', ''), 'oracleColumns': o.get('columns', ''), 'oracleError': o.get('error', 'missing oracle'), 'strictResult': status, 'differPixels': differ, 'maxDelta': delta}
    rows.append(row)
    images = []
    for label, path in [('before lib96', bpath), ('after lib96', apath), ('Office screen106', opath)]:
        if path.exists():
            images.append((label, Image.open(path).convert('RGBA')))
    if images:
        canvas = Image.new('RGBA', (sum((im.width + 20 for _, im in images)), max((im.height for _, im in images)) + 42), '#eeeeee')
        draw = ImageDraw.Draw(canvas)
        x = 0
        for label, im in images:
            draw.text((x + 5, 5), f'{label}: {im.width}x{im.height}', fill='black')
            # Display only: explicitly composite alpha onto white, retaining raw PNGs.
            backdrop = Image.new('RGBA', im.size, 'white')
            backdrop.alpha_composite(im)
            canvas.paste(backdrop, (x + 5, 30))
            x += im.width + 20
        compare = f'{k[0]}-{k[1]}-compare.png'
        canvas.save(d / compare)
        cards.append(f'''<section><h2>{html.escape(k[0])} / {k[1]}</h2><p>96 geometry: {row['fixed96DimensionsPass']}; strict Office: {status}. No image resized.</p><img src="{compare}"></section>''')
with (d / 'metrics.csv').open('w', encoding='utf-8-sig', newline='') as f:
    w = csv.DictWriter(f, rows[0])
    w.writeheader()
    w.writerows(rows)
summary = {'exports': len(rows), 'successfulExports': sum((not r['error'] for r in rows)), 'fixed96DimensionsPassBefore': sum((not before[r['case'], r['action']]['error'] and (int(r['beforeWidth']), int(r['beforeHeight'])) == (r['expected96Width'], r['expected96Height']) for r in rows)), 'fixed96DimensionsPass': sum((r['fixed96DimensionsPass'] for r in rows)), 'widthChanged': sum((r['widthChanged'] for r in rows)), 'officialWidthMatchesBefore': sum((int(before[r['case'], r['action']]['width']) == int(r['goldWidth'] or 0) and (not r['error']) for r in rows)), 'officialWidthMatchesAfter': sum((int(r['width']) == int(r['goldWidth'] or 0) and (not r['error']) for r in rows)), 'strictPass': sum((r['strictResult'] == 'PASS' for r in rows)), 'sizeDiffer': sum((r['strictResult'] == 'sizeDiffer' for r in rows)), 'notImplemented': sum((r['strictResult'] == 'NotImplemented' for r in rows)), 'exportErrors': sum((r['strictResult'] == 'exportError' for r in rows)), 'oracleErrors': sum((bool(r['oracleError']) for r in rows))}
(d / 'summary.json').write_text(json.dumps(summary, indent=2), encoding='utf-8')
(d / 'index.html').write_text('<!doctype html><meta charset="utf-8"><title>PR10 fixed96 geometry</title><style>body{font:15px system-ui;margin:24px}section{border-top:1px solid #bbb;margin-top:24px}img{max-width:none}pre{white-space:pre-wrap}</style><h1>Shared column geometry: native-size comparisons</h1><p>Office logical screen DPI 106; library geometry contract 96. These are NOT rescaled oracles. P0 before/after use identical fair inputs and the same renderer; only the probe/report was repaired, not sheet content or renderer pixels. Different DPI glyph quantization is not a uniform scale. Hidden-source spark remains NotImplemented. Chart formula/caches/actual cells are validated before either CLI phase. localized-normal is the retained combined experiment, not a pure localization test; name-only, XF-index-only, Normal-font-only and cell-font-only are separate. White display backgrounds alpha-composite raw PNGs; raw alpha remains part of strict comparison. Sheet specimens also contain an embedded sparkline which the sheet renderer does not draw; this existing content residual is not a DPI difference.</p><pre>' + html.escape(json.dumps(summary, indent=2)) + '</pre>' + ''.join(cards), encoding='utf-8')
print(json.dumps(summary, indent=2))
for r in rows:
    if r['action'] == 'sheet':
        print(r['case'], r['beforeWidth'], r['width'], r['goldWidth'], r['strictResult'])
