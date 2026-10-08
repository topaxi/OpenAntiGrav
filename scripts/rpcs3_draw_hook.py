"""A `rpcs3-drive.py place --hook` module: dump one frame of RSX state.

Stages, one per call: the command stream (following JUMPs the dump has not
reached), then the fragment programs and index ranges, the vertex ranges, the
textures of normal-map programs, and the first bytes of every bound texture
(`HOOK_LIGHT=1` skips the index and vertex stages). Returning `None` asks the
driver to resume, wait and try again when the captured frame is incomplete.
`HOOK_TGT=cc0000,10000` limits the index and vertex stages to draws on those render
targets (a few draws instead of all 700); `HOOK_NEED_SCREEN=1` also retries a frame whose
bloom chain and the HUD after it are missing, which the 200-draw floor does not catch.
See docs/reverse-engineering/rpcs3-capture.md, "Capturing one frame's draws".
"""
import sys, glob, struct, subprocess, re, os
LIGHT=os.environ.get('HOOK_LIGHT')=='1'
NO_HEADS=os.environ.get('HOOK_NO_HEADS')=='1'
# HOOK_TGT=cc0000,10000 limits the index and vertex stages to draws whose render target (0x210) is listed.
TGTS={int(x,16) for x in os.environ.get('HOOK_TGT','').split(',') if x}
def keep(d): return not TGTS or d.get(0x210) in TGTS
# HOOK_NEED_SCREEN=1 also retries a frame with no bloom-chain end (a draw on target 0x2240000) followed
# by a draw on a screen buffer (0x10000 or 0x394000): the frame stops before the passes it is read for.
def post_chain_complete(draws):
    ends=[i for i,d in enumerate(draws) if d.get(0x210)==0x2240000]
    return bool(ends) and any(d.get(0x210) in (0x10000,0x394000) for d in draws[ends[-1]:])
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
import rsx_fifo as r
BPP={0x85:4,0x86:.5,0x87:1,0x88:1,0x81:1,0x84:2,0x82:2,0x83:2,0x8b:2,0x8d:4,0x8f:.5,0x9b:2,0xa5:4,0xa6:.5,0xa7:1,0xa8:1}
def size(fmt,rect,cap=0x90000):
    f=(fmt>>8)&0x9f; w=rect>>16; h=rect&0xffff; mips=(fmt>>16)&0xf
    return min(cap,int(w*h*BPP.get(f,4)*(1.34 if mips>1 else 1))+0x100)
