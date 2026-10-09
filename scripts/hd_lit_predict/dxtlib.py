import numpy as np
def _rgb565(c):
    return np.stack([((c>>11)&31)*255/31,((c>>5)&63)*255/63,(c&31)*255/31],-1)
def decode_dxt5(data,w,h):
    """-> (h,w,4) float 0..1, rgba; blocks little-endian, row-major."""
    nb=(w//4)*(h//4)
    raw=np.frombuffer(data[:nb*16],dtype=np.uint8).reshape(nb,16)
    a0=raw[:,0].astype(float); a1=raw[:,1].astype(float)
    abits=np.zeros(nb,dtype=np.uint64)
    for i in range(6): abits|=raw[:,2+i].astype(np.uint64)<<np.uint64(8*i)
    apal=np.zeros((nb,8))
    apal[:,0]=a0; apal[:,1]=a1
    g=(a0>a1)
    for k in range(1,7):
        apal[:,1+k]=np.where(g,((7-k)*a0+k*a1)/7,np.where(k<=4,((5-k)*a0+k*a1)/5,0))
    apal[:,7]=np.where(g,apal[:,7],255)
    c0=raw[:,8].astype(np.uint16)|(raw[:,9].astype(np.uint16)<<8)
    c1=raw[:,10].astype(np.uint16)|(raw[:,11].astype(np.uint16)<<8)
    cb=raw[:,12].astype(np.uint32)|(raw[:,13].astype(np.uint32)<<8)|(raw[:,14].astype(np.uint32)<<16)|(raw[:,15].astype(np.uint32)<<24)
    p0=_rgb565(c0); p1=_rgb565(c1)
    pal=np.zeros((nb,4,3)); pal[:,0]=p0; pal[:,1]=p1
    gt=(c0>c1)[:,None]
    pal[:,2]=np.where(gt,(2*p0+p1)/3,(p0+p1)/2)
    pal[:,3]=np.where(gt,(p0+2*p1)/3,0)
    out=np.zeros((h//4,w//4,4,4,4))
    ar=np.arange(nb)
    for k in range(16):
        ci=((cb>>(2*k))&3).astype(int)
        ai=((abits>>np.uint64(3*k))&np.uint64(7)).astype(int)
        px=np.concatenate([pal[ar,ci],apal[ar,ai][:,None]],1).reshape(h//4,w//4,4)
        out[:,:,k//4,k%4]=px
    return out.transpose(0,2,1,3,4).reshape(h,w,4)/255
def decode_dxt1(data,w,h):
    nb=(w//4)*(h//4)
    raw=np.frombuffer(data[:nb*8],dtype=np.uint8).reshape(nb,8)
    c0=raw[:,0].astype(np.uint16)|(raw[:,1].astype(np.uint16)<<8)
    c1=raw[:,2].astype(np.uint16)|(raw[:,3].astype(np.uint16)<<8)
    cb=raw[:,4].astype(np.uint32)|(raw[:,5].astype(np.uint32)<<8)|(raw[:,6].astype(np.uint32)<<16)|(raw[:,7].astype(np.uint32)<<24)
    p0=_rgb565(c0); p1=_rgb565(c1)
    pal=np.zeros((nb,4,3)); pal[:,0]=p0; pal[:,1]=p1
    gt=(c0>c1)[:,None]
    pal[:,2]=np.where(gt,(2*p0+p1)/3,(p0+p1)/2)
    pal[:,3]=np.where(gt,(p0+2*p1)/3,0)
    out=np.zeros((h//4,w//4,4,4,3)); ar=np.arange(nb)
    for k in range(16):
        ci=((cb>>(2*k))&3).astype(int)
        out[:,:,k//4,k%4]=pal[ar,ci].reshape(h//4,w//4,3)
    return out.transpose(0,2,1,3,4).reshape(h,w,3)/255

def decode_dxt3(data,w,h):
    nb=(w//4)*(h//4)
    raw=np.frombuffer(data[:nb*16],dtype=np.uint8).reshape(nb,16)
    col=decode_dxt1_blocks(raw[:,8:],w,h)
    out=np.zeros((h//4,w//4,4,4))
    for k in range(16):
        byte=raw[:,k//2]; nib=(byte>>(4*(k%2)))&15
        out[:,:,k//4,k%4]=(nib/15.0).reshape(h//4,w//4)
    a=out.transpose(0,2,1,3).reshape(h,w)
    return np.concatenate([col,a[...,None]],-1)
def decode_dxt1_blocks(raw,w,h):
    nb=raw.shape[0]
    c0=raw[:,0].astype(np.uint16)|(raw[:,1].astype(np.uint16)<<8)
    c1=raw[:,2].astype(np.uint16)|(raw[:,3].astype(np.uint16)<<8)
    cb=raw[:,4].astype(np.uint32)|(raw[:,5].astype(np.uint32)<<8)|(raw[:,6].astype(np.uint32)<<16)|(raw[:,7].astype(np.uint32)<<24)
    p0=_rgb565(c0); p1=_rgb565(c1)
    pal=np.zeros((nb,4,3)); pal[:,0]=p0; pal[:,1]=p1
    gt=(c0>c1)[:,None]
    pal[:,2]=np.where(gt,(2*p0+p1)/3,(p0+p1)/2)
    pal[:,3]=np.where(gt,(p0+2*p1)/3,0)
    out=np.zeros((h//4,w//4,4,4,3)); ar=np.arange(nb)
    for k in range(16):
        ci=((cb>>(2*k))&3).astype(int)
        out[:,:,k//4,k%4]=pal[ar,ci].reshape(h//4,w//4,3)
    return out.transpose(0,2,1,3,4).reshape(h,w,3)/255
