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

- The orbit guard has no test: `rg -c '#\[test\]' crates/view` finds none.
  `crates/view/src/offscreen.rs` already reads a frame back headless, so a
  test may not need a window; unconfirmed.

## Next Steps

- Build an `OrbitView` over a model with no vertices and draw one frame
  through the offscreen path; assert no panic.
