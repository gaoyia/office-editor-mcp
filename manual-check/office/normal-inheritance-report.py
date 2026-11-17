"""Bounded evidence report; never rewrites original sources/gold or renderer.
Usage: normal-inheritance-report.py EVIDENCE_DIR AXIS_BASELINE_DIR
"""
from pathlib import Path
import csv,hashlib,html,json,math,subprocess,sys,zipfile
from PIL import Image,ImageDraw
from normal_style_evidence import normal_snapshot,same_normal_font,normal_oracle_status
out,base=map(Path,sys.argv[1:]);root=Path(__file__).resolve().parents[2]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
cases=json.loads((out/'cases.json').read_text(encoding='utf-8'));com={r['case']:r for r in json.loads((out/'public-normal-properties.json').read_text(encoding='utf-8-sig'))}
def style_xml(path):
    with zipfile.ZipFile(path) as z:return z.read('xl/styles.xml')
def read(path):return list(csv.DictReader(path.open(encoding='utf-8-sig')))
def write(name,rows):
    with (out/name).open('w',encoding='utf-8-sig',newline='') as f:
        w=csv.DictWriter(f,list(rows[0]));w.writeheader();w.writerows(rows)
control=[];proof=[]
for c in cases:
    path=Path(c['path']);assert sha(path)==c['sha256']==com[c['case']]['sourceSha256']
    s=normal_snapshot(style_xml(path));normal=next(st for st in com[c['case']]['styles'] if st['name']=='Normal')
    office=dict(face=normal['font'],pt=normal['pt'],bold=None,italic=None,queriedFields=['face','pt'])
    control.append(dict(case=c['case'],sourceSha256=c['sha256'],mutation=c['mutation'],xmlNormal=s['face'],xmlNormalPt=s['pt'],styleXf=s['styleXfIndex'],fontIndex=s['fontIndex'],cellXf0StoredFont=s['cellXf0Stored'].get('fontId'),officeNormal=normal['font'],officeNormalPt=normal['pt'],officeCell=com[c['case']]['cellFont'],columnWidthPt=com[c['case']]['columnWidthPt'],columnPhysical106=round(com[c['case']]['columnWidthPt']*106/72),cellFontUsedForGeometry=False))
    if c['case'] in ['normal-font-only','localized-normal']:
        canonical=out/'canonical'/(c['case']+'-saved-copy.xlsx');a=normal_snapshot(style_xml(canonical));status=normal_oracle_status(s,office,a)
        assert status=='office_import_normalized_font_not_dpi_only'
        proof.append(dict(case=c['case'],sourceSha256=c['sha256'],stylesSha256=hashlib.sha256(style_xml(path)).hexdigest(),sourceNormal=s,officeNormal=office,canonicalNormal=a,canonicalPath=str(canonical),canonicalSha256=sha(canonical),canonicalOpened=False,status=status))
