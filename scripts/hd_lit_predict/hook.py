"""`rpcs3-drive.py place --hook` module: dump the lightmapped (unit1 `f[TC4].zwzz`) draws of one frame in full.

    ROAD_TOPN=16 rpcs3-drive.py ... place ... --dump 0x40cc0000:0x384000 --dump-attempts 12 \
        --hook scripts/hd_lit_predict/hook.py

Turn "Write Color Buffers" on in the private RPCS3 config first, or the scene dump reads zero.
`pred.py` predicts the frame from the dump (see renderer.md, "Every lit program ends on a /2 output scale").

Stages: 0 command stream; 1 fragment programs + every draw's indices;
2 classify by program text, vertex ranges of the lightmapped draws;
3 unit 0 and unit 1 textures of the biggest lightmapped draws, full size.
"""
import sys, glob, struct, subprocess, re, os
SCR=os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0,SCR)
import rsx_fifo as r
import importlib.util
_s=importlib.util.spec_from_file_location('rdl',SCR+'/rsx-draw-list.py'); rdl=importlib.util.module_from_spec(_s); _s.loader.exec_module(rdl)
TOPN=int(os.environ.get('ROAD_TOPN','10'))
BPP={0x85:4,0x86:.5,0x87:1,0x88:1,0x81:1,0x84:2,0x82:2,0x83:2,0x8b:2,0x8d:4,0x8f:.5,0x9b:2,0xa5:4,0xa6:.5,0xa7:1,0xa8:1}
def size(fmt,rect):
    f=(fmt>>8)&0x9f; w=rect>>16; h=rect&0xffff; mips=(fmt>>16)&0xf
    return int(w*h*BPP.get(f,4)*(1.34 if mips>1 else 1))+0x100
def base(loc): return 0xC0000000 if loc==1 else 0x40000000
ST={'stage':0,'asked':set(),'light':None}
def draws_of(out,stem):
    paths=sorted(glob.glob('%s/%s-4*.bin'%(out,stem)))
    m=r.Mem.load(paths); wk=r.Walk(m)
    for _ in wk.run(0x1000): pass
    ds,_ev=rdl.draws(out,stem)
    return m,wk,ds
def cnt_first(d):
    a=d.get(0x1824) or 0
    return sum(c for _,c in d.get('batches',[])) or (((a>>24)&0xff)+1), a&0xffffff
def fptext(out,stem,d):
    fp=d[0x8e4]&0xfffffff0
    p='%s/%s-%08x.bin'%(out,stem,0xC0000000+fp)
    try: return subprocess.run(['python3',os.path.join(SCR,'ps3-fp-live.py'),p],capture_output=True,text=True,timeout=20).stdout
    except Exception: return ''
def regions(out, stem, rnd):
    m,wk,draws=draws_of(out,stem)
    if ST['stage']==0:
        need=[]
        for pos in sorted(wk.missing):
            a=pos&~0xfff
            if a in ST['asked']: continue
            ST['asked'].add(a); need.append((0x40000000+a,0x40000))
        print('hook ring: %d draws, %d missing'%(len(draws),len(wk.missing)),flush=True)
        if need: return need
        if len(draws)<200:
            print('hook: only %d draws, retry'%len(draws),flush=True); return None
        ST['stage']=1
        res=[]
        for p in sorted(set(d[0x8e4]&0xfffffff0 for d in draws)):
            res.append((base(draws[0][0x8e4]&3)+p,0x600))
        seen=set()
        for d in draws:
            if d.get(0x1824) is None: continue
            ix=d.get(0x181c)
            if ix is None: continue
            for f,c in d.get('batches',[]):
                key=(ix,f,c)
                if key in seen: continue
                seen.add(key); res.append((0x40000000+ix+f*2,c*2))
        print('hook stage1: %d spans'%len(res),flush=True)
        return res
    if ST['stage']==1:
        ST['stage']=2
        light=[]
        for di,d in enumerate(draws):
            if d.get(0x1824) is None or d.get(0x8e4) is None: continue
            t=fptext(out,stem,d)
            if 'zwzz unit1' in t and 'LG2' in t:
                c,f=cnt_first(d); light.append((di,c,d))
        ST['light']=light
        print('hook: %d lightmapped draws of %d'%(len(light),len(draws)),flush=True)
        res=[]; rngs={}
        for di,c,d in light:
            ixoff=d[0x181c]; lo=0xffff; hi=0
            for first,cc in d['batches']:
                try: blob=open('%s/%s-%08x.bin'%(out,stem,0x40000000+ixoff+first*2),'rb').read()
                except Exception: continue
                ix=struct.unpack('>%dH'%cc,blob[:cc*2]); lo=min(lo,min(ix)); hi=max(hi,max(ix))
            if hi<lo: continue
            for a in range(16):
                off=d.get(0x1680+4*a); fmt=d.get(0x1740+4*a)
                if not off or not fmt or (fmt&0xf)==0 or ((fmt>>4)&0xf)==0: continue
                stride=(fmt>>8)&0xff
                if not stride: continue
                loc=off>>31; o=off&0x7fffffff
                key=(loc,)
                rngs.setdefault(key,[]).append((o+lo*stride,o+(hi+1)*stride))
        for (loc,),l in rngs.items():
            l.sort(); cur=None
            for lo,hi in l:
                if cur and lo<=cur[1]+0x100: cur[1]=max(cur[1],hi)
                else:
                    if cur: res.append(((0x40000000 if loc else 0xC0000000)+cur[0],cur[1]-cur[0]))
                    cur=[lo,hi]
            if cur: res.append(((0x40000000 if loc else 0xC0000000)+cur[0],cur[1]-cur[0]))
        res=[(a,min(s,0x400000)) for a,s in res]
        print('hook stage2: %d vertex spans, %d bytes'%(len(res),sum(s for a,s in res)),flush=True)
        return res
    if ST['stage']==2:
        ST['stage']=3
        light=sorted(ST['light'],key=lambda t:-t[1])[:int(os.environ.get('ROAD_TOPN','16'))]
        res=[]; seen=set()
        for di,c,d in light:
            for u in (0,1):
                b=0x1a00+0x20*u
                k=(d[b],d[b+4])
                if k in seen: continue
                seen.add(k); res.append((base(d[b+4]&3)+d[b],size(d[b+4],d[b+0x18])))
        print('hook stage3: %d textures, %d bytes'%(len(res),sum(s for a,s in res)),flush=True)
        return res
    return []
