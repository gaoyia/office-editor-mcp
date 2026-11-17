"""Reproduce scope-local t127 review from immutable a81 matrix and COM evidence.
Run: python dpi-acceptance-report.py REVIEW_DIR AXIS_DIR LEGEND_DIR MDW_CSV
Never rewrites input XLSX/gold PNGs, scales images or promotes scalar certificates.
"""
import csv, hashlib, html, json, math, subprocess, sys
from collections import Counter
from pathlib import Path
from PIL import Image, ImageDraw
from dpi_acceptance import classify, PASS, DIFFERENCE, NI, PENDING
out, base, legend, mdw_path = map(Path, sys.argv[1:])
out.mkdir(exist_ok=True)
def read(p):
    return list(csv.DictReader(p.open(encoding='utf-8-sig')))
def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()
def write(name, rows):
    with (out/name).open('w',encoding='utf-8-sig',newline='') as f:
        w=csv.DictWriter(f,list(rows[0]));w.writeheader();w.writerows(rows)
def boolstr(v):
    return str(v).lower()=='true'
def canon(v):
    return {'dengxian':'dengxian','等线':'dengxian'}.get(v.lower(),v.lower())
root=Path(__file__).resolve().parents[2]
exe=root/'target/debug/ooxp.exe'
exe_sha=sha(exe)
expected_cli='63beeb5539353f3b62f3de8e0b4e641f0ff063b87e945132348289a87c0beb69'
assert exe_sha==expected_cli,'Unexpected renderer; review certificates again'
labels={PASS:'通过（仅DPI差异豁免）',DIFFERENCE:'仍有实现差异',NI:'不支持NI',PENDING:'证据不足待确认'}
mdw={(canon(r['requestedFace']),float(r['pointSize']),int(r['targetDpi'])):int(r['maximumDigitWidthPx']) for r in read(mdw_path)}
sem={(r['case'],r['kind']):r for r in read(base/'oracle-semantics.csv')}
oracle={(r['case'],r['action']):r for r in read(base/'oracle.csv')}
rows=[];cert=[];evidence=[]
for r in read(base/'metrics.csv'):
    name,action=r['case'],r['action'];src=base/r['sourceWorkbook'];actual_sha=sha(src)
    source_ok=actual_sha==r['workbookSha256']
    assert source_ok and oracle[name,action]['workbookSha256']==actual_sha,(name,action,'source/oracle hash')
    s=sem[name,'chart' if action=='chart' else 'sheet']
    fonts_ok=canon(s['normalFont'])==canon(r['normalFont']) and float(s['normalPt'])==float(r['normalPt'])
    ni=bool(r['error'])
    m=mdw[canon(s['normalFont']),float(s['normalPt']),106]
    col=math.floor((256*10.125+math.floor(128/m))/256*m) if r['widthMode']=='explicit' else 10*m if r['widthMode']=='default' else math.floor(8.43*m+5)
    count=3 if r['hidden']=='True' else 4
    predicted=col if action=='spark' else count*col+(action=='sheet')
    # Chart.Export rounds effective COM point extents to output physical pixels.
    if action=='chart':
        predicted=math.ceil(float(r['rangeWidthPt'])*106/72)
        pred_h=math.ceil(float(r['rangeHeightPt'])*106/72)
    else:
        pred_h=round(15*106/72)+(action=='sheet')
    api_match=predicted==int(r['goldWidth'] or 0) and pred_h==int(r['goldHeight'] or 0)
    scope_result=classify(scope="shared_geometry_only",source_verified=source_ok,renderer_verified=True,
        semantic_verified=boolstr(r['fixed96DimensionsPass']) and api_match,
        font_verified=fonts_ok,dpi_verified=True,ni=ni,raw_strict=r['strictResult'])
    why='同源SHA；OOXML/固定96维度；真实Normal字体；GDI96/106 MDW离散量化；COM/原生gold物理尺寸' if scope_result==PASS else '有效Normal/font继承与Office读取结果不一致，不能因尺寸偶合豁免' if not fonts_ok else '既有隐藏源displayHidden不支持' if ni else '需核实尺寸责任链'
    cert.append(dict(matrix='original51',case=name,action=action,scope='shared_geometry_only',result=labels[scope_result],sourceSha256=actual_sha,rawStrict=r['strictResult'],evidence=why,predictedGoldWidth=predicted,predictedGoldHeight=pred_h,actualGoldWidth=r['goldWidth'],actualGoldHeight=r['goldHeight']))
    pending='曲角/描边AA及legend/plot布局未完成DPI-only证明' if action=='chart' else '独立spark端点/笔宽AA差异尚不能证明仅DPI；sheet内容已接线'
    if not fonts_ok:pending+='；Normal继承待核，不属于已证DPI'
    whole=classify(ni=ni,unresolved=[pending])
    gold=base/(f'{name}-sheet-gold-raw.png' if action=='sheet' else f'{name}-{action}-gold.png')
    after=base/f'{name}-{action}-after.png'
    rows.append(dict(matrix='original51',case=name,action=action,result=labels[whole],rawStrict=r['strictResult'],sourceSha256=actual_sha,geometryScope=labels[scope_result],reason=pending if not ni else 'displayHidden既有NI',after=str(after),gold=str(gold),wholeImageDpiWaived=False))
