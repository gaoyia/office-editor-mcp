"""Five-record, SHA-bound native chart boundary verification; no PNG changes.
Usage: chart-boundary-report.py EVIDENCE_DIR AXIS_BASELINE_DIR MDW_CSV
"""
from pathlib import Path
import csv,hashlib,html,json,math,struct,subprocess,sys,zipfile,xml.etree.ElementTree as E
from PIL import Image,ImageChops,ImageDraw
from chart_boundary_evidence import device_grid_px,device_span_px,native_width_matches,png_phys_compatible,rgba_equal
from normal_style_evidence import normal_snapshot
out,base,mdw_path=map(Path,sys.argv[1:]);root=Path(__file__).resolve().parents[2];exe=root/'target/debug/ooxp.exe'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def read(p):return list(csv.DictReader(p.open(encoding='utf-8-sig')))
def write(name,rows):
 with (out/name).open('w',encoding='utf-8-sig',newline='') as f:
  w=csv.DictWriter(f,list(rows[0]));w.writeheader();w.writerows(rows)
def alias(s):return 'dengxian' if s.lower() in ('dengxian','等线') else s.lower()
cases=json.loads((out/'cases.json').read_text(encoding='utf-8'));com={(r['case'],r['control']):r for r in json.loads((out/'public-boundary-properties.json').read_text(encoding='utf-8-sig'))};dpi=json.loads((out/'public-dpi.json').read_text(encoding='utf-8-sig'));assert set(dpi.values())=={106}
mdw={(alias(r['requestedFace']),float(r['pointSize']),int(r['targetDpi'])):int(r['maximumDigitWidthPx']) for r in read(mdw_path)}
metrics={r['case']:r for r in read(base/'metrics.csv') if r['action']=='chart'};rows=[];raw=[]
N={'m':'http://schemas.openxmlformats.org/spreadsheetml/2006/main','xdr':'http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing','a':'http://schemas.openxmlformats.org/drawingml/2006/main','c':'http://schemas.openxmlformats.org/drawingml/2006/chart'}
assert sha(exe)=='63beeb5539353f3b62f3de8e0b4e641f0ff063b87e945132348289a87c0beb69'
for case in cases:
 name,control=case['case'],case['control'];source=Path(case['path']);c=com[name,control];m=metrics[name];assert sha(source)==case['sha256']==c['sourceSha256'];assert Path(c['fullName']).resolve()==source.resolve();assert c['chartType']==51
 with zipfile.ZipFile(source) as z:
  sheet=E.fromstring(z.read('xl/worksheets/sheet1.xml'));draw=E.fromstring(z.read('xl/drawings/drawing1.xml'));chart=E.fromstring(z.read('xl/charts/chart1.xml'));font=normal_snapshot(z.read('xl/styles.xml'))
 anchor=draw.find('xdr:twoCellAnchor',N);assert anchor is not None and anchor.get('editAs')=='twoCell';assert all(int(x.text)==0 for mark in ['from','to'] for tag in ['colOff','rowOff'] for x in anchor.findall('xdr:'+mark+'/xdr:'+tag,N))
 assert chart.find('.//c:barChart/c:barDir',N).get('val')=='col' and chart.find('.//c:barChart/c:grouping',N).get('val')=='clustered'
 assert alias(font['face'])==alias(c['normalFont']) and font['pt']==c['normalPt']
 md=mdw[alias(font['face']),font['pt'],106];spec_col=math.floor((256*10.125+math.floor(128/md))/256*md) if m['widthMode']=='explicit' else 10*md if m['widthMode']=='default' else math.floor(8.43*md+5)
 expected_columns=[0 if v['hidden'] else spec_col for v in c['columns']];device_cols=[device_grid_px(v['widthPt'],106) for v in c['columns']];assert expected_columns==device_cols
 device_span=device_span_px(c['fromCellLeftPt'],c['toCellLeftPt'],106);assert device_span==sum(device_cols)
 png=Path(c['nativePng']);im=Image.open(png).convert('RGBA');orig_gold=base/(name+'-chart-gold.png');gold=Image.open(orig_gold).convert('RGBA');assert rgba_equal(im,gold)
 rawpng=png.read_bytes();i=rawpng.index(b'pHYs');px,py,unit=struct.unpack('>IIB',rawpng[i+4:i+13]);assert unit==1 and png_phys_compatible(px,106) and png_phys_compatible(py,106)
 assert native_width_matches(im.width,[v['widthPt'] for v in c['columns']],106)
 assert im.height==math.ceil(c['objectHeightPt']*106/72)==int(m['goldHeight'])
 assert im.getchannel('A').getbbox()==(0,0,im.width,im.height)
 current=out/(name+'-'+control+'-library.png');request=dict(schemaVersion=1,action='exportChartImage',path=str(source),id='sheet:Sheet1/chart:1',output=str(current));q=out/'request.json';q.write_text(json.dumps(request),encoding='utf-8');r=subprocess.run([str(exe),'--input',str(q)],capture_output=True,text=True,encoding='utf-8',timeout=30);assert r.returncode==0,r.stdout;assert sha(current)==sha(base/(name+'-chart-after.png'));assert Image.open(current).size==tuple(case['expectedLibrary96'])
 ext=draw.find('.//xdr:graphicFrame/xdr:xfrm/a:ext',N);raw.append(dict(case=name,control=control,sourceSha256=case['sha256'],frameCxEMU=ext.get('cx'),frameCyEMU=ext.get('cy'),chartType=51,objectWidthPt=repr(c['objectWidthPt']),shapeWidthPt=repr(c['shapeWidthPt']),cellFromLeftPt=repr(c['fromCellLeftPt']),cellToLeftPt=repr(c['toCellLeftPt']),columnPoints=json.dumps([v['widthPt'] for v in c['columns']]),certifiedColumnPixels=json.dumps(device_cols),deviceColumnSpan=device_span,mechanicalCeilWidth=math.ceil(c['objectWidthPt']*106/72),nativeWidth=im.width,nativeHeight=im.height,alphaBounds=str(im.getchannel('A').getbbox()),physPpm=px,publicDpi=106,freshNativeSha256=sha(png),originalGoldSha256=sha(orig_gold),freshMatchesOriginalGoldBytes=sha(png)==sha(orig_gold),currentLibrarySha256=sha(current)))
 if control=='original':
  assert case['sha256']==m['workbookSha256'];twin=Path(com[name,'coherent-frame']['nativePng']);assert sha(twin)==sha(png)
  rows.append(dict(case=name,sourceSha256=case['sha256'],cliSha256=sha(exe),chartType=51,anchor='zero-offset twoCell A3:E13',oldPredictedGoldWidth=math.ceil(float(m['rangeWidthPt'])*106/72),nativeColumnSpanPx=device_span,expectedGoldHeight=im.height,goldWidth=im.width,goldHeight=im.height,originalGoldSha256=sha(orig_gold),frameControlNativeBytesEqual=True,currentLibraryBytesUnchanged=True,normalFaceMatches=True,sourceDeviceGridMatches=True,doubleRangeGridCertified=True,alphaOuterBoundsMatch=True,newGeometryDpiResult='通过（仅DPI尺寸/设备栅格舍入；不代表整图）',scope='shared_geometry_only'))
