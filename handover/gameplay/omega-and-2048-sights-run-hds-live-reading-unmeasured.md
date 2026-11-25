---
status: measure
titles: [omega, 2048]
updated: 2026-11-25
summary: "Omega and 2048 inherit HD's measured missile/LeachBeam sight update (chase gain 4.0, extent units); measure each or document why not"
categories: [gameplay]
size: M (40)
difficulty: medium (50)
---

# Omega and 2048 sights run HD's live reading, unmeasured

## Open

The maintainer reported that Omega does not seem to lock a Missile, or at least its HUD does not show a lock. The lock law is shared by every title (`oag_race::sight`, driven by `crates/raceplay/src/lock.rs`); per title there is only data, `HudArt::sights`.

HD's sight update was measured live (`docs/ghidra/functions/ps3-hdfury-eu/hud-sight.md`, "Measured live"): chase gain `4.0` on an on-screen target, and the ring extent units. 2048 and Omega had `live: None`, which:
- chases the target at a quarter of HD's rate;
- runs the LeachBeam under the Missile's law;
- draws the rings at their authored size.

**Quick fix (2026-11-25, maintainer: "inherit by lineage"):** both now use `oag_hd::hud::LIVE_SIGHT`. The test `omega_and_2048_reticles_run_hds_live_sight` (`crates/omega/src/tests.rs`) pins it. Nothing about either title's own update has been read or measured.

## Next Steps

1. **Reproduce the report and confirm the fix in ours:** a headless Omega race and an HD race (`--race --no-audio`), the player holding a Missile 60 units behind a rival, recording the tick of the lock and a frame of the reticle. With the fix, Omega should lock on HD's tick and draw `MissileSightLockedOn*`. If it still does not, the cause is elsewhere (Omega's `Sighting::of` grid, the widget names resolving in Omega's layouts, or its HUD roots).
2. **2048's ring size:** its HUD grid is not HD's, so HD's extent units may draw the rings too large or too small. Compare a reticle frame of ours against a Vita3K frame of 2048 with a Missile held. If they differ, give 2048 its own extent units, measured.
3. **Measure each title's own update**, or document why it cannot be done:
   - **Omega:** the executable is in Ghidra. The bind is `FUN_0126c020`; search its functions for the sight widgets' offsets the bind stores at, then read the gain and extents. Live values are not checkable (shadPS4 has no memory readout), so a static read is the measure.
   - **2048:** the bind is `FUN_81158d88` (v1.04); same search. Vita3K can confirm a lock frame.
   - Record each answer on `hud-sight.md` as **ported**, **checked, differs** or **not checkable**, with evidence.
4. **The LeachBeam's own centre chase on HD** (`Hud_UpdateLeachBeamSight`, `0x00090d30`) was never checked for the gain. Read it once, since all three titles now run it.
