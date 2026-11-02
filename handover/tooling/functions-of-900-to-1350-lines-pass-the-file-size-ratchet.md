---
status: ready-to-wire
titles: [all]
updated: 2026-11-02
summary: "Eight functions run 830 to 1,350 lines and 14 files sit at 1,300+; check-size caps files, not functions, so none of it shows"
size: XL (35)
difficulty: medium (35)
kind: debt
---

# Functions of 900 to 1,350 lines pass the file-size ratchet

2026-11-02, `debt-survey`. `scripts/check-file-size.py` caps a file at 1,500 lines and an inline test
mod at 500. Nothing caps a function, so one function can fill a file. End lines were read in the
source (the closing brace at the `fn`'s indent):

| function | lines | length |
| --- | --- | --- |
| `crates/raceplay/src/load.rs:88` `load` | 88-1437 | 1,350 |
| `crates/game/src/race_capture.rs:393` `capture` | 393-1467 | 1,075 |
| `crates/game/src/main/session/frame.rs:41` `frame` | 41-1072 | 1,032 |
| `crates/game/src/capture.rs:199` `run` | 199-1223 | 1,025 |
| `crates/raceplay/src/scene/frame.rs:259` `prepare_view` | 259-1230 | 972 |
| `crates/game/src/main/session/draw.rs:124` `draw` | 124-1070 | 947 |
| `crates/raceplay/src/start.rs:30` `start` | 30-968 | 939 |
| `crates/mesh/src/mesh_render.rs:338` `build_with` | 338-1220 | 883 |

Next tier (roughly 660 to 720 lines): `raceplay/src/tick.rs:58` `tick_body` (58-721),
`game/src/main/session/apply.rs:172` `apply_setting` (172-830, see the settings thread).

The file ceiling was raised from 1,000 to 1,500 on 2026-10-12 and the tree grew into it: 83 non-test
`.rs` files are over 1,000 lines and 14 over 1,300 (`vex/src/vex.rs` 1,498, `fx/src/psys.rs` 1,489,
`raceplay/src/drawable.rs` 1,470, `game/src/race_capture.rs` 1,467, `fx/src/exhaust.rs` 1,452,
`raceplay/src/load.rs` 1,437, `rcs/src/gcn/isa.rs` 1,418). Files just under the cap are the ratchet's
blind side: the next edit either trips the gate or goes into a new sibling.

`load()` is the worst case. In its body 98 top-level `let`s and 30 `report.push` lines run in one
scope, and the file carries 186 `feature = "view"` sites (the crate has 647 in 39 of 345 files), so
view-only and simulation-only steps are interleaved in one straight line. That interleaving is
probably what keeps it from being cut by phase.

## Why it hurts

A contributor cannot hold a 1,000-line function in their head, cannot test a phase alone, and cannot
tell which `let` a later line depends on. Review diffs inside it are unreadable, and the `view`
cfg lines make the sim-only build (`oag-server`) hard to reason about.

## Open

- Whether each long function is a straight sequence (cut into phase functions returning a struct) or
  shares mutable state through all of it (needs a context struct first). Unmeasured: read each.
- `load()` decomposition comes before the crate split in the thread on splitting `oag-raceplay`'s load
  and scene (`the-workspace-is-split-further-splits-are-ranked-and-unstarted`): the phases
  are the seam that thread's "does `load` need `Race`" question needs answered.
- Threshold: `clippy::too_many_lines` at `too-many-lines-threshold = 300` flags 21 functions (measured 2026-11-02; 60 at 200, 201 at the default 100); a ratchet like
  `check-size` (frozen list, only shrinks) is the shape that fits this repo.

## Next Steps

1. Add a function-length ratchet beside `check-size` (script plus frozen `BASELINE` of the eight above
   and the 300+ tier). Code edit: a different lane from this one.
2. Take `raceplay::load::load` first: cut by phase along the existing `// ` banners
   (about 170 comment lines at one indent) into functions under `load/`, view-only phases behind one
   `cfg` each. Test count identical before and after (`cargo nextest list -p oag-raceplay`).
3. Then `start`, `tick_body`, `prepare_view`; one function per lane.

Verify: `just` (the new ratchet), `just test-data-affected` for `oag-raceplay` (the load and race
ground-truth tests), and `just check-size` for the file lengths.
