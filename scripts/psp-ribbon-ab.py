#!/usr/bin/env python3
"""Exact A/B of the original's engine ribbon on one frame: load a race state, stop at its first `Trail_DrawRibbon`
(0x0892acc8) hit and run on to the state's own race clock, poke the player's `Trail` ring there, and keep the EDRAM
frame (RGBA, 480x272) of the frames that follow.

    source <lane>/pp/env.sh; export DISPLAY
    python3 scripts/psp-ribbon-ab.py OUTDIR VARIANT POKE_HITS TAG     # TARGET_CLOCK=38.4082 by default

`VARIANT`: `base` (no poke), `off` (ring count 0: the ribbon is not drawn), `lz:012` / `lw:012` (zero / whiten the
baked vertex colours of the listed layers), `sq:K` and `vb:K` (scale the ring / vertex-buffer positions; neither
changed a pixel on 2026-10-21, so the geometry is not read from them at the poke). `gl:OP=VAL[,OP=VAL]` rewrites the payload of every word with that opcode in the ribbon's own state list
(0x08B65700), `clut:W` / `clut:K` whitens or scales the ribbon texture's palette, `nz:K` sets the baked normals' z (the bake
writes them every frame, so it changes nothing). None of these changed the wedge, which is why the cause was a later draw,
found by replay (`scripts/psp-ge-replay.py`). `POKE_HITS` are player draw hits
counted from the clock stop; the frame the player's draw at hit 1 builds is the one in `<TAG>_h2_B.npy` (buffer B
at hit 2 equals the pair's `theirs.png` to 0.43 grey). Emulation from the load is bit-identical run to run
(`base` twice: 0 differing bytes), so two variants differ only by the poke. Loading with the breakpoint armed puts
the first stop at the state's own first frame, which `psp-state.py load` + `freeze` does not (its stop lands
0.6 s late). Output is derived game data: keep it under data/, never commit it.
"""

import sys,os,json,struct,time,subprocess,importlib.util,shutil
from pathlib import Path
R=Path(__file__).resolve().parent.parent
sys.path.insert(0,str(R/'scripts'))
from ppsspp_debugger import Debugger
from psp_trace_fields import FLARE_POINTER,RACER_POINTER
spec=importlib.util.spec_from_file_location("st",R/"scripts/psp-state.py"); st=importlib.util.module_from_spec(spec); spec.loader.exec_module(st)
spec=importlib.util.spec_from_file_location("rf",R/"scripts/psp-race-field.py"); rf=importlib.util.module_from_spec(spec); spec.loader.exec_module(rf)
DRAW=0x0892ACC8
out=Path(sys.argv[1]); out.mkdir(parents=True,exist_ok=True)
variant=sys.argv[2]; poke_hits=[int(x) for x in sys.argv[3].split(',')]; tag=sys.argv[4]
d=Debugger(int(os.environ['OAG_PPSSPP_PORT']))
if d.is_stepping(): d.call("cpu.resume"); time.sleep(1)
target=st.state_dir(os.environ['PP_HOME'])/"UCUS98712_1.00_1.ppst"
shutil.copy(R/os.environ.get('RIBBON_STATE','data/saves/pulse-psp/race-talons-venom-alongside.ppst'),target)
st.set_menu(d,True)
d.brk()
d.add_breakpoint(DRAW)
st.click(st.LOAD_X, st.SLOT_Y0+st.SLOT_DY*1)
time.sleep(2.5)
d.pending=[]
d.call('cpu.resume')
msg=d.wait_for_break(DRAW,timeout=60)
print('first stop after load; clock',d.read_f32(0x08B344B0))
craft=rf.Field(d).player["craft"]; racer=d.read_u32(craft+RACER_POINTER); flare=d.read_u32(racer+FLARE_POINTER); node=d.read_u32(flare+0x64)+0x60
TARGET=float(os.environ.get('TARGET_CLOCK','38.4082'))
def regs():
    return dict(zip(*[(c["registerNames"],c["uintValues"]) for c in d.call("cpu.getAllRegs")["categories"][:1]][0]))
def next_player():
    while True:
        d.call("cpu.status"); d.pending=[]; d.call("cpu.resume"); d.wait_for_break(DRAW,timeout=30)
        if regs()["a0"]==node: return

n=0
while d.read_f32(0x08B344B0) < TARGET-0.0005:
    next_player(); n+=1
