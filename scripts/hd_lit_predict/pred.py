"""Predict the lightmapped draws of a captured frame from live state and compare with the scene target.

    cap = Cap(dir); did, attr = raster(cap, draw_ids); mask, rgb, _ = shade_road(cap, did, attr, ids)

The two `shade_*` functions evaluate the two program families read so far, with the output scale."""
import sys,os,struct,hashlib,glob
import numpy as np
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
import hook as h
from dxtlib import decode_dxt5,decode_dxt1,decode_dxt3
W,H=1280,720
def f16(b):  # big-endian half array
    return np.frombuffer(b,dtype='>f2').astype(np.float32)
def srgb_dec(x):
    x=np.asarray(x,dtype=np.float32)
    return np.where(x<=0.04045,x/12.92,((x+0.055)/1.055)**2.4)
def srgb_enc(x):
    x=np.clip(x,0,1)
    return np.where(x<=0.0031308,x*12.92,1.055*x**(1/2.4)-0.055)
class Cap:
    def __init__(self,out):
        self.out=out
        self.m,self.wk,self.ds=h.draws_of(out,'00')
        self.scene=np.fromfile('%s/00-40cc0000.bin'%out,dtype=np.uint8).reshape(H,W,4)[:,:,1:4]/255.0
        self.tex={}
    def file(self,addr):
        return open('%s/00-%08x.bin'%(self.out,addr),'rb').read()
    def texture(self,off,fmt,rect):
        key=(off,fmt)
        if key in self.tex: return self.tex[key]
        w=rect>>16; hh=rect&0xffff; f=(fmt>>8)&0x9f
        loc=fmt&3; addr=(0xC0000000 if loc==1 else 0x40000000)+off
        d=self.file(addr)
        if f==0x88: im=decode_dxt5(d,w,hh)
        elif f==0x87: im=decode_dxt3(d,w,hh)
        elif f==0x86: im=np.concatenate([decode_dxt1(d,w,hh),np.ones((hh,w,1))],-1)
        else: raise Exception('fmt %x'%f)
        self.tex[key]=im.astype(np.float32); return self.tex[key]
    def verts(self,d):
        """-> dict of arrays for the draw's referenced vertices, indexed by raw index"""
        ixoff=d[0x181c]
        idx=[]
        for first,c in d['batches']:
            b=self.m.bytes(ixoff+first*2,c*2)
            idx.append(np.frombuffer(b,dtype='>u2'))
        idx=np.concatenate(idx).astype(np.int64)
        # attrs
        def arr(a):
            off=d[0x1680+4*a]; fmt=d[0x1740+4*a]
            stride=(fmt>>8)&0xff; typ=fmt&0xf; size=(fmt>>4)&0xf
            return (off&0x7fffffff),stride,typ,size
        o0,s0,t0,n0=arr(0)
        u=np.unique(idx)
        lo,hi=u.min(),u.max()
        blob=self.m.bytes(o0+lo*s0,(hi-lo+1)*s0+0)  # positions block; attrs are interleaved in the same stride
        base=min(arr(a)[0] for a in range(4))
        blob=self.m.bytes(base+lo*s0,(hi-lo+1)*s0)
        if blob is None: raise Exception('vertex span missing')
        buf=np.frombuffer(blob,dtype=np.uint8).reshape(-1,s0)
        def col(a,nbytes):
            o,s,t,n=arr(a); rel=o-base
            return buf[:,rel:rel+nbytes]
        pos=col(0,6).copy().view('>i2').reshape(-1,3).astype(np.float32)
        nrm=col(1,4).copy().view('>u4').reshape(-1).astype(np.uint32)
        uv=col(2,4).copy().view('>f2').reshape(-1,2).astype(np.float32)
        lm=col(3,4).copy().view('>f2').reshape(-1,2).astype(np.float32)
        nx=(nrm&0x7ff).astype(np.int32); ny=((nrm>>11)&0x7ff).astype(np.int32); nz=((nrm>>22)&0x3ff).astype(np.int32)
        def sx(v,bits): return np.where(v>=(1<<(bits-1)),v-(1<<bits),v)/float((1<<(bits-1))-1)
        n=np.stack([sx(nx,11),sx(ny,11),sx(nz,10)],-1).astype(np.float32)
        col4=np.zeros((len(pos),4),dtype=np.float32)
        o4,s4,t4,n4=arr(4)
        if n4>0 and s4:
            b4=self.m.bytes(o4+lo*s4,(hi-lo+1)*s4)
            if b4 is not None:
                col4=np.frombuffer(b4,dtype=np.uint8).reshape(-1,s4)[:,:4].astype(np.float32)/255.0
        return idx,lo,dict(pos=pos,nrm=n,uv=uv,lm=lm,col=col4)
    def cvec(self,d,k):
        v=d['c'][256+k]; return np.array(struct.unpack('>4f',struct.pack('>4I',*v)),dtype=np.float32)

