# lane/hd-gantry-wire - report

## Headline

**The countdown mechanism is found and wired, and it reads `3`/`2`/`1`/`GO`
correctly on a real capture.** It is **one** Edge Animation curve - material
2's, on `321go_startfinish.rcsmodel` - not four. That one curve drives both
the digit glyph mesh (`pasted__Go_HD_start_light_321goShape`, the node with
the five UV cells `docs/rendering/start-gantry.md` already measured) and the
backdrop panel behind it (`Go_HD_start_light_backgroundShape`), because both
share the same texture and material. The other three curved materials are
real and now replay correctly too, but their own geometry - the
chequered-flag state, slot 7's embedded `fx350` art, and the `FINAL LAP`
state - is removed from every frame by mechanisms this project already had
(`oag_render::gantry::clip_to_panel`, `::strip_fx350_art`), so they never
reach a player's eye regardless of what their curves sample. This is the
second correction of this lane's own docs the same day - see "Corrections
made in flight", below, read it before trusting the headline of an earlier
commit.

## Frames (absolute paths, `oag-game --race --track
/data/environments/talons_junction/track.vex --no-audio --screenshot`,
`hdfury-ps3-eu-dec.iso`)

| tick | seconds | file | what shows |
| --- | --- | --- | --- |
| 0 | 0.0 | `/home/topaxi/projects/oag-lane-wire/scratch/gantry-wire-frames/00-t0-blank.png` | board present, nothing lit |
| 90 | 1.5 | `/home/topaxi/projects/oag-lane-wire/scratch/gantry-wire-frames/01-t90-3-and-2.png` | **`3` and `2` both legible**, red banner |
| 180 | 3.0 | `/home/topaxi/projects/oag-lane-wire/scratch/gantry-wire-frames/02-t180-3-2-1.png` | **`3`, `2`, `1` all legible together**, red banner |
| 240 | 4.0 | `/home/topaxi/projects/oag-lane-wire/scratch/gantry-wire-frames/03-t240-go.png` | **`GO` alone, digits gone**, banner now green |
| 360 | 6.0 | `/home/topaxi/projects/oag-lane-wire/scratch/gantry-wire-frames/04-t360-teleported-out.png` | board gone - the pre-existing, unrelated `Anim Transform` teleport-out at the loop's 6.000 s close, untouched by this lane |

Intermediate ticks 30/45/60/75/105 (also captured, not tabled) show a
garbled, part-revealed banner - the wipe genuinely takes a fraction of a
second per glyph, it does not snap. `3` and `2` were never caught mid-reveal
one without the other in this sweep; they become legible together, both by
tick 90.

**Control render, curve replay disabled** (`GpuVertex::anim` forced to `0`
for every PS3 vertex, a one-line temporary edit, reverted before committing -
`git diff` was empty afterwards): same ticks, same track.

| tick | file | what shows |
| --- | --- | --- |
| 0 | `/home/topaxi/projects/oag-lane-wire/scratch/gantry-wire-frames/control/c0-t0.png` | blank (same as the real render) |
| 90 | `/home/topaxi/projects/oag-lane-wire/scratch/gantry-wire-frames/control/c1-t90.png` | **nothing** - no banner, no digits, no colour |
| 180 | `/home/topaxi/projects/oag-lane-wire/scratch/gantry-wire-frames/control/c2-t180.png` | **nothing** |
| 240 | `/home/topaxi/projects/oag-lane-wire/scratch/gantry-wire-frames/control/c3-t240.png` | **nothing** |

This is the discriminator an advisor review asked for before trusting the
headline: if a separate, curve-independent `Anim Transform` swapped a red
digit panel for a green `GO` panel, the control render would still show that
swap (node motion is untouched by disabling `anim`). It does not - the
backdrop and the digits vanish **together**, at every tick, which is what
you get when one shared texture crop (governed by the curve) is the only
thing making either visible at all, not two separately-triggered panels.

## What is established, in order

1. **Generic curve replay, wired and committed** (`66687bba`). Every
   `.rcsmodel` material carrying an Edge Animation curve becomes an
   `oag_render::mesh::AnimTrack::Rcs`, sampled every frame through the same
   per-model clock Pulse's own `TEXOFFSET` tracks already ride via
   `mesh.wgsl`'s existing `TexAnims` table - no shader change, no new time
   base. `curve_track::material_anim_tracks` resolves each curved material's
   own static `uvOffset`/`uvScale` (default identity) and overwrites only
   the components its own curve's channels drive, matched by resolved
   parameter name hash. Verified against the real disc,
   `hd_gantry_curve_replay_ground_truth.rs`: all four of the gantry's own
   curved materials reach `AnimTrack::Rcs`, a vertex of each selects one, and
   the sampled table differs between two points in time. `MaterialSetup`
   moved to `mesh/rcs/setup.rs` and the new `AnimTrack` enum to
   `mesh/anim_track.rs` to stay under the 1,000-line file-size ratchet.
2. **The per-node `+0xe4` static UV override table, chased and ruled out,
   live.** `Billboard_UpdateInstanceUvs`'s own Ghidra plate comment (already
   confidence 88) gives the precise mechanism: `node+0xe4` is a *pointer*,
   and the actual per-component override floats sit at `[pointer+0x18..0x28)`
   (`uvScale.z`, `.w`, `uvOffset.x`, `.y`), each individually gated by a
   `0xffffffff` "leave alone" sentinel. `scratch/gantry_dump_e4.py` (patched
   watchpoint RPCS3, interpreter mode, reusing `scripts/rpcs3-drive.py`/
   `rpcs3_debugger.py` exactly as the previous lane's `gantry_watch.py` does)
   reads that pointer for all 19 of slot 8's own instances on Talon's
   Junction, at load, +3 s and +6 s into a real countdown: **`NULL` on every
   instance, every time, including the digit board's own submesh.** This
   table plays no role in the countdown; the two previously-unexplained
   rest-state `uvOffset` values (submesh 9, 18) are simply those materials'
   own static parameters, not this table's doing.
3. **A first render of the countdown** (the "Frames" table above) showed a
   correct `3`/`2`/`1`/`GO` sequence, and this lane's first pass at writing
   it up concluded "four curves, one per glyph, each its own material" -
   wrong about *which* curve does the work, caught by an advisor review
   before this report was finalised (see below).
4. **A node-level check and a control render settled which curve actually
   reaches the screen.** `hd_gantry_curve_replay_ground_truth.rs`, run with
   a temporary diagnostic printing each animated draw's node name (reverted
   after use, not committed), shows material 2 alone binds both
   `pasted__Go_HD_start_light_321goShape` (the digit glyph) and
   `Go_HD_start_light_backgroundShape` (the backdrop); materials 1/3/4 bind
   `polySurface7Shape` (chequered flag), `polySurface151Shape`..`157Shape`
   (slot 7's own `fx350` art) and `pasted__Final_Lap*Shape` (`FINAL LAP`) -
   all three already excluded from every frame by
   `oag_render::gantry::clip_to_panel`/`::strip_fx350_art`. The
   curve-disabled control render (above) confirms it: disabling every
   curve drops the backdrop and the digits together, which only happens if
   one shared mechanism draws both.
5. **All of this written up**, including the two in-flight corrections
   themselves, in `docs/formats/edge-animation.md`,
   `docs/rendering/start-gantry.md` and
   `docs/ghidra/functions/ps3-hdfury-eu/billboards.md` - each correction is
   left in place with what it corrects rather than silently overwritten, per
   this project's own documentation standard.

## Corrections made in flight, and why they are here rather than squashed

Two commits' worth of docs text turned out to need fixing the same day, both
caught by stopping to ask "does this actually hold" before calling the lane
done:

1. **First commit (`ce2565bf`)**: claimed the curve was a wipe and "does not
   make the digits read correctly", written *before* the countdown was
   actually rendered. Corrected once it was: it does read correctly.
2. **This report's own first draft** then claimed "four curves, one per
   glyph, each its own material" - an inference from "four materials carry a
   curve" that was never checked against which of those four materials'
   geometry a player actually sees. An advisor review asked the discriminating
   question directly (does the red/green change come from node motion or
   from the curve?) and named the exact check (a curve-disabled control
   render) before this went out. The control render and the node-level dump
   above are that check, and they narrow the finding to one curve.

Both corrections are left visible in the docs (search each file for
"Corrected" / "resolved") rather than rewritten as if the first reading
never happened - the same standard `docs/ghidra/functions/ps3-hdfury-eu/billboards.md`
already holds itself to for its own past mistakes (the wrong table-base name,
the stale `lwz 0x834(` lead).

## Clean-room

No leaked, licensed or confidential vendor source consulted. The Edge
Animation Tools layout comes from this binary's own embedded assert strings
and decompile (prior lane) plus this lane's own re-read of the existing
Ghidra plate comment on `Billboard_UpdateInstanceUvs` - nothing external.

## Gate

- `flock ~/.cache/oag/gate.lock just`: **exit 0**, 3827 tests passed, 896
  skipped, all checks (docs, deps, transcendentals, file-size, ghidra-names,
  ghidra-captures, handover-size, link-worktree-data self-check, strings,
  just-args, gen-re-coverage) clean. Watched to completion, not left for a
  notification.
- `flock ~/.cache/oag/gate.lock env OAG_REQUIRE_GAME_DATA=1 just test-data`:
  **4723/4723 passed, 0 skipped, 0 failed.** Overall recipe exit 1, but only
  from `check-test-budget`: `ai_roll_ground_truth` (525s/361s/341s/319s) and
  `ram_ground_truth` (379s) over the 300s per-test ceiling under full-suite
  contention - the exact known non-test red the brief names (measured 71.4s
  in isolation on `main`). No `BASELINE` row added, nothing split. Watched to
  completion (`/tmp/test-data-1.log`), read after exit.
- `race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
  present, unmodified (grep-checked before and after; this lane never touched
  `crates/game/tests/race_ground_truth.rs`).
- Both gate runs happened before the docs-only corrections above and before
  the temporary diagnostic/control edits - those never left the working tree
  committed (`git status`/`git diff` clean against `ce2565bf` after each was
  reverted) and touch only `docs/` otherwise, so per `CLAUDE.md` only
  `check-docs` (clean, re-run after every doc edit) applied to them, not a
  full gate re-run. `cargo build -p oag-render`/`-p oag-game` and the
  `hd_gantry_curve_replay_ground_truth` test were run after each temporary
  edit and after each revert, confirming a clean revert each time.
- `just check-status`/`gen-status`: not applicable - no Ghidra rename landed
  this pass (the `+0xe4` mechanism was already named at confidence 88 by a
  previous lane; this pass only re-read the existing decompile more
  carefully and ran a new live check against it).

## Lane boundaries

Touched only `crates/rcs/src/rcsmodel/material/curve.rs` (new `Channel::name_hash`
field), `crates/render/src/mesh.rs`, `crates/render/src/mesh/anim_track.rs`
(new), `crates/render/src/mesh/rcs.rs`, `crates/render/src/mesh/rcs/curve_track.rs`
(new), `crates/render/src/mesh/rcs/setup.rs` (new), `crates/render/src/mesh/rcs/pads.rs`,
a `material_anim: Vec::new()` line in five other `Model`-constructing files
(`collision.rs`, `track.rs`, `mesh/sky_cube.rs`, `mesh/merge.rs`, `pvs/tests.rs`)
forced by the new `Model::material_anim` field, and test files (`race/tests.rs`,
several `crates/render/tests/*.rs`) forced by the same field or by
`Model::anim_tracks`'s type change from `Vec<vex::TexTransform>` to
`Vec<AnimTrack>`. Did not touch `mesh.wgsl`, `mesh/emissive.rs` or `sky_cube.rs`'s
own logic (only the one forced field addition) - the existing `TexAnims`
shader path already did everything the replay needed.

## What is still open

- **`3` and `2` reveal together rather than strictly in sequence**, and the
  full sequence runs in ~4s of the asset's clock against the ~6s measured
  thrust gate - both reported as measured, not reconciled. Consistent with
  two of the glyph's five UV cells crossing their own reveal threshold close
  together, the same shape Pulse's own texel-grid table shows for its four
  cells, but not independently confirmed against the texture's own texel
  grid the way Pulse's page did.
- **Only Talon's Junction, one circuit, one boot**, for the capture sweep,
  the control render and the live `+0xe4` read. The `+0xe4` negative is
  treated as file-level (the `.vex`'s own node bytes do not vary by circuit)
  rather than circuit-specific, but this was not independently confirmed on
  a second circuit.
- **The other 78 curved `.rcsmodel` files are wired but not individually
  inspected** - the generic mechanism is verified end-to-end on the gantry
  alone; a front-end flyer or an environment model's own curve was not
  rendered and looked at this pass.
- **The `321Go_Zone`/`321Go_HD_Zone_Battle`/`321go_hd_detonator` mode
  variants** are untouched, per the standing instruction to do the plain
  race first.
- **Materials 1/3/4's own curves were not looked at further** once their
  geometry was confirmed off-screen - what they'd show if `clip_to_panel`'s
  own trigger were ever recovered (the chequered flag, `FINAL LAP`) is
  unknown and out of this lane's scope.
