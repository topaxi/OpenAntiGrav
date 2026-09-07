# PVS culling: three ceilings, one of them invented

(1) A craft's bounding sphere reaches 6-14 of the 64 sections, and **splitting batches by section at load** is the only lever that moves this - it trades draw-call count, so measure before building. (2) Which section a *mesh* belongs to is still unrecovered: `oag_render::pvs` invents it by sphere-versus-box overlap and says so, and recovering the original's association would retire the only invented part of the pipeline. (3) Moving entities have no section and are never culled by tier one.

## Open

- Which section a mesh belongs to is unrecovered; `oag_render::pvs` invents it by sphere-versus-box overlap
- Moving entities have no section and are never culled by tier one
- Whether splitting batches by section at load is worth it (trades draw-call count) is unmeasured

## Next Steps

- Recover the original's mesh-to-section association to retire the invented sphere-versus-box overlap
- Measure the draw-call tradeoff before building batch-splitting by section
