import sys,re
def strip(src):
    out=[];i=0;n=len(src)
    while i<n:
        c=src[i]
        if src.startswith('//',i):
            j=src.find('\n',i); i=n if j<0 else j; continue
        if src.startswith('/*',i):
            d=1;i+=2
            while i<n and d:
                if src.startswith('/*',i): d+=1;i+=2
                elif src.startswith('*/',i): d-=1;i+=2
                else: i+=1
            continue
        m=re.match(r'b?r(#*)"',src[i:])
        if m:
            end='"'+m.group(1); j=src.find(end,i+len(m.group(0)))
            j=n if j<0 else j+len(end); out.append(src[i:j]); i=j; continue
        if c=='"' or (c=='b' and i+1<n and src[i+1]=='"'):
            j=i+(2 if c=='b' else 1)
            while j<n and src[j]!='"':
                j+= 2 if src[j]=='\\' else 1
            out.append(src[i:j+1]); i=j+1; continue
        if c=="'":
            m=re.match(r"'(\\.[^']*|[^\\'])'",src[i:])
            if m: out.append(m.group(0)); i+=len(m.group(0)); continue
        out.append(c); i+=1
    return '\n'.join(l.strip() for l in ''.join(out).splitlines() if l.strip())
a,b=open(sys.argv[1]).read(),open(sys.argv[2]).read()
sys.exit(0 if strip(a)==strip(b) else 1)
