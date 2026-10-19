#!/usr/bin/env python3
"""HD's engine-flare shape draws in one dumped frame, and whether each submits only camera-facing triangles.

    rsx-flare-shapes.py <dir> <stem>

`<dir>/<stem>-*.bin` come from `rpcs3_draw_hook.py` with the index and vertex stages kept
for the flare draws (`HOOK_LIGHT` unset; a keep() on target 0xf50000, blend on, colour mask
0x00010101, cull off, alpha func 0x204, dst factor ONE). Prints the 5 draws, their vertex
layout, and per draw the count of triangles whose authored winding faces the eye, with the
model matrix read as columns or as rows (the columns reading gives 498 of 499).
"""
import sys, struct, glob, math
import importlib.util
spec=importlib.util.spec_from_file_location('dl',__import__('os').path.join(__import__('os').path.dirname(__import__('os').path.abspath(__file__)),'rsx-draw-list.py')); dl=importlib.util.module_from_spec(spec); spec.loader.exec_module(dl)
d_dir,stem=sys.argv[1],sys.argv[2]
ds,_=dl.draws(d_dir,stem)
def rd(addr,n):
    # find any dumped file covering addr
    for f in glob.glob('%s/%s-*.bin'%(d_dir,stem)):
        base=int(f.split('-')[-1][:-4],16)
        import os
        sz=os.path.getsize(f)
        if base<=addr and addr+n<=base+sz:
            return open(f,'rb').read()[addr-base:addr-base+n]
    return None
sel=[d for d in ds if d.get(0x210)==0xf50000 and d.get(0x310,0)&1 and d.get(0x324)==0x00010101 and not d.get(0x183c,0)&1 and d.get(0x308)==0x204 and d.get(0x318)==0x10001]
print(len(sel),'flame draws')
for d in sel:
    batches=d.get('batches')
    ix=[]
    for first,cnt in batches:
        blob=rd(0x40000000+d[0x181c]+first*2,cnt*2)
        if blob is None: print('no index data'); break
        ix+=struct.unpack('>%dH'%cnt,blob)
    print('draw',d['n'],'batches',batches[:6],'mode',hex(d.get(0x1808,0)), 'n idx',len(ix),'range',min(ix) if ix else None,max(ix) if ix else None, 'ixfmt',hex(d.get(0x1820,0)))
    for a in range(16):
        off=d.get(0x1680+4*a); fmt=d.get(0x1740+4*a)
        if off is None or fmt is None: continue
        if (fmt&0xf)==0 or ((fmt>>4)&0xf)==0: continue
        print('   attr',a,'off %08x'%off,'fmt %08x'%fmt,'stride',(fmt>>8)&0xff,'type',fmt&0xf,'size',(fmt>>4)&0xf)

def sext(v,b): return v-(1<<b) if v&(1<<(b-1)) else v
def unpack_n(u):
    return (sext(u&0x7ff,11)/1023.0, sext((u>>11)&0x7ff,11)/1023.0, sext((u>>22)&0x3ff,10)/511.0)

def mat(d):
    f=lambda k: struct.unpack('>4f',struct.pack('>4I',*d['c'][k]))
    return [f(256)[:3],f(257)[:3],f(258)[:3]],f(259)[:3],f(465)[:3]
def matvec(M,v): return [sum(M[r][c]*v[c] for c in range(3)) for r in range(3)]
def transpose(M): return [[M[c][r] for c in range(3)] for r in range(3)]
def inv3(M):
    a,b,c=M[0];d,e,f=M[1];g,h,i=M[2]
    det=a*(e*i-f*h)-b*(d*i-f*g)+c*(d*h-e*g)
    return [[(e*i-f*h)/det,-(b*i-c*h)/det,(b*f-c*e)/det],[-(d*i-f*g)/det,(a*i-c*g)/det,-(a*f-c*d)/det],[(d*h-e*g)/det,-(a*h-b*g)/det,(a*e-b*d)/det]]
for d in sel:
    M,t,eye=mat(d)
    ix=[]
    for first,cnt in d['batches']:
        ix+=struct.unpack('>%dH'%cnt,rd(0x40000000+d[0x181c]+first*2,cnt*2))
    base=d[0x1680]; loc=base>>31; off=base&0x7fffffff
    lo,hi=min(ix),max(ix)
    blob=rd((0x40000000 if loc else 0xC0000000)+off+lo*18,(hi-lo+1)*18)
    if blob is None: print('draw',d['n'],'no vertex data'); continue
    pos=lambda i: struct.unpack('>3h',blob[(i-lo)*18:(i-lo)*18+6])
    tris=[ix[i:i+3] for i in range(0,len(ix)-2,3)]
    out=[]
    for name,MM in (('rows',M),('cols',transpose(M))):
        el=matvec(inv3(MM),[eye[k]-t[k] for k in range(3)])
        fr=0
        for tr in tris:
            p0,p1,p2=[pos(i) for i in tr]
            e1=[p1[k]-p0[k] for k in range(3)]; e2=[p2[k]-p0[k] for k in range(3)]
            g=[e1[1]*e2[2]-e1[2]*e2[1],e1[2]*e2[0]-e1[0]*e2[2],e1[0]*e2[1]-e1[1]*e2[0]]
            if sum(g[k]*el[k] for k in range(3))>0: fr+=1
        out.append((name,fr))
    print('draw',d['n'],'tris',len(tris),out)
