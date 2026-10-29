#!/usr/bin/env python3
"""Decode the displayed PSMCT32 framebuffer out of a PCSX2 GS dump's VRAM."""
import struct, sys
import numpy as np
from PIL import Image
BLK=[[0,1,4,5,16,17,20,21],[2,3,6,7,18,19,22,23],[8,9,12,13,24,25,28,29],[10,11,14,15,26,27,30,31]]
COL=[[0,1,4,5,8,9,12,13],[2,3,6,7,10,11,14,15],[16,17,20,21,24,25,28,29],[18,19,22,23,26,27,30,31],
     [32,33,36,37,40,41,44,45],[34,35,38,39,42,43,46,47],[48,49,52,53,56,57,60,61],[50,51,54,55,58,59,62,63]]
def vram(path, off):
    d=open(path,'rb').read()
    hs=struct.unpack_from('<I',d,4)[0]
    base=8+hs
    return np.frombuffer(d,dtype='<u4',count=1048576,offset=base+off)
def decode(v,fbp,fbw,w,h):
    # build index map
    ys,xs=np.mgrid[0:h,0:w]
    page=fbp+(ys//32)*fbw+(xs//64)
    blk=np.array(BLK)[(ys%32)//8,(xs%64)//8]
    col=np.array(COL)[ys%8,xs%8]
    addr=page*2048+blk*64+col
    return v[addr%1048576]
if __name__=='__main__':
    path,off,out=sys.argv[1],int(sys.argv[2]),sys.argv[3]
    v=vram(path,off)
    px=decode(v,0,10,640,448)
    rgba=np.stack([px&255,(px>>8)&255,(px>>16)&255,np.full(px.shape,255)],-1).astype(np.uint8)
    Image.fromarray(rgba).save(out)