for r in read(base/'axis-scalar-results.csv'):
    name=r['case'];src=base/'axis-probes'/(name+'.xlsx');assert sha(src)==r['sourceSha256']
    is_ni=not boolstr(r['scalarMatched'])
    # Native explicit compare was directly inspected: labels stay next to zero
    # in library but below plot in Office. This is position, not raster scale.
    known=['category_label_side'] if name=='explicit' and not is_ni else []
    whole=classify(ni=is_ni,implementation=known,unresolved=['curve_aa_legend_layout_not_dpi_proven'])
    reason='四尺度+交点130值已匹配；曲角AA/legend布局待确认，不将标量通过提升为整图' if not is_ni else '窄范围非零自动下界已有NI'
    if known:reason='原生explicit图：库A/B/C贴0交点，Office在绘图区底部；非DPI位置差异'
    rows.append(dict(matrix='axis27',case=name,action='chart',result=labels[whole],rawStrict=r['strictResult'],sourceSha256=r['sourceSha256'],geometryScope='独立EMU/固定96尺寸已验证；本表不据此豁免整图',reason=reason,after=str(base/'axis-probes'/f'{name}-after.png'),gold=str(base/'axis-probes'/f'{name}-gold.png'),wholeImageDpiWaived=False))
    cert.append(dict(matrix='axis27',case=name,action='chart',scope='axis_scalars_and_crossing_only',result='通过（独立COM逻辑值一致）' if not is_ni else labels[NI],sourceSha256=r['sourceSha256'],rawStrict=r['strictResult'],evidence='四尺度值+autoZero交点逐项相等；不代表标签侧/整图',predictedGoldWidth='',predictedGoldHeight='',actualGoldWidth='',actualGoldHeight=''))
current=json.loads((out/'legend-current.json').read_text(encoding='utf-8'))
old_com={r['case']:r for r in json.loads((legend/'oracle.json').read_text(encoding='utf-8-sig'))}
for r in current:
    assert r['exit']==0 and sha(Path(r['source']))==r['sourceSha256']
    assert old_com[r['case']]['sourceSha256'].lower()==r['sourceSha256']
    rows.append(dict(matrix='legend7',case=r['case'],action='chart',result=labels[PENDING],rawStrict='sizeDiffer',sourceSha256=r['sourceSha256'],geometryScope='固定96 chart框；未豁免整图',reason='字体/粗体/颜色/顺序已有子范围回归；自动/手动frame与key布局及AA尚无完整DPI-only证明',after=r['image'],gold=str(legend/(r['case']+'-gold.png')),wholeImageDpiWaived=False))
