---
status: ready-to-wire
titles: [all]
updated: 2026-11-02
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
| `crates/game/src/movie.rs` | 1715 | 1756 | `Feed`, `Ring`, `FrameStore`, `Decode` and `Frames` are separate types; `mod bink`, `mp4`, `gst` etc. are already out |
| `crates/game/src/boot.rs` | 1661 | 1661 | `boot/options.rs` is the precedent (see the comment on its `BASELINE` row) |
| `crates/trace/src/main.rs` | 1543 | 1543 | `main.rs` mixes the clap structs, `drive_scenario`, `plan_scenario`, `dump_track`, `dump_pads`, `run`; each subcommand is a candidate module |

`movie.rs` is already 41 lines under its row: `just check-size` prints the hint
to lower it. The ratchet only lowers, so that row can drop to 1715 now.

## Open

- Whether `race_ground_truth.rs` can be split without paying a second disc read
  per slice (CLAUDE.md: a disc-backed test shares nothing between tests). Check
  what the tests load before choosing the split.
- Which of the four is cheapest. `crates/trace/src/main.rs` is a binary with no
  other crate reaching it, so it is the likely first move.

## Next Steps

1. Lower `movie.rs`'s `BASELINE` row to its current length (pure bookkeeping).
2. Split `crates/trace/src/main.rs` along its subcommands; delete its row.
3. One lane per remaining file; each move checked with `cargo nextest list -p
   <crate>` before and after, so no test disappears.
4. When `BASELINE` is empty, delete this thread.

## 2026-11-02 addition: slack and the files just under

- Re-measured: `movie.rs` 1,715 against a row of 1,756, `boot.rs` 1,657 against a row of 1,661.
  Lowering both rows edits `scripts/check-file-size.py` (a code lane); nobody has done it.
- 14 non-test files sit at 1,300 to 1,500 lines (`vex/src/vex.rs` 1,498, `fx/src/psys.rs` 1,489,
  `raceplay/src/drawable.rs` 1,470), 83 over 1,000. The cap rose from 1,000 on 2026-10-12 and the tree
  grew into it. A warn tier at 1,400 in `check-size` would show them before they are over. See the
  thread on functions of 900+ lines for the function side of the same blind spot.
