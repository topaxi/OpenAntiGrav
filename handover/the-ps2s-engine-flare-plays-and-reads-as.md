# The PS2's engine flare plays and reads as nothing on screen

2026-08-12. `WO_SHIP_ENGINEFLARE` is wired, attaches per craft, and **is** drawing (the nozzle glow changes with `psys::ColourScale`) - a maintainer just sees only the trail ribbon. The asset side is closed: both engine-flare textures are on the PS2 archive after all, under the declared name with the extension rewritten to `.pct` (`Engine_noise` 64x64 8bpp, `grabbedEngineFlare128x64x8` 128x64 8bpp) - a name-lookup gap, not an asset gap. See [texture-names.md](../docs/ghidra/functions/ps2-pulse-eu/texture-names.md). **Still open**: whether the flare reads on screen now (no frame captured either side of the fix) and whether the real PS2 build looks like this at all (needs a PCSX2 capture; no automated route exists, unlike the rocket explosion's `psp-fire-weapon.py`). Do not tune the flare to taste - a missing mechanism, not a coefficient, if it needs more. **The related boost-plume half is closed**: [The PS2 boost plume draws, and the PS2 file is not the PSP file wearing the same name](the-ps2-boost-plume-draws-and-the-ps2.md), dated 2026-08-23. Both are render-side, neither touches the simulation.

## Open

- Whether the flare reads on screen now is unconfirmed - no frame captured either side of the fix.
- Whether the real PS2 build looks like this at all is unconfirmed - needs a PCSX2 capture, and no automated capture route exists yet.

## Next Steps

- Capture frames either side of the fix to confirm the flare reads on screen.
- Build a PCSX2 capture route (none exists automated, unlike the rocket explosion's `psp-fire-weapon.py`) and take one to verify against the real PS2 build.
- Do not tune the flare to taste - treat it as a missing mechanism if it needs more.