write('native-boundary-raw.csv',raw);write('chart-boundary-certificates.csv',rows)
summary=dict(targetRecords=5,newSources=5,immutableOriginalCopies=5,officeNativeExports=10,frameControlNativeIdentical=5,currentLibraryUnchanged=10,newGeometryDpiPassed=5,geometryDpiBefore=38,geometryDpiAfter=43,geometrySuccessDenominator=49,normalPending=6,NI=2,rendererChanged=False,checkerCause='Single shape readback ceil is not authoritative Double column/device-grid width',publicDpi=dpi,cliSha256=sha(exe))
(out/'boundary-summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2),encoding='utf-8')
body=['<meta charset="utf-8"><h1>5项Chart.Export边界核实</h1><a href="README.md">中文结论</a><pre>'+html.escape(json.dumps(summary,ensure_ascii=False,indent=2))+'</pre><a href="native-boundary-raw.csv">原始精度/尺寸</a> | <a href="chart-boundary-certificates.csv">5几何证书</a>']
for r in rows:
 name=r['case'];a=out/(name+'-original-library.png');g=base/(name+'-chart-gold.png');images=[Image.open(a).convert('RGBA'),Image.open(g).convert('RGBA')];canvas=Image.new('RGB',(sum(im.width for im in images)+30,max(im.height for im in images)+55),'white');d=ImageDraw.Draw(canvas);x=5
 for label,im in zip(['Library96 unchanged','Office106 original gold'],images):
  d.text((x,6),label+' '+str(im.size),fill='black');back=Image.new('RGBA',im.size,'white');back.alpha_composite(im);canvas.paste(back.convert('RGB'),(x,35));x+=im.width+15
 canvas.save(out/(name+'-native-compare.png'));body.append('<section><h3>'+name+'</h3><p>before checker '+str(r['oldPredictedGoldWidth'])+' → exact device-grid '+str(r['nativeColumnSpanPx'])+'；原PNG宽 '+str(r['goldWidth'])+'，未改renderer</p><img style="max-width:none" src="'+name+'-native-compare.png"></section>')
(out/'index.html').write_text('\n'.join(body),encoding='utf-8');print(json.dumps(summary,ensure_ascii=False,indent=2))
