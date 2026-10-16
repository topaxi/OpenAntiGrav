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
  scale), the Quake road ripple, the track chrome shine, GE hull lights.
- `<StartBoost>` is still withheld off the PSP (`load.rs`), a simulation change.

## Next Steps

- Measure the PS2 LeachBeam reticle (`FUN_00138d78`) on PCSX2.
- Read how the PS2 builds the hull overlay UVs from a VIF batch.