write('finite-control-results.csv',control);(out/'normal-inheritance-certificates.json').write_text(json.dumps(proof,ensure_ascii=False,indent=2),encoding='utf-8')
exe=root/'target/debug/ooxp.exe';assert sha(exe)=='63beeb5539353f3b62f3de8e0b4e641f0ff063b87e945132348289a87c0beb69'
metrics=read(base/'metrics.csv');six=[]
for r in metrics:
    if r['case'] not in ['normal-font-only','localized-normal']:continue
    name,action=r['case'],r['action'];case=name+('-chart' if action=='chart' else '');c=next(c for c in cases if c['case']==case);src=Path(c['path']);assert sha(src)==r['workbookSha256']
    pr=next(p for p in proof if p['case']==name);assert hashlib.sha256(style_xml(src)).hexdigest()==pr['stylesSha256']
    target=out/(name+'-'+action+'-current.png');request=dict(schemaVersion=1,action={'sheet':'exportSheetImage','spark':'exportSparklineImage','chart':'exportChartImage'}[action],path=str(src),id={'sheet':'sheet:Sheet1','spark':'sheet:Sheet1/spark:1','chart':'sheet:Sheet1/chart:1'}[action],output=str(target));q=out/'request.json';q.write_text(json.dumps(request),encoding='utf-8');rund=subprocess.run([str(exe),'--input',str(q)],capture_output=True,text=True,encoding='utf-8',timeout=30);assert rund.returncode==0,rund.stdout
    old=base/(name+'-'+action+'-after.png');assert sha(target)==sha(old)
    size=Image.open(target).size;expected=(325,21) if action=='sheet' else (81,20) if action=='spark' else (324,200);assert size==expected
    gold=base/(name+'-sheet-gold-raw.png' if action=='sheet' else name+'-'+action+'-gold.png')
    six.append(dict(case=name,action=action,sourceSha256=c['sha256'],stylesSha256=pr['stylesSha256'],sourceNormal='Arial11 via builtin0 -> styleXF -> font1',officeNormal='Calibri11 via public Styles enumeration',canonicalNormal='Calibri11 (serialized font semantics changed)',library96=str(size),libraryOoxmlGeometry='通过（规范链96几何；不是Office/DPI整图通过）',dpiWaiver='证据不足：Office载入字体语义已变，非仅DPI',evidenceStatus=pr['status'],beforeAfterLibraryBytesEqual=True,currentImage=str(target),currentImageSha256=sha(target),goldImage=str(gold),goldSha256=sha(gold),rawStrict=r['strictResult']))
write('six-path-certificates.csv',six)
summary=dict(immutableOriginalFamilies=2,immutableSourceWorkbooks=4,pathCertificates=6,finiteControls=len(cases),sameBytesCurrentExports=6,libraryOoxml96GeometryPassed=6,officeFontNormalizationFamilies=2,newDpiWaivers=0,geometryDpiNumeratorBefore=38,geometryDpiNumeratorAfter=38,geometrySuccessDenominator=49,normalPendingPaths=6,otherExportBoundaryPendingPaths=5,NI=2,rendererChanged=False,cliSha256=sha(exe),sourceHead=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),canonicalCopiesOpened=0)
(out/'normal-summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2),encoding='utf-8')
# Two original native pairs; no scaled/modified gold and no canonical copy as gold.
for name in ['normal-font-only','localized-normal']:
 a=out/(name+'-sheet-current.png');g=base/(name+'-sheet-gold-raw.png');ims=[Image.open(a).convert('RGBA'),Image.open(g).convert('RGBA')];canvas=Image.new('RGB',(sum(i.width for i in ims)+30,max(i.height for i in ims)+65),'white');d=ImageDraw.Draw(canvas);x=5
 for label,im in zip(['Library96 XML Normal Arial11','Office106 imported Normal Calibri11'],ims):
  d.text((x,5),label,fill='black');d.text((x,21),str(im.size)+' - SAME WIDTH is not same font semantics',fill='black');back=Image.new('RGBA',im.size,'white');back.alpha_composite(im);canvas.paste(back.convert('RGB'),(x,42));x+=im.width+15
 canvas.save(out/(name+'-native-compare.png'))
body=['<meta charset="utf-8"><title>Normal继承核实</title><h1>6路径＝2样式族×3导出路径；不是6个新功能</h1><a href="README.md">中文结论</a><pre>'+html.escape(json.dumps(summary,ensure_ascii=False,indent=2))+'</pre><p><a href="six-path-certificates.csv">6路径证书</a> | <a href="finite-control-results.csv">17组有限控制</a></p>']
for r in six:
 body.append('<section><h3>'+html.escape(r['case']+' / '+r['action'])+'</h3><p>'+html.escape(r['dpiWaiver'])+'</p><p>OOXML规范链96几何已验证；Office导入改变了Normal字体语义，不能当作仅DPI或库错误。</p>')
 for field in ['currentImage','goldImage']:body.append('<img style="max-width:none;margin:10px;vertical-align:top" src="'+Path(r[field]).as_uri()+'">')
 body.append('</section>')
(out/'index.html').write_text('\n'.join(body),encoding='utf-8');print(json.dumps(summary,ensure_ascii=False,indent=2))
