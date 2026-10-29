#!/usr/bin/env python3
"""patch_display.py SRC DST [--dy N] [--dby N] [--dx N]: rewrite DISPLAY1/2 DY/DX and DISPFB1/2 DBY in a GS dump."""
import struct, sys, argparse
ap=argparse.ArgumentParser(); ap.add_argument('src'); ap.add_argument('dst')
ap.add_argument('--dy',type=int); ap.add_argument('--dby',type=int); ap.add_argument('--dx',type=int)
a=ap.parse_args()
d=bytearray(open(a.src,'rb').read())
hs=struct.unpack_from('<I',d,4)[0]; ss=struct.unpack_from('<I',d,12)[0]
def patch(off):
    n=0
    for o in (0x80,0xa0):
        v=struct.unpack_from('<Q',d,off+o)[0]
        if a.dy is not None: v=(v&~(0x7ff<<12))|(a.dy<<12)
        if a.dx is not None: v=(v&~0xfff)|a.dx
        struct.pack_into('<Q',d,off+o,v)
    for o in (0x70,0x90):
        v=struct.unpack_from('<Q',d,off+o)[0]
        if a.dby is not None: v=(v&~(0x7ff<<43))|(a.dby<<43)
        struct.pack_into('<Q',d,off+o,v)
p=8+hs+ss
patch(p)
i=p+8192; cnt=0
while i<len(d):
    t=d[i]
    if t==0: i+=6+struct.unpack_from('<I',d,i+2)[0]
    elif t==1: i+=2
    elif t==2: i+=5+0
    elif t==3: patch(i+1); i+=8193; cnt+=1
    else: raise SystemExit('bad')
print('patched privileged + %d register packets'%cnt)
open(a.dst,'wb').write(d)
