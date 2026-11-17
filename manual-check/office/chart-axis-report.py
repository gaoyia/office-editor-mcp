"""Check fair OOXML/COM/Rust axis scalars and raw CLI images; no gold resizing."""
import argparse,csv,hashlib,html,json,math,re,subprocess,xml.etree.ElementTree as E
from pathlib import Path
from zipfile import ZipFile
from PIL import Image,ImageDraw
P=argparse.ArgumentParser();P.add_argument('out',type=Path);d=P.parse_args().out
ROOT=Path(__file__).resolve().parents[2];exe=ROOT/'target/debug/ooxp.exe'
sha=lambda f:hashlib.sha256(f.read_bytes()).hexdigest()
cases=json.loads((d/'axis-probes/cases.json').read_text(encoding='utf-8'))
oracle={r['case']:r for r in csv.DictReader((d/'axis-oracle.csv').open(encoding='utf-8-sig'))}
fixture={r['case']:r for r in json.loads((ROOT/'crates/ooxp/tests/data/chart_axis_oracle.json').read_text(encoding='utf-8'))}
native={}
for m in re.findall(r'AXIS_ROW\|(\{[^\n]+\})',(d/'axis-unit-final.log').read_text(encoding='utf-8')):
 r=json.loads(m);native[r['case']]=r
