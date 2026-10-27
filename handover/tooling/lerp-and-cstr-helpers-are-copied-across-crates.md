---
status: ready-to-wire
titles: [all]
updated: 2026-10-27
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

## Open

- The two orderings of lerp round differently in `f32`. Simulation-side copies (`tables`) feed
  ground-truth tests, so merging them may move a bit; check before choosing the form.
- Whether the clamping variants are deliberate.

## Next Steps

1. Put `cstr_at` on `oag_bytes` and call it from the four readers (no re-export shims).
2. Add `lerp` to `oag-core` math (render-side copies may keep their own if the clamp matters).
3. Run the ground-truth tests of `oag-tables`, `oag-vex`, `oag-rcs` before and after.
