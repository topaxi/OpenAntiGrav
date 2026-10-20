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

- ~~HD's Zone handling, trailing ray and bank-to-yaw~~: closed 2026-10-20 (`hd-zone-handling`), each
  read live on RPCS3 and wired as data ([hover-four-point.md](../../docs/ghidra/functions/ps3-hdfury-eu/hover-four-point.md#zone-read-live-the-ray-the-class-the-grid-and-the-brakes-2026-10-20-hd-zone-handling)):
  class 0 is the Zone file's (`ZONE_HANDLING`), the ray at `0.5` along the hit normal plus brakes off
  (`ZONE_HOVER_RIG`), no grid clamp in Zone (`LaunchHover::in_zone`), bank-to-yaw `30`
  (`CraftLaws::bank_to_yaw_zone`). Pure's `Zone_01` (marked `type="Zone"`) is still the open
  handling question.
- **HD's Zone wall friction** (`Collision_PairFriction` `+0x38` against `+0x2c` under the mask) is
  not measured: next, an interpreter boot to mode 6 and a `Z0` on `0x000be068`'s return during a
  wall scrape (also in `hd-wall-response-leftovers.md`). `scripts/rpcs3-hd-zone-probe.py` is the cold walk
  into mode 6 (`--rights 5`) that reached Zone; add the breakpoint there.
- **HD's Zone engine**: `Craft_UpdateThrottle` (`0x000efb50`) tests the mask at `0x000efeb8`; not
  compared with this engine's auto-speed law.
- **Omega's Zone**: the file is HD's (checked, applies, not wired: Omega's racing hover is Pulse's
  two-point here, so neither the ray nor the file is wired); its hover selector is unlocated, the
  mask survives at `0x0150bb28` and two more sites. Omega and 2048 fly Zone bank-to-yaw `30`,
  inherited from HD. 2048's mask is not found as a literal (ARM `movw`/`movt` not searched).
- **HD Zone ride height**: ours rests `0.09` high on the grid (`3.29` against `3.20`) and `0.04` in flight
  (`3.28` against `3.24`), `hd_zone_hover_ground_truth.rs`; not chased.
- **HD sweep shield**: `hd_zone_hover_sweep` reports the shield unchanged at 100 on every circuit (it
  read 95, the team file's, before the Zone file was wired); the autopilot takes no damage in 7200
  ticks, so the sweep says nothing about Zone damage.
- **Pure's four-corner hover** (`0x0892e5b8`) is Pulse's law with three small differences
  (zone-hover.md, "The other titles"); not wired until Pure's mode 6 is confirmed as Zone and its
  brake gate is read.
- **Pulse `05_Track` in Zone**: in a 7200-tick autopilot run on main and on this lane alike, it ends
  one lap in at shield 0 with a mean speed of 12. Not investigated
  (`crates/raceplay/tests/zone_hover_sweep.rs`, `OAG_SWEEP=1`; the sweep walks every forward race
  circuit, so the four without `availableInZone` fail to load there, as they should).
- **Zone grid height**: a Zone craft now spawns at the four-corner law's own rest point
  (`oag_gameplay::spawn::rig_spawn_height`, `target - 1.25`, about 2.9), derived; Pulse's Zone grid
  height was not measured. HD's is (centre `3.20` above the floor, `hd_zone_hover_ground_truth.rs`);
  the spawn itself still uses the racing rest height and settles within a few ticks.
- Pure ships no boost plume under any recovered name.
- **HD's Zone hull, `hd-zone-material` (2026-10-20): the cells and the gloss are played, the normal map is open.** Zone mode flies Zone_1. The probe tint `iblScalePower` is `(0.698, 0.698, 0.698, 1)` there (wired, `env_probes::ProbeTint::ZONE`), and `dg_zonelights1`'s flash is played (`mesh::rcs::damage_lights`; `K` chosen = 6). Open: (1) what moves the tint inside a Zone race (both frames at `Sub Flash`; per-stage `Sky reflection colour` is the lead; the `r17` path at `0x003ad748` unread); (2) what moves `K` (`0xffede334`, 6 at zone 3, 7 at zone 4) and `C` (`0xef18f362`, 0 live); (3) **the "hull is greyer than the original's white" reading is unproven: both Zone frames were `Write Color Buffers`-off captures, which read 1.6-1.8x bright (`hd-hull-brightness`, 2026-10-20); re-shoot Zone with it on before chasing it. The probe maps are sRGB-decoded now (the Zone dump's units 4 and 5 carry the same gamma-nibble-7 word), which darkens the Zone hull too.** The live sun colour `(1.144, 1.091, 1.906)` is a constant of the draw, not an artefact, and is not compared with what ours feeds; (4) **the normal map is wired** (`hd-hull-normals`, 2026-10-20): every HD ship hull samples its `_n` through the authored vertex tangent, see `docs/rendering/hd-ship-materials.md`, "Hull normal maps"; the sun colour `(1.144, 1.091, 1.906)` is still not composed from any authored stage value; (5) Omega's `hdships/Zone` has `dg_zonelights1` too, not wired on its path. Evidence: `docs/rendering/hd-ship-materials.md`, "The Zone-mode hull". 2048's Zone hull is not checkable (PFS-encrypted).
- Both PSP titles' Zone HUD draws labels with no values (the `IG_HUD_*` substitution gap).

## Next Steps

- HD's Zone wall friction live (above), then Omega's hover selector so its Zone can follow HD's.