N={'c':'http://schemas.openxmlformats.org/drawingml/2006/chart','m':'http://schemas.openxmlformats.org/spreadsheetml/2006/main','xdr':'http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing'}
rows=[];cards=[]
def display(im):
 bg=Image.new('RGBA',im.size,'white');dr=ImageDraw.Draw(bg)
 for y in range(0,im.height,8):
  for x in range(0,im.width,8):
   if (x//8+y//8)%2:dr.rectangle((x,y,x+7,y+7),fill='#ddd')
 return Image.alpha_composite(bg,im.convert('RGBA')).convert('RGB')
for c in cases:
 name=c['case'];path=Path(c['path']);o=oracle[name];f=fixture[name]
 assert sha(path)==c['sha256']==o['sourceSha256'].lower()==f['sourceSha256']
 with ZipFile(path) as z:
  sheet=E.fromstring(z.read('xl/worksheets/sheet1.xml'));chart=E.fromstring(z.read('xl/charts/chart1.xml'));drawing=E.fromstring(z.read('xl/drawings/drawing1.xml'))
  actual=[float(sheet.find('.//m:c[@r="B'+str(i)+'"]/m:v',N).text) for i in range(21,24)]
  cache=[float(v.text) for v in chart.findall('.//c:ser/c:val/c:numRef/c:numCache/c:pt/c:v',N)]
  assert actual==cache==c['values']==json.loads(o['values'])
  assert chart.find('.//c:ser/c:val/c:numRef/c:f',N).text=='Sheet1!$B$21:$B$23'
  assert int(drawing.find('.//xdr:oneCellAnchor/xdr:ext',N).get('cy'))==c['height96']*9525
  frame=next(el for el in drawing.iter() if el.tag.split('}')[-1]=='graphicFrame');ext=next(el for el in frame.iter() if el.tag.split('}')[-1]=='ext');assert int(ext.get('cy'))==c['height96']*9525 and int(ext.get('cx'))==c['width96']*9525
 for k in ['min','max','major','minor']:
  assert math.isclose(float(o[k]),f['expected'][k],rel_tol=1e-10,abs_tol=1e-12)
 assert int(o['scaleType'])==-4132 and o['reversed']=='False' and int(o['crosses'])==-4105
 for field,key in [('minAuto','min'),('maxAuto','max'),('majorAuto','majorUnit'),('minorAuto','minorUnit')]:
  assert (o[field]=='True')==(key not in c['explicit'])
 out=d/'axis-probes'/(name+'-after.png');req={'schemaVersion':1,'action':'exportChartImage','path':str(path),'id':'sheet:Sheet1/chart:1','output':str(out)}
 q=d/'axis-probes/request.json';q.write_text(json.dumps(req),encoding='utf-8');run=subprocess.run([str(exe),'--input',str(q)],capture_output=True,text=True,encoding='utf-8',timeout=30);response=run.stdout+run.stderr
 supported=name!='narrow'
 if supported:
  assert run.returncode==0 and out.exists(),response
  r=native[name]
  for k in ['min','max','major','minor']:assert math.isclose(float(o[k]),r[k],rel_tol=1e-9,abs_tol=1e-12),(name,k)
  assert math.isclose(float(o['crossesAt']),r['crossing'],abs_tol=1e-9),name
 else:assert 'NotImplemented' in response and not out.exists(),response
 row=dict(case=name,sourceSha256=sha(path),height96=c['height96'],officeMin=o['min'],officeMax=o['max'],officeMajor=o['major'],officeMinor=o['minor'],libraryMin=native.get(name,{}).get('min','NI'),libraryMax=native.get(name,{}).get('max','NI'),libraryMajor=native.get(name,{}).get('major','NI'),libraryMinor=native.get(name,{}).get('minor','NI'),officeCrossing=o['crossesAt'],libraryCrossing=native.get(name,{}).get('crossing','NI'),scalarMatched=supported,sourceCacheCellsMatched=True,explicit=str(c['explicit']),rawResponse=response,strictResult='NotImplemented' if not supported else 'sizeDiffer')
 if supported:
  phases=['before','after','gold'] if (d/'axis-probes'/(name+'-before.png')).exists() else ['before-crossing','after','gold'] if (d/'axis-probes'/(name+'-before-crossing.png')).exists() else ['after','gold']; ims=[Image.open(d/'axis-probes'/(name+'-'+phase+'.png')) for phase in phases]; ai=phases.index('after');gi=phases.index('gold')
  row.update(intermediateBeforeCrossingSize=str(ims[0].size) if 'before-crossing' in phases else '',beforeSize=str(ims[0].size) if 'before' in phases else 'not captured for coherent new probe; use original17 baseline',afterSize=str(ims[ai].size),goldSize=str(ims[gi].size));assert ims[ai].size==(284,c['height96'])
  row['strictResult']='sizeDiffer' if ims[ai].size!=ims[gi].size else ('PASS' if ims[ai].convert('RGBA').tobytes()==ims[gi].convert('RGBA').tobytes() else 'pixelDiffer')
  panel=Image.new('RGB',(sum(im.width for im in ims)+24,max(im.height for im in ims)+26),'white');dr=ImageDraw.Draw(panel);x=0
  for label,im in zip([{'before':'before96','before-crossing':'intermediate crossing-bottom96','after':'after96','gold':'Office106'}[phase] for phase in phases],ims):dr.text((x,3),label+' '+str(im.size),fill='black');panel.paste(display(im),(x,22));x+=im.width+8
  panel.save(d/'axis-probes'/(name+'-native-compare.png'))
  cards.append('<section><h2>'+html.escape(name)+'</h2><p>'+html.escape(str(row))+'</p><img src="axis-probes/'+name+'-native-compare.png"></section>')
 rows.append(row)
keys=list(dict.fromkeys(k for r in rows for k in r))
with (d/'axis-scalar-results.csv').open('w',encoding='utf-8-sig',newline='') as f:w=csv.DictWriter(f,keys);w.writeheader();w.writerows(rows)
summary=dict(oracleCases=len(rows),sourceCacheCellsVerified=len(rows),supportedScalarMatched=sum(r['scalarMatched'] for r in rows),supportedScalarValues=sum(r['scalarMatched'] for r in rows)*4,supportedCrossingMatched=sum(r['scalarMatched'] for r in rows),honestNarrowNI=sum(not r['scalarMatched'] for r in rows),strictPass=sum(r['strictResult']=='PASS' for r in rows),strictSizeDiffer=sum(r['strictResult']=='sizeDiffer' for r in rows))
old=d.parent/'office-editor-PR10-圆角透明修复-20261117-135846';books=list(d.glob('*.xlsx'));assert len(books)==34 and all(sha(f)==sha(old/f.name) for f in books)
summary['old34SourcesSameSha']=True
for kind in ['sheet','spark']:
 pairs=[(f,d/f.name.replace('-before.png','-after.png')) for f in d.glob('*-'+kind+'-before.png')];summary[kind+'UnchangedBytes']=sum(a.exists() and sha(a)==sha(b) for b,a in pairs)
summary['old17ChartAlphaUnchanged']=sum(Image.open(f).convert('RGBA').getchannel('A').tobytes()==Image.open(d/f.name.replace('-before.png','-after.png')).convert('RGBA').getchannel('A').tobytes() for f in d.glob('*-chart-before.png'))
(d/'axis-summary.json').write_text(json.dumps(summary,indent=2),encoding='utf-8')
(d/'axis-probes/image-manifest.json').write_text(json.dumps([dict(path=f.name,sha256=sha(f)) for f in sorted((d/'axis-probes').glob('*.png'))],indent=2),encoding='utf-8')
if (d/'index.html').exists():(d/'geometry-index.html').write_bytes((d/'index.html').read_bytes())
(d/'index.html').write_text('<!doctype html><meta charset="utf-8"><title>PR10 axis scalar repair</title><style>body{font:15px system-ui;margin:20px}section{border-top:1px solid #ccc}img{max-width:none}</style><h1>Axis scalars: independent COM / Rust / same-source CLI</h1><p>Native images, no scaling. Scalars match does NOT mean image PASS. Narrow automatic nonzero minima are NI. Default autoZero crossing is independently checked and drives the category line/ticks/labels; custom crossings/formats remain NI. Scalar matching is not a pixel-layout PASS. <a href="geometry-index.html">Original 17x3 matrix</a></p><pre>'+html.escape(json.dumps(summary,indent=2))+'</pre>'+''.join(cards),encoding='utf-8')
print(json.dumps(summary,indent=2))
assert summary['supportedScalarMatched']==26 and summary['honestNarrowNI']==1
assert summary['sheetUnchangedBytes']==16 and summary['sparkUnchangedBytes']==16 and summary['old17ChartAlphaUnchanged']==17
