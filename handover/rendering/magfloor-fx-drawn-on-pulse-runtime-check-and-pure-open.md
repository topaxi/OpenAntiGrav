---
categories: [rendering, audio]
---

# Magfloor fx: drawn on Pulse from the recovered anchor; runtime check and Pure are open

**Landed 2026-10-05 (lane `magfloor-pulse`).** Every Pulse craft on a
magstrip now shows `MagEffect1.vex`/`MagEffect2.vex` under it
(`crates/raceplay/src/mag_floor_fx.rs`). The law is on
[`magfloor-fx.md`](../../docs/ghidra/functions/psp-pulse-usa/magfloor-fx.md),
confidence 88, PSP and PS2 agreeing:

- Both models sit at `(0, -2.5, 0)` in the craft's frame. The constructor's
  "boilerplate" identity block has row 3 `y` overwritten by
  `[0x08ab0ef0] = -2.5`. The 2026-09-02 reading missed this.
- MagEffect1 is a translate-only child of the craft, so it keeps the craft's
  rotation, roll and 0.75 scale. MagEffect2 hangs off the scene root.
  `MagFloorFx_Update` (`0x0885962c`) rebuilds it every tick on the track's up,
  pointed along the craft's nose, at unit scale.
- `craft+0x8bc` is the effect object. `Craft_Construct` builds one for every
  craft. The tint is opaque white (`MagFloorFx_InitTint` `0x08859904`).
- Shown while this tick's mag-floor probe hits. That is read back exactly
  off `ShipState::mag_lock_blend`'s ramp, so the simulation is untouched.

**Sfx: there is none to wire, confidence 80.** No sound code reads
`craft+0x240`/`+0x280`. Neither effect model authors a sound node. Track
emitters are not placed along the strips: the `magfloor_emitter_reach`
example shows hum-named emitters are densest on the circuits with no
magstrip. Our `TrackEmitters` already plays every authored emitter.

Verified as a player would: `oag-game --race --pose` on the strip on
circuits 01 and 05. Purple splayed streaks under and around the craft;
an A/B build with the effect disabled shows none. **Not** compared against
the original running.

## Open

- **Runtime check done 2026-10-05** (anchor 95, see the page's runtime
  section): offset, scale, ride-versus-lie and tint read live. Left open:
  the original's streaks **flicker per frame** (patch delta 2-54) and ours
  holds 14-15 at about 70 % of the mean brightness; cause unknown (the
  original's capture has bloom and blur ours lacks, or the Anim Transform
  rate differs). A `MagFloorFx_Show` breakpoint hit is also uncaught.
- The track's down for MagEffect2 comes from the nearest spline sample
  (chosen, the sim's own stand-in), not the located sample `craft+0xb10`.
- Pure: closed, a negative. Its executable has no magstrip class, no
  `visual_effects` and no `MagEffect` string, on either pressing
  ([magfloor-absent.md](../../docs/ghidra/functions/psp-pure-usa/magfloor-absent.md)).
  `WeaponModels::mag_floor` is `None` there as a finding.
- HD/Fury, 2048 and Omega belong to the `MagstripWake` lane, not this one.

## Next Steps

- Find what makes the original flicker: compare the Anim Transform's
  sampled values over the original's clock against ours, with the `--presented`
  path on our side. Matched-view recipe: Talon's Junction is `16_Track`
  (not `01_Track`), start pose `6.07,-50.07,-196.05`; ours needs a way to force
  the effect on there (a local env patch was used, `force-on.patch` in the
  lane scratch, not committed).
