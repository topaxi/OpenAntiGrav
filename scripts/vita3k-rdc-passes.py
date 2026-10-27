"""List the render passes of one RenderDoc capture of Vita3K and save every texture they touch.

Run it inside qrenderdoc (the standalone python module is not packaged):

    OAG_RDC=frame.rdc OAG_RDC_OUT=outdir DISPLAY=:98 env -u WAYLAND_DISPLAY \
        qrenderdoc --ui-python scripts/vita3k-rdc-passes.py

It writes `outdir/draws.txt` (one block per draw: index count, colour
targets with size and format, blend state, bound textures with format, mips
and the sampler's LOD bias) and `outdir/tex_<id>.png` (mip 0 of every output
and input; a float target is clamped to 0..1). It exits qrenderdoc when done.
See docs/reverse-engineering/vita3k-capture.md, "RenderDoc: the passes of one
frame".
"""
import renderdoc as rd, os
OUT=os.environ['OAG_RDC_OUT']
os.makedirs(OUT,exist_ok=True)
log=open(OUT+'/draws.txt','w')
def P(*a):
    log.write(' '.join(str(x) for x in a)+'\n'); log.flush()
def work(ctrl):
    try:
        textures={t.resourceId:t for t in ctrl.GetTextures()}
        P('textures',len(textures))
        def walk(a,depth=0):
            for c in a:
                if c.flags & rd.ActionFlags.Drawcall:
                    ctrl.SetFrameEvent(c.eventId,True)
                    st=ctrl.GetVulkanPipelineState()
                    P('--- draw',c.eventId,c.GetName(ctrl.GetStructuredFile()),'idx',c.numIndices)
                    outs=ctrl.GetPipelineState().GetOutputTargets()
                    for o in outs:
                        t=textures.get(o.resource)
                        P('  out',o.resource,t.width if t else None,t.height if t else None,t.format.Name() if t else None)
                    for i,b in enumerate(st.colorBlend.blends):
                        P('  blend',i,'en',b.enabled,'src',b.colorBlend.source,'dst',b.colorBlend.destination,'op',b.colorBlend.operation,'asrc',b.alphaBlend.source,'adst',b.alphaBlend.destination,'wm',b.writeMask)
                    P('  blendconst',st.colorBlend.blendFactor)
                    for sname,sh in (('vs',st.vertexShader),('fs',st.fragmentShader)):
                        P('  ',sname,sh.resourceId)
                    ro=ctrl.GetPipelineState().GetReadOnlyResources(rd.ShaderStage.Fragment)
                    for r in ro:
                        e=r.descriptor
                        t=textures.get(e.resource)
                        P('  tex bind',r.access.index,e.resource,t.width if t else None,t.height if t else None,t.format.Name() if t else None,'mips',t.mips if t else None,'view',e.format.Name() if e.format else None,'sampler',r.sampler.filter if hasattr(r,'sampler') else None, getattr(r.sampler,'mipBias',None) if hasattr(r,'sampler') else None)
                walk(c.children,depth+1)
        walk(ctrl.GetRootActions())
        # save every texture that is an output or input as png
        want=set()
        def walk2(a):
            for c in a:
                if c.flags & rd.ActionFlags.Drawcall:
                    ctrl.SetFrameEvent(c.eventId,True)
                    for o in ctrl.GetPipelineState().GetOutputTargets(): want.add(o.resource)
                    for r in ctrl.GetPipelineState().GetReadOnlyResources(rd.ShaderStage.Fragment):
                        want.add(r.descriptor.resource)
                walk2(c.children)
        walk2(ctrl.GetRootActions())
        for rid in [w for w in want if w in textures]:
            ts=rd.TextureSave(); ts.resourceId=rid; ts.destType=rd.FileType.PNG; ts.mip=0; ts.slice.sliceIndex=0
            ts.alpha=rd.AlphaMapping.Preserve
            ctrl.SaveTexture(ts,OUT+'/tex_%s.png'%int(rid))
            t=textures.get(rid)
            P('saved',int(rid),t.width,t.height,t.format.Name())
    except Exception as e:
        import traceback; P('ERR',traceback.format_exc())
    P('DONE')
def loaded(*a):
    pass
cap=os.environ['OAG_RDC']
pyrenderdoc.LoadCapture(cap, rd.ReplayOptions(), cap, False, False)
pyrenderdoc.Replay().BlockInvoke(work)
pyrenderdoc.Replay().BlockInvoke(lambda c: os._exit(0))
