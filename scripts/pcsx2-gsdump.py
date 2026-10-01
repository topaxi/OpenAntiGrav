#!/usr/bin/env python3
"""Reads a PCSX2 single-frame GS dump (`.gs`, already `zstd -d`'d) and lists its draws.

    zstd -d "WipEout Pulse_SCES-54748_<stamp>.gs.zst" -o frame.gs
    scripts/pcsx2-gsdump.py frame.gs                 # one line per state group
    scripts/pcsx2-gsdump.py frame.gs --draw 602      # that group's registers and vertices

A *state group* is a run of primitives under one set of drawing registers
(`TEX0`, `ALPHA`, `TEST`, `FRAME`, `ZBUF`, `CLAMP`, `MIPTBP`, ...). Each vertex
carries the `RGBAQ`, `ST/Q`, `UV` and `XYZ(F)` that were current when it kicked.
See `docs/reverse-engineering/pcsx2-debugger.md` "GS dumps" for how a dump is
taken, the file layout this walks, and the traps (a dump is truncated until the
emulator has run several more frames; texture-pool addresses differ between two
runs of the same frame, so compare groups by `PRIM`, count and state, not `TBP`).

Layout, measured on PCSX2 v2.7.494: `u32 0xffffffff`, `u32 header_size`, the
header (`u32 state_version`, `u32 state_size`, serial, a screenshot), then
`state_size` bytes of GS state (the 4 MiB of VRAM among them), 8,192 bytes of
privileged registers, and packets: `0` transfer (`u8 path`, `u32 size`, GIF
data), `1` vsync (`u8 field`), `2` read FIFO (`u32 size`), `3` registers (8,192
bytes). Only transfers are decoded here.
"""
"""Minimal PCSX2 .gs dump reader and GIF decoder: yields draws with the GS state in force."""
import struct, collections, sys, argparse
REGNAME={0x00:'PRIM',0x01:'RGBAQ',0x02:'ST',0x03:'UV',0x04:'XYZF2',0x05:'XYZ2',0x06:'TEX0_1',0x07:'TEX0_2',0x08:'CLAMP_1',0x09:'CLAMP_2',0x0a:'FOG',0x0c:'XYZF3',0x0d:'XYZ3',
 0x14:'TEX1_1',0x15:'TEX1_2',0x16:'TEX2_1',0x17:'TEX2_2',0x18:'XYOFFSET_1',0x19:'XYOFFSET_2',0x1a:'PRMODECONT',0x1b:'PRMODE',0x1c:'TEXCLUT',0x22:'SCANMSK',
 0x34:'MIPTBP1_1',0x35:'MIPTBP1_2',0x36:'MIPTBP2_1',0x37:'MIPTBP2_2',0x3b:'TEXA',0x3d:'FOGCOL',0x3f:'TEXFLUSH',0x40:'SCISSOR_1',0x41:'SCISSOR_2',0x42:'ALPHA_1',0x43:'ALPHA_2',
 0x44:'DIMX',0x45:'DTHE',0x46:'COLCLAMP',0x47:'TEST_1',0x48:'TEST_2',0x49:'PABE',0x4a:'FBA_1',0x4b:'FBA_2',0x4c:'FRAME_1',0x4d:'FRAME_2',0x4e:'ZBUF_1',0x4f:'ZBUF_2',
 0x50:'BITBLTBUF',0x51:'TRXPOS',0x52:'TRXREG',0x53:'TRXDIR',0x54:'HWREG'}
