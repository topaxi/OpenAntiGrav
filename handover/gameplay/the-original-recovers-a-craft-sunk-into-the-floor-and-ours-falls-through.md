# Falling off the track: what was measured, and the one divergence found

2026-09-29, from a maintainer report made by feel ("easier to fall off than in the
original", no circuit named). Full write-up with tables:
[docs/gameplay/leaving-the-track.md](../../docs/gameplay/leaving-the-track.md). A
measurement pass; **no physics changed**.

**Ruled out, with numbers**: flying too high (rest ride height 2.888 vs 2.888,
0.015 apart at speed on the straight; airtime 14 vs 14 and 13 vs 13 ticks, apex
ratio 0.98/1.01 on the two crests the captures hold), wall response (3 of 11,772
shove trials cross a `Wall` triangle, none leaves the track), authored containment
we drop (the PSP ships no `Cage Collision`), a speed-dependent vertical force.

**Where craft leave** (survey, 24 circuit-directions x 4 tiers x lone AI and slot 0
on the same driver; ~980 of 1,098 events are Novice `13`/`29`, the known
`grip_believed` pathology): `17_Track` (`01_Track` reversed) index ~900-910, an
authored `Reset` volume, every lap at every tier (65 events; the gate runs forward
circuits only and never sees it); `01_Track` index ~31, the same fall at a slow
crossing (21); grid slots 1/3/5/7 on `01_Track` and `17_Track` spawn past the corridor
edge and fall from tick 0 - **the gate's `01_Track` respawn at 794 is this, not a
driving fall, and slot 0 is unaffected**; and open edges on `04`, `05`, `07`, `10`,
`20r`, `23r`, `26r`, `30r`, where 12-21% of shoves leave and the wall-less stretch is
in the collision data itself.

**The divergence**: placed 3.6 units below its rest height on level `Floor`
(`03_Track` index 200), the original **pushes the craft back to y 4.64** (its rest
height) within 60 ticks and ours **falls through the floor** (y -155). The ray in
`Ship_CastHoverProbes` starts at the probe in both, so it is not a lifted ray;
`hover::sweep`'s doc says "nothing in the original does this", which this contradicts.
Letting `wall::responds` accept `Floor`/`MagFloor` reproduces the recovery (y 4.65 at
tick 5) **and breaks the regression gate** (`01_Track` lost at `[794, 71, 72, 72, 75]`,
no clean lap), so it was reverted.

Instruments left behind, all `#[ignore]`d/`OAG_SWEEP`-gated: `falloff_survey_ground_truth.rs`
(survey, grid, shove sweep, edge profile, single-trial trace with a `place` line and
`OAG_SINK`), `ride_height_ground_truth.rs`, `airtime_arc_ground_truth.rs`,
`steady_state_vertical_ground_truth.rs`, `shove_wall_pose_walk_ground_truth.rs`
(env-driven, scratch capture). `RespawnCause` and `Race::last_respawn_cause_of` /
`respawn_given_up_of` record what fired a respawn (outside `state_hash`).

## Open

- Which original mechanism recovers a sunk craft (`Collision_BoxAgainstMesh`
  `0x08815cd4`, `Collision_StepNarrowphase` `0x088159c0`, `Ship_HoverTwoPoint`
  `0x0884a658`), and whether it matters in play: a probe under a face needs a steep
  landing or a crest lip. Deep Ghidra work.
- What the original does at an open edge: unmeasured; only `16`, `03` and `18r` are
  reachable from the front end. A teleported shove tumbles the original's craft (up.y
  -0.9 by the first recorded tick), so it is not a usable test.
- The `01_Track` odd-column grid: `Race_ComputeGridLayout` (`0x0882b3b0`) may clamp or
  scale the stagger by width; read the original's eight slots on that circuit. It
  changes the number the gate quotes, so it is a separate change.
- `17_Track` index ~909 and `01_Track` index ~31: the racing line drops through a hole
  under samples 31-42 into a `Reset` volume, and the craft arrives at 16-30 u/s. An AI
  line question (the driver ignores speed pads), not a physics one; not chased.

## Next Steps

1. Read the sunk-craft recovery out of the PSP binary, then port the narrowest
   version that keeps `01_Track` clean (an opus follow-up).
2. Drive, not teleport, an original craft out of the line at `03_Track` index 1780 with
   `just scripted-emu` and replay the same script through `oag-trace run`.
3. Add the reversed circuits to the regression gate's lone-craft test, so `17_Track`
   is seen.
