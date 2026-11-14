---
status: ready-to-wire
titles: [all]
updated: 2026-11-14
summary: "Add test coverage for the orbit guard at crates/view/src/orbit.rs:560 (empty vertex or index buffer); the race-side guard question is moot"
difficulty: easy (35)
---

# Task #31 residual: the orbit guard has no test

`oag-view --collision`'s panic on an empty vertex buffer is fixed by a guard at
`crates/view/src/orbit.rs:560` (`if !self.model.vertices.is_empty() &&
!self.model.indices.is_empty()`), skipping every `slice(..)` below it. The
race side no longer has the question: `crates/raceplay/src/drawable.rs` has no
`Buffer::slice(..)` call today (its draw goes through `DrawCall`), and the
early-return the older note cited moved with it.

## Open

- The orbit guard has no test and cannot get one as the code stands. It is
  inside `Session::frame`, which needs a winit window and surface;
  `crates/view/src/offscreen.rs` is a texture readback (`capture(asset, path)`)
  and draws no model. The headless model draw is `oag_mesh::capture_pixels_from`
  (`crates/mesh/src/capture.rs:371`, the twin guard), which is a different
  call site.

## Next Steps

- Either split the render pass out of `Session::frame` into a function taking a
  device, queue and `Model` and test it through an offscreen target (in the
  `gpu-vram` group), or test the twin in `capture.rs` with an empty `Model`
  and accept that `orbit.rs:560` itself stays unguarded by a test.
