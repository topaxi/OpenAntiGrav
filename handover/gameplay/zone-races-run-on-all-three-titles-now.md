# Zone races run on all three titles now, and the craft axis closed the way the circuit one did

2026-08-19, same branch. `--race --mode zone` loads and drives on Pulse, Pure and HD/Fury, each with its own look - Pulse cyan/teal, Pure white/lavender, HD the white ribbon. **The craft axis became a type the moment HD was measured**, and the sequence is the point: it was refused a day earlier at two-of-three (Pulse and Pure measured, HD unread), then HD's answer turned out to be `/data/ships/zone/ship.vex` - already on the roster as one of `oag_hd::names::MODE_SHIPS`, listed before anything wanted it - and three-with-no-hole is what ADR-0022 licenses. `oag_title::ZoneCraft` is the result and **it splits the corpus exactly the way `ZoneCircuit` does**: Pulse hangs Zone off the entities a race already has (the player's team gains `Zone.vex`, the race circuit gains `zone_track.vex`), Pure and HD give Zone entities of its own (its own ship directory, its own circuits). One decision per title, showing up twice. Pulse's is the only branch with a recovered *selector* (`Ship_LoadModel` `case 6`, 84); the other two are name resolution at 94 and neither executable has been read. **The finding that does not fit the enum, and is the one worth reading**: all three titles ship a Zone-mode craft *directory* whose `handlingstats.xml` opens `<Stats team="ZoneMode">` and authors **no `<Class>` block at all**, where a team file authors four or five - `Data\Ships\Zone_01` on *both* PSP titles (Pulse has one too, which was the surprise) and `/data/ships/zone` on HD. So the shipped answer to "what handling does a Zone craft have" is **one block shared by every team**, which fits a mode that replaces the engine with the auto-speed law, disables the brakes and flies a four-corner hover variant - exactly what a `<Class>` would otherwise carry. **This engine does not read it**: `oag_gameplay::handling_for` `.expect()`s a class and would panic, so a Zone race still flies the player's own team's numbers and `race::load` prints that divergence on every Zone load. Closing it is a physics question (which blocks the mode supplies, and where turning comes from when no class authors one), not a naming one. **Trap**: `Data\Ships\Zone` on Pulse is `<Stats team="Zone">` **with** classes and is the unlockable Zone *livery*, a raceable team - `Zone_01` is the mode, and the two differ by a suffix. **What each title still lacks** is unchanged by this and pre-existing: Pure ships no boost plume under any recovered name (reported, race runs without one), HD's Zone hull draws textured with authored normals (corrected 2026-10-20, below), and both PSP titles' Zone HUD draws its labels with no values behind them (the `IG_HUD_*` substitution gap). `crates/raceplay/tests/zone_ground_truth.rs` is eight tests across four discs.

**2026-10-20 (`zone-hover`): the handling question is answered per title, and Pulse's Zone hover is ported.**
Pulse: a Zone craft flies its own team's `<Class>` for the race's class, which is what this engine does;
`Zone_01` is read by nothing ([zone-rest.md](../../docs/ghidra/functions/psp-pulse-usa/zone-rest.md), 82).
Pulse's `Ship_HoverFourCorner` is read whole, verified live to the bit and ported as
`oag_pulse::race::ZONE_HOVER_RIG` on `RaceDefaults::zone_hover_rig`
([zone-hover.md](../../docs/ghidra/functions/psp-pulse-usa/zone-hover.md)), brakes off included.
HD: its Zone modes (mask `0x206040`) do read `Data\Ships\Zone\handlingstats.xml`, whose blocks sit
directly under `<Stats>`; HD's parser fills **class 0 alone** from such a file (70)
([hover-four-point.md](../../docs/ghidra/functions/ps3-hdfury-eu/hover-four-point.md#zone-the-same-law-plus-the-trailing-ray-and-the-handling-files-one-class-2026-10-20-zone-hover)).

## Open

- **HD's Zone handling**: which class index a Zone race reads from HD's stats block. If it is 0 (or
  forced to 0), an HD Zone craft flies the Zone file's numbers; this engine still flies the team's
  own. Next: the class index `Craft_Update` uses in mode 6, or an RPCS3 read of `engine->stats`
  plus the class slot in a Zone race. Pure's `Zone_01` (marked `type="Zone"`) is the same question.
- **HD's Zone trailing ray** (`Craft_HoverFourPoint` with `r4 = 1`, gain `0.5`) is read, not wired.
  The `Rig::snap` axis carries Pulse's `0.25`; HD needs its gain (and the ray's axis re-read).
- **`ShipState::four_corner` gives every title's Zone bank-to-yaw `50`**; HD's Zone runs
  `Craft_HoverFourPoint`, whose gain is `30`. Pre-existing, not changed by this lane.
- **Pure's four-corner hover** (`0x0892e5b8`) is Pulse's law with three small differences
  (zone-hover.md, "The other titles"); not wired until Pure's mode 6 is confirmed as Zone and its
  brake gate is read.
- **Pulse `05_Track` in Zone**: in a 7200-tick autopilot run on main and on this lane alike, it ends
  one lap in at shield 0 with a mean speed of 12. Not investigated
  (`crates/raceplay/tests/zone_hover_sweep.rs`, `OAG_SWEEP=1`; the sweep walks every forward race
  circuit, so the four without `availableInZone` fail to load there, as they should).
- **Zone grid height**: a Zone craft now spawns at the four-corner law's own rest point
  (`oag_gameplay::spawn::rig_spawn_height`, `target - 1.25`, about 2.9), derived; the original's
  Zone grid height was not measured.
- Pure ships no boost plume under any recovered name.
- **HD's Zone hull, `hd-zone-material` (2026-10-20): the cells and the gloss are played, the normal map is open.** Zone mode flies Zone_1. The probe tint `iblScalePower` is `(0.698, 0.698, 0.698, 1)` there (wired, `env_probes::ProbeTint::ZONE`), and `dg_zonelights1`'s flash is played (`mesh::rcs::damage_lights`; `K` chosen = 6). Open: (1) what moves the tint inside a Zone race (both frames at `Sub Flash`; per-stage `Sky reflection colour` is the lead; the `r17` path at `0x003ad748` unread); (2) what moves `K` (`0xffede334`, 6 at zone 3, 7 at zone 4) and `C` (`0xef18f362`, 0 live); (3) the hull is greyer than the original's white: the live sun colour `(1.144, 1.091, 1.906)` is not compared with what ours feeds; (4) **the normal map is wired** (`hd-hull-normals`, 2026-10-20): every HD ship hull samples its `_n` through the authored vertex tangent, see `docs/rendering/hd-ship-materials.md`, "Hull normal maps"; the sun colour `(1.144, 1.091, 1.906)` is still not composed from any authored stage value; (5) Omega's `hdships/Zone` has `dg_zonelights1` too, not wired on its path. Evidence: `docs/rendering/hd-ship-materials.md`, "The Zone-mode hull". 2048's Zone hull is not checkable (PFS-encrypted).
- Both PSP titles' Zone HUD draws labels with no values (the `IG_HUD_*` substitution gap).

## Next Steps

- HD's Zone class index (above), then decide whether HD's Zone craft flies `Data\Ships\Zone`'s class-0 blocks.
