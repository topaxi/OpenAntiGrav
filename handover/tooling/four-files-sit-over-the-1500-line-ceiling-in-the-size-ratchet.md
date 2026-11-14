---
status: ready-to-wire
titles: [all]
updated: 2026-11-14
summary: "Four files are frozen over check-size's 1,500-line ceiling; split each along its existing seams so BASELINE empties"
size: M (30)
difficulty: easy (35)
kind: debt
---

# Four files sit over the 1,500-line ceiling in the size ratchet

2026-10-27, found by `just debt`. `scripts/check-file-size.py` freezes four Rust
files above the 1,500-line limit in its `BASELINE`. The line counts below were
read with `wc -l` on that date:

| file | lines | frozen at | seam already visible |
| --- | --- | --- | --- |
| `crates/game/tests/race_ground_truth.rs` | 2266 | 2266 | 26 `#[test]` functions; split by circuit or claim into modules of `crates/game/tests/it_*.rs` |
| `crates/game/src/movie.rs` | 1715 | 1715 | `Feed`, `Ring`, `FrameStore`, `Decode` and `Frames` are separate types; `mod bink`, `mp4`, `gst` etc. are already out |
| `crates/game/src/boot.rs` | 1546 | 1572 | `boot/options.rs` is the precedent (see the comment on its `BASELINE` row) |
| `crates/trace/src/main.rs` | 1543 | 1543 | `main.rs` mixes the clap structs, `drive_scenario`, `plan_scenario`, `dump_track`, `dump_pads`, `run`; each subcommand is a candidate module |

Rechecked 2026-11-14 with `wc -l` against `scripts/check-file-size.py`'s `BASELINE`
(line 119): `race_ground_truth.rs` 2266/2266, `movie.rs` 1715/1715, `trace/src/main.rs`
1543/1543 are unchanged and still over. `boot.rs` has shrunk to 1546 under a row of
1572, so its row can be lowered by 26 (a one-number change in the script, left to
whoever splits it). All four still need a split.

## Open

- Whether `race_ground_truth.rs` can be split without paying a second disc read
  per slice (CLAUDE.md: a disc-backed test shares nothing between tests). Check
  what the tests load before choosing the split.
- Which of the four is cheapest. `crates/trace/src/main.rs` is a binary with no
  other crate reaching it, so it is the likely first move.

## Next Steps

1. Lower `boot.rs`'s row from 1572 to its length (1546).
2. Split `crates/trace/src/main.rs` along its subcommands; delete its row.
3. One lane per remaining file; each move checked with `cargo nextest list -p
   <crate>` before and after, so no test disappears.
4. When `BASELINE` is empty, delete this thread.

## 2026-11-02 addition: slack and the files just under

- 14 non-test files sit at 1,300 to 1,500 lines (`vex/src/vex.rs` 1,498, `fx/src/psys.rs` 1,489,
  `raceplay/src/drawable.rs` 1,470), 83 over 1,000. The cap rose from 1,000 on 2026-10-12 and the tree
  grew into it. A warn tier at 1,400 in `check-size` would show them before they are over. See the
  thread on functions of 900+ lines for the function side of the same blind spot.