def load(path):
    d=open(path,'rb').read()
    hs=struct.unpack_from('<I',d,4)[0]
    state_size=struct.unpack_from('<I',d,12)[0]
    p=8+hs+state_size+8192
    pk=[]; i=p
    while i<len(d):
        t=d[i]
        if t==0:
            path=d[i+1]; sz=struct.unpack_from('<I',d,i+2)[0]; pk.append(('T',path,d[i+6:i+6+sz])); i+=6+sz
        elif t==1: pk.append(('V',d[i+1],b'')); i+=2
        elif t==2: sz=struct.unpack_from('<I',d,i+1)[0]; pk.append(('R',0,sz)); i+=5
        elif t==3: pk.append(('G',0,d[i+1:i+8193])); i+=8193
        else: raise Exception('bad %d at %d'%(t,i))
    return d,pk
class GS:
    def __init__(s):
        s.r={}; s.rgba=(0,0,0,0,0.0); s.st=(0.0,0.0,1.0); s.uv=(0,0); s.prim=0; s.q=1.0
        s.verts=[]; s.draws=[]; s.cur=None; s.kick=0; s.fog=0; s.uploads=[]
    def setreg(s,a,v):
        if a==0x00: s.prim=v&0x7ff; s.verts=[]
        elif a==0x01: s.rgba=(v&255,(v>>8)&255,(v>>16)&255,(v>>24)&255,s.q)
        elif a==0x02:
            f=struct.unpack('<ff',struct.pack('<Q',v)); s.st=(f[0],f[1])
        elif a==0x03: s.uv=(v&0x3fff,(v>>16)&0x3fff)
        elif a in (0x04,0x05,0x0c,0x0d):
            x=v&0xffff; y=(v>>16)&0xffff
            if a in (4,0xc): z=(v>>32)&0xffffff; f=(v>>56)
            else: z=(v>>32)&0xffffffff; f=0
            adc=a in (0x0c,0x0d)
            s.vertex(x,y,z,adc)
        elif a==0x0a: s.fog=(v>>56)
        else: s.r[a]=v
    def vertex(s,x,y,z,adc):
        s.verts.append(dict(xy=(x/16.0,y/16.0),z=z,rgba=s.rgba,st=s.st,q=s.q,uv=s.uv,fog=s.fog))
        if adc: return
        pt=s.prim&7; need={0:1,1:2,2:2,3:3,4:3,5:3,6:2}.get(pt,3)
        n=len(s.verts)
        ok = (n>=need) if pt in (2,4,5) else (n%need==0 if pt!=6 else n%2==0)
        if pt in (3,6,0,1) and n>=need and n%need==0: ok=True
        if ok:
            s.emit_prim(pt)
    def emit_prim(s,pt):
        key=s.statekey()
        if s.cur is None or s.cur['key']!=key or s.cur['prim']!=s.prim:
            s.cur=dict(key=key,prim=s.prim,state={k:s.r.get(k) for k in s.statekeys()},npr=0,verts=[],kick0=s.kick)
            s.draws.append(s.cur)
        s.cur['npr']+=1
        pt3=pt
        need={0:1,1:2,2:2,3:3,4:3,5:3,6:2}[pt]
        s.cur['verts'].append(list(s.verts[-need:]))
        if pt in (3,0,1,6): s.verts=[]
        s.kick+=1
    @staticmethod
    def statekeys(): return (0x06,0x07,0x14,0x15,0x42,0x43,0x47,0x48,0x4c,0x4d,0x4e,0x4f,0x08,0x09,0x34,0x35,0x36,0x37,0x1b,0x3b,0x1c,0x46,0x49,0x45,0x3d)
    def statekey(s): return tuple(s.r.get(k) for k in s.statekeys())
