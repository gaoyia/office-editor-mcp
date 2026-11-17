"""Render an honest suite index; raw images are never resized or overwritten."""
import argparse
import collections
import csv
import hashlib
import html
import json
import math
from pathlib import Path
import shutil
from PIL import Image, ImageDraw


def digest(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def comparison(gold, lib, native, view, dpi):
    a,b=Image.open(gold).convert('RGBA'),Image.open(lib).convert('RGBA')
    def paste_pair(x,y,out,title):
        w=max(x.width,y.width)+20; h=max(x.height,y.height)+65
        canvas=Image.new('RGB',(2*w,h),'white');d=ImageDraw.Draw(canvas)
        d.text((8,5),title,fill='#9b0000');d.text((8,24),'Office',fill='black');d.text((w+8,24),'Library 96 DPI',fill='black')
        canvas.paste(x,(8,48),x);canvas.paste(y,(w+8,48),y);canvas.save(out)
    paste_pair(a,b,native,'RAW: original pixels, no resize')
    if dpi and dpi!=96:
        scaled=a.resize((max(1,round(a.width*96/dpi)),max(1,round(a.height*96/dpi))),Image.Resampling.LANCZOS)
        paste_pair(scaled,b,view,f'VIEW ONLY: Office {dpi}->96 DPI resampling; NOT GOLD / NOT PIXEL PASS')
    else:
        paste_pair(a,b,view,'VIEW ONLY: no DPI resampling (no proven current DPI)')
    return list(a.size),list(b.size)


def main():
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('directory',type=Path);args=ap.parse_args();p=args.directory
    m=json.loads((p/'suite-manifest.json').read_text(encoding='utf-8'))
    proof={}
    for f in (p/'excel-native').glob('*/office-export.json'):
        q=json.loads(f.read_text(encoding='utf-8-sig'))
        if q['status']=='RAW_EXPORT_CLEANUP_VERIFIED_NO_MATCHING_RECOVERY_LOG' and q.get('exclusiveRead') and q['sourceUnchanged']:
            proof[q['sourceShaBefore'].lower()]=q
    records=[];cards=[]
    for r in m['entries']:
        folder=p/'entries'/r['group']/r['case'];folder.mkdir(parents=True,exist_ok=True)
        current=r.get('currentCli',{}); lib=Path(current.get('output','__absent__'))
        valid_current=lib.is_file() and current.get('returnCode')==0
        q=proof.get(r.get('prepared',{}).get('sourceSha256'))
        fresh=q is not None and valid_current
        if fresh:
            gold=Path(q['output']);dpi=q['excelWindowDpi'];state='FRESH_SAME_SOURCE_RANGE_NOT_SEMANTIC_PASS'
            r['officeStatus']=q['status'];r['officeEvidence']=q
        else:
            gold=Path(r['oldGold']);dpi=None;state='LEGACY_REFERENCE_NOT_REEXPORTED' if r['scope']=='main' else 'HISTORICAL_REFERENCE_ONLY'
        if not valid_current:lib=Path(r['oldLibrary'])
        sizes=comparison(gold,lib,folder/'native-pair.png',folder/'view-only-96dpi.png',dpi)
        rec={'entry':r['entry'],'scope':r['scope'],'status':state,'freshOffice':fresh,'nativeSameSize':sizes[0]==sizes[1],
             'officeWidth':sizes[0][0],'officeHeight':sizes[0][1],'libraryWidth':sizes[1][0],'libraryHeight':sizes[1][1],
             'officeDpi':dpi,'libraryDpi':96,'sourceSha256':r.get('prepared',{}).get('sourceSha256',''),
             'officePngSha256':digest(gold),'libraryPngSha256':digest(lib),'pixelDiffer':None,'acceptance':'USER_REVIEW_NOT_STARTED'}
        records.append(rec)
        rel=folder.relative_to(p).as_posix();title=html.escape(r['entry']);note='新同源参考（106 DPI）；展示归一非gold' if fresh else '历史参考未重导：不能假称当前同源验收完成'
        cards.append(f'<details><summary>{title} — {note}</summary><p>raw {sizes[0]} vs {sizes[1]} / {state}</p><p><a href="{rel}/native-pair.png">原像素并排</a> · <a href="{rel}/view-only-96dpi.png">仅展示96DPI视图</a></p><img src="{rel}/view-only-96dpi.png" alt="仅展示，非gold" loading="lazy"></details>')
    fresh=[r for r in records if r['freshOffice']]
    stats={'mainFeatureGroups':22,'mainImageEntries':58,'historicalReferenceEntries':12,'allArchiveEntries':70,
           'featuresWithoutImageEvidence':2,'preparedSources':m['summary']['preparedSources'],
           'currentLibraryImages':sum(bool(r.get('currentCli',{}).get('sha256')) for r in m['entries']),
           'freshOfficeSheetSources':len(proof),'freshOfficeEntries':len(fresh),
           'freshNativeSameSize':sum(r['nativeSameSize'] for r in fresh),
           'freshNativeSizeMismatch':sum(not r['nativeSameSize'] for r in fresh),
           'mainAwaitingFreshOfficeEntries':sum(r['scope']=='main' and not r['freshOffice'] for r in records),
           'semanticReview':'NOT_STARTED_NO_AUTOMATIC_PR_MARKS','note':'Infrastructure progress only. DPI known does not prove all content/geometry differences DPI-only.'}
    (p/'progress-summary.json').write_text(json.dumps(stats,ensure_ascii=False,indent=2),encoding='utf-8')
    (p/'suite-manifest.json').write_text(json.dumps(m,ensure_ascii=False,indent=2),encoding='utf-8')
    with (p/'comparison-status.csv').open('w',encoding='utf-8-sig',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(records[0]));w.writeheader();w.writerows(records)
    old=Path(m['summary']['archive'])
    shutil.copyfile(old/'audit.csv',p/'archived-feature-rows.csv')
    header='''<!doctype html><meta charset="utf-8"><title>PR10 整套测试修正（进行中）</title><style>body{font:16px sans-serif;max-width:1200px;margin:auto}details{border:1px solid #ccc;padding:10px;margin:8px}img{max-width:none}pre{white-space:pre-wrap}.warn{color:#a00000}</style><h1>PR10 整套测试修正：进行中，暂不逐项验收</h1><p class="warn">只修测试基础设施，不批量修renderer、不勾PR。旧0914快照原封保留。下列展示归一图不是raw gold，不代表像素通过。</p>'''
    (p/'index.html').write_text(header+'<pre>'+html.escape(json.dumps(stats,ensure_ascii=False,indent=2))+'</pre><p><a href="README.md">中文说明</a> · <a href="comparison-status.csv">逐项状态/哈希</a></p>'+''.join(cards),encoding='utf-8')
    print(json.dumps(stats,ensure_ascii=False,indent=2))


if __name__=='__main__':main()
