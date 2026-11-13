---
status: ready-to-wire
titles: [all]
updated: 2026-11-13
summary: "Allocator-API arena measured, not adopted; open: HUD text quad list and sway list re-allocate every frame (plain reuse)"
size: S (50)
difficulty: easy (60)
---

# A per-frame arena on the stable `Allocator` API: not adopted

**Verdict: do not adopt.** Rust 1.100's `Allocator` (`Vec::new_in`, `Box::new_in`) gives no
measurable per-frame win on the render side. Full evidence: `docs/rendering/frame-thread-cpu.md`,
"A per-frame arena on the stable `Allocator` API: measured, not adopted".

- About 97 percent of a frame's Rust allocations are inside wgpu, wgpu-hal and gpu-allocator
  (Pulse 331 of 343, HD 846 of 857 in one bench frame). No wgpu API takes an allocator.
- The two largest own sites were stable-sort scratch buffers, which no `Allocator` reaches. They
  are fixed with plain code and landed (-2 allocations and about 25 KiB a frame, no frame-time
  claim).
- The arena prototype on the view's `DrawList` (`478f21394`, reverted in `92ae1bb78`) removed one
  allocation a view. That is below the bench's noise by two orders of magnitude. It costs
  `unsafe impl`s under an `allow(unsafe_code)`, a lifetime in `DrawList`'s type, and 146 lines.
- `bumpalo` 3.20.3's `allocator_api` feature fails on stable (E0554).

Revisit only if a wgpu release takes an allocator for its command lists, or a profile shows a
single own site of hundreds of allocations a frame.

## Open

- **The HUD's text quad list regrows every frame**: `crates/game/src/render/text.rs`, the
  `quads.push`. That is 8 reallocations and 26 KiB on Pulse, 7 and 28 KiB on HD, per frame, one
  view.
- **`crates/hud/src/sway.rs`'s placed list is collected anew every frame**: 10 allocations and
  12 KiB on HD.
- **`String` per text draw**: `ui::frontend::draw`'s `Draw::Text` (26 on HD) and
  `hud/draw.rs`'s `text.clone()`. This is the batch-two "Left" item; no allocator reaches it.

## Next Steps

1. Keep the text renderer's `quads` between frames (`clear()`, not a new `Vec`). Then place the
   sway list in place. Count with `perf-probe` marks `hud-frame` and `hud-paint`:
   `cargo build --release -p oag-game --features perf-probe`, then
   `OAG_RENDER_PERF=1 target/release/oag-game data/images/hdfury-ps3-eu-dec.iso --race --mode single_race --seed 1 --autopilot --no-audio --ticks 300 --screenshot out.png`.
   Screenshots must stay byte-identical.
