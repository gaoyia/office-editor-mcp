"""Measure native chart alpha independently of size/layout; never rescale a gold."""
import argparse, csv, hashlib, html, json
from pathlib import Path
from PIL import Image, ImageChops, ImageDraw
p = argparse.ArgumentParser()
p.add_argument('out', type=Path)
d = p.parse_args().out
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
def white(im):
    bg = Image.new('RGBA', im.size, 'white')
    return Image.alpha_composite(bg, im.convert('RGBA')).convert('RGB')
def checker(im):
    bg = Image.new('RGBA', im.size, 'white')
    draw = ImageDraw.Draw(bg)
    for y in range(0,im.height,8):
        for x in range(0,im.width,8):
            if (x//8+y//8)%2: draw.rectangle((x,y,x+7,y+7),fill=(205,205,205,255))
    return Image.alpha_composite(bg,im.convert('RGBA')).convert('RGB')
rows=[]; cards=[]
for f in sorted(d.glob('*-chart-before.png')):
    name=f.name.removesuffix('-chart-before.png')
    paths=[f,d/(name+'-chart-after.png'),d/(name+'-chart-gold.png')]
    ims=[Image.open(p).convert('RGBA') for p in paths]
    b,a,g=ims
    al=list(a.getchannel('A').getdata()); gl=list(g.getchannel('A').getdata())
    interior=sum(x!=y for x,y,z in zip(b.convert('RGB').getdata(),a.convert('RGB').getdata(),al) if z==255)
    comp=sum(x!=y for x,y in zip(white(b).getdata(),white(a).getdata()))
    corners=lambda im: [im.getpixel(q)[3] for q in [(0,0),(im.width-1,0),(0,im.height-1),(im.width-1,im.height-1)]]
    row=dict(case=name,beforeMode=Image.open(f).mode,afterMode=Image.open(paths[1]).mode,beforeSize=str(b.size),afterSize=str(a.size),goldSize=str(g.size),beforeCornerAlpha=str(corners(b)),afterCornerAlpha=str(corners(a)),goldCornerAlpha=str(corners(g)),afterTransparent=sum(v==0 for v in al),afterPartial=sum(0<v<255 for v in al),goldTransparent=sum(v==0 for v in gl),goldPartial=sum(0<v<255 for v in gl),opaqueRgbDiffer=interior,whiteCompositeDiffer=comp,sourceSha256=sha(d/(name+'-chart.xlsx')),beforeSha256=sha(f),afterSha256=sha(paths[1]),goldSha256=sha(paths[2]),strictResult='sizeDiffer' if a.size!=g.size else ('PASS' if a.tobytes()==g.tobytes() else 'pixelDiffer'))
    rows.append(row)
    width=sum(im.width for im in ims)+24; height=max(im.height for im in ims)+50
    panel=Image.new('RGB',(width,height),'white'); draw=ImageDraw.Draw(panel); x=0
    for label,im in zip(['P1 before96','P3 after96','Office native106'],ims):
        draw.text((x,4),label+' '+str(im.size),fill='black');panel.paste(checker(im),(x,24));x+=im.width+8
    panel.save(d/(name+'-alpha-native-compare.png'))
    crop=Image.new('RGB',(3*48,55),'white'); cd=ImageDraw.Draw(crop)
    for i,(label,im) in enumerate(zip(['before','after','Office'],ims)):
        cd.text((i*48,0),label,fill='black');crop.paste(checker(im.crop((0,0,32,32))),(i*48,18))
    crop.save(d/(name+'-alpha-corner-native.png'))
    cards.append('<section><h2>'+html.escape(name)+'</h2><p>'+html.escape(str(row))+'</p><img src="'+name+'-alpha-native-compare.png"><p>Native 32px corner crops; not rescaled / not strict gold.</p><img src="'+name+'-alpha-corner-native.png"></section>')
with (d/'alpha-metrics.csv').open('w',encoding='utf-8-sig',newline='') as f:
    w=csv.DictWriter(f,rows[0]);w.writeheader();w.writerows(rows)
summary=dict(chartCases=len(rows),cornerAlphaFixed=sum(r['afterCornerAlpha']=='[0, 0, 0, 0]' for r in rows),opaqueRgbDiffer=sum(r['opaqueRgbDiffer'] for r in rows),whiteCompositeDiffer=sum(r['whiteCompositeDiffer'] for r in rows),strictPass=sum(r['strictResult']=='PASS' for r in rows),strictSizeDiffer=sum(r['strictResult']=='sizeDiffer' for r in rows),goldRescaled=False)
for kind in ['sheet','spark']:
    pairs=[(f,d/(f.name.replace('-before.png','-after.png'))) for f in d.glob('*-'+kind+'-before.png')]
    summary[kind+'UnchangedBytes']=sum(sha(b)==sha(a) for b,a in pairs if a.exists())
(d/'alpha-summary.json').write_text(json.dumps(summary,indent=2),encoding='utf-8')
(d/'index.html').write_text('<!doctype html><meta charset="utf-8"><title>P3 native chart alpha</title><style>body{font:15px system-ui;margin:20px}section{border-top:1px solid #bbb}img{max-width:none}</style><h1>P3 chart alpha (native size)</h1><p>Checkerboard is display only. Raw gold/after/before remain unscaled. Same-source P0 native gold reused; separately named round-oracle images record optional new COM observations. White composite and opaque RGB are independent regressions, not Office PASS.</p><pre>'+html.escape(json.dumps(summary,indent=2))+'</pre>'+''.join(cards),encoding='utf-8')
manifest=[dict(path=f.name,sha256=sha(f),size=f.stat().st_size) for f in sorted(d.glob('*.png'))]
(d/'image-manifest.json').write_text(json.dumps(manifest,indent=2),encoding='utf-8')
print(json.dumps(summary,indent=2))
assert summary['cornerAlphaFixed']==len(rows)
assert summary['opaqueRgbDiffer']==0
assert summary['whiteCompositeDiffer']==0
