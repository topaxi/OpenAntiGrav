---
status: ready-to-wire
titles: [pulse]
updated: 2026-11-14
summary: "yaw_authority_ground_truth still skips on steer-left/right, which are untracked; allowlist and commit them to make absence a panic"
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

- `maglock_ground_truth.rs` now panics on a missing capture (done).
- `crates/trace/tests/yaw_authority_ground_truth.rs` `capture_path` still skips
  on a missing `talons-junction-steer-left.csv` / `-steer-right.csv`: those two
  are NOT tracked (`.gitignore` has no `!` line for them; they exist only in the
  maintainer's `data/traces/`). To make the skip a panic, allowlist both in
  `.gitignore` and commit them (42 KB each, ADR-0046), then apply the
  `require_capture` shape. The other `skipping:` sites in `crates/trace/tests`
  (`pads_ground_truth.rs:30`, `lap_capture_ground_truth.rs:93`) were not checked.

## Next Steps

1. Decide: allowlist and commit the two steer captures, then drop the skip.
