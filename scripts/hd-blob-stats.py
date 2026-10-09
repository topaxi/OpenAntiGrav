#!/usr/bin/env python3
"""Lamp-blob statistics for a ceiling band: connected components of min-channel >= 250 (size, count, mean colour).

    uv run --with scipy --with numpy --with pillow python3 scripts/hd-blob-stats.py original.png ours.png ...
"""
import sys, numpy as np
from PIL import Image
from scipy import ndimage
# blob stats over the lamp zone: connected components of max-channel >= T
def stats(p, box, T=250):
    a=np.asarray(Image.open(p).convert("RGB")).astype(float)
    x0,y0,x1,y1=box; sub=a[y0:y1,x0:x1]; L=sub.min(axis=2)
    lab,n=ndimage.label(L>=T)
    sizes=ndimage.sum(np.ones_like(L),lab,range(1,n+1)) if n else []
    big=[s for s in sizes if s>=20]
    return dict(clipped=(L>=T).mean()*100, n=len(big), med=float(np.median(big)) if big else 0, mx=max(sizes) if n else 0, total=float(sum(big)),
                mean=sub.mean(axis=(0,1)).round(1).tolist())
box=(0,0,1882,420)  # ceiling band above the grid horizon
for p in sys.argv[1:]:
    print(p, stats(p,box))
