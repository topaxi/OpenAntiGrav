import subprocess, sys, numpy as np
d=sys.argv[1]; w,h=1000,570
cmd=["ffmpeg","-loglevel","error","-i",d+"/film.mkv","-vf","crop=1882:1058:118:7,crop=%d:%d:700:190"%(w,h),"-f","rawvideo","-pix_fmt","rgb24","-"]
p=subprocess.Popen(cmd,stdout=subprocess.PIPE)
prev=None;n=0;same=0;rows=[]
while True:
    b=p.stdout.read(w*h*3)
    if len(b)<w*h*3:break
    a=np.frombuffer(b,np.uint8).reshape(h,w,3)
    if prev is not None and np.array_equal(a,prev): same+=1
    prev=a.copy()
    s=a[:500,20:980].astype(int); m=s.min(axis=2)>60
    ys,xs=np.nonzero(m)
    rows.append((n,len(xs), int(np.percentile(xs,1)) if len(xs)>500 else -1, int(np.percentile(xs,99)) if len(xs)>500 else -1))
    n+=1
print("frames",n,"identical-to-previous",same)
import json;json.dump(rows,open(d+"/series.json","w"))
