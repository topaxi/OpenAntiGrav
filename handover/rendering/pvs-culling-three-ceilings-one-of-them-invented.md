# PVS culling: two ceilings remain

**Corrected 2026-09-10 (thread audit note): this thread's own title and first bullet were already stale the day it was written.** It originally named three ceilings, the first being "which section a mesh belongs to is unrecovered, `oag_render::pvs` invents it by sphere-versus-box overlap". Commit `4a9b259e` (2026-08-03, "render(pvs): place draw calls by their authored section group, and the magstrip draws clean") had already replaced that invented placement with the authored one via `oag_vex::pvs::governing_sections`, formalised in [ADR-0014](../../docs/architecture/adr/0014-authored-section-placement.md) - three weeks *before* this thread was split out (`6cf2b45d`, 2026-08-25). `crates/render/src/pvs.rs`'s own module doc confirms placement is "structural, not spatial" today, and no sphere/box overlap test exists anywhere in the file. So there were never three ceilings for this thread's own lifetime, only two.

(1) A craft's bounding sphere reaches 6-14 of the 64 sections, and **splitting batches by section at load** is the only lever that moves this - it trades draw-call count, so measure before building. (2) Moving entities have no section and are never culled by tier one.

## Open

- Moving entities have no section and are never culled by tier one (`crates/render/src/pvs.rs`'s `visible()` returns `true` unconditionally for any `draw.moving`)
- Whether splitting batches by section at load is worth it (trades draw-call count) is unmeasured

## Next Steps

- Measure the draw-call tradeoff before building batch-splitting by section
