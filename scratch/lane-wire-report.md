# lane/hd-gantry-wire - report

## Headline

**The countdown mechanism is found and wired, and it reads `3`/`2`/`1`/`GO`
correctly on a real capture.** The four Edge Animation curves
`lane/hd-edgeanim` decoded are not a wipe layered on top of some other
selector - they *are* the selector, one curve per glyph's own material, each
fading its own crop window in at its own staggered point in the shared
13.333 s loop. Wiring the replay generically (not gantry-specific: 79 of the
disc's 379 `.rcsmodel` files carry a live curve, and all 79 now play instead
of freezing at frame zero) and then actually rendering a countdown is what
showed this - sampling one curve's own numbers in isolation, which is as far
as the previous two lanes got, looks exactly like a wipe with nothing to
select, and that reading was wrong about the aggregate.

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
tick 90. This is reported as measured, not smoothed into "one at a time,
one second each."

**What does not match a naive expectation, reported rather than hidden**:
`3`/`2` reveal together instead of strictly in sequence, and the whole
`3`→`GO` sequence takes ~4 s of the asset's own clock, not the ~6 s the
measured thrust gate takes. Both are read exactly as the disc's own four
curves author them.

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
3. **Both findings written up** in `docs/formats/edge-animation.md`,
   `docs/rendering/start-gantry.md` and
   `docs/ghidra/functions/ps3-hdfury-eu/billboards.md`, including an explicit
   correction of this lane's own earlier-committed doc text (both files'
   "curve is a wipe, not a selector, does not make the digits read
   correctly" framing was written *before* the frames were captured, and
   corrected once they were - noted in place, not silently overwritten).

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
- Both gate runs happened *before* the docs-only corrections in this report's
  "What is established" step 3 - those touch only `docs/`, so per `CLAUDE.md`
  only `check-docs` (clean, re-run after the edits) was needed for them, not
  a full re-run.
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
  thrust gate - both reported as measured, not reconciled.
- **Only Talon's Junction, one circuit, one boot**, for both the capture
  sweep and the live `+0xe4` read. The `+0xe4` negative is treated as
  file-level (the `.vex`'s own node bytes do not vary by circuit) rather than
  circuit-specific, but this was not independently confirmed on a second
  circuit.
- **The other 78 curved `.rcsmodel` files are wired but not individually
  inspected** - the generic mechanism is verified end-to-end on the gantry
  alone; a front-end flyer or an environment model's own curve was not
  rendered and looked at this pass.
- **The `321Go_Zone`/`321Go_HD_Zone_Battle`/`321go_hd_detonator` mode
  variants** are untouched, per the standing instruction to do the plain
  race first.
