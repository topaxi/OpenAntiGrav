# PS2 Pulse carries the PSP's reticle art, wreck, circuit effects and record; what is unseen

2026-10-16. The PSP's law was carried to the PS2 for the HUD sight models and the
cockpit countdown model (texture set from the entry before), the `shipwreck.vex`
swap (`hull_wreck`), the circuit's placed particle effects, weather and destroy
camera (`circuit_effects`) and `stats.xml` (the RECORD readout). Evidence and the
disc census: `docs/ui/hud.md` and `crates/raceplay/tests/ps2_hud_sights_ground_truth.rs`.

## Open

- The PS2 reticle is wired to the measured 512x512 law and matches a PCSX2 reference (`docs/ui/hud.md`); not seen: a lock on a real AI craft via the original's own pickup, the LeachBeam reticle, the widescreen scale path. Headless: `--mode single_race` is needed for any weapon HUD (the default `time_trial` has none).
- Steam (circuits 5, 7, 11), weather and the destroy camera cut are loaded and
  reported on the PS2, not seen on a frame.
- Still PSP-only: absorb and LeachBeam hull overlays (a PS2 VIF batch has no GE batch
  scale), the track chrome shine, GE hull lights.
- The Quake road ripple now plays on the PS2 (2026-10-17, `road_ripple` rule, PSP's
  law carried over, unmeasured on PCSX2): the table's `+0x48` is an offset into one
  parameter pool past the record table, `+0x44` is not a position pointer
  (`crates/vex/tests/quake_ground_truth.rs`, `quake_ripple_ground_truth.rs`). Not seen
  on a frame. Open: a few PS2 stretches sit a constant distance off their place
  (`04_Track/track_reversed.vex` spans 454 to 461, 194 units; `09_Track`,
  `02_Track` reversed), cause unchecked - locator ambiguity or real; `08`, `11`,
  `12` and `15` are skipped by the placement test as on the PSP. The orange screen
  tint is not drawn on the PS2.
- `<StartBoost>` is still withheld off the PSP (`load.rs`), a simulation change.

## Next Steps

- Measure the PS2 LeachBeam reticle (`FUN_00138d78`) on PCSX2.
- Read how the PS2 builds the hull overlay UVs from a VIF batch.
