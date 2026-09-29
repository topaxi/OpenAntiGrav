# Falling off the track: what was measured, and the one divergence found

2026-09-29, from a maintainer report made by feel ("easier to fall off than in the
original", no circuit named). Full write-up with tables:
[docs/gameplay/leaving-the-track.md](../../docs/gameplay/leaving-the-track.md). A
measurement pass, then (same day) a partial port of the one divergence it found.

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

**The divergence, now read and partly ported (2026-09-29)**: a craft whose hull
is through a floor fell through in ours and is recovered in the original. The
original has three mechanisms, all read with addresses on
[collision.md](../../docs/ghidra/functions/psp-pulse-usa/collision.md#every-mesh-surface-reaches-the-hull-narrowphase-2026-09-29):
(1) the hull narrowphase reads no surface type, so floors make contacts (which
never damage, `0x08842648`); (2) `Collision_AddContact` keeps a contact only if
the sample's projection is in the crossed triangle; (3) `Body_StepWorld`'s
pass 1 clips the body back along its velocity (0.9 of the overshoot). **(1) is
shipped**: sunk craft recover on `16_Track` and `03_Track`, the gate stays clean
(`01_Track` `[794]` to `[794, 72, 71]`, the craft now beached on its flank below
the hole at samples 31-42 - which the original also does to a flank-down craft,
measured), and the survey's rescues drop 583 to 267 (`13_Track` Novice 345 to 26).
**(2)+(3)** match every placement trial tick for tick but cost `01_Track` its clean
lap, and (2) alone wrecks `05_Track`'s walls; parked on local branch
`sunk-craft-parked` (patch in `data/scratch/sunk-craft/gate-pass1.patch`).
Full tables: [leaving-the-track.md](../../docs/gameplay/leaving-the-track.md).

Instruments left behind, all `#[ignore]`d/`OAG_SWEEP`-gated: `falloff_survey_ground_truth.rs`
(survey, grid, shove sweep, edge profile, single-trial trace with a `place` line and
`OAG_SINK`), `ride_height_ground_truth.rs`, `airtime_arc_ground_truth.rs`,
`steady_state_vertical_ground_truth.rs`, `shove_wall_pose_walk_ground_truth.rs`
(env-driven, scratch capture). `RespawnCause` and `Race::last_respawn_cause_of` /
`respawn_given_up_of` record what fired a respawn (outside `state_hash`).

## Open

- **Land the projection gate and pass 1 together** once `01_Track`'s AI line
  stops dropping into the hole under samples 31-42 (the craft rolls onto its
  flank falling through it and, faithfully, stays there). Re-run the gate,
  `05_Track`'s wall-contact count, and the placement trials in
  `leaving-the-track.md`. Pass 1 would replace `wall::swept_contact` and may make
  `hover::sweep` redundant; neither was checked.
- `05_Track` under the gate: 60 wall-contact ticks to 2,475 without pass 1, 2,696
  with it (maxlat 8.53). One seed, one tier; unexplained.
- The hover penetration escape: at `03_Track` the original's two probes each
  translate by `1 - h` in one frame (+1.82); ours gave +0.81 in the matching
  prototype frame. Check that `HoverProbe::escape` sums both probes and gates on
  a mesh hit rather than on `Surface::Floor` (`craft+0x208 == 1`, engine.md).
- The original's first free frame after a clean placement at `03_Track` carries
  +1.9 u/s upward from nowhere; unexplained, likely placement residue.
- What the original does at an open edge: still unmeasured. The survey's
  "a teleported shove tumbles the original" was a placement artefact (a single
  write plus a late-attaching `psp-trace`); use the four-write method described
  in `leaving-the-track.md` and repeat the shove.
- The `01_Track` odd-column grid (`grid-stagger` lane).
- `17_Track` index ~909 and `01_Track` index ~31: the racing line drops through a
  hole into a `Reset` volume or onto the floor below. An AI line question.

## Next Steps

1. Fold the four-write placement and a per-tick log into `scripts/psp-drive.py
   place` (`--rewrites N --log out.csv`), so the trials are reproducible from
   the repository.
2. Fix or reroute `01_Track`'s line past samples 31-42, then land the parked
   gate + pass 1 and re-measure.
3. Repeat the open-edge shove at `03_Track` index 1780 with the clean placement.
4. Add the reversed circuits to the regression gate's lone-craft test, so
   `17_Track` is seen.