def fetch(im,u,v,wrap=True):
    """bilinear sample, texel centres, repeat; im (h,w,c); u,v arrays in 0..1"""
    hh,ww=im.shape[:2]
    x=u*ww-0.5; y=v*hh-0.5
    x0=np.floor(x).astype(np.int64); y0=np.floor(y).astype(np.int64)
    fx=(x-x0)[...,None]; fy=(y-y0)[...,None]
    def g(xx,yy): return im[yy%hh,xx%ww]
    return (g(x0,y0)*(1-fx)*(1-fy)+g(x0+1,y0)*fx*(1-fy)+g(x0,y0+1)*(1-fx)*fy+g(x0+1,y0+1)*fx*fy)

def raster(cap,draw_ids,yflip=True):
    """z-buffered rasterisation of given draws; returns per-pixel (draw id, interpolated attributes)."""
    zbuf=np.full((H,W),np.inf,dtype=np.float64)
    did=np.full((H,W),-1,dtype=np.int32)
    attr=np.zeros((H,W,15),dtype=np.float32)  # uv2 lm2 nrm3 pos3 clipw
    for n in draw_ids:
        d=cap.ds[n]
        try: idx,lo,v=cap.verts(d)
        except Exception as e:
            print('skip draw',n,e); continue
        M=[cap.cvec(d,k) for k in range(4)]
        scale=cap.cvec(d,210)[:3]; bias=cap.cvec(d,211)[:3]
        pos=v['pos']*scale+bias
        clip=pos[:,0:1]*M[0]+pos[:,1:2]*M[1]+pos[:,2:3]*M[2]+M[3]
        cw=clip[:,3]
        ok=cw>1e-6
        ndc=clip[:,:3]/np.where(ok,cw,1)[:,None]
        sx=(ndc[:,0]*0.5+0.5)*W
        sy=((0.5-ndc[:,1]*0.5) if yflip else (ndc[:,1]*0.5+0.5))*H
        sz=ndc[:,2]
        # per-vertex attribute rows
        A=np.concatenate([v['uv'],v['lm'],v['nrm'],pos,cw[:,None],v['col']],1)
        mode=d.get(0x1808)
        if mode==6:
            tri=np.stack([idx[:-2],idx[1:-1],idx[2:]],1)-lo
            tri=tri[(tri[:,0]!=tri[:,1])&(tri[:,1]!=tri[:,2])&(tri[:,0]!=tri[:,2])]
        elif mode==5:
            nn=len(idx)//3*3; tri=idx[:nn].reshape(-1,3)-lo
        else:
            print('skip mode',mode); continue
        for t in tri:
            if not (ok[t].all()): continue
            x=sx[t]; y=sy[t]; z=sz[t]; w=cw[t]
            x0=max(int(np.floor(x.min())),0); x1=min(int(np.ceil(x.max())),W-1)
            y0=max(int(np.floor(y.min())),0); y1=min(int(np.ceil(y.max())),H-1)
            if x1<x0 or y1<y0: continue
            det=(x[1]-x[0])*(y[2]-y[0])-(x[2]-x[0])*(y[1]-y[0])
            if abs(det)<1e-9: continue
            xs=np.arange(x0,x1+1)+0.5; ys=np.arange(y0,y1+1)+0.5
            X,Y=np.meshgrid(xs,ys)
            l1=((X-x[0])*(y[2]-y[0])-(x[2]-x[0])*(Y-y[0]))/det
            l2=((x[1]-x[0])*(Y-y[0])-(X-x[0])*(y[1]-y[0]))/det
            l0=1-l1-l2
            inside=(l0>=0)&(l1>=0)&(l2>=0)
            if not inside.any(): continue
            Z=l0*z[0]+l1*z[1]+l2*z[2]
            sub=zbuf[y0:y1+1,x0:x1+1]
            win=inside&(Z<sub)&(Z>-1)&(Z<1)
            if not win.any(): continue
            pw0=l0/w[0]; pw1=l1/w[1]; pw2=l2/w[2]; s=pw0+pw1+pw2
            a=(pw0[...,None]*A[t[0]]+pw1[...,None]*A[t[1]]+pw2[...,None]*A[t[2]])/s[...,None]
            sub[win]=Z[win]
            did[y0:y1+1,x0:x1+1][win]=n
            attr[y0:y1+1,x0:x1+1][win]=a[win]
    return did,attr

SUN_DIR=np.array([-0.7776,0.58152,-0.23911],dtype=np.float32)
SUN_COL=np.array([2,1.82745,0.886275],dtype=np.float32)
FOG_COL=np.array([0.262745,0.215686,0.380392],dtype=np.float32)

