---
status: ready-to-wire
titles: [all]
updated: 2026-11-14
summary: "lerp is private in a dozen places and cstr in four; one shared helper each, with the float-order differences checked"
size: S (30)
difficulty: easy (40)
kind: debt
---

# lerp and cstr helpers are copied across crates

2026-10-27, `just debt` sweep, each site read. `rg 'fn lerp\w*\('` over `crates/`
finds 12 definitions and `oag-core` has none. Examples: `crates/tables/src/track_stats.rs:445`
(`a + (b - a) * t`), `crates/tables/src/ai_race_stats.rs:99` (`a * (1 - t) + t * b`, a
different float order), `crates/vex/src/probes.rs:127` (array form), `crates/ui/src/menu/skin.rs:889`
(clamps `t`), `crates/ui-screens/src/picker/hd/hex.rs:251` (does not).
NUL-terminated string readers: `crates/rcs/src/rcsmodel/material.rs:549`,
`crates/rcs/src/rcsmodel/psp2/material.rs:496`, `crates/rcs/src/rcsmodel/psp2/nodes.rs:302`,
`crates/vex/src/vex.rs:712`, while `oag_bytes::Reader::cstr` (`crates/bytes/src/reader.rs:240`) exists.

Rechecked 2026-11-14: nothing has landed. `oag-core` and `oag-bytes` still have no `lerp` or
slice-form `cstr_at`/`u*_at`; the `cstr` copies are now `vex/src/vex.rs:638`, `rcs/src/rcsmodel/psp2/nodes.rs:311`,
`psp2/material.rs:496`, `rcs/src/rcsmodel/material.rs:549`; `fn (u16|u32|f32)_at` still has 22 non-test hits.
Line numbers elsewhere in this note drift.

2026-11-02 addition: the same copying holds for offset readers. `rg 'fn (u16|u32|f32)_at'`
finds 22 non-test definitions: `crates/vex/src/vex.rs:622,626,630` (panics), `vex/src/track.rs:841,845`
and `vex/src/collision.rs:895,899,903` (take a `ByteOrder`), `formats/src/psarc.rs:718,722`,
`video/src/ipf.rs:176,180`, `video/src/bik.rs:222`, `rcs/src/gcn.rs:281`, `rcs/src/rcsmaterial/vertex.rs:194,198`,
`rcs/src/rcsmodel/psp2/nodes.rs:293,297`, `psp2/material.rs:516` (these three return `Option`),
`psp2/container.rs:117` and `rcs/src/rcsskeleton.rs:425` (`Result` with a `what` label),
`psp2.rs:976`. `oag_bytes` has only the cursor `Reader`, no slice-and-offset form.

## Open

- The `_at` copies disagree on failure: panic, `Option`, `Result` with `what`, or an extra `ByteOrder`
  argument. One helper set needs all three result shapes or the callers a decision on which they want.
  Panicking ones (`vex.rs:626`) are fine only where a prior length check exists; confirm per call site.

- The two orderings of lerp round differently in `f32`. Simulation-side copies (`tables`) feed
  ground-truth tests, so merging them may move a bit; check before choosing the form.
- Whether the clamping variants are deliberate.

## Next Steps

1. Put `cstr_at` on `oag_bytes` and call it from the four readers (no re-export shims).
2. Add `lerp` to `oag-core` math (render-side copies may keep their own if the clamp matters).
3. Add `u16_at`/`u32_at`/`f32_at` (returning `Option`, plus a `ByteOrder` form) to `oag_bytes` and replace the 22 copies, one crate per lane.
4. Run the ground-truth tests of `oag-tables`, `oag-vex`, `oag-rcs` before and after.