def run(path):
    d,pk=load(path)
    g=GS()
    for kind,path_,data in pk:
        if kind!='T': continue
        i=0
        while i+16<=len(data):
            lo,hi=struct.unpack_from('<QQ',data,i); i+=16
            nloop=lo&0x7fff; eop=(lo>>15)&1; pre=(lo>>46)&1; prim=(lo>>47)&0x7ff; flg=(lo>>58)&3; nreg=(lo>>60)&15 or 16
            regs=[(hi>>(4*k))&15 for k in range(nreg)]
            if pre and flg==0: g.setreg(0,prim)
            if flg==0:
                for _ in range(nloop):
                    for rg in regs:
                        a,b=struct.unpack_from('<QQ',data,i); i+=16
                        if rg==0: g.setreg(0,a&0x7ff)
                        elif rg==1: g.rgba=(a&255,(a>>32)&255,b&255,(b>>32)&255,0)  ; g.rgba=(a&0xff,(a>>32)&0xff,b&0xff,(b>>32)&0xff,g.q)
                        elif rg==2:
                            s_,t_=struct.unpack('<ff',struct.pack('<Q',a)); g.q=struct.unpack('<f',struct.pack('<I',b&0xffffffff))[0]; g.st=(s_,t_)
                        elif rg==3: g.uv=(a&0x3fff,(a>>32)&0x3fff)
                        elif rg in (4,5,0xc,0xd):
                            x=a&0xffff; y=(a>>32)&0xffff; z=(b&0xffffffff) if rg in (5,0xd) else ((b>>4)&0xffffff)
                            if rg in (4,0xc): g.fog=(b>>36)&255
                            adc=(b>>111-64)&1 if False else ((b>>47)&1)
                            if rg in (0xc,0xd): adc=1
                            g.vertex(x,y,z,bool((b>>47)&1) or rg in(0xc,0xd))
                        elif rg==6: g.r[0x06]=a
                        elif rg==7: g.r[0x07]=a
                        elif rg==8: g.r[0x08]=a
                        elif rg==9: g.r[0x09]=a
                        elif rg==0xa: g.fog=(b>>36)&255
                        elif rg==0xe: g.setreg(b&0xff,a)
                        elif rg==0xf: pass
            elif flg==1:
                for _ in range(nloop):
                    for rg in regs:
                        a=struct.unpack_from('<Q',data,i)[0]; i+=8
                        g.setreg(rg,a)
                i=(i+15)&~15
            else:
                if flg==2:
                    g.uploads.append(dict(bitblt=g.r.get(0x50),trxpos=g.r.get(0x51),trxreg=g.r.get(0x52),data=data[i:i+nloop*16],kick=g.kick))
                i+=nloop*16
    return g

def describe(g, i, d, verbose=False):
    s = d['state']; t = s[0x06] or 0
    line = ('%4d  prim %#05x  prims %4d  tex %s  alpha %s  test %s' % (
        i, d['prim'], d['npr'],
        ('tbp %#06x psm %#04x %dx%d' % (t & 0x3fff, (t >> 20) & 63, 1 << ((t >> 26) & 15), 1 << ((t >> 30) & 15))) if t else '-',
        ('%#x' % s[0x42]) if s[0x42] is not None else '-',
        ('%#x' % s[0x47]) if s[0x47] is not None else '-'))
    print(line)
    if verbose:
        for k, v in d['state'].items():
            if v is not None:
                print('      %-10s %016x' % (REGNAME.get(k, hex(k)), v))
        seq = [v for v in d['verts'][0]] + [tri[-1] for tri in d['verts'][1:]]
        for n, v in enumerate(seq[:40]):
            q = v['q'] or 1.0
            print('      v%-3d xy (%.2f, %.2f) z %d rgba %s u %.4f v %.4f fog %d' % (
                n, v['xy'][0], v['xy'][1], v['z'], v['rgba'][:4], v['st'][0] / q, v['st'][1] / q, v['fog']))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[1])
    ap.add_argument('dump')
    ap.add_argument('--draw', type=int, help='print this state group in full')
    args = ap.parse_args()
    g = run(args.dump)
    if args.draw is not None:
        describe(g, args.draw, g.draws[args.draw], verbose=True)
    else:
        print(len(g.draws), 'state groups,', g.kick, 'primitives,', len(g.uploads), 'image uploads')
        for i, d in enumerate(g.draws):
            describe(g, i, d)


if __name__ == '__main__':
    main()
