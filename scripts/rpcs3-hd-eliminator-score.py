#!/usr/bin/env python3
"""Log every change of each craft's Eliminator score on a live HD race (RPCS3).

Attach to a `rpcs3-drive.py serve` sitting in a Racebox Eliminator (`OAG_RPCS3_ATTACH=1`). Writes
`<out>/events.csv`, one row per change of a craft's score `+0x6f80`, kills `+0x6f7c`, deaths
`+0x6f78`, shield `+0x5fa0`, lap `+0x7810` or state `+0x5fa4`.

`--script` is `;`-separated `seconds:action[:arg]`: `down`/`up`/`tap` a button, `give:<state>`
the player's held pair (`6` Autopilot, `2` Quake), `shot` a frame, `target:<n>` the race target.
Autopilot only gets the craft round; never read a physics law off a run that uses it.
`docs/ghidra/functions/ps3-hdfury-eu/eliminator-score.md`.
"""
import argparse, importlib.util, struct, sys, time
from pathlib import Path
R=Path(__file__).resolve().parent; sys.path.insert(0,str(R))
s=importlib.util.spec_from_file_location('rpcs3_drive',R/'rpcs3-drive.py'); drive=importlib.util.module_from_spec(s); s.loader.exec_module(drive)
G=0x300000000; TABLE=0x98d7c0; GS=0x936fe8; RMSLOT=0x8a6814
ap=argparse.ArgumentParser(); ap.add_argument('--image',default=str(R.parent/'data/images/hdfury-ps3-eu-dec.iso'))
ap.add_argument('--out',required=True); ap.add_argument('--seconds',type=float,default=120); ap.add_argument('--script',default='')
ap.add_argument('--killat',default='',help='sec:craftindex;.. write shield to 1 at time'); ap.add_argument('--start',action='store_true')
a=ap.parse_args(); out=Path(a.out); out.mkdir(parents=True,exist_ok=True)
with drive.Session(a.image,str(out/'logs')) as sess:
    mem=sess.open_mem()
    def rd(ad,n): mem.seek(G+ad); return mem.read(n)
    u=lambda ad: struct.unpack('>I',rd(ad,4))[0]
    i=lambda ad: struct.unpack('>i',rd(ad,4))[0]
    f=lambda ad: struct.unpack('>f',rd(ad,4))[0]
    def wf(ad,v): mem.seek(G+ad); mem.write(struct.pack('>f',v))
    if a.start: sess.pad.press('cross',0.15); time.sleep(0.5)
    n=u(GS); mode=u(GS+0xe0); diff=u(GS+0xdc); tgt=u(GS+0x20); cell=u(GS+0x90); cls=u(GS+0xd4)
    rm=u(u(RMSLOT))
    print('players',n,'mode',mode,'difficulty(+0xdc)',diff,'target',tgt,'cell',hex(cell),'+0xd4',cls,'RM',hex(rm),flush=True)
    print('RM+0x1930..0x1950',[round(f(rm+0x1930+o),4) for o in range(0,0x24,4)],flush=True)
    if cell: print('cell targets',[u(cell+0xd4+k*4) for k in range(3)],flush=True)
    cr=[u(rm+0xe8+k*4) for k in range(8)]
    print('crafts',[hex(c) for c in cr],'roles',[i(c+0x628c) if c else None for c in cr],'6284',[i(c+0x6284) if c else None for c in cr],'6288',[i(c+0x6288) if c else None for c in cr],flush=True)
    print('max shield',[round(f(u(u(c+0x5fac)+0x78)+diff*4+0x90),2) for c in cr if c],flush=True)
    print('gs4e4',[hex(u(GS+0x4e4+k*4)) for k in range(8)],flush=True)
    steps=[]
    for it in filter(None,a.script.split(';')):
        p=it.split(':'); steps.append((float(p[0]),p[1],p[2] if len(p)>2 else None))
    kills=[]
    for it in filter(None,a.killat.split(';')):
        p=it.split(':'); kills.append((float(p[0]),int(p[1])))
    fh=open(out/'events.csv','w'); fh.write('t,craft,score,kills,w6f78,shield,lap,lap2,state,attacker,dmgcause,lastdmg\n')
    last={}; t0=time.time(); next_step=0; steps.sort(); kills.sort()
    nextcr=0
    while time.time()-t0<a.seconds:
        t=time.time()-t0
        if t>nextcr:
            nextcr=t+5; cr=[u(rm+0xe8+k*4) for k in range(8)]
            print('t',round(t),'n',sum(1 for c in cr if c),'p13e8',hex(u(rm+0x13e8)),'dist',[round(f(c+0x7010)) for c in cr if c],'frame',u(rm+0x1970),flush=True)
        while steps and steps[0][0]<=t:
            _,ac,arg=steps.pop(0)
            if ac=='down': sess.pad.set(arg,True)
            elif ac=='up': sess.pad.set(arg,False)
            elif ac=='tap': sess.pad.press(arg,0.06)
            elif ac=='give':
                sl=u(u(rm+0x13e8)+0x5edc); mem.seek(G+sl+0x204); mem.write(struct.pack('>ii',int(arg),int(arg)))
            elif ac=='shot':
                drive.screenshot(str(out/('shot-%05.1f.png'%t)),trim=True)
            elif ac=='target':
                mem.seek(G+GS+0x20); mem.write(struct.pack('>I',int(arg))); print('target',arg,'at',round(t,2),flush=True)
        while kills and kills[0][0]<=t:
            _,k=kills.pop(0); wf(cr[k]+0x5fa0,1.0); print('set shield=1 craft',k,'at',round(t,2),flush=True)
        for k,c in enumerate(cr):
            if not c: continue
            b=rd(c+0x5f90,0x20); sh=struct.unpack_from('>f',b,0x10)[0]; st=struct.unpack_from('>i',b,0x14)[0]
            b2=rd(c+0x6f78,12); w,ki,sc=struct.unpack('>iif',b2)
            lp=struct.unpack('>ii',rd(c+0x7810,8))
            slot=u(c+0x5edc); at=i(slot+0x13c) if slot else -9; kd=i(slot+0x138) if slot else -9
            key=(sc,ki,w,round(sh,3),lp,st)
            if last.get(k)!=key:
                last[k]=key
                fh.write(f'{t:.3f},{k},{sc},{ki},{w},{sh},{lp[0]},{lp[1]},{st},{at},{kd},{f(c+0x7d20)}\n'); fh.flush()
        time.sleep(0.004)
    print('ended7002',[rd(c+0x7002,1)[0] for c in cr if c],'phase?',[u(rm+0x1974)],flush=True)
    print('final',[(k,round(f(c+0x6f80),3),i(c+0x6f7c),i(c+0x7810),i(c+0x5fa4)) for k,c in enumerate(cr) if c],flush=True)
    for b in ('cross','triangle','square'): 
        try: sess.pad.set(b,False)
        except Exception: pass