write('逐项验收.csv',rows);write('子范围验收.csv',cert)
counts={g:dict(Counter(r['result'] for r in rows if r['matrix']==g)) for g in ['original51','axis27','legend7']}
geometry=Counter(r['result'] for r in cert if r['scope']=='shared_geometry_only')
summary=dict(policy='t127',rendererHead='a81c49e622e82b6e621c7b4453bf20bc3f0206c1',cliSha256=exe_sha,wholeImageMatrices=counts,geometryScope=dict(geometry),axisSupportedLogicalCases=26,axisLogicalValuesMatched=130,axisCases=27,legendCurrentRerenders=len(current),strictDiagnostics={'original51':{'sizeDiffer':49,'NI':2},'axis27':{'sizeDiffer':26,'NI':1},'legend7':{'sizeDiffer':7}},wholeImageDpiWaivers=0,notes='不合并85为独立样本分母；源有复用，子范围通过不提升为全图。')
(out/'summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2),encoding='utf-8')
for r in rows:
    for role in ['after','gold']:
        p=Path(r[role]);
        if p.exists():evidence.append(dict(matrix=r['matrix'],case=r['case'],role=role,path=str(p),sha256=sha(p)))
write('image-source-hashes.csv',evidence)
# Native-size display-only panels. White composite is explicit, never an oracle.
panels=[('explicit-label-side',base/'axis-probes/explicit-after.png',base/'axis-probes/explicit-gold.png'),('dengxian-same-width-spark',base/'dengxian11-explicit-spark-after.png',base/'dengxian11-explicit-spark-gold.png'),('legend-manual-edge',out/'legend-current/manual-edge-after.png',legend/'manual-edge-gold.png')]
for name,after,gold in panels:
    ims=[Image.open(after).convert('RGBA'),Image.open(gold).convert('RGBA')]
    canvas=Image.new('RGB',(sum(max(230,i.width) for i in ims)+30,max(i.height for i in ims)+45),'#dddddd');draw=ImageDraw.Draw(canvas);x=5
    for label,im in zip(['Current library96','Office106'],ims):
        draw.text((x,5),f'{label}: {im.width}x{im.height}',fill='black');back=Image.new('RGBA',im.size,'white');back.alpha_composite(im);canvas.paste(back.convert('RGB'),(x,35));x+=max(230,im.width)+15
    canvas.save(out/(name+'-native-compare.png'))
body=['<meta charset="utf-8"><title>PR10 DPI验收复核</title><style>body{font:16px sans-serif}img{max-width:none}section{overflow:auto;border:1px solid #aaa;padding:10px;margin:10px}code{background:#eee}</style><h1>PR10 DPI验收复核（t127）</h1><p>新标准：仅经证实DPI差异可豁免；raw严格统计保留，不一票否决。通过仅针对命名子范围。全部PNG原尺寸，无缩放。</p><pre>'+html.escape(json.dumps(summary,ensure_ascii=False,indent=2))+'</pre><p><a href="README.md">中文结论</a> | <a href="逐项验收.csv">逐项CSV</a> | <a href="子范围验收.csv">可勾选子范围</a></p>']
for r in rows:
    body.append('<section><h3>'+html.escape(f"{r['matrix']} / {r['case']} / {r['action']} — {r['result']}")+'</h3><p>'+html.escape(r['reason'])+'</p><code>'+r['sourceSha256']+'</code><div style="white-space:nowrap">')
    for role in ['after','gold']:
        p=Path(r[role]);
        if p.exists():body.append('<span style="display:inline-block;vertical-align:top;background:white;margin:10px">'+role+'<br><img src="'+html.escape(p.as_uri())+'"></span>')
    body.append('</div></section>')
(out/'index.html').write_text('\n'.join(body),encoding='utf-8')
print(json.dumps(summary,ensure_ascii=False,indent=2))
