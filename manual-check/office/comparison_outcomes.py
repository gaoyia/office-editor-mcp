"""Portable reference qualification and remaining-target plans. No Office/desktop side effects."""
import hashlib
import json
from pathlib import Path
import zipfile
import xml.etree.ElementTree as ET

CLEAN = {'CLEAN_PUBLIC_OFFICE_EXPORT', 'RAW_EXPORT_CLEANUP_VERIFIED_NO_MATCHING_RECOVERY_LOG'}

def digest(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()

def read_json(p):
    return json.loads(Path(p).read_text(encoding='utf-8-sig'))

def qualify(c, entry, root):
    if c.get('status') not in CLEAN or c.get('referenceQualified') is False:
        return False, 'reference-status-not-qualified'
    expected=entry['prepared']['sourceSha256'].lower()
    if any(str(c.get(k,'')).lower()!=expected for k in ('sourceShaBefore','sourceShaAfter')):
        return False,'source-hash-binding-mismatch'
    if not c.get('sourceUnchanged') or not c.get('exclusiveRead'):
        return False,'source-or-close-not-verified'
    if not (c.get('ownDocumentClosed') or c.get('ownWorkbookClosed') or c.get('ownWorkbookCount')==0):
        return False,'own-document-close-not-verified'
    if c.get('repairLogs'):
        return False,'office-repair-disqualifies-reference'
    if c.get('id') and c['id']!=entry['request']['id']:
        return False,'object-id-mismatch'
    output=Path(c.get('output','')); source=Path(c.get('source',''))
    if not output.resolve().is_relative_to(root.resolve()) or not source.resolve().is_relative_to(root.resolve()):
        return False,'reference-path-outside-explicit-output-root'
    try:
        if digest(output).lower()!=str(c.get('outputSha256','')).lower():return False,'image-hash-mismatch'
        if digest(source)!=expected:return False,'current-source-hash-mismatch'
    except OSError:
        return False,'source-or-output-currently-unreadable-not-assumed-changed'
    return True,'qualified-source-range-and-cleanup'

def collect(root, manifest):
    root=Path(root);certs={};sheet={};word={}
    sp=root/'excel-oracle-plan.json'
    if sp.exists():
        for plan in read_json(sp):
            f=Path(plan['Destination'])/'office-export.json'
            if f.exists():
                c=read_json(f)
                if c.get('range')==plan['Range']:
                    for name in plan['entries']:sheet[name]=c
    for name in ('powerpoint-plan.json','excel-remaining-plan.json'):
        f=root/name
        if f.exists():
            for plan in read_json(f):
                cp=Path(plan['Certificate'])
                if cp.exists():certs[(plan['SourceSha'],plan['Id'])]=read_json(cp)
    wp=root/'word-terminal-status.json'
    if wp.exists():word={(q['sourceSha256'],q['id']):q for q in read_json(wp)}
    rows=[]
    for entry in manifest['entries']:
        q=entry.get('request',{});key=(entry.get('prepared',{}).get('sourceSha256'),q.get('id'))
        row={'entry':entry['entry'],'scope':entry['scope'],'status':'HISTORICAL_REFERENCE_ONLY','qualified':False,'semanticAcceptance':'NOT_REVIEWED_NO_AUTOMATIC_PR_MARK'}
        if entry['scope']=='main':
            if q.get('action')=='exportPageImage':
                row.update(status='BLOCKED_WORD_ORACLE_UNQUALIFIED',reason=word.get(key,{}).get('reason','No qualified Word page reference supplied'),actualVectorAttempt=word.get(key,{}).get('actualVectorAttempt',False))
            else:
                c=sheet.get(entry['entry']) if q.get('action')=='exportSheetImage' else certs.get(key)
                if c:
                    good,reason=qualify(c,entry,root);row.update(status='REFERENCE_READY' if good else 'FAILED_REFERENCE_QUALIFICATION',qualified=good,reason=reason,certificate=c)
                else:row.update(status='FAILED_REFERENCE_MISSING',reason='No successful current source-bound export certificate; not reported as run or repaired')
        rows.append(row)
    return rows

def build_plans(root):
    root=Path(root);m=read_json(root/'suite-manifest.json');plans={'word':[],'powerpoint':[],'excel-remaining':[]};seen=set()
    for r in m['entries']:
        if r['scope']!='main' or r.get('prepared',{}).get('staticErrors'):continue
        q=r.get('request',{});action=q.get('action')
        platform={'exportPageImage':'word','exportSlideImage':'powerpoint','exportChartImage':'excel-remaining','exportSparklineImage':'excel-remaining'}.get(action)
        if not platform:continue
        source=r['prepared']['source'];idx=int(q['id'].rsplit(':',1)[1]);key=(source,action,idx)
        if key in seen:continue
        seen.add(key);folder=root/'platform-native'/('excel' if platform=='excel-remaining' else platform)/Path(source).stem;folder.mkdir(parents=True,exist_ok=True)
        base=f'{action}-{idx}' if platform=='excel-remaining' else str(idx)
        item={'Source':source,'SourceSha':r['prepared']['sourceSha256'],'Action':action,'Id':q['id'],'Index':idx,'Output':str(folder/(base+'.png')),'Certificate':str(folder/(base+'.json'))}
        with zipfile.ZipFile(source) as z:
            if platform=='powerpoint':
                sz=ET.fromstring(z.read('ppt/presentation.xml')).find('{http://schemas.openxmlformats.org/presentationml/2006/main}sldSz');item['CanvasEmu']=[int(sz.get('cx')),int(sz.get('cy'))]
            elif platform=='word':
                n='{http://schemas.openxmlformats.org/wordprocessingml/2006/main}';sizes={(int(s.get(n+'w')),int(s.get(n+'h'))) for s in ET.fromstring(z.read('word/document.xml')).iter(n+'pgSz')}
                if len(sizes)!=1:raise ValueError('Word page-to-section size mapping needs explicit evidence')
                item['PaperTwips']=list(next(iter(sizes)));item['Vector']=str(folder/(base+'.emf'))
            elif action=='exportSparklineImage':
                g=ET.fromstring(z.read('xl/worksheets/sheet1.xml')).findall('.//{http://schemas.microsoft.com/office/spreadsheetml/2009/9/main}sparklineGroup')[idx-1]
                hosts=[s.text for s in g.findall('.//{http://schemas.microsoft.com/office/excel/2006/main}sqref')]
                if len(hosts)!=1 or g.get('dateAxis')=='1':raise ValueError('Existing single-host non-date-axis subset only')
                item.update(Range=hosts[0],SparkType=g.get('type','line'))
            else:
                from geometry_fixture import validate_fixture
                validate_fixture(source);item['CacheFormulaCells']='CONSISTENT'
        plans[platform].append(item)
    for name,data in plans.items():(root/(name+'-plan.json')).write_text(json.dumps(data,ensure_ascii=False,indent=2),encoding='utf-8')
    return {k:len(v) for k,v in plans.items()}

if __name__=='__main__':
    import argparse
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('out_root',type=Path);a=ap.parse_args();print(json.dumps(build_plans(a.out_root),indent=2))
