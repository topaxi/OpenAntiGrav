#!/usr/bin/env python3
"""Edge-free box band energy, ours against the original: texture-sharpness.py ours.png theirs.png.

Boxes are 120x72 tiles with no hard edge in either image, outside the craft and the HUD rows. Per box the std of two
difference-of-Gaussian bands (fine 0.7-1.4 px, mid 1.4-2.8 px); the median ratio ours/original is printed for the
lower (near) and upper (far) half. Laplacian variance is NOT used: it counts polygon edges (docs/rendering/hd-track-tone.md,
"Texture crispness").
"""
import sys, numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter, sobel
def grey(p):
    a=np.asarray(Image.open(p).convert("RGB"),dtype=np.float64); return a@[.299,.587,.114]
def bands(a):
    g07,g14,g28=gaussian_filter(a,.7),gaussian_filter(a,1.4),gaussian_filter(a,2.8)
    return g07-g14, g14-g28
def edgemax(a):
    return np.hypot(sobel(a,0),sobel(a,1))
def boxes(go,gt,W=120,H=72):
    h,w=go.shape; eo,et=edgemax(go),edgemax(gt); out=[]
    for y in range(int(h*.15),int(h*.82)-H,H):
        for x in range(0,w-W,W):
            if x>w*.55 and y>h*.5: continue  # craft
            sl=(slice(y+4,y+H-4),slice(x+4,x+W-4))
            if max(np.quantile(eo[sl],.995),np.quantile(et[sl],.995))>160: continue
            if gt[sl].std()<5: continue
            out.append((x,y,W,H))
    return out
def measure(go,gt,bx):
    bo,bt=bands(go),bands(gt); rows=[]
    for x,y,W,H in bx:
        sl=(slice(y+6,y+H-6),slice(x+6,x+W-6)); r=[x,y]
        for k in (0,1): r+= [bt[k][sl].std(), bo[k][sl].std()]
        r+=[gt[sl].std(),go[sl].std()]
        rows.append(r)
    return np.array(rows)
def summary(ours,theirs):
    go,gt=grey(ours),grey(theirs); bx=boxes(go,gt); m=measure(go,gt,bx)
    if len(m)==0: return None
    near=m[:,1]>=go.shape[0]*.5
    def rat(sel,k): 
        s=m[sel]; return np.median(s[:,2*k+3]/s[:,2*k+2]) if len(s) else float('nan')
    return len(m),near.sum(),rat(near,0),rat(~near,0),rat(near,1),rat(~near,1),np.median(m[:,7]/m[:,6])
if __name__=="__main__":
    s=summary(sys.argv[1],sys.argv[2])
    print("n=%d (near %d) fine near %.2f far %.2f | mid near %.2f far %.2f | broadband std ratio %.2f"%s)