def base(loc): return 0xC0000000 if loc==1 else 0x40000000
ST={'stage':0,'asked':set()}
def draws_of(out,stem):
    paths=sorted(glob.glob('%s/%s-4*.bin'%(out,stem)))
    m=r.Mem.load(paths); regs={}; draws=[]
    wk=r.Walk(m)
    for pos,meth,ni,args in wk.run(0x1000):
        if meth in (0x1efc,0xb80): continue
        for i,v in enumerate(args): regs[meth if ni else meth+4*i]=v
        if meth==0x1824: regs['batches']=[(a&0xffffff,((a>>24)&0xff)+1) for a in args]
        if meth in (0x1814,0x1824): draws.append(dict(regs))
    return m,wk,draws
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
            print('hook: only %d draws, retry'%len(draws),flush=True)
            return None
        if os.environ.get('HOOK_NEED_SCREEN')=='1' and not post_chain_complete(draws):
            # a pause that caught the main thread mid-write ends in the scene, with the post chain
            # (the part measured) and the HUD missing
            print('hook: no complete post chain followed by a screen-buffer draw, retry',flush=True)
            return None
        ST['stage']=1
        res=[]
        for p in sorted(set(d[0x8e4]&0xfffffff0 for d in draws)):
            res.append((base(draws[0][0x8e4]&3)+p,0x600))
        seen=set()
        for d in ([] if LIGHT else [x for x in draws if keep(x)]):
            ixoff=d.get(0x181c)
            if d.get(0x1824) is None: continue
            for first,cnt in d.get('batches') or [(d[0x1824]&0xffffff,((d[0x1824]>>24)&0xff)+1)]:
                key=(ixoff,first,cnt)
                if key in seen: continue
                seen.add(key)
                res.append((0x40000000+ixoff+first*2,cnt*2+2))
        print('hook stage1: %d spans'%len(res),flush=True)
        if res: return res
        ST['stage']=1
    if ST['stage']==1:
        ST['stage']=2
        res=[]
        # vertex ranges per draw
        rngs={}
        for d in ([] if LIGHT else [x for x in draws if keep(x)]):
            if 0x1824 not in d or d.get(0x1824) is None: continue
            ixoff=d[0x181c]
            ix=[]
            for first,cnt in d.get('batches') or [(d[0x1824]&0xffffff,((d[0x1824]>>24)&0xff)+1)]:
                try:
                    blob=open('%s/%s-%08x.bin'%(out,stem,0x40000000+ixoff+first*2),'rb').read()
                except Exception: continue
                ix+=struct.unpack('>%dH'%cnt,blob[:cnt*2])
            if not ix: continue
            lo,hi=min(ix),max(ix)
            for a in range(16):
                off=d.get(0x1680+4*a); fmt=d.get(0x1740+4*a)
                if not off or not fmt or (fmt&0xf)==0 or ((fmt>>4)&0xf)==0: continue
                stride=(fmt>>8)&0xff
                loc=off>>31; o=off&0x7fffffff
                if not stride: continue
                key=(a,loc,o)
                span=(o+lo*stride,(hi-lo+1)*stride)
                rngs.setdefault(key,[span[0],span[0]+span[1]])
                rngs[key][0]=min(rngs[key][0],span[0]); rngs[key][1]=max(rngs[key][1],span[0]+span[1])
        tot=0
        merged={}
        for (a,loc,o),(lo,hi) in rngs.items():
            merged.setdefault(loc,[]).append((lo,hi))
        for loc,l in merged.items():
            l.sort(); cur=None
            for lo,hi in l:
                if cur and lo<=cur[1]+0x100: cur[1]=max(cur[1],hi)
                else:
                    if cur: res.append((base(1 if loc==0 else 2 if False else 0)+cur[0] if False else (0x40000000 if loc else 0xC0000000)+cur[0],cur[1]-cur[0]))
                    cur=[lo,hi]
            if cur: res.append(((0x40000000 if loc else 0xC0000000)+cur[0],cur[1]-cur[0]))
        res=[(a,min(s,0x200000)) for a,s in res]
        print('hook stage2: %d vertex spans, %d bytes'%(len(res),sum(s for a,s in res)),flush=True)
        if res: return res
        ST['stage']=2
    if ST['stage']==2:
        ST['stage']=3
        # textures: all draws whose program has TEX unit1 and the {2, -1} normal-map pattern, plus the mesh vp? keep it to those
        res=[]; seen=set()
        for d in draws:
            fp=d[0x8e4]&0xfffffff0
            try: txt=subprocess.run(['python3',os.path.join(os.path.dirname(os.path.abspath(__file__)),'ps3-fp-live.py'),'%s/%s-%08x.bin'%(out,stem,0xC0000000+fp)],capture_output=True,text=True,timeout=20).stdout
            except Exception: continue
            if '{2, -1' in txt and txt.count('unit0')>=2:
                for u in range(6):
                    b=0x1a00+0x20*u; c=d.get(b+0xc)
                    if c and c&0x80000000:
                        k=(d[b],d[b+4],d[b+0x18])
                        if k in seen: continue
                        seen.add(k); res.append((base(d[b+4]&3)+d[b],size(d[b+4],d[b+0x18])))
        print('hook stage3: %d textures'%len(res),flush=True)
        if res: return res
        ST['stage']=3
    if ST['stage']==3:
        ST['stage']=4
        if NO_HEADS: return []
        res=[]; seen=set()
        for d in draws:
            for u in range(16):
                b=0x1a00+0x20*u; c=d.get(b+0xc)
                if c and c&0x80000000 and d.get(b+4):
                    k=(d[b],d[b+4]&3)
                    if k in seen: continue
                    seen.add(k)
                    for o in (0,0x400,0x1000,0x4000,0x10000):
                        at=base(d[b+4]&3)+d[b]+o
                        if not os.path.exists('%s/%s-%08x.bin'%(out,stem,at)):
                            res.append((at,64))
        print('hook stage4: %d head spans'%len(res),flush=True)
        return res
    return []
