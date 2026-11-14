---
status: ready-to-wire
titles: [all]
updated: 2026-11-14
summary: "shared helpers landed; ByteOrder moves to oag-bytes and oag-video takes oag-bytes (decided), then the vex and video copies go"
size: S (30)
difficulty: easy (40)
kind: debt
---

# lerp and cstr helpers: what is left after the shared ones landed

`oag_bytes` now has `cstr_at` / `cstr_at_or_end` (NUL required / unterminated tail
allowed; the four string readers in `oag-vex` and `oag-rcs` call them), the
`Option` word readers `u16_at` / `u32_at` / `u64_at` / `f32_at(data, at, ByteOrder)`
and the fixed-order `le::` / `be::` forms. `oag_core::math::lerp` is
`a + (b - a) * t`. `oag-rcs` and the `oag-formats` PSARC reader use them, as do the
`lerp` copies with that exact expression in `oag-ui`, `oag-fx`, `oag-game`,
`oag-render` and `oag-raceplay`.

## Open

- `oag-vex` keeps `vex.rs`'s `le::{u16,u32,f32}_at` (about 100 call sites) and
  the `ByteOrder`-taking `track.rs` / `collision.rs` forwarders. They panic on a short slice and
  forward to `oag_formats::ByteOrder`, which is a second `ByteOrder` type beside
  `oag_bytes::ByteOrder`. Moving them means a bounds audit per call site and
  deciding which `ByteOrder` survives.
- `oag-video` (`ipf.rs`, `bik.rs`) keeps its panicking `u16_at` / `u32_at`: the crate
  is documented as having no workspace dependencies, and every call follows a header length
  check. Taking `oag-bytes` is a decision for the maintainer.
- `lerp` copies that stay: `oag-tables` (`track_stats.rs` uses the shared expression
  but the crate has zero dependencies; `ai_race_stats.rs` uses `a * (1 - t) + t * b`, which
  rounds differently, so it must not move), `oag-vex` (`probes.rs`, `fog.rs`: no `oag-core`
  edge), `oag-ui-screens` (`hex.rs`, `pulse.rs`: no `oag-core` edge). Each would be a new
  dependency edge for a one-line function.
- `rcs/src/rcsmodel/psp2.rs`'s two panicking `f32_at` reads now return `None` from the
  enclosing parser on a short slice instead of panicking.

## Next Steps

Decided by the maintainer (2026-11-14): `ByteOrder` moves to `oag-bytes` (one type;
`oag_formats::ByteOrder` goes), and `oag-video` may depend on `oag-bytes` (update its row in
CLAUDE.md's crate table and `docs/architecture/workspace-layout.md`).

1. One `ByteOrder` in `oag_bytes`; `oag-formats` and its users take it; the `vex` forwarders and
   `vex.rs`'s `le::` readers go, with a bounds check per panicking call site.
2. `oag-video` takes `oag-bytes` and drops its `u16_at`/`u32_at` copies.
3. Delete this thread (the `lerp` copies listed above stay, each with its reason).
