"""Single portable index: qualified raw references, explicit blocked rows, view-only previews."""
import argparse
import collections
import csv
from functools import reduce
import html
import json
from pathlib import Path
from PIL import Image,ImageChops,ImageDraw
from comparison_outcomes import collect,read_json,digest


def pair(gold,lib,native,view,dpi,qualified):
    a,b=Image.open(gold).convert('RGBA'),Image.open(lib).convert('RGBA')
    def save(x,y,path,label):
        w=max(x.width,y.width)+20;canvas=Image.new('RGB',(w*2,max(x.height,y.height)+65),'white');d=ImageDraw.Draw(canvas)
        d.text((8,5),label,fill='#a00000');d.text((8,24),'Office' if qualified else 'LEGACY Office: not a qualified current gold',fill='black');d.text((w+8,24),'Current library 96 DPI',fill='black');canvas.paste(x,(8,48),x);canvas.paste(y,(w+8,48),y);canvas.save(path)
    save(a,b,native,'RAW original pixels: no resize / no acceptance implied')
    if qualified and dpi and dpi!=96:
        x=a.resize((max(1,round(a.width*96/dpi)),max(1,round(a.height*96/dpi))),Image.Resampling.LANCZOS)
        save(x,b,view,f'VIEW ONLY: Office {dpi}->96 physical-scale preview. NOT GOLD / NOT PIXEL PASS')
    else:save(a,b,view,'VIEW ONLY: no resampling; blocked rows retain clearly marked legacy reference')
    difference=None
    if qualified and a.size==b.size:
        delta=reduce(ImageChops.lighter,ImageChops.difference(a,b).split());difference=sum(delta.histogram()[1:])
    return list(a.size),list(b.size),difference


def main():
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('out_root',type=Path);p=ap.parse_args().out_root
    m=read_json(p/'suite-manifest.json');outcomes=collect(p,m);records=[];cards=[]
    for entry,outcome in zip(m['entries'],outcomes):
        folder=p/'entries'/entry['group']/entry['case'];folder.mkdir(parents=True,exist_ok=True)
        q=outcome.get('certificate',{});qualified=outcome['qualified'];gold=Path(q['output']) if qualified else Path(entry['oldGold'])
        current=entry.get('currentCli',{});current_path=Path(current.get('output','__missing__'));has_current=current_path.is_file() and current.get('returnCode')==0
        lib=current_path if has_current else Path(entry['oldLibrary']);dpi=q.get('excelWindowDpi',q.get('targetDpi')) if qualified else None
        sizes,difference=None,None
        a,b,difference=pair(gold,lib,folder/'native-pair.png',folder/'view-only-96dpi.png',dpi,qualified)
        action=entry.get('request',{}).get('action','historical');platform={'exportPageImage':'word','exportSlideImage':'powerpoint','exportChartImage':'excel-chart','exportSparklineImage':'excel-spark','exportSheetImage':'excel-sheet'}.get(action,'historical')
        r={'entry':entry['entry'],'scope':entry['scope'],'platform':platform,'status':outcome['status'],'qualifiedReference':qualified,'reason':outcome.get('reason','historical reference only'),'actualWordVectorAttempt':outcome.get('actualVectorAttempt',False),
           'currentLibraryAvailable':has_current,'officeWidth':a[0],'officeHeight':a[1],'libraryWidth':b[0],'libraryHeight':b[1],'nativeSameSize':a==b if qualified else None,'rawPixelDifferDiagnostic':difference,'officeDpi':dpi,'libraryDpi':96,
           'sourceSha256':entry.get('prepared',{}).get('sourceSha256',''),'officeImageSha256':digest(gold),'libraryImageSha256':digest(lib),'semanticAcceptance':'NOT_REVIEWED_NO_AUTOMATIC_PR_MARK','canvasNote':q.get('apiCanvasNote','')}
        records.append(r);rel=folder.relative_to(p).as_posix();title=html.escape(entry['entry']);note=html.escape(outcome['status']);reason=html.escape(outcome.get('reason',''))
        cards.append(f'<details><summary>{title} — {note}</summary><p>{reason}</p><p>raw {a} vs {b}; pixel diagnostic={difference}</p><p><a href="{rel}/native-pair.png">原尺寸并排</a> · <a href="{rel}/view-only-96dpi.png">仅展示视图</a></p><img src="{rel}/view-only-96dpi.png" loading="lazy" alt="仅展示，不是gold"></details>')
    main_rows=[r for r in records if r['scope']=='main'];ready=[r for r in main_rows if r['qualifiedReference']]
    platform_counts={k:{'entries':sum(r['platform']==k for r in main_rows),'qualified':sum(r['platform']==k for r in ready)} for k in sorted({r['platform'] for r in main_rows})}
    stats={'mainFeatures':22,'mainImageRecords':len(main_rows),'historicalRecords':len(records)-len(main_rows),'noImageFeatureRows':2,'mainTerminalStates':dict(collections.Counter(r['status'] for r in main_rows)),
           'qualifiedReferenceRecords':len(ready),'qualifiedNativeSameSize':sum(r['nativeSameSize'] for r in ready),'qualifiedNativeSizeDifferent':sum(not r['nativeSameSize'] for r in ready),
           'rawDiagnosticPixelZero':sum(r['rawPixelDifferDiagnostic']==0 for r in ready),'platforms':platform_counts,'semanticReview':'NOT_PERFORMED; no renderer changes or PR acceptance flags',
           'word':'one full-paper vector diagnostic not qualified; remaining pages explicitly blocked, not reported as run','viewOnly':'Physical-scale preview uses measured DPI only; never replaces raw gold. Discrete MDW/canvas seams are not automatically waived.'}
    (p/'progress-summary.json').write_text(json.dumps(stats,ensure_ascii=False,indent=2),encoding='utf-8')
    (p/'terminal-outcomes.json').write_text(json.dumps(outcomes,ensure_ascii=False,indent=2),encoding='utf-8')
    with (p/'comparison-status.csv').open('w',encoding='utf-8-sig',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(records[0]));w.writeheader();w.writerows(records)
    header='''<!doctype html><meta charset="utf-8"><title>PR10 测试集收口与交接</title><style>body{font:16px sans-serif;max-width:1200px;margin:auto}details{border:1px solid #ccc;padding:10px;margin:8px}img{max-width:none}pre{white-space:pre-wrap}.warn{color:#a00000}</style><h1>PR10 测试基础修正：有效状态清单与交接</h1><p class="warn">这不是58个功能全部通过。Word阻塞行未伪造参考；原始图保留，view-only不是gold。用户后续9项已取消，不继续实现。</p>'''
    (p/'index.html').write_text(header+'<pre>'+html.escape(json.dumps(stats,ensure_ascii=False,indent=2))+'</pre><p><a href="README.md">中文交接说明</a> · <a href="comparison-status.csv">逐记录尺寸/状态/SHA</a></p>'+''.join(cards),encoding='utf-8');print(json.dumps(stats,ensure_ascii=False,indent=2))

if __name__=='__main__':main()
