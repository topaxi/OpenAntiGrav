---
status: ready-to-wire
titles: [all]
updated: 2026-11-25
summary: "Track, craft, particles, sky, fog and transparent cost inside SCENE's one pass is untimed on tilers: desktop in-pass queries or a dev A/B toggle"
categories: [rendering, tooling]
size: M (60)
difficulty: medium (60)
---

# What happens inside the scene pass cannot be timed on a tiler by timestamps

Every pass the frame loop encodes on its own thread is timed (the `ROWS` table in
`crates/present/src/perf/passes.rs` names them). Still untimed besides the inside of `SCENE`:
passes recorded on another thread (each view of a split screen, a race with further windows) and
the FSR3 readback harness, which only tests run.

Every separately-encoded pass now has its own overlay row and frame-log field
(`docs/rendering/dynamic-resolution.md`, "Every other pass has a row of its own"). What stays
untimed is the inside of `SCENE`: the race is **one render pass** holding the sky, the track, its
shine, the craft, the particles, the fog and the transparent and additive layers, so its total
says nothing about which of them costs.

## Why timestamps inside a pass are not the answer on a tiler

A tile-based GPU (Mali in the S21, Xclipse in the S24, Apple's)
shades a pass tile by tile: it bins every draw of the pass first, then runs the whole pass once
per tile. A timestamp written between two draws is written once per tile or once per pass at the
bin stage, never "between those draws" in time, so the span between two marks measures nothing the
draws did. wgpu's `TIMESTAMP_QUERY_INSIDE_PASSES` is the optional feature that would allow it (not
generally offered on tilers; not checked per adapter here), and where a driver does offer it the numbers need not add up to the pass. Only the pass-boundary pair (`RenderPassTimestampWrites`) is meaningful there, which is all the per-pass
rows use.

## Open

- **Desktop only: in-pass queries.** `oag_gpu::perfprobe::marks` already writes named timestamps
  between draws on an adapter with `TIMESTAMP_QUERY_INSIDE_PASSES` (probe:
  `oag_gpu::timing::Timing::inside_passes`), but only for a bench built with `perf-probe`. Bring it
  to the overlay as a per-group split of `SCENE` on adapters that report the bit, off elsewhere,
  with the groups named by the scene's own draw order (sky, track, craft, particles, fog,
  transparent, additive).
- **Everywhere: a dev-overlay A/B toggle.** Switch one group off for N frames and show the
  frame-time delta (mean with the group on against mean with it off, over the same N frames), so
  a tiler gets an answer that costs the frame and not a timestamp. `OAG_SKIP_PASSES_TOGGLE`
  (`oag_gpu::skip`, `scripts/android-skip-windows.py`) already does this for whole passes on
  Android and splits the frame log into its two arms; extend it to the groups inside the scene pass
  and surface it on the dev overlay.

## Next Steps

1. List the draw groups the scene pass encodes, in order, from `crates/raceplay/src/scene/record.rs`.
2. Wire a per-group skip into `oag_gpu::skip` and the dev overlay (the A/B toggle first: it works
   on every adapter).
3. Offer the in-pass split on adapters that report `inside_passes`, reusing `perfprobe::marks`.