def apply(v):
    w=d.read_u32(node+0x10); ptr=d.read_u32(node+0x5C)
    if v=='base': return
    if v.startswith('lw:'):
        for L in [int(c) for c in v[3:]]:
            p=struct.unpack_from('<I',d.read(node+0x68+4*L,4))[0]
            raw=bytearray(d.read(p,90*32))
            for i in range(90): raw[i*32+4:i*32+8]=bytes([255,255,255,255])
            d.write(p,bytes(raw))
        return
    if v.startswith('lz:'):   # zero the baked colours of the listed layers, e.g. lz:12
        for L in [int(c) for c in v[3:]]:
            p=struct.unpack_from('<I',d.read(node+0x68+4*L,4))[0]
            raw=bytearray(d.read(p,90*32))
            for i in range(90): raw[i*32+4:i*32+8]=bytes(4)
            d.write(p,bytes(raw))
        return
    if v.startswith('vb:'):
        k=float(v[3:])
        for L in range(3):
            p=struct.unpack_from('<I',d.read(node+0x68+4*L,4))[0]
            raw=bytearray(d.read(p,90*32)); p0=struct.unpack_from('<3f',raw,20)
            for i in range(90):
                q=struct.unpack_from('<3f',raw,i*32+20); struct.pack_into('<3f',raw,i*32+20,*[p0[j]+(q[j]-p0[j])*k for j in range(3)])
            d.write(p,bytes(raw))
        return
    if v=='off': d.write_u32(node+0xC,0); return
    if v.startswith('gl:'):   # gl:OP=VAL[,OP=VAL] rewrites the payload of every word with that opcode in the ribbon's state list (0x08B65700, 0xc0 bytes), e.g. gl:23=000000 turns the depth test off
        raw=bytearray(d.read(0x08B65700,0xC0))
        print('state list',' '.join('%08x'%struct.unpack_from('<I',raw,i)[0] for i in range(0,0xC0,4)))
        for item in v[3:].split(','):
            op,val=item.split('='); op=int(op,16); val=int(val,16)
            for i in range(0,0xC0,4):
                if raw[i+3]==op: raw[i:i+3]=val.to_bytes(3,'little')
        d.write(0x08B65700,bytes(raw)); return
    if v.startswith('nz:'):   # nz:K sets the normal's z of every baked vertex of the three layers to K (the bake stores 0.725/1.275 here)
        k=float(v[3:])
        for L in range(3):
            p=struct.unpack_from('<I',d.read(node+0x68+4*L,4))[0]
            raw=bytearray(d.read(p,90*32))
            for i in range(90): struct.pack_into('<f',raw,i*32+16,k)
            d.write(p,bytes(raw))
        return
    if v.startswith('clut:'):   # clut:W whitens the ribbon texture's palette; clut:N scales it by N (clamped)
        addr=0x041F1680; raw=bytearray(d.read(addr,1024))
        for i in range(256):
            for c in range(3):
                raw[i*4+c]=255 if v[5:]=='W' else min(255,int(raw[i*4+c]*float(v[5:])))
        d.write(addr,bytes(raw)); return
    b=bytearray(d.read(ptr,320)); nw=struct.unpack_from('<3f',b,((w-1)%10)*32+4)
    if v.startswith('sq:'):
        k=float(v[3:])
        for i in range(10):
            p=struct.unpack_from('<3f',b,i*32+4); struct.pack_into('<3f',b,i*32+4,*[nw[j]+(p[j]-nw[j])*k for j in range(3)])
    elif v.startswith('seg:'):
        K=int(v[4:])
        pts=[struct.unpack_from('<3f',b,((w-1-i)%10)*32+4) for i in range(10)]
        for i in range(10):
            struct.pack_into('<3f',b,((w-1-i)%10)*32+4,*pts[min(i,K)])
    d.write(ptr,bytes(b))
import numpy as np
for h in range(0,4):
    # at hit h (h=0 is the tick0 hit already stopped at)
    if h>0: next_player()
    if h in poke_hits: apply(variant)
    if h>=1:
        raw=d.read(0x04000000+0x88000,512*272*4)
        np.save(out/('%s_h%d_B.npy'%(tag,h)),np.frombuffer(raw,dtype=np.uint8).reshape(272,512,4)[:,:480])
        raw=d.read(0x04000000,512*272*4)
        np.save(out/('%s_h%d_A.npy'%(tag,h)),np.frombuffer(raw,dtype=np.uint8).reshape(272,512,4)[:,:480])
d.brk(); d.remove_breakpoint(DRAW)
print(tag,'done')
