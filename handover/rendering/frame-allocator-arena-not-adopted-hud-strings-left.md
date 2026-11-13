---
status: measure
titles: [all]
updated: 2026-11-13
summary: "Allocator-API arena measured, not adopted; HUD quad list was already reused and the sway slide now moves in place; the per-draw Strings are left"
size: S (40)
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

- **`String` per text draw**: `ui::frontend::draw`'s `Draw::Text` (26 on HD) and `hud/draw.rs`'s
  `format!`s and `text.clone()`. This is the batch-two "Left" item; no allocator reaches it. The
  sway slide's copy of every string is gone (`docs/rendering/frame-thread-cpu.md`, "The HUD's two
  plain-reuse candidates"); what is left is the strings `draw_list` builds, an unmeasured share of
  HD's 67 `hud-frame` allocations.

## Next Steps

1. Count first: which of HD's 67 `hud-frame` allocations are strings, with the repeat patch the
   doc section describes. Reuse a buffer across frames only where a measured count says it pays
   (a frame is 17 to 40 us here; no frame-time claim has been made for any of this).
