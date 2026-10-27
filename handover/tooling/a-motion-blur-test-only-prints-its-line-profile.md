---
status: ready-to-wire
titles: [all]
updated: 2026-10-27
summary: "post's line_profile_is_reported prints pixel rows and asserts nothing; make it an ignored sweep or give it an assertion"
size: XS (45)
difficulty: easy (50)
kind: debt
---

# A motion-blur test only prints its line profile

2026-10-27. `crates/post/src/motion_blur/ghost_tests.rs` `line_profile_is_reported` runs on every
`just test` (no `#[ignore]`), loops over three velocities and only `eprintln!`s pixel rows
(`v {v} y {y}: {row:?}`); it has no assertion, so it can never fail. Its neighbour
`no_energy_lands_off_a_surfaces_swept_path` asserts. CLAUDE.md: a test that only prints is a scratch
sweep, `#[ignore]` with a reason and an early return unless `OAG_SWEEP` is set.

## Open

- Whether a profile property (peak centred on the line, symmetry) is worth asserting instead.

## Next Steps

1. Mark it `#[ignore = "scratch: prints the line profile; set OAG_SWEEP"]` with the env early return, or add an assertion.
2. Run `cargo nextest list -p oag-post` before and after.
