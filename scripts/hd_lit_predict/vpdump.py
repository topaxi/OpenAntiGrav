import sys,glob,importlib.util,struct
import os
SC=os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
def load(n,f):
    s=importlib.util.spec_from_file_location(n,SC+'/'+f); m=importlib.util.module_from_spec(s); sys.modules[n]=m; s.loader.exec_module(m); return m
mc=load('ps3_microcode','ps3-microcode.py'); fifo=load('rsx_fifo','rsx_fifo.py')
def vp_per_draw(out,stem):
    memory=fifo.Mem.load(sorted(glob.glob('%s/%s-4*.bin'%(out,stem))))
    program,ld,start,n={}, 0,0,0; res={}
    for pos,meth,ni,args in fifo.Walk(memory).run(0x1000):
        if (memory.word(pos)>>13)&7: continue
        if meth==0x1E9C: ld=args[0]
        elif meth==0xB80:
            for i in range(0,len(args)-3,4):
                program[ld]=tuple(args[i:i+4]); ld+=1
        elif meth==0x1EA0: start=args[0]&0xfff
        elif meth in (0x1814,0x1824):
            words=[];slot=start
            while slot in program and len(words)<2048:
                words.extend(program[slot]); slot+=1
                if program[slot-1][3]&1: break
            res[n]=(start,tuple(words)); n+=1
    return res
def text(words):
    o=[]
    for i in range(0,len(words)-3,4):
        o.append(mc.vp_render(*words[i:i+4]))
    return o
if __name__=='__main__':
    out,stem=sys.argv[1:3]
    r=vp_per_draw(out,stem)
    for n in map(int,sys.argv[3:]):
        print('draw',n); print('\n'.join(text(r[n][1])))
