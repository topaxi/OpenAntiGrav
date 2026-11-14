---
status: ready-to-wire
titles: [pulse]
updated: 2026-11-14
summary: "maglock_ground_truth and yaw_authority_ground_truth still skip on tracked traces; a stale comment says the -omega capture was never committed"
difficulty: easy (25)
---

# Reference traces: what is left of the skip-hid-it audit

Of the six captures, all are now tracked in git (`git ls-files data/traces`
lists `pad0-boost.csv`, `talons-junction-standing-start.csv`,
`talons-junction-pitch-both-ways.csv`, `talons-junction-time-trial-lap-omega.csv`
(3,140 data rows, commit `d43a8c398`), `talons-junction-time-trial-lap.csv` and
`venom-straight.csv` (200 rows, recaptured `bedcfc1a1`)). The lesson that
survives: `just test-data` green is weaker than it looks, `OAG_REQUIRE_GAME_DATA=1`
turns an absent optional input into a failure, and
[ADR-0046](../../docs/architecture/adr/0046-test-referenced-traces-are-tracked-in-git.md)
tracks a trace a test names. The doc question on `-omega` is settled in
`docs/physics/cornering-ground-truth.md`, "The capture names above drifted".

## Open

- `crates/raceplay/tests/maglock_ground_truth.rs` `rows()` and its doc comment
  (about lines 130-185) still say the capture "was never committed" and "no
  checkout has" it, and skip when absent. It is tracked, so absence is a broken
  tree and should panic like `chase_camera_ground_truth.rs::require_capture`.
  `the_capture_is_still_on_the_allowlist` can stay.
- `crates/trace/tests/yaw_authority_ground_truth.rs` `capture_path` (about line
  133) skips on a missing `steer-left`/`steer-right` capture unless
  `OAG_REQUIRE_GAME_DATA` is set; both are tracked, so it should panic
  unconditionally. Check the other `skipping:` sites in `crates/trace/tests`
  (`pads_ground_truth.rs:30`, `lap_capture_ground_truth.rs:93`) are disc images,
  which may keep skipping.

## Next Steps

1. Replace those two skips with the `require_capture` shape and rewrite the
   stale `maglock_ground_truth.rs` comment. Under an hour; `oag-raceplay` and
   `oag-trace` test files only.