import vpdump as vpd
def shade_d26301(cap,did,attr,ids,lm_scale=4.0,lm_pow=2.0,spec_pow=1.0,lm_srgb=False,parts=False):
    """Evaluate the road program (family d26301) for pixels owned by `ids`. Returns (mask, rgb linear, parts)."""
    vpmap=vpd.vp_per_draw(cap.out,'00')
    mask=np.isin(did,ids)
    out=np.zeros((H,W,3),dtype=np.float32); P={}
    for n in ids:
        sel=did==n
        if not sel.any(): continue
        d=cap.ds[n]; a=attr[sel]
        u0=cap.texture(d[0x1a00],d[0x1a04],d[0x1a18]); u1=cap.texture(d[0x1a20],d[0x1a24],d[0x1a38])
        alb=fetch(u0,a[:,0],a[:,1]); lm=fetch(u1,a[:,2],a[:,3])
        alb[:,:3]=srgb_dec(alb[:,:3])
        if lm_srgb: lm[:,:3]=srgb_dec(lm[:,:3])
        nrm=a[:,4:7]; pos=a[:,7:10]; w=a[:,10]
        eye=cap.cvec(d,209)[:3]
        c208=cap.cvec(d,208)
        vptxt='\n'.join(vpd.text(vpmap[n][1]))
        if 'v[4].wwww' in vptxt:
            vl=a[:,11:14]*np.exp2(a[:,14]*c208[0]-c208[1])[:,None]; amb=c208[2]
        else:
            vl=np.zeros((len(a),3),dtype=np.float32)+c208[0]; amb=c208[0]
        nl=np.linalg.norm(nrm,axis=1,keepdims=True); nn=nrm/np.maximum(nl,1e-9)
        view=eye-pos; vn=view/np.maximum(np.linalg.norm(view,axis=1,keepdims=True),1e-9)
        hv=vn+SUN_DIR; hl=np.linalg.norm(hv,axis=1)
        ndh=np.clip((hv*nn).sum(1)/np.maximum(hl,1e-9),0,1)
        ndl=(nn*SUN_DIR).sum(1)
        prelit=lm_scale*np.power(np.maximum(lm[:,:3],0),lm_pow)
        sun=(lm[:,3]*ndl)[:,None]*SUN_COL
        spec_f=np.power(ndh,spec_pow)*np.clip(ndl,0,1)*8*lm[:,3]
        diff=alb[:,:3]*(vl+prelit+sun)
        spec=(spec_f*1.5)[:,None]*SUN_COL*alb[:,3:4]
        fogf=np.clip(np.exp2(-(0.001*w)**2*1.442695),0,1)[:,None]
        col=0.5*(fogf*(diff+spec)+FOG_COL*(1-fogf)+amb)
        out[sel]=col
        for k,v in (('prelit',prelit),('sun',sun),('alb',alb[:,:3]),('diff',diff),('spec',spec),('fog',fogf),('lm',lm[:,:3]),('lma',lm[:,3:4])):
            P.setdefault(k,[]).append(v)
        P.setdefault('vl',[]).append(vl)
    if parts: P={k:np.concatenate(v) for k,v in P.items()}
    return mask,out,P

def shade_road(cap,did,attr,ids,lm_scale=4.0,lm_pow=2.0,lm_srgb=False,use_vl=True,parts=False):
    out=np.zeros((H,W,3),dtype=np.float32); P={}
    mask=np.isin(did,ids)
    for n in ids:
        sel=did==n
        if not sel.any(): continue
        d=cap.ds[n]; a=attr[sel]
        u0=cap.texture(d[0x1a00],d[0x1a04],d[0x1a18]); u1=cap.texture(d[0x1a20],d[0x1a24],d[0x1a38])
        alb=fetch(u0,a[:,0],a[:,1])[:,:3]; lm=fetch(u1,a[:,2],a[:,3])[:,:3]
        alb=srgb_dec(alb)
        if lm_srgb: lm=srgb_dec(lm)
        c208=cap.cvec(d,208)
        vl=a[:,11:14]*np.exp2(a[:,14]*c208[0]-c208[1])[:,None]
        if not use_vl: vl=vl*0
        prelit=lm_scale*np.power(np.maximum(lm,0),lm_pow)
        w=a[:,10]
        fogf=np.clip(np.exp2(-(0.001*w)**2*1.442695),0,1)[:,None]
        col=0.5*(fogf*(alb*(prelit+vl))+FOG_COL*(1-fogf))  # MAD R2=H2*H1-fog ; H0=R0.x*R2+fog
        out[sel]=col
        for k,v in (('prelit',prelit),('vl',vl),('alb',alb),('lm',lm),('fog',fogf)): P.setdefault(k,[]).append(v)
    if parts: P={k:np.concatenate(v) for k,v in P.items()}
    return mask,out,P
